//! Flutter counterpart: material_ui's `generated/switch_defaults_m3.g.dart`, a part of
//! `switch.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use waft_embedder::{Color, Size};
use waft_foundation::App;
use waft_painting::{BoxShadow, EdgeInsetsGeometry};
use waft_widgets::{
    BuildContext, WidgetState, WidgetStateMouseCursor, WidgetStateProperty, WidgetStatePropertyRef,
    WidgetStates,
};

use crate::color_scheme::ColorScheme;
use crate::colors::Colors;
use crate::constants::K_MIN_INTERACTIVE_DIMENSION;
use crate::shadows::k_elevation_to_shadow;
use crate::switch::SwitchConfig;
use crate::switch_theme::{SwitchThemeData, SwitchThemeDataGetters};
use crate::theme::Theme;
use crate::theme_data_getters::DefaultsContext;
use waft_services::MouseCursorRef;

/// Dart's `_SwitchDefaultsM3`.
pub(crate) struct SwitchDefaultsM3 {
    /// Dart's `super(..)` arguments.
    switch_theme_data: SwitchThemeData,
    context: DefaultsContext,
}

impl SwitchDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> SwitchDefaultsM3 {
        SwitchDefaultsM3 {
            switch_theme_data: SwitchThemeData::new(),
            context: DefaultsContext::new(context),
        }
    }
}

impl SwitchThemeDataGetters for SwitchDefaultsM3 {
    fn switch_theme_data(&self) -> &SwitchThemeData {
        &self.switch_theme_data
    }

    fn thumb_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let colors = self.context.colors(app).clone();
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Disabled) {
                    if states.contains(&WidgetState::Selected) {
                        return Some(colors.surface);
                    }
                    return Some(colors.on_surface.with_opacity(0.38));
                }
                if states.contains(&WidgetState::Selected) {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(colors.primary_container());
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(colors.primary_container());
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(colors.primary_container());
                    }
                    return Some(colors.on_primary);
                }
                if states.contains(&WidgetState::Pressed) {
                    return Some(colors.on_surface_variant());
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(colors.on_surface_variant());
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(colors.on_surface_variant());
                }
                Some(colors.outline())
            },
        ))
    }

    fn track_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let colors = self.context.colors(app).clone();
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Disabled) {
                    if states.contains(&WidgetState::Selected) {
                        return Some(colors.on_surface.with_opacity(0.12));
                    }
                    return Some(colors.surface_container_highest().with_opacity(0.12));
                }
                if states.contains(&WidgetState::Selected) {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(colors.primary);
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(colors.primary);
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(colors.primary);
                    }
                    return Some(colors.primary);
                }
                if states.contains(&WidgetState::Pressed) {
                    return Some(colors.surface_container_highest());
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(colors.surface_container_highest());
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(colors.surface_container_highest());
                }
                Some(colors.surface_container_highest())
            },
        ))
    }

    fn track_outline_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let colors = self.context.colors(app).clone();
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    return Some(Colors::TRANSPARENT);
                }
                if states.contains(&WidgetState::Disabled) {
                    return Some(colors.on_surface.with_opacity(0.12));
                }
                Some(colors.outline())
            },
        ))
    }

    fn overlay_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let colors = self.context.colors(app).clone();
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(colors.primary.with_opacity(0.1));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(colors.primary.with_opacity(0.08));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(colors.primary.with_opacity(0.1));
                    }
                    return None;
                }
                if states.contains(&WidgetState::Pressed) {
                    return Some(colors.on_surface.with_opacity(0.1));
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(colors.on_surface.with_opacity(0.08));
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(colors.on_surface.with_opacity(0.1));
                }
                None
            },
        ))
    }

    fn mouse_cursor(
        &self,
        _app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<MouseCursorRef>>> {
        Some(<dyn WidgetStateProperty<_>>::resolve_with(
            |states: &WidgetStates| Some(WidgetStateMouseCursor::clickable().resolve(states)),
        ))
    }

    fn track_outline_width(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<f64>>> {
        Some(<dyn WidgetStateProperty<Option<f64>>>::all(Some(2.0)))
    }

    fn splash_radius(&self, _app: &mut App) -> Option<f64> {
        Some(40.0 / 2.0)
    }

    fn padding(&self, _app: &mut App) -> Option<EdgeInsetsGeometry> {
        Some(EdgeInsetsGeometry::symmetric(0.0, 4.0))
    }
}

/// Dart's `_SwitchConfigM3`.
pub(crate) struct SwitchConfigM3 {
    colors: ColorScheme,
}

impl SwitchConfigM3 {
    /// Dart's `_SwitchConfigM3(context)`.
    pub(crate) fn new(app: &mut App, context: BuildContext) -> SwitchConfigM3 {
        SwitchConfigM3 {
            colors: Theme::of(app, context).color_scheme.clone(),
        }
    }

    pub(crate) const ICON_SIZE: f64 = 16.0;
}

impl SwitchConfig for SwitchConfigM3 {
    fn active_thumb_radius(&self) -> f64 {
        24.0 / 2.0
    }

    fn icon_color(&self) -> WidgetStatePropertyRef<Color> {
        let colors = self.colors.clone();
        <dyn WidgetStateProperty<Color>>::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) {
                if states.contains(&WidgetState::Selected) {
                    return colors.on_surface.with_opacity(0.38);
                }
                return colors.surface_container_highest().with_opacity(0.38);
            }
            if states.contains(&WidgetState::Selected) {
                if states.contains(&WidgetState::Pressed) {
                    return colors.on_primary_container();
                }
                if states.contains(&WidgetState::Hovered) {
                    return colors.on_primary_container();
                }
                if states.contains(&WidgetState::Focused) {
                    return colors.on_primary_container();
                }
                return colors.on_primary_container();
            }
            if states.contains(&WidgetState::Pressed) {
                return colors.surface_container_highest();
            }
            if states.contains(&WidgetState::Hovered) {
                return colors.surface_container_highest();
            }
            if states.contains(&WidgetState::Focused) {
                return colors.surface_container_highest();
            }
            colors.surface_container_highest()
        })
    }

    fn inactive_thumb_radius(&self) -> f64 {
        16.0 / 2.0
    }

    fn pressed_thumb_radius(&self) -> f64 {
        28.0 / 2.0
    }

    fn switch_height(&self) -> f64 {
        self.switch_min_size().height() + 8.0
    }

    fn switch_height_collapsed(&self) -> f64 {
        self.switch_min_size().height()
    }

    fn switch_width(&self) -> f64 {
        52.0
    }

    fn thumb_radius_with_icon(&self) -> f64 {
        24.0 / 2.0
    }

    fn thumb_shadow(&self) -> Option<Vec<BoxShadow>> {
        k_elevation_to_shadow(0).map(<[BoxShadow]>::to_vec)
    }

    fn track_height(&self) -> f64 {
        32.0
    }

    fn track_width(&self) -> f64 {
        52.0
    }

    // The thumb size at the middle of the track. Hand coded default based on the animation specs.
    fn transitional_thumb_size(&self) -> Size {
        Size::new(34.0, 22.0)
    }

    // Hand coded default based on the animation specs.
    fn toggle_duration(&self) -> u64 {
        300
    }

    // Hand coded default based on the animation specs.
    fn thumb_offset(&self) -> Option<f64> {
        None
    }

    fn switch_min_size(&self) -> Size {
        Size::new(
            K_MIN_INTERACTIVE_DIMENSION,
            K_MIN_INTERACTIVE_DIMENSION - 8.0,
        )
    }
}
