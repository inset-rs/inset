//! How a frame reaches a macOS display: drawn at the vsyncs of the screen's display link, and, for
//! the frame drawn inside a resize, presented through the window's own Core Animation transaction,
//! the one the view's layer was placed again in as the window's view changed (`composition`). A
//! frame that paints nothing besides its composition layers is not drawn at all: the view's layer
//! shows them over its background.

use std::sync::Arc;

use objc2::MainThreadMarker;
use waft_embedder::VsyncWaiter;

use super::AppKit;
use super::display_links::{ScreenDisplayLink, screens_have_display_links};
use super::vsync_waiter::DisplayLinkVsyncWaiter;
use crate::main_loop::MainLoopProxy;
use crate::os::capabilities::FramePresentation;
use crate::vsync_waiter::LoopVsyncWaiter;

impl FramePresentation for AppKit {
    type DisplayLink = ScreenDisplayLink;

    /// At the ticks of each screen's display link, in the run loop's common modes, so a wait is
    /// served while a menu tracks the pointer; by the main loop before macOS 14, where a screen
    /// has no display link and the loop's timers stand in for it.
    fn vsync_waiter(proxy: &MainLoopProxy) -> Arc<dyn VsyncWaiter> {
        match MainThreadMarker::new() {
            Some(main_thread) if screens_have_display_links() => {
                Arc::new(DisplayLinkVsyncWaiter::new(main_thread))
            }
            _ => Arc::new(LoopVsyncWaiter(proxy.clone())),
        }
    }

    /// AppKit commits window geometry before winit's next redraw, so a resize presents the
    /// new layout before it returns, through the window's own transaction so the frame and
    /// the geometry it was laid out for are committed together.
    const FRAME_ON_RESIZE: bool = true;

    /// On for the frame drawn inside a resize, as gpui does for its synchronous draws; off
    /// otherwise, since it makes every present wait to be scheduled.
    fn present_in_transaction(surface: &mut valo::Surface, wanted: bool) {
        surface.set_presents_with_transaction(wanted);
    }
}
