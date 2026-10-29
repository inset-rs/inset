//! An answer the system gives once, on the Java main thread or another, and the future the app
//! awaits it through: a picker's documents, or the user's answer to a permission. The invariant:
//! the future is answered exactly once, with what the system gave, or with what its giver set for
//! an answer dropped unused, so the app never waits for ever.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

/// The answer, on its way from the system to the app.
struct Shared<T> {
    answered: Option<T>,
    waiter: Option<Waker>,
}

/// The system's side: given once, from any thread, or the dropped answer if dropped unused.
pub(crate) struct Answer<T> {
    shared: Arc<Mutex<Shared<T>>>,
    if_dropped: Option<T>,
}

/// The app's side: the future that resolves to the answer.
pub(crate) struct Answered<T>(Arc<Mutex<Shared<T>>>);

/// An answer and the future awaiting it; `if_dropped` answers the future if the answer is dropped
/// without being given.
pub(crate) fn answer<T>(if_dropped: T) -> (Answer<T>, Answered<T>) {
    let shared = Arc::new(Mutex::new(Shared {
        answered: None,
        waiter: None,
    }));
    (
        Answer {
            shared: Arc::clone(&shared),
            if_dropped: Some(if_dropped),
        },
        Answered(shared),
    )
}

impl<T> Answer<T> {
    pub(crate) fn give(mut self, answer: T) {
        self.if_dropped = None;
        give(&self.shared, answer);
    }
}

impl<T> Drop for Answer<T> {
    fn drop(&mut self) {
        if let Some(answer) = self.if_dropped.take() {
            give(&self.shared, answer);
        }
    }
}

/// Answers `answer` unless an answer was given already, and wakes the app's task.
fn give<T>(shared: &Mutex<Shared<T>>, answer: T) {
    let waiter = {
        let mut shared = locked(shared);
        if shared.answered.is_some() {
            return;
        }
        shared.answered = Some(answer);
        shared.waiter.take()
    };
    if let Some(waiter) = waiter {
        waiter.wake();
    }
}

impl<T> Future for Answered<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut shared = locked(&self.0);
        match shared.answered.take() {
            Some(answer) => Poll::Ready(answer),
            None => {
                shared.waiter = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

fn locked<T>(shared: &Mutex<T>) -> MutexGuard<'_, T> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_answer_is_given_once_and_a_dropped_one_gives_its_stand_in() {
        let mut context = Context::from_waker(Waker::noop());
        let (given, mut answered) = answer(0);
        given.give(7);
        assert_eq!(Pin::new(&mut answered).poll(&mut context), Poll::Ready(7));
        let (dropped, mut answered) = answer(0);
        drop(dropped);
        assert_eq!(Pin::new(&mut answered).poll(&mut context), Poll::Ready(0));
    }
}
