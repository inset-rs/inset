//! Views the app asks for, each in a window of its own: on Android, a task of its own, a card in
//! Recents with the view's icon. Every view shows the same page, numbered: buttons that ask for
//! another view, now or in five seconds (long enough to put the app in the background), that push
//! a page, that bring the first other view forward, that minimize the view's window and that close
//! it, and a field to type in. Back goes back within a view, and at its first screen minimizes its
//! window, which on Android sends the task to the background with its card kept, as WaOS does for
//! a program.
//!
//! A view's number is its initial data, so a card Android brings back after the process died shows
//! its number again; the first view, which no request made, is numbered 0. What happens is printed,
//! which Android's log carries under `waft`.
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

use waft::{App, Handle, Listener, Timer, ValueKey};
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
    BuildContext, Builder, IntoWidget, KeyRef, ListView, MediaQuery, Navigator, Padding, PopScope,
    Route, State, StateData, StatefulWidget, Text, View, ViewCollection, WidgetRef, WidgetsBinding,
    WidgetsBindingObserverObject, WidgetsBindingObserverRef, run_widget,
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
    run_widget(app, EveryView.into_widget());
}

/// Prints each request that got no view, before the binding hears of it.
fn print_failed_requests(app: &mut App) {
    WidgetsBinding::instance(app);
    let callbacks = app.platform_callbacks_mut();
    let binding = callbacks.on_view_request_failed.take();
    callbacks.on_view_request_failed = Some(Rc::new(
        move |app: &mut App, request: RequestId, error: ViewError| {
            println!("views: request {} got no view: {error}", request.0);
            if let Some(binding) = &binding {
                binding(app, request, error);
            }
        },
    ));
}

/// Every view the app has, given or asked for, each showing its [`ViewPage`].
#[derive(Debug)]
struct EveryView;

struct EveryViewState {
    state: StateData<EveryView>,
    views: Vec<ViewRef>,
    /// The number the next view asked for takes.
    next_number: Rc<Cell<u32>>,
    observer: Option<WidgetsBindingObserverRef>,
}

impl StatefulWidget for EveryView {
    type State = EveryViewState;

    fn create_state(&self) -> EveryViewState {
        EveryViewState {
            state: StateData::new(),
            views: Vec::new(),
            next_number: Rc::new(Cell::new(1)),
            observer: None,
        }
    }
}

impl EveryViewState {
    /// Shows `view`, which arrived, and numbers the next request past its own number.
    fn show(self: Handle<Self>, app: &mut App, view: ViewRef) {
        let number = number_of(&view);
        println!(
            "views: view {} arrived, numbered {number}, for request {:?}",
            view.id().0,
            view.request_id().map(|request| request.0),
        );
        let next = &app.get(self).next_number;
        next.set(next.get().max(number + 1));
        if view.request_id().is_none()
            && let Some(window) = view.window()
        {
            window.set_title(&title_of(number));
            window.set_icon(Some(&icon_of(number)));
        }
        app.get_mut(self).views.push(view);
    }
}

impl WidgetsBindingObserverObject for EveryViewState {
    fn did_add_view(self: Handle<Self>, app: &mut App, view: &ViewRef) {
        let view = Rc::clone(view);
        self.show(app, view);
        self.set_state(app, |_| {});
    }

    fn did_remove_view(self: Handle<Self>, app: &mut App, view: ViewId) {
        println!("views: view {} went", view.0);
        self.set_state(app, |state| state.views.retain(|shown| shown.id() != view));
    }

    fn did_request_view_close(self: Handle<Self>, app: &mut App, view: ViewId) -> bool {
        if let Some(window) = app.platform().view(view).and_then(|view| view.window()) {
            window.close();
        }
        true
    }
}

impl State for EveryViewState {
    type Widget = EveryView;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let observer: WidgetsBindingObserverRef = Rc::new(self);
        WidgetsBinding::instance(app).add_observer(app, observer.clone());
        app.get_mut(self).observer = Some(observer);
        let views = app.platform().views();
        if views.is_empty() {
            let number = app.get(self).next_number.get();
            ask_for_view(app, number);
        }
        for view in views {
            self.show(app, view);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(observer) = app.get_mut(self).observer.take() {
            WidgetsBinding::instance(app).remove_observer(app, &observer);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let next_number = Rc::clone(&app.get(self).next_number);
        let views = app.get(self).views.clone();
        let pages = views
            .into_iter()
            .map(|view| {
                let key: KeyRef = Rc::new(ValueKey::new(view.id().0));
                let page = ViewPage {
                    view: Rc::clone(&view),
                    next_number: Rc::clone(&next_number),
                };
                View::new(view, page).key(key).into_widget()
            })
            .collect();
        ViewCollection::new(pages).into_widget()
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

/// Asks for the view numbered `number`, in a window titled and iconed as it.
fn ask_for_view(app: &mut App, number: u32) {
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
        Ok(request) => println!("views: asked for view {number}, request {}", request.0),
        Err(error) => println!("views: view {number} was refused: {error}"),
    }
}

/// What one view shows: an app of its own, whose first screen is [`home`].
struct ViewPage {
    view: ViewRef,
    next_number: Rc<Cell<u32>>,
}

impl fmt::Debug for ViewPage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewPage")
            .field("view", &self.view.id())
            .finish_non_exhaustive()
    }
}

impl waft_widgets::StatelessWidget for ViewPage {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let number = number_of(&self.view);
        let home = PopScope::new(home(&self.view, &self.next_number))
            .can_pop(false)
            .on_pop_invoked_with_result(minimizer(&self.view));
        CupertinoApp::new()
            .title("Views")
            .home(screen(&title_of(number), home))
            .into_widget()
    }
}

/// What Back at the first screen does: minimizes the view's window.
fn minimizer(view: &ViewRef) -> waft_widgets::PopInvokedWithResultCallback {
    let view = Rc::clone(view);
    Rc::new(move |_app, did_pop, _result| {
        if did_pop {
            return;
        }
        println!("views: Back at view {}'s first screen", view.id().0);
        if let Some(window) = view.window() {
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

/// The first screen of `view`'s page.
fn home(view: &ViewRef, next_number: &Rc<Cell<u32>>) -> WidgetRef {
    let number = number_of(view);
    let ask = {
        let next_number = Rc::clone(next_number);
        move |app: &mut App| {
            let number = next_number.get();
            next_number.set(number + 1);
            ask_for_view(app, number);
        }
    };
    let ask_later = {
        let ask = ask.clone();
        Listener::new(move |app| {
            println!("views: asking for a view in 5 seconds");
            let ask = ask.clone();
            Timer::new(app, Duration::from_secs(5), Listener::new(ask));
        })
    };
    let window_of = {
        let view = Rc::clone(view);
        move || view.window()
    };
    let minimize = {
        let window_of = window_of.clone();
        Listener::new(move |_app| {
            if let Some(window) = window_of() {
                window.set_minimized(true);
            }
        })
    };
    let close = Listener::new(move |_app| {
        if let Some(window) = window_of() {
            window.close();
        }
    });
    let bring_forward = {
        let view = view.id();
        Listener::new(move |app| bring_another_forward(app, view))
    };
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
