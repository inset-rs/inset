//! A compositor, frame readers, players, media assets and cameras in memory, as a host shows
//! nothing: each keeps what the app asked of it or gave it, and reports the state the test gives
//! it, a player its position, metadata and cues too, an asset what it reads, and the cameras their
//! list.

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use waft_embedder::image_codecs::{
    ImageError, ImageErrorKind, ImageFuture, PixelBuffer, PixelFormat,
};
use waft_embedder::{
    Camera, CameraDevice, CameraError, CameraFormat, CameraListFuture, CameraOpenFuture,
    CameraOutputs, CameraStartFuture, Cameras, CaptureSession, CaptureState, CompositionLayer,
    Compositor, Cue, HardwareBuffer, HardwareBufferUsage, MediaAsset, MediaAssetFuture,
    MediaAssets, MediaChangeFuture, MediaError, MediaMetadata, MediaPlayer, MediaTrack,
    MovieOptions, Photo, PhotoFuture, Placement, PlayerError, PlayerState, Players,
    RecordingFuture, SeekTolerance, Size, VideoFrame, VideoFrameReader, VideoFrameReaders,
    VideoOutput,
};

use crate::MockHardwareBuffer;
use crate::image_codecs::pixels_of;

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
        _buffer: Arc<dyn HardwareBuffer>,
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

