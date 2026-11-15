//! A hardware keyboard's presses as dart:ui [`KeyData`], as Flutter's iOS
//! `FlutterEmbedderKeyResponder` makes its key events: the physical key from the press's HID usage
//! (`KeyCodeMap.g.mm`'s `keyCodeToPhysicalKey`, the usage on the keyboard page), the logical key from
//! the special keys' names, the keys Flutter names by code, or the character the key types without
//! modifiers, and the logical key each held key went down with, kept until it goes up.
//!
//! Not ported: `synchronizeModifiers`, which sends the framework the modifier keys pressed or
//! released while the app did not hear them, and Caps Lock's synthesized tap.

use std::collections::HashMap;
use std::time::Duration;

use waft_embedder::{KeyData, KeyEventDeviceType, KeyEventType};

/// Mask for the 32-bit value portion of a key code.
const VALUE_MASK: u64 = 0x000ffffffff;
/// The plane of the keys that have a Unicode representation.
const UNICODE_PLANE: u64 = 0x00000000000;
/// The plane of the keys the iOS embedding names itself.
const IOS_PLANE: u64 = 0x01300000000;

/// What a press says of its key: UIKit's `UIPress` and its `UIKey`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Press {
    /// The key's HID usage on the keyboard page, `UIKey.keyCode`.
    pub(crate) key_code: u32,
    /// What the key types with the modifiers held, `UIKey.characters`.
    pub(crate) characters: String,
    /// What it types without them, `UIKey.charactersIgnoringModifiers`.
    pub(crate) characters_ignoring_modifiers: String,
    /// When it happened, `UIPress.timestamp`.
    pub(crate) time_stamp: Duration,
}

/// The logical key each held physical key went down with: Flutter's `_pressingRecords`.
#[derive(Default)]
pub(crate) struct KeyResponder {
    pressing_records: HashMap<u64, u64>,
}

impl KeyResponder {
    /// Flutter's `handlePressBegin`: a key down, or `None` for a key already held, whose down
    /// Flutter answers handled with an empty event.
    pub(crate) fn press_began(&mut self, press: &Press) -> Option<KeyData> {
        let physical = physical_key(press.key_code);
        let special = special_key(&press.characters_ignoring_modifiers);
        let logical = special.unwrap_or_else(|| logical_key(press));
        if self.pressing_records.contains_key(&physical) {
            return None;
        }
        self.pressing_records.insert(physical, logical);
        let character = match special {
            Some(_) => None,
            None => event_characters(&press.characters, press.key_code),
        };
        Some(KeyData {
            time_stamp: press.time_stamp,
            event_type: KeyEventType::Down,
            device_type: KeyEventDeviceType::Keyboard,
            physical,
            logical,
            character,
            synthesized: false,
        })
    }

    /// Flutter's `handlePressEnd`: a key up with the logical key the key went down with, or `None`
    /// for a key not held.
    pub(crate) fn press_ended(&mut self, press: &Press) -> Option<KeyData> {
        let physical = physical_key(press.key_code);
        let logical = self.pressing_records.remove(&physical)?;
        Some(KeyData {
            time_stamp: press.time_stamp,
            event_type: KeyEventType::Up,
            device_type: KeyEventDeviceType::Keyboard,
            physical,
            logical,
            character: None,
            synthesized: false,
        })
    }
}

/// `GetPhysicalKeyForKeyCode`: the usage on the keyboard page for the codes Flutter's generated
/// `keyCodeToPhysicalKey` lists, every one of which maps so, and the iOS plane for any other.
fn physical_key(key_code: u32) -> u64 {
    const LISTED: [(u32, u32); 14] = [
        (0x00, 0x31),
        (0x33, 0x75),
        (0x77, 0x77),
        (0x79, 0x81),
        (0x85, 0x85),
        (0x87, 0x8b),
        (0x90, 0x94),
        (0x9b, 0x9b),
        (0xa3, 0xa3),
        (0xb6, 0xb7),
        (0xbb, 0xbb),
        (0xd0, 0xd4),
        (0xd7, 0xd9),
        (0xe0, 0xe7),
    ];
    if LISTED
        .iter()
        .any(|(first, last)| (*first..=*last).contains(&key_code))
    {
        return 0x0007_0000 | u64::from(key_code);
    }
    key_of_plane(u64::from(key_code), IOS_PLANE)
}

/// `GetLogicalKeyForEvent` for a press that is no special key: the key Flutter names by code, the
/// character the key types without modifiers, lower-cased, or a new code on the iOS plane.
fn logical_key(press: &Press) -> u64 {
    if let Some(logical) = logical_of_code(press.key_code) {
        return logical;
    }
    let label = event_characters(&press.characters_ignoring_modifiers, press.key_code);
    let units: Vec<u16> = label.as_deref().unwrap_or("").encode_utf16().collect();
    let printable = !units.is_empty()
        && !is_control_character(&units)
        && !is_unprintable_key(&units)
        && units.len() <= 2;
    if printable {
        let code_unit = units
            .iter()
            .fold(0u64, |code, unit| (code << 16) | u64::from(*unit));
        return key_of_plane(to_lower(code_unit), UNICODE_PLANE);
    }
    key_of_plane(u64::from(press.key_code), IOS_PLANE)
}

