//! When each view's frames are drawn: at a vsync of the screen its window is on, after the
//! framework asks for a frame of it, as Flutter's `Animator` asks its `VsyncWaiter` for the next
//! vsync when a frame is requested and draws at it. Every view on a screen shares one vsync
//! source, and a vsync draws one refresh of the views on that screen that are due.
//!
//! A screen's vsyncs do not stop for its windows
//! (`2026-09-29-view-display-link-unseen` in the `experiments` folder beside this repository), so a
//! view whose window is minimized or occluded is never due: a frame asked of it is held, and drawn
//! at the first vsync once the window is in sight again, as a browser pauses
//! `requestAnimationFrame` in a hidden tab. A frame asked of a view with no window yet is held the
//! same way.
//!
//! A resize draws its view's frame inline, so that the picture and the geometry it was laid out
//! for reach the window together; that frame stands for the vsync's, and the view's next vsync
//! draws none.
//!
//! The rule a later editor could break: a frame asked of a view in sight is drawn once and no more
//! than once between two vsyncs of its screen, so every view that owes a frame has asked its
//! screen's source for the vsync it is drawn at. The scheduler learns a window's screen and
//! whether it is in sight only from the window's events ([`FrameScheduler::follow_window`]).

use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Instant;

use inset_embedder::{ScreenId, ViewId};
use winit::monitor::MonitorHandle;

use crate::os::{Screens, ThisOs};
use crate::vsync_source::VsyncSource;

/// When each view's frame is drawn; owned by the loop.
pub(crate) struct FrameScheduler {
    /// Each view's frames, in the order the views came.
    views: BTreeMap<ViewId, ViewFrames>,
    /// The vsyncs of each screen a view's window is on.
    screens: Vec<ScreenVsyncs>,
    /// What a screen's vsync sends the loop.
    send_vsync: Rc<dyn Fn(ScreenId)>,
}

/// One screen's vsyncs, shared by every view whose window is on it.
struct ScreenVsyncs {
    screen: ScreenId,
    vsync: VsyncSource,
}

/// One view's frames: whether one is asked for, and whether its window takes vsyncs.
#[derive(Debug)]
struct ViewFrames {
    state: FrameState,
    /// The screen its window is on; `None` while it has no window.
    screen: Option<ScreenId>,
    /// Whether its window is in sight: neither minimized nor occluded.
    in_sight: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameState {
    /// No frame asked for.
    Idle,
    /// A frame is asked for, to be drawn at the next vsync once the window is in sight.
    WaitingForVsync,
    /// A resize drew this vsync's frame already, so the next vsync draws none; a frame asked
    /// for since is drawn at the vsync after it.
    DrawnForResize { frame_asked: bool },
}

impl FrameScheduler {
    /// A scheduler whose screens' vsyncs reach the loop through `send_vsync`, from wherever their
    /// display links call it.
    pub(crate) fn new(send_vsync: impl Fn(ScreenId) + 'static) -> FrameScheduler {
        FrameScheduler {
            views: BTreeMap::new(),
            screens: Vec::new(),
            send_vsync: Rc::new(send_vsync),
        }
    }

    /// The framework asked for a frame of `view`: drawn at the next vsync of its window's screen
    /// while the window is in sight, and held until then.
    pub(crate) fn request_frame(&mut self, view: ViewId) {
        self.view_frames(view).frame_asked();
        self.request_vsync_if_waiting(view);
    }

    /// The window of `view` is on `monitor` now, `None` for a view without a window, and is in
    /// sight or not: a frame held for it is drawn at the next vsync once it is in sight.
    pub(crate) fn follow_window(
        &mut self,
        view: ViewId,
        monitor: Option<&MonitorHandle>,
        in_sight: bool,
    ) {
        let screen = monitor.map(|monitor| self.follow_screen(monitor));
        self.view_frames(view).follow_window(screen, in_sight);
        self.request_vsync_if_waiting(view);
        self.drop_screens_without_views();
    }

    /// `view` is gone: no frame is drawn for it again.
    pub(crate) fn remove_view(&mut self, view: ViewId) {
        self.views.remove(&view);
        self.drop_screens_without_views();
    }

