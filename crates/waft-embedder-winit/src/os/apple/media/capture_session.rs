//! A camera running for the app: an `AVCaptureSession` given its camera, the camera's format and
//! every output at once, between `beginConfiguration` and `commitConfiguration`, then run, on a
//! serial queue of its own, where Apple's documentation of `AVCaptureSession` starts a session,
//! since `startRunning` blocks; it first asks for the microphone where a recording wants sound.
//! The app is answered the session once it runs, or is interrupted, or why it did not start: a
//! configuration the system refuses, or the runtime error Apple's documentation of
//! `startRunning` posts for a session that fails to start. Its preview is an
//! `AVCaptureVideoPreviewLayer` shown as a composition layer's content, its frames a video data
//! output a reader reads (`frames`), the same output the composition layer reads what its preview
//! shows from (`layer_pictures`), its photos and recordings the system's outputs (`outputs`).
//! Its state follows AVFoundation's notices: an interruption, its end, and a runtime error, which
//! ends the session as a configuration the system refuses does. Its rotation angles are its
//! coordinator's (`camera_rotation`), kept applied to its preview layer's connection and, where it
//! asked for upright frames, to its video data output's, which then turns and mirrors the frames
//! it delivers; a change of them is a change the app hears.
//!
//! The state is written where the system answers, the session's queue and the main queue's
//! notices, and read on the main thread, so it lives behind a mutex shared with them. The
//! invariant: an ended session stays ended, the start is answered once, at the first of a run,
//! an interruption or an end, and a session the app let go of before its queue configured it
//! never runs.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use dispatch2::{DispatchQueue, DispatchQueueAttr, DispatchRetained};
use objc2::rc::Retained;
use objc2::{ClassType, Message};
use objc2_av_foundation::{
    AVCaptureDevice, AVCaptureDeviceInput, AVCaptureOutput, AVCaptureSession,
    AVCaptureSessionErrorKey, AVCaptureSessionInterruptionEndedNotification,
    AVCaptureSessionInterruptionReason, AVCaptureSessionRuntimeErrorNotification,
    AVCaptureSessionWasInterruptedNotification, AVCaptureVideoPreviewLayer, AVError,
    AVFoundationErrorDomain, AVLayerVideoGravityResizeAspect,
};
use objc2_core_media::CMVideoDimensions;
use objc2_foundation::{NSError, NSNotification};
use waft_embedder::{
    CameraError, CameraFormat, CameraInterruption, CameraOutputs, CameraStartFuture,
    CaptureSession, CaptureState, CompositionLayer, MediaChangeFuture, MediaError, PhotoFuture,
    RecordingFuture, Size, VideoFrameReader,
};

use super::camera::{CrossThreadDevice, ask_for, audio, camera_error_of_unanswered, faces_user};
use super::camera_formats::{device_format_for, photo_dimensions_of};
use super::camera_rotation::{RotationAngles, SessionRotation, deliver_upright, turn_connection};
use super::camera_sharing::CameraShare;
use super::frames::{
    AvFrameReader, CameraFrames, FrameOutput, report_foreign_reader, report_reader_in_use,
};
use super::layer_pictures::CameraPicture;
use super::observing::NotificationObservation;
use super::outputs::{MovieRecorder, PhotoTaker};
use super::{SystemPicture, in_transaction, show_system_layer};
use crate::media::{Changes, MediaAnswer, media_answer};

/// A session running one device.
pub(super) struct AvCaptureSession {
    shared: Arc<SessionShared>,
    share: CameraShare,
    rotation: SessionRotation,
    preview: Option<Preview>,
    reader: Option<FedReader>,
    photos: Option<PhotoTaker>,
    movies: Option<MovieRecorder>,
    _notices: NotificationObservation,
}

/// The preview layer and the composition layer it is the content of, which the session holds as
/// a producer does, and what the composition layer reads it shows from.
struct Preview {
    layer: Retained<AVCaptureVideoPreviewLayer>,
    _picture: Arc<dyn SystemPicture>,
    _shown_on: Arc<dyn CompositionLayer>,
}

