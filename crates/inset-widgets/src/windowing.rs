//! Flutter counterpart: `WindowController` and its delegate's `onWindowCloseRequested` from
//! `widgets/_window.dart` (experimental).
//!
//! The views the app asks for itself: a window's, which a [`WindowController`] asks for, and one
//! on a composition layer of the app's own, which the app places and sizes itself, as Flutter's
//! embedder adds a view (`FlutterEngineAddView`). A [`ViewRequest`] is either kind: the request
//! and where it is ([`ViewRequestState`]), pending, holding its view, refused or ended, each change
//! of which its listeners hear.
//! The binding's [`WindowingOwner`], Flutter's `WidgetsBinding.windowingOwner`, keeps the requests
//! and the window controllers by their requests' ids and hears every view event first, claiming
//! the views the app asked for: a view says which request it arrived for (`View::request_id`), so
//! each reaches its request whatever order the host reports things in. A view it does not claim
//! goes to the binding's observers, where a `MultiViewApp` takes the views the platform gave.
//!
//! Flutter's `WindowController` makes its window within its constructor. Here the host makes
//! it on its own loop, so a controller exists at once and its view arrives a moment later,
//! which its listeners hear, as they hear the view go (Flutter's `onWindowDestroyed`). They also
//! hear the window minimized or restored, occluded or shown, or moved, at the host's word, as
//! Flutter's controllers notify theirs at the platform's.

use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use inset_embedder::{
    CompositionLayer, RequestId, ViewError, ViewHost, ViewId, ViewMetrics, ViewRef, WindowConfig,
    WindowRef,
};
use inset_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, ListenableObject, Listener,
};

use crate::binding::WidgetsBinding;

/// What a request to close a view does, given the view: Flutter's
/// `WindowControllerDelegate.onWindowCloseRequested`. It closes the view, or keeps it open.
pub type ViewCloseHandler = Rc<dyn Fn(&mut App, ViewId)>;

/// A view the app asked for: a window's, which a [`WindowController`] holds, or one on a
/// composition layer of the app's own ([`on_layer`](Self::on_layer)). Its listeners hear each
/// change of its [`state`](Self::state).
pub struct ViewRequest {
    change_notifier: ChangeNotifierData,
    /// The id the platform answered the request with; `None` when it refused the request at once.
    request_id: Option<RequestId>,
    state: ViewRequestState,
}

/// Where a [`ViewRequest`] is: waiting, holding its view, or over, with no view to come.
pub enum ViewRequestState {
    /// Asked for, with nothing yet.
    Pending,
    /// The view, from its arrival until it goes.
    Arrived(ViewRef),
    /// The platform gave no view, and why.
    Refused(ViewError),
    /// The view went, or its window was closed before it arrived.
    Ended,
}

impl ViewRequest {
    /// Asks the platform's view embedder for a view drawing into `layer`, which the app made with
    /// the platform's compositor and places itself, at `metrics`, asking what it asks of the system
    /// of `host` (`ViewEmbedder::add_view`). The app gives the view its metrics, says whether it is
    /// in sight and removes it through the view embedder. `None` where the platform has no view
    /// embedder; refused at once where it cannot draw a view into `layer`.
    pub fn on_layer(
        app: &mut App,
        layer: &Arc<dyn CompositionLayer>,
        metrics: ViewMetrics,
        host: Rc<dyn ViewHost>,
    ) -> Option<Handle<ViewRequest>> {
        let platform = app.platform();
        let answer = platform.view_embedder()?.add_view(layer, metrics, host);
        Some(ViewRequest::answered(app, answer))
    }

    /// Asks the platform for a view in a window configured as `config` (`Platform::request_view`).
    fn in_window(app: &mut App, config: WindowConfig) -> Handle<ViewRequest> {
        let answer = app.platform().request_view(config, None);
        ViewRequest::answered(app, answer)
    }

    /// The request the platform answered with `answer`, which the windowing owner hands its view
    /// once it arrives.
    fn answered(app: &mut App, answer: Result<RequestId, ViewError>) -> Handle<ViewRequest> {
        let (request_id, state) = match answer {
            Ok(request_id) => (Some(request_id), ViewRequestState::Pending),
            Err(error) => (None, ViewRequestState::Refused(error)),
        };
        let request = app.create(ViewRequest {
            change_notifier: ChangeNotifierData::new(),
            request_id,
            state,
        });
        if let Some(request_id) = request_id {
            let owner = WidgetsBinding::instance(app).windowing_owner(app);
            owner.add_request(app, request_id, request);
        }
        request
    }

