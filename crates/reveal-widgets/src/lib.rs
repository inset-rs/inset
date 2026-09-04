//! Flutter counterpart: `packages/flutter/lib/src/widgets`.
//!
//! The widget framework: `framework.dart` (widgets, elements, state, the build owner).
//! The binding, `runApp`, and the widgets themselves follow.
#![feature(arbitrary_self_types)]

mod binding;
mod framework;
#[cfg(test)]
mod test_harness;
mod view;
mod widgets;

pub use binding::*;
pub use framework::*;
pub use view::*;
pub use widgets::basic::*;
pub use widgets::gesture_detector::*;
pub use widgets::icon_theme::*;
pub use widgets::icon_theme_data::*;
pub use widgets::image::*;
pub use widgets::media_query::*;
pub use widgets::text::*;
pub use widgets::ticker_provider::*;
pub use widgets::transitions::*;
pub use widgets::widget_state::*;
