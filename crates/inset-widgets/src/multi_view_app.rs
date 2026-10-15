//! Flutter counterpart: the `MultiViewApp` of Flutter web's embedding guide (not a framework
//! class there), which builds a `View` for every view the page adds.
//!
//! A [`MultiViewApp`] builds every view the platform gives the app: a phone's screen, a web
//! page's elements, a window WaOS gives a program, a host app's views. It takes the views there
//! at start, then hears each view the platform adds or removes and each request to close one
//! of them from the binding, after the windowing owner has claimed the views its window
//! controllers asked for. Flutter's reads `platformDispatcher.views` again at each
//! `didChangeMetrics` instead.
//!
//! The invariant a later editor must keep: it builds only views that carry no request id, and
//! each once, keyed by its id, so a view keeps its subtree's state while others come and go.

use std::fmt;
use std::rc::Rc;

use inset_embedder::{ViewId, ViewRef};
use inset_foundation::{App, Handle, ValueKey};

use crate::binding::{
    WidgetsBinding, WidgetsBindingObserverObject, WidgetsBindingObserverRef, close_or_ask_to_quit,
};
use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::state_accessors;
use crate::view::{View, ViewCollection};
use crate::widgets::basic::Builder;
use crate::windowing::ViewCloseHandler;

/// What a [`MultiViewApp`] builds in one of its views.
pub type ViewBuilder = Rc<dyn Fn(&mut App, &ViewRef) -> WidgetRef>;

/// Builds every view the platform gives the app.
pub struct MultiViewApp {
    pub key: Option<KeyRef>,
    builder: ViewBuilder,
    on_close_requested: Option<ViewCloseHandler>,
}

impl MultiViewApp {
    /// An app that builds `builder`'s widget in each view it is given; a view's
    /// [`initial_data`](inset_embedder::View::initial_data) tells views apart.
    pub fn new(builder: impl Fn(&mut App, &ViewRef) -> WidgetRef + 'static) -> MultiViewApp {
        MultiViewApp {
            key: None,
            builder: Rc::new(builder),
            on_close_requested: None,
        }
    }

    /// What a request to close one of its views does. By default the app's last view asks to
    /// quit, as Cmd-Q does, and any other closes.
    pub fn on_close_requested(mut self, handler: ViewCloseHandler) -> MultiViewApp {
        self.on_close_requested = Some(handler);
        self
    }
}

impl fmt::Debug for MultiViewApp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MultiViewApp").finish_non_exhaustive()
    }
}

impl StatefulWidget for MultiViewApp {
    type State = MultiViewAppState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> MultiViewAppState {
        MultiViewAppState {
            state: StateData::new(),
            views: Vec::new(),
            observer: None,
        }
    }
}

/// The state of a [`MultiViewApp`]: the views it builds, in the order they came.
pub struct MultiViewAppState {
    state: StateData<MultiViewApp>,
    views: Vec<ViewRef>,
    /// Retains the observer identity for removal on disposal.
    observer: Option<WidgetsBindingObserverRef>,
}

impl MultiViewAppState {
    fn shows(self: Handle<Self>, app: &App, view: ViewId) -> bool {
        app.get(self).views.iter().any(|shown| shown.id() == view)
    }
}

/// Whether the platform gave `view`, rather than the app asking for it.
fn was_given(view: &ViewRef) -> bool {
    view.request_id().is_none()
}

impl WidgetsBindingObserverObject for MultiViewAppState {
    fn did_add_view(self: Handle<Self>, app: &mut App, view: &ViewRef) {
        if was_given(view) && !self.shows(app, view.id()) {
            let view = Rc::clone(view);
            self.set_state(app, |state| state.views.push(view));
        }
    }

    fn did_remove_view(self: Handle<Self>, app: &mut App, view: ViewId) {
        if self.shows(app, view) {
            self.set_state(app, |state| state.views.retain(|shown| shown.id() != view));
        }
    }

    fn did_request_view_close(self: Handle<Self>, app: &mut App, view: ViewId) -> bool {
        if !self.shows(app, view) {
            return false;
        }
        let handler = self.widget(app).on_close_requested.clone();
        match handler {
            Some(handler) => handler(app, view),
            None => close_or_ask_to_quit(app, view),
        }
        true
    }
}

impl State for MultiViewAppState {
    type Widget = MultiViewApp;
    state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let observer: WidgetsBindingObserverRef = Rc::new(self);
        WidgetsBinding::instance(app).add_observer(app, observer.clone());
        app.get_mut(self).observer = Some(observer);
        let views = app
            .platform()
            .views()
            .into_iter()
            .filter(was_given)
            .collect();
        app.get_mut(self).views = views;
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(observer) = app.get_mut(self).observer.take() {
            WidgetsBinding::instance(app).remove_observer(app, &observer);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let builder = Rc::clone(&self.widget(app).builder);
        let views = app.get(self).views.clone();
        let view_widgets = views
            .into_iter()
            .map(|view| view_widget(view, Rc::clone(&builder)))
            .collect();
        ViewCollection::new(view_widgets).into_widget()
    }
}

