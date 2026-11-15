//! Flutter counterpart: material_ui's `generated/navigation_rail_defaults_m3.g.dart`, a part of
//! `navigation_rail.dart`.
//!
//! Dart's private `_NavigationRailDefaultsM3` is a `NavigationRailThemeData` whose getters read
//! the context's theme; [`navigation_rail_defaults_m3`] returns the data those getters answer,
//! read once from the theme.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_foundation::App;
use waft_painting::{BorderSide, StadiumBorder};
use waft_widgets::{BuildContext, IconThemeData};

use crate::navigation_rail::NavigationRailLabelType;
use crate::navigation_rail_theme::NavigationRailThemeData;
use crate::theme::Theme;

/// Dart's `_NavigationRailDefaultsM3(context)`.
pub(crate) fn navigation_rail_defaults_m3(
    app: &mut App,
    context: BuildContext,
) -> NavigationRailThemeData {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;
    let text_theme = &theme.text_theme;
    NavigationRailThemeData::new()
        .elevation(0.0)
        .group_alignment(-1.0)
        .label_type(NavigationRailLabelType::None)
        .use_indicator(true)
        .min_width(80.0)
        .min_extended_width(256.0)
        .background_color(colors.surface)
        .unselected_label_text_style(
            text_theme
                .label_medium
                .as_ref()
                .expect("the M3 text theme has labelMedium")
                .copy_with()
                .color(colors.on_surface),
        )
        .selected_label_text_style(
            text_theme
                .label_medium
                .as_ref()
                .expect("the M3 text theme has labelMedium")
                .copy_with()
                .color(colors.on_surface),
        )
        .unselected_icon_theme(
            IconThemeData::new()
                .size(24.0)
                .color(colors.on_surface_variant()),
        )
        .selected_icon_theme(
            IconThemeData::new()
                .size(24.0)
                .color(colors.on_secondary_container()),
        )
        .indicator_color(colors.secondary_container())
        .indicator_shape(StadiumBorder::new(BorderSide::NONE))
}
