//! The Dock icon's menu: AppKit asks the application's delegate for it each time it shows it
//! (`applicationDockMenu:`), above its own items. winit's delegate does not answer that, so the
//! method is added to its class, as `app_delegate` adds the delegate's other methods, and answers
//! the menu last set, built from the framework's value. A choice reaches the app as
//! `dock_menu_selected`.

use std::cell::RefCell;

use inset_embedder::{DockMenu, MenuEntry};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{NSApplication, NSMenu};

use super::super::app_delegate::{add_method, delegate_class};
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
    /// The Dock icon's menu, empty; none where there is no delegate to answer AppKit through.
    pub(super) fn new(main_thread: MainThreadMarker, events: MenuEvents) -> Option<AppDockMenu> {
        let class = delegate_class()?;
        add_method(
            class,
            sel!(applicationDockMenu:),
            dock_menu as extern "C" fn(_, _, _) -> _,
        );
        let chosen = events.clone();
        let target = MenuTarget::new(
            main_thread,
            events,
            Box::new(move |id| chosen.send(MenuEvent::DockSelected(id))),
        );
        Some(AppDockMenu {
            main_thread,
            target,
        })
    }
}

impl DockMenu for AppDockMenu {
    fn set_menu(&self, menu: &[MenuEntry]) {
        let built = (!menu.is_empty())
            .then(|| native_menu(self.main_thread, "", menu, &self.target, Place::Elsewhere));
        DOCK_MENU.set(built);
    }
}

/// `applicationDockMenu:`: the menu last set, or none.
extern "C" fn dock_menu(
    _delegate: &AnyObject,
    _cmd: Sel,
    _application: &NSApplication,
) -> Option<&'static NSMenu> {
    DOCK_MENU.with_borrow(|menu| {
        // SAFETY: the menu is kept until the next one is set, which happens on this thread outside
        // AppKit's call; AppKit retains what it is answered before it shows it.
        menu.as_ref()
            .map(|menu| unsafe { &*Retained::as_ptr(menu) })
    })
}
