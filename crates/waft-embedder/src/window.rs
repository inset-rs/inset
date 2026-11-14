//! Asking for views, and the windows around them. Not a Flutter file.
//!
//! An app asks for a view in a window configured as a [`WindowConfig`]
//! (`Platform::request_view`), and the view arrives later, as every view does, carrying the
//! request's [`RequestId`], as iOS hands back the activity of `requestSceneSessionActivation` with
//! the connecting scene. On a desktop the host makes the window, and on Android a task with an
//! activity of its own; where the platform gives the app views and makes none, on an iPhone and
//! when embedded, a request is refused. A view in a window
//! the app may manage, one it asked for or one the platform gave, has a [`HostWindow`]
//! (`View::window`): the configuration every window wants up front, whether the window is
//! minimized or occluded, which the host reports with the window's moves
//! (`EmbedderClient::window_changed`), the limits the user resizes it within where users resize
//! windows ([`WindowSizeLimits`]), the native handle for anything beyond it, and the window's
//! closing, which is how a view closes. On Android a window is an activity's task, the card in
//! Recents: its title and icon are the card's, closing it removes the task, and minimizing it sends
//! the task to the background.
//!
//! Flutter's windowing API (`widgets/_window.dart`, experimental) builds window archetypes
//! over the engine's views, each controller making its window within its constructor.

use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

pub use raw_window_handle::RawWindowHandle;

use crate::{IconImage, Rect, Size, ViewRef};

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
    /// The window's own icon, where the system shows one for a window rather than for the app:
    /// the title bar's and the taskbar's on Windows and X11, the card's in Android's Recents. The
    /// app's icon where `None`.
    pub icon: Option<IconImage>,
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
    /// What only macOS windows offer; other systems leave it out.
    pub macos: MacosWindowConfig,
    /// What only Windows' windows offer; other systems leave it out.
    pub windows: WindowsWindowConfig,
    /// What only Linux's windows offer; other systems leave it out.
    pub linux: LinuxWindowConfig,
}

impl Default for WindowConfig {
    fn default() -> WindowConfig {
        WindowConfig {
            title: String::new(),
            icon: None,
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
            macos: MacosWindowConfig::default(),
            windows: WindowsWindowConfig::default(),
            linux: LinuxWindowConfig::default(),
        }
    }
}

/// What a window offers on macOS alone.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MacosWindowConfig {
    pub title_bar: MacosTitleBar,
}

/// The title bar of a macOS window.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MacosTitleBar {
    /// The system's, with the window's title in it.
    #[default]
    Standard,
    /// None of its own, as Safari's and Finder's windows: the view runs to the window's top edge,
    /// and the close, minimize and zoom buttons sit over it, centred in a band `height` points tall
    /// at the top, as far from the left edge as from the band's top. Presses in the band reach the
    /// app, which moves the window, and does a title bar's double press, from where it draws
    /// nothing to press ([`HostWindow::drag`], [`HostWindow::title_bar_double_click_action`]).
    Hidden { height: f64 },
}

/// What a window offers on Windows alone: the app it is on the taskbar, which one process may give
/// each of its windows apart, and what a pin of it starts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindowsWindowConfig {
    /// The window's own AppUserModelID (`System.AppUserModel.ID`): the taskbar groups the window
    /// with that app's, apart from the process's, and pins it as that app. The process's ID where
    /// `None`. At most 128 characters, without spaces.
    pub app_user_model_id: Option<String>,
    /// What a pin of the window starts, where no Start menu shortcut carries its ID: such a
    /// shortcut is what the taskbar pins instead. Only with an `app_user_model_id`.
    pub relaunch: Option<WindowsRelaunch>,
}

/// What a pin of a window starts, and how the pin shows (`System.AppUserModel.RelaunchCommand`,
/// `RelaunchDisplayNameResource`, `RelaunchIconResource`).
#[derive(Clone, Debug, PartialEq)]
pub struct WindowsRelaunch {
    /// The whole command line a pin runs, its program quoted where its path has spaces.
    pub command: String,
    /// The pin's name.
    pub display_name: String,
    /// The pin's icon; the program's where `None`. An icon Windows cannot load makes it drop the
    /// whole relaunch, command and name too, without a word: the pin then starts the process's
    /// executable alone.
    pub icon: Option<WindowsIconResource>,
}

