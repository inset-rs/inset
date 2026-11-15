//! Flutter counterpart: material_ui's `generated/slider_defaults_m3.g.dart`, a part of
//! `slider.dart`.

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
use crate::slider_parts::SliderTickMarkShapeRef;
use crate::slider_parts::SliderTrackShapeRef;
use crate::slider_parts::{
    GappedSliderTrackShape, HandleThumbShape, RoundSliderTickMarkShape,
    RoundedRectSliderValueIndicatorShape,
};
use crate::slider_theme::{SliderThemeData, SliderThemeDataGetters};
use crate::slider_value_indicator_shape::RoundSliderOverlayShape;
use crate::slider_value_indicator_shape::SliderComponentShapeRef;
use crate::theme_data_getters::DefaultsContext;
use waft_embedder::Color;
use waft_painting::AnyColor;
use waft_painting::TextStyle;
use waft_widgets::WidgetStatePropertyRef;

/// Dart's `_SliderDefaultsM3`.
pub(crate) struct SliderDefaultsM3 {
    /// Dart's `super(..)` arguments.
    slider_theme_data: SliderThemeData,
    context: DefaultsContext,
}

impl SliderDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> SliderDefaultsM3 {
        SliderDefaultsM3 {
            slider_theme_data: SliderThemeData::new().track_height(16.0),
            context: DefaultsContext::new(context),
        }
    }
}

impl SliderThemeDataGetters for SliderDefaultsM3 {
    fn slider_theme_data(&self) -> &SliderThemeData {
        &self.slider_theme_data
    }

    fn active_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary)
    }

    fn inactive_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.secondary_container())
    }

    fn secondary_active_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary.with_opacity(0.54))
    }

    fn disabled_active_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface.with_opacity(0.38))
    }

    fn disabled_inactive_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface.with_opacity(0.12))
    }

    fn disabled_secondary_active_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface.with_opacity(0.38))
    }

    fn active_tick_mark_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_primary.with_opacity(1.0))
    }

    fn inactive_tick_mark_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_secondary_container().with_opacity(1.0))
    }

    fn disabled_active_tick_mark_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_inverse_surface())
    }

    fn disabled_inactive_tick_mark_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface)
    }

    fn thumb_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary)
    }

    fn disabled_thumb_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface.with_opacity(0.38))
    }

    fn overlay_color(&self, app: &mut App) -> Option<AnyColor> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let primary = colors.primary;
        Some(
            WidgetStateColor::resolve_with(move |states: &WidgetStates| {
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
            })
            .into_any(),
        )
    }

    fn value_indicator_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.inverse_surface())
    }

    fn value_indicator_shape(&self, _app: &mut App) -> Option<SliderComponentShapeRef> {
        Some(Rc::new(RoundedRectSliderValueIndicatorShape::new()))
    }

    fn thumb_shape(&self, _app: &mut App) -> Option<SliderComponentShapeRef> {
        Some(Rc::new(HandleThumbShape::new()))
    }

    fn track_shape(&self, _app: &mut App) -> Option<SliderTrackShapeRef> {
        Some(Rc::new(GappedSliderTrackShape::new()))
    }

    fn overlay_shape(&self, _app: &mut App) -> Option<SliderComponentShapeRef> {
        Some(Rc::new(RoundSliderOverlayShape::new()))
    }

    fn tick_mark_shape(&self, _app: &mut App) -> Option<SliderTickMarkShapeRef> {
        Some(Rc::new(
            RoundSliderTickMarkShape::new().tick_mark_radius(4.0 / 2.0),
        ))
    }

    fn thumb_size(&self, _app: &mut App) -> Option<WidgetStatePropertyRef<Option<Size>>> {
        Some(<dyn WidgetStateProperty<Option<Size>>>::resolve_with(
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
    }

    fn track_gap(&self, _app: &mut App) -> Option<f64> {
        Some(6.0)
    }

    fn value_indicator_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(
            theme
                .text_theme
                .label_large
                .as_ref()
                .expect("the text theme has a label large style")
                .copy_with()
                .color(colors.on_inverse_surface()),
        )
    }
}
