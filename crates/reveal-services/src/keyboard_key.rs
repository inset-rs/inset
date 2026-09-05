//! Flutter counterpart: `services/keyboard_key.g.dart`.
//!
//! Transcribed from that file, which Flutter itself generates from
//! `dev/tools/gen_keycodes`. The key constants and the four tables below are
//! machine-written; edit the generator, not this file.

use std::collections::HashSet;
use std::fmt::{self, Debug};

/// A base class for all keyboard key types.
///
/// See also:
///
///  * [`PhysicalKeyboardKey`], a type with constants that describe the keys that
///    are returned from `RawKeyEvent.physicalKey`.
///  * [`LogicalKeyboardKey`], a type with constants that describe the keys that
///    are returned from `RawKeyEvent.logicalKey`.
pub trait KeyboardKey: Copy + Eq + Debug {}

/// A type with constants that describe the keys that are returned from
/// `RawKeyEvent.logicalKey`.
///
/// These represent *logical* keys, which are keys which are interpreted in the
/// context of any modifiers, modes, or keyboard layouts which may be in effect.
///
/// This is contrast to [`PhysicalKeyboardKey`], which represents a physical key
/// in a particular location on the keyboard, without regard for the modifier
/// state, mode, or keyboard layout.
///
/// As an example, if you wanted to implement an app where the "Q" key "quit"
/// something, you'd want to look at the logical key to detect this, since you
/// would like to have it match the key with "Q" on it, instead of always
/// looking for "the key next to the TAB key", since on a French keyboard,
/// the key next to the TAB key has an "A" on it.
///
/// Conversely, if you wanted a game where the key next to the CAPS LOCK (the
/// "A" key on a QWERTY keyboard) moved the player to the left, you'd want to
/// look at the physical key to make sure that regardless of the character the
/// key produces, you got the key that is in that location on the keyboard.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LogicalKeyboardKey {
    /// A unique code representing this key.
    ///
    /// This is an opaque code. It should not be unpacked to derive information
    /// from it, as the representation of the code could change at any time.
    pub key_id: u64,
}

impl KeyboardKey for LogicalKeyboardKey {}

impl LogicalKeyboardKey {
    /// Creates a new [`LogicalKeyboardKey`] for a key ID.
    pub const fn new(key_id: u64) -> LogicalKeyboardKey {
        LogicalKeyboardKey { key_id }
    }

    /// Returns the bits that are not included in [`VALUE_MASK`](Self::VALUE_MASK),
    /// shifted to the right.
    ///
    /// For example, if the input is 0x12abcdabcd, then the result is 0x12.
    fn non_value_bits(n: u64) -> u64 {
        const VALUE_MASK_WIDTH: u32 = 32;
        // Dart limits the non-value bits to what a JavaScript number can hold.
        const MAX_SAFE_INTEGER_WIDTH: u32 = 52;
        const NON_VALUE_MASK: u64 = (1 << (MAX_SAFE_INTEGER_WIDTH - VALUE_MASK_WIDTH)) - 1;
        (n >> VALUE_MASK_WIDTH) & NON_VALUE_MASK
    }

    fn unicode_key_label(key_id: u64) -> Option<String> {
        if LogicalKeyboardKey::non_value_bits(key_id) == 0
            && let Ok(code_point) = u32::try_from(key_id)
            && let Some(character) = char::from_u32(code_point)
        {
            return Some(character.to_uppercase().to_string());
        }
        None
    }

    /// A description representing the character produced by a `RawKeyEvent`.
    ///
    /// This value is useful for providing readable strings for keys or keyboard
    /// shortcuts. Do not use this value to compare equality of keys; compare
    /// [`key_id`](Self::key_id) instead.
    ///
    /// For printable keys, this is usually the printable character in upper case
    /// ignoring modifiers or combining keys, such as 'A', '1', or '/'. This
    /// might also return accented letters (such as 'Ù') for keys labeled as so,
    /// but not if such character is a result from preceding combining keys ('`̀'
    /// followed by key U).
    ///
    /// For other keys, [`key_label`](Self::key_label) looks up the full key name
    /// from a predefined map, such as 'F1', 'Shift Left', or 'Media Down'. This
    /// value is an empty string if there's no key label data for a key.
    pub fn key_label(&self) -> String {
        LogicalKeyboardKey::unicode_key_label(self.key_id)
            .or_else(|| key_label_of(self.key_id).map(str::to_owned))
            .unwrap_or_default()
    }

    /// The debug string to print for this keyboard key, which will be `None` in
    /// release mode.
    ///
    /// For printable keys, this is usually a more descriptive name related to
    /// [`key_label`](Self::key_label), such as 'Key A', 'Digit 1', 'Backslash'.
    /// This might also return accented letters (such as 'Key Ù') for keys
    /// labeled as so.
    ///
    /// For other keys, this looks up the full key name from a predefined map (the
    /// same value as [`key_label`](Self::key_label)), such as 'F1', 'Shift Left',
    /// or 'Media Down'. If there's no key label data for a key, this returns a
    /// name that explains the ID (such as 'Key with ID 0x00100012345').
    pub fn debug_name(&self) -> Option<String> {
        if !cfg!(debug_assertions) {
            return None;
        }
        if let Some(label) = key_label_of(self.key_id) {
            return Some(label.to_owned());
        }
        if let Some(label) = LogicalKeyboardKey::unicode_key_label(self.key_id) {
            return Some(format!("Key {label}"));
        }
        Some(format!("Key with ID 0x{:011x}", self.key_id))
    }

    /// Returns the [`LogicalKeyboardKey`] constant that matches the given ID, or
    /// `None`, if not found.
    pub fn find_key_by_key_id(key_id: u64) -> Option<LogicalKeyboardKey> {
        KNOWN_LOGICAL_KEYS
            .binary_search_by_key(&key_id, |key| key.key_id)
            .ok()
            .map(|index| KNOWN_LOGICAL_KEYS[index])
    }

    /// Returns true if the given label represents a Unicode control character.
    ///
    /// Examples of control characters are characters like "U+000A LINE FEED (LF)"
    /// or "U+001B ESCAPE (ESC)".
    ///
    /// See <https://en.wikipedia.org/wiki/Unicode_control_characters> for more
    /// information.
    pub fn is_control_character(label: &str) -> bool {
        let mut characters = label.chars();
        let (Some(code_unit), None) = (characters.next(), characters.next()) else {
            return false;
        };
        let code_unit = u32::from(code_unit);
        code_unit <= 0x1f || (0x7f..=0x9f).contains(&code_unit)
    }

    /// Returns true if the [`key_id`](Self::key_id) of this object is one that is
    /// auto-generated by Flutter.
    ///
    /// Auto-generated key IDs are generated in response to platform key codes
    /// which Flutter doesn't recognize, and their IDs shouldn't be used in a
    /// persistent way.
    ///
    /// Auto-generated IDs should be a rare occurrence: Flutter supports most keys.
    ///
    /// Keys that generate Unicode characters (even if unknown to Flutter) will
    /// not return true for [`is_autogenerated`](Self::is_autogenerated), since
    /// they will be assigned a Unicode-based code that will remain stable.
    pub fn is_autogenerated(&self) -> bool {
        (self.key_id & LogicalKeyboardKey::PLANE_MASK)
            >= LogicalKeyboardKey::START_OF_PLATFORM_PLANES
    }

    /// Returns a set of pseudo-key synonyms for this key.
    ///
    /// This allows finding the pseudo-keys that also represent a concrete key
    /// so that a class with a key map can match pseudo-keys as well as the actual
    /// generated keys.
    ///
    /// Pseudo-keys returned in the set are typically used to represent keys which
    /// appear in multiple places on the keyboard, such as the
    /// [`SHIFT`](Self::SHIFT), [`ALT`](Self::ALT), [`CONTROL`](Self::CONTROL), and
    /// [`META`](Self::META) keys. Pseudo-keys in the returned set won't ever be
    /// generated directly, but if a more specific key event is received, then
    /// this set can be used to find the more general pseudo-key. For example, if
    /// this is a [`SHIFT_LEFT`](Self::SHIFT_LEFT) key, this accessor will return
    /// the set `{ shift }`.
    pub fn synonyms(&self) -> HashSet<LogicalKeyboardKey> {
        synonyms_of(*self)
            .unwrap_or_default()
            .iter()
            .copied()
            .collect()
    }

    /// Takes a set of keys, and returns the same set, but with any keys that have
    /// synonyms replaced.
    ///
    /// It is used, for example, to take sets of keys with members like
    /// [`CONTROL_RIGHT`](Self::CONTROL_RIGHT) and
    /// [`CONTROL_LEFT`](Self::CONTROL_LEFT) and convert that set to contain just
    /// [`CONTROL`](Self::CONTROL), so that the question "is any control key
    /// down?" can be asked.
    pub fn collapse_synonyms(input: &HashSet<LogicalKeyboardKey>) -> HashSet<LogicalKeyboardKey> {
        expand_with(input, synonyms_of)
    }

    /// Returns the given set with any pseudo-keys expanded into their synonyms.
    ///
    /// It is used, for example, to take sets of keys with members like
    /// [`CONTROL`](Self::CONTROL) and [`SHIFT`](Self::SHIFT) and convert that set
    /// to contain [`CONTROL_LEFT`](Self::CONTROL_LEFT),
    /// [`CONTROL_RIGHT`](Self::CONTROL_RIGHT), [`SHIFT_LEFT`](Self::SHIFT_LEFT),
    /// and [`SHIFT_RIGHT`](Self::SHIFT_RIGHT).
    pub fn expand_synonyms(input: &HashSet<LogicalKeyboardKey>) -> HashSet<LogicalKeyboardKey> {
        expand_with(input, reverse_synonyms_of)
    }

    /// Mask for the 32-bit value portion of the key code.
    ///
    /// This is used by platform-specific code to generate Flutter key codes.
    pub const VALUE_MASK: u64 = 0x000ffffffff;

    /// Mask for the plane prefix portion of the key code.
    ///
    /// This is used by platform-specific code to generate Flutter key codes.
    pub const PLANE_MASK: u64 = 0x0ff00000000;

    /// The plane value for keys which have a Unicode representation.
    ///
    /// This is used by platform-specific code to generate Flutter key codes.
    pub const UNICODE_PLANE: u64 = 0x00000000000;

    /// The plane value for keys defined by Chromium and does not have a Unicode
    /// representation.
    ///
    /// This is used by platform-specific code to generate Flutter key codes.
    pub const UNPRINTABLE_PLANE: u64 = 0x00100000000;

    /// The plane value for keys defined by Flutter.
    ///
    /// This is used by platform-specific code to generate Flutter key codes.
    pub const FLUTTER_PLANE: u64 = 0x00200000000;

    /// The platform plane with the lowest mask value, beyond which the keys are
    /// considered autogenerated.
    ///
    /// This is used by platform-specific code to generate Flutter key codes.
    pub const START_OF_PLATFORM_PLANES: u64 = 0x01100000000;

    /// The plane value for the private keys defined by the Android embedding.
    ///
    /// This is used by platform-specific code to generate Flutter key codes.
    pub const ANDROID_PLANE: u64 = 0x01100000000;

    /// The plane value for the private keys defined by the Fuchsia embedding.
    ///
    /// This is used by platform-specific code to generate Flutter key codes.
    pub const FUCHSIA_PLANE: u64 = 0x01200000000;

    /// The plane value for the private keys defined by the iOS embedding.
    ///
    /// This is used by platform-specific code to generate Flutter key codes.
    pub const IOS_PLANE: u64 = 0x01300000000;

    /// The plane value for the private keys defined by the macOS embedding.
    ///
    /// This is used by platform-specific code to generate Flutter key codes.
    pub const MACOS_PLANE: u64 = 0x01400000000;

    /// The plane value for the private keys defined by the Gtk embedding.
    ///
    /// This is used by platform-specific code to generate Flutter key codes.
    pub const GTK_PLANE: u64 = 0x01500000000;

    /// The plane value for the private keys defined by the Windows embedding.
    ///
    /// This is used by platform-specific code to generate Flutter key codes.
    pub const WINDOWS_PLANE: u64 = 0x01600000000;

    /// The plane value for the private keys defined by the Web embedding.
    ///
    /// This is used by platform-specific code to generate Flutter key codes.
    pub const WEB_PLANE: u64 = 0x01700000000;

    /// The plane value for the private keys defined by the GLFW embedding.
    ///
    /// This is used by platform-specific code to generate Flutter key codes.
    pub const GLFW_PLANE: u64 = 0x01800000000;

    /// Represents the logical "Space" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SPACE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000020);

    /// Represents the logical "Exclamation" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const EXCLAMATION: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000021);

    /// Represents the logical "Quote" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const QUOTE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000022);

    /// Represents the logical "Number Sign" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMBER_SIGN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000023);

    /// Represents the logical "Dollar" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const DOLLAR: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000024);

    /// Represents the logical "Percent" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PERCENT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000025);

    /// Represents the logical "Ampersand" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AMPERSAND: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000026);

    /// Represents the logical "Quote Single" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const QUOTE_SINGLE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000027);

    /// Represents the logical "Parenthesis Left" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PARENTHESIS_LEFT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000028);

    /// Represents the logical "Parenthesis Right" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PARENTHESIS_RIGHT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000029);

    /// Represents the logical "Asterisk" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ASTERISK: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000002a);

    /// Represents the logical "Add" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ADD: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000002b);

    /// Represents the logical "Comma" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const COMMA: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000002c);

    /// Represents the logical "Minus" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MINUS: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000002d);

    /// Represents the logical "Period" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PERIOD: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000002e);

    /// Represents the logical "Slash" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SLASH: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000002f);

    /// Represents the logical "Digit 0" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const DIGIT0: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000030);

    /// Represents the logical "Digit 1" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const DIGIT1: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000031);

    /// Represents the logical "Digit 2" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const DIGIT2: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000032);

    /// Represents the logical "Digit 3" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const DIGIT3: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000033);

    /// Represents the logical "Digit 4" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const DIGIT4: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000034);

    /// Represents the logical "Digit 5" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const DIGIT5: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000035);

    /// Represents the logical "Digit 6" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const DIGIT6: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000036);

    /// Represents the logical "Digit 7" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const DIGIT7: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000037);

    /// Represents the logical "Digit 8" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const DIGIT8: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000038);

    /// Represents the logical "Digit 9" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const DIGIT9: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000039);

    /// Represents the logical "Colon" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const COLON: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000003a);

    /// Represents the logical "Semicolon" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SEMICOLON: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000003b);

    /// Represents the logical "Less" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LESS: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000003c);

    /// Represents the logical "Equal" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const EQUAL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000003d);

    /// Represents the logical "Greater" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GREATER: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000003e);

    /// Represents the logical "Question" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const QUESTION: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000003f);

    /// Represents the logical "At" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000040);

    /// Represents the logical "Bracket Left" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BRACKET_LEFT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000005b);

    /// Represents the logical "Backslash" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BACKSLASH: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000005c);

    /// Represents the logical "Bracket Right" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BRACKET_RIGHT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000005d);

    /// Represents the logical "Caret" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CARET: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000005e);

    /// Represents the logical "Underscore" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const UNDERSCORE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000005f);

    /// Represents the logical "Backquote" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BACKQUOTE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000060);

    /// Represents the logical "Key A" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_A: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000061);

    /// Represents the logical "Key B" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_B: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000062);

    /// Represents the logical "Key C" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_C: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000063);

    /// Represents the logical "Key D" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_D: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000064);

    /// Represents the logical "Key E" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_E: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000065);

    /// Represents the logical "Key F" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_F: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000066);

    /// Represents the logical "Key G" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_G: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000067);

    /// Represents the logical "Key H" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_H: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000068);

    /// Represents the logical "Key I" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_I: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000069);

    /// Represents the logical "Key J" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_J: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000006a);

    /// Represents the logical "Key K" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_K: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000006b);

    /// Represents the logical "Key L" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_L: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000006c);

    /// Represents the logical "Key M" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_M: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000006d);

    /// Represents the logical "Key N" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_N: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000006e);

    /// Represents the logical "Key O" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_O: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000006f);

    /// Represents the logical "Key P" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_P: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000070);

    /// Represents the logical "Key Q" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_Q: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000071);

    /// Represents the logical "Key R" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_R: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000072);

    /// Represents the logical "Key S" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_S: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000073);

    /// Represents the logical "Key T" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_T: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000074);

    /// Represents the logical "Key U" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_U: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000075);

    /// Represents the logical "Key V" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_V: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000076);

    /// Represents the logical "Key W" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_W: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000077);

    /// Represents the logical "Key X" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_X: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000078);

    /// Represents the logical "Key Y" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_Y: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00000000079);

    /// Represents the logical "Key Z" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY_Z: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000007a);

    /// Represents the logical "Brace Left" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BRACE_LEFT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000007b);

    /// Represents the logical "Bar" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BAR: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000007c);

    /// Represents the logical "Brace Right" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BRACE_RIGHT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000007d);

    /// Represents the logical "Tilde" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TILDE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0000000007e);

    /// Represents the logical "Unidentified" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const UNIDENTIFIED: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000001);

    /// Represents the logical "Backspace" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BACKSPACE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000008);

    /// Represents the logical "Tab" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TAB: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000009);

    /// Represents the logical "Enter" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ENTER: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000000d);

    /// Represents the logical "Escape" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ESCAPE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000001b);

    /// Represents the logical "Delete" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const DELETE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000007f);

    /// Represents the logical "Accel" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ACCEL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000101);

    /// Represents the logical "Alt Graph" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ALT_GRAPH: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000103);

    /// Represents the logical "Caps Lock" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CAPS_LOCK: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000104);

    /// Represents the logical "Fn" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const FN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000106);

    /// Represents the logical "Fn Lock" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const FN_LOCK: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000107);

    /// Represents the logical "Hyper" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const HYPER: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000108);

    /// Represents the logical "Num Lock" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUM_LOCK: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000010a);

    /// Represents the logical "Scroll Lock" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SCROLL_LOCK: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000010c);

    /// Represents the logical "Super" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SUPER_KEY: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000010e);

    /// Represents the logical "Symbol" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SYMBOL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000010f);

    /// Represents the logical "Symbol Lock" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SYMBOL_LOCK: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000110);

    /// Represents the logical "Shift Level 5" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SHIFT_LEVEL5: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000111);

    /// Represents the logical "Arrow Down" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ARROW_DOWN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000301);

    /// Represents the logical "Arrow Left" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ARROW_LEFT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000302);

    /// Represents the logical "Arrow Right" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ARROW_RIGHT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000303);

    /// Represents the logical "Arrow Up" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ARROW_UP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000304);

    /// Represents the logical "End" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const END: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000305);

    /// Represents the logical "Home" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const HOME: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000306);

    /// Represents the logical "Page Down" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PAGE_DOWN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000307);

    /// Represents the logical "Page Up" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PAGE_UP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000308);

    /// Represents the logical "Clear" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CLEAR: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000401);

    /// Represents the logical "Copy" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const COPY: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000402);

    /// Represents the logical "Cr Sel" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CR_SEL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000403);

    /// Represents the logical "Cut" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CUT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000404);

    /// Represents the logical "Erase Eof" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ERASE_EOF: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000405);

    /// Represents the logical "Ex Sel" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const EX_SEL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000406);

    /// Represents the logical "Insert" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const INSERT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000407);

    /// Represents the logical "Paste" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PASTE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000408);

    /// Represents the logical "Redo" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const REDO: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000409);

    /// Represents the logical "Undo" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const UNDO: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000040a);

    /// Represents the logical "Accept" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ACCEPT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000501);

    /// Represents the logical "Again" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AGAIN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000502);

