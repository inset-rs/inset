//! The main loop's frames and wakes: when the app is woken, when each screen's refresh is drawn
//! and its waits served, and the scenes a surface still owes.

use std::sync::Weak;
use std::time::Instant;

use inset_embedder::{EmbedderClient, Frame, HostWindow, ScreenId, View, ViewId, VsyncObserver};
use winit::event_loop::ActiveEventLoop;
use winit::monitor::MonitorHandle;

use crate::os::System;
use crate::os::capabilities::{Screens, WindowVisibility};
use crate::view::WinitView;
use crate::vsync_source::monitor_of;
use crate::windows::OpenWindow;

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

    /// The loop's wait ended at `at`, the time it waited for: the vsyncs of the timers standing
    /// in for display links, a wake the framework asked for, or both.
    pub(super) fn on_wait_ended(&mut self, at: Instant) {
        for screen in self.frame_scheduler.timer_vsyncs_by(at) {
            self.on_vsync(screen);
        }
        self.wake_if_due(at);
    }

    /// Wakes the framework if a wake it asked for is due by `now`.
    pub(super) fn wake_if_due(&mut self, now: Instant) {
        if self.wakeups.take_due(now) {
            self.wake_client();
        }
    }

    /// A frame of `view` is asked for, by the framework or for a surface just given: drawn at the
    /// next vsync of its window's screen while the window is in sight, and held until it is.
    /// One asked of a view that is gone is dropped.
    pub(super) fn request_frame(&mut self, view: ViewId) {
        if self.views().get(view).is_some() {
            self.frame_scheduler.request_frame(view);
        }
    }

    /// Someone waits, through the platform's waiter, for the next vsync of `screen`.
    pub(super) fn on_vsync_waited(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        screen: ScreenId,
        observer: Weak<dyn VsyncObserver>,
    ) {
        self.frame_scheduler
            .wait_for_vsync(screen, observer, || monitor_numbered(event_loop, screen));
    }

    /// A vsync of `screen`: the waits on it are served, then one refresh of the views on it whose
    /// frames are due, unless none is, and the scenes their surfaces timed out on.
    pub(super) fn on_vsync(&mut self, screen: ScreenId) {
        let display_time = self.frame_scheduler.display_time(screen);
        self.tell_display_time(screen, display_time);
        let refresh = self.frame_scheduler.on_vsync(screen);
        serve_waits(&refresh.waiting, Instant::now(), display_time);
        if !refresh.views.is_empty() {
            self.draw_frame(&refresh.views);
        }
        self.redraw_without_the_framework(screen);
    }

    /// Tells the views on `screen` that the picture they draw at this vsync reaches the screen at
    /// `display_time`, the moment their videos' frames are asked for.
    fn tell_display_time(&mut self, screen: ScreenId, display_time: Instant) {
        for view_id in self.frame_scheduler.views_on(screen) {
            if let Some(view) = self.views().get(view_id) {
                view.set_display_time(display_time);
            }
        }
    }

    /// The frame of `view` a resize draws before it returns, presented through the window's own
    /// transaction: a refresh of that view alone.
    pub(super) fn draw_resize_frame(&mut self, view: &WinitView) {
        let _transaction = view.present_in_transaction();
        self.draw_frame(&[view.id()]);
        self.frame_scheduler.drew_frame_for_resize(view.id());
    }

    /// Draws a refresh of `views`: the framework builds, lays out, paints and presents them; a
    /// scene a surface timed out on is drawn again at the next vsync.
    fn draw_frame(&mut self, views: &[ViewId]) {
        let frame = Frame {
            elapsed: self.platform.elapsed(),
        };
        self.client.frame(views, frame);
        for &view in views {
            self.request_vsync_if_wanted(view);
        }
    }

    /// Draws again, without the framework, the views on `screen` whose surfaces timed out on
    /// their scene and those in sight whose videos have new frames, and asks for the next vsync
    /// while a view still wants one.
    fn redraw_without_the_framework(&mut self, screen: ScreenId) {
        for view_id in self.frame_scheduler.views_on(screen) {
            let Some(view) = self.views().get(view_id) else {
                continue;
            };
            if view.retries_at_vsync() {
                view.retry_owed_scene();
            }
            if view.shows_video() && self.frame_scheduler.is_in_sight(view_id) {
                view.draw_new_video_frames();
            }
            self.request_vsync_if_wanted(view_id);
        }
    }

    /// Asks for a vsync of the screen `view`'s window is on while the view draws something then
    /// without the framework: a scene its surface timed out on, or, while it is in sight, a
    /// video's next frame.
    pub(super) fn request_vsync_if_wanted(&mut self, view_id: ViewId) {
        let in_sight = self.frame_scheduler.is_in_sight(view_id);
        if self
            .views()
            .get(view_id)
            .is_some_and(|view| view.retries_at_vsync() || (view.shows_video() && in_sight))
        {
            self.frame_scheduler.request_vsync(view_id);
        }
    }

    /// Tells the frame scheduler which screen `view`'s window is on now and whether it is in
    /// sight, at each event that can change either; a view without a window takes no vsyncs, and
    /// one that comes into sight takes its videos' frames again.
    pub(super) fn sync_frame_scheduler(&mut self, view: ViewId) {
        match self.views().by_view(view) {
            Some(open) => {
                let monitor = monitor_of(&*open.window);
                self.frame_scheduler
                    .follow_window(view, monitor.as_ref(), is_in_sight(&open));
            }
            None => self.frame_scheduler.follow_window(view, None, false),
        }
        self.request_vsync_if_wanted(view);
    }
}

/// Calls everyone who waited for a vsync served `at`, whose picture reaches the screen at
/// `display_time`, in the order they asked; one that is gone is not called.
fn serve_waits(waiting: &[Weak<dyn VsyncObserver>], at: Instant, display_time: Instant) {
    for observer in waiting.iter().filter_map(Weak::upgrade) {
        observer.vsync(at, display_time);
    }
}

/// The monitor that is `screen`, if it is connected.
fn monitor_numbered(event_loop: &dyn ActiveEventLoop, screen: ScreenId) -> Option<MonitorHandle> {
    event_loop
        .available_monitors()
        .find(|monitor| System::screen_id(monitor) == screen)
}

/// Whether the window of `open` is in sight: neither minimized nor occluded, as the app's handle
/// on it answers. The screen view's window, which the app has no handle on, as the system answers
/// where it can be asked.
fn is_in_sight(open: &OpenWindow) -> bool {
    match &open.handle {
        Some(handle) => !handle.is_minimized() && !handle.is_occluded(),
        None => System::is_occluded(&*open.window) != Some(true),
    }
}
