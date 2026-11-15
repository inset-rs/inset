//! Flutter counterpart: material_ui's `generated/popup_menu_defaults_m3.g.dart`, a Dart `part`
//! of `popup_menu.dart`.
//!
//! Dart's private `_PopupMenuDefaultsM3` is a `PopupMenuThemeData` whose getters read the
//! context's theme; [`popup_menu_defaults_m3`] returns the data those getters answer, read once
//! from the theme.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use waft_embedder::Radius;
use waft_foundation::App;
use waft_painting::{
    BorderRadiusGeometry, BorderSide, EdgeInsets, EdgeInsetsGeometry, RoundedRectangleBorder,
    TextStyle,
};
use waft_widgets::{BuildContext, WidgetState, WidgetStateProperty};

use crate::colors::Colors;
use crate::popup_menu_theme::PopupMenuThemeData;
use crate::theme::Theme;

/// Dart's `_PopupMenuDefaultsM3(context)`.
pub(crate) fn popup_menu_defaults_m3(app: &mut App, context: BuildContext) -> PopupMenuThemeData {
    let theme = Theme::of(app, context);
    let colors = theme.color_scheme.clone();
    let label_large = theme.text_theme.label_large.clone();
    PopupMenuThemeData::new()
        .elevation(3.0)
        .label_text_style(<dyn WidgetStateProperty<Option<TextStyle>>>::resolve_with(
            move |states| {
                // TODO(quncheng): Update this hard-coded value to use the latest tokens.
                let style = label_large.as_ref().expect("textTheme.labelLarge");
                if states.contains(&WidgetState::Disabled) {
                    return Some(
                        style
                            .apply()
                            .color(colors.on_surface.with_opacity(0.38))
                            .into_style(),
                    );
                }
                Some(style.apply().color(colors.on_surface).into_style())
            },
        ))
        .color(theme.color_scheme.surface_container())
        .shadow_color(theme.color_scheme.shadow())
        .surface_tint_color(Colors::TRANSPARENT)
        .shape(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::all(Radius::circular(4.0)),
        ))
        // TODO(bleroux): This is taken from https://m3.material.io/components/menus/specs
        // Update this when the token is available.
        .menu_padding(EdgeInsetsGeometry::symmetric(8.0, 0.0))
}

/// Dart's `_PopupMenuDefaultsM3.menuItemPadding`.
// TODO(tahatesser): This is taken from https://m3.material.io/components/menus/specs
// Update this when the token is available.
pub(crate) const MENU_ITEM_PADDING: EdgeInsets = EdgeInsets::symmetric(0.0, 12.0);
