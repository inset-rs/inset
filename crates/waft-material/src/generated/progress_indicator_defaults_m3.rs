//! Flutter counterpart: material_ui's `generated/progress_indicator_defaults_m3.g.dart`, a part
//! of `progress_indicator.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::{Color, Radius};
use waft_foundation::App;
use waft_painting::{BorderRadius, BorderRadiusGeometry, EdgeInsets, EdgeInsetsGeometry};
use waft_rendering::BoxConstraints;
use waft_widgets::BuildContext;

use crate::progress_indicator::CircularProgressIndicator;
use crate::progress_indicator_theme::{
    ProgressIndicatorThemeData, ProgressIndicatorThemeDataGetters,
};
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_CircularProgressIndicatorDefaultsM3`.
pub(crate) struct CircularProgressIndicatorDefaultsM3 {
    /// Dart's `super(..)` arguments.
    progress_indicator_theme_data: ProgressIndicatorThemeData,
    context: DefaultsContext,
    /// Dart's `indeterminate`.
    indeterminate: bool,
}

impl CircularProgressIndicatorDefaultsM3 {
    pub(crate) fn new(
        context: BuildContext,
        indeterminate: bool,
    ) -> CircularProgressIndicatorDefaultsM3 {
        CircularProgressIndicatorDefaultsM3 {
            progress_indicator_theme_data: ProgressIndicatorThemeData::new(),
            context: DefaultsContext::new(context),
            indeterminate,
        }
    }
}

impl ProgressIndicatorThemeDataGetters for CircularProgressIndicatorDefaultsM3 {
    fn progress_indicator_theme_data(&self) -> &ProgressIndicatorThemeData {
        &self.progress_indicator_theme_data
    }

    fn color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).primary)
    }

    fn circular_track_color(&self, app: &mut App) -> Option<Color> {
        if self.indeterminate {
            None
        } else {
            Some(self.context.colors(app).secondary_container())
        }
    }

    fn stroke_width(&self, _app: &mut App) -> Option<f64> {
        Some(4.0)
    }

    fn stroke_align(&self, _app: &mut App) -> Option<f64> {
        Some(CircularProgressIndicator::STROKE_ALIGN_INSIDE)
    }

    fn constraints(&self, _app: &mut App) -> Option<BoxConstraints> {
        Some(BoxConstraints::new().min_width(40.0).min_height(40.0))
    }

    fn track_gap(&self, _app: &mut App) -> Option<f64> {
        Some(4.0)
    }

    fn circular_track_padding(&self, _app: &mut App) -> Option<EdgeInsetsGeometry> {
        Some(EdgeInsets::all(4.0).into())
    }
}

/// Dart's `_LinearProgressIndicatorDefaultsM3`.
pub(crate) struct LinearProgressIndicatorDefaultsM3 {
    /// Dart's `super(..)` arguments.
    progress_indicator_theme_data: ProgressIndicatorThemeData,
    context: DefaultsContext,
}

impl LinearProgressIndicatorDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> LinearProgressIndicatorDefaultsM3 {
        LinearProgressIndicatorDefaultsM3 {
            progress_indicator_theme_data: ProgressIndicatorThemeData::new(),
            context: DefaultsContext::new(context),
        }
    }
}

impl ProgressIndicatorThemeDataGetters for LinearProgressIndicatorDefaultsM3 {
    fn progress_indicator_theme_data(&self) -> &ProgressIndicatorThemeData {
        &self.progress_indicator_theme_data
    }

    fn color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).primary)
    }

    fn linear_track_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).secondary_container())
    }

    fn linear_min_height(&self, _app: &mut App) -> Option<f64> {
        Some(4.0)
    }

    fn border_radius(&self, _app: &mut App) -> Option<BorderRadiusGeometry> {
        Some(BorderRadius::all(Radius::circular(4.0 / 2.0)).into())
    }

    fn stop_indicator_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).primary)
    }

    fn stop_indicator_radius(&self, _app: &mut App) -> Option<f64> {
        Some(4.0 / 2.0)
    }

    fn track_gap(&self, _app: &mut App) -> Option<f64> {
        Some(4.0)
    }
}
