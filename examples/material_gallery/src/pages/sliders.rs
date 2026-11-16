//! Material 3's sliders: a continuous and a discrete `Slider` with a value label, a disabled
//! one, and a continuous, a discrete and a disabled `RangeSlider`.
//!
//! The value of each slider is shown in a caption under it, so a drag is visible without the
//! value indicator.

use std::rc::Rc;

use waft_foundation::{App, Handle};
use waft_material::{RangeLabels, RangeSlider, RangeValues, Slider};
use waft_widgets::{
    BuildContext, IntoWidget, ListView, State, StateData, StatefulWidget, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header};

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    page(Entry::Sliders.title(), SlidersDemo).into_widget()
}

/// The value of each interactive slider.
#[derive(Debug)]
pub struct SlidersDemo;

/// Dart would spell this `_SlidersDemoState`.
pub struct SlidersDemoState {
    state: StateData<SlidersDemo>,
    continuous: f64,
    discrete: f64,
    range: RangeValues,
    discrete_range: RangeValues,
}

impl StatefulWidget for SlidersDemo {
    type State = SlidersDemoState;

    fn create_state(&self) -> SlidersDemoState {
        SlidersDemoState {
            state: StateData::new(),
            continuous: 40.0,
            discrete: 60.0,
            range: RangeValues::new(20.0, 70.0),
            discrete_range: RangeValues::new(20.0, 80.0),
        }
    }
}

impl State for SlidersDemoState {
    type Widget = SlidersDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let (continuous, discrete, range, discrete_range) = {
            let state = app.get(self);
            (
                state.continuous,
                state.discrete,
                state.range,
                state.discrete_range,
            )
        };
        let children = vec![
            section_header(app, context, "Slider"),
            self.continuous_slider(continuous),
            caption(app, context, format!("Continuous: {continuous:.1}")),
            self.discrete_slider(discrete),
            caption(
                app,
                context,
                format!("Discrete, five divisions: {discrete:.0}"),
            ),
            Slider::new(30.0, None).min(0.0).max(100.0).into_widget(),
            caption(app, context, "Disabled"),
            section_header(app, context, "Range slider"),
            self.range_slider(range),
            caption(
                app,
                context,
                format!("Continuous: {:.1} to {:.1}", range.start, range.end),
            ),
            self.discrete_range_slider(discrete_range),
            caption(
                app,
                context,
                format!(
                    "Discrete: {:.0} to {:.0}",
                    discrete_range.start, discrete_range.end
                ),
            ),
            RangeSlider::new(RangeValues::new(25.0, 75.0), None)
                .min(0.0)
                .max(100.0)
                .into_widget(),
            caption(app, context, "Disabled"),
        ];
        ListView::new().children(children).into_widget()
    }
}

impl SlidersDemoState {
    fn continuous_slider(self: Handle<Self>, value: f64) -> WidgetRef {
        Slider::new(
            value,
            Some(Rc::new(move |app: &mut App, value: f64| {
                self.set_state(app, |state| state.continuous = value);
            })),
        )
        .min(0.0)
        .max(100.0)
        .label(format!("{value:.1}"))
        .into_widget()
    }

    fn discrete_slider(self: Handle<Self>, value: f64) -> WidgetRef {
        Slider::new(
            value,
            Some(Rc::new(move |app: &mut App, value: f64| {
                self.set_state(app, |state| state.discrete = value);
            })),
        )
        .min(0.0)
        .max(100.0)
        .divisions(5)
        .label(format!("{value:.0}"))
        .into_widget()
    }

    fn range_slider(self: Handle<Self>, values: RangeValues) -> WidgetRef {
        RangeSlider::new(
            values,
            Some(Rc::new(move |app: &mut App, values: RangeValues| {
                self.set_state(app, |state| state.range = values);
            })),
        )
        .min(0.0)
        .max(100.0)
        .labels(RangeLabels::new(
            format!("{:.1}", values.start),
            format!("{:.1}", values.end),
        ))
        .into_widget()
    }

    fn discrete_range_slider(self: Handle<Self>, values: RangeValues) -> WidgetRef {
        RangeSlider::new(
            values,
            Some(Rc::new(move |app: &mut App, values: RangeValues| {
                self.set_state(app, |state| state.discrete_range = values);
            })),
        )
        .min(0.0)
        .max(100.0)
        .divisions(10)
        .labels(RangeLabels::new(
            format!("{:.0}", values.start),
            format!("{:.0}", values.end),
        ))
        .into_widget()
    }
}
