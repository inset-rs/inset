//! The cell the [`App`] lives in, and a task's view of it. Not a Dart file.
//!
//! Dart's isolate owns its heap and runs its microtask queue between events. Here the heap is
//! [`App`], the [`AppCell`] is what the shell and the tests hold, and a task reaches the heap
//! through [`AsyncApp::update`], borrowing the cell for one closure at a time — gpui's
//! `AppCell` and `AsyncApp`. Microtasks and task continuations run only through
//! [`AppCell::checkpoint`], with nothing borrowed.

use std::cell::{Ref, RefCell, RefMut};
use std::rc::{Rc, Weak};
use std::time::Duration;

use inset_embedder::{InertPlatform, PlatformRef};

use crate::app::App;
use crate::executor::{ExecutorHandle, ForegroundExecutor};
use crate::timers::Timers;

/// Round budget for one checkpoint: a microtask and a task feeding each other forever must
/// fail loud, not starve the event loop; each round's own drain has its own budget.
const CHECKPOINT_BUDGET: usize = 100_000;

/// Owns the [`App`] and the one door through which its microtasks and tasks run.
pub struct AppCell {
    app: RefCell<App>,
}

impl AppCell {
    /// An `App` on an inert platform, for tests.
    pub fn new() -> Rc<AppCell> {
        AppCell::with_platform(Rc::new(InertPlatform))
    }

    /// The `App` of a running program, on the host's platform.
    pub fn with_platform(platform: PlatformRef) -> Rc<AppCell> {
        Rc::new_cyclic(|this| AppCell {
            app: RefCell::new(App::build(
                this.clone(),
                platform,
                ForegroundExecutor::new(),
            )),
        })
    }

    /// # Panics
    ///
    /// While [`borrow_mut`](Self::borrow_mut) is held.
    pub fn borrow(&self) -> Ref<'_, App> {
        self.app.borrow()
    }

    /// The `App` for one platform event or one test step. Release it before
    /// [`checkpoint`](Self::checkpoint) or [`elapse`](Self::elapse).
    ///
    /// # Panics
    ///
    /// While another borrow is held.
    pub fn borrow_mut(&self) -> RefMut<'_, App> {
        self.app.borrow_mut()
    }

    /// The microtask checkpoint — the HTML event loop's "perform a microtask checkpoint",
    /// V8's `PerformMicrotaskCheckpoint`, the point where Dart drains its microtask queue:
    /// runs every queued microtask and polls every ready task, including the ones they
    /// queue, until nothing is left. Returns the number of rounds.
    /// The shell runs it at the end of every platform event and [`elapse`](Self::elapse)
    /// around every timer; a test runs it where it would pump.
    ///
    /// # Panics
    ///
    /// While the `App` is borrowed — a task borrows it itself, one closure at a time — and
    /// after `CHECKPOINT_BUDGET` rounds, on the assumption of a cycle.
    pub fn checkpoint(&self) -> usize {
        assert!(
            self.app.try_borrow_mut().is_ok(),
            "AppCell::checkpoint while the App is borrowed: release it first; each task \
             borrows the App for its own steps"
        );
        let executor = self.borrow().executor_handle();
        let mut rounds = 0usize;
        loop {
            self.borrow_mut().release_dropped_retained_handles();
            self.borrow_mut().flush_entity_effects();
            let had_microtasks = self.borrow().has_pending_microtasks();
            if had_microtasks {
                self.borrow_mut().drain_microtasks();
            }
            let polled = executor.drain();
            if !had_microtasks && polled == 0 {
                break;
            }
            rounds += 1;
            assert!(
                rounds <= CHECKPOINT_BUDGET,
                "the checkpoint did not converge after {CHECKPOINT_BUDGET} rounds; are a \
                 microtask and a task queueing each other in a cycle?"
            );
        }
        rounds
    }

    /// A host wake that carried no time: runs the checkpoint, for whatever a native callback
    /// posted, and re-arms the host for the next timer, since the post's own wake took the
    /// host's deadline.
    ///
    /// # Panics
    ///
    /// While the `App` is borrowed.
    pub fn wake(&self) {
        self.checkpoint();
        self.borrow_mut().request_wake_for_next_timer();
    }

    /// Advances the App clock by `duration`, firing the timers that come due.
    ///
    /// A [`checkpoint`](Self::checkpoint) first, then each due timer in due order with a
    /// checkpoint after it, so a future awaiting one timer can schedule the next within the
    /// same call — Dart's event-loop position for `Timer`, and FakeAsync's `elapse` for
    /// tests. The shell calls this as platform time passes.
    ///
    /// # Panics
    ///
    /// While the `App` is borrowed.
    pub fn elapse(&self, duration: Duration) {
        let target = self.borrow().clock() + duration;
        self.checkpoint();
        let mut fired = 0usize;
        loop {
            let fired_one = self.borrow_mut().fire_next_due(target);
            if !fired_one {
                break;
            }
            fired += 1;
            Timers::assert_fire_budget(fired);
            self.checkpoint();
        }
        self.borrow_mut().advance_clock_to(target);
        self.borrow_mut().request_wake_for_next_timer();
    }
}

