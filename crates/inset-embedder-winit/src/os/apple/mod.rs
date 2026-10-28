//! What macOS and iOS share: ImageIO; the notification center (`notifications`); and for their
//! pasteboards, the UTIs the framework's MIME types are named by (`pasteboard_types`), the app's
//! futures run on its main thread on behalf of the system (`app_futures`), and the file a promised
//! file's bytes are written into (`promised_files`).

pub(crate) mod app_futures;
pub(crate) mod notifications;
pub(crate) mod pasteboard_types;
pub(crate) mod promised_files;

use valo_codec::Decoder;

/// ImageIO's decoders, where this build carries them.
pub(crate) fn image_io_decoders() -> Vec<Box<dyn Decoder>> {
    vec![
        #[cfg(feature = "image-apple")]
        Box::new(valo_codec_apple::AppleDecoder::default()),
    ]
}
