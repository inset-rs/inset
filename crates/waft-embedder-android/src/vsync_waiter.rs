//! The platform's [`VsyncWaiter`]: a wait asked from any thread is set down and the looper woken,
//! and the loop serves every wait at the display's next refresh, from the same choreographer the
//! framework's frames are drawn at (`vsync.rs`), so a producer that paces by it, as a composition
//! layer's does, draws at the display's rate.
//!
//! The invariant: a wait keeps the signal from pausing until it is served, so an app with nothing
//! of its own to draw still hears every refresh a producer waits on.

use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::{Duration, Instant};

use waft_embedder::{ScreenId, VsyncObserver, VsyncWaiter};

use crate::loop_waker::LoopWaker;

/// The waits not yet served, shared by the waiter any thread holds and the loop.
pub(crate) struct Waits {
    observers: Mutex<Vec<Weak<dyn VsyncObserver>>>,
    waker: LoopWaker,
}

impl Waits {
    pub(crate) fn new(waker: LoopWaker) -> Arc<Waits> {
        Arc::new(Waits {
            observers: Mutex::new(Vec::new()),
            waker,
        })
    }

    /// Whether a wait is waiting for the next refresh.
    pub(crate) fn any(&self) -> bool {
        !self.locked().is_empty()
    }

    /// Serves every wait set down before this refresh, which `period` apart from the next is
    /// expected to reach the screen one refresh from now, as Flutter's Android waiter targets it.
    pub(crate) fn serve(&self, period: Duration) {
        let observers = std::mem::take(&mut *self.locked());
        let at = Instant::now();
        let display_time = at + period;
        for observer in observers.iter().filter_map(Weak::upgrade) {
            observer.vsync(at, display_time);
        }
    }

    fn locked(&self) -> std::sync::MutexGuard<'_, Vec<Weak<dyn VsyncObserver>>> {
        self.observers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// The display's refreshes as any thread waits on them. A phone has one screen, the activity's,
/// and a wait on any other is never served.
pub(crate) struct ChoreographerWaiter {
    pub(crate) waits: Arc<Waits>,
    /// The screen the activity is on, as the app was last told it.
    pub(crate) screen: Arc<Mutex<Option<ScreenId>>>,
}

impl VsyncWaiter for ChoreographerWaiter {
    fn next_vsync(&self, screen: ScreenId, observer: Weak<dyn VsyncObserver>) {
        let on_the_screen = *self.screen.lock().unwrap_or_else(PoisonError::into_inner);
        if on_the_screen != Some(screen) {
            return;
        }
        let mut observers = self.waits.locked();
        if observers.iter().any(|waiting| waiting.ptr_eq(&observer)) {
            return;
        }
        observers.push(observer);
        drop(observers);
        self.waits.waker.wake();
    }
}
