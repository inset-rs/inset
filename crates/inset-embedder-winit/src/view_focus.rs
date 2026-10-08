//! Which view has the focus: the framework's requests to focus a view, which the event loop
//! honours as Flutter's macOS embedder does, and the system's focus changes, which it reports.

use inset_embedder::{ViewFocusDirection, ViewFocusEvent, ViewFocusState, ViewId};
use winit::window::Window;

use crate::view::WinitView;

/// Focuses `window`, `view`'s, if `request` asks for the focus and it does not have it; a
/// request to give the focus up is ignored, as Flutter's macOS embedder ignores it. The system's
/// focus event reports the result.
pub(crate) fn focus_window_if_asked(request: &ViewFocusEvent, window: &Window, view: &WinitView) {
    if request.state == ViewFocusState::Focused && !window.has_focus() {
        window.focus_window();
        // Focusing orders the window front, maybe on another screen: its surfaces follow it.
        view.follow_window();
    }
}

/// The system's focus change of `view`'s window, as the framework hears it: winit reports no
/// direction the focus moved in.
pub(crate) fn focus_changed(view: ViewId, focused: bool) -> ViewFocusEvent {
    ViewFocusEvent {
        view_id: view,
        state: if focused {
            ViewFocusState::Focused
        } else {
            ViewFocusState::Unfocused
        },
        direction: ViewFocusDirection::Undefined,
    }
}
