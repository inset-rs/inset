//! Flutter counterpart: material_ui's `generated/expansion_tile_defaults_m3.g.dart`, a Dart
//! `part` of `expansion_tile.dart`: `ExpansionTile`'s Material 3 colours read it.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_foundation::App;
use waft_widgets::BuildContext;

use crate::expansion_tile_theme::ExpansionTileThemeData;
use crate::theme::Theme;

/// Dart's `_ExpansionTileDefaultsM3(context)`: the [`ExpansionTileThemeData`] whose fields
/// are the values of that subclass's getters.
pub(crate) fn expansion_tile_defaults_m3(
    app: &mut App,
    context: BuildContext,
) -> ExpansionTileThemeData {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;

    ExpansionTileThemeData::new()
        .text_color(colors.on_surface)
        .icon_color(colors.primary)
        .collapsed_text_color(colors.on_surface)
        .collapsed_icon_color(colors.on_surface_variant())
}
