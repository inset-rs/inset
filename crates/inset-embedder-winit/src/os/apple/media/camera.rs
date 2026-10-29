//! Cameras: AVFoundation's capture devices, listed by a discovery session, and a session on one,
//! an `AVCaptureSession` configured and run on a serial queue of its own, as Apple asks, since
//! starting it blocks. The session asks the user for the camera, and for the microphone where a
//! recording wants sound, at its start; its preview is an `AVCaptureVideoPreviewLayer` hung in a
//! surface, and its other outputs (`outputs`, `frames`) are added to it as they are asked for.
//!
//! The state is written where the system answers, the session's queue and the access requests'
//! handlers, and read on the main thread, so it lives behind a mutex shared with them.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use block2::RcBlock;
use dispatch2::{DispatchQueue, DispatchQueueAttr, DispatchRetained};
use inset_embedder::{
    Camera, CameraFacing, CameraId, CameraState, CameraStatus, Cameras, CaptureSession,
    FrameReader, MediaChangeFuture, MovieOptions, MovieOutput, PhotoOutput, Size, Surface,
};
use objc2::Message;
use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2_av_foundation::{
    AVAuthorizationStatus, AVCaptureDevice, AVCaptureDeviceDiscoverySession, AVCaptureDeviceInput,
    AVCaptureDevicePosition, AVCaptureDeviceType, AVCaptureDeviceTypeBuiltInWideAngleCamera,
    AVCaptureOutput, AVCaptureSession, AVCaptureSessionDidStartRunningNotification,
    AVCaptureSessionDidStopRunningNotification, AVCaptureSessionErrorKey,
    AVCaptureSessionRuntimeErrorNotification, AVCaptureVideoPreviewLayer,
    AVLayerVideoGravityResizeAspect, AVMediaType, AVMediaTypeAudio, AVMediaTypeVideo,
};
use objc2_core_media::CMVideoFormatDescriptionGetDimensions;
use objc2_foundation::{NSArray, NSError, NSString};

use super::frames::CameraFrames;
use super::observing::NotificationObservation;
use super::outputs::{MovieRecorder, PhotoTaker};
use super::show_layer_on;
use crate::media::Changes;

/// AVFoundation's cameras.
pub(crate) struct AvCameras;

impl Cameras for AvCameras {
    fn cameras(&self) -> Vec<Camera> {
        discovered_devices()
            .iter()
            .map(|device| camera_of(&device))
            .collect()
    }

    fn open_session(&self, camera: &Camera) -> Box<dyn CaptureSession> {
        // SAFETY: looking a device up by the id the system gave it.
        let device =
            unsafe { AVCaptureDevice::deviceWithUniqueID(&NSString::from_str(&camera.id.0)) };
        Box::new(AvCaptureSession::new(device))
    }
}

/// The video devices an app may use as cameras: a device's built-in ones, and on macOS the ones
/// plugged in or on another device.
fn discovered_devices() -> Retained<NSArray<AVCaptureDevice>> {
    // SAFETY: AVFoundation's constant, which it exports for the life of the process.
    let built_in = unsafe { AVCaptureDeviceTypeBuiltInWideAngleCamera };
    let types: Vec<&AVCaptureDeviceType> = std::iter::once(built_in)
        .chain(external_device_types())
        .collect();
    let types = NSArray::from_slice(&types);
    // SAFETY: a discovery session of these types, for video.
    unsafe {
        AVCaptureDeviceDiscoverySession::discoverySessionWithDeviceTypes_mediaType_position(
            &types,
            AVMediaTypeVideo,
            AVCaptureDevicePosition::Unspecified,
        )
        .devices()
    }
}

/// The type of the cameras plugged in or on another device, which macOS lists.
#[cfg(target_os = "macos")]
fn external_device_types() -> Option<&'static AVCaptureDeviceType> {
    // `AVCaptureDeviceTypeExternal` succeeds it from macOS 14 with the same value, and would be
    // linked strongly, so that an app could not start on an earlier macOS.
    // SAFETY: AVFoundation's constant, which it exports for the life of the process.
    #[allow(deprecated)]
    Some(unsafe { objc2_av_foundation::AVCaptureDeviceTypeExternalUnknown })
}

