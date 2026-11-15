//! Flutter counterpart: material_ui's `generated/banner_defaults_m3.g.dart`, a part of
//! `banner.dart`.
//!
//! Dart's private `_BannerDefaultsM3` is a `MaterialBannerThemeData` whose getters read the
//! context's theme; [`banner_defaults_m3`] returns the data those getters answer, read once
//! from the theme.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_foundation::App;
use waft_widgets::BuildContext;

use crate::banner_theme::MaterialBannerThemeData;
use crate::colors::Colors;
use crate::theme::Theme;

/// Dart's `_BannerDefaultsM3(context)`.
pub(crate) fn banner_defaults_m3(app: &mut App, context: BuildContext) -> MaterialBannerThemeData {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;
    let text_theme = &theme.text_theme;
    let mut defaults = MaterialBannerThemeData::new()
        .elevation(1.0)
        .background_color(colors.surface_container_low())
        .surface_tint_color(Colors::TRANSPARENT)
        .divider_color(colors.outline_variant());
    defaults.content_text_style = text_theme.body_medium.clone();
    defaults
}
