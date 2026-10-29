//! The Dock icon's menu: AppKit asks the application's delegate for it each time it shows it
//! (`applicationDockMenu:`), above its own items, and the embedder's delegate (`app_delegate`)
//! answers the menu last set, built from the framework's value. A choice reaches the app as
//! `dock_menu_selected`.

use std::cell::RefCell;

use inset_embedder::{DockMenu, MenuEntry};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::NSMenu;

use super::{MenuTarget, Place, native_menu};
use crate::menus::{MenuEvent, MenuEvents};

thread_local! {
    /// The menu AppKit is answered, kept until the next is set. Main thread.
    static DOCK_MENU: RefCell<Option<Retained<NSMenu>>> = const { RefCell::new(None) };
}

/// The Dock icon's menu.
pub(super) struct AppDockMenu {
    main_thread: MainThreadMarker,
    target: Retained<MenuTarget>,
}

impl AppDockMenu {
    /// The Dock icon's menu, empty.
    pub(super) fn new(main_thread: MainThreadMarker, events: MenuEvents) -> AppDockMenu {
        let chosen = events.clone();
        let target = MenuTarget::new(
            main_thread,
            events,
            Box::new(move |id| chosen.send(MenuEvent::DockSelected(id))),
        );
        AppDockMenu {
            main_thread,
            target,
        }
    }
}

impl DockMenu for AppDockMenu {
    fn set_menu(&self, menu: &[MenuEntry]) {
        let built = (!menu.is_empty())
            .then(|| native_menu(self.main_thread, "", menu, &self.target, Place::Elsewhere));
        DOCK_MENU.set(built);
    }
}

/// The menu the delegate answers `applicationDockMenu:` with: the one last set, or none.
pub(super) fn shown() -> Option<Retained<NSMenu>> {
    DOCK_MENU.with_borrow(Clone::clone)
}
