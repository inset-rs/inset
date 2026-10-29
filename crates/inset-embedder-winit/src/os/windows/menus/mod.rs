//! Win32's menus as the framework's: a menu at a point in a window, `TrackPopupMenuEx`'s
//! (`popup_menu`), and the app's icons in the notification area, `Shell_NotifyIcon`'s
//! (`notify_icons`), which tell what happens to them, and open their menus, through the embedder's
//! hidden window (`hidden_window`). An icon's image is a GDI object (`bitmaps`). Windows has
//! no menu bar of the system's and no Dock: those answer none, and the app draws its menu bar.
//!
//! Every menu is opened from the hidden window's procedure, on a message posted once the
//! framework's call has returned, so its modal loop runs outside every call into the app, while
//! Windows goes on dispatching winit's window messages, as it does while a window is moved.

mod bitmaps;
mod notify_icons;
mod popup_menu;

pub(super) use notify_icons::{notified as icon_notified, taskbar_created};

use std::rc::Rc;

use inset_embedder::{MenuEntry, MenuId, Offset, TrayIcons};
use winit::window::Window;

use super::Windows;
use crate::menus::MenuEvents;
use crate::os::NativeMenus;

impl NativeMenus for Windows {
    const POPUP_MENUS: bool = true;

    fn open_popup_menu(
        window: &dyn Window,
        at: Offset,
        menu: Vec<MenuEntry>,
        events: MenuEvents,
        closed: Box<dyn FnOnce(Option<MenuId>) + Send>,
    ) {
        popup_menu::open_in_window(window, at, menu, events, closed);
    }

    fn tray_icons(events: MenuEvents) -> Option<Rc<dyn TrayIcons>> {
        Some(Rc::new(notify_icons::NotifyIcons::new(events)))
    }
}
