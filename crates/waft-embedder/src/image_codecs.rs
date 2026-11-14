//! The system's image codecs, as a host hands them to the app. Not a dart:ui file: dart:ui's
//! `instantiateImageCodec` (this crate's `image`) decodes for the framework's own drawing and
//! answers an [`Image`](crate::Image) the renderer draws, never its pixels. These are the system's
//! codecs as an app uses them itself, to read and write image files: Apple's Image I/O,
//! Android's `AImageDecoder` and `AndroidBitmap_compress`, Windows' `BitmapDecoder` and
//! `BitmapEncoder`, the web's `ImageDecoder`. They follow `wasdk:media/image-codecs` one to one,
//! and are a module of their own, named as that interface is, since the crate's root holds
//! dart:ui's `ImageFrame`, which this module's [`ImageFrame`] would collide with.
//!
//! Pixels are in one of two kinds of memory, and every move between them is a call of its own, so
//! an app sees each copy it makes. An image decodes into the app's memory, a [`PixelBuffer`], on
//! every host ([`ImageSource::decode_pixels`]), which
//! [`Platform::import_pixels`](crate::Platform::import_pixels) uploads as an image to draw, and
//! encodes from one ([`ImageCodecs::encode_pixels`]). Where the system can, its hardware codecs
//! ([`HardwareImageCodecs`]) decode into GPU memory, a [`HardwareBuffer`] of the app's own, which
//! [`Platform::import_hardware_buffer`](crate::Platform::import_hardware_buffer) draws with no
//! copy and which holds no slot in a pool, and encode from one, a buffer the app drew or a video
//! frame's ([`VideoFrame::hardware_buffer`](crate::VideoFrame::hardware_buffer)) among them:
//! Apple's codecs decode into an `IOSurface`; Android's NDK decoder writes only the app's memory,
//! so Android's host has none, a hardware bitmap being Java's `ImageDecoder`'s alone. A hardware
//! buffer comes back to the app's memory only through [`HardwareBuffer::read_pixels`].
//!
//! An image is opened from its bytes, or from a file, which the system reads only as far as it
//! needs; opening reads what the image says of itself, its format, size, frames, orientation and
//! metadata, and its pixels are decoded when asked, upright, at the size asked. A photo's location
//! is the user's location data, given as the file holds it to the app that holds the file; encoding
//! writes no metadata, so pixels encoded again carry no location.
//!
//! The invariant a host keeps: each future resolves, once, whether or not the app still holds
//! what it asked of, since the system decodes and encodes in the background.

use std::any::Any;
use std::fmt;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

pub use valo::{AlphaType, PixelBuffer, PixelFormat, PixelLayout};

use crate::{HardwareBuffer, Size};

/// The system's image codecs, or `None` from
/// [`Platform::image_codecs`](crate::Platform::image_codecs) where it has none.
pub trait ImageCodecs {
    /// The formats it decodes, by MIME type, such as `image/jpeg`.
    fn decoded_formats(&self) -> Vec<String>;

    /// The formats it encodes, by MIME type.
    fn encoded_formats(&self) -> Vec<String>;

    /// The image `data` encodes, read as far as its size, frames, orientation and metadata:
    /// [`ImageErrorKind::Unsupported`] for bytes of no format it decodes,
    /// [`ImageErrorKind::Damaged`] for bytes it cannot read that far.
    fn open(&self, data: Arc<[u8]>) -> ImageFuture<Box<dyn ImageSource>>;

    /// The image in the file at `path`, which the system reads only as far as it needs, as a
    /// photo's metadata or the small picture it carries: [`ImageErrorKind::NotFound`] for a path
    /// with no file.
    fn open_file(&self, path: &Path) -> ImageFuture<Box<dyn ImageSource>>;

