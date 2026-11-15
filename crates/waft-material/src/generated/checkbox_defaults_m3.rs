//! Flutter counterpart: material_ui's `generated/checkbox_defaults_m3.g.dart`, a Dart `part`
//! of `checkbox.dart`: `Checkbox`'s Material 3 build reads it.

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

use crate::checkbox_theme::CheckboxThemeData;
use crate::colors::Colors;
use crate::theme::Theme;
use crate::theme_data::VisualDensity;

/// Dart's `_CheckboxDefaultsM3(context)`: the [`CheckboxThemeData`] whose fields are the
/// values of that subclass's getters, read from the context's theme when it is made.
pub(crate) fn checkbox_defaults_m3(app: &mut App, context: BuildContext) -> CheckboxThemeData {
    let theme = Theme::of(app, context);
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

    CheckboxThemeData::new()
        .side(side)
        .fill_color(fill_color)
        .check_color(check_color)
        .overlay_color(overlay_color)
        .splash_radius(40.0 / 2.0)
        .material_tap_target_size(theme.material_tap_target_size)
        .visual_density(VisualDensity::STANDARD)
        .shape(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::all(Radius::circular(2.0)),
        ))
}
