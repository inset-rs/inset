//! Linux: what winit covers by itself, arboard's plain text for the clipboard, the desktop's tray,
//! whose icons are StatusNotifierItems with their DBus menus (`status_notifier_items`), and the
//! desktop portal's file dialogs (`file_dialogs`).
//! Linux's desktops have no menu bar and no menu at a point of the system's: GTK and Qt draw
//! theirs in the app's own surfaces, so those answer none and the app draws its own.

mod file_dialogs;
mod notifications;
mod opener;
mod portal;
mod session_bus;
mod status_notifier_items;

use std::rc::Rc;

use inset_embedder::{TargetPlatform, TrayIcons};

use crate::clipboard::{ChangeProbe, TextClipboard};
use crate::menus::MenuEvents;

use super::absent::{
    NoDisplayLink, NoTrackpadTouchObserver, NoVisibilityNotices, NoWindowSurfaces,
};
use super::{
    ApplicationControl, ApplicationLifecycle, FramePresentation, ImageDecoding, NativeComposition,
    NativeMenus, OperatingSystem, PointerQueries, Screens, SystemChanges, SystemClipboard,
    SystemMedia, SystemRequests, TrackpadTouches, WindowChrome, WindowGeometry, WindowMotion,
    WindowVisibility, WorkQueue,
};

/// Linux. A type only: never made, its answers are associated functions.
pub(crate) enum Linux {}

impl OperatingSystem for Linux {
    const TARGET_PLATFORM: TargetPlatform = TargetPlatform::Linux;
}

/// arboard's plain text, as winit has no clipboard (`clipboard/text.rs`), its changes told by the
/// text itself, as arboard has no cheaper probe there; a copy that expires is cleared by the main
/// loop.
impl SystemClipboard for Linux {
    type Clipboard = TextClipboard;

    fn clipboard(_look: Box<dyn Fn()>) -> TextClipboard {
        TextClipboard::new(ChangeProbe::Text)
    }
}

/// The tray, where the desktop shows one.
impl NativeMenus for Linux {
    fn tray_icons(events: MenuEvents) -> Option<Rc<dyn TrayIcons>> {
        let items = status_notifier_items::StatusNotifierItems::new(events)?;
        Some(Rc::new(items))
    }
}

impl FramePresentation for Linux {
    type DisplayLink = NoDisplayLink;
}

impl NativeComposition for Linux {
    type WindowSurfaces = NoWindowSurfaces;
}

impl WindowVisibility for Linux {
    type Notices = NoVisibilityNotices;
}

impl TrackpadTouches for Linux {
    type Observer = NoTrackpadTouchObserver;
}

impl WindowGeometry for Linux {}
impl SystemMedia for Linux {}
impl WindowMotion for Linux {}
impl WindowChrome for Linux {}
impl Screens for Linux {}
impl PointerQueries for Linux {}
impl ImageDecoding for Linux {}
impl WorkQueue for Linux {}
/// The system starts the app with what it opens on its command line, which is the app's own.
impl SystemRequests for Linux {}
impl SystemChanges for Linux {}
impl ApplicationControl for Linux {}
impl ApplicationLifecycle for Linux {}
