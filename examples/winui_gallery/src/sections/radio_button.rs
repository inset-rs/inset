//! `RadioButton`: a group of three with one checked, and a disabled checked one.
use crate::{column, example, example_row, label, section};
use inset_foundation::{App, Handle};
use inset_widgets::*;
use inset_winui::*;

/// Builds the examples for this feature destination.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("RadioButton", resources, RadioButtonSection.into_widget())
}

/// Owns both radio selections, as separate `GroupName` values would in XAML.
#[derive(Debug)]
struct RadioButtonSection;

/// Values and property choices retained while this page is visited.
struct RadioButtonSectionState {
    /// Native state binding for the two groups.
    state: StateData<RadioButtonSection>,

    /// Selected index in the original options group.
    selected: usize,

    /// Number of checked events raised by the original group.
    checked_count: u32,

    /// Selection in the independent delivery group.
    delivery: usize,

    /// Enables the second delivery choice.
    express_enabled: bool,
}

impl StatefulWidget for RadioButtonSection {
    type State = RadioButtonSectionState;

    /// Initializes each example with a distinct useful starting value.
    fn create_state(&self) -> RadioButtonSectionState {
        RadioButtonSectionState {
            state: StateData::new(),
            selected: 1,
            checked_count: 0,
            delivery: 0,
            express_enabled: true,
        }
    }
}

/// Labels retained for the original selection and event-count example.
const OPTIONS: [&str; 3] = ["Option 1", "Option 2", "Option 3"];

impl State for RadioButtonSectionState {
    type Widget = RadioButtonSection;
    inset_widgets::state_accessors!();

    /// Builds the examples from their current property choices.
    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let (selected, checked_count) = {
            let state = app.get(self);
            (state.selected, state.checked_count)
        };
        let mut children: Vec<WidgetRef> = OPTIONS
            .iter()
            .enumerate()
            .map(|(index, name)| {
                RadioButton::new(index == selected, move |app| {
                    self.set_state(app, |s| {
                        s.selected = index;
                        s.checked_count += 1;
                    })
                })
                .content(Text::new(*name))
                .into_widget()
            })
            .collect();
        children.push(
            RadioButton::new(true, |_| {})
                .content(Text::new("Disabled"))
                .is_enabled(false)
                .into_widget(),
        );
        children.push(label(
            format!("RadioButton: {}", OPTIONS[selected]),
            TextBlockStyle::Body,
            resources.text_fill_color_secondary,
        ));
        children.push(label(
            format!("Checked {checked_count} times"),
            TextBlockStyle::Caption,
            resources.text_fill_color_secondary,
        ));
        let (delivery, express_enabled) = {
            let state = app.get(self);
            (state.delivery, state.express_enabled)
        };
        column(
            vec![
                example(
                    "A single selection",
                    "Selecting another option updates the group and its checked-event count.",
                    &resources,
                    column(children, 8.0),
                ),
                example(
                    "Independent group and content",
                    "This selection does not change the options above. Disable Express to keep its current selection while preventing input.",
                    &resources,
                    column(
                        vec![
                            CheckBox::new(Some(express_enabled), move |app, value| {
                                self.set_state(app, |s| s.express_enabled = value == Some(true))
                            })
                            .content(Text::new("Enable Express delivery"))
                            .into_widget(),
                            example_row(
                                [
                                    ("Standard delivery", "Arrives in 3–5 days"),
                                    ("Express delivery", "Arrives the next day"),
                                ]
                                .into_iter()
                                .enumerate()
                                .map(|(index, (name, detail))| {
                                    RadioButton::new(delivery == index, move |app| {
                                        self.set_state(app, |s| s.delivery = index)
                                    })
                                    .is_enabled(index == 0 || express_enabled)
                                    .content(column(
                                        vec![
                                            Text::new(name).into_widget(),
                                            label(
                                                detail,
                                                TextBlockStyle::Caption,
                                                resources.text_fill_color_secondary,
                                            ),
                                        ],
                                        2.0,
                                    ))
                                    .into_widget()
                                })
                                .collect(),
                                24.0,
                            ),
                            label(
                                format!(
                                    "Delivery: {}",
                                    if delivery == 0 { "Standard" } else { "Express" }
                                ),
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
