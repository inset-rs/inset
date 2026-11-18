//! Flutter counterpart: material_ui's `generated/popup_menu_defaults_m3.g.dart`, a part of
//! `popup_menu.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use waft_embedder::{Color, Radius};
use waft_foundation::App;
use waft_painting::{
    BorderRadiusGeometry, BorderSide, EdgeInsets, EdgeInsetsGeometry, RoundedRectangleBorder,
    ShapeBorderRef, TextStyle,
};
use waft_widgets::{BuildContext, WidgetState, WidgetStateProperty, WidgetStatePropertyRef};

use crate::colors::Colors;
use crate::popup_menu_theme::{PopupMenuThemeData, PopupMenuThemeDataGetters};
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_PopupMenuDefaultsM3`.
pub(crate) struct PopupMenuDefaultsM3 {
    /// Dart's `super(..)` arguments.
    popup_menu_theme_data: PopupMenuThemeData,
    context: DefaultsContext,
}

impl PopupMenuDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> PopupMenuDefaultsM3 {
        PopupMenuDefaultsM3 {
            popup_menu_theme_data: PopupMenuThemeData::new().elevation(3.0),
            context: DefaultsContext::new(context),
        }
    }
}

impl PopupMenuThemeDataGetters for PopupMenuDefaultsM3 {
    fn popup_menu_theme_data(&self) -> &PopupMenuThemeData {
        &self.popup_menu_theme_data
    }

    fn label_text_style(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<TextStyle>>> {
        let colors = self.context.colors(app).clone();
        let label_large = self.context.text_theme(app).label_large.clone();
        Some(<dyn WidgetStateProperty<Option<TextStyle>>>::resolve_with(
            move |states| {
                // TODO(quncheng): Update this hard-coded value to use the latest tokens.
                let style = label_large.as_ref().expect("textTheme.labelLarge");
                if states.contains(&WidgetState::Disabled) {
                    return Some(
                        style
                            .apply()
                            .color(colors.on_surface().with_opacity(0.38))
                            .into_style(),
                    );
                }
                Some(style.apply().color(colors.on_surface()).into_style())
            },
        ))
    }

    fn color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).surface_container())
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
            BorderRadiusGeometry::all(Radius::circular(4.0)),
        )))
    }

    fn menu_padding(&self, _app: &mut App) -> Option<EdgeInsetsGeometry> {
        // TODO(bleroux): This is taken from https://m3.material.io/components/menus/specs
        // Update this when the token is available.
        Some(EdgeInsetsGeometry::symmetric(8.0, 0.0))
    }
}

/// Dart's `_PopupMenuDefaultsM3.menuItemPadding`.
// TODO(tahatesser): This is taken from https://m3.material.io/components/menus/specs
// Update this when the token is available.
pub(crate) const MENU_ITEM_PADDING: EdgeInsets = EdgeInsets::symmetric(0.0, 12.0);
