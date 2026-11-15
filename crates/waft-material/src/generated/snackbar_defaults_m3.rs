//! Flutter counterpart: material_ui's `generated/snackbar_defaults_m3.g.dart`, a part of
//! `snack_bar.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::{Color, Radius};
use waft_foundation::App;
use waft_painting::{
    AnyColor, BorderRadiusGeometry, BorderSide, EdgeInsets, RoundedRectangleBorder, ShapeBorderRef,
    TextStyle,
};
use waft_widgets::{BuildContext, WidgetState, WidgetStateColor};

use crate::snack_bar_theme::{SnackBarBehavior, SnackBarThemeData, SnackBarThemeDataGetters};
use crate::theme::Theme;
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_SnackbarDefaultsM3`.
pub(crate) struct SnackbarDefaultsM3 {
    /// Dart's `super(..)` arguments.
    snack_bar_theme_data: SnackBarThemeData,
    context: DefaultsContext,
}

impl SnackbarDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> SnackbarDefaultsM3 {
        SnackbarDefaultsM3 {
            snack_bar_theme_data: SnackBarThemeData::new(),
            context: DefaultsContext::new(context),
        }
    }
}

impl SnackBarThemeDataGetters for SnackbarDefaultsM3 {
    fn snack_bar_theme_data(&self) -> &SnackBarThemeData {
        &self.snack_bar_theme_data
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).inverse_surface())
    }

    fn action_text_color(&self, app: &mut App) -> Option<AnyColor> {
        let inverse_primary = self.context.colors(app).inverse_primary();
        Some(
            WidgetStateColor::resolve_with(move |states| {
                if states.contains(&WidgetState::Disabled) {
                    return inverse_primary;
                }
                if states.contains(&WidgetState::Pressed) {
                    return inverse_primary;
                }
                if states.contains(&WidgetState::Hovered) {
                    return inverse_primary;
                }
                if states.contains(&WidgetState::Focused) {
                    return inverse_primary;
                }
                inverse_primary
            })
            .into_any(),
        )
    }

    fn disabled_action_text_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).inverse_primary())
    }

    fn content_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let on_inverse_surface = self.context.colors(app).on_inverse_surface();
        Some(
            Theme::of(app, self.context.context())
                .text_theme
                .body_medium
                .clone()
                .expect("a theme's text theme has a body medium style")
                .copy_with()
                .color(on_inverse_surface),
        )
    }

    fn elevation(&self, _app: &mut App) -> Option<f64> {
        Some(6.0)
    }

    fn shape(&self, _app: &mut App) -> Option<ShapeBorderRef> {
        Some(ShapeBorderRef::new(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::all(Radius::circular(4.0)),
        )))
    }

    fn behavior(&self, _app: &mut App) -> Option<SnackBarBehavior> {
        Some(SnackBarBehavior::Fixed)
    }

    fn inset_padding(&self, _app: &mut App) -> Option<EdgeInsets> {
        Some(EdgeInsets::from_ltrb(15.0, 5.0, 15.0, 10.0))
    }

    fn show_close_icon(&self, _app: &mut App) -> Option<bool> {
        Some(false)
    }

    fn close_icon_color(&self, app: &mut App) -> Option<AnyColor> {
        Some(self.context.colors(app).on_inverse_surface().into())
    }

    fn action_overflow_threshold(&self, _app: &mut App) -> Option<f64> {
        Some(0.25)
    }
}