    /// `pixels` encoded in `format`, one of the [`encoded_formats`](Self::encoded_formats), at
    /// `quality` from `0.0`, the smallest, to `1.0`, the best, where the format loses detail, or
    /// the system's own where `None`: [`ImageErrorKind::Unsupported`] for a format it does not
    /// encode.
    fn encode_pixels(
        &self,
        pixels: PixelBuffer,
        format: &str,
        quality: Option<f32>,
    ) -> ImageFuture<Vec<u8>>;

    /// The system's hardware codecs, or `None` where it neither decodes into hardware buffers
    /// nor encodes from them.
    fn hardware_image_codecs(&self) -> Option<&dyn HardwareImageCodecs>;
}

/// An image opened by the system's codecs from its encoded bytes, as Apple's `CGImageSource` and
/// Android's `ImageDecoder.Source` are: what it says of itself, read as it opened, and its frames
/// decoded as the app asks for them. A frame it does not have fails, as [`ImageErrorKind::Other`].
///
/// A host gets its own type back from an image it is given by downcasting through `Any`.
pub trait ImageSource: Any {
    /// Its format's MIME type, one of the [`ImageCodecs::decoded_formats`].
    fn mime_type(&self) -> String;

    /// Its size in pixels, upright, as its first frame decodes at full size; an animation's
    /// frames share it.
    fn size(&self) -> Size;

    /// Its frames, in order: one for a still image, an animation's each with how long it shows,
    /// and the pages of a file of several.
    fn frames(&self) -> Vec<ImageFrame>;

    /// How the file stores the picture turned from upright. It is decoded upright, and
    /// [`size`](Self::size) is upright, so an app reads it to show it or to keep it.
    fn orientation(&self) -> ImageOrientation;

    /// What the file says of how it was taken.
    fn metadata(&self) -> ImageMetadata;

    /// Frame `frame` of [`frames`](Self::frames) in the app's memory, upright, scaled down to fit
    /// `max_size`, in pixels, keeping its shape, where given; in a layout the system picks, which
    /// the buffer names.
    fn decode_pixels(&self, frame: u32, max_size: Option<Size>) -> ImageFuture<PixelBuffer>;
}

/// The system's codecs between encoded images and hardware buffers, with no copy through the
/// app's memory, or `None` from [`ImageCodecs::hardware_image_codecs`] where it has none.
pub trait HardwareImageCodecs {
    /// Frame `frame` of `image`'s frames in a hardware buffer of the app's own, for sampling,
    /// upright, scaled down to fit `max_size`, in pixels, keeping its shape, where given:
    /// [`ImageErrorKind::TooLarge`] where it is larger than a hardware buffer the system makes. An
    /// image another host opened fails, as [`ImageErrorKind::Unsupported`].
    fn decode(
        &self,
        image: &dyn ImageSource,
        frame: u32,
        max_size: Option<Size>,
    ) -> ImageFuture<Arc<dyn HardwareBuffer>>;

    /// `buffer`'s pixels encoded in `format`, one of the image codecs'
    /// [`encoded_formats`](ImageCodecs::encoded_formats), at `quality` as
    /// [`ImageCodecs::encode_pixels`] takes it, reading what the GPU has finished writing, as
    /// [`HardwareBuffer::read_pixels`] does: [`ImageErrorKind::Unsupported`] for a format it does
    /// not encode, or a buffer whose pixels it does not read, such as one another host handed out.
    fn encode(
        &self,
        buffer: &dyn HardwareBuffer,
        format: &str,
        quality: Option<f32>,
    ) -> ImageFuture<Vec<u8>>;
}

/// One of an image's frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImageFrame {
    /// How long it shows in an animation; `None` for a still image's one frame, a page, and a
    /// frame whose file says nothing, for which an app picks its own.
    pub duration: Option<Duration>,
}

/// How the picture a file stores is turned from upright: EXIF's `Orientation`, mirrored left to
/// right where `mirrored`, then turned clockwise by `rotation`, to show it upright.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImageOrientation {
    pub mirrored: bool,
    pub rotation: ImageRotation,
}

/// How far a picture is turned clockwise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageRotation {
    #[default]
    None,
    QuarterTurn,
    HalfTurn,
    ThreeQuarterTurn,
}

