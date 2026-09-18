//! Flutter counterpart: `dart:async` `Completer`, the `Future` it completes, and `Future.wait`.
//!
//! A future's listeners run on the microtask queue, as Dart's do: [`CompleterFuture::then`]
//! queues its callback through the [`App`] and [`Completer::complete`] queues the pending ones.
//! An `await` resumes its task at the checkpoint instead; the shell runs both.

use std::cell::{Cell, OnceCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use crate::app::App;
use crate::change_notifier::Listener;

type Callback<T> = Box<dyn FnOnce(&mut App, T)>;

/// The state a completer and its futures share. The value is written once, so it is a
/// `OnceCell`; the lists are only ever taken out whole and put back, so they are `Cell`s —
/// nothing here can hit a runtime borrow conflict.
struct Shared<T> {
    value: OnceCell<T>,
    wakers: Cell<Vec<Waker>>,
    callbacks: Cell<Vec<Callback<T>>>,
}

impl<T> Shared<T> {
    fn new(value: Option<T>) -> Rc<Shared<T>> {
        let shared = Shared {
            value: OnceCell::new(),
            wakers: Cell::new(Vec::new()),
            callbacks: Cell::new(Vec::new()),
        };
        if let Some(value) = value {
            let _ = shared.value.set(value);
        }
        Rc::new(shared)
    }
}

/// A way to produce `Future` objects and to complete them later with a value.
///
/// The resolving half; [`future`](Self::future) hands out the listening half any number of
/// times. A completer dropped before [`complete`](Self::complete) leaves its futures pending
/// forever, as a Dart future nobody completes.
pub struct Completer<T> {
    shared: Rc<Shared<T>>,
}

impl<T> Completer<T> {
    pub fn new() -> Completer<T> {
        Completer {
            shared: Shared::new(None),
        }
    }

    /// A completer that has already completed with `value`.
    pub fn completed(value: T) -> Completer<T> {
        Completer {
            shared: Shared::new(Some(value)),
        }
    }

    /// Whether the future has been completed.
    pub fn is_completed(&self) -> bool {
        self.shared.value.get().is_some()
    }

    /// The future that is completed by this completer.
    pub fn future(&self) -> CompleterFuture<T> {
        CompleterFuture {
            shared: Rc::clone(&self.shared),
        }
    }
}

impl<T: Clone + 'static> Completer<T> {
    /// Completes the future with the supplied value. Registered callbacks run on the
    /// microtask queue and awaiting tasks resume at the checkpoint — never inline.
    ///
    /// # Panics
    ///
    /// On a second call, where Dart's throws a `StateError`.
    pub fn complete(&self, app: &mut App, value: T) {
        assert!(
            self.shared.value.set(value.clone()).is_ok(),
            "Completer.complete called twice (Dart's StateError)"
        );
        for waker in self.shared.wakers.take() {
            waker.wake();
        }
        for callback in self.shared.callbacks.take() {
            schedule(app, callback, value.clone());
        }
    }
}

impl<T> Default for Completer<T> {
    fn default() -> Completer<T> {
        Completer::new()
    }
}

/// Another reference to the same completion, as a Dart completer is one shared object: what
/// lets a completer held in arena data be completed with the `App` borrowed mutably.
impl<T> Clone for Completer<T> {
    fn clone(&self) -> Completer<T> {
        Completer {
            shared: Rc::clone(&self.shared),
        }
    }
}

/// Queues `callback` with `value` on the microtask queue.
fn schedule<T: 'static>(app: &mut App, callback: Callback<T>, value: T) {
    let pending = Cell::new(Some((callback, value)));
    app.schedule_microtask(Listener::new(move |app| {
        if let Some((callback, value)) = pending.take() {
            callback(app, value);
        }
    }));
}

/// A [`Completer`]'s future. Every clone resolves with its own clone of the value, as every
/// `await` of one Dart future sees the same result.
pub struct CompleterFuture<T> {
    shared: Rc<Shared<T>>,
}

impl<T> CompleterFuture<T> {
    /// A future that is already complete: Dart's `Future.value` and `SynchronousFuture`.
    pub fn ready(value: T) -> CompleterFuture<T> {
        Completer::completed(value).future()
    }

    /// Whether the value is known yet — what Dart learns by handing `then` a callback and
    /// seeing whether it ran synchronously.
    pub fn is_completed(&self) -> bool {
        self.shared.value.get().is_some()
    }
}

impl<T: Clone + 'static> CompleterFuture<T> {
    /// Runs `future` as a task and hands out its result as a completer's future: the future
    /// of a Dart `async` function whose body awaits, for a method that returns a nameable,
    /// shareable future over an awaited chain.
    pub fn spawn(app: &App, future: impl Future<Output = T> + 'static) -> CompleterFuture<T> {
        let completer = Completer::new();
        let result = completer.future();
        app.spawn(async move |cx| {
            let value = future.await;
            cx.update(|app| completer.complete(app, value));
        });
        result
    }

    /// The value, if the completer has completed.
    pub fn peek(&self) -> Option<T> {
        self.shared.value.get().cloned()
    }

    /// Dart's `then`: runs `callback` with the value on the microtask queue once the future
    /// has completed — never inline, even on an already resolved future.
    pub fn then(self, app: &mut App, callback: impl FnOnce(&mut App, T) + 'static) {
        match self.shared.value.get() {
            Some(value) => schedule(app, Box::new(callback), value.clone()),
            None => {
                let mut callbacks = self.shared.callbacks.take();
                callbacks.push(Box::new(callback));
                self.shared.callbacks.set(callbacks);
            }
        }
    }
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
        if let Some(value) = self.shared.value.get() {
            return Poll::Ready(value.clone());
        }
        let waker = cx.waker();
        let mut wakers = self.shared.wakers.take();
        if !wakers.iter().any(|known| known.will_wake(waker)) {
            wakers.push(waker.clone());
        }
        self.shared.wakers.set(wakers);
        Poll::Pending
    }
}

/// Dart's `Future.wait`: resolves with every value once the last future has resolved.
pub async fn wait_all<T>(futures: impl IntoIterator<Item = impl Future<Output = T>>) -> Vec<T> {
    let mut values = Vec::new();
    for future in futures {
        values.push(future.await);
    }
    values
}
