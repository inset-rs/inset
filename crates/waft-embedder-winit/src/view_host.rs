//! What a window's view asks of the system ([`ViewHost`]): the input methods of its window
//! (`text_input.rs`), the cursor, which it sets on its window at once (`mouse_cursor.rs`), and the
//! focus, which the event loop gives the view's window (`view_focus.rs`) on a turn of its own, since
//! a focus request is made inside the framework's call (`app_requests.rs`).
//!
//! The window's code makes the host with the view and holds it beside the view's window, as an app
//! holds the host of a view it adds: keys and input methods arrive at the window, and the loop hands
//! them to the host's text input.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Weak};

use waft_embedder::{
    SystemMouseCursorKind, TextInputHost, ViewFocusDirection, ViewFocusEvent, ViewFocusState,
    ViewHost, ViewId,
};
use winit::event::{Ime, KeyEvent};
use winit::keyboard::ModifiersState;
use winit::window::Window;

use crate::app_requests::{AppRequest, AppRequests, WindowRequest};
use crate::ime::ImeOutcome;
use crate::mouse_cursor;
use crate::text_input::{TextInput, Typed};

/// The host of one window's view.
pub(crate) struct WindowViewHost {
    view: ViewId,
    /// The view's window, held weakly: the host of a view with no window, before its window opens
    /// or once it closed, shows no cursor.
    window: RefCell<Option<Weak<dyn Window>>>,
    text_input: TextInput,
    /// Where the focus requests go, to the loop.
    requests: Rc<AppRequests>,
}

impl WindowViewHost {
    /// The host of view `view`, whose window opens later, sending its requests through
    /// `requests`.
    pub(crate) fn new(view: ViewId, requests: Rc<AppRequests>) -> WindowViewHost {
        WindowViewHost {
            view,
            window: RefCell::new(None),
            text_input: TextInput::new(),
            requests,
        }
    }

    /// The view's window from now on: it shows the view's cursor, and its input methods edit the
    /// view's text.
    pub(crate) fn attach(&self, window: &Arc<dyn Window>) {
        *self.window.borrow_mut() = Some(Arc::downgrade(window));
        self.text_input.attach(window);
    }

    /// What a key the framework did not handle does to the view's active text input.
    pub(crate) fn type_key(&self, event: &KeyEvent, modifiers: ModifiersState) -> Typed {
        self.text_input.type_key(event, modifiers)
    }

    /// What an IME event makes of the editing value of the view's text input.
    pub(crate) fn take_ime(&self, ime: &Ime) -> ImeOutcome {
        self.text_input.take_ime(ime)
    }
}

impl ViewHost for WindowViewHost {
    fn text_input(&self) -> Option<&dyn TextInputHost> {
        Some(&self.text_input)
    }

    /// On the view's window, which shows it while the pointer is over it; one mouse, so the device
    /// picks nothing.
    fn activate_system_cursor(&self, _device: i64, kind: SystemMouseCursorKind) {
        let window = self.window.borrow().as_ref().and_then(Weak::upgrade);
        if let Some(window) = window {
            mouse_cursor::show(&*window, kind);
        }
    }

    fn request_focus_change(&self, state: ViewFocusState, direction: ViewFocusDirection) {
        self.requests
            .send(AppRequest::Window(WindowRequest::ChangeFocus(
                ViewFocusEvent {
                    view_id: self.view,
                    state,
                    direction,
                },
            )));
    }
}
