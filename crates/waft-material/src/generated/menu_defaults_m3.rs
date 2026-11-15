//! Flutter counterpart: material_ui's `generated/menu_defaults_m3.g.dart`, a part of
//! `menu_anchor.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use std::fmt::Debug;
use std::rc::Rc;

use waft_embedder::{Radius, Size};
use waft_foundation::App;
use waft_painting::{
    AlignmentGeometry, BorderRadiusGeometry, BorderSide, EdgeInsetsGeometry, OutlinedBorderRef,
    RoundedRectangleBorder,
};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, MediaQuery, WidgetState, WidgetStateMouseCursor, WidgetStateProperty,
    WidgetStatePropertyRef,
};

use crate::button_style::{ButtonStyle, ButtonStyleGetters};
use crate::button_style_button::ButtonStyleButton;
use crate::colors::Colors;
use crate::constants::K_THEME_CHANGE_DURATION;
use crate::ink_well::InteractiveInkFeatureFactoryRef;
use crate::material_state::MaterialStatePropertyAll;
use crate::menu_anchor::{
    K_LABEL_ITEM_DEFAULT_SPACING, K_MENU_VERTICAL_MIN_PADDING, K_MENU_VIEW_PADDING,
    K_TOP_LEVEL_MENU_HORIZONTAL_MIN_PADDING,
};
use crate::menu_style::{MenuStyle, MenuStyleGetters};
use crate::theme::Theme;
use crate::theme_data::MaterialTapTargetSize;
use crate::theme_data::VisualDensity;
use crate::theme_data_getters::DefaultsContext;
use waft_embedder::Color;
use waft_painting::TextStyle;

/// Dart's `_MenuBarDefaultsM3`.
pub(crate) struct MenuBarDefaultsM3 {
    /// Dart's `super(..)` arguments.
    menu_style: MenuStyle,
    context: DefaultsContext,
}

impl MenuBarDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> MenuBarDefaultsM3 {
        MenuBarDefaultsM3 {
            menu_style: MenuStyle::new()
                .elevation(all(3.0))
                .shape(all(default_menu_border()))
                .alignment(AlignmentGeometry::BOTTOM_START),
            context: DefaultsContext::new(context),
        }
    }
}

impl MenuStyleGetters for MenuBarDefaultsM3 {
    fn menu_style(&self) -> &MenuStyle {
        &self.menu_style
    }

    fn background_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(all(colors.surface_container()))
    }

    fn shadow_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(all(colors.shadow()))
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        Some(all(Colors::TRANSPARENT))
    }

    fn padding(
        &self,
        _app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>> {
        Some(all(EdgeInsetsGeometry::directional(
            K_TOP_LEVEL_MENU_HORIZONTAL_MIN_PADDING,
            0.0,
            K_TOP_LEVEL_MENU_HORIZONTAL_MIN_PADDING,
            0.0,
        )))
    }

    fn visual_density(&self, app: &mut App) -> Option<VisualDensity> {
        let theme = self.context.theme(app);
        Some(theme.visual_density)
    }
}

/// Dart's `_MenuButtonDefaultsM3`.
pub(crate) struct MenuButtonDefaultsM3 {
    /// Dart's `super(..)` arguments.
    button_style: ButtonStyle,
    context: DefaultsContext,
}

impl MenuButtonDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> MenuButtonDefaultsM3 {
        MenuButtonDefaultsM3 {
            button_style: ButtonStyle::new()
                .animation_duration(K_THEME_CHANGE_DURATION)
                .enable_feedback(true)
                .alignment(AlignmentGeometry::CENTER_START),
            context: DefaultsContext::new(context),
        }
    }
}

impl ButtonStyleGetters for MenuButtonDefaultsM3 {
    fn button_style(&self) -> &ButtonStyle {
        &self.button_style
    }

    fn background_color(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        Some(all(Colors::TRANSPARENT))
    }

    // No default shadow color
    // No default surface tint color
    fn elevation(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<f64>>> {
        Some(all(0.0))
    }

    fn foreground_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let on_surface = colors.on_surface;
        Some(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
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
    }

    fn icon_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let on_surface = colors.on_surface;
        let on_surface_variant = colors.on_surface_variant();
        Some(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
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
    }

    // No default fixedSize
    fn icon_size(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<f64>>> {
        Some(all(24.0))
    }

    fn maximum_size(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Size>>> {
        Some(all(Size::INFINITE))
    }

    fn minimum_size(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Size>>> {
        Some(all(Size::new(64.0, 48.0)))
    }

    fn mouse_cursor(
        &self,
        _app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<MouseCursorRef>>> {
        Some(adaptive_clickable())
    }

    fn overlay_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let on_surface = colors.on_surface;
        Some(<dyn WidgetStateProperty<_>>::resolve_with(move |states| {
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
    }

    fn padding(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>> {
        Some(all(scaled_padding(app, self.context.context())))
    }

    // No default side
    fn shape(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<OutlinedBorderRef>>> {
        Some(all(OutlinedBorderRef::new(
            RoundedRectangleBorder::default(),
        )))
    }

    fn splash_factory(&self, app: &mut App) -> Option<InteractiveInkFeatureFactoryRef> {
        let theme = self.context.theme(app);
        Some(theme.splash_factory.clone())
    }

    fn tap_target_size(&self, app: &mut App) -> Option<MaterialTapTargetSize> {
        let theme = self.context.theme(app);
        Some(theme.material_tap_target_size)
    }

    // TODO(tahatesser): This is taken from https://m3.material.io/components/menus/specs
    // Update this when the token is available.
    fn text_style(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<TextStyle>>> {
        let theme = self.context.theme(app);
        Some(Rc::new(MaterialStatePropertyAll::new(
            theme.text_theme.label_large.clone(),
        )))
    }

    fn visual_density(&self, app: &mut App) -> Option<VisualDensity> {
        let theme = self.context.theme(app);
        Some(theme.visual_density)
    }
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

/// Dart's `_MenuDefaultsM3`.
pub(crate) struct MenuDefaultsM3 {
    /// Dart's `super(..)` arguments.
    menu_style: MenuStyle,
    context: DefaultsContext,
}

impl MenuDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> MenuDefaultsM3 {
        MenuDefaultsM3 {
            menu_style: MenuStyle::new()
                .elevation(all(3.0))
                .shape(all(default_menu_border()))
                .alignment(AlignmentGeometry::TOP_END),
            context: DefaultsContext::new(context),
        }
    }
}

impl MenuStyleGetters for MenuDefaultsM3 {
    fn menu_style(&self) -> &MenuStyle {
        &self.menu_style
    }

    fn background_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(all(colors.surface_container()))
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        Some(all(Colors::TRANSPARENT))
    }

    fn shadow_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(all(colors.shadow()))
    }

    fn padding(
        &self,
        _app: &mut App,
    ) -> Option<WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>> {
        Some(all(EdgeInsetsGeometry::directional(
            0.0,
            K_MENU_VERTICAL_MIN_PADDING,
            0.0,
            K_MENU_VERTICAL_MIN_PADDING,
        )))
    }

    fn visual_density(&self, app: &mut App) -> Option<VisualDensity> {
        let theme = self.context.theme(app);
        Some(theme.visual_density)
    }
}

/// Dart's `_MenuBarDefaultsM3._defaultMenuBorder` and `_MenuDefaultsM3._defaultMenuBorder`.
fn default_menu_border() -> OutlinedBorderRef {
    OutlinedBorderRef::new(RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadiusGeometry::all(Radius::circular(4.0)),
    ))
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
