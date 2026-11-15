//! The catalog index.
//!
//! A `Scaffold` whose `AppBar` carries the theme toggle, whose `Drawer` lists the entries
//! again, and whose body is a `ListView` of `ListTile`s — the same widgets the Lists, Drawers
//! and App Bars entries demonstrate.

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::Brightness;
use waft_foundation::{App, Listener, ValueChanged};
use waft_material::{
    AppBar, Drawer, DrawerHeader, IconButton, Icons, ListTile, Scaffold, Theme, ThemeMode,
};
use waft_painting::{BoxDecoration, EdgeInsetsGeometry};
use waft_widgets::{
    BuildContext, Builder, Icon, IntoWidget, ListView, PreferredSizeWidgetRef, StatelessWidget,
    Text, WidgetRef,
};

use crate::app::open_entry;
use crate::catalog::Entry;

/// The index's own title.
pub const INDEX_TITLE: &str = "Material Gallery";

/// The gallery's index page.
pub struct Home {
    /// Called with the mode the toggle asks for: the brightness the index is not showing.
    on_theme_mode_changed: ValueChanged<ThemeMode>,
}

impl Home {
    pub fn new(on_theme_mode_changed: ValueChanged<ThemeMode>) -> Home {
        Home {
            on_theme_mode_changed,
        }
    }
}

impl Debug for Home {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Home").finish_non_exhaustive()
    }
}

impl StatelessWidget for Home {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let tiles: Vec<WidgetRef> = Entry::ALL
            .iter()
            .map(|entry| entry_tile(*entry, context))
            .collect();

        Scaffold::new()
            .app_bar(PreferredSizeWidgetRef::new(
                AppBar::new()
                    .title(Text::new(INDEX_TITLE))
                    .actions(vec![self.theme_toggle(app, context)]),
            ))
            .drawer(index_drawer())
            .body(ListView::new().children(tiles))
            .into_widget()
    }
}

impl Home {
    /// The app-bar action that flips the app to the other brightness.
    fn theme_toggle(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let dark = Theme::of(app, context).brightness() == Brightness::Dark;
        let (icon, tooltip, next) = if dark {
            (Icons::light_mode(), "Use the light theme", ThemeMode::Light)
        } else {
            (Icons::dark_mode(), "Use the dark theme", ThemeMode::Dark)
        };
        let on_theme_mode_changed = Rc::clone(&self.on_theme_mode_changed);
        IconButton::new(
            Some(Listener::new(move |app: &mut App| {
                on_theme_mode_changed(app, next);
            })),
            Icon::new(Some(icon)),
        )
        .tooltip(tooltip)
        .into_widget()
    }
}

/// The index's tile for `entry`: tapping it pushes the entry's page.
fn entry_tile(entry: Entry, context: BuildContext) -> WidgetRef {
    ListTile::new()
        .leading(Icon::new(Some(entry.icon())))
        .title(Text::new(entry.title()))
        .on_tap(Listener::new(move |app: &mut App| {
            open_entry(app, context, entry);
        }))
        .into_widget()
}

/// The same entries in the index's drawer. A tile closes the drawer before it pushes, so the
/// page comes back to a closed drawer.
fn index_drawer() -> WidgetRef {
    Drawer::new()
        .child(Builder::new(|app: &mut App, context: BuildContext| {
            let primary_container = Theme::of(app, context).color_scheme.primary_container();
            let mut children = vec![
                DrawerHeader::new(Some(Text::new(INDEX_TITLE).into_widget()))
                    .decoration(BoxDecoration::new().color(primary_container))
                    .into_widget(),
            ];
            children.extend(Entry::ALL.iter().map(|entry| {
                let entry = *entry;
                ListTile::new()
                    .leading(Icon::new(Some(entry.icon())))
                    .title(Text::new(entry.title()))
                    .on_tap(Listener::new(move |app: &mut App| {
                        Scaffold::of(app, context).close_drawer(app);
                        open_entry(app, context, entry);
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
