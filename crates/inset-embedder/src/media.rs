//! Players and cameras: the system's own media stack, as a host hands it to the app. Not a
//! dart:ui file: Flutter reaches media through its `video_player` and `camera` plugins, whose
//! controllers come from their method channels. These follow the systems' own model instead,
//! which they share: a player is one object made from a URL, loading by itself, which the app
//! controls and whose state it observes (`AVPlayer`, Media3's `ExoPlayer`, Windows'
//! `MediaPlayer`, the web's `<video>`); a camera is a session on one device with outputs, each for
//! one use (`AVCaptureSession` with its preview layer and its photo, video data and movie file
//! outputs, CameraX's use cases).
//!
//! Every system has two levels: a view that takes a player (`AVPlayerLayer(player:)`, Media3's
//! `PlayerView.player`), which is the framework's, and a player that puts its pictures where it
//! is given (Media3's `setVideoSurface`), which is the host's and this module's. A player's
//! picture goes to one destination at a time, which the app makes and gives it
//! ([`MediaPlayer::set_video_output`]): a [`CompositionLayer`], whose content becomes a layer the
//! system draws the picture into, which is the only way protected and HDR video show at all, with
//! no copy; or a [`VideoFrameReader`], made on its own ([`VideoFrameReaders::make_reader`]), which
//! hands the frames to the app's GPU, as an Android app makes an `ImageReader` and an Apple app an
//! `AVPlayerItemVideoOutput` and gives it to its player. A camera session shows its preview on a
//! layer ([`CaptureSession::set_preview_layer`]) and hands its frames to a reader
//! ([`CaptureSession::set_frame_reader`]) the same way. A camera's photos are encoded by the
//! system and its recordings encoded and written to a file by the system: the app encodes
//! nothing.
//!
//! The platform answers `None` where its system has no players, no cameras or no frames. A player
//! or a session reports its state through [`MediaPlayer::state`] and [`CaptureSession::state`],
//! read when the future from `changed` resolves: the invariant a host keeps is that such a future
//! resolves at every change of what the state answers, and when the player or session goes, so
//! the app never waits on one for ever.

use std::any::Any;
use std::fmt;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::{CompositionLayer, Size};

/// The system's players, or `None` from [`Platform::players`](crate::Platform::players) where it
/// has none.
pub trait Players {
    /// A player of the media at `url`, which loads by itself, its state saying how far it got: a
    /// link the system fetches (`https:`, a streaming playlist), or a picked file's
    /// [`PickedFile::url`](crate::PickedFile::url), as every system's player takes a URL. A URL
    /// the system cannot play gives a player that failed, with the system's reason. A player of
    /// bytes the app makes itself is for later.
    fn open(&self, url: &str) -> Box<dyn MediaPlayer>;
}

/// One player, controlled on the app's main thread. Dropped, it stops, and its picture goes
/// nowhere: it leaves the layer it showed on, and its reader reads nothing more.
pub trait MediaPlayer {
    /// Plays from where it is, at the rate last set: from the start once it has ended.
    fn play(&self);

    fn pause(&self);

    /// Moves to `position`, exactly, playing on from there if it was playing.
    fn seek(&self, position: Duration);

    /// How fast it plays, `1.0` the media's own speed; kept while paused, for the next play.
    fn set_rate(&self, rate: f64);

    /// How loud it plays, from `0.0`, silent, to `1.0`, the media's own volume.
    fn set_volume(&self, volume: f64);

    /// Whether it starts again from the beginning when it reaches the end, rather than ending.
    fn set_looping(&self, looping: bool);

    /// The player's state now.
    fn state(&self) -> PlayerState;

    /// Resolves once [`state`](Self::state) may answer something else than when this was
    /// called, or once the player is gone.
    fn changed(&self) -> MediaChangeFuture;

    /// Sends the picture to `output` from now on, and nowhere it went before; `None` sends it
    /// nowhere, as Media3's `setVideoSurface` does. The player holds the output while it sends
    /// the picture there, as a producer holds its layer. A layer no compositor of this host made
    /// shows nothing, and a reader no [`VideoFrameReaders`] of this host made reads nothing.
    fn set_video_output(&self, output: Option<VideoOutput>);
}

