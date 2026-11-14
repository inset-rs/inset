//! A camera session's rotation angles: Apple's `AVCaptureDeviceRotationCoordinator` for the
//! session's device and its preview layer, whose two angles, one that keeps the preview level and
//! one that keeps what is captured level, the session reports, and keeps applied as the device
//! turns: the preview's to the preview layer's connection, which turns what it shows, and, where
//! the session asked for upright frames, the capture's to its video data output's connection,
//! which turns the frames it delivers (`capture_session`). A frame delivered unturned reports what
//! of the capture's angle its connection did not apply, and a mirror for a camera that looks at
//! its user where its connection did not mirror it (`frames`).
//!
//! The coordinator exists from macOS 14 and iOS 17; before, both angles are 0 and frames cannot be
//! asked upright. The coordinator tells its angles' changes on the main queue.
//!
//! The invariant: the angles read on any thread are the coordinator's last, and a connection is
//! given an angle only where it says it supports it.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use objc2::AllocAnyThread;
use objc2::rc::Retained;
use objc2::runtime::AnyClass;
use objc2_av_foundation::{
    AVCaptureConnection, AVCaptureDevice, AVCaptureDeviceRotationCoordinator,
};
use objc2_quartz_core::CALayer;
use waft_embedder::CameraError;

use super::observing::PropertyObservation;

/// The angles, in degrees clockwise, that keep a camera's picture level, read from any thread.
#[derive(Default)]
pub(super) struct RotationAngles {
    preview: AtomicU32,
    capture: AtomicU32,
}

impl RotationAngles {
    /// The turn that keeps the preview level.
    pub(super) fn preview(&self) -> u32 {
        self.preview.load(Ordering::Relaxed)
    }

    /// The turn that keeps what is captured level.
    pub(super) fn capture(&self) -> u32 {
        self.capture.load(Ordering::Relaxed)
    }
}

/// A session's rotation coordinator, observed while it lives.
pub(super) struct SessionRotation {
    angles: Arc<RotationAngles>,
    /// The coordinator and the observation of its angles; none before macOS 14 and iOS 17.
    _coordinated: Option<(
        Retained<AVCaptureDeviceRotationCoordinator>,
        PropertyObservation,
    )>,
}

impl SessionRotation {
    /// The coordinator of `device` and `preview_layer`, where the system has one, read into
    /// `angles` now and at each change, after which `turned` runs with them. On the main thread.
    pub(super) fn new(
        device: &AVCaptureDevice,
        preview_layer: Option<&CALayer>,
        angles: Arc<RotationAngles>,
        turned: impl Fn(&RotationAngles) + 'static,
    ) -> SessionRotation {
        if !coordinator_exists() {
            return SessionRotation {
                angles,
                _coordinated: None,
            };
        }
        // SAFETY: a coordinator of a video device and a layer, both live, made on the main thread
        // whose queue it tells its changes on.
        let coordinator = unsafe {
            AVCaptureDeviceRotationCoordinator::initWithDevice_previewLayer(
                AVCaptureDeviceRotationCoordinator::alloc(),
                device,
                preview_layer,
            )
        };
        read_angles(&coordinator, &angles);
        turned(&angles);
        let observed = Arc::clone(&angles);
        let read = coordinator.clone();
        let observation = PropertyObservation::calling(
            &coordinator,
            &[
                "videoRotationAngleForHorizonLevelPreview",
                "videoRotationAngleForHorizonLevelCapture",
            ],
            move || {
                read_angles(&read, &observed);
                turned(&observed);
            },
        );
        SessionRotation {
            angles,
            _coordinated: Some((coordinator, observation)),
        }
    }

    pub(super) fn angles(&self) -> &Arc<RotationAngles> {
        &self.angles
    }
}

/// Reads `coordinator`'s angles into `angles`.
fn read_angles(coordinator: &AVCaptureDeviceRotationCoordinator, angles: &RotationAngles) {
    // SAFETY: reading the coordinator's angles, on the main thread it tells them on.
    let (preview, capture) = unsafe {
        (
            coordinator.videoRotationAngleForHorizonLevelPreview(),
            coordinator.videoRotationAngleForHorizonLevelCapture(),
        )
    };
    angles
        .preview
        .store(quarter_turns_of(preview), Ordering::Relaxed);
    angles
        .capture
        .store(quarter_turns_of(capture), Ordering::Relaxed);
}

