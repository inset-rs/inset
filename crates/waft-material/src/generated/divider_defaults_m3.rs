//! Flutter counterpart: material_ui's `generated/divider_defaults_m3.g.dart`, a part of
//! `divider.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::Color;
use waft_foundation::App;
use waft_widgets::BuildContext;

use crate::divider_theme::{DividerThemeData, DividerThemeDataGetters};
use crate::theme::Theme;

/// Dart's `_DividerDefaultsM3`.
pub(crate) struct DividerDefaultsM3 {
    /// Dart's `super(..)` arguments.
    divider_theme_data: DividerThemeData,
    context: BuildContext,
}

impl DividerDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> DividerDefaultsM3 {
        DividerDefaultsM3 {
            divider_theme_data: DividerThemeData::new()
                .space(16.0)
                .thickness(1.0)
                .indent(0.0)
                .end_indent(0.0),
            context,
        }
    }
}

impl DividerThemeDataGetters for DividerDefaultsM3 {
    fn divider_theme_data(&self) -> &DividerThemeData {
        &self.divider_theme_data
    }

    fn color(&self, app: &mut App) -> Option<Color> {
        Some(Theme::of(app, self.context).color_scheme.outline_variant())
    }
}
