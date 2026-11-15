//! Flutter counterpart: material_ui's `generated/badge_defaults_m3.g.dart`, a part of
//! `badge.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::Color;
use waft_foundation::App;
use waft_painting::{AlignmentGeometry, EdgeInsetsGeometry, TextStyle};
use waft_widgets::BuildContext;

use crate::badge_theme::{BadgeThemeData, BadgeThemeDataGetters};
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_BadgeDefaultsM3`.
pub(crate) struct BadgeDefaultsM3 {
    /// Dart's `super(..)` arguments.
    badge_theme_data: BadgeThemeData,
    context: DefaultsContext,
}

impl BadgeDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> BadgeDefaultsM3 {
        BadgeDefaultsM3 {
            badge_theme_data: BadgeThemeData::new()
                .small_size(6.0)
                .large_size(16.0)
                .padding(EdgeInsetsGeometry::symmetric(0.0, 4.0))
                .alignment(AlignmentGeometry::TOP_END),
            context: DefaultsContext::new(context),
        }
    }
}

impl BadgeThemeDataGetters for BadgeDefaultsM3 {
    fn badge_theme_data(&self) -> &BadgeThemeData {
        &self.badge_theme_data
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).error)
    }

    fn text_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).on_error)
    }

    fn text_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).label_small.clone()
    }
}
