//! Pixels encoded: the app's rows handed to `AndroidBitmap_compress` as RGBA in sRGB, with their
//! alpha as the buffer says it, in the format and at the quality asked. Android reads no BGRA, so
//! BGRA pixels are copied into RGBA first. Nothing is written but the picture: no metadata.

use std::borrow::Cow;

#[cfg(target_os = "android")]
use waft_embedder::image_codecs::ImageError;
use waft_embedder::image_codecs::{PixelBuffer, PixelFormat};

#[cfg(target_os = "android")]
use super::bitmap_compress::{RgbaRows, compressed};
#[cfg(target_os = "android")]
use super::formats::{compression_of, quality_of};

/// `pixels` encoded in `format`, a MIME type, at `quality`.
#[cfg(target_os = "android")]
pub(super) fn pixels_encoded(
    pixels: &PixelBuffer,
    format: &str,
    quality: Option<f32>,
) -> Result<Vec<u8>, ImageError> {
    let compression = compression_of(format)?;
    let quality = quality_of(quality)?;
    let layout = pixels.layout();
    let rows = RgbaRows {
        bytes: &rgba_of(pixels),
        size: layout.size,
        stride: layout.row_bytes,
        alpha: layout.alpha,
    };
    compressed(&rows, compression, quality)
}

/// `pixels`' bytes as RGBA, in their rows: their own where they are RGBA, a copy where BGRA.
fn rgba_of(pixels: &PixelBuffer) -> Cow<'_, [u8]> {
    let layout = pixels.layout();
    match layout.format {
        PixelFormat::Rgba8 => Cow::Borrowed(pixels.bytes()),
        PixelFormat::Bgra8 => {
            let mut rgba = pixels.bytes().to_vec();
            let row_pixels = layout.size[0] as usize * 4;
            for row in rgba.chunks_mut(layout.row_bytes as usize) {
                for pixel in row[..row_pixels].as_chunks_mut::<4>().0 {
                    pixel.swap(0, 2);
                }
            }
            Cow::Owned(rgba)
        }
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::image_codecs::{AlphaType, PixelLayout};

    use super::*;

    #[test]
    fn bgra_pixels_are_read_as_rgba_and_rgba_as_they_are() {
        let layout = |format| PixelLayout::packed([2, 1], format, AlphaType::Straight);
        let bytes = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let bgra = PixelBuffer::new(layout(PixelFormat::Bgra8), bytes.clone()).unwrap();
        assert_eq!(&*rgba_of(&bgra), [3, 2, 1, 4, 7, 6, 5, 8]);
        let rgba = PixelBuffer::new(layout(PixelFormat::Rgba8), bytes.clone()).unwrap();
        assert!(matches!(rgba_of(&rgba), Cow::Borrowed(read) if read == bytes));
    }
}
