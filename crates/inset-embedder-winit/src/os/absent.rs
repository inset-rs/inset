//! What a system without a display link or a compositor answers: nothing ticks, and no
//! surface is ever placed.

use std::sync::Arc;

use inset_embedder::{SurfacePlacement, ViewId};
use winit::window::Window;

use super::{DisplayLink, WindowSurfaces};

/// No refresh signal: the host paces frames by a timer at the display's rate. Never made.
pub(crate) enum NoDisplayLink {}

impl DisplayLink for NoDisplayLink {
    fn start(_window: &Window, _on_tick: Box<dyn Fn()>) -> Option<NoDisplayLink> {
        None
    }

    fn set_paused(&self, _paused: bool) {
        match *self {}
    }
}

/// No compositor: no surface is opened, so a window has none to place.
pub(crate) struct NoWindowSurfaces;

impl WindowSurfaces for NoWindowSurfaces {
    fn new(_window: Arc<Window>) -> Self {
        NoWindowSurfaces
    }

    fn place(&self, _placements: &[SurfacePlacement], _view: ViewId) {}
}
