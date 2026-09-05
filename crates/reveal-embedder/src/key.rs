//! Flutter counterpart: `engine/src/flutter/lib/ui/key.dart`.

use std::fmt::{self, Display};
use std::time::Duration;

/// The type of a key event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyEventType {
    /// The key is pressed.
    Down,

    /// The key is released.
    Up,

    /// The key is held, causing a repeated key input.
    Repeat,
}

impl KeyEventType {
    /// A human readable name, used by [`KeyData`]'s textual descriptions.
    pub fn label(&self) -> &'static str {
        match self {
            KeyEventType::Down => "Key Down",
            KeyEventType::Up => "Key Up",
            KeyEventType::Repeat => "Key Repeat",
        }
    }
}

/// The source device for the key event.
///
/// Not all platforms supply an accurate type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyEventDeviceType {
    /// The device is a keyboard.
    #[default]
    Keyboard,

    /// The device is a directional pad on something like a television remote
    /// control or similar.
    DirectionalPad,

    /// The device is a gamepad button
    Gamepad,

    /// The device is a joystick button
    Joystick,

    /// The device is a device connected to an HDMI bus.
    Hdmi,
}

impl KeyEventDeviceType {
    /// A human readable name, used by [`KeyData`]'s textual descriptions.
    pub fn label(&self) -> &'static str {
        match self {
            KeyEventDeviceType::Keyboard => "Keyboard",
            KeyEventDeviceType::DirectionalPad => "Directional Pad",
            KeyEventDeviceType::Gamepad => "Gamepad",
            KeyEventDeviceType::Joystick => "Joystick",
            KeyEventDeviceType::Hdmi => "HDMI",
        }
    }
}

/// Information about a key event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyData {
    /// Time of event dispatch, relative to an arbitrary timeline.
    ///
    /// For synthesized events, the [`time_stamp`](Self::time_stamp) might not be
    /// the actual time that the key press or release happens.
    pub time_stamp: Duration,

    /// The type of the event.
    pub event_type: KeyEventType,

    /// Describes what type of device (keyboard, directional pad, etc.) this event
    /// originated from.
    pub device_type: KeyEventDeviceType,

    /// The key code for the physical key that has changed.
    pub physical: u64,

    /// The key code for the logical key that has changed.
    pub logical: u64,

    /// Character input from the event.
    ///
    /// Ignored for up events.
    pub character: Option<String>,

    /// If [`synthesized`](Self::synthesized) is true, this event does not
    /// correspond to a native event.
    ///
    /// Although most of Flutter's keyboard events are transformed from native
    /// events, some events are not based on native events, and are synthesized
    /// only to conform Flutter's key event model (as documented in the
    /// `HardwareKeyboard` class in the framework).
    ///
    /// For example, some key downs or ups might be lost when the window loses
    /// focus. Some platforms provide ways to query whether a key is being held.
    /// If the embedder detects an inconsistency between its internal record and
    /// the state returned by the system, the embedder will synthesize a
    /// corresponding event to synchronize the state without breaking the event
    /// model.
    ///
    /// As another example, macOS treats CapsLock in a special way by sending down
    /// and up events at the down of alternate presses to indicate the direction
    /// in which the lock is toggled instead of that the physical key is going. A
    /// macOS embedder should normalize the behavior by converting a native down
    /// event into a down event followed immediately by a synthesized up event,
    /// and the native up event also into a down event followed immediately by a
    /// synthesized up event.
    ///
    /// Synthesized events do not have a trustworthy
    /// [`time_stamp`](Self::time_stamp), and should not be processed as if the
    /// key actually went down or up at the time of the callback.
    ///
    /// A key repeat event is never synthesized.
    pub synthesized: bool,
}

impl Default for KeyData {
    fn default() -> KeyData {
        KeyData {
            time_stamp: Duration::ZERO,
            event_type: KeyEventType::Down,
            device_type: KeyEventDeviceType::Keyboard,
            physical: 0,
            logical: 0,
            character: None,
            synthesized: false,
        }
    }
}

