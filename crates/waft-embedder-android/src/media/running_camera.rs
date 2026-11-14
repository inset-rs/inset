//! A camera running for this process's sessions: opened once, in one format, its capture session
//! feeding one stream of pictures for the GPU and the compositor (`camera_frames`) and, where the
//! first session takes photos, one of JPEG photos (`photos`), which every session on the camera
//! shares, since Android lets a process open a camera once and gives a camera one capture session
//! at a time. The first session's format and photo size are the camera's; a later session joins
//! them, as a later session on Apple's systems joins the format its camera runs in.
//!
//! Its state follows what Android tells. It runs once its session is configured and repeating. It
//! is interrupted, its device let go of, where Android disconnects it, as another app taking the
//! camera does (`InUseByAnotherApp`), or reports it in use (the same) or disabled (no reason:
//! Android disables a camera for an app in the background and by the device's policy alike); and
//! it is opened again when Android says a camera became available, or the app comes to the front.
//! It ends, for good, where the camera failed, is no longer attached, or the app may not use it.
//!
//! The invariants: a camera is opened, configured, closed and opened again on the cameras' thread
//! alone (`camera_queue`); a device's events are acted on only while that device is the camera's;
//! and an ended camera stays ended, and is joined by no new session.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError, Weak};

use ndk::hardware_buffer::HardwareBufferUsage;
use ndk::media::image_reader::ImageFormat;
use ndk::native_window::NativeWindow;
use ndk_sys::{ACameraDevice_request_template, acamera_metadata_tag, camera_status_t};
use waft_embedder::{
    CameraError, CameraFormat, CameraInterruption, CaptureState, MediaError, PhotoFuture,
};

use super::camera_device::{
    CameraDevice, CaptureRequest, CaptureSession, DeviceError, DeviceEvents,
};
use super::camera_frames::CameraFrames;
use super::camera_images::ImageStream;
use super::camera_manager::CameraManager;
use super::camera_queue::queue;
use super::changes::Changes;
use super::orientation::SensorMount;
use super::photos::Photos;

/// The pictures a camera's stream lends at once: the compositor holds two or three a preview, the
/// newest is kept, and Android's `acquireLatestImage` needs one free beside them.
const PICTURES_LENT: i32 = 8;
/// The photos its JPEG stream lends at once.
const PHOTOS_LENT: i32 = 2;

/// The cameras this process's sessions run.
static RUNNING: Mutex<Vec<Weak<RunningCamera>>> = Mutex::new(Vec::new());

/// How a camera runs, as its first session asks.
pub(super) struct CameraRun {
    pub(super) id: String,
    pub(super) format: CameraFormat,
    /// The auto-exposure target range its session asks for, where the camera lists any.
    pub(super) target_frame_rates: Option<[i32; 2]>,
    /// The size of its photos, where it takes any.
    pub(super) photo_size: Option<[u32; 2]>,
    pub(super) mount: SensorMount,
}

/// A camera running for this process's sessions.
pub(super) struct RunningCamera {
    run: CameraRun,
    frames: Arc<CameraFrames>,
    photos: Arc<Photos>,
    /// The device and its session while the camera is open; `None` before it opens, while it is
    /// interrupted and once it ended.
    open: Mutex<Option<OpenCamera>>,
    /// Which opening the device open now is, so that an earlier device's events are not taken for
    /// its.
    openings: AtomicU64,
    state: Mutex<CaptureState>,
    /// The changes of each session's state, told at each change of the camera's.
    watchers: Mutex<Vec<Changes>>,
}

/// A camera opened: its session closed first as this drops, then its device, then its streams.
struct OpenCamera {
    session: CaptureSession,
    device: CameraDevice,
    picture_window: NativeWindow,
    photo_window: Option<NativeWindow>,
    _pictures: ImageStream,
    _photos: Option<ImageStream>,
}

/// Why a camera did not open, as Android said it.
enum Refusal {
    Status(camera_status_t),
    Failed(String),
}

impl RunningCamera {
    /// The camera `run.id` as this process runs it: the one its sessions run, which the session
    /// asking joins in its format and photo size, or else a new one, not opened yet, as `run`
    /// asks.
    pub(super) fn share(run: CameraRun) -> Arc<RunningCamera> {
        let mut running = RUNNING.lock().unwrap_or_else(PoisonError::into_inner);
        running.retain(|camera| camera.strong_count() > 0);
        let joined = running
            .iter()
            .filter_map(Weak::upgrade)
            .find(|camera| camera.run.id == run.id && !camera.has_ended());
        if let Some(camera) = joined {
            return camera;
        }
        let camera = Arc::new(RunningCamera {
            frames: CameraFrames::new(run.mount),
            photos: Arc::default(),
            run,
            open: Mutex::new(None),
            openings: AtomicU64::new(0),
            state: Mutex::new(CaptureState::Running),
            watchers: Mutex::new(Vec::new()),
        });
        running.push(Arc::downgrade(&camera));
        listen_for_cameras_available();
        camera
    }

