//! The main loop's frames and wakes: when the app is woken, when its frame is drawn, and the
//! scenes a surface still owes.

use std::time::Instant;

use inset_embedder::{EmbedderClient, Frame};
use winit::monitor::MonitorHandle;

use crate::frame_scheduler::NextFrame;
use crate::view::WinitView;
use crate::vsync_source::display_of;

use super::MainLoop;

impl<C: EmbedderClient> MainLoop<C> {
    /// The framework asks to be woken at its timer's `wakeup`, or no longer at all.
    pub(super) fn on_wake_at(&mut self, wakeup: Option<Instant>) {
        self.wakeups.set_timer_wakeup(wakeup);
    }

    /// A ready task asks for the framework to be woken, which the loop does before it waits.
    pub(super) fn on_wake_now(&mut self) {
        self.wakeups.wake_now();
    }

    /// The loop's wait ended at `at`, the time it waited for: the vsync of the timer standing in
    /// for a display link, a wake the framework asked for, or both.
    pub(super) fn on_wait_ended(&mut self, at: Instant) {
        if self.vsync_timer_fired(at) {
            self.on_vsync();
        }
        self.wake_if_due(at);
    }

    /// Wakes the framework if a wake it asked for is due by `now`.
    pub(super) fn wake_if_due(&mut self, now: Instant) {
        if self.wakeups.take_due(now) {
            self.wake_client();
        }
    }

    /// A frame is asked for, by the framework or for surfaces just given: drawn at the next vsync
    /// of the first window's display, or now with no window.
    pub(super) fn request_frame(&mut self) {
        let display = self.first_window_display();
        if self.frame_scheduler.request_frame(display.as_ref()) == NextFrame::DrawNow {
            self.draw_frame();
        }
    }

    /// A vsync: the frame asked for, unless a resize drew this vsync's already, and the scenes
    /// surfaces timed out on.
    pub(super) fn on_vsync(&mut self) {
        if self.frame_scheduler.on_vsync() {
            self.draw_frame();
        }
        self.retry_owed_scenes();
    }

    /// Whether the timer standing in for a display link gave its vsync by `at`.
    pub(super) fn vsync_timer_fired(&self, at: Instant) -> bool {
        self.frame_scheduler
            .vsync_timer_due()
            .is_some_and(|due| due <= at)
    }

    /// The frame a resize draws before it returns, presented through the window's own
    /// transaction.
    pub(super) fn draw_resize_frame(&mut self, view: &WinitView) {
        let _transaction = view.present_in_transaction();
        self.draw_frame();
        self.frame_scheduler.drew_frame_for_resize();
    }

    /// Draws one frame: the framework builds, lays out and paints, and every view presents; a
    /// scene a surface timed out on is drawn again at the next vsync.
    pub(super) fn draw_frame(&mut self) {
        self.run_client_frame();
        self.request_vsync_for_owed_scenes();
    }

    pub(super) fn run_client_frame(&mut self) {
        self.client.frame(Frame {
            elapsed: self.platform.elapsed(),
        });
    }

    /// Draws again the scenes their surfaces timed out on, and asks for the next vsync while a
    /// view still owes one.
    pub(super) fn retry_owed_scenes(&mut self) {
        for view in self.views().views() {
            if view.retries_at_vsync() {
                view.retry_owed_scene();
            }
        }
        self.request_vsync_for_owed_scenes();
    }

    /// Asks for a vsync of the first window's display while a view owes its surface a scene it
    /// draws again then.
    pub(super) fn request_vsync_for_owed_scenes(&mut self) {
        let owed = self
            .views()
            .views()
            .iter()
            .any(|view| view.retries_at_vsync());
        if owed && let Some(display) = self.first_window_display() {
            self.frame_scheduler.request_vsync(&display);
        }
    }

    /// The display whose vsyncs the frames take: the first window's; `None` with no window.
    pub(super) fn first_window_display(&self) -> Option<MonitorHandle> {
        let first = self.views().first_window()?;
        display_of(&first.window)
    }
}
