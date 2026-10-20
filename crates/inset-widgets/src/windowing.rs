//! Flutter counterpart: `WindowController` and its delegate's `onWindowCloseRequested` from
//! `widgets/_window.dart` (experimental).
//!
//! The windows the app asks for itself. A [`WindowController`] asks for one; the binding's
//! [`WindowingOwner`], Flutter's `WidgetsBinding.windowingOwner`, keeps the controllers by
//! their requests and hears every view event first, claiming the views its controllers asked
//! for: a view says which request it arrived for (`View::request_id`), so each reaches its
//! controller whatever order the host reports things in. A view it does not claim goes to the
//! binding's observers, where a `MultiViewApp` takes the views the platform gave.
//!
//! Flutter's `WindowController` makes its window within its constructor. Here the host makes
//! it on its own loop, so a controller exists at once and its view arrives a moment later,
//! which its listeners hear, as they hear the view go (Flutter's `onWindowDestroyed`). They also
//! hear the window minimized or restored, occluded or shown, or moved, at the host's word, as
//! Flutter's controllers notify theirs at the platform's.

use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

use inset_embedder::{RequestId, ViewError, ViewId, ViewRef, ViewRequest, WindowConfig, WindowRef};
use inset_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle};

use crate::binding::WidgetsBinding;

/// What a request to close a view does, given the view: Flutter's
/// `WindowControllerDelegate.onWindowCloseRequested`. It closes the view, or keeps it open.
pub type ViewCloseHandler = Rc<dyn Fn(&mut App, ViewId)>;

/// A window the app asks for itself. Its listeners hear when its view arrives and when it goes,
/// and when the window is minimized or restored, becomes occluded or shown, or moves.
pub struct WindowController {
    change_notifier: ChangeNotifierData,
    /// The request the window was asked for with; `None` when the platform refused it.
    request_id: Option<RequestId>,
    /// The window's view, from its arrival until it goes.
    view: Option<ViewRef>,
    /// Why the platform made no window for the request.
    error: Option<ViewError>,
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
        let request = app.platform().request_view(ViewRequest {
            config,
            initial_data: None,
        });
        let controller = app.create(WindowController {
            change_notifier: ChangeNotifierData::new(),
            request_id: request.as_ref().ok().copied(),
            view: None,
            error: request.err(),
            on_close_requested: None,
            closed_before_arrival: false,
        });
        if let Some(request_id) = app.get(controller).request_id {
            let owner = WidgetsBinding::instance(app).windowing_owner(app);
            owner.add(app, request_id, controller);
        }
        controller
    }

    /// What a request to close the window does from now on, instead of closing it.
    pub fn on_close_requested(self: Handle<Self>, app: &mut App, handler: ViewCloseHandler) {
        app.get_mut(self).on_close_requested = Some(handler);
    }

    /// The window's view, from its arrival until it goes.
    pub fn view(self: Handle<Self>, app: &App) -> Option<ViewRef> {
        app.get(self).view.clone()
    }

    /// Why the platform made no window, where it made none.
    pub fn error(self: Handle<Self>, app: &App) -> Option<ViewError> {
        app.get(self).error.clone()
    }

    /// The window around the view, once the view has arrived.
    pub fn window(self: Handle<Self>, app: &App) -> Option<WindowRef> {
        self.view(app)?.window()
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
    /// as it arrives, unseen.
    pub fn close(self: Handle<Self>, app: &mut App) {
        match self.window(app) {
            Some(window) => window.close(),
            None => app.get_mut(self).closed_before_arrival = true,
        }
    }

    pub(crate) fn view_arrived(self: Handle<Self>, app: &mut App, view: ViewRef) {
        app.get_mut(self).view = Some(view);
        if app.get(self).closed_before_arrival {
            self.close(app);
            return;
        }
        self.notify_listeners(app);
    }

    pub(crate) fn view_gone(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).view = None;
        self.notify_listeners(app);
    }

    pub(crate) fn request_failed(self: Handle<Self>, app: &mut App, error: ViewError) {
        app.get_mut(self).error = Some(error);
        self.notify_listeners(app);
    }

    /// The window was minimized or restored, became occluded or shown, or moved.
    pub(crate) fn window_changed(self: Handle<Self>, app: &mut App) {
        self.notify_listeners(app);
    }

    /// The user asked to close the window: its handler decides, or it closes.
    pub(crate) fn close_requested(self: Handle<Self>, app: &mut App, view: ViewId) {
        match app.get(self).on_close_requested.clone() {
            Some(on_close_requested) => on_close_requested(app, view),
            None => self.close(app),
        }
    }
}

impl ChangeNotifier for WindowController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl fmt::Debug for WindowController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WindowController")
            .field("request_id", &self.request_id)
            .field("view", &self.view.as_ref().map(|view| view.id()))
            .finish_non_exhaustive()
    }
}

