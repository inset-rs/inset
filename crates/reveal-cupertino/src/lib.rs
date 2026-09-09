//! Flutter counterpart: `packages/flutter/lib/src/cupertino`.
//!
//! The iOS-style widgets: colors, theme, text theme, constants, the icons, and the button.
#![feature(arbitrary_self_types)]

mod activity_indicator;
mod adaptive_text_selection_toolbar;
mod app;
mod button;
mod colors;
mod constants;
mod debug;
mod desktop_text_selection;
mod desktop_text_selection_toolbar;
mod desktop_text_selection_toolbar_button;
mod dialog;
mod expansion_tile;
mod focus_halo;
mod form_row;
mod form_section;
mod icon_theme_data;
mod icons;
mod interface_level;
mod list_section;
mod list_tile;
mod localizations;
mod nav_bar;
mod page_scaffold;
mod route;
mod scrollbar;
mod segmented_control;
mod sheet;
#[cfg(test)]
mod test_support;
mod text_field;
mod text_selection;
mod text_selection_toolbar;
mod text_selection_toolbar_button;
mod text_theme;
mod theme;

pub use activity_indicator::*;
pub use adaptive_text_selection_toolbar::*;
pub use app::*;
pub use button::*;
pub use colors::*;
pub use constants::*;
pub use debug::*;
pub use desktop_text_selection::*;
pub use desktop_text_selection_toolbar::*;
pub use desktop_text_selection_toolbar_button::*;
pub use dialog::*;
pub use expansion_tile::*;
pub use focus_halo::*;
pub use form_row::*;
pub use form_section::*;
pub use icon_theme_data::*;
pub use icons::*;
pub use interface_level::*;
pub use list_section::*;
pub use list_tile::*;
pub use localizations::*;
pub use nav_bar::*;
pub use page_scaffold::*;
pub use route::*;
pub use scrollbar::*;
pub use segmented_control::*;
pub use sheet::*;
pub use text_field::*;
pub use text_selection::*;
pub use text_selection_toolbar::*;
pub use text_selection_toolbar_button::*;
pub use text_theme::*;
pub use theme::*;
