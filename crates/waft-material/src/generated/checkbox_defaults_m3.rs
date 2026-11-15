//! Flutter counterpart: material_ui's `generated/checkbox_defaults_m3.g.dart`, a part of
//! `checkbox.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use waft_embedder::{Color, Radius};
use waft_foundation::App;
use waft_painting::{BorderRadiusGeometry, BorderSide, RoundedRectangleBorder};
use waft_widgets::{
    BuildContext, WidgetState, WidgetStateBorderSide, WidgetStateProperty, WidgetStates,
};

use crate::checkbox_theme::{CheckboxThemeData, CheckboxThemeDataGetters};
use crate::colors::Colors;
use crate::theme_data::MaterialTapTargetSize;
use crate::theme_data::VisualDensity;
use crate::theme_data_getters::DefaultsContext;
use waft_painting::AnyBorderSide;
use waft_painting::OutlinedBorderRef;
use waft_widgets::WidgetStatePropertyRef;

/// Dart's `_CheckboxDefaultsM3`.
pub(crate) struct CheckboxDefaultsM3 {
    /// Dart's `super(..)` arguments.
    checkbox_theme_data: CheckboxThemeData,
    context: DefaultsContext,
}

impl CheckboxDefaultsM3 {
    pub(crate) fn new(app: &mut App, context: BuildContext) -> CheckboxDefaultsM3 {
        CheckboxDefaultsM3 {
            checkbox_theme_data: CheckboxThemeData::new(),
            context: DefaultsContext::with_theme(app, context),
        }
    }
}

impl CheckboxThemeDataGetters for CheckboxDefaultsM3 {
    fn checkbox_theme_data(&self) -> &CheckboxThemeData {
        &self.checkbox_theme_data
    }

    fn side(&self, app: &mut App) -> Option<AnyBorderSide> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let on_surface = colors.on_surface;
        let on_surface_variant = colors.on_surface_variant();
        let error = colors.error;
        let side = WidgetStateBorderSide::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) {
                if states.contains(&WidgetState::Selected) {
                    return Some(BorderSide {
                        width: 2.0,
                        color: Colors::TRANSPARENT,
                        ..BorderSide::default()
                    });
                }
                return Some(BorderSide {
                    width: 2.0,
                    color: on_surface.with_opacity(0.38),
                    ..BorderSide::default()
                });
            }
            if states.contains(&WidgetState::Selected) {
                return Some(BorderSide {
                    width: 0.0,
                    color: Colors::TRANSPARENT,
                    ..BorderSide::default()
                });
            }
            if states.contains(&WidgetState::Error) {
                return Some(BorderSide {
                    width: 2.0,
                    color: error,
                    ..BorderSide::default()
                });
            }
            if states.contains(&WidgetState::Pressed) {
                return Some(BorderSide {
                    width: 2.0,
                    color: on_surface,
                    ..BorderSide::default()
                });
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(BorderSide {
                    width: 2.0,
                    color: on_surface,
                    ..BorderSide::default()
                });
            }
            if states.contains(&WidgetState::Focused) {
                return Some(BorderSide {
                    width: 2.0,
                    color: on_surface,
                    ..BorderSide::default()
                });
            }
            Some(BorderSide {
                width: 2.0,
                color: on_surface_variant,
                ..BorderSide::default()
            })
        });
        Some(side.into_any())
    }

    fn fill_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let on_surface = colors.on_surface;
        let error = colors.error;
        let primary = colors.primary;
        let fill_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Disabled) {
                if states.contains(&WidgetState::Selected) {
                    return Some(on_surface.with_opacity(0.38));
                }
                return Some(Colors::TRANSPARENT);
            }
            if states.contains(&WidgetState::Selected) {
                if states.contains(&WidgetState::Error) {
                    return Some(error);
                }
                return Some(primary);
            }
            Some(Colors::TRANSPARENT)
        });
        Some(fill_color)
    }

    fn check_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let surface = colors.surface;
        let on_error = colors.on_error;
        let on_primary = colors.on_primary;
        let check_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Disabled) {
                if states.contains(&WidgetState::Selected) {
                    return Some(surface);
                }
                return Some(Colors::TRANSPARENT); // No icons available when the checkbox is unselected.
            }
            if states.contains(&WidgetState::Selected) {
                if states.contains(&WidgetState::Error) {
                    return Some(on_error);
                }
                return Some(on_primary);
            }
            Some(Colors::TRANSPARENT) // No icons available when the checkbox is unselected.
        });
        Some(check_color)
    }

    fn overlay_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let on_surface = colors.on_surface;
        let error = colors.error;
        let primary = colors.primary;
        let overlay_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Error) {
                if states.contains(&WidgetState::Pressed) {
                    return Some(error.with_opacity(0.1));
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(error.with_opacity(0.08));
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(error.with_opacity(0.1));
                }
            }
            if states.contains(&WidgetState::Selected) {
                if states.contains(&WidgetState::Pressed) {
                    return Some(on_surface.with_opacity(0.1));
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(primary.with_opacity(0.08));
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(primary.with_opacity(0.1));
                }
                return Some(Colors::TRANSPARENT);
            }
            if states.contains(&WidgetState::Pressed) {
                return Some(primary.with_opacity(0.1));
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(on_surface.with_opacity(0.08));
            }
            if states.contains(&WidgetState::Focused) {
                return Some(on_surface.with_opacity(0.1));
            }
            Some(Colors::TRANSPARENT)
        });
        Some(overlay_color)
    }

    fn splash_radius(&self, _app: &mut App) -> Option<f64> {
        Some(40.0 / 2.0)
    }

    fn material_tap_target_size(&self, app: &mut App) -> Option<MaterialTapTargetSize> {
        let theme = self.context.theme(app);
        Some(theme.material_tap_target_size)
    }

    fn visual_density(&self, _app: &mut App) -> Option<VisualDensity> {
        Some(VisualDensity::STANDARD)
    }

    fn shape(&self, _app: &mut App) -> Option<OutlinedBorderRef> {
        Some(OutlinedBorderRef::new(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::all(Radius::circular(2.0)),
        )))
    }
}