/// The video data output and the reader that reads it.
struct FedReader {
    frames: Arc<CameraFrames>,
    reader: Arc<AvFrameReader>,
}

/// What the session's queue, the system's answers and the main thread share.
pub(super) struct SessionShared {
    session: CrossThreadSession,
    queue: DispatchRetained<DispatchQueue>,
    state: Mutex<CaptureState>,
    changes: Changes,
    /// The start's answer, until it is given.
    started: Mutex<Option<StartAnswer>>,
    /// Whether the app let go of the session.
    stopped: AtomicBool,
}

/// Whether the session started: it runs or is interrupted, or why it did not.
type StartAnswer = MediaAnswer<Result<(), CameraError>>;

/// What the session's queue gives the session, all at once.
struct Configuration {
    device: CrossThreadDevice,
    /// The format to set the camera to; `None` where the session joined the one it runs in.
    format: Option<CameraFormat>,
    outputs: Vec<CrossThreadOutput>,
    photos: Option<PhotoTaker>,
    /// The preview layer, whose connection is turned to the preview's angle once it has one.
    preview: Option<CrossThreadPreviewLayer>,
    /// The frames' output, delivering upright frames where the session asked for them.
    upright_frames: Option<Arc<CameraFrames>>,
    angles: Arc<RotationAngles>,
    faces_user: bool,
}

impl AvCaptureSession {
    /// Starts `device` in `share`'s format, feeding `outputs`; answers the session once it runs
    /// or is interrupted, or why it did not start.
    pub(super) fn start(
        device: Retained<AVCaptureDevice>,
        share: CameraShare,
        outputs: CameraOutputs,
    ) -> CameraStartFuture {
        let (started, answered) = media_answer();
        let session = AvCaptureSession::run(device, share, outputs, started);
        Box::pin(async move {
            let started = answered.await.map_err(camera_error_of_unanswered)?;
            started.map(|()| Box::new(session) as Box<dyn CaptureSession>)
        })
    }

    /// Runs `device` in `share`'s format, feeding `outputs`, telling `started` whether it started.
    fn run(
        device: Retained<AVCaptureDevice>,
        share: CameraShare,
        outputs: CameraOutputs,
        started: StartAnswer,
    ) -> AvCaptureSession {
        let shared = Arc::new(SessionShared::new(started));
        let notices = observe_notices(&shared);
        let faces_user = faces_user(&device);
        let angles = Arc::new(RotationAngles::default());
        let frames = (outputs.preview.is_some() || outputs.frames.is_some())
            .then(|| Arc::new(CameraFrames::new(Arc::clone(&angles), faces_user)));
        let preview = outputs
            .preview
            .zip(frames.clone())
            .map(|(shown_on, frames)| Preview::show(&shared.session.0, shown_on, frames));
        let reader = outputs
            .frames
            .zip(frames.clone())
            .and_then(|(reader, frames)| feed(reader, frames));
        // The video data output, which the preview and the reader hold, is the session's to feed;
        // a reader refused reads nothing, so a session without a preview needs none then.
        let frames = frames.filter(|_| preview.is_some() || reader.is_some());
        let photos = outputs
            .photos
            .map(|size| PhotoTaker::new(Arc::clone(&shared), photo_size_in(share.format(), size)));
        let movies = outputs
            .movies
            .map(|_| MovieRecorder::new(Arc::clone(&shared)));
        let upright_frames = frames.clone().filter(|_| outputs.upright_frames);
        let rotation = keep_turned(
            &device,
            &shared,
            preview.as_ref().map(|preview| preview.layer.clone()),
            upright_frames.clone(),
            Arc::clone(&angles),
        );
        let configuration = Configuration {
            device: CrossThreadDevice(device),
            format: share.sets_format().then(|| share.format().clone()),
            outputs: outputs_of(frames.as_deref(), photos.as_ref(), movies.as_ref()),
            photos: photos.clone(),
            preview: preview
                .as_ref()
                .map(|preview| CrossThreadPreviewLayer(preview.layer.clone())),
            upright_frames,
            angles,
            faces_user,
        };
        let sound = outputs.movies.is_some_and(|options| options.sound);
        shared.configure_and_run(configuration, sound);
        AvCaptureSession {
            shared,
            share,
            rotation,
            preview,
            reader,
            photos,
            movies,
            _notices: notices,
        }
    }
}

