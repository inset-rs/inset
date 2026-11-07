//! Flutter counterpart: the `package:vector_math` surface the framework uses on
//! `Matrix4` — `transform3`, `decompose` and `compose`, with the `Vector3` and
//! `Quaternion` those take.

use std::ops::{Add, Mul};

use crate::{Matrix4, Offset};

/// A three-dimensional vector.
///
/// Flutter counterpart: `Vector3` (package:vector_math), narrowed to the `f32` the
/// host's [`Matrix4`] stores.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector3 {
    /// The first component.
    pub x: f32,
    /// The second component.
    pub y: f32,
    /// The third component.
    pub z: f32,
}

impl Vector3 {
    /// Constructs a new vector.
    pub const fn new(x: f32, y: f32, z: f32) -> Vector3 {
        Vector3 { x, y, z }
    }

    /// The zero vector.
    pub const ZERO: Vector3 = Vector3::new(0.0, 0.0, 0.0);

    /// The length of the vector.
    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
}

impl Add for Vector3 {
    type Output = Vector3;

    fn add(self, other: Vector3) -> Vector3 {
        Vector3::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl Mul<f32> for Vector3 {
    type Output = Vector3;

    fn mul(self, scale: f32) -> Vector3 {
        Vector3::new(self.x * scale, self.y * scale, self.z * scale)
    }
}

/// A rotation as a unit quaternion.
///
/// Flutter counterpart: `Quaternion` (package:vector_math), narrowed to the `f32` the
/// host's [`Matrix4`] stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quaternion {
    /// The first imaginary component.
    pub x: f32,
    /// The second imaginary component.
    pub y: f32,
    /// The third imaginary component.
    pub z: f32,
    /// The real component.
    pub w: f32,
}

impl Quaternion {
    /// Constructs a quaternion from its four components.
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Quaternion {
        Quaternion { x, y, z, w }
    }

    /// The rotation that changes nothing.
    pub const IDENTITY: Quaternion = Quaternion::new(0.0, 0.0, 0.0, 1.0);

    /// The norm of the quaternion.
    pub fn length(&self) -> f32 {
        self.length2().sqrt()
    }

    /// The squared norm of the quaternion.
    pub fn length2(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w
    }

    /// A copy of this scaled by `scale`.
    pub fn scaled(&self, scale: f32) -> Quaternion {
        Quaternion::new(
            self.x * scale,
            self.y * scale,
            self.z * scale,
            self.w * scale,
        )
    }

    /// A copy of this with unit length.
    ///
    /// A zero-length quaternion is returned unchanged, as `Quaternion.normalize`
    /// leaves it.
    pub fn normalized(&self) -> Quaternion {
        let length = self.length();
        if length == 0.0 {
            return *self;
        }
        self.scaled(1.0 / length)
    }
}

impl Default for Quaternion {
    fn default() -> Quaternion {
        Quaternion::IDENTITY
    }
}

impl Add for Quaternion {
    type Output = Quaternion;

    fn add(self, other: Quaternion) -> Quaternion {
        Quaternion::new(
            self.x + other.x,
            self.y + other.y,
            self.z + other.z,
            self.w + other.w,
        )
    }
}

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

/// Decomposes `matrix` into its translation, rotation and scale components.
///
/// Flutter counterpart: `Matrix4.decompose(translation, rotation, scale)`, which
/// fills three out parameters.
pub fn decompose(matrix: Matrix4) -> (Vector3, Quaternion, Vector3) {
    let m = matrix.to_flutter_array();
    let mut sx = Vector3::new(m[0], m[1], m[2]).length();
    let sy = Vector3::new(m[4], m[5], m[6]).length();
    let sz = Vector3::new(m[8], m[9], m[10]).length();

    if determinant(&m) < 0.0 {
        sx = -sx;
    }

    let translation = Vector3::new(m[12], m[13], m[14]);

    let inv_sx = 1.0 / sx;
    let inv_sy = 1.0 / sy;
    let inv_sz = 1.0 / sz;

    // `Matrix4.copyRotation` over the scale-normalized matrix, into column-major
    // 3×3 storage.
    let rotation_matrix = [
        m[0] * inv_sx,
        m[1] * inv_sx,
        m[2] * inv_sx,
        m[4] * inv_sy,
        m[5] * inv_sy,
        m[6] * inv_sy,
        m[8] * inv_sz,
        m[9] * inv_sz,
        m[10] * inv_sz,
    ];
    let rotation = quaternion_from_rotation(&rotation_matrix);

    (translation, rotation, Vector3::new(sx, sy, sz))
}

/// Composes a matrix from `translation`, `rotation` and `scale`.
///
/// Flutter counterpart: `Matrix4.compose`, which is
/// `setFromTranslationRotation` followed by `scaleByVector3`.
pub fn compose(translation: Vector3, rotation: Quaternion, scale: Vector3) -> Matrix4 {
    let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);
    let (x2, y2, z2) = (x + x, y + y, z + z);
    let (xx, xy, xz) = (x * x2, x * y2, x * z2);
    let (yy, yz, zz) = (y * y2, y * z2, z * z2);
    let (wx, wy, wz) = (w * x2, w * y2, w * z2);

