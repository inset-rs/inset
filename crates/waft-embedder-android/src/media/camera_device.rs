//! A camera opened, the NDK's `ACameraDevice` (`camera/NdkCameraDevice.h`), with the capture
//! session it runs (`ACameraCaptureSession`) and the requests it is given (`ACaptureRequest`):
//! a session feeds the windows it was made with, a repeating request keeps it feeding them, and
//! a single request captures once, as a still. The device tells its disconnection and its errors
//! on a thread of the camera service's.
//!
//! The invariants: a device's callbacks' context lives until the device is closed, which stops
//! its callbacks; a session's state callbacks have no context, so its asynchronous `onClosed`
//! reaches nothing that went; and a capture's context lives until Android ends its sequence,
//! completed or aborted, which it does for every capture, a session closed included.

// The NDK's C API is `unsafe` to call; this module writes it for its callers.
#![allow(unsafe_code)]

use std::ffi::{CString, c_int, c_void};
use std::ptr::{self, NonNull};
use std::sync::{Mutex, PoisonError};

use ndk::native_window::NativeWindow;
use ndk_sys::{
    ACameraCaptureFailure, ACameraCaptureSession, ACameraCaptureSession_capture,
    ACameraCaptureSession_captureCallbacks, ACameraCaptureSession_close,
    ACameraCaptureSession_setRepeatingRequest, ACameraCaptureSession_stateCallbacks, ACameraDevice,
    ACameraDevice_StateCallbacks, ACameraDevice_close, ACameraDevice_createCaptureRequest,
    ACameraDevice_createCaptureSession, ACameraDevice_request_template, ACameraManager,
    ACameraManager_openCamera, ACameraOutputTarget, ACameraOutputTarget_create,
    ACameraOutputTarget_free, ACaptureRequest, ACaptureRequest_addTarget, ACaptureRequest_free,
    ACaptureRequest_setEntry_i32, ACaptureSessionOutput, ACaptureSessionOutput_create,
    ACaptureSessionOutput_free, ACaptureSessionOutputContainer, ACaptureSessionOutputContainer_add,
    ACaptureSessionOutputContainer_create, ACaptureSessionOutputContainer_free,
    acamera_metadata_tag, camera_status_t,
};

/// What a device tells while it is open, on a thread of the camera service's.
pub(super) struct DeviceEvents {
    /// Android's `onDisconnected`: the camera is no longer this app's, taken by an app of a
    /// higher priority, unplugged, or disabled.
    pub(super) disconnected: Box<dyn Fn() + Send + Sync>,
    /// Android's `onError`, after which the device is of no more use.
    pub(super) failed: Box<dyn Fn(DeviceError) + Send + Sync>,
}

/// Android's `ERROR_CAMERA_*` codes of a device's `onError`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DeviceError {
    /// `ERROR_CAMERA_IN_USE`: another client of a higher priority has the camera.
    CameraInUse,
    /// `ERROR_MAX_CAMERAS_IN_USE`: the system runs as many cameras as it can.
    MaxCamerasInUse,
    /// `ERROR_CAMERA_DISABLED`: the device's policy, or the app's state, keeps the camera from it.
    CameraDisabled,
    /// `ERROR_CAMERA_DEVICE`: the camera failed.
    CameraDevice,
    /// `ERROR_CAMERA_SERVICE`: the camera service failed.
    CameraService,
}

impl DeviceError {
    fn of(code: c_int) -> DeviceError {
        match code {
            1 => DeviceError::CameraInUse,
            2 => DeviceError::MaxCamerasInUse,
            3 => DeviceError::CameraDisabled,
            5 => DeviceError::CameraService,
            _ => DeviceError::CameraDevice,
        }
    }
}

/// A camera opened, closed when dropped.
pub(super) struct CameraDevice {
    raw: NonNull<ACameraDevice>,
    /// The callbacks' context, freed after the device is closed.
    _events: Box<DeviceEvents>,
}

// SAFETY: the NDK's camera device guards its own state, as Java's `CameraDevice` may be called
// from any thread.
unsafe impl Send for CameraDevice {}
unsafe impl Sync for CameraDevice {}

