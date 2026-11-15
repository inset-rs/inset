//! Waking the framework from any thread: the [`Dispatcher`] the platform hands out, and the wakes it
//! asked for, which the host serves on the main queue.
//!
//! A wake asked for now is a block on the main queue; one at a time is a block put on the main
//! queue for that time, unless one already waits for an earlier time, which serves it as it comes
//! by asking again. The main queue is served by the main run loop in its common modes, so a wake
//! comes while a scroll tracks a finger too. A block that finds nothing due wakes nothing.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use dispatch2::{DispatchQueue, DispatchTime};
use waft_embedder::Dispatcher;

use crate::host::{HostEvent, deliver};

/// The two reasons the host may have to wake the framework, kept apart so a ready task never
/// overwrites a timer that has not come.
#[derive(Default)]
pub(crate) struct Wakes {
    /// When to wake the framework for its earliest waiting timer, as it last set it.
    timer_wakeup: Mutex<Option<Instant>>,
    /// The time a block on the main queue waits for now, if one does.
    block_due: Mutex<Option<Instant>>,
    /// A ready task asking for the framework to be woken.
    wake_now_requested: AtomicBool,
}

impl Wakes {
    /// Takes the framework's word for when to wake it next for a timer, and puts a block on the
    /// main queue for it unless an earlier one waits already.
    fn set_timer_wakeup(&self, wanted: Option<Instant>) {
        *self
            .timer_wakeup
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = wanted;
        let Some(wanted) = wanted else {
            return;
        };
        let mut block_due = self
            .block_due
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if block_due.is_some_and(|due| due <= wanted) {
            return;
        }
        *block_due = Some(wanted);
        drop(block_due);
        let delay = wanted.saturating_duration_since(Instant::now());
        let when = DispatchTime::try_from(delay).unwrap_or(DispatchTime::FOREVER);
        let _ = DispatchQueue::main().after(when, || deliver(HostEvent::Wake));
    }

    /// Asks for the framework to be woken as soon as the main queue serves it.
    fn request_wake_now(&self) {
        self.wake_now_requested.store(true, Ordering::Release);
        DispatchQueue::main().exec_async(|| deliver(HostEvent::Wake));
    }

    /// Whether the framework should be woken now: a ready task asked for it, the timer wakeup has
    /// come, or both. A timer wakeup that has come is cleared here, and the framework sets the next
    /// one before the wake ends; a block for a time still to come is put on the queue again.
    pub(crate) fn should_wake(&self, now: Instant) -> bool {
        let asked = self.wake_now_requested.swap(false, Ordering::AcqRel);
        let mut wakeup = self
            .timer_wakeup
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let timer_came = wakeup.is_some_and(|wakeup| wakeup <= now);
        if timer_came {
            *wakeup = None;
        }
        let still_to_come = *wakeup;
        drop(wakeup);
        let mut block_due = self
            .block_due
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if block_due.is_some_and(|due| due <= now) {
            *block_due = None;
        }
        drop(block_due);
        if still_to_come.is_some() {
            self.set_timer_wakeup(still_to_come);
        }
        asked || timer_came
    }
}

/// The host's threads as any thread reaches them: a wake goes through the main queue; work goes to
/// a thread of its own.
pub(crate) struct IosDispatcher(pub(crate) std::sync::Arc<Wakes>);

impl Dispatcher for IosDispatcher {
    fn wake_at(&self, timer_wakeup: Option<Instant>) {
        self.0.set_timer_wakeup(timer_wakeup);
    }

    fn wake_now(&self) {
        self.0.request_wake_now();
    }

    fn dispatch(&self, work: Box<dyn FnOnce() + Send>) {
        std::thread::Builder::new()
            .name("waft-worker".into())
            .spawn(work)
            .expect("a thread for work off the main thread");
    }
}
