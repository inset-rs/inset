//! W3C `KeyboardEvent.code` / `key` as Flutter HID usages and logical key ids.
//!
//! The same tables the winit host uses, keyed by the strings the browser already
//! reports (which is what winit's `KeyCode` names are).

/// USB HID usage for a W3C `code` value. `None` if Flutter has no usage for it.
pub fn physical_key_usage(code: &str) -> Option<u64> {
    Some(match code {
        "Backquote" => 0x00070035,
        "Backslash" => 0x00070031,
        "BracketLeft" => 0x0007002f,
        "BracketRight" => 0x00070030,
        "Comma" => 0x00070036,
        "Digit0" => 0x00070027,
        "Digit1" => 0x0007001e,
        "Digit2" => 0x0007001f,
        "Digit3" => 0x00070020,
        "Digit4" => 0x00070021,
        "Digit5" => 0x00070022,
        "Digit6" => 0x00070023,
        "Digit7" => 0x00070024,
        "Digit8" => 0x00070025,
        "Digit9" => 0x00070026,
        "Equal" => 0x0007002e,
        "IntlBackslash" => 0x00070064,
        "IntlRo" => 0x00070087,
        "IntlYen" => 0x00070089,
        "KeyA" => 0x00070004,
        "KeyB" => 0x00070005,
        "KeyC" => 0x00070006,
        "KeyD" => 0x00070007,
        "KeyE" => 0x00070008,
        "KeyF" => 0x00070009,
        "KeyG" => 0x0007000a,
        "KeyH" => 0x0007000b,
        "KeyI" => 0x0007000c,
        "KeyJ" => 0x0007000d,
        "KeyK" => 0x0007000e,
        "KeyL" => 0x0007000f,
        "KeyM" => 0x00070010,
        "KeyN" => 0x00070011,
        "KeyO" => 0x00070012,
        "KeyP" => 0x00070013,
        "KeyQ" => 0x00070014,
        "KeyR" => 0x00070015,
        "KeyS" => 0x00070016,
        "KeyT" => 0x00070017,
        "KeyU" => 0x00070018,
        "KeyV" => 0x00070019,
        "KeyW" => 0x0007001a,
        "KeyX" => 0x0007001b,
        "KeyY" => 0x0007001c,
        "KeyZ" => 0x0007001d,
        "Minus" => 0x0007002d,
        "Period" => 0x00070037,
        "Quote" => 0x00070034,
        "Semicolon" => 0x00070033,
        "Slash" => 0x00070038,
        "AltLeft" => 0x000700e2,
        "AltRight" => 0x000700e6,
        "Backspace" => 0x0007002a,
        "CapsLock" => 0x00070039,
        "ContextMenu" => 0x00070065,
        "ControlLeft" => 0x000700e0,
        "ControlRight" => 0x000700e4,
        "Enter" => 0x00070028,
        "MetaLeft" | "OSLeft" => 0x000700e3,
        "MetaRight" | "OSRight" => 0x000700e7,
        "ShiftLeft" => 0x000700e1,
        "ShiftRight" => 0x000700e5,
        "Space" => 0x0007002c,
        "Tab" => 0x0007002b,
        "Delete" => 0x0007004c,
        "End" => 0x0007004d,
        "Help" => 0x00070075,
        "Home" => 0x0007004a,
        "Insert" => 0x00070049,
        "PageDown" => 0x0007004e,
        "PageUp" => 0x0007004b,
        "ArrowDown" => 0x00070051,
        "ArrowLeft" => 0x00070050,
        "ArrowRight" => 0x0007004f,
        "ArrowUp" => 0x00070052,
        "Escape" => 0x00070029,
        "F1" => 0x0007003a,
        "F2" => 0x0007003b,
        "F3" => 0x0007003c,
        "F4" => 0x0007003d,
        "F5" => 0x0007003e,
        "F6" => 0x0007003f,
        "F7" => 0x00070040,
        "F8" => 0x00070041,
        "F9" => 0x00070042,
        "F10" => 0x00070043,
        "F11" => 0x00070044,
        "F12" => 0x00070045,
        _ => return None,
    })
}

