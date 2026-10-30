//! A view that draws nowhere.

use std::cell::{Cell, RefCell};
use std::rc::Weak;
use std::sync::Arc;

use inset_embedder::{
    HostWindow, RequestId, Scene, ScreenId, View, ViewConstraints, ViewId, ViewMetrics, WindowRef,
};

/// The id of a [`TestView`] given none: the view of a test with one, as the engine numbers
/// Flutter's implicit view.
pub const IMPLICIT_VIEW_ID: ViewId = ViewId(0);

/// A view of a fixed physical size and pixel ratio that counts the scenes presented to it, keeps
/// the last, and shows none. `flutter_test`'s `TestFlutterView` in its two settable dimensions.
pub struct TestView {
    id: ViewId,
    size: Cell<[f64; 2]>,
    device_pixel_ratio: f64,
    /// The screen its metrics say it is on; none by default.
    screen: Option<ScreenId>,
    presented: Cell<u32>,
    last_scene: RefCell<Option<Arc<Scene>>>,
    request_id: Option<RequestId>,
    initial_data: Option<String>,
    /// The window the view is in, until it closes; held weakly, since the window holds the view.
    window: RefCell<Option<Weak<dyn HostWindow>>>,
}

impl TestView {
    /// View [`IMPLICIT_VIEW_ID`] at pixel ratio 1, `width` by `height` physical pixels.
    pub fn new(width: f64, height: f64) -> TestView {
        TestView::with_pixel_ratio(width, height, 1.0)
    }

    /// View [`IMPLICIT_VIEW_ID`] at `device_pixel_ratio`, `width` by `height` physical pixels.
    pub fn with_pixel_ratio(width: f64, height: f64, device_pixel_ratio: f64) -> TestView {
        TestView {
            id: IMPLICIT_VIEW_ID,
            size: Cell::new([width, height]),
            device_pixel_ratio,
            screen: None,
            presented: Cell::new(0),
            last_scene: RefCell::new(None),
            request_id: None,
            initial_data: None,
            window: RefCell::new(None),
        }
    }

    /// The same view under another id, for a platform with several.
    pub fn with_id(mut self, id: ViewId) -> TestView {
        self.id = id;
        self
    }

    /// The same view as one that arrived for request `id`, carrying `initial_data`.
    pub fn with_request(mut self, id: RequestId, initial_data: Option<String>) -> TestView {
        self.request_id = Some(id);
        self.initial_data = initial_data;
        self
    }

    /// The same view on `screen`, as its metrics say.
    pub fn on_screen(mut self, screen: ScreenId) -> TestView {
        self.screen = Some(screen);
        self
    }

    /// The same view as one an embedder gave with `initial_data`.
    pub fn with_initial_data(mut self, initial_data: impl Into<String>) -> TestView {
        self.initial_data = Some(initial_data.into());
        self
    }

    /// Changes the physical size; the platform's client hears of it as it would from a host.
    pub fn resize(&self, width: f64, height: f64) {
        self.size.set([width, height]);
    }

    /// How many pictures have been presented.
    pub fn presented(&self) -> u32 {
        self.presented.get()
    }

    /// The scene presented last, the frame's layer tree, which `inset_embedder::slices` turns
    /// into what it draws and the surfaces it places.
    pub fn last_scene(&self) -> Option<Arc<Scene>> {
        self.last_scene.borrow().clone()
    }

    /// The view is in `window` from now on, or in none.
    pub(crate) fn set_window(&self, window: Option<Weak<dyn HostWindow>>) {
        *self.window.borrow_mut() = window;
    }
}

impl View for TestView {
    fn id(&self) -> ViewId {
        self.id
    }

    fn metrics(&self) -> ViewMetrics {
        let [width, height] = self.size.get();
        ViewMetrics {
            physical_size: [width, height],
            physical_constraints: ViewConstraints::tight(width, height),
            device_pixel_ratio: self.device_pixel_ratio,
            screen: self.screen,
            ..ViewMetrics::default()
        }
    }

    fn present(&self, scene: Arc<Scene>) {
        self.presented.set(self.presented.get() + 1);
        *self.last_scene.borrow_mut() = Some(scene);
    }

    fn initial_data(&self) -> Option<String> {
        self.initial_data.clone()
    }

    fn request_id(&self) -> Option<RequestId> {
        self.request_id
    }

    fn window(&self) -> Option<WindowRef> {
        self.window.borrow().as_ref()?.upgrade()
    }
}
