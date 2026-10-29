//! Players and cameras as the framework's (`inset_embedder`'s `media`): what each system plays and
//! captures with is `os/`'s; here is what the systems share: telling the app that a player's or a
//! session's state changed, from whatever thread the system says so on, and answering a photo or
//! a recording once.
//!
//! The invariant: a change future resolves at every change after it was made and once its player
//! or session is gone, and an answer is given exactly once, a failure when the system drops it, so
//! the app never waits on either for ever.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

use inset_embedder::{MediaChangeFuture, MediaError};

/// The changes of one player's or session's state, told from any thread.
#[derive(Clone, Default)]
pub(crate) struct Changes(Arc<Mutex<ChangesState>>);

#[derive(Default)]
struct ChangesState {
    /// How many changes there were.
    count: u64,
    /// Whether the player or session is gone.
    ended: bool,
    waiters: Vec<Waker>,
}

impl Changes {
    /// Resolves at the next change, or once the changes end.
    pub(crate) fn next(&self) -> MediaChangeFuture {
        let seen = lock(&self.0).count;
        Box::pin(NextChange {
            changes: self.clone(),
            seen,
        })
    }

    /// The state changed.
    pub(crate) fn notify(&self) {
        let waiters = {
            let mut state = lock(&self.0);
            state.count += 1;
            std::mem::take(&mut state.waiters)
        };
        waiters.into_iter().for_each(Waker::wake);
    }

    /// The player or session is gone: every future resolves.
    pub(crate) fn end(&self) {
        let waiters = {
            let mut state = lock(&self.0);
            state.ended = true;
            std::mem::take(&mut state.waiters)
        };
        waiters.into_iter().for_each(Waker::wake);
    }
}

struct NextChange {
    changes: Changes,
    seen: u64,
}

impl Future for NextChange {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = lock(&self.changes.0);
        if state.ended || state.count != self.seen {
            return Poll::Ready(());
        }
        state.waiters.push(cx.waker().clone());
        Poll::Pending
    }
}

/// The system's side of an answer: given once, from any thread, or a failure if dropped unused.
pub(crate) struct MediaAnswer<T>(Arc<Mutex<Answer<T>>>);

struct Answer<T> {
    answered: Option<Result<T, MediaError>>,
    waiter: Option<Waker>,
}

/// What an answer's future resolves to.
pub(crate) type MediaAnswered<T> = Pin<Box<dyn Future<Output = Result<T, MediaError>>>>;

/// An answer: what the system gives, and the future the app awaits.
pub(crate) fn media_answer<T: Send + 'static>() -> (MediaAnswer<T>, MediaAnswered<T>) {
    let shared = Arc::new(Mutex::new(Answer {
        answered: None,
        waiter: None,
    }));
    (MediaAnswer(Arc::clone(&shared)), Box::pin(Answered(shared)))
}

impl<T> MediaAnswer<T> {
    pub(crate) fn give(self, answer: Result<T, MediaError>) {
        give(&self.0, answer);
    }
}

impl<T> Drop for MediaAnswer<T> {
    fn drop(&mut self) {
        give(
            &self.0,
            Err(MediaError::Failed(
                "the system ended without an answer".to_owned(),
            )),
        );
    }
}

/// Answers `answer` unless an answer was given already, and wakes the app's task.
fn give<T>(shared: &Mutex<Answer<T>>, answer: Result<T, MediaError>) {
    let waiter = {
        let mut shared = lock(shared);
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

struct Answered<T>(Arc<Mutex<Answer<T>>>);

impl<T> Future for Answered<T> {
    type Output = Result<T, MediaError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared = lock(&self.0);
        match shared.answered.take() {
            Some(answer) => Poll::Ready(answer),
            None => {
                shared.waiter = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::task::Waker;

    use super::*;

    fn poll<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
        future.poll(&mut Context::from_waker(Waker::noop()))
    }

    #[test]
    fn a_change_future_resolves_at_the_next_change_and_when_the_changes_end() {
        let changes = Changes::default();
        let mut before = changes.next();
        assert!(poll(before.as_mut()).is_pending());
        changes.notify();
        assert!(poll(before.as_mut()).is_ready());

        let mut after = changes.next();
        assert!(
            poll(after.as_mut()).is_pending(),
            "a change before the future was made is not one after it"
        );
        changes.end();
        assert!(poll(after.as_mut()).is_ready());
    }

    #[test]
    fn an_answer_dropped_unanswered_is_a_failure() {
        let (answer, mut answered) = media_answer::<u8>();
        assert!(poll(answered.as_mut()).is_pending());
        drop(answer);
        assert!(matches!(
            poll(answered.as_mut()),
            Poll::Ready(Err(MediaError::Failed(_)))
        ));

        let (answer, mut answered) = media_answer::<u8>();
        answer.give(Ok(7));
        assert!(matches!(poll(answered.as_mut()), Poll::Ready(Ok(7))));
    }
}
