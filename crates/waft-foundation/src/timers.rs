//! Isolate timer queue. Dart counterpart: `dart:async` `Timer` and `Future.delayed`.
//!
//! [`App`](crate::App) owns one of these and forwards the public methods.

use std::rc::Rc;
use std::time::Duration;

use crate::app::App;
use crate::change_notifier::Listener;
use crate::completer::{Completer, CompleterFuture};

/// Drain budget for one [`AppCell::elapse`](crate::AppCell::elapse): a callback that
/// keeps scheduling an already-due timer must fail loud, not spin. Not a Dart
/// constant.
const TIMER_BUDGET: usize = 100_000;

/// How a turn of the event loop moves the clock and which timers it fires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TimerMode {
    /// A running app's, as Dart's VM (`timer_impl.dart`, `_runTimers`): the clock is already at
    /// the turn's time, the turn fires the timers that were due when it began, each once, a
    /// periodic one's [`tick`](Timer::tick) jumping over the periods it missed, and a timer made
    /// or re-armed during the turn waits for the next one, so nothing a stall missed is replayed.
    CatchUp,
    /// A test's fake clock, as FakeAsync's `elapse`: the clock walks to each due timer in turn,
    /// so every period fires and a timer made on the way fires in the same walk if it comes due
    /// in it; only a zero-period timer waits for the next turn, as it would never move the clock.
    EachPeriod,
}

/// A scheduled callback — `dart:async` `Timer(duration, callback)` and
/// `Timer.periodic(duration, callback)`, which is handed its timer.
pub(crate) type TimerCallback = Rc<dyn Fn(&mut App, Timer)>;

/// A scheduled timer — `dart:async` `Timer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timer {
    id: u64,
}

impl Timer {
    /// Dart's `Timer(duration, callback)`.
    pub fn new(app: &mut App, duration: Duration, callback: Listener) -> Timer {
        app.schedule_timer(duration, callback)
    }

    /// Dart's `Timer.periodic(duration, callback)`, which fires every `duration` from when it
    /// was made until it is cancelled, and hands `callback` the timer, whose [`tick`](Self::tick)
    /// counts the periods that passed.
    ///
    /// A running app that was away for several periods fires once when it runs again and
    /// `tick` jumps over them, as Dart's VM does ([`AppCell::elapse_to`](crate::AppCell::elapse_to));
    /// a test's [`elapse`](crate::AppCell::elapse) fires each period in turn, as FakeAsync does.
    /// A zero `duration` fires once per turn of either clock.
    pub fn periodic(
        app: &mut App,
        duration: Duration,
        callback: impl Fn(&mut App, Timer) + 'static,
    ) -> Timer {
        app.schedule_periodic_timer(duration, callback)
    }

    /// Dart's `Timer.cancel()`.
    pub fn cancel(self, app: &mut App) {
        app.cancel_timer(self);
    }

    /// Dart's `Timer.isActive`.
    pub fn is_active(self, app: &App) -> bool {
        app.timer_is_active(self)
    }

    /// Dart's `Timer.tick`: the periods that elapsed before the timer's most recent fire, 1 in
    /// the first callback of a [`periodic`](Self::periodic) timer and 0 before one fires. A
    /// cancelled timer, or a one-shot that has fired, reads 0 here, where Dart keeps the count it
    /// last had.
    pub fn tick(self, app: &App) -> u64 {
        app.timer_tick(self)
    }
}

/// Dart's `Future.delayed(duration)`: a future that completes when the app's clock has advanced
/// by `duration`, which a test's [`AppCell::elapse`](crate::AppCell::elapse) does and the host's
/// timers do on a running app.
pub fn delayed(app: &mut App, duration: Duration) -> CompleterFuture<()> {
    let completer: Completer<()> = Completer::new();
    let future = completer.future();
    Timer::new(
        app,
        duration,
        Listener::new(move |app| completer.complete(app, ())),
    );
    future
}

