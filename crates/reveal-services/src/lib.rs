//! Flutter counterpart: `packages/flutter/lib/src/services`.
//!
//! Only the mouse cursor and mouse tracking annotation are here. Platform channels, the
//! keyboard, text input, the clipboard, and system chrome wait.

mod mouse_cursor;
mod mouse_tracking;

pub use mouse_cursor::*;
pub use mouse_tracking::*;
