//! Windows: what winit covers by itself, the clipboard's sequence number, which tells the app
//! another writer's change without reading the clipboard (`clipboard/text.rs`), Win32's menus at a
//! point and icons in the notification area (`menus`), the shell's file dialogs
//! (`file_dialogs`), whose loops run from the embedder's hidden window (`hidden_window`), the
//! shell's opener (`opener`), WinRT's toasts (`notifications`), the console of the process that
//! started a GUI program (`console`), the app's icon in its executable's resources, DirectComposition's
//! composition layers beneath a view drawn on a visual of its own (`composition`), over textures
//! shared by NT handle (`shared_textures`), and each display's refreshes (`display_links`).

// Win32 is reached through system calls, which Rust reaches through `unsafe`; this module and its
// children are the one place on Windows that writes it.
#![allow(unsafe_code)]

mod composition;
mod console;
mod display_links;
mod file_dialogs;
mod hidden_window;
mod menus;
mod notifications;
mod opener;
mod shared_textures;

pub use shared_textures::SharedTexture;

use std::sync::Arc;

use waft_embedder::{
    Compositor, HardwareBuffer, IconImage, TargetPlatform, TitleBarAction, WindowConfig,
};
use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON, SYSTEM_METRICS_INDEX,
};
use winit::dpi::PhysicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::icon::Icon;
use winit::platform::windows::{WinIcon, WindowAttributesWindows, WindowExtWindows};
use winit::window::{Window, WindowAttributes};

use crate::LinuxSettings;
use crate::clipboard::{ChangeProbe, TextClipboard};
use crate::host_window::winit_icon;

use super::absent::{NoTrackpadTouchObserver, NoVisibilityNotices};
use super::capabilities::{
    ApplicationControl, ApplicationLifecycle, EventLoopSetup, FramePresentation, GpuInterop,
    ImageDecoding, NativeComposition, OperatingSystem, PointerQueries, Screens, StandardStreams,
    SystemChanges, SystemClipboard, SystemDrags, SystemImageCodecs, SystemMedia, SystemRequests,
    TrackpadTouches, WindowChrome, WindowGeometry, WindowMotion, WindowVisibility, WorkQueue,
};
use super::window_handle_layer::{NoWindowHold, WindowHandleLayer};
use crate::gpu::Gpu;
use composition::{DirectComposition, WindowsCompositor};
use display_links::VBlankLink;

/// Windows. A type only: never made, its answers are associated functions.
pub(crate) enum Windows {}

impl OperatingSystem for Windows {
    const TARGET_PLATFORM: TargetPlatform = TargetPlatform::Windows;
}

/// arboard's plain text, as winit has no clipboard, its changes told by the sequence number; a
/// copy that expires is cleared by the main loop.
impl SystemClipboard for Windows {
    type Clipboard = TextClipboard;

    fn clipboard(_look: Box<dyn Fn()>) -> TextClipboard {
        TextClipboard::new(ChangeProbe::Count(sequence_number))
    }
}

/// The clipboard's sequence number, which Windows moves at each change of its contents.
fn sequence_number() -> u32 {
    // SAFETY: the call takes nothing and reads a counter of the window station's; any thread may
    // make it.
    unsafe { GetClipboardSequenceNumber() }
}

/// A display's refreshes from `IDXGIOutput::WaitForVBlank` (`display_links`).
impl FramePresentation for Windows {
    type DisplayLink = VBlankLink;
}

/// Composition layers as DirectComposition visuals beneath the view's picture, itself a visual's
/// composition swapchain, each layer showing a copy of its producer's shared texture in a swapchain
/// of its own (`composition`). Waft allocates no buffers here: its producers bring theirs.
impl NativeComposition for Windows {
    type ViewLayer = WindowHandleLayer<DirectComposition>;
    type WindowHold = NoWindowHold;

    fn compositor() -> Option<Arc<dyn Compositor>> {
        let compositor = WindowsCompositor::new()?;
        Some(Arc::new(compositor))
    }

    fn device_made(gpu: &Gpu) {
        composition::device_made(gpu);
    }