struct ScheduledTimer {
    id: u64,
    /// The next time it is due: its first fire's time, or the one after its last expected fire.
    due: Duration,
    /// How often it fires again from its last expected fire; `None` for a one-shot.
    period: Option<Duration>,
    /// The fires it has had, which [`Timer::tick`] answers.
    tick: u64,
    /// The turn of the event loop it was made or re-armed in: a turn fires only the timers
    /// armed before it, under [`TimerMode::CatchUp`], and a zero-period timer once a turn always.
    armed_turn: u64,
    callback: TimerCallback,
}

impl ScheduledTimer {
    /// Whether the turn `turn` may fire it, in `mode`, once it is due.
    fn fires_in_turn(&self, turn: u64, mode: TimerMode) -> bool {
        let armed_before = self.armed_turn < turn;
        match mode {
            TimerMode::CatchUp => armed_before,
            TimerMode::EachPeriod => armed_before || self.period != Some(Duration::ZERO),
        }
    }

    /// Dart's VM's catch-up in `_runTimers`: a periodic timer more than a period overdue at
    /// `now` moves its due time and tick over the periods it missed.
    fn skip_missed_periods(&mut self, period: Duration, now: Duration) {
        let overdue = now.saturating_sub(self.due);
        if period.is_zero() || overdue <= period {
            return;
        }
        let missed = u32::try_from(overdue.as_nanos() / period.as_nanos()).unwrap_or(u32::MAX);
        self.due += period * missed;
        self.tick += u64::from(missed);
    }
}

/// Pending timers and the logical clock they measure from.
#[derive(Default)]
pub(crate) struct Timers {
    queue: Vec<ScheduledTimer>,
    next_id: u64,
    now: Duration,
    /// The timer wakeup the host has, so it is not told the same one twice. Only
    /// [`timer_wakeup_if_changed`](Self::timer_wakeup_if_changed) writes it: a value kept
    /// in two places is a value that can disagree with itself.
    host_timer_wakeup: Option<Duration>,
    /// The event-loop turn, counted so a timer armed during one is told from those due in it.
    turn: u64,
}

impl Timers {
    /// Schedules `callback` at `now + duration`.
    pub(crate) fn schedule(&mut self, duration: Duration, callback: Listener) -> Timer {
        self.add(duration, None, Rc::new(move |app, _| callback.call(app)))
    }

    /// Schedules `callback` at `now + duration` and every `duration` after it.
    pub(crate) fn schedule_periodic(
        &mut self,
        duration: Duration,
        callback: TimerCallback,
    ) -> Timer {
        self.add(duration, Some(duration), callback)
    }

    fn add(
        &mut self,
        duration: Duration,
        period: Option<Duration>,
        callback: TimerCallback,
    ) -> Timer {
        let id = self.next_id;
        self.next_id += 1;
        let due = self.now + duration;
        self.queue.push(ScheduledTimer {
            id,
            due,
            period,
            tick: 0,
            armed_turn: self.turn,
            callback,
        });
        Timer { id }
    }

    /// Begins an event-loop turn: the timers armed before it may fire in it.
    pub(crate) fn begin_turn(&mut self) {
        self.turn += 1;
    }

    /// How far off the earliest waiting timer is, when that is not what the host was last
    /// told, and [`None`] when the host already has it right. The inner [`None`] means no
    /// timer is waiting, which the host is told so it stops waiting for one.
    pub(crate) fn timer_wakeup_if_changed(&mut self) -> Option<Option<Duration>> {
        let due = self.earliest_due();
        if due == self.host_timer_wakeup {
            return None;
        }
        self.host_timer_wakeup = due;
        Some(due.map(|due| due.saturating_sub(self.now)))
    }

    pub(crate) fn cancel(&mut self, timer: Timer) {
        self.queue.retain(|scheduled| scheduled.id != timer.id);
    }

    pub(crate) fn is_active(&self, timer: Timer) -> bool {
        self.queue.iter().any(|scheduled| scheduled.id == timer.id)
    }

