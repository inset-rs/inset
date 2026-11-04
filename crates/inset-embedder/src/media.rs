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
//! `AVPlayerItemVideoOutput` and gives it to its player. A camera, once opened, lists the formats
//! it runs in, and starts in one of them with its outputs given whole ([`CameraOutputs`]): a layer
//! for its preview and a reader for its frames, made the same way, beside its photos and
//! recordings, all of which the system configures together, as Apple configures an
//! `AVCaptureSession` between `beginConfiguration` and `commitConfiguration` and Android binds
//! CameraX's use cases at once. A camera's photos are encoded by the system and its recordings
//! encoded and written to a file by the system: the app encodes nothing.
//!
//! Listing the cameras, opening one and starting it answer futures, as systems that answer them
//! later do: the web's `enumerateDevices` and `getUserMedia` answer promises, and Apple's
//! documentation of `AVCaptureSession` has a session started on a serial queue of its own, since
//! `startRunning` blocks until the session runs or fails.
//!
//! A player tells its media's tracks, plays one audio track and shows one text track or none, as
//! every system's player does (Apple's media selection groups, Media3's track overrides); it tells
//! what the media's tags say ([`MediaMetadata`]); and it hands the app the [`Cue`]s of its text
//! track to draw, the system drawing none, as most players draw their own (Media3's
//! `SubtitleView`, Safari over AVFoundation's legible output). A cue carries the media's own
//! styling, and the user's caption style is a system setting beside it, which the platform's
//! caption settings answer ([`Platform::caption_settings`](crate::Platform::caption_settings))
//! and the app merges in as it draws, as Media3's `SubtitleView` and WebKit's cue style sheet do.
//!
//! A player's position is read when the app needs it ([`MediaPlayer::position`]), as every
//! system lets an app read it (AVPlayer's `currentTime()`, Media3's `getCurrentPosition()`,
//! Windows' `PlaybackSession.Position`, the web's `currentTime`), and told only when it jumps (a
//! seek landing, a loop), as the systems tell it (Apple's `AVPlayerItemTimeJumped`, Media3's
//! `onPositionDiscontinuity`, the web's `seeked`): while it plays it moves on by itself, and an
//! app that shows it reads it at each frame it draws.
//!
//! The platform answers `None` where its system has no players, no cameras or no frames. A player
//! or a session reports its state through [`MediaPlayer::state`] and [`CaptureSession::state`],
//! and the cameras their list through [`Cameras::cameras`], each read when the future from its
//! `changed` resolves: the invariant a host keeps is that such a future resolves at every change
//! of what it reads, a player's metadata and cues included, at every jump of a player's position,
//! and when the player or session goes, so the app never waits on one for ever.

use std::any::Any;
use std::fmt;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::{CompositionLayer, Cue, Size};

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

    /// Where it is in the media now, read from the system at each call; it moves on by itself
    /// while the player plays.
    fn position(&self) -> Duration;

    /// Resolves once [`state`](Self::state), [`metadata`](Self::metadata) or
    /// [`cues`](Self::cues) may answer something else than when this was called, once the
    /// [`position`](Self::position) jumped, as a seek landing or a loop makes it, or once the
    /// player is gone; not as the position moves on while the player plays.
    fn changed(&self) -> MediaChangeFuture;

    /// Sends the picture to `output` from now on, and nowhere it went before; `None` sends it
    /// nowhere, as Media3's `setVideoSurface` does. The player holds the output while it sends
    /// the picture there, as a producer holds its layer. A layer no compositor of this host made
    /// shows nothing, and a reader no [`VideoFrameReaders`] of this host made reads nothing.
    fn set_video_output(&self, output: Option<VideoOutput>);

    /// Plays the audio track `id`, one of the state's [`tracks`](PlayerState::tracks), in place
    /// of the one playing.
    fn select_audio_track(&self, id: &str);

    /// Shows the text track `id`, one of the state's [`tracks`](PlayerState::tracks), or none:
    /// subtitles off. Its cues are what [`cues`](Self::cues) answers.
    fn select_text_track(&self, id: Option<&str>);

    /// What the media's tags say about it; empty until the player has read them.
    fn metadata(&self) -> MediaMetadata;

    /// The cues of the text track shown, to show now, as the media styles them: the user's
    /// caption style is not merged in. Empty while no text track is shown.
    fn cues(&self) -> Vec<Cue>;
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
    /// How long the media is; `None` until it is known, and for a live stream.
    pub duration: Option<Duration>,
    /// The picture's size, in its own pixels, as it is presented: its aspect corrected; `None`
    /// until it is known, and for media without a picture.
    pub size: Option<Size>,
    /// The media's tracks; empty until it is ready.
    pub tracks: Vec<MediaTrack>,
    /// Whether it reached the end of media it does not loop, until it plays or seeks again.
    pub ended: bool,
}

