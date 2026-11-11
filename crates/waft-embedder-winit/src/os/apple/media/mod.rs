//! Players and cameras on macOS and iOS: AVFoundation, the system's own media stack. A player is
//! an `AVPlayer` (`player`) with its media's tracks (`tracks`), tags (`metadata`) and subtitles as
//! cues (`cues`, `cue_markup`), failing with the kinds of AVFoundation's errors (`player_errors`)
//! and timed in Core Media's times (`media_times`); media read without playing it is an
//! `AVURLAsset` (`asset`), listing its tracks and tags as a player does, its pictures an image
//! generator's (`asset_frames`), drawn into frames as a reader hands them out (`drawn_frames`), as
//! the hardware image codecs' pictures are (`apple/image_codecs`); a frame's pixels are read back
//! while its buffer is locked (`locked_pixels`); what a player's or a camera's picture on a
//! composition layer shows now is read from an output of its own beside that layer
//! (`layer_pictures`); the cameras are a discovery session's devices, and a camera a
//! capture device (`camera`) with its formats (`camera_formats`), shared in one format by this
//! process's sessions (`camera_sharing`), and a session an `AVCaptureSession` (`capture_session`)
//! with its photo and movie outputs (`outputs`); a picture goes to a composition layer as a layer
//! the system draws into, shown as its content so that Core Animation clips and scales it with it
//! (`apple/composition`), or to a frame reader the app made (`frames`), each frame an
//! `IOSurface`-backed pixel buffer (`video_frame`) whose surface is its hardware buffer, imported
//! as a texture as any other is (`apple/gpu_interop`). AVFoundation tells changes by key-value
//! observing and notifications (`observing`).
//!
//! A player and a session are controlled on the main thread, as AVFoundation asks of a player and
//! the framework asks of everything; a session runs on a queue of its own, since starting it
//! blocks. The invariant: nothing here reads a player's state off the main thread, where
//! AVFoundation changes it, so its observers only tell the app that something changed, and the
//! app reads the state when it hears.

// AVFoundation's classes and their delegates need `unsafe`; this module writes it for them.
#![allow(unsafe_code)]

mod asset;
mod asset_frames;
mod camera;
mod camera_formats;
mod camera_sharing;
mod capture_session;
mod cue_markup;
mod cues;
pub(in crate::os::apple) mod drawn_frames;
mod frames;
mod layer_pictures;
pub(in crate::os::apple) mod locked_pixels;
mod media_times;
mod metadata;
mod observing;
mod outputs;
mod player;
mod player_errors;
mod tracks;
mod video_frame;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process;
use std::rc::Rc;

use objc2::MainThreadMarker;
use waft_embedder::{Cameras, MediaAssets, Players, VideoFrameReaders};

pub(super) use super::composition::{SystemPicture, in_transaction, show_system_layer};
use asset::AvMediaAssets;
use camera::AvCameras;
use frames::AvFrameReaders;
use player::AvPlayers;
pub use video_frame::CoreVideoFrame;
#[cfg(test)]
pub(in crate::os::apple) use video_frame::test_frame;

/// AVFoundation's players; `None` off the main thread, where none is controlled.
pub(crate) fn players() -> Option<Rc<dyn Players>> {
    Some(Rc::new(AvPlayers::new(MainThreadMarker::new()?)))
}

/// AVFoundation's reading of media without playing it.
pub(crate) fn media_assets() -> Option<Rc<dyn MediaAssets>> {
    Some(Rc::new(AvMediaAssets))
}

/// AVFoundation's cameras.
pub(crate) fn cameras() -> Option<Rc<dyn Cameras>> {
    Some(Rc::new(AvCameras::default()))
}

/// The readers AVFoundation's players and cameras feed.
pub(crate) fn video_frame_readers() -> Option<Rc<dyn VideoFrameReaders>> {
    Some(Rc::new(AvFrameReaders))
}

/// A CF string as the NSString it is toll-free bridged to.
pub(in crate::os::apple) fn as_ns_string(
    string: &objc2_core_foundation::CFString,
) -> &objc2_foundation::NSString {
    let pointer: *const objc2_core_foundation::CFString = string;
    // SAFETY: a CFString is an NSString.
    unsafe { &*pointer.cast::<objc2_foundation::NSString>() }
}

/// Runs `body`, the body of a method AVFoundation calls, and aborts the process if it panics,
/// once the panic hook has printed the panic: AVFoundation's frames were never built to be
/// unwound.
pub(in crate::os::apple) fn abort_on_panic<R>(body: impl FnOnce() -> R) -> R {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or_else(|_| process::abort())
}
