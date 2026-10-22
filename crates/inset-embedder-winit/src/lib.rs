//! Desktop embedder: winit event loop and native windows. Each view's frames are drawn at the
//! vsyncs of the screen its window is on, and held while the window is minimized or occluded:
//! from the screen's display link on macOS, from a timer at the screen's rate elsewhere. Presents
//! through valo.
//!
//! Flutter's multi-view model, three things with lifetimes of their own: the app, started at
//! launch and running until it quits; its views, each with a window here, zero or more, opened
//! and closed while it runs; and the surface each view draws on, which the system gives and takes
//! away, and which the app never sees. On a desktop the app opens every window it has, its first
//! included, as Flutter's multi-window apps do, and has no view until it asks. Where the system's
//! window is the screen, as on iOS, the app is given that window's view, the screen view, which
//! is Flutter's implicit view: it is made with the app, before its window, as Flutter's engine
//! adds it with empty metrics, and the window opens at winit's first call.

// wgpu's handle registry nests auto-trait obligations past the default depth.
#![recursion_limit = "256"]

mod app_exit;
mod app_requests;
mod appearance;
mod clipboard;
mod dispatcher;
mod drag_drop;
mod frame_scheduler;
mod gpu;
mod host_window;
mod images;
mod ime;
mod keyboard;
mod keys;
mod launch;
mod lifecycle;
mod locales;
mod macos;
mod main_loop;
mod mouse_cursor;
mod os;
mod platform;
mod pointer;
mod popup_menus;
mod rasterizer;
mod screens;
mod state;
mod system_changes;
mod system_requests;
mod text_input;
mod touch;
mod view;
mod view_focus;
mod vsync_source;
mod windows;

use inset_embedder::{EmbedderClient, PlatformRef};

pub use images::{DecodeExecution, create_image_loader};
pub use macos::MacOs;
pub use platform::WinitPlatform;
pub use system_requests::{Reopen, ReopenHandler};

/// Runs the winit event loop until quit. The app decides what happens when its last window
/// closes, as it decides every window's close.
#[derive(Default)]
pub struct WinitEmbedder;

impl WinitEmbedder {
    /// Runs the app until it quits. `start` runs first, before winit's loop, and returns the
    /// client; where the system's window is the screen, the screen view is already among the
    /// platform's views then, with no window yet, which opens at winit's first call.
    pub fn run<C: EmbedderClient + 'static>(self, start: impl FnOnce(PlatformRef) -> C + 'static) {
        launch::run(Box::new(start), None);
    }

    /// Starts the native loop with an image loader of the application's choosing.
    ///
    /// The factory receives the renderer's device resources once the embedder has a device: before
    /// `start` runs where the system's window is the screen, and otherwise when the app's first
    /// window opens. Use
    /// `create_image_loader(images, DecodeExecution::Local)` to decode without a worker
    /// thread, or build a `valo_codec::ImageLoader` with decoders and an order of your own.
    pub fn run_with_image_loader<C: EmbedderClient + 'static>(
        self,
        make_loader: impl FnOnce(valo::ImageContext) -> valo_codec::ImageLoader + 'static,
        start: impl FnOnce(PlatformRef) -> C + 'static,
    ) {
        launch::run(Box::new(start), Some(Box::new(make_loader)));
    }
}
