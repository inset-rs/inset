//! How a frame reaches a macOS display: drawn at the vsyncs of the screen's display link, and,
//! for the frame drawn inside a resize, presented through the window's own Core Animation
//! transaction.

use super::AppKit;
use super::display_links::ScreenDisplayLink;
use crate::os::FramePresentation;

impl FramePresentation for AppKit {
    type DisplayLink = ScreenDisplayLink;

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
