//! The host an application gets without choosing one: winit on native, the
//! canvas WebGPU host in the browser.
//!
//! An application depends on this crate instead of picking an embedder and
//! repeating target-cfg dependencies. `DefaultEmbedder::run` is the same
//! closure on both.
//!
//! A `wasm32-wasi` build, a component for a WASI host, has no default host: the
//! host's own embedder crate exports the guest's start and runs the app, and
//! this crate is empty there.
#![cfg(not(all(target_arch = "wasm32", target_os = "wasi")))]

use inset_embedder::EmbedderClient;

#[cfg(not(target_arch = "wasm32"))]
pub use inset_embedder_winit::ImplicitViewConfig;

/// Runs the application on this target's host.
#[derive(Default)]
pub struct DefaultEmbedder {
    #[cfg(not(target_arch = "wasm32"))]
    inner: inset_embedder_winit::WinitEmbedder,
    #[cfg(target_arch = "wasm32")]
    inner: inset_embedder_web::WebEmbedder,
}

impl DefaultEmbedder {
    /// Native implicit window. Ignored on wasm, where the page supplies the canvas.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn implicit_view(mut self, config: Option<ImplicitViewConfig>) -> DefaultEmbedder {
        self.inner.implicit_view = config;
        self
    }

    /// Canvas element id. Ignored on native.
    #[cfg(target_arch = "wasm32")]
    pub fn canvas_id(mut self, id: impl Into<String>) -> DefaultEmbedder {
        self.inner = self.inner.canvas_id(id);
        self
    }

    /// Starts the host loop. `start` runs once the implicit view exists (native)
    /// or the canvas GPU is ready (wasm).
    pub fn run<C: EmbedderClient + 'static>(
        self,
        start: impl FnOnce(inset_embedder::PlatformRef) -> C + 'static,
    ) {
        self.inner.run(start);
    }
}
