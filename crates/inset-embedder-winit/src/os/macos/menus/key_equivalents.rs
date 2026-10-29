//! Keys go to the framework first, and a key equivalent the framework leaves goes to the menu bar
//! after, as AppKit offers a key equivalent to the key window's views before its main menu, and
//! Flutter's macOS embedder redispatches a key its framework left.
//!
//! winit's view takes no key equivalent, so AppKit would offer each key to the main menu before
//! the view hears it. The menu bar's top menu is a [`MainMenu`], which declines every key while
//! one of the app's own windows is key, so AppKit hands the key to the window's view, and winit
//! to the framework; once the framework leaves it, the main loop offers it to the menu again
//! ([`perform_key_equivalent`]), which then takes it if an item has it. A key while another
//! window is key, as the About panel, or none, goes to the menu at once.
//!
//! An app that answers a key later than AppKit lets it wait claims it, and hands it back to the
//! menu bar once it leaves it ([`perform_kept_key_equivalent`]): each key-down reported is kept a
//! short while (`kept_keys`), and offered to the menu bar the same way. A key no longer kept
//! answers no rather than an event rebuilt from its data, which has no modifiers and no keyboard
//! layout, and so could choose another item than the one the user's keys name.
//!
//! The invariant: the menu declines a key only while the app's own window is key, so a key never
//! goes to nobody.

use std::cell::{Cell, RefCell};
use std::time::Instant;

use inset_embedder::{KeyData, KeyEventType, MenuShortcut};
use objc2::rc::{Retained, Weak};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSApplication, NSEvent, NSEventModifierFlags, NSEventType, NSMenu, NSWindow};
use objc2_foundation::NSString;
use winit::window::Window;

use super::super::{abort_on_panic, ns_window};
use super::kept_keys::KeptKeys;
use crate::menus::{ShortcutKey, shortcut_key};

thread_local! {
    /// The app's own windows, whose keys go to the framework first. Main thread.
    static APP_WINDOWS: RefCell<Vec<Weak<NSWindow>>> = RefCell::default();
    /// Whether the main loop is offering the menu a key the framework left. Main thread.
    static REDISPATCHING: Cell<bool> = const { Cell::new(false) };
    /// The key-downs AppKit delivered lately, by the key reported for each. Main thread.
    static KEPT_KEYS: RefCell<KeptKeys<Retained<NSEvent>>> = const { RefCell::new(KeptKeys::new()) };
}

/// `window` is one of the app's own, whose keys go to the framework before the menu bar.
pub(in crate::os::macos) fn app_window_opened(window: &dyn Window) {
    let Some(window) = ns_window(window) else {
        return;
    };
    APP_WINDOWS.with_borrow_mut(|windows| {
        windows.retain(|kept| kept.load().is_some());
        windows.push(Weak::from_retained(&window));
    });
}

/// Whether the key window is one of the app's own.
fn app_window_is_key(main_thread: MainThreadMarker) -> bool {
    let Some(key) = NSApplication::sharedApplication(main_thread).keyWindow() else {
        return false;
    };
    APP_WINDOWS.with_borrow(|windows| {
        windows
            .iter()
            .filter_map(Weak::load)
            .any(|window| Retained::as_ptr(&window) == Retained::as_ptr(&key))
    })
}

/// Offers the key-down AppKit is delivering, which the framework left, to the menu bar; answers
/// whether an item took it.
pub(super) fn perform_key_equivalent() -> bool {
    let Some(main_thread) = MainThreadMarker::new() else {
        return false;
    };
    match key_down_being_delivered(main_thread) {
        Some(event) => offer_to_menu_bar(main_thread, &event),
        None => false,
    }
}

/// Keeps the key-down AppKit is delivering, which the host reports as `key`, for a later
/// [`perform_kept_key_equivalent`]. A key up, a modifier's change and a synthesized key are not
/// key equivalents, and are not kept.
pub(super) fn keep_key_event(key: &KeyData) {
    if key.synthesized || key.event_type == KeyEventType::Up {
        return;
    }
    let Some(main_thread) = MainThreadMarker::new() else {
        return;
    };
    if let Some(event) = key_down_being_delivered(main_thread) {
        KEPT_KEYS.with_borrow_mut(|kept| kept.keep(key, event, Instant::now()));
    }
}

/// Offers the menu bar the key-down kept for `key`, which an app kept back and has left; answers
/// whether an item took it, and no once the key is no longer kept.
pub(super) fn perform_kept_key_equivalent(main_thread: MainThreadMarker, key: &KeyData) -> bool {
    match KEPT_KEYS.with_borrow_mut(|kept| kept.take(key, Instant::now())) {
        Some(event) => offer_to_menu_bar(main_thread, &event),
        None => false,
    }
}

