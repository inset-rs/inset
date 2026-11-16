//! Autocomplete: a text field whose options are filtered by what the user has typed.
//!
//! The default field and options list; options that are a custom type, shown by one of its
//! fields; an initial value; the options opening upward; a shorter options list; an options
//! builder that suggests nothing until two letters are typed; and a custom field and options
//! view. Every demo reports its selection in a caption.

use std::fmt::{self, Display};
use std::rc::Rc;

use waft_foundation::{App, Handle, Listener, Task};
use waft_material::{
    Autocomplete, InputDecoration, ListTile, Material, OutlineInputBorder, TextField,
};
use waft_painting::EdgeInsetsGeometry;
use waft_rendering::BoxConstraints;
use waft_services::TextEditingValue;
use waft_widgets::{
    Align, AlignmentGeometry, AutocompleteHighlightedOption, AutocompleteOnSelected, BuildContext,
    ConstrainedBox, IntoWidget, ListView, OptionsViewOpenDirection, Padding, State, StateData,
    StatefulWidget, Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header};

const FRUITS: [&str; 10] = [
    "Apple",
    "Apricot",
    "Banana",
    "Blueberry",
    "Cherry",
    "Grape",
    "Lemon",
    "Mango",
    "Orange",
    "Pear",
];

/// An option that is not a string: the field shows its name, the list shows both.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Person {
    name: &'static str,
    email: &'static str,
}

impl Display for Person {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", self.name, self.email)
    }
}

const PEOPLE: [Person; 3] = [
    Person {
        name: "Alice",
        email: "alice@example.com",
    },
    Person {
        name: "Bob",
        email: "bob@example.com",
    },
    Person {
        name: "Charlie",
        email: "charlie123@gmail.com",
    },
];

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    AutocompleteDemo.into_widget()
}

/// What each autocomplete last selected lives here, in the smallest widget that renders it.
#[derive(Debug)]
pub struct AutocompleteDemo;

/// Dart would spell this `_AutocompleteDemoState`.
pub struct AutocompleteDemoState {
    state: StateData<AutocompleteDemo>,
    fruit: Option<String>,
    person: Option<String>,
    initial: Option<String>,
    upward: Option<String>,
    short: Option<String>,
    two_letters: Option<String>,
    custom: Option<String>,
}

impl StatefulWidget for AutocompleteDemo {
    type State = AutocompleteDemoState;

    fn create_state(&self) -> AutocompleteDemoState {
        AutocompleteDemoState {
            state: StateData::new(),
            fruit: None,
            person: None,
            initial: None,
            upward: None,
            short: None,
            two_letters: None,
            custom: None,
        }
    }
}

impl State for AutocompleteDemoState {
    type Widget = AutocompleteDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let selections = {
            let state = app.get(self);
            [
                state.fruit.clone(),
                state.person.clone(),
                state.initial.clone(),
                state.upward.clone(),
                state.short.clone(),
                state.two_letters.clone(),
                state.custom.clone(),
            ]
        };
        let [fruit, person, initial, upward, short, two_letters, custom] = selections;
        let content = ListView::new().children(vec![
            section_header(app, context, "Default"),
            selected_caption(app, context, &fruit),
            field(fruit_autocomplete().on_selected(selecting(self, |state| &mut state.fruit))),
            section_header(app, context, "Custom option type"),
            selected_caption(app, context, &person),
            field(
                Autocomplete::<Person>::new(person_options)
                    .display_string_for_option(|person: &Person| person.name.to_string())
                    .on_selected(move |app, person| {
                        self.set_state(app, |state| state.person = Some(person.name.to_string()));
                    }),
            ),
            section_header(app, context, "Initial value"),
            selected_caption(app, context, &initial),
            field(
                fruit_autocomplete()
                    .initial_value(TextEditingValue::new().text("ap"))
                    .on_selected(selecting(self, |state| &mut state.initial)),
            ),
            section_header(app, context, "Short options list"),
            selected_caption(app, context, &short),
            field(
                fruit_autocomplete()
                    .options_max_height(100.0)
                    .on_selected(selecting(self, |state| &mut state.short)),
            ),
            section_header(app, context, "Suggests after two letters"),
            selected_caption(app, context, &two_letters),
            field(
                Autocomplete::<String>::new(|app, value| {
                    if value.text.chars().count() < 2 {
                        return Task::ready(Vec::new());
                    }
                    fruit_options(app, value)
                })
                .on_selected(selecting(self, |state| &mut state.two_letters)),
            ),
            section_header(app, context, "Custom field and options"),
            selected_caption(app, context, &custom),
            field(
                fruit_autocomplete()
                    .field_view_builder(|_app, _context, controller, focus_node, on_submitted| {
                        TextField::new()
                            .controller(controller)
                            .focus_node(focus_node)
                            .decoration(Some(
                                InputDecoration::new()
                                    .border(OutlineInputBorder::new())
                                    .label_text("Fruit"),
                            ))
                            .on_submitted(move |app, _value| on_submitted(app))
                            .into_widget()
                    })
                    .options_view_builder(tile_options)
                    .on_selected(selecting(self, |state| &mut state.custom)),
            ),
            section_header(app, context, "Opens upward"),
            selected_caption(app, context, &upward),
            field(
                fruit_autocomplete()
                    .options_view_open_direction(OptionsViewOpenDirection::Up)
                    .on_selected(selecting(self, |state| &mut state.upward)),
            ),
        ]);
        page(Entry::Autocomplete.title(), content).into_widget()
    }
}

