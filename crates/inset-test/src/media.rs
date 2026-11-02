//! A compositor, frame readers, players and cameras in memory, as a host shows nothing: each
//! keeps what the app asked of it, and reports the state the test gives it, a player its
//! metadata and cues too.

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
    Camera, CameraState, Cameras, CaptureSession, CompositionLayer, Compositor, Cue,
    HardwareBuffer, MediaChangeFuture, MediaError, MediaMetadata, MediaPlayer, MovieOptions,
    MovieOutput, Photo, PhotoFuture, PhotoOutput, Placement, PlayerState, Players, RecordingFuture,
    VideoFrame, VideoFrameReader, VideoFrameReaders, VideoOutput,
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

/// Cameras that list what the test gave, and keep each session opened, which reports the state
/// the test gives it. Every clone is the same cameras.
#[derive(Clone, Default)]
pub struct MockCameras(Rc<CamerasShared>);

#[derive(Default)]
struct CamerasShared {
    cameras: Vec<Camera>,
    sessions: RefCell<Vec<MockCaptureSession>>,
}

impl MockCameras {
    /// Cameras listing `cameras`.
    pub fn new(cameras: Vec<Camera>) -> MockCameras {
        MockCameras(Rc::new(CamerasShared {
            cameras,
            sessions: RefCell::default(),
        }))
    }

    /// The sessions opened, in order.
    pub fn sessions(&self) -> Vec<MockCaptureSession> {
        self.0.sessions.borrow().clone()
    }
}

impl Cameras for MockCameras {
    fn cameras(&self) -> Vec<Camera> {
        self.0.cameras.clone()
    }

    fn open_session(&self, camera: &Camera) -> Box<dyn CaptureSession> {
        let session = MockCaptureSession(Rc::new(SessionShared {
            camera: camera.clone(),
            state: RefCell::new(CameraState::stopped()),
            calls: RefCell::default(),
            preview_layer: RefCell::default(),
            frame_reader: RefCell::default(),
            photo: RefCell::default(),
            recordings: RefCell::default(),
            changes: Changes::default(),
            dropped: Cell::new(false),
        }));
        self.0.sessions.borrow_mut().push(session.clone());
        Box::new(OpenedSession(session))
    }
}

/// What a camera session was asked to do.
#[derive(Clone, Debug, PartialEq)]
pub enum SessionCall {
    Start,
    Stop,
    /// Whether a layer was given, or none.
    SetPreviewLayer(bool),
    /// Whether a reader was given, or none.
    SetFrameReader(bool),
    PhotoOutput,
    MovieOutput(MovieOptions),
}

/// A camera session the app opened, as the test sees it. Its photos answer what the test gave,
/// or that the session does not run; its recordings answer when they are stopped, writing
/// nothing. Every clone is the same session.
#[derive(Clone)]
pub struct MockCaptureSession(Rc<SessionShared>);

struct SessionShared {
    camera: Camera,
    state: RefCell<CameraState>,
    calls: RefCell<Vec<SessionCall>>,
    preview_layer: RefCell<Option<Arc<dyn CompositionLayer>>>,
    frame_reader: RefCell<Option<Arc<dyn VideoFrameReader>>>,
    photo: RefCell<Option<Photo>>,
    /// The paths recorded to, the last one first to answer when stopped.
    recordings: RefCell<Vec<PathBuf>>,
    changes: Changes,
    dropped: Cell<bool>,
}

impl MockCaptureSession {
    pub fn camera(&self) -> Camera {
        self.0.camera.clone()
    }

    /// What the session was asked, in order.
    pub fn calls(&self) -> Vec<SessionCall> {
        self.0.calls.borrow().clone()
    }

    /// The layer the session's preview is on; `None` once it is put nowhere.
    pub fn preview_layer(&self) -> Option<Arc<dyn CompositionLayer>> {
        self.0.preview_layer.borrow().clone()
    }

    /// The reader the session's frames go to; `None` once they go to none.
    pub fn frame_reader(&self) -> Option<Arc<dyn VideoFrameReader>> {
        self.0.frame_reader.borrow().clone()
    }

