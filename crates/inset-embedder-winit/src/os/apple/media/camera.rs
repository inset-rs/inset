//! Cameras: AVFoundation's capture devices, listed by a discovery session, which asks the user
//! nothing, and opened once the user lets the app use the cameras, asking where the user has not
//! answered. An opened device lists its formats (`camera_formats`) and starts a session
//! (`capture_session`) in the format it shares with this process's other sessions on it
//! (`camera_sharing`).

use std::sync::{Mutex, PoisonError};

use block2::RcBlock;
use inset_embedder::{
    Camera, CameraDevice, CameraError, CameraFacing, CameraFormat, CameraId, CameraOpenFuture,
    CameraOutputs, Cameras, CaptureSession, MediaError,
};
use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2_av_foundation::{
    AVAuthorizationStatus, AVCaptureDevice, AVCaptureDeviceDiscoverySession,
    AVCaptureDevicePosition, AVCaptureDeviceType, AVCaptureDeviceTypeBuiltInWideAngleCamera,
    AVMediaType, AVMediaTypeAudio, AVMediaTypeVideo,
};
use objc2_foundation::{NSArray, NSString};

use super::camera_formats::formats_of;
use super::camera_sharing::CameraShare;
use super::capture_session::AvCaptureSession;
use crate::media::media_answer;

/// AVFoundation's cameras.
pub(crate) struct AvCameras;

impl Cameras for AvCameras {
    fn cameras(&self) -> Vec<Camera> {
        discovered_devices()
            .iter()
            .map(|device| camera_of(&device))
            .collect()
    }

    fn open(&self, camera: &Camera) -> CameraOpenFuture {
        // SAFETY: looking a device up by the id the system gave it.
        let device =
            unsafe { AVCaptureDevice::deviceWithUniqueID(&NSString::from_str(&camera.id.0)) };
        let Some(device) = device else {
            return Box::pin(std::future::ready(Err(CameraError::Unplugged)));
        };
        let (answer, answered) = media_answer::<bool>();
        let answer = Mutex::new(Some(answer));
        ask_for(video(), move |allowed| {
            let answer = answer.lock().unwrap_or_else(PoisonError::into_inner).take();
            if let Some(answer) = answer {
                answer.give(Ok(allowed));
            }
        });
        Box::pin(async move {
            match answered.await {
                Ok(true) => Ok(Box::new(AvCameraDevice(device)) as Box<dyn CameraDevice>),
                Ok(false) => Err(CameraError::NotAllowed),
                Err(MediaError::Failed(reason)) => Err(CameraError::Failed(reason)),
                Err(other) => Err(CameraError::Failed(other.to_string())),
            }
        })
    }
}

/// A device the app may use.
struct AvCameraDevice(Retained<AVCaptureDevice>);

impl CameraDevice for AvCameraDevice {
    fn formats(&self) -> Vec<CameraFormat> {
        formats_of(&self.0)
    }

    fn start(
        &self,
        format: &CameraFormat,
        outputs: CameraOutputs,
    ) -> Result<Box<dyn CaptureSession>, CameraError> {
        if !is_allowed(video()) {
            return Err(CameraError::NotAllowed);
        }
        // SAFETY: reading whether the device is still attached.
        if !unsafe { self.0.isConnected() } {
            return Err(CameraError::Unplugged);
        }
        if !formats_of(&self.0).contains(format) {
            return Err(CameraError::Failed(
                "the camera does not list this format".to_owned(),
            ));
        }
        let share = CameraShare::take(&self.0, format);
        Ok(Box::new(AvCaptureSession::start(
            self.0.clone(),
            share,
            outputs,
        )))
    }
}

/// The video devices an app may use as cameras: a device's built-in ones, and on macOS the ones
/// plugged in or on another device.
pub(super) fn discovered_devices() -> Retained<NSArray<AVCaptureDevice>> {
    // SAFETY: AVFoundation's constant, which it exports for the life of the process.
    let built_in = unsafe { AVCaptureDeviceTypeBuiltInWideAngleCamera };
    let types: Vec<&AVCaptureDeviceType> = std::iter::once(built_in)
        .chain(external_device_types())
        .collect();
    let types = NSArray::from_slice(&types);
    // SAFETY: a discovery session of these types, for video.
    unsafe {
        AVCaptureDeviceDiscoverySession::discoverySessionWithDeviceTypes_mediaType_position(
            &types,
            AVMediaTypeVideo,
            AVCaptureDevicePosition::Unspecified,
        )
        .devices()
    }
}

/// The type of the cameras plugged in or on another device, which macOS lists.
#[cfg(target_os = "macos")]
fn external_device_types() -> Option<&'static AVCaptureDeviceType> {
    // `AVCaptureDeviceTypeExternal` succeeds it from macOS 14 with the same value, and would be
    // linked strongly, so that an app could not start on an earlier macOS.
    // SAFETY: AVFoundation's constant, which it exports for the life of the process.
    #[allow(deprecated)]
    Some(unsafe { objc2_av_foundation::AVCaptureDeviceTypeExternalUnknown })
}

