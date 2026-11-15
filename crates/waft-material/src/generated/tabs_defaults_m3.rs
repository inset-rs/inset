//! Flutter counterpart: material_ui's `generated/tabs_defaults_m3.g.dart`, a part of
//! `tabs.dart`.
//!
//! Dart's private `_TabsPrimaryDefaultsM3` and `_TabsSecondaryDefaultsM3` are
//! `TabBarThemeData`s whose getters read the context's theme; their `new` returns the data
//! those getters answer, read once from the theme, and their static members are associated
//! items.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

// Dart's generated `withOpacity`, deprecated there too.
#![allow(deprecated)]

use waft_embedder::Color;
use waft_foundation::App;
use waft_painting::EdgeInsetsGeometry;
use waft_widgets::{BuildContext, WidgetState, WidgetStateProperty};

use crate::tab_bar_theme::TabBarThemeData;
use crate::tabs::{TabAlignment, TabBarIndicatorSize};
use crate::theme::Theme;

/// Dart's `_TabsPrimaryDefaultsM3`.
pub(crate) enum TabsPrimaryDefaultsM3 {}

impl TabsPrimaryDefaultsM3 {
    /// Dart's `_TabsPrimaryDefaultsM3(context, isScrollable)`.
    #[expect(
        clippy::new_ret_no_self,
        reason = "Dart's constructor of a TabBarThemeData subclass, here the data it answers"
    )]
    pub(crate) fn new(
        app: &mut App,
        context: BuildContext,
        is_scrollable: bool,
    ) -> TabBarThemeData {
        let theme = Theme::of(app, context);
        let colors = theme.color_scheme.clone();
        let text_theme = &theme.text_theme;
        let overlay_colors = colors.clone();
        let mut defaults = TabBarThemeData::new()
            .indicator_size(TabBarIndicatorSize::Label)
            // This value comes from Divider widget defaults. Token db deprecated 'primary-navigation-tab.divider.color' token.
            .divider_color(colors.outline_variant())
            // This value comes from Divider widget defaults. Token db deprecated 'primary-navigation-tab.divider.height' token.
            .divider_height(1.0)
            .indicator_color(colors.primary)
            .label_color(colors.primary)
            .unselected_label_color(colors.on_surface_variant())
            .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                move |states| {
                    let colors = &overlay_colors;
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
                        return Some(colors.primary.with_opacity(0.1));
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
            .splash_factory(theme.splash_factory.clone())
            .tab_alignment(if is_scrollable {
                TabAlignment::StartOffset
            } else {
                TabAlignment::Fill
            });
        defaults.label_style = text_theme.title_small.clone();
        defaults.unselected_label_style = text_theme.title_small.clone();
        defaults
    }

    /// Dart's `_TabsPrimaryDefaultsM3.indicatorWeight`.
    pub(crate) fn indicator_weight(indicator_size: TabBarIndicatorSize) -> f64 {
        match indicator_size {
            TabBarIndicatorSize::Label => 3.0,
            TabBarIndicatorSize::Tab => 2.0,
        }
    }

    // TODO(davidmartos96): This value doesn't currently exist in
    // https://m3.material.io/components/tabs/specs
    // Update this when the token is available.
    /// Dart's `_TabsPrimaryDefaultsM3.iconMargin`.
    pub(crate) const ICON_MARGIN: EdgeInsetsGeometry = EdgeInsetsGeometry::only(0.0, 0.0, 0.0, 2.0);
}

/// Dart's `_TabsSecondaryDefaultsM3`.
pub(crate) enum TabsSecondaryDefaultsM3 {}

impl TabsSecondaryDefaultsM3 {
    /// Dart's `_TabsSecondaryDefaultsM3(context, isScrollable)`.
    #[expect(
        clippy::new_ret_no_self,
        reason = "Dart's constructor of a TabBarThemeData subclass, here the data it answers"
    )]
    pub(crate) fn new(
        app: &mut App,
        context: BuildContext,
        is_scrollable: bool,
    ) -> TabBarThemeData {
        let theme = Theme::of(app, context);
        let colors = theme.color_scheme.clone();
        let text_theme = &theme.text_theme;
        let overlay_colors = colors.clone();
        let mut defaults = TabBarThemeData::new()
            .indicator_size(TabBarIndicatorSize::Tab)
            // This value comes from Divider widget defaults. Token db deprecated 'secondary-navigation-tab.divider.color' token.
            .divider_color(colors.outline_variant())
            // This value comes from Divider widget defaults. Token db deprecated 'secondary-navigation-tab.divider.height' token.
            .divider_height(1.0)
            .indicator_color(colors.primary)
            .label_color(colors.on_surface)
            .unselected_label_color(colors.on_surface_variant())
            .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                move |states| {
                    let colors = &overlay_colors;
                    if states.contains(&WidgetState::Selected) {
                        if states.contains(&WidgetState::Pressed) {
                            return Some(colors.on_surface.with_opacity(0.1));
                        }
                        if states.contains(&WidgetState::Hovered) {
                            return Some(colors.on_surface.with_opacity(0.08));
                        }
                        if states.contains(&WidgetState::Focused) {
                            return Some(colors.on_surface.with_opacity(0.1));
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
            .splash_factory(theme.splash_factory.clone())
            .tab_alignment(if is_scrollable {
                TabAlignment::StartOffset
            } else {
                TabAlignment::Fill
            });
        defaults.label_style = text_theme.title_small.clone();
        defaults.unselected_label_style = text_theme.title_small.clone();
        defaults
    }

    /// Dart's `_TabsSecondaryDefaultsM3.indicatorWeight`, a `static double` Dart never
    /// assigns.
    pub(crate) const INDICATOR_WEIGHT: f64 = 2.0;
}
