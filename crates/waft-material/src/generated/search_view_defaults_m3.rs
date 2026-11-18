//! Flutter counterpart: material_ui's `generated/search_view_defaults_m3.g.dart`, a part of
//! `search_anchor.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use waft_embedder::Color;
use waft_foundation::App;
use waft_painting::{
    BorderRadiusGeometry, BorderSide, EdgeInsets, EdgeInsetsGeometry, OutlinedBorderRef,
    RoundedRectangleBorder, TextStyle,
};
use waft_rendering::BoxConstraints;
use waft_widgets::BuildContext;

use crate::colors::Colors;
use crate::search_view_theme::{SearchViewThemeData, SearchViewThemeDataGetters};
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_SearchViewDefaultsM3`.
pub(crate) struct SearchViewDefaultsM3 {
    /// Dart's `super(..)` arguments: none.
    search_view_theme_data: SearchViewThemeData,
    context: DefaultsContext,
    is_full_screen: bool,
}

impl SearchViewDefaultsM3 {
    /// Dart's `static double fullScreenBarHeight`.
    pub(crate) const FULL_SCREEN_BAR_HEIGHT: f64 = 72.0;

    pub(crate) fn new(context: BuildContext, is_full_screen: bool) -> SearchViewDefaultsM3 {
        SearchViewDefaultsM3 {
            search_view_theme_data: SearchViewThemeData::new(),
            context: DefaultsContext::new(context),
            is_full_screen,
        }
    }
}

impl SearchViewThemeDataGetters for SearchViewDefaultsM3 {
    fn search_view_theme_data(&self) -> &SearchViewThemeData {
        &self.search_view_theme_data
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).surface_container_high())
    }

    fn elevation(&self, _app: &mut App) -> Option<f64> {
        Some(6.0)
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    // No default side
    fn shape(&self, _app: &mut App) -> Option<OutlinedBorderRef> {
        Some(OutlinedBorderRef::new(if self.is_full_screen {
            RoundedRectangleBorder::default()
        } else {
            RoundedRectangleBorder::new(BorderSide::NONE, BorderRadiusGeometry::circular(28.0))
        }))
    }

    fn header_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let on_surface = self.context.colors(app).on_surface();
        self.context
            .text_theme(app)
            .body_large
            .as_ref()
            .map(|body_large| body_large.copy_with().color(on_surface))
    }

    fn header_hint_style(&self, app: &mut App) -> Option<TextStyle> {
        let on_surface_variant = self.context.colors(app).on_surface_variant();
        self.context
            .text_theme(app)
            .body_large
            .as_ref()
            .map(|body_large| body_large.copy_with().color(on_surface_variant))
    }

    fn constraints(&self, _app: &mut App) -> Option<BoxConstraints> {
        Some(BoxConstraints {
            min_width: 360.0,
            min_height: 240.0,
            ..BoxConstraints::new()
        })
    }

    fn bar_padding(&self, _app: &mut App) -> Option<EdgeInsetsGeometry> {
        Some(EdgeInsets::symmetric(0.0, 8.0).into())
    }

    fn shrink_wrap(&self, _app: &mut App) -> Option<bool> {
        Some(false)
    }

    fn divider_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).outline())
    }
}
