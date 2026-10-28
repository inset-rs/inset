//! The system clipboard as the framework's `Clipboard`, and when the app hears it changed.
//!
//! Each system's clipboard is its own (`os/`): macOS's pasteboard and iOS's carry every form the
//! framework's clipboard has (`os/macos/pasteboard`, `os/ios/pasteboard`); elsewhere winit has no
//! clipboard, and arboard's plain text is all the embedder reaches (`text`), every other form
//! refused.
//!
//! No system here tells an application every change another one makes to the clipboard: the
//! embedder looks when it can matter to the app, as the app becomes active and as one of its
//! windows takes the focus, as Flutter's macOS embedder looks at its pasteboard only when the
//! framework asks, and when the system says it changed where it says so; and the app hears of a
//! change only when the clipboard says another writer changed it since the last look, or a copy
//! of the app's expired and was cleared (`main_loop/app.rs`). The app's own writes it counts as
//! seen: the app made them. A copy that expires on a system without an expiry of its own is
//! cleared by the main loop, which wakes for it as it wakes for the app's timers.

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
mod text;

use std::task::{Context, Poll, Waker};
use std::time::Instant;

use inset_embedder::{Clipboard, PastedItem};

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub(crate) use text::{ChangeProbe, TextClipboard};

/// What a system's clipboard says of its changes, and the expiry the embedder keeps for it.
pub(crate) trait ClipboardChanges {
    /// Whether another writer changed the clipboard since this was last asked; the app's own
    /// writes count as seen.
    fn has_changed(&self) -> bool;

    /// When the main loop is to clear a copy of the app's that expires; none while no copy waits
    /// to, or where the system or the clipboard clears it itself.
    fn expiry(&self) -> Option<Instant> {
        None
    }

    /// Clears the app's copy whose time has come by `now`, if the clipboard still holds it;
    /// answers whether it cleared one, which the app hears as a change.
    fn clear_expired(&self, _now: Instant) -> bool {
        false
    }
}

/// What is on `clipboard` now, as a paste from the system's own Edit menu hands it to the app:
/// every system's clipboard here lists its items in the call, so one listing that does not is
/// taken as empty.
pub(crate) fn listed_now(clipboard: &dyn Clipboard) -> Vec<Box<dyn PastedItem>> {
    let mut listing = clipboard.items();
    match listing
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(items) => items,
        Poll::Pending => Vec::new(),
    }
}
