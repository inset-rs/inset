//! What Waft's hosts on Apple's systems share, the winit host's macOS and the iOS host alike, with
//! no windowing of their own: ImageIO, as the framework's decoders and as the app's image codecs
//! (`image_codecs`); `IOSurface`s, the hardware buffers (`hardware_buffers`), and Metal's textures
//! over them on the app's GPU device (`gpu_interop`); Core Animation's compositor of composition
//! layers (`composition`) and its clock beside the host's (`media_time`); AVFoundation's players
//! and cameras (`media`), and Media Accessibility's caption settings (`caption_appearance`); the
//! notification center (`notifications`); and for their pasteboards, the UTIs the framework's MIME
//! types are named by (`pasteboard_types`), the app's futures run on its main thread on behalf of
//! the system (`app_futures`), and the file a promised file's bytes are written into
//! (`promised_files`).
//!
//! A host places what it makes here in its own views and windows: this crate never names a window,
//! so a layer, a buffer or a notification behaves the same under either host. Elsewhere than macOS
//! and iOS the crate is empty.
#![cfg(any(target_os = "macos", target_os = "ios"))]

mod answer;
pub mod app_futures;
pub mod caption_appearance;
pub mod composition;
pub mod gpu_interop;
pub mod hardware_buffers;
pub mod image_codecs;
pub mod media;
mod media_futures;
pub mod media_time;
pub mod notifications;
pub mod pasteboard_types;
pub mod promised_files;

use valo_codec::Decoder;

/// ImageIO's decoders, where this build carries them.
pub fn image_io_decoders() -> Vec<Box<dyn Decoder>> {
    vec![
        #[cfg(feature = "image-apple")]
        Box::new(valo_codec_apple::AppleDecoder::default()),
    ]
}
