//! What macOS and iOS share: ImageIO, as the framework's decoders and as the app's image codecs
//! (`image_codecs`); `IOSurface`s, the hardware buffers (`hardware_buffers`), and Metal's textures
//! over them on the app's GPU device (`gpu_interop`); Core Animation's
//! compositor of composition layers (`composition`) and its clock beside the host's
//! (`media_time`); AVFoundation's players and cameras (`media`), and Media Accessibility's caption settings (`caption_appearance`); the
//! notification center
//! (`notifications`); and for their pasteboards, the UTIs the framework's MIME types are named by
//! (`pasteboard_types`), the app's futures run on its main thread on behalf of the system
//! (`app_futures`), and the file a promised file's bytes are written into (`promised_files`).

pub(crate) mod app_futures;
pub(crate) mod caption_appearance;
pub(crate) mod composition;
pub(crate) mod gpu_interop;
pub(crate) mod hardware_buffers;
pub(crate) mod image_codecs;
pub(crate) mod media;
pub(crate) mod media_time;
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
