//! A camera's pictures as an `AImageReader` takes them: a stream of one size and format whose
//! window the camera feeds, each picture an `AImage` lent with its slot in the reader's pool and
//! handed, the newest at each delivery, to what the stream was made with, on the reader's own
//! thread. A `PRIVATE` picture's memory is its `AHardwareBuffer`, which the compositor shows and the
//! GPU samples; a JPEG one's is the encoded photo.
//!
//! The invariant: a picture holds its slot until it is dropped, and holds the reader until then
//! too, so the reader is deleted only once its last picture is, never under one still shown.

// The image reader's pictures cross threads, which `ndk` cannot vouch for; this module does.
#![allow(unsafe_code)]

use std::sync::{Arc, Mutex, PoisonError, Weak};

use ndk::hardware_buffer::{HardwareBufferRef, HardwareBufferUsage};
use ndk::media::image_reader::{AcquireResult, Image, ImageFormat, ImageReader};
use ndk::media_error::MediaError;
use ndk::native_window::NativeWindow;

/// A stream of pictures, its reader deleted once the stream and its last picture are dropped.
pub(super) struct ImageStream(Arc<Reader>);

/// The reader, behind a lock only so that its listener can be set once it is shared.
struct Reader(Mutex<ImageReader>);

// SAFETY: an `AImageReader` guards its own state, and is acquired from on its listener's thread
// and deleted on whichever thread lets go of it last.
unsafe impl Send for Reader {}
unsafe impl Sync for Reader {}

impl ImageStream {
    /// A stream of `size` pictures in `format` for `usage`, at most `max_images` of them lent at
    /// once, each newest one handed to `delivered`.
    pub(super) fn new(
        size: [u32; 2],
        format: ImageFormat,
        usage: HardwareBufferUsage,
        max_images: i32,
        delivered: impl Fn(CameraImage) + Send + 'static,
    ) -> Result<ImageStream, MediaError> {
        let reader =
            ImageReader::new_with_usage(size[0] as i32, size[1] as i32, format, usage, max_images)?;
        let reader = Arc::new(Reader(Mutex::new(reader)));
        let listened: Weak<Reader> = Arc::downgrade(&reader);
        reader
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .set_image_listener(Box::new(move |reader: &ImageReader| {
                let Some(held) = listened.upgrade() else {
                    return;
                };
                if let Ok(AcquireResult::Image(image)) = reader.acquire_latest_image() {
                    delivered(CameraImage {
                        image,
                        _reader: held,
                    });
                }
            }))?;
        Ok(ImageStream(reader))
    }

    /// The window the camera feeds the stream through.
    pub(super) fn window(&self) -> Result<NativeWindow, MediaError> {
        self.0
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .window()
    }
}

/// One picture, lent with its slot in its reader's pool until dropped.
pub(super) struct CameraImage {
    image: Image,
    _reader: Arc<Reader>,
}

// SAFETY: an `AImage` is read and deleted from any thread, one at a time, which owning it here
// makes sure of; its reader is kept while it lives.
unsafe impl Send for CameraImage {}
unsafe impl Sync for CameraImage {}

impl CameraImage {
    /// The picture's memory, a reference of its own on it.
    pub(super) fn hardware_buffer(&self) -> Result<HardwareBufferRef, MediaError> {
        Ok(self.image.hardware_buffer()?.acquire())
    }

    /// The bytes of its one plane: a JPEG picture's encoded photo.
    pub(super) fn encoded(&self) -> Result<Vec<u8>, MediaError> {
        Ok(self.image.plane_data(0)?.to_vec())
    }
}
