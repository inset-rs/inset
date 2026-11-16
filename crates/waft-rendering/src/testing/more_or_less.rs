//! flutter_test's `moreOrLessEquals` family (`matchers.dart`): `moreOrLessEquals`,
//! `offsetMoreOrLessEquals`, `rectMoreOrLessEquals` and `matrixMoreOrLessEquals`, as one trait over
//! the values a case compares, and [`assert_f32_equals`] for Dart's exact `expect(a, b)` on a
//! value that went through valo's `f32`.

use std::fmt::Debug;

use waft_embedder::{Matrix4, Offset, Rect};

/// flutter's `precisionErrorTolerance`, the default `epsilon` of the `moreOrLessEquals` family.
pub const PRECISION_ERROR_TOLERANCE: f64 = 1e-10;

/// A value compared within an `epsilon`, as Dart's `moreOrLessEquals` matchers compare.
pub trait MoreOrLessEquals: Debug {
    /// Whether `self` is within `epsilon` of `expected`: each coordinate of an offset, each
    /// edge of a rect and each entry of a matrix.
    fn more_or_less_equals(&self, expected: &Self, epsilon: f64) -> bool;

    /// Whether `self` equals `expected` within `f32` rounding: each coordinate, edge or entry
    /// within `f32::EPSILON` relative to the expected value (absolute below one).
    fn equals_within_f32(&self, expected: &Self) -> bool;
}

/// `f32`'s rounding of `expected`: `f32::EPSILON` relative to it, absolute below one.
fn f32_rounding(expected: f64) -> f64 {
    f64::from(f32::EPSILON) * expected.abs().max(1.0)
}

impl MoreOrLessEquals for f64 {
    fn more_or_less_equals(&self, expected: &f64, epsilon: f64) -> bool {
        (self - expected).abs() <= epsilon
    }

    fn equals_within_f32(&self, expected: &f64) -> bool {
        self.more_or_less_equals(expected, f32_rounding(*expected))
    }
}

impl MoreOrLessEquals for Offset {
    fn more_or_less_equals(&self, expected: &Offset, epsilon: f64) -> bool {
        self.dx().more_or_less_equals(&expected.dx(), epsilon)
            && self.dy().more_or_less_equals(&expected.dy(), epsilon)
    }

    fn equals_within_f32(&self, expected: &Offset) -> bool {
        self.dx().equals_within_f32(&expected.dx()) && self.dy().equals_within_f32(&expected.dy())
    }
}

impl MoreOrLessEquals for Rect {
    fn more_or_less_equals(&self, expected: &Rect, epsilon: f64) -> bool {
        [
            (self.left, expected.left),
            (self.top, expected.top),
            (self.right, expected.right),
            (self.bottom, expected.bottom),
        ]
        .iter()
        .all(|(actual, expected)| actual.more_or_less_equals(expected, epsilon))
    }

    fn equals_within_f32(&self, expected: &Rect) -> bool {
        [
            (self.left, expected.left),
            (self.top, expected.top),
            (self.right, expected.right),
            (self.bottom, expected.bottom),
        ]
        .iter()
        .all(|(actual, expected)| actual.equals_within_f32(expected))
    }
}

/// Waft's `Matrix4` keeps `f32` entries where Dart's keeps `f64`, so a case comparing a matrix
/// usually passes an `epsilon` above f32's rounding.
impl MoreOrLessEquals for Matrix4 {
    fn more_or_less_equals(&self, expected: &Matrix4, epsilon: f64) -> bool {
        self.to_flutter_array()
            .iter()
            .zip(expected.to_flutter_array().iter())
            .all(|(actual, expected)| {
                f64::from(*actual).more_or_less_equals(&f64::from(*expected), epsilon)
            })
    }

    fn equals_within_f32(&self, expected: &Matrix4) -> bool {
        self.to_flutter_array()
            .iter()
            .zip(expected.to_flutter_array().iter())
            .all(|(actual, expected)| f64::from(*actual).equals_within_f32(&f64::from(*expected)))
    }
}

/// Dart's `expect(actual, moreOrLessEquals(expected))`, within
/// [`PRECISION_ERROR_TOLERANCE`].
#[track_caller]
pub fn assert_more_or_less_equals<T: MoreOrLessEquals>(actual: T, expected: T) {
    assert_more_or_less_equals_within(actual, expected, PRECISION_ERROR_TOLERANCE);
}

/// Dart's `expect(actual, moreOrLessEquals(expected, epsilon: epsilon))`.
#[track_caller]
pub fn assert_more_or_less_equals_within<T: MoreOrLessEquals>(
    actual: T,
    expected: T,
    epsilon: f64,
) {
    assert!(
        actual.more_or_less_equals(&expected, epsilon),
        "Expected: {expected:?} (±{epsilon})\n  Actual: {actual:?}"
    );
}

/// Dart's exact `expect(actual, expected)` on a value that went through valo's `f32` (a global
/// position, a transformed rect, a text width): equal within `f32` rounding, where Dart's `f64`
/// compares equal.
#[track_caller]
pub fn assert_f32_equals<T: MoreOrLessEquals>(actual: T, expected: T) {
    assert!(
        actual.equals_within_f32(&expected),
        "Expected: {expected:?} (within f32 rounding)\n  Actual: {actual:?}"
    );
}