    /// Where the request is now.
    pub fn state(self: Handle<Self>, app: &App) -> &ViewRequestState {
        &app.get(self).state
    }

    /// The view, while it is there.
    fn arrived_view(self: Handle<Self>, app: &App) -> Option<&ViewRef> {
        match self.state(app) {
            ViewRequestState::Arrived(view) => Some(view),
            _ => None,
        }
    }

    fn view_arrived(self: Handle<Self>, app: &mut App, view: ViewRef) {
        self.change_to(app, ViewRequestState::Arrived(view));
    }

    fn refused(self: Handle<Self>, app: &mut App, error: ViewError) {
        self.change_to(app, ViewRequestState::Refused(error));
    }

    /// The view went, or its window was closed before it arrived: no view comes any more.
    fn ended(self: Handle<Self>, app: &mut App) {
        self.change_to(app, ViewRequestState::Ended);
    }

    fn change_to(self: Handle<Self>, app: &mut App, state: ViewRequestState) {
        app.get_mut(self).state = state;
        self.notify_listeners(app);
    }
}

impl ChangeNotifier for ViewRequest {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl fmt::Debug for ViewRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewRequest")
            .field("request_id", &self.request_id)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for ViewRequestState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewRequestState::Pending => f.write_str("Pending"),
            ViewRequestState::Arrived(view) => f.debug_tuple("Arrived").field(&view.id()).finish(),
            ViewRequestState::Refused(error) => f.debug_tuple("Refused").field(error).finish(),
            ViewRequestState::Ended => f.write_str("Ended"),
        }
    }
}

/// A window the app asks for itself: the request for its view, with what only a window has, what
/// a request to close it does, and its closing. Its listeners are its request's: they hear when
/// its view arrives and when it goes, and when the window is minimized or restored, becomes
/// occluded or shown, or moves.
pub struct WindowController {
    /// The request for the window's view.
    request: Handle<ViewRequest>,
    /// What a request to close the window does; `None` closes it.
    on_close_requested: Option<ViewCloseHandler>,
    /// Whether the app closed the window before its view arrived, so that it closes as it
    /// arrives.
    closed_before_arrival: bool,
}

impl WindowController {
    /// Asks the platform for a window configured as `config`. Where the platform makes no
    /// windows on request, the controller holds the error and never gets a view.
    pub fn new(app: &mut App, config: WindowConfig) -> Handle<WindowController> {
        let request = ViewRequest::in_window(app, config);
        let controller = app.create(WindowController {
            request,
            on_close_requested: None,
            closed_before_arrival: false,
        });
        if let Some(request_id) = app.get(request).request_id {
            let owner = WidgetsBinding::instance(app).windowing_owner(app);
            owner.add_window(app, request_id, controller);
        }
        controller
    }

    /// What a request to close the window does from now on, instead of closing it.
    pub fn on_close_requested(self: Handle<Self>, app: &mut App, handler: ViewCloseHandler) {
        app.get_mut(self).on_close_requested = Some(handler);
    }

    /// The request for the window's view, whose state says where the view is.
    pub fn request(self: Handle<Self>, app: &App) -> Handle<ViewRequest> {
        app.get(self).request
    }

    /// The window around the view, once the view has arrived.
    pub fn window(self: Handle<Self>, app: &App) -> Option<WindowRef> {
        self.request(app).arrived_view(app)?.window()
    }

    /// Whether the window is minimized, as its host says: Flutter's `isMinimized`. `false` while
    /// there is no window, before its view arrives and after it goes.
    pub fn is_minimized(self: Handle<Self>, app: &App) -> bool {
        self.window(app).is_some_and(|window| window.is_minimized())
    }

    /// Whether the window is occluded, covered all over or otherwise not shown, as its host says;
    /// `false` while there is no window, before its view arrives and after it goes. Not in
    /// Flutter's controller.
    pub fn is_occluded(self: Handle<Self>, app: &App) -> bool {
        self.window(app).is_some_and(|window| window.is_occluded())
    }

    /// Closes the window; its view goes with it. A window whose view has not arrived yet closes
    /// as it arrives, unseen, and its request ends with no view.
    pub fn close(self: Handle<Self>, app: &mut App) {
        match self.window(app) {
            Some(window) => window.close(),
            None => app.get_mut(self).closed_before_arrival = true,
        }
    }

