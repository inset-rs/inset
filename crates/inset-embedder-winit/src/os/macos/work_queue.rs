//! Work off the main thread, on Grand Central Dispatch's worker pool.
//!
//! A panic in that work is caught and dropped, so it ends that work alone, as a thread of its own
//! would end on another system: the queue calls work through an `extern "C"` function, where the
//! panic would otherwise abort the process. The panic hook still prints it.

use std::panic::{AssertUnwindSafe, catch_unwind};

use dispatch2::{DispatchQueue, DispatchQueueGlobalPriority, GlobalQueueIdentifier};

use super::AppKit;
use crate::os::WorkQueue;

impl WorkQueue for AppKit {
    /// On the system's default-priority global queue: Grand Central Dispatch's worker pool,
    /// which the system sizes and schedules with everything else on the machine.
    fn run_off_main(work: Box<dyn FnOnce() + Send>) {
        let queue = DispatchQueue::global_queue(GlobalQueueIdentifier::Priority(
            DispatchQueueGlobalPriority::Default,
        ));
        queue.exec_async(catching_panic(work));
    }
}

/// `work`, with a panic in it caught and dropped. What the work held is dropped as the panic
/// unwinds, and nothing else of the embedder's is reached from a worker thread.
fn catching_panic(work: Box<dyn FnOnce() + Send>) -> impl FnOnce() + Send {
    move || {
        let _dropped = catch_unwind(AssertUnwindSafe(work));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;

    /// Run the way `run_off_main` runs work, through the queue's `extern "C"` function, but
    /// synchronously, so an abort there would end this test's process before it returns.
    #[test]
    fn a_panic_in_work_off_the_main_thread_ends_that_work_alone() {
        let queue = DispatchQueue::global_queue(GlobalQueueIdentifier::Priority(
            DispatchQueueGlobalPriority::Default,
        ));
        let after_the_panic = Arc::new(AtomicBool::new(false));
        queue.exec_sync(catching_panic(Box::new(|| {
            panic!("in work off the main thread")
        })));
        queue.exec_sync(catching_panic(Box::new({
            let after_the_panic = Arc::clone(&after_the_panic);
            move || after_the_panic.store(true, Ordering::SeqCst)
        })));
        assert!(after_the_panic.load(Ordering::SeqCst), "the queue runs on");
    }
}
