//! What the framework and the app ask of the embedder through the platform, a window, a view's host
//! or the window maker, on its way to the main loop: each request is an [`AppRequest`], sent as it is made, from
//! inside the framework's call, and served on a turn of the loop of its own, in the order made, as
//! Flutter posts a task to the platform thread and Chromium a `PostTask`.
//!
//! The loop's proxy carries only what may cross threads, since the dispatcher's wakes and the
//! display links' vsyncs send through it from other threads, while a request may carry the
//! framework's `Rc`s: a window's reply. So a request waits in [`AppRequests`], on the main thread,
//! and the proxy carries a [`MainLoopEvent::AppRequest`] for it, which the loop answers by serving
//! the oldest request waiting. The invariant a later editor must keep: a request is queued only
//! together with its event, and taken only for one, so that the requests are served in the order
//! of their events among everything else the proxy carries.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use waft_embedder::{AppExitResponse, PointerReport, ViewFocusEvent, ViewId, ViewMetrics};
use winit::window::WindowId;

use crate::main_loop::MainLoopEvent;
use crate::os::ViewLayer;
use crate::view::WinitView;
use crate::windows::RequestedWindow;

/// A request the framework or the app makes of the embedder, for the main loop to serve.
pub(crate) enum AppRequest {
    /// Draw a frame of this view at the next vsync of its window's screen.
    RequestFrame(ViewId),
    /// What the framework asks of the windows: opening, closing and focusing them.
    Window(WindowRequest),
    /// What the app asks of the views it embeds, through the platform's `ViewEmbedder`.
    Embedder(EmbedderRequest),
    /// The app's exit: its own, or its answer to the system's request.
    Exit(ExitRequest),
    /// Look at the clipboard, and tell the app if it changed: the system cleared a copy of the
    /// app's that expired, or says the clipboard changed.
    LookAtClipboard,
}

/// What the framework asks of the windows the embedder makes.
pub(crate) enum WindowRequest {
    /// Open a window for a view the app asked for; the view arrives through `view_added`.
    Open(RequestedWindow),
    /// A created window's `close`: the window is freed, and its view goes.
    Close(WindowId),
    /// Focus a view's window, or give up its focus.
    ChangeFocus(ViewFocusEvent),
}

/// The app's exit.
pub(crate) enum ExitRequest {
    /// Quit: the app's required exit.
    Quit,
    /// The framework's answer to a request to exit: quit on exit.
    Answer(AppExitResponse),
}

/// What the app asks, through the platform's `ViewEmbedder`, of the views it embeds: what a window
/// does for its own view, and what the system tells the framework of one.
pub(crate) enum EmbedderRequest {
    /// Add this view the app asked for on its layer, at these metrics: Flutter's `AddView`.
    AddView(Rc<WinitView>, ViewLayer, ViewMetrics),
    /// Give a view the app added these metrics: Flutter's `SendWindowMetricsEvent`.
    SetViewMetrics(ViewId, ViewMetrics),
    /// A view the app added is in sight, or not, as the app says.
    SetViewInSight(ViewId, bool),
    /// Remove a view the app added: Flutter's `RemoveView`.
    RemoveView(ViewId),
    /// Hand the framework what the app says pointing devices did: Flutter's `SendPointerEvent`.
    SendPointerReports(Vec<PointerReport>),
    /// Hand the framework a view's focus change the app reports: Flutter's `SendViewFocusEvent`.
    SendViewFocusEvent(ViewFocusEvent),
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

    use waft_embedder::{ViewFocusDirection, ViewFocusState};

    use super::*;

    fn focus(view: u64) -> AppRequest {
        AppRequest::Window(WindowRequest::ChangeFocus(ViewFocusEvent {
            view_id: ViewId(view),
            state: ViewFocusState::Focused,
            direction: ViewFocusDirection::Undefined,
        }))
    }

    fn focused_view(request: Option<AppRequest>) -> Option<u64> {
        match request? {
            AppRequest::Window(WindowRequest::ChangeFocus(change)) => Some(change.view_id.0),
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
        queue.send(AppRequest::LookAtClipboard);
        queue.send(focus(1));
        assert_eq!(events.get(), 3, "one event for each request");

        assert_eq!(focused_view(queue.take_next()), Some(2));
        assert!(matches!(
            queue.take_next(),
            Some(AppRequest::LookAtClipboard)
        ));
        assert_eq!(focused_view(queue.take_next()), Some(1));
        assert!(queue.take_next().is_none(), "each served once");
    }
}
