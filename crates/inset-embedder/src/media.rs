//! Players and cameras: the system's own media stack, as a host hands it to the app. Not a
//! dart:ui file: Flutter reaches media through its `video_player` and `camera` plugins, whose
//! controllers come from their method channels. These follow the systems' own model instead,
//! which they share: a player is one object made from a source, loading by itself, which the app
//! controls and whose state it observes (`AVPlayer`, Media3's `ExoPlayer`, Windows'
//! `MediaPlayer`, the web's `<video>`); a camera is a session on one device with outputs, each for
//! one use (`AVCaptureSession` with its preview layer and its photo, video data and movie file
//! outputs, CameraX's use cases).
//!
//! A player or a camera's picture goes to one destination at a time: shown on a [`Surface`],
//! where the system draws it into the surface's own layer, which is the only way protected and
//! HDR video show at all, with no copy; or read as frames, the newest each time, for the app's
//! GPU ([`FrameReader`]). A camera's photos are encoded by the system and its recordings encoded
//! and written to a file by the system: the app encodes nothing.
//!
//! The platform answers `None` where its system has no players or no cameras. A player or a
//! session reports its state through [`MediaPlayer::state`] and [`CaptureSession::state`], read
//! when the future from `changed` resolves: the invariant a host keeps is that such a future
//! resolves at every change of what the state answers, and when the player or session goes, so
//! the app never waits on one for ever.

use std::fmt;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use crate::{PickedFile, Size, Surface};

/// The system's players, or `None` from [`Platform::players`](crate::Platform::players) where it
/// has none.
pub trait Players {
    /// A player of `source`, which loads by itself, its state saying how far it got. A source
    /// the system cannot play gives a player that failed, with the system's reason.
    fn open(&self, source: MediaSource) -> Box<dyn MediaPlayer>;
}

/// Where a player's media comes from.
#[derive(Debug)]
pub enum MediaSource {
    /// A file the user picked, as the file dialogs answer it. A system plays the picks it gives
    /// a path for; a handle or a stream gives a player that failed, until a host reads one.
    File(PickedFile),
    /// A link the system fetches: `https:` and `file:`, and what else its media stack takes.
    Link(String),
}

impl MediaSource {
    pub fn link(link: impl Into<String>) -> MediaSource {
        MediaSource::Link(link.into())
    }

    pub fn file(file: PickedFile) -> MediaSource {
        MediaSource::File(file)
    }
}

/// One player, controlled on the app's main thread. Dropped, it stops, and its picture leaves
/// the surface it showed on.
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

    /// Shows the picture on `surface` from now on, drawn by the system into the surface's
    /// layer, and nowhere else: neither on the surface it showed on before nor to a frame
    /// reader. The player holds the surface while it shows on it, as a producer does. A surface
    /// no compositor of this host opened shows nothing.
    fn show_on(&self, surface: Arc<dyn Surface>);

    /// Hands the picture to a frame reader from now on, and to no surface; a reader asked for
    /// before reads nothing more.
    fn read_frames(&self) -> Arc<dyn FrameReader>;
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
/// Dropped, it stops, and its preview leaves the surface it showed on.
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

    /// Shows the camera's preview on `surface` from now on, drawn by the system into the
    /// surface's layer, and on no surface it showed on before. The session holds the surface
    /// while it shows on it. A surface no compositor of this host opened shows nothing.
    fn show_on(&self, surface: Arc<dyn Surface>);

    /// The camera's frames, the newest each time.
    fn frame_output(&self) -> Arc<dyn FrameReader>;

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

/// The newest pictures of a player or a camera, read from any thread. A host keeps only the
/// newest: one not read before the next arrives is dropped.
pub trait FrameReader: Send + Sync {
    /// The newest picture since the last read; `None` when none came since.
    fn newest(&self) -> Option<PixelBuffer>;
}

/// A picture of a player or a camera: the platform's own type, which the GPU reads with no copy.
/// On Apple platforms a `CVPixelBuffer` backed by an `IOSurface`, in 32-bit BGRA.
#[cfg(target_vendor = "apple")]
pub type PixelBuffer = objc2_core_foundation::CFRetained<objc2_core_video::CVPixelBuffer>;

/// A platform with no players and no cameras: no picture can exist, so a reader never answers
/// one.
#[cfg(not(target_vendor = "apple"))]
pub enum PixelBuffer {}

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
