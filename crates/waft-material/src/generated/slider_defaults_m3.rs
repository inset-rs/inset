//! Flutter counterpart: material_ui's `generated/slider_defaults_m3.g.dart`, a Dart `part` of
//! `slider.dart`: `Slider`'s Material 3 build reads it when `year2023` is false.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use std::rc::Rc;

use waft_embedder::Size;
use waft_foundation::App;
use waft_widgets::{
    BuildContext, WidgetState, WidgetStateColor, WidgetStateProperty, WidgetStates,
};

use crate::colors::Colors;
use crate::slider_parts::{
    GappedSliderTrackShape, HandleThumbShape, RoundSliderTickMarkShape,
    RoundedRectSliderValueIndicatorShape,
};
use crate::slider_theme::SliderThemeData;
use crate::slider_value_indicator_shape::RoundSliderOverlayShape;
use crate::theme::Theme;

/// Dart's `_SliderDefaultsM3(context)`: the [`SliderThemeData`] whose fields are the
/// constructor's arguments and the values of that subclass's getters, read from the
/// context's theme when it is made.
///
/// A getter's `!` on a text theme style that is null leaves its field null here, so the
/// slider's `expect` fails where it reads the field, as Dart's getter fails when it is read.
pub(crate) fn slider_defaults_m3(app: &mut App, context: BuildContext) -> SliderThemeData {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;
    let primary = colors.primary;

    let mut defaults = SliderThemeData::new()
        .track_height(16.0)
        .active_track_color(colors.primary)
        .inactive_track_color(colors.secondary_container())
        .secondary_active_track_color(colors.primary.with_opacity(0.54))
        .disabled_active_track_color(colors.on_surface.with_opacity(0.38))
        .disabled_inactive_track_color(colors.on_surface.with_opacity(0.12))
        .disabled_secondary_active_track_color(colors.on_surface.with_opacity(0.38))
        .active_tick_mark_color(colors.on_primary.with_opacity(1.0))
        .inactive_tick_mark_color(colors.on_secondary_container().with_opacity(1.0))
        .disabled_active_tick_mark_color(colors.on_inverse_surface())
        .disabled_inactive_tick_mark_color(colors.on_surface)
        .thumb_color(colors.primary)
        .disabled_thumb_color(colors.on_surface.with_opacity(0.38))
        .overlay_color(WidgetStateColor::resolve_with(
            move |states: &WidgetStates| {
                if states.contains(&WidgetState::Dragged) {
                    return primary.with_opacity(0.1);
                }
                if states.contains(&WidgetState::Hovered) {
                    return primary.with_opacity(0.08);
                }
                if states.contains(&WidgetState::Focused) {
                    return primary.with_opacity(0.1);
                }

                Colors::TRANSPARENT
            },
        ))
        .value_indicator_color(colors.inverse_surface())
        .value_indicator_shape(Rc::new(RoundedRectSliderValueIndicatorShape::new()))
        .thumb_shape(Rc::new(HandleThumbShape::new()))
        .track_shape(Rc::new(GappedSliderTrackShape::new()))
        .overlay_shape(Rc::new(RoundSliderOverlayShape::new()))
        .tick_mark_shape(Rc::new(
            RoundSliderTickMarkShape::new().tick_mark_radius(4.0 / 2.0),
        ))
        .thumb_size(<dyn WidgetStateProperty<Option<Size>>>::resolve_with(
            |states: &WidgetStates| {
                if states.contains(&WidgetState::Disabled) {
                    return Some(Size::new(4.0, 44.0));
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(Size::new(4.0, 44.0));
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(Size::new(2.0, 44.0));
                }
                if states.contains(&WidgetState::Pressed) {
                    return Some(Size::new(2.0, 44.0));
                }
                Some(Size::new(4.0, 44.0))
            },
        ))
        .track_gap(6.0);
    defaults.value_indicator_text_style = theme
        .text_theme
        .label_large
        .as_ref()
        .map(|label_large| label_large.copy_with().color(colors.on_inverse_surface()));
    defaults
}