/// A task's handle on the [`App`]: what an `async` continuation captures across its `await`s.
///
/// Carries the executor's queue and the host besides the cell, as gpui's `AsyncApp` carries
/// its executors: a [`post`](Self::post) reaches both without the `App`.
#[derive(Clone)]
pub struct AsyncApp {
    app: Weak<AppCell>,
    executor: ExecutorHandle,
    platform: PlatformRef,
}

impl AsyncApp {
    pub(crate) fn new(
        app: Weak<AppCell>,
        executor: ExecutorHandle,
        platform: PlatformRef,
    ) -> AsyncApp {
        AsyncApp {
            app,
            executor,
            platform,
        }
    }

    /// Runs `f` on the `App`, borrowing the cell for just that closure.
    ///
    /// # Panics
    ///
    /// If the `App` has been dropped, or is borrowed — which means a checkpoint ran from
    /// inside App code instead of from the cell.
    pub fn update<R>(&self, f: impl FnOnce(&mut App) -> R) -> R {
        let cell = self
            .app
            .upgrade()
            .expect("the App was dropped while one of its tasks still runs");
        let mut app = cell.app.try_borrow_mut().unwrap_or_else(|_| {
            panic!(
                "AsyncApp::update while the App is borrowed: tasks run only from \
                 AppCell::checkpoint, never from inside App code"
            )
        });
        f(&mut app)
    }

    /// Queues `f` to run on the `App` at the next checkpoint and wakes the host so one comes.
    ///
    /// The door for a native callback — a notification, an observer — that the host delivers
    /// outside any event it drives, possibly while the `App` is borrowed: nothing here touches
    /// the `App`, so it never panics where [`update`](Self::update) would. Dart's counterpart
    /// is a native port message, which lands in the isolate's event queue. After the `App` is
    /// dropped the closure is discarded.
    pub fn post(&self, f: impl FnOnce(&mut App) + 'static) {
        if self.app.strong_count() == 0 {
            return;
        }
        let cx = self.clone();
        // Dropping the handle detaches the task; it runs to completion regardless.
        drop(self.executor.spawn(async move { cx.update(f) }));
        self.platform.wake_at(self.platform.now());
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::time::Duration;

    use super::*;
    use crate::change_notifier::Listener;
    use crate::completer::{Completer, CompleterFuture};
    use crate::timers::Timer;

    fn counter() -> Rc<Cell<u32>> {
        Rc::new(Cell::new(0))
    }

    fn log() -> Rc<RefCell<Vec<&'static str>>> {
        Rc::new(RefCell::new(Vec::new()))
    }

    #[test]
    fn a_post_runs_at_the_next_checkpoint_even_when_made_while_borrowed() {
        let cell = AppCell::new();
        let ran = counter();
        let cx = cell.borrow().to_async();
        {
            let _borrowed = cell.borrow_mut();
            let ran = Rc::clone(&ran);
            cx.post(move |_app| ran.set(ran.get() + 1));
        }
        assert_eq!(ran.get(), 0, "a post never runs inline");
        cell.checkpoint();
        assert_eq!(ran.get(), 1);
        cell.checkpoint();
        assert_eq!(ran.get(), 1, "a post runs once");
    }

    #[test]
    fn a_post_after_the_app_is_gone_is_dropped() {
        let cx = AppCell::new().borrow().to_async();
        cx.post(|_app| panic!("the App is gone"));
    }

    #[test]
    fn a_wake_runs_the_checkpoint() {
        let cell = AppCell::new();
        let ran = counter();
        let cx = cell.borrow().to_async();
        {
            let ran = Rc::clone(&ran);
            cx.post(move |_app| ran.set(ran.get() + 1));
        }
        cell.wake();
        assert_eq!(ran.get(), 1);
    }

    #[test]
    fn a_spawned_continuation_runs_at_the_checkpoint_not_inline() {
        let cell = AppCell::new();
        let ran = counter();
        {
            let seen = ran.clone();
            cell.borrow()
                .spawn(async move |_cx| seen.set(seen.get() + 1));
            assert_eq!(ran.get(), 0, "spawn never polls inline");
        }
        cell.checkpoint();
        assert_eq!(ran.get(), 1);
    }

    #[test]
    fn a_task_reaches_the_app_through_update() {
        let cell = AppCell::new();
        let handle = cell.borrow_mut().create(3u32);
        cell.borrow().spawn(async move |cx| {
            cx.update(|app| *app.get_mut(handle) += 4);
        });
        cell.checkpoint();
        assert_eq!(*cell.borrow().get(handle), 7);
    }

    #[test]
    fn awaiting_a_completer_resumes_at_the_checkpoint_after_completion() {
        let cell = AppCell::new();
        let completer: Completer<u32> = Completer::new();
        let future = completer.future();
        let seen = counter();
        let out = seen.clone();
        cell.borrow().spawn(async move |_cx| out.set(future.await));
        cell.checkpoint();
        assert_eq!(seen.get(), 0, "pending: the waker is parked");

        completer.complete(&mut cell.borrow_mut(), 7);
        assert_eq!(seen.get(), 0, "completion wakes; it does not poll inline");
        cell.checkpoint();
        assert_eq!(seen.get(), 7);
    }

