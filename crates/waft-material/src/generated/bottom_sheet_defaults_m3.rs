//! Flutter counterpart: material_ui's `generated/bottom_sheet_defaults_m3.g.dart`, a part of
//! `bottom_sheet.dart`.
//!
//! Dart's private `_BottomSheetDefaultsM3` is a `BottomSheetThemeData` whose getters read the
//! context's theme; [`bottom_sheet_defaults_m3`] returns the data those getters answer, read
//! once from the theme.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::{Radius, Size};
use waft_foundation::App;
use waft_painting::{BorderRadiusGeometry, BorderSide, RoundedRectangleBorder};
use waft_rendering::BoxConstraints;
use waft_widgets::BuildContext;

use crate::bottom_sheet_theme::BottomSheetThemeData;
use crate::colors::Colors;
use crate::theme::Theme;

/// Dart's `_BottomSheetDefaultsM3(context)`.
pub(crate) fn bottom_sheet_defaults_m3(
    app: &mut App,
    context: BuildContext,
) -> BottomSheetThemeData {
    let colors = Theme::of(app, context).color_scheme.clone();
    BottomSheetThemeData::new()
        .elevation(1.0)
        .modal_elevation(1.0)
        .shape(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::vertical(Radius::circular(28.0), Radius::ZERO),
        ))
        .background_color(colors.surface_container_low())
        .surface_tint_color(Colors::TRANSPARENT)
        .shadow_color(Colors::TRANSPARENT)
        .drag_handle_color(colors.on_surface_variant())
        .drag_handle_size(Size::new(32.0, 4.0))
        .constraints(BoxConstraints::new().max_width(640.0))
}
