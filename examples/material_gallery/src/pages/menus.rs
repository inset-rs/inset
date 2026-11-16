//! Menus: a `MenuBar` of submenus with shortcuts, checkbox and radio items; a `MenuAnchor` that
//! a button opens; a `PopupMenuButton`; a `DropdownButton`; and a `DropdownMenu`.
//!
//! Every menu reports its choice in a caption. Each family also shows its disabled form: a
//! menu with no callback or no items to choose cannot be opened.

use std::rc::Rc;

use waft_foundation::{App, Handle, Listener};
use waft_material::{
    CheckboxMenuButton, DropdownButton, DropdownMenu, DropdownMenuEntry, DropdownMenuItem,
    FilledButton, Icons, MenuAnchor, MenuBar, MenuItemButton, PopupMenuButton, PopupMenuItem,
    RadioMenuButton, SubmenuButton,
};
use waft_services::LogicalKeyboardKey;
use waft_widgets::{
    BuildContext, Icon, IntoWidget, ListView, SingleActivator, State, StateData, StatefulWidget,
    Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header, showcase};

const FRUITS: [&str; 3] = ["Apple", "Banana", "Cherry"];
const SIZES: [&str; 3] = ["Small", "Medium", "Large"];

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    MenusDemo.into_widget()
}

/// What each menu last chose lives here, in the smallest widget that renders it.
#[derive(Debug)]
pub struct MenusDemo;

/// Dart would spell this `_MenusDemoState`.
pub struct MenusDemoState {
    state: StateData<MenusDemo>,
    last_menu_item: Option<&'static str>,
    show_grid: bool,
    size: &'static str,
    popup_choice: Option<&'static str>,
    dropdown_choice: Option<&'static str>,
    dropdown_menu_choice: Option<&'static str>,
}

impl StatefulWidget for MenusDemo {
    type State = MenusDemoState;

    fn create_state(&self) -> MenusDemoState {
        MenusDemoState {
            state: StateData::new(),
            last_menu_item: None,
            show_grid: false,
            size: SIZES[1],
            popup_choice: None,
            dropdown_choice: None,
            dropdown_menu_choice: None,
        }
    }
}

impl State for MenusDemoState {
    type Widget = MenusDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let chosen = |choice: Option<&'static str>| choice.unwrap_or("\u{2014}");
        let (last_menu_item, popup_choice, dropdown_choice, dropdown_menu_choice) = {
            let state = app.get(self);
            (
                state.last_menu_item,
                state.popup_choice,
                state.dropdown_choice,
                state.dropdown_menu_choice,
            )
        };
        let content = ListView::new().children(vec![
            section_header(app, context, "Menu bar"),
            caption(
                app,
                context,
                format!("Last item: {}", chosen(last_menu_item)),
            ),
            showcase(vec![menu_bar(self, app)]),
            section_header(app, context, "Menu anchor"),
            showcase(vec![menu_anchor(self)]),
            section_header(app, context, "Popup menu"),
            caption(app, context, format!("Chose: {}", chosen(popup_choice))),
            showcase(popup_menus(self)),
            section_header(app, context, "Dropdown button"),
            caption(app, context, format!("Chose: {}", chosen(dropdown_choice))),
            showcase(dropdown_buttons(self, dropdown_choice)),
            section_header(app, context, "Dropdown menu"),
            caption(
                app,
                context,
                format!("Chose: {}", chosen(dropdown_menu_choice)),
            ),
            showcase(dropdown_menus(self)),
        ]);
        page(Entry::Menus.title(), content).into_widget()
    }
}

/// A listener that records `name` as the menu bar's last item.
fn pressing(demo: Handle<MenusDemoState>, name: &'static str) -> Listener {
    Listener::new(move |app: &mut App| {
        demo.set_state(app, |state| state.last_menu_item = Some(name));
    })
}

fn menu_bar(demo: Handle<MenusDemoState>, app: &mut App) -> WidgetRef {
    let (show_grid, size) = {
        let state = app.get(demo);
        (state.show_grid, state.size)
    };
    MenuBar::new(vec![
        SubmenuButton::new(
            vec![
                MenuItemButton::new()
                    .on_pressed(pressing(demo, "New"))
                    .shortcut(Rc::new(
                        SingleActivator::new(LogicalKeyboardKey::KEY_N).control(true),
                    ))
                    .leading_icon(Icon::new(Some(Icons::add())))
                    .child(Text::new("New"))
                    .into_widget(),
                MenuItemButton::new()
                    .on_pressed(pressing(demo, "Open"))
                    .shortcut(Rc::new(
                        SingleActivator::new(LogicalKeyboardKey::KEY_O).control(true),
                    ))
                    .child(Text::new("Open"))
                    .into_widget(),
                MenuItemButton::new()
                    .child(Text::new("Disabled item"))
                    .into_widget(),
            ],
            Some(Text::new("File")),
        )
        .into_widget(),
        SubmenuButton::new(
            vec![
                CheckboxMenuButton::new(
                    Some(show_grid),
                    Some(Rc::new(move |app: &mut App, value: Option<bool>| {
                        demo.set_state(app, |state| state.show_grid = value == Some(true));
                    })),
                    Some(Text::new("Show grid")),
                )
                .into_widget(),
                SubmenuButton::new(
                    SIZES
                        .iter()
                        .map(|name| size_item(demo, name, size))
                        .collect(),
                    Some(Text::new("Size")),
                )
                .into_widget(),
            ],
            Some(Text::new("View")),
        )
        .into_widget(),
    ])
    .into_widget()
}