    /// Represents the logical "Attn" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ATTN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000503);

    /// Represents the logical "Cancel" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CANCEL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000504);

    /// Represents the logical "Context Menu" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CONTEXT_MENU: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000505);

    /// Represents the logical "Execute" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const EXECUTE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000506);

    /// Represents the logical "Find" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const FIND: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000507);

    /// Represents the logical "Help" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const HELP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000508);

    /// Represents the logical "Pause" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PAUSE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000509);

    /// Represents the logical "Play" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PLAY: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000050a);

    /// Represents the logical "Props" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PROPS: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000050b);

    /// Represents the logical "Select" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SELECT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000050c);

    /// Represents the logical "Zoom In" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ZOOM_IN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000050d);

    /// Represents the logical "Zoom Out" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ZOOM_OUT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000050e);

    /// Represents the logical "Brightness Down" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BRIGHTNESS_DOWN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000601);

    /// Represents the logical "Brightness Up" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BRIGHTNESS_UP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000602);

    /// Represents the logical "Camera" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CAMERA: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000603);

    /// Represents the logical "Eject" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const EJECT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000604);

    /// Represents the logical "Log Off" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LOG_OFF: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000605);

    /// Represents the logical "Power" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const POWER: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000606);

    /// Represents the logical "Power Off" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const POWER_OFF: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000607);

    /// Represents the logical "Print Screen" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PRINT_SCREEN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000608);

    /// Represents the logical "Hibernate" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const HIBERNATE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000609);

    /// Represents the logical "Standby" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const STANDBY: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000060a);

    /// Represents the logical "Wake Up" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const WAKE_UP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000060b);

    /// Represents the logical "All Candidates" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ALL_CANDIDATES: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000701);

    /// Represents the logical "Alphanumeric" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ALPHANUMERIC: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000702);

    /// Represents the logical "Code Input" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CODE_INPUT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000703);

    /// Represents the logical "Compose" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const COMPOSE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000704);

    /// Represents the logical "Convert" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CONVERT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000705);

    /// Represents the logical "Final Mode" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const FINAL_MODE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000706);

    /// Represents the logical "Group First" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GROUP_FIRST: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000707);

    /// Represents the logical "Group Last" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GROUP_LAST: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000708);

    /// Represents the logical "Group Next" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GROUP_NEXT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000709);

    /// Represents the logical "Group Previous" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GROUP_PREVIOUS: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000070a);

    /// Represents the logical "Mode Change" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MODE_CHANGE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000070b);

    /// Represents the logical "Next Candidate" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NEXT_CANDIDATE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000070c);

    /// Represents the logical "Non Convert" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NON_CONVERT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000070d);

    /// Represents the logical "Previous Candidate" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PREVIOUS_CANDIDATE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000070e);

    /// Represents the logical "Process" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PROCESS: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000070f);

    /// Represents the logical "Single Candidate" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SINGLE_CANDIDATE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000710);

    /// Represents the logical "Hangul Mode" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const HANGUL_MODE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000711);

    /// Represents the logical "Hanja Mode" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const HANJA_MODE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000712);

    /// Represents the logical "Junja Mode" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const JUNJA_MODE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000713);

    /// Represents the logical "Eisu" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const EISU: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000714);

    /// Represents the logical "Hankaku" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const HANKAKU: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000715);

    /// Represents the logical "Hiragana" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const HIRAGANA: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000716);

    /// Represents the logical "Hiragana Katakana" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const HIRAGANA_KATAKANA: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000717);

    /// Represents the logical "Kana Mode" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KANA_MODE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000718);

    /// Represents the logical "Kanji Mode" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KANJI_MODE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000719);

    /// Represents the logical "Katakana" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KATAKANA: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000071a);

    /// Represents the logical "Romaji" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ROMAJI: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000071b);

    /// Represents the logical "Zenkaku" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ZENKAKU: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000071c);

    /// Represents the logical "Zenkaku Hankaku" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ZENKAKU_HANKAKU: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000071d);

    /// Represents the logical "F1" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F1: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000801);

    /// Represents the logical "F2" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F2: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000802);

    /// Represents the logical "F3" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F3: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000803);

    /// Represents the logical "F4" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F4: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000804);

    /// Represents the logical "F5" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F5: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000805);

    /// Represents the logical "F6" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F6: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000806);

    /// Represents the logical "F7" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F7: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000807);

    /// Represents the logical "F8" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F8: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000808);

    /// Represents the logical "F9" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F9: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000809);

    /// Represents the logical "F10" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F10: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000080a);

    /// Represents the logical "F11" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F11: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000080b);

    /// Represents the logical "F12" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F12: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000080c);

    /// Represents the logical "F13" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F13: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000080d);

    /// Represents the logical "F14" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F14: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000080e);

    /// Represents the logical "F15" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F15: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000080f);

    /// Represents the logical "F16" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F16: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000810);

    /// Represents the logical "F17" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F17: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000811);

    /// Represents the logical "F18" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F18: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000812);

    /// Represents the logical "F19" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F19: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000813);

    /// Represents the logical "F20" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F20: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000814);

    /// Represents the logical "F21" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F21: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000815);

    /// Represents the logical "F22" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F22: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000816);

    /// Represents the logical "F23" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F23: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000817);

    /// Represents the logical "F24" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const F24: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000818);

    /// Represents the logical "Soft 1" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SOFT1: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000901);

    /// Represents the logical "Soft 2" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SOFT2: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000902);

    /// Represents the logical "Soft 3" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SOFT3: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000903);

    /// Represents the logical "Soft 4" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SOFT4: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000904);

    /// Represents the logical "Soft 5" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SOFT5: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000905);

    /// Represents the logical "Soft 6" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SOFT6: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000906);

    /// Represents the logical "Soft 7" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SOFT7: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000907);

    /// Represents the logical "Soft 8" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SOFT8: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000908);

    /// Represents the logical "Close" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CLOSE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000a01);

    /// Represents the logical "Mail Forward" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MAIL_FORWARD: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000a02);

    /// Represents the logical "Mail Reply" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MAIL_REPLY: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000a03);

    /// Represents the logical "Mail Send" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MAIL_SEND: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000a04);

    /// Represents the logical "Media Play Pause" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_PLAY_PAUSE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000a05);

    /// Represents the logical "Media Stop" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_STOP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000a07);

    /// Represents the logical "Media Track Next" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_TRACK_NEXT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000a08);

    /// Represents the logical "Media Track Previous" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_TRACK_PREVIOUS: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000a09);

    /// Represents the logical "New" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NEW_KEY: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000a0a);

    /// Represents the logical "Open" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const OPEN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000a0b);

    /// Represents the logical "Print" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PRINT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000a0c);

    /// Represents the logical "Save" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SAVE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000a0d);

    /// Represents the logical "Spell Check" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SPELL_CHECK: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000a0e);

    /// Represents the logical "Audio Volume Down" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AUDIO_VOLUME_DOWN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000a0f);

    /// Represents the logical "Audio Volume Up" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AUDIO_VOLUME_UP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000a10);

    /// Represents the logical "Audio Volume Mute" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AUDIO_VOLUME_MUTE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000a11);

    /// Represents the logical "Launch Application 2" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LAUNCH_APPLICATION2: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000b01);

    /// Represents the logical "Launch Calendar" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LAUNCH_CALENDAR: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000b02);

    /// Represents the logical "Launch Mail" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LAUNCH_MAIL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000b03);

    /// Represents the logical "Launch Media Player" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LAUNCH_MEDIA_PLAYER: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000b04);

    /// Represents the logical "Launch Music Player" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LAUNCH_MUSIC_PLAYER: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000b05);

    /// Represents the logical "Launch Application 1" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LAUNCH_APPLICATION1: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000b06);

    /// Represents the logical "Launch Screen Saver" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LAUNCH_SCREEN_SAVER: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000b07);

    /// Represents the logical "Launch Spreadsheet" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LAUNCH_SPREADSHEET: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000b08);

    /// Represents the logical "Launch Web Browser" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LAUNCH_WEB_BROWSER: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000b09);

    /// Represents the logical "Launch Web Cam" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LAUNCH_WEB_CAM: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000b0a);

    /// Represents the logical "Launch Word Processor" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LAUNCH_WORD_PROCESSOR: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000b0b);

    /// Represents the logical "Launch Contacts" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LAUNCH_CONTACTS: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000b0c);

    /// Represents the logical "Launch Phone" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LAUNCH_PHONE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000b0d);

    /// Represents the logical "Launch Assistant" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LAUNCH_ASSISTANT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000b0e);

    /// Represents the logical "Launch Control Panel" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LAUNCH_CONTROL_PANEL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000b0f);

    /// Represents the logical "Browser Back" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BROWSER_BACK: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000c01);

    /// Represents the logical "Browser Favorites" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BROWSER_FAVORITES: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000c02);

    /// Represents the logical "Browser Forward" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BROWSER_FORWARD: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000c03);

    /// Represents the logical "Browser Home" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BROWSER_HOME: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000c04);

    /// Represents the logical "Browser Refresh" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BROWSER_REFRESH: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000c05);

    /// Represents the logical "Browser Search" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BROWSER_SEARCH: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000c06);

    /// Represents the logical "Browser Stop" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const BROWSER_STOP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000c07);

    /// Represents the logical "Audio Balance Left" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AUDIO_BALANCE_LEFT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d01);

    /// Represents the logical "Audio Balance Right" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AUDIO_BALANCE_RIGHT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d02);

    /// Represents the logical "Audio Bass Boost Down" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AUDIO_BASS_BOOST_DOWN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d03);

    /// Represents the logical "Audio Bass Boost Up" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AUDIO_BASS_BOOST_UP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d04);

    /// Represents the logical "Audio Fader Front" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AUDIO_FADER_FRONT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d05);

    /// Represents the logical "Audio Fader Rear" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AUDIO_FADER_REAR: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d06);

    /// Represents the logical "Audio Surround Mode Next" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AUDIO_SURROUND_MODE_NEXT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d07);

    /// Represents the logical "AVR Input" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AVR_INPUT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d08);

    /// Represents the logical "AVR Power" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AVR_POWER: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d09);

    /// Represents the logical "Channel Down" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CHANNEL_DOWN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d0a);

    /// Represents the logical "Channel Up" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CHANNEL_UP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d0b);

    /// Represents the logical "Color F0 Red" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const COLOR_F0_RED: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d0c);

    /// Represents the logical "Color F1 Green" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const COLOR_F1_GREEN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d0d);

    /// Represents the logical "Color F2 Yellow" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const COLOR_F2_YELLOW: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d0e);

    /// Represents the logical "Color F3 Blue" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const COLOR_F3_BLUE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d0f);

    /// Represents the logical "Color F4 Grey" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const COLOR_F4_GREY: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d10);

    /// Represents the logical "Color F5 Brown" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const COLOR_F5_BROWN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d11);

    /// Represents the logical "Closed Caption Toggle" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CLOSED_CAPTION_TOGGLE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d12);

    /// Represents the logical "Dimmer" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const DIMMER: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d13);

    /// Represents the logical "Display Swap" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const DISPLAY_SWAP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d14);

    /// Represents the logical "Exit" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const EXIT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d15);

    /// Represents the logical "Favorite Clear 0" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const FAVORITE_CLEAR0: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d16);

    /// Represents the logical "Favorite Clear 1" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const FAVORITE_CLEAR1: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d17);

    /// Represents the logical "Favorite Clear 2" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const FAVORITE_CLEAR2: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d18);

    /// Represents the logical "Favorite Clear 3" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const FAVORITE_CLEAR3: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d19);

    /// Represents the logical "Favorite Recall 0" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const FAVORITE_RECALL0: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d1a);

    /// Represents the logical "Favorite Recall 1" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const FAVORITE_RECALL1: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d1b);

    /// Represents the logical "Favorite Recall 2" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const FAVORITE_RECALL2: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d1c);

    /// Represents the logical "Favorite Recall 3" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const FAVORITE_RECALL3: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d1d);

    /// Represents the logical "Favorite Store 0" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const FAVORITE_STORE0: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d1e);

    /// Represents the logical "Favorite Store 1" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const FAVORITE_STORE1: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d1f);

    /// Represents the logical "Favorite Store 2" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const FAVORITE_STORE2: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d20);

    /// Represents the logical "Favorite Store 3" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const FAVORITE_STORE3: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d21);

    /// Represents the logical "Guide" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GUIDE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d22);

    /// Represents the logical "Guide Next Day" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GUIDE_NEXT_DAY: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d23);

    /// Represents the logical "Guide Previous Day" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GUIDE_PREVIOUS_DAY: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d24);

    /// Represents the logical "Info" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const INFO: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d25);

    /// Represents the logical "Instant Replay" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const INSTANT_REPLAY: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d26);

    /// Represents the logical "Link" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LINK: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d27);

    /// Represents the logical "List Program" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LIST_PROGRAM: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d28);

    /// Represents the logical "Live Content" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LIVE_CONTENT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d29);

    /// Represents the logical "Lock" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LOCK: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d2a);

    /// Represents the logical "Media Apps" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_APPS: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d2b);

    /// Represents the logical "Media Fast Forward" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_FAST_FORWARD: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d2c);

    /// Represents the logical "Media Last" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_LAST: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d2d);

    /// Represents the logical "Media Pause" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_PAUSE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d2e);

    /// Represents the logical "Media Play" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_PLAY: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d2f);

    /// Represents the logical "Media Record" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_RECORD: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d30);

    /// Represents the logical "Media Rewind" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_REWIND: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d31);

    /// Represents the logical "Media Skip" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_SKIP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d32);

    /// Represents the logical "Next Favorite Channel" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NEXT_FAVORITE_CHANNEL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d33);

    /// Represents the logical "Next User Profile" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NEXT_USER_PROFILE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d34);

    /// Represents the logical "On Demand" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ON_DEMAND: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d35);

    /// Represents the logical "P In P Down" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const P_IN_P_DOWN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d36);

    /// Represents the logical "P In P Move" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const P_IN_P_MOVE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d37);

    /// Represents the logical "P In P Toggle" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const P_IN_P_TOGGLE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d38);

    /// Represents the logical "P In P Up" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const P_IN_P_UP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d39);

    /// Represents the logical "Play Speed Down" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PLAY_SPEED_DOWN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d3a);

    /// Represents the logical "Play Speed Reset" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PLAY_SPEED_RESET: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d3b);

    /// Represents the logical "Play Speed Up" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PLAY_SPEED_UP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d3c);

    /// Represents the logical "Random Toggle" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const RANDOM_TOGGLE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d3d);

    /// Represents the logical "Rc Low Battery" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const RC_LOW_BATTERY: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d3e);

    /// Represents the logical "Record Speed Next" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const RECORD_SPEED_NEXT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d3f);

    /// Represents the logical "Rf Bypass" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const RF_BYPASS: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d40);

    /// Represents the logical "Scan Channels Toggle" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SCAN_CHANNELS_TOGGLE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d41);

    /// Represents the logical "Screen Mode Next" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SCREEN_MODE_NEXT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d42);

    /// Represents the logical "Settings" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SETTINGS: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d43);

    /// Represents the logical "Split Screen Toggle" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SPLIT_SCREEN_TOGGLE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d44);

    /// Represents the logical "STB Input" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const STB_INPUT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d45);

    /// Represents the logical "STB Power" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const STB_POWER: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d46);

    /// Represents the logical "Subtitle" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SUBTITLE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d47);

    /// Represents the logical "Teletext" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TELETEXT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d48);

    /// Represents the logical "TV" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d49);

    /// Represents the logical "TV Input" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_INPUT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d4a);

    /// Represents the logical "TV Power" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_POWER: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d4b);

    /// Represents the logical "Video Mode Next" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const VIDEO_MODE_NEXT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d4c);

    /// Represents the logical "Wink" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const WINK: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d4d);

    /// Represents the logical "Zoom Toggle" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ZOOM_TOGGLE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d4e);

    /// Represents the logical "DVR" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const DVR: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d4f);

    /// Represents the logical "Media Audio Track" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_AUDIO_TRACK: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d50);

    /// Represents the logical "Media Skip Backward" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_SKIP_BACKWARD: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d51);

    /// Represents the logical "Media Skip Forward" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_SKIP_FORWARD: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d52);

    /// Represents the logical "Media Step Backward" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_STEP_BACKWARD: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d53);

    /// Represents the logical "Media Step Forward" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_STEP_FORWARD: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d54);

    /// Represents the logical "Media Top Menu" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_TOP_MENU: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d55);

    /// Represents the logical "Navigate In" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NAVIGATE_IN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d56);

    /// Represents the logical "Navigate Next" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NAVIGATE_NEXT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d57);

    /// Represents the logical "Navigate Out" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NAVIGATE_OUT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d58);

    /// Represents the logical "Navigate Previous" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NAVIGATE_PREVIOUS: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d59);

    /// Represents the logical "Pairing" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const PAIRING: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d5a);

    /// Represents the logical "Media Close" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MEDIA_CLOSE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000d5b);

    /// Represents the logical "Audio Bass Boost Toggle" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AUDIO_BASS_BOOST_TOGGLE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000e02);

    /// Represents the logical "Audio Treble Down" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AUDIO_TREBLE_DOWN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000e04);

    /// Represents the logical "Audio Treble Up" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const AUDIO_TREBLE_UP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000e05);

    /// Represents the logical "Microphone Toggle" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MICROPHONE_TOGGLE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000e06);

    /// Represents the logical "Microphone Volume Down" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MICROPHONE_VOLUME_DOWN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000e07);

    /// Represents the logical "Microphone Volume Up" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MICROPHONE_VOLUME_UP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000e08);

    /// Represents the logical "Microphone Volume Mute" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MICROPHONE_VOLUME_MUTE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000e09);

    /// Represents the logical "Speech Correction List" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SPEECH_CORRECTION_LIST: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000f01);

    /// Represents the logical "Speech Input Toggle" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SPEECH_INPUT_TOGGLE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100000f02);

    /// Represents the logical "App Switch" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const APP_SWITCH: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001001);

    /// Represents the logical "Call" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CALL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001002);

    /// Represents the logical "Camera Focus" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CAMERA_FOCUS: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001003);

    /// Represents the logical "End Call" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const END_CALL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001004);

    /// Represents the logical "Go Back" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GO_BACK: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001005);

    /// Represents the logical "Go Home" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GO_HOME: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001006);

    /// Represents the logical "Headset Hook" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const HEADSET_HOOK: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001007);

    /// Represents the logical "Last Number Redial" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LAST_NUMBER_REDIAL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001008);

    /// Represents the logical "Notification" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NOTIFICATION: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001009);

    /// Represents the logical "Manner Mode" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const MANNER_MODE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000100a);

    /// Represents the logical "Voice Dial" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const VOICE_DIAL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000100b);

    /// Represents the logical "TV 3 D Mode" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV3_D_MODE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001101);

    /// Represents the logical "TV Antenna Cable" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_ANTENNA_CABLE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001102);

    /// Represents the logical "TV Audio Description" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_AUDIO_DESCRIPTION: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001103);

    /// Represents the logical "TV Audio Description Mix Down" key on the
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_AUDIO_DESCRIPTION_MIX_DOWN: LogicalKeyboardKey =
        LogicalKeyboardKey::new(0x00100001104);

    /// Represents the logical "TV Audio Description Mix Up" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_AUDIO_DESCRIPTION_MIX_UP: LogicalKeyboardKey =
        LogicalKeyboardKey::new(0x00100001105);

    /// Represents the logical "TV Contents Menu" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_CONTENTS_MENU: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001106);

    /// Represents the logical "TV Data Service" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_DATA_SERVICE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001107);