impl CameraDevice {
    /// Opens the camera `id` through `manager`, which tells `events` what befalls it.
    pub(super) fn open(
        manager: NonNull<ACameraManager>,
        id: &str,
        events: DeviceEvents,
    ) -> Result<CameraDevice, camera_status_t> {
        let id = CString::new(id).map_err(|_| camera_status_t::ACAMERA_ERROR_INVALID_PARAMETER)?;
        let events = Box::new(events);
        let mut callbacks = ACameraDevice_StateCallbacks {
            context: ptr::from_ref(&*events).cast_mut().cast(),
            onDisconnected: Some(device_disconnected),
            onError: Some(device_failed),
        };
        let mut raw: *mut ACameraDevice = ptr::null_mut();
        // SAFETY: a live manager, a nul-terminated id, callbacks the device copies, whose context
        // lives until the device is closed, and a place for the device.
        let status = unsafe {
            ACameraManager_openCamera(manager.as_ptr(), id.as_ptr(), &mut callbacks, &mut raw)
        };
        if status != camera_status_t::ACAMERA_OK {
            return Err(status);
        }
        let raw = NonNull::new(raw).ok_or(camera_status_t::ACAMERA_ERROR_UNKNOWN)?;
        Ok(CameraDevice {
            raw,
            _events: events,
        })
    }

    /// A request of `template`'s settings, which the camera fills in for its use.
    pub(super) fn request(
        &self,
        template: ACameraDevice_request_template,
    ) -> Result<CaptureRequest, camera_status_t> {
        let mut raw: *mut ACaptureRequest = ptr::null_mut();
        // SAFETY: a live device and a place for the request, which the request frees.
        let status =
            unsafe { ACameraDevice_createCaptureRequest(self.raw.as_ptr(), template, &mut raw) };
        if status != camera_status_t::ACAMERA_OK {
            return Err(status);
        }
        Ok(CaptureRequest {
            raw: NonNull::new(raw).ok_or(camera_status_t::ACAMERA_ERROR_UNKNOWN)?,
            targets: Vec::new(),
        })
    }

    /// A capture session feeding `windows`, configured together; the device's session before it,
    /// if any, is closed by Android.
    pub(super) fn session(
        &self,
        windows: Vec<NativeWindow>,
    ) -> Result<CaptureSession, camera_status_t> {
        let mut container: *mut ACaptureSessionOutputContainer = ptr::null_mut();
        // SAFETY: a place for the container, freed by the session.
        let status = unsafe { ACaptureSessionOutputContainer_create(&mut container) };
        if status != camera_status_t::ACAMERA_OK {
            return Err(status);
        }
        let mut session = CaptureSession {
            raw: None,
            container: NonNull::new(container).ok_or(camera_status_t::ACAMERA_ERROR_UNKNOWN)?,
            outputs: Vec::new(),
            windows: Vec::new(),
        };
        for window in windows {
            session.add_output(window)?;
        }
        let callbacks = ACameraCaptureSession_stateCallbacks {
            context: ptr::null_mut(),
            onClosed: Some(session_state_changed),
            onReady: Some(session_state_changed),
            onActive: Some(session_state_changed),
        };
        let mut raw: *mut ACameraCaptureSession = ptr::null_mut();
        // SAFETY: a live device and container, callbacks with no context, and a place for the
        // session, which the session closes.
        let status = unsafe {
            ACameraDevice_createCaptureSession(
                self.raw.as_ptr(),
                session.container.as_ptr(),
                &callbacks,
                &mut raw,
            )
        };
        if status != camera_status_t::ACAMERA_OK {
            return Err(status);
        }
        session.raw = NonNull::new(raw);
        Ok(session)
    }
}

impl Drop for CameraDevice {
    fn drop(&mut self) {
        // SAFETY: a device opened here, closed once; its callbacks stop with it, before their
        // context is freed after this.
        unsafe { ACameraDevice_close(self.raw.as_ptr()) };
    }
}

unsafe extern "C" fn device_disconnected(context: *mut c_void, _device: *mut ACameraDevice) {
    super::abort_on_panic(|| {
        // SAFETY: the context is the device's events, which live until it is closed.
        let events = unsafe { &*context.cast::<DeviceEvents>() };
        (events.disconnected)();
    });
}

