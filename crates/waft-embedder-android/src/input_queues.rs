//! The queues the views' input arrives in: each activity's `AInputQueue`, attached to the app's
//! looper so that input wakes the loop, and read on the app's thread.
//!
//! The invariant a later editor will break: every event read is finished (`finishEvent`), handled
//! or not, or Android takes the app for one that stopped answering; and a queue is detached before
//! Android destroys it, which waits for that (`activity_events.rs`).

use std::collections::BTreeMap;

use ndk::event::InputEvent;
use ndk::input_queue::InputQueue;
use ndk::looper::ForeignLooper;
use waft_embedder::ViewId;

use crate::log::warn_to_log;

/// What the app's looper answers when a view's input arrives.
const INPUT_ARRIVED: i32 = 1;

/// The input queue of each view whose activity has one.
#[derive(Default)]
pub(crate) struct InputQueues {
    queues: BTreeMap<ViewId, InputQueue>,
}

impl InputQueues {
    /// `queue` is where `view`'s input arrives from now on, which wakes `looper`.
    pub(crate) fn attach(&mut self, view: ViewId, queue: InputQueue, looper: &ForeignLooper) {
        queue.attach_looper(looper, INPUT_ARRIVED);
        if let Some(replaced) = self.queues.insert(view, queue) {
            replaced.detach_looper();
        }
    }

    /// `view`'s input queue is taken away.
    pub(crate) fn detach(&mut self, view: ViewId) {
        if let Some(queue) = self.queues.remove(&view) {
            queue.detach_looper();
        }
    }

    /// Hands `on_event` every event waiting in the queues, with the view it arrived in, and
    /// finishes each as `on_event` answers whether it handled it. A key the keyboard takes first
    /// (`preDispatchEvent`) is the keyboard's, and comes back in the queue if it leaves it.
    pub(crate) fn read(&self, mut on_event: impl FnMut(ViewId, &InputEvent) -> bool) {
        for (&view, queue) in &self.queues {
            loop {
                let event = match queue.event() {
                    Ok(Some(event)) => event,
                    Ok(None) => break,
                    Err(error) => {
                        warn_to_log!("waft: a view's input could not be read: {error}");
                        break;
                    }
                };
                let Some(event) = queue.pre_dispatch(event) else {
                    continue;
                };
                let handled = on_event(view, &event);
                queue.finish_event(event, handled);
            }
        }
    }
}