/// iOS lists a device's own cameras alone.
#[cfg(not(target_os = "macos"))]
fn external_device_types() -> Option<&'static AVCaptureDeviceType> {
    None
}

/// `device` as the app knows it.
fn camera_of(device: &AVCaptureDevice) -> Camera {
    // SAFETY: reading a device's id, name, place and type.
    let (id, name, position, built_in) = unsafe {
        (
            device.uniqueID().to_string(),
            device.localizedName().to_string(),
            device.position(),
            &*device.deviceType() == AVCaptureDeviceTypeBuiltInWideAngleCamera,
        )
    };
    Camera {
        id: CameraId(id),
        name,
        facing: facing_of(position, built_in),
    }
}

/// Which way a device at `position` looks: a Mac's built-in camera has no position, and looks at
/// the user.
fn facing_of(position: AVCaptureDevicePosition, built_in: bool) -> CameraFacing {
    if position == AVCaptureDevicePosition::Front {
        CameraFacing::Front
    } else if position == AVCaptureDevicePosition::Back {
        CameraFacing::Back
    } else if built_in {
        CameraFacing::Front
    } else {
        CameraFacing::External
    }
}

/// A session on one device.
struct AvCaptureSession {
    shared: Arc<SessionShared>,
    preview: RefCell<Option<Preview>>,
    frames: RefCell<Option<Arc<CameraFrames>>>,
    photos: RefCell<Option<PhotoTaker>>,
    movies: RefCell<Option<MovieRecorder>>,
    _notifications: NotificationObservation,
}

/// The preview layer and the surface it hangs in, which the session holds as a producer does.
struct Preview {
    layer: Retained<AVCaptureVideoPreviewLayer>,
    _surface: Arc<dyn Surface>,
}

/// What the session's queue, the system's answers and the main thread share.
pub(super) struct SessionShared {
    session: CrossThreadSession,
    /// `None` for a camera no longer attached, which fails at its start.
    device: Option<CrossThreadDevice>,
    queue: DispatchRetained<DispatchQueue>,
    state: Mutex<CameraState>,
    changes: Changes,
    /// Whether a recording wants sound, so the session takes the microphone at its next start.
    wants_sound: AtomicBool,
    /// Whether the session has its camera's input already.
    configured: AtomicBool,
}

impl AvCaptureSession {
    fn new(device: Option<Retained<AVCaptureDevice>>) -> AvCaptureSession {
        // SAFETY: a new capture session, not yet running.
        let session = unsafe { AVCaptureSession::new() };
        let shared = Arc::new(SessionShared {
            session: CrossThreadSession(session),
            device: device.map(CrossThreadDevice),
            queue: DispatchQueue::new("rs.inset.camera-session", DispatchQueueAttr::SERIAL),
            state: Mutex::new(CameraState::stopped()),
            changes: Changes::default(),
            wants_sound: AtomicBool::new(false),
            configured: AtomicBool::new(false),
        });
        let notifications = observe_running(&shared);
        AvCaptureSession {
            shared,
            preview: RefCell::new(None),
            frames: RefCell::new(None),
            photos: RefCell::new(None),
            movies: RefCell::new(None),
            _notifications: notifications,
        }
    }
}

impl CaptureSession for AvCaptureSession {
    fn start(&self) {
        if self.shared.device.is_none() {
            self.shared.set_status(CameraStatus::Failed(
                "the camera is no longer attached".to_owned(),
            ));
            return;
        }
        self.shared.set_status(CameraStatus::Starting);
        let shared = Arc::clone(&self.shared);
        ask_for(video(), move |granted| {
            if !granted {
                shared.set_status(CameraStatus::Denied);
                return;
            }
            let wants_sound = shared.wants_sound.load(Ordering::Relaxed);
            let starting = Arc::clone(&shared);
            let run = move |sound: bool| {
                let shared = Arc::clone(&starting);
                starting
                    .queue
                    .exec_async(move || shared.configure_and_run(sound));
            };
            match audio().filter(|_| wants_sound) {
                Some(audio) => ask_for(audio, run),
                None => run(false),
            }
        });
    }

