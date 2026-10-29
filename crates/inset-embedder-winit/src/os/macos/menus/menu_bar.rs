//! The menu bar: AppKit's main menu, built from the framework's value each time it is set, and the
//! system's standard one (the application's menu, Edit, Window) until the app sets its own, as
//! Flutter's macOS template and Xcode's show, in place of winit's, which is left out. The first
//! menu is titled with the process's name, whatever the app calls it, as AppKit titles it.
//!
//! What AppKit gives a menu bar is given as the value says: the Services submenu where it has the
//! provided Services item, and the window list in the menu that has the provided window items.
//! Whether the menu bar sends Edit's cut, copy and paste is what a text field asks
//! ([`provides_edit_commands`]), so it leaves those keys to the menu.

use std::cell::Cell;

use inset_embedder::{KeyData, MenuBar, MenuEntry, ProvidedItem};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
use winit::event_loop::EventLoopBuilder;
use winit::platform::macos::EventLoopBuilderExtMacOS;

use super::key_equivalents::{self, MainMenu};
use super::{MenuTarget, Place, application_name, native_item};
use crate::menus::{MenuEvent, MenuEvents};

thread_local! {
    /// Whether the menu bar holds the provided Cut, Copy and Paste. Main thread.
    static EDIT_COMMANDS: Cell<bool> = const { Cell::new(false) };
}

/// Whether the menu bar sends the focused view Edit's cut, copy and paste.
pub(super) fn provides_edit_commands() -> bool {
    EDIT_COMMANDS.get()
}

/// winit's own menu bar is left out: the system's standard one replaces it.
pub(in crate::os::macos) fn leave_winit_menu_out(builder: &mut EventLoopBuilder) {
    builder.with_default_menu(false);
}

/// The app's menu bar.
pub(super) struct AppMenuBar {
    main_thread: MainThreadMarker,
    /// What the app's items and Edit's send their actions to, kept while the menus show.
    target: Retained<MenuTarget>,
}

impl AppMenuBar {
    /// The menu bar, showing the system's standard one.
    pub(super) fn new(main_thread: MainThreadMarker, events: MenuEvents) -> AppMenuBar {
        let chosen = events.clone();
        let target = MenuTarget::new(
            main_thread,
            events,
            Box::new(move |id| chosen.send(MenuEvent::Selected(id))),
        );
        let bar = AppMenuBar {
            main_thread,
            target,
        };
        bar.set_menus(&standard_menus());
        bar
    }
}

impl MenuBar for AppMenuBar {
    fn set_menus(&self, menus: &[MenuEntry]) {
        let main_menu = MainMenu::new(self.main_thread);
        let app = NSApplication::sharedApplication(self.main_thread);
        let mut services = None;
        let mut windows = None;
        for menu in menus {
            let Some(item) = native_item(self.main_thread, menu, &self.target, Place::MenuBar)
            else {
                continue;
            };
            if let (MenuEntry::Submenu { entries, .. }, Some(submenu)) = (menu, item.submenu()) {
                if services.is_none() {
                    services = services_menu(entries, &submenu);
                }
                if windows.is_none() && holds_window_items(entries) {
                    windows = Some(submenu);
                }
            }
            main_menu.addItem(&item);
        }
        app.setMainMenu(Some(&main_menu));
        app.setServicesMenu(services.as_deref());
        app.setWindowsMenu(windows.as_deref());
        EDIT_COMMANDS.set(holds_edit_commands(menus));
    }

    fn perform_key_equivalent(&self, key: &KeyData) -> bool {
        key_equivalents::perform_kept_key_equivalent(self.main_thread, key)
    }
}

/// The Services submenu of `menu`, built from `entries`, if they hold the provided Services
/// item: in the menu bar every entry has its item, at its own place.
fn services_menu(entries: &[MenuEntry], menu: &NSMenu) -> Option<Retained<NSMenu>> {
    let index = entries.iter().position(|entry| {
        matches!(
            entry,
            MenuEntry::Provided {
                item: ProvidedItem::Services,
                ..
            }
        )
    })?;
    let item: Retained<NSMenuItem> = menu.itemAtIndex(index as isize)?;
    item.submenu()
}

/// Whether `entries` hold one of the window's provided items, which makes theirs the menu AppKit
/// lists the windows in.
fn holds_window_items(entries: &[MenuEntry]) -> bool {
    entries.iter().any(|entry| {
        matches!(
            entry,
            MenuEntry::Provided {
                item: ProvidedItem::Minimize | ProvidedItem::Zoom | ProvidedItem::BringAllToFront,
                ..
            }
        )
    })
}

/// Whether `menus` hold the provided Cut, Copy and Paste.
fn holds_edit_commands(menus: &[MenuEntry]) -> bool {
    [ProvidedItem::Cut, ProvidedItem::Copy, ProvidedItem::Paste]
        .iter()
        .all(|wanted| holds_provided(menus, *wanted))
}

fn holds_provided(entries: &[MenuEntry], wanted: ProvidedItem) -> bool {
    entries.iter().any(|entry| match entry {
        MenuEntry::Provided { item, .. } => *item == wanted,
        MenuEntry::Submenu { entries, .. } => holds_provided(entries, wanted),
        MenuEntry::Item { .. } | MenuEntry::Separator => false,
    })
}

/// The system's standard menu bar: the application's menu, Edit and Window, every item provided.
fn standard_menus() -> Vec<MenuEntry> {
    let provided = |item| MenuEntry::Provided {
        id: 0,
        item,
        enabled: true,
    };
    let menu = |label: String, entries| MenuEntry::Submenu {
        id: 0,
        label,
        enabled: true,
        entries,
    };
    vec![
        menu(
            application_name(),
            vec![
                provided(ProvidedItem::About),
                MenuEntry::Separator,
                provided(ProvidedItem::Services),
                MenuEntry::Separator,
                provided(ProvidedItem::Hide),
                provided(ProvidedItem::HideOthers),
                provided(ProvidedItem::ShowAll),
                MenuEntry::Separator,
                provided(ProvidedItem::Quit),
            ],
        ),
        menu(
            "Edit".to_owned(),
            vec![
                provided(ProvidedItem::Undo),
                provided(ProvidedItem::Redo),
                MenuEntry::Separator,
                provided(ProvidedItem::Cut),
                provided(ProvidedItem::Copy),
                provided(ProvidedItem::Paste),
                provided(ProvidedItem::Delete),
                provided(ProvidedItem::SelectAll),
            ],
        ),
        menu(
            "Window".to_owned(),
            vec![
                provided(ProvidedItem::Minimize),
                provided(ProvidedItem::Zoom),
                MenuEntry::Separator,
                provided(ProvidedItem::ToggleFullScreen),
                MenuEntry::Separator,
                provided(ProvidedItem::BringAllToFront),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standard_menu_bar_provides_edits_commands_and_a_window_menu() {
        let menus = standard_menus();
        assert!(holds_edit_commands(&menus));
        let MenuEntry::Submenu { entries, .. } = &menus[2] else {
            panic!("the Window menu");
        };
        assert!(holds_window_items(entries));
        assert!(
            !holds_edit_commands(&menus[..1]),
            "the application menu has none"
        );
    }
}