impl PlayerState {
    /// A player that is loading: nothing known yet.
    pub fn loading() -> PlayerState {
        PlayerState {
            status: PlayerStatus::Loading,
            playing: false,
            duration: None,
            size: None,
            tracks: Vec::new(),
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

/// One of a player's media's tracks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaTrack {
    /// Names the track for this player's run.
    pub id: String,
    pub kind: MediaTrackKind,
    /// A BCP 47 language tag; empty where the media gives none.
    pub language: String,
    /// Its name as the media gives it; empty where it gives none.
    pub label: String,
    /// Whether it plays now: one audio track at a time, and one text track or none.
    pub selected: bool,
}

/// What a track carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaTrackKind {
    Audio,
    Video,
    /// Subtitles or captions.
    Text,
}

/// What a player's media says about itself, from its tags; a field is `None` where it does not
/// say.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MediaMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    /// The cover image, encoded as the media holds it.
    pub artwork: Option<EncodedImage>,
}

/// An image as its file format encodes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedImage {
    /// Its format's MIME type, such as `image/jpeg`.
    pub mime_type: String,
    /// Shared, so that a player hands the same image out again without copying it.
    pub data: Arc<[u8]>,
}

/// The system's cameras, or `None` from [`Platform::cameras`](crate::Platform::cameras) where it
/// has none.
pub trait Cameras {
    /// The cameras attached now, in the system's order. Listing asks the user nothing.
    fn cameras(&self) -> CameraListFuture;

    /// Resolves once [`cameras`](Self::cameras) may answer something else than when this was
    /// called: a camera plugged in or out, as Apple's discovery session, Android's
    /// `CameraManager.AvailabilityCallback`, the web's `devicechange` event and Windows'
    /// `DeviceWatcher` tell it.
    fn changed(&self) -> MediaChangeFuture;

    /// Opens `camera`, asking the user for the cameras first where the system asks and the user
    /// has not answered, as an Android app opens Camera2's `CameraDevice` once it holds the
    /// camera permission: [`CameraError::NotAllowed`] where the user or the system's policy does
    /// not let the app use them, and [`CameraError::Unplugged`] for a camera no longer attached.
    fn open(&self, camera: &Camera) -> CameraOpenFuture;
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

/// A camera the app opened, controlled on the app's main thread: Camera2's `CameraDevice`. A
/// session started on it runs on after it is dropped.
pub trait CameraDevice {
    /// The formats the camera runs in, in the system's order, each with all its outputs: Apple's
    /// `AVCaptureDevice.formats`, Android's stream configuration map.
    fn formats(&self) -> Vec<CameraFormat>;

    /// Starts the camera in `format`, one of [`formats`](Self::formats), feeding `outputs`, which
    /// the system configures together, and answers the session once it runs, or is interrupted,
    /// from which it resumes by itself. Where the camera already runs for another session and the
    /// system runs a camera in one format at a time, the session joins that format instead, and
    /// its [`format`](CaptureSession::format) says so. It fails for what can happen at any time:
    /// the camera in use, unplugged, or not allowed; and for a format the camera does not list.
    fn start(&self, format: &CameraFormat, outputs: CameraOutputs) -> CameraStartFuture;
}

/// One way a camera runs, with all its outputs.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraFormat {
    /// The size of its frames and its preview, in the camera's own pixels.
    pub size: Size,
    /// The slowest frame rate it keeps at that size, in frames a second.
    pub min_frame_rate: f64,
    /// The fastest frame rate it keeps at that size, in frames a second.
    pub max_frame_rate: f64,
    /// The sizes it takes photos at while it runs so, in pixels.
    pub photo_sizes: Vec<Size>,
}

/// What a camera feeds, given whole to [`CameraDevice::start`]: each output the system's own, for
/// one use. An output left out is not fed.
#[derive(Clone, Default)]
pub struct CameraOutputs {
    /// The layer the preview shows on, drawn by the system into a layer of its own that is this
    /// layer's content. The session holds it while it runs. A layer no compositor of this host
    /// made shows nothing.
    pub preview: Option<Arc<dyn CompositionLayer>>,
    /// The reader the frames go to, for the app to draw. A reader no [`VideoFrameReaders`] of this
    /// host made reads nothing.
    pub frames: Option<Arc<dyn VideoFrameReader>>,
    /// The size photos are taken at, one of the format's
    /// [`photo_sizes`](CameraFormat::photo_sizes), each captured, processed and encoded by the
    /// system. In a joined format that does not list it, photos come at the system's own size
    /// for that format.
    pub photos: Option<Size>,
    /// Recordings, each encoded and written to a file by the system, with sound where `options`
    /// ask for it, which asks the user for the microphone as the session starts.
    pub movies: Option<MovieOptions>,
}

impl CameraOutputs {
    /// Outputs that feed nothing.
    pub fn new() -> CameraOutputs {
        CameraOutputs::default()
    }

    pub fn preview(mut self, layer: Arc<dyn CompositionLayer>) -> CameraOutputs {
        self.preview = Some(layer);
        self
    }

    pub fn frames(mut self, reader: Arc<dyn VideoFrameReader>) -> CameraOutputs {
        self.frames = Some(reader);
        self
    }

    pub fn photos(mut self, size: Size) -> CameraOutputs {
        self.photos = Some(size);
        self
    }