    /// Represents the logical "TV Input Component 1" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_INPUT_COMPONENT1: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001108);

    /// Represents the logical "TV Input Component 2" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_INPUT_COMPONENT2: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001109);

    /// Represents the logical "TV Input Composite 1" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_INPUT_COMPOSITE1: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000110a);

    /// Represents the logical "TV Input Composite 2" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_INPUT_COMPOSITE2: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000110b);

    /// Represents the logical "TV Input HDMI 1" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_INPUT_HDMI1: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000110c);

    /// Represents the logical "TV Input HDMI 2" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_INPUT_HDMI2: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000110d);

    /// Represents the logical "TV Input HDMI 3" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_INPUT_HDMI3: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000110e);

    /// Represents the logical "TV Input HDMI 4" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_INPUT_HDMI4: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000110f);

    /// Represents the logical "TV Input VGA 1" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_INPUT_VGA1: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001110);

    /// Represents the logical "TV Media Context" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_MEDIA_CONTEXT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001111);

    /// Represents the logical "TV Network" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_NETWORK: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001112);

    /// Represents the logical "TV Number Entry" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_NUMBER_ENTRY: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001113);

    /// Represents the logical "TV Radio Service" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_RADIO_SERVICE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001114);

    /// Represents the logical "TV Satellite" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_SATELLITE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001115);

    /// Represents the logical "TV Satellite BS" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_SATELLITE_BS: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001116);

    /// Represents the logical "TV Satellite CS" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_SATELLITE_CS: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001117);

    /// Represents the logical "TV Satellite Toggle" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_SATELLITE_TOGGLE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001118);

    /// Represents the logical "TV Terrestrial Analog" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_TERRESTRIAL_ANALOG: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001119);

    /// Represents the logical "TV Terrestrial Digital" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_TERRESTRIAL_DIGITAL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000111a);

    /// Represents the logical "TV Timer" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const TV_TIMER: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0010000111b);

    /// Represents the logical "Key 11" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY11: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001201);

    /// Represents the logical "Key 12" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const KEY12: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00100001202);

    /// Represents the logical "Suspend" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SUSPEND: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000000);

    /// Represents the logical "Resume" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const RESUME: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000001);

    /// Represents the logical "Sleep" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SLEEP: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000002);

    /// Represents the logical "Abort" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ABORT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000003);

    /// Represents the logical "Lang 1" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LANG1: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000010);

    /// Represents the logical "Lang 2" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LANG2: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000011);

    /// Represents the logical "Lang 3" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LANG3: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000012);

    /// Represents the logical "Lang 4" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LANG4: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000013);

    /// Represents the logical "Lang 5" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const LANG5: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000014);

    /// Represents the logical "Intl Backslash" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const INTL_BACKSLASH: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000020);

    /// Represents the logical "Intl Ro" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const INTL_RO: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000021);

    /// Represents the logical "Intl Yen" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const INTL_YEN: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000022);

    /// Represents the logical "Control Left" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CONTROL_LEFT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000100);

    /// Represents the logical "Control Right" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const CONTROL_RIGHT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000101);

    /// Represents the logical "Shift Left" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SHIFT_LEFT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000102);

    /// Represents the logical "Shift Right" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const SHIFT_RIGHT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000103);

    /// Represents the logical "Alt Left" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ALT_LEFT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000104);

    /// Represents the logical "Alt Right" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const ALT_RIGHT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000105);

    /// Represents the logical "Meta Left" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const META_LEFT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000106);

    /// Represents the logical "Meta Right" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const META_RIGHT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000107);

    /// Represents the logical "Control" key on the keyboard.
    ///
    /// This key represents the union of the keys {controlLeft, controlRight} when
    /// comparing keys. This key will never be generated directly, its main use is
    /// in defining key maps.
    pub const CONTROL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x002000001f0);

    /// Represents the logical "Shift" key on the keyboard.
    ///
    /// This key represents the union of the keys {shiftLeft, shiftRight} when
    /// comparing keys. This key will never be generated directly, its main use is
    /// in defining key maps.
    pub const SHIFT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x002000001f2);

    /// Represents the logical "Alt" key on the keyboard.
    ///
    /// This key represents the union of the keys {altLeft, altRight} when
    /// comparing keys. This key will never be generated directly, its main use is
    /// in defining key maps.
    pub const ALT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x002000001f4);

    /// Represents the logical "Meta" key on the keyboard.
    ///
    /// This key represents the union of the keys {metaLeft, metaRight} when
    /// comparing keys. This key will never be generated directly, its main use is
    /// in defining key maps.
    pub const META: LogicalKeyboardKey = LogicalKeyboardKey::new(0x002000001f6);

    /// Represents the logical "Numpad Enter" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD_ENTER: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000020d);

    /// Represents the logical "Numpad Paren Left" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD_PAREN_LEFT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000228);

    /// Represents the logical "Numpad Paren Right" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD_PAREN_RIGHT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000229);

    /// Represents the logical "Numpad Multiply" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD_MULTIPLY: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000022a);

    /// Represents the logical "Numpad Add" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD_ADD: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000022b);

    /// Represents the logical "Numpad Comma" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD_COMMA: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000022c);

    /// Represents the logical "Numpad Subtract" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD_SUBTRACT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000022d);

    /// Represents the logical "Numpad Decimal" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD_DECIMAL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000022e);

    /// Represents the logical "Numpad Divide" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD_DIVIDE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000022f);

    /// Represents the logical "Numpad 0" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD0: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000230);

    /// Represents the logical "Numpad 1" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD1: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000231);

    /// Represents the logical "Numpad 2" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD2: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000232);

    /// Represents the logical "Numpad 3" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD3: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000233);

    /// Represents the logical "Numpad 4" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD4: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000234);

    /// Represents the logical "Numpad 5" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD5: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000235);

    /// Represents the logical "Numpad 6" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD6: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000236);

    /// Represents the logical "Numpad 7" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD7: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000237);

    /// Represents the logical "Numpad 8" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD8: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000238);

    /// Represents the logical "Numpad 9" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD9: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000239);

    /// Represents the logical "Numpad Equal" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const NUMPAD_EQUAL: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000023d);

    /// Represents the logical "Game Button 1" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON1: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000301);

    /// Represents the logical "Game Button 2" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON2: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000302);

    /// Represents the logical "Game Button 3" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON3: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000303);

    /// Represents the logical "Game Button 4" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON4: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000304);

    /// Represents the logical "Game Button 5" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON5: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000305);

    /// Represents the logical "Game Button 6" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON6: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000306);

    /// Represents the logical "Game Button 7" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON7: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000307);

    /// Represents the logical "Game Button 8" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON8: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000308);

    /// Represents the logical "Game Button 9" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON9: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000309);

    /// Represents the logical "Game Button 10" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON10: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000030a);

    /// Represents the logical "Game Button 11" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON11: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000030b);

    /// Represents the logical "Game Button 12" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON12: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000030c);

    /// Represents the logical "Game Button 13" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON13: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000030d);

    /// Represents the logical "Game Button 14" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON14: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000030e);

    /// Represents the logical "Game Button 15" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON15: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000030f);

    /// Represents the logical "Game Button 16" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON16: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000310);

    /// Represents the logical "Game Button A" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON_A: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000311);

    /// Represents the logical "Game Button B" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON_B: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000312);

    /// Represents the logical "Game Button C" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON_C: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000313);

    /// Represents the logical "Game Button Left 1" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON_LEFT1: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000314);

    /// Represents the logical "Game Button Left 2" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON_LEFT2: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000315);

    /// Represents the logical "Game Button Mode" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON_MODE: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000316);

    /// Represents the logical "Game Button Right 1" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON_RIGHT1: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000317);

    /// Represents the logical "Game Button Right 2" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON_RIGHT2: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000318);

    /// Represents the logical "Game Button Select" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON_SELECT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x00200000319);

    /// Represents the logical "Game Button Start" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON_START: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000031a);

    /// Represents the logical "Game Button Thumb Left" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON_THUMB_LEFT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000031b);

    /// Represents the logical "Game Button Thumb Right" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON_THUMB_RIGHT: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000031c);

    /// Represents the logical "Game Button X" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON_X: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000031d);

    /// Represents the logical "Game Button Y" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON_Y: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000031e);

    /// Represents the logical "Game Button Z" key on the keyboard.
    ///
    /// See the function `RawKeyEvent.logicalKey` for more information.
    pub const GAME_BUTTON_Z: LogicalKeyboardKey = LogicalKeyboardKey::new(0x0020000031f);

    /// All predefined constant [`LogicalKeyboardKey`]s.
    pub fn known_logical_keys() -> impl Iterator<Item = LogicalKeyboardKey> {
        KNOWN_LOGICAL_KEYS.iter().copied()
    }
}

impl Debug for LogicalKeyboardKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LogicalKeyboardKey(keyId: 0x{:08x}, keyLabel: {}",
            self.key_id,
            self.key_label()
        )?;
        if let Some(debug_name) = self.debug_name() {
            write!(f, ", debugName: {debug_name}")?;
        }
        f.write_str(")")
    }
}

/// Dart's `Set.expand`: every element is replaced by its entry in `table`, or kept
/// as itself when the table has none.
fn expand_with(
    input: &HashSet<LogicalKeyboardKey>,
    table: fn(LogicalKeyboardKey) -> Option<&'static [LogicalKeyboardKey]>,
) -> HashSet<LogicalKeyboardKey> {
    let mut result = HashSet::new();
    for &element in input {
        match table(element) {
            Some(replacements) => result.extend(replacements.iter().copied()),
            None => {
                result.insert(element);
            }
        }
    }
    result
}

/// A type with constants that describe the keys that are returned from
/// `RawKeyEvent.physicalKey`.
///
/// [`PhysicalKeyboardKey`]s are used to describe and test for keys in a particular
/// location on a keyboard. The keys are described by the USB HID usage code that
/// the key would produce, ignoring the key map, modifier keys (like SHIFT), and
/// the label on the key.
///
/// This is contrast to [`LogicalKeyboardKey`], which represents a logical key
/// interpreted in the context of modifiers, modes, and/or keyboard layouts.
///
/// For instance, if you wanted a game where the key next to the CAPS LOCK (the
/// "A" key on a QWERTY keyboard) moved the player to the left, you'd want to
/// look at the physical key to make sure that regardless of the character the
/// key produces, you got the key that is in that location on the keyboard.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PhysicalKeyboardKey {
    /// The unique USB HID usage ID of this physical key on the keyboard.
    ///
    /// Due to the variations in platform APIs, this may not be the actual HID
    /// usage code from the hardware, but a value derived from available
    /// information on the platform.
    ///
    /// See <https://www.usb.org/sites/default/files/documents/hut1_12v2.pdf>
    /// for the HID usage values and their meanings.
    pub usb_hid_usage: u64,
}

impl KeyboardKey for PhysicalKeyboardKey {}

impl PhysicalKeyboardKey {
    /// Creates a new [`PhysicalKeyboardKey`] for a USB HID usage.
    pub const fn new(usb_hid_usage: u64) -> PhysicalKeyboardKey {
        PhysicalKeyboardKey { usb_hid_usage }
    }

    /// The debug string to print for this keyboard key, which will be `None` in
    /// release mode.
    pub fn debug_name(&self) -> Option<String> {
        if !cfg!(debug_assertions) {
            return None;
        }
        Some(
            debug_name_of(self.usb_hid_usage)
                .map(str::to_owned)
                .unwrap_or_else(|| format!("Key with ID 0x{:08x}", self.usb_hid_usage)),
        )
    }

    /// Finds a known [`PhysicalKeyboardKey`] that matches the given USB HID usage
    /// code.
    pub fn find_key_by_code(usage_code: u64) -> Option<PhysicalKeyboardKey> {
        KNOWN_PHYSICAL_KEYS
            .binary_search_by_key(&usage_code, |key| key.usb_hid_usage)
            .ok()
            .map(|index| KNOWN_PHYSICAL_KEYS[index])
    }

    // Key constants for all keyboard keys in the USB HID specification at the
    // time Flutter was built.

    /// Represents the location of the "Hyper" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const HYPER: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00000010);

    /// Represents the location of the "Super Key" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const SUPER_KEY: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00000011);

    /// Represents the location of the "Fn" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const FN: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00000012);

    /// Represents the location of the "Fn Lock" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const FN_LOCK: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00000013);

    /// Represents the location of the "Suspend" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const SUSPEND: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00000014);

    /// Represents the location of the "Resume" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const RESUME: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00000015);

    /// Represents the location of the "Turbo" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const TURBO: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00000016);

    /// Represents the location of the "Privacy Screen Toggle" key on a
    /// generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const PRIVACY_SCREEN_TOGGLE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00000017);

    /// Represents the location of the "Microphone Mute Toggle" key on a
    /// generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const MICROPHONE_MUTE_TOGGLE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00000018);

    /// Represents the location of the "Sleep" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const SLEEP: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00010082);

    /// Represents the location of the "Wake Up" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const WAKE_UP: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00010083);

    /// Represents the location of the "Display Toggle Int Ext" key on a
    /// generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const DISPLAY_TOGGLE_INT_EXT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000100b5);

    /// Represents the location of the "Game Button 1" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON1: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff01);

    /// Represents the location of the "Game Button 2" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON2: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff02);

    /// Represents the location of the "Game Button 3" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON3: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff03);

    /// Represents the location of the "Game Button 4" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON4: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff04);

    /// Represents the location of the "Game Button 5" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON5: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff05);

    /// Represents the location of the "Game Button 6" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON6: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff06);

    /// Represents the location of the "Game Button 7" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON7: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff07);

    /// Represents the location of the "Game Button 8" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON8: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff08);

    /// Represents the location of the "Game Button 9" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON9: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff09);

    /// Represents the location of the "Game Button 10" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON10: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff0a);

    /// Represents the location of the "Game Button 11" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON11: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff0b);

    /// Represents the location of the "Game Button 12" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON12: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff0c);

    /// Represents the location of the "Game Button 13" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON13: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff0d);

    /// Represents the location of the "Game Button 14" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON14: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff0e);

    /// Represents the location of the "Game Button 15" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON15: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff0f);

    /// Represents the location of the "Game Button 16" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON16: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff10);

    /// Represents the location of the "Game Button A" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON_A: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff11);

    /// Represents the location of the "Game Button B" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON_B: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff12);

    /// Represents the location of the "Game Button C" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON_C: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff13);

    /// Represents the location of the "Game Button Left 1" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON_LEFT1: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff14);

    /// Represents the location of the "Game Button Left 2" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON_LEFT2: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff15);

    /// Represents the location of the "Game Button Mode" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON_MODE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff16);

    /// Represents the location of the "Game Button Right 1" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON_RIGHT1: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff17);

    /// Represents the location of the "Game Button Right 2" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON_RIGHT2: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff18);

    /// Represents the location of the "Game Button Select" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON_SELECT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff19);

    /// Represents the location of the "Game Button Start" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON_START: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff1a);

    /// Represents the location of the "Game Button Thumb Left" key on a
    /// generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON_THUMB_LEFT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff1b);

    /// Represents the location of the "Game Button Thumb Right" key on a
    /// generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON_THUMB_RIGHT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff1c);

    /// Represents the location of the "Game Button X" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON_X: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff1d);

    /// Represents the location of the "Game Button Y" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON_Y: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff1e);

    /// Represents the location of the "Game Button Z" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const GAME_BUTTON_Z: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0005ff1f);

    /// Represents the location of the "Usb Reserved" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const USB_RESERVED: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070000);

    /// Represents the location of the "Usb Error Roll Over" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const USB_ERROR_ROLL_OVER: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070001);

    /// Represents the location of the "Usb Post Fail" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const USB_POST_FAIL: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070002);

    /// Represents the location of the "Usb Error Undefined" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const USB_ERROR_UNDEFINED: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070003);

    /// Represents the location of the "Key A" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_A: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070004);

    /// Represents the location of the "Key B" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_B: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070005);

    /// Represents the location of the "Key C" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_C: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070006);

    /// Represents the location of the "Key D" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_D: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070007);

    /// Represents the location of the "Key E" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_E: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070008);

    /// Represents the location of the "Key F" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_F: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070009);

    /// Represents the location of the "Key G" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_G: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007000a);

    /// Represents the location of the "Key H" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_H: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007000b);

    /// Represents the location of the "Key I" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_I: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007000c);

    /// Represents the location of the "Key J" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_J: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007000d);

    /// Represents the location of the "Key K" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_K: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007000e);

    /// Represents the location of the "Key L" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_L: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007000f);

    /// Represents the location of the "Key M" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_M: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070010);

    /// Represents the location of the "Key N" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_N: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070011);

    /// Represents the location of the "Key O" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_O: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070012);

    /// Represents the location of the "Key P" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_P: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070013);

    /// Represents the location of the "Key Q" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_Q: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070014);

    /// Represents the location of the "Key R" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_R: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070015);

    /// Represents the location of the "Key S" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_S: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070016);

    /// Represents the location of the "Key T" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_T: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070017);

    /// Represents the location of the "Key U" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_U: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070018);

    /// Represents the location of the "Key V" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_V: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070019);

    /// Represents the location of the "Key W" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_W: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007001a);

    /// Represents the location of the "Key X" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_X: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007001b);

    /// Represents the location of the "Key Y" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_Y: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007001c);

    /// Represents the location of the "Key Z" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEY_Z: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007001d);

    /// Represents the location of the "Digit 1" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const DIGIT1: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007001e);

    /// Represents the location of the "Digit 2" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const DIGIT2: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007001f);

    /// Represents the location of the "Digit 3" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const DIGIT3: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070020);

    /// Represents the location of the "Digit 4" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const DIGIT4: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070021);

    /// Represents the location of the "Digit 5" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const DIGIT5: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070022);

    /// Represents the location of the "Digit 6" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const DIGIT6: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070023);

    /// Represents the location of the "Digit 7" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const DIGIT7: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070024);

    /// Represents the location of the "Digit 8" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const DIGIT8: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070025);

    /// Represents the location of the "Digit 9" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const DIGIT9: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070026);

    /// Represents the location of the "Digit 0" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const DIGIT0: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070027);

    /// Represents the location of the "Enter" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const ENTER: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070028);

    /// Represents the location of the "Escape" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const ESCAPE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070029);

    /// Represents the location of the "Backspace" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BACKSPACE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007002a);

    /// Represents the location of the "Tab" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const TAB: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007002b);

    /// Represents the location of the "Space" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const SPACE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007002c);

    /// Represents the location of the "Minus" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const MINUS: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007002d);

    /// Represents the location of the "Equal" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const EQUAL: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007002e);

    /// Represents the location of the "Bracket Left" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BRACKET_LEFT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007002f);

    /// Represents the location of the "Bracket Right" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BRACKET_RIGHT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070030);

    /// Represents the location of the "Backslash" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BACKSLASH: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070031);

    /// Represents the location of the "Semicolon" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const SEMICOLON: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070033);

    /// Represents the location of the "Quote" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const QUOTE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070034);

    /// Represents the location of the "Backquote" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BACKQUOTE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070035);

    /// Represents the location of the "Comma" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const COMMA: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070036);

    /// Represents the location of the "Period" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const PERIOD: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070037);

    /// Represents the location of the "Slash" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const SLASH: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070038);

    /// Represents the location of the "Caps Lock" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const CAPS_LOCK: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070039);

    /// Represents the location of the "F1" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F1: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007003a);

    /// Represents the location of the "F2" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F2: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007003b);

