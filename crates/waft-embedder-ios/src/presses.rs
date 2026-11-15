//! A `UIPress` as the key responder takes it (`keys.rs`): its key's HID usage, what it types with
//! and without the modifiers, and when.

use std::time::Duration;

use objc2::MainThreadMarker;
use objc2_ui_kit::UIPress;
use waft_embedder::KeyData;

use crate::keys::{KeyResponder, Press};

/// Where a press is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PressPhase {
    Began,
    /// Ended or cancelled: the key is up either way.
    Ended,
}

/// What `press` in `phase` hands the framework: `None` for a press of no key, such as a remote's,
/// `Some(None)` for a key the responder answers itself, and the key's data otherwise.
pub(crate) fn key_data_of(
    press: &UIPress,
    phase: PressPhase,
    keys: &mut KeyResponder,
    main_thread: MainThreadMarker,
) -> Option<Option<KeyData>> {
    let key = press.key(main_thread)?;
    let press = Press {
        key_code: u32::try_from(key.keyCode().0).unwrap_or(u32::MAX),
        characters: key.characters().to_string(),
        characters_ignoring_modifiers: key.charactersIgnoringModifiers().to_string(),
        time_stamp: Duration::from_secs_f64(press.timestamp().max(0.0)),
    };
    Some(match phase {
        PressPhase::Began => keys.press_began(&press),
        PressPhase::Ended => keys.press_ended(&press),
    })
}
