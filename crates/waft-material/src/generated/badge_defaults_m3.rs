//! Flutter counterpart: material_ui's `generated/badge_defaults_m3.g.dart`, a part of
//! `badge.dart`.
//!
//! Dart's private `_BadgeDefaultsM3` is a `BadgeThemeData` whose getters read the context's
//! theme; [`badge_defaults_m3`] returns the data those getters answer, read once from the theme.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_foundation::App;
use waft_painting::{AlignmentGeometry, EdgeInsetsGeometry};
use waft_widgets::BuildContext;

use crate::badge_theme::BadgeThemeData;
use crate::theme::Theme;

/// Dart's `_BadgeDefaultsM3(context)`.
pub(crate) fn badge_defaults_m3(app: &mut App, context: BuildContext) -> BadgeThemeData {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;
    let mut defaults = BadgeThemeData::new()
        .small_size(6.0)
        .large_size(16.0)
        .padding(EdgeInsetsGeometry::symmetric(0.0, 4.0))
        .alignment(AlignmentGeometry::TOP_END)
        .background_color(colors.error)
        .text_color(colors.on_error);
    defaults.text_style = theme.text_theme.label_small.clone();
    defaults
}
