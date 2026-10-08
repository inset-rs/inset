//! What the framework and the app ask of the embedder through the platform, a window or the window
//! maker, on its way to the main loop: each request is an [`AppRequest`], sent as it is made, from
//! inside the framework's call, and served on a turn of the loop of its own, in the order made, as
//! Flutter posts a task to the platform thread and Chromium a `PostTask`.
//!
//! winit's proxy carries only what may cross threads, since the dispatcher's wakes and the display
//! links' vsyncs send through it from other threads, while a request may carry the framework's
//! `Rc`s: a window's reply, an exit's. So a request waits in [`AppRequests`], on the main thread,
//! and the proxy carries a [`MainLoopEvent::AppRequest`] for it, which the loop answers by serving
//! the oldest request waiting. The invariant a later editor must keep: a request is queued only
//! together with its event, and taken only for one, so that the requests are served in the order
//! of their events among everything else the proxy carries.

use std::cell::RefCell;
use std::collections::VecDeque;

use inset_embedder::{SystemMouseCursorKind, ViewFocusEvent};
use winit::window::WindowId;

use crate::app_exit::ExitRequest;
use crate::main_loop::MainLoopEvent;
use crate::windows::WindowRequest;

/// A request the framework or the app makes of the embedder, for the main loop to serve.
pub(crate) enum AppRequest {
    /// Draw a frame at the next vsync.
    RequestFrame,
    /// Open a window; its future answers once it is open.
    OpenWindow(WindowRequest),
    /// A created window's `close`: the window is freed, and its view goes.
    CloseWindow(WindowId),
    /// Quit, asking the framework first unless the exit is required.
    Exit(ExitRequest),
    /// Show this cursor on the window the pointer is in.
    ShowCursor(SystemMouseCursorKind),
    /// Focus a view's window, or give up its focus.
    ChangeViewFocus(ViewFocusEvent),
    /// Hand the files waiting to be opened to the handler the app just set.
    HandOverFiles,
}

/// The requests sent and not served yet, oldest first. Main thread.
pub(crate) struct AppRequests {
    waiting: RefCell<VecDeque<AppRequest>>,
    /// Sends the loop the [`MainLoopEvent::AppRequest`] a request is served at.
    send_event: Box<dyn Fn(MainLoopEvent)>,
}

impl AppRequests {
    /// A queue whose events go to the loop through `send_event`.
    pub(crate) fn new(send_event: impl Fn(MainLoopEvent) + 'static) -> AppRequests {
        AppRequests {
            waiting: RefCell::default(),
            send_event: Box::new(send_event),
        }
    }

    /// Sends the loop `request`, which it serves after those sent before.
    pub(crate) fn send(&self, request: AppRequest) {
        self.waiting.borrow_mut().push_back(request);
        (self.send_event)(MainLoopEvent::AppRequest);
    }

    /// The request a [`MainLoopEvent::AppRequest`] is served for: the oldest waiting.
    pub(crate) fn take_next(&self) -> Option<AppRequest> {
        self.waiting.borrow_mut().pop_front()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use inset_embedder::{ViewFocusDirection, ViewFocusState, ViewId};

    use super::*;

    fn focus(view: u64) -> AppRequest {
        AppRequest::ChangeViewFocus(ViewFocusEvent {
            view_id: ViewId(view),
            state: ViewFocusState::Focused,
            direction: ViewFocusDirection::Undefined,
        })
    }

    fn focused_view(request: Option<AppRequest>) -> Option<u64> {
        match request? {
            AppRequest::ChangeViewFocus(change) => Some(change.view_id.0),
            _ => None,
        }
    }

    #[test]
    fn each_request_sends_its_event_and_they_are_served_in_the_order_sent() {
        let events = Rc::new(Cell::new(0));
        let queue = AppRequests::new({
            let events = Rc::clone(&events);
            move |event| {
                assert!(matches!(event, MainLoopEvent::AppRequest));
                events.set(events.get() + 1);
            }
        });
        queue.send(focus(2));
        queue.send(AppRequest::HandOverFiles);
        queue.send(focus(1));
        assert_eq!(events.get(), 3, "one event for each request");

        assert_eq!(focused_view(queue.take_next()), Some(2));
        assert!(matches!(queue.take_next(), Some(AppRequest::HandOverFiles)));
        assert_eq!(focused_view(queue.take_next()), Some(1));
        assert!(queue.take_next().is_none(), "each served once");
    }
}
