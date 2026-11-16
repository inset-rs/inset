//! What AppKit adds to winit, one concern per module: what of the application winit's event loop
//! leaves to it (`event_loop`), how a window moves and the screens it can be on (`screens`), what
//! its configuration asks beyond winit's attributes (`window_chrome`), how a frame reaches the
//! display (`frame_presentation`, `display_links`) and how a wait on a screen's vsync is served
//! (`vsync_waiter`), the pointer (`pointer`), composition layers, a window's view's own among them,
//! in its view's layer (`composition`, over the compositor iOS shares, `waft-embedder-apple`'s `composition`),
//! whether a window is visible, for the app (`visibility`), work macOS must not throttle while the
//! user waits on it (`user_activity`), the menus: the menu bar, pop-up menus,
//! the Dock icon's menu and the status items (`menus`), work off the main thread
//! (`waft-embedder-apple`'s `work_queue`),
//! what AppKit asks of the application through its delegate, how it quits and hiding it
//! (`app_delegate`), what it says of the application's activation and visibility
//! (`app_lifecycle`), and what it says changed about the screens and the user's languages
//! (`system_changes`). Notifications are the center iOS shares (`waft-embedder-apple`'s `notifications`).
//!
//! A panic must not unwind from the embedder into AppKit, whose frames were never built to be
//! unwound: it aborts the process once the panic hook has printed it, as one does in winit's own
//! methods. A function AppKit or the main dispatch queue calls is `extern "C"`, where Rust aborts
//! on a panic by itself: the delegate methods the embedder adds, and dispatch2's wrapper of work on
//! a queue. A method `define_class!` makes is `extern "C-unwind"`, so its body runs through
//! [`abort_on_panic`]. Work on Grand Central Dispatch's worker threads is the one exception
//! (`waft-embedder-apple`'s `work_queue`).

// AppKit's selectors and view hierarchy need `unsafe`; this module and its children are
// the one place in the embedder that writes it.
#![allow(unsafe_code)]

mod app_delegate;
mod app_lifecycle;
mod composition;
mod display_links;
mod event_loop;
mod file_dialogs;
mod frame_presentation;
mod menus;
mod opener;
mod pasteboard;
mod pointer;
mod screens;
mod system_changes;
mod title_bar;
mod trackpad_touches;
mod user_activity;
mod visibility;
mod vsync_waiter;
mod window_chrome;

pub use user_activity::UserActivity;

use std::rc::Rc;
use std::sync::Arc;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSView, NSWindow};
use valo_codec::Decoder;
use waft_embedder::image_codecs::ImageCodecs;
use waft_embedder::{
    Cameras, CaptionSettings, HardwareBuffer, MediaAssets, Notifications, Players, SignedDuration,
    TargetPlatform, VideoFrameReaders,
};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use super::capabilities::{
    ApplicationIdentity, GpuInterop, ImageDecoding, LocalTimeZone, OperatingSystem,
    StandardStreams, SystemImageCodecs, SystemMedia, SystemNotifications, WindowGeometry,
    WorkQueue,
};
use crate::notifications::NotificationActivations;
use waft_embedder_apple::{abort_on_panic, image_io_decoders, notifications};

/// macOS, as AppKit answers. A type only: never made, its answers are associated functions.
pub(crate) enum AppKit {}

/// On Grand Central Dispatch's worker pool, which iOS shares (`waft-embedder-apple`'s
/// `work_queue`).
impl WorkQueue for AppKit {
    fn run_off_main(work: Box<dyn FnOnce() + Send>) {
        waft_embedder_apple::work_queue::run_off_main(work);
    }
}

impl OperatingSystem for AppKit {
    const TARGET_PLATFORM: TargetPlatform = TargetPlatform::MacOS;
}

/// winit's sizes are AppKit's, and a desktop window has no safe area.
impl WindowGeometry for AppKit {}
impl StandardStreams for AppKit {}
impl ApplicationIdentity for AppKit {}

/// The C library's zone, which iOS shares.
impl LocalTimeZone for AppKit {
    fn time_zone_offset(seconds_since_epoch: i64) -> SignedDuration {
        waft_embedder_apple::local_time::time_zone_offset(seconds_since_epoch)
    }

    fn time_zone_name(seconds_since_epoch: i64) -> String {
        waft_embedder_apple::local_time::time_zone_name(seconds_since_epoch)
    }
}

/// AVFoundation's players and cameras, and Media Accessibility's caption settings, which iOS
/// shares.
impl SystemMedia for AppKit {
    fn players() -> Option<Rc<dyn Players>> {
        waft_embedder_apple::media::players()
    }

    fn media_assets() -> Option<Rc<dyn MediaAssets>> {
        waft_embedder_apple::media::media_assets()
    }

    fn cameras() -> Option<Rc<dyn Cameras>> {
        waft_embedder_apple::media::cameras()
    }

    fn video_frame_readers() -> Option<Rc<dyn VideoFrameReaders>> {
        waft_embedder_apple::media::video_frame_readers()
    }

    /// Media Accessibility's.
    fn caption_settings() -> Option<Rc<dyn CaptionSettings>> {
        waft_embedder_apple::caption_appearance::caption_settings()
    }
}

/// Image I/O's, which iOS shares.
impl SystemImageCodecs for AppKit {
    fn image_codecs() -> Option<Rc<dyn ImageCodecs>> {
        waft_embedder_apple::image_codecs::image_codecs()
    }
}

/// Metal's texture over the buffer's `IOSurface`, which iOS shares (`waft-embedder-apple`'s `gpu_interop`).
impl GpuInterop for AppKit {
    fn import_texture(buffer: &dyn HardwareBuffer, device: &wgpu::Device) -> Option<wgpu::Texture> {
        waft_embedder_apple::gpu_interop::import_texture(buffer, device)
    }
}

impl ImageDecoding for AppKit {
    fn system_decoders() -> Vec<Box<dyn Decoder>> {
        image_io_decoders()
    }
}

/// `UNUserNotificationCenter`'s, for an app that runs from its bundle (`waft-embedder-apple`'s `notifications`).
impl SystemNotifications for AppKit {
    fn hear_notification_activations(activations: NotificationActivations) {
        notifications::hear_activations(Arc::new(move |activation| activations.send(activation)));
    }

    fn notifications(_activations: NotificationActivations) -> Option<Rc<dyn Notifications>> {
        notifications::notifications()
    }
}

/// The window's content view; `None` off the main thread, where AppKit's views may not be
/// touched.
fn ns_view(window: &dyn Window) -> Option<Retained<NSView>> {
    MainThreadMarker::new()?;
    let handle = window.window_handle().ok()?;
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return None;
    };
    // SAFETY: winit's AppKit handle is the NSView of a live window, and this is the main
    // thread, checked above.
    unsafe { Retained::retain(appkit.ns_view.cast::<NSView>().as_ptr()) }
}

fn ns_window(window: &dyn Window) -> Option<Retained<NSWindow>> {
    ns_view(window)?.window()
}
