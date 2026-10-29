//! How a frame reaches a macOS display: drawn at the vsyncs of the screen's display link, and,
//! for the frame drawn inside a resize, presented through the window's own Core Animation
//! transaction; or, for a frame with no contents of its own, not drawn at all, its surfaces shown
//! over the view's background, which the layer of the window's view shows.

use std::sync::Arc;

use inset_embedder::VsyncWaiter;
use objc2::MainThreadMarker;
use objc2_core_graphics::CGColor;
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

use super::AppKit;
use super::display_links::{ScreenDisplayLink, screens_have_display_links};
use super::vsync_waiter::DisplayLinkVsyncWaiter;
use crate::main_loop::MainLoopEvent;
use crate::os::FramePresentation;
use crate::os::apple::composition::{WindowLayer, in_transaction};
use crate::vsync_waiter::LoopVsyncWaiter;

impl FramePresentation for AppKit {
    type DisplayLink = ScreenDisplayLink;

    /// At the ticks of each screen's display link, in the run loop's common modes, so a wait is
    /// served while a menu tracks the pointer; by the main loop before macOS 14, where a screen
    /// has no display link and the loop's timers stand in for it.
    fn vsync_waiter(proxy: &EventLoopProxy<MainLoopEvent>) -> Arc<dyn VsyncWaiter> {
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

    /// The surfaces are layers beneath the window's content, which Core Animation composites
    /// whether or not anything is drawn above them.
    const SHOWS_FRAMES_UNDRAWN: bool = true;

    /// The background colour of the layer of the window's view, beneath every surface placed in
    /// it.
    fn show_background(window: &Window, color: valo::Color) {
        let Some(layer) = AppKit::window_layer(window) else {
            return;
        };
        let color = CGColor::new_srgb(
            color.r.into(),
            color.g.into(),
            color.b.into(),
            color.a.into(),
        );
        in_transaction(|| layer.setBackgroundColor(Some(&color)));
    }
}
