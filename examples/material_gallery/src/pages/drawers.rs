//! `Drawer` and `DrawerHeader`: a drawer from the start edge and an end drawer, each opened by
//! its button in the app bar and closed by a tile, the scrim or a swipe.
//!
//! An `AppBar` on a scaffold with a drawer puts the drawer button where the back button would
//! go, so this page names its `BackButton` as the leading widget and moves the drawer button to
//! the actions, beside the end-drawer button that giving the bar actions displaces.
//!
//! The body also shows the destination-based navigation widgets that share a family with the
//! drawers: a `NavigationBar`, a `NavigationRail` (collapsed and extended) and a
//! `NavigationDrawer`, each remembering its selected destination.

use std::rc::Rc;
use waft_foundation::{App, Handle, Listener};
use waft_material::{
    AppBar, BackButton, Drawer, DrawerButton, DrawerHeader, EndDrawerButton, Icons, ListTile,
    NavigationBar, NavigationDestination, NavigationDrawer, NavigationDrawerDestination,
    NavigationRail, NavigationRailDestination, NavigationRailLabelType, Scaffold, ScaffoldState,
    Theme,
};
use waft_painting::{BoxDecoration, EdgeInsetsGeometry};

use waft_widgets::{
    BuildContext, Builder, Icon, IconData, IntoWidget, ListView, Padding, PreferredSizeWidgetRef,
    Row, SizedBox, State, StateData, StatefulWidget, Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, section_header};

pub const DRAWER_HEADER: &str = "Drawer header";
pub const END_DRAWER_HEADER: &str = "End drawer header";

/// A drawer tile: its title, and the glyph leading it.
type DrawerTile = (&'static str, fn() -> IconData);

/// The start drawer's tiles.
const DESTINATIONS: [DrawerTile; 3] = [
    ("Inbox", Icons::inbox),
    ("Outbox", Icons::send),
    ("Favourites", Icons::favorite),
];

/// The end drawer's tiles.
const FILTERS: [DrawerTile; 2] = [("Unread first", Icons::mail), ("Starred only", Icons::star)];

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    DrawersDemo.into_widget()
}

/// The last tile chosen from either drawer lives here.
#[derive(Debug)]
pub struct DrawersDemo;

/// Dart would spell this `_DrawersDemoState`.
pub struct DrawersDemoState {
    state: StateData<DrawersDemo>,
    chosen: Option<&'static str>,
    bar_index: usize,
    rail_index: usize,
    drawer_index: i32,
}

impl StatefulWidget for DrawersDemo {
    type State = DrawersDemoState;

    fn create_state(&self) -> DrawersDemoState {
        DrawersDemoState {
            state: StateData::new(),
            chosen: None,
            bar_index: 0,
            rail_index: 0,
            drawer_index: 0,
        }
    }
}

impl State for DrawersDemoState {
    type Widget = DrawersDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let (chosen, bar_index, rail_index, drawer_index) = {
            let state = app.get(self);
            (
                state.chosen,
                state.bar_index,
                state.rail_index,
                state.drawer_index,
            )
        };
        let status = match chosen {
            Some(name) => format!("Chose {name}"),
            None => "Open a drawer from the app bar".to_owned(),
        };

        Scaffold::new()
            .app_bar(PreferredSizeWidgetRef::new(
                AppBar::new()
                    .leading(BackButton::new())
                    .title(Text::new(Entry::Drawers.title()))
                    .actions(vec![
                        DrawerButton::new().into_widget(),
                        EndDrawerButton::new().into_widget(),
                    ]),
            ))
            .drawer(drawer(
                self,
                DRAWER_HEADER,
                &DESTINATIONS,
                ScaffoldState::close_drawer,
            ))
            .end_drawer(drawer(
                self,
                END_DRAWER_HEADER,
                &FILTERS,
                ScaffoldState::close_end_drawer,
            ))
            .body(ListView::new().children(vec![
                caption(app, context, status),
                section_header(app, context, "Navigation bar"),
                navigation_bar(self, bar_index),
                section_header(app, context, "Navigation rail"),
                navigation_rails(self, rail_index),
                section_header(app, context, "Navigation drawer"),
                navigation_drawer(self, drawer_index),
            ]))
            .into_widget()
    }
}

