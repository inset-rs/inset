//! A compositor, frame readers, players and cameras in memory, as a host shows nothing: each
//! keeps what the app asked of it or gave it, and reports the state the test gives it, a player
//! its metadata and cues too.

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use inset_embedder::{
    Camera, CameraDevice, CameraError, CameraFormat, CameraOpenFuture, CameraOutputs, Cameras,
    CaptureSession, CaptureState, CompositionLayer, Compositor, Cue, HardwareBuffer,
    MediaChangeFuture, MediaError, MediaMetadata, MediaPlayer, MovieOptions, Photo, PhotoFuture,
    Placement, PlayerState, Players, RecordingFuture, Size, VideoFrame, VideoFrameReader,
    VideoFrameReaders, VideoOutput,
};

/// A compositor whose layers show nowhere, and which keeps each layer it made. Every clone is the
/// same compositor.
#[derive(Clone, Default)]
pub struct MockCompositor(Arc<Mutex<Vec<MadeLayer>>>);

/// A layer the app made.
#[derive(Clone)]
pub struct MadeLayer {
    pub opaque: bool,
    pub layer: Arc<dyn CompositionLayer>,
}

impl MockCompositor {
    pub fn new() -> MockCompositor {
        MockCompositor::default()
    }

    /// The layers made, in order.
    pub fn made(&self) -> Vec<MadeLayer> {
        self.0.lock().expect("never held across a panic").clone()
    }
}

impl Compositor for MockCompositor {
    fn make_layer(&self, opaque: bool) -> Arc<dyn CompositionLayer> {
        let layer: Arc<dyn CompositionLayer> = Arc::new(MockLayer);
        self.0
            .lock()
            .expect("never held across a panic")
            .push(MadeLayer {
                opaque,
                layer: Arc::clone(&layer),
            });
        layer
    }
}

/// A layer that shows nothing: presents release their buffers at once.
struct MockLayer;

impl CompositionLayer for MockLayer {
    fn present(
        &self,
        _buffer: HardwareBuffer,
        _children: Vec<(Arc<dyn CompositionLayer>, Placement)>,
        released: Box<dyn FnOnce() + Send + Sync>,
    ) {
        released();
    }
}

/// Frame readers that never have a frame, and which keep each reader made. Every clone is the
/// same readers.
#[derive(Clone, Default)]
pub struct MockVideoFrameReaders(Rc<RefCell<Vec<Arc<dyn VideoFrameReader>>>>);

impl MockVideoFrameReaders {
    pub fn new() -> MockVideoFrameReaders {
        MockVideoFrameReaders::default()
    }

    /// The readers made, in order.
    pub fn made(&self) -> Vec<Arc<dyn VideoFrameReader>> {
        self.0.borrow().clone()
    }
}

impl VideoFrameReaders for MockVideoFrameReaders {
    fn make_reader(&self) -> Arc<dyn VideoFrameReader> {
        let reader: Arc<dyn VideoFrameReader> = Arc::new(NoFrames);
        self.0.borrow_mut().push(Arc::clone(&reader));
        reader
    }
}

/// Players that keep each player opened, which reports the state, metadata and cues the test
/// gives it. Every clone is the same players.
#[derive(Clone, Default)]
pub struct MockPlayers(Rc<RefCell<Vec<MockPlayer>>>);

impl MockPlayers {
    pub fn new() -> MockPlayers {
        MockPlayers::default()
    }

    /// The players opened, in order.
    pub fn opened(&self) -> Vec<MockPlayer> {
        self.0.borrow().clone()
    }
}

impl Players for MockPlayers {
    fn open(&self, url: &str) -> Box<dyn MediaPlayer> {
        let player = MockPlayer(Rc::new(PlayerShared {
            url: url.to_owned(),
            state: RefCell::new(PlayerState::loading()),
            metadata: RefCell::default(),
            cues: RefCell::default(),
            calls: RefCell::default(),
            video_output: RefCell::default(),
            changes: Changes::default(),
            dropped: Cell::new(false),
        }));
        self.0.borrow_mut().push(player.clone());
        Box::new(OpenedPlayer(player))
    }
}

/// What a player was asked to do.
#[derive(Clone, Debug, PartialEq)]
pub enum PlayerCall {
    Play,
    Pause,
    Seek(Duration),
    SetRate(f64),
    SetVolume(f64),
    SetLooping(bool),
    /// What kind of output was given, or none.
    SetVideoOutput(Option<VideoOutputKind>),
    SelectAudioTrack(String),
    SelectTextTrack(Option<String>),
}

/// The kind of a [`VideoOutput`], which a [`PlayerCall`] records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoOutputKind {
    Layer,
    Frames,
}

