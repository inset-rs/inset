//! The main loop's windows and views: opening one the app asks for and the screen view's,
//! closing, focus, resizing, moving, being minimized or occluded, and the surfaces the system
//! gives and takes.

use std::rc::Rc;
use std::sync::Arc;

use inset_embedder::{EmbedderClient, View, ViewError, ViewFocusEvent, ViewId};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::os::{FramePresentation, ThisOs, WindowVisibility};
use crate::view::WinitView;
use crate::view_focus;
use crate::windows::{OpenWindow, ViewAfterClose, WindowRequest, open_screen_window};

use super::{MainLoop, MainLoopEvent};

impl<C: EmbedderClient> MainLoop<C> {
    /// Opens a window for the view the app asked for. The app hears of the view through
    /// `view_added`, which it tells by its request's id, or of the failure.
    pub(super) fn open_window(&mut self, event_loop: &ActiveEventLoop, request: WindowRequest) {
        if let Err(reason) = self.make_requested_window(event_loop, &request) {
            self.client
                .view_request_failed(request.request_id, ViewError::Failed(reason));
        }
    }

    /// The window `request` asked for, with a surface if the system gives the windows theirs
    /// now, and its view added.
    pub(super) fn make_requested_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        request: &WindowRequest,
    ) -> Result<(), String> {
        let open = self.view_maker.make_requested(
            event_loop,
            request,
            Rc::clone(self.state.requests()),
        )?;
        if self.surfaces_given {
            open.view.give_surface(&open.window)?;
        }
        self.add_view(open);
        Ok(())
    }

    /// A view the app asked for, with its window: it joins the platform's views, with the system's
    /// notices that its window may have been minimized or occluded, and then the app hears of it.
    pub(super) fn add_view(&mut self, open: OpenWindow) {
        let view_id = open.view.id();
        let window = Arc::clone(&open.window);
        let visibility_notices =
            ThisOs::observe_visibility(&window, self.send_visibility_notice(window.id()));
        self.state.add_view_with_window(open, visibility_notices);
        self.sync_frame_scheduler(view_id);
        self.client.view_added(view_id);
        self.read_theme_of(&window);
    }

    /// What a notice that the window `id` may have been minimized or occluded does: it queues for
    /// the loop, which reads the window's state again on a turn of its own.
    fn send_visibility_notice(&self, id: WindowId) -> Box<dyn Fn()> {
        let proxy = self.proxy.clone();
        Box::new(move || {
            let _ = proxy.send_event(MainLoopEvent::WindowVisibility(id));
        })
    }

    /// Opens the screen view's window, at winit's first call: the first moment winit lets a
    /// window be made. The app, started with the view at no size, hears its new metrics.
    pub(super) fn open_screen_window(&mut self, event_loop: &ActiveEventLoop) {
        let Some(config) = self.screen_window.take() else {
            return;
        };
        let view = self
            .views()
            .screen_view()
            .expect("the screen view was made with the app");
        let window =
            open_screen_window(event_loop, &config, &view).expect("open the screen view's window");
        if self.surfaces_given {
            give_surface(&view, &window);
        }
        self.views().attach_screen_window(Arc::clone(&window));
        self.sync_frame_scheduler(view.id());
        self.client.view_metrics_changed(view.id());
        self.read_theme_of(&window);
    }

    /// Closes a window: it is freed, and its view goes with it, but for the screen view,
    /// which outlives its window and takes no frames until it has one again.
    pub(super) fn close_window(&mut self, id: WindowId) {
        let Some(closing) = self.view_id_of(id) else {
            return;
        };
        let Some(after_close) = self.views().close(id) else {
            return;
        };
        match after_close {
            ViewAfterClose::Removed(view) => {
                self.frame_scheduler.remove_view(view);
                self.client.view_removed(view);
            }
            ViewAfterClose::Stays => self.sync_frame_scheduler(closing),
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
        if let Some(open) = self.views().by_view(change.view_id) {
            view_focus::focus_window_if_asked(&change, &open.window);
        }
    }

    /// A window was resized, which may come with its being minimized or restored: its view takes
    /// the new size.
    pub(super) fn on_resized(&mut self, id: WindowId) {
        self.resize_view(id);
        self.sync_window_state(id);
    }

    /// The view of window `id` takes the window's new size: the app hears of its new metrics, and
    /// of the system's theme, which some systems report as a resize, and where the system needs
    /// it the resized layout is drawn at once.
    fn resize_view(&mut self, id: WindowId) {
        let Some(open) = self.views().by_window(id) else {
            return;
        };
        self.read_theme_of(&open.window);
        let Some(size) = open.view.window_resized(&open.window) else {
            return;
        };
        self.client.view_metrics_changed(open.view.id());
        // Present the resized layout before the system commits the window geometry, rather
        // than stretching the old frame.
        if ThisOs::FRAME_ON_RESIZE && has_area(size) {
            self.draw_resize_frame(&open.view);
        }
    }

    /// Reads again which screen each window is on, and tells the client and the frame scheduler
    /// of the views whose screen changed.
    pub(super) fn sync_screens(&mut self) {
        let changed: Vec<ViewId> = self
            .views()
            .open_windows()
            .into_iter()
            .filter(|open| open.view.refresh_screen(&open.window))
            .map(|open| open.view.id())
            .collect();
        for view_id in changed {
            self.client.view_metrics_changed(view_id);
            self.sync_frame_scheduler(view_id);
        }
    }

    /// Reads every view's metrics again, for a system that reports no event for a change
    /// of the safe area, and tells the client of the views whose metrics moved.
    pub(super) fn sync_metrics(&mut self) {
        let changed: Vec<ViewId> = self
            .views()
            .open_windows()
            .into_iter()
            .filter(|open| open.view.refresh_metrics(&open.window))
            .map(|open| open.view.id())
            .collect();
        for view_id in changed {
            self.client.view_metrics_changed(view_id);
        }
    }

    /// The system gives the windows surfaces, at launch and each time it gives them back: every
    /// view gets one, at the metrics its window has now, and draws its last scene on it, since the
    /// framework presents only what changed; and the framework is asked for a frame of each.
    pub(super) fn give_surfaces(&mut self) {
        self.surfaces_given = true;
        let open_windows = self.views().open_windows();
        for open in &open_windows {
            give_surface(&open.view, &open.window);
        }
        self.sync_metrics();
        for open in open_windows {
            open.view.draw_last_scene();
            self.request_frame(open.view.id());
        }
    }

    /// The system takes every window's surface away: the app and its views stay, and each view
    /// keeps its last scene, owed until it is given a surface again.
    pub(super) fn take_surfaces(&mut self) {
        self.surfaces_given = false;
        for view in self.views().views() {
            view.take_surface();
        }
    }

    /// Only the system asks winit for redraws; the framework's frames come at the display's
    /// vsyncs. A window seen again, which is the system's word that its surface
    /// takes frames now, or one the system wants repainted, gets its latest picture
    /// presented again if it is not on the surface; otherwise the content is current and
    /// nothing is drawn.
    pub(super) fn on_redraw_requested(&mut self, id: WindowId) {
        let Some(open) = self.views().by_window(id) else {
            return;
        };
        open.view.draw_last_scene();
        self.request_vsync_if_owed(open.view.id());
    }

    /// A window moved, and may be on another screen now: the app hears of its view's new
    /// screen, as Flutter's macOS embedder sends a window's metrics again when its screen changes,
    /// and of the move, and its view's frames follow it to that screen's vsyncs. AppKit moves the windows off a screen that goes away, so this is also how their
    /// vsyncs leave it.
    pub(super) fn on_moved(&mut self, id: WindowId) {
        let Some(open) = self.views().by_window(id) else {
            return;
        };
        if open.view.refresh_screen(&open.window) {
            self.client.view_metrics_changed(open.view.id());
        }
        self.report_window_changed(&open);
        self.sync_frame_scheduler(open.view.id());
    }

    /// winit says a window is occluded, or shown: the app hears of it once the window answers so,
    /// and a window seen again is when a picture its surface refused while occluded is presented
    /// (wgpu issue 8309, `view.rs`).
    pub(super) fn on_occluded(&mut self, id: WindowId, occluded: bool) {
        if let Some(handle) = self.views().by_window(id).and_then(|open| open.handle) {
            handle.set_occluded(occluded);
        }
        self.sync_window_state(id);
        if !occluded {
            self.on_redraw_requested(id);
        }
    }

    /// Reads again whether window `id` is minimized and whether it is occluded: its view's frames
    /// are held while it is either, and the app hears if either changed since it last heard.
    pub(super) fn sync_window_state(&mut self, id: WindowId) {
        let Some(open) = self.views().by_window(id) else {
            return;
        };
        self.sync_frame_scheduler(open.view.id());
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

/// Gives `view` a surface on `window`, its own; a view that gets none is reported and draws
/// nothing. A window the app asks for answers the app with the error instead
/// (`make_requested_window`).
fn give_surface(view: &WinitView, window: &Arc<Window>) {
    if let Err(reason) = view.give_surface(window) {
        eprintln!("inset: no surface on view {:?}: {reason}", view.id());
    }
}

fn has_area([width, height]: [u32; 2]) -> bool {
    width > 0 && height > 0
}