/// A radio item of the View menu's size group.
fn size_item(demo: Handle<MenusDemoState>, name: &'static str, current: &'static str) -> WidgetRef {
    RadioMenuButton::new(
        name,
        Some(current),
        Some(Rc::new(
            move |app: &mut App, value: Option<&'static str>| {
                if let Some(value) = value {
                    demo.set_state(app, |state| state.size = value);
                }
            },
        )),
        Some(Text::new(name)),
    )
    .into_widget()
}

/// A button that opens and closes a menu anchored under it.
fn menu_anchor(demo: Handle<MenusDemoState>) -> WidgetRef {
    MenuAnchor::new(
        ["Copy", "Paste"]
            .into_iter()
            .map(|name| {
                MenuItemButton::new()
                    .on_pressed(pressing(demo, name))
                    .child(Text::new(name))
                    .into_widget()
            })
            .collect(),
    )
    .builder(|_app, _context, controller, _child| {
        FilledButton::tonal(
            Some(Listener::new(move |app: &mut App| {
                if controller.is_open(app) {
                    controller.close(app);
                } else {
                    controller.open(app, None);
                }
            })),
            Some(Text::new("Open menu")),
        )
        .into_widget()
    })
    .into_widget()
}

fn popup_menus(demo: Handle<MenusDemoState>) -> Vec<WidgetRef> {
    let items = |_app: &mut App, _context: BuildContext| {
        FRUITS
            .iter()
            .map(|name| {
                waft_material::PopupMenuEntryRef::new(
                    PopupMenuItem::new(Some(Text::new(*name))).value(*name),
                )
            })
            .collect()
    };
    vec![
        PopupMenuButton::<&'static str>::new(items)
            .tooltip("Show menu")
            .on_selected(move |app, name| {
                demo.set_state(app, |state| state.popup_choice = Some(name));
            })
            .into_widget(),
        PopupMenuButton::<&'static str>::new(items)
            .child(Text::new("With a child"))
            .on_selected(move |app, name| {
                demo.set_state(app, |state| state.popup_choice = Some(name));
            })
            .into_widget(),
        PopupMenuButton::<&'static str>::new(items)
            .enabled(false)
            .into_widget(),
    ]
}

fn dropdown_buttons(demo: Handle<MenusDemoState>, current: Option<&'static str>) -> Vec<WidgetRef> {
    let items = || {
        FRUITS
            .iter()
            .map(|name| DropdownMenuItem::new(Text::new(*name)).value(*name))
            .collect::<Vec<_>>()
    };
    vec![
        DropdownButton::new(
            Some(items()),
            Some(Rc::new(
                move |app: &mut App, value: Option<&'static str>| {
                    demo.set_state(app, |state| state.dropdown_choice = value);
                },
            )),
        )
        .value(current)
        .hint(Text::new("Pick a fruit"))
        .into_widget(),
        DropdownButton::new(Some(items()), None)
            .hint(Text::new("Disabled"))
            .into_widget(),
    ]
}

fn dropdown_menus(demo: Handle<MenusDemoState>) -> Vec<WidgetRef> {
    let entries = || {
        vec![
            DropdownMenuEntry::new("Apple", "Apple").leading_icon(Icon::new(Some(Icons::eco()))),
            DropdownMenuEntry::new("Banana", "Banana"),
            DropdownMenuEntry::new("Cherry", "Cherry (out of season)").enabled(false),
        ]
    };
    vec![
        DropdownMenu::new(entries())
            .label(Text::new("Fruit"))
            .hint_text("Choose")
            .on_selected(move |app, value| {
                demo.set_state(app, |state| {
                    state.dropdown_menu_choice = value;
                });
            })
            .into_widget(),
        DropdownMenu::new(entries())
            .label(Text::new("Searchable"))
            .enable_filter(true)
            .into_widget(),
        DropdownMenu::new(entries())
            .label(Text::new("Disabled"))
            .enabled(false)
            .into_widget(),
    ]
}
