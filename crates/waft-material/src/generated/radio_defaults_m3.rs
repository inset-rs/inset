//! Flutter counterpart: material_ui's `generated/radio_defaults_m3.g.dart`, a part of
//! `radio.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use waft_embedder::Color;
use waft_foundation::App;
use waft_widgets::{BuildContext, WidgetState, WidgetStateProperty};

use crate::colors::Colors;
use crate::radio_theme::{RadioThemeData, RadioThemeDataGetters};
use crate::theme_data::MaterialTapTargetSize;
use crate::theme_data::VisualDensity;
use crate::theme_data_getters::DefaultsContext;
use waft_widgets::WidgetStatePropertyRef;

/// Dart's `_RadioDefaultsM3`.
pub(crate) struct RadioDefaultsM3 {
    /// Dart's `super(..)` arguments.
    radio_theme_data: RadioThemeData,
    context: DefaultsContext,
}

impl RadioDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> RadioDefaultsM3 {
        RadioDefaultsM3 {
            radio_theme_data: RadioThemeData::new(),
            context: DefaultsContext::new(context),
        }
    }
}

impl RadioThemeDataGetters for RadioDefaultsM3 {
    fn radio_theme_data(&self) -> &RadioThemeData {
        &self.radio_theme_data
    }

    fn fill_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some({
            let colors = colors.clone();
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Selected) {
                    if states.contains(&WidgetState::Disabled) {
                        return Some(colors.on_surface().with_opacity(0.38));
                    }
                    if states.contains(&WidgetState::Pressed) {
                        return Some(colors.primary());
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(colors.primary());
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(colors.primary());
                    }
                    return Some(colors.primary());
                }
                if states.contains(&WidgetState::Disabled) {
                    return Some(colors.on_surface().with_opacity(0.38));
                }
                if states.contains(&WidgetState::Pressed) {
                    return Some(colors.on_surface());
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(colors.on_surface());
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(colors.on_surface());
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
                        return Some(colors.on_surface().with_opacity(0.1));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(colors.primary().with_opacity(0.08));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(colors.primary().with_opacity(0.1));
                    }
                    return Some(Colors::TRANSPARENT);
                }
                if states.contains(&WidgetState::Pressed) {
                    return Some(colors.primary().with_opacity(0.1));
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(colors.on_surface().with_opacity(0.08));
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(colors.on_surface().with_opacity(0.1));
                }
                Some(Colors::TRANSPARENT)
            },
        ))
    }

    fn material_tap_target_size(&self, app: &mut App) -> Option<MaterialTapTargetSize> {
        let theme = self.context.theme(app);
        Some(theme.material_tap_target_size)
    }

    fn visual_density(&self, app: &mut App) -> Option<VisualDensity> {
        let theme = self.context.theme(app);
        Some(theme.visual_density)
    }

    fn background_color(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        Some(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
            Colors::TRANSPARENT,
        )))
    }
}
