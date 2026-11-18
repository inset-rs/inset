//! Flutter counterpart: material_ui's `generated/navigation_rail_defaults_m3.g.dart`, a part of
//! `navigation_rail.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_foundation::App;
use waft_painting::{BorderSide, StadiumBorder};
use waft_widgets::{BuildContext, IconThemeData};

use crate::navigation_rail::NavigationRailLabelType;
use crate::navigation_rail_theme::{NavigationRailThemeData, NavigationRailThemeDataGetters};
use crate::theme_data_getters::DefaultsContext;
use waft_embedder::Color;
use waft_painting::ShapeBorderRef;
use waft_painting::TextStyle;

/// Dart's `_NavigationRailDefaultsM3`.
pub(crate) struct NavigationRailDefaultsM3 {
    /// Dart's `super(..)` arguments.
    navigation_rail_theme_data: NavigationRailThemeData,
    context: DefaultsContext,
}

impl NavigationRailDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> NavigationRailDefaultsM3 {
        NavigationRailDefaultsM3 {
            navigation_rail_theme_data: NavigationRailThemeData::new()
                .elevation(0.0)
                .group_alignment(-1.0)
                .label_type(NavigationRailLabelType::None)
                .use_indicator(true)
                .min_width(80.0)
                .min_extended_width(256.0),
            context: DefaultsContext::new(context),
        }
    }
}

impl NavigationRailThemeDataGetters for NavigationRailDefaultsM3 {
    fn navigation_rail_theme_data(&self) -> &NavigationRailThemeData {
        &self.navigation_rail_theme_data
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.surface())
    }

    fn unselected_label_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let text_theme = &theme.text_theme;
        Some(
            text_theme
                .label_medium
                .as_ref()
                .expect("the M3 text theme has labelMedium")
                .copy_with()
                .color(colors.on_surface()),
        )
    }

    fn selected_label_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let text_theme = &theme.text_theme;
        Some(
            text_theme
                .label_medium
                .as_ref()
                .expect("the M3 text theme has labelMedium")
                .copy_with()
                .color(colors.on_surface()),
        )
    }

    fn unselected_icon_theme(&self, app: &mut App) -> Option<IconThemeData> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(
            IconThemeData::new()
                .size(24.0)
                .color(colors.on_surface_variant()),
        )
    }

    fn selected_icon_theme(&self, app: &mut App) -> Option<IconThemeData> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(
            IconThemeData::new()
                .size(24.0)
                .color(colors.on_secondary_container()),
        )
    }

    fn indicator_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.secondary_container())
    }

    fn indicator_shape(&self, _app: &mut App) -> Option<ShapeBorderRef> {
        Some(ShapeBorderRef::new(StadiumBorder::new(BorderSide::NONE)))
    }
}
