//! Flutter counterpart: material_ui's `generated/progress_indicator_defaults_m3.g.dart`, a
//! part of `progress_indicator.dart`.
//!
//! Dart's private `_CircularProgressIndicatorDefaultsM3` and `_LinearProgressIndicatorDefaultsM3`
//! are `ProgressIndicatorThemeData`s whose getters read the context's colour scheme; each
//! function here returns the data those getters answer, read once from the theme.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::Radius;
use waft_foundation::App;
use waft_painting::{BorderRadius, EdgeInsets};
use waft_rendering::BoxConstraints;
use waft_widgets::BuildContext;

use crate::progress_indicator::CircularProgressIndicator;
use crate::progress_indicator_theme::ProgressIndicatorThemeData;
use crate::theme::Theme;

/// Dart's `_CircularProgressIndicatorDefaultsM3(context, indeterminate:)`.
pub(crate) fn circular_progress_indicator_defaults_m3(
    app: &mut App,
    context: BuildContext,
    indeterminate: bool,
) -> ProgressIndicatorThemeData {
    let colors = Theme::of(app, context).color_scheme.clone();

    ProgressIndicatorThemeData {
        color: Some(colors.primary),
        circular_track_color: if indeterminate {
            None
        } else {
            Some(colors.secondary_container())
        },
        stroke_width: Some(4.0),
        stroke_align: Some(CircularProgressIndicator::STROKE_ALIGN_INSIDE),
        constraints: Some(BoxConstraints::new().min_width(40.0).min_height(40.0)),
        track_gap: Some(4.0),
        circular_track_padding: Some(EdgeInsets::all(4.0).into()),
        ..ProgressIndicatorThemeData::new()
    }
}

/// Dart's `_LinearProgressIndicatorDefaultsM3(context)`.
pub(crate) fn linear_progress_indicator_defaults_m3(
    app: &mut App,
    context: BuildContext,
) -> ProgressIndicatorThemeData {
    let colors = Theme::of(app, context).color_scheme.clone();

    ProgressIndicatorThemeData::new()
        .color(colors.primary)
        .linear_track_color(colors.secondary_container())
        .linear_min_height(4.0)
        .border_radius(BorderRadius::all(Radius::circular(4.0 / 2.0)))
        .stop_indicator_color(colors.primary)
        .stop_indicator_radius(4.0 / 2.0)
        .track_gap(4.0)
}