unsafe extern "C" fn device_failed(
    context: *mut c_void,
    _device: *mut ACameraDevice,
    error: c_int,
) {
    super::abort_on_panic(|| {
        // SAFETY: as for `device_disconnected`.
        let events = unsafe { &*context.cast::<DeviceEvents>() };
        (events.failed)(DeviceError::of(error));
    });
}

/// What the session's state callbacks hear, which nothing here needs: Android asks for them.
unsafe extern "C" fn session_state_changed(
    _context: *mut c_void,
    _session: *mut ACameraCaptureSession,
) {
}

/// A request, freed when dropped, with the targets it feeds.
pub(super) struct CaptureRequest {
    raw: NonNull<ACaptureRequest>,
    targets: Vec<NonNull<ACameraOutputTarget>>,
}

// SAFETY: a request is plain settings until it is captured, which copies it, and is changed by one
// thread at a time, which owning it here makes sure of.
unsafe impl Send for CaptureRequest {}
unsafe impl Sync for CaptureRequest {}

impl CaptureRequest {
    /// Feeds `window`, one of its session's, with each capture of the request.
    pub(super) fn add_target(&mut self, window: &NativeWindow) -> Result<(), camera_status_t> {
        let mut target: *mut ACameraOutputTarget = ptr::null_mut();
        // SAFETY: a live window, which the target holds a reference on, and a place for it.
        let status = unsafe { ACameraOutputTarget_create(window.ptr().as_ptr(), &mut target) };
        if status != camera_status_t::ACAMERA_OK {
            return Err(status);
        }
        let target = NonNull::new(target).ok_or(camera_status_t::ACAMERA_ERROR_UNKNOWN)?;
        self.targets.push(target);
        // SAFETY: a live request and target, which the request keeps until it is freed.
        status_of(unsafe { ACaptureRequest_addTarget(self.raw.as_ptr(), target.as_ptr()) })
    }

    pub(super) fn set_i32s(
        &mut self,
        tag: acamera_metadata_tag,
        values: &[i32],
    ) -> Result<(), camera_status_t> {
        // SAFETY: a live request, and `values` live across the call, which copies them.
        status_of(unsafe {
            ACaptureRequest_setEntry_i32(
                self.raw.as_ptr(),
                tag.0,
                values.len() as u32,
                values.as_ptr(),
            )
        })
    }
}

impl Drop for CaptureRequest {
    fn drop(&mut self) {
        // SAFETY: the request and targets made here, freed once; a capture copied the request.
        unsafe {
            ACaptureRequest_free(self.raw.as_ptr());
            for target in self.targets.drain(..) {
                ACameraOutputTarget_free(target.as_ptr());
            }
        }
    }
}

/// A capture session, closed when dropped, holding the windows it feeds.
pub(super) struct CaptureSession {
    raw: Option<NonNull<ACameraCaptureSession>>,
    container: NonNull<ACaptureSessionOutputContainer>,
    outputs: Vec<NonNull<ACaptureSessionOutput>>,
    windows: Vec<NativeWindow>,
}

// SAFETY: the NDK's capture session guards its own state, as Java's `CameraCaptureSession` may be
// called from any thread.
unsafe impl Send for CaptureSession {}
unsafe impl Sync for CaptureSession {}

impl CaptureSession {
    fn add_output(&mut self, window: NativeWindow) -> Result<(), camera_status_t> {
        let mut output: *mut ACaptureSessionOutput = ptr::null_mut();
        // SAFETY: a live window, kept while the session is, and a place for the output.
        let status = unsafe { ACaptureSessionOutput_create(window.ptr().as_ptr(), &mut output) };
        if status != camera_status_t::ACAMERA_OK {
            return Err(status);
        }
        let output = NonNull::new(output).ok_or(camera_status_t::ACAMERA_ERROR_UNKNOWN)?;
        self.outputs.push(output);
        self.windows.push(window);
        // SAFETY: a live container and output, both freed by the session after it closes.
        status_of(unsafe {
            ACaptureSessionOutputContainer_add(self.container.as_ptr(), output.as_ptr())
        })
    }

    fn raw(&self) -> Result<NonNull<ACameraCaptureSession>, camera_status_t> {
        self.raw
            .ok_or(camera_status_t::ACAMERA_ERROR_SESSION_CLOSED)
    }

