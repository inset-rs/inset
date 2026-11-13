//! What a system without a display link, a compositor or notices of a window's visibility
//! answers: no display link is made, no composition layer is ever placed in a window, and no
//! notice is observed. iOS has a compositor, Core Animation's, which it shares with macOS, Linux
//! has Wayland's, and Windows has DirectComposition and a display link of its own.

#[cfg(not(any(target_os = "ios", target_os = "linux", target_os = "windows")))]
use std::sync::Arc;

#[cfg(not(any(target_os = "ios", target_os = "linux", target_os = "windows")))]
use waft_embedder::{CompositionLayer, Placement, ViewId};
#[cfg(not(target_os = "windows"))]
use winit::monitor::MonitorHandle;
#[cfg(not(any(target_os = "ios", target_os = "linux", target_os = "windows")))]
use winit::window::Window;

#[cfg(not(target_os = "windows"))]
use super::capabilities::DisplayLink;
#[cfg(not(any(target_os = "ios", target_os = "linux", target_os = "windows")))]
use super::window_handle_layer::WindowComposition;

/// No display link: the embedder takes its vsyncs from a timer at the display's rate. Never made.
#[cfg(not(target_os = "windows"))]
pub(crate) enum NoDisplayLink {}

#[cfg(not(target_os = "windows"))]
impl DisplayLink for NoDisplayLink {
    fn start(_display: &MonitorHandle, _on_vsync: Box<dyn Fn()>) -> Option<NoDisplayLink> {
        None
    }

    fn set_wanted(&self, _wanted: bool) {
        match *self {}
    }

    fn target_time(&self) -> Option<std::time::Instant> {
        match *self {}
    }

    fn vsync_time(&self) -> Option<std::time::Instant> {
        match *self {}
    }
}

/// No compositor: no layer is made, so a window has none to place.
#[cfg(not(any(target_os = "ios", target_os = "linux", target_os = "windows")))]
pub(crate) struct NoWindowComposition;

#[cfg(not(any(target_os = "ios", target_os = "linux", target_os = "windows")))]
impl WindowComposition for NoWindowComposition {
    fn new(_window: Arc<dyn Window>) -> Self {
        NoWindowComposition
    }

    fn place<'a>(
        &self,
        _placements: impl IntoIterator<Item = (&'a Arc<dyn CompositionLayer>, &'a Placement)>,
        _view: ViewId,
    ) {
    }
}

/// No notices of a window's visibility beyond winit's events. Never made.
pub(crate) enum NoVisibilityNotices {}

/// No report of a trackpad's touches. Never made.
pub(crate) enum NoTrackpadTouchObserver {}
