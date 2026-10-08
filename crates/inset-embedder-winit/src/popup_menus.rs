//! The system's own popup menus, as the framework's [`PopupMenus`].

use inset_embedder::{PopupMenuEntry, PopupMenus};
use winit::event_loop::EventLoopProxy;

use crate::main_loop::MainLoopEvent;
use crate::os::{NativeMenus, ThisOs};

/// Shows a menu in the system's own loop, and tells the event loop once it has closed.
pub(crate) struct NativePopupMenus {
    proxy: EventLoopProxy<MainLoopEvent>,
}

impl NativePopupMenus {
    pub(crate) fn new(proxy: EventLoopProxy<MainLoopEvent>) -> NativePopupMenus {
        NativePopupMenus { proxy }
    }
}

impl PopupMenus for NativePopupMenus {
    fn show(&self, entries: &[PopupMenuEntry]) -> Option<usize> {
        let chosen = ThisOs::popup_menu(entries);
        // The menu ran its own event loop and kept the release of the button that opened
        // it; the next pass reconciles the buttons.
        let _ = self.proxy.send_event(MainLoopEvent::MenuClosed);
        chosen
    }
}
