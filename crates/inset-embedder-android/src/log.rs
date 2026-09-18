//! Where the host's own messages go.
//!
//! An Android app has no stderr: the system gives its processes none, so anything written
//! there is dropped and a message about a window the host could not use would be lost. The
//! platform's log takes them instead, under one tag, which is where `adb logcat` reads.

// Liblog is a C API, and the `ndk` crate wraps none of it.
#![allow(unsafe_code)]

use std::ffi::CString;

/// The tag every message the host writes carries.
const TAG: &[u8] = b"inset\0";

/// Android's `ANDROID_LOG_WARN`, the priority the host's messages carry: each one is
/// something the app's author wants to know about and none is fatal.
const WARN: i32 = 5;

/// Writes one line to the platform's log.
pub(crate) fn warn(message: &str) {
    let Ok(text) = CString::new(message) else {
        return;
    };
    // SAFETY: both pointers are to nul-terminated bytes that outlive the call, and liblog
    // is part of the platform every Android process links.
    unsafe {
        ndk_sys::__android_log_write(WARN, TAG.as_ptr().cast(), text.as_ptr());
    }
}

/// Writes one line to the platform's log, in the shape [`format!`] takes.
macro_rules! warn_to_log {
    ($($argument:tt)*) => { $crate::log::warn(&format!($($argument)*)) };
}

pub(crate) use warn_to_log;