impl CaptureSession for AvCaptureSession {
    fn format(&self) -> CameraFormat {
        self.share.format().clone()
    }

    fn state(&self) -> CaptureState {
        self.shared.state()
    }

    fn video_rotation_angle_for_horizon_level_preview(&self) -> u32 {
        self.rotation.angles().preview()
    }

    fn video_rotation_angle_for_horizon_level_capture(&self) -> u32 {
        self.rotation.angles().capture()
    }

    fn changed(&self) -> MediaChangeFuture {
        self.shared.changes.next()
    }

    fn take_photo(&self) -> PhotoFuture {
        match &self.photos {
            Some(photos) => photos.take(),
            None => not_an_output("photos"),
        }
    }

    fn start_recording(&self, path: PathBuf) -> RecordingFuture {
        match &self.movies {
            Some(movies) => movies.start(path),
            None => not_an_output("movies"),
        }
    }

    fn stop_recording(&self) {
        if let Some(movies) = &self.movies {
            movies.stop();
        }
    }
}

impl Drop for AvCaptureSession {
    /// The camera stops, its preview leaves the layer it showed on, its reader reads nothing
    /// more, and the app hears the session gone; the share goes after, letting go of the camera.
    fn drop(&mut self) {
        if let Some(preview) = self.preview.take() {
            in_transaction(|| preview.layer.removeFromSuperlayer());
        }
        if let Some(fed) = self.reader.take() {
            fed.reader
                .stop_reading(&(fed.frames as Arc<dyn FrameOutput>));
        }
        self.shared.stop();
    }
}

impl Preview {
    /// A preview of `session`, showing all of its picture, as `shown_on`'s content, which reads
    /// what it shows from `frames`, the session's video data output.
    fn show(
        session: &AVCaptureSession,
        shown_on: Arc<dyn CompositionLayer>,
        frames: Arc<CameraFrames>,
    ) -> Preview {
        // SAFETY: a layer of the session, made on the main thread.
        let layer = unsafe { AVCaptureVideoPreviewLayer::layerWithSession(session) };
        // SAFETY: a constant AVFoundation exports for the life of the process.
        if let Some(gravity) = unsafe { AVLayerVideoGravityResizeAspect } {
            unsafe { layer.setVideoGravity(gravity) };
        }
        let picture: Arc<dyn SystemPicture> = Arc::new(CameraPicture(frames));
        show_system_layer(&shown_on, &layer, Some(Arc::downgrade(&picture)));
        Preview {
            layer,
            _picture: picture,
            _shown_on: shown_on,
        }
    }
}

/// `frames`, the session's video data output, read by `reader`; `None`, the camera's frames going
/// to no reader, for a reader this host did not make or one another source feeds, which the host
/// says once.
fn feed(reader: Arc<dyn VideoFrameReader>, frames: Arc<CameraFrames>) -> Option<FedReader> {
    let Some(reader) = AvFrameReader::downcast_from(reader) else {
        report_foreign_reader();
        return None;
    };
    if !reader.read(Arc::clone(&frames) as Arc<dyn FrameOutput>) {
        report_reader_in_use();
        return None;
    }
    Some(FedReader { frames, reader })
}