/// An icon in a file, as Windows names one: an `.ico` file and the index of its icon, or a module
/// and a resource id as a negative index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsIconResource {
    pub path: PathBuf,
    pub index: i32,
}

/// What a window offers on Linux alone.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LinuxWindowConfig {
    /// The window's own app id (xdg-shell's `set_app_id`; the class, `WM_CLASS`, on X11), by which
    /// the desktop matches it to the desktop file `<app_id>.desktop`, in place of the app's own.
    /// The app's where `None`.
    pub app_id: Option<String>,
    /// The XDG activation token the window shows with, which lets it take the focus the desktop
    /// gave the launch that brought it (`XDG_ACTIVATION_TOKEN`): a later launch's, passed on.
    pub activation_token: Option<String>,
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

/// What a double-click on a window's title bar does
/// ([`HostWindow::title_bar_double_click_action`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TitleBarAction {
    Maximize,
    Minimize,
    Nothing,
    /// The host can't tell, or the system's setting has a value it doesn't know.
    #[default]
    Unknown,
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

    /// The window's own icon from now on ([`WindowConfig::icon`]); the app's icon where `None`.
    /// A system that shows no icon for a window, as macOS and Wayland do not, drops it.
    fn set_icon(&self, icon: Option<&IconImage>);

    fn show(&self);

    fn hide(&self);

    /// Flutter `WindowController.isMinimized`; macOS `isMiniaturized`, Windows `IsIconic`.
    /// `false` where the system does not tell a client, as Wayland does not.
    fn is_minimized(&self) -> bool;

    /// Covered all over, or otherwise not shown: macOS's occlusion state without its visible bit,
    /// winit's `Occluded`. Not in Flutter's controller; Waft's, after the platforms. `false`
    /// where the system does not tell a window, as Windows and Wayland do not.
    fn is_occluded(&self) -> bool;

    /// Closes the window; its view goes away with it. The user's asking to close it only reports
    /// it ([`EmbedderClient::view_close_requested`](crate::EmbedderClient::view_close_requested));
    /// this closes it.
    fn close(&self);

    /// The smallest and largest size the user may resize this window to; none where users don't
    /// resize windows (phones). Flutter `WindowController.setConstraints`.
    fn size_limits(&self) -> Option<&dyn WindowSizeLimits>;

    /// Starts the system's own move of the window from the press under way, as a press on its
    /// title bar does: for an app that draws its own ([`MacosTitleBar::Hidden`]). Call it from
    /// the press.
    fn drag(&self);

    /// Flutter `WindowController.isMaximized`; on macOS, zoomed.
    fn is_maximized(&self) -> bool;

    /// Flutter `WindowController.setMaximized`; on macOS, zooms the window or returns it to its
    /// size from before.
    fn set_maximized(&self, maximized: bool);

    /// Flutter `WindowController.setMinimized`.
    fn set_minimized(&self, minimized: bool);

    /// What the system's settings say a double-click on a title bar does, for an app that draws
    /// its own ([`MacosTitleBar::Hidden`]): macOS's "Double-click a window's title bar to".
    fn title_bar_double_click_action(&self) -> TitleBarAction {
        TitleBarAction::Unknown
    }

    /// The native window, for configuration this interface does not carry.
    fn native_handle(&self) -> Option<RawWindowHandle>;
}

/// The limits a window the user resizes keeps its view within, its content without the system's
/// title bar and border, in logical points: Flutter's `BoxConstraints` on a window, its minimum
/// and maximum apart. A limit holds from when it is set until it is set again, and `None` removes
/// it.
pub trait WindowSizeLimits {
    fn set_min_size(&self, size: Option<Size>);

    fn set_max_size(&self, size: Option<Size>);
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
        assert_eq!(config.macos.title_bar, MacosTitleBar::Standard);
        assert_eq!(config.windows, WindowsWindowConfig::default());
        assert_eq!(config.linux, LinuxWindowConfig::default());
    }

    #[test]
    fn every_background_but_solid_sees_through() {
        assert!(!WindowBackground::Solid.sees_through());
        assert!(WindowBackground::Transparent.sees_through());
        assert!(WindowBackground::Blurred.sees_through());
        assert!(WindowBackground::Glass.sees_through());
    }
}
