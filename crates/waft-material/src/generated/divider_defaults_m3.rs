//! Flutter counterpart: material_ui's `generated/divider_defaults_m3.g.dart`, a part of
//! `divider.dart`.
//!
//! Dart's private `_DividerDefaultsM3` is a `DividerThemeData` whose `color` getter reads
//! the context's theme; [`divider_defaults_m3`] returns the data that getter answers, read
//! once from the theme.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_foundation::App;
use waft_widgets::BuildContext;

use crate::divider_theme::DividerThemeData;
use crate::theme::Theme;

/// Dart's `_DividerDefaultsM3(context)`.
pub(crate) fn divider_defaults_m3(app: &mut App, context: BuildContext) -> DividerThemeData {
    DividerThemeData::new()
        .space(16.0)
        .thickness(1.0)
        .indent(0.0)
        .end_indent(0.0)
        .color(Theme::of(app, context).color_scheme.outline_variant())
}
