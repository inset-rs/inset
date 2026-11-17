//! The catalog index, and the page it opens.
//!
//! The index is a `Scaffold` whose `AppBar` carries the theme toggle, whose `Drawer` lists the
//! entries again, and whose body is a `ListView` of `ListTile`s — the same widgets the Lists,
//! Drawers and App Bars entries demonstrate. With the page it opens it is a list-detail layout:
//! on a wide window the page sits beside the index, and on a phone it is pushed over it.

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::Brightness;
use waft_foundation::{App, Handle, Listener, ValueChanged};
use waft_material::{
    AppBar, Drawer, DrawerHeader, IconButton, Icons, ListDetailPaneScaffoldRole, ListTile,
    PaneScaffoldDirective, Scaffold, Theme, ThemeMode, ThreePaneNavigator, ThreePaneScaffold,
    ThreePaneScaffoldDestinationItem, ThreePaneSceneStrategy, WindowSizeClass,
};
use waft_painting::{BoxDecoration, EdgeInsetsGeometry};
use waft_widgets::{
    BuildContext, Builder, ColoredBox, Icon, IntoWidget, ListView, PreferredSizeWidgetRef, State,
    StateData, StatefulWidget, Text, WidgetRef,
};

use crate::app::open_entry;
use crate::catalog::Entry;
use crate::panes::{pane, side_by_side, window_color};
use crate::support::empty_state;

/// The index's own title.
pub const INDEX_TITLE: &str = "Material Gallery";

/// The gallery's index, and the page open beside it or over it.
pub struct Home {
    /// Called with the mode the toggle asks for: the brightness the index is not showing.
    on_theme_mode_changed: ValueChanged<ThemeMode>,
    /// The page open when the gallery starts.
    opening: Option<Entry>,
}

impl Home {
    pub fn new(on_theme_mode_changed: ValueChanged<ThemeMode>) -> Home {
        Home {
            on_theme_mode_changed,
            opening: None,
        }
    }

    /// Opens `entry`'s page from the start.
    pub fn opening(mut self, opening: Option<Entry>) -> Home {
        self.opening = opening;
        self
    }
}

impl Debug for Home {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Home").finish_non_exhaustive()
    }
}

/// One entry of the gallery's back stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GalleryEntry {
    Index,
    Page(Entry),
}

impl GalleryEntry {
    /// The pane this entry fills.
    fn pane(&self) -> Option<ThreePaneScaffoldDestinationItem<Entry>> {
        Some(match *self {
            GalleryEntry::Index => {
                ThreePaneScaffoldDestinationItem::new(ListDetailPaneScaffoldRole::LIST, None)
            }
            GalleryEntry::Page(entry) => ThreePaneScaffoldDestinationItem::new(
                ListDetailPaneScaffoldRole::DETAIL,
                Some(entry),
            ),
        })
    }
}

impl StatefulWidget for Home {
    type State = HomeState;

    fn create_state(&self) -> HomeState {
        let mut stack = vec![GalleryEntry::Index];
        stack.extend(self.opening.map(GalleryEntry::Page));
        HomeState {
            state: StateData::new(),
            stack,
        }
    }
}

/// Dart would spell this `_HomeState`.
pub struct HomeState {
    state: StateData<Home>,
    /// The index, then the open page.
    stack: Vec<GalleryEntry>,
}

impl State for HomeState {
    type Widget = Home;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let directive =
            PaneScaffoldDirective::from_window_size_class(WindowSizeClass::of(app, context));
        let background = window_color(&Theme::of(app, context).color_scheme);
        let wide = side_by_side(&directive);
        let navigator = ThreePaneNavigator::new(
            ThreePaneSceneStrategy::list_detail(directive),
            app.get(self).stack.clone(),
            GalleryEntry::pane,
            move |app, context, index| match app.get(self).stack[index] {
                GalleryEntry::Index => self.index(app, context),
                GalleryEntry::Page(entry) => entry.body(app, context),
            },
            move |app, context, scene| {
                let directive = &scene.directive;
                let index = self.index(app, context);
                let page = match self.open_page(app) {
                    Some(entry) => entry.body(app, context),
                    None => empty_state(
                        app,
                        context,
                        Icons::widgets(),
                        "No page open",
                        "Pick a family of widgets from the index.",
                    ),
                };
                ThreePaneScaffold::list_detail(
                    directive.clone(),
                    scene.value,
                    pane(app, context, directive, index),
                    pane(app, context, directive, page),
                )
                .into_widget()
            },
            move |app, len| self.set_state(app, |state| state.stack.truncate(len)),
        );
        if wide {
            ColoredBox::new(background).child(navigator).into_widget()
        } else {
            navigator.into_widget()
        }
    }
}

impl HomeState {
    /// The page open over or beside the index.
    fn open_page(self: Handle<Self>, app: &App) -> Option<Entry> {
        app.get(self)
            .stack
            .iter()
            .rev()
            .find_map(|entry| match entry {
                GalleryEntry::Page(entry) => Some(*entry),
                GalleryEntry::Index => None,
            })
    }

    /// Opens `entry`: a page of its own over the whole gallery when it fills the window, and
    /// the page beside or over the index otherwise.
    fn open(self: Handle<Self>, app: &mut App, context: BuildContext, entry: Entry) {
        if entry.fills_window() {
            open_entry(app, context, entry);
            return;
        }
        self.set_state(app, |state| {
            state.stack = vec![GalleryEntry::Index, GalleryEntry::Page(entry)];
        });
    }

    /// The index page: the toggle, the drawer and the tiles.
    fn index(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let selected = self.open_page(app);
        let tiles: Vec<WidgetRef> = Entry::ALL
            .iter()
            .map(|entry| self.entry_tile(*entry, selected == Some(*entry), context))
            .collect();
        Scaffold::new()
            .app_bar(PreferredSizeWidgetRef::new(
                AppBar::new()
                    .title(Text::new(INDEX_TITLE))
                    .actions(vec![self.theme_toggle(app, context)]),
            ))
            .drawer(self.index_drawer())
            .body(ListView::new().children(tiles))
            .into_widget()
    }

    /// The app-bar action that flips the app to the other brightness.
    fn theme_toggle(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let dark = Theme::of(app, context).brightness() == Brightness::Dark;
        let (icon, tooltip, next) = if dark {
            (Icons::light_mode(), "Use the light theme", ThemeMode::Light)
        } else {
            (Icons::dark_mode(), "Use the dark theme", ThemeMode::Dark)
        };
        let on_theme_mode_changed = Rc::clone(&self.widget(app).on_theme_mode_changed);
        IconButton::new(
            Some(Listener::new(move |app: &mut App| {
                on_theme_mode_changed(app, next);
            })),
            Icon::new(Some(icon)),
        )
        .tooltip(tooltip)
        .into_widget()
    }

    /// The index's tile for `entry`: tapping it opens the entry's page.
    fn entry_tile(
        self: Handle<Self>,
        entry: Entry,
        selected: bool,
        context: BuildContext,
    ) -> WidgetRef {
        ListTile::new()
            .leading(Icon::new(Some(entry.icon())))
            .title(Text::new(entry.title()))
            .selected(selected)
            .on_tap(Listener::new(move |app: &mut App| {
                self.open(app, context, entry);
            }))
            .into_widget()
    }

    /// The same entries in the index's drawer. A tile closes the drawer before it opens the
    /// page, so the page comes back to a closed drawer.
    fn index_drawer(self: Handle<Self>) -> WidgetRef {
        Drawer::new()
            .child(Builder::new(move |app: &mut App, context: BuildContext| {
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
                            self.open(app, context, entry);
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
}
