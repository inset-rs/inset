//! Flutter counterpart: material_ui's `generated/app_bar_defaults_m3.g.dart`, a part of
//! `app_bar.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_foundation::App;
use waft_painting::{EdgeInsets, EdgeInsetsGeometry, TextStyle};
use waft_widgets::{BuildContext, IconThemeData, NavigationToolbar};

use crate::app_bar::ScrollUnderFlexibleConfig;
use crate::app_bar_theme::{AppBarThemeData, AppBarThemeDataGetters};
use crate::color_scheme::ColorScheme;
use crate::colors::Colors;
use crate::text_theme::TextTheme;
use crate::theme::Theme;
use crate::theme_data_getters::DefaultsContext;
use waft_embedder::Color;

/// Dart's `_AppBarDefaultsM3`.
pub(crate) struct AppBarDefaultsM3 {
    /// Dart's `super(..)` arguments.
    app_bar_theme_data: AppBarThemeData,
    context: DefaultsContext,
}

impl AppBarDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> AppBarDefaultsM3 {
        AppBarDefaultsM3 {
            app_bar_theme_data: AppBarThemeData::new()
                .elevation(0.0)
                .scrolled_under_elevation(3.0)
                .title_spacing(NavigationToolbar::K_MIDDLE_SPACING)
                .toolbar_height(64.0),
            context: DefaultsContext::new(context),
        }
    }
}

impl AppBarThemeDataGetters for AppBarDefaultsM3 {
    fn app_bar_theme_data(&self) -> &AppBarThemeData {
        &self.app_bar_theme_data
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.surface())
    }

    fn foreground_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface())
    }

    fn shadow_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn icon_theme(&self, app: &mut App) -> Option<IconThemeData> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(IconThemeData::new().color(colors.on_surface()).size(24.0))
    }

    fn actions_icon_theme(&self, app: &mut App) -> Option<IconThemeData> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(
            IconThemeData::new()
                .color(colors.on_surface_variant())
                .size(24.0),
        )
    }

    fn toolbar_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let theme = self.context.theme(app);
        let text_theme = &theme.text_theme;
        Some(
            text_theme
                .body_medium
                .clone()
                .expect("a theme's text theme has a body medium style"),
        )
    }

    fn title_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let theme = self.context.theme(app);
        let text_theme = &theme.text_theme;
        Some(
            text_theme
                .title_large
                .clone()
                .expect("a theme's text theme has a title large style"),
        )
    }

    // TODO(Craftplacer): Consider using EdgeInsets.only(right: 8.0) instead of
    // EdgeInsets.zero for Material 3 in the future,
    // https://github.com/flutter/flutter/issues/155747
    fn actions_padding(&self, _app: &mut App) -> Option<EdgeInsetsGeometry> {
        Some(EdgeInsets::ZERO.into())
    }
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
            colors: theme.color_scheme.clone(),
            text_theme: theme.text_theme.clone(),
        }
    }
}

impl ScrollUnderFlexibleConfig for MediumScrollUnderFlexibleConfig {
    fn collapsed_text_style(&self) -> Option<TextStyle> {
        self.text_theme
            .title_large
            .as_ref()
            .map(|style| style.apply().color(self.colors.on_surface()).into_style())
    }

    fn expanded_text_style(&self) -> Option<TextStyle> {
        self.text_theme
            .headline_small
            .as_ref()
            .map(|style| style.apply().color(self.colors.on_surface()).into_style())
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
            colors: theme.color_scheme.clone(),
            text_theme: theme.text_theme.clone(),
        }
    }
}

impl ScrollUnderFlexibleConfig for LargeScrollUnderFlexibleConfig {
    fn collapsed_text_style(&self) -> Option<TextStyle> {
        self.text_theme
            .title_large
            .as_ref()
            .map(|style| style.apply().color(self.colors.on_surface()).into_style())
    }

    fn expanded_text_style(&self) -> Option<TextStyle> {
        self.text_theme
            .headline_medium
            .as_ref()
            .map(|style| style.apply().color(self.colors.on_surface()).into_style())
    }

    fn expanded_title_padding(&self) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::from_ltrb(16.0, 0.0, 16.0, 28.0)
    }
}