    /// Whether the app closed the window before `view`, its view, arrived: the window then closes
    /// now, and its request ends without the view.
    fn closes_as_it_arrives(self: Handle<Self>, app: &App, view: &ViewRef) -> bool {
        if !app.get(self).closed_before_arrival {
            return false;
        }
        if let Some(window) = view.window() {
            window.close();
        }
        true
    }

    /// The window was minimized or restored, became occluded or shown, or moved.
    fn window_changed(self: Handle<Self>, app: &mut App) {
        self.request(app).notify_listeners(app);
    }

    /// The user asked to close the window: its handler decides, or it closes.
    fn close_requested(self: Handle<Self>, app: &mut App, view: ViewId) {
        match app.get(self).on_close_requested.clone() {
            Some(on_close_requested) => on_close_requested(app, view),
            None => self.close(app),
        }
    }
}

/// The controller's listeners are its request's.
impl ListenableObject for WindowController {
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        let request = app.get(self).request;
        ListenableObject::add_listener(request, app, listener);
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        if !app.contains(self) {
            return;
        }
        let request = app.get(self).request;
        ListenableObject::remove_listener(request, app, listener);
    }
}

impl fmt::Debug for WindowController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WindowController")
            .field("request", &self.request)
            .finish_non_exhaustive()
    }
}

/// The views the app asked for, by their requests, the window controllers among them, and every
/// event about those views: Flutter's `WindowingOwner`, which `WidgetsBinding` holds. Flutter's
/// makes each platform's controllers; here the host makes the windows and the views, and the owner
/// only hands each request and each window what concerns its own.
#[derive(Default)]
pub struct WindowingOwner {
    /// The requests waiting for their views or holding them, a window's or one on a layer, through
    /// which every arriving or departing view is handed.
    requested: BTreeMap<RequestId, Handle<ViewRequest>>,
    /// The window controllers, by the requests for their views, for what only a window hears: a
    /// request to close it, and a change of the window.
    windows: BTreeMap<RequestId, Handle<WindowController>>,
}

impl WindowingOwner {
    pub(crate) fn new(app: &mut App) -> Handle<WindowingOwner> {
        app.create(WindowingOwner::default())
    }

    fn add_request(self: Handle<Self>, app: &mut App, id: RequestId, request: Handle<ViewRequest>) {
        app.get_mut(self).requested.insert(id, request);
    }

    fn add_window(
        self: Handle<Self>,
        app: &mut App,
        id: RequestId,
        window: Handle<WindowController>,
    ) {
        app.get_mut(self).windows.insert(id, window);
    }

    /// Hands `view` to the request it arrived for; answers whether the app asked for it. A window
    /// the app closed before then closes unseen, and its request ends and is let go of.
    pub(crate) fn view_added(self: Handle<Self>, app: &mut App, view: &ViewRef) -> bool {
        let Some(request_id) = view.request_id() else {
            return false;
        };
        let Some(request) = app.get(self).requested.get(&request_id).copied() else {
            return false;
        };
        let window = app.get(self).windows.get(&request_id).copied();
        if window.is_some_and(|window| window.closes_as_it_arrives(app, view)) {
            self.forget(app, request_id);
            request.ended(app);
        } else {
            request.view_arrived(app, Rc::clone(view));
        }
        true
    }

    /// Tells the request whose view `view` was that it went, and lets go of it; answers whether
    /// the app asked for the view.
    pub(crate) fn view_removed(self: Handle<Self>, app: &mut App, view: ViewId) -> bool {
        let request = self
            .request_showing(app, view)
            .and_then(|request_id| self.forget(app, request_id));
        let Some(request) = request else {
            return false;
        };
        request.ended(app);
        true
    }

    /// Hands the request to close `view` to the controller whose window shows it; answers
    /// whether one does.
    pub(crate) fn view_close_requested(self: Handle<Self>, app: &mut App, view: ViewId) -> bool {
        let Some(window) = self.window_showing(app, view) else {
            return false;
        };
        window.close_requested(app, view);
        true
    }

    /// Tells the controller whose window shows `view` that the window was minimized or restored,
    /// became occluded or shown, or moved. A view no controller asked for has nobody to tell.
    pub(crate) fn window_changed(self: Handle<Self>, app: &mut App, view: ViewId) {
        if let Some(window) = self.window_showing(app, view) {
            window.window_changed(app);
        }
    }

