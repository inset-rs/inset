//! A system winit covers by itself: nothing to add, and nothing to ask.

use std::time::Duration;

use inset_embedder::{PopupMenuEntry, Rect, WindowConfig};
use winit::window::{Window, WindowAttributes};

/// Whether a resize draws a frame before it returns, for a system that commits window
/// geometry on its own before the next redraw.
pub(crate) const FRAME_ON_RESIZE: bool = false;

/// winit's attributes carry everything the configuration asks for.
pub(crate) fn extend_attributes(
    attributes: WindowAttributes,
    _config: &WindowConfig,
) -> WindowAttributes {
    attributes
}

pub(crate) fn configure(_window: &Window, _config: &WindowConfig) {}

pub(crate) fn prepare_surface(_surface: &mut valo::Surface) {}

/// The system cannot be asked which mouse buttons are down.
pub(crate) fn pressed_buttons() -> Option<i64> {
    None
}

/// The system cannot be asked where the pointer is.
pub(crate) fn pointer_position(_window: &Window) -> Option<[f64; 2]> {
    None
}

/// The system animates no frames; the caller sets the frame at once.
pub(crate) fn animate_frame(_window: &Window, _frame: Rect, _duration: Duration) -> bool {
    false
}

/// The system has no popup menu.
pub(crate) fn popup_menu(_entries: &[PopupMenuEntry]) -> Option<usize> {
    None
}