    fn view_layer_in(window: &Arc<dyn Window>) -> (Self::ViewLayer, NoWindowHold) {
        WindowHandleLayer::in_window(window)
    }
}

impl WindowVisibility for Windows {
    type Notices = NoVisibilityNotices;
}

impl TrackpadTouches for Windows {
    type Observer = NoTrackpadTouchObserver;
}

impl EventLoopSetup for Windows {}
impl WindowGeometry for Windows {}
impl SystemMedia for Windows {}
impl SystemImageCodecs for Windows {}
impl SystemDrags for Windows {}
impl WindowMotion for Windows {}
/// A window keeps two icons, as Win32 has them: winit's window icon, the title bar's, and the
/// taskbar's and the task switcher's, larger. Where it names none, each is the executable's at its
/// own size, which Windows picks out of the icon's images as it does a window class's icon. A
/// caption's double-click maximizes or restores the window, which Windows has no setting for.
impl WindowChrome for Windows {
    fn extend_attributes(
        _event_loop: &dyn ActiveEventLoop,
        attributes: WindowAttributes,
        config: &WindowConfig,
        _linux: &LinuxSettings,
    ) -> WindowAttributes {
        // The window shows DirectComposition's visuals alone, its view's picture among them.
        let windows = WindowAttributesWindows::default()
            .with_taskbar_icon(taskbar_icon(config.icon.as_ref()))
            .with_no_redirection_bitmap(true);
        attributes.with_platform_attributes(Box::new(windows))
    }

    fn app_icon() -> Option<Icon> {
        executable_icon(SM_CXSMICON, SM_CYSMICON)
    }

    fn set_taskbar_icon(window: &dyn Window, icon: Option<&IconImage>) {
        window.set_taskbar_icon(taskbar_icon(icon));
    }

    fn title_bar_double_click_action() -> TitleBarAction {
        TitleBarAction::Maximize
    }
}

/// The taskbar's icon for `icon`, a window's own: the executable's where it names none.
fn taskbar_icon(icon: Option<&IconImage>) -> Option<Icon> {
    icon.and_then(winit_icon)
        .or_else(|| executable_icon(SM_CXICON, SM_CYICON))
}

/// The icon `cargo waft` puts in the executable, by the id it gives it, at the size the system
/// metrics `width` and `height` give. An executable built otherwise has none there, and its
/// windows keep the system's default icon.
fn executable_icon(width: SYSTEM_METRICS_INDEX, height: SYSTEM_METRICS_INDEX) -> Option<Icon> {
    // SAFETY: the calls read the system's metrics and take no pointers.
    let size = unsafe { [GetSystemMetrics(width), GetSystemMetrics(height)] }
        .map(|side| u32::try_from(side).unwrap_or(0));
    WinIcon::from_resource(APP_ICON_RESOURCE, Some(PhysicalSize::new(size[0], size[1])))
        .ok()
        .map(Icon::from)
}

/// The resource id of the app's icon in its executable, as `waft-cli`'s `windows_resources`
/// writes it: 1, the lowest, whose icon Explorer shows for the file.
const APP_ICON_RESOURCE: u16 = 1;

impl StandardStreams for Windows {
    fn attach_to_parent_console() {
        console::attach_to_parent_console();
    }
}

impl Screens for Windows {}
impl PointerQueries for Windows {}
/// A shared texture opened on the device, anew for each import.
impl GpuInterop for Windows {
    fn import_texture(buffer: &dyn HardwareBuffer, device: &wgpu::Device) -> Option<wgpu::Texture> {
        let buffer: &dyn std::any::Any = buffer;
        let shared = buffer.downcast_ref::<SharedTexture>()?;
        shared.open_on(
            device,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
        )
    }
}
impl ImageDecoding for Windows {}
impl WorkQueue for Windows {}
/// The system starts the app with what it opens on its command line, which is the app's own.
impl SystemRequests for Windows {}
impl SystemChanges for Windows {}
impl ApplicationControl for Windows {}
impl ApplicationLifecycle for Windows {}
