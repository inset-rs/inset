//! The layer of a window's view, in which an app places a composition layer of its own: the layer
//! of AppKit's view behind the window's native handle.

// Reaching the view through the handle's raw pointer is what this module is for; the rest of the
// example has no `unsafe`.
#![allow(unsafe_code)]

use inset_embedder::{RawWindowHandle, WindowRef};
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use objc2_quartz_core::CALayer;

/// The layer of `window`'s view, which AppKit gives it as it hosts the view's drawing; `None` for
/// a window that has closed. Main thread.
pub(crate) fn of(window: &WindowRef) -> Option<Retained<CALayer>> {
    let RawWindowHandle::AppKit(handle) = window.native_handle()? else {
        return None;
    };
    // SAFETY: the handle names the window's view, which is alive while the window is open, and
    // this runs on the main thread, where AppKit's views are reached.
    let view: &NSView = unsafe { handle.ns_view.cast().as_ref() };
    view.layer()
}