/// `getEventCharacters`: a function key types nothing, whatever UIKit says.
fn event_characters(characters: &str, key_code: u32) -> Option<String> {
    if characters.is_empty() || is_function_key(key_code) {
        return None;
    }
    Some(characters.to_owned())
}

fn key_of_plane(base_key: u64, plane: u64) -> u64 {
    plane | (base_key & VALUE_MASK)
}

/// `IsControlCharacter`: one code unit among C0 and C1.
fn is_control_character(units: &[u16]) -> bool {
    matches!(units, [unit] if *unit <= 0x1f || (0x7f..=0x9f).contains(unit))
}

/// `IsUnprintableKey`: one code unit among the function keys' private use characters.
fn is_unprintable_key(units: &[u16]) -> bool {
    matches!(units, [unit] if (0xf700..=0xf8ff).contains(unit))
}

/// `toLower`: ASCII and Latin-1 capitals to their small letters, whatever the locale, as Flutter
/// lowers them, the multiplication sign to the division sign among them.
fn to_lower(code: u64) -> u64 {
    match code {
        0x41..=0x5a => code - 0x41 + 0x61,
        0xc0..=0xde => code - 0xc0 + 0xe0,
        _ => code,
    }
}

/// `functionKeyCodes`: F1 to F24.
fn is_function_key(key_code: u32) -> bool {
    (0x3a..=0x45).contains(&key_code) || (0x68..=0x73).contains(&key_code)
}

/// `specialKeyMapping`: the keys UIKit names by a literal on their label.
fn special_key(label: &str) -> Option<u64> {
    Some(match label {
        "UIKeyInputEscape" => 0x10000001b,
        "UIKeyInputF1" => 0x100000801,
        "UIKeyInputF2" => 0x100000802,
        "UIKeyInputF3" => 0x100000803,
        "UIKeyInputF4" => 0x100000804,
        "UIKeyInputF5" => 0x100000805,
        "UIKeyInputF6" => 0x100000806,
        "UIKeyInputF7" => 0x100000807,
        "UIKeyInputF8" => 0x100000808,
        "UIKeyInputF9" => 0x100000809,
        "UIKeyInputF10" => 0x10000080a,
        "UIKeyInputF11" => 0x10000080b,
        "UIKeyInputF12" => 0x10000080c,
        "UIKeyInputUpArrow" => 0x100000304,
        "UIKeyInputDownArrow" => 0x100000301,
        "UIKeyInputLeftArrow" => 0x100000302,
        "UIKeyInputRightArrow" => 0x100000303,
        "UIKeyInputHome" => 0x100000306,
        "UIKeyInputEnd" => 0x10000000d,
        "UIKeyInputPageUp" => 0x100000308,
        "UIKeyInputPageDown" => 0x100000307,
        _ => return None,
    })
}

