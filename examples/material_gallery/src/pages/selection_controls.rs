//! Material 3's selection controls: checkboxes (two-state, tristate, error, disabled),
//! radios in a group, switches (plain, with a thumb icon, disabled), and the list-tile form
//! of each.
//!
//! The state of every control lives in [`SelectionDemo`]; a control with no change callback is
//! disabled, which is how the disabled rows below are made.

use std::rc::Rc;

use waft_foundation::{App, Handle};
use waft_material::{
    Checkbox, CheckboxListTile, Icons, Radio, RadioListTile, Switch, SwitchListTile,
};
use waft_rendering::MainAxisSize;
use waft_widgets::{
    BuildContext, Column, Icon, IntoWidget, ListView, RadioGroup, State, StateData, StatefulWidget,
    Text, WidgetRef, WidgetState, WidgetStateProperty,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header, showcase};

const FRUITS: [&str; 3] = ["Apple", "Banana", "Cherry"];

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    page(Entry::SelectionControls.title(), SelectionDemo).into_widget()
}

/// Which of the three two-state, tristate and radio values each demo holds.
#[derive(Debug)]
pub struct SelectionDemo;

/// Dart would spell this `_SelectionDemoState`.
pub struct SelectionDemoState {
    state: StateData<SelectionDemo>,
    checked: bool,
    tristate: Option<bool>,
    fruit: Option<usize>,
    switched: bool,
    icon_switched: bool,
    tile_checked: bool,
    tile_fruit: Option<usize>,
    tile_switched: bool,
}

impl StatefulWidget for SelectionDemo {
    type State = SelectionDemoState;

    fn create_state(&self) -> SelectionDemoState {
        SelectionDemoState {
            state: StateData::new(),
            checked: true,
            tristate: None,
            fruit: Some(0),
            switched: true,
            icon_switched: true,
            tile_checked: false,
            tile_fruit: Some(1),
            tile_switched: false,
        }
    }
}

impl State for SelectionDemoState {
    type Widget = SelectionDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let children = vec![
            section_header(app, context, "Checkboxes"),
            showcase(self.checkboxes(app)),
            caption(
                app,
                context,
                "Tristate cycles unchecked, checked and mixed.",
            ),
            section_header(app, context, "Radios"),
            self.radios(app),
            section_header(app, context, "Switches"),
            showcase(self.switches(app)),
            section_header(app, context, "List tiles"),
            self.checkbox_tiles(app),
            self.radio_tiles(app),
            self.switch_tiles(app),
        ];
        ListView::new().children(children).into_widget()
    }
}

impl SelectionDemoState {
    fn checkboxes(self: Handle<Self>, app: &App) -> Vec<WidgetRef> {
        let (checked, tristate) = {
            let state = app.get(self);
            (state.checked, state.tristate)
        };
        vec![
            Checkbox::new(
                Some(checked),
                Some(Rc::new(move |app: &mut App, value: Option<bool>| {
                    self.set_state(app, |state| state.checked = value.unwrap_or(false));
                })),
            )
            .into_widget(),
            Checkbox::new(
                tristate,
                Some(Rc::new(move |app: &mut App, value: Option<bool>| {
                    self.set_state(app, |state| state.tristate = value);
                })),
            )
            .tristate(true)
            .into_widget(),
            Checkbox::new(
                Some(false),
                Some(Rc::new(|_: &mut App, _: Option<bool>| {})),
            )
            .is_error(true)
            .into_widget(),
            Checkbox::new(Some(true), None).into_widget(),
            Checkbox::new(Some(false), None).into_widget(),
        ]
    }

    fn radios(self: Handle<Self>, app: &App) -> WidgetRef {
        let fruit = app.get(self).fruit;
        let radios = FRUITS
            .iter()
            .enumerate()
            .map(|(index, _)| Radio::new(index).into_widget())
            .chain([Radio::new(FRUITS.len()).enabled(false).into_widget()])
            .collect();
        self.radio_group(fruit, showcase(radios), |state, fruit| state.fruit = fruit)
    }