/// A drawer of `tiles` under a header; a tile records its name, then `close`s the drawer it is
/// in.
fn drawer(
    state: Handle<DrawersDemoState>,
    header: &'static str,
    tiles: &'static [DrawerTile],
    close: fn(Handle<ScaffoldState>, &mut App),
) -> WidgetRef {
    Drawer::new()
        .child(Builder::new(move |app: &mut App, context: BuildContext| {
            let primary_container = Theme::of(app, context).color_scheme.primary_container();
            let mut children = vec![
                DrawerHeader::new(Some(Text::new(header).into_widget()))
                    .decoration(BoxDecoration::new().color(primary_container))
                    .into_widget(),
            ];
            children.extend(tiles.iter().map(|(name, icon)| {
                let name = *name;
                ListTile::new()
                    .leading(Icon::new(Some(icon())))
                    .title(Text::new(name))
                    .on_tap(Listener::new(move |app: &mut App| {
                        state.set_state(app, |state| state.chosen = Some(name));
                        close(Scaffold::of(app, context), app);
                    }))
                    .into_widget()
            }));
            ListView::new()
                .padding(EdgeInsetsGeometry::ZERO)
                .children(children)
                .into_widget()
        }))
        .into_widget()
}

/// The destinations the bar, the rails and the embedded drawer share.
const SECTIONS: [DrawerTile; 3] = [
    ("Explore", Icons::explore),
    ("Chats", Icons::chat),
    ("Profile", Icons::person),
];

/// A bottom navigation bar of the three sections.
fn navigation_bar(state: Handle<DrawersDemoState>, selected: usize) -> WidgetRef {
    NavigationBar::new(SECTIONS.iter().map(|(name, icon)| {
        NavigationDestination::new(Icon::new(Some(icon())), *name).into_widget()
    }))
    .selected_index(selected)
    .on_destination_selected(move |app, index| {
        state.set_state(app, |state| state.bar_index = index);
    })
    .into_widget()
}

/// A rail with labels under the selected destination, and an extended one beside it.
fn navigation_rails(state: Handle<DrawersDemoState>, selected: usize) -> WidgetRef {
    let rail = move |extended: bool| {
        let destinations = SECTIONS
            .iter()
            .map(|(name, icon)| {
                NavigationRailDestination::new(Icon::new(Some(icon())), Text::new(*name))
            })
            .collect();
        NavigationRail::new(destinations, Some(selected))
            .extended(extended)
            .label_type(if extended {
                NavigationRailLabelType::None
            } else {
                NavigationRailLabelType::Selected
            })
            .on_destination_selected(move |app, index| {
                state.set_state(app, |state| state.rail_index = index);
            })
            .into_widget()
    };
    SizedBox::new()
        .height(280.0)
        .child(Row::new().children([rail(false), rail(true)]))
        .into_widget()
}

/// A navigation drawer built into the page, with a header and a section label.
fn navigation_drawer(state: Handle<DrawersDemoState>, selected: i32) -> WidgetRef {
    let mut children = vec![
        Padding::new(EdgeInsetsGeometry::from_ltrb(28.0, 16.0, 16.0, 10.0))
            .child(Text::new("Sections"))
            .into_widget(),
    ];
    children.extend(SECTIONS.iter().map(|(name, icon)| {
        NavigationDrawerDestination::new(Icon::new(Some(icon())), Text::new(*name)).into_widget()
    }));
    SizedBox::new()
        .height(260.0)
        .width(304.0)
        .child(
            NavigationDrawer::new(children)
                .selected_index(Some(selected))
                .on_destination_selected(Rc::new(move |app: &mut App, index: i32| {
                    state.set_state(app, |state| state.drawer_index = index);
                })),
        )
        .into_widget()
}
