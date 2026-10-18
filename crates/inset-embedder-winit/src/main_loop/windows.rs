//! The main loop's windows and views: opening one the app asks for and the screen view's,
//! closing, focus, resizing, moving, and the surfaces the system gives and takes.

use std::rc::Rc;
use std::sync::Arc;

use inset_embedder::{EmbedderClient, View, ViewError, ViewFocusEvent, ViewId};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::os::{FramePresentation, ThisOs};
use crate::view::WinitView;
use crate::view_focus;
use crate::windows::{OpenWindow, ViewAfterClose, WindowRequest, open_screen_window};

use super::MainLoop;

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

    /// A view the app asked for, with its window: it joins the platform's views, and then the
    /// app hears of it.
    pub(super) fn add_view(&mut self, open: OpenWindow) {
        let view_id = open.view.id();
        let window = Arc::clone(&open.window);
        self.state.add_view_with_window(open);
        self.client.view_added(view_id);
        self.read_theme_of(&window);
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
        self.client.view_metrics_changed(view.id());
        self.read_theme_of(&window);
    }

    /// Closes a window: it is freed, and its view goes with it, but for the screen view,
    /// which outlives its window.
    pub(super) fn close_window(&mut self, id: WindowId) {
        let Some(after_close) = self.views().close(id) else {
            return;
        };
        if let ViewAfterClose::Removed(view) = after_close {
            self.client.view_removed(view);
        }
    }

    /// The user asked to close a window: the app decides, and closes it or not.
    pub(super) fn on_close_requested(&mut self, id: WindowId) {
        if let Some(view) = self.view_id_of(id) {
            self.client.view_close_requested(view);
        }
    }

    pub(super) fn on_focused(&mut self, id: WindowId, focused: bool) {
        if let Some(view) = self.view_id_of(id) {
            self.client
                .view_focus_changed(view_focus::focus_changed(view, focused));
        }
    }

    /// Honours the framework's request to focus a view, or to give up its focus.
    pub(super) fn change_view_focus(&mut self, change: ViewFocusEvent) {
        if let Some(open) = self.views().by_view(change.view_id) {
            view_focus::focus_window_if_asked(&change, &open.window, &open.view);
        }
    }

    pub(super) fn on_resized(&mut self, id: WindowId) {
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

    /// Reads again which screen each window is on, and tells the client of the views whose
    /// screen changed.
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
    /// view gets one, at the metrics its window has now, and the framework is asked for a frame
    /// to draw on them, as Flutter's shell asks when a view's surface is made.
    pub(super) fn give_surfaces(&mut self) {
        self.surfaces_given = true;
        for open in self.views().open_windows() {
            give_surface(&open.view, &open.window);
        }
        self.sync_metrics();
        self.request_frame();
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
        if let Some(open) = self.views().by_window(id) {
            open.view.draw_last_scene();
        }
        self.request_vsync_for_owed_scenes();
    }

    /// A window moved, and may be on another screen now: the app hears of its view's new
    /// screen, as Flutter's macOS embedder sends a window's metrics again when its screen changes,
    /// its native surfaces follow it, and so do the framework's vsyncs if it is the first window.
    /// AppKit moves the windows off a screen that goes away, so this is also how their vsyncs
    /// leave it.
    pub(super) fn on_moved(&mut self, id: WindowId) {
        if let Some(open) = self.views().by_window(id) {
            if open.view.refresh_screen(&open.window) {
                self.client.view_metrics_changed(open.view.id());
            }
            open.view.follow_window();
        }
        if let Some(display) = self.first_window_display() {
            self.frame_scheduler.follow_display(&display);
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
