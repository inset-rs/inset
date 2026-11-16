//! flutter_test's paint matchers (`mock_canvas.dart`), its `moreOrLessEquals` family, and
//! flutter's `rendering_tester.dart`: the one `paints` matcher over the display
//! list a render object or a painter records ([`paints`]), and what paints a render object to
//! read it ([`pictures`], [`painted_ops`]). Behind the `test-support` feature, so the tests of a
//! crate above this one read paint as this crate's own do.

mod more_or_less;
mod paints;
mod pictures;
mod rendering_tester;
#[cfg(test)]
mod tests;

pub use more_or_less::{
    MoreOrLessEquals, PRECISION_ERROR_TOLERANCE, assert_f32_equals, assert_more_or_less_equals,
    assert_more_or_less_equals_within,
};
pub use paints::{
    CanvasMethod, PaintCall, PaintMismatch, PaintedStyle, RecordedCall, SomethingPredicate,
    assert_paints, colors_match, count_calls, draws, paints, paints_exactly,
};
pub use pictures::{painted_ops, painted_ops_deep, pictures, pictures_deep};
pub use rendering_tester::{
    EnginePhase, TEST_VIEW_SIZE, layout, layout_with, pump_frame, test_pipeline_owner,
    test_render_root,
};
