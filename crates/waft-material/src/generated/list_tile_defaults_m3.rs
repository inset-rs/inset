//! Flutter counterpart: material_ui's `generated/list_tile_defaults_m3.g.dart`, a Dart `part`
//! of `list_tile.dart`: `ListTile`'s Material 3 build reads it.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_foundation::App;
use waft_painting::{AnyColor, EdgeInsetsDirectional, RoundedRectangleBorder};
use waft_widgets::BuildContext;

use crate::colors::Colors;
use crate::list_tile_theme::ListTileThemeData;
use crate::theme::Theme;

/// Dart's `_ListTileDefaultsM3(context)`: the [`ListTileThemeData`] whose fields are the
/// constructor's arguments and the values of that subclass's getters.
///
/// A getter's `!` on a text theme style that is null leaves its field null here, so the
/// tile's `expect` fails where it reads the field, as Dart's getter fails when it is read.
pub(crate) fn list_tile_defaults_m3(app: &mut App, context: BuildContext) -> ListTileThemeData {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;
    let text_theme = &theme.text_theme;

    let mut defaults = ListTileThemeData::new()
        .content_padding(EdgeInsetsDirectional::only(16.0, 0.0, 24.0, 0.0))
        .min_leading_width(24.0)
        .min_vertical_padding(8.0)
        .shape(RoundedRectangleBorder::default())
        .tile_color(Colors::TRANSPARENT)
        .selected_color(colors.primary)
        .icon_color(AnyColor::new(colors.on_surface_variant()));
    defaults.title_text_style = text_theme
        .body_large
        .as_ref()
        .map(|body_large| body_large.copy_with().color(colors.on_surface));
    defaults.subtitle_text_style = text_theme
        .body_medium
        .as_ref()
        .map(|body_medium| body_medium.copy_with().color(colors.on_surface_variant()));
    defaults.leading_and_trailing_text_style = text_theme
        .label_small
        .as_ref()
        .map(|label_small| label_small.copy_with().color(colors.on_surface_variant()));
    defaults
}
