//! winit's key events as dart:ui [`KeyData`]: the physical and logical keys from Flutter's
//! tables (`keys.rs`), with the logical key each held key went down with.
//!
//! A key the framework declines is offered to the view's text input afterwards (`text_input.rs`)
//! — the order Flutter's macOS `FlutterKeyboardManager` keeps between the framework and its text
//! input plugin.

use std::collections::HashMap;
use std::time::Duration;

use inset_embedder::{KeyData, KeyEventDeviceType, KeyEventType};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::ModifiersState;

use crate::keys;

/// The keyboard as Flutter's `KeyData`: the modifiers held, and which logical key each
/// held physical key went down with.
pub(crate) struct Keyboard {
    modifiers: ModifiersState,
    pressing_records: PressingRecords,
}

impl Keyboard {
    pub(crate) fn new() -> Keyboard {
        Keyboard {
            modifiers: ModifiersState::default(),
            pressing_records: PressingRecords::default(),
        }
    }

    /// The modifiers held now.
    pub(crate) fn modifiers(&self) -> ModifiersState {
        self.modifiers
    }

    pub(crate) fn set_modifiers(&mut self, modifiers: ModifiersState) {
        self.modifiers = modifiers;
    }

    /// A winit key event as dart:ui [`KeyData`], or `None` for a key this embedder
    /// cannot name in Flutter's tables.
    pub(crate) fn key_data(
        &mut self,
        event: &KeyEvent,
        is_synthetic: bool,
        time_stamp: Duration,
    ) -> Option<KeyData> {
        let physical = keys::physical_key_usage(event.physical_key)?;
        let event_type = match (event.state, event.repeat) {
            (ElementState::Pressed, false) => KeyEventType::Down,
            (ElementState::Pressed, true) => KeyEventType::Repeat,
            (ElementState::Released, _) => KeyEventType::Up,
        };
        let logical = self.pressing_records.logical_key(
            event_type,
            physical,
            keys::logical_key_id(&event.logical_key, event.location),
        )?;
        let character = match event_type {
            KeyEventType::Up => None,
            KeyEventType::Down | KeyEventType::Repeat => keys::character_of(event.text.as_deref()),
        };
        Some(KeyData {
            time_stamp,
            event_type,
            device_type: KeyEventDeviceType::Keyboard,
            physical,
            logical,
            character,
            synthesized: is_synthetic,
        })
    }
}

/// The logical key each held physical key went down with, keyed by USB HID usage. Flutter's
/// embedders remember it, so a key's repeats and its up report the key its down did even when
/// the modifiers changed in between (`_pressingRecords` in the engine's `KeyboardConverter`).
#[derive(Default)]
struct PressingRecords(HashMap<u64, u64>);

impl PressingRecords {
    /// The logical key an event of `event_type` of the key `physical` reports, where winit
    /// names `logical` for it now; `None` for a key Flutter's tables cannot name.
    fn logical_key(
        &mut self,
        event_type: KeyEventType,
        physical: u64,
        logical: Option<u64>,
    ) -> Option<u64> {
        match event_type {
            KeyEventType::Down => {
                let logical = logical?;
                self.0.insert(physical, logical);
                Some(logical)
            }
            KeyEventType::Repeat => self.0.get(&physical).copied().or(logical),
            KeyEventType::Up => self.0.remove(&physical).or(logical),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGIT_1: u64 = 0x0007001e;
    const LOGICAL_1: u64 = 0x31;
    const LOGICAL_EXCLAMATION: u64 = 0x21;

    #[test]
    fn a_key_reports_the_logical_key_it_went_down_with_until_it_goes_up() {
        let mut records = PressingRecords::default();
        let down = records.logical_key(KeyEventType::Down, DIGIT_1, Some(LOGICAL_1));
        assert_eq!(down, Some(LOGICAL_1));
        let shift_since = Some(LOGICAL_EXCLAMATION);
        assert_eq!(
            records.logical_key(KeyEventType::Repeat, DIGIT_1, shift_since),
            Some(LOGICAL_1)
        );
        assert_eq!(
            records.logical_key(KeyEventType::Up, DIGIT_1, shift_since),
            Some(LOGICAL_1)
        );
        assert!(records.0.is_empty(), "the up ends the record");
    }

    #[test]
    fn a_key_never_seen_going_down_reports_the_logical_key_named_now() {
        let mut records = PressingRecords::default();
        assert_eq!(
            records.logical_key(KeyEventType::Up, DIGIT_1, Some(LOGICAL_EXCLAMATION)),
            Some(LOGICAL_EXCLAMATION)
        );
        assert_eq!(
            records.logical_key(KeyEventType::Repeat, DIGIT_1, None),
            None
        );
    }

    #[test]
    fn a_down_with_no_logical_key_is_not_reported_nor_recorded() {
        let mut records = PressingRecords::default();
        assert_eq!(records.logical_key(KeyEventType::Down, DIGIT_1, None), None);
        assert!(records.0.is_empty());
    }
}
