//! Flutter counterpart: material_ui's `generated/expansion_tile_defaults_m3.g.dart`, a part of
//! `expansion_tile.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::Color;
use waft_foundation::App;
use waft_widgets::BuildContext;

use crate::expansion_tile_theme::{ExpansionTileThemeData, ExpansionTileThemeDataGetters};
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_ExpansionTileDefaultsM3`.
pub(crate) struct ExpansionTileDefaultsM3 {
    /// Dart's `super(..)` arguments.
    expansion_tile_theme_data: ExpansionTileThemeData,
    context: DefaultsContext,
}

impl ExpansionTileDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> ExpansionTileDefaultsM3 {
        ExpansionTileDefaultsM3 {
            expansion_tile_theme_data: ExpansionTileThemeData::new(),
            context: DefaultsContext::new(context),
        }
    }
}

impl ExpansionTileThemeDataGetters for ExpansionTileDefaultsM3 {
    fn expansion_tile_theme_data(&self) -> &ExpansionTileThemeData {
        &self.expansion_tile_theme_data
    }

    fn text_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).on_surface)
    }

    fn icon_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).primary)
    }

    fn collapsed_text_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).on_surface)
    }

    fn collapsed_icon_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).on_surface_variant())
    }
}
