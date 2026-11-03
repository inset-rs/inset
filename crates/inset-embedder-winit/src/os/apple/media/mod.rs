//! Players and cameras on macOS and iOS: AVFoundation, the system's own media stack. A player is
//! an `AVPlayer` (`player`) with its media's tracks (`tracks`), tags (`metadata`) and subtitles as
//! cues (`cues`, `cue_markup`); a camera a capture device (`camera`) with its formats
//! (`camera_formats`), shared in one format by this process's sessions (`camera_sharing`), and a
//! session an `AVCaptureSession` (`capture_session`) with its photo and movie outputs
//! (`outputs`); a picture goes to a composition layer as a layer the system draws into, shown as
//! its content so that Core Animation clips and scales it with it (`apple/composition`), or to a
//! frame reader the app made (`frames`), each frame an `IOSurface`-backed pixel buffer
//! (`video_frame`) whose texture CoreVideo's texture cache makes (`frame_textures`).
//! AVFoundation tells changes by key-value observing and notifications (`observing`).
//!
//! A player and a session are controlled on the main thread, as AVFoundation asks of a player and
//! the framework asks of everything; a session runs on a queue of its own, since starting it
//! blocks. The invariant: nothing here reads a player's state off the main thread, where
//! AVFoundation changes it, so its observers only tell the app that something changed, and the
//! app reads the state when it hears.

// AVFoundation's classes and their delegates need `unsafe`; this module writes it for them.
#![allow(unsafe_code)]

mod camera;
mod camera_formats;
mod camera_sharing;
mod capture_session;
mod cue_markup;
mod cues;
mod frame_textures;
mod frames;
mod metadata;
mod observing;
mod outputs;
mod player;
mod tracks;
mod video_frame;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process;
use std::rc::Rc;

use inset_embedder::{Cameras, Players, VideoFrameReaders};
use objc2::MainThreadMarker;

pub(super) use super::composition::{in_transaction, show_system_layer};
use camera::AvCameras;
use frames::AvFrameReaders;
use player::AvPlayers;
pub use video_frame::CoreVideoFrame;

/// AVFoundation's players; `None` off the main thread, where none is controlled.
pub(crate) fn players() -> Option<Rc<dyn Players>> {
    Some(Rc::new(AvPlayers::new(MainThreadMarker::new()?)))
}

/// AVFoundation's cameras.
pub(crate) fn cameras() -> Option<Rc<dyn Cameras>> {
    Some(Rc::new(AvCameras))
}

/// The readers AVFoundation's players and cameras feed.
pub(crate) fn video_frame_readers() -> Option<Rc<dyn VideoFrameReaders>> {
    Some(Rc::new(AvFrameReaders))
}

/// A CF string as the NSString it is toll-free bridged to.
fn as_ns_string(string: &objc2_core_foundation::CFString) -> &objc2_foundation::NSString {
    let pointer: *const objc2_core_foundation::CFString = string;
    // SAFETY: a CFString is an NSString.
    unsafe { &*pointer.cast::<objc2_foundation::NSString>() }
}

/// Runs `body`, the body of a method AVFoundation calls, and aborts the process if it panics,
/// once the panic hook has printed the panic: AVFoundation's frames were never built to be
/// unwound.
fn abort_on_panic<R>(body: impl FnOnce() -> R) -> R {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or_else(|_| process::abort())
}
