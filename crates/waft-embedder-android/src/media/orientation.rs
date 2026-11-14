//! How a camera's pictures are turned to show upright: Android reports how its sensor is mounted
//! (`SENSOR_ORIENTATION`, the clockwise turn that shows its pictures upright in the device's
//! natural orientation) and which way it looks, and the host knows how the screen shows its own
//! picture turned (`Display.getRotation`, kept here as the host reads it). From them come the turn
//! of a camera's preview on a layer, mirrored for a camera that looks at its user as Android's own
//! camera apps show it, and the orientation a photo is taken at (`JPEG_ORIENTATION`), which Android
//! writes into the photo's EXIF; both as CameraX computes them (`getRelativeImageRotation`).
//!
//! The same turn is the session's rotation angle, for its preview and for what it captures alike,
//! since Android tells no turn of the device apart from the screen's; and each frame a reader
//! hands out reports it as its rotation, mirrored for a camera that looks at its user, its pixels
//! left in the camera's own, as the format's size says.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::buffer_transform::BufferTransform;

/// How the screen shows its picture turned now, in degrees clockwise.
static DISPLAY_ROTATION: AtomicU32 = AtomicU32::new(0);

/// The screen shows its picture turned `degrees` clockwise from now on; answers whether that is a
/// change.
pub(super) fn display_rotated(degrees: u32) -> bool {
    DISPLAY_ROTATION.swap(degrees % 360, Ordering::Relaxed) != degrees % 360
}

/// How the screen shows its picture turned now, in degrees clockwise.
fn display_rotation() -> u32 {
    DISPLAY_ROTATION.load(Ordering::Relaxed)
}

/// How a camera's sensor is mounted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SensorMount {
    /// The clockwise turn, in degrees, that shows its pictures upright in the device's natural
    /// orientation.
    pub(super) orientation: u32,
    /// Whether it looks at the user, so its preview is shown as a mirror.
    pub(super) faces_user: bool,
}

impl SensorMount {
    /// The clockwise turn that shows its pictures upright on a screen that shows its own picture
    /// turned `display` degrees clockwise.
    fn turn_on(self, display: u32) -> u32 {
        if self.faces_user {
            (self.orientation + display) % 360
        } else {
            (self.orientation + 360 - display % 360) % 360
        }
    }

    /// The turn, in degrees clockwise, that shows its pictures upright on the screen now: the
    /// session's rotation angle, a frame's rotation, and the orientation a photo is taken at, for
    /// `JPEG_ORIENTATION`.
    pub(super) fn rotation_angle(self) -> u32 {
        self.turn_on(display_rotation())
    }

    /// How its preview is turned on the screen now: upright, and mirrored where it looks at the
    /// user.
    pub(super) fn preview_transform(self) -> BufferTransform {
        BufferTransform::turned(self.rotation_angle(), self.faces_user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A phone's back camera, mounted a quarter turn, upright when the phone is, and turned less as
    /// the screen's picture turns with the phone; its front camera, mounted three quarters, turned
    /// more.
    #[test]
    fn a_cameras_turn_follows_the_screens() {
        let back = SensorMount {
            orientation: 90,
            faces_user: false,
        };
        assert_eq!(back.turn_on(0), 90);
        assert_eq!(back.turn_on(90), 0);
        assert_eq!(back.turn_on(270), 180);
        let front = SensorMount {
            orientation: 270,
            faces_user: true,
        };
        assert_eq!(front.turn_on(0), 270);
        assert_eq!(front.turn_on(90), 0);
    }
}
