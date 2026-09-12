//! Flutter counterpart: dart:ui `instantiateImageCodec`, `Codec` and `FrameInfo`.
//!
//! Decoding belongs to the host, as it does in Flutter, and for the same reason: a platform's own
//! codec reads formats no bundled decoder carries and can scale an image while it decodes. What
//! crosses back is an [`Image`](crate::Image) the renderer can already draw, so the framework
//! never handles pixels and never learns which codec answered.
//!
//! The shape is Dart's: opening bytes answers a codec, and the codec hands out one frame after
//! another. Both answers are futures, so a host may decode on a worker thread and wake the
//! framework when it is done, or decode locally. Dropping a future abandons its result; a host
//! may skip queued work, but an already-running native decode or GPU command can still finish.

use std::fmt::{self, Display};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::Image;

/// A codec being opened. Flutter's `Future<ui.Codec>`.
pub type ImageCodecFuture =
    Pin<Box<dyn Future<Output = Result<Box<dyn ImageCodec>, ImageDecodeError>>>>;

/// A frame being decoded. Flutter's `Future<ui.FrameInfo>`.
///
/// It borrows the codec it came from until it arrives, which is what keeps one codec to one frame
/// at a time.
pub type ImageFrameFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ImageFrame, ImageDecodeError>> + 'a>>;

/// One encoded image, opened and ready to give up its frames. Flutter dart:ui `Codec`.
///
/// A still image is a codec with one frame, so a caller never has to know in advance which it
/// has. Dropping the codec is what tells the host the image is no longer wanted.
pub trait ImageCodec {
    /// How many frames the file holds; one for a still image.
    fn frame_count(&self) -> u32;

    /// What happens when the last frame has been shown.
    fn repetition(&self) -> ImageRepetition;

    /// The next frame, wrapping back to the first after the last, as Dart's does.
    ///
    /// A caller that waits for a frame holds the codec while it does, so nothing can ask the same
    /// codec for two frames at once.
    fn next_frame(&mut self) -> ImageFrameFuture<'_>;
}

/// One decoded frame, prepared and ready to draw. Flutter dart:ui `FrameInfo`.
#[derive(Clone, Debug)]
pub struct ImageFrame {
    /// The frame itself, at whatever size the host produced.
    pub image: Image,
    /// How long this frame stays on screen.
    ///
    /// Zero for a still image, and for an animation whose file gives no delay — a caller showing
    /// one picks its own, as browsers do.
    pub duration: Duration,
}

/// How an animation repeats once it has played through.
///
/// Dart counts this as an integer, with minus one meaning forever and zero meaning once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageRepetition {
    /// Play once and stop, which is also what a still image does.
    #[default]
    Once,
    /// Play this many more times after the first pass.
    Times(u32),
    /// Never stop, which is what most animated files ask for.
    Forever,
}

/// Why an image could not be read. Flutter reports these as an exception on the codec future.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageDecodeError {
    /// This host has no decoder at all.
    NoDecoder,
    /// No decoder here reads this format.
    UnknownFormat,
    /// The format was recognised and the bytes are damaged; the text is for a log, not a user.
    Damaged(String),
    /// The file decoded to nothing.
    Empty,
    /// A host or decoder failed without establishing that the encoded image is damaged.
    Failed(String),
}

impl Display for ImageDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageDecodeError::NoDecoder => formatter.write_str("this host decodes no images"),
            ImageDecodeError::UnknownFormat => formatter.write_str("unrecognised image format"),
            ImageDecodeError::Damaged(reason) => write!(formatter, "damaged image: {reason}"),
            ImageDecodeError::Empty => formatter.write_str("image has no pixels"),
            ImageDecodeError::Failed(reason) => write!(formatter, "image decode failed: {reason}"),
        }
    }
}

impl std::error::Error for ImageDecodeError {}
