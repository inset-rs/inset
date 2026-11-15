//! Flutter counterpart: material_ui's `generated/filled_button_defaults_m3.g.dart`, a part
//! of `filled_button.dart`.
//!
//! Dart's private `_FilledButtonDefaultsM3` is a `ButtonStyle` whose getters read the
//! context's theme; [`filled_button_defaults_m3`] returns the style those getters answer,
//! read once from the theme as the button's `default_style_of` asks for it.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use std::fmt::Debug;
use std::rc::Rc;

use waft_embedder::Size;
use waft_foundation::App;
use waft_painting::{AlignmentGeometry, BorderSide, OutlinedBorder, StadiumBorder};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, WidgetState, WidgetStateMouseCursor, WidgetStateProperty, WidgetStatePropertyRef,
};

use crate::button_style::ButtonStyle;
use crate::colors::Colors;
use crate::constants::K_THEME_CHANGE_DURATION;
use crate::filled_button::scaled_padding;
use crate::material_state::MaterialStatePropertyAll;
use crate::theme::Theme;

/// Dart's `_FilledButtonDefaultsM3(context)`.
pub(crate) fn filled_button_defaults_m3(app: &mut App, context: BuildContext) -> ButtonStyle {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;
    let (primary, on_primary, on_surface) = (colors.primary, colors.on_primary, colors.on_surface);
    ButtonStyle::new()
        .animation_duration(K_THEME_CHANGE_DURATION)
        .enable_feedback(true)
        .alignment(AlignmentGeometry::CENTER)
        .text_style(Rc::new(MaterialStatePropertyAll::new(
            theme.text_theme.label_large.clone(),
        )))
        .background_color(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Disabled) {
                return Some(on_surface.with_opacity(0.12));
            }
            Some(primary)
        }))
        .foreground_color(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Disabled) {
                return Some(on_surface.with_opacity(0.38));
            }
            Some(on_primary)
        }))
        .overlay_color(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Pressed) {
                return Some(on_primary.with_opacity(0.1));
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(on_primary.with_opacity(0.08));
            }
            if states.contains(&WidgetState::Focused) {
                return Some(on_primary.with_opacity(0.1));
            }
            None
        }))
        .shadow_color(all(colors.shadow()))
        .surface_tint_color(all(Colors::TRANSPARENT))
        .elevation(<dyn WidgetStateProperty<_>>::resolve_with(|states| {
            if states.contains(&WidgetState::Disabled) {
                return Some(0.0);
            }
            if states.contains(&WidgetState::Pressed) {
                return Some(0.0);
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(1.0);
            }
            if states.contains(&WidgetState::Focused) {
                return Some(0.0);
            }
            Some(0.0)
        }))
        .padding(all(scaled_padding(app, context)))
        .minimum_size(all(Size::new(64.0, 40.0)))
        // No default fixedSize
        .icon_size(all(18.0))
        .icon_color(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Disabled) {
                return Some(on_surface.with_opacity(0.38));
            }
            if states.contains(&WidgetState::Pressed) {
                return Some(on_primary);
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(on_primary);
            }
            if states.contains(&WidgetState::Focused) {
                return Some(on_primary);
            }
            Some(on_primary)
        }))
        .maximum_size(all(Size::INFINITE))
        // No default side
        .shape(all(
            Box::new(StadiumBorder::new(BorderSide::NONE)) as Box<dyn OutlinedBorder>
        ))
        .mouse_cursor(adaptive_clickable())
        .visual_density(theme.visual_density)
        .tap_target_size(theme.material_tap_target_size)
        .splash_factory(theme.splash_factory.clone())
}

/// Dart's `const MaterialStatePropertyAll<T>(value)`.
fn all<T: Clone + Debug + 'static>(value: T) -> WidgetStatePropertyRef<Option<T>> {
    Rc::new(MaterialStatePropertyAll::new(Some(value)))
}

/// Dart's `WidgetStateMouseCursor.adaptiveClickable` as a `WidgetStateProperty<MouseCursor?>`.
fn adaptive_clickable() -> WidgetStatePropertyRef<Option<MouseCursorRef>> {
    let cursor = WidgetStateMouseCursor::adaptive_clickable();
    <dyn WidgetStateProperty<_>>::resolve_with(move |states| Some(cursor.resolve(states)))
}
