//! Players and cameras on macOS and iOS: AVFoundation, the system's own media stack. A player is
//! an `AVPlayer` (`player`), a camera session an `AVCaptureSession` (`camera`) with its photo and
//! movie outputs (`outputs`); a picture goes to a surface as a layer the system draws into, hung in
//! the surface's own layers so that Core Animation clips and scales it with them
//! (`apple/composition`), or to a frame reader as `IOSurface`-backed pixel buffers (`frames`).
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
mod frames;
mod observing;
mod outputs;
mod player;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process;
use std::rc::Rc;

use inset_embedder::{Cameras, Players};
use objc2::MainThreadMarker;

pub(super) use super::composition::{in_transaction, show_layer_on};
use camera::AvCameras;
use player::AvPlayers;

/// AVFoundation's players; `None` off the main thread, where none is controlled.
pub(crate) fn players() -> Option<Rc<dyn Players>> {
    Some(Rc::new(AvPlayers::new(MainThreadMarker::new()?)))
}

/// AVFoundation's cameras.
pub(crate) fn cameras() -> Option<Rc<dyn Cameras>> {
    Some(Rc::new(AvCameras))
}

/// Runs `body`, the body of a method AVFoundation calls, and aborts the process if it panics,
/// once the panic hook has printed the panic: AVFoundation's frames were never built to be
/// unwound.
fn abort_on_panic<R>(body: impl FnOnce() -> R) -> R {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or_else(|_| process::abort())
}
