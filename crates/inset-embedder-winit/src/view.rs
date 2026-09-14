//! The host's [`View`]: one window as the framework sees it — its identity, its metrics,
//! its text input, and the picture it hands over.
//!
//! Presenting to a window's surface, and what to do when the surface refuses.
//!
//! On macOS wgpu asks AppKit whether the window is visible, its occlusion state, before
//! it asks the layer for a drawable, and refuses with `Occluded` while it is not (wgpu
//! issue 8309). A window just ordered on screen counts as occluded until AppKit reports
//! otherwise, which it does through `NSWindowDidChangeOcclusionState`, winit's
//! `Occluded(false)`: that event is when a refused picture is presented, as wgpu's own
//! guidance says. The layer's other refusals are a drawable timeout, which the next
//! refresh clears, and a lost surface. Flutter's macOS embedder never meets any of this,
//! as it composites IOSurfaces through layers Core Animation retains; gpui drops a refused
//! frame and lets its display link draw the next. This host keeps the framework's last
//! picture instead, since the framework draws only when asked, and pays it when the
//! system says the window can take it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use inset_embedder::{
    Matrix4, Offset, Picture, Rect, Size, TextEditingValue, TextInputConfiguration, TextInputHost,
    View, ViewConstraints, ViewId, ViewMetrics, ViewPadding, transform3,
};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::{ImePurpose, Window};

use crate::surface::WinitSurface;
use crate::text_input::ActiveTextInput;
use crate::windows::WinitWindow;

/// Stable view identity, current winit geometry, and the valo present line.
pub(crate) struct WinitView {
    pub(crate) id: ViewId,
    pub(crate) metrics: Cell<ViewMetrics>,
    pub(crate) surface: Arc<Mutex<WinitSurface>>,
    /// The window this view draws into, for retrying a present the surface refused.
    pub(crate) window: Arc<Window>,
    pub(crate) editing_state: RefCell<TextEditingValue>,
    /// Set from `TextInputHost::start` to `TextInputHost::stop`; an unhandled key types into it.
    pub(crate) text_input: Cell<Option<ActiveTextInput>>,
    pub(crate) transform: RefCell<Option<Matrix4>>,
    /// What a frame starts from: white, or nothing for a window that shows what is behind it.
    pub(crate) clear: valo::Color,
    /// The last picture the framework handed over, kept to present again when the window
    /// is seen again, and to tell a picture equal to it from one worth drawing.
    pub(crate) latest: RefCell<Option<Arc<Picture>>>,
    /// Whether the latest picture is on the surface: not while the window is hidden or
    /// occluded, when the surface gives no texture, nor after the surface was resized.
    pub(crate) on_screen: Cell<bool>,
    /// Why the surface last refused the picture, while it is still not on the surface:
    /// what brings it there is the system's word for an occluded window and the next
    /// refresh after a timeout.
    pub(crate) refused: Cell<Option<valo::Refused>>,
    /// Refreshes tried since a timeout; a surface that keeps timing out is left to the
    /// system's next redraw request rather than asked at every refresh for ever.
    pub(crate) retries: Cell<u8>,
    /// Set around the frame a resize draws inline: that one presents through the window's
    /// transaction.
    pub(crate) resizing: Cell<bool>,
}

/// How many refreshes a picture is retried after the surface timed out.
const TIMEOUT_RETRIES: u8 = 8;

impl WinitView {
    /// Renders and presents `picture`, or records why the surface refused it, so that the
    /// picture is presented when the refusal ends.
    fn draw(&self, picture: &Picture) {
        let drawn = self.surface.lock().expect("surface lock").draw(
            picture,
            self.clear,
            self.resizing.get(),
        );
        match drawn {
            Ok(()) => {
                self.on_screen.set(true);
                self.refused.set(None);
                self.retries.set(0);
            }
            Err(refused) => {
                self.on_screen.set(false);
                self.refused.set(Some(refused));
            }
        }
    }

    /// Presents the latest picture again if it is not on the surface: an owed frame, paid
    /// without the framework.
    pub(crate) fn represent(&self) {
        if self.on_screen.get() {
            return;
        }
        let latest = self.latest.borrow().clone();
        if let Some(latest) = latest {
            self.draw(&latest);
        }
    }

    /// Whether the latest picture is owed to the surface: one was handed over, and what
    /// is on the surface is not it.
    fn owes_a_frame(&self) -> bool {
        !self.on_screen.get() && self.latest.borrow().is_some()
    }

