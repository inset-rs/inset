//! Flutter counterpart: material_ui's `generated/fab_defaults_m3.g.dart`, a Dart `part` of
//! `floating_action_button.dart`: `FloatingActionButton`'s Material 3 build reads it.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use waft_embedder::Radius;
use waft_foundation::App;
use waft_painting::{
    BorderRadius, BorderSide, EdgeInsetsDirectional, RoundedRectangleBorder, ShapeBorder,
};
use waft_rendering::BoxConstraints;
use waft_widgets::BuildContext;

use crate::floating_action_button::FloatingActionButtonType;
use crate::floating_action_button_theme::FloatingActionButtonThemeData;
use crate::theme::Theme;

/// Dart's `_FABDefaultsM3(context, type, hasChild)`: the [`FloatingActionButtonThemeData`]
/// whose fields are the constructor's arguments and the values of that subclass's getters.
pub(crate) fn fab_defaults_m3(
    app: &mut App,
    context: BuildContext,
    r#type: FloatingActionButtonType,
    has_child: bool,
) -> FloatingActionButtonThemeData {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;
    let text_theme = &theme.text_theme;
    let is_extended = r#type == FloatingActionButtonType::Extended;

    let mut defaults = FloatingActionButtonThemeData::new()
        .elevation(6.0)
        .focus_elevation(6.0)
        .hover_elevation(8.0)
        .highlight_elevation(6.0)
        .enable_feedback(true)
        .size_constraints(BoxConstraints::tight_for(Some(56.0), Some(56.0)))
        .small_size_constraints(BoxConstraints::tight_for(Some(40.0), Some(40.0)))
        .large_size_constraints(BoxConstraints::tight_for(Some(96.0), Some(96.0)))
        .extended_size_constraints(BoxConstraints::tight_for(None, Some(56.0)))
        .extended_icon_label_spacing(8.0)
        .foreground_color(colors.on_primary_container())
        .background_color(colors.primary_container())
        .splash_color(colors.on_primary_container().with_opacity(0.1))
        .focus_color(colors.on_primary_container().with_opacity(0.1))
        .hover_color(colors.on_primary_container().with_opacity(0.08))
        .shape(shape(r#type))
        .icon_size(icon_size(r#type))
        .extended_padding(
            EdgeInsetsDirectional::only(
                if has_child && is_extended { 16.0 } else { 20.0 },
                0.0,
                20.0,
                0.0,
            )
            .into(),
        );
    defaults.extended_text_style = text_theme.label_large.clone();
    defaults
}

/// Dart's `_FABDefaultsM3.shape`.
fn shape(r#type: FloatingActionButtonType) -> Box<dyn ShapeBorder> {
    match r#type {
        FloatingActionButtonType::Regular => rounded(16.0),
        FloatingActionButtonType::Small => rounded(12.0),
        FloatingActionButtonType::Large => rounded(28.0),
        FloatingActionButtonType::Extended => rounded(16.0),
    }
}

/// Dart's `_FABDefaultsM3.iconSize`.
fn icon_size(r#type: FloatingActionButtonType) -> f64 {
    match r#type {
        FloatingActionButtonType::Regular => 24.0,
        FloatingActionButtonType::Small => 24.0,
        FloatingActionButtonType::Large => 36.0,
        FloatingActionButtonType::Extended => 24.0,
    }
}

/// Dart's `const RoundedRectangleBorder(borderRadius: BorderRadius.all(Radius.circular(r)))`.
fn rounded(radius: f64) -> Box<dyn ShapeBorder> {
    Box::new(RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadius::all(Radius::circular(radius)).into(),
    ))
}
