//! What the scheduler's tests share: an app on a platform with views, and the refreshes a host
//! gives it.

use std::rc::Rc;
use std::time::Duration;

use inset_embedder::ViewId;
use inset_foundation::{App, AppCell};
use inset_test::TestPlatform;

use crate::SchedulerBinding;

/// An app on a platform that has a view numbered by each of `ids`, and the platform.
pub(crate) fn app_with_views(ids: &[ViewId]) -> (Rc<AppCell>, Rc<TestPlatform>) {
    let platform = Rc::new(TestPlatform::new().with_views(ids));
    (AppCell::with_platform(platform.clone()), platform)
}

/// A refresh of `views` at `at`, with the microtasks drained after each half, as the shell runs
/// one.
pub(crate) fn pump(app: &mut App, views: &[ViewId], at: Duration) {
    SchedulerBinding::handle_begin_frame(app, views, Some(at));
    app.drain_microtasks();
    SchedulerBinding::handle_draw_frame(app, views);
    app.drain_microtasks();
}