fn key_down_being_delivered(main_thread: MainThreadMarker) -> Option<Retained<NSEvent>> {
    let event = NSApplication::sharedApplication(main_thread).currentEvent()?;
    (event.r#type() == NSEventType::KeyDown).then_some(event)
}

/// Offers `event` to the menu bar past its wait for the framework; answers whether an item took
/// it.
fn offer_to_menu_bar(main_thread: MainThreadMarker, event: &NSEvent) -> bool {
    let Some(menu) = NSApplication::sharedApplication(main_thread).mainMenu() else {
        return false;
    };
    REDISPATCHING.set(true);
    let taken = menu.performKeyEquivalent(event);
    REDISPATCHING.set(false);
    taken
}

define_class!(
    // SAFETY: NSMenu has no subclassing requirements, and `MainMenu` adds no state.
    #[unsafe(super(NSMenu))]
    #[thread_kind = MainThreadOnly]
    #[name = "InsetMainMenu"]
    pub(super) struct MainMenu;

    impl MainMenu {
        /// Declines a key while one of the app's windows is key and the framework has not left
        /// it yet.
        #[unsafe(method(performKeyEquivalent:))]
        fn perform_key_equivalent(&self, event: &NSEvent) -> bool {
            abort_on_panic(|| {
                if !REDISPATCHING.get() && app_window_is_key(self.mtm()) {
                    return false;
                }
                // SAFETY: NSMenu's own `performKeyEquivalent:`, with the event AppKit passed.
                unsafe { msg_send![super(self), performKeyEquivalent: event] }
            })
        }
    }
);

impl MainMenu {
    /// An empty menu bar.
    pub(super) fn new(main_thread: MainThreadMarker) -> Retained<MainMenu> {
        let this = main_thread.alloc::<MainMenu>();
        // SAFETY: NSMenu's designated initialiser, on an allocated instance.
        unsafe { msg_send![this, initWithTitle: &*NSString::from_str("")] }
    }
}

/// The key equivalent AppKit shows and matches for `shortcut`, and its modifiers; none for a key
/// a menu cannot show.
pub(super) fn key_equivalent(shortcut: &MenuShortcut) -> Option<(String, NSEventModifierFlags)> {
    let key = match shortcut_key(shortcut.key)? {
        ShortcutKey::Character(character) => character.to_lowercase().collect(),
        ShortcutKey::Backspace => "\u{8}".to_owned(),
        ShortcutKey::Tab => "\t".to_owned(),
        ShortcutKey::Enter => "\r".to_owned(),
        ShortcutKey::Escape => "\u{1b}".to_owned(),
        ShortcutKey::Delete => '\u{F728}'.to_string(),
        ShortcutKey::Insert => '\u{F727}'.to_string(),
        ShortcutKey::ArrowUp => '\u{F700}'.to_string(),
        ShortcutKey::ArrowDown => '\u{F701}'.to_string(),
        ShortcutKey::ArrowLeft => '\u{F702}'.to_string(),
        ShortcutKey::ArrowRight => '\u{F703}'.to_string(),
        ShortcutKey::Home => '\u{F729}'.to_string(),
        ShortcutKey::End => '\u{F72B}'.to_string(),
        ShortcutKey::PageUp => '\u{F72C}'.to_string(),
        ShortcutKey::PageDown => '\u{F72D}'.to_string(),
        ShortcutKey::Function(number) => char::from_u32(0xF703 + u32::from(number))?.to_string(),
    };
    let modifiers = shortcut.modifiers;
    let mut mask = NSEventModifierFlags::empty();
    for (held, flag) in [
        (modifiers.meta, NSEventModifierFlags::Command),
        (modifiers.control, NSEventModifierFlags::Control),
        (modifiers.alt, NSEventModifierFlags::Option),
        (modifiers.shift, NSEventModifierFlags::Shift),
    ] {
        if held {
            mask |= flag;
        }
    }
    Some((key, mask))
}

#[cfg(test)]
mod tests {
    use inset_embedder::ShortcutModifiers;

    use super::*;

    fn shortcut(key: u64, meta: bool, shift: bool) -> MenuShortcut {
        MenuShortcut {
            key,
            modifiers: ShortcutModifiers {
                meta,
                shift,
                ..ShortcutModifiers::default()
            },
        }
    }

    #[test]
    fn a_shortcut_is_appkits_key_equivalent_with_its_modifiers() {
        assert_eq!(
            key_equivalent(&shortcut(0x6f, true, false)),
            Some(("o".to_owned(), NSEventModifierFlags::Command))
        );
        assert_eq!(
            key_equivalent(&shortcut(0x7a, true, true)),
            Some((
                "z".to_owned(),
                NSEventModifierFlags::Command | NSEventModifierFlags::Shift
            ))
        );
        assert_eq!(
            key_equivalent(&shortcut(0x00100000801, false, false)),
            Some(('\u{F704}'.to_string(), NSEventModifierFlags::empty())),
            "F1"
        );
        assert_eq!(
            key_equivalent(&shortcut(0x00100000304, true, false)),
            Some(('\u{F700}'.to_string(), NSEventModifierFlags::Command)),
            "the up arrow"
        );
    }
}
