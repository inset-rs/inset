//! Desktop host: winit event loop and native windows. Frame pacing is
//! `Window::request_redraw`. Presents through valo.

// wgpu's handle registry nests auto-trait obligations past the default depth.
#![recursion_limit = "256"]

mod gpu;
mod ime;
mod keys;
mod text_input;
mod window;

use reveal_embedder::{EmbedderClient, PlatformRef};

/// Configuration for the implicit view created before application startup.
#[derive(Clone, Debug, PartialEq)]
pub struct ImplicitViewConfig {
    pub title: String,
    pub logical_size: [f64; 2],
}

impl Default for ImplicitViewConfig {
    fn default() -> ImplicitViewConfig {
        ImplicitViewConfig {
            title: "reveal".to_owned(),
            logical_size: [900.0, 600.0],
        }
    }
}

/// Runs the winit event loop until quit.
pub struct WinitEmbedder {
    /// `None` starts without an implicit native window.
    pub implicit_view: Option<ImplicitViewConfig>,
}

impl Default for WinitEmbedder {
    fn default() -> WinitEmbedder {
        WinitEmbedder {
            implicit_view: Some(ImplicitViewConfig::default()),
        }
    }
}

impl WinitEmbedder {
    /// Starts the native loop. `start` runs on the first `resumed`, after the
    /// configured implicit view is created, and returns the client.
    pub fn run<C: EmbedderClient + 'static>(self, start: impl FnOnce(PlatformRef) -> C + 'static) {
        window::run(self, start);
    }
}