    /// Represents the location of the "F3" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F3: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007003c);

    /// Represents the location of the "F4" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F4: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007003d);

    /// Represents the location of the "F5" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F5: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007003e);

    /// Represents the location of the "F6" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F6: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007003f);

    /// Represents the location of the "F7" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F7: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070040);

    /// Represents the location of the "F8" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F8: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070041);

    /// Represents the location of the "F9" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F9: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070042);

    /// Represents the location of the "F10" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F10: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070043);

    /// Represents the location of the "F11" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F11: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070044);

    /// Represents the location of the "F12" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F12: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070045);

    /// Represents the location of the "Print Screen" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const PRINT_SCREEN: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070046);

    /// Represents the location of the "Scroll Lock" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const SCROLL_LOCK: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070047);

    /// Represents the location of the "Pause" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const PAUSE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070048);

    /// Represents the location of the "Insert" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const INSERT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070049);

    /// Represents the location of the "Home" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const HOME: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007004a);

    /// Represents the location of the "Page Up" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const PAGE_UP: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007004b);

    /// Represents the location of the "Delete" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const DELETE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007004c);

    /// Represents the location of the "End" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const END: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007004d);

    /// Represents the location of the "Page Down" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const PAGE_DOWN: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007004e);

    /// Represents the location of the "Arrow Right" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const ARROW_RIGHT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007004f);

    /// Represents the location of the "Arrow Left" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const ARROW_LEFT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070050);

    /// Represents the location of the "Arrow Down" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const ARROW_DOWN: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070051);

    /// Represents the location of the "Arrow Up" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const ARROW_UP: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070052);

    /// Represents the location of the "Num Lock" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUM_LOCK: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070053);

    /// Represents the location of the "Numpad Divide" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_DIVIDE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070054);

    /// Represents the location of the "Numpad Multiply" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_MULTIPLY: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070055);

    /// Represents the location of the "Numpad Subtract" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_SUBTRACT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070056);

    /// Represents the location of the "Numpad Add" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_ADD: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070057);

    /// Represents the location of the "Numpad Enter" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_ENTER: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070058);

    /// Represents the location of the "Numpad 1" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD1: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070059);

    /// Represents the location of the "Numpad 2" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD2: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007005a);

    /// Represents the location of the "Numpad 3" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD3: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007005b);

    /// Represents the location of the "Numpad 4" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD4: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007005c);

    /// Represents the location of the "Numpad 5" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD5: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007005d);

    /// Represents the location of the "Numpad 6" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD6: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007005e);

    /// Represents the location of the "Numpad 7" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD7: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007005f);

    /// Represents the location of the "Numpad 8" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD8: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070060);

    /// Represents the location of the "Numpad 9" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD9: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070061);

    /// Represents the location of the "Numpad 0" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD0: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070062);

    /// Represents the location of the "Numpad Decimal" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_DECIMAL: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070063);

    /// Represents the location of the "Intl Backslash" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const INTL_BACKSLASH: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070064);

    /// Represents the location of the "Context Menu" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const CONTEXT_MENU: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070065);

    /// Represents the location of the "Power" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const POWER: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070066);

    /// Represents the location of the "Numpad Equal" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_EQUAL: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070067);

    /// Represents the location of the "F13" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F13: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070068);

    /// Represents the location of the "F14" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F14: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070069);

    /// Represents the location of the "F15" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F15: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007006a);

    /// Represents the location of the "F16" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F16: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007006b);

    /// Represents the location of the "F17" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F17: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007006c);

    /// Represents the location of the "F18" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F18: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007006d);

    /// Represents the location of the "F19" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F19: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007006e);

    /// Represents the location of the "F20" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F20: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007006f);

    /// Represents the location of the "F21" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F21: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070070);

    /// Represents the location of the "F22" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F22: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070071);

    /// Represents the location of the "F23" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F23: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070072);

    /// Represents the location of the "F24" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const F24: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070073);

    /// Represents the location of the "Open" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const OPEN: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070074);

    /// Represents the location of the "Help" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const HELP: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070075);

    /// Represents the location of the "Select" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const SELECT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070077);

    /// Represents the location of the "Again" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const AGAIN: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070079);

    /// Represents the location of the "Undo" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const UNDO: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007007a);

    /// Represents the location of the "Cut" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const CUT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007007b);

    /// Represents the location of the "Copy" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const COPY: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007007c);

    /// Represents the location of the "Paste" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const PASTE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007007d);

    /// Represents the location of the "Find" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const FIND: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007007e);

    /// Represents the location of the "Audio Volume Mute" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const AUDIO_VOLUME_MUTE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007007f);

    /// Represents the location of the "Audio Volume Up" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const AUDIO_VOLUME_UP: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070080);

    /// Represents the location of the "Audio Volume Down" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const AUDIO_VOLUME_DOWN: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070081);

    /// Represents the location of the "Numpad Comma" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_COMMA: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070085);

    /// Represents the location of the "Intl Ro" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const INTL_RO: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070087);

    /// Represents the location of the "Kana Mode" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KANA_MODE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070088);

    /// Represents the location of the "Intl Yen" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const INTL_YEN: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070089);

    /// Represents the location of the "Convert" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const CONVERT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007008a);

    /// Represents the location of the "Non Convert" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NON_CONVERT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007008b);

    /// Represents the location of the "Lang 1" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LANG1: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070090);

    /// Represents the location of the "Lang 2" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LANG2: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070091);

    /// Represents the location of the "Lang 3" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LANG3: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070092);

    /// Represents the location of the "Lang 4" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LANG4: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070093);

    /// Represents the location of the "Lang 5" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LANG5: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x00070094);

    /// Represents the location of the "Abort" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const ABORT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x0007009b);

    /// Represents the location of the "Props" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const PROPS: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700a3);

    /// Represents the location of the "Numpad Paren Left" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_PAREN_LEFT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700b6);

    /// Represents the location of the "Numpad Paren Right" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_PAREN_RIGHT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700b7);

    /// Represents the location of the "Numpad Backspace" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_BACKSPACE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700bb);

    /// Represents the location of the "Numpad Memory Store" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_MEMORY_STORE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700d0);

    /// Represents the location of the "Numpad Memory Recall" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_MEMORY_RECALL: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700d1);

    /// Represents the location of the "Numpad Memory Clear" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_MEMORY_CLEAR: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700d2);

    /// Represents the location of the "Numpad Memory Add" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_MEMORY_ADD: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700d3);

    /// Represents the location of the "Numpad Memory Subtract" key on a
    /// generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_MEMORY_SUBTRACT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700d4);

    /// Represents the location of the "Numpad Sign Change" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_SIGN_CHANGE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700d7);

    /// Represents the location of the "Numpad Clear" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_CLEAR: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700d8);

    /// Represents the location of the "Numpad Clear Entry" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NUMPAD_CLEAR_ENTRY: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700d9);

    /// Represents the location of the "Control Left" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const CONTROL_LEFT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700e0);

    /// Represents the location of the "Shift Left" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const SHIFT_LEFT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700e1);

    /// Represents the location of the "Alt Left" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const ALT_LEFT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700e2);

    /// Represents the location of the "Meta Left" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const META_LEFT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700e3);

    /// Represents the location of the "Control Right" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const CONTROL_RIGHT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700e4);

    /// Represents the location of the "Shift Right" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const SHIFT_RIGHT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700e5);

    /// Represents the location of the "Alt Right" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const ALT_RIGHT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700e6);

    /// Represents the location of the "Meta Right" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const META_RIGHT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000700e7);

    /// Represents the location of the "Info" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const INFO: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0060);

    /// Represents the location of the "Closed Caption Toggle" key on a
    /// generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const CLOSED_CAPTION_TOGGLE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0061);

    /// Represents the location of the "Brightness Up" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BRIGHTNESS_UP: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c006f);

    /// Represents the location of the "Brightness Down" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BRIGHTNESS_DOWN: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0070);

    /// Represents the location of the "Brightness Toggle" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BRIGHTNESS_TOGGLE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0072);

    /// Represents the location of the "Brightness Minimum" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BRIGHTNESS_MINIMUM: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0073);

    /// Represents the location of the "Brightness Maximum" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BRIGHTNESS_MAXIMUM: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0074);

    /// Represents the location of the "Brightness Auto" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BRIGHTNESS_AUTO: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0075);

    /// Represents the location of the "Kbd Illum Up" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KBD_ILLUM_UP: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0079);

    /// Represents the location of the "Kbd Illum Down" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KBD_ILLUM_DOWN: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c007a);

    /// Represents the location of the "Media Last" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const MEDIA_LAST: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0083);

    /// Represents the location of the "Launch Phone" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LAUNCH_PHONE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c008c);

    /// Represents the location of the "Program Guide" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const PROGRAM_GUIDE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c008d);

    /// Represents the location of the "Exit" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const EXIT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0094);

    /// Represents the location of the "Channel Up" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const CHANNEL_UP: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c009c);

    /// Represents the location of the "Channel Down" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const CHANNEL_DOWN: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c009d);

    /// Represents the location of the "Media Play" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const MEDIA_PLAY: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c00b0);

    /// Represents the location of the "Media Pause" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const MEDIA_PAUSE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c00b1);

    /// Represents the location of the "Media Record" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const MEDIA_RECORD: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c00b2);

    /// Represents the location of the "Media Fast Forward" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const MEDIA_FAST_FORWARD: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c00b3);

    /// Represents the location of the "Media Rewind" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const MEDIA_REWIND: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c00b4);

    /// Represents the location of the "Media Track Next" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const MEDIA_TRACK_NEXT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c00b5);

    /// Represents the location of the "Media Track Previous" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const MEDIA_TRACK_PREVIOUS: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c00b6);

    /// Represents the location of the "Media Stop" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const MEDIA_STOP: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c00b7);

    /// Represents the location of the "Eject" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const EJECT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c00b8);

    /// Represents the location of the "Media Play Pause" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const MEDIA_PLAY_PAUSE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c00cd);

    /// Represents the location of the "Speech Input Toggle" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const SPEECH_INPUT_TOGGLE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c00cf);

    /// Represents the location of the "Bass Boost" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BASS_BOOST: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c00e5);

    /// Represents the location of the "Media Select" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const MEDIA_SELECT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0183);

    /// Represents the location of the "Launch Word Processor" key on a
    /// generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LAUNCH_WORD_PROCESSOR: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0184);

    /// Represents the location of the "Launch Spreadsheet" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LAUNCH_SPREADSHEET: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0186);

    /// Represents the location of the "Launch Mail" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LAUNCH_MAIL: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c018a);

    /// Represents the location of the "Launch Contacts" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LAUNCH_CONTACTS: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c018d);

    /// Represents the location of the "Launch Calendar" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LAUNCH_CALENDAR: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c018e);

    /// Represents the location of the "Launch App2" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LAUNCH_APP2: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0192);

    /// Represents the location of the "Launch App1" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LAUNCH_APP1: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0194);

    /// Represents the location of the "Launch Internet Browser" key on a
    /// generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LAUNCH_INTERNET_BROWSER: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0196);

    /// Represents the location of the "Log Off" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LOG_OFF: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c019c);

    /// Represents the location of the "Lock Screen" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LOCK_SCREEN: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c019e);

    /// Represents the location of the "Launch Control Panel" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LAUNCH_CONTROL_PANEL: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c019f);

    /// Represents the location of the "Select Task" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const SELECT_TASK: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c01a2);

    /// Represents the location of the "Launch Documents" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LAUNCH_DOCUMENTS: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c01a7);

    /// Represents the location of the "Spell Check" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const SPELL_CHECK: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c01ab);

    /// Represents the location of the "Launch Keyboard Layout" key on a
    /// generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LAUNCH_KEYBOARD_LAYOUT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c01ae);

    /// Represents the location of the "Launch Screen Saver" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LAUNCH_SCREEN_SAVER: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c01b1);

    /// Represents the location of the "Launch Audio Browser" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LAUNCH_AUDIO_BROWSER: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c01b7);

    /// Represents the location of the "Launch Assistant" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const LAUNCH_ASSISTANT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c01cb);

    /// Represents the location of the "New Key" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const NEW_KEY: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0201);

    /// Represents the location of the "Close" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const CLOSE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0203);

    /// Represents the location of the "Save" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const SAVE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0207);

    /// Represents the location of the "Print" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const PRINT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0208);

    /// Represents the location of the "Browser Search" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BROWSER_SEARCH: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0221);

    /// Represents the location of the "Browser Home" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BROWSER_HOME: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0223);

    /// Represents the location of the "Browser Back" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BROWSER_BACK: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0224);

    /// Represents the location of the "Browser Forward" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BROWSER_FORWARD: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0225);

    /// Represents the location of the "Browser Stop" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BROWSER_STOP: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0226);

    /// Represents the location of the "Browser Refresh" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BROWSER_REFRESH: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0227);

    /// Represents the location of the "Browser Favorites" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const BROWSER_FAVORITES: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c022a);

    /// Represents the location of the "Zoom In" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const ZOOM_IN: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c022d);

    /// Represents the location of the "Zoom Out" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const ZOOM_OUT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c022e);

    /// Represents the location of the "Zoom Toggle" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const ZOOM_TOGGLE: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0232);

    /// Represents the location of the "Redo" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const REDO: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0279);

    /// Represents the location of the "Mail Reply" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const MAIL_REPLY: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c0289);

    /// Represents the location of the "Mail Forward" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const MAIL_FORWARD: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c028b);

    /// Represents the location of the "Mail Send" key on a generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const MAIL_SEND: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c028c);

    /// Represents the location of the "Keyboard Layout Select" key on a
    /// generalized keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const KEYBOARD_LAYOUT_SELECT: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c029d);

    /// Represents the location of the "Show All Windows" key on a generalized
    /// keyboard.
    ///
    /// See the function `RawKeyEvent.physicalKey` for more information.
    pub const SHOW_ALL_WINDOWS: PhysicalKeyboardKey = PhysicalKeyboardKey::new(0x000c029f);

    /// All predefined constant [`PhysicalKeyboardKey`]s.
    pub fn known_physical_keys() -> impl Iterator<Item = PhysicalKeyboardKey> {
        KNOWN_PHYSICAL_KEYS.iter().copied()
    }
}

impl Debug for PhysicalKeyboardKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PhysicalKeyboardKey(usbHidUsage: 0x{:08x}",
            self.usb_hid_usage
        )?;
        if let Some(debug_name) = self.debug_name() {
            write!(f, ", debugName: {debug_name}")?;
        }
        f.write_str(")")
    }
}

/// Dart's `LogicalKeyboardKey._keyLabels`, sorted by key ID.
fn key_label_of(key_id: u64) -> Option<&'static str> {
    KEY_LABELS
        .binary_search_by_key(&key_id, |&(id, _)| id)
        .ok()
        .map(|index| KEY_LABELS[index].1)
}

/// Dart's `PhysicalKeyboardKey._debugNames`, sorted by USB HID usage.
fn debug_name_of(usb_hid_usage: u64) -> Option<&'static str> {
    DEBUG_NAMES
        .binary_search_by_key(&usb_hid_usage, |&(usage, _)| usage)
        .ok()
        .map(|index| DEBUG_NAMES[index].1)
}

/// Dart's `LogicalKeyboardKey._synonyms`: a key to the pseudo-key synonym for it.
fn synonyms_of(key: LogicalKeyboardKey) -> Option<&'static [LogicalKeyboardKey]> {
    SYNONYMS
        .iter()
        .find(|&&(candidate, _)| candidate == key)
        .map(|&(_, keys)| keys)
}

/// Dart's `LogicalKeyboardKey._reverseSynonyms`: a pseudo-key to the keys it stands for.
fn reverse_synonyms_of(key: LogicalKeyboardKey) -> Option<&'static [LogicalKeyboardKey]> {
    REVERSE_SYNONYMS
        .iter()
        .find(|&&(candidate, _)| candidate == key)
        .map(|&(_, keys)| keys)
}

