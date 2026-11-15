//! `Drawer` and `DrawerHeader`: a drawer from the start edge and an end drawer, each opened by
//! its button in the app bar and closed by a tile, the scrim or a swipe.
//!
//! An `AppBar` on a scaffold with a drawer puts the drawer button where the back button would
//! go, so this page names its `BackButton` as the leading widget and moves the drawer button to
//! the actions, beside the end-drawer button that giving the bar actions displaces.

use waft_foundation::{App, Handle, Listener};
use waft_material::{
    AppBar, BackButton, Drawer, DrawerButton, DrawerHeader, EndDrawerButton, Icons, ListTile,
    Scaffold, ScaffoldState, Theme,
};
use waft_painting::{BoxDecoration, EdgeInsetsGeometry};
use waft_widgets::{
    BuildContext, Builder, Center, Icon, IconData, IntoWidget, ListView, PreferredSizeWidgetRef,
    State, StateData, StatefulWidget, Text, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::caption;

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
}

impl StatefulWidget for DrawersDemo {
    type State = DrawersDemoState;

    fn create_state(&self) -> DrawersDemoState {
        DrawersDemoState {
            state: StateData::new(),
            chosen: None,
        }
    }
}

impl State for DrawersDemoState {
    type Widget = DrawersDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let chosen = app.get(self).chosen;
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
            .body(Center::new().child(caption(app, context, status)))
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