    fn stop(&self) {
        let shared = Arc::clone(&self.shared);
        self.shared.queue.exec_async(move || {
            // SAFETY: stopping the session on its own queue.
            unsafe { shared.session.0.stopRunning() };
            shared.set_status(CameraStatus::Stopped);
        });
    }

    fn state(&self) -> CameraState {
        self.shared.state()
    }

    fn changed(&self) -> MediaChangeFuture {
        self.shared.changes.next()
    }

    fn show_on(&self, surface: Arc<dyn Surface>) {
        let mut preview = self.preview.borrow_mut();
        let layer = match preview.take() {
            Some(Preview { layer, .. }) => layer,
            None => new_preview_layer(&self.shared.session.0),
        };
        show_layer_on(&surface, &layer);
        *preview = Some(Preview {
            layer,
            _surface: surface,
        });
    }

    fn frame_output(&self) -> Arc<dyn FrameReader> {
        self.frames
            .borrow_mut()
            .get_or_insert_with(|| {
                let frames = Arc::new(CameraFrames::new());
                self.shared.add_output(frames.output());
                frames
            })
            .clone()
    }

    fn photo_output(&self) -> Box<dyn PhotoOutput> {
        let photos = self
            .photos
            .borrow_mut()
            .get_or_insert_with(|| {
                let photos = PhotoTaker::new(Arc::clone(&self.shared));
                self.shared.add_output(photos.output());
                photos
            })
            .clone();
        Box::new(photos)
    }

    fn movie_output(&self, options: MovieOptions) -> Box<dyn MovieOutput> {
        if options.sound {
            self.shared.wants_sound.store(true, Ordering::Relaxed);
        }
        let movies = self
            .movies
            .borrow_mut()
            .get_or_insert_with(|| {
                let movies = MovieRecorder::new(Arc::clone(&self.shared));
                self.shared.add_output(movies.output());
                movies
            })
            .clone();
        Box::new(movies)
    }
}

impl Drop for AvCaptureSession {
    /// The camera stops, its preview leaves the surface it showed on, and the app hears the
    /// session gone.
    fn drop(&mut self) {
        if let Some(preview) = self.preview.get_mut().take() {
            super::in_transaction(|| preview.layer.removeFromSuperlayer());
        }
        let shared = Arc::clone(&self.shared);
        // SAFETY: stopping the session on its own queue.
        self.shared
            .queue
            .exec_async(move || unsafe { shared.session.0.stopRunning() });
        self.shared.changes.end();
    }
}