    /// A vsync of `screen` came: answers the views on it whose frames are drawn now, in the order
    /// they came. A view whose resize drew this vsync's frame waits for the next.
    pub(crate) fn on_vsync(&mut self, screen: ScreenId) -> Vec<ViewId> {
        let Some(vsyncs) = self.screen_vsyncs(screen) else {
            return Vec::new();
        };
        vsyncs.vsync.vsync_came();
        let mut due = Vec::new();
        for (&view, frames) in &mut self.views {
            if frames.screen == Some(screen) && frames.vsync_came() {
                due.push(view);
            }
        }
        for view in self.views_on(screen) {
            self.request_vsync_if_waiting(view);
        }
        due
    }

    /// A resize drew a frame of `view` between vsyncs: it stands for the frame asked for, and the
    /// view's next vsync draws none.
    pub(crate) fn drew_frame_for_resize(&mut self, view: ViewId) {
        self.view_frames(view).drawn_for_resize();
        self.request_vsync(view);
    }

    /// Asks for a vsync of the screen `view`'s window is on that draws no frame: for a scene the
    /// view owes its surface, which it draws again then.
    pub(crate) fn request_vsync(&mut self, view: ViewId) {
        let Some(screen) = self.views.get(&view).and_then(|frames| frames.screen) else {
            return;
        };
        if let Some(vsyncs) = self.screen_vsyncs(screen) {
            vsyncs.vsync.request_vsync();
        }
    }

    /// The views whose windows are on `screen`, in the order they came.
    pub(crate) fn views_on(&self, screen: ScreenId) -> Vec<ViewId> {
        self.views
            .iter()
            .filter(|(_, frames)| frames.screen == Some(screen))
            .map(|(&view, _)| view)
            .collect()
    }

    /// When the first of the timers standing in for display links gives its next vsync, for the
    /// loop to wait until; `None` on display links, and while no vsync is asked for.
    pub(crate) fn vsync_timer_due(&self) -> Option<Instant> {
        self.screens
            .iter()
            .filter_map(|vsyncs| vsyncs.vsync.timer_due())
            .min()
    }

    /// The screens whose timers, standing in for display links, gave their vsync by `at`.
    pub(crate) fn timer_vsyncs_by(&self, at: Instant) -> Vec<ScreenId> {
        self.screens
            .iter()
            .filter(|vsyncs| vsyncs.vsync.timer_due().is_some_and(|due| due <= at))
            .map(|vsyncs| vsyncs.screen)
            .collect()
    }

    /// The frames of `view`, as a view with none asked for and no window when it is new.
    fn view_frames(&mut self, view: ViewId) -> &mut ViewFrames {
        self.views.entry(view).or_insert_with(ViewFrames::new)
    }

    /// Asks `view`'s screen for its next vsync if the view waits for one to be drawn at.
    fn request_vsync_if_waiting(&mut self, view: ViewId) {
        if self
            .views
            .get(&view)
            .is_some_and(ViewFrames::waits_for_vsync)
        {
            self.request_vsync(view);
        }
    }

    /// The screen `monitor` is, whose vsyncs are followed from now on if they were not.
    fn follow_screen(&mut self, monitor: &MonitorHandle) -> ScreenId {
        let screen = ThisOs::screen_id(monitor);
        if self.screen_vsyncs(screen).is_none() {
            let send_vsync = Rc::clone(&self.send_vsync);
            self.screens.push(ScreenVsyncs {
                screen,
                vsync: VsyncSource::on(monitor, move || send_vsync(screen)),
            });
        }
        screen
    }

    fn screen_vsyncs(&mut self, screen: ScreenId) -> Option<&mut ScreenVsyncs> {
        self.screens
            .iter_mut()
            .find(|vsyncs| vsyncs.screen == screen)
    }

    /// Lets go of the vsyncs of every screen no view's window is on.
    fn drop_screens_without_views(&mut self) {
        let views = &self.views;
        self.screens.retain(|vsyncs| {
            views
                .values()
                .any(|frames| frames.screen == Some(vsyncs.screen))
        });
    }
}

impl ViewFrames {
    /// A view with no frame asked for, and no window yet.
    fn new() -> ViewFrames {
        ViewFrames {
            state: FrameState::Idle,
            screen: None,
            in_sight: false,
        }
    }

