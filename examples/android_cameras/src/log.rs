//! Where the example reports: the system's log, which `adb logcat` reads. An Android app has no
//! stderr.

// Liblog is a C API; this module writes the `unsafe` for it.
#![allow(unsafe_code)]

use std::ffi::CString;

/// The tag every line carries.
const TAG: &std::ffi::CStr = c"android_cameras";

/// `ANDROID_LOG_INFO`.
const INFO: i32 = 4;

/// Writes one line to the system's log.
pub(crate) fn info(message: &str) {
    let Ok(text) = CString::new(message) else {
        return;
    };
    // SAFETY: both pointers are to nul-terminated bytes that outlive the call.
    unsafe {
        ndk_sys::__android_log_write(INFO, TAG.as_ptr(), text.as_ptr());
    }
}
