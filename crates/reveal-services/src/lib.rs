//! Flutter counterpart: `packages/flutter/lib/src/services`.
//!
//! Only the hardware keyboard, haptic feedback, the mouse cursor, the mouse tracking
//! annotation, state restoration, the application switcher description, the system overlay
//! style, `TextInput` / `TextInputConnection` / `TextInputClient` / `TextSelectionDelegate`,
//! text layout metric statics, the clipboard, text input formatters, `SelectionChangedCause`,
//! autofill (`AutofillHints` / `AutofillClient` / `AutofillScope`), keyboard-inserted
//! content, spell check, live text, process text, `UndoManager` / `UndoManagerClient`,
//! `SystemContextMenuController`, and re-exports of the text-input value types are here. Platform channels, the raw
//! keyboard, and the rest of system chrome wait.
#![feature(arbitrary_self_types)]

mod autofill;
mod clipboard;
mod haptic_feedback;
mod hardware_keyboard;
mod keyboard_inserted_content;
mod keyboard_key;
mod live_text;
mod mouse_cursor;
mod mouse_tracking;
mod process_text;
mod restoration;
mod spell_check;
mod system_chrome;
mod system_context_menu;
mod text_editing;
mod text_formatter;
mod text_input;
mod text_layout_metrics;
mod undo_manager;

pub use autofill::*;
pub use clipboard::*;
pub use haptic_feedback::*;
pub use hardware_keyboard::*;
pub use keyboard_inserted_content::*;
pub use keyboard_key::*;
pub use live_text::*;
pub use mouse_cursor::*;
pub use mouse_tracking::*;
pub use process_text::*;
pub use restoration::*;
pub use spell_check::*;
pub use system_chrome::*;
pub use system_context_menu::*;
pub use text_editing::*;
pub use text_formatter::*;
pub use text_input::*;
pub use text_layout_metrics::*;
pub use undo_manager::*;