    let mut m = [
        1.0 - (yy + zz),
        xy + wz,
        xz - wy,
        0.0,
        xy - wz,
        1.0 - (xx + zz),
        yz + wx,
        0.0,
        xz + wy,
        yz - wx,
        1.0 - (xx + yy),
        0.0,
        translation.x,
        translation.y,
        translation.z,
        1.0,
    ];

    // `Matrix4.scaleByVector3`, which scales the fourth column by 1.0.
    for (column, factor) in [scale.x, scale.y, scale.z, 1.0].into_iter().enumerate() {
        for row in 0..4 {
            m[column * 4 + row] *= factor;
        }
    }

    Matrix4::from_flutter_array(&m)
}

/// `Matrix4.determinant`, over column-major storage.
fn determinant(m: &[f32; 16]) -> f32 {
    let det2_01_01 = m[0] * m[5] - m[1] * m[4];
    let det2_01_02 = m[0] * m[6] - m[2] * m[4];
    let det2_01_03 = m[0] * m[7] - m[3] * m[4];
    let det2_01_12 = m[1] * m[6] - m[2] * m[5];
    let det2_01_13 = m[1] * m[7] - m[3] * m[5];
    let det2_01_23 = m[2] * m[7] - m[3] * m[6];
    let det3_201_012 = m[8] * det2_01_12 - m[9] * det2_01_02 + m[10] * det2_01_01;
    let det3_201_013 = m[8] * det2_01_13 - m[9] * det2_01_03 + m[11] * det2_01_01;
    let det3_201_023 = m[8] * det2_01_23 - m[10] * det2_01_03 + m[11] * det2_01_02;
    let det3_201_123 = m[9] * det2_01_23 - m[10] * det2_01_13 + m[11] * det2_01_12;
    -det3_201_123 * m[12] + det3_201_023 * m[13] - det3_201_013 * m[14] + det3_201_012 * m[15]
}

/// `Quaternion.setFromRotation`, over column-major 3×3 storage.
fn quaternion_from_rotation(m: &[f32; 9]) -> Quaternion {
    /// `Matrix3.index`.
    fn index(row: usize, col: usize) -> usize {
        col * 3 + row
    }

    let trace = m[0] + m[4] + m[8];
    let mut q = [0.0f32; 4];
    if trace > 0.0 {
        let mut s = (trace + 1.0).sqrt();
        q[3] = s * 0.5;
        s = 0.5 / s;
        q[0] = (m[5] - m[7]) * s;
        q[1] = (m[6] - m[2]) * s;
        q[2] = (m[1] - m[3]) * s;
    } else {
        let i = if m[0] < m[4] {
            if m[4] < m[8] { 2 } else { 1 }
        } else if m[0] < m[8] {
            2
        } else {
            0
        };
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        let mut s = (m[index(i, i)] - m[index(j, j)] - m[index(k, k)] + 1.0).sqrt();
        q[i] = s * 0.5;
        s = 0.5 / s;
        q[3] = (m[index(k, j)] - m[index(j, k)]) * s;
        q[j] = (m[index(j, i)] + m[index(i, j)]) * s;
        q[k] = (m[index(k, i)] + m[index(i, k)]) * s;
    }
    Quaternion::new(q[0], q[1], q[2], q[3])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: Matrix4, b: Matrix4) {
        let (a, b) = (a.to_flutter_array(), b.to_flutter_array());
        for (index, (a, b)) in a.iter().zip(b.iter()).enumerate() {
            assert!((a - b).abs() < 1e-5, "entry {index}: {a} vs {b}");
        }
    }

    #[test]
    fn decompose_reads_back_a_translation_and_a_scale() {
        let matrix = Matrix4::translation(3.0, 4.0).then(&Matrix4::scale(2.0, 5.0));
        let (translation, rotation, scale) = decompose(matrix);
        assert_eq!(translation, Vector3::new(3.0, 4.0, 0.0));
        assert_eq!(rotation, Quaternion::IDENTITY);
        assert_eq!(scale, Vector3::new(2.0, 5.0, 1.0));
    }

    #[test]
    fn compose_inverts_decompose() {
        let matrix = Matrix4::translation(3.0, 4.0)
            .then(&Matrix4::rotation(0.7))
            .then(&Matrix4::scale(2.0, 5.0));
        let (translation, rotation, scale) = decompose(matrix);
        assert_close(compose(translation, rotation, scale), matrix);
    }

    #[test]
    fn a_rotation_round_trips_through_a_quaternion() {
        let matrix = Matrix4::rotation(std::f32::consts::FRAC_PI_3);
        let (translation, rotation, scale) = decompose(matrix);
        assert!((rotation.length() - 1.0).abs() < 1e-6);
        assert_close(compose(translation, rotation, scale), matrix);
    }

    #[test]
    fn a_mirrored_matrix_decomposes_with_a_negative_x_scale() {
        let matrix = Matrix4::scale(-2.0, 3.0);
        let (_, _, scale) = decompose(matrix);
        assert_eq!(scale.x, -2.0);
        assert_eq!(scale.y, 3.0);
    }
}
