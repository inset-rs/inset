//! Flutter counterpart: material_ui's `generated/dialog_defaults_m3.g.dart`, a Dart `part` of
//! `dialog.dart`.
//!
//! Dart's private `_DialogDefaultsM3` is a `DialogThemeData` whose getters read the context's
//! theme; [`dialog_defaults_m3`] returns the data those getters answer, read once from the
//! theme.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::{Clip, Radius};
use waft_foundation::App;
use waft_painting::{
    AlignmentGeometry, BorderRadiusGeometry, BorderSide, EdgeInsetsGeometry, RoundedRectangleBorder,
};
use waft_widgets::BuildContext;

use crate::colors::Colors;
use crate::dialog_theme::DialogThemeData;
use crate::theme::Theme;

/// Dart's `_DialogDefaultsM3(context)`.
pub(crate) fn dialog_defaults_m3(app: &mut App, context: BuildContext) -> DialogThemeData {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;
    let text_theme = &theme.text_theme;
    let mut defaults = DialogThemeData::new()
        .alignment(AlignmentGeometry::CENTER)
        .elevation(6.0)
        .shape(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::all(Radius::circular(28.0)),
        ))
        .clip_behavior(Clip::None)
        .icon_color(colors.secondary)
        .background_color(colors.surface_container_high())
        .shadow_color(Colors::TRANSPARENT)
        .surface_tint_color(Colors::TRANSPARENT)
        .actions_padding(EdgeInsetsGeometry::only(24.0, 0.0, 24.0, 24.0));
    defaults.title_text_style = text_theme.headline_small.clone();
    defaults.content_text_style = text_theme.body_medium.clone();
    defaults
}
