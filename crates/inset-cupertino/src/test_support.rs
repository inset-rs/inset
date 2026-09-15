//! Shared test scaffolding: a platform with one view to run widgets under, and one pumped
//! frame.

use std::rc::Rc;
use std::time::Duration;

use inset_embedder::{TargetPlatform, ViewRef};
use inset_foundation::{App, AppCell};
use inset_scheduler::SchedulerBinding;
use inset_test::{TestPlatform, TestView};
use inset_widgets::{IntoWidget, View, WidgetRef, run_widget};

/// The 800x600 physical view at 2x every test app draws into.
pub(crate) fn test_view() -> ViewRef {
    Rc::new(TestView::with_pixel_ratio(800.0, 600.0, 2.0))
}

/// An app on a platform with one 2x view, so pointer packets find their view.
pub(crate) fn test_cell() -> Rc<AppCell> {
    let platform = TestPlatform::new()
        .on(TargetPlatform::IOS)
        .with_view(test_view());
    AppCell::with_platform(Rc::new(platform))
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
