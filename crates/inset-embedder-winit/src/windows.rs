//! Windows the app creates: asked for on the app's turn, made on the event loop's.
//!
//! winit hands out its `ActiveEventLoop` only inside its callbacks, so a request waits in
//! the platform until the loop next turns, and the window arrives through the future the
//! request returned. The host then runs the window like the implicit view: one surface,
//! one `View`, the same events.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Weak;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use inset_embedder::{
    HostWindow, RawWindowHandle, Rect, ViewRef, WindowConfig, WindowError, WindowFuture,
    WindowLevel, WindowRef, WindowingOwner,
};
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event_loop::EventLoopProxy;
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::os;
use crate::view::WinitView;
use crate::window::HostEvent;

/// A window the app asked for, waiting for the loop.
pub(crate) struct WindowRequest {
    pub config: WindowConfig,
    pub reply: Reply<Result<WindowRef, WindowError>>,
}

/// The platform's window maker: queues requests and wakes the loop to serve them.
pub(crate) struct WinitWindowing {
    requests: RefCell<Vec<WindowRequest>>,
    proxy: EventLoopProxy<HostEvent>,
}

impl WinitWindowing {
    pub(crate) fn new(proxy: EventLoopProxy<HostEvent>) -> WinitWindowing {
        WinitWindowing {
            requests: RefCell::new(Vec::new()),
            proxy,
        }
    }

    pub(crate) fn take_requests(&self) -> Vec<WindowRequest> {
        std::mem::take(&mut *self.requests.borrow_mut())
    }
}

impl WindowingOwner for WinitWindowing {
    fn create(&self, config: WindowConfig) -> WindowFuture {
        let (reply, future) = reply();
        self.requests
            .borrow_mut()
            .push(WindowRequest { config, reply });
        let _ = self.proxy.send_event(HostEvent::Requests);
        Box::pin(future)
    }
}

/// winit's attributes for a configuration. What winit has no attribute for, the system's
/// module applies after creation.
pub(crate) fn attributes_for(config: &WindowConfig) -> WindowAttributes {
    let level = match config.level {
        WindowLevel::Normal => winit::window::WindowLevel::Normal,
        WindowLevel::AlwaysOnTop => winit::window::WindowLevel::AlwaysOnTop,
        WindowLevel::AlwaysOnBottom => winit::window::WindowLevel::AlwaysOnBottom,
    };
    let mut attributes = Window::default_attributes()
        .with_title(&config.title)
        .with_inner_size(LogicalSize::new(config.size[0], config.size[1]))
        .with_decorations(config.decorations)
        .with_resizable(config.resizable)
        .with_transparent(config.background.sees_through())
        .with_window_level(level)
        .with_visible(config.visible)
        .with_active(config.activating);
    if let Some([x, y]) = config.position {
        attributes = attributes.with_position(LogicalPosition::new(x, y));
    }
    os::extend_attributes(attributes, config)
}

/// A created window as the app holds it. Holds the window weakly: the host owns it, and
/// dropping the host's handle is what closes it.
pub(crate) struct WinitWindow {
    id: WindowId,
    window: Weak<Window>,
    view: Rc<WinitView>,
    proxy: EventLoopProxy<HostEvent>,
    close_requested: RefCell<Option<Rc<dyn Fn()>>>,
}

impl WinitWindow {
    pub(crate) fn new(
        id: WindowId,
        window: Weak<Window>,
        view: Rc<WinitView>,
        proxy: EventLoopProxy<HostEvent>,
    ) -> WinitWindow {
        WinitWindow {
            id,
            window,
            view,
            proxy,
            close_requested: RefCell::new(None),
        }
    }

    pub(crate) fn close_handler(&self) -> Option<Rc<dyn Fn()>> {
        self.close_requested.borrow().clone()
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
            && os::animate_frame(&window, frame, duration)
        {
            return;
        }
        if os::set_frame(&window, frame) {
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
        }
    }

    fn hide(&self) {
        if let Some(window) = self.window.upgrade() {
            window.set_visible(false);
        }
    }

    fn close(&self) {
        // Off screen at once; the host drops the window when it next turns, and the
        // framework lets go of the view when its `View` widget goes.
        if let Some(window) = self.window.upgrade() {
            window.set_visible(false);
        }
        let _ = self.proxy.send_event(HostEvent::CloseWindow(self.id));
    }

    fn native_handle(&self) -> Option<RawWindowHandle> {
        let window = self.window.upgrade()?;
        window.window_handle().ok().map(|handle| handle.as_raw())
    }

    fn set_close_requested(&self, handler: Option<Rc<dyn Fn()>>) {
        *self.close_requested.borrow_mut() = handler;
    }
}

/// One answer, delivered once, to a future polled at the app's checkpoints.
struct Shared<T> {
    value: Option<T>,
    waker: Option<Waker>,
}

pub(crate) struct Reply<T>(Rc<RefCell<Shared<T>>>);

impl<T> Reply<T> {
    pub(crate) fn complete(self, value: T) {
        let waker = {
            let mut shared = self.0.borrow_mut();
            shared.value = Some(value);
            shared.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

struct ReplyFuture<T>(Rc<RefCell<Shared<T>>>);

impl<T> Future for ReplyFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut shared = self.0.borrow_mut();
        match shared.value.take() {
            Some(value) => Poll::Ready(value),
            None => {
                shared.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

fn reply<T>() -> (Reply<T>, ReplyFuture<T>) {
    let shared = Rc::new(RefCell::new(Shared {
        value: None,
        waker: None,
    }));
    (Reply(Rc::clone(&shared)), ReplyFuture(shared))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_resolves_its_future_once_completed() {
        let (reply, mut future) = reply::<u32>();
        let mut cx = Context::from_waker(Waker::noop());
        assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
        reply.complete(7);
        assert_eq!(Pin::new(&mut future).poll(&mut cx), Poll::Ready(7));
    }
}