/// All the predefined constant [`LogicalKeyboardKey`]s, sorted by key ID so they
/// can be searched.
static KNOWN_LOGICAL_KEYS: &[LogicalKeyboardKey] = &[
    LogicalKeyboardKey::SPACE,
    LogicalKeyboardKey::EXCLAMATION,
    LogicalKeyboardKey::QUOTE,
    LogicalKeyboardKey::NUMBER_SIGN,
    LogicalKeyboardKey::DOLLAR,
    LogicalKeyboardKey::PERCENT,
    LogicalKeyboardKey::AMPERSAND,
    LogicalKeyboardKey::QUOTE_SINGLE,
    LogicalKeyboardKey::PARENTHESIS_LEFT,
    LogicalKeyboardKey::PARENTHESIS_RIGHT,
    LogicalKeyboardKey::ASTERISK,
    LogicalKeyboardKey::ADD,
    LogicalKeyboardKey::COMMA,
    LogicalKeyboardKey::MINUS,
    LogicalKeyboardKey::PERIOD,
    LogicalKeyboardKey::SLASH,
    LogicalKeyboardKey::DIGIT0,
    LogicalKeyboardKey::DIGIT1,
    LogicalKeyboardKey::DIGIT2,
    LogicalKeyboardKey::DIGIT3,
    LogicalKeyboardKey::DIGIT4,
    LogicalKeyboardKey::DIGIT5,
    LogicalKeyboardKey::DIGIT6,
    LogicalKeyboardKey::DIGIT7,
    LogicalKeyboardKey::DIGIT8,
    LogicalKeyboardKey::DIGIT9,
    LogicalKeyboardKey::COLON,
    LogicalKeyboardKey::SEMICOLON,
    LogicalKeyboardKey::LESS,
    LogicalKeyboardKey::EQUAL,
    LogicalKeyboardKey::GREATER,
    LogicalKeyboardKey::QUESTION,
    LogicalKeyboardKey::AT,
    LogicalKeyboardKey::BRACKET_LEFT,
    LogicalKeyboardKey::BACKSLASH,
    LogicalKeyboardKey::BRACKET_RIGHT,
    LogicalKeyboardKey::CARET,
    LogicalKeyboardKey::UNDERSCORE,
    LogicalKeyboardKey::BACKQUOTE,
    LogicalKeyboardKey::KEY_A,
    LogicalKeyboardKey::KEY_B,
    LogicalKeyboardKey::KEY_C,
    LogicalKeyboardKey::KEY_D,
    LogicalKeyboardKey::KEY_E,
    LogicalKeyboardKey::KEY_F,
    LogicalKeyboardKey::KEY_G,
    LogicalKeyboardKey::KEY_H,
    LogicalKeyboardKey::KEY_I,
    LogicalKeyboardKey::KEY_J,
    LogicalKeyboardKey::KEY_K,
    LogicalKeyboardKey::KEY_L,
    LogicalKeyboardKey::KEY_M,
    LogicalKeyboardKey::KEY_N,
    LogicalKeyboardKey::KEY_O,
    LogicalKeyboardKey::KEY_P,
    LogicalKeyboardKey::KEY_Q,
    LogicalKeyboardKey::KEY_R,
    LogicalKeyboardKey::KEY_S,
    LogicalKeyboardKey::KEY_T,
    LogicalKeyboardKey::KEY_U,
    LogicalKeyboardKey::KEY_V,
    LogicalKeyboardKey::KEY_W,
    LogicalKeyboardKey::KEY_X,
    LogicalKeyboardKey::KEY_Y,
    LogicalKeyboardKey::KEY_Z,
    LogicalKeyboardKey::BRACE_LEFT,
    LogicalKeyboardKey::BAR,
    LogicalKeyboardKey::BRACE_RIGHT,
    LogicalKeyboardKey::TILDE,
    LogicalKeyboardKey::UNIDENTIFIED,
    LogicalKeyboardKey::BACKSPACE,
    LogicalKeyboardKey::TAB,
    LogicalKeyboardKey::ENTER,
    LogicalKeyboardKey::ESCAPE,
    LogicalKeyboardKey::DELETE,
    LogicalKeyboardKey::ACCEL,
    LogicalKeyboardKey::ALT_GRAPH,
    LogicalKeyboardKey::CAPS_LOCK,
    LogicalKeyboardKey::FN,
    LogicalKeyboardKey::FN_LOCK,
    LogicalKeyboardKey::HYPER,
    LogicalKeyboardKey::NUM_LOCK,
    LogicalKeyboardKey::SCROLL_LOCK,
    LogicalKeyboardKey::SUPER_KEY,
    LogicalKeyboardKey::SYMBOL,
    LogicalKeyboardKey::SYMBOL_LOCK,
    LogicalKeyboardKey::SHIFT_LEVEL5,
    LogicalKeyboardKey::ARROW_DOWN,
    LogicalKeyboardKey::ARROW_LEFT,
    LogicalKeyboardKey::ARROW_RIGHT,
    LogicalKeyboardKey::ARROW_UP,
    LogicalKeyboardKey::END,
    LogicalKeyboardKey::HOME,
    LogicalKeyboardKey::PAGE_DOWN,
    LogicalKeyboardKey::PAGE_UP,
    LogicalKeyboardKey::CLEAR,
    LogicalKeyboardKey::COPY,
    LogicalKeyboardKey::CR_SEL,
    LogicalKeyboardKey::CUT,
    LogicalKeyboardKey::ERASE_EOF,
    LogicalKeyboardKey::EX_SEL,
    LogicalKeyboardKey::INSERT,
    LogicalKeyboardKey::PASTE,
    LogicalKeyboardKey::REDO,
    LogicalKeyboardKey::UNDO,
    LogicalKeyboardKey::ACCEPT,
    LogicalKeyboardKey::AGAIN,
    LogicalKeyboardKey::ATTN,
    LogicalKeyboardKey::CANCEL,
    LogicalKeyboardKey::CONTEXT_MENU,
    LogicalKeyboardKey::EXECUTE,
    LogicalKeyboardKey::FIND,
    LogicalKeyboardKey::HELP,
    LogicalKeyboardKey::PAUSE,
    LogicalKeyboardKey::PLAY,
    LogicalKeyboardKey::PROPS,
    LogicalKeyboardKey::SELECT,
    LogicalKeyboardKey::ZOOM_IN,
    LogicalKeyboardKey::ZOOM_OUT,
    LogicalKeyboardKey::BRIGHTNESS_DOWN,
    LogicalKeyboardKey::BRIGHTNESS_UP,
    LogicalKeyboardKey::CAMERA,
    LogicalKeyboardKey::EJECT,
    LogicalKeyboardKey::LOG_OFF,
    LogicalKeyboardKey::POWER,
    LogicalKeyboardKey::POWER_OFF,
    LogicalKeyboardKey::PRINT_SCREEN,
    LogicalKeyboardKey::HIBERNATE,
    LogicalKeyboardKey::STANDBY,
    LogicalKeyboardKey::WAKE_UP,
    LogicalKeyboardKey::ALL_CANDIDATES,
    LogicalKeyboardKey::ALPHANUMERIC,
    LogicalKeyboardKey::CODE_INPUT,
    LogicalKeyboardKey::COMPOSE,
    LogicalKeyboardKey::CONVERT,
    LogicalKeyboardKey::FINAL_MODE,
    LogicalKeyboardKey::GROUP_FIRST,
    LogicalKeyboardKey::GROUP_LAST,
    LogicalKeyboardKey::GROUP_NEXT,
    LogicalKeyboardKey::GROUP_PREVIOUS,
    LogicalKeyboardKey::MODE_CHANGE,
    LogicalKeyboardKey::NEXT_CANDIDATE,
    LogicalKeyboardKey::NON_CONVERT,
    LogicalKeyboardKey::PREVIOUS_CANDIDATE,
    LogicalKeyboardKey::PROCESS,
    LogicalKeyboardKey::SINGLE_CANDIDATE,
    LogicalKeyboardKey::HANGUL_MODE,
    LogicalKeyboardKey::HANJA_MODE,
    LogicalKeyboardKey::JUNJA_MODE,
    LogicalKeyboardKey::EISU,
    LogicalKeyboardKey::HANKAKU,
    LogicalKeyboardKey::HIRAGANA,
    LogicalKeyboardKey::HIRAGANA_KATAKANA,
    LogicalKeyboardKey::KANA_MODE,
    LogicalKeyboardKey::KANJI_MODE,
    LogicalKeyboardKey::KATAKANA,
    LogicalKeyboardKey::ROMAJI,
    LogicalKeyboardKey::ZENKAKU,
    LogicalKeyboardKey::ZENKAKU_HANKAKU,
    LogicalKeyboardKey::F1,
    LogicalKeyboardKey::F2,
    LogicalKeyboardKey::F3,
    LogicalKeyboardKey::F4,
    LogicalKeyboardKey::F5,
    LogicalKeyboardKey::F6,
    LogicalKeyboardKey::F7,
    LogicalKeyboardKey::F8,
    LogicalKeyboardKey::F9,
    LogicalKeyboardKey::F10,
    LogicalKeyboardKey::F11,
    LogicalKeyboardKey::F12,
    LogicalKeyboardKey::F13,
    LogicalKeyboardKey::F14,
    LogicalKeyboardKey::F15,
    LogicalKeyboardKey::F16,
    LogicalKeyboardKey::F17,
    LogicalKeyboardKey::F18,
    LogicalKeyboardKey::F19,
    LogicalKeyboardKey::F20,
    LogicalKeyboardKey::F21,
    LogicalKeyboardKey::F22,
    LogicalKeyboardKey::F23,
    LogicalKeyboardKey::F24,
    LogicalKeyboardKey::SOFT1,
    LogicalKeyboardKey::SOFT2,
    LogicalKeyboardKey::SOFT3,
    LogicalKeyboardKey::SOFT4,
    LogicalKeyboardKey::SOFT5,
    LogicalKeyboardKey::SOFT6,
    LogicalKeyboardKey::SOFT7,
    LogicalKeyboardKey::SOFT8,
    LogicalKeyboardKey::CLOSE,
    LogicalKeyboardKey::MAIL_FORWARD,
    LogicalKeyboardKey::MAIL_REPLY,
    LogicalKeyboardKey::MAIL_SEND,
    LogicalKeyboardKey::MEDIA_PLAY_PAUSE,
    LogicalKeyboardKey::MEDIA_STOP,
    LogicalKeyboardKey::MEDIA_TRACK_NEXT,
    LogicalKeyboardKey::MEDIA_TRACK_PREVIOUS,
    LogicalKeyboardKey::NEW_KEY,
    LogicalKeyboardKey::OPEN,
    LogicalKeyboardKey::PRINT,
    LogicalKeyboardKey::SAVE,
    LogicalKeyboardKey::SPELL_CHECK,
    LogicalKeyboardKey::AUDIO_VOLUME_DOWN,
    LogicalKeyboardKey::AUDIO_VOLUME_UP,
    LogicalKeyboardKey::AUDIO_VOLUME_MUTE,
    LogicalKeyboardKey::LAUNCH_APPLICATION2,
    LogicalKeyboardKey::LAUNCH_CALENDAR,
    LogicalKeyboardKey::LAUNCH_MAIL,
    LogicalKeyboardKey::LAUNCH_MEDIA_PLAYER,
    LogicalKeyboardKey::LAUNCH_MUSIC_PLAYER,
    LogicalKeyboardKey::LAUNCH_APPLICATION1,
    LogicalKeyboardKey::LAUNCH_SCREEN_SAVER,
    LogicalKeyboardKey::LAUNCH_SPREADSHEET,
    LogicalKeyboardKey::LAUNCH_WEB_BROWSER,
    LogicalKeyboardKey::LAUNCH_WEB_CAM,
    LogicalKeyboardKey::LAUNCH_WORD_PROCESSOR,
    LogicalKeyboardKey::LAUNCH_CONTACTS,
    LogicalKeyboardKey::LAUNCH_PHONE,
    LogicalKeyboardKey::LAUNCH_ASSISTANT,
    LogicalKeyboardKey::LAUNCH_CONTROL_PANEL,
    LogicalKeyboardKey::BROWSER_BACK,
    LogicalKeyboardKey::BROWSER_FAVORITES,
    LogicalKeyboardKey::BROWSER_FORWARD,
    LogicalKeyboardKey::BROWSER_HOME,
    LogicalKeyboardKey::BROWSER_REFRESH,
    LogicalKeyboardKey::BROWSER_SEARCH,
    LogicalKeyboardKey::BROWSER_STOP,
    LogicalKeyboardKey::AUDIO_BALANCE_LEFT,
    LogicalKeyboardKey::AUDIO_BALANCE_RIGHT,
    LogicalKeyboardKey::AUDIO_BASS_BOOST_DOWN,
    LogicalKeyboardKey::AUDIO_BASS_BOOST_UP,
    LogicalKeyboardKey::AUDIO_FADER_FRONT,
    LogicalKeyboardKey::AUDIO_FADER_REAR,
    LogicalKeyboardKey::AUDIO_SURROUND_MODE_NEXT,
    LogicalKeyboardKey::AVR_INPUT,
    LogicalKeyboardKey::AVR_POWER,
    LogicalKeyboardKey::CHANNEL_DOWN,
    LogicalKeyboardKey::CHANNEL_UP,
    LogicalKeyboardKey::COLOR_F0_RED,
    LogicalKeyboardKey::COLOR_F1_GREEN,
    LogicalKeyboardKey::COLOR_F2_YELLOW,
    LogicalKeyboardKey::COLOR_F3_BLUE,
    LogicalKeyboardKey::COLOR_F4_GREY,
    LogicalKeyboardKey::COLOR_F5_BROWN,
    LogicalKeyboardKey::CLOSED_CAPTION_TOGGLE,
    LogicalKeyboardKey::DIMMER,
    LogicalKeyboardKey::DISPLAY_SWAP,
    LogicalKeyboardKey::EXIT,
    LogicalKeyboardKey::FAVORITE_CLEAR0,
    LogicalKeyboardKey::FAVORITE_CLEAR1,
    LogicalKeyboardKey::FAVORITE_CLEAR2,
    LogicalKeyboardKey::FAVORITE_CLEAR3,
    LogicalKeyboardKey::FAVORITE_RECALL0,
    LogicalKeyboardKey::FAVORITE_RECALL1,
    LogicalKeyboardKey::FAVORITE_RECALL2,
    LogicalKeyboardKey::FAVORITE_RECALL3,
    LogicalKeyboardKey::FAVORITE_STORE0,
    LogicalKeyboardKey::FAVORITE_STORE1,
    LogicalKeyboardKey::FAVORITE_STORE2,
    LogicalKeyboardKey::FAVORITE_STORE3,
    LogicalKeyboardKey::GUIDE,
    LogicalKeyboardKey::GUIDE_NEXT_DAY,
    LogicalKeyboardKey::GUIDE_PREVIOUS_DAY,
    LogicalKeyboardKey::INFO,
    LogicalKeyboardKey::INSTANT_REPLAY,
    LogicalKeyboardKey::LINK,
    LogicalKeyboardKey::LIST_PROGRAM,
    LogicalKeyboardKey::LIVE_CONTENT,
    LogicalKeyboardKey::LOCK,
    LogicalKeyboardKey::MEDIA_APPS,
    LogicalKeyboardKey::MEDIA_FAST_FORWARD,
    LogicalKeyboardKey::MEDIA_LAST,
    LogicalKeyboardKey::MEDIA_PAUSE,
    LogicalKeyboardKey::MEDIA_PLAY,
    LogicalKeyboardKey::MEDIA_RECORD,
    LogicalKeyboardKey::MEDIA_REWIND,
    LogicalKeyboardKey::MEDIA_SKIP,
    LogicalKeyboardKey::NEXT_FAVORITE_CHANNEL,
    LogicalKeyboardKey::NEXT_USER_PROFILE,
    LogicalKeyboardKey::ON_DEMAND,
    LogicalKeyboardKey::P_IN_P_DOWN,
    LogicalKeyboardKey::P_IN_P_MOVE,
    LogicalKeyboardKey::P_IN_P_TOGGLE,
    LogicalKeyboardKey::P_IN_P_UP,
    LogicalKeyboardKey::PLAY_SPEED_DOWN,
    LogicalKeyboardKey::PLAY_SPEED_RESET,
    LogicalKeyboardKey::PLAY_SPEED_UP,
    LogicalKeyboardKey::RANDOM_TOGGLE,
    LogicalKeyboardKey::RC_LOW_BATTERY,
    LogicalKeyboardKey::RECORD_SPEED_NEXT,
    LogicalKeyboardKey::RF_BYPASS,
    LogicalKeyboardKey::SCAN_CHANNELS_TOGGLE,
    LogicalKeyboardKey::SCREEN_MODE_NEXT,
    LogicalKeyboardKey::SETTINGS,
    LogicalKeyboardKey::SPLIT_SCREEN_TOGGLE,
    LogicalKeyboardKey::STB_INPUT,
    LogicalKeyboardKey::STB_POWER,
    LogicalKeyboardKey::SUBTITLE,
    LogicalKeyboardKey::TELETEXT,
    LogicalKeyboardKey::TV,
    LogicalKeyboardKey::TV_INPUT,
    LogicalKeyboardKey::TV_POWER,
    LogicalKeyboardKey::VIDEO_MODE_NEXT,
    LogicalKeyboardKey::WINK,
    LogicalKeyboardKey::ZOOM_TOGGLE,
    LogicalKeyboardKey::DVR,
    LogicalKeyboardKey::MEDIA_AUDIO_TRACK,
    LogicalKeyboardKey::MEDIA_SKIP_BACKWARD,
    LogicalKeyboardKey::MEDIA_SKIP_FORWARD,
    LogicalKeyboardKey::MEDIA_STEP_BACKWARD,
    LogicalKeyboardKey::MEDIA_STEP_FORWARD,
    LogicalKeyboardKey::MEDIA_TOP_MENU,
    LogicalKeyboardKey::NAVIGATE_IN,
    LogicalKeyboardKey::NAVIGATE_NEXT,
    LogicalKeyboardKey::NAVIGATE_OUT,
    LogicalKeyboardKey::NAVIGATE_PREVIOUS,
    LogicalKeyboardKey::PAIRING,
    LogicalKeyboardKey::MEDIA_CLOSE,
    LogicalKeyboardKey::AUDIO_BASS_BOOST_TOGGLE,
    LogicalKeyboardKey::AUDIO_TREBLE_DOWN,
    LogicalKeyboardKey::AUDIO_TREBLE_UP,
    LogicalKeyboardKey::MICROPHONE_TOGGLE,
    LogicalKeyboardKey::MICROPHONE_VOLUME_DOWN,
    LogicalKeyboardKey::MICROPHONE_VOLUME_UP,
    LogicalKeyboardKey::MICROPHONE_VOLUME_MUTE,
    LogicalKeyboardKey::SPEECH_CORRECTION_LIST,
    LogicalKeyboardKey::SPEECH_INPUT_TOGGLE,
    LogicalKeyboardKey::APP_SWITCH,
    LogicalKeyboardKey::CALL,
    LogicalKeyboardKey::CAMERA_FOCUS,
    LogicalKeyboardKey::END_CALL,
    LogicalKeyboardKey::GO_BACK,
    LogicalKeyboardKey::GO_HOME,
    LogicalKeyboardKey::HEADSET_HOOK,
    LogicalKeyboardKey::LAST_NUMBER_REDIAL,
    LogicalKeyboardKey::NOTIFICATION,
    LogicalKeyboardKey::MANNER_MODE,
    LogicalKeyboardKey::VOICE_DIAL,
    LogicalKeyboardKey::TV3_D_MODE,
    LogicalKeyboardKey::TV_ANTENNA_CABLE,
    LogicalKeyboardKey::TV_AUDIO_DESCRIPTION,
    LogicalKeyboardKey::TV_AUDIO_DESCRIPTION_MIX_DOWN,
    LogicalKeyboardKey::TV_AUDIO_DESCRIPTION_MIX_UP,
    LogicalKeyboardKey::TV_CONTENTS_MENU,
    LogicalKeyboardKey::TV_DATA_SERVICE,
    LogicalKeyboardKey::TV_INPUT_COMPONENT1,
    LogicalKeyboardKey::TV_INPUT_COMPONENT2,
    LogicalKeyboardKey::TV_INPUT_COMPOSITE1,
    LogicalKeyboardKey::TV_INPUT_COMPOSITE2,
    LogicalKeyboardKey::TV_INPUT_HDMI1,
    LogicalKeyboardKey::TV_INPUT_HDMI2,
    LogicalKeyboardKey::TV_INPUT_HDMI3,
    LogicalKeyboardKey::TV_INPUT_HDMI4,
    LogicalKeyboardKey::TV_INPUT_VGA1,
    LogicalKeyboardKey::TV_MEDIA_CONTEXT,
    LogicalKeyboardKey::TV_NETWORK,
    LogicalKeyboardKey::TV_NUMBER_ENTRY,
    LogicalKeyboardKey::TV_RADIO_SERVICE,
    LogicalKeyboardKey::TV_SATELLITE,
    LogicalKeyboardKey::TV_SATELLITE_BS,
    LogicalKeyboardKey::TV_SATELLITE_CS,
    LogicalKeyboardKey::TV_SATELLITE_TOGGLE,
    LogicalKeyboardKey::TV_TERRESTRIAL_ANALOG,
    LogicalKeyboardKey::TV_TERRESTRIAL_DIGITAL,
    LogicalKeyboardKey::TV_TIMER,
    LogicalKeyboardKey::KEY11,
    LogicalKeyboardKey::KEY12,
    LogicalKeyboardKey::SUSPEND,
    LogicalKeyboardKey::RESUME,
    LogicalKeyboardKey::SLEEP,
    LogicalKeyboardKey::ABORT,
    LogicalKeyboardKey::LANG1,
    LogicalKeyboardKey::LANG2,
    LogicalKeyboardKey::LANG3,
    LogicalKeyboardKey::LANG4,
    LogicalKeyboardKey::LANG5,
    LogicalKeyboardKey::INTL_BACKSLASH,
    LogicalKeyboardKey::INTL_RO,
    LogicalKeyboardKey::INTL_YEN,
    LogicalKeyboardKey::CONTROL_LEFT,
    LogicalKeyboardKey::CONTROL_RIGHT,
    LogicalKeyboardKey::SHIFT_LEFT,
    LogicalKeyboardKey::SHIFT_RIGHT,
    LogicalKeyboardKey::ALT_LEFT,
    LogicalKeyboardKey::ALT_RIGHT,
    LogicalKeyboardKey::META_LEFT,
    LogicalKeyboardKey::META_RIGHT,
    LogicalKeyboardKey::CONTROL,
    LogicalKeyboardKey::SHIFT,
    LogicalKeyboardKey::ALT,
    LogicalKeyboardKey::META,
    LogicalKeyboardKey::NUMPAD_ENTER,
    LogicalKeyboardKey::NUMPAD_PAREN_LEFT,
    LogicalKeyboardKey::NUMPAD_PAREN_RIGHT,
    LogicalKeyboardKey::NUMPAD_MULTIPLY,
    LogicalKeyboardKey::NUMPAD_ADD,
    LogicalKeyboardKey::NUMPAD_COMMA,
    LogicalKeyboardKey::NUMPAD_SUBTRACT,
    LogicalKeyboardKey::NUMPAD_DECIMAL,
    LogicalKeyboardKey::NUMPAD_DIVIDE,
    LogicalKeyboardKey::NUMPAD0,
    LogicalKeyboardKey::NUMPAD1,
    LogicalKeyboardKey::NUMPAD2,
    LogicalKeyboardKey::NUMPAD3,
    LogicalKeyboardKey::NUMPAD4,
    LogicalKeyboardKey::NUMPAD5,
    LogicalKeyboardKey::NUMPAD6,
    LogicalKeyboardKey::NUMPAD7,
    LogicalKeyboardKey::NUMPAD8,
    LogicalKeyboardKey::NUMPAD9,
    LogicalKeyboardKey::NUMPAD_EQUAL,
    LogicalKeyboardKey::GAME_BUTTON1,
    LogicalKeyboardKey::GAME_BUTTON2,
    LogicalKeyboardKey::GAME_BUTTON3,
    LogicalKeyboardKey::GAME_BUTTON4,
    LogicalKeyboardKey::GAME_BUTTON5,
    LogicalKeyboardKey::GAME_BUTTON6,
    LogicalKeyboardKey::GAME_BUTTON7,
    LogicalKeyboardKey::GAME_BUTTON8,
    LogicalKeyboardKey::GAME_BUTTON9,
    LogicalKeyboardKey::GAME_BUTTON10,
    LogicalKeyboardKey::GAME_BUTTON11,
    LogicalKeyboardKey::GAME_BUTTON12,
    LogicalKeyboardKey::GAME_BUTTON13,
    LogicalKeyboardKey::GAME_BUTTON14,
    LogicalKeyboardKey::GAME_BUTTON15,
    LogicalKeyboardKey::GAME_BUTTON16,
    LogicalKeyboardKey::GAME_BUTTON_A,
    LogicalKeyboardKey::GAME_BUTTON_B,
    LogicalKeyboardKey::GAME_BUTTON_C,
    LogicalKeyboardKey::GAME_BUTTON_LEFT1,
    LogicalKeyboardKey::GAME_BUTTON_LEFT2,
    LogicalKeyboardKey::GAME_BUTTON_MODE,
    LogicalKeyboardKey::GAME_BUTTON_RIGHT1,
    LogicalKeyboardKey::GAME_BUTTON_RIGHT2,
    LogicalKeyboardKey::GAME_BUTTON_SELECT,
    LogicalKeyboardKey::GAME_BUTTON_START,
    LogicalKeyboardKey::GAME_BUTTON_THUMB_LEFT,
    LogicalKeyboardKey::GAME_BUTTON_THUMB_RIGHT,
    LogicalKeyboardKey::GAME_BUTTON_X,
    LogicalKeyboardKey::GAME_BUTTON_Y,
    LogicalKeyboardKey::GAME_BUTTON_Z,
];