/// The rotation coordinator of `device` and the session's `preview_layer`, its angles kept applied
/// as they change, on the main thread: the preview's to the preview layer's connection, and the
/// capture's to the video data output's connection of `upright_frames`, on the session's queue,
/// where the session asked for upright frames; and each change told to the app.
fn keep_turned(
    device: &AVCaptureDevice,
    shared: &Arc<SessionShared>,
    preview_layer: Option<Retained<AVCaptureVideoPreviewLayer>>,
    upright_frames: Option<Arc<CameraFrames>>,
    angles: Arc<RotationAngles>,
) -> SessionRotation {
    let heard = Arc::downgrade(shared);
    let layer = preview_layer.clone();
    SessionRotation::new(
        device,
        preview_layer.as_deref().map(|layer| layer.as_super()),
        angles,
        move |angles| {
            // SAFETY: the preview layer's connection, read on the main thread.
            if let Some(connection) = layer
                .as_ref()
                .and_then(|layer| unsafe { layer.connection() })
            {
                turn_connection(&connection, angles.preview());
            }
            let Some(shared) = heard.upgrade() else {
                return;
            };
            if let Some(frames) = upright_frames.clone() {
                let capture = angles.capture();
                shared.queue.exec_async(move || {
                    if let Some(connection) = frames.video_connection() {
                        turn_connection(&connection, capture);
                    }
                });
            }
            shared.changes.notify();
        },
    )
}

/// The AVFoundation outputs of the frames, photos and movies the app asked for.
fn outputs_of(
    frames: Option<&CameraFrames>,
    photos: Option<&PhotoTaker>,
    movies: Option<&MovieRecorder>,
) -> Vec<CrossThreadOutput> {
    [
        frames.map(|frames| frames.output().as_super()),
        photos.map(PhotoTaker::output),
        movies.map(MovieRecorder::output),
    ]
    .into_iter()
    .flatten()
    .map(|output| CrossThreadOutput(output.retain()))
    .collect()
}

/// The photo size the app asked for, where the format the camera runs in lists it; `None` leaves
/// the system's own, which Apple documents as the format's smallest.
fn photo_size_in(format: &CameraFormat, size: Size) -> Option<CMVideoDimensions> {
    format
        .photo_sizes
        .contains(&size)
        .then(|| photo_dimensions_of(size))
}

/// What asking a session for an output it was not given answers.
fn not_an_output<T: 'static>(
    outputs: &str,
) -> Pin<Box<dyn Future<Output = Result<T, MediaError>>>> {
    let reason = format!("the session has no {outputs}: they were not among its outputs");
    Box::pin(std::future::ready(Err(MediaError::Failed(reason))))
}

impl SessionShared {
    /// A session not yet configured, which tells `started` whether it started.
    fn new(started: StartAnswer) -> SessionShared {
        SessionShared {
            // SAFETY: a new capture session, not yet running.
            session: CrossThreadSession(unsafe { AVCaptureSession::new() }),
            queue: DispatchQueue::new("rs.waft.camera-session", DispatchQueueAttr::SERIAL),
            state: Mutex::new(CaptureState::Running),
            changes: Changes::default(),
            started: Mutex::new(Some(started)),
            stopped: AtomicBool::new(false),
        }
    }

