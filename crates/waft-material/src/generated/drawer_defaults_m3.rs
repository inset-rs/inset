//! Flutter counterpart: material_ui's `generated/drawer_defaults_m3.g.dart`, a Dart `part`
//! of `drawer.dart`: `Drawer`'s Material 3 build reads it.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use std::cell::OnceCell;

use waft_embedder::{Clip, Color, Radius, TextDirection};
use waft_foundation::App;
use waft_painting::{BorderRadiusDirectional, BorderSide, RoundedRectangleBorder, ShapeBorderRef};
use waft_widgets::{BuildContext, Directionality};

use crate::colors::Colors;
use crate::drawer_theme::{DrawerThemeData, DrawerThemeDataGetters};
use crate::theme::Theme;

/// Dart's `_DrawerDefaultsM3`.
pub(crate) struct DrawerDefaultsM3 {
    /// Dart's `super(..)` arguments.
    drawer_theme_data: DrawerThemeData,
    context: BuildContext,
    direction: OnceCell<TextDirection>,
}

impl DrawerDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> DrawerDefaultsM3 {
        DrawerDefaultsM3 {
            drawer_theme_data: DrawerThemeData::new()
                .elevation(1.0)
                .clip_behavior(Clip::HardEdge),
            context,
            direction: OnceCell::new(),
        }
    }

    /// Dart's `late final TextDirection direction = Directionality.of(context)`.
    fn direction(&self, app: &mut App) -> TextDirection {
        *self
            .direction
            .get_or_init(|| Directionality::of(app, self.context))
    }
}

impl DrawerThemeDataGetters for DrawerDefaultsM3 {
    fn drawer_theme_data(&self) -> &DrawerThemeData {
        &self.drawer_theme_data
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        Some(
            Theme::of(app, self.context)
                .color_scheme
                .surface_container_low(),
        )
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn shadow_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    // There isn't currently a token for this value, but it is shown in the spec,
    // so hard coding here for now.
    fn shape(&self, app: &mut App) -> Option<ShapeBorderRef> {
        Some(ShapeBorderRef::new(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusDirectional::horizontal(Radius::ZERO, Radius::circular(16.0))
                .resolve(Some(self.direction(app)))
                .into(),
        )))
    }

    // There isn't currently a token for this value, but it is shown in the spec,
    // so hard coding here for now.
    fn end_shape(&self, app: &mut App) -> Option<ShapeBorderRef> {
        Some(ShapeBorderRef::new(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusDirectional::horizontal(Radius::circular(16.0), Radius::ZERO)
                .resolve(Some(self.direction(app)))
                .into(),
        )))
    }
}