    /// The view's window is on `screen` now, `None` without a window, and in sight or not.
    fn follow_window(&mut self, screen: Option<ScreenId>, in_sight: bool) {
        self.screen = screen;
        self.in_sight = in_sight;
    }

    /// A frame is asked for: it is drawn at the next vsync once the window is in sight, or after a
    /// resize at the one after.
    fn frame_asked(&mut self) {
        self.state = match self.state {
            FrameState::Idle | FrameState::WaitingForVsync => FrameState::WaitingForVsync,
            FrameState::DrawnForResize { .. } => FrameState::DrawnForResize { frame_asked: true },
        };
    }

    /// A resize drew a frame between vsyncs: the next vsync draws none.
    fn drawn_for_resize(&mut self) {
        self.state = FrameState::DrawnForResize { frame_asked: false };
    }

    /// Whether the view waits for its screen's next vsync to be drawn at.
    fn waits_for_vsync(&self) -> bool {
        self.state == FrameState::WaitingForVsync && self.in_sight && self.screen.is_some()
    }

    /// A vsync of the view's screen came: answers whether its frame is drawn now. A frame held
    /// for a window out of sight stays held; one asked for since a resize waits for the vsync
    /// after.
    fn vsync_came(&mut self) -> bool {
        match self.state {
            FrameState::Idle | FrameState::DrawnForResize { frame_asked: false } => {
                self.state = FrameState::Idle;
                false
            }
            FrameState::WaitingForVsync => {
                if !self.in_sight {
                    return false;
                }
                self.state = FrameState::Idle;
                true
            }
            FrameState::DrawnForResize { frame_asked: true } => {
                self.state = FrameState::WaitingForVsync;
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: ScreenId = ScreenId(1);

    /// A view whose window is in sight on a screen.
    fn shown() -> ViewFrames {
        let mut frames = ViewFrames::new();
        frames.follow_window(Some(SCREEN), true);
        frames
    }

    #[test]
    fn a_frame_asked_for_is_drawn_once_at_the_next_vsync() {
        let mut frames = shown();
        frames.frame_asked();
        assert!(frames.waits_for_vsync());
        assert!(frames.vsync_came());
        assert!(!frames.vsync_came(), "drawn once");
    }

    #[test]
    fn a_frame_asked_for_twice_before_the_vsync_is_drawn_once() {
        let mut frames = shown();
        frames.frame_asked();
        frames.frame_asked();
        assert!(frames.vsync_came());
        assert!(!frames.vsync_came());
    }

    #[test]
    fn the_vsync_after_a_resize_draws_no_frame() {
        let mut frames = shown();
        frames.frame_asked();
        frames.drawn_for_resize();
        assert!(!frames.vsync_came(), "the resize drew it");
        assert!(!frames.vsync_came());
    }

    #[test]
    fn a_frame_asked_for_after_a_resize_is_drawn_at_the_vsync_after_the_next() {
        let mut frames = shown();
        frames.drawn_for_resize();
        frames.frame_asked();
        assert!(!frames.vsync_came(), "the vsync after the resize");
        assert!(frames.waits_for_vsync(), "it asks for the one after");
        assert!(frames.vsync_came());
        assert!(!frames.vsync_came());
    }

    #[test]
    fn a_frame_asked_of_a_window_out_of_sight_is_held_until_it_shows() {
        let mut frames = shown();
        frames.follow_window(Some(SCREEN), false);
        frames.frame_asked();
        assert!(!frames.waits_for_vsync(), "no vsync is asked for it");
        assert!(
            !frames.vsync_came(),
            "a vsync another window asked for draws nothing"
        );

        frames.follow_window(Some(SCREEN), true);
        assert!(frames.waits_for_vsync(), "in sight, it asks for the next");
        assert!(frames.vsync_came());
    }

    #[test]
    fn a_frame_asked_of_a_view_without_a_window_is_held() {
        let mut frames = ViewFrames::new();
        frames.follow_window(None, true);
        frames.frame_asked();
        assert!(!frames.waits_for_vsync());

        frames.follow_window(Some(SCREEN), true);
        assert!(frames.waits_for_vsync());
    }
}
