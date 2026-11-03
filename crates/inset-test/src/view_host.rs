//! A view's host that does what a test reads back: it records the cursors shown over its view, the
//! focus requested for it and what its text input was asked, and moves nothing.

use std::cell::RefCell;

use inset_embedder::{
    Matrix4, Rect, Size, SystemMouseCursorKind, TextEditingValue, TextInputConfiguration,
    TextInputHost, ViewFocusDirection, ViewFocusState, ViewHost,
};

/// A view's host that records what the view asks of the system, as `flutter_test`'s
/// `TestTextInput` logs the text input channel and its tests log `SystemChannels.mouseCursor`.
#[derive(Default)]
pub struct TestViewHost {
    cursors: RefCell<Vec<(i64, SystemMouseCursorKind)>>,
    focus_requests: RefCell<Vec<(ViewFocusState, ViewFocusDirection)>>,
    text_input: RecordingTextInput,
}

impl TestViewHost {
    /// Each cursor shown over the view, with the device it was shown for, oldest first.
    pub fn cursors(&self) -> Vec<(i64, SystemMouseCursorKind)> {
        self.cursors.borrow().clone()
    }

    /// Each focus change requested for the view, oldest first.
    pub fn focus_requests(&self) -> Vec<(ViewFocusState, ViewFocusDirection)> {
        self.focus_requests.borrow().clone()
    }

    /// What the view's text input was asked, oldest first.
    pub fn text_input_calls(&self) -> Vec<TextInputCall> {
        self.text_input.calls.borrow().clone()
    }
}

impl ViewHost for TestViewHost {
    fn text_input(&self) -> Option<&dyn TextInputHost> {
        Some(&self.text_input)
    }

    fn activate_system_cursor(&self, device: i64, kind: SystemMouseCursorKind) {
        self.cursors.borrow_mut().push((device, kind));
    }

    fn request_focus_change(&self, state: ViewFocusState, direction: ViewFocusDirection) {
        self.focus_requests.borrow_mut().push((state, direction));
    }
}

/// What a [`TestViewHost`]'s text input was asked.
#[derive(Clone, Debug, PartialEq)]
pub enum TextInputCall {
    Start(TextInputConfiguration),
    Stop,
    SetEditingState(TextEditingValue),
    SetComposingRect(Rect),
    SetCaretRect(Rect),
    SetClientGeometry(Size, Matrix4),
}

/// A text input that records each call, edits nothing itself and provides no commands of its own.
#[derive(Default)]
struct RecordingTextInput {
    calls: RefCell<Vec<TextInputCall>>,
}

impl RecordingTextInput {
    fn record(&self, call: TextInputCall) {
        self.calls.borrow_mut().push(call);
    }
}

impl TextInputHost for RecordingTextInput {
    fn start(&self, configuration: &TextInputConfiguration) {
        self.record(TextInputCall::Start(configuration.clone()));
    }

    fn stop(&self) {
        self.record(TextInputCall::Stop);
    }

    fn set_editing_state(&self, value: &TextEditingValue) {
        self.record(TextInputCall::SetEditingState(value.clone()));
    }

    fn set_composing_rect(&self, rect: Rect) {
        self.record(TextInputCall::SetComposingRect(rect));
    }

    fn set_caret_rect(&self, rect: Rect) {
        self.record(TextInputCall::SetCaretRect(rect));
    }

    fn set_client_geometry(&self, size: Size, transform: &Matrix4) {
        self.record(TextInputCall::SetClientGeometry(size, *transform));
    }
}
