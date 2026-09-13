//! Windows the app creates, beyond the implicit view. Not a Flutter file.
//!
//! Flutter's windowing API (`widgets/_window.dart`, experimental) builds window archetypes
//! over the engine's views, and on macOS its `FlutterViewController` is a view the app places
//! in a window it owns. Here a window is a view the host puts on screen: the configuration
//! every window wants up front, the native handle for anything beyond it, and adoption for
//! a window only the app could make. Mobile and web hosts have no owner; the implicit view
//! is all there is.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration;

pub use raw_window_handle::RawWindowHandle;

use crate::{Rect, ViewRef};

/// How a window is made. Sizes and positions are logical, in the host's screen space with
/// the origin at the top left of the primary screen.
///
/// Every option names what the window is for, not how a system does it: the host maps each
/// onto what its system has, and one its system lacks it leaves at the system's default.
/// What no option expresses is set through the window's
/// [`native_handle`](HostWindow::native_handle).
#[derive(Clone, Debug, PartialEq)]
pub struct WindowConfig {
    pub title: String,
    pub size: [f64; 2],
    /// Where the window opens; the host chooses when `None`.
    pub position: Option<[f64; 2]>,
    /// The system's title bar and border.
    pub decorations: bool,
    pub resizable: bool,
    pub level: WindowLevel,
    /// Whether showing or clicking the window makes it the focused one. A host may not be
    /// able to refuse activation; a window that must never take focus is one to `adopt`.
    pub activating: bool,
    /// Whether the window is on every virtual desktop: macOS Spaces, X11 workspaces.
    pub all_desktops: bool,
    pub background: WindowBackground,
    /// Whether the window casts the system's shadow.
    pub shadow: bool,
    pub visible: bool,
}

impl Default for WindowConfig {
    fn default() -> WindowConfig {
        WindowConfig {
            title: String::new(),
            size: [400.0, 300.0],
            position: None,
            decorations: true,
            resizable: true,
            level: WindowLevel::Normal,
            activating: true,
            all_desktops: false,
            background: WindowBackground::Solid,
            shadow: true,
            visible: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowLevel {
    #[default]
    Normal,
    /// Above ordinary windows, as a palette or a panel sits.
    AlwaysOnTop,
    /// Below ordinary windows, as a desktop widget sits.
    AlwaysOnBottom,
}

/// What the window shows behind the app's painting. Anything but `Solid` lets what is
/// behind the window show where the app paints nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowBackground {
    #[default]
    Solid,
    Transparent,
    /// The system's blur of what lies behind the window (macOS vibrancy).
    Blurred,
    /// The system's glass material (macOS 26's Liquid Glass); the blur where the system
    /// has none.
    Glass,
}

impl WindowBackground {
    /// Whether what is behind the window can show through it.
    pub fn sees_through(self) -> bool {
        self != WindowBackground::Solid
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowError {
    /// This host makes no windows, or cannot adopt one.
    Unsupported,
    Failed(String),
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WindowError::Unsupported => f.write_str("the host does not support this window"),
            WindowError::Failed(reason) => {
                write!(f, "the host could not make the window: {reason}")
            }
        }
    }
}

impl std::error::Error for WindowError {}

/// A window being made: the host finishes it on its own loop.
pub type WindowFuture = Pin<Box<dyn Future<Output = Result<WindowRef, WindowError>>>>;

pub type WindowRef = Rc<dyn HostWindow>;

/// A window the host has on screen. Its [`view`](Self::view) is what the framework draws
/// into and receives input from, like the implicit view.
pub trait HostWindow: 'static {
    fn view(&self) -> ViewRef;

    /// The window's logical frame in screen space.
    fn frame(&self) -> Rect;

    /// Moves and sizes the window, over `animate` when the host can animate a frame.
    fn set_frame(&self, frame: Rect, animate: Option<Duration>);

    fn set_title(&self, title: &str);

    fn show(&self);

    fn hide(&self);

    /// Closes the window; its view goes away with it.
    fn close(&self);

    /// The native window, for configuration this interface does not carry.
    fn native_handle(&self) -> Option<RawWindowHandle>;

    /// What happens when the user asks to close the window. With no handler the host
    /// closes it.
    fn set_close_requested(&self, handler: Option<Rc<dyn Fn()>>);
}

/// The host's window maker. `None` from [`crate::Platform::windowing_owner`] where the
/// implicit view is the only one there can be.
pub trait WindowingOwner: 'static {
    /// Makes a window. The host finishes it on its own loop, so the window arrives through
    /// the future.
    fn create(&self, config: WindowConfig) -> WindowFuture;

    /// Takes over a native window the app made, for what no configuration can express:
    /// on macOS an `NSPanel` that never activates. The host installs its view in it and
    /// drives rendering and input for it. Hosts that cannot answer `Unsupported`.
    fn adopt(&self, handle: RawWindowHandle) -> WindowFuture {
        let _ = handle;
        Box::pin(async { Err(WindowError::Unsupported) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_default_window_is_an_ordinary_visible_one() {
        let config = WindowConfig::default();
        assert!(config.decorations && config.visible && config.activating);
        assert_eq!(config.level, WindowLevel::Normal);
        assert_eq!(config.background, WindowBackground::Solid);
    }

    #[test]
    fn every_background_but_solid_sees_through() {
        assert!(!WindowBackground::Solid.sees_through());
        assert!(WindowBackground::Transparent.sees_through());
        assert!(WindowBackground::Blurred.sees_through());
        assert!(WindowBackground::Glass.sees_through());
    }

    struct NoWindows;

    impl WindowingOwner for NoWindows {
        fn create(&self, _config: WindowConfig) -> WindowFuture {
            Box::pin(async { Err(WindowError::Unsupported) })
        }
    }

    #[test]
    fn adoption_is_unsupported_unless_a_host_says_otherwise() {
        let owner = NoWindows;
        let mut future = owner.adopt(RawWindowHandle::Web(
            raw_window_handle::WebWindowHandle::new(1),
        ));
        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);
        match future.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(Err(WindowError::Unsupported)) => {}
            other => panic!(
                "expected Unsupported, got {:?}",
                other.map(|r| r.map(|_| ()))
            ),
        }
    }
}