    /// Captures `request` again and again, as fast as the camera runs, until the session closes
    /// or another repeating request replaces it.
    pub(super) fn set_repeating(&self, request: &CaptureRequest) -> Result<(), camera_status_t> {
        let mut requests = [request.raw.as_ptr()];
        // SAFETY: a live session and request, which the call copies; no callbacks.
        status_of(unsafe {
            ACameraCaptureSession_setRepeatingRequest(
                self.raw()?.as_ptr(),
                ptr::null_mut(),
                1,
                requests.as_mut_ptr(),
                ptr::null_mut(),
            )
        })
    }

    /// Captures `request` once; `failed` runs, on a thread of the camera service's, where Android
    /// says the capture failed or was aborted, and never where it completed.
    pub(super) fn capture(
        &self,
        request: &CaptureRequest,
        failed: Box<dyn FnOnce() + Send>,
    ) -> Result<(), camera_status_t> {
        let context = Box::into_raw(Box::new(Capture(Mutex::new(Some(failed)))));
        let mut callbacks = ACameraCaptureSession_captureCallbacks {
            context: context.cast(),
            onCaptureStarted: None,
            onCaptureProgressed: None,
            onCaptureCompleted: None,
            onCaptureFailed: Some(capture_failed),
            onCaptureSequenceCompleted: Some(capture_sequence_completed),
            onCaptureSequenceAborted: Some(capture_sequence_aborted),
            onCaptureBufferLost: None,
        };
        let mut requests = [request.raw.as_ptr()];
        // SAFETY: a live session and request, which the call copies with the callbacks, whose
        // context lives until the capture's sequence ends.
        let status = unsafe {
            ACameraCaptureSession_capture(
                self.raw()?.as_ptr(),
                &mut callbacks,
                1,
                requests.as_mut_ptr(),
                ptr::null_mut(),
            )
        };
        if status != camera_status_t::ACAMERA_OK {
            // SAFETY: the context made above, which no callback reaches: the capture was refused.
            drop(unsafe { Box::from_raw(context) });
        }
        status_of(status)
    }
}

impl Drop for CaptureSession {
    fn drop(&mut self) {
        // SAFETY: the session, outputs and container made here, each let go of once, the session
        // first.
        unsafe {
            if let Some(raw) = self.raw.take() {
                ACameraCaptureSession_close(raw.as_ptr());
            }
            for output in self.outputs.drain(..) {
                ACaptureSessionOutput_free(output.as_ptr());
            }
            ACaptureSessionOutputContainer_free(self.container.as_ptr());
        }
    }
}

/// One capture's context: what runs where it fails, taken by the first to reach it.
struct Capture(Mutex<Option<Box<dyn FnOnce() + Send>>>);

impl Capture {
    fn fail(&self) {
        let failed = self.0.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(failed) = failed {
            failed();
        }
    }
}

unsafe extern "C" fn capture_failed(
    context: *mut c_void,
    _session: *mut ACameraCaptureSession,
    _request: *mut ACaptureRequest,
    _failure: *mut ACameraCaptureFailure,
) {
    super::abort_on_panic(|| {
        // SAFETY: the context is the capture's, live until its sequence ends, after this.
        unsafe { &*context.cast::<Capture>() }.fail();
    });
}

unsafe extern "C" fn capture_sequence_completed(
    context: *mut c_void,
    _session: *mut ACameraCaptureSession,
    _sequence_id: c_int,
    _frame_number: i64,
) {
    super::abort_on_panic(|| {
        // SAFETY: the capture's context, which its sequence's end frees; nothing reaches it after.
        drop(unsafe { Box::from_raw(context.cast::<Capture>()) });
    });
}

unsafe extern "C" fn capture_sequence_aborted(
    context: *mut c_void,
    _session: *mut ACameraCaptureSession,
    _sequence_id: c_int,
) {
    super::abort_on_panic(|| {
        // SAFETY: as for `capture_sequence_completed`; an aborted capture gave no image.
        let capture = unsafe { Box::from_raw(context.cast::<Capture>()) };
        capture.fail();
    });
}

fn status_of(status: camera_status_t) -> Result<(), camera_status_t> {
    if status == camera_status_t::ACAMERA_OK {
        Ok(())
    } else {
        Err(status)
    }
}
