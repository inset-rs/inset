//! Flutter counterpart: `dart:async` `Completer`, and the `Future` it completes.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

struct Shared<T> {
    value: Option<T>,
    wakers: Vec<Waker>,
}

/// A way to produce `Future` objects and to complete them later with a value.
///
/// The resolving half; [`future`](Self::future) hands out the listening half any number of
/// times. A completer dropped before [`complete`](Self::complete) leaves its futures pending
/// forever, as a Dart future nobody completes.
pub struct Completer<T> {
    shared: Rc<RefCell<Shared<T>>>,
}

impl<T> Completer<T> {
    pub fn new() -> Completer<T> {
        Completer {
            shared: Rc::new(RefCell::new(Shared {
                value: None,
                wakers: Vec::new(),
            })),
        }
    }

    /// Completes the future with the supplied value. Listeners resume at the next drain,
    /// Dart's microtask position.
    ///
    /// # Panics
    ///
    /// On a second call, where Dart's throws a `StateError`.
    pub fn complete(&self, value: T) {
        let mut shared = self.shared.borrow_mut();
        assert!(
            shared.value.is_none(),
            "Completer.complete called twice (Dart's StateError)"
        );
        shared.value = Some(value);
        for waker in shared.wakers.drain(..) {
            waker.wake();
        }
    }

    /// Whether the future has been completed.
    pub fn is_completed(&self) -> bool {
        self.shared.borrow().value.is_some()
    }

    /// The future that is completed by this completer.
    pub fn future(&self) -> CompleterFuture<T> {
        CompleterFuture {
            shared: Rc::clone(&self.shared),
        }
    }
}

impl<T> Default for Completer<T> {
    fn default() -> Completer<T> {
        Completer::new()
    }
}

/// A [`Completer`]'s future. Every clone resolves with its own clone of the value, as every
/// `await` of one Dart future sees the same result.
pub struct CompleterFuture<T> {
    shared: Rc<RefCell<Shared<T>>>,
}

impl<T> Clone for CompleterFuture<T> {
    fn clone(&self) -> CompleterFuture<T> {
        CompleterFuture {
            shared: Rc::clone(&self.shared),
        }
    }
}

impl<T: Clone> Future for CompleterFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut shared = self.shared.borrow_mut();
        if let Some(value) = &shared.value {
            return Poll::Ready(value.clone());
        }
        let waker = cx.waker();
        if !shared.wakers.iter().any(|known| known.will_wake(waker)) {
            shared.wakers.push(waker.clone());
        }
        Poll::Pending
    }
}
