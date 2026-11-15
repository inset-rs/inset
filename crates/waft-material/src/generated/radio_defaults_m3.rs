//! Flutter counterpart: material_ui's `generated/radio_defaults_m3.g.dart`, a Dart `part` of
//! `radio.dart`: `Radio`'s Material 3 paint reads it.
//!
//! Dart's private `_RadioDefaultsM3` is a `RadioThemeData` whose getters read the context's
//! theme; [`radio_defaults_m3`] returns the data those getters answer, read once from the theme.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use waft_embedder::Color;
use waft_foundation::App;
use waft_widgets::{BuildContext, WidgetState, WidgetStateProperty};

use crate::colors::Colors;
use crate::radio_theme::RadioThemeData;
use crate::theme::Theme;

/// Dart's `_RadioDefaultsM3(context)`.
pub(crate) fn radio_defaults_m3(app: &mut App, context: BuildContext) -> RadioThemeData {
    let theme = Theme::of(app, context);
    let colors = theme.color_scheme.clone();

    RadioThemeData::new()
        .fill_color({
            let colors = colors.clone();
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Selected) {
                    if states.contains(&WidgetState::Disabled) {
                        return Some(colors.on_surface.with_opacity(0.38));
                    }
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
                if states.contains(&WidgetState::Disabled) {
                    return Some(colors.on_surface.with_opacity(0.38));
                }
                if states.contains(&WidgetState::Pressed) {
                    return Some(colors.on_surface);
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(colors.on_surface);
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(colors.on_surface);
                }
                Some(colors.on_surface_variant())
            })
        })
        .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states| {
                if states.contains(&WidgetState::Selected) {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(colors.on_surface.with_opacity(0.1));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(colors.primary.with_opacity(0.08));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(colors.primary.with_opacity(0.1));
                    }
                    return Some(Colors::TRANSPARENT);
                }
                if states.contains(&WidgetState::Pressed) {
                    return Some(colors.primary.with_opacity(0.1));
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(colors.on_surface.with_opacity(0.08));
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(colors.on_surface.with_opacity(0.1));
                }
                Some(Colors::TRANSPARENT)
            },
        ))
        .material_tap_target_size(theme.material_tap_target_size)
        .visual_density(theme.visual_density)
        .background_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
            Colors::TRANSPARENT,
        )))
}