    /// Whether the next refresh should try the owed picture again: the surface timed
    /// out, and not too many refreshes ago. An occluded window waits for the system's
    /// word instead, and a lost surface for its remaking.
    pub(crate) fn retries_at_refresh(&self) -> bool {
        self.owes_a_frame()
            && self.refused.get() == Some(valo::Refused::Timeout)
            && self.retries.get() < TIMEOUT_RETRIES
    }

    /// One more refresh spent on the owed picture.
    pub(crate) fn retry(&self) {
        self.retries.set(self.retries.get().saturating_add(1));
        self.represent();
    }

    /// The surface was reconfigured: whatever it showed is gone, and whatever it refused
    /// is forgotten with it.
    pub(crate) fn resized(&self, size: [u32; 2]) {
        self.surface
            .lock()
            .expect("surface lock")
            .surface
            .resize(size);
        self.on_screen.set(false);
        self.refused.set(None);
        self.retries.set(0);
    }
}

impl View for WinitView {
    fn id(&self) -> ViewId {
        self.id
    }

    fn metrics(&self) -> ViewMetrics {
        self.metrics.get()
    }

    fn present(&self, picture: Arc<Picture>) {
        let unchanged = self.on_screen.get()
            && self
                .latest
                .borrow()
                .as_ref()
                .is_some_and(|latest| **latest == *picture);
        if unchanged {
            return;
        }
        *self.latest.borrow_mut() = Some(Arc::clone(&picture));
        self.draw(&picture);
    }

    fn text_input(&self) -> Option<&dyn TextInputHost> {
        Some(self)
    }
}

impl TextInputHost for WinitView {
    fn start(&self, configuration: &TextInputConfiguration) {
        self.text_input
            .set(Some(ActiveTextInput::new(configuration)));
        self.window.set_ime_allowed(true);
        self.window.set_ime_purpose(if configuration.obscure_text {
            ImePurpose::Password
        } else {
            ImePurpose::Normal
        });
    }

    fn stop(&self) {
        self.text_input.set(None);
        self.window.set_ime_allowed(false);
    }

    fn set_editing_state(&self, value: &TextEditingValue) {
        *self.editing_state.borrow_mut() = value.clone();
    }

    fn set_composing_rect(&self, rect: Rect) {
        self.set_ime_area(rect);
    }

    fn set_caret_rect(&self, rect: Rect) {
        self.set_ime_area(rect);
    }

    fn set_client_geometry(&self, _size: Size, transform: &Matrix4) {
        *self.transform.borrow_mut() = Some(*transform);
    }
}

impl WinitView {
    fn set_ime_area(&self, rect: Rect) {
        let origin = match *self.transform.borrow() {
            Some(transform) => transform3(transform, Offset::new(rect.left, rect.top)),
            None => Offset::new(rect.left, rect.top),
        };
        let scale = self.metrics.get().device_pixel_ratio;
        self.window.set_ime_cursor_area(
            PhysicalPosition::new(origin.dx() * scale, origin.dy() * scale),
            PhysicalSize::new(rect.width() * scale, rect.height() * scale),
        );
    }
}

pub(crate) struct HostedView {
    pub(crate) window: Arc<Window>,
    pub(crate) view: Rc<WinitView>,
    /// The app's handle on a window it created; the implicit window has none.
    pub(crate) handle: Option<Rc<WinitWindow>>,
}

pub(crate) fn window_metrics(window: &Window) -> ViewMetrics {
    let [width, height] = surface_size(window).map(f64::from);
    let padding = safe_area_padding(window, width, height);
    ViewMetrics {
        physical_size: [width, height],
        physical_constraints: ViewConstraints::tight(width, height),
        device_pixel_ratio: window.scale_factor(),
        padding,
        view_padding: padding,
        view_insets: ViewPadding::ZERO,
    }
}

/// The pixels valo presents into. On iOS winit's `inner_size` is the safe
/// area while the layer covers the whole window, which `outer_size` reports.
pub(crate) fn surface_size(window: &Window) -> [u32; 2] {
    let size = if cfg!(target_os = "ios") {
        window.outer_size()
    } else {
        window.inner_size()
    };
    [size.width, size.height]
}

/// The safe area as Flutter reports it: the whole view is the size, the status
/// bar and home indicator are padding. Zero on desktops.
fn safe_area_padding(window: &Window, width: f64, height: f64) -> ViewPadding {
    if !cfg!(target_os = "ios") {
        return ViewPadding::ZERO;
    }
    let safe = window.inner_size();
    let origin = window.inner_position().unwrap_or_default();
    let left = f64::from(origin.x);
    let top = f64::from(origin.y);
    ViewPadding {
        left,
        top,
        right: (width - f64::from(safe.width) - left).max(0.0),
        bottom: (height - f64::from(safe.height) - top).max(0.0),
    }
}
