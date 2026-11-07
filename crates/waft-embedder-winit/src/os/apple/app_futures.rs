//! Futures of the app's, which stay on the main thread they were made on, run there on behalf of
//! the system: polled on the main dispatch queue each time they are woken, where the system takes
//! the answer later, through a completion handler ([`spawn`]).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Wake, Waker};

use dispatch2::DispatchQueue;

/// A future of the app's, which must stay on the thread it was made on.
pub(crate) type LocalFuture<T> = Pin<Box<dyn Future<Output = T>>>;

thread_local! {
    /// The futures being run, each by its number, until it completes. Main thread.
    static RUNNING: RefCell<HashMap<u64, LocalFuture<()>>> = RefCell::default();
    static NEXT_NUMBER: Cell<u64> = const { Cell::new(0) };
}

/// Runs `future` on the main thread to its end: polled now, and again on the main dispatch queue
/// each time it is woken, from whichever thread. Main thread.
pub(crate) fn spawn(future: LocalFuture<()>) {
    let number = NEXT_NUMBER.replace(NEXT_NUMBER.get() + 1);
    RUNNING.with_borrow_mut(|running| running.insert(number, future));
    poll(number);
}

/// Polls the future `number`, which is let go once it completes.
fn poll(number: u64) {
    let Some(mut future) = RUNNING.with_borrow_mut(|running| running.remove(&number)) else {
        return;
    };
    let waker = Waker::from(Arc::new(PollOnMainQueue(number)));
    if future
        .as_mut()
        .poll(&mut Context::from_waker(&waker))
        .is_pending()
    {
        RUNNING.with_borrow_mut(|running| running.insert(number, future));
    }
}

/// Polls a future again on the main dispatch queue.
struct PollOnMainQueue(u64);

impl Wake for PollOnMainQueue {
    fn wake(self: Arc<Self>) {
        let number = self.0;
        DispatchQueue::main().exec_async(move || poll(number));
    }
}