    pub(super) fn state(&self) -> CaptureState {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(super) fn queue(&self) -> &DispatchQueue {
        &self.queue
    }

    fn interrupt(&self, interruption: Option<CameraInterruption>) {
        self.change_unless_ended(CaptureState::Interrupted(interruption));
        self.answer_start(Ok(()));
    }

    fn resume(&self) {
        self.change_unless_ended(CaptureState::Running);
        self.answer_start(Ok(()));
    }

    fn end(&self, error: CameraError) {
        self.answer_start(Err(error.clone()));
        self.change_unless_ended(CaptureState::Ended(error));
    }

    /// Answers the start with `started`, unless it was answered already.
    fn answer_start(&self, started: Result<(), CameraError>) {
        let answer = self
            .started
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(answer) = answer {
            answer.give(Ok(started));
        }
    }

    fn change_unless_ended(&self, next: CaptureState) {
        {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            if matches!(*state, CaptureState::Ended(_)) || *state == next {
                return;
            }
            *state = next;
        }
        self.changes.notify();
    }

    /// Gives the session `configuration`, with the microphone where `sound` asks and the user
    /// allows it, and runs it, on its queue.
    fn configure_and_run(self: &Arc<Self>, configuration: Configuration, sound: bool) {
        let configuring = Arc::clone(self);
        let configuration = Mutex::new(Some(configuration));
        let run = move |sound: bool| {
            let Some(configuration) = configuration
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take()
            else {
                return;
            };
            let shared = Arc::clone(&configuring);
            configuring
                .queue
                .exec_async(move || shared.run_configured(&configuration, sound));
        };
        match audio().filter(|_| sound) {
            Some(audio) => ask_for(audio, run),
            None => run(false),
        }
    }

    /// Configures the session and runs it, unless the app let go of it first. On the session's
    /// queue.
    fn run_configured(&self, configuration: &Configuration, sound: bool) {
        if self.stopped.load(Ordering::Relaxed) {
            return;
        }
        if let Err(error) = self.configure(configuration, sound) {
            self.end(error);
            return;
        }
        // SAFETY: running the session on its own queue, which this blocks until it runs; a
        // failure to start comes as a runtime error notice, as Apple documents `startRunning`, and
        // a start iOS defers as an interruption notice, as Apple documents its reasons.
        let running = unsafe {
            self.session.0.startRunning();
            self.session.0.isRunning()
        };
        if running {
            self.answer_start(Ok(()));
        }
    }

    /// Gives the session its camera, the camera's format, its outputs and, where `sound` asks, the
    /// microphone, together; then the photo size, which Apple's documentation of
    /// `maxPhotoDimensions` asks for of the format as the session runs it.
    fn configure(&self, configuration: &Configuration, sound: bool) -> Result<(), CameraError> {
        let session = &self.session.0;
        // SAFETY: configuring the session on its own queue.
        unsafe { session.beginConfiguration() };
        let added =
            add_all(session, configuration, sound).and_then(|()| turn_upright(configuration));
        // SAFETY: as above.
        unsafe { session.commitConfiguration() };
        added?;
        turn_preview(configuration);
        if let Some(photos) = &configuration.photos {
            photos.prepare();
        }
        Ok(())
    }

    /// The app let go of the session: it stops, and its changes end.
    fn stop(self: &Arc<Self>) {
        self.stopped.store(true, Ordering::Relaxed);
        let shared = Arc::clone(self);
        // SAFETY: stopping the session on its own queue.
        self.queue
            .exec_async(move || unsafe { shared.session.0.stopRunning() });
        self.changes.end();
    }
}

/// Has the frames' output deliver its frames upright, where the session asked for it, inside its
/// configuration, as Apple asks of a turn that reconfigures the pipeline.
fn turn_upright(configuration: &Configuration) -> Result<(), CameraError> {
    let Some(frames) = &configuration.upright_frames else {
        return Ok(());
    };
    let connection = frames.video_connection().ok_or_else(|| {
        CameraError::NotSupported("the camera's frames have no connection to turn".to_owned())
    })?;
    deliver_upright(
        &connection,
        configuration.angles.capture(),
        configuration.faces_user,
    )
}

/// Turns the preview layer's connection, which it has once the session has its camera, to the
/// preview's angle.
fn turn_preview(configuration: &Configuration) {
    let Some(layer) = &configuration.preview else {
        return;
    };
    // SAFETY: the preview layer's connection, which AVFoundation guards.
    if let Some(connection) = unsafe { layer.0.connection() } {
        turn_connection(&connection, configuration.angles.preview());
    }
}

/// Adds the camera's input, sets the format, adds each output and the microphone, inside
/// `session`'s configuration.
fn add_all(
    session: &AVCaptureSession,
    configuration: &Configuration,
    sound: bool,
) -> Result<(), CameraError> {
    let device = &configuration.device.0;
    add_input(session, device)?;
    if let Some(format) = &configuration.format {
        set_format(device, format)?;
    }
    for output in &configuration.outputs {
        // SAFETY: adding an output on the session's queue.
        unsafe {
            if !session.canAddOutput(&output.0) {
                return Err(CameraError::Failed(
                    "the camera cannot feed this output beside the session's others".to_owned(),
                ));
            }
            session.addOutput(&output.0);
        }
    }
    if sound {
        add_microphone(session);
    }
    Ok(())
}

/// Adds `device`'s input to `session`, inside its configuration.
fn add_input(session: &AVCaptureSession, device: &AVCaptureDevice) -> Result<(), CameraError> {
    // SAFETY: an input of a device the app may use, added on the session's queue.
    unsafe {
        let input = AVCaptureDeviceInput::deviceInputWithDevice_error(device)
            .map_err(|error| camera_error_of(&error))?;
        if !session.canAddInput(&input) {
            return Err(CameraError::Failed(
                "the session cannot take the camera".to_owned(),
            ));
        }
        session.addInput(&input);
    }
    Ok(())
}

/// Sets `device` to the device format `format` was listed from, while this process holds its
/// lock (`camera_sharing`), inside the session's configuration, as Apple's documentation of
/// `activeFormat` sets a format with its session's.
fn set_format(device: &AVCaptureDevice, format: &CameraFormat) -> Result<(), CameraError> {
    let device_format = device_format_for(device, format)
        .ok_or_else(|| CameraError::Failed("the camera no longer lists the format".to_owned()))?;
    // SAFETY: one of the device's own formats, set while the process holds its lock.
    unsafe { device.setActiveFormat(&device_format) };
    Ok(())
}

/// Adds the default microphone's input to `session`, inside its configuration. Without it a
/// recording has no sound, and the camera still runs.
fn add_microphone(session: &AVCaptureSession) {
    // SAFETY: looking up the default microphone.
    let microphone =
        audio().and_then(|audio| unsafe { AVCaptureDevice::defaultDeviceWithMediaType(audio) });
    if let Some(microphone) = microphone {
        let _ = add_input(session, &microphone);
    }
}

/// Hears the session interrupted, its interruption end and its runtime errors, which the state
/// says.
fn observe_notices(shared: &Arc<SessionShared>) -> NotificationObservation {
    // SAFETY: constants AVFoundation exports for the life of the process.
    let (interrupted, interruption_ended, failed) = unsafe {
        (
            AVCaptureSessionWasInterruptedNotification,
            AVCaptureSessionInterruptionEndedNotification,
            AVCaptureSessionRuntimeErrorNotification,
        )
    };
    let heard = Arc::downgrade(shared);
    NotificationObservation::new(
        &shared.session.0,
        &[interrupted, interruption_ended, failed],
        move |notice| {
            let Some(shared) = heard.upgrade() else {
                return;
            };
            let name = notice.name();
            if &*name == interrupted {
                shared.interrupt(interruption_of(notice));
            } else if &*name == interruption_ended {
                shared.resume();
            } else {
                shared.end(runtime_error_of(notice));
            }
        },
    )
}

/// Why an interruption notice says the session gives no frames, where it says.
fn interruption_of(notice: &NSNotification) -> Option<CameraInterruption> {
    interruption_reason_of(notice).and_then(interruption_for)
}

/// The reason an interruption notice gives, on iOS.
#[cfg(not(target_os = "macos"))]
fn interruption_reason_of(notice: &NSNotification) -> Option<AVCaptureSessionInterruptionReason> {
    use objc2_av_foundation::AVCaptureSessionInterruptionReasonKey;
    use objc2_foundation::NSNumber;
    // SAFETY: the notice's user info holds an NSNumber under AVFoundation's key.
    let reason = unsafe {
        notice
            .userInfo()
            .and_then(|info| info.objectForKey(AVCaptureSessionInterruptionReasonKey))
            .and_then(|reason| reason.downcast::<NSNumber>().ok())
    }?;
    Some(AVCaptureSessionInterruptionReason(reason.integerValue()))
}

/// None on macOS, where Apple's `AVCaptureSessionInterruptionReasonKey` is unavailable.
#[cfg(target_os = "macos")]
fn interruption_reason_of(_notice: &NSNotification) -> Option<AVCaptureSessionInterruptionReason> {
    None
}

/// The interruption Apple's `reason` is; none for a reason the app has no counterpart of, such as
/// another client having the audio device, a multi-app layout or sensitive content.
fn interruption_for(reason: AVCaptureSessionInterruptionReason) -> Option<CameraInterruption> {
    match reason {
        AVCaptureSessionInterruptionReason::VideoDeviceInUseByAnotherClient => {
            Some(CameraInterruption::InUseByAnotherApp)
        }
        AVCaptureSessionInterruptionReason::VideoDeviceNotAvailableInBackground => {
            Some(CameraInterruption::Background)
        }
        AVCaptureSessionInterruptionReason::VideoDeviceNotAvailableDueToSystemPressure => {
            Some(CameraInterruption::SystemPressure)
        }
        _ => None,
    }
}

/// The error a runtime error notice ends the session for.
fn runtime_error_of(notice: &NSNotification) -> CameraError {
    // SAFETY: the notice's user info holds an NSError under AVFoundation's key.
    let error = unsafe {
        notice
            .userInfo()
            .and_then(|info| info.objectForKey(AVCaptureSessionErrorKey))
            .and_then(|error| error.downcast::<NSError>().ok())
    };
    error.map_or_else(
        || CameraError::Failed("the camera failed".to_owned()),
        |error| camera_error_of(&error),
    )
}

/// `error` as the app knows it: AVFoundation's codes for a device another app has, one gone and
/// one the app may not use, and the system's own words for the rest.
fn camera_error_of(error: &NSError) -> CameraError {
    // SAFETY: AVFoundation's constant, which it exports for the life of the process.
    let from_avfoundation =
        unsafe { AVFoundationErrorDomain }.is_some_and(|domain| &*error.domain() == domain);
    if from_avfoundation {
        match AVError(error.code()) {
            AVError::DeviceInUseByAnotherApplication => return CameraError::InUse,
            AVError::DeviceWasDisconnected | AVError::DeviceNotConnected => {
                return CameraError::Unplugged;
            }
            AVError::ApplicationIsNotAuthorizedToUseDevice => return CameraError::NotAllowed,
            _ => {}
        }
    }
    CameraError::Failed(error.localizedDescription().to_string())
}

/// The session handed between its queue, the system's answers and the main thread.
pub(super) struct CrossThreadSession(pub(super) Retained<AVCaptureSession>);

// SAFETY: a capture session is configured and run on its own queue, and its preview layer made of
// it on the main thread, as Apple's samples do; AVFoundation guards its own state, and the binding
// cannot say so.
unsafe impl Send for CrossThreadSession {}
unsafe impl Sync for CrossThreadSession {}

struct CrossThreadOutput(Retained<AVCaptureOutput>);

/// The preview layer, whose connection the session's queue turns once it is configured.
struct CrossThreadPreviewLayer(Retained<AVCaptureVideoPreviewLayer>);

// SAFETY: only the layer's connection is read off the main thread, an AVCaptureConnection that
// AVFoundation guards; the layer itself is changed on the main thread alone.
unsafe impl Send for CrossThreadPreviewLayer {}
unsafe impl Sync for CrossThreadPreviewLayer {}

// SAFETY: an output is added to its session on the session's queue alone.
unsafe impl Send for CrossThreadOutput {}
unsafe impl Sync for CrossThreadOutput {}

#[cfg(test)]
mod tests {
    use objc2_foundation::NSString;

