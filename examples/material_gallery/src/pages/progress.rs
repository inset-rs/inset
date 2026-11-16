//! Material 3's progress indicators: linear and circular, determinate and indeterminate, with
//! a slider that drives the determinate ones.

use std::rc::Rc;

use waft_foundation::{App, Handle};
use waft_material::{CircularProgressIndicator, LinearProgressIndicator, Slider};
use waft_painting::EdgeInsetsGeometry;
use waft_widgets::{
    BuildContext, IntoWidget, ListView, Padding, State, StateData, StatefulWidget, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header, showcase};

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    page(Entry::Progress.title(), ProgressDemo).into_widget()
}

/// The value the determinate indicators show.
#[derive(Debug)]
pub struct ProgressDemo;

/// Dart would spell this `_ProgressDemoState`.
pub struct ProgressDemoState {
    state: StateData<ProgressDemo>,
    value: f64,
}

impl StatefulWidget for ProgressDemo {
    type State = ProgressDemoState;

    fn create_state(&self) -> ProgressDemoState {
        ProgressDemoState {
            state: StateData::new(),
            value: 0.6,
        }
    }
}

impl State for ProgressDemoState {
    type Widget = ProgressDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let value = app.get(self).value;
        let children = vec![
            section_header(app, context, "Value"),
            Slider::new(
                value,
                Some(Rc::new(move |app: &mut App, value: f64| {
                    self.set_state(app, |state| state.value = value);
                })),
            )
            .into_widget(),
            caption(
                app,
                context,
                format!("Determinate indicators show {:.0}%", value * 100.0),
            ),
            section_header(app, context, "Linear"),
            padded(LinearProgressIndicator::new().value(value).into_widget()),
            caption(app, context, "Determinate"),
            padded(LinearProgressIndicator::new().into_widget()),
            caption(app, context, "Indeterminate"),
            section_header(app, context, "Circular"),
            showcase(vec![
                CircularProgressIndicator::new().value(value).into_widget(),
                CircularProgressIndicator::new().into_widget(),
            ]),
            caption(app, context, "Determinate, then indeterminate"),
        ];
        ListView::new().children(children).into_widget()
    }
}

fn padded(indicator: WidgetRef) -> WidgetRef {
    Padding::new(EdgeInsetsGeometry::symmetric(8.0, 16.0))
        .child(indicator)
        .into_widget()
}