impl SessionShared {
    pub(super) fn state(&self) -> CameraState {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(super) fn queue(&self) -> &DispatchQueue {
        &self.queue
    }

    fn set_status(&self, status: CameraStatus) {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .status = status;
        self.changes.notify();
    }

    fn set_size(&self, size: Option<Size>) {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .size = size;
        self.changes.notify();
    }

    /// Adds `output` to the session on its queue; a session that cannot take it alongside its
    /// other outputs fails.
    fn add_output(self: &Arc<Self>, output: &AVCaptureOutput) {
        let output = CrossThreadOutput(output.retain());
        let shared = Arc::clone(self);
        self.queue.exec_async(move || {
            let output = output;
            let session = &shared.session.0;
            // SAFETY: configuring the session on its own queue.
            unsafe {
                session.beginConfiguration();
                if session.canAddOutput(&output.0) {
                    session.addOutput(&output.0);
                } else {
                    shared.set_status(CameraStatus::Failed(
                        "the camera cannot add this output beside the session's others".to_owned(),
                    ));
                }
                session.commitConfiguration();
            }
        });
    }

    /// Gives the session its camera, and the microphone where `sound` asks, then runs it. On the
    /// session's queue.
    fn configure_and_run(&self, sound: bool) {
        if let Err(reason) = self.configure(sound) {
            self.set_status(CameraStatus::Failed(reason));
            return;
        }
        // SAFETY: running the session on its own queue, which this blocks until it runs.
        unsafe { self.session.0.startRunning() };
        // SAFETY: as above.
        if !unsafe { self.session.0.isRunning() } {
            self.set_status(CameraStatus::Failed("the camera did not start".to_owned()));
        }
    }

    /// Adds the camera's input once, and the microphone's where `sound` asks and none is there.
    fn configure(&self, sound: bool) -> Result<(), String> {
        let device = &self
            .device
            .as_ref()
            .ok_or("the camera is no longer attached")?
            .0;
        let session = &self.session.0;
        // SAFETY: configuring the session on its own queue.
        unsafe {
            session.beginConfiguration();
            let mut added = Ok(());
            if !self.configured.swap(true, Ordering::Relaxed) {
                added = add_input(session, device);
                self.set_size(size_of_format(device));
            }
            if sound && added.is_ok() {
                if let Some(microphone) =
                    audio().and_then(|audio| AVCaptureDevice::defaultDeviceWithMediaType(audio))
                {
                    // Without the microphone, a recording has no sound, and the camera still runs.
                    let _ = add_input(session, &microphone);
                }
                self.wants_sound.store(false, Ordering::Relaxed);
            }
            session.commitConfiguration();
            added
        }
    }
}

/// Adds `device`'s input to `session`, inside its configuration.
fn add_input(session: &AVCaptureSession, device: &AVCaptureDevice) -> Result<(), String> {
    // SAFETY: an input of a device the app may use, added on the session's queue.
    unsafe {
        let input = AVCaptureDeviceInput::deviceInputWithDevice_error(device)
            .map_err(|error| error.localizedDescription().to_string())?;
        if !session.canAddInput(&input) {
            return Err("the session cannot take the device".to_owned());
        }
        session.addInput(&input);
    }
    Ok(())
}

/// The size of the pictures `device`'s active format gives, in its own pixels.
fn size_of_format(device: &AVCaptureDevice) -> Option<Size> {
    // SAFETY: reading the device's active format, whose description is a video one.
    let dimensions = unsafe {
        CMVideoFormatDescriptionGetDimensions(&device.activeFormat().formatDescription())
    };
    (dimensions.width > 0 && dimensions.height > 0)
        .then(|| Size::new(f64::from(dimensions.width), f64::from(dimensions.height)))
}

/// A preview of `session`, showing all of its picture.
fn new_preview_layer(session: &AVCaptureSession) -> Retained<AVCaptureVideoPreviewLayer> {
    // SAFETY: a layer of the session, made on the main thread.
    let layer = unsafe { AVCaptureVideoPreviewLayer::layerWithSession(session) };
    // SAFETY: a constant AVFoundation exports for the life of the process.
    if let Some(gravity) = unsafe { AVLayerVideoGravityResizeAspect } {
        unsafe { layer.setVideoGravity(gravity) };
    }
    layer
}

/// Hears the session start, stop and fail, which the state says.
fn observe_running(shared: &Arc<SessionShared>) -> NotificationObservation {
    // SAFETY: constants AVFoundation exports for the life of the process.
    let (started, stopped, failed) = unsafe {
        (
            AVCaptureSessionDidStartRunningNotification,
            AVCaptureSessionDidStopRunningNotification,
            AVCaptureSessionRuntimeErrorNotification,
        )
    };
    let heard = Arc::downgrade(shared);
    NotificationObservation::new(
        &shared.session.0,
        &[started, stopped, failed],
        move |notification| {
            let Some(shared) = heard.upgrade() else {
                return;
            };
            let name = notification.name();
            if &*name == started {
                shared.set_status(CameraStatus::Running);
            } else if &*name == stopped {
                if shared.state().status == CameraStatus::Running {
                    shared.set_status(CameraStatus::Stopped);
                }
            } else {
                shared.set_status(CameraStatus::Failed(runtime_error_of(notification)));
            }
        },
    )
}

/// Why a session's runtime error notification says it failed.
fn runtime_error_of(notification: &objc2_foundation::NSNotification) -> String {
    // SAFETY: the notification's user info holds an NSError under AVFoundation's key.
    let error = unsafe {
        notification
            .userInfo()
            .and_then(|info| info.objectForKey(AVCaptureSessionErrorKey))
            .and_then(|error| error.downcast::<NSError>().ok())
    };
    error
        .map(|error| error.localizedDescription().to_string())
        .unwrap_or_else(|| "the camera failed".to_owned())
}

/// Calls `answered` with whether the app may use `media`: at once where the user answered, after
/// asking where they have not, on whatever thread the system answers on.
fn ask_for(media: &'static AVMediaType, answered: impl Fn(bool) + Send + Sync + 'static) {
    // SAFETY: reading the app's access to a kind of device.
    let status = unsafe { AVCaptureDevice::authorizationStatusForMediaType(media) };
    if status == AVAuthorizationStatus::Authorized {
        answered(true);
    } else if status == AVAuthorizationStatus::NotDetermined {
        let handler = RcBlock::new(move |granted: Bool| answered(granted.as_bool()));
        // SAFETY: asking the user, with a handler the system calls once.
        unsafe { AVCaptureDevice::requestAccessForMediaType_completionHandler(media, &handler) };
    } else {
        answered(false);
    }
}

fn video() -> &'static AVMediaType {
    // SAFETY: a constant AVFoundation exports for the life of the process.
    unsafe { AVMediaTypeVideo }.expect("AVFoundation's video media type")
}

