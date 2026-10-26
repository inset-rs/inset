//! A future of the app's, waited for on the thread it was made on where AppKit wants its answer
//! before its call returns, as it wants a promised form's.

use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

use crate::os::apple::app_futures::LocalFuture;

/// Waits on this thread for `future`, up to `limit`; none once the limit has passed.
///
/// Nothing else runs on this thread meanwhile, so `future` must be completed from another one,
/// as a program's answer is.
pub(super) fn wait<T>(mut future: LocalFuture<T>, limit: Duration) -> Option<T> {
    let waker = Waker::from(Arc::new(Unpark(thread::current())));
    let mut context = Context::from_waker(&waker);
    let deadline = Instant::now() + limit;
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return Some(output);
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return None;
        }
        thread::park_timeout(left);
    }
}

/// Wakes the thread that waits.
struct Unpark(Thread);

impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// An answer another thread gives, and who waits for it.
    #[derive(Clone, Default)]
    struct Answer(Arc<Mutex<(Option<u32>, Option<Waker>)>>);

    impl Answer {
        fn give(&self, value: u32) {
            let mut slot = self.0.lock().unwrap();
            slot.0 = Some(value);
            if let Some(waiting) = slot.1.take() {
                waiting.wake();
            }
        }

        fn awaited(self) -> LocalFuture<u32> {
            Box::pin(std::future::poll_fn(move |context| {
                let mut slot = self.0.lock().unwrap();
                match slot.0.take() {
                    Some(value) => Poll::Ready(value),
                    None => {
                        slot.1 = Some(context.waker().clone());
                        Poll::Pending
                    }
                }
            }))
        }
    }

    #[test]
    fn a_wait_ends_with_the_answer_another_thread_gives() {
        let answer = Answer::default();
        let giver = answer.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            giver.give(7);
        });
        assert_eq!(wait(answer.awaited(), Duration::from_secs(5)), Some(7));
    }

    #[test]
    fn a_wait_gives_up_at_its_limit() {
        let never: LocalFuture<()> = Box::pin(std::future::pending());
        assert_eq!(wait(never, Duration::from_millis(20)), None);
    }
}
