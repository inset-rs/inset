//! Desktop host: winit event loop and native windows. Frames are paced to the display,
//! one per refresh: from a display link on macOS, from a timer at the display's rate
//! elsewhere. Presents through valo.

// wgpu's handle registry nests auto-trait obligations past the default depth.
#![recursion_limit = "256"]

mod drag_drop;
mod frames;
mod gpu;
mod images;
mod ime;
mod input;
mod keys;
mod os;
mod pacing;
mod platform;
mod pointer;
mod surface;
mod text_input;
mod view;
mod window;
mod windows;

use inset_embedder::{EmbedderClient, PlatformRef};

pub use images::{DecodeExecution, create_image_loader};
pub use platform::WinitPlatform;

/// Configuration for the implicit view created before application startup.
#[derive(Clone, Debug, PartialEq)]
pub struct ImplicitViewConfig {
    pub title: String,
    pub logical_size: [f64; 2],
}

impl Default for ImplicitViewConfig {
    fn default() -> ImplicitViewConfig {
        ImplicitViewConfig {
            title: "Inset".to_owned(),
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
        window::run(self, start, None);
    }

    /// Starts the native loop with an image loader of the application's choosing.
    ///
    /// The factory receives the renderer's device resources before `start` runs. Use
    /// `create_image_loader(images, DecodeExecution::Local)` to decode without a worker
    /// thread, or build a `valo_codec::ImageLoader` with decoders and an order of your own.
    pub fn run_with_image_loader<C: EmbedderClient + 'static>(
        self,
        make_loader: impl FnOnce(valo::ImageContext) -> valo_codec::ImageLoader + 'static,
        start: impl FnOnce(PlatformRef) -> C + 'static,
    ) {
        window::run(self, start, Some(Box::new(make_loader)));
    }
}