/// What an image's file says of how it was taken: EXIF's fields a photo app shows. A field is
/// `None` where the file says nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImageMetadata {
    /// When it was taken, as the camera's clock read it: EXIF's `DateTimeOriginal`, with its
    /// `OffsetTimeOriginal`.
    pub taken: Option<DateTime>,
    /// The camera's maker, as the camera names itself: EXIF's `Make`.
    pub camera_make: Option<String>,
    /// The camera's model, as the camera names itself: EXIF's `Model`.
    pub camera_model: Option<String>,
    /// The lens, as the camera names it: EXIF's `LensModel`.
    pub lens_model: Option<String>,
    /// How long the shutter was open, in seconds: EXIF's `ExposureTime`.
    pub exposure_time: Option<f64>,
    /// The aperture, as an f-number: EXIF's `FNumber`.
    pub f_number: Option<f64>,
    /// The sensor's sensitivity, as an ISO speed: EXIF's `PhotographicSensitivity`.
    pub iso: Option<u32>,
    /// The lens's focal length, in millimetres: EXIF's `FocalLength`.
    pub focal_length: Option<f64>,
    /// The focal length on a 35 mm film camera with the same view, in millimetres: EXIF's
    /// `FocalLengthIn35mmFilm`.
    pub equivalent_focal_length: Option<f64>,
    /// How far the exposure was set from the camera's metering, in stops: EXIF's
    /// `ExposureBiasValue`.
    pub exposure_bias: Option<f64>,
    /// Whether the flash fired: EXIF's `Flash`.
    pub flash_fired: Option<bool>,
    /// Where it was taken: EXIF's GPS position.
    pub location: Option<Location>,
}

/// A date and time of day as a clock read it, and how far that clock was from UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateTime {
    pub year: i32,
    /// From 1, January, to 12.
    pub month: u8,
    /// From 1.
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    /// Minutes ahead of UTC, negative behind it; `None` where the file does not say, so the
    /// moment is the camera's local time, wherever it was.
    pub utc_offset: Option<i16>,
}

/// A place on Earth, as GPS gives it: WGS 84.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Location {
    /// Degrees north of the equator, negative south of it.
    pub latitude: f64,
    /// Degrees east of Greenwich, negative west of it.
    pub longitude: f64,
    /// Metres above sea level, negative below it.
    pub altitude: Option<f64>,
}

/// Why pixels were not read, decoded or encoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageError {
    pub kind: ImageErrorKind,
    /// The system's own words, for logs: not for showing as they are.
    pub message: String,
}

impl ImageError {
    pub fn new(kind: ImageErrorKind, message: impl Into<String>) -> ImageError {
        ImageError {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.kind {
            ImageErrorKind::Unsupported => "unsupported",
            ImageErrorKind::Damaged => "damaged",
            ImageErrorKind::TooLarge => "too large",
            ImageErrorKind::NotFound => "not found",
            ImageErrorKind::OutOfMemory => "out of memory",
            ImageErrorKind::Other => "failed",
        };
        write!(f, "image {kind}: {}", self.message)
    }
}

impl std::error::Error for ImageError {}

/// The kind of an image's failure, which an app words for its user and acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageErrorKind {
    /// A format the system does not decode or encode, or pixels in a layout it does not read.
    Unsupported,
    /// The bytes are of a format the system reads, and are not a whole image of it.
    Damaged,
    /// The image is larger than the system decodes it at into that memory; asked for at a
    /// maximum size, it may decode.
    TooLarge,
    /// No file at the path.
    NotFound,
    /// More memory than the system has left for a hardware buffer, or than the platform lets the
    /// app hold.
    OutOfMemory,
    /// Anything else the system reports.
    Other,
}

/// What the image codecs, an image source and a hardware buffer's pixels answer.
pub type ImageFuture<T> = Pin<Box<dyn Future<Output = Result<T, ImageError>>>>;
