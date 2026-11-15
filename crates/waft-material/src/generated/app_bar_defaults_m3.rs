//! Flutter counterpart: material_ui's `generated/app_bar_defaults_m3.g.dart`, a Dart `part` of
//! `app_bar.dart`.
//!
//! Dart's private `_AppBarDefaultsM3` is an `AppBarThemeData` whose getters read the context's
//! theme; [`app_bar_defaults_m3`] returns the data those getters answer, read once from the
//! theme. The two variant configurations keep the colour scheme and text theme Dart's late
//! fields read.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_foundation::App;
use waft_painting::{EdgeInsets, EdgeInsetsGeometry, TextStyle};
use waft_widgets::{BuildContext, IconThemeData, NavigationToolbar};

use crate::app_bar::ScrollUnderFlexibleConfig;
use crate::app_bar_theme::AppBarThemeData;
use crate::color_scheme::ColorScheme;
use crate::colors::Colors;
use crate::text_theme::TextTheme;
use crate::theme::Theme;

/// Dart's `_AppBarDefaultsM3(context)`.
pub(crate) fn app_bar_defaults_m3(app: &mut App, context: BuildContext) -> AppBarThemeData {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;
    let text_theme = &theme.text_theme;
    AppBarThemeData::new()
        .elevation(0.0)
        .scrolled_under_elevation(3.0)
        .title_spacing(NavigationToolbar::K_MIDDLE_SPACING)
        .toolbar_height(64.0)
        .background_color(colors.surface)
        .foreground_color(colors.on_surface)
        .shadow_color(Colors::TRANSPARENT)
        .surface_tint_color(Colors::TRANSPARENT)
        .icon_theme(IconThemeData::new().color(colors.on_surface).size(24.0))
        .actions_icon_theme(
            IconThemeData::new()
                .color(colors.on_surface_variant())
                .size(24.0),
        )
        .toolbar_text_style(
            text_theme
                .body_medium
                .clone()
                .expect("a theme's text theme has a body medium style"),
        )
        .title_text_style(
            text_theme
                .title_large
                .clone()
                .expect("a theme's text theme has a title large style"),
        )
        // TODO(Craftplacer): Consider using EdgeInsets.only(right: 8.0) instead of
        // EdgeInsets.zero for Material 3 in the future,
        // https://github.com/flutter/flutter/issues/155747
        .actions_padding(EdgeInsets::ZERO.into())
}

// Variant configuration

/// Dart's `_MediumScrollUnderFlexibleConfig`.
pub(crate) struct MediumScrollUnderFlexibleConfig {
    colors: ColorScheme,
    text_theme: TextTheme,
}

impl MediumScrollUnderFlexibleConfig {
    pub(crate) const COLLAPSED_HEIGHT: f64 = 64.0;
    pub(crate) const EXPANDED_HEIGHT: f64 = 112.0;

    /// Dart's `_MediumScrollUnderFlexibleConfig(context)`.
    pub(crate) fn new(app: &mut App, context: BuildContext) -> MediumScrollUnderFlexibleConfig {
        let theme = Theme::of(app, context);
        MediumScrollUnderFlexibleConfig {
            colors: theme.color_scheme,
            text_theme: theme.text_theme,
        }
    }
}

impl ScrollUnderFlexibleConfig for MediumScrollUnderFlexibleConfig {
    fn collapsed_text_style(&self) -> Option<TextStyle> {
        self.text_theme
            .title_large
            .as_ref()
            .map(|style| style.apply().color(self.colors.on_surface).into_style())
    }

    fn expanded_text_style(&self) -> Option<TextStyle> {
        self.text_theme
            .headline_small
            .as_ref()
            .map(|style| style.apply().color(self.colors.on_surface).into_style())
    }

    fn expanded_title_padding(&self) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::from_ltrb(16.0, 0.0, 16.0, 20.0)
    }
}

/// Dart's `_LargeScrollUnderFlexibleConfig`.
pub(crate) struct LargeScrollUnderFlexibleConfig {
    colors: ColorScheme,
    text_theme: TextTheme,
}

impl LargeScrollUnderFlexibleConfig {
    pub(crate) const COLLAPSED_HEIGHT: f64 = 64.0;
    pub(crate) const EXPANDED_HEIGHT: f64 = 152.0;

    /// Dart's `_LargeScrollUnderFlexibleConfig(context)`.
    pub(crate) fn new(app: &mut App, context: BuildContext) -> LargeScrollUnderFlexibleConfig {
        let theme = Theme::of(app, context);
        LargeScrollUnderFlexibleConfig {
            colors: theme.color_scheme,
            text_theme: theme.text_theme,
        }
    }
}

impl ScrollUnderFlexibleConfig for LargeScrollUnderFlexibleConfig {
    fn collapsed_text_style(&self) -> Option<TextStyle> {
        self.text_theme
            .title_large
            .as_ref()
            .map(|style| style.apply().color(self.colors.on_surface).into_style())
    }

    fn expanded_text_style(&self) -> Option<TextStyle> {
        self.text_theme
            .headline_medium
            .as_ref()
            .map(|style| style.apply().color(self.colors.on_surface).into_style())
    }

    fn expanded_title_padding(&self) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::from_ltrb(16.0, 0.0, 16.0, 28.0)
    }
}
