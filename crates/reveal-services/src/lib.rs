//! Flutter counterpart: `packages/flutter/lib/src/services`.
//!
//! Only the mouse cursor, the mouse tracking annotation, and `TextSelection` are here.
//! Platform channels, the keyboard, text input, the clipboard, and system chrome wait.

mod mouse_cursor;
mod mouse_tracking;
mod text_editing;

pub use mouse_cursor::*;
pub use mouse_tracking::*;
pub use text_editing::*;
