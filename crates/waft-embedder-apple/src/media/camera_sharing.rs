//! The cameras this process's sessions run, each in one format, since two sessions in one process
//! use one camera at once (`experiments/2026-10-29-two-camera-sessions`) and a device has one
//! active format. The first session on a camera sets the format it picked, and the process holds
//! the device's lock while any of its sessions runs it, which Apple's documentation of
//! `lockForConfiguration` gives as the way to keep a device's settings unchanged; a later session
//! joins that format. The lock keeps a later session's preset from changing the format, but this
//! process's own `lockForConfiguration` is granted again, and a format set under it changes every
//! session's frames, so what keeps the format is that a joining session sets none of its own
//! (`experiments/2026-11-03-camera-format-sharing`). A camera whose lock another process holds
//! keeps the format it runs in, and the session joins that one.
//!
//! The invariant: a camera's entry lives exactly as long as a session here holds a share of it,
//! the lock taken for it is given back with the last share, and a share that joined a camera sets
//! no format.

use std::sync::{Mutex, MutexGuard, PoisonError};

use objc2::Message;
use objc2_av_foundation::AVCaptureDevice;
use waft_embedder::CameraFormat;

use super::camera::CrossThreadDevice;
use super::camera_formats::active_format_of;

/// The cameras this process's sessions run.
static RUNNING_CAMERAS: Mutex<Vec<RunningCamera>> = Mutex::new(Vec::new());

/// A camera this process's sessions run, and the format they run it in.
struct RunningCamera {
    id: String,
    format: CameraFormat,
    shares: usize,
    /// The device whose lock this process took to keep the format; `None` where another process
    /// holds it.
    locked: Option<CrossThreadDevice>,
}

/// A session's share of a camera running in one format. Dropped, it lets go of the camera, and of
/// its lock with the last share.
pub(super) struct CameraShare {
    id: String,
    format: CameraFormat,
    sets_format: bool,
}

impl CameraShare {
    /// A share of `device` in the format this process's sessions run it in; where none does, in
    /// `picked`, locking the device for the session to set it; or, where another process holds
    /// the device's lock, in the format the device runs in.
    pub(super) fn take(device: &AVCaptureDevice, picked: &CameraFormat) -> CameraShare {
        // SAFETY: reading a device's id.
        let id = unsafe { device.uniqueID() }.to_string();
        let mut running = running_cameras();
        if let Some(camera) = running.iter_mut().find(|camera| camera.id == id) {
            camera.shares += 1;
            return CameraShare {
                id,
                format: camera.format.clone(),
                sets_format: false,
            };
        }
        // SAFETY: asking for the device's lock, given back as the last share goes.
        let (format, locked) = match unsafe { device.lockForConfiguration() } {
            Ok(()) => (picked.clone(), Some(CrossThreadDevice(device.retain()))),
            Err(_) => (active_format_of(device), None),
        };
        let sets_format = locked.is_some();
        running.push(RunningCamera {
            id: id.clone(),
            format: format.clone(),
            shares: 1,
            locked,
        });
        CameraShare {
            id,
            format,
            sets_format,
        }
    }

    /// The format the camera runs in for this share's session.
    pub(super) fn format(&self) -> &CameraFormat {
        &self.format
    }

    /// Whether the session sets the camera's format: the first share, which took the lock.
    pub(super) fn sets_format(&self) -> bool {
        self.sets_format
    }
}

impl Drop for CameraShare {
    fn drop(&mut self) {
        let mut running = running_cameras();
        let Some(index) = running.iter().position(|camera| camera.id == self.id) else {
            return;
        };
        running[index].shares -= 1;
        if running[index].shares > 0 {
            return;
        }
        let camera = running.swap_remove(index);
        if let Some(device) = camera.locked {
            // SAFETY: giving back the lock `take` took.
            unsafe { device.0.unlockForConfiguration() };
        }
    }
}

fn running_cameras() -> MutexGuard<'static, Vec<RunningCamera>> {
    RUNNING_CAMERAS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}
