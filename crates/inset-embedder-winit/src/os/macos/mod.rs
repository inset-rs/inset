//! What AppKit adds to winit, one concern per module: how a window moves and the screens it can
//! be on (`screens`), what its configuration asks beyond winit's attributes (`window_chrome`), how
//! a frame reaches the display (`frame_presentation`, `display_links`) and how a wait on a
//! screen's vsync is served (`vsync_waiter`), the pointer (`pointer`), native surfaces
//! (`composition`), whether a window is visible, for the app (`visibility`), the menus: the menu
//! bar, pop-up menus, the Dock icon's menu and the status items (`menus`), work off the main thread (`work_queue`), what AppKit asks of the application
//! through its delegate, how it quits and hiding it (`app_delegate`), what it says of the
//! application's activation and visibility (`app_lifecycle`), and what it says changed about the
//! screens and the user's languages (`system_changes`).
//!
//! A panic must not unwind from the embedder into AppKit, whose frames were never built to be
//! unwound: it aborts the process once the panic hook has printed it, as one does in winit's own
//! methods. A function AppKit or the main dispatch queue calls is `extern "C"`, where Rust aborts
//! on a panic by itself: the delegate methods the embedder adds, and dispatch2's wrapper of work on
//! a queue. A method `define_class!` makes is `extern "C-unwind"`, so its body runs through
//! [`abort_on_panic`]. Work on Grand Central Dispatch's worker threads is the one exception
//! (`work_queue`).

// AppKit's selectors and view hierarchy need `unsafe`; this module and its children are
// the one place in the embedder that writes it.
#![allow(unsafe_code)]

mod app_delegate;
mod app_lifecycle;
mod composition;
mod display_links;
mod file_dialogs;
mod frame_presentation;
mod menus;
mod opener;
mod pasteboard;
mod pointer;
mod screens;
mod system_changes;
mod visibility;
mod vsync_waiter;
mod window_chrome;
mod work_queue;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process;

use inset_embedder::TargetPlatform;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSView, NSWindow};
use valo_codec::Decoder;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use super::apple::image_io_decoders;
use super::{ImageDecoding, OperatingSystem, WindowGeometry};

/// macOS, as AppKit answers. A type only: never made, its answers are associated functions.
pub(crate) enum AppKit {}

impl OperatingSystem for AppKit {
    const TARGET_PLATFORM: TargetPlatform = TargetPlatform::MacOS;
}

/// winit's sizes are AppKit's, and a desktop window has no safe area.
impl WindowGeometry for AppKit {}

impl ImageDecoding for AppKit {
    fn system_decoders() -> Vec<Box<dyn Decoder>> {
        image_io_decoders()
    }
}

/// The window's content view; `None` off the main thread, where AppKit's views may not be
/// touched.
fn ns_view(window: &Window) -> Option<Retained<NSView>> {
    MainThreadMarker::new()?;
    let handle = window.window_handle().ok()?;
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return None;
    };
    // SAFETY: winit's AppKit handle is the NSView of a live window, and this is the main
    // thread, checked above.
    unsafe { Retained::retain(appkit.ns_view.cast::<NSView>().as_ptr()) }
}

fn ns_window(window: &Window) -> Option<Retained<NSWindow>> {
    ns_view(window)?.window()
}

/// Runs `body`, the body of a method AppKit calls, and aborts the process if it panics, once the
/// panic hook has printed the panic.
fn abort_on_panic<R>(body: impl FnOnce() -> R) -> R {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or_else(|_| process::abort())
}
