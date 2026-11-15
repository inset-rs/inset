//! Flutter counterpart: material_ui's `generated/navigation_drawer_defaults_m3.g.dart`, a part of
//! `navigation_drawer.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use waft_embedder::{Color, Size};
use waft_foundation::App;
use waft_painting::{BorderSide, StadiumBorder, TextStyle};
use waft_widgets::{BuildContext, IconThemeData, WidgetState, WidgetStateProperty, WidgetStates};

use crate::colors::Colors;
use crate::navigation_drawer_theme::{NavigationDrawerThemeData, NavigationDrawerThemeDataGetters};
use crate::theme_data_getters::DefaultsContext;
use waft_widgets::WidgetStatePropertyRef;

/// Dart's `_NavigationDrawerDefaultsM3`.
pub(crate) struct NavigationDrawerDefaultsM3 {
    /// Dart's `super(..)` arguments.
    navigation_drawer_theme_data: NavigationDrawerThemeData,
    context: DefaultsContext,
}

impl NavigationDrawerDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> NavigationDrawerDefaultsM3 {
        NavigationDrawerDefaultsM3 {
            navigation_drawer_theme_data: NavigationDrawerThemeData::new()
                .elevation(1.0)
                .tile_height(56.0)
                .indicator_shape(StadiumBorder::new(BorderSide::NONE))
                .indicator_size(Size::new(336.0, 56.0)),
            context: DefaultsContext::new(context),
        }
    }
}

impl NavigationDrawerThemeDataGetters for NavigationDrawerDefaultsM3 {
    fn navigation_drawer_theme_data(&self) -> &NavigationDrawerThemeData {
        &self.navigation_drawer_theme_data
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some(colors.surface_container_low())
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn shadow_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn indicator_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some(colors.secondary_container())
    }

    fn icon_theme(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<IconThemeData>>> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        let icon_colors = colors.clone();
        Some(
            <dyn WidgetStateProperty<Option<IconThemeData>>>::resolve_with(
                move |states: &WidgetStates| {
                    Some(IconThemeData::new().size(24.0).color(
                        if states.contains(&WidgetState::Disabled) {
                            icon_colors.on_surface_variant().with_opacity(0.38)
                        } else if states.contains(&WidgetState::Selected) {
                            icon_colors.on_secondary_container()
                        } else {
                            icon_colors.on_surface_variant()
                        },
                    ))
                },
            ),
        )
    }

    fn label_text_style(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<TextStyle>>> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        let text_theme = theme.text_theme.clone();
        let label_colors = colors.clone();
        Some(<dyn WidgetStateProperty<Option<TextStyle>>>::resolve_with(
            move |states: &WidgetStates| {
                let style: &TextStyle = text_theme
                    .label_large
                    .as_ref()
                    .expect("the Material 3 text theme gives labelLarge");
                let color: Color = if states.contains(&WidgetState::Disabled) {
                    label_colors.on_surface_variant().with_opacity(0.38)
                } else if states.contains(&WidgetState::Selected) {
                    label_colors.on_secondary_container()
                } else {
                    label_colors.on_surface_variant()
                };
                Some(style.apply().color(color).into_style())
            },
        ))
    }
}
