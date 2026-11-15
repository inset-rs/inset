//! Flutter counterpart: material_ui's `generated/filled_card_defaults_m3.g.dart`, a part of
//! `card.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::{Clip, Color};
use waft_foundation::App;
use waft_painting::{
    BorderRadiusGeometry, BorderSide, EdgeInsetsGeometry, RoundedRectangleBorder, ShapeBorderRef,
};
use waft_widgets::BuildContext;

use crate::card_theme::{CardThemeData, CardThemeDataGetters};
use crate::colors::Colors;
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_FilledCardDefaultsM3`.
pub(crate) struct FilledCardDefaultsM3 {
    /// Dart's `super(..)` arguments.
    card_theme_data: CardThemeData,
    context: DefaultsContext,
}

impl FilledCardDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> FilledCardDefaultsM3 {
        FilledCardDefaultsM3 {
            card_theme_data: CardThemeData::new()
                .clip_behavior(Clip::None)
                .elevation(0.0)
                .margin(EdgeInsetsGeometry::all(4.0)),
            context: DefaultsContext::new(context),
        }
    }
}

impl CardThemeDataGetters for FilledCardDefaultsM3 {
    fn card_theme_data(&self) -> &CardThemeData {
        &self.card_theme_data
    }

    fn color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).surface_container_highest())
    }

    fn shadow_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).shadow())
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn shape(&self, _app: &mut App) -> Option<ShapeBorderRef> {
        Some(ShapeBorderRef::new(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::circular(12.0),
        )))
    }
}
