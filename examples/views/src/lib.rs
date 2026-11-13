//! Views the app asks for, each in a window of its own: on Android, a task of its own, a card in
//! Recents with the view's icon. Every view shows the same page, numbered: buttons that ask for
//! another view, now or in five seconds (long enough to put the app in the background), that push
//! a page, that bring the first other view forward, that minimize the view's window and that close
//! it, and a field to type in. Back goes back within a view, and at its first screen minimizes its
//! window, which on Android sends the task to the background with its card kept, as WaOS does for
//! a program.
//!
//! A view's number is its initial data, which says what each view shows, the first, one asked for
//! and one brought back alike, so a card Android brings back after the process died shows its
//! number again; the first view, where the platform gives one, is numbered 0, and a window asked
//! for at the start, where it gives none, 1. The window is reached from inside the page. What
//! happens is printed, which Android's log carries under `waft`.
//!
//! ```text
//! cargo run -p views
//! cargo waft run -d emulator-5554 -p views
//! ```
#![feature(arbitrary_self_types)]

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use waft::{App, Handle, Listener, Timer};
use waft_cupertino::{
    CupertinoApp, CupertinoButton, CupertinoNavigationBar, CupertinoPageRoute,
    CupertinoPageScaffold, CupertinoTextField, ObstructingPreferredSizeWidgetRef,
};
use waft_embedder::{
    IconImage, RequestId, ViewError, ViewFocusDirection, ViewFocusState, ViewId, ViewRef,
    WindowConfig,
};
use waft_painting::EdgeInsetsGeometry;
use waft_widgets::{
    BuildContext, Builder, IntoWidget, ListView, MediaQuery, MultiViewApp, Navigator, Padding,
    PopScope, Route, State, StateData, StatefulWidget, Text, View, WidgetRef, WidgetsBinding,
    WidgetsBindingObserver, WidgetsBindingObserverRef, WindowScope, run_widget,
};

/// The side of each view's icon, in pixels.
const ICON_SIDE: u32 = 96;
/// The colours the views' icons take, by number.
const ICON_COLORS: [[u8; 3]; 6] = [
    [0x3b, 0x82, 0xf6],
    [0xef, 0x44, 0x44],
    [0x22, 0xc5, 0x5e],
    [0xf5, 0x9e, 0x0b],
    [0xa8, 0x55, 0xf7],
    [0x14, 0xb8, 0xa6],
];

#[waft::main]
fn main(app: &mut App) {
    print_failed_requests(app);
    let next_number = Rc::new(Cell::new(1));
    if app.platform().views().is_empty() {
        ask_for_view(app, &next_number);
    }
    let pages = MultiViewApp::new(move |_app, view| {
        let number = number_of(view);
        next_number.set(next_number.get().max(number + 1));
        Some(
            ViewPage {
                number,
                next_number: Rc::clone(&next_number),
            }
            .into_widget(),
        )
    });
    run_widget(app, pages.into_widget());
}

/// Prints each request that got no view.
fn print_failed_requests(app: &mut App) {
    let observer: WidgetsBindingObserverRef = Rc::new(FailedRequests);
    WidgetsBinding::instance(app).add_observer(app, observer);
}

/// Prints why a request got no view.
struct FailedRequests;

impl WidgetsBindingObserver for FailedRequests {
    fn did_fail_view_request(&self, _app: &mut App, _request: RequestId, error: &ViewError) {
        println!("views: a view asked for got none: {error}");
    }
}

/// The number a view carries as its initial data; 0 for one with none.
fn number_of(view: &ViewRef) -> u32 {
    view.initial_data()
        .and_then(|data| data.parse().ok())
        .unwrap_or(0)
}

fn title_of(number: u32) -> String {
    format!("View {number}")
}

/// A square of the view's colour.
fn icon_of(number: u32) -> IconImage {
    let [red, green, blue] = ICON_COLORS[number as usize % ICON_COLORS.len()];
    let pixels = (ICON_SIDE * ICON_SIDE) as usize;
    let rgba: Vec<u8> = [red, green, blue, 0xff].repeat(pixels);
    IconImage {
        width: ICON_SIDE,
        height: ICON_SIDE,
        rgba: Arc::from(rgba),
        template: false,
    }
}

/// Asks for the next view, in a window titled and iconed as its number.
fn ask_for_view(app: &mut App, next_number: &Cell<u32>) {
    let number = next_number.get();
    next_number.set(number + 1);
    let config = WindowConfig {
        title: title_of(number),
        icon: Some(icon_of(number)),
        size: [420.0, 640.0],
        ..WindowConfig::default()
    };
    match app
        .platform()
        .request_view(config, Some(number.to_string()))
    {
        Ok(_) => println!("views: asked for view {number}"),
        Err(error) => println!("views: view {number} was refused: {error}"),
    }
}

/// What one view shows: an app of its own, whose first screen is [`home`]. Its window, a card in
/// Recents on Android, is titled and iconed as its number, which a window Android brings back
/// after the process died has not been.
struct ViewPage {
    number: u32,
    next_number: Rc<Cell<u32>>,
}

impl fmt::Debug for ViewPage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewPage")
            .field("number", &self.number)
            .finish_non_exhaustive()
    }
}

struct ViewPageState {
    state: StateData<ViewPage>,
    /// Whether its window was titled and iconed.
    described: bool,
}

