//! A window the app created, as it holds it (`HostWindow`).

use std::rc::Rc;
use std::sync::Weak;
use std::time::Duration;

use inset_embedder::{HostWindow, RawWindowHandle, Rect, ViewRef};
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{Window, WindowId};

use crate::app_requests::{AppRequest, AppRequests};
use crate::os::{ThisOs, WindowMotion};
use crate::view::WinitView;

/// A created window as the app holds it. Holds the window weakly: the embedder's registry owns it,
/// and closing it there frees it (`windows.rs`); from then on a call here changes nothing.
pub(crate) struct WinitWindow {
    id: WindowId,
    window: Weak<Window>,
    view: Rc<WinitView>,
    requests: Rc<AppRequests>,
}

impl WinitWindow {
    pub(crate) fn new(
        id: WindowId,
        window: Weak<Window>,
        view: Rc<WinitView>,
        requests: Rc<AppRequests>,
    ) -> WinitWindow {
        WinitWindow {
            id,
            window,
            view,
            requests,
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
            && ThisOs::animate_frame(&window, frame, duration)
        {
            return;
        }
        if ThisOs::set_frame(&window, frame) {
            return;
        }
        // Size before position: a system that anchors a window at its bottom-left, as
        // AppKit does, places the top edge from the size the window has at the time.
        let _ = window.request_inner_size(LogicalSize::new(frame.width(), frame.height()));
        window.set_outer_position(LogicalPosition::new(frame.left, frame.top));
    }

    fn set_title(&self, title: &str) {
        if let Some(window) = self.window.upgrade() {
            window.set_title(title);
        }
    }

    fn show(&self) {
        if let Some(window) = self.window.upgrade() {
            window.set_visible(true);
            self.view.follow_window();
        }
    }

    fn hide(&self) {
        if let Some(window) = self.window.upgrade() {
            window.set_visible(false);
            self.view.follow_window();
        }
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
