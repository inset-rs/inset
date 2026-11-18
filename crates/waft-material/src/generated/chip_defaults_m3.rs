//! Flutter counterpart: material_ui's `generated/chip_defaults_m3.g.dart`, a part of `chip.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

#![allow(deprecated)]

use waft_embedder::{Color, clamp_double};
use waft_foundation::App;
use waft_painting::{
    AnyBorderSide, AnyColor, BorderRadiusGeometry, BorderSide, EdgeInsets, EdgeInsetsGeometry,
    OutlinedBorderRef, RoundedRectangleBorder, TextStyle,
};
use waft_widgets::{BuildContext, IconThemeData, MediaQuery, WidgetStatePropertyRef};

use crate::chip_theme::{ChipThemeData, ChipThemeDataGetters};
use crate::colors::Colors;
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_ChipDefaultsM3`.
pub(crate) struct ChipDefaultsM3 {
    /// Dart's `super(..)` arguments.
    chip_theme_data: ChipThemeData,
    context: DefaultsContext,
    is_enabled: bool,
}

impl ChipDefaultsM3 {
    pub(crate) fn new(context: BuildContext, is_enabled: bool) -> ChipDefaultsM3 {
        ChipDefaultsM3 {
            chip_theme_data: ChipThemeData::new()
                .elevation(0.0)
                .shape(OutlinedBorderRef::new(RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadiusGeometry::circular(8.0),
                )))
                .show_checkmark(true),
            context: DefaultsContext::new(context),
            is_enabled,
        }
    }
}

impl ChipThemeDataGetters for ChipDefaultsM3 {
    fn chip_theme_data(&self) -> &ChipThemeData {
        &self.chip_theme_data
    }

    fn label_style(&self, app: &mut App) -> Option<TextStyle> {
        let on_surface_variant = self.context.colors(app).on_surface_variant();
        let on_surface = self.context.colors(app).on_surface();
        let color = if self.is_enabled {
            on_surface_variant
        } else {
            on_surface
        };
        self.context
            .text_theme(app)
            .label_large
            .as_ref()
            .map(|label_large| label_large.copy_with().color(color))
    }

    // Subclasses override this getter.
    fn color(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        None
    }

    fn shadow_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn surface_tint_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn checkmark_color(&self, _app: &mut App) -> Option<Color> {
        None
    }

    fn delete_icon_color(&self, app: &mut App) -> Option<AnyColor> {
        let colors = self.context.colors(app);
        Some(AnyColor::from(if self.is_enabled {
            colors.on_surface_variant()
        } else {
            colors.on_surface()
        }))
    }

    fn side(&self, app: &mut App) -> Option<AnyBorderSide> {
        let colors = self.context.colors(app);
        Some(AnyBorderSide::new(if self.is_enabled {
            BorderSide {
                color: colors.outline_variant(),
                ..BorderSide::default()
            }
        } else {
            BorderSide {
                color: colors.on_surface().with_opacity(0.12),
                ..BorderSide::default()
            }
        }))
    }

    fn icon_theme(&self, app: &mut App) -> Option<IconThemeData> {
        let colors = self.context.colors(app);
        Some(
            IconThemeData::new()
                .color(if self.is_enabled {
                    colors.primary()
                } else {
                    colors.on_surface()
                })
                .size(18.0),
        )
    }

    fn padding(&self, _app: &mut App) -> Option<EdgeInsetsGeometry> {
        Some(EdgeInsets::all(8.0).into())
    }

    /// The label padding of the chip scales with the font size specified in the
    /// [`label_style`](ChipThemeDataGetters::label_style), and the system font size settings
    /// that scale font sizes globally.
    ///
    /// The chip at effective font size 14.0 starts with 8px on each side and as
    /// the font size scales up to closer to 28.0, the label padding is linearly
    /// interpolated from 8px to 4px. Once the label has a font size of 2 or
    /// higher, label padding remains 4px.
    fn label_padding(&self, app: &mut App) -> Option<EdgeInsetsGeometry> {
        let font_size = self
            .label_style(app)
            .and_then(|label_style| label_style.font_size)
            .unwrap_or(14.0);
        let font_size_ratio =
            MediaQuery::text_scaler_of(app, self.context.context()).scale(font_size) / 14.0;
        Some(
            EdgeInsets::lerp(
                Some(EdgeInsets::symmetric(0.0, 8.0)),
                Some(EdgeInsets::symmetric(0.0, 4.0)),
                clamp_double(font_size_ratio - 1.0, 0.0, 1.0),
            )
            .expect("both insets are given")
            .into(),
        )
    }
}
