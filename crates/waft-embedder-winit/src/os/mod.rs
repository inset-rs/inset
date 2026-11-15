//! What an operating system adds beyond winit, as one small trait per concern, in the way of Rust's
//! own `sys` module: the traits are in [`capabilities`], [`System`] is the one system this build
//! targets, which the embedder calls through, and this system's types for what the traits leave to
//! each system carry their plain names here (`os::DisplayLink`, `os::Clipboard`).
//!
//! Every method has the answer of a system that adds nothing, so a system implements only what it
//! has. The exceptions are the target platform and the associated types: the display link, which
//! `absent` answers for a system that has none; what a view shows into, one kind for every view of
//! a system, a composition layer of the view's own on macOS and the view's window elsewhere
//! (`window_handle_layer`, with the window's composition layers `absent` answers for a system with no
//! compositor); and the clipboard, which is arboard's plain text on a system without code of its
//! own for one. What a system lets the app do is answered in one place, on [`OperatingSystem`].
//! Four systems: macOS adds a window frame that animates, its menus (the menu bar, a menu at a
//! point, the Dock icon's and the status items), its pasteboard, a display link, composition
//! layers, a view drawn into a composition layer of its own, its own answers about
//! its screens, whether a window is occluded and its notices that a window was minimized, the
//! fingers coming down on a trackpad, and the requests it makes of the application and what it says
//! changed, among others (`macos`); Windows tells the clipboard's changes by its
//! sequence number, and has a menu at a point, icons in the notification area, toasts, a display
//! link and DirectComposition's composition layers, a view drawn on a visual of its own
//! (`windows`); Linux has the desktop's tray and its notification server (`linux`); every other
//! system winit covers by itself (`winit_only`). What macOS shares with the iOS host, its
//! notification center and the compositor of composition layers among it, is in
//! `waft-embedder-apple`.
//!
//! A system's module is compiled for its own target only, so a change to a trait is checked
//! against that system's answers only when building for it: build every target a change
//! touches.

#[cfg(not(target_os = "macos"))]
mod absent;
#[cfg(target_os = "linux")]
pub use linux::DmabufBuffer;
#[cfg(target_os = "macos")]
pub(crate) use waft_embedder_apple::composition::{place_layer_in, show_system_layer};
#[cfg(target_os = "macos")]
pub use waft_embedder_apple::hardware_buffers::SurfaceBuffer;
#[cfg(target_os = "macos")]
pub use waft_embedder_apple::media::CoreVideoFrame;
#[cfg(target_os = "windows")]
pub use windows::SharedTexture;
pub(crate) mod capabilities;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_os = "macos"))]
mod window_handle_layer;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
mod winit_only;

use capabilities::{
    FramePresentation, NativeComposition, OperatingSystem, SystemClipboard, TrackpadTouches,
    WindowVisibility,
};

/// The system this build targets.
#[cfg(target_os = "macos")]
pub(crate) type System = macos::AppKit;
#[cfg(target_os = "windows")]
pub(crate) type System = windows::Windows;
#[cfg(target_os = "linux")]
pub(crate) type System = linux::Linux;
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub(crate) type System = winit_only::WinitOnly;

/// This system's vsyncs for a display.
pub(crate) type DisplayLink = <System as FramePresentation>::DisplayLink;

/// What a view shows into on this system.
pub(crate) type ViewLayer = <System as NativeComposition>::ViewLayer;

/// This system's window's hold on the layer its view shows into.
pub(crate) type WindowHold = <System as NativeComposition>::WindowHold;

/// This system's notices that a window may have been minimized or occluded, for one window.
pub(crate) type VisibilityNotices = <System as WindowVisibility>::Notices;

/// This system's report of the fingers coming down on a trackpad, for one window.
pub(crate) type TrackpadTouchObserver = <System as TrackpadTouches>::Observer;

/// This system's clipboard.
pub(crate) type Clipboard = <System as SystemClipboard>::Clipboard;

/// The build fails here, rather than at some call site, when this system leaves a concern out.
const _: fn() = answers_everything::<System>;

fn answers_everything<T: OperatingSystem>() {}
