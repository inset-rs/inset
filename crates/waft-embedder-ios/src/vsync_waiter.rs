//! The platform's [`VsyncWaiter`]: a wait asked from any thread is set down and the host told on the
//! main queue, so the display link runs, and the host serves every wait at the link's next tick,
//! the one the framework's frames are drawn at (`display_link.rs`), so a producer that paces by it,
//! as a composition layer's does, draws at the display's rate.
//!
//! The invariant: a wait keeps the display link from pausing until it is served, so an app with
//! nothing of its own to draw still hears every refresh a producer waits on.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Instant;

use dispatch2::DispatchQueue;
use waft_embedder::{ScreenId, VsyncObserver, VsyncWaiter};

use crate::host::{HostEvent, deliver};
use crate::view::SCREEN;

/// The waits not yet served, shared by the waiter any thread holds and the host.
#[derive(Default)]
pub(crate) struct Waits {
    observers: Mutex<Vec<Weak<dyn VsyncObserver>>>,
}

impl Waits {
    /// Whether a wait is waiting for the next refresh.
    pub(crate) fn any(&self) -> bool {
        !self.locked().is_empty()
    }

    /// Serves every wait set down before the refresh at `at`, whose picture reaches the screen at
    /// `display_time`.
    pub(crate) fn serve(&self, at: Instant, display_time: Instant) {
        let observers = std::mem::take(&mut *self.locked());
        for observer in observers.iter().filter_map(Weak::upgrade) {
            observer.vsync(at, display_time);
        }
    }

    fn locked(&self) -> MutexGuard<'_, Vec<Weak<dyn VsyncObserver>>> {
        self.observers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// The display's refreshes as any thread waits on them. An iPhone has one screen, and a wait on any
/// other is never served.
pub(crate) struct DisplayLinkWaiter(pub(crate) Arc<Waits>);

impl VsyncWaiter for DisplayLinkWaiter {
    fn next_vsync(&self, screen: ScreenId, observer: Weak<dyn VsyncObserver>) {
        if screen != SCREEN {
            return;
        }
        let mut observers = self.0.locked();
        if observers.iter().any(|waiting| waiting.ptr_eq(&observer)) {
            return;
        }
        observers.push(observer);
        drop(observers);
        DispatchQueue::main().exec_async(|| deliver(HostEvent::VsyncWaited));
    }
}
