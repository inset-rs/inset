//! The gallery's shell, and the routes its pages ride on.
//!
//! The shell is a [`MaterialApp`]: it supplies the navigator every page is pushed onto, the
//! `ScaffoldMessenger` the snack bars and banners are shown through, the Material
//! localizations the back button's tooltip comes from, and the light and dark themes. Nothing
//! here keeps a page stack of its own — the navigator is the stack.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_embedder::{Brightness, Color, WindowConfig};
use waft_foundation::{App, Handle, Listener, Timer, ValueChanged};
use waft_material::{
    Colors, MaterialApp, MaterialPageRoute, ThemeData, ThemeMode, install_material_icon_font,
};
use waft_widgets::{
    BuildContext, GlobalKey, IntoWidget, Navigator, NavigatorState, RouteRef, State, StateData,
    StatefulWidget, WidgetBuilder, WidgetRef, run_app_with,
};

use crate::catalog::Entry;
use crate::pages::home::Home;

/// The colour both themes' schemes are generated from.
fn seed_color() -> Color {
    Colors::DEEP_PURPLE.primary()
}

/// The whole gallery: the index under a `MaterialApp` with Material 3 light and dark themes.
///
/// It is stateful for one value, the theme mode: it starts by following the platform, and the
/// index's toggle pins it to the brightness the index is not showing.
#[derive(Default)]
pub struct Gallery {
    navigator_key: Option<GlobalKey>,
}

impl Gallery {
    pub fn new() -> Gallery {
        Gallery::default()
    }

    /// The key the app's navigator is reached through from outside the tree.
    pub fn navigator_key(mut self, navigator_key: GlobalKey) -> Gallery {
        self.navigator_key = Some(navigator_key);
        self
    }
}

impl Debug for Gallery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Gallery").finish_non_exhaustive()
    }
}

impl StatefulWidget for Gallery {
    type State = GalleryState;

    fn create_state(&self) -> GalleryState {
        GalleryState {
            state: StateData::new(),
            theme_mode: ThemeMode::System,
        }
    }
}

/// Dart would spell this `_GalleryState`.
pub struct GalleryState {
    state: StateData<Gallery>,
    theme_mode: ThemeMode,
}

impl State for GalleryState {
    type Widget = Gallery;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let on_theme_mode_changed: ValueChanged<ThemeMode> =
            Rc::new(move |app: &mut App, theme_mode: ThemeMode| {
                self.set_state(app, |state| state.theme_mode = theme_mode);
            });
        let theme_mode = app.get(self).theme_mode;
        let navigator_key = self.widget(app).navigator_key.clone();

        let gallery = MaterialApp::new()
            .title("Material Gallery")
            .theme(theme(app, Brightness::Light))
            .dark_theme(theme(app, Brightness::Dark))
            .theme_mode(theme_mode)
            .home(Home::new(on_theme_mode_changed));
        match navigator_key {
            Some(navigator_key) => gallery.navigator_key(navigator_key),
            None => gallery,
        }
        .into_widget()
    }
}

/// Dart's `ThemeData(colorSchemeSeed: seed, brightness: brightness)`: Material 3, the default.
fn theme(app: &App, brightness: Brightness) -> ThemeData {
    ThemeData::builder()
        .color_scheme_seed(seed_color())
        .brightness(brightness)
        .build(app)
}

/// Runs the gallery in a window of `size`, a phone's when `None`, then opens `opening` over the
/// index once the tree is attached: the page a user would tap to, reached without input, for a
/// bench, a smoke run or a screenshot.
pub fn run_gallery(app: &mut App, opening: Option<Entry>, size: Option<[f64; 2]>) {
    let navigator = GlobalKey::new();
    let window = WindowConfig {
        title: "Waft — material gallery".to_owned(),
        size: size.unwrap_or([420.0, 720.0]),
        ..WindowConfig::default()
    };
    run_app_with(
        app,
        Gallery::new()
            .navigator_key(navigator.clone())
            .into_widget(),
        window,
    );
    // AFTER `run_app`, which is what installs the default font collection this registers
    // into. Without the icon face every glyph in the gallery — the back arrow, the drawer
    // button, every tile's leading icon — is a missing glyph and draws nothing at all,
    // silently. There is no asset manifest here, so registration is explicit.
    install_material_icon_font(app);
    let Some(entry) = opening else {
        return;
    };
    // `run_app` attaches the tree on the next timer turn; this timer runs right after it.
    Timer::new(
        app,
        Duration::ZERO,
        Listener::new(move |app| {
            let state = navigator
                .current_state::<NavigatorState>(app)
                .expect("the gallery's navigator is mounted once run_app has attached the tree");
            let route = entry_route(app, entry);
            state.push(app, route);
        }),
    );
}

/// The route one catalog entry's page is pushed as.
pub fn entry_route(app: &mut App, entry: Entry) -> RouteRef {
    let builder: WidgetBuilder =
        Rc::new(move |app: &mut App, context: BuildContext| entry.body(app, context));
    MaterialPageRoute::new(app, builder).into_erased()
}

/// The route one of an entry's own sub-pages is pushed as.
pub fn sub_route(app: &mut App, entry: Entry, index: usize) -> RouteRef {
    let builder: WidgetBuilder =
        Rc::new(move |app: &mut App, context: BuildContext| entry.sub_body(index, app, context));
    MaterialPageRoute::new(app, builder).into_erased()
}

/// Opens an entry's page, from a tile on the index or in its drawer.
pub fn open_entry(app: &mut App, context: BuildContext, entry: Entry) {
    let route = entry_route(app, entry);
    Navigator::push(app, context, route);
}

/// Opens one of an entry's sub-pages, from a tile on that entry's own page.
pub fn open_sub_page(app: &mut App, context: BuildContext, entry: Entry, index: usize) {
    let route = sub_route(app, entry, index);
    Navigator::push(app, context, route);
}
