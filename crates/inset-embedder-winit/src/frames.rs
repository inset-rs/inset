//! When the framework's frames are drawn.
//!
//! A frame the framework asks for lands at the display's refresh, through `pacing`; a
//! resize is the exception and draws inline, so that the picture and the geometry it was
//! laid out for reach the window together. A picture a view still owes its surface is
//! tried again at those refreshes only once the surface has timed out on it — an occluded
//! window waits for the system's word instead, which is `view.rs`.

use std::sync::Arc;

use inset_embedder::{EmbedderClient, Frame};

use crate::window::{HostEvent, WinitApp};

impl<C: EmbedderClient> WinitApp<C> {
    /// Sees that a frame the framework asked for is drawn at the display's next refresh
    /// and no sooner. With no window there is no display to pace by, and the frame is
    /// drawn at once.
    pub(crate) fn pace(&mut self) {
        if !self.platform.frame_requested.get() && !self.retries_at_refresh() {
            return;
        }
        let source = self
            .frame_source
            .and_then(|id| self.views.get(&id))
            .map(|hosted| Arc::clone(&hosted.window));
        let Some(window) = source else {
            self.platform.frame_requested.set(false);
            self.draw_frame();
            return;
        };
        let proxy = self.platform.proxy.clone();
        self.pacing.keep_ticking(&window, move || {
            let _ = proxy.send_event(HostEvent::Vsync);
        });
    }

    /// A refresh of the display: draws the frame the framework asked for, unless one was
    /// drawn in this interval already, and tries again any picture a surface timed out on.
    pub(crate) fn tick(&mut self) {
        let requested = self.platform.frame_requested.get();
        if self.pacing.tick(requested || self.retries_at_refresh()) && requested {
            self.platform.frame_requested.set(false);
            self.draw_frame();
        }
        for hosted in self.views.values() {
            if hosted.view.retries_at_refresh() {
                hosted.view.retry();
            }
        }
    }

    /// Whether a view has a picture to try again at the next refresh.
    fn retries_at_refresh(&self) -> bool {
        self.views
            .values()
            .any(|hosted| hosted.view.retries_at_refresh())
    }

    /// Draws one frame: the framework builds, lays out and paints, and every view
    /// presents.
    pub(crate) fn draw_frame(&mut self) {
        let Some(client) = &mut self.client else {
            return;
        };
        client.frame(Frame {
            elapsed: self.platform.elapsed(),
        });
    }
}
