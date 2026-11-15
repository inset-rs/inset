//! Flutter counterpart: material_ui's `generated/outlined_card_defaults_m3.g.dart`, a part of `card.dart`.
//!
//! Dart's private `_OutlinedCardDefaultsM3` is a `CardThemeData` whose getters read the context's
//! theme; [`outlined_card_defaults_m3`] returns the data those getters answer, read once from the theme.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::Clip;
use waft_foundation::App;
use waft_painting::{BorderRadiusGeometry, BorderSide, EdgeInsetsGeometry, RoundedRectangleBorder};
use waft_widgets::BuildContext;

use crate::card_theme::CardThemeData;
use crate::colors::Colors;
use crate::theme::Theme;

/// Dart's `_OutlinedCardDefaultsM3(context)`.
pub(crate) fn outlined_card_defaults_m3(app: &mut App, context: BuildContext) -> CardThemeData {
    let colors = Theme::of(app, context).color_scheme.clone();
    CardThemeData::new()
        .clip_behavior(Clip::None)
        .elevation(0.0)
        .margin(EdgeInsetsGeometry::all(4.0))
        .color(colors.surface)
        .shadow_color(colors.shadow())
        .surface_tint_color(Colors::TRANSPARENT)
        .shape(
            RoundedRectangleBorder::new(BorderSide::NONE, BorderRadiusGeometry::circular(12.0))
                .copy_with(
                    Some(BorderSide {
                        color: colors.outline_variant(),
                        ..BorderSide::default()
                    }),
                    None,
                ),
        )
}
