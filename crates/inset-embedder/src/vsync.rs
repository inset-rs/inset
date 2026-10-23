//! The screens' refreshes, for whoever paces by them beside the framework: Flutter's
//! `VsyncWaiter`, which a host with screen ticks hands out for every screen it has, and the
//! observer it calls. A native surface's producer, or a host of programs that draw for their own
//! views, waits here on the same refreshes the host draws the framework's frames at.
//!
//! The invariant: one wait is one call, at the first refresh of its screen after it was asked,
//! whether or not a window on that screen is in sight; what a producer shows, and whether anyone
//! sees it, are its own to know.

use std::sync::Weak;

use web_time::Instant;

use crate::ScreenId;

/// Flutter's `VsyncWaiter`, for any screen.
pub trait VsyncWaiter: Send + Sync {
    /// Calls `observer` once, on the main thread, at the next refresh of `screen`. May be called
    /// from any thread. Asked again with the same observer before that refresh, it is still
    /// called once. An observer that is gone by then is not called, nor one waiting on a screen
    /// that is not connected.
    fn next_vsync(&self, screen: ScreenId, observer: Weak<dyn VsyncObserver>);
}

/// What a [`VsyncWaiter`] calls at the refresh it was asked for.
pub trait VsyncObserver: Send + Sync {
    /// `at` is the host's clock when the refresh was served.
    fn vsync(&self, at: Instant);
}
