//! The host's answer to a framework request that the event loop serves on a later turn: a
//! window, an exit. The framework awaits the future; the loop completes the reply.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

/// One answer, delivered once, to a future polled at the app's checkpoints.
struct Shared<T> {
    value: Option<T>,
    waker: Option<Waker>,
}

pub(crate) struct Reply<T>(Rc<RefCell<Shared<T>>>);

impl<T> Reply<T> {
    pub(crate) fn complete(self, value: T) {
        let waker = {
            let mut shared = self.0.borrow_mut();
            shared.value = Some(value);
            shared.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

pub(crate) struct ReplyFuture<T>(Rc<RefCell<Shared<T>>>);

impl<T> Future for ReplyFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut shared = self.0.borrow_mut();
        match shared.value.take() {
            Some(value) => Poll::Ready(value),
            None => {
                shared.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

pub(crate) fn reply<T>() -> (Reply<T>, ReplyFuture<T>) {
    let shared = Rc::new(RefCell::new(Shared {
        value: None,
        waker: None,
    }));
    (Reply(Rc::clone(&shared)), ReplyFuture(shared))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_resolves_its_future_once_completed() {
        let (reply, mut future) = reply::<u32>();
        let mut cx = Context::from_waker(Waker::noop());
        assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
        reply.complete(7);
        assert_eq!(Pin::new(&mut future).poll(&mut cx), Poll::Ready(7));
    }
}
