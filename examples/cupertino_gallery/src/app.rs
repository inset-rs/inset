//! The gallery's shell, and the routes its screens ride on.
//!
//! The shell is a [`CupertinoApp`]: it supplies the navigator every screen is pushed onto, the
//! `HeroController` the bar and swatch flights run through, and the Cupertino localizations the
//! back button's label comes from. Nothing here keeps a page stack of its own — the navigator
//! is the stack.

use std::rc::Rc;
use std::time::Duration;

use inset_cupertino::{CupertinoApp, CupertinoPageRoute, install_cupertino_icon_font};
use inset_foundation::{App, Handle, Listener, Timer};
use inset_widgets::{
    AnyRoute, BuildContext, GlobalKey, IntoWidget, Navigator, NavigatorState, Route,
    RouteSettingsRef, WidgetBuilder, run_app,
};

use crate::catalog::Entry;
use crate::pages;

/// The index's own title: its large navigation-bar title, and the back label of every screen
/// pushed from it.
pub const INDEX_TITLE: &str = "Gallery";

/// The whole gallery: the catalog index under the Cupertino chrome.
///
/// The index arrives through `on_generate_route` rather than `home` for one reason: a route
/// built here can carry a title, and the title of the route below is what the next screen's
/// back button reads. Through `home` the app builds the route itself, untitled, and the first
/// push would land on a bare "Back".
pub fn gallery() -> CupertinoApp {
    CupertinoApp::new()
        .title("Cupertino Gallery")
        .on_generate_route(|app, settings| {
            (settings.name.as_deref() == Some(Navigator::DEFAULT_ROUTE_NAME)).then(|| {
                let builder: WidgetBuilder = Rc::new(|_app: &mut App, _context: BuildContext| {
                    pages::home::Home.into_widget()
                });
                page_route(app, builder, INDEX_TITLE)
                    .settings(app, RouteSettingsRef::Settings(settings.clone()))
                    .as_route()
            })
        })
}

/// Runs the gallery, then opens `opening` over the index once the tree is attached: the
/// screen a user would tap to, reached without input, for a bench or a smoke run.
pub fn run_gallery(app: &mut App, opening: Option<Entry>) {
    let navigator = GlobalKey::new();
    run_app(
        app,
        gallery().navigator_key(navigator.clone()).into_widget(),
    );
    // AFTER `run_app`, which is what installs the default font collection this registers
    // into. Without the icon face every glyph in the gallery — the back chevron, the row
    // badges, the whole Icons entry — is a missing glyph and draws nothing at all, silently.
    // There is no asset manifest here, so registration is explicit.
    install_cupertino_icon_font(app);
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

/// The route one catalog entry's screen is pushed as.
pub fn entry_route(app: &mut App, entry: Entry) -> AnyRoute {
    let builder: WidgetBuilder =
        Rc::new(move |app: &mut App, context: BuildContext| entry.body(app, context));
    page_route(app, builder, entry.title()).as_route()
}

/// The route one of an entry's own sub-screens is pushed as.
pub fn sub_route(app: &mut App, entry: Entry, index: usize) -> AnyRoute {
    let builder: WidgetBuilder =
        Rc::new(move |app: &mut App, context: BuildContext| entry.sub_body(index, app, context));
    page_route(app, builder, entry.sub_title(index)).as_route()
}

/// A screen's route.
///
/// The title is what the bar shows in the middle and what the NEXT screen's back button reads,
/// so it is set here rather than on each screen's bar.
fn page_route(app: &mut App, builder: WidgetBuilder, title: &str) -> Handle<CupertinoPageRoute> {
    CupertinoPageRoute::new(app, builder).title(app, title.to_owned())
}

/// Opens an entry's screen, from a row on the index.
pub fn open_entry(app: &mut App, context: BuildContext, entry: Entry) {
    let route = entry_route(app, entry);
    Navigator::push(app, context, route);
}

/// Opens one of an entry's sub-screens, from a row on that entry's own screen.
pub fn open_sub_page(app: &mut App, context: BuildContext, entry: Entry, index: usize) {
    let route = sub_route(app, entry, index);
    Navigator::push(app, context, route);
}