/// iOS lists a device's own cameras alone.
#[cfg(not(target_os = "macos"))]
fn external_device_types() -> Option<&'static AVCaptureDeviceType> {
    None
}

/// `device` as the app knows it.
fn camera_of(device: &AVCaptureDevice) -> Camera {
    // SAFETY: reading a device's id, name, place and type.
    let (id, name, position, built_in) = unsafe {
        (
            device.uniqueID().to_string(),
            device.localizedName().to_string(),
            device.position(),
            &*device.deviceType() == AVCaptureDeviceTypeBuiltInWideAngleCamera,
        )
    };
    Camera {
        id: CameraId(id),
        name,
        facing: facing_of(position, built_in),
    }
}

/// Which way a device at `position` looks: a Mac's built-in camera has no position, and looks at
/// the user.
fn facing_of(position: AVCaptureDevicePosition, built_in: bool) -> CameraFacing {
    if position == AVCaptureDevicePosition::Front {
        CameraFacing::Front
    } else if position == AVCaptureDevicePosition::Back {
        CameraFacing::Back
    } else if built_in {
        CameraFacing::Front
    } else {
        CameraFacing::External
    }
}

/// Whether the user let the app use `media`.
fn is_allowed(media: &'static AVMediaType) -> bool {
    // SAFETY: reading the app's access to a kind of device.
    let status = unsafe { AVCaptureDevice::authorizationStatusForMediaType(media) };
    status == AVAuthorizationStatus::Authorized
}

/// Calls `answered` with whether the app may use `media`: at once where the user answered, after
/// asking where they have not, on whatever thread the system answers on.
pub(super) fn ask_for(
    media: &'static AVMediaType,
    answered: impl Fn(bool) + Send + Sync + 'static,
) {
    // SAFETY: reading the app's access to a kind of device.
    let status = unsafe { AVCaptureDevice::authorizationStatusForMediaType(media) };
    if status == AVAuthorizationStatus::Authorized {
        answered(true);
    } else if status == AVAuthorizationStatus::NotDetermined {
        let handler = RcBlock::new(move |granted: Bool| answered(granted.as_bool()));
        // SAFETY: asking the user, with a handler the system calls once.
        unsafe { AVCaptureDevice::requestAccessForMediaType_completionHandler(media, &handler) };
    } else {
        answered(false);
    }
}

fn video() -> &'static AVMediaType {
    // SAFETY: a constant AVFoundation exports for the life of the process.
    unsafe { AVMediaTypeVideo }.expect("AVFoundation's video media type")
}

pub(super) fn audio() -> Option<&'static AVMediaType> {
    // SAFETY: a constant AVFoundation exports for the life of the process.
    unsafe { AVMediaTypeAudio }
}

/// A device handed between the main thread, a session's queue and the cameras this process runs.
pub(super) struct CrossThreadDevice(pub(super) Retained<AVCaptureDevice>);

// SAFETY: a capture device is configured on a session's queue, and locked and unlocked on the main
// thread; AVFoundation guards its own state, and the binding cannot say so.
unsafe impl Send for CrossThreadDevice {}
unsafe impl Sync for CrossThreadDevice {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_faces_where_it_is_placed_and_a_macs_built_in_one_faces_the_user() {
        assert_eq!(
            facing_of(AVCaptureDevicePosition::Back, true),
            CameraFacing::Back
        );
        assert_eq!(
            facing_of(AVCaptureDevicePosition::Front, true),
            CameraFacing::Front
        );
        assert_eq!(
            facing_of(AVCaptureDevicePosition::Unspecified, true),
            CameraFacing::Front
        );
        assert_eq!(
            facing_of(AVCaptureDevicePosition::Unspecified, false),
            CameraFacing::External
        );
    }

    /// Listing asks the user nothing; this Mac's cameras, if it has any, each have an id and a
    /// name.
    #[test]
    fn the_cameras_are_listed_without_asking() {
        for camera in AvCameras.cameras() {
            assert!(!camera.id.0.is_empty() && !camera.name.is_empty());
        }
    }

    /// A camera no longer attached does not open, and opening it asks nothing.
    #[test]
    fn a_camera_no_longer_attached_does_not_open() {
        let gone = Camera {
            id: CameraId("no such camera".to_owned()),
            name: "Gone".to_owned(),
            facing: CameraFacing::External,
        };
        let mut opening = AvCameras.open(&gone);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(matches!(
            opening.as_mut().poll(&mut context),
            std::task::Poll::Ready(Err(CameraError::Unplugged))
        ));
    }
}