impl KeyData {
    /// Returns the bits that are not included in the value mask, shifted to the
    /// right.
    ///
    /// For example, if the input is 0x12abcdabcd, then the result is 0x12.
    fn non_value_bits(n: u64) -> u64 {
        const VALUE_MASK_WIDTH: u32 = 32;
        // Dart limits the non-value bits to what a JavaScript number can hold.
        const MAX_SAFE_INTEGER_WIDTH: u32 = 52;
        const NON_VALUE_MASK: u64 = (1 << (MAX_SAFE_INTEGER_WIDTH - VALUE_MASK_WIDTH)) - 1;
        (n >> VALUE_MASK_WIDTH) & NON_VALUE_MASK
    }

    fn logical_to_string(&self) -> String {
        let plane_description = match KeyData::non_value_bits(self.logical) & 0x0ff {
            0x000 => " (Unicode)",
            0x001 => " (Unprintable)",
            0x002 => " (Flutter)",
            0x011 => " (Android)",
            0x012 => " (Fuchsia)",
            0x013 => " (iOS)",
            0x014 => " (macOS)",
            0x015 => " (GTK)",
            0x016 => " (Windows)",
            0x017 => " (Web)",
            0x018 => " (GLFW)",
            _ => "",
        };
        format!("0x{:x}{}", self.logical, plane_description)
    }

    fn escape_character(&self) -> String {
        let Some(character) = &self.character else {
            return "<none>".to_owned();
        };
        match character.as_str() {
            "\n" => r#""\n""#.to_owned(),
            "\t" => r#""\t""#.to_owned(),
            "\r" => r#""\r""#.to_owned(),
            "\u{8}" => r#""\b""#.to_owned(),
            "\u{c}" => r#""\f""#.to_owned(),
            _ => format!("\"{character}\""),
        }
    }

    fn quoted_char_code(&self) -> String {
        let Some(character) = &self.character else {
            return String::new();
        };
        let hex_chars: Vec<String> = character
            .encode_utf16()
            .map(|code| format!("{code:02x}"))
            .collect();
        format!(" (0x{})", hex_chars.join(" "))
    }

    /// Returns a complete textual description of the information in this object.
    pub fn to_string_full(&self) -> String {
        format!(
            "KeyData(type: {}, \
             deviceType: {}, \
             timeStamp: {:?}, \
             physical: 0x{:x}, \
             logical: 0x{:x}, \
             character: {}, \
             synthesized: {}\
             )",
            self.event_type.label(),
            self.device_type.label(),
            self.time_stamp,
            self.physical,
            self.logical,
            self.escape_character(),
            self.synthesized,
        )
    }
}

impl Display for KeyData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KeyData({}, physical: 0x{:x}, logical: {}, character: {}{}{}",
            self.event_type.label(),
            self.physical,
            self.logical_to_string(),
            self.escape_character(),
            self.quoted_char_code(),
            if self.synthesized {
                ", synthesized"
            } else {
                ""
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{KeyData, KeyEventDeviceType, KeyEventType};

    #[test]
    fn to_string_names_the_plane_and_quotes_the_character() {
        let data = KeyData {
            time_stamp: Duration::from_millis(5),
            event_type: KeyEventType::Down,
            device_type: KeyEventDeviceType::Keyboard,
            physical: 0x00070004,
            logical: 0x00000000061,
            character: Some("a".to_owned()),
            synthesized: false,
        };
        assert_eq!(
            data.to_string(),
            "KeyData(Key Down, physical: 0x70004, logical: 0x61 (Unicode), character: \"a\" (0x61)"
        );
    }

    #[test]
    fn to_string_full_reports_every_field() {
        let data = KeyData {
            time_stamp: Duration::from_millis(5),
            event_type: KeyEventType::Up,
            device_type: KeyEventDeviceType::Gamepad,
            physical: 0x00070004,
            logical: 0x00100000301,
            character: None,
            synthesized: true,
        };
        assert_eq!(
            data.to_string_full(),
            "KeyData(type: Key Up, deviceType: Gamepad, timeStamp: 5ms, physical: 0x70004, \
             logical: 0x100000301, character: <none>, synthesized: true)"
        );
    }
}
