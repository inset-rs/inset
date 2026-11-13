//! The host an application gets without choosing one: winit on the desktops and iOS, the
//! activity's own loop on Android, the canvas WebGPU host in the browser.
//!
//! An application depends on this crate instead of picking an embedder and repeating
//! target-cfg dependencies. `DefaultEmbedder::run` is the same closure on all of them; what
//! differs is what each host needs before it can start, which is nothing on the desktops, a
//! canvas in the browser, and the `AndroidApp` Waft's Android host hands `android_main`, once a
//! process.
//!
//! A `wasm32-wasi` build, a component for a WASI host, has no default host: the host's own
//! embedder crate exports the guest's start and runs the app, and this crate is empty there.
#![cfg(not(all(target_arch = "wasm32", target_os = "wasi")))]

use waft_embedder::EmbedderClient;

#[cfg(target_os = "android")]
pub use waft_embedder_android::AndroidApp;
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
pub use waft_embedder_winit::LinuxSettings;

/// Runs the application on this target's host.
#[derive(Default)]
pub struct DefaultEmbedder {
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    inner: waft_embedder_winit::WinitEmbedder,
    #[cfg(target_os = "android")]
    app: Option<AndroidApp>,
    #[cfg(target_arch = "wasm32")]
    inner: waft_embedder_web::WebEmbedder,
}

impl DefaultEmbedder {
    /// Canvas element id. Ignored on native.
    #[cfg(target_arch = "wasm32")]
    pub fn canvas_id(mut self, id: impl Into<String>) -> DefaultEmbedder {
        self.inner = self.inner.canvas_id(id);
        self
    }

    /// Whether every window draws on the system's software adapter, WebGPU's
    /// `forceFallbackAdapter`: lavapipe or llvmpipe on Linux, WARP on Windows. Off by default,
    /// which takes a hardware adapter wherever there is one.
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    pub fn force_fallback_adapter(mut self, force: bool) -> DefaultEmbedder {
        self.inner = self.inner.force_fallback_adapter(force);
        self
    }

    /// What the app tells Linux's desktops, its app id among it; ignored on every other system.
    /// `#[waft::main]` gives the app id from the manifest; an entry written by hand gives
    /// `waft::linux_app_id!()`.
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    pub fn linux(mut self, settings: LinuxSettings) -> DefaultEmbedder {
        self.inner = self.inner.linux(settings);
        self
    }

    /// The `AndroidApp` Waft's Android host handed `android_main`, the process's app, which the
    /// host's loop runs for. Required there, and `run` panics without it.
    #[cfg(target_os = "android")]
    pub fn android_app(mut self, app: AndroidApp) -> DefaultEmbedder {
        self.app = Some(app);
        self
    }

    /// Sends the app's standard output and error to the terminal it was run from, where the
    /// system starts a GUI program without it: on Windows, the console of the process that
    /// started the app (`WinitEmbedder::attach_to_parent_console`). `#[waft::main]`'s entry calls
    /// it first; an entry written out calls it before it writes anything. Nothing elsewhere.
    pub fn attach_to_parent_console() {
        #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
        waft_embedder_winit::WinitEmbedder::attach_to_parent_console();
    }

    /// Starts the host loop. `start` runs once, at launch, with the views the host has then,
    /// and returns the client the host drives. On Android the loop runs for the rest of the
    /// process, and this never returns.
    pub fn run<C: EmbedderClient + 'static>(
        self,
        start: impl FnOnce(waft_embedder::PlatformRef) -> C + 'static,
    ) {
        #[cfg(target_os = "android")]
        {
            let app = self
                .app
                .expect("DefaultEmbedder::android_app: the AndroidApp android_main received");
            waft_embedder_android::AndroidEmbedder::new(app).run(start);
        }
        #[cfg(not(target_os = "android"))]
        {
            self.inner.run(start);
        }
    }
}
