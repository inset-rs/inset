//! Cameras on Android: the NDK's own camera API, with no Java but the permission's question. The
//! cameras are the camera manager's (`camera_manager`), each described by its characteristics
//! (`camera_metadata`, `camera_formats`) and listed and opened as Waft's `Cameras` and
//! `CameraDevice` (`cameras`); a camera starts, on a thread of its own (`camera_queue`), as an
//! `ACameraDevice` with a capture session (`camera_device`) feeding image readers
//! (`camera_images`), opened once for this process's sessions and shared by them
//! (`running_camera`, `capture_session`). Its pictures, in the camera's own format, go as they are
//! to a preview's composition layer, turned upright (`preview`, `orientation`), and drawn into
//! RGBA8 frames for a frame reader (`camera_frames`, `frame_conversion`, `frames`,
//! `video_frame`); its photos are JPEG from the camera's own stream (`photos`). What changed is
//! told to the app from whichever thread Android tells it on (`changes`).
//!
//! The invariant: nothing here waits on the app's thread for Android; what Android tells on its
//! threads is handed to the app through futures and change futures, which wake the app's loop.

#[cfg(target_os = "android")]
mod camera_device;
mod camera_formats;
#[cfg(target_os = "android")]
mod camera_frames;
#[cfg(target_os = "android")]
mod camera_images;
#[cfg(target_os = "android")]
mod camera_manager;
#[cfg(target_os = "android")]
mod camera_metadata;
#[cfg(target_os = "android")]
mod camera_queue;
#[cfg(target_os = "android")]
mod cameras;
#[cfg(target_os = "android")]
mod capture_session;
mod changes;
#[cfg(target_os = "android")]
mod frame_conversion;
#[cfg(target_os = "android")]
mod frames;
mod orientation;
#[cfg(target_os = "android")]
mod photos;
#[cfg(target_os = "android")]
mod preview;
#[cfg(target_os = "android")]
mod running_camera;
#[cfg(target_os = "android")]
mod video_frame;

#[cfg(target_os = "android")]
use std::panic::{AssertUnwindSafe, catch_unwind};

#[cfg(target_os = "android")]
pub(crate) use cameras::NdkCameras;
#[cfg(target_os = "android")]
pub(crate) use frames::NdkFrameReaders;
#[cfg(target_os = "android")]
pub use video_frame::AndroidVideoFrame;

/// The screen shows its picture turned `degrees` clockwise from now on: the cameras' sessions hear
/// their rotation angles change.
#[cfg(target_os = "android")]
pub(crate) fn display_rotated(degrees: u32) {
    if orientation::display_rotated(degrees) {
        running_camera::tell_turned();
    }
}

/// The app came to the front: a camera another app took meanwhile may be given back.
#[cfg(target_os = "android")]
pub(crate) fn app_in_front() {
    running_camera::resume_interrupted();
}

/// Runs `body`, the body of a callback the NDK calls, and aborts the process if it panics, once
/// the panic hook has printed the panic: the camera service's frames were never built to be
/// unwound.
#[cfg(target_os = "android")]
fn abort_on_panic<R>(body: impl FnOnce() -> R) -> R {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or_else(|_| std::process::abort())
}
