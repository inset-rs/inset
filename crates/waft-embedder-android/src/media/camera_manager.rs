//! Android's camera manager, the NDK's `ACameraManager` (`camera/NdkCameraManager.h`), which
//! neither `ndk` nor `ndk-sys` links: the ids of the cameras attached, each one's
//! characteristics, opening one, and hearing the cameras come and go and be taken and let go of
//! by apps, which its availability callbacks tell on a thread of the camera service's.
//!
//! One manager for the process, made at its first use and kept: the availability callbacks it
//! calls hold a context that must outlive every call, and only a manager that never goes makes
//! sure of that.

// The NDK's C API is `unsafe` to call; this module writes it for its callers.
#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_char, c_void};
use std::ptr::{self, NonNull};
use std::sync::{Mutex, OnceLock, PoisonError};

use ndk_sys::{
    ACameraIdList, ACameraManager, ACameraManager_AvailabilityCallbacks, ACameraManager_create,
    ACameraManager_deleteCameraIdList, ACameraManager_getCameraCharacteristics,
    ACameraManager_getCameraIdList, ACameraManager_registerAvailabilityCallback, ACameraMetadata,
    camera_status_t,
};

use super::camera_device::{CameraDevice, DeviceEvents};
use super::camera_metadata::CameraMetadata;

// `libcamera2ndk`, where the NDK's camera calls are; `ndk-sys` declares them and links no library.
#[link(name = "camera2ndk")]
unsafe extern "C" {}

/// The process's camera manager.
pub(super) struct CameraManager {
    raw: NonNull<ACameraManager>,
    /// What is told when the cameras' availability changes, once listening started.
    heard: Mutex<Vec<Box<dyn Fn() + Send + Sync>>>,
}

// SAFETY: the NDK's camera manager guards its own state, as Java's `CameraManager` is made to be
// called from any thread.
unsafe impl Send for CameraManager {}
unsafe impl Sync for CameraManager {}

impl CameraManager {
    /// The process's manager, made at the first call; `None` where the system made none.
    pub(super) fn get() -> Option<&'static CameraManager> {
        static MANAGER: OnceLock<Option<CameraManager>> = OnceLock::new();
        MANAGER
            .get_or_init(|| {
                // SAFETY: no arguments; the manager is kept for the process.
                let raw = NonNull::new(unsafe { ACameraManager_create() })?;
                Some(CameraManager {
                    raw,
                    heard: Mutex::new(Vec::new()),
                })
            })
            .as_ref()
    }

    /// The ids of the cameras attached now, in the system's order.
    pub(super) fn camera_ids(&self) -> Vec<String> {
        let mut list: *mut ACameraIdList = ptr::null_mut();
        // SAFETY: a live manager and a place for the list, which is freed below.
        let status = unsafe { ACameraManager_getCameraIdList(self.raw.as_ptr(), &mut list) };
        if status != camera_status_t::ACAMERA_OK || list.is_null() {
            return Vec::new();
        }
        // SAFETY: the list holds `numCameras` nul-terminated ids, live until it is freed.
        let ids = unsafe {
            let list = &*list;
            (0..list.numCameras.max(0) as usize)
                .map(|index| {
                    CStr::from_ptr(*list.cameraIds.add(index))
                        .to_string_lossy()
                        .into_owned()
                })
                .collect()
        };
        // SAFETY: the list the manager made, freed once.
        unsafe { ACameraManager_deleteCameraIdList(list) };
        ids
    }

    /// The characteristics of the camera `id`; `None` for a camera not attached.
    pub(super) fn characteristics(&self, id: &str) -> Option<CameraMetadata> {
        let id = CString::new(id).ok()?;
        let mut metadata: *mut ACameraMetadata = ptr::null_mut();
        // SAFETY: a live manager, a nul-terminated id and a place for the metadata, which the
        // caller frees.
        let status = unsafe {
            ACameraManager_getCameraCharacteristics(self.raw.as_ptr(), id.as_ptr(), &mut metadata)
        };
        if status != camera_status_t::ACAMERA_OK {
            return None;
        }
        // SAFETY: the metadata the manager made for the caller.
        Some(unsafe { CameraMetadata::from_owned(NonNull::new(metadata)?) })
    }

    /// Opens the camera `id`, which tells `events` when it is disconnected or fails.
    pub(super) fn open(
        &self,
        id: &str,
        events: DeviceEvents,
    ) -> Result<CameraDevice, camera_status_t> {
        CameraDevice::open(self.raw, id, events)
    }

    /// Calls `heard` each time a camera comes, goes, or is taken or let go of by an app, on a
    /// thread of the camera service's, from now on for the process; the first call also starts
    /// listening, which Android answers with each camera available now.
    pub(super) fn on_availability_change(&'static self, heard: impl Fn() + Send + Sync + 'static) {
        let mut listening = self.heard.lock().unwrap_or_else(PoisonError::into_inner);
        let first = listening.is_empty();
        listening.push(Box::new(heard));
        drop(listening);
        if first {
            self.listen();
        }
    }

    /// Registers the availability callbacks, whose context is this manager, which lives for the
    /// process.
    fn listen(&'static self) {
        let callbacks = ACameraManager_AvailabilityCallbacks {
            context: ptr::from_ref(self).cast_mut().cast(),
            onCameraAvailable: Some(availability_changed),
            onCameraUnavailable: Some(availability_changed),
        };
        // SAFETY: a live manager, which copies the callbacks; their context lives for the
        // process.
        let status =
            unsafe { ACameraManager_registerAvailabilityCallback(self.raw.as_ptr(), &callbacks) };
        if status != camera_status_t::ACAMERA_OK {
            crate::log::warn_to_log!(
                "waft: the system would not tell when cameras come and go: {}",
                status.0
            );
        }
    }

    fn tell_heard(&self) {
        for heard in self
            .heard
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
        {
            heard();
        }
    }
}

/// A camera came, went, or was taken or let go of.
unsafe extern "C" fn availability_changed(context: *mut c_void, _camera_id: *const c_char) {
    super::abort_on_panic(|| {
        // SAFETY: the context is the process's manager, which lives for the process.
        let manager = unsafe { &*context.cast::<CameraManager>() };
        manager.tell_heard();
    });
}
