//! Flutter counterpart: material_ui's `generated/dialog_fullscreen_defaults_m3.g.dart`, a Dart
//! `part` of `dialog.dart`.
//!
//! Dart's private `_DialogFullscreenDefaultsM3` is a `DialogThemeData` whose getter reads the
//! context's theme; [`dialog_fullscreen_defaults_m3`] returns the data it answers, read once
//! from the theme.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::Clip;
use waft_foundation::App;
use waft_widgets::BuildContext;

use crate::dialog_theme::DialogThemeData;
use crate::theme::Theme;

/// Dart's `_DialogFullscreenDefaultsM3(context)`.
pub(crate) fn dialog_fullscreen_defaults_m3(
    app: &mut App,
    context: BuildContext,
) -> DialogThemeData {
    DialogThemeData::new()
        .clip_behavior(Clip::None)
        .background_color(Theme::of(app, context).color_scheme.surface)
}
