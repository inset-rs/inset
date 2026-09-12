//! Isolate timer queue. Dart counterpart: `dart:async` `Timer`.
//!
//! [`App`](crate::App) owns one of these and forwards the public methods.

use std::time::Duration;

use crate::change_notifier::Listener;

/// Drain budget for one [`App::elapse`](crate::App::elapse): a callback that
/// keeps scheduling an already-due timer must fail loud, not spin. Not a Dart
/// constant.
const TIMER_BUDGET: usize = 100_000;

/// A scheduled one-shot callback — `dart:async` `Timer(duration, callback)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timer {
    id: u64,
}

impl Timer {
    /// Dart's `Timer(duration, callback)`.
    pub fn new(app: &mut crate::App, duration: Duration, callback: Listener) -> Timer {
        app.schedule_timer(duration, callback)
    }

    /// Dart's `Timer.cancel()`.
    pub fn cancel(self, app: &mut crate::App) {
        app.cancel_timer(self);
    }

    /// Dart's `Timer.isActive`.
    pub fn is_active(self, app: &crate::App) -> bool {
        app.timer_is_active(self)
    }
}

struct ScheduledTimer {
    id: u64,
    due: Duration,
    callback: Listener,
}

/// Pending one-shot timers and the logical clock they measure from.
#[derive(Default)]
pub(crate) struct Timers {
    queue: Vec<ScheduledTimer>,
    next_id: u64,
    now: Duration,
    /// The due time the platform was last asked to wake for, so it is not asked twice.
    wake_requested_for: Option<Duration>,
}

impl Timers {
    /// Schedules `callback` at `now + duration`. Returns the timer and, when
    /// this is the new earliest deadline, the delay to pass to `wake_at`.
    pub(crate) fn schedule(
        &mut self,
        duration: Duration,
        callback: Listener,
    ) -> (Timer, Option<Duration>) {
        let id = self.next_id;
        self.next_id += 1;
        let due = self.now + duration;
        self.queue.push(ScheduledTimer { id, due, callback });
        (Timer { id }, self.next_wake())
    }

    /// The delay to pass to `wake_at` for the earliest pending timer, unless the platform was
    /// already asked to wake for that deadline.
    pub(crate) fn next_wake(&mut self) -> Option<Duration> {
        let due = self.earliest_due()?;
        if self.wake_requested_for == Some(due) {
            return None;
        }
        self.wake_requested_for = Some(due);
        Some(due.saturating_sub(self.now))
    }

    pub(crate) fn cancel(&mut self, timer: Timer) {
        self.queue.retain(|scheduled| scheduled.id != timer.id);
    }

    pub(crate) fn is_active(&self, timer: Timer) -> bool {
        self.queue.iter().any(|scheduled| scheduled.id == timer.id)
    }

    pub(crate) fn now(&self) -> Duration {
        self.now
    }

    /// Removes and returns the next timer due at or before `target`, advancing
    /// the clock to that timer's due time. `None` when none are due.
    pub(crate) fn pop_next_due(&mut self, target: Duration) -> Option<Listener> {
        let index = self
            .queue
            .iter()
            .enumerate()
            .filter(|(_, scheduled)| scheduled.due <= target)
            .min_by_key(|(_, scheduled)| (scheduled.due, scheduled.id))
            .map(|(index, _)| index)?;
        let scheduled = self.queue.remove(index);
        if scheduled.due > self.now {
            self.now = scheduled.due;
        }
        if self.wake_requested_for == Some(scheduled.due) {
            // The platform's deadline was this timer's; it has passed.
            self.wake_requested_for = None;
        }
        Some(scheduled.callback)
    }

    pub(crate) fn advance_to(&mut self, target: Duration) {
        self.now = target;
    }

    pub(crate) fn assert_fire_budget(fired: usize) {
        assert!(
            fired <= TIMER_BUDGET,
            "timers did not converge after {TIMER_BUDGET} callbacks; is a \
             callback rescheduling an already-due timer?"
        );
    }

    pub(crate) fn earliest_due(&self) -> Option<Duration> {
        self.queue.iter().map(|scheduled| scheduled.due).min()
    }
}
