//! A view's host that does what a test reads back: it records the cursors shown over its view, the
//! focus requested for it, what its text input was asked, what the framework said of handling
//! Back in it and the bars' styles set over it, and moves nothing.

use std::cell::RefCell;

use waft_embedder::{
    Matrix4, Rect, SelectionRect, Size, SystemMouseCursorKind, SystemUiOverlayStyle,
    TextEditingValue, TextInputConfiguration, TextInputHost, ViewFocusDirection, ViewFocusState,
    ViewHost,
};

/// A view's host that records what the view asks of the system, as `flutter_test`'s
/// `TestTextInput` logs the text input channel and its tests log `SystemChannels.mouseCursor`,
/// `SystemChannels.navigation`'s `setFrameworkHandlesBack` and `SystemChannels.platform`'s
/// overlay style.
#[derive(Default)]
pub struct TestViewHost {
    cursors: RefCell<Vec<(i64, SystemMouseCursorKind)>>,
    focus_requests: RefCell<Vec<(ViewFocusState, ViewFocusDirection)>>,
    text_input: RecordingTextInput,
    framework_handles_back: RefCell<Vec<bool>>,
    overlay_styles: RefCell<Vec<SystemUiOverlayStyle>>,
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

    /// Each value the framework said of whether it handles back in the view
    /// (`ViewHost::set_framework_handles_back`), oldest first.
    pub fn framework_handles_back(&self) -> Vec<bool> {
        self.framework_handles_back.borrow().clone()
    }

    /// Each style set for the system's bars over the view (`ViewHost::set_overlay_style`), oldest
    /// first.
    pub fn overlay_styles(&self) -> Vec<SystemUiOverlayStyle> {
        self.overlay_styles.borrow().clone()
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

    fn set_framework_handles_back(&self, handles: bool) {
        self.framework_handles_back.borrow_mut().push(handles);
    }

    fn set_overlay_style(&self, style: &SystemUiOverlayStyle) {
        self.overlay_styles.borrow_mut().push(*style);
    }
}

/// What a [`TestViewHost`]'s text input was asked.
#[derive(Clone, Debug, PartialEq)]
pub enum TextInputCall {
    SetClient(TextInputConfiguration),
    Show,
    UpdateConfig(TextInputConfiguration),
    ClearClient,
    Hide,
    SetEditingState(TextEditingValue),
    SetComposingRect(Rect),
    SetCaretRect(Rect),
    SetClientGeometry(Size, Matrix4),
    SetSelectionRects(Vec<SelectionRect>),
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
    fn set_client(&self, configuration: &TextInputConfiguration) {
        self.record(TextInputCall::SetClient(configuration.clone()));
    }

    fn show(&self) {
        self.record(TextInputCall::Show);
    }

    fn update_config(&self, configuration: &TextInputConfiguration) {
        self.record(TextInputCall::UpdateConfig(configuration.clone()));
    }

    fn clear_client(&self) {
        self.record(TextInputCall::ClearClient);
    }

    fn hide(&self) {
        self.record(TextInputCall::Hide);
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

    fn set_selection_rects(&self, selection_rects: &[SelectionRect]) {
        self.record(TextInputCall::SetSelectionRects(selection_rects.to_vec()));
    }
}
