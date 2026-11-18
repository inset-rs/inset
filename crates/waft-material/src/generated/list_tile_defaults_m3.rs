//! Flutter counterpart: material_ui's `generated/list_tile_defaults_m3.g.dart`, a part of
//! `list_tile.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::Color;
use waft_foundation::App;
use waft_painting::{AnyColor, EdgeInsetsDirectional, RoundedRectangleBorder, TextStyle};
use waft_widgets::BuildContext;

use crate::colors::Colors;
use crate::list_tile_theme::{ListTileThemeData, ListTileThemeDataGetters};
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_ListTileDefaultsM3`.
pub(crate) struct ListTileDefaultsM3 {
    /// Dart's `super(..)` arguments.
    list_tile_theme_data: ListTileThemeData,
    context: DefaultsContext,
}

impl ListTileDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> ListTileDefaultsM3 {
        ListTileDefaultsM3 {
            list_tile_theme_data: ListTileThemeData::new()
                .content_padding(EdgeInsetsDirectional::only(16.0, 0.0, 24.0, 0.0))
                .min_leading_width(24.0)
                .min_vertical_padding(8.0)
                .shape(RoundedRectangleBorder::default()),
            context: DefaultsContext::new(context),
        }
    }
}

impl ListTileThemeDataGetters for ListTileDefaultsM3 {
    fn list_tile_theme_data(&self) -> &ListTileThemeData {
        &self.list_tile_theme_data
    }

    fn tile_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn title_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let on_surface = self.context.colors(app).on_surface();
        Some(
            self.context
                .text_theme(app)
                .body_large
                .as_ref()
                .expect("the text theme has a body large style")
                .copy_with()
                .color(on_surface),
        )
    }

    fn subtitle_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let on_surface_variant = self.context.colors(app).on_surface_variant();
        Some(
            self.context
                .text_theme(app)
                .body_medium
                .as_ref()
                .expect("the text theme has a body medium style")
                .copy_with()
                .color(on_surface_variant),
        )
    }

    fn leading_and_trailing_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let on_surface_variant = self.context.colors(app).on_surface_variant();
        Some(
            self.context
                .text_theme(app)
                .label_small
                .as_ref()
                .expect("the text theme has a label small style")
                .copy_with()
                .color(on_surface_variant),
        )
    }

    fn selected_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).primary())
    }

    fn icon_color(&self, app: &mut App) -> Option<AnyColor> {
        Some(AnyColor::new(self.context.colors(app).on_surface_variant()))
    }
}