    #[test]
    fn a_completer_resolves_every_listener() {
        let cell = AppCell::new();
        let completer: Completer<u32> = Completer::new();
        let seen = counter();
        for _ in 0..3 {
            let future = completer.future();
            let out = seen.clone();
            cell.borrow().spawn(async move |_cx| {
                let value = future.await;
                out.set(out.get() + value);
            });
        }
        cell.checkpoint();
        completer.complete(&mut cell.borrow_mut(), 5);
        cell.checkpoint();
        assert_eq!(seen.get(), 15);
    }

    #[test]
    fn a_task_awaited_as_a_task_yields_its_result() {
        let cell = AppCell::new();
        let seen = counter();
        let out = seen.clone();
        let inner = cell.borrow().spawn(async move |_cx| 40u32);
        cell.borrow()
            .spawn(async move |_cx| out.set(inner.await + 2));
        cell.checkpoint();
        assert_eq!(seen.get(), 42);
    }

    /// Dart never ends a checkpoint with work queued: what a continuation queues runs in the
    /// same checkpoint.
    #[test]
    fn a_checkpoint_runs_what_its_continuations_queue() {
        let cell = AppCell::new();
        let events = log();
        let completer: Completer<()> = Completer::new();
        let future = completer.future();
        let seen = events.clone();
        cell.borrow().spawn(async move |cx| {
            future.await;
            seen.borrow_mut().push("task");
            let seen = seen.clone();
            cx.update(|app| {
                app.schedule_microtask(Listener::new(move |_app| {
                    seen.borrow_mut().push("microtask from the task")
                }))
            });
        });
        cell.checkpoint();

        completer.complete(&mut cell.borrow_mut(), ());
        cell.checkpoint();
        assert_eq!(*events.borrow(), ["task", "microtask from the task"]);
        assert!(!cell.borrow().has_pending_microtasks());
    }

    /// A timer as a future: what `Future.delayed` awaits.
    fn delayed(app: &mut App, duration: Duration) -> CompleterFuture<()> {
        let completer: Completer<()> = Completer::new();
        let future = completer.future();
        Timer::new(
            app,
            duration,
            Listener::new(move |app| completer.complete(app, ())),
        );
        future
    }

    #[test]
    fn elapse_lets_a_task_chain_timers_within_one_call() {
        let cell = AppCell::new();
        let seen = counter();
        let out = seen.clone();
        cell.borrow().spawn(async move |cx| {
            let first = cx.update(|app| delayed(app, Duration::from_millis(10)));
            first.await;
            out.set(1);
            let second = cx.update(|app| delayed(app, Duration::from_millis(10)));
            second.await;
            out.set(2);
        });
        cell.checkpoint();
        cell.elapse(Duration::from_millis(15));
        assert_eq!(
            seen.get(),
            1,
            "the first timer fired, the second is still due"
        );
        cell.elapse(Duration::from_millis(10));
        assert_eq!(
            seen.get(),
            2,
            "the second timer was scheduled by the resumed task"
        );
    }

    #[test]
    fn dropping_the_cell_releases_queued_and_pending_tasks() {
        let cell = AppCell::new();
        let token = Rc::new(());
        let queued = token.clone();
        cell.borrow().spawn(async move |_cx| drop(queued));
        let completer = cell.borrow_mut().create(Completer::<()>::new());
        let future = cell.borrow().get(completer).future();
        let pending = token.clone();
        cell.borrow().spawn(async move |_cx| {
            future.await;
            drop(pending);
        });
        assert_eq!(Rc::strong_count(&token), 3);

        drop(cell);
        assert_eq!(
            Rc::strong_count(&token),
            1,
            "the queued and the pending task's futures were dropped with the App"
        );
    }

    #[test]
    #[should_panic(expected = "AppCell::checkpoint while the App is borrowed")]
    fn a_checkpoint_while_borrowed_panics() {
        let cell = AppCell::new();
        let _app = cell.borrow_mut();
        cell.checkpoint();
    }

    #[test]
    fn then_runs_on_the_microtask_queue_after_completion() {
        let cell = AppCell::new();
        let completer: Completer<u32> = Completer::new();
        let seen = counter();
        {
            let mut app = cell.borrow_mut();
            let out = seen.clone();
            completer
                .future()
                .then(&mut app, move |_app, value| out.set(value));
            completer.complete(&mut app, 9);
            assert_eq!(seen.get(), 0, "a listener never runs inline");
            app.drain_microtasks();
            assert_eq!(seen.get(), 9, "a listener is a microtask");
        }
        let later = counter();
        {
            let mut app = cell.borrow_mut();
            let out = later.clone();
            completer
                .future()
                .then(&mut app, move |_app, value| out.set(value));
            assert_eq!(later.get(), 0, "even on a resolved future");
        }
        cell.checkpoint();
        assert_eq!(later.get(), 9);
    }
}
