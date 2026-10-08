//! What AppKit answers about the pointer outside the events winit reports: the buttons held
//! down, and where the pointer is in a window.

use objc2_app_kit::NSEvent;
use winit::window::Window;

use super::{AppKit, ns_view};
use crate::os::PointerQueries;

impl PointerQueries for AppKit {
    /// The same bits AppKit reports. A native menu swallows the release of the button that
    /// opened it, so the embedder reconciles its own record with this once the menu is gone.
    fn pressed_buttons() -> Option<i64> {
        Some(NSEvent::pressedMouseButtons() as i64)
    }

    /// winit's entry event carries no position; AppKit's answer is current.
    fn pointer_position_in(window: &Window) -> Option<[f64; 2]> {
        let view = ns_view(window)?;
        let ns_window = view.window()?;
        let in_window = ns_window.convertPointFromScreen(NSEvent::mouseLocation());
        let in_view = view.convertPoint_fromView(in_window, None);
        let y = if view.isFlipped() {
            in_view.y
        } else {
            view.bounds().size.height - in_view.y
        };
        let scale = ns_window.backingScaleFactor();
        Some([in_view.x * scale, y * scale])
    }
}