    pub fn movies(mut self, options: MovieOptions) -> CameraOutputs {
        self.movies = Some(options);
        self
    }
}

/// A camera running for the app, controlled on the app's main thread. Dropped, it stops for the
/// app: its preview leaves the layer it showed on, and its reader reads nothing more.
pub trait CaptureSession {
    /// The format it runs in: the one picked, unless it joined the format the camera ran in for
    /// another session.
    fn format(&self) -> CameraFormat;

    /// The session's state now.
    fn state(&self) -> CaptureState;

    /// Resolves once [`state`](Self::state) may answer something else than when this was
    /// called, or once the session is gone.
    fn changed(&self) -> MediaChangeFuture;

    /// Takes a photo while the session runs with photos among its outputs, answering it once the
    /// system has encoded it.
    fn take_photo(&self) -> PhotoFuture;

    /// Records to a new file at `path`, as a QuickTime movie, while the session runs with movies
    /// among its outputs, until [`stop_recording`](Self::stop_recording); answers once the file is
    /// written whole, or why it was not.
    fn start_recording(&self, path: PathBuf) -> RecordingFuture;

    /// Ends the recording, whose future then answers.
    fn stop_recording(&self);
}

/// What a camera session is doing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptureState {
    Running,
    /// No frames for now, for this reason where the system gives one; it resumes by itself.
    /// macOS gives none: Apple marks the interruption's reason unavailable there.
    Interrupted(Option<CameraInterruption>),
    /// For good, for this reason.
    Ended(CameraError),
}

/// Why a camera session gives no frames for now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraInterruption {
    /// Another app has the camera, as a phone allows one at a time.
    InUseByAnotherApp,
    /// The app is in the background.
    Background,
    /// The system is too hot or too busy.
    SystemPressure,
}

/// Why a camera did not open, start or run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CameraError {
    /// The user did not let the app use the cameras, or the system's policy does not.
    NotAllowed,
    /// Another app has the camera, and the system does not share it.
    InUse,
    /// The camera is no longer attached.
    Unplugged,
    /// The system failed, for its reason.
    Failed(String),
}

impl fmt::Display for CameraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CameraError::NotAllowed => f.write_str("the app may not use the camera"),
            CameraError::InUse => f.write_str("another app has the camera"),
            CameraError::Unplugged => f.write_str("the camera is no longer attached"),
            CameraError::Failed(reason) => write!(f, "the camera failed: {reason}"),
        }
    }
}

impl std::error::Error for CameraError {}

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

/// The system's frame readers, or `None` from
/// [`Platform::video_frame_readers`](crate::Platform::video_frame_readers) where it hands out no
/// frames.
pub trait VideoFrameReaders {
    /// A reader fed by nothing yet, which reads the frames of the source it is given to: a
    /// player's [`set_video_output`](MediaPlayer::set_video_output), a camera's outputs
    /// ([`CameraOutputs::frames`]). The reader goes when its last handle does: the app's, and its
    /// source's.
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
    fn frame_for(&self, display_time: Instant) -> Option<Arc<dyn VideoFrame>>;
}

/// One picture of a video as the system hands it out, as WebCodecs' `VideoFrame`. It holds its
/// slot in the decoder's pool until it, and every texture made of it, are dropped.
///
/// Each host implements it over the frames its system hands out. Platform code reaches the
/// system's own frame by downcasting through `Any` to the type of the host that handed it out:
/// on Apple's systems `inset-embedder-winit`'s `CoreVideoFrame`, whose `pixel_buffer` is the
/// `CVPixelBuffer` AVFoundation handed out.
pub trait VideoFrame: Any + Send + Sync {
    /// The picture's width, in its own pixels.
    fn width(&self) -> u32;

    /// The picture's height, in its own pixels.
    fn height(&self) -> u32;

    /// A texture of the frame on `device`, for sampling, sharing its memory where the system can.
    /// The texture holds the frame. `None` for a device that cannot take the frame's memory: on
    /// macOS a wgpu device not on Metal, or one Core Video makes no texture of the frame on.
    fn texture(&self, device: &wgpu::Device) -> Option<wgpu::Texture>;
}

/// Why a camera session's photo or recording answered nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MediaError {
    /// The session is interrupted or ended.
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

/// What a player's, a session's, the cameras' or the caption settings' `changed` answers.
pub type MediaChangeFuture = Pin<Box<dyn Future<Output = ()>>>;
/// What listing the cameras answers.
pub type CameraListFuture = Pin<Box<dyn Future<Output = Vec<Camera>>>>;
/// What opening a camera answers.
pub type CameraOpenFuture =
    Pin<Box<dyn Future<Output = Result<Box<dyn CameraDevice>, CameraError>>>>;
/// What starting a camera answers.
pub type CameraStartFuture =
    Pin<Box<dyn Future<Output = Result<Box<dyn CaptureSession>, CameraError>>>>;
/// What taking a photo answers.
pub type PhotoFuture = Pin<Box<dyn Future<Output = Result<Photo, MediaError>>>>;
/// What a recording answers once it ends.
pub type RecordingFuture = Pin<Box<dyn Future<Output = Result<(), MediaError>>>>;