/// `keyCodeToLogicalKey`: the keys Flutter names by their code rather than what they type.
fn logical_of_code(key_code: u32) -> Option<u64> {
    Some(match key_code {
        0x28 => 0x0010000000d, // Enter
        0x29 => 0x0010000001b, // Escape
        0x2a => 0x00100000008, // Backspace
        0x2b => 0x00100000009, // Tab
        0x39 => 0x00100000104, // CapsLock
        0x3a => 0x00100000801, // F1
        0x3b => 0x00100000802, // F2
        0x3c => 0x00100000803, // F3
        0x3d => 0x00100000804, // F4
        0x3e => 0x00100000805, // F5
        0x3f => 0x00100000806, // F6
        0x40 => 0x00100000807, // F7
        0x41 => 0x00100000808, // F8
        0x42 => 0x00100000809, // F9
        0x43 => 0x0010000080a, // F10
        0x44 => 0x0010000080b, // F11
        0x45 => 0x0010000080c, // F12
        0x49 => 0x00100000407, // Insert
        0x4a => 0x00100000306, // Home
        0x4b => 0x00100000308, // PageUp
        0x4c => 0x0010000007f, // Delete
        0x4d => 0x00100000305, // End
        0x4e => 0x00100000307, // PageDown
        0x4f => 0x00100000303, // ArrowRight
        0x50 => 0x00100000302, // ArrowLeft
        0x51 => 0x00100000301, // ArrowDown
        0x52 => 0x00100000304, // ArrowUp
        0x53 => 0x0010000010a, // NumLock
        0x54 => 0x0020000022f, // NumpadDivide
        0x55 => 0x0020000022a, // NumpadMultiply
        0x56 => 0x0020000022d, // NumpadSubtract
        0x57 => 0x0020000022b, // NumpadAdd
        0x58 => 0x0020000020d, // NumpadEnter
        0x59 => 0x00200000231, // Numpad1
        0x5a => 0x00200000232, // Numpad2
        0x5b => 0x00200000233, // Numpad3
        0x5c => 0x00200000234, // Numpad4
        0x5d => 0x00200000235, // Numpad5
        0x5e => 0x00200000236, // Numpad6
        0x5f => 0x00200000237, // Numpad7
        0x60 => 0x00200000238, // Numpad8
        0x61 => 0x00200000239, // Numpad9
        0x62 => 0x00200000230, // Numpad0
        0x63 => 0x0020000022e, // NumpadDecimal
        0x65 => 0x00100000505, // ContextMenu
        0x67 => 0x0020000023d, // NumpadEqual
        0x68 => 0x0010000080d, // F13
        0x69 => 0x0010000080e, // F14
        0x6a => 0x0010000080f, // F15
        0x6b => 0x00100000810, // F16
        0x6c => 0x00100000811, // F17
        0x6d => 0x00100000812, // F18
        0x6e => 0x00100000813, // F19
        0x6f => 0x00100000814, // F20
        0x7f => 0x00100000a11, // AudioVolumeMute
        0x80 => 0x00100000a10, // AudioVolumeUp
        0x81 => 0x00100000a0f, // AudioVolumeDown
        0x85 => 0x0020000022c, // NumpadComma
        0x87 => 0x00200000021, // IntlRo
        0x89 => 0x00200000022, // IntlYen
        0x90 => 0x00200000010, // Lang1
        0x91 => 0x00200000011, // Lang2
        0x92 => 0x00200000012, // Lang3
        0x93 => 0x00200000013, // Lang4
        0x94 => 0x00200000014, // Lang5
        0xe0 => 0x00200000100, // ControlLeft
        0xe1 => 0x00200000102, // ShiftLeft
        0xe2 => 0x00200000104, // AltLeft
        0xe3 => 0x00200000106, // MetaLeft
        0xe4 => 0x00200000101, // ControlRight
        0xe5 => 0x00200000103, // ShiftRight
        0xe6 => 0x00200000105, // AltRight
        0xe7 => 0x00200000107, // MetaRight
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_A: u32 = 0x04;
    const SHIFT_LEFT: u32 = 0xe1;

    fn press(key_code: u32, characters: &str, ignoring_modifiers: &str) -> Press {
        Press {
            key_code,
            characters: characters.to_owned(),
            characters_ignoring_modifiers: ignoring_modifiers.to_owned(),
            time_stamp: Duration::from_millis(5),
        }
    }

    #[test]
    fn a_letter_is_its_usage_and_its_small_letter_and_types_what_shift_makes_it() {
        let mut keys = KeyResponder::default();
        let down = keys.press_began(&press(KEY_A, "A", "a")).expect("a down");
        assert_eq!(down.physical, 0x00070004);
        assert_eq!(down.logical, 0x61);
        assert_eq!(down.character.as_deref(), Some("A"));
        let up = keys.press_ended(&press(KEY_A, "", "a")).expect("an up");
        assert_eq!(up.event_type, KeyEventType::Up);
        assert_eq!(up.logical, 0x61);
        assert_eq!(up.character, None);
    }

    #[test]
    fn a_named_key_and_a_special_key_type_no_character_flutter_reads() {
        let mut keys = KeyResponder::default();
        let shift = keys
            .press_began(&press(SHIFT_LEFT, "", ""))
            .expect("a down");
        assert_eq!(shift.logical, 0x00200000102);
        assert_eq!(shift.character, None);
        let escape = keys
            .press_began(&press(0x29, "UIKeyInputEscape", "UIKeyInputEscape"))
            .expect("a down");
        assert_eq!(escape.logical, 0x10000001b);
        assert_eq!(escape.character, None);
        let f1 = keys
            .press_began(&press(0x3a, "\u{10}", "\u{10}"))
            .expect("a down");
        assert_eq!(f1.character, None);
    }

    #[test]
    fn a_key_held_goes_down_once_and_up_with_the_logical_key_it_went_down_with() {
        let mut keys = KeyResponder::default();
        let down = keys.press_began(&press(KEY_A, "a", "a")).expect("a down");
        assert_eq!(keys.press_began(&press(KEY_A, "a", "a")), None);
        let up = keys.press_ended(&press(KEY_A, "", "q")).expect("an up");
        assert_eq!(up.logical, down.logical);
        assert_eq!(keys.press_ended(&press(KEY_A, "", "a")), None);
    }

    #[test]
    fn latin_1_capitals_lower_as_flutter_lowers_them() {
        assert_eq!(to_lower(u64::from('É')), u64::from('é'));
        assert_eq!(to_lower(u64::from('1')), u64::from('1'));
    }
}
