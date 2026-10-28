//! A shortcut's key as the systems name the keys a menu shows: the key Flutter's logical key id
//! names ([`shortcut_key`]), and the shortcut as Windows writes it after an item's label and as the
//! DBus menu names it, which their menus show and never act on: the framework's own shortcuts do.

#[cfg(any(target_os = "windows", target_os = "linux", test))]
use inset_embedder::MenuShortcut;

/// A shortcut's key, as the systems name the keys a menu shows: a character, or a key that types
/// none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShortcutKey {
    Character(char),
    Backspace,
    Tab,
    Enter,
    Escape,
    Delete,
    Insert,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    End,
    Home,
    PageDown,
    PageUp,
    /// A function key, by its number.
    Function(u8),
}

/// Where Flutter's logical keys that type nothing start.
const UNPRINTABLE_PLANE: u64 = 0x001_0000_0000;

/// The key Flutter's logical key `key` names, where a menu can show it: a character's own key is
/// its Unicode value.
pub(crate) fn shortcut_key(key: u64) -> Option<ShortcutKey> {
    if key < UNPRINTABLE_PLANE {
        let character = char::from_u32(u32::try_from(key).ok()?)?;
        return Some(ShortcutKey::Character(character));
    }
    Some(match key - UNPRINTABLE_PLANE {
        0x008 => ShortcutKey::Backspace,
        0x009 => ShortcutKey::Tab,
        0x00d => ShortcutKey::Enter,
        0x01b => ShortcutKey::Escape,
        0x07f => ShortcutKey::Delete,
        0x301 => ShortcutKey::ArrowDown,
        0x302 => ShortcutKey::ArrowLeft,
        0x303 => ShortcutKey::ArrowRight,
        0x304 => ShortcutKey::ArrowUp,
        0x305 => ShortcutKey::End,
        0x306 => ShortcutKey::Home,
        0x307 => ShortcutKey::PageDown,
        0x308 => ShortcutKey::PageUp,
        0x407 => ShortcutKey::Insert,
        function @ 0x801..=0x818 => ShortcutKey::Function((function - 0x800) as u8),
        _ => return None,
    })
}

/// `shortcut` as Windows writes it after an item's label, behind a tab: `Ctrl+Shift+O`. None for
/// a key a menu cannot show.
#[cfg(any(target_os = "windows", test))]
pub(crate) fn accelerator_text(shortcut: &MenuShortcut) -> Option<String> {
    let key = match shortcut_key(shortcut.key)? {
        ShortcutKey::Character(' ') => "Space".to_owned(),
        ShortcutKey::Character(character) => character.to_uppercase().collect(),
        ShortcutKey::Backspace => "Backspace".to_owned(),
        ShortcutKey::Tab => "Tab".to_owned(),
        ShortcutKey::Enter => "Enter".to_owned(),
        ShortcutKey::Escape => "Esc".to_owned(),
        ShortcutKey::Delete => "Del".to_owned(),
        ShortcutKey::Insert => "Ins".to_owned(),
        ShortcutKey::ArrowDown => "Down".to_owned(),
        ShortcutKey::ArrowLeft => "Left".to_owned(),
        ShortcutKey::ArrowRight => "Right".to_owned(),
        ShortcutKey::ArrowUp => "Up".to_owned(),
        ShortcutKey::End => "End".to_owned(),
        ShortcutKey::Home => "Home".to_owned(),
        ShortcutKey::PageDown => "PgDn".to_owned(),
        ShortcutKey::PageUp => "PgUp".to_owned(),
        ShortcutKey::Function(number) => format!("F{number}"),
    };
    let modifiers = shortcut.modifiers;
    let held = [
        (modifiers.meta, "Win"),
        (modifiers.control, "Ctrl"),
        (modifiers.alt, "Alt"),
        (modifiers.shift, "Shift"),
    ];
    let mut text = String::new();
    for (_, name) in held.iter().filter(|(held, _)| *held) {
        text.push_str(name);
        text.push('+');
    }
    text.push_str(&key);
    Some(text)
}

