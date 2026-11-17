//! Expansion panels: a list whose panels open independently, and the radio list where opening
//! one closes the other.

use std::rc::Rc;

use waft_foundation::{App, Handle};
use waft_material::{
    ExpansionPanel, ExpansionPanelHeaderBuilder, ExpansionPanelList, ExpansionPanelRadio,
};
use waft_widgets::{
    BuildContext, IntoWidget, ListView, SizedBox, State, StateData, StatefulWidget, Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header};

/// The body text of the first panel.
pub const FIRST_PANEL_BODY: &str = "The first panel's body";

const HEADERS: [&str; 2] = ["Shipping", "Payment"];
const RADIO_HEADERS: [&str; 2] = ["Radio shipping", "Radio payment"];

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    page(Entry::ExpansionPanels.title(), ExpansionPanelsDemo).into_widget()
}

/// Which panels are open.
#[derive(Debug)]
pub struct ExpansionPanelsDemo;

pub struct ExpansionPanelsDemoState {
    state: StateData<ExpansionPanelsDemo>,
    expanded: [bool; HEADERS.len()],
}

impl StatefulWidget for ExpansionPanelsDemo {
    type State = ExpansionPanelsDemoState;

    fn create_state(&self) -> ExpansionPanelsDemoState {
        ExpansionPanelsDemoState {
            state: StateData::new(),
            expanded: [true, false],
        }
    }
}

fn header(label: &'static str) -> ExpansionPanelHeaderBuilder {
    Rc::new(
        move |_app: &mut App, _context: BuildContext, expanded: bool| {
            Text::new(format!(
                "{label} ({})",
                if expanded { "open" } else { "closed" }
            ))
            .into_widget()
        },
    )
}

fn body_text(index: usize) -> WidgetRef {
    SizedBox::new()
        .height(72.0)
        .child(Text::new(if index == 0 {
            FIRST_PANEL_BODY
        } else {
            "The second panel's body"
        }))
        .into_widget()
}

impl State for ExpansionPanelsDemoState {
    type Widget = ExpansionPanelsDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let expanded = app.get(self).expanded;
        let panels: Vec<ExpansionPanel> = (0..HEADERS.len())
            .map(|index| {
                ExpansionPanel::new(header(HEADERS[index]), body_text(index))
                    .is_expanded(expanded[index])
                    .can_tap_on_header(true)
            })
            .collect();
        let list = ExpansionPanelList::new()
            .children(panels)
            .expansion_callback(Rc::new(
                move |app: &mut App, index: usize, is_expanded: bool| {
                    self.set_state(app, |state| state.expanded[index] = is_expanded);
                },
            ));
        let radio = ExpansionPanelList::radio(vec![
            ExpansionPanelRadio::new(0u32, header(RADIO_HEADERS[0]), body_text(0))
                .can_tap_on_header(true),
            ExpansionPanelRadio::new(1u32, header(RADIO_HEADERS[1]), body_text(1))
                .can_tap_on_header(true),
        ]);

        ListView::new()
            .children(vec![
                section_header(app, context, "Independent panels"),
                list.into_widget(),
                caption(app, context, "Each panel opens on its own."),
                section_header(app, context, "Radio panels"),
                radio.into_widget(),
                caption(app, context, "Opening one of these closes the other."),
            ])
            .into_widget()
    }
}
