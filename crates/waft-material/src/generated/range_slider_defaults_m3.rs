//! Flutter counterpart: material_ui's `generated/range_slider_defaults_m3.g.dart`, a part of
//! `range_slider.dart`.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.
#![allow(deprecated)]

use std::rc::Rc;

use waft_embedder::Size;
use waft_foundation::App;
use waft_widgets::{
    BuildContext, WidgetState, WidgetStateProperty, WidgetStatePropertyRef, WidgetStates,
};

use crate::range_slider_parts::{
    GappedRangeSliderTrackShape, HandleRangeSliderThumbShape, RangeSliderThumbShapeRef,
    RangeSliderTickMarkShapeRef, RangeSliderTrackShapeRef, RangeSliderValueIndicatorShapeRef,
    RoundRangeSliderTickMarkShape, RoundedRectRangeSliderValueIndicatorShape,
};
use crate::slider_theme::{ShowValueIndicator, SliderThemeData, SliderThemeDataGetters};
use crate::slider_value_indicator_shape::{RoundSliderOverlayShape, SliderComponentShapeRef};
use crate::theme_data_getters::DefaultsContext;
use waft_embedder::Color;
use waft_painting::{AnyColor, TextStyle};

/// Dart's `_RangeSliderDefaultsM3`.
pub(crate) struct RangeSliderDefaultsM3 {
    /// Dart's `super(..)` arguments.
    slider_theme_data: SliderThemeData,
    context: DefaultsContext,
}

impl RangeSliderDefaultsM3 {
    pub(crate) fn new(context: BuildContext) -> RangeSliderDefaultsM3 {
        RangeSliderDefaultsM3 {
            slider_theme_data: SliderThemeData::new().track_height(16.0),
            context: DefaultsContext::new(context),
        }
    }
}

impl SliderThemeDataGetters for RangeSliderDefaultsM3 {
    fn slider_theme_data(&self) -> &SliderThemeData {
        &self.slider_theme_data
    }

    fn active_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary())
    }

    fn inactive_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.secondary_container())
    }

    fn disabled_active_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface().with_opacity(0.38))
    }

    fn disabled_inactive_track_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface().with_opacity(0.12))
    }

    fn active_tick_mark_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_primary().with_opacity(1.0))
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
        Some(colors.on_surface())
    }

    fn thumb_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary())
    }

    fn overlapping_shape_stroke_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.surface())
    }

    fn disabled_thumb_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.on_surface().with_opacity(0.38))
    }

    fn overlay_color(&self, app: &mut App) -> Option<AnyColor> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.primary().with_opacity(0.12).into())
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

    fn value_indicator_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.inverse_surface())
    }

    fn range_track_shape(&self, _app: &mut App) -> Option<RangeSliderTrackShapeRef> {
        Some(Rc::new(GappedRangeSliderTrackShape::new()))
    }

    fn range_tick_mark_shape(&self, _app: &mut App) -> Option<RangeSliderTickMarkShapeRef> {
        Some(Rc::new(
            RoundRangeSliderTickMarkShape::new().tick_mark_radius(4.0 / 2.0),
        ))
    }

    fn range_thumb_shape(&self, _app: &mut App) -> Option<RangeSliderThumbShapeRef> {
        Some(Rc::new(HandleRangeSliderThumbShape::new()))
    }

    fn overlay_shape(&self, _app: &mut App) -> Option<SliderComponentShapeRef> {
        Some(Rc::new(RoundSliderOverlayShape::new()))
    }

    fn range_value_indicator_shape(
        &self,
        _app: &mut App,
    ) -> Option<RangeSliderValueIndicatorShapeRef> {
        Some(Rc::new(RoundedRectRangeSliderValueIndicatorShape::new()))
    }

    fn show_value_indicator(&self, _app: &mut App) -> Option<ShowValueIndicator> {
        Some(ShowValueIndicator::OnlyForDiscrete)
    }

    fn min_thumb_separation(&self, _app: &mut App) -> Option<f64> {
        Some(0.0)
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
}