    pub(super) fn format(&self) -> &CameraFormat {
        &self.run.format
    }

    pub(super) fn frames(&self) -> &Arc<CameraFrames> {
        &self.frames
    }

    pub(super) fn state(&self) -> CaptureState {
        self.state_guard().clone()
    }

    /// The turn that shows the camera's pictures upright on the screen now, in degrees clockwise:
    /// its sessions' preview and capture angles alike.
    pub(super) fn rotation_angle(&self) -> u32 {
        self.run.mount.rotation_angle()
    }

    /// Opens the camera where it is not open yet, on the cameras' thread, and answers whether it
    /// started: it runs, or is interrupted, from which it resumes by itself, or why it did not.
    pub(super) fn start(self: &Arc<Self>) -> Result<(), CameraError> {
        if self.lock_open().is_none()
            && !self.has_ended()
            && let Err(refusal) = self.open_device()
        {
            let error = start_error_of(&refusal, &self.run.id);
            self.end(error);
        }
        match self.state() {
            CaptureState::Ended(error) => Err(error),
            _ => Ok(()),
        }
    }

    /// Tells `changes` at each change of the camera's state, until `unwatch`.
    pub(super) fn watch(&self, changes: Changes) {
        self.watchers().push(changes);
    }

    pub(super) fn unwatch(&self, changes: &Changes) {
        self.watchers().retain(|watcher| !watcher.is(changes));
    }

    /// Takes a photo with the camera's JPEG stream, answered once Android delivers it.
    pub(super) fn take_photo(self: &Arc<Self>) -> PhotoFuture {
        if self.run.photo_size.is_none() {
            return Box::pin(std::future::ready(Err(MediaError::Failed(
                "the camera runs without photos: the session it runs for first started with none"
                    .to_owned(),
            ))));
        }
        if self.state() != CaptureState::Running {
            return Box::pin(std::future::ready(Err(MediaError::NotRunning)));
        }
        let (number, answered) = self.photos.expect();
        let camera = Arc::clone(self);
        queue(move || camera.capture_still(number));
        Box::pin(answered)
    }

    /// Asks Android for the still `number` into the JPEG stream, turned as the screen is now. On
    /// the cameras' thread.
    fn capture_still(&self, number: u64) {
        let open = self.lock_open();
        let Some(open) = open.as_ref() else {
            self.photos.failed(number, MediaError::NotRunning);
            return;
        };
        let photos = Arc::clone(&self.photos);
        let asked = still_request(open, self.run.mount).and_then(|request| {
            open.session.capture(
                &request,
                Box::new(move || {
                    photos.failed(
                        number,
                        MediaError::Failed("the camera did not take the photo".to_owned()),
                    );
                }),
            )
        });
        if let Err(status) = asked {
            self.photos.failed(
                number,
                MediaError::Failed(format!(
                    "Android refused the photo: camera status {}",
                    status.0
                )),
            );
        }
    }

    /// Opens the device, its streams and its session, repeating the request that feeds its
    /// pictures, and runs. On the cameras' thread.
    fn open_device(self: &Arc<Self>) -> Result<(), Refusal> {
        let manager = CameraManager::get()
            .ok_or_else(|| Refusal::Failed("the system has no camera manager".to_owned()))?;
        let opening = self.openings.fetch_add(1, Ordering::Relaxed) + 1;
        let device = manager
            .open(&self.run.id, self.device_events(opening))
            .map_err(Refusal::Status)?;
        let pictures = self.picture_stream()?;
        let photos = self.photo_stream()?;
        let picture_window = window_of(&pictures)?;
        let photo_window = photos.as_ref().map(window_of).transpose()?;
        let windows = std::iter::once(picture_window.clone())
            .chain(photo_window.clone())
            .collect();
        let session = device.session(windows).map_err(Refusal::Status)?;
        let repeating = self.repeating_request(&device, &picture_window)?;
        session.set_repeating(&repeating).map_err(Refusal::Status)?;
        *self.lock_open() = Some(OpenCamera {
            session,
            device,
            picture_window,
            photo_window,
            _pictures: pictures,
            _photos: photos,
        });
        self.change(CaptureState::Running);
        Ok(())
    }

