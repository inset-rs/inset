//! Flutter counterpart: material_ui's `generated/dialog_defaults_m3.g.dart`, a part of
//! `dialog.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::{Clip, Color, Radius};
use waft_foundation::App;
use waft_painting::{
    AlignmentGeometry, BorderRadiusGeometry, BorderSide, EdgeInsetsGeometry,
    RoundedRectangleBorder, TextStyle,
};
use waft_widgets::BuildContext;

use crate::colors::Colors;
use crate::dialog_theme::{DialogThemeData, DialogThemeDataGetters};
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_DialogDefaultsM3`.
pub(crate) struct DialogDefaultsM3 {
    /// Dart's `super(..)` arguments.
    dialog_theme_data: DialogThemeData,
    context: DefaultsContext,
}

impl DialogDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> DialogDefaultsM3 {
        DialogDefaultsM3 {
            dialog_theme_data: DialogThemeData::new()
                .alignment(AlignmentGeometry::CENTER)
                .elevation(6.0)
                .shape(RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadiusGeometry::all(Radius::circular(28.0)),
                ))
                .clip_behavior(Clip::None),
            context: DefaultsContext::new(context),
        }
    }
}

impl DialogThemeDataGetters for DialogDefaultsM3 {
    fn dialog_theme_data(&self) -> &DialogThemeData {
        &self.dialog_theme_data
    }

    fn icon_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).secondary)
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).surface_container_high())
    }

    fn shadow_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn title_text_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).headline_small.clone()
    }

    fn content_text_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).body_medium.clone()
    }

    fn actions_padding(&self, _app: &mut App) -> Option<EdgeInsetsGeometry> {
        Some(EdgeInsetsGeometry::only(24.0, 0.0, 24.0, 24.0))
    }
}