    /// Tells the request with `request_id` why it gets no view, and lets go of it.
    pub(crate) fn view_request_failed(
        self: Handle<Self>,
        app: &mut App,
        request_id: RequestId,
        error: ViewError,
    ) {
        if let Some(request) = self.forget(app, request_id) {
            request.refused(app, error);
        }
    }

    /// The id of the request whose view is `view`.
    fn request_showing(self: Handle<Self>, app: &App, view: ViewId) -> Option<RequestId> {
        app.get(self)
            .requested
            .iter()
            .find(|(_, request)| {
                request
                    .arrived_view(app)
                    .is_some_and(|shown| shown.id() == view)
            })
            .map(|(request_id, _)| *request_id)
    }

    /// The controller whose window shows `view`.
    fn window_showing(
        self: Handle<Self>,
        app: &App,
        view: ViewId,
    ) -> Option<Handle<WindowController>> {
        let request_id = self.request_showing(app, view)?;
        app.get(self).windows.get(&request_id).copied()
    }

    /// Lets go of the request with `request_id`, and of its window's controller where it has one;
    /// answers the request, where the owner kept it.
    fn forget(
        self: Handle<Self>,
        app: &mut App,
        request_id: RequestId,
    ) -> Option<Handle<ViewRequest>> {
        app.get_mut(self).windows.remove(&request_id);
        app.get_mut(self).requested.remove(&request_id)
    }
}

