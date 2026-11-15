//! Flutter counterpart: material_ui's `generated/filled_icon_button_defaults_m3.g.dart`, a Dart `part`
//! of `icon_button.dart`: `IconButton`'s Material 3 build reads it.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use std::rc::Rc;

use waft_embedder::{Color, Size};
use waft_foundation::App;
use waft_painting::{AlignmentGeometry, EdgeInsetsGeometry, OutlinedBorder, StadiumBorder};
use waft_services::MouseCursorRef;
use waft_widgets::{BuildContext, WidgetState, WidgetStateMouseCursor, WidgetStateProperty};

use crate::button_style::ButtonStyle;
use crate::colors::Colors;
use crate::constants::K_THEME_CHANGE_DURATION;
use crate::material_state::MaterialStatePropertyAll;
use crate::theme::Theme;
use crate::theme_data::VisualDensity;

/// Dart's `_FilledIconButtonDefaultsM3(context, toggleable)`: the [`ButtonStyle`] whose fields
/// are the values of that subclass's getters.
pub(crate) fn filled_icon_button_defaults_m3(
    app: &mut App,
    context: BuildContext,
    toggleable: bool,
) -> ButtonStyle {
    let theme = Theme::of(app, context);
    let colors = theme.color_scheme.clone();
    let adaptive_clickable = WidgetStateMouseCursor::adaptive_clickable();

    ButtonStyle::new()
        .animation_duration(K_THEME_CHANGE_DURATION)
        .enable_feedback(true)
        .alignment(AlignmentGeometry::CENTER)
        // No default text style
        .background_color({
            let colors = colors.clone();
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Disabled) {
                    return Some(colors.on_surface.with_opacity(0.12));
                }
                if states.contains(&WidgetState::Selected) {
                    return Some(colors.primary);
                }
                if toggleable {
                    // toggleable but unselected case
                    return Some(colors.surface_container_highest());
                }
                Some(colors.primary)
            })
        })
        .foreground_color({
            let colors = colors.clone();
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Disabled) {
                    return Some(colors.on_surface.with_opacity(0.38));
                }
                if states.contains(&WidgetState::Selected) {
                    return Some(colors.on_primary);
                }
                if toggleable {
                    // toggleable but unselected case
                    return Some(colors.primary);
                }
                Some(colors.on_primary)
            })
        })
        .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states| {
                if states.contains(&WidgetState::Selected) {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(colors.on_primary.with_opacity(0.1));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(colors.on_primary.with_opacity(0.08));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(colors.on_primary.with_opacity(0.1));
                    }
                }
                if toggleable {
                    // toggleable but unselected case
                    if states.contains(&WidgetState::Pressed) {
                        return Some(colors.primary.with_opacity(0.1));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(colors.primary.with_opacity(0.08));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(colors.primary.with_opacity(0.1));
                    }
                }
                if states.contains(&WidgetState::Pressed) {
                    return Some(colors.on_primary.with_opacity(0.1));
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(colors.on_primary.with_opacity(0.08));
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(colors.on_primary.with_opacity(0.1));
                }
                Some(Colors::TRANSPARENT)
            },
        ))
        .elevation(Rc::new(MaterialStatePropertyAll::new(Some(0.0))))
        .shadow_color(Rc::new(MaterialStatePropertyAll::new(Some(
            Colors::TRANSPARENT,
        ))))
        .surface_tint_color(Rc::new(MaterialStatePropertyAll::new(Some(
            Colors::TRANSPARENT,
        ))))
        .padding(Rc::new(MaterialStatePropertyAll::new(Some(
            EdgeInsetsGeometry::all(8.0),
        ))))
        .minimum_size(Rc::new(MaterialStatePropertyAll::new(Some(Size::new(
            40.0, 40.0,
        )))))
        // No default fixedSize
        .maximum_size(Rc::new(MaterialStatePropertyAll::new(Some(Size::INFINITE))))
        .icon_size(Rc::new(MaterialStatePropertyAll::new(Some(24.0))))
        // `side` is null.
        .shape(Rc::new(MaterialStatePropertyAll::new(Some(
            Box::new(StadiumBorder::default()) as Box<dyn OutlinedBorder>,
        ))))
        .mouse_cursor(
            <dyn WidgetStateProperty<Option<MouseCursorRef>>>::resolve_with(move |states| {
                Some(adaptive_clickable.resolve(states))
            }),
        )
        .visual_density(VisualDensity::STANDARD)
        .tap_target_size(theme.material_tap_target_size)
        .splash_factory(theme.splash_factory.clone())
}
