//! `CheckBox` unchecked, checked, three-state (starting indeterminate) and disabled, with a label reporting their `IsChecked` values.
use crate::{column, example, example_row, label, section};
use inset_foundation::{App, Handle};
use inset_widgets::*;
use inset_winui::*;

/// Builds the examples for this feature destination.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("CheckBox", resources, CheckBoxDemo.into_widget())
}

/// Owns the independent interactive examples on this page.
#[derive(Debug)]
struct CheckBoxDemo;

/// Values and property choices retained while this page is visited.
struct CheckBoxDemoState {
    /// Native state binding for this example.
    state: StateData<CheckBoxDemo>,

    /// Current value of the initially unchecked option.
    two_state: Option<bool>,

    /// Current value of the initially checked option.
    checked: Option<bool>,

    /// Current value of the initially mixed option.
    three_state: Option<bool>,

    /// Independent value of the configurable example.
    sample: Option<bool>,

    /// Whether the configurable example accepts input.
    enabled: bool,

    /// Whether its click cycle includes the indeterminate state.
    allow_mixed: bool,
}

impl StatefulWidget for CheckBoxDemo {
    type State = CheckBoxDemoState;

    /// Initializes each example with a distinct useful starting value.
    fn create_state(&self) -> CheckBoxDemoState {
        CheckBoxDemoState {
            state: StateData::new(),
            two_state: Some(false),
            checked: Some(true),
            three_state: None,
            sample: Some(false),
            enabled: true,
            allow_mixed: true,
        }
    }
}

/// User-facing name of each nullable checked value.
fn describe(is_checked: Option<bool>) -> &'static str {
    match is_checked {
        Some(true) => "checked",
        Some(false) => "unchecked",
        None => "indeterminate",
    }
}

impl State for CheckBoxDemoState {
    type Widget = CheckBoxDemo;
    inset_widgets::state_accessors!();

    /// Builds the examples from their current property choices.
    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let (two_state, checked, three_state) = {
            let state = app.get(self);
            (state.two_state, state.checked, state.three_state)
        };
        let status = format!(
            "CheckBox: {} · {} · {}",
            describe(two_state),
            describe(checked),
            describe(three_state)
        );
        let basic = column(
            vec![
                CheckBox::new(two_state, move |app, value| {
                    self.set_state(app, |s| s.two_state = value)
                })
                .content(Text::new("Two-state option"))
                .into_widget(),
                CheckBox::new(checked, move |app, value| {
                    self.set_state(app, |s| s.checked = value)
                })
                .content(Text::new("Checked option"))
                .into_widget(),
                CheckBox::new(three_state, move |app, value| {
                    self.set_state(app, |s| s.three_state = value)
                })
                .is_three_state(true)
                .content(Text::new("Three-state option"))
                .into_widget(),
                CheckBox::new(Some(true), |_, _| {})
                    .is_enabled(false)
                    .content(Text::new("Disabled option"))
                    .into_widget(),
                label(
                    status,
                    TextBlockStyle::Body,
                    resources.text_fill_color_secondary,
                ),
            ],
            8.0,
        );
        let (sample, enabled, allow_mixed) = {
            let state = app.get(self);
            (state.sample, state.enabled, state.allow_mixed)
        };
        column(
            vec![
                example(
                    "Selection states",
                    "Compare the two-state and three-state click cycles.",
                    &resources,
                    basic,
                ),
                example(
                    "Configure a check box",
                    "Change the properties, then click the option to inspect its value.",
                    &resources,
                    column(
                        vec![
                            example_row(
                                vec![
                                    CheckBox::new(Some(enabled), move |app, value| {
                                        self.set_state(app, |s| s.enabled = value == Some(true))
                                    })
                                    .content(Text::new("Enable configurable check box"))
                                    .into_widget(),
                                    CheckBox::new(Some(allow_mixed), move |app, value| {
                                        self.set_state(app, |s| s.allow_mixed = value == Some(true))
                                    })
                                    .content(Text::new("Allow indeterminate state"))
                                    .into_widget(),
                                ],
                                16.0,
                            ),
                            CheckBox::new(sample, move |app, value| {
                                self.set_state(app, |s| s.sample = value)
                            })
                            .is_enabled(enabled)
                            .is_three_state(allow_mixed)
                            .content(column(
                                vec![
                                    Text::new("Include optional files").into_widget(),
                                    label(
                                        "Content can contain more than a single line of text.",
                                        TextBlockStyle::Caption,
                                        resources.text_fill_color_secondary,
                                    ),
                                ],
                                2.0,
                            ))
                            .into_widget(),
                            label(
                                format!("Configurable value: {}", describe(sample)),
                                TextBlockStyle::Body,
                                resources.text_fill_color_secondary,
                            ),
                        ],
                        12.0,
                    ),
                ),
            ],
            16.0,
        )
    }
}