/// Players that keep each player opened, which reports the state, position, metadata and cues
/// the test gives it. Every clone is the same players.
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
            position: Cell::new(Duration::ZERO),
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
    Seek(Duration, SeekTolerance),
    SetRate(f64),
    SetVolume(f64),
    SetMuted(bool),
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
    position: Cell<Duration>,
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

    /// Answers `position` from now on, telling the app nothing, as a playing player moves on.
    pub fn move_to(&self, position: Duration) {
        self.0.position.set(position);
    }

    /// Answers `position` from now on, telling the app it jumped, as a seek landing or a loop
    /// does.
    pub fn jump_to(&self, position: Duration) {
        self.0.position.set(position);
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

    fn seek(&self, position: Duration, tolerance: SeekTolerance) {
        self.0.call(PlayerCall::Seek(position, tolerance));
    }

    fn set_rate(&self, rate: f64) {
        self.0.call(PlayerCall::SetRate(rate));
    }

    fn set_volume(&self, volume: f64) {
        self.0.call(PlayerCall::SetVolume(volume));
    }

    fn set_muted(&self, muted: bool) {
        self.0.call(PlayerCall::SetMuted(muted));
    }

    fn set_looping(&self, looping: bool) {
        self.0.call(PlayerCall::SetLooping(looping));
    }

    fn state(&self) -> PlayerState {
        self.0.0.state.borrow().clone()
    }

    fn position(&self) -> Duration {
        self.0.0.position.get()
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

/// Media assets that each answer what the test set, at once, and keep each asset opened with the
/// pictures asked of it. Every clone is the same assets.
#[derive(Clone)]
pub struct MockMediaAssets(Rc<AssetsShared>);

struct AssetsShared {
    answers: RefCell<AssetAnswers>,
    opened: RefCell<Vec<MockMediaAsset>>,
}

/// What every asset answers: its length, `None` for a live stream, its tracks, its tags, and the
/// size of its pictures, each or a failure.
#[derive(Clone, Debug, PartialEq)]
pub struct AssetAnswers {
    pub duration: Result<Option<Duration>, PlayerError>,
    pub tracks: Result<Vec<MediaTrack>, PlayerError>,
    pub metadata: Result<MediaMetadata, PlayerError>,
    /// The size of each picture, which answers a [`MockVideoFrame`] of it.
    pub frame_size: Result<Size, PlayerError>,
}

impl MockMediaAssets {
    /// Assets that each answer `answers`.
    pub fn new(answers: AssetAnswers) -> MockMediaAssets {
        MockMediaAssets(Rc::new(AssetsShared {
            answers: RefCell::new(answers),
            opened: RefCell::default(),
        }))
    }

    /// Each asset answers `answers` from now on.
    pub fn set_answers(&self, answers: AssetAnswers) {
        *self.0.answers.borrow_mut() = answers;
    }

    /// The assets opened, in order.
    pub fn opened(&self) -> Vec<MockMediaAsset> {
        self.0.opened.borrow().clone()
    }
}

impl MediaAssets for MockMediaAssets {
    fn open(&self, url: &str) -> Box<dyn MediaAsset> {
        let asset = MockMediaAsset(Rc::new(AssetShared {
            url: url.to_owned(),
            frames_asked: RefCell::default(),
            dropped: Cell::new(false),
        }));
        self.0.opened.borrow_mut().push(asset.clone());
        Box::new(OpenedAsset {
            assets: self.clone(),
            asset,
        })
    }
}

/// An asset the app opened, as the test sees it. Every clone is the same asset.
#[derive(Clone)]
pub struct MockMediaAsset(Rc<AssetShared>);

struct AssetShared {
    url: String,
    frames_asked: RefCell<Vec<FrameAsked>>,
    dropped: Cell<bool>,
}

/// A picture the app asked an asset for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameAsked {
    pub time: Duration,
    pub tolerance: SeekTolerance,
    pub max_size: Option<Size>,
}

impl MockMediaAsset {
    /// The URL the asset was opened from.
    pub fn url(&self) -> String {
        self.0.url.clone()
    }

    /// The pictures asked of it, in order.
    pub fn frames_asked(&self) -> Vec<FrameAsked> {
        self.0.frames_asked.borrow().clone()
    }

    /// Whether the app let go of the asset.
    pub fn is_dropped(&self) -> bool {
        self.0.dropped.get()
    }
}

/// The asset the app holds.
struct OpenedAsset {
    assets: MockMediaAssets,
    asset: MockMediaAsset,
}

impl OpenedAsset {
    fn answers(&self) -> AssetAnswers {
        self.assets.0.answers.borrow().clone()
    }
}

impl MediaAsset for OpenedAsset {
    fn duration(&self) -> MediaAssetFuture<Option<Duration>> {
        Box::pin(std::future::ready(self.answers().duration))
    }

    fn tracks(&self) -> MediaAssetFuture<Vec<MediaTrack>> {
        Box::pin(std::future::ready(self.answers().tracks))
    }

    fn metadata(&self) -> MediaAssetFuture<MediaMetadata> {
        Box::pin(std::future::ready(self.answers().metadata))
    }

    fn frame_at(
        &self,
        time: Duration,
        tolerance: SeekTolerance,
        max_size: Option<Size>,
    ) -> MediaAssetFuture<Arc<dyn VideoFrame>> {
        self.asset.0.frames_asked.borrow_mut().push(FrameAsked {
            time,
            tolerance,
            max_size,
        });
        let frame = self.answers().frame_size.map(|size| {
            Arc::new(MockVideoFrame {
                width: size.width() as u32,
                height: size.height() as u32,
            }) as Arc<dyn VideoFrame>
        });
        Box::pin(std::future::ready(frame))
    }
}

impl Drop for OpenedAsset {
    fn drop(&mut self) {
        self.asset.0.dropped.set(true);
    }
}

/// A picture of a size, with no pixels on any GPU: its buffer is imported as no texture, as a
/// frame a device cannot take, which a drawing skips, and its pixels read back opaque black, in
/// BGRA as Apple's frames are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MockVideoFrame {
    pub width: u32,
    pub height: u32,
}

impl VideoFrame for MockVideoFrame {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn read_pixels(&self) -> ImageFuture<PixelBuffer> {
        let read = if self.width == 0 || self.height == 0 {
            Err(ImageError::new(
                ImageErrorKind::Other,
                "a frame with no pixels",
            ))
        } else {
            Ok(pixels_of(
                [self.width, self.height],
                [0, 0, 0, 255],
                PixelFormat::Bgra8,
            ))
        };
        Box::pin(std::future::ready(read))
    }

    /// A [`MockHardwareBuffer`] of its size, for sampling.
    fn hardware_buffer(&self) -> Arc<dyn HardwareBuffer> {
        Arc::new(MockHardwareBuffer {
            width: self.width,
            height: self.height,
            usage: HardwareBufferUsage::GPU_SAMPLED,
        })
    }
}

/// Cameras that list what the test gave, telling the app each time the test changes the list,
/// each running in the formats the test gave, and keep each session started, which reports the
/// state the test gives it. A session started on a camera another session still runs joins that
/// one's format, as a system that runs a camera in one format at a time does. Every clone is the
/// same cameras.
#[derive(Clone, Default)]
pub struct MockCameras(Rc<CamerasShared>);

