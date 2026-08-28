//! Host capabilities the framework calls at runtime.
//!
//! Unlike Dart's isolate-global `PlatformDispatcher`, this object is supplied
//! by the embedder and held by `App`. Incoming host events travel through
//! `EmbedderClient`, so this interface contains requests and state only.

use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::{View, ViewId};

pub type PlatformRef = Rc<dyn Platform>;
pub type ViewRef = Rc<dyn View>;

/// The long-lived host object held by the application.
///
/// Implementations must queue requests and return. They must not synchronously
/// re-enter the [`EmbedderClient`](crate::EmbedderClient) while `App` is active.
pub trait Platform: 'static {
    /// Requests one isolate frame at the host's next appropriate opportunity.
    fn request_frame(&self);

    /// The host clock used for frame and timer timestamps.
    fn now(&self) -> Instant;

    /// Asks the host to wake the application at `deadline`.
    fn wake_at(&self, deadline: Instant);

    /// Current host-provided views.
    fn views(&self) -> Vec<ViewRef>;

    /// Looks up a current host-provided view.
    fn view(&self, id: ViewId) -> Option<ViewRef>;

    /// The stable implicit view, when this embedding provides one.
    fn implicit_view(&self) -> Option<ViewRef>;
}

/// Platform for hand-pumped tests.
pub struct InertPlatform;

impl Platform for InertPlatform {
    fn request_frame(&self) {}

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn wake_at(&self, _deadline: Instant) {}

    fn views(&self) -> Vec<ViewRef> {
        Vec::new()
    }

    fn view(&self, _id: ViewId) -> Option<ViewRef> {
        None
    }

    fn implicit_view(&self) -> Option<ViewRef> {
        None
    }
}

/// One complete engine frame delivered to the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    pub elapsed: Duration,
}
