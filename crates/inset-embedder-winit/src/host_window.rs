//! A window the app created, as it holds it (`HostWindow`), and how it was when the app last heard
//! of it: whether it was minimized and whether it was occluded, so that the embedder reports a
//! change once, whichever of the system's events it comes with.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Weak;
use std::time::Duration;

use inset_embedder::{HostWindow, IconImage, RawWindowHandle, Rect, ViewRef};
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{Icon, Window, WindowId};

use crate::app_requests::{AppRequest, AppRequests};
use crate::os::{ThisOs, WindowMotion, WindowVisibility};
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
            && ThisOs::animate_frame(&*window, frame, duration)
        {
            return;
        }
        if ThisOs::set_frame(&*window, frame) {
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

    /// winit's: the title bar's icon on Windows and X11, nothing on macOS and Wayland, whose
    /// windows have none of their own.
    fn set_icon(&self, icon: Option<IconImage>) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let icon = icon.and_then(|icon| {
            Icon::from_rgba(icon.rgba.to_vec(), icon.width, icon.height)
                .inspect_err(|error| eprintln!("inset: the window's icon was refused: {error}"))
                .ok()
        });
        window.set_window_icon(icon);
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
        ThisOs::is_occluded(&*window).unwrap_or(self.occluded.get())
    }

    fn close(&self) {
        // Off screen at once; the embedder drops the window when it next turns, and the
        // framework lets go of the view when its `View` widget goes.
        if let Some(window) = self.window.upgrade() {
            window.set_visible(false);
        }
        self.requests.send(AppRequest::CloseWindow(self.id));
    }

    fn native_handle(&self) -> Option<RawWindowHandle> {
        let window = self.window.upgrade()?;
        window.window_handle().ok().map(|handle| handle.as_raw())
    }
}
