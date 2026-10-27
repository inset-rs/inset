//! `ToggleButton` states, `RepeatButton` timing and `HyperlinkButton` content.

use crate::{column, example, example_row, label, row, section};
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
use inset_winui::*;
use std::time::Duration;

/// Builds the ToggleButton examples and their retained state.
pub fn toggle_button(resources: &ThemeResources) -> Vec<WidgetRef> {
    section(
        "ToggleButton",
        resources,
        example(
            "States",
            "The second button cycles through checked, indeterminate and unchecked states.",
            resources,
            ToggleButtonDemo.into_widget(),
        ),
    )
}

/// Builds the RepeatButton examples and their counter.
pub fn repeat_button(resources: &ThemeResources) -> Vec<WidgetRef> {
    section(
        "RepeatButton",
        resources,
        example(
            "Timing",
            "Hold a button to compare the default delay and interval with a faster custom timing.",
            resources,
            RepeatButtonDemo.into_widget(),
        ),
    )
}

/// Builds the HyperlinkButton examples and their counter.
pub fn hyperlink_button(resources: &ThemeResources) -> Vec<WidgetRef> {
    section(
        "HyperlinkButton",
        resources,
        example(
            "Content and enabled state",
            "HyperlinkButton accepts text or composed widget content while retaining link styling.",
            resources,
            HyperlinkButtonDemo.into_widget(),
        ),
    )
}

/// The status label under each demo, in the secondary text colour.
fn status(app: &mut App, context: BuildContext, text: String) -> WidgetRef {
    let resources = ThemeResources::of(app, context);
    label(
        text,
        TextBlockStyle::Body,
        resources.text_fill_color_secondary,
    )
}

/// Formats the three possible `ToggleButton` values for the status label.
fn describe(is_checked: Option<bool>) -> &'static str {
    match is_checked {
        Some(true) => "checked",
        Some(false) => "unchecked",
        None => "indeterminate",
    }
}

/// Stateful content for the two-state, three-state and disabled toggle examples.
#[derive(Debug)]
struct ToggleButtonDemo;

/// Retains the values shown by `ToggleButtonDemo`.
struct ToggleButtonDemoState {
    /// Widget lifecycle data.
    state: StateData<ToggleButtonDemo>,

    /// Value of the ordinary two-state button.
    two_state: Option<bool>,

    /// Value of the three-state button.
    three_state: Option<bool>,
}

impl StatefulWidget for ToggleButtonDemo {
    type State = ToggleButtonDemoState;

    fn create_state(&self) -> ToggleButtonDemoState {
        ToggleButtonDemoState {
            state: StateData::new(),
            two_state: Some(false),
            three_state: Some(false),
        }
    }
}

impl State for ToggleButtonDemoState {
    type Widget = ToggleButtonDemo;

    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let (two_state, three_state) = (app.get(self).two_state, app.get(self).three_state);
        column(
            vec![
                example_row(
                    vec![
                        ToggleButton::text("Toggle me", two_state, move |app, value| {
                            self.set_state(app, |s| s.two_state = value)
                        })
                        .into_widget(),
                        ToggleButton::text("Three-state", three_state, move |app, value| {
                            self.set_state(app, |s| s.three_state = value)
                        })
                        .is_three_state(true)
                        .into_widget(),
                        ToggleButton::text("Disabled unchecked", Some(false), |_, _| {})
                            .is_enabled(false)
                            .into_widget(),
                        ToggleButton::text("Disabled checked", Some(true), |_, _| {})
                            .is_enabled(false)
                            .into_widget(),
                    ],
                    8.0,
                ),
                status(
                    app,
                    context,
                    format!(
                        "ToggleButton: {} · three-state {}",
                        describe(two_state),
                        describe(three_state)
                    ),
                ),
            ],
            8.0,
        )
    }
}

/// Stateful content for the repeat timing and counter examples.
#[derive(Debug)]
struct RepeatButtonDemo;

/// Retains the number of repeat activations shown by `RepeatButtonDemo`.
struct RepeatButtonDemoState {
    /// Widget lifecycle data.
    state: StateData<RepeatButtonDemo>,

    /// Number of activations received by the enabled repeat buttons.
    clicks: u32,
}

impl StatefulWidget for RepeatButtonDemo {
    type State = RepeatButtonDemoState;

    fn create_state(&self) -> RepeatButtonDemoState {
        RepeatButtonDemoState {
            state: StateData::new(),
            clicks: 0,
        }
    }
}

impl State for RepeatButtonDemoState {
    type Widget = RepeatButtonDemo;

    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let clicks = app.get(self).clicks;
        let click = Listener::new(move |app| self.set_state(app, |s| s.clicks += 1));
        column(
            vec![
                example_row(
                    vec![
                        RepeatButton::text("Hold me", click.clone()).into_widget(),
                        RepeatButton::text("Fast repeat", click)
                            .delay(Duration::from_millis(350))
                            .interval(Duration::from_millis(120))
                            .into_widget(),
                        RepeatButton::text("Disabled repeat", Listener::new(|_| {}))
                            .is_enabled(false)
                            .into_widget(),
                    ],
                    8.0,
                ),
                status(app, context, format!("RepeatButton: {clicks} clicks")),
            ],
            8.0,
        )
    }
}

/// Stateful content for the text and composed-content hyperlink examples.
#[derive(Debug)]
struct HyperlinkButtonDemo;

/// Retains the number of enabled hyperlink activations shown in the status label.
struct HyperlinkButtonDemoState {
    /// Widget lifecycle data.
    state: StateData<HyperlinkButtonDemo>,

    /// Number of activations received by the enabled text link.
    clicks: u32,
}

impl StatefulWidget for HyperlinkButtonDemo {
    type State = HyperlinkButtonDemoState;

    fn create_state(&self) -> HyperlinkButtonDemoState {
        HyperlinkButtonDemoState {
            state: StateData::new(),
            clicks: 0,
        }
    }
}

impl State for HyperlinkButtonDemoState {
    type Widget = HyperlinkButtonDemo;

    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let clicks = app.get(self).clicks;
        let click = Listener::new(move |app| self.set_state(app, |s| s.clicks += 1));
        column(
            vec![
                example_row(
                    vec![
                        HyperlinkButton::text("Learn more", click.clone()).into_widget(),
                        HyperlinkButton::new(
                            row(
                                vec![
                                    FontIcon::symbol(FluentSymbol::Search).into_widget(),
                                    Text::new("Icon link").into_widget(),
                                ],
                                8.0,
                            ),
                            click,
                        )
                        .into_widget(),
                        HyperlinkButton::text("Disabled link", Listener::new(|_| {}))
                            .is_enabled(false)
                            .into_widget(),
                    ],
                    8.0,
                ),
                status(app, context, format!("HyperlinkButton: {clicks} clicks")),
            ],
            8.0,
        )
    }
}
