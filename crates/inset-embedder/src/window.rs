//! Asking for views, and the windows around them. Not a Flutter file.
//!
//! An app asks for a view with a [`ViewRequest`] (`Platform::request_view`), and the view
//! arrives later, as every view does, carrying the request's [`RequestId`], as iOS hands back
//! the activity of `requestSceneSessionActivation` with the connecting scene. On a desktop the
//! host makes a window for it, and on Android a task, Android's own window, with its card in
//! Recents; where the platform gives the app views and makes none, on an iPhone and when
//! embedded, a request is refused. A view in a window the app may manage, one it asked for or
//! one the platform gave, has a [`HostWindow`] (`View::window`): the configuration every window
//! wants up front, whether the window is minimized or occluded, which the host reports with the
//! window's moves (`EmbedderClient::window_changed`), the native handle for anything beyond it,
//! and the window's closing, which is how a view closes.
//!
//! Flutter's windowing API (`widgets/_window.dart`, experimental) builds window archetypes
//! over the engine's views, each controller making its window within its constructor.

use std::fmt;
use std::rc::Rc;
use std::time::Duration;

pub use raw_window_handle::RawWindowHandle;

use crate::{IconImage, Rect, ViewRef};

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
    /// able to refuse activation.
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
            size: [900.0, 600.0],
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

/// What an app asks for when it asks for a view.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViewRequest {
    /// The window a desktop makes for the view.
    pub config: WindowConfig,
    /// What the view carries when it arrives ([`View::initial_data`](crate::View::initial_data)),
    /// such as which screen to show.
    pub initial_data: Option<String>,
}

/// The number of a request for a view, which the view carries when it arrives
/// ([`View::request_id`](crate::View::request_id)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RequestId(pub u64);

/// Why a request for a view got no view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewError {
    /// This platform gives the app its views and makes none on request.
    Unsupported,
    Failed(String),
}

impl fmt::Display for ViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewError::Unsupported => f.write_str("the platform makes no views on request"),
            ViewError::Failed(reason) => {
                write!(f, "the platform could not make the view: {reason}")
            }
        }
    }
}

impl std::error::Error for ViewError {}

pub type WindowRef = Rc<dyn HostWindow>;

/// The window around a view the host puts on screen in a window the app may manage.
pub trait HostWindow: 'static {
    fn view(&self) -> ViewRef;

    /// The window's logical frame in screen space.
    fn frame(&self) -> Rect;

    /// Moves and sizes the window, over `animate` when the host can animate a frame.
    fn set_frame(&self, frame: Rect, animate: Option<Duration>);

    fn set_title(&self, title: &str);

    /// The window's own icon, where its system shows one: in the title bar on Windows and Linux,
    /// on the task's card in Recents on Android. `None` gives the window back the app's icon. A
    /// system whose windows have no icon of their own, as macOS's do not, leaves it unset.
    fn set_icon(&self, icon: Option<IconImage>);

    fn show(&self);

    fn hide(&self);

    /// Flutter `WindowController.isMinimized`; macOS `isMiniaturized`, Windows `IsIconic`.
    /// `false` where the system does not tell a client, as Wayland does not.
    fn is_minimized(&self) -> bool;

    /// Covered all over, or otherwise not shown: macOS's occlusion state without its visible bit,
    /// winit's `Occluded`. Not in Flutter's controller; Inset's, after the platforms. `false`
    /// where the system does not tell a window, as Windows and Wayland do not.
    fn is_occluded(&self) -> bool;

    /// Closes the window; its view goes away with it. The user's asking to close it only reports
    /// it ([`EmbedderClient::view_close_requested`](crate::EmbedderClient::view_close_requested));
    /// this closes it.
    fn close(&self);

    /// The native window, for configuration this interface does not carry.
    fn native_handle(&self) -> Option<RawWindowHandle>;
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
}