impl VideoOutputKind {
    fn of(output: &VideoOutput) -> VideoOutputKind {
        match output {
            VideoOutput::Layer(_) => VideoOutputKind::Layer,
            VideoOutput::Frames(_) => VideoOutputKind::Frames,
        }
    }
}

/// A player the app opened, as the test sees it. Every clone is the same player.
#[derive(Clone)]
pub struct MockPlayer(Rc<PlayerShared>);

struct PlayerShared {
    url: String,
    state: RefCell<PlayerState>,
    metadata: RefCell<MediaMetadata>,
    cues: RefCell<Vec<Cue>>,
    calls: RefCell<Vec<PlayerCall>>,
    video_output: RefCell<Option<VideoOutput>>,
    changes: Changes,
    dropped: Cell<bool>,
}

impl MockPlayer {
    /// The URL the player was opened from.
    pub fn url(&self) -> String {
        self.0.url.clone()
    }

    /// What the player was asked, in order.
    pub fn calls(&self) -> Vec<PlayerCall> {
        self.0.calls.borrow().clone()
    }

    /// The layer the player's picture is on; `None` once it is sent nowhere, or to a frame
    /// reader.
    pub fn video_layer(&self) -> Option<Arc<dyn CompositionLayer>> {
        match &*self.0.video_output.borrow() {
            Some(VideoOutput::Layer(layer)) => Some(Arc::clone(layer)),
            _ => None,
        }
    }

    /// The reader the player's frames go to; `None` once its picture is sent nowhere, or to a
    /// layer.
    pub fn frame_reader(&self) -> Option<Arc<dyn VideoFrameReader>> {
        match &*self.0.video_output.borrow() {
            Some(VideoOutput::Frames(reader)) => Some(Arc::clone(reader)),
            _ => None,
        }
    }

    /// Whether the app let go of the player.
    pub fn is_dropped(&self) -> bool {
        self.0.dropped.get()
    }

    /// Reports `state` from now on, as the system would have changed it.
    pub fn set_state(&self, state: PlayerState) {
        *self.0.state.borrow_mut() = state;
        self.0.changes.notify();
    }

    /// Reports `metadata` from now on, as the system would have read it from the media.
    pub fn set_metadata(&self, metadata: MediaMetadata) {
        *self.0.metadata.borrow_mut() = metadata;
        self.0.changes.notify();
    }

    /// Reports `cues` from now on, as the system would have reached them in the text track.
    pub fn set_cues(&self, cues: Vec<Cue>) {
        *self.0.cues.borrow_mut() = cues;
        self.0.changes.notify();
    }

    fn call(&self, call: PlayerCall) {
        self.0.calls.borrow_mut().push(call);
    }
}

/// The player the app holds; dropping it ends the player's changes.
struct OpenedPlayer(MockPlayer);

impl MediaPlayer for OpenedPlayer {
    fn play(&self) {
        self.0.call(PlayerCall::Play);
    }

    fn pause(&self) {
        self.0.call(PlayerCall::Pause);
    }

    fn seek(&self, position: Duration) {
        self.0.call(PlayerCall::Seek(position));
    }

    fn set_rate(&self, rate: f64) {
        self.0.call(PlayerCall::SetRate(rate));
    }

    fn set_volume(&self, volume: f64) {
        self.0.call(PlayerCall::SetVolume(volume));
    }

    fn set_looping(&self, looping: bool) {
        self.0.call(PlayerCall::SetLooping(looping));
    }

    fn state(&self) -> PlayerState {
        self.0.0.state.borrow().clone()
    }

    fn changed(&self) -> MediaChangeFuture {
        self.0.0.changes.next()
    }

    fn set_video_output(&self, output: Option<VideoOutput>) {
        self.0.call(PlayerCall::SetVideoOutput(
            output.as_ref().map(VideoOutputKind::of),
        ));
        *self.0.0.video_output.borrow_mut() = output;
    }

    fn select_audio_track(&self, id: &str) {
        self.0.call(PlayerCall::SelectAudioTrack(id.to_owned()));
    }

    fn select_text_track(&self, id: Option<&str>) {
        self.0
            .call(PlayerCall::SelectTextTrack(id.map(str::to_owned)));
    }

    fn metadata(&self) -> MediaMetadata {
        self.0.0.metadata.borrow().clone()
    }

    fn cues(&self) -> Vec<Cue> {
        self.0.0.cues.borrow().clone()
    }
}

impl Drop for OpenedPlayer {
    fn drop(&mut self) {
        self.0.0.dropped.set(true);
        self.0.0.changes.end();
    }
}

/// Cameras that list what the test gave, each running in the formats the test gave, and keep each
/// session started, which reports the state the test gives it. A session started on a camera
/// another session still runs joins that one's format, as a system that runs a camera in one
/// format at a time does. Every clone is the same cameras.
#[derive(Clone, Default)]
pub struct MockCameras(Rc<CamerasShared>);