    pub(crate) fn tick(&self, timer: Timer) -> u64 {
        self.queue
            .iter()
            .find(|scheduled| scheduled.id == timer.id)
            .map_or(0, |scheduled| scheduled.tick)
    }

    pub(crate) fn now(&self) -> Duration {
        self.now
    }

    pub(crate) fn has_pending(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Removes and returns the next timer that fires at or before `target` in this turn,
    /// advancing the clock to its due time if it is behind, and re-arms a periodic timer for its
    /// next fire. `None` when none does. Which timers a turn fires is [`TimerMode`]'s.
    pub(crate) fn pop_next_due(
        &mut self,
        target: Duration,
        mode: TimerMode,
    ) -> Option<(Timer, TimerCallback)> {
        let index = self
            .queue
            .iter()
            .enumerate()
            .filter(|(_, scheduled)| {
                scheduled.due <= target && scheduled.fires_in_turn(self.turn, mode)
            })
            .min_by_key(|(_, scheduled)| (scheduled.due, scheduled.id))
            .map(|(index, _)| index)?;
        let mut scheduled = self.queue.remove(index);
        self.now = self.now.max(scheduled.due);
        if let Some(period) = scheduled.period {
            scheduled.skip_missed_periods(period, self.now);
        }
        scheduled.tick += 1;
        let fired = (Timer { id: scheduled.id }, Rc::clone(&scheduled.callback));
        if let Some(period) = scheduled.period {
            self.rearm(scheduled, period);
        }
        Some(fired)
    }

    /// Queues a periodic timer that just fired for its next fire, a period after the one it
    /// had, or now for a zero period, as the VM's `_advanceWakeupTime` does.
    fn rearm(&mut self, mut scheduled: ScheduledTimer, period: Duration) {
        scheduled.due = if period.is_zero() {
            self.now
        } else {
            scheduled.due + period
        };
        scheduled.armed_turn = self.turn;
        self.queue.push(scheduled);
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    use crate::app_cell::AppCell;

    use super::*;

    /// What each fire saw: the timer's tick at the time.
    fn ticks() -> Rc<RefCell<Vec<u64>>> {
        Rc::default()
    }

    #[test]
    fn a_periodic_timer_fires_every_period_with_its_tick_and_stops_when_cancelled() {
        let cell = AppCell::new();
        let seen = ticks();
        let sink = Rc::clone(&seen);
        let timer = Timer::periodic(&mut cell.borrow_mut(), Duration::from_millis(10), {
            let seen = Rc::clone(&sink);
            move |app, timer| seen.borrow_mut().push(timer.tick(app))
        });

        cell.elapse(Duration::from_millis(10));
        assert_eq!(*seen.borrow(), [1], "the first callback's tick is 1");
        cell.elapse(Duration::from_millis(15));
        assert_eq!(
            *seen.borrow(),
            [1, 2],
            "the second period fired with tick 2"
        );
        assert!(timer.is_active(&cell.borrow()));

        timer.cancel(&mut cell.borrow_mut());
        cell.elapse(Duration::from_millis(50));
        assert_eq!(
            *seen.borrow(),
            [1, 2],
            "a cancelled timer never fires again"
        );
        assert!(!timer.is_active(&cell.borrow()));
    }

    #[test]
    fn elapsing_past_several_periods_fires_each_one() {
        let cell = AppCell::new();
        let seen = ticks();
        let sink = Rc::clone(&seen);
        Timer::periodic(&mut cell.borrow_mut(), Duration::from_millis(10), {
            let seen = Rc::clone(&sink);
            move |app, timer| seen.borrow_mut().push(timer.tick(app))
        });

        cell.elapse(Duration::from_millis(35));
        assert_eq!(*seen.borrow(), [1, 2, 3]);
    }

    #[test]
    fn a_periodic_timer_can_cancel_itself() {
        let cell = AppCell::new();
        let seen = ticks();
        let sink = Rc::clone(&seen);
        let timer = Rc::new(RefCell::new(None::<Timer>));
        let handle = Rc::clone(&timer);
        let started = Timer::periodic(
            &mut cell.borrow_mut(),
            Duration::from_millis(10),
            move |app, timer| {
                sink.borrow_mut().push(timer.tick(app));
                if let Some(itself) = *handle.borrow() {
                    itself.cancel(app);
                }
            },
        );
        *timer.borrow_mut() = Some(started);

        cell.elapse(Duration::from_millis(50));
        assert_eq!(*seen.borrow(), [1], "the callback's cancel stopped it");
        assert!(!started.is_active(&cell.borrow()));
    }

    #[test]
    fn a_running_apps_timer_catches_up_with_one_fire_after_a_stall() {
        let cell = AppCell::new();
        let seen = ticks();
        let sink = Rc::clone(&seen);
        let timer = Timer::periodic(&mut cell.borrow_mut(), Duration::from_millis(1), {
            let seen = Rc::clone(&sink);
            move |app, timer| seen.borrow_mut().push(timer.tick(app))
        });

        cell.elapse_to(Duration::from_secs(100));
        assert_eq!(
            *seen.borrow(),
            [100_000],
            "one fire, its tick jumping over the 100 s of periods"
        );
        assert_eq!(timer.tick(&cell.borrow()), 100_000);

        cell.elapse_to(Duration::from_millis(100_001));
        assert_eq!(*seen.borrow(), [100_000, 100_001]);
    }

    #[test]
    fn a_running_apps_timer_runs_at_the_time_the_app_came_back() {
        let cell = AppCell::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = Rc::clone(&seen);
        Timer::periodic(&mut cell.borrow_mut(), Duration::from_millis(1), {
            move |app, _timer| sink.borrow_mut().push(app.clock())
        });

        cell.elapse_to(Duration::from_secs(100));
        assert_eq!(
            *seen.borrow(),
            [Duration::from_secs(100)],
            "the VM's clock is the real time the event loop reached the timer"
        );
    }

    #[test]
    fn a_running_apps_delay_chain_waits_a_whole_delay_after_a_stall() {
        let cell = AppCell::new();
        let fires = Rc::new(std::cell::Cell::new(0u32));
        let count = Rc::clone(&fires);
        cell.borrow().spawn(async move |cx| {
            loop {
                cx.update(|app| delayed(app, Duration::from_millis(10)))
                    .await;
                count.set(count.get() + 1);
            }
        });
        cell.checkpoint();

        cell.elapse_to(Duration::from_secs(100));
        assert_eq!(
            fires.get(),
            1,
            "the next delay is measured from when the app came back, not replayed"
        );
        cell.elapse_to(Duration::from_millis(100_010));
        assert_eq!(fires.get(), 2);
    }

    #[test]
    fn a_zero_period_timer_fires_once_a_turn_and_never_spins() {
        let cell = AppCell::new();
        let seen = ticks();
        let sink = Rc::clone(&seen);
        Timer::periodic(&mut cell.borrow_mut(), Duration::ZERO, {
            let seen = Rc::clone(&sink);
            move |app, timer| seen.borrow_mut().push(timer.tick(app))
        });

        cell.elapse(Duration::from_millis(100));
        assert_eq!(
            *seen.borrow(),
            [1],
            "one fire in the turn, not 100 ms of them"
        );
        cell.elapse(Duration::ZERO);
        assert_eq!(*seen.borrow(), [1, 2], "the next turn fires once more");
    }

    #[test]
    fn a_delayed_future_completes_when_the_clock_passes_its_delay() {
        let cell = AppCell::new();
        let done = Rc::new(std::cell::Cell::new(false));
        let out = Rc::clone(&done);
        cell.borrow().spawn(async move |cx| {
            cx.update(|app| delayed(app, Duration::from_millis(25)))
                .await;
            out.set(true);
        });
        cell.checkpoint();
        cell.elapse(Duration::from_millis(20));
        assert!(!done.get(), "the delay has not passed");
        cell.elapse(Duration::from_millis(10));
        assert!(
            done.get(),
            "the awaitable delay completed on the app's timers"
        );
    }
}
