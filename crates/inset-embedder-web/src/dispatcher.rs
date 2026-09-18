//! The browser's event loop as the framework's [`Dispatcher`].
//!
//! The framework asks the host to wake it through a `Send + Sync` handle, because the asking
//! happens inside a task's `Waker`, and Rust requires a waker and everything it captures to be
//! sendable whether or not the target has threads. This platform is not sendable — it holds
//! the canvas and `Rc` state — and the web runs the whole app on one thread anyway. So the
//! handle carries nothing and finds the platform's wake through a thread-local, the same shim
//! wasm-bindgen-futures uses for its own single-threaded state. On a target with threads the
//! handle would hold a sendable channel to the loop instead, as the winit host's does.
//!
//! The framework sets its next timer wakeup whole, and the host reads it to choose its next
//! timeout or animation frame. A wake asked for by a ready task is kept separate from it,
//! because that timer has not run yet. Work has no other thread to go to: it runs on this
//! one, as a task of the browser's loop, once the caller's own work has ended.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use inset_embedder::Dispatcher;
use web_time::Instant;

use crate::platform::OnSchedule;

/// The platform's wake: the timer wakeup that was set, or a request to wake right away.
type Wake = Rc<dyn Fn(WakeRequest)>;

/// What the framework asks of the host's loop.
#[derive(Clone, Copy)]
pub(crate) enum WakeRequest {
    /// When to wake for the earliest waiting timer, or `None` when no timer is waiting.
    TimerWakeup(Option<Instant>),
    /// Wake as soon as the loop can.
    Now,
}

thread_local! {
    /// The platform's wake, installed once the platform exists.
    static WAKE: RefCell<Option<Wake>> = const { RefCell::new(None) };
}

/// The handle the platform hands out: empty, since the wake it reaches lives in this thread.
pub(crate) struct WebDispatcher;

impl Dispatcher for WebDispatcher {
    fn wake_at(&self, timer_wakeup: Option<Instant>) {
        wake(WakeRequest::TimerWakeup(timer_wakeup));
    }

    fn wake_now(&self) {
        wake(WakeRequest::Now);
    }

    fn dispatch(&self, work: Box<dyn FnOnce() + Send>) {
        wasm_bindgen_futures::spawn_local(async move { work() });
    }
}

fn wake(request: WakeRequest) {
    let wake = WAKE.with(|wake| wake.borrow().clone());
    if let Some(wake) = wake {
        wake(request);
    }
}

/// Leaves the platform's wake where [`WebDispatcher`] finds it: `timer_wakeup` is when to
/// wake for the earliest waiting timer, `wake_now_requested` is a ready task asking to be
/// woken, and `on_schedule` is the host's way of arranging that, read at each wake so the
/// host may install it later.
pub(crate) fn install(
    timer_wakeup: Rc<Cell<Option<Instant>>>,
    wake_now_requested: Rc<Cell<bool>>,
    on_schedule: OnSchedule,
) {
    WAKE.with(|wake| {
        *wake.borrow_mut() = Some(Rc::new(move |request: WakeRequest| {
            match request {
                // Set whole, so it replaces what was held: the new time can be later than
                // the old one, and `None` means no timer is waiting.
                WakeRequest::TimerWakeup(wakeup) => timer_wakeup.set(wakeup),
                WakeRequest::Now => wake_now_requested.set(true),
            }
            if let Some(callback) = on_schedule.borrow().as_ref() {
                callback();
            }
        }));
    });
}