#[derive(Default)]
struct CamerasShared {
    cameras: Vec<Camera>,
    formats: RefCell<Vec<CameraFormat>>,
    /// Whether the user does not let the app use the cameras.
    denied: Cell<bool>,
    sessions: RefCell<Vec<MockCaptureSession>>,
}

impl MockCameras {
    /// Cameras listing `cameras`, which run in no format until [`with_formats`](Self::with_formats).
    pub fn new(cameras: Vec<Camera>) -> MockCameras {
        MockCameras(Rc::new(CamerasShared {
            cameras,
            ..CamerasShared::default()
        }))
    }

    /// Each camera runs in `formats`.
    pub fn with_formats(self, formats: Vec<CameraFormat>) -> MockCameras {
        *self.0.formats.borrow_mut() = formats;
        self
    }

    /// The user does not let the app use the cameras: opening one fails.
    pub fn denying(self) -> MockCameras {
        self.0.denied.set(true);
        self
    }

    /// The sessions started, in order.
    pub fn sessions(&self) -> Vec<MockCaptureSession> {
        self.0.sessions.borrow().clone()
    }

    /// The format a session still running `camera` runs it in.
    fn running_format(&self, camera: &Camera) -> Option<CameraFormat> {
        self.0
            .sessions
            .borrow()
            .iter()
            .find(|session| session.camera() == *camera && !session.is_dropped())
            .map(MockCaptureSession::format)
    }
}

impl Cameras for MockCameras {
    fn cameras(&self) -> Vec<Camera> {
        self.0.cameras.clone()
    }

    fn open(&self, camera: &Camera) -> CameraOpenFuture {
        let opened: Result<Box<dyn CameraDevice>, CameraError> = if self.0.denied.get() {
            Err(CameraError::NotAllowed)
        } else {
            Ok(Box::new(OpenedCamera {
                cameras: self.clone(),
                camera: camera.clone(),
            }))
        };
        Box::pin(std::future::ready(opened))
    }
}

/// A camera the app opened.
struct OpenedCamera {
    cameras: MockCameras,
    camera: Camera,
}

impl CameraDevice for OpenedCamera {
    fn formats(&self) -> Vec<CameraFormat> {
        self.cameras.0.formats.borrow().clone()
    }

    fn start(
        &self,
        format: &CameraFormat,
        outputs: CameraOutputs,
    ) -> Result<Box<dyn CaptureSession>, CameraError> {
        if !self.cameras.0.formats.borrow().contains(format) {
            return Err(CameraError::Failed(
                "the camera does not list this format".to_owned(),
            ));
        }
        let running = self.cameras.running_format(&self.camera);
        let session = MockCaptureSession(Rc::new(SessionShared {
            camera: self.camera.clone(),
            picked: format.clone(),
            format: running.unwrap_or_else(|| format.clone()),
            outputs,
            state: RefCell::new(CaptureState::Running),
            photo: RefCell::default(),
            recordings: RefCell::default(),
            recording: RefCell::default(),
            changes: Changes::default(),
            dropped: Cell::new(false),
        }));
        self.cameras.0.sessions.borrow_mut().push(session.clone());
        Ok(Box::new(StartedSession(session)))
    }
}

/// A camera session the app started, as the test sees it. Its photos answer what the test gave,
/// or that the session does not run; its recordings answer when they are stopped, writing
/// nothing. Every clone is the same session.
#[derive(Clone)]
pub struct MockCaptureSession(Rc<SessionShared>);

struct SessionShared {
    camera: Camera,
    picked: CameraFormat,
    format: CameraFormat,
    outputs: CameraOutputs,
    state: RefCell<CaptureState>,
    photo: RefCell<Option<Photo>>,
    /// The paths recorded to, in order.
    recordings: RefCell<Vec<PathBuf>>,
    /// The recording under way, which answers when stopped.
    recording: RefCell<Option<Rc<Recording>>>,
    changes: Changes,
    dropped: Cell<bool>,
}

impl MockCaptureSession {
    pub fn camera(&self) -> Camera {
        self.0.camera.clone()
    }

    /// The format the app picked.
    pub fn picked(&self) -> CameraFormat {
        self.0.picked.clone()
    }

    /// The format it runs in: the one picked, or the one of the session it joined.
    pub fn format(&self) -> CameraFormat {
        self.0.format.clone()
    }

    /// The layer the preview shows on, among the outputs given.
    pub fn preview_layer(&self) -> Option<Arc<dyn CompositionLayer>> {
        self.0.outputs.preview.clone()
    }

    /// The reader the frames go to, among the outputs given.
    pub fn frame_reader(&self) -> Option<Arc<dyn VideoFrameReader>> {
        self.0.outputs.frames.clone()
    }

