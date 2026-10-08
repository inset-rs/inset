//! What macOS and iOS share: ImageIO.

use valo_codec::Decoder;

/// ImageIO's decoders, where this build carries them.
pub(super) fn image_io_decoders() -> Vec<Box<dyn Decoder>> {
    vec![
        #[cfg(feature = "image-apple")]
        Box::new(valo_codec_apple::AppleDecoder::default()),
    ]
}