/// Whether the system has a rotation coordinator: macOS 14 and iOS 17 and later.
pub(super) fn coordinator_exists() -> bool {
    AnyClass::get(c"AVCaptureDeviceRotationCoordinator").is_some()
}

/// Turns what `connection` carries by `angle`, where it supports it: as a preview layer's, what
/// the layer shows. On the thread its session is configured or shown on.
pub(super) fn turn_connection(connection: &AVCaptureConnection, angle: u32) {
    // SAFETY: an angle the connection says it supports, of 0, 90, 180 or 270.
    unsafe {
        if connection.isVideoRotationAngleSupported(f64::from(angle)) {
            connection.setVideoRotationAngle(f64::from(angle));
        }
    }
}

/// Has `connection`, a video data output's, deliver its frames turned by `angle` and mirrored
/// where `mirrored` says, so that they show as they are; `NotSupported` where it cannot. Inside the
/// session's configuration, on its queue, as Apple asks of a turn that reconfigures the pipeline.
pub(super) fn deliver_upright(
    connection: &AVCaptureConnection,
    angle: u32,
    mirrored: bool,
) -> Result<(), CameraError> {
    // SAFETY: reading what the connection supports, and setting only what it does.
    unsafe {
        let turns =
            coordinator_exists() && connection.isVideoRotationAngleSupported(f64::from(angle));
        if !turns {
            return Err(CameraError::NotSupported(
                "the camera's connection does not turn its frames on this system".to_owned(),
            ));
        }
        if mirrored && !connection.isVideoMirroringSupported() {
            return Err(CameraError::NotSupported(
                "the camera's connection does not mirror its frames".to_owned(),
            ));
        }
        connection.setVideoRotationAngle(f64::from(angle));
        if connection.isVideoMirroringSupported() {
            connection.setAutomaticallyAdjustsVideoMirroring(false);
            connection.setVideoMirrored(mirrored);
        }
    }
    Ok(())
}

/// How a frame `connection` delivered shows: the turn of `capture_angle` it did not apply, and a
/// mirror for a camera that `faces_user` where it did not mirror the frame, or the other way.
pub(super) fn shown_orientation(
    connection: &AVCaptureConnection,
    capture_angle: u32,
    faces_user: bool,
) -> (u32, bool) {
    // SAFETY: reading the connection's angle and mirroring, which AVFoundation reads from any
    // thread.
    let (applied, mirrored) = unsafe {
        let applied = if coordinator_exists() {
            quarter_turns_of(connection.videoRotationAngle())
        } else {
            0
        };
        (applied, connection.isVideoMirrored())
    };
    left_to_turn(capture_angle, applied, faces_user, mirrored)
}

/// The turn of `wanted` not yet `applied`, and whether a mirror is still owed, for a camera that
/// `faces_user`, whose frame is `mirrored` already or not.
fn left_to_turn(wanted: u32, applied: u32, faces_user: bool, mirrored: bool) -> (u32, bool) {
    ((wanted + 360 - applied % 360) % 360, faces_user != mirrored)
}

/// `angle`, in degrees, as one of 0, 90, 180 and 270, the nearest quarter turn.
fn quarter_turns_of(angle: f64) -> u32 {
    ((angle / 90.0).round().rem_euclid(4.0) as u32) * 90
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_angle_is_the_nearest_quarter_turn() {
        assert_eq!(quarter_turns_of(0.0), 0);
        assert_eq!(quarter_turns_of(89.9), 90);
        assert_eq!(quarter_turns_of(270.0), 270);
        assert_eq!(quarter_turns_of(360.0), 0);
        assert_eq!(quarter_turns_of(-90.0), 270);
    }

    /// A phone's back camera held upright, its connection turning nothing: the frame shows turned a
    /// quarter. Its front camera, its connection neither turning nor mirroring: turned and mirrored.
    /// A connection that turned and mirrored it already: nothing is left.
    #[test]
    fn a_frame_shows_turned_by_what_its_connection_did_not_do() {
        assert_eq!(left_to_turn(90, 0, false, false), (90, false));
        assert_eq!(left_to_turn(90, 0, true, false), (90, true));
        assert_eq!(left_to_turn(90, 90, true, true), (0, false));
        assert_eq!(left_to_turn(0, 180, false, false), (180, false));
    }
}
