//! The main loop's views: a view's coming, changing and going, whoever holds it, as Flutter's
//! embedder API has them, `AddView` ([`MainLoop::add_view`]), `SendWindowMetricsEvent`
//! ([`MainLoop::set_view_metrics`]) and `RemoveView` ([`MainLoop::remove_view`]); whether it is in
//! sight, as its holder says ([`MainLoop::set_view_in_sight`]); and the surfaces the system gives
//! the views and takes away.
//!
//! Each does the view's own part in the registry of views (`views.rs`), then what the loop alone
//! can do, since it owns the frame scheduler and the client: the view's frames follow the screen
//! its metrics name and are held while it is out of sight (`frame_scheduler.rs`), and the
//! framework hears of it at once. A view is in sight until its holder says otherwise.
//!
//! A window's code is a holder: it turns its window's opening, resizing, moving to another screen
//! and closing into these calls, and its being minimized or occluded into its view's being out of
//! sight (`main_loop/windows.rs`).

use std::rc::Rc;

use inset_embedder::{EmbedderClient, View, ViewId, ViewMetrics};
use winit::event_loop::ActiveEventLoop;

use crate::os::ViewLayer;
use crate::view::WinitView;

use super::MainLoop;
use super::frames::monitor_of_screen;

impl<C: EmbedderClient> MainLoop<C> {
    /// Flutter's `AddView`: `view`, which its holder made, draws into `layer` at `metrics` from now
    /// on, on a surface while the system gives the views theirs; its frames follow the screen its
    /// metrics name, and the framework hears of it.
    pub(super) fn add_view(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        view: Rc<WinitView>,
        layer: ViewLayer,
        metrics: ViewMetrics,
    ) {
        self.views().add(Rc::clone(&view), layer, metrics);
        self.follow_screen(event_loop, &view);
        self.client.view_added(view.id());
    }

    /// Flutter's `SendWindowMetricsEvent`: the view `id` takes `metrics`, its frames follow the
    /// screen they name, and the framework hears of them. Answers whether they changed: the same
    /// metrics again are nothing to tell.
    pub(super) fn set_view_metrics(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        id: ViewId,
        metrics: ViewMetrics,
    ) -> bool {
        let Some(view) = self.views().get(id) else {
            return false;
        };
        if !view.set_metrics(metrics) {
            return false;
        }
        self.follow_screen(event_loop, &view);
        self.client.view_metrics_changed(id);
        true
    }

    /// Flutter's `RemoveView`: the view `id` lets go of its layer and is the app's no more, no
    /// frame is drawn for it again, and the framework hears of it.
    pub(super) fn remove_view(&mut self, id: ViewId) {
        if !self.views().remove(id) {
            return;
        }
        self.frame_scheduler.remove_view(id);
        self.client.view_removed(id);
    }

    /// The view `id` is in sight or not, as its holder says: its frames are held while it is not,
    /// and one that comes into sight takes its videos' frames again.
    pub(super) fn set_view_in_sight(&mut self, id: ViewId, in_sight: bool) {
        self.frame_scheduler.set_in_sight(id, in_sight);
        self.request_vsync_if_wanted(id);
    }

    /// The system gives the views surfaces, at launch and each time it gives them back: every view
    /// gets one and draws its last scene on it, since the framework presents only what changed;
    /// and the framework is asked for a frame of each.
    pub(super) fn give_surfaces(&mut self) {
        self.views().give_surfaces();
        for view in self.views().all() {
            view.draw_last_scene();
            self.request_frame(view.id());
        }
    }

    /// The frames of `view` follow the screen its metrics name: they are drawn at that screen's
    /// vsyncs from now on.
    fn follow_screen(&mut self, event_loop: &dyn ActiveEventLoop, view: &WinitView) {
        let monitor = monitor_of_screen(event_loop, view.metrics().screen);
        self.frame_scheduler.set_screen(view.id(), monitor.as_ref());
        self.request_vsync_if_wanted(view.id());
    }
}
