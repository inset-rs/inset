//! Flutter counterpart: `packages/flutter/lib/src/cupertino`.
//!
//! The iOS-style widgets: colors, theme, text theme, constants, and the button.
#![feature(arbitrary_self_types)]

mod button;
mod colors;
mod constants;
mod icon_theme_data;
mod interface_level;
#[cfg(test)]
mod test_support;
mod text_theme;
mod theme;

pub use button::*;
pub use colors::*;
pub use constants::*;
pub use icon_theme_data::*;
pub use interface_level::*;
pub use text_theme::*;
pub use theme::*;