    use super::*;

    fn poll<F: Future + ?Sized>(future: Pin<&mut F>) -> std::task::Poll<F::Output> {
        future.poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
    }

    /// A session runs from its start, an interruption and its end come and go, and an end is for
    /// good.
    #[test]
    fn a_session_runs_is_interrupted_and_resumes_until_it_ends() {
        let shared = SessionShared::new(media_answer().0);
        assert_eq!(shared.state(), CaptureState::Running);
        shared.interrupt(Some(CameraInterruption::SystemPressure));
        assert_eq!(
            shared.state(),
            CaptureState::Interrupted(Some(CameraInterruption::SystemPressure))
        );
        shared.resume();
        assert_eq!(shared.state(), CaptureState::Running);
        shared.end(CameraError::Unplugged);
        shared.resume();
        shared.interrupt(None);
        assert_eq!(shared.state(), CaptureState::Ended(CameraError::Unplugged));
    }

    /// The start answers at the first of a run, an interruption or an end, and only then.
    #[test]
    fn the_start_answers_once_the_session_is_interrupted_or_ends() {
        let (started, mut answered) = media_answer();
        let shared = SessionShared::new(started);
        assert!(poll(answered.as_mut()).is_pending());
        shared.interrupt(None);
        shared.end(CameraError::Unplugged);
        assert_eq!(
            poll(answered.as_mut()),
            std::task::Poll::Ready(Ok(Ok(()))),
            "interrupted, it started, and resumes by itself"
        );

        let (started, mut answered) = media_answer();
        let shared = SessionShared::new(started);
        shared.end(CameraError::InUse);
        assert_eq!(
            poll(answered.as_mut()),
            std::task::Poll::Ready(Ok(Err(CameraError::InUse)))
        );
    }

