//! A picture turned upright by the host, where Android's decoder leaves it as the file stores it:
//! `AImageDecoder` applies a JPEG's, a WebP's or a HEIF's orientation itself, but on some versions
//! not a PNG's, which the trait still promises upright, as Apple's host decodes it. The stored
//! picture is mirrored left to right where the orientation says, then turned clockwise.

use waft_embedder::image_codecs::{
    ImageError, ImageErrorKind, ImageOrientation, ImageRotation, PixelBuffer, PixelLayout,
};

/// The size the picture is stored at, for one shown upright at `upright` by `orientation`.
pub(super) fn stored_size(upright: [u32; 2], orientation: ImageOrientation) -> [u32; 2] {
    match orientation.rotation {
        ImageRotation::QuarterTurn | ImageRotation::ThreeQuarterTurn => [upright[1], upright[0]],
        ImageRotation::None | ImageRotation::HalfTurn => upright,
    }
}

/// `stored`, a picture as its file stores it, turned upright by `orientation`, in a buffer of
/// the same format and alpha, packed.
pub(super) fn upright(
    stored: &PixelBuffer,
    orientation: ImageOrientation,
) -> Result<PixelBuffer, ImageError> {
    let layout = stored.layout();
    let [width, height] = layout.size;
    let size = stored_size([width, height], orientation);
    let upright_layout = PixelLayout::packed(size, layout.format, layout.alpha);
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(size[0] as usize * size[1] as usize * 4)
        .map_err(|_| {
            ImageError::new(ImageErrorKind::OutOfMemory, "no memory to turn the picture")
        })?;
    for y in 0..size[1] {
        for x in 0..size[0] {
            let [from_x, from_y] = stored_position([x, y], [width, height], orientation);
            let start = (from_y * layout.row_bytes + from_x * 4) as usize;
            pixels.extend_from_slice(&stored.bytes()[start..start + 4]);
        }
    }
    PixelBuffer::new(upright_layout, pixels)
        .map_err(|error| ImageError::new(ImageErrorKind::Other, format!("{error:?}")))
}

/// Where the pixel at `upright` in the upright picture is in the stored one, `stored` in size.
fn stored_position(upright: [u32; 2], stored: [u32; 2], orientation: ImageOrientation) -> [u32; 2] {
    let [x, y] = upright;
    let [width, height] = stored;
    // Where it is in the stored picture mirrored, which is turned clockwise to show it.
    let [mirrored_x, mirrored_y] = match orientation.rotation {
        ImageRotation::None => [x, y],
        ImageRotation::QuarterTurn => [y, height - 1 - x],
        ImageRotation::HalfTurn => [width - 1 - x, height - 1 - y],
        ImageRotation::ThreeQuarterTurn => [width - 1 - y, x],
    };
    if orientation.mirrored {
        [width - 1 - mirrored_x, mirrored_y]
    } else {
        [mirrored_x, mirrored_y]
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::image_codecs::{AlphaType, PixelFormat};

    use super::*;

    /// A picture 3 by 2 whose pixels are numbered in reading order, each pixel's four bytes its
    /// number.
    fn numbered() -> PixelBuffer {
        let layout = PixelLayout::packed([3, 2], PixelFormat::Rgba8, AlphaType::Straight);
        let bytes = (0..6u8).flat_map(|number| [number; 4]).collect();
        PixelBuffer::new(layout, bytes).unwrap()
    }

    /// `pixels`' numbers, row by row.
    fn rows(pixels: &PixelBuffer) -> Vec<Vec<u8>> {
        let [width, _] = pixels.layout().size;
        pixels
            .bytes()
            .chunks(width as usize * 4)
            .map(|row| row.chunks(4).map(|pixel| pixel[0]).collect())
            .collect()
    }

    fn turned(mirrored: bool, rotation: ImageRotation) -> Vec<Vec<u8>> {
        let orientation = ImageOrientation { mirrored, rotation };
        rows(&upright(&numbered(), orientation).unwrap())
    }

    #[test]
    fn each_orientation_shows_the_stored_picture_as_exif_means_it() {
        // Stored:  0 1 2
        //          3 4 5
        assert_eq!(turned(false, ImageRotation::None), [[0, 1, 2], [3, 4, 5]]);
        assert_eq!(
            turned(true, ImageRotation::None),
            [[2, 1, 0], [5, 4, 3]],
            "EXIF 2"
        );
        assert_eq!(
            turned(false, ImageRotation::HalfTurn),
            [[5, 4, 3], [2, 1, 0]],
            "EXIF 3"
        );
        assert_eq!(
            turned(true, ImageRotation::HalfTurn),
            [[3, 4, 5], [0, 1, 2]],
            "EXIF 4"
        );
        assert_eq!(
            turned(false, ImageRotation::QuarterTurn),
            [[3, 0], [4, 1], [5, 2]],
            "EXIF 6: the left column on top"
        );
        assert_eq!(
            turned(false, ImageRotation::ThreeQuarterTurn),
            [[2, 5], [1, 4], [0, 3]],
            "EXIF 8"
        );
        assert_eq!(
            turned(true, ImageRotation::QuarterTurn),
            [[5, 2], [4, 1], [3, 0]],
            "EXIF 7"
        );
        assert_eq!(
            turned(true, ImageRotation::ThreeQuarterTurn),
            [[0, 3], [1, 4], [2, 5]],
            "EXIF 5: transposed"
        );
    }

    #[test]
    fn a_quarter_turn_stores_the_sides_swapped() {
        let quarter = ImageOrientation {
            mirrored: false,
            rotation: ImageRotation::QuarterTurn,
        };
        assert_eq!(stored_size([32, 48], quarter), [48, 32]);
        assert_eq!(stored_size([32, 48], ImageOrientation::default()), [32, 48]);
    }
}