/// `builder`'s widget in `view`, keyed by the view's id.
fn view_widget(view: ViewRef, builder: ViewBuilder) -> WidgetRef {
    let key: KeyRef = Rc::new(ValueKey::new(view.id().0));
    let content = {
        let view = Rc::clone(&view);
        Builder::new(move |app, _context| builder(app, &view))
    };
    View::new(view, content).key(key).into_widget()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    use inset_embedder::{View as _, ViewId, WindowConfig};
    use inset_foundation::AppCell;
    use inset_test::{TestPlatform, TestView, TestWindow, TestWindows};

    use super::*;
    use crate::test_harness::{
        Quits, report_view_added, report_view_removed, report_windows_made, request_view_close,
        views_cell,
    };
    use crate::{AppLifecycleListener, SizedBox, Window, WindowController, run_app, run_app_with};

    struct Platform {
        cell: Rc<AppCell>,
        windows: Rc<TestWindows>,
        quits: Rc<Quits>,
    }

    fn platform(views: &[Rc<TestView>]) -> Platform {
        let windows = Rc::new(TestWindows::new());
        let quits = Rc::new(Quits::default());
        let cell = views_cell(views, &windows, &quits);
        Platform {
            cell,
            windows,
            quits,
        }
    }

    fn view(id: u64) -> Rc<TestView> {
        Rc::new(TestView::new(300.0, 200.0).with_id(ViewId(id)))
    }

    /// Mounts `root` and runs the frame that shows it.
    fn mount(cell: &AppCell, root: WidgetRef) {
        crate::binding::run_widget(&mut cell.borrow_mut(), root);
        cell.elapse(Duration::ZERO);
        crate::test_harness::binding_pump(&mut cell.borrow_mut(), Duration::ZERO);
    }

    fn run_app_titled(cell: &AppCell, title: &str) {
        let config = WindowConfig {
            title: title.to_owned(),
            ..WindowConfig::default()
        };
        run_app_with(
            &mut cell.borrow_mut(),
            SizedBox::new().into_widget(),
            config,
        );
        cell.elapse(Duration::ZERO);
    }

    #[test]
    fn every_view_the_app_is_given_shows_the_content() {
        let (first, second) = (view(1), view(2));
        let platform = platform(&[Rc::clone(&first), Rc::clone(&second)]);
        run_app_titled(&platform.cell, "unused where views are given");
        assert_eq!(first.presented(), 1);
        assert_eq!(second.presented(), 1);
        assert!(platform.windows.made().is_empty(), "no window asked for");
    }

    #[test]
    fn given_no_view_the_app_asks_for_its_window_and_shows_the_content_in_it() {
        let platform = platform(&[]);
        run_app_titled(&platform.cell, "Notes");
        let window = Rc::clone(&platform.windows.made()[0]);
        assert_eq!(window.config().title, "Notes");

        report_windows_made(&platform.cell, &platform.windows, Duration::from_millis(16));
        assert_eq!(window.test_view().presented(), 1);
    }

    #[test]
    fn a_view_is_built_from_what_it_was_given_with() {
        let inspector = Rc::new(
            TestView::new(300.0, 200.0)
                .with_id(ViewId(2))
                .with_initial_data("inspector"),
        );
        let platform = platform(&[view(1), inspector]);
        let built: Rc<RefCell<Vec<Option<String>>>> = Rc::default();
        let seen = Rc::clone(&built);
        mount(
            &platform.cell,
            MultiViewApp::new(move |_app, view| {
                seen.borrow_mut().push(view.initial_data());
                SizedBox::new().into_widget()
            })
            .into_widget(),
        );
        assert_eq!(*built.borrow(), [None, Some("inspector".to_owned())]);
    }

    #[test]
    fn a_window_a_controller_asked_for_is_not_built_by_the_app() {
        let platform = platform(&[view(1)]);
        let built = Rc::new(std::cell::Cell::new(0));
        let counted = Rc::clone(&built);
        let controller =
            WindowController::new(&mut platform.cell.borrow_mut(), WindowConfig::default());
        mount(
            &platform.cell,
            ViewCollection::new(vec![
                MultiViewApp::new(move |_app, _view| {
                    counted.set(counted.get() + 1);
                    SizedBox::new().into_widget()
                })
                .into_widget(),
                Window::new(controller, SizedBox::new()).into_widget(),
            ])
            .into_widget(),
        );
        report_windows_made(&platform.cell, &platform.windows, Duration::from_millis(16));
        assert_eq!(built.get(), 1, "the given view only");
        assert_eq!(platform.windows.made()[0].test_view().presented(), 1);
    }

    #[test]
    fn closing_the_apps_last_view_asks_to_quit() {
        let platform = platform(&[]);
        run_app_titled(&platform.cell, "Notes");
        report_windows_made(&platform.cell, &platform.windows, Duration::from_millis(16));
        let view = platform.windows.made()[0].test_view().id();

        request_view_close(&platform.cell, view);
        assert_eq!(platform.quits.count(), 1);
    }

    #[test]
    fn closing_the_apps_last_view_keeps_it_when_the_app_refuses_to_quit() {
        let platform = platform(&[]);
        let listener = {
            let mut app = platform.cell.borrow_mut();
            AppLifecycleListener::new(&mut app).on_exit_requested(
                &mut app,
                Rc::new(|_app| {
                    inset_foundation::Task::ready(inset_embedder::AppExitResponse::Cancel)
                }),
            )
        };
        run_app_titled(&platform.cell, "Notes");
        report_windows_made(&platform.cell, &platform.windows, Duration::from_millis(16));
        let view = platform.windows.made()[0].test_view().id();

        request_view_close(&platform.cell, view);
        assert_eq!(platform.quits.count(), 0);
        assert_eq!(platform.windows.made()[0].closes(), 0);
        listener.dispose(&mut platform.cell.borrow_mut());
    }

    #[test]
    fn closing_one_of_several_views_closes_just_it() {
        let platform = platform(&[view(1)]);
        let controller =
            WindowController::new(&mut platform.cell.borrow_mut(), WindowConfig::default());
        run_app_titled(&platform.cell, "unused");
        report_windows_made(&platform.cell, &platform.windows, Duration::from_millis(16));
        assert!(controller.view(&platform.cell.borrow()).is_some());

        request_view_close(&platform.cell, ViewId(1));
        assert_eq!(platform.quits.count(), 0, "not the last view");
    }

    #[test]
    fn closing_a_given_view_in_a_window_closes_the_window_while_other_views_remain() {
        let window = TestWindow::around(
            TestView::new(300.0, 200.0).with_id(ViewId(1)),
            WindowConfig::default(),
        );
        let platform = platform(&[Rc::clone(window.test_view()), view(2)]);
        run_app_titled(&platform.cell, "unused where views are given");

        request_view_close(&platform.cell, ViewId(1));
        assert_eq!(window.closes(), 1);
        assert_eq!(platform.quits.count(), 0, "not the last view");
    }

    /// An embedder's app: no view at start, no windows on request, views given later.
    fn embedded() -> (Rc<AppCell>, Rc<TestPlatform>) {
        let platform = Rc::new(TestPlatform::new());
        let cell = AppCell::with_platform(Rc::clone(&platform) as inset_embedder::PlatformRef);
        WidgetsBinding::instance(&mut cell.borrow_mut());
        (cell, platform)
    }

    #[test]
    fn a_view_given_after_the_start_shows_the_content() {
        let (cell, platform) = embedded();
        mount(
            &cell,
            MultiViewApp::new(|_app, _view| SizedBox::new().into_widget()).into_widget(),
        );
        let given = view(1);
        platform.give_view(Rc::clone(&given) as ViewRef);
        report_view_added(&cell, ViewId(1), Duration::from_millis(16));
        assert_eq!(given.presented(), 1);
    }

    #[test]
    fn a_given_view_taken_back_is_no_longer_built() {
        let (cell, platform) = embedded();
        let given = view(1);
        platform.give_view(Rc::clone(&given) as ViewRef);
        mount(
            &cell,
            MultiViewApp::new(|_app, _view| SizedBox::new().into_widget()).into_widget(),
        );
        platform.take_view(ViewId(1));
        report_view_removed(&cell, ViewId(1), Duration::from_millis(16));

        let presented = given.presented();
        crate::test_harness::binding_pump(&mut cell.borrow_mut(), Duration::from_millis(32));
        assert_eq!(given.presented(), presented, "not drawn once taken back");
    }

    #[test]
    fn closing_the_apps_only_given_view_asks_to_quit() {
        let platform = platform(&[view(1)]);
        run_app_titled(&platform.cell, "unused where views are given");

        request_view_close(&platform.cell, ViewId(1));
        assert_eq!(platform.quits.count(), 1);
    }

    #[test]
    #[should_panic(expected = "starts with MultiViewApp")]
    fn run_app_given_no_view_where_no_window_can_be_made_says_how_to_start() {
        let (cell, _platform) = embedded();
        run_app(&mut cell.borrow_mut(), SizedBox::new().into_widget());
    }
}
