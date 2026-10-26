//! Waking the framework: the [`Dispatcher`] any thread reaches the embedder's threads through, and
//! the wakes it asked for as the event loop keeps them, which decide how long the loop waits.

use std::time::Instant;

use inset_embedder::Dispatcher;
use winit::event_loop::{ControlFlow, EventLoopProxy};

use crate::main_loop::MainLoopEvent;
use crate::os::{ThisOs, WorkQueue};

/// The embedder's threads as any thread reaches them: a wake is an event through winit's
/// proxy, which the loop keeps as its earliest due wake and serves when the time comes; work
/// goes to the system's queue, or a thread where the system has none.
pub(crate) struct WinitDispatcher(pub(crate) EventLoopProxy<MainLoopEvent>);

impl Dispatcher for WinitDispatcher {
    fn wake_at(&self, timer_wakeup: Option<Instant>) {
        let _ = self.0.send_event(MainLoopEvent::TimerWakeup(timer_wakeup));
    }

    fn wake_now(&self) {
        let _ = self.0.send_event(MainLoopEvent::WakeNow);
    }

    fn dispatch(&self, work: Box<dyn FnOnce() + Send>) {
        ThisOs::run_off_main(work);
    }
}

/// The wakes the framework asked for, as the event loop heard them.
#[derive(Default)]
pub(crate) struct Wakeups {
    /// When to wake the framework for its earliest waiting timer, as it last set it;
    /// `None` when no timer is waiting, or once that wake has been given.
    timer_wakeup: Option<Instant>,
    /// A ready task asking for the framework to be woken, cleared by the next wake.
    wake_now_requested: bool,
}

impl Wakeups {
    /// The framework sets this whole, so it replaces what was held: the new time can be later
    /// than the old one, and `None` means no timer is waiting.
    pub(crate) fn set_timer_wakeup(&mut self, timer_wakeup: Option<Instant>) {
        self.timer_wakeup = timer_wakeup;
    }

    /// A ready task asks for the framework to be woken; it never changes the timer wakeup.
    pub(crate) fn wake_now(&mut self) {
        self.wake_now_requested = true;
    }

    /// Whether the framework should be woken by `now`: a ready task asked for it, or its
    /// timer wakeup has arrived. A timer wakeup that has arrived is cleared here, and the
    /// framework sets the next one before the wake ends.
    pub(crate) fn take_due(&mut self, now: Instant) -> bool {
        let asked = std::mem::take(&mut self.wake_now_requested);
        let timer_came = self.timer_wakeup.is_some_and(|wakeup| wakeup <= now);
        if timer_came {
            self.timer_wakeup = None;
        }
        asked || timer_came
    }

    /// How the loop waits once this turn is done: not at all while a ready task waits,
    /// otherwise until the earlier of the timer wakeup and `loop_due`, the earliest of the loop's
    /// own times (the next vsync of a timer standing in for a display link, a copy's expiry), or
    /// until an event when there is neither.
    pub(crate) fn control_flow(&self, loop_due: Option<Instant>) -> ControlFlow {
        if self.wake_now_requested {
            return ControlFlow::Poll;
        }
        let resume_at = earliest(self.timer_wakeup, loop_due);
        match resume_at {
            Some(resume_at) => ControlFlow::WaitUntil(resume_at),
            None => ControlFlow::Wait,
        }
    }
}

/// The earlier of two times, either of which may be none.
pub(crate) fn earliest(first: Option<Instant>, second: Option<Instant>) -> Option<Instant> {
    match (first, second) {
        (Some(first), Some(second)) => Some(first.min(second)),
        (first, second) => first.or(second),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn a_timer_wakeup_is_due_once_it_has_arrived_and_only_once() {
        let now = Instant::now();
        let mut wakeups = Wakeups::default();
        wakeups.set_timer_wakeup(Some(now + Duration::from_millis(10)));
        assert!(!wakeups.take_due(now));
        assert!(wakeups.take_due(now + Duration::from_millis(10)));
        assert!(
            !wakeups.take_due(now + Duration::from_millis(20)),
            "given once"
        );
    }

    #[test]
    fn a_ready_task_wakes_at_once_and_keeps_the_timer_wakeup() {
        let now = Instant::now();
        let later = now + Duration::from_secs(1);
        let mut wakeups = Wakeups::default();
        wakeups.set_timer_wakeup(Some(later));
        wakeups.wake_now();
        assert_eq!(wakeups.control_flow(None), ControlFlow::Poll);
        assert!(wakeups.take_due(now));
        assert_eq!(wakeups.control_flow(None), ControlFlow::WaitUntil(later));
    }

    #[test]
    fn the_loop_waits_until_the_earlier_of_the_timer_wakeup_and_the_timer_vsync() {
        let now = Instant::now();
        let vsync = now + Duration::from_millis(16);
        let mut wakeups = Wakeups::default();
        assert_eq!(wakeups.control_flow(None), ControlFlow::Wait);
        assert_eq!(
            wakeups.control_flow(Some(vsync)),
            ControlFlow::WaitUntil(vsync)
        );
        wakeups.set_timer_wakeup(Some(now + Duration::from_secs(1)));
        assert_eq!(
            wakeups.control_flow(Some(vsync)),
            ControlFlow::WaitUntil(vsync)
        );
    }
}