    fn switches(self: Handle<Self>, app: &App) -> Vec<WidgetRef> {
        let (switched, icon_switched) = {
            let state = app.get(self);
            (state.switched, state.icon_switched)
        };
        let check = Rc::new(Icon::new(Some(Icons::check())));
        let close = Rc::new(Icon::new(Some(Icons::close())));
        vec![
            Switch::new(
                switched,
                Some(Rc::new(move |app: &mut App, value: bool| {
                    self.set_state(app, |state| state.switched = value);
                })),
            )
            .into_widget(),
            Switch::new(
                icon_switched,
                Some(Rc::new(move |app: &mut App, value: bool| {
                    self.set_state(app, |state| state.icon_switched = value);
                })),
            )
            .thumb_icon(<dyn WidgetStateProperty<Option<Rc<Icon>>>>::resolve_with(
                move |states| {
                    Some(if states.contains(&WidgetState::Selected) {
                        Rc::clone(&check)
                    } else {
                        Rc::clone(&close)
                    })
                },
            ))
            .into_widget(),
            Switch::new(true, None).into_widget(),
            Switch::new(false, None).into_widget(),
        ]
    }

    fn checkbox_tiles(self: Handle<Self>, app: &App) -> WidgetRef {
        let checked = app.get(self).tile_checked;
        let enabled = CheckboxListTile::new(
            Some(checked),
            Some(Rc::new(move |app: &mut App, value: Option<bool>| {
                self.set_state(app, |state| state.tile_checked = value.unwrap_or(false));
            })),
        )
        .title(Text::new("Checkbox tile"))
        .subtitle(Text::new("With a subtitle"));
        let disabled =
            CheckboxListTile::new(Some(true), None).title(Text::new("Disabled checkbox tile"));
        column(vec![enabled.into_widget(), disabled.into_widget()])
    }

    fn radio_tiles(self: Handle<Self>, app: &App) -> WidgetRef {
        let fruit = app.get(self).tile_fruit;
        let mut tiles: Vec<WidgetRef> = FRUITS
            .iter()
            .enumerate()
            .map(|(index, name)| {
                RadioListTile::new(index)
                    .title(Text::new(*name))
                    .into_widget()
            })
            .collect();
        tiles.push(
            RadioListTile::new(FRUITS.len())
                .title(Text::new("Disabled"))
                .enabled(false)
                .into_widget(),
        );
        self.radio_group(fruit, column(tiles), |state, fruit| {
            state.tile_fruit = fruit
        })
    }

    fn switch_tiles(self: Handle<Self>, app: &App) -> WidgetRef {
        let switched = app.get(self).tile_switched;
        let enabled = SwitchListTile::new(
            switched,
            Some(Rc::new(move |app: &mut App, value: bool| {
                self.set_state(app, |state| state.tile_switched = value);
            })),
        )
        .title(Text::new("Switch tile"))
        .secondary(Icon::new(Some(Icons::wifi())));
        let disabled = SwitchListTile::new(true, None).title(Text::new("Disabled switch tile"));
        column(vec![enabled.into_widget(), disabled.into_widget()])
    }

    /// A radio group over `child` that stores its value with `store`.
    fn radio_group(
        self: Handle<Self>,
        value: Option<usize>,
        child: WidgetRef,
        store: fn(&mut SelectionDemoState, Option<usize>),
    ) -> WidgetRef {
        let group = RadioGroup::new(
            Rc::new(move |app: &mut App, value: Option<usize>| {
                self.set_state(app, |state| store(state, value));
            }),
            child,
        );
        match value {
            Some(value) => group.group_value(value).into_widget(),
            None => group.into_widget(),
        }
    }
}

fn column(children: Vec<WidgetRef>) -> WidgetRef {
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .children(children)
        .into_widget()
}
