//! Flutter counterpart: material_ui's `generated/navigation_bar_defaults_m3.g.dart`, a part of
//! `navigation_bar.dart`.
//!
//! Dart's private `_NavigationBarDefaultsM3` is a `NavigationBarThemeData` whose getters read
//! the context's theme; [`navigation_bar_defaults_m3`] returns the data those getters answer,
//! read once from the theme.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use waft_foundation::App;
use waft_painting::{BorderSide, EdgeInsets, StadiumBorder};
use waft_widgets::{BuildContext, IconThemeData, WidgetState, WidgetStateProperty};

use crate::colors::Colors;
use crate::navigation_bar::NavigationDestinationLabelBehavior;
use crate::navigation_bar_theme::NavigationBarThemeData;
use crate::theme::Theme;

/// Dart's `_NavigationBarDefaultsM3(context)`.
pub(crate) fn navigation_bar_defaults_m3(
    app: &mut App,
    context: BuildContext,
) -> NavigationBarThemeData {
    let theme = Theme::of(app, context);
    let colors = theme.color_scheme.clone();
    let text_theme = theme.text_theme.clone();

    let icon_colors = colors.clone();
    let label_colors = colors.clone();
    NavigationBarThemeData::new()
        .height(80.0)
        .elevation(3.0)
        .label_behavior(NavigationDestinationLabelBehavior::AlwaysShow)
        .background_color(colors.surface_container())
        .shadow_color(Colors::TRANSPARENT)
        .surface_tint_color(Colors::TRANSPARENT)
        .icon_theme(
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
        .indicator_color(colors.secondary_container())
        .indicator_shape(StadiumBorder::new(BorderSide::NONE))
        .label_text_style(<dyn WidgetStateProperty<Option<_>>>::resolve_with(
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
                            label_colors.on_surface
                        } else {
                            label_colors.on_surface_variant()
                        })
                        .into_style(),
                )
            },
        ))
        .label_padding(EdgeInsets::only(0.0, 4.0, 0.0, 0.0))
}
