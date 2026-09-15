//! The foreground executor: `!Send` futures polled on the main thread at the microtask
//! checkpoint. Dart's counterpart is the isolate's microtask queue, where `await`
//! continuations run — one queue with `scheduleMicrotask`'s callbacks, in scheduling order.
//!
//! The shape is gpui's `ForegroundExecutor` over `async-task`: a woken task's runnable lands on
//! the ready queue, and [`AppCell::checkpoint`](crate::AppCell::checkpoint) polls the queue
//! with no borrow of the [`App`](crate::App) held, so each poll can borrow it through its own
//! [`AsyncApp`](crate::AsyncApp). Waking also asks the host for a turn, since the task may
//! have been woken on another thread while the main thread sleeps in the host's wait — what
//! gpui's dispatch to the main queue does for every runnable. Work for another thread goes
//! to the same host dispatcher and comes back as a task.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll};

use async_task::Runnable;
use inset_embedder::{Dispatcher, Instant};

use crate::background;

/// Poll budget for one checkpoint's run: two tasks waking each other forever must fail loud,
/// not starve the event loop — the counterpart of the microtask budget.
const POLL_BUDGET: usize = 100_000;

/// Tasks woken and not yet polled. A `Waker` may be called from any thread, so the queue is
/// shared; the runnables themselves only ever run on the main thread.
type ReadyQueue = Arc<Mutex<VecDeque<Runnable>>>;

/// Spawns `!Send` futures for the main thread. Lives on the `App`, and owns the queue: the
/// `App` going away cancels the tasks still queued. Everyone else — the cell's checkpoint,
/// every `AsyncApp` — holds an [`ExecutorHandle`] onto the same queue.
pub(crate) struct ForegroundExecutor {
    shared: ExecutorHandle,
}

impl ForegroundExecutor {
    /// An executor whose woken tasks ask `dispatcher` for a turn, and whose background work
    /// runs on it.
    pub(crate) fn new(dispatcher: Arc<dyn Dispatcher>) -> ForegroundExecutor {
        ForegroundExecutor {
            shared: ExecutorHandle {
                ready: Arc::default(),
                dispatcher,
            },
        }
    }

    /// Queues `future` for its first poll at the next checkpoint — never inline, since the
    /// caller holds the `App` the future will borrow.
    pub(crate) fn spawn<R: 'static>(&self, future: impl Future<Output = R> + 'static) -> Task<R> {
        self.shared.spawn(future)
    }

    /// [`ExecutorHandle::run_in_background`].
    pub(crate) fn run_in_background<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> Task<T> {
        self.shared.run_in_background(work)
    }

    pub(crate) fn handle(&self) -> ExecutorHandle {
        self.shared.clone()
    }
}

/// A queued runnable and the queue own each other through the task's schedule closure, so the
/// queue is emptied by hand; dropping a runnable can wake further tasks, hence the loop.
impl Drop for ForegroundExecutor {
    fn drop(&mut self) {
        loop {
            let queued: Vec<_> = lock(&self.shared.ready).drain(..).collect();
            if queued.is_empty() {
                break;
            }
            drop(queued);
        }
    }
}

/// A [`ForegroundExecutor`] without its ownership: the cell's checkpoint polls through one,
/// and every `AsyncApp` carries one to spawn with.
#[derive(Clone)]
pub(crate) struct ExecutorHandle {
    ready: ReadyQueue,
    dispatcher: Arc<dyn Dispatcher>,
}

impl ExecutorHandle {
    /// Queues `future` from outside the `App`: the door for a host callback that arrives while
    /// the `App` may be borrowed, since the queue is all this touches.
    pub(crate) fn spawn<R: 'static>(&self, future: impl Future<Output = R> + 'static) -> Task<R> {
        spawn_on(&self.ready, &self.dispatcher, future)
    }

    /// Hands `work` to the host to run off the main thread and answers its result as a task:
    /// dart:isolate's `Isolate.run`.
    pub(crate) fn run_in_background<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> Task<T> {
        self.spawn(background::run(&self.dispatcher, work))
    }

    /// Polls the ready tasks until none is left, including the tasks they wake, and returns
    /// how many polls ran.
    ///
    /// # Panics
    ///
    /// After `POLL_BUDGET` polls, on the assumption that two tasks are waking each other in a
    /// cycle.
    pub(crate) fn poll_ready_tasks(&self) -> usize {
        let mut polled = 0usize;
        loop {
            let next = lock(&self.ready).pop_front();
            let Some(runnable) = next else {
                break;
            };
            polled += 1;
            assert!(
                polled <= POLL_BUDGET,
                "tasks did not converge after {POLL_BUDGET} polls; are two tasks waking each \
                 other in a cycle?"
            );
            runnable.run();
        }
        polled
    }
}

