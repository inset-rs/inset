//! The embedder's [`View`]: a place the framework draws, as it sees it — its identity, its metrics,
//! its host, which takes what the view asks of the system (`ViewHost`), and the scenes it presents:
//! their slices drawn by its rasterizer (`rasterizer.rs`) into the layer the view shows into
//! (`os`'s `ViewLayer`), with the composition layers they place beneath the drawing.
//!
//! A view is controlled as Flutter's embedder controls a view, whoever holds it: the main loop adds
//! it on a layer with its metrics ([`WinitView::add`], `AddView`), gives it new metrics
//! ([`WinitView::set_metrics`], `SendWindowMetricsEvent`) and removes it from its layer
//! ([`WinitView::remove`], `RemoveView`), for its holder (`main_loop/views.rs`). The holder places
//! the layer itself, through a hold of its own on it. A window's code is the holder of its window's
//! view: the view is added on a layer the window places in itself and keeps filling (`windows.rs`),
//! takes the window's metrics as the window resizes or moves to another screen, and is removed as
//! the window closes.
//!
//! A view is made by its holder's code with its host, at no size, on no layer, and is added on
//! one: the screen view at winit's first call, after the app has started, as Flutter's engine has
//! it before its window; any other view as soon as it is made. A view on no layer, and one without
//! a surface, draws nothing; the system gives the views their surfaces and takes them away
//! (`views.rs`).
//!
//! The registry of windows owns each window (`windows.rs`); the framework and the app may hold
//! a view for a while after its window closes. The invariant: a view holds its window only through
//! its layer, where that is the window, and weakly through its host's text input and the app's
//! handle on the window, and lets go of the layer and the handle as it is removed (`remove`), so
//! that closing a window and removing its view frees it.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Instant;

use inset_embedder::{RequestId, Scene, View, ViewHost, ViewId, ViewMetrics, WindowRef};

use crate::gpu::LazyGpu;
use crate::host_window::WinitWindow;
use crate::os::ViewLayer;
use crate::rasterizer::{PresentingInTransaction, Rasterizer};

/// Stable view identity, the metrics its holder gave it, and what presents its scenes.
pub(crate) struct WinitView {
    id: ViewId,
    metrics: Cell<ViewMetrics>,
    /// The view's layer, the surface made from it, and the scenes drawn there.
    rasterizer: RefCell<Rasterizer>,
    /// What the view asks of the system, which its holder gave it.
    host: Rc<dyn ViewHost>,
    /// The request the view arrived for; `None` for the screen view, which the system gave.
    request_id: Option<RequestId>,
    initial_data: Option<String>,
    /// The app's handle on the view's window, while it is open; none for the screen view, whose
    /// window the app may not manage.
    window_handle: RefCell<Weak<WinitWindow>>,
}

impl WinitView {
    /// A view on no layer yet, at no size, asking what it asks of the system of `host`, and
    /// drawing with `gpu`'s device once it is on a layer and has a surface. A view whose window
    /// sees through gets surfaces whose alpha the compositor honours, cleared to nothing, so what
    /// the app leaves unpainted shows what is behind the window.
    pub(crate) fn new(
        id: ViewId,
        gpu: &LazyGpu,
        sees_through: bool,
        host: Rc<dyn ViewHost>,
    ) -> WinitView {
        WinitView {
            id,
            metrics: Cell::new(ViewMetrics::default()),
            rasterizer: RefCell::new(Rasterizer::new(gpu, sees_through)),
            host,
            request_id: None,
            initial_data: None,
            window_handle: RefCell::new(Weak::new()),
        }
    }

    /// The same view as the one request `request_id` asked for, carrying `initial_data`.
    pub(crate) fn with_request(
        mut self,
        request_id: RequestId,
        initial_data: Option<String>,
    ) -> WinitView {
        self.request_id = Some(request_id);
        self.initial_data = initial_data;
        self
    }

    /// Flutter's `AddView`: the view draws into `layer` from now on, at `metrics`. Its surface
    /// comes when the system gives the views theirs.
    pub(crate) fn add(&self, layer: ViewLayer, metrics: ViewMetrics) {
        self.metrics.set(metrics);
        self.rasterizer.borrow_mut().draw_into(
            layer,
            physical_size_of(&metrics),
            metrics.device_pixel_ratio,
        );
    }