/// The window controllers the app made, by their requests, and every event about the views they
/// asked for: Flutter's `WindowingOwner`, which `WidgetsBinding` holds. Flutter's makes each
/// platform's controllers; here the host makes the windows, and the owner only hands each
/// controller what concerns its own.
#[derive(Default)]
pub struct WindowingOwner {
    /// The controllers waiting for their views or showing them.
    controllers: BTreeMap<RequestId, Handle<WindowController>>,
}

impl WindowingOwner {
    pub(crate) fn new(app: &mut App) -> Handle<WindowingOwner> {
        app.create(WindowingOwner::default())
    }

    pub(crate) fn add(
        self: Handle<Self>,
        app: &mut App,
        request_id: RequestId,
        controller: Handle<WindowController>,
    ) {
        app.get_mut(self).controllers.insert(request_id, controller);
    }

    /// Hands `view` to the controller that asked for it; answers whether one did.
    pub(crate) fn view_added(self: Handle<Self>, app: &mut App, view: &ViewRef) -> bool {
        let controller = view
            .request_id()
            .and_then(|request_id| app.get(self).controllers.get(&request_id).copied());
        let Some(controller) = controller else {
            return false;
        };
        controller.view_arrived(app, Rc::clone(view));
        true
    }

    /// Tells the controller whose window showed `view` that it went, and lets go of it; answers
    /// whether one did.
    pub(crate) fn view_removed(self: Handle<Self>, app: &mut App, view: ViewId) -> bool {
        let Some((request_id, controller)) = self.controller_of_view(app, view) else {
            return false;
        };
        app.get_mut(self).controllers.remove(&request_id);
        controller.view_gone(app);
        true
    }

    /// Hands the request to close `view` to the controller whose window shows it; answers
    /// whether one did.
    pub(crate) fn view_close_requested(self: Handle<Self>, app: &mut App, view: ViewId) -> bool {
        let Some((_, controller)) = self.controller_of_view(app, view) else {
            return false;
        };
        controller.close_requested(app, view);
        true
    }

    /// Tells the controller whose window shows `view` that the window was minimized or restored,
    /// became occluded or shown, or moved. A view no controller asked for has nobody to tell.
    pub(crate) fn window_changed(self: Handle<Self>, app: &mut App, view: ViewId) {
        if let Some((_, controller)) = self.controller_of_view(app, view) {
            controller.window_changed(app);
        }
    }

    /// Tells the controller that asked with `request_id` why it gets no window.
    pub(crate) fn view_request_failed(
        self: Handle<Self>,
        app: &mut App,
        request_id: RequestId,
        error: ViewError,
    ) {
        if let Some(controller) = app.get_mut(self).controllers.remove(&request_id) {
            controller.request_failed(app, error);
        }
    }

    fn controller_of_view(
        self: Handle<Self>,
        app: &App,
        view: ViewId,
    ) -> Option<(RequestId, Handle<WindowController>)> {
        app.get(self)
            .controllers
            .iter()
            .find(|(_, controller)| controller.view(app).is_some_and(|shown| shown.id() == view))
            .map(|(request_id, controller)| (*request_id, *controller))
    }
}

impl fmt::Debug for WindowingOwner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WindowingOwner")
            .field("requests", &self.controllers.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Duration;

    use inset_embedder::{View as _, ViewError, WindowConfig};
    use inset_foundation::{AppCell, Listenable, Listener};
    use inset_test::{TestPlatform, TestView, TestWindows};

    use super::*;
    use crate::test_harness::{
        Quits, report_view_removed, report_window_changed, report_windows_made, request_view_close,
        views_cell,
    };
    use crate::{IntoWidget, SizedBox, ViewCollection, Window};

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

    /// How many times `controller`'s listeners are told something, from now on.
    fn notifications_of(cell: &AppCell, controller: Handle<WindowController>) -> Rc<Cell<u32>> {
        let heard = Rc::new(Cell::new(0));
        let counted = Rc::clone(&heard);
        let listenable: Rc<dyn Listenable> = Rc::new(controller);
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
            controller.view(&desktop.cell.borrow()).is_none(),
            "not yet arrived"
        );

        report_windows_made(&desktop.cell, &desktop.windows, Duration::ZERO);
        let window = &desktop.windows.made()[0];
        assert_eq!(window.config().title, "Inspector");
        let view = controller.view(&desktop.cell.borrow()).expect("arrived");
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
        assert!(controller.view(&desktop.cell.borrow()).is_none());
    }

    #[test]
    fn a_window_closed_before_its_view_arrives_closes_unseen_as_it_arrives() {
        let desktop = desktop();
        let controller = new_controller(&desktop.cell, "Inspector");
        let heard = notifications_of(&desktop.cell, controller);
        controller.close(&mut desktop.cell.borrow_mut());

        report_windows_made(&desktop.cell, &desktop.windows, Duration::ZERO);
        assert_eq!(desktop.windows.made()[0].closes(), 1);
        assert_eq!(heard.get(), 0, "its listeners never hear it arrive");
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
            controller
                .view(&desktop.cell.borrow())
                .expect("arrived")
                .id()
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
        assert_eq!(
            controller.error(&cell.borrow()),
            Some(ViewError::Unsupported)
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
}
