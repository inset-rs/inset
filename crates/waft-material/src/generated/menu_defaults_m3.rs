//! Flutter counterpart: material_ui's `generated/menu_defaults_m3.g.dart`, a Dart `part` of
//! `menu_anchor.dart`.
//!
//! Dart's private `_MenuBarDefaultsM3`, `_MenuButtonDefaultsM3` and `_MenuDefaultsM3` are a
//! `MenuStyle` or `ButtonStyle` whose getters read the context's theme; each function here
//! returns the style those getters answer, read once from the theme.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use std::fmt::Debug;
use std::rc::Rc;

use waft_embedder::{Radius, Size};
use waft_foundation::App;
use waft_painting::{
    AlignmentGeometry, BorderRadiusGeometry, BorderSide, EdgeInsetsGeometry, OutlinedBorder,
    RoundedRectangleBorder,
};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, MediaQuery, WidgetState, WidgetStateMouseCursor, WidgetStateProperty,
    WidgetStatePropertyRef,
};

use crate::button_style::ButtonStyle;
use crate::button_style_button::ButtonStyleButton;
use crate::colors::Colors;
use crate::constants::K_THEME_CHANGE_DURATION;
use crate::material_state::MaterialStatePropertyAll;
use crate::menu_anchor::{
    K_LABEL_ITEM_DEFAULT_SPACING, K_MENU_VERTICAL_MIN_PADDING, K_MENU_VIEW_PADDING,
    K_TOP_LEVEL_MENU_HORIZONTAL_MIN_PADDING,
};
use crate::menu_style::MenuStyle;
use crate::theme::Theme;
use crate::theme_data::VisualDensity;

/// Dart's `_MenuBarDefaultsM3(context)`.
pub(crate) fn menu_bar_defaults_m3(app: &mut App, context: BuildContext) -> MenuStyle {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;
    MenuStyle::new()
        .elevation(all(3.0))
        .shape(all(default_menu_border()))
        .alignment(AlignmentGeometry::BOTTOM_START)
        .background_color(all(colors.surface_container()))
        .shadow_color(all(colors.shadow()))
        .surface_tint_color(all(Colors::TRANSPARENT))
        .padding(all(EdgeInsetsGeometry::directional(
            K_TOP_LEVEL_MENU_HORIZONTAL_MIN_PADDING,
            0.0,
            K_TOP_LEVEL_MENU_HORIZONTAL_MIN_PADDING,
            0.0,
        )))
        .visual_density(theme.visual_density)
}

/// Dart's `_MenuButtonDefaultsM3(context)`.
pub(crate) fn menu_button_defaults_m3(app: &mut App, context: BuildContext) -> ButtonStyle {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;
    let (on_surface, on_surface_variant) = (colors.on_surface, colors.on_surface_variant());
    ButtonStyle::new()
        .animation_duration(K_THEME_CHANGE_DURATION)
        .enable_feedback(true)
        .alignment(AlignmentGeometry::CENTER_START)
        .background_color(all(Colors::TRANSPARENT))
        // No default shadow color
        // No default surface tint color
        .elevation(all(0.0))
        .foreground_color(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Disabled) {
                return Some(on_surface.with_opacity(0.38));
            }
            if states.contains(&WidgetState::Pressed) {
                return Some(on_surface);
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(on_surface);
            }
            if states.contains(&WidgetState::Focused) {
                return Some(on_surface);
            }
            Some(on_surface)
        }))
        .icon_color(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Disabled) {
                return Some(on_surface.with_opacity(0.38));
            }
            if states.contains(&WidgetState::Pressed) {
                return Some(on_surface_variant);
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(on_surface_variant);
            }
            if states.contains(&WidgetState::Focused) {
                return Some(on_surface_variant);
            }
            Some(on_surface_variant)
        }))
        // No default fixedSize
        .icon_size(all(24.0))
        .maximum_size(all(Size::INFINITE))
        .minimum_size(all(Size::new(64.0, 48.0)))
        .mouse_cursor(adaptive_clickable())
        .overlay_color(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Pressed) {
                return Some(on_surface.with_opacity(0.1));
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(on_surface.with_opacity(0.08));
            }
            if states.contains(&WidgetState::Focused) {
                return Some(on_surface.with_opacity(0.1));
            }
            Some(Colors::TRANSPARENT)
        }))
        .padding(all(scaled_padding(app, context)))
        // No default side
        .shape(all(
            Box::new(RoundedRectangleBorder::default()) as Box<dyn OutlinedBorder>
        ))
        .splash_factory(theme.splash_factory.clone())
        .tap_target_size(theme.material_tap_target_size)
        // TODO(tahatesser): This is taken from https://m3.material.io/components/menus/specs
        // Update this when the token is available.
        .text_style(Rc::new(MaterialStatePropertyAll::new(
            theme.text_theme.label_large.clone(),
        )))
        .visual_density(theme.visual_density)
}

// The horizontal padding number comes from the spec.
fn scaled_padding(app: &mut App, context: BuildContext) -> EdgeInsetsGeometry {
    let theme = Theme::of(app, context);
    let mut visual_density = theme.visual_density;
    // When horizontal VisualDensity is greater than zero, set it to zero
    // because the [ButtonStyleButton] has already handle the padding based on the density.
    // However, the [ButtonStyleButton] doesn't allow the [VisualDensity] adjustment
    // to reduce the width of the left/right padding, so we need to handle it here if
    // the density is less than zero, such as on desktop platforms.
    if visual_density.horizontal > 0.0 {
        visual_density = VisualDensity::new(0.0, visual_density.vertical);
    }
    // Since the threshold paddings used below are empirical values determined
    // at a font size of 14.0, 14.0 is used as the base value for scaling the
    // padding.
    let font_size = theme
        .text_theme
        .label_large
        .as_ref()
        .and_then(|label_large| label_large.font_size)
        .unwrap_or(14.0);
    let font_size_ratio = MediaQuery::text_scaler_of(app, context).scale(font_size) / 14.0;
    let dx = visual_density.base_size_adjustment().dx();
    <dyn ButtonStyleButton>::scaled_padding(
        EdgeInsetsGeometry::symmetric(
            0.0,
            K_MENU_VIEW_PADDING.max(K_LABEL_ITEM_DEFAULT_SPACING + dx),
        ),
        EdgeInsetsGeometry::symmetric(0.0, K_MENU_VIEW_PADDING.max(8.0 + dx)),
        EdgeInsetsGeometry::symmetric(0.0, K_MENU_VIEW_PADDING),
        font_size_ratio,
    )
}

/// Dart's `_MenuDefaultsM3(context)`.
pub(crate) fn menu_defaults_m3(app: &mut App, context: BuildContext) -> MenuStyle {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;
    MenuStyle::new()
        .elevation(all(3.0))
        .shape(all(default_menu_border()))
        .alignment(AlignmentGeometry::TOP_END)
        .background_color(all(colors.surface_container()))
        .surface_tint_color(all(Colors::TRANSPARENT))
        .shadow_color(all(colors.shadow()))
        .padding(all(EdgeInsetsGeometry::directional(
            0.0,
            K_MENU_VERTICAL_MIN_PADDING,
            0.0,
            K_MENU_VERTICAL_MIN_PADDING,
        )))
        .visual_density(theme.visual_density)
}

/// Dart's `_MenuBarDefaultsM3._defaultMenuBorder` and `_MenuDefaultsM3._defaultMenuBorder`.
fn default_menu_border() -> Box<dyn OutlinedBorder> {
    Box::new(RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadiusGeometry::all(Radius::circular(4.0)),
    ))
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
