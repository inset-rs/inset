//! Flutter counterpart: `Matrix4.transform3` (package:vector_math).

use crate::{Matrix4, Offset};

/// Transforms a point as a 3-vector, with no perspective division.
///
/// Flutter counterpart: `Matrix4.transform3` applied to `Vector3(dx, dy, 0)`.
/// It takes the transformed x and y and ignores the resulting w.
///
/// This is not [`Matrix4::map_point`], which divides by w.
pub fn transform3(matrix: Matrix4, point: Offset) -> Offset {
    let m = matrix.to_flutter_array();
    let (x, y) = (point.dx() as f32, point.dy() as f32);
    Offset::new(
        (m[0] * x + m[4] * y + m[12]) as f64,
        (m[1] * x + m[5] * y + m[13]) as f64,
    )
}
