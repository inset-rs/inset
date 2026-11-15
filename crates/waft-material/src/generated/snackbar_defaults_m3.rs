//! Flutter counterpart: material_ui's `generated/snackbar_defaults_m3.g.dart`, a part of
//! `snack_bar.dart`.
//!
//! Dart's private `_SnackbarDefaultsM3` is a `SnackBarThemeData` whose getters read the
//! context's theme; [`snackbar_defaults_m3`] returns the data those getters answer, read
//! once from the theme.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::Radius;
use waft_foundation::App;
use waft_painting::{BorderRadiusGeometry, BorderSide, EdgeInsets, RoundedRectangleBorder};
use waft_widgets::{BuildContext, WidgetState, WidgetStateColor};

use crate::snack_bar_theme::{SnackBarBehavior, SnackBarThemeData};
use crate::theme::Theme;

/// Dart's `_SnackbarDefaultsM3(context)`.
pub(crate) fn snackbar_defaults_m3(app: &mut App, context: BuildContext) -> SnackBarThemeData {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;
    let inverse_primary = colors.inverse_primary();
    SnackBarThemeData::new()
        .background_color(colors.inverse_surface())
        .action_text_color(WidgetStateColor::resolve_with(move |states| {
            if states.contains(&WidgetState::Disabled) {
                return inverse_primary;
            }
            if states.contains(&WidgetState::Pressed) {
                return inverse_primary;
            }
            if states.contains(&WidgetState::Hovered) {
                return inverse_primary;
            }
            if states.contains(&WidgetState::Focused) {
                return inverse_primary;
            }
            inverse_primary
        }))
        .disabled_action_text_color(colors.inverse_primary())
        .content_text_style(
            Theme::of(app, context)
                .text_theme
                .body_medium
                .clone()
                .expect("a theme's text theme has a body medium style")
                .copy_with()
                .color(colors.on_inverse_surface()),
        )
        .elevation(6.0)
        .shape(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::all(Radius::circular(4.0)),
        ))
        .behavior(SnackBarBehavior::Fixed)
        .inset_padding(EdgeInsets::from_ltrb(15.0, 5.0, 15.0, 10.0))
        .show_close_icon(false)
        .close_icon_color(colors.on_inverse_surface())
        .action_overflow_threshold(0.25)
}
