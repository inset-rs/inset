//! Flutter counterpart: material_ui's `generated/dialog_fullscreen_defaults_m3.g.dart`, a part
//! of `dialog.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::{Clip, Color};
use waft_foundation::App;
use waft_widgets::BuildContext;

use crate::dialog_theme::{DialogThemeData, DialogThemeDataGetters};
use crate::theme::Theme;

/// Dart's `_DialogFullscreenDefaultsM3`.
pub(crate) struct DialogFullscreenDefaultsM3 {
    /// Dart's `super(..)` arguments.
    dialog_theme_data: DialogThemeData,
    context: BuildContext,
}

impl DialogFullscreenDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> DialogFullscreenDefaultsM3 {
        DialogFullscreenDefaultsM3 {
            dialog_theme_data: DialogThemeData::new().clip_behavior(Clip::None),
            context,
        }
    }
}

impl DialogThemeDataGetters for DialogFullscreenDefaultsM3 {
    fn dialog_theme_data(&self) -> &DialogThemeData {
        &self.dialog_theme_data
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        Some(Theme::of(app, self.context).color_scheme.surface)
    }
}