fn spawn_on<R: 'static>(
    ready: &ReadyQueue,
    dispatcher: &Arc<dyn Dispatcher>,
    future: impl Future<Output = R> + 'static,
) -> Task<R> {
    let ready = Arc::clone(ready);
    let host = Arc::clone(dispatcher);
    // Runs on whichever thread woke the task: the task joins the queue, and the host is asked
    // for a turn now, in case the main thread is asleep. A host keeps the earliest deadline
    // asked for and serves a burst of asks with one turn, so a task woken during a checkpoint
    // costs at most one turn with nothing to poll.
    let schedule = move |runnable| {
        lock(&ready).push_back(runnable);
        host.wake_at(Instant::now());
    };
    let (runnable, task) = async_task::spawn_local(future, schedule);
    runnable.schedule();
    Task::spawned(task)
}

fn lock(ready: &ReadyQueue) -> MutexGuard<'_, VecDeque<Runnable>> {
    ready
        .lock()
        .expect("the ready queue is never poisoned: nothing runs under its lock")
}

/// A spawned continuation's result, a `Future` itself.
///
/// Dart's futures always run to completion; dropping this handle likewise lets the task
/// finish unobserved, where gpui's `Task` would cancel it.
pub struct Task<T> {
    inner: TaskInner<T>,
}

enum TaskInner<T> {
    /// A value known now: what an `async` body that never awaits hands back.
    Ready(Option<T>),
    Spawned(Option<async_task::Task<T>>),
}

impl<T> Task<T> {
    /// A task that resolves with `value` at once: Dart's `Future.value`, for an async callback
    /// whose body had nothing to await.
    pub fn ready(value: T) -> Task<T> {
        Task {
            inner: TaskInner::Ready(Some(value)),
        }
    }

    pub(crate) fn spawned(task: async_task::Task<T>) -> Task<T> {
        Task {
            inner: TaskInner::Spawned(Some(task)),
        }
    }
}

// Sound: `poll` never projects a pin into the payload — `Ready` is taken by value and
// `async_task::Task` is itself `Unpin`.
impl<T> Unpin for Task<T> {}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        match &mut self.get_mut().inner {
            TaskInner::Ready(value) => Poll::Ready(
                value
                    .take()
                    .expect("a Task is polled until it completes, then never again"),
            ),
            TaskInner::Spawned(task) => {
                let task = task
                    .as_mut()
                    .expect("a Task is polled until it completes, then never again");
                Pin::new(task).poll(cx)
            }
        }
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        if let TaskInner::Spawned(task) = &mut self.inner
            && let Some(task) = task.take()
        {
            task.detach();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::future::poll_fn;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::task::Waker;

    use super::*;

    /// A host that counts the turns asked of it and runs work at once.
    struct CountingDispatcher(AtomicUsize);

    impl Dispatcher for CountingDispatcher {
        fn wake_at(&self, _deadline: Instant) {
            self.0.fetch_add(1, Ordering::AcqRel);
        }

        fn dispatch(&self, work: Box<dyn FnOnce() + Send>) {
            work();
        }
    }

    fn executor() -> (ForegroundExecutor, Arc<CountingDispatcher>) {
        let host = Arc::new(CountingDispatcher(AtomicUsize::new(0)));
        let executor = ForegroundExecutor::new(Arc::clone(&host) as Arc<dyn Dispatcher>);
        (executor, host)
    }

    fn turns_asked(host: &CountingDispatcher) -> usize {
        host.0.load(Ordering::Acquire)
    }

    #[test]
    fn a_spawn_asks_the_host_for_a_turn_and_the_task_runs_in_it() {
        let (executor, host) = executor();
        drop(executor.spawn(async {}));
        assert_eq!(turns_asked(&host), 1);
        assert_eq!(executor.handle().poll_ready_tasks(), 1);
    }

    #[test]
    fn a_task_woken_from_another_thread_asks_for_a_turn_and_runs_in_it() {
        let (executor, host) = executor();
        let parked: Arc<Mutex<Option<Waker>>> = Arc::default();
        let done = Arc::new(AtomicBool::new(false));
        let ran = Rc::new(Cell::new(false));
        let (slot, flag, seen) = (Arc::clone(&parked), Arc::clone(&done), Rc::clone(&ran));
        drop(executor.spawn(async move {
            poll_fn(|cx| {
                if flag.load(Ordering::Acquire) {
                    Poll::Ready(())
                } else {
                    *slot.lock().unwrap() = Some(cx.waker().clone());
                    Poll::Pending
                }
            })
            .await;
            seen.set(true);
        }));
        let handle = executor.handle();
        handle.poll_ready_tasks();
        assert_eq!(
            turns_asked(&host),
            1,
            "the spawn asked for a turn, which parked the task"
        );

        std::thread::spawn(move || {
            done.store(true, Ordering::Release);
            let waker = parked.lock().unwrap().take();
            waker.expect("the turn parked the waker").wake();
        })
        .join()
        .unwrap();
        assert_eq!(
            turns_asked(&host),
            2,
            "the worker's wake asks for another turn"
        );
        assert!(!ran.get(), "nothing runs outside the main thread's turn");
        handle.poll_ready_tasks();
        assert!(ran.get());
    }
}
