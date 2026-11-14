//! The system's image codecs on Android: the NDK's own decoder and encoder, `AImageDecoder` and
//! `AndroidBitmap_compress`, which Java's `ImageDecoder` and `Bitmap.compress` stand on, and
//! Java's `ExifInterface` for what neither reads. An image opens as a decoder over its bytes or
//! over its file, which it reads only as far as it is asked, and reads what its header says, its
//! format and size, and an animation's frames with their durations (`source`, over the NDK's
//! decoder in `image_decoder`); and what its EXIF says, its orientation and how it was taken,
//! through `ExifInterface` over JNI (`exif_interface`, `exif`). A frame decodes upright, as the
//! decoder applies the file's orientation, at the size asked, into the app's memory in RGBA,
//! premultiplied, in sRGB (`decoding`, `target_size`). Pixels encode from the app's memory as
//! JPEG, PNG or lossy WebP (`encoding`, over the NDK's encoder in `bitmap_compress`). What a device
//! decodes beyond what every Android device does, and which formats' pictures its decoder turns
//! upright itself, the host turning the others (`orienting`), are found by decoding a small
//! picture of each such format, once, as the codecs are made (`abilities`, `formats`).
//!
//! There are no hardware codecs: `AImageDecoder` writes only memory its caller holds, and only
//! Java's `ImageDecoder` makes a hardware bitmap, whose buffer the host does not reach.
//!
//! The work runs on the codecs' threads (`workers`), off the app's, and each answer comes back
//! through a future the app awaits on its own thread ([`in_background`]).
//!
//! The invariant: each future resolves once, with what Android answered or why not, a panic in
//! the work among the reasons, so an app never waits for ever.

#[cfg(target_os = "android")]
mod abilities;
#[cfg(target_os = "android")]
mod bitmap_compress;
#[cfg(target_os = "android")]
mod decoding;
mod encoding;
mod exif;
#[cfg(target_os = "android")]
mod exif_interface;
mod formats;
#[cfg(target_os = "android")]
mod image_decoder;
mod orienting;
#[cfg(target_os = "android")]
mod source;
mod target_size;
#[cfg(target_os = "android")]
mod workers;

#[cfg(target_os = "android")]
pub(crate) use android::AndroidImageCodecs;

#[cfg(target_os = "android")]
mod android {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::path::Path;
    use std::sync::{Arc, OnceLock};

    use waft_embedder::image_codecs::{
        HardwareImageCodecs, ImageCodecs, ImageError, ImageErrorKind, ImageFuture, ImageSource,
        PixelBuffer,
    };

    use super::abilities::Abilities;
    use super::decoding::Encoded;
    use super::{encoding, formats, source, workers};
    use crate::answer::answer;
    use crate::application::Application;

    /// Android's decoder and encoder, between encoded images and the app's memory.
    pub(crate) struct AndroidImageCodecs {
        /// What the device's decoder does, found once, on the codecs' threads from the start.
        abilities: Arc<OnceLock<Abilities>>,
        /// The process's application, through which the codecs' threads reach Java's
        /// `ExifInterface`.
        application: Application,
    }

    impl AndroidImageCodecs {
        /// The codecs, starting to find what the device's decoder does: a first HEIF decoder
        /// takes a tenth of a second or more to start, which the app's thread does not wait for
        /// unless it asks before then.
        pub(crate) fn new(application: Application) -> AndroidImageCodecs {
            let abilities = Arc::new(OnceLock::new());
            let finding = Arc::clone(&abilities);
            workers::run(move || {
                finding.get_or_init(Abilities::found);
            });
            AndroidImageCodecs {
                abilities,
                application,
            }
        }

        /// The image `encoded` holds, opened on the codecs' threads.
        fn opening(&self, encoded: Encoded) -> ImageFuture<Box<dyn ImageSource>> {
            let application = self.application.clone();
            let abilities = Arc::clone(&self.abilities);
            opened(in_background(move || {
                let abilities = abilities.get_or_init(Abilities::found);
                source::open(encoded, &application, abilities)
            }))
        }
    }

    impl ImageCodecs for AndroidImageCodecs {
        /// What every device decodes, and what this one's codecs decode beyond it, found by
        /// decoding a picture of each; asked before that is done, it waits for it.
        fn decoded_formats(&self) -> Vec<String> {
            self.abilities.get_or_init(Abilities::found).decoded.clone()
        }

        fn encoded_formats(&self) -> Vec<String> {
            formats::encoded()
        }

        fn open(&self, data: Arc<[u8]>) -> ImageFuture<Box<dyn ImageSource>> {
            self.opening(Encoded::Bytes(data))
        }

        fn open_file(&self, path: &Path) -> ImageFuture<Box<dyn ImageSource>> {
            self.opening(Encoded::File(path.to_owned()))
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

        /// None: `AImageDecoder` decodes into no hardware buffer, and `AndroidBitmap_compress`
        /// reads none.
        fn hardware_image_codecs(&self) -> Option<&dyn HardwareImageCodecs> {
            None
        }
    }

    /// The image `opening` answers, as the app holds it.
    fn opened(opening: ImageFuture<source::AndroidImage>) -> ImageFuture<Box<dyn ImageSource>> {
        Box::pin(async move { Ok(Box::new(opening.await?) as Box<dyn ImageSource>) })
    }

    /// What `work` answers, run on the codecs' threads, as a future of the app's; a failure where
    /// it panicked.
    pub(super) fn in_background<T: Send + 'static>(
        work: impl FnOnce() -> Result<T, ImageError> + Send + 'static,
    ) -> ImageFuture<T> {
        let (given, answered) = answer(Err(ImageError::new(
            ImageErrorKind::Other,
            "the image's work ended without an answer",
        )));
        workers::run(move || {
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
    pub(super) fn failed<T: 'static>(error: ImageError) -> ImageFuture<T> {
        Box::pin(std::future::ready(Err(error)))
    }
}

#[cfg(target_os = "android")]
use android::{failed, in_background};