    #[test]
    fn apples_interruption_reasons_are_the_apps() {
        assert_eq!(
            interruption_for(
                AVCaptureSessionInterruptionReason::VideoDeviceNotAvailableInBackground
            ),
            Some(CameraInterruption::Background)
        );
        assert_eq!(
            interruption_for(
                AVCaptureSessionInterruptionReason::VideoDeviceNotAvailableDueToSystemPressure
            ),
            Some(CameraInterruption::SystemPressure)
        );
        assert_eq!(
            interruption_for(AVCaptureSessionInterruptionReason::VideoDeviceInUseByAnotherClient),
            Some(CameraInterruption::InUseByAnotherApp)
        );
        assert_eq!(
            interruption_for(AVCaptureSessionInterruptionReason::AudioDeviceInUseByAnotherClient),
            None,
            "a reason the app has no counterpart of"
        );
    }

    /// macOS's interruption notice gives no reason, and the interruption has none.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_macos_interruption_has_no_reason() {
        // SAFETY: AVFoundation's constant, which it exports for the life of the process, and a
        // notice of it about nothing.
        let notice = unsafe {
            NSNotification::notificationWithName_object(
                AVCaptureSessionWasInterruptedNotification,
                None,
            )
        };
        assert_eq!(interruption_of(&notice), None);
    }

    #[test]
    fn avfoundations_errors_are_the_apps_and_others_keep_the_systems_words() {
        // SAFETY: AVFoundation's constant, which it exports for the life of the process.
        let domain = unsafe { AVFoundationErrorDomain }.expect("AVFoundation's error domain");
        let error = |code: AVError| NSError::new(code.0, domain);
        assert_eq!(
            camera_error_of(&error(AVError::DeviceInUseByAnotherApplication)),
            CameraError::InUse
        );
        assert_eq!(
            camera_error_of(&error(AVError::DeviceWasDisconnected)),
            CameraError::Unplugged
        );
        assert_eq!(
            camera_error_of(&error(AVError::ApplicationIsNotAuthorizedToUseDevice)),
            CameraError::NotAllowed
        );
        let other = NSError::new(-11808, &NSString::from_str("Elsewhere"));
        assert!(matches!(camera_error_of(&other), CameraError::Failed(_)));
    }
}
