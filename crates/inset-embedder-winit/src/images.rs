//! Image decoder composition: which decoders this build carries, and where they run.

use valo::ImageContext;
use valo_codec::{Decoder, ImageLoader};

/// Selects where the built-in image decoders run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DecodeExecution {
    /// Polls decoding on the caller's executor and starts no threads.
    #[cfg_attr(
        not(all(feature = "image-worker", not(target_arch = "wasm32"))),
        default
    )]
    Local,

    /// Owns decoder state on one native worker, waking the event loop for results.
    #[cfg(all(feature = "image-worker", not(target_arch = "wasm32")))]
    #[default]
    Worker,
}

/// `create_image_loader` registers the decoders this build carries, in preference order.
///
/// The platform's own codec comes first where it was compiled in: it reads formats no portable
/// decoder carries, and hands back frames the GPU can sample without an upload. The software
/// decoder takes whatever that one declines. The loader behaves the same either side of
/// [`DecodeExecution`].
pub fn create_image_loader(
    images: ImageContext,
    execution: DecodeExecution,
) -> std::io::Result<ImageLoader> {
    let decoders: Vec<Box<dyn Decoder>> = vec![
        #[cfg(all(feature = "image-apple", any(target_os = "macos", target_os = "ios")))]
        Box::new(valo_codec_apple::AppleDecoder::default()),
        #[cfg(feature = "image-software")]
        Box::new(valo_codec_software::SoftwareDecoder),
    ];
    match execution {
        DecodeExecution::Local => Ok(ImageLoader::new(images, decoders)),
        #[cfg(all(feature = "image-worker", not(target_arch = "wasm32")))]
        DecodeExecution::Worker => ImageLoader::with_worker(images, decoders),
    }
}
