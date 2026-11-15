//! Flutter counterpart: material_ui's `generated/input_decorator_defaults_m3.g.dart`, a part of
//! `input_decorator.dart`.

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

use crate::input_decorator::{InputDecorationThemeData, InputDecorationThemeDataGetters};
use crate::theme_data_getters::DefaultsContext;
use waft_painting::AnyBorderSide;
use waft_painting::AnyColor;
use waft_painting::AnyTextStyle;

/// Dart's `_InputDecoratorDefaultsM3`.
pub(crate) struct InputDecoratorDefaultsM3 {
    /// Dart's `super(..)` arguments.
    input_decoration_theme_data: InputDecorationThemeData,
    context: DefaultsContext,
}

impl InputDecoratorDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> InputDecoratorDefaultsM3 {
        InputDecoratorDefaultsM3 {
            input_decoration_theme_data: InputDecorationThemeData::new(),
            context: DefaultsContext::new(context),
        }
    }
}

impl InputDecorationThemeDataGetters for InputDecoratorDefaultsM3 {
    fn input_decoration_theme_data(&self) -> &InputDecorationThemeData {
        &self.input_decoration_theme_data
    }

    fn hint_style(&self, app: &mut App) -> Option<AnyTextStyle> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let on_surface = colors.on_surface;
        let on_surface_variant = colors.on_surface_variant();
        let hint_style = WidgetStateTextStyle::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) {
                return TextStyle::new().color(on_surface.with_opacity(0.38));
            }
            TextStyle::new().color(on_surface_variant)
        });
        Some(hint_style.into())
    }

    fn fill_color(&self, app: &mut App) -> Option<AnyColor> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let on_surface = colors.on_surface;
        let surface_container_highest = colors.surface_container_highest();
        let fill_color = WidgetStateColor::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) {
                return on_surface.with_opacity(0.04);
            }
            surface_container_highest
        });
        Some(fill_color.into())
    }

    fn active_indicator_border(&self, app: &mut App) -> Option<AnyBorderSide> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let on_surface = colors.on_surface;
        let on_surface_variant = colors.on_surface_variant();
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
        Some(active_indicator_border.into())
    }

    fn outline_border(&self, app: &mut App) -> Option<AnyBorderSide> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let on_surface = colors.on_surface;
        let error = colors.error;
        let on_error_container = colors.on_error_container();
        let primary = colors.primary;
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
        Some(outline_border.into())
    }

    fn icon_color(&self, app: &mut App) -> Option<AnyColor> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let on_surface_variant = colors.on_surface_variant();
        let icon_color = on_surface_variant;
        Some(icon_color.into())
    }

    fn prefix_icon_color(&self, app: &mut App) -> Option<AnyColor> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let on_surface = colors.on_surface;
        let on_surface_variant = colors.on_surface_variant();
        let prefix_icon_color = WidgetStateColor::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) {
                return on_surface.with_opacity(0.38);
            }
            on_surface_variant
        });
        Some(prefix_icon_color.into())
    }

    fn suffix_icon_color(&self, app: &mut App) -> Option<AnyColor> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let on_surface = colors.on_surface;
        let on_surface_variant = colors.on_surface_variant();
        let error = colors.error;
        let on_error_container = colors.on_error_container();
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
        Some(suffix_icon_color.into())
    }

    fn label_style(&self, app: &mut App) -> Option<AnyTextStyle> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let text_theme = &theme.text_theme;
        let on_surface = colors.on_surface;
        let on_surface_variant = colors.on_surface_variant();
        let error = colors.error;
        let on_error_container = colors.on_error_container();
        let primary = colors.primary;
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
        Some(label_style.into())
    }

    fn floating_label_style(&self, app: &mut App) -> Option<AnyTextStyle> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let text_theme = &theme.text_theme;
        let on_surface = colors.on_surface;
        let on_surface_variant = colors.on_surface_variant();
        let error = colors.error;
        let on_error_container = colors.on_error_container();
        let primary = colors.primary;
        let body_large = text_theme.body_large.clone().unwrap_or_default();
        let floating_label_style =
            WidgetStateTextStyle::resolve_with(move |states: &WidgetStates| {
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
        Some(floating_label_style.into())
    }

    fn helper_style(&self, app: &mut App) -> Option<AnyTextStyle> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let text_theme = &theme.text_theme;
        let on_surface = colors.on_surface;
        let on_surface_variant = colors.on_surface_variant();
        let body_small = text_theme.body_small.clone().unwrap_or_default();
        let helper_body_small = body_small.clone();
        let helper_style = WidgetStateTextStyle::resolve_with(move |states: &WidgetStates| {
            let text_style = &helper_body_small;
            if states.contains(&WidgetState::Disabled) {
                return text_style.copy_with().color(on_surface.with_opacity(0.38));
            }
            text_style.copy_with().color(on_surface_variant)
        });
        Some(helper_style.into())
    }

    fn error_style(&self, app: &mut App) -> Option<AnyTextStyle> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let text_theme = &theme.text_theme;
        let error = colors.error;
        let body_small = text_theme.body_small.clone().unwrap_or_default();
        let error_style = WidgetStateTextStyle::resolve_with(move |_states: &WidgetStates| {
            let text_style = &body_small;
            text_style.copy_with().color(error)
        });
        Some(error_style.into())
    }
}
