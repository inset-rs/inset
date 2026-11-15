//! Flutter counterpart: material_ui's `generated/bottom_sheet_defaults_m3.g.dart`, a part of
//! `bottom_sheet.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::{Color, Radius, Size};
use waft_foundation::App;
use waft_painting::{AnyColor, BorderRadiusGeometry, BorderSide, RoundedRectangleBorder};
use waft_rendering::BoxConstraints;
use waft_widgets::BuildContext;

use crate::bottom_sheet_theme::{BottomSheetThemeData, BottomSheetThemeDataGetters};
use crate::colors::Colors;
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_BottomSheetDefaultsM3`.
pub(crate) struct BottomSheetDefaultsM3 {
    /// Dart's `super(..)` arguments.
    bottom_sheet_theme_data: BottomSheetThemeData,
    context: DefaultsContext,
}

impl BottomSheetDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> BottomSheetDefaultsM3 {
        BottomSheetDefaultsM3 {
            bottom_sheet_theme_data: BottomSheetThemeData::new()
                .elevation(1.0)
                .modal_elevation(1.0)
                .shape(RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadiusGeometry::vertical(Radius::circular(28.0), Radius::ZERO),
                ))
                .constraints(BoxConstraints::new().max_width(640.0)),
            context: DefaultsContext::new(context),
        }
    }
}

impl BottomSheetThemeDataGetters for BottomSheetDefaultsM3 {
    fn bottom_sheet_theme_data(&self) -> &BottomSheetThemeData {
        &self.bottom_sheet_theme_data
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).surface_container_low())
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn shadow_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn drag_handle_color(&self, app: &mut App) -> Option<AnyColor> {
        Some(self.context.colors(app).on_surface_variant().into())
    }

    fn drag_handle_size(&self, _app: &mut App) -> Option<Size> {
        Some(Size::new(32.0, 4.0))
    }

    fn constraints(&self, _app: &mut App) -> Option<BoxConstraints> {
        Some(BoxConstraints::new().max_width(640.0))
    }
}