fn audio() -> Option<&'static AVMediaType> {
    // SAFETY: a constant AVFoundation exports for the life of the process.
    unsafe { AVMediaTypeAudio }
}

/// The session handed between its queue, the system's answers and the main thread.
pub(super) struct CrossThreadSession(pub(super) Retained<AVCaptureSession>);

// SAFETY: a capture session is configured and run on its own queue, and its preview layer made of
// it on the main thread, as Apple's samples do; AVFoundation guards its own state, and the binding
// cannot say so.
unsafe impl Send for CrossThreadSession {}
unsafe impl Sync for CrossThreadSession {}

struct CrossThreadDevice(Retained<AVCaptureDevice>);

// SAFETY: a capture device is read and given to its session on the session's queue alone.
unsafe impl Send for CrossThreadDevice {}
unsafe impl Sync for CrossThreadDevice {}

struct CrossThreadOutput(Retained<AVCaptureOutput>);

// SAFETY: an output is added to its session on the session's queue alone.
unsafe impl Send for CrossThreadOutput {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_faces_where_it_is_placed_and_a_macs_built_in_one_faces_the_user() {
        assert_eq!(
            facing_of(AVCaptureDevicePosition::Back, true),
            CameraFacing::Back
        );
        assert_eq!(
            facing_of(AVCaptureDevicePosition::Front, true),
            CameraFacing::Front
        );
        assert_eq!(
            facing_of(AVCaptureDevicePosition::Unspecified, true),
            CameraFacing::Front
        );
        assert_eq!(
            facing_of(AVCaptureDevicePosition::Unspecified, false),
            CameraFacing::External
        );
    }

    /// Listing asks the user nothing; this Mac's cameras, if it has any, each have an id and a
    /// name.
    #[test]
    fn the_cameras_are_listed_without_asking() {
        for camera in AvCameras.cameras() {
            assert!(!camera.id.0.is_empty() && !camera.name.is_empty());
        }
    }

    /// A session made and given outputs asks the user nothing and runs nothing until it starts.
    #[test]
    fn a_session_is_stopped_until_it_starts() {
        let session = AvCaptureSession::new(None);
        let _photos = session.photo_output();
        let _frames = session.frame_output();
        assert_eq!(session.state(), CameraState::stopped());
        session.start();
        assert!(matches!(session.state().status, CameraStatus::Failed(_)));
    }
}
