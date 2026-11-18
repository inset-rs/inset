//! Flutter counterpart: material_ui's `generated/tabs_defaults_m3.g.dart`, a part of
//! `tabs.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

// Dart's generated `withOpacity`, deprecated there too.
#![allow(deprecated)]

use waft_embedder::Color;
use waft_foundation::App;
use waft_painting::EdgeInsetsGeometry;
use waft_widgets::{BuildContext, WidgetState, WidgetStateProperty};

use crate::ink_well::InteractiveInkFeatureFactoryRef;
use crate::tab_bar_theme::{TabBarThemeData, TabBarThemeDataGetters};
use crate::tabs::{TabAlignment, TabBarIndicatorSize};
use crate::theme_data_getters::DefaultsContext;
use waft_painting::AnyColor;
use waft_painting::TextStyle;
use waft_widgets::WidgetStatePropertyRef;

/// Dart's `_TabsPrimaryDefaultsM3`.
pub(crate) struct TabsPrimaryDefaultsM3 {
    /// Dart's `super(..)` arguments.
    tab_bar_theme_data: TabBarThemeData,
    context: DefaultsContext,
    is_scrollable: bool,
}

impl TabsPrimaryDefaultsM3 {
    pub(crate) fn new(context: BuildContext, is_scrollable: bool) -> TabsPrimaryDefaultsM3 {
        TabsPrimaryDefaultsM3 {
            tab_bar_theme_data: TabBarThemeData::new().indicator_size(TabBarIndicatorSize::Label),
            context: DefaultsContext::new(context),
            is_scrollable,
        }
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

impl TabBarThemeDataGetters for TabsPrimaryDefaultsM3 {
    fn tab_bar_theme_data(&self) -> &TabBarThemeData {
        &self.tab_bar_theme_data
    }

    // This value comes from Divider widget defaults. Token db deprecated 'primary-navigation-tab.divider.color' token.
    fn divider_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some(colors.outline_variant())
    }

    // This value comes from Divider widget defaults. Token db deprecated 'primary-navigation-tab.divider.height' token.
    fn divider_height(&self, _app: &mut App) -> Option<f64> {
        Some(1.0)
    }

    fn indicator_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some(colors.primary())
    }

    fn label_color(&self, app: &mut App) -> Option<AnyColor> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some(colors.primary().into())
    }

    fn unselected_label_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some(colors.on_surface_variant())
    }

    fn overlay_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        let overlay_colors = colors.clone();
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states| {
                let colors = &overlay_colors;
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
                    return None;
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
                None
            },
        ))
    }

    fn splash_factory(&self, app: &mut App) -> Option<InteractiveInkFeatureFactoryRef> {
        let theme = self.context.theme(app);
        Some(theme.splash_factory.clone())
    }

    fn tab_alignment(&self, _app: &mut App) -> Option<TabAlignment> {
        Some(if self.is_scrollable {
            TabAlignment::StartOffset
        } else {
            TabAlignment::Fill
        })
    }

    fn label_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).title_small.clone()
    }

    fn unselected_label_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).title_small.clone()
    }
}

/// Dart's `_TabsSecondaryDefaultsM3`.
pub(crate) struct TabsSecondaryDefaultsM3 {
    /// Dart's `super(..)` arguments.
    tab_bar_theme_data: TabBarThemeData,
    context: DefaultsContext,
    is_scrollable: bool,
}

impl TabsSecondaryDefaultsM3 {
    pub(crate) fn new(context: BuildContext, is_scrollable: bool) -> TabsSecondaryDefaultsM3 {
        TabsSecondaryDefaultsM3 {
            tab_bar_theme_data: TabBarThemeData::new().indicator_size(TabBarIndicatorSize::Tab),
            context: DefaultsContext::new(context),
            is_scrollable,
        }
    }

    /// Dart's `_TabsSecondaryDefaultsM3.indicatorWeight`, a `static double` Dart never
    /// assigns.
    pub(crate) const INDICATOR_WEIGHT: f64 = 2.0;
}

impl TabBarThemeDataGetters for TabsSecondaryDefaultsM3 {
    fn tab_bar_theme_data(&self) -> &TabBarThemeData {
        &self.tab_bar_theme_data
    }

    // This value comes from Divider widget defaults. Token db deprecated 'secondary-navigation-tab.divider.color' token.
    fn divider_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some(colors.outline_variant())
    }

    // This value comes from Divider widget defaults. Token db deprecated 'secondary-navigation-tab.divider.height' token.
    fn divider_height(&self, _app: &mut App) -> Option<f64> {
        Some(1.0)
    }

    fn indicator_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some(colors.primary())
    }

    fn label_color(&self, app: &mut App) -> Option<AnyColor> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some(colors.on_surface().into())
    }

    fn unselected_label_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        Some(colors.on_surface_variant())
    }

    fn overlay_color(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        let theme = self.context.theme(app);
        let colors = theme.color_scheme.clone();
        let overlay_colors = colors.clone();
        Some(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states| {
                let colors = &overlay_colors;
                if states.contains(&WidgetState::Selected) {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(colors.on_surface().with_opacity(0.1));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(colors.on_surface().with_opacity(0.08));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(colors.on_surface().with_opacity(0.1));
                    }
                    return None;
                }
                if states.contains(&WidgetState::Pressed) {
                    return Some(colors.on_surface().with_opacity(0.1));
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(colors.on_surface().with_opacity(0.08));
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(colors.on_surface().with_opacity(0.1));
                }
                None
            },
        ))
    }

    fn splash_factory(&self, app: &mut App) -> Option<InteractiveInkFeatureFactoryRef> {
        let theme = self.context.theme(app);
        Some(theme.splash_factory.clone())
    }

    fn tab_alignment(&self, _app: &mut App) -> Option<TabAlignment> {
        Some(if self.is_scrollable {
            TabAlignment::StartOffset
        } else {
            TabAlignment::Fill
        })
    }

    fn label_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).title_small.clone()
    }

    fn unselected_label_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).title_small.clone()
    }
}
