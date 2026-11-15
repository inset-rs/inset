//! Flutter counterpart: material_ui's `generated/input_decorator_defaults_m3.g.dart`, a Dart
//! `part` of `input_decorator.dart`: `InputDecorator`'s Material 3 build reads it.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use waft_foundation::App;
use waft_painting::{BorderSide, TextStyle};
use waft_widgets::{
    BuildContext, WidgetState, WidgetStateBorderSide, WidgetStateColor, WidgetStateTextStyle,
    WidgetStates,
};

use crate::input_decorator::InputDecorationThemeData;
use crate::theme::Theme;

/// Dart's `_InputDecoratorDefaultsM3(context)`: the [`InputDecorationThemeData`] whose fields
/// are the values of that subclass's getters, read from the context's theme when it is made.
pub(crate) fn input_decorator_defaults_m3(
    app: &mut App,
    context: BuildContext,
) -> InputDecorationThemeData {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;
    let text_theme = &theme.text_theme;

    // For InputDecorator, focused state should take precedence over hovered state.
    // For instance, the focused state increases border width (2dp) and applies bright
    // colors (primary color or error color) while the hovered state has the same border
    // than the non-focused state (1dp) and uses a color a little darker than non-focused
    // state. On desktop, it is also very common that a text field is focused and hovered
    // because users often rely on mouse selection.
    // For other widgets, hovered state takes precedence over focused state, because it
    // is mainly used to determine the overlay color,
    // see https://github.com/flutter/flutter/pull/125905.

    let on_surface = colors.on_surface;
    let on_surface_variant = colors.on_surface_variant();
    let hint_style = WidgetStateTextStyle::resolve_with(move |states: &WidgetStates| {
        if states.contains(&WidgetState::Disabled) {
            return TextStyle::new().color(on_surface.with_opacity(0.38));
        }
        TextStyle::new().color(on_surface_variant)
    });

    let surface_container_highest = colors.surface_container_highest();
    let fill_color = WidgetStateColor::resolve_with(move |states: &WidgetStates| {
        if states.contains(&WidgetState::Disabled) {
            return on_surface.with_opacity(0.04);
        }
        surface_container_highest
    });

    let error = colors.error;
    let on_error_container = colors.on_error_container();
    let primary = colors.primary;
    let active_indicator_border =
        WidgetStateBorderSide::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) {
                return Some(BorderSide {
                    color: on_surface.with_opacity(0.38),
                    ..BorderSide::default()
                });
            }
            if states.contains(&WidgetState::Error) {
                if states.contains(&WidgetState::Focused) {
                    return Some(BorderSide {
                        width: 2.0,
                        color: error,
                        ..BorderSide::default()
                    });
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(BorderSide {
                        color: on_error_container,
                        ..BorderSide::default()
                    });
                }
                return Some(BorderSide {
                    color: error,
                    ..BorderSide::default()
                });
            }
            if states.contains(&WidgetState::Focused) {
                return Some(BorderSide {
                    width: 2.0,
                    color: primary,
                    ..BorderSide::default()
                });
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(BorderSide {
                    color: on_surface,
                    ..BorderSide::default()
                });
            }
            Some(BorderSide {
                color: on_surface_variant,
                ..BorderSide::default()
            })
        });

    let outline = colors.outline();
    let outline_border = WidgetStateBorderSide::resolve_with(move |states: &WidgetStates| {
        if states.contains(&WidgetState::Disabled) {
            return Some(BorderSide {
                color: on_surface.with_opacity(0.12),
                ..BorderSide::default()
            });
        }
        if states.contains(&WidgetState::Error) {
            if states.contains(&WidgetState::Focused) {
                return Some(BorderSide {
                    width: 2.0,
                    color: error,
                    ..BorderSide::default()
                });
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(BorderSide {
                    color: on_error_container,
                    ..BorderSide::default()
                });
            }
            return Some(BorderSide {
                color: error,
                ..BorderSide::default()
            });
        }
        if states.contains(&WidgetState::Focused) {
            return Some(BorderSide {
                width: 2.0,
                color: primary,
                ..BorderSide::default()
            });
        }
        if states.contains(&WidgetState::Hovered) {
            return Some(BorderSide {
                color: on_surface,
                ..BorderSide::default()
            });
        }
        Some(BorderSide {
            color: outline,
            ..BorderSide::default()
        })
    });

    let icon_color = on_surface_variant;

    let prefix_icon_color = WidgetStateColor::resolve_with(move |states: &WidgetStates| {
        if states.contains(&WidgetState::Disabled) {
            return on_surface.with_opacity(0.38);
        }
        on_surface_variant
    });

    let suffix_icon_color = WidgetStateColor::resolve_with(move |states: &WidgetStates| {
        if states.contains(&WidgetState::Disabled) {
            return on_surface.with_opacity(0.38);
        }
        if states.contains(&WidgetState::Error) {
            if states.contains(&WidgetState::Hovered) {
                return on_error_container;
            }
            return error;
        }
        on_surface_variant
    });

    let body_large = text_theme.body_large.clone().unwrap_or_default();
    let label_body_large = body_large.clone();
    let label_style = WidgetStateTextStyle::resolve_with(move |states: &WidgetStates| {
        let text_style = &label_body_large;
        if states.contains(&WidgetState::Disabled) {
            return text_style.copy_with().color(on_surface.with_opacity(0.38));
        }
        if states.contains(&WidgetState::Error) {
            if states.contains(&WidgetState::Focused) {
                return text_style.copy_with().color(error);
            }
            if states.contains(&WidgetState::Hovered) {
                return text_style.copy_with().color(on_error_container);
            }
            return text_style.copy_with().color(error);
        }
        if states.contains(&WidgetState::Focused) {
            return text_style.copy_with().color(primary);
        }
        if states.contains(&WidgetState::Hovered) {
            return text_style.copy_with().color(on_surface_variant);
        }
        text_style.copy_with().color(on_surface_variant)
    });

    let floating_label_style = WidgetStateTextStyle::resolve_with(move |states: &WidgetStates| {
        let text_style = &body_large;
        if states.contains(&WidgetState::Disabled) {
            return text_style.copy_with().color(on_surface.with_opacity(0.38));
        }
        if states.contains(&WidgetState::Error) {
            if states.contains(&WidgetState::Focused) {
                return text_style.copy_with().color(error);
            }
            if states.contains(&WidgetState::Hovered) {
                return text_style.copy_with().color(on_error_container);
            }
            return text_style.copy_with().color(error);
        }
        if states.contains(&WidgetState::Focused) {
            return text_style.copy_with().color(primary);
        }
        if states.contains(&WidgetState::Hovered) {
            return text_style.copy_with().color(on_surface_variant);
        }
        text_style.copy_with().color(on_surface_variant)
    });

    let body_small = text_theme.body_small.clone().unwrap_or_default();
    let helper_body_small = body_small.clone();
    let helper_style = WidgetStateTextStyle::resolve_with(move |states: &WidgetStates| {
        let text_style = &helper_body_small;
        if states.contains(&WidgetState::Disabled) {
            return text_style.copy_with().color(on_surface.with_opacity(0.38));
        }
        text_style.copy_with().color(on_surface_variant)
    });

    let error_style = WidgetStateTextStyle::resolve_with(move |_states: &WidgetStates| {
        let text_style = &body_small;
        text_style.copy_with().color(error)
    });

    InputDecorationThemeData::new()
        .hint_style(hint_style)
        .fill_color(fill_color)
        .active_indicator_border(active_indicator_border)
        .outline_border(outline_border)
        .icon_color(icon_color)
        .prefix_icon_color(prefix_icon_color)
        .suffix_icon_color(suffix_icon_color)
        .label_style(label_style)
        .floating_label_style(floating_label_style)
        .helper_style(helper_style)
        .error_style(error_style)
}
