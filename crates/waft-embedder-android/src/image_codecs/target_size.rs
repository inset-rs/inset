//! The size a frame decodes at: the image's own, scaled down to fit the size the app asks for,
//! keeping its shape, never up, and at least a pixel a side.

use waft_embedder::Size;
use waft_embedder::image_codecs::{ImageError, ImageErrorKind};

/// `size`, in pixels, scaled down to fit `max_size` where given; why not, for a `max_size` of no
/// pixels.
pub(super) fn fitted(size: [u32; 2], max_size: Option<Size>) -> Result<[u32; 2], ImageError> {
    let [width, height] = size.map(f64::from);
    let scale = match max_size {
        Some(max) if !(max.width() >= 1.0 && max.height() >= 1.0) => {
            return Err(ImageError::new(
                ImageErrorKind::Other,
                format!("no image fits in {max:?}"),
            ));
        }
        Some(max) => (max.width() / width).min(max.height() / height).min(1.0),
        None => 1.0,
    };
    let side = |length: f64| ((length * scale).round() as u32).max(1);
    Ok([side(width), side(height)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_image_scales_down_to_fit_keeping_its_shape_and_never_up() {
        let fit = |size, max| fitted(size, max).unwrap();
        assert_eq!(fit([4000, 3000], Some(Size::new(400.0, 400.0))), [400, 300]);
        assert_eq!(fit([3000, 4000], Some(Size::new(400.0, 200.0))), [150, 200]);
        assert_eq!(fit([64, 48], Some(Size::new(400.0, 400.0))), [64, 48]);
        assert_eq!(fit([64, 48], None), [64, 48]);
        assert_eq!(fit([10000, 1], Some(Size::new(10.0, 10.0))), [10, 1]);
        let nothing = fitted([64, 48], Some(Size::new(0.0, 10.0)));
        assert_eq!(nothing.unwrap_err().kind, ImageErrorKind::Other);
    }
}
