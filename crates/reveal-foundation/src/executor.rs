//! The foreground executor: `!Send` futures polled on the main thread at the microtask
//! checkpoint. Dart's counterpart is the isolate's microtask queue, where `await`
//! continuations run — one queue with `scheduleMicrotask`'s callbacks, in scheduling order.
//!
//! The shape is gpui's `ForegroundExecutor` over `async-task`. A woken task's runnable lands on
//! a ready queue, and [`AppCell::checkpoint`](crate::AppCell::checkpoint) polls the queue with
//! no borrow of the [`App`](crate::App) held, so each poll can borrow it through its own
//! [`AsyncApp`](crate::AsyncApp).

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll};

use async_task::Runnable;

/// Poll budget for one drain: two tasks waking each other forever must fail loud, not starve
/// the event loop — the counterpart of the microtask budget.
const DRAIN_BUDGET: usize = 100_000;

/// Woken tasks waiting for the checkpoint. A `Waker` may be called from any thread, so the
/// queue is shared; the runnables themselves only ever run on the main thread.
type ReadyQueue = Arc<Mutex<VecDeque<Runnable>>>;

/// Spawns `!Send` futures for the main thread. Lives on the `App`; the cell polls through an
/// [`ExecutorHandle`].
pub(crate) struct ForegroundExecutor {
    ready: ReadyQueue,
}

impl ForegroundExecutor {
    pub(crate) fn new() -> ForegroundExecutor {
        ForegroundExecutor {
            ready: Arc::default(),
        }
    }

    /// Queues `future` for its first poll at the next checkpoint — never inline, since the
    /// caller holds the `App` the future will borrow.
    pub(crate) fn spawn<R: 'static>(&self, future: impl Future<Output = R> + 'static) -> Task<R> {
        let ready = Arc::clone(&self.ready);
        let schedule = move |runnable| lock(&ready).push_back(runnable);
        let (runnable, task) = async_task::spawn_local(future, schedule);
        runnable.schedule();
        Task::spawned(task)
    }

    pub(crate) fn handle(&self) -> ExecutorHandle {
        ExecutorHandle {
            ready: Arc::clone(&self.ready),
        }
    }
}

/// The `App` going away cancels the tasks still queued. A queued runnable and the queue own
/// each other through the task's schedule closure, so the queue is emptied by hand; dropping
/// a runnable can wake further tasks, hence the loop.
impl Drop for ForegroundExecutor {
    fn drop(&mut self) {
        loop {
            let queued: Vec<_> = lock(&self.ready).drain(..).collect();
            if queued.is_empty() {
                break;
            }
            drop(queued);
        }
    }
}

/// The polling side of a [`ForegroundExecutor`], held by the `AppCell`.
pub(crate) struct ExecutorHandle {
    ready: ReadyQueue,
}

impl ExecutorHandle {
    /// Polls ready tasks until none is left, including the tasks they wake, and returns how
    /// many polls ran.
    ///
    /// # Panics
    ///
    /// After `DRAIN_BUDGET` polls in one drain, on the assumption that two tasks are waking
    /// each other in a cycle.
    pub(crate) fn drain(&self) -> usize {
        let mut polled = 0usize;
        loop {
            let next = lock(&self.ready).pop_front();
            let Some(runnable) = next else {
                break;
            };
            polled += 1;
            assert!(
                polled <= DRAIN_BUDGET,
                "tasks did not converge after {DRAIN_BUDGET} polls; are two tasks waking each \
                 other in a cycle?"
            );
            runnable.run();
        }
        polled
    }
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