/// All the predefined constant [`PhysicalKeyboardKey`]s, sorted by USB HID usage
/// so they can be searched.
static KNOWN_PHYSICAL_KEYS: &[PhysicalKeyboardKey] = &[
    PhysicalKeyboardKey::HYPER,
    PhysicalKeyboardKey::SUPER_KEY,
    PhysicalKeyboardKey::FN,
    PhysicalKeyboardKey::FN_LOCK,
    PhysicalKeyboardKey::SUSPEND,
    PhysicalKeyboardKey::RESUME,
    PhysicalKeyboardKey::TURBO,
    PhysicalKeyboardKey::PRIVACY_SCREEN_TOGGLE,
    PhysicalKeyboardKey::MICROPHONE_MUTE_TOGGLE,
    PhysicalKeyboardKey::SLEEP,
    PhysicalKeyboardKey::WAKE_UP,
    PhysicalKeyboardKey::DISPLAY_TOGGLE_INT_EXT,
    PhysicalKeyboardKey::GAME_BUTTON1,
    PhysicalKeyboardKey::GAME_BUTTON2,
    PhysicalKeyboardKey::GAME_BUTTON3,
    PhysicalKeyboardKey::GAME_BUTTON4,
    PhysicalKeyboardKey::GAME_BUTTON5,
    PhysicalKeyboardKey::GAME_BUTTON6,
    PhysicalKeyboardKey::GAME_BUTTON7,
    PhysicalKeyboardKey::GAME_BUTTON8,
    PhysicalKeyboardKey::GAME_BUTTON9,
    PhysicalKeyboardKey::GAME_BUTTON10,
    PhysicalKeyboardKey::GAME_BUTTON11,
    PhysicalKeyboardKey::GAME_BUTTON12,
    PhysicalKeyboardKey::GAME_BUTTON13,
    PhysicalKeyboardKey::GAME_BUTTON14,
    PhysicalKeyboardKey::GAME_BUTTON15,
    PhysicalKeyboardKey::GAME_BUTTON16,
    PhysicalKeyboardKey::GAME_BUTTON_A,
    PhysicalKeyboardKey::GAME_BUTTON_B,
    PhysicalKeyboardKey::GAME_BUTTON_C,
    PhysicalKeyboardKey::GAME_BUTTON_LEFT1,
    PhysicalKeyboardKey::GAME_BUTTON_LEFT2,
    PhysicalKeyboardKey::GAME_BUTTON_MODE,
    PhysicalKeyboardKey::GAME_BUTTON_RIGHT1,
    PhysicalKeyboardKey::GAME_BUTTON_RIGHT2,
    PhysicalKeyboardKey::GAME_BUTTON_SELECT,
    PhysicalKeyboardKey::GAME_BUTTON_START,
    PhysicalKeyboardKey::GAME_BUTTON_THUMB_LEFT,
    PhysicalKeyboardKey::GAME_BUTTON_THUMB_RIGHT,
    PhysicalKeyboardKey::GAME_BUTTON_X,
    PhysicalKeyboardKey::GAME_BUTTON_Y,
    PhysicalKeyboardKey::GAME_BUTTON_Z,
    PhysicalKeyboardKey::USB_RESERVED,
    PhysicalKeyboardKey::USB_ERROR_ROLL_OVER,
    PhysicalKeyboardKey::USB_POST_FAIL,
    PhysicalKeyboardKey::USB_ERROR_UNDEFINED,
    PhysicalKeyboardKey::KEY_A,
    PhysicalKeyboardKey::KEY_B,
    PhysicalKeyboardKey::KEY_C,
    PhysicalKeyboardKey::KEY_D,
    PhysicalKeyboardKey::KEY_E,
    PhysicalKeyboardKey::KEY_F,
    PhysicalKeyboardKey::KEY_G,
    PhysicalKeyboardKey::KEY_H,
    PhysicalKeyboardKey::KEY_I,
    PhysicalKeyboardKey::KEY_J,
    PhysicalKeyboardKey::KEY_K,
    PhysicalKeyboardKey::KEY_L,
    PhysicalKeyboardKey::KEY_M,
    PhysicalKeyboardKey::KEY_N,
    PhysicalKeyboardKey::KEY_O,
    PhysicalKeyboardKey::KEY_P,
    PhysicalKeyboardKey::KEY_Q,
    PhysicalKeyboardKey::KEY_R,
    PhysicalKeyboardKey::KEY_S,
    PhysicalKeyboardKey::KEY_T,
    PhysicalKeyboardKey::KEY_U,
    PhysicalKeyboardKey::KEY_V,
    PhysicalKeyboardKey::KEY_W,
    PhysicalKeyboardKey::KEY_X,
    PhysicalKeyboardKey::KEY_Y,
    PhysicalKeyboardKey::KEY_Z,
    PhysicalKeyboardKey::DIGIT1,
    PhysicalKeyboardKey::DIGIT2,
    PhysicalKeyboardKey::DIGIT3,
    PhysicalKeyboardKey::DIGIT4,
    PhysicalKeyboardKey::DIGIT5,
    PhysicalKeyboardKey::DIGIT6,
    PhysicalKeyboardKey::DIGIT7,
    PhysicalKeyboardKey::DIGIT8,
    PhysicalKeyboardKey::DIGIT9,
    PhysicalKeyboardKey::DIGIT0,
    PhysicalKeyboardKey::ENTER,
    PhysicalKeyboardKey::ESCAPE,
    PhysicalKeyboardKey::BACKSPACE,
    PhysicalKeyboardKey::TAB,
    PhysicalKeyboardKey::SPACE,
    PhysicalKeyboardKey::MINUS,
    PhysicalKeyboardKey::EQUAL,
    PhysicalKeyboardKey::BRACKET_LEFT,
    PhysicalKeyboardKey::BRACKET_RIGHT,
    PhysicalKeyboardKey::BACKSLASH,
    PhysicalKeyboardKey::SEMICOLON,
    PhysicalKeyboardKey::QUOTE,
    PhysicalKeyboardKey::BACKQUOTE,
    PhysicalKeyboardKey::COMMA,
    PhysicalKeyboardKey::PERIOD,
    PhysicalKeyboardKey::SLASH,
    PhysicalKeyboardKey::CAPS_LOCK,
    PhysicalKeyboardKey::F1,
    PhysicalKeyboardKey::F2,
    PhysicalKeyboardKey::F3,
    PhysicalKeyboardKey::F4,
    PhysicalKeyboardKey::F5,
    PhysicalKeyboardKey::F6,
    PhysicalKeyboardKey::F7,
    PhysicalKeyboardKey::F8,
    PhysicalKeyboardKey::F9,
    PhysicalKeyboardKey::F10,
    PhysicalKeyboardKey::F11,
    PhysicalKeyboardKey::F12,
    PhysicalKeyboardKey::PRINT_SCREEN,
    PhysicalKeyboardKey::SCROLL_LOCK,
    PhysicalKeyboardKey::PAUSE,
    PhysicalKeyboardKey::INSERT,
    PhysicalKeyboardKey::HOME,
    PhysicalKeyboardKey::PAGE_UP,
    PhysicalKeyboardKey::DELETE,
    PhysicalKeyboardKey::END,
    PhysicalKeyboardKey::PAGE_DOWN,
    PhysicalKeyboardKey::ARROW_RIGHT,
    PhysicalKeyboardKey::ARROW_LEFT,
    PhysicalKeyboardKey::ARROW_DOWN,
    PhysicalKeyboardKey::ARROW_UP,
    PhysicalKeyboardKey::NUM_LOCK,
    PhysicalKeyboardKey::NUMPAD_DIVIDE,
    PhysicalKeyboardKey::NUMPAD_MULTIPLY,
    PhysicalKeyboardKey::NUMPAD_SUBTRACT,
    PhysicalKeyboardKey::NUMPAD_ADD,
    PhysicalKeyboardKey::NUMPAD_ENTER,
    PhysicalKeyboardKey::NUMPAD1,
    PhysicalKeyboardKey::NUMPAD2,
    PhysicalKeyboardKey::NUMPAD3,
    PhysicalKeyboardKey::NUMPAD4,
    PhysicalKeyboardKey::NUMPAD5,
    PhysicalKeyboardKey::NUMPAD6,
    PhysicalKeyboardKey::NUMPAD7,
    PhysicalKeyboardKey::NUMPAD8,
    PhysicalKeyboardKey::NUMPAD9,
    PhysicalKeyboardKey::NUMPAD0,
    PhysicalKeyboardKey::NUMPAD_DECIMAL,
    PhysicalKeyboardKey::INTL_BACKSLASH,
    PhysicalKeyboardKey::CONTEXT_MENU,
    PhysicalKeyboardKey::POWER,
    PhysicalKeyboardKey::NUMPAD_EQUAL,
    PhysicalKeyboardKey::F13,
    PhysicalKeyboardKey::F14,
    PhysicalKeyboardKey::F15,
    PhysicalKeyboardKey::F16,
    PhysicalKeyboardKey::F17,
    PhysicalKeyboardKey::F18,
    PhysicalKeyboardKey::F19,
    PhysicalKeyboardKey::F20,
    PhysicalKeyboardKey::F21,
    PhysicalKeyboardKey::F22,
    PhysicalKeyboardKey::F23,
    PhysicalKeyboardKey::F24,
    PhysicalKeyboardKey::OPEN,
    PhysicalKeyboardKey::HELP,
    PhysicalKeyboardKey::SELECT,
    PhysicalKeyboardKey::AGAIN,
    PhysicalKeyboardKey::UNDO,
    PhysicalKeyboardKey::CUT,
    PhysicalKeyboardKey::COPY,
    PhysicalKeyboardKey::PASTE,
    PhysicalKeyboardKey::FIND,
    PhysicalKeyboardKey::AUDIO_VOLUME_MUTE,
    PhysicalKeyboardKey::AUDIO_VOLUME_UP,
    PhysicalKeyboardKey::AUDIO_VOLUME_DOWN,
    PhysicalKeyboardKey::NUMPAD_COMMA,
    PhysicalKeyboardKey::INTL_RO,
    PhysicalKeyboardKey::KANA_MODE,
    PhysicalKeyboardKey::INTL_YEN,
    PhysicalKeyboardKey::CONVERT,
    PhysicalKeyboardKey::NON_CONVERT,
    PhysicalKeyboardKey::LANG1,
    PhysicalKeyboardKey::LANG2,
    PhysicalKeyboardKey::LANG3,
    PhysicalKeyboardKey::LANG4,
    PhysicalKeyboardKey::LANG5,
    PhysicalKeyboardKey::ABORT,
    PhysicalKeyboardKey::PROPS,
    PhysicalKeyboardKey::NUMPAD_PAREN_LEFT,
    PhysicalKeyboardKey::NUMPAD_PAREN_RIGHT,
    PhysicalKeyboardKey::NUMPAD_BACKSPACE,
    PhysicalKeyboardKey::NUMPAD_MEMORY_STORE,
    PhysicalKeyboardKey::NUMPAD_MEMORY_RECALL,
    PhysicalKeyboardKey::NUMPAD_MEMORY_CLEAR,
    PhysicalKeyboardKey::NUMPAD_MEMORY_ADD,
    PhysicalKeyboardKey::NUMPAD_MEMORY_SUBTRACT,
    PhysicalKeyboardKey::NUMPAD_SIGN_CHANGE,
    PhysicalKeyboardKey::NUMPAD_CLEAR,
    PhysicalKeyboardKey::NUMPAD_CLEAR_ENTRY,
    PhysicalKeyboardKey::CONTROL_LEFT,
    PhysicalKeyboardKey::SHIFT_LEFT,
    PhysicalKeyboardKey::ALT_LEFT,
    PhysicalKeyboardKey::META_LEFT,
    PhysicalKeyboardKey::CONTROL_RIGHT,
    PhysicalKeyboardKey::SHIFT_RIGHT,
    PhysicalKeyboardKey::ALT_RIGHT,
    PhysicalKeyboardKey::META_RIGHT,
    PhysicalKeyboardKey::INFO,
    PhysicalKeyboardKey::CLOSED_CAPTION_TOGGLE,
    PhysicalKeyboardKey::BRIGHTNESS_UP,
    PhysicalKeyboardKey::BRIGHTNESS_DOWN,
    PhysicalKeyboardKey::BRIGHTNESS_TOGGLE,
    PhysicalKeyboardKey::BRIGHTNESS_MINIMUM,
    PhysicalKeyboardKey::BRIGHTNESS_MAXIMUM,
    PhysicalKeyboardKey::BRIGHTNESS_AUTO,
    PhysicalKeyboardKey::KBD_ILLUM_UP,
    PhysicalKeyboardKey::KBD_ILLUM_DOWN,
    PhysicalKeyboardKey::MEDIA_LAST,
    PhysicalKeyboardKey::LAUNCH_PHONE,
    PhysicalKeyboardKey::PROGRAM_GUIDE,
    PhysicalKeyboardKey::EXIT,
    PhysicalKeyboardKey::CHANNEL_UP,
    PhysicalKeyboardKey::CHANNEL_DOWN,
    PhysicalKeyboardKey::MEDIA_PLAY,
    PhysicalKeyboardKey::MEDIA_PAUSE,
    PhysicalKeyboardKey::MEDIA_RECORD,
    PhysicalKeyboardKey::MEDIA_FAST_FORWARD,
    PhysicalKeyboardKey::MEDIA_REWIND,
    PhysicalKeyboardKey::MEDIA_TRACK_NEXT,
    PhysicalKeyboardKey::MEDIA_TRACK_PREVIOUS,
    PhysicalKeyboardKey::MEDIA_STOP,
    PhysicalKeyboardKey::EJECT,
    PhysicalKeyboardKey::MEDIA_PLAY_PAUSE,
    PhysicalKeyboardKey::SPEECH_INPUT_TOGGLE,
    PhysicalKeyboardKey::BASS_BOOST,
    PhysicalKeyboardKey::MEDIA_SELECT,
    PhysicalKeyboardKey::LAUNCH_WORD_PROCESSOR,
    PhysicalKeyboardKey::LAUNCH_SPREADSHEET,
    PhysicalKeyboardKey::LAUNCH_MAIL,
    PhysicalKeyboardKey::LAUNCH_CONTACTS,
    PhysicalKeyboardKey::LAUNCH_CALENDAR,
    PhysicalKeyboardKey::LAUNCH_APP2,
    PhysicalKeyboardKey::LAUNCH_APP1,
    PhysicalKeyboardKey::LAUNCH_INTERNET_BROWSER,
    PhysicalKeyboardKey::LOG_OFF,
    PhysicalKeyboardKey::LOCK_SCREEN,
    PhysicalKeyboardKey::LAUNCH_CONTROL_PANEL,
    PhysicalKeyboardKey::SELECT_TASK,
    PhysicalKeyboardKey::LAUNCH_DOCUMENTS,
    PhysicalKeyboardKey::SPELL_CHECK,
    PhysicalKeyboardKey::LAUNCH_KEYBOARD_LAYOUT,
    PhysicalKeyboardKey::LAUNCH_SCREEN_SAVER,
    PhysicalKeyboardKey::LAUNCH_AUDIO_BROWSER,
    PhysicalKeyboardKey::LAUNCH_ASSISTANT,
    PhysicalKeyboardKey::NEW_KEY,
    PhysicalKeyboardKey::CLOSE,
    PhysicalKeyboardKey::SAVE,
    PhysicalKeyboardKey::PRINT,
    PhysicalKeyboardKey::BROWSER_SEARCH,
    PhysicalKeyboardKey::BROWSER_HOME,
    PhysicalKeyboardKey::BROWSER_BACK,
    PhysicalKeyboardKey::BROWSER_FORWARD,
    PhysicalKeyboardKey::BROWSER_STOP,
    PhysicalKeyboardKey::BROWSER_REFRESH,
    PhysicalKeyboardKey::BROWSER_FAVORITES,
    PhysicalKeyboardKey::ZOOM_IN,
    PhysicalKeyboardKey::ZOOM_OUT,
    PhysicalKeyboardKey::ZOOM_TOGGLE,
    PhysicalKeyboardKey::REDO,
    PhysicalKeyboardKey::MAIL_REPLY,
    PhysicalKeyboardKey::MAIL_FORWARD,
    PhysicalKeyboardKey::MAIL_SEND,
    PhysicalKeyboardKey::KEYBOARD_LAYOUT_SELECT,
    PhysicalKeyboardKey::SHOW_ALL_WINDOWS,
];

