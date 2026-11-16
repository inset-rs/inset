//! A view that draws nowhere, whose metrics a test sets.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

use waft_embedder::{
    GestureSettings, HostWindow, RequestId, Scene, ScreenId, Size, View, ViewConstraints, ViewHost,
    ViewId, ViewMetrics, ViewPadding, WindowRef,
};

use crate::view_host::TestViewHost;

/// The id of a [`TestView`] given none: the view of a test with one, as the engine numbers
/// Flutter's implicit view.
pub const IMPLICIT_VIEW_ID: ViewId = ViewId(0);

/// What hears that a [`TestView`]'s metrics changed: the host's client, which
/// `EmbedderClient::view_metrics_changed` reaches in an app a host runs.
pub type MetricsListener = Rc<dyn Fn(ViewId)>;

/// A view that counts the scenes presented to it, keeps the last, shows none, and records what it
/// asks of the system in its host: `flutter_test`'s `TestFlutterView`.
///
/// Its metrics are what it was made with until a test sets one; each setter, and each `reset_*`
/// that puts a metric back to what the view was made with, tells the [`MetricsListener`], as
/// `TestFlutterView`'s setters call `onMetricsChanged`.
pub struct TestView {
    id: ViewId,
    /// What the view was made with, which the `reset_*` methods return to.
    initial_metrics: ViewMetrics,
    metrics: Cell<ViewMetrics>,
    gesture_settings: Cell<GestureSettings>,
    metrics_listener: RefCell<Option<MetricsListener>>,
    presented: Cell<u32>,
    last_scene: RefCell<Option<Arc<Scene>>>,
    request_id: Option<RequestId>,
    initial_data: Option<String>,
    /// The window the view is in, until it closes; held weakly, since the window holds the view.
    window: RefCell<Option<Weak<dyn HostWindow>>>,
    /// What the view asks of the system, recorded.
    host: TestViewHost,
    /// The host the app gave a view it added, which takes what the view asks of the system in
    /// place of `host`.
    app_host: Option<Rc<dyn ViewHost>>,
}

impl TestView {
    /// View [`IMPLICIT_VIEW_ID`] at pixel ratio 1, `width` by `height` physical pixels.
    pub fn new(width: f64, height: f64) -> TestView {
        TestView::with_pixel_ratio(width, height, 1.0)
    }