    /// The paths the app recorded to, in order.
    pub fn recordings(&self) -> Vec<PathBuf> {
        self.0.recordings.borrow().clone()
    }

    pub fn is_dropped(&self) -> bool {
        self.0.dropped.get()
    }

    /// Answers each capture with `photo` from now on, while the session runs.
    pub fn answering_photo(&self, photo: Photo) {
        *self.0.photo.borrow_mut() = Some(photo);
    }

    /// Reports `state` from now on, as the system would have changed it.
    pub fn set_state(&self, state: CameraState) {
        *self.0.state.borrow_mut() = state;
        self.0.changes.notify();
    }

    fn call(&self, call: SessionCall) {
        self.0.calls.borrow_mut().push(call);
    }
}

/// The session the app holds; dropping it ends the session's changes.
struct OpenedSession(MockCaptureSession);

impl CaptureSession for OpenedSession {
    fn start(&self) {
        self.0.call(SessionCall::Start);
    }

    fn stop(&self) {
        self.0.call(SessionCall::Stop);
    }

    fn state(&self) -> CameraState {
        self.0.0.state.borrow().clone()
    }

    fn changed(&self) -> MediaChangeFuture {
        self.0.0.changes.next()
    }

    fn set_preview_layer(&self, layer: Option<Arc<dyn CompositionLayer>>) {
        self.0.call(SessionCall::SetPreviewLayer(layer.is_some()));
        *self.0.0.preview_layer.borrow_mut() = layer;
    }

    fn set_frame_reader(&self, reader: Option<Arc<dyn VideoFrameReader>>) {
        self.0.call(SessionCall::SetFrameReader(reader.is_some()));
        *self.0.0.frame_reader.borrow_mut() = reader;
    }

    fn photo_output(&self) -> Box<dyn PhotoOutput> {
        self.0.call(SessionCall::PhotoOutput);
        Box::new(MockPhotos(self.0.clone()))
    }

    fn movie_output(&self, options: MovieOptions) -> Box<dyn MovieOutput> {
        self.0.call(SessionCall::MovieOutput(options));
        Box::new(MockMovies {
            session: self.0.clone(),
            recording: RefCell::default(),
        })
    }
}

impl Drop for OpenedSession {
    fn drop(&mut self) {
        self.0.0.dropped.set(true);
        self.0.0.changes.end();
    }
}

/// A session's photos: what the test gave, while the session runs.
struct MockPhotos(MockCaptureSession);

impl PhotoOutput for MockPhotos {
    fn capture(&self) -> PhotoFuture {
        let session = &self.0.0;
        let answer = if session.state.borrow().status != inset_embedder::CameraStatus::Running {
            Err(MediaError::NotRunning)
        } else {
            session
                .photo
                .borrow()
                .clone()
                .ok_or_else(|| MediaError::Failed("the test gave no photo".to_owned()))
        };
        Box::pin(std::future::ready(answer))
    }
}

/// A session's recordings: each answers when stopped.
struct MockMovies {
    session: MockCaptureSession,
    recording: RefCell<Option<Rc<Recording>>>,
}

/// A recording under way, which answers once stopped.
#[derive(Default)]
struct Recording {
    stopped: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

impl MovieOutput for MockMovies {
    fn start(&self, path: PathBuf) -> RecordingFuture {
        if self.session.0.state.borrow().status != inset_embedder::CameraStatus::Running {
            return Box::pin(std::future::ready(Err(MediaError::NotRunning)));
        }
        self.session.0.recordings.borrow_mut().push(path);
        let recording = Rc::new(Recording::default());
        *self.recording.borrow_mut() = Some(Rc::clone(&recording));
        Box::pin(RecordingEnd(recording))
    }

    fn stop(&self) {
        if let Some(recording) = self.recording.borrow_mut().take() {
            recording.stopped.set(true);
            if let Some(waker) = recording.waker.borrow_mut().take() {
                waker.wake();
            }
        }
    }
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
