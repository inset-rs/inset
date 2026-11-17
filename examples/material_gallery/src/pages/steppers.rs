//! Steppers: a vertical stepper whose steps open in place, and the horizontal layout where the
//! content sits below the icons.

use std::rc::Rc;

use waft_foundation::{App, Handle, Listener};
use waft_material::{Material, Step, StepState, Stepper, StepperType};
use waft_widgets::{
    BuildContext, IntoWidget, ListView, SizedBox, State, StateData, StatefulWidget, Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header};

/// The title the continue button reports on the caption.
pub const LAST_ACTION: &str = "Last action: \u{2014}";

const TITLES: [&str; 3] = ["Address", "Payment", "Review"];

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    page(Entry::Steppers.title(), SteppersDemo).into_widget()
}

/// The current step and the last reported action live here.
#[derive(Debug)]
pub struct SteppersDemo;

pub struct SteppersDemoState {
    state: StateData<SteppersDemo>,
    current: usize,
    last_action: String,
}

impl StatefulWidget for SteppersDemo {
    type State = SteppersDemoState;

    fn create_state(&self) -> SteppersDemoState {
        SteppersDemoState {
            state: StateData::new(),
            current: 0,
            last_action: LAST_ACTION.to_string(),
        }
    }
}

impl SteppersDemoState {
    fn steps(self: Handle<Self>, app: &mut App) -> Vec<Step> {
        let current = app.get(self).current;
        TITLES
            .iter()
            .enumerate()
            .map(|(index, title)| {
                let mut step = Step::new(
                    Text::new(*title),
                    SizedBox::new()
                        .height(48.0)
                        .child(Text::new(format!("Content for {title}")))
                        .into_widget(),
                )
                .is_active(index == current);
                if index < current {
                    step = step.state(StepState::Complete);
                }
                step.subtitle(Text::new(format!("Step {}", index + 1)))
            })
            .collect()
    }

    fn report(self: Handle<Self>, app: &mut App, message: impl Into<String>) {
        let message = message.into();
        self.set_state(app, |state| state.last_action = message);
    }
}

impl State for SteppersDemoState {
    type Widget = SteppersDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let current = app.get(self).current;
        let last_action = app.get(self).last_action.clone();

        let vertical = Stepper::new(self.steps(app))
            .current_step(current)
            .on_step_tapped(Rc::new(move |app: &mut App, index: usize| {
                self.set_state(app, |state| state.current = index);
                self.report(app, format!("Tapped step {}", index + 1));
            }))
            .on_step_continue(Listener::new(move |app: &mut App| {
                let next = (app.get(self).current + 1).min(TITLES.len() - 1);
                self.set_state(app, |state| state.current = next);
                self.report(app, format!("Continued to step {}", next + 1));
            }))
            .on_step_cancel(Listener::new(move |app: &mut App| {
                let previous = app.get(self).current.saturating_sub(1);
                self.set_state(app, |state| state.current = previous);
                self.report(app, format!("Cancelled to step {}", previous + 1));
            }));

        let horizontal = Stepper::new(self.steps(app))
            .r#type(StepperType::Horizontal)
            .current_step(current)
            .on_step_tapped(Rc::new(move |app: &mut App, index: usize| {
                self.set_state(app, |state| state.current = index);
                self.report(app, format!("Tapped step {}", index + 1));
            }));

        ListView::new()
            .children(vec![
                section_header(app, context, "Vertical"),
                Material::new().child(vertical).into_widget(),
                caption(app, context, last_action),
                section_header(app, context, "Horizontal"),
                SizedBox::new()
                    .height(420.0)
                    .child(Material::new().child(horizontal))
                    .into_widget(),
                caption(
                    app,
                    context,
                    "The horizontal stepper keeps its content below the row.",
                ),
            ])
            .into_widget()
    }
}
