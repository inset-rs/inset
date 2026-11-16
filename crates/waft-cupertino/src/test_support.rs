//! What this crate's tests share beside `waft_widgets::testing`'s one tester.

use waft_embedder::Offset;

/// Dart's `expect(actual, expected)` on a position that went through a render object's
/// transform: `Matrix4` is valo's f32 matrix, so it carries f32 rounding that Dart's f64 has
/// not, and nothing more.
pub(crate) fn assert_near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= f64::from(f32::EPSILON) * expected.abs().max(1.0),
        "{actual} is not {expected}"
    );
}

/// [`assert_near`] on both coordinates of a global position.
pub(crate) fn assert_offset_near(actual: Offset, expected: Offset) {
    assert_near(actual.dx(), expected.dx());
    assert_near(actual.dy(), expected.dy());
}
