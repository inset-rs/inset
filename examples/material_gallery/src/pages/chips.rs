//! Material 3's chips: action, choice, filter and input chips with their elevated forms, the
//! plain `Chip` and a `RawChip`, each enabled and disabled.
//!
//! The demos keep what the chips report: how often the action chip was pressed, which choice
//! is picked, which filters are on and which input chips are still there.

use std::rc::Rc;

use waft_foundation::{App, Handle, Listener, ValueChanged};
use waft_material::{ActionChip, Chip, ChoiceChip, FilterChip, Icons, InputChip, RawChip};
use waft_widgets::{
    BuildContext, Icon, IntoWidget, ListView, State, StateData, StatefulWidget, Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header, showcase};

const SIZES: [&str; 3] = ["Small", "Medium", "Large"];
const FILTERS: [&str; 3] = ["Tea", "Coffee", "Water"];
const RECIPIENTS: [&str; 3] = ["Ada", "Grace", "Linus"];

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    page(Entry::Chips.title(), ChipsDemo.into_widget()).into_widget()
}

/// What the chips report lives here, in the smallest widget that renders it.
#[derive(Debug)]
pub struct ChipsDemo;

pub struct ChipsDemoState {
    state: StateData<ChipsDemo>,
    action_presses: u32,
    choice: usize,
    filters: [bool; FILTERS.len()],
    recipients: [bool; RECIPIENTS.len()],
    raw_selected: bool,
}

impl StatefulWidget for ChipsDemo {
    type State = ChipsDemoState;

    fn create_state(&self) -> ChipsDemoState {
        ChipsDemoState {
            state: StateData::new(),
            action_presses: 0,
            choice: 1,
            filters: [false; FILTERS.len()],
            recipients: [true; RECIPIENTS.len()],
            raw_selected: false,
        }
    }
}

impl State for ChipsDemoState {
    type Widget = ChipsDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let report = format!(
            "Pressed {} times; size {}; filters on: {}; recipients left: {}",
            app.get(self).action_presses,
            SIZES[app.get(self).choice],
            FILTERS
                .iter()
                .zip(app.get(self).filters)
                .filter(|(_, on)| *on)
                .map(|(name, _)| *name)
                .collect::<Vec<_>>()
                .join(", "),
            RECIPIENTS
                .iter()
                .zip(app.get(self).recipients)
                .filter(|(_, present)| *present)
                .map(|(name, _)| *name)
                .collect::<Vec<_>>()
                .join(", "),
        );
        let children = vec![
            caption(app, context, report),
            section_header(app, context, "Action chips"),
            showcase(self.action_chips(app)),
            section_header(app, context, "Choice chips"),
            showcase(self.choice_chips(app)),
            section_header(app, context, "Filter chips"),
            showcase(self.filter_chips(app)),
            section_header(app, context, "Input chips"),
            showcase(self.input_chips(app)),
            section_header(app, context, "Chip and RawChip"),
            showcase(self.plain_chips(app)),
        ];
        ListView::new().children(children).into_widget()
    }
}

impl ChipsDemoState {
    fn action_chips(self: Handle<Self>, _app: &mut App) -> Vec<WidgetRef> {
        let press = Listener::new(move |app: &mut App| {
            self.set_state(app, |state| state.action_presses += 1);
        });
        vec![
            ActionChip::new(Text::new("Action"))
                .on_pressed(press.clone())
                .into_widget(),
            ActionChip::new(Text::new("With icon"))
                .avatar(Icon::new(Some(Icons::event())))
                .on_pressed(press.clone())
                .into_widget(),
            ActionChip::elevated(Text::new("Elevated"))
                .on_pressed(press)
                .into_widget(),
            ActionChip::new(Text::new("Disabled")).into_widget(),
        ]
    }

    fn choice_chips(self: Handle<Self>, app: &mut App) -> Vec<WidgetRef> {
        let picked = app.get(self).choice;
        let mut chips: Vec<WidgetRef> = SIZES
            .iter()
            .enumerate()
            .map(|(index, size)| {
                ChoiceChip::new(Text::new(*size), picked == index)
                    .on_selected(Self::pick(self, index))
                    .into_widget()
            })
            .collect();
        chips.push(
            ChoiceChip::elevated(Text::new("Elevated"), picked == 1)
                .on_selected(Self::pick(self, 1))
                .into_widget(),
        );
        chips.push(ChoiceChip::new(Text::new("Disabled"), true).into_widget());
        chips
    }

    fn pick(self: Handle<Self>, index: usize) -> ValueChanged<bool> {
        Rc::new(move |app: &mut App, selected: bool| {
            if selected {
                self.set_state(app, |state| state.choice = index);
            }
        })
    }

    fn filter_chips(self: Handle<Self>, app: &mut App) -> Vec<WidgetRef> {
        let on = app.get(self).filters;
        let mut chips: Vec<WidgetRef> = FILTERS
            .iter()
            .enumerate()
            .map(|(index, name)| {
                FilterChip::new(Text::new(*name), Some(Self::toggle_filter(self, index)))
                    .selected(on[index])
                    .into_widget()
            })
            .collect();
        chips.push(
            FilterChip::elevated(Text::new("Elevated"), Some(Self::toggle_filter(self, 0)))
                .selected(on[0])
                .into_widget(),
        );
        chips.push(
            FilterChip::new(Text::new("Disabled"), None)
                .selected(true)
                .into_widget(),
        );
        chips
    }

    fn toggle_filter(self: Handle<Self>, index: usize) -> ValueChanged<bool> {
        Rc::new(move |app: &mut App, selected: bool| {
            self.set_state(app, |state| state.filters[index] = selected);
        })
    }

    fn input_chips(self: Handle<Self>, app: &mut App) -> Vec<WidgetRef> {
        let present = app.get(self).recipients;
        let mut chips: Vec<WidgetRef> = RECIPIENTS
            .iter()
            .enumerate()
            .filter(|(index, _)| present[*index])
            .map(|(index, name)| {
                InputChip::new(Text::new(*name))
                    .avatar(Icon::new(Some(Icons::person())))
                    .on_deleted(Listener::new(move |app: &mut App| {
                        self.set_state(app, |state| state.recipients[index] = false);
                    }))
                    .on_pressed(Listener::new(|_app: &mut App| {}))
                    .into_widget()
            })
            .collect();
        chips.push(
            ActionChip::new(Text::new("Restore"))
                .on_pressed(Listener::new(move |app: &mut App| {
                    self.set_state(app, |state| state.recipients = [true; RECIPIENTS.len()]);
                }))
                .into_widget(),
        );
        chips.push(InputChip::new(Text::new("Disabled")).into_widget());
        chips
    }

    fn plain_chips(self: Handle<Self>, app: &mut App) -> Vec<WidgetRef> {
        let selected = app.get(self).raw_selected;
        vec![
            Chip::new(Text::new("Chip"))
                .avatar(Icon::new(Some(Icons::info())))
                .into_widget(),
            RawChip::new(Text::new("RawChip"))
                .selected(selected)
                .on_selected(Rc::new(move |app: &mut App, value: bool| {
                    self.set_state(app, |state| state.raw_selected = value);
                }))
                .into_widget(),
        ]
    }
}