#[derive(Default)]
struct CamerasShared {
    cameras: RefCell<Vec<Camera>>,
    changes: Changes,
    formats: RefCell<Vec<CameraFormat>>,
    /// Whether the user does not let the app use the cameras.
    denied: Cell<bool>,
    sessions: RefCell<Vec<MockCaptureSession>>,
}

impl MockCameras {
    /// Cameras listing `cameras`, which run in no format until [`with_formats`](Self::with_formats).
    pub fn new(cameras: Vec<Camera>) -> MockCameras {
        MockCameras(Rc::new(CamerasShared {
            cameras: RefCell::new(cameras),
            ..CamerasShared::default()
        }))
    }

    /// Lists `cameras` from now on, as the system would once a camera was plugged in or out.
    pub fn set_cameras(&self, cameras: Vec<Camera>) {
        *self.0.cameras.borrow_mut() = cameras;
        self.0.changes.notify();
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
    fn cameras(&self) -> CameraListFuture {
        Box::pin(std::future::ready(self.0.cameras.borrow().clone()))
    }

    fn changed(&self) -> MediaChangeFuture {
        self.0.changes.next()
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

    /// Answers at once: the session runs as it starts.
    fn start(&self, format: &CameraFormat, outputs: CameraOutputs) -> CameraStartFuture {
        Box::pin(std::future::ready(self.started(format, outputs)))
    }
}

impl OpenedCamera {
    fn started(
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
    fn frame_for(&self, _display_time: std::time::Instant) -> Option<Arc<dyn VideoFrame>> {
        None
    }
}

/// Each `next` future resolves at the next change, or once the changes end.
#[derive(Default)]
pub(crate) struct Changes(Rc<ChangesShared>);

#[derive(Default)]
struct ChangesShared {
    count: Cell<u64>,
    ended: Cell<bool>,
    wakers: RefCell<Vec<Waker>>,
}

impl Changes {
    pub(crate) fn next(&self) -> MediaChangeFuture {
        Box::pin(NextChange {
            changes: Rc::clone(&self.0),
            seen: self.0.count.get(),
        })
    }

    pub(crate) fn notify(&self) {
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

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    use waft_embedder::{MediaTrackKind, PlayerErrorKind};

    use super::*;

    fn now<T>(future: impl Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("answers at once"),
        }
    }

    fn answers() -> AssetAnswers {
        AssetAnswers {
            duration: Ok(Some(Duration::from_secs(90))),
            tracks: Ok(vec![MediaTrack {
                id: "track:1".to_owned(),
                kind: MediaTrackKind::Video,
                language: String::new(),
                label: String::new(),
                selected: true,
            }]),
            metadata: Ok(MediaMetadata {
                title: Some("Bip Bop".to_owned()),
                ..MediaMetadata::default()
            }),
            frame_size: Ok(Size::new(320.0, 180.0)),
        }
    }

    #[test]
    fn an_asset_answers_what_the_test_set_and_keeps_the_pictures_asked() {
        let assets = MockMediaAssets::new(answers());
        let asset = assets.open("file:///data/clip.mp4");
        assert_eq!(now(asset.duration()), Ok(Some(Duration::from_secs(90))));
        assert_eq!(now(asset.tracks()).unwrap()[0].id, "track:1");
        assert_eq!(
            now(asset.metadata()).unwrap().title.as_deref(),
            Some("Bip Bop")
        );
        let asked = FrameAsked {
            time: Duration::from_secs(9),
            tolerance: SeekTolerance::ANY,
            max_size: Some(Size::new(320.0, 320.0)),
        };
        let frame = now(asset.frame_at(asked.time, asked.tolerance, asked.max_size)).unwrap();
        assert_eq!([frame.width(), frame.height()], [320, 180]);
        let opened = &assets.opened()[0];
        assert_eq!(opened.url(), "file:///data/clip.mp4");
        assert_eq!(opened.frames_asked(), [asked]);

        let refused = PlayerError {
            kind: PlayerErrorKind::Refused,
            message: "no".to_owned(),
        };
        assets.set_answers(AssetAnswers {
            duration: Err(refused.clone()),
            ..answers()
        });
        assert_eq!(now(asset.duration()), Err(refused));
        drop(asset);
        assert!(opened.is_dropped());
    }
}