impl StatefulWidget for ViewPage {
    type State = ViewPageState;

    fn create_state(&self) -> ViewPageState {
        ViewPageState {
            state: StateData::new(),
            described: false,
        }
    }
}

impl State for ViewPageState {
    type Widget = ViewPage;
    waft_widgets::state_accessors!();

    /// Titles and icons its window, once.
    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        if std::mem::replace(&mut app.get_mut(self).described, true) {
            return;
        }
        let number = self.widget(app).number;
        let view = View::of(app, self.context(app));
        println!("views: view {} arrived, numbered {number}", view.id().0);
        if let Some(window) = WindowScope::maybe_of(app, self.context(app)) {
            window.set_title(&title_of(number));
            window.set_icon(Some(&icon_of(number)));
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        let number = widget.number;
        let next_number = Rc::clone(&widget.next_number);
        let home = Builder::new(move |_app, context| {
            PopScope::new(home(context, number, &next_number))
                .can_pop(false)
                .on_pop_invoked_with_result(minimizer(context))
                .into_widget()
        });
        CupertinoApp::new()
            .title("Views")
            .home(screen(&title_of(number), home))
            .into_widget()
    }
}

/// What Back at the first screen does: minimizes the window of the view of `context`.
fn minimizer(context: BuildContext) -> waft_widgets::PopInvokedWithResultCallback {
    Rc::new(move |app, did_pop, _result| {
        if did_pop {
            return;
        }
        println!("views: Back at a view's first screen");
        if let Some(window) = WindowScope::maybe_of(app, context) {
            window.set_minimized(true);
        }
    })
}

/// A page under a navigation bar titled `title`.
fn screen<K>(title: &str, body: impl IntoWidget<K>) -> WidgetRef {
    CupertinoPageScaffold::new(body)
        .navigation_bar(ObstructingPreferredSizeWidgetRef::new(
            CupertinoNavigationBar::new().middle(Text::new(title.to_owned())),
        ))
        .into_widget()
}

/// The first screen of view `number`'s page, built in `context`.
fn home(context: BuildContext, number: u32, next_number: &Rc<Cell<u32>>) -> WidgetRef {
    let ask = {
        let next_number = Rc::clone(next_number);
        move |app: &mut App| ask_for_view(app, &next_number)
    };
    let ask_later = {
        let ask = ask.clone();
        Listener::new(move |app| {
            println!("views: asking for a view in 5 seconds");
            let ask = ask.clone();
            Timer::new(app, Duration::from_secs(5), Listener::new(ask));
        })
    };
    let minimize = Listener::new(move |app| {
        if let Some(window) = WindowScope::maybe_of(app, context) {
            window.set_minimized(true);
        }
    });
    let close = Listener::new(move |app| {
        if let Some(window) = WindowScope::maybe_of(app, context) {
            window.close();
        }
    });
    let bring_forward = Listener::new(move |app| {
        let this = View::of(app, context).id();
        bring_another_forward(app, this);
    });
    let push = Builder::new(move |_app, context| {
        button(
            "Push a page",
            Listener::new(move |app| push_page(app, context, number)),
        )
    });
    ListBody(vec![
        button("Open a view", Listener::new(ask)),
        button("Open a view in 5 seconds", ask_later),
        push.into_widget(),
        button("Bring another view forward", bring_forward),
        button("Minimize", minimize),
        button("Close", close),
        field(number),
    ])
    .into_widget()
}

/// Brings forward the first view of the app's that is not `this`.
fn bring_another_forward(app: &mut App, this: ViewId) {
    let other = app
        .platform()
        .views()
        .into_iter()
        .find(|view| view.id() != this);
    let Some(other) = other else {
        println!("views: no other view to bring forward");
        return;
    };
    println!("views: bringing view {} forward", other.id().0);
    other
        .host()
        .request_focus_change(ViewFocusState::Focused, ViewFocusDirection::Undefined);
}

/// Pushes a second screen, with a field of its own, over view `number`'s first, from `context`.
fn push_page(app: &mut App, context: BuildContext, number: u32) {
    let builder: waft_widgets::WidgetBuilder = Rc::new(move |_app, _context| {
        screen(
            &format!("{} — page 2", title_of(number)),
            ListBody(vec![field(number)]),
        )
    });
    let route = CupertinoPageRoute::new(app, builder);
    Navigator::push(app, context, route.as_route());
}

fn button(label: &str, on_pressed: Listener) -> WidgetRef {
    CupertinoButton::filled(Text::new(label.to_owned()).into_widget(), Some(on_pressed))
        .into_widget()
}

fn field(number: u32) -> WidgetRef {
    CupertinoTextField::new()
        .placeholder(format!("Type in view {number}"))
        .into_widget()
}

/// Rows under the bar and above the bottom inset, scrolling where they do not fit.
#[derive(Debug)]
struct ListBody(Vec<WidgetRef>);

impl waft_widgets::StatelessWidget for ListBody {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let padding = MediaQuery::padding_of(app, context);
        let rows = self.0.iter().map(|row| {
            Padding::new(EdgeInsetsGeometry::symmetric(6.0, 16.0))
                .child(row.clone())
                .into_widget()
        });
        ListView::new()
            .padding(EdgeInsetsGeometry::only(
                0.0,
                padding.top + 12.0,
                0.0,
                padding.bottom,
            ))
            .children(rows)
            .into_widget()
    }
}
