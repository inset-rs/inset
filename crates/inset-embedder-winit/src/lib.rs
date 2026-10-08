//! Desktop embedder: winit event loop and native windows. The framework's frames are drawn one per
//! vsync of the display the first window is on: from its display link on macOS, from a timer at
//! the display's rate elsewhere. Presents through valo.
//!
//! Flutter's multi-view model, three things with lifetimes of their own: the app, started at
//! launch and running until it quits; its views, each with a window here, zero or more, opened
//! and closed while it runs; and the surface each view draws on, which the system gives and takes
//! away, and which the app never sees. The implicit view, if the embedder is set to have one, is
//! made with the app, before its window, as Flutter's engine adds it with empty metrics; its
//! window opens at winit's first call. It stays the same view for the app's whole life, as in
//! Flutter's macOS embedder: once its window closes, the view stays among the views but draws
//! nowhere.

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
mod macos;
mod main_loop;
mod mouse_cursor;
mod os;
mod platform;
mod pointer;
mod popup_menus;
mod rasterizer;
mod reply;
mod state;
mod system_requests;
mod text_input;
mod touch;
mod view;
mod view_focus;
mod vsync_source;
mod windows;

use inset_embedder::{EmbedderClient, PlatformRef, WindowConfig};

pub use images::{DecodeExecution, create_image_loader};
pub use macos::MacOs;
pub use platform::WinitPlatform;
pub use system_requests::{Reopen, ReopenHandler};

/// Runs the winit event loop until quit.
pub struct WinitEmbedder {
    /// The implicit view's window, which `run_app` draws into; `None` for an app that opens
    /// all its windows itself and starts with `run_widget`.
    pub implicit_view: Option<WindowConfig>,
    /// Whether closing the last window quits the app.
    pub quit_when_last_window_closes: bool,
}

impl Default for WinitEmbedder {
    fn default() -> WinitEmbedder {
        WinitEmbedder {
            implicit_view: Some(WindowConfig {
                title: "Inset".to_owned(),
                size: [900.0, 600.0],
                ..WindowConfig::default()
            }),
            quit_when_last_window_closes: true,
        }
    }
}

impl WinitEmbedder {
    /// The implicit view's window, or `None` to start without an implicit view, as an app that
    /// opens all its windows itself does.
    pub fn implicit_view(mut self, config: Option<WindowConfig>) -> WinitEmbedder {
        self.implicit_view = config;
        self
    }

    /// Whether closing the last window asks the app to quit, as Cmd-Q does: `true` by default,
    /// as the application delegate of Flutter's macOS template answers AppKit's question. The
    /// app quits if it answers exit, as one with no listener does at once, and runs on with no
    /// window if it cancels. With `false` the app keeps running with no window, as one that
    /// lives in the Dock does, until it quits itself or the system quits it.
    pub fn quit_when_last_window_closes(mut self, quit: bool) -> WinitEmbedder {
        self.quit_when_last_window_closes = quit;
        self
    }

    /// Runs the app until it quits. `start` runs first, before winit's loop, with the implicit
    /// view already among the platform's views and no window yet, and returns the client; the
    /// view's window opens at winit's first call.
    pub fn run<C: EmbedderClient + 'static>(self, start: impl FnOnce(PlatformRef) -> C + 'static) {
        launch::run(self, Box::new(start), None);
    }

    /// Starts the native loop with an image loader of the application's choosing.
    ///
    /// The factory receives the renderer's device resources once the embedder has a device: before
    /// `start` runs where the embedder has an implicit view, and otherwise when the app's first
    /// window opens. Use
    /// `create_image_loader(images, DecodeExecution::Local)` to decode without a worker
    /// thread, or build a `valo_codec::ImageLoader` with decoders and an order of your own.
    pub fn run_with_image_loader<C: EmbedderClient + 'static>(
        self,
        make_loader: impl FnOnce(valo::ImageContext) -> valo_codec::ImageLoader + 'static,
        start: impl FnOnce(PlatformRef) -> C + 'static,
    ) {
        launch::run(self, Box::new(start), Some(Box::new(make_loader)));
    }
}
