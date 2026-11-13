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

mod answer;
mod app_exit;
mod app_requests;
mod appearance;
mod clipboard;
mod dispatcher;
mod drag_drop;
mod file_dialogs;
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
// Only the systems with players and cameras use what their media share.
#[cfg(any(target_os = "macos", target_os = "ios"))]
mod media;
mod menus;
mod mouse_cursor;
mod notifications;
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
mod trackpad;
mod video_frames;
mod view;
mod view_focus;
mod view_host;
mod views;
mod vsync_source;
mod vsync_waiter;
mod windows;

use waft_embedder::{EmbedderClient, PlatformRef};

/// The system's players, media assets, image codecs, frame readers, compositor and hardware
/// buffers without the event loop, for a test that serves the main thread itself, as AVFoundation
/// changes a player's state on the main queue. Not an API.
#[cfg(feature = "test-support")]
#[doc(hidden)]
pub mod test_support {
    use std::rc::Rc;
    use std::sync::Arc;

    use waft_embedder::image_codecs::ImageCodecs;
    use waft_embedder::{
        Compositor, HardwareBufferAllocator, MediaAssets, Players, VideoFrameReaders,
    };

    use crate::os::System;
    use crate::os::capabilities::{NativeComposition, SystemImageCodecs, SystemMedia};

    /// The system's players, as `Platform::players` answers them; on the main thread.
    pub fn players() -> Option<Rc<dyn Players>> {
        System::players()
    }

    /// The system's reading of media without playing it, as `Platform::media_assets` answers it.
    pub fn media_assets() -> Option<Rc<dyn MediaAssets>> {
        System::media_assets()
    }

    /// The system's image codecs, as `Platform::image_codecs` answers them.
    pub fn image_codecs() -> Option<Rc<dyn ImageCodecs>> {
        System::image_codecs()
    }

    /// The system's frame readers, as `Platform::video_frame_readers` answers them.
    pub fn video_frame_readers() -> Option<Rc<dyn VideoFrameReaders>> {
        System::video_frame_readers()
    }

    /// The system's compositor, as `Platform::compositor` answers it.
    pub fn compositor() -> Option<Arc<dyn Compositor>> {
        System::compositor()
    }

    /// The system's hardware buffer allocator, as `Platform::hardware_buffer_allocator` answers
    /// it.
    pub fn hardware_buffer_allocator() -> Option<Arc<dyn HardwareBufferAllocator>> {
        System::hardware_buffer_allocator()
    }
}

pub use images::{DecodeExecution, create_image_loader};
pub use macos::MacOs;
#[cfg(target_os = "linux")]
pub use os::DmabufBuffer;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use os::{CoreVideoFrame, SurfaceBuffer};
pub use platform::WinitPlatform;
pub use system_requests::{Reopen, ReopenHandler};

/// Runs the winit event loop until quit. The app decides what happens when its last window
/// closes, as it decides every window's close.
#[derive(Default)]
pub struct WinitEmbedder {
    linux: LinuxSettings,
    force_fallback_adapter: bool,
}

/// What the app tells Linux's desktops about itself, which no other system asks of it.
#[derive(Clone, Debug, Default)]
pub struct LinuxSettings {
    /// The app's id, by which the desktop matches its windows to its desktop file,
    /// `<app_id>.desktop`: each window's Wayland app id (xdg-shell's `set_app_id`) and its X11
    /// class (`WM_CLASS`). `#[waft::main]` gives `[package.metadata.waft] identifier`, as
    /// `cargo waft` names the desktop file. Without it a window is of no app the desktop knows.
    pub app_id: Option<String>,
}

impl WinitEmbedder {
    /// Whether the device every window draws with is on the system's software adapter, WebGPU's
    /// `forceFallbackAdapter`: lavapipe or llvmpipe on Linux, WARP on Windows, where the system has
    /// one; no device otherwise. Off by default, which takes a hardware adapter wherever there is
    /// one, at the best level Waft draws at on it, before a software one.
    pub fn force_fallback_adapter(mut self, force: bool) -> WinitEmbedder {
        self.force_fallback_adapter = force;
        self
    }

    /// What the app tells Linux's desktops; ignored on every other system.
    pub fn linux(mut self, settings: LinuxSettings) -> WinitEmbedder {
        self.linux = settings;
        self
    }

    /// Sends the app's standard output and error to the terminal it was run from: on Windows, where
    /// a GUI program (`#![windows_subsystem = "windows"]`) starts with no console, the console of the
    /// process that started it, when that process has one. Called first in `main`, before anything is
    /// written. Elsewhere they are there already, and nothing changes.
    pub fn attach_to_parent_console() {
        <os::System as os::capabilities::StandardStreams>::attach_to_parent_console();
    }

    /// Runs the app until it quits. `start` runs first, before winit's loop, and returns the
    /// client; where the system's window is the screen, the screen view is already among the
    /// platform's views then, with no window yet, which opens at winit's first call.
    pub fn run<C: EmbedderClient + 'static>(self, start: impl FnOnce(PlatformRef) -> C + 'static) {
        launch::run(
            Box::new(start),
            None,
            self.linux,
            self.force_fallback_adapter,
        );
    }

    /// Starts the native loop with an image loader of the application's choosing.
    ///
    /// The factory receives the renderer's image store when the app first decodes an image or
    /// brings one, on the device every view draws with. Use
    /// `create_image_loader(images, DecodeExecution::Local)` to decode without a worker
    /// thread, or build a `valo_codec::ImageLoader` with decoders and an order of your own.
    pub fn run_with_image_loader<C: EmbedderClient + 'static>(
        self,
        make_loader: impl FnOnce(valo::ImageContext) -> valo_codec::ImageLoader + 'static,
        start: impl FnOnce(PlatformRef) -> C + 'static,
    ) {
        launch::run(
            Box::new(start),
            Some(Box::new(make_loader)),
            self.linux,
            self.force_fallback_adapter,
        );
    }
}
