//! flutter_test's widget harness: [`WidgetTester`], the [`test_widgets`] entry, the
//! [`TargetPlatformVariant`]s a case runs over, the [`find`] finders with the `finds_*` matchers
//! [`expect`] checks them against, and, from `waft_rendering::testing`, the `moreOrLessEquals`
//! family. Behind the `test-support` feature, so a crate's tests above this one use the same
//! tester as this crate's own.
//!
//! The defaults are flutter_test's: an 800x600 logical view at a device pixel ratio of 3.0, on
//! Android, with every font family FlutterTest, whose glyphs are squares of the font size. A case
//! changes the view's metrics through [`WidgetTester::view`], as Dart's sets `tester.view.*`, and
//! the framework hears of each change at once. The
//! frame rules are flutter_test's too: [`WidgetTester::pump_widget`] attaches the widget,
//! schedules a frame and pumps once; [`WidgetTester::pump`] fires no timer, and
//! [`WidgetTester::pump_by`] lets time pass first; either runs a frame only of the views that
//! asked for one.
//!
//! Each case has its own `App`, so nothing a case sets (a debug flag, the clock, a singleton)
//! reaches another; flutter_test's resets between tests have no counterpart.

mod entry;
mod finder;
mod finders;
mod gestures;
mod matchers;
mod tester;
#[cfg(test)]
mod tests;

pub use entry::{TargetPlatformVariant, test_widgets, test_widgets_variant};
pub use finder::{Finder, FindsOne, RenderTarget, find};
pub use gestures::TestPointer;
pub use matchers::{
    FinderMatcher, expect, finds_at_least_n_widgets, finds_n_widgets, finds_nothing,
    finds_one_widget, finds_widgets,
};
pub use tester::{TestVSync, WidgetTester};
pub use waft_rendering::testing::{
    MoreOrLessEquals, PRECISION_ERROR_TOLERANCE, assert_f32_equals, assert_more_or_less_equals,
    assert_more_or_less_equals_within,
};
