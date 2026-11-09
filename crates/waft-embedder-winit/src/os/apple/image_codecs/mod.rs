//! The system's image codecs on macOS and iOS: Image I/O. An image opens as a `CGImageSource`, on
//! its bytes or on a file's URL, which Image I/O reads only as far as it is asked, and reads what
//! its properties say of it there (`source`), its EXIF fields among them (`exif`). A frame decodes
//! as a thumbnail at the size asked, which applies the file's orientation, drawn into the app's
//! memory in RGBA or, by the hardware codecs, into a hardware buffer of the app's own, an
//! `IOSurface` in BGRA (`decoding`, `hardware_buffers`). Pixels encode through a
//! `CGImageDestination`, the app's from their own memory and a hardware buffer's in place, while
//! its surface is locked (`encoding`). A format is a UTI to Image I/O and a MIME type to the app,
//! as `UTType` maps them (`formats`).
//!
//! Image I/O's work runs on a global dispatch queue, off the main thread, as Apple advises for
//! decoding, and each answer comes back through a future the app awaits on its main thread
//! ([`in_background`]).
//!
//! The invariant: each future resolves once, with what Image I/O answered or why not, a panic in
//! the work among the reasons, so an app never waits for ever.

// Image I/O's, Core Graphics' and Core Video's calls need `unsafe`; this module writes it for them.
#![allow(unsafe_code)]

mod decoding;
mod encoding;
mod exif;
mod formats;
mod source;

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use dispatch2::{DispatchQoS, DispatchQueue, GlobalQueueIdentifier};
use waft_embedder::image_codecs::{
    HardwareImageCodecs, ImageCodecs, ImageError, ImageErrorKind, ImageFuture, ImageSource,
    PixelBuffer,
};
use waft_embedder::{HardwareBuffer, Size};

use super::hardware_buffers::SurfaceBuffer;
use crate::answer::answer;
use source::ImageIoImage;

/// Image I/O's codecs.
pub(crate) fn image_codecs() -> Option<Rc<dyn ImageCodecs>> {
    Some(Rc::new(ImageIoCodecs))
}

/// Image I/O, decoding into the app's memory and encoding from it, with its hardware codecs.
struct ImageIoCodecs;

impl ImageCodecs for ImageIoCodecs {
    fn decoded_formats(&self) -> Vec<String> {
        formats::decoded()
    }

    fn encoded_formats(&self) -> Vec<String> {
        formats::encoded()
    }

    fn open(&self, data: Arc<[u8]>) -> ImageFuture<Box<dyn ImageSource>> {
        opened(in_background(move || source::open_data(&data)))
    }

    fn open_file(&self, path: &Path) -> ImageFuture<Box<dyn ImageSource>> {
        let path = path.to_owned();
        opened(in_background(move || source::open_file(&path)))
    }

    fn encode_pixels(
        &self,
        pixels: PixelBuffer,
        format: &str,
        quality: Option<f32>,
    ) -> ImageFuture<Vec<u8>> {
        let format = format.to_owned();
        in_background(move || encoding::pixels_encoded(&pixels, &format, quality))
    }

    fn hardware_image_codecs(&self) -> Option<&dyn HardwareImageCodecs> {
        Some(&ImageIoHardwareCodecs)
    }
}

/// Image I/O between encoded images and hardware buffers, `IOSurface`s.
struct ImageIoHardwareCodecs;

impl HardwareImageCodecs for ImageIoHardwareCodecs {
    fn decode(
        &self,
        image: &dyn ImageSource,
        frame: u32,
        max_size: Option<Size>,
    ) -> ImageFuture<Arc<dyn HardwareBuffer>> {
        let image: &dyn Any = image;
        let Some(image) = image.downcast_ref::<ImageIoImage>() else {
            return failed(ImageError::new(
                ImageErrorKind::Unsupported,
                "an image Image I/O did not open",
            ));
        };
        let decoding = image.decoding(frame, max_size);
        let decoded = in_background(move || decoding?.buffer());
        Box::pin(async move { Ok(Arc::new(decoded.await?) as Arc<dyn HardwareBuffer>) })
    }

    fn encode(
        &self,
        buffer: &dyn HardwareBuffer,
        format: &str,
        quality: Option<f32>,
    ) -> ImageFuture<Vec<u8>> {
        let buffer: &dyn Any = buffer;
        let Some(buffer) = buffer.downcast_ref::<SurfaceBuffer>() else {
            return failed(ImageError::new(
                ImageErrorKind::Unsupported,
                "a hardware buffer this host did not hand out",
            ));
        };
        let (buffer, format) = (buffer.clone(), format.to_owned());
        in_background(move || encoding::buffer_encoded(&buffer, &format, quality))
    }
}

/// The image `opening` answers, as the app holds it.
fn opened(opening: ImageFuture<ImageIoImage>) -> ImageFuture<Box<dyn ImageSource>> {
    Box::pin(async move { Ok(Box::new(opening.await?) as Box<dyn ImageSource>) })
}

/// What `work` answers, run on a global dispatch queue, as a future of the app's; a failure where
/// it panicked.
fn in_background<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, ImageError> + Send + 'static,
) -> ImageFuture<T> {
    let (given, answered) = answer(Err(ImageError::new(
        ImageErrorKind::Other,
        "the image's work ended without an answer",
    )));
    let queue = DispatchQueue::global_queue(GlobalQueueIdentifier::QualityOfService(
        DispatchQoS::UserInitiated,
    ));
    queue.exec_async(move || {
        let done = catch_unwind(AssertUnwindSafe(work)).unwrap_or_else(|_| {
            Err(ImageError::new(
                ImageErrorKind::Other,
                "the image's work panicked",
            ))
        });
        given.give(done);
    });
    Box::pin(answered)
}

/// A future that answers `error` at once.
fn failed<T: 'static>(error: ImageError) -> ImageFuture<T> {
    Box::pin(std::future::ready(Err(error)))
}
