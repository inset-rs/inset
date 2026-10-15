//! When the framework's frames are drawn: at the vsync after the framework asks for one, as
//! Flutter's `Animator` asks its `VsyncWaiter` for the next vsync when a frame is requested and
//! draws at it. Asked for again before that vsync, the frame is still drawn once.
//!
//! The vsyncs are those of the display the first window is on, the first opened of the windows
//! still open, the screen view's while it is, asked for anew with each frame so that they
//! follow that window to another display. A display's vsyncs do not stop for its windows: hiding
//! a window stops its presents, not the vsyncs every window's frames wait on
//! (`2026-09-29-view-display-link-unseen` in the `experiments` folder beside this repository).
//! With no window there is no vsync, and the
//! frame is drawn at once.
//!
//! A resize draws a frame inline, so that the picture and the geometry it was laid out for reach
//! the window together; that frame stands for the vsync's, and the vsync after it draws none.
//!
//! The rule a later editor could break: a frame asked for is drawn once and no more than once
//! between two vsyncs, so every state that owes a frame has asked the vsync source for the vsync
//! it is drawn at.

use std::time::Instant;

use winit::monitor::MonitorHandle;

use crate::vsync_source::VsyncSource;

/// When the framework's frame is drawn; owned by the loop.
pub(crate) struct FrameScheduler {
    state: FrameState,
    vsync: VsyncSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameState {
    /// No frame asked for.
    Idle,
    /// A frame is asked for, to be drawn at the next vsync.
    WaitingForVsync,
    /// A resize drew this vsync's frame already, so the next vsync draws none; a frame asked
    /// for since is drawn at the vsync after it.
    DrawnForResize { frame_asked: bool },
}

/// When a frame the framework asked for is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NextFrame {
    /// Now, by the caller.
    DrawNow,
    /// At the next vsync, which the scheduler has asked for.
    AtNextVsync,
}

impl FrameScheduler {
    /// A scheduler whose vsyncs reach the loop through `send_vsync`, from wherever the display
    /// link calls it.
    pub(crate) fn new(send_vsync: impl Fn() + 'static) -> FrameScheduler {
        FrameScheduler {
            state: FrameState::Idle,
            vsync: VsyncSource::new(send_vsync),
        }
    }

    /// The framework asked for a frame. `display` is the first window's; with no window there
    /// is no vsync, and the frame is drawn now.
    pub(crate) fn request_frame(&mut self, display: Option<&MonitorHandle>) -> NextFrame {
        let Some(display) = display else {
            self.state = FrameState::Idle;
            return NextFrame::DrawNow;
        };
        self.vsync.request_vsync(display);
        self.frame_asked();
        NextFrame::AtNextVsync
    }

    /// A vsync came: answers whether the frame asked for is drawn now. One asked for since a
    /// resize waits for the vsync after, which this asks for.
    pub(crate) fn on_vsync(&mut self) -> bool {
        self.vsync.vsync_came();
        match self.state {
            FrameState::Idle | FrameState::DrawnForResize { frame_asked: false } => {
                self.state = FrameState::Idle;
                false
            }
            FrameState::WaitingForVsync => {
                self.state = FrameState::Idle;
                true
            }
            FrameState::DrawnForResize { frame_asked: true } => {
                self.state = FrameState::WaitingForVsync;
                self.vsync.request_next_vsync();
                false
            }
        }
    }

    /// A resize drew a frame between vsyncs: it stands for the frame asked for, and the next
    /// vsync draws none.
    pub(crate) fn drew_frame_for_resize(&mut self) {
        self.state = FrameState::DrawnForResize { frame_asked: false };
        self.vsync.request_next_vsync();
    }

    /// Asks for a vsync that draws no frame, of `display`, the first window's: for a scene a view
    /// owes its surface, which it draws again then.
    pub(crate) fn request_vsync(&mut self, display: &MonitorHandle) {
        self.vsync.request_vsync(display);
    }

    /// The first window may be on another display now, `display`: a vsync asked for comes from
    /// it.
    pub(crate) fn follow_display(&mut self, display: &MonitorHandle) {
        self.vsync.follow_display(display);
    }

    /// When the timer standing in for a display link gives its next vsync, for the loop to wait
    /// until; `None` on a display link, and while no vsync is asked for.
    pub(crate) fn vsync_timer_due(&self) -> Option<Instant> {
        self.vsync.timer_due()
    }

    /// A frame is asked for, and the vsync it waits for is: it is drawn at that vsync, or after a
    /// resize at the one after.
    fn frame_asked(&mut self) {
        self.state = match self.state {
            FrameState::Idle | FrameState::WaitingForVsync => FrameState::WaitingForVsync,
            FrameState::DrawnForResize { .. } => FrameState::DrawnForResize { frame_asked: true },
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scheduler as the loop has it before any window's display is known; a frame asked for
    /// with a window is `frame_asked`, the vsync it asks for having been asked.
    fn scheduler() -> FrameScheduler {
        FrameScheduler::new(|| {})
    }

    #[test]
    fn a_frame_asked_for_is_drawn_once_at_the_next_vsync() {
        let mut frames = scheduler();
        frames.frame_asked();
        assert!(frames.on_vsync());
        assert!(!frames.on_vsync(), "drawn once");
    }

    #[test]
    fn a_frame_asked_for_twice_before_the_vsync_is_drawn_once() {
        let mut frames = scheduler();
        frames.frame_asked();
        frames.frame_asked();
        assert!(frames.on_vsync());
        assert!(!frames.on_vsync());
    }

    #[test]
    fn the_vsync_after_a_resize_draws_no_frame() {
        let mut frames = scheduler();
        frames.frame_asked();
        frames.drew_frame_for_resize();
        assert!(!frames.on_vsync(), "the resize drew it");
        assert!(!frames.on_vsync());
    }

    #[test]
    fn a_frame_asked_for_after_a_resize_is_drawn_at_the_vsync_after_the_next() {
        let mut frames = scheduler();
        frames.drew_frame_for_resize();
        frames.frame_asked();
        assert!(!frames.on_vsync(), "the vsync after the resize");
        assert!(frames.on_vsync());
        assert!(!frames.on_vsync());
    }

    #[test]
    fn with_no_window_a_frame_is_drawn_now_and_not_again_at_a_vsync() {
        let mut frames = scheduler();
        frames.frame_asked();
        assert_eq!(frames.request_frame(None), NextFrame::DrawNow);
        assert!(!frames.on_vsync());
    }
}
