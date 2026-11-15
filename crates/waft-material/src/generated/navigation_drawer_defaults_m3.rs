//! Flutter counterpart: material_ui's `generated/navigation_drawer_defaults_m3.g.dart`, a
//! part of `navigation_drawer.dart`.
//!
//! Dart's private `_NavigationDrawerDefaultsM3` is a `NavigationDrawerThemeData` whose getters
//! read the context's theme; [`navigation_drawer_defaults_m3`] returns the data those getters
//! answer, read once from the theme.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use waft_embedder::{Color, Size};
use waft_foundation::App;
use waft_painting::{BorderSide, StadiumBorder, TextStyle};
use waft_widgets::{BuildContext, IconThemeData, WidgetState, WidgetStateProperty, WidgetStates};

use crate::colors::Colors;
use crate::navigation_drawer_theme::NavigationDrawerThemeData;
use crate::theme::Theme;

/// Dart's `_NavigationDrawerDefaultsM3(context)`.
pub(crate) fn navigation_drawer_defaults_m3(
    app: &mut App,
    context: BuildContext,
) -> NavigationDrawerThemeData {
    let theme = Theme::of(app, context);
    let colors = theme.color_scheme.clone();
    let text_theme = theme.text_theme.clone();

    let icon_colors = colors.clone();
    let label_colors = colors.clone();
    NavigationDrawerThemeData::new()
        .elevation(1.0)
        .tile_height(56.0)
        .indicator_shape(StadiumBorder::new(BorderSide::NONE))
        .indicator_size(Size::new(336.0, 56.0))
        .background_color(colors.surface_container_low())
        .surface_tint_color(Colors::TRANSPARENT)
        .shadow_color(Colors::TRANSPARENT)
        .indicator_color(colors.secondary_container())
        .icon_theme(
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
        .label_text_style(<dyn WidgetStateProperty<Option<TextStyle>>>::resolve_with(
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
