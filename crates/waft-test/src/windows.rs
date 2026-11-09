//! Requests for views that a desktop answers at once, with a window each.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

use waft_embedder::{
    HostWindow, RawWindowHandle, Rect, RequestId, ScreenId, Size, ViewId, ViewRef, WindowConfig,
    WindowSizeLimits,
};

use crate::TestView;

/// The number of the first view made on request, out of the way of the views a test gives.
const FIRST_VIEW_ID: u64 = 1000;

/// The number of the view made for request `id`: a window's, or one added on a layer.
pub(crate) fn view_of_request(id: RequestId) -> ViewId {
    ViewId(FIRST_VIEW_ID + id.0)
}

/// A desktop's answer to requests for views: each request makes a window at once, over a view
/// that carries the request's id and initial data. A test hands it to a platform
/// ([`TestPlatform::with_windows`](crate::TestPlatform::with_windows)), which lists the views
/// of the windows still open; the test tells the framework of each new view itself, as a host
/// would through `EmbedderClient::view_added`.
pub struct TestWindows {
    made: RefCell<Vec<Rc<TestWindow>>>,
    /// The screen every window is made on; none by default.
    screen: Option<ScreenId>,
}

impl TestWindows {
    pub fn new() -> TestWindows {
        TestWindows {
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

    /// Makes the window configured as `config` that request `id` asks for, over a view carrying
    /// `initial_data`.
    pub(crate) fn request(
        &self,
        id: RequestId,
        config: WindowConfig,
        initial_data: Option<String>,
    ) {
        let [width, height] = config.size;
        let mut view = TestView::new(width, height)
            .with_id(view_of_request(id))
            .with_request(id, initial_data);
        if let Some(screen) = self.screen {
            view = view.on_screen(screen);
        }
        self.made
            .borrow_mut()
            .push(TestWindow::around(view, config));
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

/// A window that shows nothing, keeps the frame it was last given, whether it was last shown or
/// hidden, whether it is maximized and the size limits it was last given, and counts how often it
/// was closed and dragged: one of
/// [`TestWindows`], or one a test gives the app a view in. A closed window's view is in no window any more, and is no longer among the
/// views of [`TestWindows`]. It is neither minimized nor occluded until a test says so, and the
/// test then reports the change itself, as a host would through `EmbedderClient::window_changed`.
pub struct TestWindow {
    view: Rc<TestView>,
    config: WindowConfig,
    frame: Cell<Rect>,
    shown: Cell<bool>,
    closes: Cell<u32>,
    drags: Cell<u32>,
    minimized: Cell<bool>,
    maximized: Cell<bool>,
    occluded: Cell<bool>,
    min_size: Cell<Option<Size>>,
    max_size: Cell<Option<Size>>,
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
                drags: Cell::new(0),
                minimized: Cell::new(false),
                maximized: Cell::new(false),
                occluded: Cell::new(false),
                min_size: Cell::new(None),
                max_size: Cell::new(None),
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

    /// How many times the window was dragged.
    pub fn drags(&self) -> u32 {
        self.drags.get()
    }

    /// The window is minimized, or restored, from now on.
    pub fn set_minimized(&self, minimized: bool) {
        self.minimized.set(minimized);
    }

    /// The window is occluded, or shown, from now on.
    pub fn set_occluded(&self, occluded: bool) {
        self.occluded.set(occluded);
    }

    /// The smallest size the window was last given for its view; none until one is set.
    pub fn min_size(&self) -> Option<Size> {
        self.min_size.get()
    }

    /// The largest size the window was last given for its view; none until one is set.
    pub fn max_size(&self) -> Option<Size> {
        self.max_size.get()
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

    fn size_limits(&self) -> Option<&dyn WindowSizeLimits> {
        Some(self)
    }

    fn drag(&self) {
        self.drags.set(self.drags.get() + 1);
    }

    fn is_maximized(&self) -> bool {
        self.maximized.get()
    }

    fn set_maximized(&self, maximized: bool) {
        self.maximized.set(maximized);
    }

    fn set_minimized(&self, minimized: bool) {
        self.minimized.set(minimized);
    }

    fn native_handle(&self) -> Option<RawWindowHandle> {
        None
    }
}

impl WindowSizeLimits for TestWindow {
    fn set_min_size(&self, size: Option<Size>) {
        self.min_size.set(size);
    }

    fn set_max_size(&self, size: Option<Size>) {
        self.max_size.set(size);
    }
}
