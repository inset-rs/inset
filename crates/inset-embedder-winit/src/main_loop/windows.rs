//! The main loop's windows: opening one the app asks for and the screen view's, closing, focus,
//! resizing, moving, being minimized or occluded. A window holds its view (`main_loop/views.rs`):
//! its opening adds the view on the layer it placed for it, a resize or a move to another screen
//! gives the view the window's metrics, its being minimized or occluded has the view out of sight,
//! and its closing removes the view. What is a window's alone, its state as the app's handle on it
//! answers it, stays here.

use std::rc::Rc;
use std::time::Instant;

use inset_embedder::{EmbedderClient, HostWindow, View, ViewError, ViewFocusEvent, ViewMetrics};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::os::System;
use crate::os::capabilities::{FramePresentation, TrackpadTouches, WindowVisibility};
use crate::screens::screen_of;
use crate::view_focus;
use crate::views::SCREEN_VIEW;
use crate::windows::{
    OpenWindow, OpenedWindow, WindowObservers, WindowRequest, open_screen_window, window_metrics,
};

use super::{MainLoop, MainLoopEvent};

impl<C: EmbedderClient> MainLoop<C> {
    /// Opens a window for the view the app asked for. The app hears of the view through
    /// `view_added`, which it tells by its request's id, or of the failure.
    pub(super) fn open_window(&mut self, event_loop: &dyn ActiveEventLoop, request: WindowRequest) {
        let opened = self
            .view_maker
            .make_requested(event_loop, &request, self.state.views());
        match opened {
            Ok(opened) => self.add_requested_window(event_loop, opened),
            Err(reason) => self
                .client
                .view_request_failed(request.request_id, ViewError::Failed(reason)),
        }
    }

    /// A window the app asked for opened: it joins the app's windows, with what the embedder
    /// observes of it, and its view is added on the layer it placed for it, at its metrics, in
    /// sight as it is.
    fn add_requested_window(&mut self, event_loop: &dyn ActiveEventLoop, opened: OpenedWindow) {
        let OpenedWindow {
            open,
            view_layer,
            hold,
        } = opened;
        let observers = self.observe_window(&*open.window);
        self.windows().add(open.clone(), hold, observers);
        let metrics = window_metrics(&*open.window);
        self.add_view(event_loop, Rc::clone(&open.view), view_layer, metrics);
        self.set_view_in_sight(open.view.id(), is_in_sight(&open));
        self.read_theme_of(&*open.window);
    }

    /// What the embedder observes of `window` beyond winit's events, for as long as it is open.
    fn observe_window(&self, window: &dyn Window) -> WindowObservers {
        WindowObservers {
            visibility: System::observe_visibility(
                window,
                self.send_visibility_notice(window.id()),
            ),
            trackpad_touches: System::observe_trackpad_touches(
                window,
                self.send_trackpad_touch(window.id()),
            ),
        }
    }

    /// What fingers coming down on a trackpad over the window `id` do: the moment queues for the
    /// loop.
    fn send_trackpad_touch(&self, id: WindowId) -> Box<dyn Fn(Instant)> {
        let proxy = self.proxy.clone();
        Box::new(move |at| proxy.send(MainLoopEvent::TrackpadTouched(id, at)))
    }

    /// What a notice that the window `id` may have been minimized or occluded does: it queues for
    /// the loop, which reads the window's state again on a turn of its own.
    fn send_visibility_notice(&self, id: WindowId) -> Box<dyn Fn()> {
        let proxy = self.proxy.clone();
        Box::new(move || proxy.send(MainLoopEvent::WindowVisibility(id)))
    }

    /// Opens the screen view's window, at winit's first call: the first moment winit lets a
    /// window be made. The screen view draws into the layer the window placed for it, and the
    /// app, started with the view at no size, hears its new metrics.
    pub(super) fn open_screen_window(&mut self, event_loop: &dyn ActiveEventLoop) {
        let Some(screen) = self.screen_window.take() else {
            return;
        };
        let OpenedWindow {
            open,
            view_layer,
            hold,
        } = open_screen_window(event_loop, screen).expect("open the screen view's window");
        self.views().place_screen_view(view_layer);
        self.windows()
            .add(open.clone(), hold, WindowObservers::default());
        self.set_view_metrics(event_loop, SCREEN_VIEW, window_metrics(&*open.window));
        self.set_view_in_sight(SCREEN_VIEW, is_in_sight(&open));
        self.read_theme_of(&*open.window);
    }

    /// Closes a window: it is freed, and its view removed.
    pub(super) fn close_window(&mut self, id: WindowId) {
        if let Some(view) = self.windows().close(id) {
            self.remove_view(view);
        }
    }

    /// The user asked to close a window: the app decides, and closes it or not.
    pub(super) fn on_close_requested(&mut self, id: WindowId) {
        if let Some(view) = self.view_id_of(id) {
            self.client.view_close_requested(view);
        }
    }

    /// A window gained or lost the focus, which may come with its being minimized or restored;
    /// one that gains it has the clipboard looked at.
    pub(super) fn on_focused(&mut self, id: WindowId, focused: bool) {
        if let Some(view) = self.view_id_of(id) {
            self.client
                .view_focus_changed(view_focus::focus_changed(view, focused));
        }
        self.sync_window_state(id);
        if focused {
            self.look_at_clipboard();
        }
    }

