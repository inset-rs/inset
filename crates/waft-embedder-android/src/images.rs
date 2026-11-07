//! Which image decoders this build carries, and where they run.

use valo::ImageContext;
use valo_codec::{Decoder, ImageLoader};

/// The decoders this build carries, on a worker where it has one.
pub(crate) fn loader(images: ImageContext) -> Option<ImageLoader> {
    let decoders: Vec<Box<dyn Decoder>> = vec![
        #[cfg(feature = "image-software")]
        Box::new(valo_codec_software::SoftwareDecoder),
    ];
    #[cfg(feature = "image-worker")]
    {
        ImageLoader::with_worker(images, decoders).ok()
    }
    #[cfg(not(feature = "image-worker"))]
    {
        Some(ImageLoader::new(images, decoders))
    }
}
