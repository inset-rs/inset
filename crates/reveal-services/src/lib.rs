//! Flutter counterpart: `packages/flutter/lib/src/services`.
//!
//! Only the hardware keyboard, haptic feedback, the mouse cursor, the mouse tracking
//! annotation, state restoration, the application switcher description, the system overlay
//! style, `TextSelection`, and `SelectionChangedCause` are here. Platform channels, the raw
//! keyboard, the rest of text input, the clipboard, and the rest of system chrome wait.
#![feature(arbitrary_self_types)]

mod haptic_feedback;
mod hardware_keyboard;
mod keyboard_key;
mod mouse_cursor;
mod mouse_tracking;
mod restoration;
mod system_chrome;
mod text_editing;
mod text_input;

pub use haptic_feedback::*;
pub use hardware_keyboard::*;
pub use keyboard_key::*;
pub use mouse_cursor::*;
pub use mouse_tracking::*;
pub use restoration::*;
pub use system_chrome::*;
pub use text_editing::*;
pub use text_input::*;
