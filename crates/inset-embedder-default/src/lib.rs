//! The host an application gets without choosing one: winit on the desktops and iOS, the
//! activity's own loop on Android, the canvas WebGPU host in the browser.
//!
//! An application depends on this crate instead of picking an embedder and repeating
//! target-cfg dependencies. `DefaultEmbedder::run` is the same closure on all of them; what
//! differs is what each host needs before it can start, which is nothing on the desktops, a
//! canvas in the browser, and the `AndroidApp` the activity handed `android_main`, where the
//! closure is also sent to the app's own thread.
//!
//! A `wasm32-wasi` build, a component for a WASI host, has no default host: the host's own
//! embedder crate exports the guest's start and runs the app, and this crate is empty there.
#![cfg(not(all(target_arch = "wasm32", target_os = "wasi")))]

use inset_embedder::EmbedderClient;

#[cfg(target_os = "android")]
pub use inset_embedder_android::AndroidApp;

/// Runs the application on this target's host.
#[derive(Default)]
pub struct DefaultEmbedder {
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    inner: inset_embedder_winit::WinitEmbedder,
    #[cfg(target_os = "android")]
    app: Option<AndroidApp>,
    #[cfg(target_arch = "wasm32")]
    inner: inset_embedder_web::WebEmbedder,
}

impl DefaultEmbedder {
    /// Canvas element id. Ignored on native.
    #[cfg(target_arch = "wasm32")]
    pub fn canvas_id(mut self, id: impl Into<String>) -> DefaultEmbedder {
        self.inner = self.inner.canvas_id(id);
        self
    }

    /// The `AndroidApp` the activity handed `android_main`, which the host's loop runs on.
    /// Required there, and `run` panics without it.
    #[cfg(target_os = "android")]
    pub fn android_app(mut self, app: AndroidApp) -> DefaultEmbedder {
        self.app = Some(app);
        self
    }

    /// Starts the host loop. `start` runs once, at launch, with the views the host has then,
    /// and returns the client the host drives.
    #[cfg(not(target_os = "android"))]
    pub fn run<C: EmbedderClient + 'static>(
        self,
        start: impl FnOnce(inset_embedder::PlatformRef) -> C + 'static,
    ) {
        self.inner.run(start);
    }

    /// Attaches the activity to the app and runs it until the activity is destroyed; the
    /// process's first activity starts the app. `start` runs once, on the app's own thread, which
    /// outlives every activity, so it is sent there.
    #[cfg(target_os = "android")]
    pub fn run<C: EmbedderClient + 'static>(
        self,
        start: impl FnOnce(inset_embedder::PlatformRef) -> C + Send + 'static,
    ) {
        let app = self
            .app
            .expect("DefaultEmbedder::android_app: the AndroidApp android_main received");
        inset_embedder_android::AndroidEmbedder::new(app).run(start);
    }
}