    /// The stream of the camera's pictures, each handed to its frames.
    fn picture_stream(&self) -> Result<ImageStream, Refusal> {
        let size = [
            self.run.format.size.width() as u32,
            self.run.format.size.height() as u32,
        ];
        let frames = Arc::downgrade(&self.frames);
        ImageStream::new(
            size,
            ImageFormat::PRIVATE,
            HardwareBufferUsage::GPU_SAMPLED_IMAGE | HardwareBufferUsage::COMPOSER_OVERLAY,
            PICTURES_LENT,
            move |picture| {
                if let Some(frames) = frames.upgrade() {
                    frames.delivered(picture);
                }
            },
        )
        .map_err(|error| Refusal::Failed(format!("no stream of the camera's pictures: {error}")))
    }

    /// The stream of the camera's photos, each answering the photo asked first; none where the
    /// camera takes none.
    fn photo_stream(&self) -> Result<Option<ImageStream>, Refusal> {
        let Some(size) = self.run.photo_size else {
            return Ok(None);
        };
        let photos = Arc::downgrade(&self.photos);
        ImageStream::new(
            size,
            ImageFormat::JPEG,
            HardwareBufferUsage::CPU_READ_OFTEN,
            PHOTOS_LENT,
            move |photo| {
                if let Some(photos) = photos.upgrade() {
                    photos.delivered(photo);
                }
            },
        )
        .map(Some)
        .map_err(|error| Refusal::Failed(format!("no stream of the camera's photos: {error}")))
    }

    /// The request that feeds the camera's pictures, at the frame rates its format keeps.
    fn repeating_request(
        &self,
        device: &CameraDevice,
        picture_window: &NativeWindow,
    ) -> Result<CaptureRequest, Refusal> {
        let mut request = device
            .request(ACameraDevice_request_template::TEMPLATE_PREVIEW)
            .map_err(Refusal::Status)?;
        request
            .add_target(picture_window)
            .map_err(Refusal::Status)?;
        if let Some(rates) = self.run.target_frame_rates {
            request
                .set_i32s(
                    acamera_metadata_tag::ACAMERA_CONTROL_AE_TARGET_FPS_RANGE,
                    &rates,
                )
                .map_err(Refusal::Status)?;
        }
        Ok(request)
    }

    /// What the device opened as `opening` tells, acted on, on the cameras' thread, while it is
    /// still the camera's.
    fn device_events(self: &Arc<Self>, opening: u64) -> DeviceEvents {
        let disconnected = Arc::downgrade(self);
        let failed = Arc::downgrade(self);
        DeviceEvents {
            disconnected: Box::new(move || {
                let camera = disconnected.clone();
                queue(move || {
                    if let Some(camera) =
                        camera.upgrade().filter(|camera| camera.is_opening(opening))
                    {
                        camera.disconnected();
                    }
                });
            }),
            failed: Box::new(move |error| {
                let camera = failed.clone();
                queue(move || {
                    if let Some(camera) =
                        camera.upgrade().filter(|camera| camera.is_opening(opening))
                    {
                        camera.device_failed(error);
                    }
                });
            }),
        }
    }

    fn is_opening(&self, opening: u64) -> bool {
        self.openings.load(Ordering::Relaxed) == opening && self.lock_open().is_some()
    }

    /// Android disconnected the device: another app has the camera, or it is no longer attached.
    fn disconnected(&self) {
        if is_attached(&self.run.id) {
            self.interrupt(Some(CameraInterruption::InUseByAnotherApp));
        } else {
            self.end(CameraError::Unplugged);
        }
    }

    /// Android's `onError` for the device.
    fn device_failed(&self, error: DeviceError) {
        match error {
            DeviceError::CameraInUse | DeviceError::MaxCamerasInUse => {
                self.interrupt(Some(CameraInterruption::InUseByAnotherApp));
            }
            DeviceError::CameraDisabled => self.interrupt(None),
            DeviceError::CameraDevice => {
                self.end(CameraError::Failed("the camera failed".to_owned()))
            }
            DeviceError::CameraService => {
                self.end(CameraError::Failed("the camera service failed".to_owned()))
            }
        }
    }

    /// Opens an interrupted camera again; it stays interrupted where Android still keeps it from
    /// the app, and ends for what will not pass. On the cameras' thread.
    fn resume(self: &Arc<Self>) {
        if !matches!(self.state(), CaptureState::Interrupted(_)) || self.lock_open().is_some() {
            return;
        }
        let Err(refusal) = self.open_device() else {
            return;
        };
        match refusal {
            Refusal::Status(
                camera_status_t::ACAMERA_ERROR_CAMERA_IN_USE
                | camera_status_t::ACAMERA_ERROR_MAX_CAMERA_IN_USE
                | camera_status_t::ACAMERA_ERROR_CAMERA_DISABLED,
            ) => {}
            refusal => self.end(start_error_of(&refusal, &self.run.id)),
        }
    }

