//! Flutter counterpart: material_ui's `generated/navigation_bar_defaults_m3.g.dart`, a part of
//! `navigation_bar.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use waft_foundation::App;
use waft_painting::{BorderSide, EdgeInsets, StadiumBorder};
use waft_widgets::{BuildContext, IconThemeData, WidgetState, WidgetStateProperty};

use crate::colors::Colors;
use crate::navigation_bar::NavigationDestinationLabelBehavior;
use crate::navigation_bar_theme::{NavigationBarThemeData, NavigationBarThemeDataGetters};
use crate::theme_data_getters::DefaultsContext;
use waft_embedder::Color;
use waft_painting::EdgeInsetsGeometry;
use waft_painting::ShapeBorderRef;
use waft_painting::TextStyle;
use waft_widgets::WidgetStatePropertyRef;

/// Dart's `_NavigationBarDefaultsM3`.
pub(crate) struct NavigationBarDefaultsM3 {
    /// Dart's `super(..)` arguments.
    navigation_bar_theme_data: NavigationBarThemeData,
    context: DefaultsContext,
}

impl NavigationBarDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> NavigationBarDefaultsM3 {
        NavigationBarDefaultsM3 {
            navigation_bar_theme_data: NavigationBarThemeData::new()
                .height(80.0)
                .elevation(3.0)
                .label_behavior(NavigationDestinationLabelBehavior::AlwaysShow),
            context: DefaultsContext::new(context),
        }
    }
}

impl NavigationBarThemeDataGetters for NavigationBarDefaultsM3 {
    fn navigation_bar_theme_data(&self) -> &NavigationBarThemeData {
        &self.navigation_bar_theme_data
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some(colors.surface_container())
    }

    fn shadow_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn icon_theme(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<IconThemeData>>> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        let icon_colors = colors.clone();
        Some(
            <dyn WidgetStateProperty<Option<IconThemeData>>>::resolve_with(move |states| {
                Some(IconThemeData::new().size(24.0).color(
                    if states.contains(&WidgetState::Disabled) {
                        icon_colors.on_surface_variant().with_opacity(0.38)
                    } else if states.contains(&WidgetState::Selected) {
                        icon_colors.on_secondary_container()
                    } else {
                        icon_colors.on_surface_variant()
                    },
                ))
            }),
        )
    }

    fn indicator_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some(colors.secondary_container())
    }

    fn indicator_shape(&self, _app: &mut App) -> Option<ShapeBorderRef> {
        Some(ShapeBorderRef::new(StadiumBorder::new(BorderSide::NONE)))
    }

    fn label_text_style(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<TextStyle>>> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        let text_theme = theme.text_theme.clone();
        let label_colors = colors.clone();
        Some(<dyn WidgetStateProperty<Option<_>>>::resolve_with(
            move |states| {
                let style = text_theme
                    .label_medium
                    .clone()
                    .expect("the text theme gives labelMedium");
                Some(
                    style
                        .apply()
                        .color(if states.contains(&WidgetState::Disabled) {
                            label_colors.on_surface_variant().with_opacity(0.38)
                        } else if states.contains(&WidgetState::Selected) {
                            label_colors.on_surface()
                        } else {
                            label_colors.on_surface_variant()
                        })
                        .into_style(),
                )
            },
        ))
    }

    fn label_padding(&self, _app: &mut App) -> Option<EdgeInsetsGeometry> {
        Some(EdgeInsets::only(0.0, 4.0, 0.0, 0.0).into())
    }
}