/// `shortcut` as the DBus menu's `shortcut` property names it: the modifiers, then the key by its
/// X keysym name, as `["Control", "Shift", "o"]`. None for a key a menu cannot show.
#[cfg(any(target_os = "linux", test))]
pub(crate) fn dbus_menu_shortcut(shortcut: &MenuShortcut) -> Option<Vec<String>> {
    let key = match shortcut_key(shortcut.key)? {
        ShortcutKey::Character(' ') => "space".to_owned(),
        ShortcutKey::Character(character) => character.to_lowercase().collect(),
        ShortcutKey::Backspace => "BackSpace".to_owned(),
        ShortcutKey::Tab => "Tab".to_owned(),
        ShortcutKey::Enter => "Return".to_owned(),
        ShortcutKey::Escape => "Escape".to_owned(),
        ShortcutKey::Delete => "Delete".to_owned(),
        ShortcutKey::Insert => "Insert".to_owned(),
        ShortcutKey::ArrowDown => "Down".to_owned(),
        ShortcutKey::ArrowLeft => "Left".to_owned(),
        ShortcutKey::ArrowRight => "Right".to_owned(),
        ShortcutKey::ArrowUp => "Up".to_owned(),
        ShortcutKey::End => "End".to_owned(),
        ShortcutKey::Home => "Home".to_owned(),
        ShortcutKey::PageDown => "Page_Down".to_owned(),
        ShortcutKey::PageUp => "Page_Up".to_owned(),
        ShortcutKey::Function(number) => format!("F{number}"),
    };
    let modifiers = shortcut.modifiers;
    let mut names: Vec<String> = [
        (modifiers.control, "Control"),
        (modifiers.alt, "Alt"),
        (modifiers.shift, "Shift"),
        (modifiers.meta, "Super"),
    ]
    .into_iter()
    .filter(|(held, _)| *held)
    .map(|(_, name)| name.to_owned())
    .collect();
    names.push(key);
    Some(names)
}

#[cfg(test)]
mod tests {
    use inset_embedder::ShortcutModifiers;

    use super::*;

    #[test]
    fn a_characters_key_is_itself_and_other_keys_are_named() {
        assert_eq!(shortcut_key(0x63), Some(ShortcutKey::Character('c')));
        assert_eq!(shortcut_key(0x2c), Some(ShortcutKey::Character(',')));
        assert_eq!(shortcut_key(0x00100000304), Some(ShortcutKey::ArrowUp));
        assert_eq!(shortcut_key(0x00100000801), Some(ShortcutKey::Function(1)));
        assert_eq!(shortcut_key(0x0010000080c), Some(ShortcutKey::Function(12)));
        assert_eq!(shortcut_key(0x0010000007f), Some(ShortcutKey::Delete));
        assert_eq!(shortcut_key(0x00200000100), None, "a key no menu shows");
    }

    fn shortcut(key: u64, control: bool, shift: bool) -> MenuShortcut {
        MenuShortcut {
            key,
            modifiers: ShortcutModifiers {
                control,
                shift,
                ..ShortcutModifiers::default()
            },
        }
    }

    #[test]
    fn windows_writes_a_shortcut_with_its_modifiers_and_the_keys_name() {
        assert_eq!(
            accelerator_text(&shortcut(0x6f, true, false)).as_deref(),
            Some("Ctrl+O")
        );
        assert_eq!(
            accelerator_text(&shortcut(0x7a, true, true)).as_deref(),
            Some("Ctrl+Shift+Z")
        );
        assert_eq!(
            accelerator_text(&shortcut(0x0010000007f, false, false)).as_deref(),
            Some("Del")
        );
        assert_eq!(
            accelerator_text(&shortcut(0x00100000805, false, false)).as_deref(),
            Some("F5")
        );
        assert_eq!(
            accelerator_text(&shortcut(0x00200000100, true, false)),
            None
        );
    }

    #[test]
    fn the_dbus_menu_names_the_modifiers_then_the_keysym() {
        assert_eq!(
            dbus_menu_shortcut(&shortcut(0x6f, true, true)),
            Some(vec![
                "Control".to_owned(),
                "Shift".to_owned(),
                "o".to_owned()
            ])
        );
        assert_eq!(
            dbus_menu_shortcut(&shortcut(0x00100000308, false, false)),
            Some(vec!["Page_Up".to_owned()])
        );
    }
}
