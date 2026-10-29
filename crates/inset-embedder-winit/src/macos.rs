//! What only macOS offers an app, beyond `Platform`: hearing the system ask the app to show itself
//! again, a window that slides in, and hiding the whole app. Flutter's framework has no API for
//! any of them, so they are this embedder's own, and an app reaches them through
//! [`WinitPlatform::macos`](crate::WinitPlatform::macos).

use std::rc::Rc;
use std::time::Duration;

use inset_embedder::{Offset, WindowRef};

use crate::os::{ApplicationControl, ThisOs, WindowMotion};
use crate::state::EmbedderState;
use crate::system_requests::ReopenHandler;

/// What only macOS offers an app. There on macOS alone, so a call here always does what it says.
pub struct MacOs {
    state: Rc<EmbedderState>,
}

impl MacOs {
    pub(crate) fn new(state: Rc<EmbedderState>) -> MacOs {
        MacOs { state }
    }

    /// What happens when the system asks the app to show itself again: a click on its Dock
    /// icon, or opening it again from the Finder, Spotlight or `open` while it runs. `handler`
    /// gets every reopen, whether or not a window of the app is visible, with where the pointer
    /// was, on the main thread and outside every framework call, so it may post to the app.
    /// Without a visible window AppKit still brings a minimized one back itself. A reopen with no
    /// handler set is dropped, not kept.
    pub fn set_reopen_handler(&self, handler: Option<ReopenHandler>) {
        self.state.handlers().set_reopen(handler);
    }

    /// Shows `window`, sliding in to where it is placed from `from` away while it fades in,
    /// over `duration`, as a panel opens from the edge it belongs to. The window keeps its
    /// size, so its surface is not configured again while it moves.
    pub fn show_window_sliding_in(&self, window: &WindowRef, from: Offset, duration: Duration) {
        let Some(open) = self.state.views().by_view(window.view().id()) else {
            return;
        };
        if !ThisOs::show_sliding_in(&*open.window, from, duration) {
            window.show();
        }
    }

    /// Hides every window of the app and hands the focus back to the app that had it, as the
    /// application menu's Hide does; showing the app again, from its Dock icon or the app
    /// switcher, shows the windows that were visible.
    pub fn hide_application(&self) {
        ThisOs::hide_application();
    }
}
