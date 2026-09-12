//! Flutter counterpart: `painting/matrix_utils.dart`.

use inset_embedder::{Matrix4, Offset, Rect};

/// Returns the transform as an [`Offset`] if it is only a 2D translation.
pub fn get_as_translation(transform: Matrix4) -> Option<Offset> {
    let m = transform.to_flutter_array();
    if m[0] == 1.0
        && m[1] == 0.0
        && m[2] == 0.0
        && m[3] == 0.0
        && m[4] == 0.0
        && m[5] == 1.0
        && m[6] == 0.0
        && m[7] == 0.0
        && m[8] == 0.0
        && m[9] == 0.0
        && m[10] == 1.0
        && m[11] == 0.0
        && m[14] == 0.0
        && m[15] == 1.0
    {
        Some(Offset::new(m[12] as f64, m[13] as f64))
    } else {
        None
    }
}

/// Returns a uniform 2D scale if the matrix is only that scale.
pub fn get_as_scale(transform: Matrix4) -> Option<f64> {
    let m = transform.to_flutter_array();
    if m[0] == m[5]
        && m[1] == 0.0
        && m[2] == 0.0
        && m[3] == 0.0
        && m[4] == 0.0
        && m[6] == 0.0
        && m[7] == 0.0
        && m[8] == 0.0
        && m[9] == 0.0
        && m[10] == 1.0
        && m[11] == 0.0
        && m[12] == 0.0
        && m[13] == 0.0
        && m[14] == 0.0
        && m[15] == 1.0
    {
        Some(m[0] as f64)
    } else {
        None
    }
}

/// Flutter `MatrixUtils.transformPoint`.
pub fn transform_point(transform: &Matrix4, point: Offset) -> Offset {
    let storage = transform.to_flutter_array();
    let x = point.dx();
    let y = point.dy();
    let rx = storage[0] as f64 * x + storage[4] as f64 * y + storage[12] as f64;
    let ry = storage[1] as f64 * x + storage[5] as f64 * y + storage[13] as f64;
    let rw = storage[3] as f64 * x + storage[7] as f64 * y + storage[15] as f64;
    if rw == 1.0 {
        Offset::new(rx, ry)
    } else {
        Offset::new(rx / rw, ry / rw)
    }
}

/// Flutter `MatrixUtils.transformRect`: the bounding box of the four
/// transformed corners. Flutter's translation/scale/affine fast paths are
/// skipped; they compute the same numbers.
pub fn transform_rect(transform: &Matrix4, rect: Rect) -> Rect {
    let mut left = f64::INFINITY;
    let mut top = f64::INFINITY;
    let mut right = f64::NEG_INFINITY;
    let mut bottom = f64::NEG_INFINITY;
    for corner in [
        Offset::new(rect.left, rect.top),
        Offset::new(rect.right, rect.top),
        Offset::new(rect.left, rect.bottom),
        Offset::new(rect.right, rect.bottom),
    ] {
        let mapped = transform_point(transform, corner);
        left = left.min(mapped.dx());
        top = top.min(mapped.dy());
        right = right.max(mapped.dx());
        bottom = bottom.max(mapped.dy());
    }
    Rect::from_ltrb(left, top, right, bottom)
}

/// Flutter `MatrixUtils.inverseTransformRect`.
pub fn inverse_transform_rect(transform: Matrix4, rect: Rect) -> Rect {
    if transform == Matrix4::IDENTITY {
        return rect;
    }
    let inverted = transform
        .invert()
        .unwrap_or_else(|| Matrix4::from_flutter_array(&[0.0; 16]));
    transform_rect(&inverted, rect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_leaves_a_rect_alone() {
        let rect = Rect::from_ltwh(3.0, 5.0, 10.0, 20.0);
        assert_eq!(transform_rect(&Matrix4::IDENTITY, rect), rect);
        assert_eq!(get_as_translation(Matrix4::IDENTITY), Some(Offset::ZERO));
        assert_eq!(get_as_scale(Matrix4::IDENTITY), Some(1.0));
    }

    #[test]
    fn translation_shifts_points_and_rects() {
        let transform = Matrix4::translation(7.0, -2.0);
        assert_eq!(get_as_translation(transform), Some(Offset::new(7.0, -2.0)));
        assert_eq!(
            transform_point(&transform, Offset::new(1.0, 1.0)),
            Offset::new(8.0, -1.0)
        );
        assert_eq!(
            transform_rect(&transform, Rect::from_ltwh(0.0, 0.0, 4.0, 4.0)),
            Rect::from_ltwh(7.0, -2.0, 4.0, 4.0)
        );
    }

    #[test]
    fn scale_grows_the_bounding_box_from_the_origin() {
        let transform = Matrix4::scale(2.0, 3.0);
        assert_eq!(get_as_scale(transform), None);
        assert_eq!(
            transform_rect(&transform, Rect::from_ltwh(1.0, 1.0, 2.0, 2.0)),
            Rect::from_ltrb(2.0, 3.0, 6.0, 9.0)
        );
        assert_eq!(get_as_scale(Matrix4::scale(2.0, 2.0)), Some(2.0));
    }
}
