//! Flutter counterpart: material_ui's `generated/banner_defaults_m3.g.dart`, a part of
//! `banner.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::Color;
use waft_foundation::App;
use waft_painting::TextStyle;
use waft_widgets::BuildContext;

use crate::banner_theme::{MaterialBannerThemeData, MaterialBannerThemeDataGetters};
use crate::colors::Colors;
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_BannerDefaultsM3`.
pub(crate) struct BannerDefaultsM3 {
    /// Dart's `super(..)` arguments.
    material_banner_theme_data: MaterialBannerThemeData,
    context: DefaultsContext,
}

impl BannerDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> BannerDefaultsM3 {
        BannerDefaultsM3 {
            material_banner_theme_data: MaterialBannerThemeData::new().elevation(1.0),
            context: DefaultsContext::new(context),
        }
    }
}

impl MaterialBannerThemeDataGetters for BannerDefaultsM3 {
    fn material_banner_theme_data(&self) -> &MaterialBannerThemeData {
        &self.material_banner_theme_data
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).surface_container_low())
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn divider_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).outline_variant())
    }

    fn content_text_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).body_medium.clone()
    }
}