    /// View [`IMPLICIT_VIEW_ID`] at `device_pixel_ratio`, `width` by `height` physical pixels.
    pub fn with_pixel_ratio(width: f64, height: f64, device_pixel_ratio: f64) -> TestView {
        let metrics = ViewMetrics {
            physical_size: [width, height],
            physical_constraints: ViewConstraints::tight(width, height),
            device_pixel_ratio,
            ..ViewMetrics::default()
        };
        TestView {
            id: IMPLICIT_VIEW_ID,
            initial_metrics: metrics,
            metrics: Cell::new(metrics),
            gesture_settings: Cell::new(GestureSettings::default()),
            metrics_listener: RefCell::new(None),
            presented: Cell::new(0),
            last_scene: RefCell::new(None),
            request_id: None,
            initial_data: None,
            window: RefCell::new(None),
            host: TestViewHost::default(),
            app_host: None,
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

    /// The same view asking what it asks of the system of `host`, as a view the app adds on a
    /// layer does; its own [`test_host`](Self::test_host) hears nothing then.
    pub fn with_host(mut self, host: Rc<dyn ViewHost>) -> TestView {
        self.app_host = Some(host);
        self
    }

    /// The same view on `screen`, as its metrics say.
    pub fn on_screen(mut self, screen: ScreenId) -> TestView {
        self.initial_metrics.screen = Some(screen);
        self.metrics.set(self.initial_metrics);
        self
    }

    /// The same view as one an embedder gave with `initial_data`.
    pub fn with_initial_data(mut self, initial_data: impl Into<String>) -> TestView {
        self.initial_data = Some(initial_data.into());
        self
    }

    /// Makes `listener` the one that hears of each change to the view's metrics, in place of the
    /// one before: what runs the app installs it, as a host hands a view's changes to its client.
    pub fn set_metrics_listener(&self, listener: MetricsListener) {
        *self.metrics_listener.borrow_mut() = Some(listener);
    }

    /// Dart's `tester.view.physicalSize = size`, which also makes the physical constraints tight
    /// to it.
    pub fn set_physical_size(&self, size: Size) {
        self.change_metrics(|metrics| {
            metrics.physical_size = [size.width(), size.height()];
            metrics.physical_constraints = ViewConstraints::tight(size.width(), size.height());
        });
    }

    /// Dart's `tester.view.resetPhysicalSize()`, which resets the physical constraints too.
    pub fn reset_physical_size(&self) {
        let initial = self.initial_metrics;
        self.change_metrics(|metrics| {
            metrics.physical_size = initial.physical_size;
            metrics.physical_constraints = initial.physical_constraints;
        });
    }

    /// Dart's `tester.view.physicalConstraints = constraints`.
    pub fn set_physical_constraints(&self, constraints: ViewConstraints) {
        self.change_metrics(|metrics| metrics.physical_constraints = constraints);
    }

    /// Dart's `tester.view.resetPhysicalConstraints()`.
    pub fn reset_physical_constraints(&self) {
        let initial = self.initial_metrics.physical_constraints;
        self.change_metrics(|metrics| metrics.physical_constraints = initial);
    }

    /// Dart's `tester.view.devicePixelRatio = ratio`.
    pub fn set_device_pixel_ratio(&self, ratio: f64) {
        self.change_metrics(|metrics| metrics.device_pixel_ratio = ratio);
    }

    /// Dart's `tester.view.resetDevicePixelRatio()`.
    pub fn reset_device_pixel_ratio(&self) {
        let initial = self.initial_metrics.device_pixel_ratio;
        self.change_metrics(|metrics| metrics.device_pixel_ratio = initial);
    }

    /// Dart's `tester.view.padding = padding`, in physical pixels.
    pub fn set_padding(&self, padding: ViewPadding) {
        self.change_metrics(|metrics| metrics.padding = padding);
    }

    /// Dart's `tester.view.resetPadding()`.
    pub fn reset_padding(&self) {
        let initial = self.initial_metrics.padding;
        self.change_metrics(|metrics| metrics.padding = initial);
    }

    /// Dart's `tester.view.viewPadding = padding`, in physical pixels.
    pub fn set_view_padding(&self, padding: ViewPadding) {
        self.change_metrics(|metrics| metrics.view_padding = padding);
    }

    /// Dart's `tester.view.resetViewPadding()`.
    pub fn reset_view_padding(&self) {
        let initial = self.initial_metrics.view_padding;
        self.change_metrics(|metrics| metrics.view_padding = initial);
    }

    /// Dart's `tester.view.viewInsets = insets`, in physical pixels.
    pub fn set_view_insets(&self, insets: ViewPadding) {
        self.change_metrics(|metrics| metrics.view_insets = insets);
    }

    /// Dart's `tester.view.resetViewInsets()`.
    pub fn reset_view_insets(&self) {
        let initial = self.initial_metrics.view_insets;
        self.change_metrics(|metrics| metrics.view_insets = initial);
    }

    /// Dart's `tester.view.systemGestureInsets = insets`, in physical pixels.
    pub fn set_system_gesture_insets(&self, insets: ViewPadding) {
        self.change_metrics(|metrics| metrics.system_gesture_insets = insets);
    }

    /// Dart's `tester.view.resetSystemGestureInsets()`.
    pub fn reset_system_gesture_insets(&self) {
        let initial = self.initial_metrics.system_gesture_insets;
        self.change_metrics(|metrics| metrics.system_gesture_insets = initial);
    }

    /// Dart's `tester.view.gestureSettings = settings`.
    pub fn set_gesture_settings(&self, settings: GestureSettings) {
        self.gesture_settings.set(settings);
        self.notify_metrics_changed();
    }

    /// Dart's `tester.view.resetGestureSettings()`.
    pub fn reset_gesture_settings(&self) {
        self.set_gesture_settings(GestureSettings::default());
    }

    /// Dart's `tester.view.reset()`: every metric back to what the view was made with, told as
    /// one change.
    pub fn reset(&self) {
        self.gesture_settings.set(GestureSettings::default());
        let initial = self.initial_metrics;
        self.change_metrics(|metrics| *metrics = initial);
    }

    /// How many pictures have been presented.
    pub fn presented(&self) -> u32 {
        self.presented.get()
    }

    /// The scene presented last, the frame's layer tree, which `waft_embedder::slices` turns
    /// into what it draws and the composition layers it places.
    pub fn last_scene(&self) -> Option<Arc<Scene>> {
        self.last_scene.borrow().clone()
    }

    /// The view's host, which records the cursors shown over the view, the focus requested for it,
    /// what its text input was asked, what the framework said of handling Back in it and the bars'
    /// styles set over it.
    pub fn test_host(&self) -> &TestViewHost {
        &self.host
    }

    /// The view is in `window` from now on, or in none.
    pub(crate) fn set_window(&self, window: Option<Weak<dyn HostWindow>>) {
        *self.window.borrow_mut() = window;
    }

    /// Applies `change` to the metrics, then tells the listener.
    fn change_metrics(&self, change: impl FnOnce(&mut ViewMetrics)) {
        let mut metrics = self.metrics.get();
        change(&mut metrics);
        self.metrics.set(metrics);
        self.notify_metrics_changed();
    }

    /// Tells the listener, if one is installed, that the metrics changed; outside its borrow, so
    /// the listener may install another.
    fn notify_metrics_changed(&self) {
        let listener = self.metrics_listener.borrow().clone();
        if let Some(listener) = listener {
            listener(self.id);
        }
    }
}

impl View for TestView {
    fn id(&self) -> ViewId {
        self.id
    }

    fn metrics(&self) -> ViewMetrics {
        self.metrics.get()
    }

    fn gesture_settings(&self) -> GestureSettings {
        self.gesture_settings.get()
    }

    fn present(&self, scene: Arc<Scene>) {
        self.presented.set(self.presented.get() + 1);
        *self.last_scene.borrow_mut() = Some(scene);
    }

    fn host(&self) -> &dyn ViewHost {
        self.app_host.as_deref().unwrap_or(&self.host)
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
