//! Determinate and indeterminate ProgressBar examples.

use crate::{column, example, example_row, section};
use inset_foundation::{App, Handle};
use inset_widgets::*;
use inset_winui::*;

/// Builds the progress examples.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("ProgressBar", resources, ProgressDemo.into_widget())
}

/// Interactive progress properties.
#[derive(Debug)]
struct ProgressDemo;

/// Values shared by the determinate and indeterminate previews.
struct ProgressDemoState {
    /// Native widget state.
    state: StateData<ProgressDemo>,

    /// Completed percentage.
    value: f64,

    /// Paused visual state.
    paused: bool,

    /// Error visual state; takes precedence over paused.
    error: bool,
}

impl StatefulWidget for ProgressDemo {
    type State = ProgressDemoState;

    fn create_state(&self) -> Self::State {
        ProgressDemoState {
            state: StateData::new(),
            value: 42.0,
            paused: false,
            error: false,
        }
    }
}

impl State for ProgressDemoState {
    type Widget = ProgressDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let (value, paused, error) = {
            let state = app.get(self);
            (state.value, state.paused, state.error)
        };
        let preview = |indeterminate| {
            ProgressBar::new()
                .value(value)
                .is_indeterminate(indeterminate)
                .show_paused(paused)
                .show_error(error)
                .into_widget()
        };

        column(
            vec![
                example(
                    "States",
                    "Pause or report an error in either kind of progress indicator.",
                    &resources,
                    example_row(
                        vec![
                            CheckBox::new(Some(paused), move |app, checked| {
                                self.set_state(app, |state| state.paused = checked == Some(true));
                            })
                            .content(Text::new("Paused"))
                            .into_widget(),
                            CheckBox::new(Some(error), move |app, checked| {
                                self.set_state(app, |state| state.error = checked == Some(true));
                            })
                            .content(Text::new("Error"))
                            .into_widget(),
                        ],
                        16.0,
                    ),
                ),
                example(
                    "Determinate",
                    "Use a value when the amount of work is known.",
                    &resources,
                    column(
                        vec![
                            Text::new(format!("Completed: {value:.0}%")).into_widget(),
                            preview(false),
                            SizedBox::new()
                                .width(f64::INFINITY)
                                .child(
                                    Slider::new(value, move |app, value| {
                                        self.set_state(app, |state| state.value = value);
                                    })
                                    .header(Text::new("Completion")),
                                )
                                .into_widget(),
                        ],
                        20.0,
                    ),
                ),
                example(
                    "Indeterminate",
                    "Moving segments indicate work whose duration is unknown.",
                    &resources,
                    preview(true),
                ),
                example(
                    "Custom range",
                    "This range runs from 20 to 80; a value of 50 fills half of the indicator.",
                    &resources,
                    ProgressBar::new()
                        .minimum(20.0)
                        .maximum(80.0)
                        .value(50.0)
                        .into_widget(),
                ),
            ],
            16.0,
        )
    }
}
