//! What this device's decoder does, found once by decoding small pictures (`formats`): the
//! formats it decodes beyond what every device does, and the formats whose pictures it turns
//! upright by their orientation itself, the rest left to the host (`orienting`).

use std::sync::Arc;

use waft_embedder::image_codecs::{PixelBuffer, PixelFormat};

use super::decoding::{Encoded, first_frame_of};
use super::formats;

/// What the device's decoder does.
pub(super) struct Abilities {
    /// The formats it decodes, by MIME type.
    pub(super) decoded: Vec<String>,
    /// The formats it turns upright itself, by MIME type.
    turned: Vec<&'static str>,
}

impl Abilities {
    /// Found by decoding each format's picture.
    pub(super) fn found() -> Abilities {
        let probed = formats::PROBED
            .into_iter()
            .filter(|(_, sample)| first_frame_of(encoded(sample)).is_some())
            .map(|(mime, _)| mime);
        let decoded = formats::DECODED
            .into_iter()
            .chain(probed)
            .map(str::to_owned)
            .collect();
        let turned = formats::ORIENTATION_PROBED
            .into_iter()
            .filter(|(_, sample)| {
                first_frame_of(encoded(sample)).is_some_and(|pixels| is_mirrored(&pixels))
            })
            .map(|(mime, _)| mime)
            .collect();
        Abilities { decoded, turned }
    }

    /// Whether the decoder leaves pictures of `mime` as their files store them, for the host to
    /// turn: only a format whose picture it decoded and did not turn. A format no picture is
    /// probed for is one whose files say no orientation, or one the decoder turns itself, as it
    /// does a raw photo's.
    pub(super) fn leaves_unturned(&self, mime: &str) -> bool {
        formats::ORIENTATION_PROBED
            .iter()
            .any(|(probed, _)| *probed == mime)
            && !self.turned.contains(&mime)
    }
}

fn encoded(sample: &'static [u8]) -> Encoded {
    Encoded::Bytes(Arc::from(sample))
}

/// Whether a probe's picture, red on the left and blue on the right as stored, came out
/// mirrored: blue on the left.
fn is_mirrored(pixels: &PixelBuffer) -> bool {
    let layout = pixels.layout();
    let start = (layout.size[1] / 2 * layout.row_bytes + 4) as usize;
    let Some(pixel) = pixels.bytes().get(start..start + 4) else {
        return false;
    };
    let (red, blue) = match layout.format {
        PixelFormat::Rgba8 => (pixel[0], pixel[2]),
        PixelFormat::Bgra8 => (pixel[2], pixel[0]),
    };
    blue > 200 && red < 60
}
