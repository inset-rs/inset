//! Telling the app that the cameras' list or a session's state changed, from whatever thread the
//! system says so on: a future the app awaits on its own thread, resolved by the next change.
//!
//! The invariant: a change future resolves at every change after it was made and once the session
//! is gone, so the app never waits on one for ever.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

use waft_embedder::MediaChangeFuture;

/// The changes of the cameras' list or of one session's state, told from any thread.
#[derive(Clone, Default)]
pub(super) struct Changes(Arc<Mutex<ChangesState>>);

#[derive(Default)]
struct ChangesState {
    /// How many changes there were.
    count: u64,
    /// Whether the session is gone.
    ended: bool,
    waiters: Vec<Waker>,
}

impl Changes {
    /// Resolves at the next change, or once the changes end.
    pub(super) fn next(&self) -> MediaChangeFuture {
        let seen = self.state().count;
        Box::pin(NextChange {
            changes: self.clone(),
            seen,
        })
    }

    /// What it reads changed.
    pub(super) fn notify(&self) {
        let waiters = {
            let mut state = self.state();
            state.count += 1;
            std::mem::take(&mut state.waiters)
        };
        waiters.into_iter().for_each(Waker::wake);
    }

    /// The session is gone: every future resolves.
    pub(super) fn end(&self) {
        let waiters = {
            let mut state = self.state();
            state.ended = true;
            std::mem::take(&mut state.waiters)
        };
        waiters.into_iter().for_each(Waker::wake);
    }

    /// Whether `other` tells the same changes.
    pub(super) fn is(&self, other: &Changes) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    fn state(&self) -> MutexGuard<'_, ChangesState> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

struct NextChange {
    changes: Changes,
    seen: u64,
}

impl Future for NextChange {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = self.changes.state();
        if state.ended || state.count != self.seen {
            return Poll::Ready(());
        }
        state.waiters.push(cx.waker().clone());
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
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
}