/// Flutter `LogicalKeyboardKey` for a W3C `key` value and `location`.
pub fn logical_key_id(key: &str, location: i16) -> Option<u64> {
    if let Some(id) = named_logical(key) {
        return Some(id);
    }
    if let Some(id) = located_logical(key, location) {
        return Some(id);
    }
    if event_key_is_key_name(key) {
        return None;
    }
    character_logical_key(key)
}

/// Flutter `KeyData.character`: drop control characters.
pub fn character_of(key: &str) -> Option<String> {
    if is_control_character(key) || event_key_is_key_name(key) {
        None
    } else {
        Some(key.to_owned())
    }
}

fn named_logical(key: &str) -> Option<u64> {
    Some(match key {
        "Enter" => 0x0010000000d,
        "Tab" => 0x00100000009,
        "Backspace" => 0x00100000008,
        "Escape" => 0x0010000001b,
        "Delete" => 0x0010000007f,
        "ArrowDown" => 0x00100000301,
        "ArrowLeft" => 0x00100000302,
        "ArrowRight" => 0x00100000303,
        "ArrowUp" => 0x00100000304,
        "Home" => 0x00100000306,
        "End" => 0x00100000305,
        "PageUp" => 0x00100000308,
        "PageDown" => 0x00100000307,
        "Insert" => 0x00100000407,
        "CapsLock" => 0x00100000104,
        " " => 0x00000020,
        _ => return None,
    })
}

fn located_logical(key: &str, location: i16) -> Option<u64> {
    let right = location == 2;
    Some(match key {
        "Shift" => {
            if right {
                0x00200000103
            } else {
                0x00200000102
            }
        }
        "Control" => {
            if right {
                0x00200000101
            } else {
                0x00200000100
            }
        }
        "Alt" => {
            if right {
                0x00200000105
            } else {
                0x00200000104
            }
        }
        "Meta" => {
            if right {
                0x00200000107
            } else {
                0x00200000106
            }
        }
        _ => return None,
    })
}

fn event_key_is_key_name(key: &str) -> bool {
    let mut units = key.encode_utf16();
    match (units.next(), units.next()) {
        (Some(first), Some(second)) => first < 0x7f && second < 0x7f,
        _ => false,
    }
}

fn character_logical_key(value: &str) -> Option<u64> {
    let character = value.chars().next()?;
    let mut lowercase = character.to_lowercase();
    match (lowercase.next(), lowercase.next()) {
        (Some(lowered), None) => Some(u64::from(lowered)),
        _ => Some(u64::from(character)),
    }
}

fn is_control_character(label: &str) -> bool {
    let mut characters = label.chars();
    let (Some(code_unit), None) = (characters.next(), characters.next()) else {
        return false;
    };
    let code_unit = u32::from(code_unit);
    code_unit <= 0x1f || (0x7f..=0x9f).contains(&code_unit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letter_codes_are_hid_usages() {
        assert_eq!(physical_key_usage("KeyA"), Some(0x00070004));
        assert_eq!(physical_key_usage("Digit1"), Some(0x0007001e));
        assert_eq!(physical_key_usage("F25"), None);
    }

    #[test]
    fn named_keys_match_flutter_logical_ids() {
        assert_eq!(logical_key_id("Enter", 0), Some(0x0010000000d));
        assert_eq!(logical_key_id("Shift", 1), Some(0x00200000102));
        assert_eq!(logical_key_id("Shift", 2), Some(0x00200000103));
        assert_eq!(logical_key_id("a", 0), Some(u64::from('a')));
    }

    #[test]
    fn control_characters_are_not_typed() {
        assert_eq!(character_of("a"), Some("a".to_owned()));
        assert_eq!(character_of("Enter"), None);
        assert_eq!(character_of("\u{8}"), None);
    }
}
