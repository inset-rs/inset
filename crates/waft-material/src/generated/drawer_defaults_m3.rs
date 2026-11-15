//! Flutter counterpart: material_ui's `generated/drawer_defaults_m3.g.dart`, a Dart `part`
//! of `drawer.dart`: `Drawer`'s Material 3 build reads it.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::{Clip, Radius};
use waft_foundation::App;
use waft_painting::{BorderRadiusDirectional, BorderSide, RoundedRectangleBorder};
use waft_widgets::{BuildContext, Directionality};

use crate::colors::Colors;
use crate::drawer_theme::DrawerThemeData;
use crate::theme::Theme;

/// Dart's `_DrawerDefaultsM3(context)`: the [`DrawerThemeData`] whose fields are the
/// constructor's arguments and the values of that subclass's getters.
pub(crate) fn drawer_defaults_m3(app: &mut App, context: BuildContext) -> DrawerThemeData {
    let direction = Directionality::of(app, context);
    let theme = Theme::of(app, context);

    DrawerThemeData::new()
        .elevation(1.0)
        .clip_behavior(Clip::HardEdge)
        .background_color(theme.color_scheme.surface_container_low())
        .surface_tint_color(Colors::TRANSPARENT)
        .shadow_color(Colors::TRANSPARENT)
        // There isn't currently a token for this value, but it is shown in the spec,
        // so hard coding here for now.
        .shape(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusDirectional::horizontal(Radius::ZERO, Radius::circular(16.0))
                .resolve(Some(direction))
                .into(),
        ))
        // There isn't currently a token for this value, but it is shown in the spec,
        // so hard coding here for now.
        .end_shape(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusDirectional::horizontal(Radius::circular(16.0), Radius::ZERO)
                .resolve(Some(direction))
                .into(),
        ))
}