    /// The size photos are taken at, among the outputs given.
    pub fn photo_size(&self) -> Option<Size> {
        self.0.outputs.photos
    }

    /// What recordings ask for, among the outputs given.
    pub fn movie_options(&self) -> Option<MovieOptions> {
        self.0.outputs.movies
    }

    /// The paths the app recorded to, in order.
    pub fn recordings(&self) -> Vec<PathBuf> {
        self.0.recordings.borrow().clone()
    }

    pub fn is_dropped(&self) -> bool {
        self.0.dropped.get()
    }

    /// Answers each photo with `photo` from now on, while the session runs.
    pub fn answering_photo(&self, photo: Photo) {
        *self.0.photo.borrow_mut() = Some(photo);
    }

    /// Reports `state` from now on, as the system would have changed it.
    pub fn set_state(&self, state: CaptureState) {
        *self.0.state.borrow_mut() = state;
        self.0.changes.notify();
    }

    fn runs(&self) -> bool {
        *self.0.state.borrow() == CaptureState::Running
    }
}

/// The session the app holds; dropping it ends the session's changes.
struct StartedSession(MockCaptureSession);

impl CaptureSession for StartedSession {
    fn format(&self) -> CameraFormat {
        self.0.format()
    }

    fn state(&self) -> CaptureState {
        self.0.0.state.borrow().clone()
    }

    fn changed(&self) -> MediaChangeFuture {
        self.0.0.changes.next()
    }

    fn take_photo(&self) -> PhotoFuture {
        let session = &self.0;
        let answer = if session.photo_size().is_none() {
            Err(MediaError::Failed(
                "photos were not among the outputs".to_owned(),
            ))
        } else if !session.runs() {
            Err(MediaError::NotRunning)
        } else {
            session
                .0
                .photo
                .borrow()
                .clone()
                .ok_or_else(|| MediaError::Failed("the test gave no photo".to_owned()))
        };
        Box::pin(std::future::ready(answer))
    }

    fn start_recording(&self, path: PathBuf) -> RecordingFuture {
        let session = &self.0;
        if session.movie_options().is_none() {
            let reason = "movies were not among the outputs".to_owned();
            return Box::pin(std::future::ready(Err(MediaError::Failed(reason))));
        }
        if !session.runs() {
            return Box::pin(std::future::ready(Err(MediaError::NotRunning)));
        }
        session.0.recordings.borrow_mut().push(path);
        let recording = Rc::new(Recording::default());
        *session.0.recording.borrow_mut() = Some(Rc::clone(&recording));
        Box::pin(RecordingEnd(recording))
    }

    fn stop_recording(&self) {
        if let Some(recording) = self.0.0.recording.borrow_mut().take() {
            recording.stopped.set(true);
            if let Some(waker) = recording.waker.borrow_mut().take() {
                waker.wake();
            }
        }
    }
}

impl Drop for StartedSession {
    fn drop(&mut self) {
        self.0.0.dropped.set(true);
        self.0.0.changes.end();
    }
}

/// A recording under way, which answers once stopped.
#[derive(Default)]
struct Recording {
    stopped: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

struct RecordingEnd(Rc<Recording>);

impl Future for RecordingEnd {
    type Output = Result<(), MediaError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.0.stopped.get() {
            return Poll::Ready(Ok(()));
        }
        *self.0.waker.borrow_mut() = Some(cx.waker().clone());
        Poll::Pending
    }
}

/// A frame reader that never has a picture.
struct NoFrames;

impl VideoFrameReader for NoFrames {
    fn frame_for(&self, _display_time: std::time::Instant) -> Option<VideoFrame> {
        None
    }
}

/// Each `next` future resolves at the next change, or once the changes end.
#[derive(Default)]
struct Changes(Rc<ChangesShared>);

#[derive(Default)]
struct ChangesShared {
    count: Cell<u64>,
    ended: Cell<bool>,
    wakers: RefCell<Vec<Waker>>,
}

impl Changes {
    fn next(&self) -> MediaChangeFuture {
        Box::pin(NextChange {
            changes: Rc::clone(&self.0),
            seen: self.0.count.get(),
        })
    }

    fn notify(&self) {
        self.0.count.set(self.0.count.get() + 1);
        self.wake();
    }

    fn end(&self) {
        self.0.ended.set(true);
        self.wake();
    }

    fn wake(&self) {
        for waker in self.0.wakers.take() {
            waker.wake();
        }
    }
}

struct NextChange {
    changes: Rc<ChangesShared>,
    seen: u64,
}

impl Future for NextChange {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.changes.ended.get() || self.changes.count.get() != self.seen {
            return Poll::Ready(());
        }
        self.changes.wakers.borrow_mut().push(cx.waker().clone());
        Poll::Pending
    }
}
