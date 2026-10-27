//! Expander direction, retained content and disabled examples.

use crate::{column, example, section};
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
use inset_winui::*;

/// Builds the expander examples.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("Expander", resources, ExpanderDemo.into_widget())
}

/// Independent expanders on a retained gallery page.
#[derive(Debug)]
struct ExpanderDemo;

/// Owner values and the most recent lifecycle event.
struct ExpanderDemoState {
    /// Native state identity.
    state: StateData<ExpanderDemo>,

    /// Downward example visibility.
    down: bool,

    /// Upward example visibility.
    up: bool,

    /// Last lifecycle event from the downward example.
    event: &'static str,
}

impl StatefulWidget for ExpanderDemo {
    type State = ExpanderDemoState;

    fn create_state(&self) -> Self::State {
        ExpanderDemoState {
            state: StateData::new(),
            down: false,
            up: true,
            event: "No expansion event yet",
        }
    }
}

impl State for ExpanderDemoState {
    type Widget = ExpanderDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let (down, up, event) = {
            let state = app.get(self);
            (state.down, state.up, state.event)
        };

        column(
            vec![
                example(
                    "Expand downward",
                    "Content retains its state while collapsed. Expansion events occur when the state changes.",
                    &resources,
                    column(
                        vec![
                            Expander::new(down, move |app, expanded| {
                                self.set_state(app, |state| state.down = expanded);
                            })
                            .header(Text::new("Download details"))
                            .content(RetainedCounter)
                            .expanding(Listener::new(move |app| {
                                self.set_state(app, |state| state.event = "Expanding event");
                            }))
                            .collapsed(Listener::new(move |app| {
                                self.set_state(app, |state| state.event = "Collapsed event");
                            }))
                            .into_widget(),
                            Text::new(event).into_widget(),
                        ],
                        12.0,
                    ),
                ),
                example(
                    "Expand upward",
                    "The header stays below its content.",
                    &resources,
                    Expander::new(up, move |app, expanded| {
                        self.set_state(app, |state| state.up = expanded);
                    })
                    .expand_direction(ExpandDirection::Up)
                    .header(Text::new("Storage details"))
                    .content(Text::new("Documents use 24 GB of the available 128 GB."))
                    .into_widget(),
                ),
                example(
                    "Disabled",
                    "The disabled header cannot be toggled.",
                    &resources,
                    Expander::new(false, |_, _| {})
                        .header(Text::new("Unavailable details"))
                        .content(Text::new("This content is unavailable."))
                        .is_enabled(false)
                        .into_widget(),
                ),
            ],
            16.0,
        )
    }
}

/// A stateful child verifies that collapsing retains its local value.
#[derive(Debug)]
struct RetainedCounter;

/// Counter owned by the content, independently of the Expander's owner.
struct RetainedCounterState {
    /// Native widget state.
    state: StateData<RetainedCounter>,

    /// Number of activations.
    count: usize,
}

impl StatefulWidget for RetainedCounter {
    type State = RetainedCounterState;

    fn create_state(&self) -> Self::State {
        RetainedCounterState {
            state: StateData::new(),
            count: 0,
        }
    }
}

impl State for RetainedCounterState {
    type Widget = RetainedCounter;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        let count = app.get(self).count;
        column(
            vec![
                Text::new(format!("Retries: {count}")).into_widget(),
                Button::text(
                    "Retry download",
                    Listener::new(move |app| {
                        self.set_state(app, |state| state.count += 1);
                    }),
                )
                .into_widget(),
            ],
            12.0,
        )
    }
}
