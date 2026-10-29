//! What a system without a display link, a compositor or notices of a window's visibility
//! answers: no display link is made, no surface is ever placed, and no notice is observed. iOS
//! has a compositor, Core Animation's, which it shares with macOS.

#[cfg(not(target_os = "ios"))]
use std::sync::Arc;

#[cfg(not(target_os = "ios"))]
use inset_embedder::{SurfacePlacement, ViewId};
use winit::monitor::MonitorHandle;
#[cfg(not(target_os = "ios"))]
use winit::window::Window;

use super::DisplayLink;
#[cfg(not(target_os = "ios"))]
use super::WindowSurfaces;

/// No display link: the embedder takes its vsyncs from a timer at the display's rate. Never made.
pub(crate) enum NoDisplayLink {}

impl DisplayLink for NoDisplayLink {
    fn start(_display: &MonitorHandle, _on_vsync: Box<dyn Fn()>) -> Option<NoDisplayLink> {
        None
    }

    fn set_wanted(&self, _wanted: bool) {
        match *self {}
    }
}

/// No compositor: no surface is opened, so a window has none to place.
#[cfg(not(target_os = "ios"))]
pub(crate) struct NoWindowSurfaces;

#[cfg(not(target_os = "ios"))]
impl WindowSurfaces for NoWindowSurfaces {
    fn new(_window: Arc<dyn Window>) -> Self {
        NoWindowSurfaces
    }

    fn place(&self, _placements: &[SurfacePlacement], _view: ViewId) {}
}

/// No notices of a window's visibility beyond winit's events. Never made.
pub(crate) enum NoVisibilityNotices {}