impl fmt::Debug for WindowingOwner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WindowingOwner")
            .field("requests", &self.requested.keys().collect::<Vec<_>>())
            .field("windows", &self.windows.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Duration;

    use inset_embedder::{Compositor, View as _, ViewError, WindowConfig};
    use inset_foundation::{AppCell, Listenable, Listener};
    use inset_test::{MockCompositor, TestPlatform, TestView, TestViewHost, TestWindows};

    use super::*;
    use crate::test_harness::{
        Quits, report_view_added, report_view_removed, report_window_changed, report_windows_made,
        request_view_close, views_cell,
    };
    use crate::{IntoWidget, MultiViewApp, SizedBox, ViewCollection, Window};

    struct Desktop {
        cell: Rc<AppCell>,
        windows: Rc<TestWindows>,
    }

    /// A desktop's app: no view given, windows made on request.
    fn desktop() -> Desktop {
        let windows = Rc::new(TestWindows::new());
        let cell = views_cell(&[], &windows, &Rc::new(Quits::default()));
        Desktop { cell, windows }
    }

    fn titled(title: &str) -> WindowConfig {
        WindowConfig {
            title: title.to_owned(),
            ..WindowConfig::default()
        }
    }

    fn new_controller(cell: &AppCell, title: &str) -> Handle<WindowController> {
        WindowController::new(&mut cell.borrow_mut(), titled(title))
    }

    /// An app on a platform that adds views on layers, with a layer of its own to add one on.
    struct Compositing {
        cell: Rc<AppCell>,
        platform: Rc<TestPlatform>,
        layer: Arc<dyn CompositionLayer>,
    }

    fn compositing() -> Compositing {
        let compositor = Arc::new(MockCompositor::new());
        let layer = compositor.make_layer(false);
        let platform = Rc::new(TestPlatform::new().with_compositor(compositor));
        let cell = AppCell::with_platform(Rc::clone(&platform) as inset_embedder::PlatformRef);
        WidgetsBinding::instance(&mut cell.borrow_mut());
        Compositing {
            cell,
            platform,
            layer,
        }
    }

    /// Asks for a view on `layer`, 200 by 100 pixels at two to a point.
    fn request_on(
        cell: &AppCell,
        layer: &Arc<dyn CompositionLayer>,
    ) -> Option<Handle<ViewRequest>> {
        let metrics = ViewMetrics {
            physical_size: [200.0, 100.0],
            device_pixel_ratio: 2.0,
            ..ViewMetrics::default()
        };
        let host = Rc::new(TestViewHost::default());
        ViewRequest::on_layer(&mut cell.borrow_mut(), layer, metrics, host)
    }

    /// The view of `controller`'s window, from its arrival until it goes.
    fn view_of_window(cell: &AppCell, controller: Handle<WindowController>) -> Option<ViewRef> {
        let app = cell.borrow();
        match controller.request(&app).state(&app) {
            ViewRequestState::Arrived(view) => Some(Rc::clone(view)),
            _ => None,
        }
    }

    /// How many times the listeners of `listened` are told something, from now on.
    fn notifications_of<T: ListenableObject>(cell: &AppCell, listened: Handle<T>) -> Rc<Cell<u32>> {
        let heard = Rc::new(Cell::new(0));
        let counted = Rc::clone(&heard);
        let listenable: Rc<dyn Listenable> = Rc::new(listened);
        listenable.add_listener(
            &mut cell.borrow_mut(),
            Listener::new(move |_app| counted.set(counted.get() + 1)),
        );
        heard
    }

    #[test]
    fn a_controller_gets_its_view_when_it_arrives() {
        let desktop = desktop();
        let controller = new_controller(&desktop.cell, "Inspector");
        let heard = notifications_of(&desktop.cell, controller);
        assert!(
            view_of_window(&desktop.cell, controller).is_none(),
            "not yet arrived"
        );

        report_windows_made(&desktop.cell, &desktop.windows, Duration::ZERO);
        let window = &desktop.windows.made()[0];
        assert_eq!(window.config().title, "Inspector");
        let view = view_of_window(&desktop.cell, controller).expect("arrived");
        assert_eq!(view.id(), window.test_view().id());
        assert_eq!(heard.get(), 1);
    }

    #[test]
    fn a_controllers_view_goes_when_its_window_closes() {
        let desktop = desktop();
        let controller = new_controller(&desktop.cell, "Inspector");
        report_windows_made(&desktop.cell, &desktop.windows, Duration::ZERO);

        let view = desktop.windows.made()[0].test_view().id();
        controller.close(&mut desktop.cell.borrow_mut());
        report_view_removed(&desktop.cell, view, Duration::ZERO);
        assert_eq!(desktop.windows.made()[0].closes(), 1);
        assert!(view_of_window(&desktop.cell, controller).is_none());
    }

    #[test]
    fn a_window_closed_before_its_view_arrives_closes_unseen_and_its_request_ends() {
        let desktop = desktop();
        let controller = new_controller(&desktop.cell, "Inspector");
        let heard = notifications_of(&desktop.cell, controller);
        controller.close(&mut desktop.cell.borrow_mut());

        report_windows_made(&desktop.cell, &desktop.windows, Duration::ZERO);
        assert_eq!(desktop.windows.made()[0].closes(), 1);
        assert_eq!(heard.get(), 1, "its listeners hear the request end");
        let app = desktop.cell.borrow();
        let state = controller.request(&app).state(&app);
        assert!(matches!(state, ViewRequestState::Ended), "{state:?}");
    }

    #[test]
    fn a_controllers_listeners_hear_its_window_change_and_it_answers_how_the_window_is() {
        let desktop = desktop();
        let controller = new_controller(&desktop.cell, "Inspector");
        let not_minimized_nor_occluded = |cell: &AppCell| {
            !controller.is_minimized(&cell.borrow()) && !controller.is_occluded(&cell.borrow())
        };
        assert!(not_minimized_nor_occluded(&desktop.cell), "no window yet");
        report_windows_made(&desktop.cell, &desktop.windows, Duration::ZERO);
        let heard = notifications_of(&desktop.cell, controller);
        let window = Rc::clone(&desktop.windows.made()[0]);
        let view = window.test_view().id();
        assert!(not_minimized_nor_occluded(&desktop.cell));

        window.set_minimized(true);
        report_window_changed(&desktop.cell, view);
        assert_eq!(heard.get(), 1);
        assert!(controller.is_minimized(&desktop.cell.borrow()));

        window.set_minimized(false);
        window.set_occluded(true);
        report_window_changed(&desktop.cell, view);
        assert_eq!(heard.get(), 2);
        assert!(!controller.is_minimized(&desktop.cell.borrow()));
        assert!(controller.is_occluded(&desktop.cell.borrow()));

        report_window_changed(&desktop.cell, view);
        assert_eq!(heard.get(), 3, "a move is heard as a change too");
    }

    #[test]
    fn a_window_change_of_a_view_no_controller_asked_for_reaches_no_controller() {
        let desktop = desktop();
        let controller = new_controller(&desktop.cell, "Inspector");
        report_windows_made(&desktop.cell, &desktop.windows, Duration::ZERO);
        let heard = notifications_of(&desktop.cell, controller);

        report_window_changed(&desktop.cell, ViewId(7));
        assert_eq!(heard.get(), 0);
    }

    #[test]
    fn a_close_request_closes_a_controllers_window_unless_its_handler_decides() {
        let desktop = desktop();
        let closing = new_controller(&desktop.cell, "closes when asked");
        let deciding = new_controller(&desktop.cell, "decides");
        let asked = Rc::new(Cell::new(0));
        let counted = Rc::clone(&asked);
        deciding.on_close_requested(
            &mut desktop.cell.borrow_mut(),
            Rc::new(move |_app, _view| counted.set(counted.get() + 1)),
        );
        report_windows_made(&desktop.cell, &desktop.windows, Duration::ZERO);
        let view_of = |controller: Handle<WindowController>| {
            let view = view_of_window(&desktop.cell, controller);
            view.expect("arrived").id()
        };

        request_view_close(&desktop.cell, view_of(closing));
        request_view_close(&desktop.cell, view_of(deciding));
        let made = desktop.windows.made();
        assert_eq!(made[0].closes(), 1);
        assert_eq!(made[1].closes(), 0, "kept open");
        assert_eq!(asked.get(), 1);
    }

    #[test]
    fn a_platform_that_makes_no_windows_leaves_the_controller_with_its_error() {
        let cell = AppCell::with_platform(Rc::new(
            TestPlatform::new().with_view(Rc::new(TestView::new(300.0, 200.0))),
        ));
        let controller = new_controller(&cell, "Inspector");
        let app = cell.borrow();
        let state = controller.request(&app).state(&app);
        assert!(
            matches!(state, ViewRequestState::Refused(ViewError::Unsupported)),
            "{state:?}"
        );
    }

    #[test]
    fn a_window_shows_its_child_once_its_view_arrives() {
        let desktop = desktop();
        let controller = new_controller(&desktop.cell, "Inspector");
        let root =
            ViewCollection::new(vec![Window::new(controller, SizedBox::new()).into_widget()]);
        crate::binding::run_widget(&mut desktop.cell.borrow_mut(), root.into_widget());
        desktop.cell.elapse(Duration::ZERO);
        let window = Rc::clone(&desktop.windows.made()[0]);
        assert_eq!(
            window.test_view().presented(),
            0,
            "nothing to draw into yet"
        );

        report_windows_made(&desktop.cell, &desktop.windows, Duration::from_millis(16));
        assert_eq!(window.test_view().presented(), 1);
    }

    #[test]
    fn a_request_on_a_layer_gets_its_view_as_it_arrives_and_loses_it_as_it_goes() {
        let compositing = compositing();
        let request = request_on(&compositing.cell, &compositing.layer).expect("a view embedder");
        let heard = notifications_of(&compositing.cell, request);
        assert!(matches!(
            request.state(&compositing.cell.borrow()),
            ViewRequestState::Pending
        ));

        let added = compositing.platform.added_views()[0].id();
        report_view_added(&compositing.cell, added, Duration::ZERO);
        assert!(matches!(
            request.state(&compositing.cell.borrow()),
            ViewRequestState::Arrived(view) if view.id() == added
        ));
        assert_eq!(heard.get(), 1);

        let platform = compositing.cell.borrow().platform();
        let embedder = platform.view_embedder().expect("a view embedder");
        embedder.remove_view(added);
        report_view_removed(&compositing.cell, added, Duration::ZERO);
        assert!(matches!(
            request.state(&compositing.cell.borrow()),
            ViewRequestState::Ended
        ));
        assert_eq!(heard.get(), 2);
    }

    #[test]
    fn a_platform_without_a_view_embedder_takes_no_request_on_a_layer() {
        let cell = AppCell::with_platform(Rc::new(TestPlatform::new()));
        WidgetsBinding::instance(&mut cell.borrow_mut());
        let layer = MockCompositor::new().make_layer(false);
        assert!(request_on(&cell, &layer).is_none());
    }

    #[test]
    fn a_view_added_on_a_layer_is_not_built_by_a_multi_view_app() {
        let compositing = compositing();
        let built = Rc::new(Cell::new(0));
        let counted = Rc::clone(&built);
        let app = MultiViewApp::new(move |_app, _view| {
            counted.set(counted.get() + 1);
            SizedBox::new().into_widget()
        });
        crate::binding::run_widget(&mut compositing.cell.borrow_mut(), app.into_widget());
        compositing.cell.elapse(Duration::ZERO);
        request_on(&compositing.cell, &compositing.layer).expect("a view embedder");

        let added = Rc::clone(&compositing.platform.added_views()[0]);
        report_view_added(&compositing.cell, added.id(), Duration::from_millis(16));
        assert_eq!(built.get(), 0);
        assert_eq!(added.presented(), 0);
    }
}