/// Where a player sends its picture, one destination at a time.
#[derive(Clone)]
pub enum VideoOutput {
    /// The system draws the picture into a layer of its own that is this layer's content.
    Layer(Arc<dyn CompositionLayer>),
    /// The app draws the frames this reader hands out.
    Frames(Arc<dyn VideoFrameReader>),
}

/// What a player is doing.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerState {
    pub status: PlayerStatus,
    /// Whether it plays, or waits for media to play on: true from `play` until `pause`, or the
    /// end of media it does not loop.
    pub playing: bool,
    /// Where it is in the media. While it plays the host updates it as the system reports time,
    /// every half second, as Flutter's `video_player` polls it, and at every jump.
    pub position: Duration,
    /// How long the media is; `None` until it is known, and for a live stream.
    pub duration: Option<Duration>,
    /// The picture's size, in its own pixels, as it is presented: its aspect corrected; `None`
    /// until it is known, and for media without a picture.
    pub size: Option<Size>,
    /// Whether it reached the end of media it does not loop, until it plays or seeks again.
    pub ended: bool,
}

impl PlayerState {
    /// A player that is loading: nothing known yet.
    pub fn loading() -> PlayerState {
        PlayerState {
            status: PlayerStatus::Loading,
            playing: false,
            position: Duration::ZERO,
            duration: None,
            size: None,
            ended: false,
        }
    }

    /// A player that failed for `reason`.
    pub fn failed(reason: impl Into<String>) -> PlayerState {
        PlayerState {
            status: PlayerStatus::Failed(reason.into()),
            ..PlayerState::loading()
        }
    }
}

/// How far a player got with its media.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlayerStatus {
    Loading,
    /// It can play.
    Ready,
    /// It cannot play, for the system's reason.
    Failed(String),
}

/// The system's cameras, or `None` from [`Platform::cameras`](crate::Platform::cameras) where it
/// has none.
pub trait Cameras {
    /// The cameras attached now, in the system's order.
    fn cameras(&self) -> Vec<Camera>;

    /// A session on `camera`, stopped. It asks the user for the camera at its first start, where
    /// the system asks.
    fn open_session(&self, camera: &Camera) -> Box<dyn CaptureSession>;
}

/// A camera, as the system lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Camera {
    pub id: CameraId,
    /// What the system calls it, which the user knows it by.
    pub name: String,
    pub facing: CameraFacing,
}

/// The system's name for a camera, kept while it stays attached.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CameraId(pub String);

/// Which way a camera looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraFacing {
    /// At the user: a phone's selfie camera, a laptop's.
    Front,
    /// Away from the user: a phone's main camera.
    Back,
    /// A camera the system cannot say of: one plugged in, or on another device.
    External,
}

/// A session on one camera, controlled on the app's main thread. Each output is the system's
/// own, made at its first asking and the same one after; the session runs every output it has.
/// Dropped, it stops, its preview leaves the layer it showed on, and its reader reads nothing
/// more.
pub trait CaptureSession {
    /// Starts the camera: asking the user for it first where the system asks and the user has
    /// not answered, then running, or failing, as the state says.
    fn start(&self);

    fn stop(&self);

    /// The session's state now.
    fn state(&self) -> CameraState;

    /// Resolves once [`state`](Self::state) may answer something else than when this was
    /// called, or once the session is gone.
    fn changed(&self) -> MediaChangeFuture;

    /// Puts the camera's preview on `layer` from now on, drawn by the system into a layer of its
    /// own that is `layer`'s content, and on no layer it was on before; `None` puts it nowhere.
    /// The session holds the layer while it shows on it. A layer no compositor of this host made
    /// shows nothing.
    fn set_preview_layer(&self, layer: Option<Arc<dyn CompositionLayer>>);

    /// Hands the camera's frames to `reader` from now on, and to no reader it handed them to
    /// before; `None` hands them to none. A reader no [`VideoFrameReaders`] of this host made
    /// reads nothing.
    fn set_frame_reader(&self, reader: Option<Arc<dyn VideoFrameReader>>);

    /// Photos, each captured, processed and encoded by the system.
    fn photo_output(&self) -> Box<dyn PhotoOutput>;

    /// Recordings, each encoded and written to a file by the system, with sound where `options`
    /// ask for it, which asks the user for the microphone at the session's next start.
    fn movie_output(&self, options: MovieOptions) -> Box<dyn MovieOutput>;
}

