//! Linux: what winit covers by itself, arboard's plain text for the clipboard, the desktop's tray,
//! whose icons are StatusNotifierItems with their DBus menus (`status_notifier_items`), the
//! desktop portal's file dialogs (`file_dialogs`), the app's id on each window, by which the
//! desktop knows whose window it is, and on Wayland composition layers as subsurfaces showing
//! dmabufs (`composition`, `dmabuf`, `wayland`).
//! Linux's desktops have no menu bar and no menu at a point of the system's: GTK and Qt draw
//! theirs in the app's own surfaces, so those answer none and the app draws its own.

mod composition;
mod dmabuf;
mod file_dialogs;
mod notifications;
mod opener;
mod portal;
mod session_bus;
mod status_notifier_items;
mod wayland;

pub use dmabuf::DmabufBuffer;

use std::rc::Rc;
use std::sync::Arc;

use waft_embedder::{
    Compositor, HardwareBufferAllocator, TargetPlatform, TrayIcons, ViewId, WindowConfig,
};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::monitor::MonitorHandle;
use winit::platform::wayland::{ActiveEventLoopExtWayland, WindowAttributesWayland};
use winit::platform::x11::{ActiveEventLoopExtX11, WindowAttributesX11};
use winit::raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use winit::window::{Window, WindowAttributes};

use crate::LinuxSettings;
use crate::clipboard::{ChangeProbe, TextClipboard};
use crate::menus::MenuEvents;

use super::absent::{NoDisplayLink, NoTrackpadTouchObserver, NoVisibilityNotices};
use super::capabilities::{
    ApplicationControl, ApplicationLifecycle, EventLoopSetup, FramePresentation, GpuInterop,
    ImageDecoding, NativeComposition, NativeMenus, OperatingSystem, PointerQueries, Screens,
    SystemChanges, SystemClipboard, SystemDrags, SystemImageCodecs, SystemMedia, SystemRequests,
    TrackpadTouches, WindowChrome, WindowGeometry, WindowMotion, WindowVisibility, WorkQueue,
};
use super::window_handle_layer::{NoWindowHold, WindowHandleLayer};
use composition::{LinuxComposition, WaylandCompositor};
use dmabuf::GbmAllocator;

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

/// Composition layers as synchronized subsurfaces beneath the window's surface on Wayland
/// (`composition`), their buffers dmabufs GBM allocates (`dmabuf`); X11 shows none.
impl NativeComposition for Linux {
    type ViewLayer = WindowHandleLayer<LinuxComposition>;
    type WindowHold = NoWindowHold;

    fn compositor() -> Option<Arc<dyn Compositor>> {
        let compositor = WaylandCompositor::new()?;
        Some(Arc::new(compositor))
    }

    fn hardware_buffer_allocator() -> Option<Arc<dyn HardwareBufferAllocator>> {
        let allocator = GbmAllocator::new()?;
        Some(Arc::new(allocator))
    }

    fn view_layer_in(window: &Arc<dyn Window>) -> (Self::ViewLayer, NoWindowHold) {
        WindowHandleLayer::in_window(window)
    }

    fn wake_on_layer_presents(wake: Box<dyn Fn() + Send + Sync>) {
        composition::wake_on_layer_presents(wake);
    }

    fn take_presented_views() -> Vec<ViewId> {
        composition::take_presented_views()
    }
}

impl WindowVisibility for Linux {
    type Notices = NoVisibilityNotices;
}

impl TrackpadTouches for Linux {
    type Observer = NoTrackpadTouchObserver;
}

/// The Wayland display winit connected to, which composition layers share (`wayland`).
impl EventLoopSetup for Linux {
    fn event_loop_built(event_loop: &EventLoop) {
        let Ok(handle) = event_loop.display_handle() else {
            return;
        };
        if let RawDisplayHandle::Wayland(display) = handle.as_raw() {
            wayland::set_display(display.display.as_ptr());
        }
    }
}
impl WindowGeometry for Linux {}
impl SystemMedia for Linux {}
impl SystemImageCodecs for Linux {}
impl SystemDrags for Linux {}
impl WindowMotion for Linux {}
/// The app's id as each window's app id on Wayland (xdg-shell's `set_app_id`) and its class on
/// X11 (`WM_CLASS`), by which the desktop matches the window to the app's desktop file,
/// `<app_id>.desktop`, whose `StartupWMClass` names the same: its dock shows the app's name and
/// icon for the window.
impl WindowChrome for Linux {
    fn extend_attributes(
        event_loop: &dyn ActiveEventLoop,
        attributes: WindowAttributes,
        _config: &WindowConfig,
        linux: &LinuxSettings,
    ) -> WindowAttributes {
        let Some(app_id) = &linux.app_id else {
            return attributes;
        };
        if event_loop.is_wayland() {
            let wayland = WindowAttributesWayland::default().with_name(app_id, app_id);
            attributes.with_platform_attributes(Box::new(wayland))
        } else if event_loop.is_x11() {
            let x11 = WindowAttributesX11::default().with_name(app_id, app_id);
            attributes.with_platform_attributes(Box::new(x11))
        } else {
            attributes
        }
    }
}
/// A Wayland window is on no output until the compositor has shown it, and winit says nothing
/// when it then enters one: a window whose view's metrics were read before it was shown would
/// stay on no screen, and a view on no screen takes no frames, a program's in WaOS among them.
/// Such a window is on the first monitor the desktop lists until winit names its own.
impl Screens for Linux {
    fn monitor_of(window: &dyn Window) -> Option<MonitorHandle> {
        window
            .current_monitor()
            .or_else(|| window.available_monitors().next())
    }
}
impl PointerQueries for Linux {}
impl GpuInterop for Linux {}
impl ImageDecoding for Linux {}
impl WorkQueue for Linux {}
/// The system starts the app with what it opens on its command line, which is the app's own.
impl SystemRequests for Linux {}
impl SystemChanges for Linux {}
impl ApplicationControl for Linux {}
impl ApplicationLifecycle for Linux {}
