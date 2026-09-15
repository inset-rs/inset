//! Work off the main thread whose result comes back as a future for it: dart:isolate's
//! `Isolate.run`, over the host's [`Dispatcher`] and a handoff the two sides share.

use std::any::Any;
use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

use inset_embedder::Dispatcher;

type Outcome<T> = Result<T, Box<dyn Any + Send>>;

/// Hands `work` to `dispatcher` and answers a future for its result. A panic in `work` resumes
/// in whoever awaits, as `Isolate.run` rethrows the isolate's error.
pub(crate) fn run<T: Send + 'static>(
    dispatcher: &Arc<dyn Dispatcher>,
    work: impl FnOnce() -> T + Send + 'static,
) -> BackgroundWork<T> {
    let handoff = Arc::new(Mutex::new(Handoff {
        outcome: None,
        awaiter: None,
    }));
    let worker = Arc::clone(&handoff);
    dispatcher.dispatch(Box::new(move || {
        let outcome = catch_unwind(AssertUnwindSafe(work));
        let awaiter = {
            let mut handoff = lock(&worker);
            handoff.outcome = Some(outcome);
            handoff.awaiter.take()
        };
        if let Some(awaiter) = awaiter {
            awaiter.wake();
        }
    }));
    BackgroundWork { handoff }
}

/// The result of [`run`]'s work, once the dispatcher has run it.
pub(crate) struct BackgroundWork<T> {
    handoff: Arc<Mutex<Handoff<T>>>,
}

/// What the worker thread leaves for the awaiting task, and how it finds that task.
struct Handoff<T> {
    outcome: Option<Outcome<T>>,
    /// The task awaiting the outcome, parked until it lands.
    awaiter: Option<Waker>,
}

impl<T> Future for BackgroundWork<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut handoff = lock(&self.handoff);
        match handoff.outcome.take() {
            Some(Ok(value)) => Poll::Ready(value),
            Some(Err(panic)) => resume_unwind(panic),
            None => {
                handoff.awaiter = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// The handoff is held for a moment on either side and never poisoned by `work`, whose panic is
/// caught outside the lock.
fn lock<T>(handoff: &Mutex<Handoff<T>>) -> MutexGuard<'_, Handoff<T>> {
    handoff.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::task::Wake;
    use std::time::Duration;

    use inset_embedder::{InertDispatcher, Instant};

    use super::*;

    /// A host with a thread per job, so the handoff crosses threads as it does in a program.
    struct ThreadDispatcher;

    impl Dispatcher for ThreadDispatcher {
        fn wake_at(&self, _deadline: Instant) {}

        fn dispatch(&self, work: Box<dyn FnOnce() + Send>) {
            std::thread::spawn(work);
        }
    }

    fn on_a_thread() -> Arc<dyn Dispatcher> {
        Arc::new(ThreadDispatcher)
    }

    /// A waker that reports over a channel, so a test can wait for the worker.
    struct Report(mpsc::Sender<()>);

    impl Wake for Report {
        fn wake(self: Arc<Self>) {
            let _ = self.0.send(());
        }
    }

    fn poll_after_the_worker<T>(mut work: BackgroundWork<T>) -> T {
        let (send, receive) = mpsc::channel();
        let waker = Waker::from(Arc::new(Report(send)));
        let mut cx = Context::from_waker(&waker);
        if let Poll::Ready(value) = Pin::new(&mut work).poll(&mut cx) {
            return value;
        }
        receive
            .recv_timeout(Duration::from_secs(5))
            .expect("the worker wakes the parked task");
        match Pin::new(&mut work).poll(&mut cx) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("the outcome is in the handoff once the worker has woken us"),
        }
    }

    #[test]
    fn the_result_arrives_through_the_parked_waker() {
        assert_eq!(poll_after_the_worker(run(&on_a_thread(), || 40 + 2)), 42);
    }

    #[test]
    fn work_run_at_once_is_ready_at_the_first_poll() {
        let dispatcher: Arc<dyn Dispatcher> = Arc::new(InertDispatcher);
        let mut work = run(&dispatcher, || 40 + 2);
        let mut cx = Context::from_waker(Waker::noop());
        assert_eq!(Pin::new(&mut work).poll(&mut cx), Poll::Ready(42));
    }

    #[test]
    #[should_panic(expected = "boom")]
    fn a_panic_in_the_work_resumes_in_the_awaiting_task() {
        poll_after_the_worker(run(&on_a_thread(), || -> u32 { panic!("boom") }));
    }
}