/// What a camera session is doing.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraState {
    pub status: CameraStatus,
    /// The size of the camera's pictures, in its own pixels; `None` until the session has the
    /// camera.
    pub size: Option<Size>,
}

impl CameraState {
    /// A session not started.
    pub fn stopped() -> CameraState {
        CameraState {
            status: CameraStatus::Stopped,
            size: None,
        }
    }
}

/// How far a camera session got.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CameraStatus {
    Stopped,
    /// Asking the user for the camera, or starting it.
    Starting,
    Running,
    /// The user did not let the app use the camera, or the system's policy does not.
    Denied,
    /// The camera could not run, for the system's reason.
    Failed(String),
}

/// A session's photos.
pub trait PhotoOutput {
    /// Captures a photo while the session runs, answering it once the system has encoded it.
    fn capture(&self) -> PhotoFuture;
}

/// A photo, as the system encoded it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Photo {
    pub bytes: Vec<u8>,
    /// The encoding's MIME type: `image/jpeg`.
    pub mime_type: String,
}

/// What a recording asks for beyond the picture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MovieOptions {
    /// Whether it records sound from the system's microphone.
    pub sound: bool,
}

/// A session's recordings.
pub trait MovieOutput {
    /// Records to a new file at `path`, as a QuickTime movie, while the session runs, until
    /// [`stop`](Self::stop); answers once the file is written whole, or why it was not.
    fn start(&self, path: PathBuf) -> RecordingFuture;

    /// Ends the recording, whose future then answers.
    fn stop(&self);
}

/// The system's frame readers, or `None` from
/// [`Platform::video_frame_readers`](crate::Platform::video_frame_readers) where it hands out no
/// frames.
pub trait VideoFrameReaders {
    /// A reader fed by nothing yet, which reads the frames of the source it is given to: a
    /// player's [`set_video_output`](MediaPlayer::set_video_output), a capture session's
    /// [`set_frame_reader`](CaptureSession::set_frame_reader). The reader goes when its last
    /// handle does: the app's, and its source's.
    fn make_reader(&self) -> Arc<dyn VideoFrameReader>;
}

/// The pictures of the one source a reader is given to, read from any thread as they are drawn.
/// It takes one source at a time, as Android's `ImageReader` takes one producer: given to a second
/// while the first still feeds it, it is refused, and the second sends its frames nowhere. A
/// reader keeps no frame for later: one not read before the next arrives is dropped.
///
/// A host gets its own type back from a reader it is given by downcasting through `Any`.
pub trait VideoFrameReader: Any + Send + Sync {
    /// The frame to show at `display_time`, the moment the picture being drawn reaches the
    /// screen: a player's frame for that moment of the video, as a display link asks
    /// AVFoundation's video output for each refresh's target time, and a camera's newest. `None`
    /// when it is the frame handed out last, or none came.
    fn frame_for(&self, display_time: Instant) -> Option<VideoFrame>;
}

/// A picture of a player or a camera, as WebCodecs' `VideoFrame`: the platform's own type, which
/// the GPU reads with no copy. On Apple platforms a `CVPixelBuffer` backed by an `IOSurface`, in
/// 32-bit BGRA.
#[cfg(target_vendor = "apple")]
pub type VideoFrame = objc2_core_foundation::CFRetained<objc2_core_video::CVPixelBuffer>;

/// A platform with no players and no cameras: no picture can exist, so a reader never answers
/// one.
#[cfg(not(target_vendor = "apple"))]
pub enum VideoFrame {}

/// Why a camera's output answered nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MediaError {
    /// The session does not run.
    NotRunning,
    /// The system failed, for its reason.
    Failed(String),
}

impl fmt::Display for MediaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MediaError::NotRunning => f.write_str("the camera session does not run"),
            MediaError::Failed(reason) => write!(f, "the camera failed: {reason}"),
        }
    }
}

impl std::error::Error for MediaError {}

/// What a player's or a session's `changed` answers.
pub type MediaChangeFuture = Pin<Box<dyn Future<Output = ()>>>;
/// What a photo capture answers.
pub type PhotoFuture = Pin<Box<dyn Future<Output = Result<Photo, MediaError>>>>;
/// What a recording answers once it ends.
pub type RecordingFuture = Pin<Box<dyn Future<Output = Result<(), MediaError>>>>;
