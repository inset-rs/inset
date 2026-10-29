//! The system's own menus as the framework's (`inset_embedder`'s `menus`): what each system draws
//! is `os/`'s; here is what every system shares: the events a menu sends the main loop and the
//! edit command a provided item sends the focused view, a shortcut's key as each system names it
//! (`shortcut_keys`), and, for Windows and the DBus menu, a menu's entries numbered (`numbered`)
//! and an icon's pixels in their layouts (`pixels`), which build and test on every system.
//!
//! A menu's choice reaches the main loop as a [`MenuEvent`] through the loop's proxy, from wherever
//! the system reports it: AppKit's action, Win32's window procedure, or DBus's thread. The loop
//! tells the app on a turn of its own (`main_loop/`).

#![cfg_attr(
    not(any(target_os = "macos", target_os = "windows", target_os = "linux")),
    allow(
        dead_code,
        reason = "a system without menus of its own sends no menu's events"
    )
)]

#[cfg(any(target_os = "windows", target_os = "linux", test))]
pub(crate) mod numbered;
#[cfg(any(target_os = "windows", target_os = "linux", test))]
pub(crate) mod pixels;
mod shortcut_keys;

use inset_embedder::{MenuId, ProvidedItem, TrayIconEvent, TrayIconId};

use crate::main_loop::{MainLoopEvent, MainLoopProxy};

#[cfg(target_os = "windows")]
pub(crate) use shortcut_keys::accelerator_text;
#[cfg(target_os = "linux")]
pub(crate) use shortcut_keys::dbus_menu_shortcut;
#[cfg(target_os = "macos")]
pub(crate) use shortcut_keys::{ShortcutKey, shortcut_key};

/// What a menu of the system's tells the app.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum MenuEvent {
    /// An item of the app's menu bar was chosen.
    #[cfg_attr(
        not(target_os = "macos"),
        allow(dead_code, reason = "only macOS has a menu bar of the system's")
    )]
    Selected(MenuId),
    /// An item of the Dock icon's menu was chosen.
    #[cfg_attr(
        not(target_os = "macos"),
        allow(dead_code, reason = "only macOS has a Dock")
    )]
    DockSelected(MenuId),
    /// A provided edit command was chosen, for the focused view.
    Edit(EditAction),
    /// Something happened to a tray icon.
    Tray(TrayIconId, TrayIconEvent),
}

/// The edit command of a provided item, which the focused view hears: cut, copy and paste as a
/// `clipboard_event`, the rest as an `edit_command`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditAction {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Delete,
}

impl EditAction {
    /// The edit command `item` is, if it is one.
    pub(crate) fn of(item: ProvidedItem) -> Option<EditAction> {
        Some(match item {
            ProvidedItem::Undo => EditAction::Undo,
            ProvidedItem::Redo => EditAction::Redo,
            ProvidedItem::Cut => EditAction::Cut,
            ProvidedItem::Copy => EditAction::Copy,
            ProvidedItem::Paste => EditAction::Paste,
            ProvidedItem::SelectAll => EditAction::SelectAll,
            ProvidedItem::Delete => EditAction::Delete,
            ProvidedItem::About
            | ProvidedItem::Services
            | ProvidedItem::Hide
            | ProvidedItem::HideOthers
            | ProvidedItem::ShowAll
            | ProvidedItem::Quit
            | ProvidedItem::Minimize
            | ProvidedItem::Zoom
            | ProvidedItem::ToggleFullScreen
            | ProvidedItem::BringAllToFront => return None,
        })
    }
}

/// Where a system's menus send what they tell the app: the main loop, through its proxy,
/// from any thread.
#[derive(Clone)]
pub(crate) struct MenuEvents(MainLoopProxy);

impl MenuEvents {
    pub(crate) fn new(proxy: MainLoopProxy) -> MenuEvents {
        MenuEvents(proxy)
    }

    pub(crate) fn send(&self, event: MenuEvent) {
        self.0.send(MainLoopEvent::Menu(event));
    }

    /// A menu that ran its own loop closed: the buttons it kept are reconciled before the app
    /// hears what was chosen.
    pub(crate) fn closed(&self) {
        self.0.send(MainLoopEvent::MenuClosed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_commands_are_the_provided_edit_items() {
        assert_eq!(EditAction::of(ProvidedItem::Copy), Some(EditAction::Copy));
        assert_eq!(
            EditAction::of(ProvidedItem::SelectAll),
            Some(EditAction::SelectAll)
        );
        assert_eq!(EditAction::of(ProvidedItem::Quit), None);
    }
}
