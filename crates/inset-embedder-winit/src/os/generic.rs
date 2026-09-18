//! A system winit covers by itself: nothing to add, and nothing to ask. iOS is one, with
//! the two things its window reports differently: its size and its safe area.

use std::time::Duration;

use inset_embedder::{PopupMenuEntry, Rect, ViewPadding, WindowConfig};
use winit::window::{Window, WindowAttributes};

/// Whether a resize draws a frame before it returns, for a system that commits window
/// geometry on its own before the next redraw.
pub(crate) const FRAME_ON_RESIZE: bool = false;

/// Whether the window is the screen, which the system sizes: a phone's. winit on iOS
/// would size the window to a request instead.
pub(crate) const WINDOW_IS_THE_SCREEN: bool = cfg!(target_os = "ios");

/// winit's attributes carry everything the configuration asks for.
pub(crate) fn extend_attributes(
    attributes: WindowAttributes,
    _config: &WindowConfig,
) -> WindowAttributes {
    attributes
}

pub(crate) fn configure(_window: &Window, _config: &WindowConfig) {}

/// The system cannot be asked which mouse buttons are down.
pub(crate) fn pressed_buttons() -> Option<i64> {
    None
}

/// The system cannot be asked where the pointer is.
pub(crate) fn pointer_position(_window: &Window) -> Option<[f64; 2]> {
    None
}

/// The system animates no frames; the caller sets the frame at once.
pub(crate) fn set_frame(_window: &Window, _frame: Rect) -> bool {
    false
}

pub(crate) fn animate_frame(_window: &Window, _frame: Rect, _duration: Duration) -> bool {
    false
}

/// The system has no popup menu.
pub(crate) fn popup_menu(_entries: &[PopupMenuEntry]) -> Option<usize> {
    None
}

/// The pixels valo presents into. On iOS winit's `inner_size` is the safe area while the
/// layer covers the whole window, which `outer_size` reports.
pub(crate) fn surface_size(window: &Window) -> [u32; 2] {
    let size = if cfg!(target_os = "ios") {
        window.outer_size()
    } else {
        window.inner_size()
    };
    [size.width, size.height]
}

/// The safe area as Flutter reports it: the whole view is the size, the status bar and
/// home indicator are padding. Zero on desktops.
pub(crate) fn view_padding(window: &Window, [width, height]: [f64; 2]) -> ViewPadding {
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

/// The system presents what the app draws when it draws it; no transaction to join.
pub(crate) fn present_in_transaction(_surface: &mut valo::Surface, _wanted: bool) {}

/// The system has no refresh signal to offer; the host paces frames by a timer instead.
pub(crate) struct Vsync;

impl Vsync {
    pub(crate) fn start(_window: &Window, _on_tick: Box<dyn Fn()>) -> Option<Vsync> {
        None
    }

    pub(crate) fn set_paused(&self, _paused: bool) {}
}

/// Runs `work` on a thread of its own: what a system without a work queue of its own gets.
pub(crate) fn run_off_main(work: Box<dyn FnOnce() + Send>) {
    std::thread::Builder::new()
        .name("inset-worker".into())
        .spawn(work)
        .expect("a thread for work off the main thread");
}
