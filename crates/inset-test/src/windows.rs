//! Requests for views that a desktop answers at once, with a window each.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

use inset_embedder::{
    HostWindow, RawWindowHandle, Rect, RequestId, ScreenId, ViewId, ViewRef, ViewRequest,
    WindowConfig,
};

use crate::TestView;

/// The number of the first view made on request, out of the way of the views a test gives.
const FIRST_VIEW_ID: u64 = 1000;

/// A desktop's answer to requests for views: each request makes a window at once, over a view
/// that carries the request's id and initial data. A test hands it to a platform
/// ([`TestPlatform::with_windows`](crate::TestPlatform::with_windows)), which lists the views
/// of the windows still open; the test tells the framework of each new view itself, as a host
/// would through `EmbedderClient::view_added`.
pub struct TestWindows {
    next_request: Cell<u64>,
    made: RefCell<Vec<Rc<TestWindow>>>,
    /// The screen every window is made on; none by default.
    screen: Option<ScreenId>,
}

impl TestWindows {
    pub fn new() -> TestWindows {
        TestWindows {
            next_request: Cell::new(1),
            made: RefCell::default(),
            screen: None,
        }
    }

    /// The same desktop making every window on `screen`, which each view's metrics name.
    pub fn on_screen(mut self, screen: ScreenId) -> TestWindows {
        self.screen = Some(screen);
        self
    }

    /// The windows made so far, in the order they were asked for, closed ones included.
    pub fn made(&self) -> Vec<Rc<TestWindow>> {
        self.made.borrow().clone()
    }

    /// Makes the window `request` asks for, and answers the request's id.
    pub(crate) fn request(&self, request: ViewRequest) -> RequestId {
        let id = RequestId(self.next_request.get());
        self.next_request.set(id.0 + 1);
        let [width, height] = request.config.size;
        let mut view = TestView::new(width, height)
            .with_id(ViewId(FIRST_VIEW_ID + id.0))
            .with_request(id, request.initial_data);
        if let Some(screen) = self.screen {
            view = view.on_screen(screen);
        }
        self.made
            .borrow_mut()
            .push(TestWindow::around(view, request.config));
        id
    }

    /// The views of the windows still open.
    pub(crate) fn open_views(&self) -> Vec<ViewRef> {
        self.made
            .borrow()
            .iter()
            .filter(|window| window.closes() == 0)
            .map(|window| Rc::clone(&window.view) as ViewRef)
            .collect()
    }
}

impl Default for TestWindows {
    fn default() -> TestWindows {
        TestWindows::new()
    }
}

/// A window that shows nothing, keeps the frame it was last given and whether it was last shown
/// or hidden, and counts how often it was closed: one of [`TestWindows`], or one a test gives the
/// app a view in. A closed window's view is in no window any more, and is no longer among the
/// views of [`TestWindows`]. It is neither minimized nor occluded until a test says so, and the
/// test then reports the change itself, as a host would through `EmbedderClient::window_changed`.
pub struct TestWindow {
    view: Rc<TestView>,
    config: WindowConfig,
    frame: Cell<Rect>,
    shown: Cell<bool>,
    closes: Cell<u32>,
    minimized: Cell<bool>,
    occluded: Cell<bool>,
}

impl TestWindow {
    /// A window configured as `config` around `view`, which is in it from now on: as a desktop
    /// makes for a request, or as WaOS shows a program's view it gives. It starts where the
    /// configuration places it, or at the origin, and shown as the configuration says.
    pub fn around(view: TestView, config: WindowConfig) -> Rc<TestWindow> {
        let [width, height] = config.size;
        let [left, top] = config.position.unwrap_or([0.0, 0.0]);
        Rc::new_cyclic(|window: &Weak<TestWindow>| {
            let window: Weak<dyn HostWindow> = window.clone();
            view.set_window(Some(window));
            TestWindow {
                view: Rc::new(view),
                frame: Cell::new(Rect::from_ltwh(left, top, width, height)),
                shown: Cell::new(config.visible),
                config,
                closes: Cell::new(0),
                minimized: Cell::new(false),
                occluded: Cell::new(false),
            }
        })
    }

    /// The view the window shows, which counts the pictures presented to it.
    pub fn test_view(&self) -> &Rc<TestView> {
        &self.view
    }

    /// The configuration the window was asked for with.
    pub fn config(&self) -> &WindowConfig {
        &self.config
    }

    /// Whether the window was last shown, rather than hidden.
    pub fn is_shown(&self) -> bool {
        self.shown.get()
    }

    /// How many times the window was closed.
    pub fn closes(&self) -> u32 {
        self.closes.get()
    }

    /// The window is minimized, or restored, from now on.
    pub fn set_minimized(&self, minimized: bool) {
        self.minimized.set(minimized);
    }

    /// The window is occluded, or shown, from now on.
    pub fn set_occluded(&self, occluded: bool) {
        self.occluded.set(occluded);
    }
}

impl HostWindow for TestWindow {
    fn view(&self) -> ViewRef {
        Rc::clone(&self.view) as ViewRef
    }

    fn frame(&self) -> Rect {
        self.frame.get()
    }

    fn set_frame(&self, frame: Rect, _animate: Option<Duration>) {
        self.frame.set(frame);
    }

    fn set_title(&self, _title: &str) {}

    fn show(&self) {
        self.shown.set(true);
    }

    fn hide(&self) {
        self.shown.set(false);
    }

    fn is_minimized(&self) -> bool {
        self.minimized.get()
    }

    fn is_occluded(&self) -> bool {
        self.occluded.get()
    }

    fn close(&self) {
        self.closes.set(self.closes.get() + 1);
        self.view.set_window(None);
    }

    fn native_handle(&self) -> Option<RawWindowHandle> {
        None
    }
}
