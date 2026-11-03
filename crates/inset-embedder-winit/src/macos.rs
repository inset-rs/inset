//! What only macOS offers an app, beyond `Platform`: hearing the system ask the app to show itself
//! again, a window that slides in, hiding the whole app, a composition layer placed in a
//! `CALayer` the app holds, and a layer the system draws shown as a composition layer's content.
//! Flutter's framework has no API for any of them, so they are this embedder's own, and an app
//! reaches them through [`WinitPlatform::macos`](crate::WinitPlatform::macos).

use std::rc::Rc;
#[cfg(target_os = "macos")]
use std::sync::Arc;
use std::time::Duration;

#[cfg(target_os = "macos")]
use inset_embedder::{CompositionLayer, Placement};
use inset_embedder::{Offset, WindowRef};
#[cfg(target_os = "macos")]
use objc2_quartz_core::CALayer;

#[cfg(target_os = "macos")]
use crate::os;
use crate::os::System;
use crate::os::capabilities::{ApplicationControl, WindowMotion};
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
        let Some(open) = self.state.windows().by_view(window.view().id()) else {
            return;
        };
        if !System::show_sliding_in(&*open.window, from, duration) {
            window.show();
        }
    }

    /// Hides every window of the app and hands the focus back to the app that had it, as the
    /// application menu's Hide does; showing the app again, from its Dock icon or the app
    /// switcher, shows the windows that were visible.
    pub fn hide_application(&self) {
        System::hide_application();
    }

    /// Places `layer` in `parent`, a `CALayer` the app holds, such as the root of a `CAContext`
    /// another process shows, above `parent`'s sublayers: where `placement` says, in pixels
    /// `scale` of which make one of `parent`'s points, as a view's frame places a layer in its
    /// window. Call it again when `parent`'s size or scale changes. The app holds `layer` from
    /// then on, so no view or other layer places it, until it goes. A layer no compositor of this
    /// host made, or one a view or another layer holds, shows nothing. Main thread.
    #[cfg(target_os = "macos")]
    pub fn place_layer_in(
        &self,
        parent: &CALayer,
        layer: &Arc<dyn CompositionLayer>,
        placement: &Placement,
        scale: f64,
    ) {
        os::place_layer_in(parent, layer, placement, scale);
    }

    /// Shows `system_layer`, which the system draws into, such as an `AVPlayerLayer` or a
    /// `CALayerHost`, as `layer`'s content, filling it and clipped and scaled with it, in place
    /// of the buffers presented to it, until the next present. A layer no compositor of this host
    /// made shows nothing. Main thread.
    #[cfg(target_os = "macos")]
    pub fn show_system_layer(&self, layer: &Arc<dyn CompositionLayer>, system_layer: &CALayer) {
        os::show_system_layer(layer, system_layer);
    }
}
