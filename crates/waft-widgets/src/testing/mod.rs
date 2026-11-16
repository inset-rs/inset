//! flutter_test's widget harness: [`WidgetTester`], the [`test_widgets`] entry, and the
//! [`TargetPlatformVariant`]s a case runs over. Behind the `test-support` feature, so a crate's
//! tests above this one use the same tester as this crate's own.
//!
//! The defaults are flutter_test's: an 800x600 logical view at a device pixel ratio of 3.0, on
//! Android, with every font family FlutterTest, whose glyphs are squares of the font size. The
//! frame rules are flutter_test's too: [`WidgetTester::pump_widget`] attaches the widget,
//! schedules a frame and pumps once; [`WidgetTester::pump`] fires no timer, and
//! [`WidgetTester::pump_by`] lets time pass first; either runs a frame only of the views that
//! asked for one.
//!
//! Each case has its own `App`, so nothing a case sets (a debug flag, the clock, a singleton)
//! reaches another; flutter_test's resets between tests have no counterpart.

mod entry;
mod finders;
mod gestures;
mod tester;
#[cfg(test)]
mod tests;

pub use entry::{TargetPlatformVariant, test_widgets, test_widgets_variant};
pub use gestures::TestPointer;
pub use tester::{TestVSync, WidgetTester};