static SYNONYMS: &[(LogicalKeyboardKey, &[LogicalKeyboardKey])] = &[
    (LogicalKeyboardKey::SHIFT_LEFT, &[LogicalKeyboardKey::SHIFT]),
    (
        LogicalKeyboardKey::SHIFT_RIGHT,
        &[LogicalKeyboardKey::SHIFT],
    ),
    (LogicalKeyboardKey::META_LEFT, &[LogicalKeyboardKey::META]),
    (LogicalKeyboardKey::META_RIGHT, &[LogicalKeyboardKey::META]),
    (LogicalKeyboardKey::ALT_LEFT, &[LogicalKeyboardKey::ALT]),
    (LogicalKeyboardKey::ALT_RIGHT, &[LogicalKeyboardKey::ALT]),
    (
        LogicalKeyboardKey::CONTROL_LEFT,
        &[LogicalKeyboardKey::CONTROL],
    ),
    (
        LogicalKeyboardKey::CONTROL_RIGHT,
        &[LogicalKeyboardKey::CONTROL],
    ),
];

static REVERSE_SYNONYMS: &[(LogicalKeyboardKey, &[LogicalKeyboardKey])] = &[
    (
        LogicalKeyboardKey::SHIFT,
        &[
            LogicalKeyboardKey::SHIFT_LEFT,
            LogicalKeyboardKey::SHIFT_RIGHT,
        ],
    ),
    (
        LogicalKeyboardKey::META,
        &[
            LogicalKeyboardKey::META_LEFT,
            LogicalKeyboardKey::META_RIGHT,
        ],
    ),
    (
        LogicalKeyboardKey::ALT,
        &[LogicalKeyboardKey::ALT_LEFT, LogicalKeyboardKey::ALT_RIGHT],
    ),
    (
        LogicalKeyboardKey::CONTROL,
        &[
            LogicalKeyboardKey::CONTROL_LEFT,
            LogicalKeyboardKey::CONTROL_RIGHT,
        ],
    ),
];

static KEY_LABELS: &[(u64, &str)] = &[
    (0x00000000020, "Space"),
    (0x00000000021, "Exclamation"),
    (0x00000000022, "Quote"),
    (0x00000000023, "Number Sign"),
    (0x00000000024, "Dollar"),
    (0x00000000025, "Percent"),
    (0x00000000026, "Ampersand"),
    (0x00000000027, "Quote Single"),
    (0x00000000028, "Parenthesis Left"),
    (0x00000000029, "Parenthesis Right"),
    (0x0000000002a, "Asterisk"),
    (0x0000000002b, "Add"),
    (0x0000000002c, "Comma"),
    (0x0000000002d, "Minus"),
    (0x0000000002e, "Period"),
    (0x0000000002f, "Slash"),
    (0x00000000030, "Digit 0"),
    (0x00000000031, "Digit 1"),
    (0x00000000032, "Digit 2"),
    (0x00000000033, "Digit 3"),
    (0x00000000034, "Digit 4"),
    (0x00000000035, "Digit 5"),
    (0x00000000036, "Digit 6"),
    (0x00000000037, "Digit 7"),
    (0x00000000038, "Digit 8"),
    (0x00000000039, "Digit 9"),
    (0x0000000003a, "Colon"),
    (0x0000000003b, "Semicolon"),
    (0x0000000003c, "Less"),
    (0x0000000003d, "Equal"),
    (0x0000000003e, "Greater"),
    (0x0000000003f, "Question"),
    (0x00000000040, "At"),
    (0x0000000005b, "Bracket Left"),
    (0x0000000005c, "Backslash"),
    (0x0000000005d, "Bracket Right"),
    (0x0000000005e, "Caret"),
    (0x0000000005f, "Underscore"),
    (0x00000000060, "Backquote"),
    (0x00000000061, "Key A"),
    (0x00000000062, "Key B"),
    (0x00000000063, "Key C"),
    (0x00000000064, "Key D"),
    (0x00000000065, "Key E"),
    (0x00000000066, "Key F"),
    (0x00000000067, "Key G"),
    (0x00000000068, "Key H"),
    (0x00000000069, "Key I"),
    (0x0000000006a, "Key J"),
    (0x0000000006b, "Key K"),
    (0x0000000006c, "Key L"),
    (0x0000000006d, "Key M"),
    (0x0000000006e, "Key N"),
    (0x0000000006f, "Key O"),
    (0x00000000070, "Key P"),
    (0x00000000071, "Key Q"),
    (0x00000000072, "Key R"),
    (0x00000000073, "Key S"),
    (0x00000000074, "Key T"),
    (0x00000000075, "Key U"),
    (0x00000000076, "Key V"),
    (0x00000000077, "Key W"),
    (0x00000000078, "Key X"),
    (0x00000000079, "Key Y"),
    (0x0000000007a, "Key Z"),
    (0x0000000007b, "Brace Left"),
    (0x0000000007c, "Bar"),
    (0x0000000007d, "Brace Right"),
    (0x0000000007e, "Tilde"),
    (0x00100000001, "Unidentified"),
    (0x00100000008, "Backspace"),
    (0x00100000009, "Tab"),
    (0x0010000000d, "Enter"),
    (0x0010000001b, "Escape"),
    (0x0010000007f, "Delete"),
    (0x00100000101, "Accel"),
    (0x00100000103, "Alt Graph"),
    (0x00100000104, "Caps Lock"),
    (0x00100000106, "Fn"),
    (0x00100000107, "Fn Lock"),
    (0x00100000108, "Hyper"),
    (0x0010000010a, "Num Lock"),
    (0x0010000010c, "Scroll Lock"),
    (0x0010000010e, "Super"),
    (0x0010000010f, "Symbol"),
    (0x00100000110, "Symbol Lock"),
    (0x00100000111, "Shift Level 5"),
    (0x00100000301, "Arrow Down"),
    (0x00100000302, "Arrow Left"),
    (0x00100000303, "Arrow Right"),
    (0x00100000304, "Arrow Up"),
    (0x00100000305, "End"),
    (0x00100000306, "Home"),
    (0x00100000307, "Page Down"),
    (0x00100000308, "Page Up"),
    (0x00100000401, "Clear"),
    (0x00100000402, "Copy"),
    (0x00100000403, "Cr Sel"),
    (0x00100000404, "Cut"),
    (0x00100000405, "Erase Eof"),
    (0x00100000406, "Ex Sel"),
    (0x00100000407, "Insert"),
    (0x00100000408, "Paste"),
    (0x00100000409, "Redo"),
    (0x0010000040a, "Undo"),
    (0x00100000501, "Accept"),
    (0x00100000502, "Again"),
    (0x00100000503, "Attn"),
    (0x00100000504, "Cancel"),
    (0x00100000505, "Context Menu"),
    (0x00100000506, "Execute"),
    (0x00100000507, "Find"),
    (0x00100000508, "Help"),
    (0x00100000509, "Pause"),
    (0x0010000050a, "Play"),
    (0x0010000050b, "Props"),
    (0x0010000050c, "Select"),
    (0x0010000050d, "Zoom In"),
    (0x0010000050e, "Zoom Out"),
    (0x00100000601, "Brightness Down"),
    (0x00100000602, "Brightness Up"),
    (0x00100000603, "Camera"),
    (0x00100000604, "Eject"),
    (0x00100000605, "Log Off"),
    (0x00100000606, "Power"),
    (0x00100000607, "Power Off"),
    (0x00100000608, "Print Screen"),
    (0x00100000609, "Hibernate"),
    (0x0010000060a, "Standby"),
    (0x0010000060b, "Wake Up"),
    (0x00100000701, "All Candidates"),
    (0x00100000702, "Alphanumeric"),
    (0x00100000703, "Code Input"),
    (0x00100000704, "Compose"),
    (0x00100000705, "Convert"),
    (0x00100000706, "Final Mode"),
    (0x00100000707, "Group First"),
    (0x00100000708, "Group Last"),
    (0x00100000709, "Group Next"),
    (0x0010000070a, "Group Previous"),
    (0x0010000070b, "Mode Change"),
    (0x0010000070c, "Next Candidate"),
    (0x0010000070d, "Non Convert"),
    (0x0010000070e, "Previous Candidate"),
    (0x0010000070f, "Process"),
    (0x00100000710, "Single Candidate"),
    (0x00100000711, "Hangul Mode"),
    (0x00100000712, "Hanja Mode"),
    (0x00100000713, "Junja Mode"),
    (0x00100000714, "Eisu"),
    (0x00100000715, "Hankaku"),
    (0x00100000716, "Hiragana"),
    (0x00100000717, "Hiragana Katakana"),
    (0x00100000718, "Kana Mode"),
    (0x00100000719, "Kanji Mode"),
    (0x0010000071a, "Katakana"),
    (0x0010000071b, "Romaji"),
    (0x0010000071c, "Zenkaku"),
    (0x0010000071d, "Zenkaku Hankaku"),
    (0x00100000801, "F1"),
    (0x00100000802, "F2"),
    (0x00100000803, "F3"),
    (0x00100000804, "F4"),
    (0x00100000805, "F5"),
    (0x00100000806, "F6"),
    (0x00100000807, "F7"),
    (0x00100000808, "F8"),
    (0x00100000809, "F9"),
    (0x0010000080a, "F10"),
    (0x0010000080b, "F11"),
    (0x0010000080c, "F12"),
    (0x0010000080d, "F13"),
    (0x0010000080e, "F14"),
    (0x0010000080f, "F15"),
    (0x00100000810, "F16"),
    (0x00100000811, "F17"),
    (0x00100000812, "F18"),
    (0x00100000813, "F19"),
    (0x00100000814, "F20"),
    (0x00100000815, "F21"),
    (0x00100000816, "F22"),
    (0x00100000817, "F23"),
    (0x00100000818, "F24"),
    (0x00100000901, "Soft 1"),
    (0x00100000902, "Soft 2"),
    (0x00100000903, "Soft 3"),
    (0x00100000904, "Soft 4"),
    (0x00100000905, "Soft 5"),
    (0x00100000906, "Soft 6"),
    (0x00100000907, "Soft 7"),
    (0x00100000908, "Soft 8"),
    (0x00100000a01, "Close"),
    (0x00100000a02, "Mail Forward"),
    (0x00100000a03, "Mail Reply"),
    (0x00100000a04, "Mail Send"),
    (0x00100000a05, "Media Play Pause"),
    (0x00100000a07, "Media Stop"),
    (0x00100000a08, "Media Track Next"),
    (0x00100000a09, "Media Track Previous"),
    (0x00100000a0a, "New"),
    (0x00100000a0b, "Open"),
    (0x00100000a0c, "Print"),
    (0x00100000a0d, "Save"),
    (0x00100000a0e, "Spell Check"),
    (0x00100000a0f, "Audio Volume Down"),
    (0x00100000a10, "Audio Volume Up"),
    (0x00100000a11, "Audio Volume Mute"),
    (0x00100000b01, "Launch Application 2"),
    (0x00100000b02, "Launch Calendar"),
    (0x00100000b03, "Launch Mail"),
    (0x00100000b04, "Launch Media Player"),
    (0x00100000b05, "Launch Music Player"),
    (0x00100000b06, "Launch Application 1"),
    (0x00100000b07, "Launch Screen Saver"),
    (0x00100000b08, "Launch Spreadsheet"),
    (0x00100000b09, "Launch Web Browser"),
    (0x00100000b0a, "Launch Web Cam"),
    (0x00100000b0b, "Launch Word Processor"),
    (0x00100000b0c, "Launch Contacts"),
    (0x00100000b0d, "Launch Phone"),
    (0x00100000b0e, "Launch Assistant"),
    (0x00100000b0f, "Launch Control Panel"),
    (0x00100000c01, "Browser Back"),
    (0x00100000c02, "Browser Favorites"),
    (0x00100000c03, "Browser Forward"),
    (0x00100000c04, "Browser Home"),
    (0x00100000c05, "Browser Refresh"),
    (0x00100000c06, "Browser Search"),
    (0x00100000c07, "Browser Stop"),
    (0x00100000d01, "Audio Balance Left"),
    (0x00100000d02, "Audio Balance Right"),
    (0x00100000d03, "Audio Bass Boost Down"),
    (0x00100000d04, "Audio Bass Boost Up"),
    (0x00100000d05, "Audio Fader Front"),
    (0x00100000d06, "Audio Fader Rear"),
    (0x00100000d07, "Audio Surround Mode Next"),
    (0x00100000d08, "AVR Input"),
    (0x00100000d09, "AVR Power"),
    (0x00100000d0a, "Channel Down"),
    (0x00100000d0b, "Channel Up"),
    (0x00100000d0c, "Color F0 Red"),
    (0x00100000d0d, "Color F1 Green"),
    (0x00100000d0e, "Color F2 Yellow"),
    (0x00100000d0f, "Color F3 Blue"),
    (0x00100000d10, "Color F4 Grey"),
    (0x00100000d11, "Color F5 Brown"),
    (0x00100000d12, "Closed Caption Toggle"),
    (0x00100000d13, "Dimmer"),
    (0x00100000d14, "Display Swap"),
    (0x00100000d15, "Exit"),
    (0x00100000d16, "Favorite Clear 0"),
    (0x00100000d17, "Favorite Clear 1"),
    (0x00100000d18, "Favorite Clear 2"),
    (0x00100000d19, "Favorite Clear 3"),
    (0x00100000d1a, "Favorite Recall 0"),
    (0x00100000d1b, "Favorite Recall 1"),
    (0x00100000d1c, "Favorite Recall 2"),
    (0x00100000d1d, "Favorite Recall 3"),
    (0x00100000d1e, "Favorite Store 0"),
    (0x00100000d1f, "Favorite Store 1"),
    (0x00100000d20, "Favorite Store 2"),
    (0x00100000d21, "Favorite Store 3"),
    (0x00100000d22, "Guide"),
    (0x00100000d23, "Guide Next Day"),
    (0x00100000d24, "Guide Previous Day"),
    (0x00100000d25, "Info"),
    (0x00100000d26, "Instant Replay"),
    (0x00100000d27, "Link"),
    (0x00100000d28, "List Program"),
    (0x00100000d29, "Live Content"),
    (0x00100000d2a, "Lock"),
    (0x00100000d2b, "Media Apps"),
    (0x00100000d2c, "Media Fast Forward"),
    (0x00100000d2d, "Media Last"),
    (0x00100000d2e, "Media Pause"),
    (0x00100000d2f, "Media Play"),
    (0x00100000d30, "Media Record"),
    (0x00100000d31, "Media Rewind"),
    (0x00100000d32, "Media Skip"),
    (0x00100000d33, "Next Favorite Channel"),
    (0x00100000d34, "Next User Profile"),
    (0x00100000d35, "On Demand"),
    (0x00100000d36, "P In P Down"),
    (0x00100000d37, "P In P Move"),
    (0x00100000d38, "P In P Toggle"),
    (0x00100000d39, "P In P Up"),
    (0x00100000d3a, "Play Speed Down"),
    (0x00100000d3b, "Play Speed Reset"),
    (0x00100000d3c, "Play Speed Up"),
    (0x00100000d3d, "Random Toggle"),
    (0x00100000d3e, "Rc Low Battery"),
    (0x00100000d3f, "Record Speed Next"),
    (0x00100000d40, "Rf Bypass"),
    (0x00100000d41, "Scan Channels Toggle"),
    (0x00100000d42, "Screen Mode Next"),
    (0x00100000d43, "Settings"),
    (0x00100000d44, "Split Screen Toggle"),
    (0x00100000d45, "STB Input"),
    (0x00100000d46, "STB Power"),
    (0x00100000d47, "Subtitle"),
    (0x00100000d48, "Teletext"),
    (0x00100000d49, "TV"),
    (0x00100000d4a, "TV Input"),
    (0x00100000d4b, "TV Power"),
    (0x00100000d4c, "Video Mode Next"),
    (0x00100000d4d, "Wink"),
    (0x00100000d4e, "Zoom Toggle"),
    (0x00100000d4f, "DVR"),
    (0x00100000d50, "Media Audio Track"),
    (0x00100000d51, "Media Skip Backward"),
    (0x00100000d52, "Media Skip Forward"),
    (0x00100000d53, "Media Step Backward"),
    (0x00100000d54, "Media Step Forward"),
    (0x00100000d55, "Media Top Menu"),
    (0x00100000d56, "Navigate In"),
    (0x00100000d57, "Navigate Next"),
    (0x00100000d58, "Navigate Out"),
    (0x00100000d59, "Navigate Previous"),
    (0x00100000d5a, "Pairing"),
    (0x00100000d5b, "Media Close"),
    (0x00100000e02, "Audio Bass Boost Toggle"),
    (0x00100000e04, "Audio Treble Down"),
    (0x00100000e05, "Audio Treble Up"),
    (0x00100000e06, "Microphone Toggle"),
    (0x00100000e07, "Microphone Volume Down"),
    (0x00100000e08, "Microphone Volume Up"),
    (0x00100000e09, "Microphone Volume Mute"),
    (0x00100000f01, "Speech Correction List"),
    (0x00100000f02, "Speech Input Toggle"),
    (0x00100001001, "App Switch"),
    (0x00100001002, "Call"),
    (0x00100001003, "Camera Focus"),
    (0x00100001004, "End Call"),
    (0x00100001005, "Go Back"),
    (0x00100001006, "Go Home"),
    (0x00100001007, "Headset Hook"),
    (0x00100001008, "Last Number Redial"),
    (0x00100001009, "Notification"),
    (0x0010000100a, "Manner Mode"),
    (0x0010000100b, "Voice Dial"),
    (0x00100001101, "TV 3 D Mode"),
    (0x00100001102, "TV Antenna Cable"),
    (0x00100001103, "TV Audio Description"),
    (0x00100001104, "TV Audio Description Mix Down"),
    (0x00100001105, "TV Audio Description Mix Up"),
    (0x00100001106, "TV Contents Menu"),
    (0x00100001107, "TV Data Service"),
    (0x00100001108, "TV Input Component 1"),
    (0x00100001109, "TV Input Component 2"),
    (0x0010000110a, "TV Input Composite 1"),
    (0x0010000110b, "TV Input Composite 2"),
    (0x0010000110c, "TV Input HDMI 1"),
    (0x0010000110d, "TV Input HDMI 2"),
    (0x0010000110e, "TV Input HDMI 3"),
    (0x0010000110f, "TV Input HDMI 4"),
    (0x00100001110, "TV Input VGA 1"),
    (0x00100001111, "TV Media Context"),
    (0x00100001112, "TV Network"),
    (0x00100001113, "TV Number Entry"),
    (0x00100001114, "TV Radio Service"),
    (0x00100001115, "TV Satellite"),
    (0x00100001116, "TV Satellite BS"),
    (0x00100001117, "TV Satellite CS"),
    (0x00100001118, "TV Satellite Toggle"),
    (0x00100001119, "TV Terrestrial Analog"),
    (0x0010000111a, "TV Terrestrial Digital"),
    (0x0010000111b, "TV Timer"),
    (0x00100001201, "Key 11"),
    (0x00100001202, "Key 12"),
    (0x00200000000, "Suspend"),
    (0x00200000001, "Resume"),
    (0x00200000002, "Sleep"),
    (0x00200000003, "Abort"),
    (0x00200000010, "Lang 1"),
    (0x00200000011, "Lang 2"),
    (0x00200000012, "Lang 3"),
    (0x00200000013, "Lang 4"),
    (0x00200000014, "Lang 5"),
    (0x00200000020, "Intl Backslash"),
    (0x00200000021, "Intl Ro"),
    (0x00200000022, "Intl Yen"),
    (0x00200000100, "Control Left"),
    (0x00200000101, "Control Right"),
    (0x00200000102, "Shift Left"),
    (0x00200000103, "Shift Right"),
    (0x00200000104, "Alt Left"),
    (0x00200000105, "Alt Right"),
    (0x00200000106, "Meta Left"),
    (0x00200000107, "Meta Right"),
    (0x002000001f0, "Control"),
    (0x002000001f2, "Shift"),
    (0x002000001f4, "Alt"),
    (0x002000001f6, "Meta"),
    (0x0020000020d, "Numpad Enter"),
    (0x00200000228, "Numpad Paren Left"),
    (0x00200000229, "Numpad Paren Right"),
    (0x0020000022a, "Numpad Multiply"),
    (0x0020000022b, "Numpad Add"),
    (0x0020000022c, "Numpad Comma"),
    (0x0020000022d, "Numpad Subtract"),
    (0x0020000022e, "Numpad Decimal"),
    (0x0020000022f, "Numpad Divide"),
    (0x00200000230, "Numpad 0"),
    (0x00200000231, "Numpad 1"),
    (0x00200000232, "Numpad 2"),
    (0x00200000233, "Numpad 3"),
    (0x00200000234, "Numpad 4"),
    (0x00200000235, "Numpad 5"),
    (0x00200000236, "Numpad 6"),
    (0x00200000237, "Numpad 7"),
    (0x00200000238, "Numpad 8"),
    (0x00200000239, "Numpad 9"),
    (0x0020000023d, "Numpad Equal"),
    (0x00200000301, "Game Button 1"),
    (0x00200000302, "Game Button 2"),
    (0x00200000303, "Game Button 3"),
    (0x00200000304, "Game Button 4"),
    (0x00200000305, "Game Button 5"),
    (0x00200000306, "Game Button 6"),
    (0x00200000307, "Game Button 7"),
    (0x00200000308, "Game Button 8"),
    (0x00200000309, "Game Button 9"),
    (0x0020000030a, "Game Button 10"),
    (0x0020000030b, "Game Button 11"),
    (0x0020000030c, "Game Button 12"),
    (0x0020000030d, "Game Button 13"),
    (0x0020000030e, "Game Button 14"),
    (0x0020000030f, "Game Button 15"),
    (0x00200000310, "Game Button 16"),
    (0x00200000311, "Game Button A"),
    (0x00200000312, "Game Button B"),
    (0x00200000313, "Game Button C"),
    (0x00200000314, "Game Button Left 1"),
    (0x00200000315, "Game Button Left 2"),
    (0x00200000316, "Game Button Mode"),
    (0x00200000317, "Game Button Right 1"),
    (0x00200000318, "Game Button Right 2"),
    (0x00200000319, "Game Button Select"),
    (0x0020000031a, "Game Button Start"),
    (0x0020000031b, "Game Button Thumb Left"),
    (0x0020000031c, "Game Button Thumb Right"),
    (0x0020000031d, "Game Button X"),
    (0x0020000031e, "Game Button Y"),
    (0x0020000031f, "Game Button Z"),
];

