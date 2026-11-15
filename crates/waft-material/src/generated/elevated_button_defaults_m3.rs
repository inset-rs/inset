//! Flutter counterpart: material_ui's `generated/elevated_button_defaults_m3.g.dart`, a part of
//! `elevated_button.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use std::fmt::Debug;
use std::rc::Rc;

use waft_embedder::Size;
use waft_foundation::App;
use waft_painting::{AlignmentGeometry, BorderSide, OutlinedBorderRef, StadiumBorder};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, WidgetState, WidgetStateMouseCursor, WidgetStateProperty, WidgetStatePropertyRef,
};

use crate::button_style::{ButtonStyle, ButtonStyleGetters};
use crate::colors::Colors;
use crate::constants::K_THEME_CHANGE_DURATION;
use crate::elevated_button::scaled_padding;
use crate::ink_well::InteractiveInkFeatureFactoryRef;
use crate::material_state::MaterialStatePropertyAll;
use crate::theme_data::MaterialTapTargetSize;
use crate::theme_data::VisualDensity;
use crate::theme_data_getters::DefaultsContext;
use waft_embedder::Color;
use waft_painting::EdgeInsetsGeometry;
use waft_painting::TextStyle;

/// Dart's `_ElevatedButtonDefaultsM3`.
pub(crate) struct ElevatedButtonDefaultsM3 {
    /// Dart's `super(..)` arguments.
    button_style: ButtonStyle,
    context: DefaultsContext,
}

impl ElevatedButtonDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> ElevatedButtonDefaultsM3 {
        ElevatedButtonDefaultsM3 {
            button_style: ButtonStyle::new()
                .animation_duration(K_THEME_CHANGE_DURATION)
                .enable_feedback(true)
                .alignment(AlignmentGeometry::CENTER),
            context: DefaultsContext::new(context),
        }
    }
}

impl ButtonStyleGetters for ElevatedButtonDefaultsM3 {
    fn button_style(&self) -> &ButtonStyle {
        &self.button_style
    }

    fn text_style(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<TextStyle>>> {
        let theme = self.context.theme(app);
        Some(Rc::new(MaterialStatePropertyAll::new(
            theme.text_theme.label_large.clone(),
        )))
    }

    fn background_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let on_surface = colors.on_surface;
        let surface_container_low = colors.surface_container_low();
        Some(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Disabled) {
                return Some(on_surface.with_opacity(0.12));
            }
            Some(surface_container_low)
        }))
    }

    fn foreground_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let primary = colors.primary;
        let on_surface = colors.on_surface;
        Some(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Disabled) {
                return Some(on_surface.with_opacity(0.38));
            }
            Some(primary)
        }))
    }

    fn overlay_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let primary = colors.primary;
        Some(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Pressed) {
                return Some(primary.with_opacity(0.1));
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(primary.with_opacity(0.08));
            }
            if states.contains(&WidgetState::Focused) {
                return Some(primary.with_opacity(0.1));
            }
            None
        }))
    }

    fn shadow_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(all(colors.shadow()))
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        Some(all(Colors::TRANSPARENT))
    }

    fn elevation(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<f64>>> {
        Some(<dyn WidgetStateProperty<_>>::resolve_with(|states| {
            if states.contains(&WidgetState::Disabled) {
                return Some(0.0);
            }
            if states.contains(&WidgetState::Pressed) {
                return Some(1.0);
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(3.0);
            }
            if states.contains(&WidgetState::Focused) {
                return Some(1.0);
            }
            Some(1.0)
        }))
    }

    fn padding(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>> {
        Some(all(scaled_padding(app, self.context.context())))
    }

    fn minimum_size(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Size>>> {
        Some(all(Size::new(64.0, 40.0)))
    }

    // No default fixedSize
    fn icon_size(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<f64>>> {
        Some(all(18.0))
    }

    fn icon_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let primary = colors.primary;
        let on_surface = colors.on_surface;
        Some(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Disabled) {
                return Some(on_surface.with_opacity(0.38));
            }
            if states.contains(&WidgetState::Pressed) {
                return Some(primary);
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(primary);
            }
            if states.contains(&WidgetState::Focused) {
                return Some(primary);
            }
            Some(primary)
        }))
    }

    fn maximum_size(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Size>>> {
        Some(all(Size::INFINITE))
    }

    // No default side
    fn shape(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<OutlinedBorderRef>>> {
        Some(all(OutlinedBorderRef::new(StadiumBorder::new(
            BorderSide::NONE,
        ))))
    }

    fn mouse_cursor(
        &self,
        _app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<MouseCursorRef>>> {
        Some(adaptive_clickable())
    }

    fn visual_density(&self, app: &mut App) -> Option<VisualDensity> {
        let theme = self.context.theme(app);
        Some(theme.visual_density)
    }

    fn tap_target_size(&self, app: &mut App) -> Option<MaterialTapTargetSize> {
        let theme = self.context.theme(app);
        Some(theme.material_tap_target_size)
    }

    fn splash_factory(&self, app: &mut App) -> Option<InteractiveInkFeatureFactoryRef> {
        let theme = self.context.theme(app);
        Some(theme.splash_factory.clone())
    }
}

/// Dart's `const MaterialStatePropertyAll<T>(value)`.
fn all<T: Clone + Debug + PartialEq + 'static>(value: T) -> WidgetStatePropertyRef<Option<T>> {
    Rc::new(MaterialStatePropertyAll::new(Some(value)))
}

/// Dart's `WidgetStateMouseCursor.adaptiveClickable` as a `WidgetStateProperty<MouseCursor?>`.
fn adaptive_clickable() -> WidgetStatePropertyRef<Option<MouseCursorRef>> {
    let cursor = WidgetStateMouseCursor::adaptive_clickable();
    <dyn WidgetStateProperty<_>>::resolve_with(move |states| Some(cursor.resolve(states)))
}