fn selected_caption(app: &mut App, context: BuildContext, selection: &Option<String>) -> WidgetRef {
    caption(
        app,
        context,
        format!("Selected: {}", selection.as_deref().unwrap_or("\u{2014}")),
    )
}

/// A demo in the page's column: a gutter around it.
fn field<K>(autocomplete: impl IntoWidget<K>) -> WidgetRef {
    Padding::new(EdgeInsetsGeometry::symmetric(8.0, 16.0))
        .child(autocomplete)
        .into_widget()
}

/// A callback that records the selected option in the field of the state `field` names.
fn selecting(
    demo: Handle<AutocompleteDemoState>,
    field: fn(&mut AutocompleteDemoState) -> &mut Option<String>,
) -> impl Fn(&mut App, String) + 'static {
    move |app, option| {
        demo.set_state(app, |state| *field(state) = Some(option));
    }
}

fn fruit_autocomplete() -> Autocomplete<String> {
    Autocomplete::new(fruit_options)
}

/// The fruits that contain what the field holds, ignoring case.
fn fruit_options(_app: &mut App, value: TextEditingValue) -> Task<Vec<String>> {
    let query = value.text.to_lowercase();
    Task::ready(
        FRUITS
            .iter()
            .filter(|fruit| fruit.to_lowercase().contains(&query))
            .map(|fruit| fruit.to_string())
            .collect(),
    )
}

/// The people whose name or email contains what the field holds, ignoring case.
fn person_options(_app: &mut App, value: TextEditingValue) -> Task<Vec<Person>> {
    let query = value.text.to_lowercase();
    Task::ready(
        PEOPLE
            .iter()
            .filter(|person| person.to_string().to_lowercase().contains(&query))
            .cloned()
            .collect(),
    )
}

/// An options view of list tiles in a card narrower than the field, with the keyboard's
/// highlighted option selected.
fn tile_options(
    app: &mut App,
    context: BuildContext,
    on_selected: AutocompleteOnSelected<String>,
    options: &[String],
) -> WidgetRef {
    let highlighted_index = AutocompleteHighlightedOption::of(app, context);
    let tiles: Vec<WidgetRef> = options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let on_selected = Rc::clone(&on_selected);
            let selection = option.clone();
            ListTile::new()
                .title(Text::new(option.clone()))
                .selected(index == highlighted_index)
                .on_tap(Listener::new(move |app| {
                    on_selected(app, selection.clone());
                }))
                .into_widget()
        })
        .collect();
    Align::new()
        .alignment(AlignmentGeometry::TOP_START)
        .child(
            Material::new().elevation(4.0).child(
                ConstrainedBox::new(BoxConstraints::new().max_width(280.0).max_height(180.0))
                    .child(
                        ListView::new()
                            .padding(EdgeInsetsGeometry::ZERO)
                            .shrink_wrap(true)
                            .children(tiles),
                    ),
            ),
        )
        .into_widget()
}
