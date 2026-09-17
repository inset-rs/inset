//! The browser's event loop as the framework's [`Dispatcher`].
//!
//! The framework asks the host for a turn through a `Send + Sync` handle, because the asking
//! happens inside a task's `Waker`, and Rust requires a waker and everything it captures to be
//! sendable whether or not the target has threads. This platform is not sendable — it holds
//! the canvas and `Rc` state — and the web runs the whole app on one thread anyway. So the
//! handle carries nothing and finds the platform's wake through a thread-local, the same shim
//! wasm-bindgen-futures uses for its own single-threaded state. On a target with threads the
//! handle would hold a sendable channel to the loop instead, as the winit host's does.
//!
//! A wake keeps the earliest deadline asked for, where the host reads it to choose its next
//! timeout or animation frame, and schedules a turn for it. Work has no other thread to go to:
//! it runs on this one, as a task of the browser's loop, once the caller's turn has ended.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use inset_embedder::Dispatcher;
use web_time::Instant;

use crate::platform::OnSchedule;

/// The platform's wake: the earliest deadline kept and a turn scheduled for it.
type WakeAt = Rc<dyn Fn(Instant)>;

thread_local! {
    /// The platform's wake, installed once the platform exists.
    static WAKE_AT: RefCell<Option<WakeAt>> = const { RefCell::new(None) };
}

/// The handle the platform hands out: empty, since the wake it reaches lives in this thread.
pub(crate) struct WebDispatcher;

impl Dispatcher for WebDispatcher {
    fn wake_at(&self, deadline: Instant) {
        let wake_at = WAKE_AT.with(|wake_at| wake_at.borrow().clone());
        if let Some(wake_at) = wake_at {
            wake_at(deadline);
        }
    }

    fn dispatch(&self, work: Box<dyn FnOnce() + Send>) {
        wasm_bindgen_futures::spawn_local(async move { work() });
    }
}

/// Leaves the platform's wake where [`WebDispatcher`] finds it: `deadline` is the platform's
/// earliest pending wake, and `on_schedule` the host's way of arranging a turn, read at each
/// wake so the host may install it later.
pub(crate) fn install(deadline: Rc<Cell<Option<Instant>>>, on_schedule: OnSchedule) {
    WAKE_AT.with(|wake_at| {
        *wake_at.borrow_mut() = Some(Rc::new(move |asked: Instant| {
            let due = deadline.get().map_or(asked, |due| due.min(asked));
            deadline.set(Some(due));
            if let Some(callback) = on_schedule.borrow().as_ref() {
                callback();
            }
        }));
    });
}
