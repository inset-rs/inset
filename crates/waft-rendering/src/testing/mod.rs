//! flutter_test's paint matchers (`mock_canvas.dart`): the one `paints` matcher over the display
//! list a render object or a painter records ([`paints`]), and what paints a render object to
//! read it ([`pictures`], [`painted_ops`]). Behind the `test-support` feature, so the tests of a
//! crate above this one read paint as this crate's own do.

mod paints;
mod pictures;
#[cfg(test)]
mod tests;

pub use paints::{
    CanvasMethod, PaintCall, PaintMismatch, PaintedStyle, RecordedCall, SomethingPredicate,
    assert_paints, colors_match, count_calls, draws, paints, paints_exactly,
};
pub use pictures::{painted_ops, painted_ops_deep, pictures, pictures_deep};
