//! A compositor, players and cameras in memory, as a host shows nothing: each keeps what the app
//! asked of it, and reports the state the test gives it.

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
    Camera, CameraState, Cameras, CaptureSession, Compositor, FrameReader, MediaChangeFuture,
    MediaError, MediaPlayer, MovieOptions, MovieOutput, Photo, PhotoFuture, PhotoOutput,
    PixelBuffer, PlayerState, Players, RecordingFuture, Surface, SurfaceFrame, SurfaceParent,
    ViewId,
};

/// A compositor whose surfaces show nowhere, and which keeps each surface opened under a view's
/// root. Every clone is the same compositor.
#[derive(Clone, Default)]
pub struct MockCompositor(Arc<Mutex<Vec<OpenedSurface>>>);

/// A surface the app opened under a view's root.
#[derive(Clone)]
pub struct OpenedSurface {
    pub view: ViewId,
    pub opaque: bool,
    pub surface: Arc<dyn Surface>,
}

impl MockCompositor {
    pub fn new() -> MockCompositor {
        MockCompositor::default()
    }

    /// The surfaces opened under the views' roots, in order.
    pub fn opened(&self) -> Vec<OpenedSurface> {
        self.0.lock().expect("never held across a panic").clone()
    }
}

impl Compositor for MockCompositor {
    fn root(&self, view: ViewId) -> Arc<dyn SurfaceParent> {
        Arc::new(MockRoot {
            view,
            opened: self.clone(),
        })
    }
}

/// A view's root, which keeps each surface opened under it in its compositor.
struct MockRoot {
    view: ViewId,
    opened: MockCompositor,
}

impl SurfaceParent for MockRoot {
    fn open_child(&self, opaque: bool) -> Arc<dyn Surface> {
        let surface: Arc<dyn Surface> = Arc::new(MockSurface);
        self.opened
            .0
            .lock()
            .expect("never held across a panic")
            .push(OpenedSurface {
                view: self.view,
                opaque,
                surface: Arc::clone(&surface),
            });
        surface
    }
}

/// A surface that shows nothing: presents release their buffers at once.
struct MockSurface;

impl Surface for MockSurface {
    fn present(&self, _frame: SurfaceFrame, released: Box<dyn FnOnce() + Send + Sync>) {
        released();
    }
}

impl SurfaceParent for MockSurface {
    fn open_child(&self, _opaque: bool) -> Arc<dyn Surface> {
        Arc::new(MockSurface)
    }
}

/// Players that keep each player opened, which reports the state the test gives it. Every clone
/// is the same players.
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
            calls: RefCell::default(),
            video_surface: RefCell::default(),
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
    /// Whether a surface was given, or none.
    SetVideoSurface(bool),
    ReadFrames,
}

/// A player the app opened, as the test sees it. Every clone is the same player.
#[derive(Clone)]
pub struct MockPlayer(Rc<PlayerShared>);

struct PlayerShared {
    url: String,
    state: RefCell<PlayerState>,
    calls: RefCell<Vec<PlayerCall>>,
    video_surface: RefCell<Option<Arc<dyn Surface>>>,
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

    /// The surface the player's picture is on; `None` once it is put nowhere, or handed to a
    /// frame reader.
    pub fn video_surface(&self) -> Option<Arc<dyn Surface>> {
        self.0.video_surface.borrow().clone()
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

    fn set_video_surface(&self, surface: Option<Arc<dyn Surface>>) {
        self.0.call(PlayerCall::SetVideoSurface(surface.is_some()));
        *self.0.0.video_surface.borrow_mut() = surface;
    }

    fn read_frames(&self) -> Arc<dyn FrameReader> {
        self.0.call(PlayerCall::ReadFrames);
        *self.0.0.video_surface.borrow_mut() = None;
        Arc::new(NoFrames)
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
            preview_surface: RefCell::default(),
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
    /// Whether a surface was given, or none.
    SetPreviewSurface(bool),
    FrameOutput,
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
    preview_surface: RefCell<Option<Arc<dyn Surface>>>,
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

    /// The surface the session's preview is on; `None` once it is put nowhere.
    pub fn preview_surface(&self) -> Option<Arc<dyn Surface>> {
        self.0.preview_surface.borrow().clone()
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

    fn set_preview_surface(&self, surface: Option<Arc<dyn Surface>>) {
        self.0
            .call(SessionCall::SetPreviewSurface(surface.is_some()));
        *self.0.0.preview_surface.borrow_mut() = surface;
    }

    fn frame_output(&self) -> Arc<dyn FrameReader> {
        self.0.call(SessionCall::FrameOutput);
        Arc::new(NoFrames)
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

impl FrameReader for NoFrames {
    fn newest(&self) -> Option<PixelBuffer> {
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
