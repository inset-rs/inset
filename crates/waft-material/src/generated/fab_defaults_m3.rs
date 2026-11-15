//! Flutter counterpart: material_ui's `generated/fab_defaults_m3.g.dart`, a part of
//! `floating_action_button.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use waft_embedder::Radius;
use waft_foundation::App;
use waft_painting::{
    BorderRadius, BorderSide, EdgeInsetsDirectional, RoundedRectangleBorder, ShapeBorderRef,
};
use waft_rendering::BoxConstraints;
use waft_widgets::BuildContext;

use crate::floating_action_button::FloatingActionButtonType;
use crate::floating_action_button_theme::{
    FloatingActionButtonThemeData, FloatingActionButtonThemeDataGetters,
};
use crate::theme_data_getters::DefaultsContext;
use waft_embedder::Color;
use waft_painting::EdgeInsetsGeometry;
use waft_painting::TextStyle;

/// Dart's `_FABDefaultsM3`.
pub(crate) struct FabDefaultsM3 {
    /// Dart's `super(..)` arguments.
    floating_action_button_theme_data: FloatingActionButtonThemeData,
    context: DefaultsContext,
    r#type: FloatingActionButtonType,
    has_child: bool,
}

impl FabDefaultsM3 {
    pub(crate) fn new(
        context: BuildContext,
        r#type: FloatingActionButtonType,
        has_child: bool,
    ) -> FabDefaultsM3 {
        FabDefaultsM3 {
            floating_action_button_theme_data: FloatingActionButtonThemeData::new()
                .elevation(6.0)
                .focus_elevation(6.0)
                .hover_elevation(8.0)
                .highlight_elevation(6.0)
                .enable_feedback(true)
                .size_constraints(BoxConstraints::tight_for(Some(56.0), Some(56.0)))
                .small_size_constraints(BoxConstraints::tight_for(Some(40.0), Some(40.0)))
                .large_size_constraints(BoxConstraints::tight_for(Some(96.0), Some(96.0)))
                .extended_size_constraints(BoxConstraints::tight_for(None, Some(56.0)))
                .extended_icon_label_spacing(8.0),
            context: DefaultsContext::new(context),
            r#type,
            has_child,
        }
    }
}

impl FloatingActionButtonThemeDataGetters for FabDefaultsM3 {
    fn floating_action_button_theme_data(&self) -> &FloatingActionButtonThemeData {
        &self.floating_action_button_theme_data
    }

    fn foreground_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_primary_container())
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary_container())
    }

    fn splash_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_primary_container().with_opacity(0.1))
    }

    fn focus_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_primary_container().with_opacity(0.1))
    }

    fn hover_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_primary_container().with_opacity(0.08))
    }

    fn shape(&self, _app: &mut App) -> Option<ShapeBorderRef> {
        Some(match self.r#type {
            FloatingActionButtonType::Regular => rounded(16.0),
            FloatingActionButtonType::Small => rounded(12.0),
            FloatingActionButtonType::Large => rounded(28.0),
            FloatingActionButtonType::Extended => rounded(16.0),
        })
    }

    fn icon_size(&self, _app: &mut App) -> Option<f64> {
        Some(match self.r#type {
            FloatingActionButtonType::Regular => 24.0,
            FloatingActionButtonType::Small => 24.0,
            FloatingActionButtonType::Large => 36.0,
            FloatingActionButtonType::Extended => 24.0,
        })
    }

    fn extended_padding(&self, _app: &mut App) -> Option<EdgeInsetsGeometry> {
        let is_extended = self.r#type == FloatingActionButtonType::Extended;
        Some(
            EdgeInsetsDirectional::only(
                if self.has_child && is_extended {
                    16.0
                } else {
                    20.0
                },
                0.0,
                20.0,
                0.0,
            )
            .into(),
        )
    }

    fn extended_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let theme = self.context.theme(app);
        let text_theme = &theme.text_theme;
        text_theme.label_large.clone()
    }
}

/// Dart's `const RoundedRectangleBorder(borderRadius: BorderRadius.all(Radius.circular(r)))`.
fn rounded(radius: f64) -> ShapeBorderRef {
    ShapeBorderRef::new(RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadius::all(Radius::circular(radius)).into(),
    ))
}
