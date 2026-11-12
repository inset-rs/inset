//! Where the host's own messages go, and the process's standard output and error.
//!
//! An Android app has no stderr: the system gives its processes none, so anything written
//! there is dropped and a message about a window the host could not use would be lost. The
//! platform's log takes them instead, under one tag, which is where `adb logcat` reads. So does
//! what the process writes to its standard output and error, a panic's message among them, which
//! the host hands on line by line ([`forward_standard_streams`]), as android-activity did.

// Liblog is a C API, and the `ndk` crate wraps none of it; handing the standard streams on takes a
// pipe and `dup2`, which the standard library does not make. That is the `unsafe` here.
#![allow(unsafe_code)]

use std::ffi::CString;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::fd::FromRawFd;
use std::sync::Once;

/// The tag every message the host writes carries.
const TAG: &[u8] = b"waft\0";

/// Android's `ANDROID_LOG_INFO`, the priority of what the process wrote to its standard streams.
const INFO: i32 = 4;
/// Android's `ANDROID_LOG_WARN`, the priority the host's messages carry: each one is
/// something the app's author wants to know about and none is fatal.
const WARN: i32 = 5;

/// Writes one line to the platform's log.
pub(crate) fn warn(message: &str) {
    write(WARN, message);
}

fn write(priority: i32, message: &str) {
    let Ok(text) = CString::new(message) else {
        return;
    };
    // SAFETY: both pointers are to nul-terminated bytes that outlive the call, and liblog
    // is part of the platform every Android process links.
    unsafe {
        ndk_sys::__android_log_write(priority, TAG.as_ptr().cast(), text.as_ptr());
    }
}

/// Has every line the process writes to its standard output and error from now on go to the
/// platform's log, read by a thread of its own; once for the process.
pub(crate) fn forward_standard_streams() {
    static FORWARDED: Once = Once::new();
    FORWARDED.call_once(|| {
        let Some(written) = standard_streams_into_a_pipe() else {
            warn("waft: the standard output and error are not in the log");
            return;
        };
        let forwarded = std::thread::Builder::new()
            .name("waft-stdio".into())
            .spawn(move || {
                for line in BufReader::new(written).lines() {
                    match line {
                        Ok(line) => write(INFO, &line),
                        Err(_) => break,
                    }
                }
            });
        if forwarded.is_err() {
            warn("waft: the standard output and error are not in the log");
        }
    });
}

/// Points the standard output and error at a new pipe, and answers its reading end.
fn standard_streams_into_a_pipe() -> Option<File> {
    let mut pipe = [-1; 2];
    // SAFETY: plain system calls on descriptors made here, and the process's own two streams.
    unsafe {
        if libc::pipe2(pipe.as_mut_ptr(), libc::O_CLOEXEC) != 0 {
            return None;
        }
        libc::dup2(pipe[1], libc::STDOUT_FILENO);
        libc::dup2(pipe[1], libc::STDERR_FILENO);
        libc::close(pipe[1]);
    }
    // SAFETY: the reading end was just made, and nothing else owns it.
    Some(unsafe { File::from_raw_fd(pipe[0]) })
}

/// Writes one line to the platform's log, in the shape [`format!`] takes.
macro_rules! warn_to_log {
    ($($argument:tt)*) => { $crate::log::warn(&format!($($argument)*)) };
}

pub(crate) use warn_to_log;