    /// Honours the framework's request to focus a view, or to give up its focus.
    pub(super) fn change_view_focus(&mut self, change: ViewFocusEvent) {
        if let Some(open) = self.windows().by_view(change.view_id) {
            view_focus::focus_window_if_asked(&change, &*open.window);
        }
    }

    /// A window was resized, which may come with its being minimized or restored: its view takes
    /// the new size.
    pub(super) fn on_resized(&mut self, event_loop: &dyn ActiveEventLoop, id: WindowId) {
        self.resize_view(event_loop, id);
        self.sync_window_state(id);
    }

    /// The view of window `id` takes the window's new metrics, and the app hears of the system's
    /// theme, which some systems report as a resize; where the system needs it, the resized layout
    /// is drawn at once. The layer the view draws into was placed again as the window's view
    /// changed (`os`'s `WindowHold`). winit reports a resize for more than one of the system's
    /// notices: the same metrics again draw nothing.
    fn resize_view(&mut self, event_loop: &dyn ActiveEventLoop, id: WindowId) {
        let Some(open) = self.windows().by_window(id) else {
            return;
        };
        self.read_theme_of(&*open.window);
        let metrics = window_metrics(&*open.window);
        if !self.set_view_metrics(event_loop, open.view.id(), metrics) {
            return;
        }
        // Present the resized layout before the system commits the window geometry, rather
        // than stretching the old frame.
        if System::FRAME_ON_RESIZE && has_area(open.view.metrics().physical_size) {
            self.draw_resize_frame(&open.view);
        }
    }

    /// Reads again which screen each window is on: the view of each window on another screen
    /// takes it.
    pub(super) fn sync_screens(&mut self, event_loop: &dyn ActiveEventLoop) {
        for open in self.windows().open_windows() {
            self.set_view_metrics(event_loop, open.view.id(), metrics_on_screen_of(&open));
        }
    }

    /// Reads every window's metrics again, as the system gives the views surfaces, for a system
    /// that reports no event for a change of the safe area: the view of each window whose metrics
    /// moved takes them.
    pub(super) fn sync_metrics(&mut self, event_loop: &dyn ActiveEventLoop) {
        for open in self.windows().open_windows() {
            self.set_view_metrics(event_loop, open.view.id(), window_metrics(&*open.window));
        }
    }

    /// Only the system asks winit for redraws; the framework's frames come at the display's
    /// vsyncs. A window seen again, which is the system's word that its surface
    /// takes frames now, or one the system wants repainted, gets its latest picture
    /// presented again if it is not on the surface; otherwise the content is current and
    /// nothing is drawn.
    pub(super) fn on_redraw_requested(&mut self, id: WindowId) {
        let Some(open) = self.windows().by_window(id) else {
            return;
        };
        open.view.draw_last_scene();
        self.request_vsync_if_wanted(open.view.id());
    }

    /// A window moved, and may be on another screen now: its view takes that screen, as Flutter's
    /// macOS embedder sends a window's metrics again when its screen changes, and the app hears of
    /// the move. AppKit moves the windows off a screen that goes away, so this is also how their
    /// views' vsyncs leave it.
    pub(super) fn on_moved(&mut self, event_loop: &dyn ActiveEventLoop, id: WindowId) {
        let Some(open) = self.windows().by_window(id) else {
            return;
        };
        self.set_view_metrics(event_loop, open.view.id(), metrics_on_screen_of(&open));
        self.report_window_changed(&open);
    }

    /// winit says a window is occluded, or shown: the app hears of it once the window answers so,
    /// and a window seen again is when a picture its surface refused while occluded is presented
    /// (wgpu issue 8309, `rasterizer.rs`).
    pub(super) fn on_occluded(&mut self, id: WindowId, occluded: bool) {
        if let Some(handle) = self.windows().by_window(id).and_then(|open| open.handle) {
            handle.set_occluded(occluded);
        }
        self.sync_window_state(id);
        if !occluded {
            self.on_redraw_requested(id);
        }
    }

    /// Reads again whether window `id` is minimized and whether it is occluded: its view is out of
    /// sight while it is either, and the app hears if either changed since it last heard.
    pub(super) fn sync_window_state(&mut self, id: WindowId) {
        let Some(open) = self.windows().by_window(id) else {
            return;
        };
        self.set_view_in_sight(open.view.id(), is_in_sight(&open));
        let Some(handle) = &open.handle else {
            return;
        };
        if handle.refresh_state() {
            self.client.window_changed(open.view.id());
        }
    }

    /// Tells the app that the window of `open` changed, once it answers its state as it is now;
    /// nothing for the screen view's window, which the app may not manage.
    fn report_window_changed(&mut self, open: &OpenWindow) {
        if let Some(handle) = &open.handle {
            handle.refresh_state();
            self.client.window_changed(open.view.id());
        }
    }
}

fn has_area([width, height]: [f64; 2]) -> bool {
    width > 0.0 && height > 0.0
}

/// The metrics of the view of `open` on the screen its window is on now: a window that moved to
/// another screen has new metrics, though its size stays as it is.
fn metrics_on_screen_of(open: &OpenWindow) -> ViewMetrics {
    ViewMetrics {
        screen: screen_of(&*open.window),
        ..open.view.metrics()
    }
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
