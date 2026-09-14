//! Shared test scaffolding: a platform with one view to run widgets under, and one pumped
//! frame.

use std::rc::Rc;
use std::time::Duration;

use inset_embedder::{
    Picture, Platform, PlatformRef, TargetPlatform, View as EmbedderView, ViewConstraints, ViewId,
    ViewMetrics, ViewRef,
};
use inset_foundation::{App, AppCell};
use inset_scheduler::SchedulerBinding;
use inset_widgets::{IntoWidget, View, WidgetRef, run_widget};

/// An 800x600 physical view at 2x that presents nowhere.
pub(crate) struct TestView;

impl EmbedderView for TestView {
    fn id(&self) -> ViewId {
        ViewId(0)
    }

    fn metrics(&self) -> ViewMetrics {
        ViewMetrics {
            physical_size: [800.0, 600.0],
            physical_constraints: ViewConstraints::tight(800.0, 600.0),
            device_pixel_ratio: 2.0,
            ..ViewMetrics::default()
        }
    }

    fn present(&self, _picture: std::sync::Arc<Picture>) {}
}

/// A platform whose implicit view is [`TestView`], so pointer packets find their view.
struct TestPlatform {
    view: ViewRef,
}

impl Platform for TestPlatform {
    fn target_platform(&self) -> TargetPlatform {
        TargetPlatform::IOS
    }

    fn request_frame(&self) {}

    fn now(&self) -> std::time::Instant {
        std::time::Instant::now()
    }

    fn wake_at(&self, _deadline: std::time::Instant) {}

    fn views(&self) -> Vec<ViewRef> {
        vec![Rc::clone(&self.view)]
    }

    fn view(&self, id: ViewId) -> Option<ViewRef> {
        (self.view.id() == id).then(|| Rc::clone(&self.view))
    }

    fn implicit_view(&self) -> Option<ViewRef> {
        Some(Rc::clone(&self.view))
    }
}

/// An app on a platform with one 2x view.
pub(crate) fn test_cell() -> Rc<AppCell> {
    let platform: PlatformRef = Rc::new(TestPlatform {
        view: Rc::new(TestView),
    });
    AppCell::with_platform(platform)
}

/// Mounts `child` under the platform's view and runs the first frame.
pub(crate) fn build(cell: &AppCell, child: WidgetRef) {
    {
        let mut app = cell.borrow_mut();
        let view = app
            .platform()
            .implicit_view()
            .expect("an app from test_support::test_cell");
        run_widget(&mut app, View::new(view, child).into_widget());
    }
    // `run_app` attaches the root widget on the next timer turn, as Dart's `Timer.run` does.
    cell.elapse(Duration::ZERO);
    pump(&mut cell.borrow_mut(), Duration::ZERO);
}

/// Runs one frame at `at`.
pub(crate) fn pump(app: &mut App, at: Duration) {
    SchedulerBinding::handle_begin_frame(app, Some(at));
    app.drain_microtasks();
    SchedulerBinding::handle_draw_frame(app);
    app.drain_microtasks();
}