    /// Flutter's `SendWindowMetricsEvent`: the view takes `metrics`, and fits its drawing to
    /// their scale and size. Answers whether they changed: the same metrics again are nothing to
    /// lay out for.
    pub(crate) fn set_metrics(&self, metrics: ViewMetrics) -> bool {
        let previous = self.metrics.replace(metrics);
        if previous == metrics {
            return false;
        }
        let mut rasterizer = self.rasterizer.borrow_mut();
        if previous.device_pixel_ratio != metrics.device_pixel_ratio {
            rasterizer.set_scale(metrics.device_pixel_ratio);
        }
        if previous.physical_size != metrics.physical_size {
            rasterizer.resized(physical_size_of(&metrics));
        }
        true
    }

    /// Flutter's `RemoveView`: the view lets go of its layer and the surface made from it, and of
    /// the app's handle on its window, while the framework may still hold the view, which is in no
    /// window from now on: a scene presented from now on is drawn nowhere.
    pub(crate) fn remove(&self) {
        self.rasterizer.borrow_mut().remove_layer();
        *self.window_handle.borrow_mut() = Weak::new();
    }

    /// Draws the last scene again if the surface does not show it: an owed frame, paid
    /// without the framework.
    pub(crate) fn draw_last_scene(&self) {
        self.rasterizer.borrow_mut().draw_last_scene();
    }

    /// Whether the next vsync should draw the owed scene again.
    pub(crate) fn retries_at_vsync(&self) -> bool {
        self.rasterizer.borrow().retries_at_vsync()
    }

    /// Draws the owed scene again, at a vsync.
    pub(crate) fn retry_owed_scene(&self) {
        self.rasterizer.borrow_mut().retry_owed_scene();
    }

    /// The picture the view draws now reaches the screen at `display_time`.
    pub(crate) fn set_display_time(&self, display_time: Instant) {
        self.rasterizer.borrow_mut().set_display_time(display_time);
    }

    /// Whether the view draws a video, whose new frames it takes at each vsync.
    pub(crate) fn shows_video(&self) -> bool {
        self.rasterizer.borrow().shows_video()
    }

    /// Draws the last scene again with its videos' new frames, at a vsync, where any came.
    pub(crate) fn draw_new_video_frames(&self) {
        self.rasterizer.borrow_mut().draw_new_video_frames();
    }

    /// Presents through the window's own transaction until the answer is dropped: for the
    /// frame a resize draws.
    pub(crate) fn present_in_transaction(&self) -> PresentingInTransaction<'_> {
        PresentingInTransaction::begin(&self.rasterizer)
    }

    /// `handle` is the app's handle on the view's window from now on: the view answers it as its
    /// window until it is removed.
    pub(crate) fn set_window_handle(&self, handle: &Rc<WinitWindow>) {
        *self.window_handle.borrow_mut() = Rc::downgrade(handle);
    }

    /// The system gives the view a surface, made from its layer: the view draws on it from now
    /// on.
    pub(crate) fn give_surface(&self) {
        self.rasterizer.borrow_mut().give_surface();
    }

    /// The system took the view's surface away: the view keeps its scene and owes it until
    /// it is given a surface again.
    pub(crate) fn take_surface(&self) {
        self.rasterizer.borrow_mut().take_surface();
    }
}

/// The size `metrics` give, in the whole pixels a surface is made of.
fn physical_size_of(metrics: &ViewMetrics) -> [u32; 2] {
    let [width, height] = metrics.physical_size;
    [width as u32, height as u32]
}

impl View for WinitView {
    fn id(&self) -> ViewId {
        self.id
    }

    fn metrics(&self) -> ViewMetrics {
        self.metrics.get()
    }

    fn present(&self, scene: Arc<Scene>) {
        let mut rasterizer = self.rasterizer.borrow_mut();
        if rasterizer.shows(&scene) {
            return;
        }
        let slices = rasterizer.slices_of(&scene);
        rasterizer.place_composition_layers(slices.composition_layers(), self.id);
        rasterizer.draw(&scene, &slices);
    }

    fn host(&self) -> &dyn ViewHost {
        &*self.host
    }

    fn initial_data(&self) -> Option<String> {
        self.initial_data.clone()
    }

    fn request_id(&self) -> Option<RequestId> {
        self.request_id
    }

    fn window(&self) -> Option<WindowRef> {
        let handle = self.window_handle.borrow().upgrade()?;
        Some(handle as WindowRef)
    }
}
