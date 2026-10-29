//! The embedder's [`View`]: a place the framework draws, as it sees it — its identity, its metrics,
//! its text input, and the scenes it presents: drawn by its rasterizer (`rasterizer.rs`), with the
//! native surfaces they place beneath the drawing (`os`'s `WindowSurfaces`).
//!
//! A view is made without a window, at no size, and is given one: the screen view at winit's
//! first call, after the app has started, as Flutter's engine has it before its window; any
//! other view as soon as it is made. A view without a window, and one whose window has no
//! surface, draws nothing; the system gives a window its surface and takes it away
//! (`main_loop/`).
//!
//! The registry of views owns each window (`windows.rs`); the framework and the app may hold
//! a view for a while after its window closes. The invariant a later editor must keep: a view
//! holds its window only through its rasterizer's surface and its native surfaces, and weakly
//! for its text input and the app's handle on the window, and lets go of them when the window
//! closes (`window_closed`), so that closing a window frees it.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

use inset_embedder::{
    RequestId, Scene, TextInputHost, View, ViewConstraints, ViewId, ViewMetrics, ViewPadding,
    WindowRef,
};
use winit::event::{Ime, KeyEvent};
use winit::keyboard::ModifiersState;
use winit::window::Window;

use crate::gpu::LazyGpu;
use crate::host_window::WinitWindow;
use crate::ime::ImeOutcome;
use crate::os::{ThisOs, ThisOsWindowSurfaces, WindowGeometry, WindowSurfaces};
use crate::rasterizer::{PresentingInTransaction, Rasterizer};
use crate::screens::screen_of;
use crate::text_input::{TextInput, Typed};

/// Stable view identity, current winit geometry, and what presents its scenes.
pub(crate) struct WinitView {
    id: ViewId,
    metrics: Cell<ViewMetrics>,
    rasterizer: RefCell<Rasterizer>,
    /// The native surfaces the window's scenes place beneath its content; `None` while the view
    /// has no window.
    native_surfaces: RefCell<Option<ThisOsWindowSurfaces>>,
    text_input: TextInput,
    /// The request the view arrived for; `None` for the screen view, which the system gave.
    request_id: Option<RequestId>,
    initial_data: Option<String>,
    /// The app's handle on the view's window, while it is open; none for the screen view, whose
    /// window the app may not manage.
    window_handle: RefCell<Weak<WinitWindow>>,
}

impl WinitView {
    /// A view with no window yet, at no size, drawing with `gpu`'s device once it has a window
    /// with a surface. A view whose window sees through gets surfaces whose alpha the compositor
    /// honours, cleared to nothing, so what the app leaves unpainted shows what is behind the
    /// window.
    pub(crate) fn new(id: ViewId, gpu: &LazyGpu, sees_through: bool) -> WinitView {
        WinitView {
            id,
            metrics: Cell::new(ViewMetrics::default()),
            native_surfaces: RefCell::new(None),
            rasterizer: RefCell::new(Rasterizer::new(gpu, sees_through)),
            text_input: TextInput::new(),
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

    /// `window` is the view's from now on: its native surfaces and text input are on it, and
    /// the view takes its metrics. Its surface comes when the system gives the windows theirs.
    pub(crate) fn attach_window(&self, window: &Arc<dyn Window>) {
        *self.native_surfaces.borrow_mut() = Some(ThisOsWindowSurfaces::new(Arc::clone(window)));
        self.text_input.attach(window);
        self.refresh_metrics(&**window);
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

    /// Presents through the window's own transaction until the answer is dropped: for the
    /// frame a resize draws.
    pub(crate) fn present_in_transaction(&self) -> PresentingInTransaction<'_> {
        PresentingInTransaction::begin(&self.rasterizer)
    }

    /// `handle` is the app's handle on the view's window from now on: the view answers it as its
    /// window until the window closes.
    pub(crate) fn set_window_handle(&self, handle: &Rc<WinitWindow>) {
        *self.window_handle.borrow_mut() = Rc::downgrade(handle);
    }

    /// The window closed: its surface and its native surfaces go, and with them the last hold
    /// on the window, while the framework may still hold the view, which is in no window from
    /// now on. A scene presented from now on is drawn nowhere.
    pub(crate) fn window_closed(&self) {
        self.native_surfaces.take();
        self.rasterizer.borrow_mut().window_closed();
        *self.window_handle.borrow_mut() = Weak::new();
    }

    /// Reads the window's metrics again; when they changed, fits the surface to the new size and
    /// answers it. `None` for the same geometry again: winit reports a resize for more than one
    /// of the system's notices.
    pub(crate) fn window_resized(&self, window: &dyn Window) -> Option<[u32; 2]> {
        if !self.refresh_metrics(window) {
            return None;
        }
        let [width, height] = self.metrics.get().physical_size;
        let size = [width as u32, height as u32];
        self.rasterizer.borrow_mut().resized(size);
        Some(size)
    }

    /// What a key the framework did not handle does to the view's active text input.
    pub(crate) fn type_key(&self, event: &KeyEvent, modifiers: ModifiersState) -> Typed {
        self.text_input.type_key(event, modifiers)
    }

    /// What an IME event makes of the editing value of the view's text input.
    pub(crate) fn take_ime(&self, ime: &Ime) -> ImeOutcome {
        self.text_input.take_ime(ime)
    }

    /// The system gives `window`, the view's, a surface: the view draws on it from now on.
    pub(crate) fn give_surface(&self, window: &Arc<dyn Window>) {
        self.rasterizer.borrow_mut().give_surface(window);
    }

    /// The system took the window's surface away: the view keeps its scene and owes it until
    /// it is given a surface again.
    pub(crate) fn take_surface(&self) {
        self.rasterizer.borrow_mut().take_surface();
    }

    /// Reads the window's metrics again and answers whether they changed. winit reports a
    /// resize for more than one of the system's notices, and some systems report none for
    /// a change of the safe area; the same geometry again is nothing to lay out for.
    pub(crate) fn refresh_metrics(&self, window: &dyn Window) -> bool {
        let metrics = read_metrics(window);
        if self.metrics.get() == metrics {
            return false;
        }
        self.metrics.set(metrics);
        true
    }

    /// Reads again which screen the window is on, and answers whether that changed: a window
    /// that moved to another screen has new metrics, though its surface stays as it is.
    pub(crate) fn refresh_screen(&self, window: &dyn Window) -> bool {
        let screen = screen_of(window);
        let mut metrics = self.metrics.get();
        if metrics.screen == screen {
            return false;
        }
        metrics.screen = screen;
        self.metrics.set(metrics);
        true
    }
}

/// The metrics of `window` as the framework takes them.
fn read_metrics(window: &dyn Window) -> ViewMetrics {
    let [width, height] = ThisOs::surface_size(window).map(f64::from);
    let padding = ThisOs::view_padding(window);
    ViewMetrics {
        physical_size: [width, height],
        physical_constraints: ViewConstraints::tight(width, height),
        device_pixel_ratio: window.scale_factor(),
        // No system winit serves reports a keyboard, so the safe area is the whole of
        // the padding and nothing covers it.
        padding,
        view_padding: padding,
        view_insets: ViewPadding::ZERO,
        screen: screen_of(window),
    }
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
        if let Some(native_surfaces) = &*self.native_surfaces.borrow() {
            native_surfaces.place(&scene.surfaces, self.id);
        }
        rasterizer.draw(&scene);
    }

    fn text_input(&self) -> Option<&dyn TextInputHost> {
        Some(&self.text_input)
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
