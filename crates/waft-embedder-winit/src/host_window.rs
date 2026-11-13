//! A window the app created, as it holds it (`HostWindow`), and how it was when the app last heard
//! of it: whether it was minimized and whether it was occluded, so that the embedder reports a
//! change once, whichever of the system's events it comes with.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Weak;
use std::time::Duration;

use waft_embedder::{
    HostWindow, IconImage, RawWindowHandle, Rect, Size, TitleBarAction, ViewRef, WindowSizeLimits,
};
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::icon::{Icon, RgbaIcon};
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{Window, WindowId};

use crate::app_requests::{AppRequest, AppRequests, WindowRequest};
use crate::os::System;
use crate::os::capabilities::{WindowChrome, WindowMotion, WindowVisibility};
use crate::view::WinitView;

/// A created window as the app holds it. Holds the window weakly: the embedder's registry owns it,
/// and closing it there frees it (`windows.rs`); from then on a call here changes nothing.
pub(crate) struct WinitWindow {
    id: WindowId,
    window: Weak<dyn Window>,
    view: Rc<WinitView>,
    requests: Rc<AppRequests>,
    /// winit's last `Occluded` event: whether the window is occluded where the system cannot be
    /// asked.
    occluded: Cell<bool>,
    /// How the window was when the app last heard of it.
    reported: Cell<WindowState>,
}

/// Whether a window is minimized and whether it is occluded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct WindowState {
    minimized: bool,
    occluded: bool,
}

impl WinitWindow {
    pub(crate) fn new(
        id: WindowId,
        window: Weak<dyn Window>,
        view: Rc<WinitView>,
        requests: Rc<AppRequests>,
    ) -> WinitWindow {
        let window = WinitWindow {
            id,
            window,
            view,
            requests,
            occluded: Cell::new(false),
            reported: Cell::default(),
        };
        window.reported.set(window.state());
        window
    }

    /// winit says the window is occluded, or shown: what it answers where the system cannot be
    /// asked.
    pub(crate) fn set_occluded(&self, occluded: bool) {
        self.occluded.set(occluded);
    }

    /// Reads again whether the window is minimized and whether it is occluded, and answers whether
    /// either changed since the app last heard of the window, which it is to hear of now.
    pub(crate) fn refresh_state(&self) -> bool {
        let state = self.state();
        self.reported.replace(state) != state
    }

    fn state(&self) -> WindowState {
        WindowState {
            minimized: self.is_minimized(),
            occluded: self.is_occluded(),
        }
    }
}

impl HostWindow for WinitWindow {
    fn view(&self) -> ViewRef {
        let view: ViewRef = self.view.clone();
        view
    }

    fn frame(&self) -> Rect {
        let Some(window) = self.window.upgrade() else {
            return Rect::from_ltwh(0.0, 0.0, 0.0, 0.0);
        };
        let scale = window.scale_factor();
        let position = window
            .outer_position()
            .map(|position| position.to_logical::<f64>(scale))
            .unwrap_or(LogicalPosition::new(0.0, 0.0));
        let size = window.outer_size().to_logical::<f64>(scale);
        Rect::from_ltwh(position.x, position.y, size.width, size.height)
    }

    fn set_frame(&self, frame: Rect, animate: Option<Duration>) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        if let Some(duration) = animate
            && System::animate_frame(&*window, frame, duration)
        {
            return;
        }
        if System::set_frame(&*window, frame) {
            return;
        }
        // Size before position: a system that anchors a window at its bottom-left, as
        // AppKit does, places the top edge from the size the window has at the time.
        let _ = window.request_surface_size(LogicalSize::new(frame.width(), frame.height()).into());
        window.set_outer_position(LogicalPosition::new(frame.left, frame.top).into());
    }

    fn set_title(&self, title: &str) {
        if let Some(window) = self.window.upgrade() {
            window.set_title(title);
        }
    }

    /// winit's window icon: the title bar's and the taskbar's on Windows, the window manager's on
    /// X11 and on a Wayland compositor with `xdg_toplevel_icon`; nothing on macOS, whose windows
    /// show the app's icon alone.
    fn set_icon(&self, icon: Option<&IconImage>) {
        if let Some(window) = self.window.upgrade() {
            window.set_window_icon(icon.and_then(winit_icon));
        }
    }

    fn show(&self) {
        if let Some(window) = self.window.upgrade() {
            window.set_visible(true);
        }
    }

    fn hide(&self) {
        if let Some(window) = self.window.upgrade() {
            window.set_visible(false);
        }
    }

    /// winit's answer, the system's: never on Wayland, which does not tell a client.
    fn is_minimized(&self) -> bool {
        self.window
            .upgrade()
            .and_then(|window| window.is_minimized())
            .unwrap_or(false)
    }

    /// The system's answer where it can be asked, as AppKit can; winit's last word otherwise,
    /// which Windows and Wayland never give.
    fn is_occluded(&self) -> bool {
        let Some(window) = self.window.upgrade() else {
            return false;
        };
        System::is_occluded(&*window).unwrap_or(self.occluded.get())
    }

    fn close(&self) {
        // Off screen at once; the embedder drops the window when it next turns, and the
        // framework lets go of the view when its `View` widget goes.
        if let Some(window) = self.window.upgrade() {
            window.set_visible(false);
        }
        self.requests
            .send(AppRequest::Window(WindowRequest::Close(self.id)));
    }

    /// Every window the app asks for is a desktop's, which its user resizes.
    fn size_limits(&self) -> Option<&dyn WindowSizeLimits> {
        Some(self)
    }

    /// winit's, which starts the system's own move from the press the system is handling; none
    /// starts outside a press, as there is nothing to move with.
    fn drag(&self) {
        if let Some(window) = self.window.upgrade() {
            let _ = window.drag_window();
        }
    }

    fn is_maximized(&self) -> bool {
        self.window
            .upgrade()
            .is_some_and(|window| window.is_maximized())
    }

    fn set_maximized(&self, maximized: bool) {
        if let Some(window) = self.window.upgrade() {
            window.set_maximized(maximized);
        }
    }

    fn set_minimized(&self, minimized: bool) {
        if let Some(window) = self.window.upgrade() {
            window.set_minimized(minimized);
        }
    }

    fn title_bar_double_click_action(&self) -> TitleBarAction {
        System::title_bar_double_click_action()
    }

    fn native_handle(&self) -> Option<RawWindowHandle> {
        let window = self.window.upgrade()?;
        window.window_handle().ok().map(|handle| handle.as_raw())
    }
}

/// winit's limits on the window's surface, which every desktop it serves keeps: AppKit's content
/// minimum and maximum, Windows' tracking sizes, X11's size hints and Wayland's toplevel's.
impl WindowSizeLimits for WinitWindow {
    fn set_min_size(&self, size: Option<Size>) {
        if let Some(window) = self.window.upgrade() {
            window.set_min_surface_size(size.map(logical_size));
        }
    }

    fn set_max_size(&self, size: Option<Size>) {
        if let Some(window) = self.window.upgrade() {
            window.set_max_surface_size(size.map(logical_size));
        }
    }
}

fn logical_size(size: Size) -> winit::dpi::Size {
    LogicalSize::new(size.width(), size.height()).into()
}

/// `image` as winit's window icon; none for an image whose pixels do not fill its size.
pub(crate) fn winit_icon(image: &IconImage) -> Option<Icon> {
    match RgbaIcon::new(image.rgba.to_vec(), image.width, image.height) {
        Ok(icon) => Some(icon.into()),
        Err(error) => {
            eprintln!("waft: a window icon was dropped: {error}");
            None
        }
    }
}