    /// No pictures for now, for `reason`: the device is let go of.
    fn interrupt(&self, reason: Option<CameraInterruption>) {
        let open = self.lock_open().take();
        drop(open);
        self.change(CaptureState::Interrupted(reason));
    }

    /// Ended, for good, for `error`: the device is let go of.
    fn end(&self, error: CameraError) {
        let open = self.lock_open().take();
        drop(open);
        self.change(CaptureState::Ended(error));
    }

    /// The state is `next` from now on, unless it ended, and every session hears of a change.
    fn change(&self, next: CaptureState) {
        {
            let mut state = self.state_guard();
            if matches!(*state, CaptureState::Ended(_)) || *state == next {
                return;
            }
            *state = next;
        }
        for watcher in self.watchers().iter() {
            watcher.notify();
        }
    }

    fn has_ended(&self) -> bool {
        matches!(*self.state_guard(), CaptureState::Ended(_))
    }

    fn state_guard(&self) -> MutexGuard<'_, CaptureState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock_open(&self) -> MutexGuard<'_, Option<OpenCamera>> {
        self.open.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn watchers(&self) -> MutexGuard<'_, Vec<Changes>> {
        self.watchers.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Drop for RunningCamera {
    /// The last session let go of the camera: its device is closed on the cameras' thread.
    fn drop(&mut self) {
        let open = self
            .open
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(open) = open {
            queue(move || drop(open));
        }
    }
}

/// Tells every session on a camera this process runs that its rotation angles changed, the screen
/// having turned.
pub(crate) fn tell_turned() {
    for camera in running() {
        for watcher in camera.watchers().iter() {
            watcher.notify();
        }
    }
}

/// The cameras this process's sessions run.
fn running() -> Vec<Arc<RunningCamera>> {
    RUNNING
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .filter_map(Weak::upgrade)
        .collect()
}

/// Opens again, on the cameras' thread, each camera this process's sessions run that is
/// interrupted: Android may give it back now.
pub(crate) fn resume_interrupted() {
    queue(|| {
        for camera in running() {
            camera.resume();
        }
    });
}

/// Hears, once for the process, each time Android says a camera's availability changed, which
/// may give an interrupted camera back.
fn listen_for_cameras_available() {
    static LISTENING: OnceLock<()> = OnceLock::new();
    LISTENING.get_or_init(|| {
        if let Some(manager) = CameraManager::get() {
            manager.on_availability_change(resume_interrupted);
        }
    });
}

/// The request of one still, into the JPEG stream and the pictures' stream, its photo turned as
/// `mount` is on the screen now.
fn still_request(open: &OpenCamera, mount: SensorMount) -> Result<CaptureRequest, camera_status_t> {
    let photo_window = open
        .photo_window
        .as_ref()
        .ok_or(camera_status_t::ACAMERA_ERROR_INVALID_OPERATION)?;
    let mut request = open
        .device
        .request(ACameraDevice_request_template::TEMPLATE_STILL_CAPTURE)?;
    request.add_target(photo_window)?;
    request.add_target(&open.picture_window)?;
    request.set_i32s(
        acamera_metadata_tag::ACAMERA_JPEG_ORIENTATION,
        &[mount.rotation_angle() as i32],
    )?;
    Ok(request)
}

fn window_of(stream: &ImageStream) -> Result<NativeWindow, Refusal> {
    stream
        .window()
        .map_err(|error| Refusal::Failed(format!("no window of a camera's stream: {error}")))
}

/// Whether the camera `id` is attached.
fn is_attached(id: &str) -> bool {
    CameraManager::get()
        .is_some_and(|manager| manager.camera_ids().iter().any(|listed| listed == id))
}

/// Why a camera did not start, as the app knows it.
fn start_error_of(refusal: &Refusal, id: &str) -> CameraError {
    match refusal {
        Refusal::Status(camera_status_t::ACAMERA_ERROR_PERMISSION_DENIED)
        | Refusal::Status(camera_status_t::ACAMERA_ERROR_CAMERA_DISABLED) => {
            CameraError::NotAllowed
        }
        Refusal::Status(
            camera_status_t::ACAMERA_ERROR_CAMERA_IN_USE
            | camera_status_t::ACAMERA_ERROR_MAX_CAMERA_IN_USE,
        ) => CameraError::InUse,
        Refusal::Status(_) if !is_attached(id) => CameraError::Unplugged,
        Refusal::Status(status) => {
            CameraError::Failed(format!("Android's camera status {}", status.0))
        }
        Refusal::Failed(reason) => CameraError::Failed(reason.clone()),
    }
}