static DEBUG_NAMES: &[(u64, &str)] = &[
    (0x00000010, "Hyper"),
    (0x00000011, "Super Key"),
    (0x00000012, "Fn"),
    (0x00000013, "Fn Lock"),
    (0x00000014, "Suspend"),
    (0x00000015, "Resume"),
    (0x00000016, "Turbo"),
    (0x00000017, "Privacy Screen Toggle"),
    (0x00000018, "Microphone Mute Toggle"),
    (0x00010082, "Sleep"),
    (0x00010083, "Wake Up"),
    (0x000100b5, "Display Toggle Int Ext"),
    (0x0005ff01, "Game Button 1"),
    (0x0005ff02, "Game Button 2"),
    (0x0005ff03, "Game Button 3"),
    (0x0005ff04, "Game Button 4"),
    (0x0005ff05, "Game Button 5"),
    (0x0005ff06, "Game Button 6"),
    (0x0005ff07, "Game Button 7"),
    (0x0005ff08, "Game Button 8"),
    (0x0005ff09, "Game Button 9"),
    (0x0005ff0a, "Game Button 10"),
    (0x0005ff0b, "Game Button 11"),
    (0x0005ff0c, "Game Button 12"),
    (0x0005ff0d, "Game Button 13"),
    (0x0005ff0e, "Game Button 14"),
    (0x0005ff0f, "Game Button 15"),
    (0x0005ff10, "Game Button 16"),
    (0x0005ff11, "Game Button A"),
    (0x0005ff12, "Game Button B"),
    (0x0005ff13, "Game Button C"),
    (0x0005ff14, "Game Button Left 1"),
    (0x0005ff15, "Game Button Left 2"),
    (0x0005ff16, "Game Button Mode"),
    (0x0005ff17, "Game Button Right 1"),
    (0x0005ff18, "Game Button Right 2"),
    (0x0005ff19, "Game Button Select"),
    (0x0005ff1a, "Game Button Start"),
    (0x0005ff1b, "Game Button Thumb Left"),
    (0x0005ff1c, "Game Button Thumb Right"),
    (0x0005ff1d, "Game Button X"),
    (0x0005ff1e, "Game Button Y"),
    (0x0005ff1f, "Game Button Z"),
    (0x00070000, "Usb Reserved"),
    (0x00070001, "Usb Error Roll Over"),
    (0x00070002, "Usb Post Fail"),
    (0x00070003, "Usb Error Undefined"),
    (0x00070004, "Key A"),
    (0x00070005, "Key B"),
    (0x00070006, "Key C"),
    (0x00070007, "Key D"),
    (0x00070008, "Key E"),
    (0x00070009, "Key F"),
    (0x0007000a, "Key G"),
    (0x0007000b, "Key H"),
    (0x0007000c, "Key I"),
    (0x0007000d, "Key J"),
    (0x0007000e, "Key K"),
    (0x0007000f, "Key L"),
    (0x00070010, "Key M"),
    (0x00070011, "Key N"),
    (0x00070012, "Key O"),
    (0x00070013, "Key P"),
    (0x00070014, "Key Q"),
    (0x00070015, "Key R"),
    (0x00070016, "Key S"),
    (0x00070017, "Key T"),
    (0x00070018, "Key U"),
    (0x00070019, "Key V"),
    (0x0007001a, "Key W"),
    (0x0007001b, "Key X"),
    (0x0007001c, "Key Y"),
    (0x0007001d, "Key Z"),
    (0x0007001e, "Digit 1"),
    (0x0007001f, "Digit 2"),
    (0x00070020, "Digit 3"),
    (0x00070021, "Digit 4"),
    (0x00070022, "Digit 5"),
    (0x00070023, "Digit 6"),
    (0x00070024, "Digit 7"),
    (0x00070025, "Digit 8"),
    (0x00070026, "Digit 9"),
    (0x00070027, "Digit 0"),
    (0x00070028, "Enter"),
    (0x00070029, "Escape"),
    (0x0007002a, "Backspace"),
    (0x0007002b, "Tab"),
    (0x0007002c, "Space"),
    (0x0007002d, "Minus"),
    (0x0007002e, "Equal"),
    (0x0007002f, "Bracket Left"),
    (0x00070030, "Bracket Right"),
    (0x00070031, "Backslash"),
    (0x00070033, "Semicolon"),
    (0x00070034, "Quote"),
    (0x00070035, "Backquote"),
    (0x00070036, "Comma"),
    (0x00070037, "Period"),
    (0x00070038, "Slash"),
    (0x00070039, "Caps Lock"),
    (0x0007003a, "F1"),
    (0x0007003b, "F2"),
    (0x0007003c, "F3"),
    (0x0007003d, "F4"),
    (0x0007003e, "F5"),
    (0x0007003f, "F6"),
    (0x00070040, "F7"),
    (0x00070041, "F8"),
    (0x00070042, "F9"),
    (0x00070043, "F10"),
    (0x00070044, "F11"),
    (0x00070045, "F12"),
    (0x00070046, "Print Screen"),
    (0x00070047, "Scroll Lock"),
    (0x00070048, "Pause"),
    (0x00070049, "Insert"),
    (0x0007004a, "Home"),
    (0x0007004b, "Page Up"),
    (0x0007004c, "Delete"),
    (0x0007004d, "End"),
    (0x0007004e, "Page Down"),
    (0x0007004f, "Arrow Right"),
    (0x00070050, "Arrow Left"),
    (0x00070051, "Arrow Down"),
    (0x00070052, "Arrow Up"),
    (0x00070053, "Num Lock"),
    (0x00070054, "Numpad Divide"),
    (0x00070055, "Numpad Multiply"),
    (0x00070056, "Numpad Subtract"),
    (0x00070057, "Numpad Add"),
    (0x00070058, "Numpad Enter"),
    (0x00070059, "Numpad 1"),
    (0x0007005a, "Numpad 2"),
    (0x0007005b, "Numpad 3"),
    (0x0007005c, "Numpad 4"),
    (0x0007005d, "Numpad 5"),
    (0x0007005e, "Numpad 6"),
    (0x0007005f, "Numpad 7"),
    (0x00070060, "Numpad 8"),
    (0x00070061, "Numpad 9"),
    (0x00070062, "Numpad 0"),
    (0x00070063, "Numpad Decimal"),
    (0x00070064, "Intl Backslash"),
    (0x00070065, "Context Menu"),
    (0x00070066, "Power"),
    (0x00070067, "Numpad Equal"),
    (0x00070068, "F13"),
    (0x00070069, "F14"),
    (0x0007006a, "F15"),
    (0x0007006b, "F16"),
    (0x0007006c, "F17"),
    (0x0007006d, "F18"),
    (0x0007006e, "F19"),
    (0x0007006f, "F20"),
    (0x00070070, "F21"),
    (0x00070071, "F22"),
    (0x00070072, "F23"),
    (0x00070073, "F24"),
    (0x00070074, "Open"),
    (0x00070075, "Help"),
    (0x00070077, "Select"),
    (0x00070079, "Again"),
    (0x0007007a, "Undo"),
    (0x0007007b, "Cut"),
    (0x0007007c, "Copy"),
    (0x0007007d, "Paste"),
    (0x0007007e, "Find"),
    (0x0007007f, "Audio Volume Mute"),
    (0x00070080, "Audio Volume Up"),
    (0x00070081, "Audio Volume Down"),
    (0x00070085, "Numpad Comma"),
    (0x00070087, "Intl Ro"),
    (0x00070088, "Kana Mode"),
    (0x00070089, "Intl Yen"),
    (0x0007008a, "Convert"),
    (0x0007008b, "Non Convert"),
    (0x00070090, "Lang 1"),
    (0x00070091, "Lang 2"),
    (0x00070092, "Lang 3"),
    (0x00070093, "Lang 4"),
    (0x00070094, "Lang 5"),
    (0x0007009b, "Abort"),
    (0x000700a3, "Props"),
    (0x000700b6, "Numpad Paren Left"),
    (0x000700b7, "Numpad Paren Right"),
    (0x000700bb, "Numpad Backspace"),
    (0x000700d0, "Numpad Memory Store"),
    (0x000700d1, "Numpad Memory Recall"),
    (0x000700d2, "Numpad Memory Clear"),
    (0x000700d3, "Numpad Memory Add"),
    (0x000700d4, "Numpad Memory Subtract"),
    (0x000700d7, "Numpad Sign Change"),
    (0x000700d8, "Numpad Clear"),
    (0x000700d9, "Numpad Clear Entry"),
    (0x000700e0, "Control Left"),
    (0x000700e1, "Shift Left"),
    (0x000700e2, "Alt Left"),
    (0x000700e3, "Meta Left"),
    (0x000700e4, "Control Right"),
    (0x000700e5, "Shift Right"),
    (0x000700e6, "Alt Right"),
    (0x000700e7, "Meta Right"),
    (0x000c0060, "Info"),
    (0x000c0061, "Closed Caption Toggle"),
    (0x000c006f, "Brightness Up"),
    (0x000c0070, "Brightness Down"),
    (0x000c0072, "Brightness Toggle"),
    (0x000c0073, "Brightness Minimum"),
    (0x000c0074, "Brightness Maximum"),
    (0x000c0075, "Brightness Auto"),
    (0x000c0079, "Kbd Illum Up"),
    (0x000c007a, "Kbd Illum Down"),
    (0x000c0083, "Media Last"),
    (0x000c008c, "Launch Phone"),
    (0x000c008d, "Program Guide"),
    (0x000c0094, "Exit"),
    (0x000c009c, "Channel Up"),
    (0x000c009d, "Channel Down"),
    (0x000c00b0, "Media Play"),
    (0x000c00b1, "Media Pause"),
    (0x000c00b2, "Media Record"),
    (0x000c00b3, "Media Fast Forward"),
    (0x000c00b4, "Media Rewind"),
    (0x000c00b5, "Media Track Next"),
    (0x000c00b6, "Media Track Previous"),
    (0x000c00b7, "Media Stop"),
    (0x000c00b8, "Eject"),
    (0x000c00cd, "Media Play Pause"),
    (0x000c00cf, "Speech Input Toggle"),
    (0x000c00e5, "Bass Boost"),
    (0x000c0183, "Media Select"),
    (0x000c0184, "Launch Word Processor"),
    (0x000c0186, "Launch Spreadsheet"),
    (0x000c018a, "Launch Mail"),
    (0x000c018d, "Launch Contacts"),
    (0x000c018e, "Launch Calendar"),
    (0x000c0192, "Launch App2"),
    (0x000c0194, "Launch App1"),
    (0x000c0196, "Launch Internet Browser"),
    (0x000c019c, "Log Off"),
    (0x000c019e, "Lock Screen"),
    (0x000c019f, "Launch Control Panel"),
    (0x000c01a2, "Select Task"),
    (0x000c01a7, "Launch Documents"),
    (0x000c01ab, "Spell Check"),
    (0x000c01ae, "Launch Keyboard Layout"),
    (0x000c01b1, "Launch Screen Saver"),
    (0x000c01b7, "Launch Audio Browser"),
    (0x000c01cb, "Launch Assistant"),
    (0x000c0201, "New Key"),
    (0x000c0203, "Close"),
    (0x000c0207, "Save"),
    (0x000c0208, "Print"),
    (0x000c0221, "Browser Search"),
    (0x000c0223, "Browser Home"),
    (0x000c0224, "Browser Back"),
    (0x000c0225, "Browser Forward"),
    (0x000c0226, "Browser Stop"),
    (0x000c0227, "Browser Refresh"),
    (0x000c022a, "Browser Favorites"),
    (0x000c022d, "Zoom In"),
    (0x000c022e, "Zoom Out"),
    (0x000c0232, "Zoom Toggle"),
    (0x000c0279, "Redo"),
    (0x000c0289, "Mail Reply"),
    (0x000c028b, "Mail Forward"),
    (0x000c028c, "Mail Send"),
    (0x000c029d, "Keyboard Layout Select"),
    (0x000c029f, "Show All Windows"),
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{LogicalKeyboardKey, PhysicalKeyboardKey};

    #[test]
    fn find_key_by_key_id_round_trips_a_sample_of_the_table() {
        for key in [
            LogicalKeyboardKey::KEY_A,
            LogicalKeyboardKey::DIGIT7,
            LogicalKeyboardKey::SPACE,
            LogicalKeyboardKey::ENTER,
            LogicalKeyboardKey::ESCAPE,
            LogicalKeyboardKey::ARROW_LEFT,
            LogicalKeyboardKey::SHIFT_RIGHT,
            LogicalKeyboardKey::F12,
            LogicalKeyboardKey::GAME_BUTTON_Z,
        ] {
            assert_eq!(
                LogicalKeyboardKey::find_key_by_key_id(key.key_id),
                Some(key)
            );
        }
        assert_eq!(LogicalKeyboardKey::find_key_by_key_id(0x0001), None);
        assert_eq!(LogicalKeyboardKey::known_logical_keys().count(), 444);
    }

    #[test]
    fn find_key_by_code_round_trips_a_sample_of_the_table() {
        for key in [
            PhysicalKeyboardKey::KEY_A,
            PhysicalKeyboardKey::DIGIT7,
            PhysicalKeyboardKey::SPACE,
            PhysicalKeyboardKey::ENTER,
            PhysicalKeyboardKey::ESCAPE,
            PhysicalKeyboardKey::ARROW_LEFT,
            PhysicalKeyboardKey::SHIFT_RIGHT,
            PhysicalKeyboardKey::F12,
            PhysicalKeyboardKey::SHOW_ALL_WINDOWS,
        ] {
            assert_eq!(
                PhysicalKeyboardKey::find_key_by_code(key.usb_hid_usage),
                Some(key)
            );
        }
        assert_eq!(PhysicalKeyboardKey::find_key_by_code(0x0001), None);
        assert_eq!(PhysicalKeyboardKey::known_physical_keys().count(), 269);
    }

    #[test]
    fn key_labels_come_from_unicode_then_the_table() {
        assert_eq!(LogicalKeyboardKey::KEY_A.key_label(), "A");
        assert_eq!(LogicalKeyboardKey::DIGIT7.key_label(), "7");
        assert_eq!(LogicalKeyboardKey::F1.key_label(), "F1");
        assert_eq!(LogicalKeyboardKey::SHIFT_LEFT.key_label(), "Shift Left");
        assert_eq!(LogicalKeyboardKey::new(0x0110000dead).key_label(), "");
    }

    #[test]
    fn debug_names_describe_known_and_unknown_keys() {
        assert_eq!(
            LogicalKeyboardKey::KEY_A.debug_name().as_deref(),
            Some("Key A")
        );
        assert_eq!(
            LogicalKeyboardKey::new(0x0110000dead)
                .debug_name()
                .as_deref(),
            Some("Key with ID 0x0110000dead")
        );
        assert_eq!(
            PhysicalKeyboardKey::KEY_A.debug_name().as_deref(),
            Some("Key A")
        );
        assert_eq!(
            PhysicalKeyboardKey::new(0xdead).debug_name().as_deref(),
            Some("Key with ID 0x0000dead")
        );
    }

    #[test]
    fn is_control_character_only_matches_single_control_characters() {
        assert!(LogicalKeyboardKey::is_control_character("\n"));
        assert!(LogicalKeyboardKey::is_control_character("\u{1b}"));
        assert!(LogicalKeyboardKey::is_control_character("\u{7f}"));
        assert!(!LogicalKeyboardKey::is_control_character("a"));
        assert!(!LogicalKeyboardKey::is_control_character(""));
        assert!(!LogicalKeyboardKey::is_control_character("ab"));
    }

    #[test]
    fn synonyms_collapse_and_expand_the_sided_modifiers() {
        assert_eq!(
            LogicalKeyboardKey::SHIFT_LEFT.synonyms(),
            HashSet::from([LogicalKeyboardKey::SHIFT])
        );
        assert!(LogicalKeyboardKey::KEY_A.synonyms().is_empty());

        let pressed = HashSet::from([
            LogicalKeyboardKey::SHIFT_RIGHT,
            LogicalKeyboardKey::CONTROL_LEFT,
            LogicalKeyboardKey::KEY_A,
        ]);
        assert_eq!(
            LogicalKeyboardKey::collapse_synonyms(&pressed),
            HashSet::from([
                LogicalKeyboardKey::SHIFT,
                LogicalKeyboardKey::CONTROL,
                LogicalKeyboardKey::KEY_A,
            ])
        );
        assert_eq!(
            LogicalKeyboardKey::expand_synonyms(&HashSet::from([
                LogicalKeyboardKey::ALT,
                LogicalKeyboardKey::KEY_A,
            ])),
            HashSet::from([
                LogicalKeyboardKey::ALT_LEFT,
                LogicalKeyboardKey::ALT_RIGHT,
                LogicalKeyboardKey::KEY_A,
            ])
        );
    }

    #[test]
    fn autogenerated_ids_start_at_the_platform_planes() {
        assert!(!LogicalKeyboardKey::KEY_A.is_autogenerated());
        assert!(!LogicalKeyboardKey::F1.is_autogenerated());
        assert!(LogicalKeyboardKey::new(0x01100000001).is_autogenerated());
    }
}
