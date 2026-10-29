//! What AppKit answers about the pointer outside the events winit reports: the buttons held
//! down.

use objc2_app_kit::NSEvent;

use super::AppKit;
use crate::os::PointerQueries;

impl PointerQueries for AppKit {
    /// The same bits AppKit reports. A native menu swallows the release of the button that
    /// opened it, so the embedder reconciles its own record with this once the menu is gone.
    fn pressed_buttons() -> Option<i64> {
        Some(NSEvent::pressedMouseButtons() as i64)
    }
}
