//! Flutter counterpart: material_ui's `generated/icon_button_defaults_m3.g.dart`, a part of
//! `icon_button.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use std::rc::Rc;

use waft_embedder::{Color, Size};
use waft_foundation::App;
use waft_painting::{AlignmentGeometry, EdgeInsetsGeometry, OutlinedBorderRef, StadiumBorder};
use waft_services::MouseCursorRef;
use waft_widgets::{BuildContext, WidgetState, WidgetStateMouseCursor, WidgetStateProperty};

use crate::button_style::{ButtonStyle, ButtonStyleGetters};
use crate::colors::Colors;
use crate::constants::K_THEME_CHANGE_DURATION;
use crate::ink_well::InteractiveInkFeatureFactoryRef;
use crate::material_state::MaterialStatePropertyAll;
use crate::theme_data::MaterialTapTargetSize;
use crate::theme_data::VisualDensity;
use crate::theme_data_getters::DefaultsContext;
use waft_widgets::WidgetStatePropertyRef;

/// Dart's `_IconButtonDefaultsM3`.
pub(crate) struct IconButtonDefaultsM3 {
    /// Dart's `super(..)` arguments.
    button_style: ButtonStyle,
    context: DefaultsContext,
    #[expect(
        dead_code,
        reason = "Dart's constructor argument, which no getter of this class reads"
    )]
    toggleable: bool,
}

impl IconButtonDefaultsM3 {
    pub(crate) fn new(context: BuildContext, toggleable: bool) -> IconButtonDefaultsM3 {
        IconButtonDefaultsM3 {
            button_style: ButtonStyle::new()
                .animation_duration(K_THEME_CHANGE_DURATION)
                .enable_feedback(true)
                .alignment(AlignmentGeometry::CENTER),
            context: DefaultsContext::new(context),
            toggleable,
        }
    }
}

impl ButtonStyleGetters for IconButtonDefaultsM3 {
    fn button_style(&self) -> &ButtonStyle {
        &self.button_style
    }

    // No default text style
    fn background_color(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        Some(Rc::new(MaterialStatePropertyAll::new(Some(
            Colors::TRANSPARENT,
        ))))
    }

    fn foreground_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some({
            let colors = colors.clone();
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Disabled) {
                    return Some(colors.on_surface().with_opacity(0.38));
                }
                if states.contains(&WidgetState::Selected) {
                    return Some(colors.primary());
                }
                Some(colors.on_surface_variant())
            })
        })
    }

    fn overlay_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states| {
                if states.contains(&WidgetState::Selected) {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(colors.primary().with_opacity(0.1));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(colors.primary().with_opacity(0.08));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(colors.primary().with_opacity(0.1));
                    }
                }
                if states.contains(&WidgetState::Pressed) {
                    return Some(colors.on_surface_variant().with_opacity(0.1));
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(colors.on_surface_variant().with_opacity(0.08));
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(colors.on_surface_variant().with_opacity(0.1));
                }
                Some(Colors::TRANSPARENT)
            },
        ))
    }

    fn elevation(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<f64>>> {
        Some(Rc::new(MaterialStatePropertyAll::new(Some(0.0))))
    }

    fn shadow_color(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        Some(Rc::new(MaterialStatePropertyAll::new(Some(
            Colors::TRANSPARENT,
        ))))
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        Some(Rc::new(MaterialStatePropertyAll::new(Some(
            Colors::TRANSPARENT,
        ))))
    }

    fn padding(
        &self,
        _app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>> {
        Some(Rc::new(MaterialStatePropertyAll::new(Some(
            EdgeInsetsGeometry::all(8.0),
        ))))
    }

    fn minimum_size(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Size>>> {
        Some(Rc::new(MaterialStatePropertyAll::new(Some(Size::new(
            40.0, 40.0,
        )))))
    }

    // No default fixedSize
    fn maximum_size(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Size>>> {
        Some(Rc::new(MaterialStatePropertyAll::new(Some(Size::INFINITE))))
    }

    fn icon_size(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<f64>>> {
        Some(Rc::new(MaterialStatePropertyAll::new(Some(24.0))))
    }

    // `side` is null.
    fn shape(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<OutlinedBorderRef>>> {
        Some(Rc::new(MaterialStatePropertyAll::new(Some(
            OutlinedBorderRef::new(StadiumBorder::default()),
        ))))
    }

    fn mouse_cursor(
        &self,
        _app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<MouseCursorRef>>> {
        let adaptive_clickable = WidgetStateMouseCursor::adaptive_clickable();
        Some(
            <dyn WidgetStateProperty<Option<MouseCursorRef>>>::resolve_with(move |states| {
                Some(adaptive_clickable.resolve(states))
            }),
        )
    }

    fn visual_density(&self, _app: &mut App) -> Option<VisualDensity> {
        Some(VisualDensity::STANDARD)
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
