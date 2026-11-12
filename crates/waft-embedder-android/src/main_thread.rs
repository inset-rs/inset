//! Work for Android's main thread, asked from any other: a window and its views are touched there
//! alone, and an activity's own Java code runs there.
//!
//! The work is set down in one queue for the process and an `eventfd` the main thread's looper
//! polls is signalled; the looper runs the work between Android's own messages. Ported from
//! android-activity's `MainCallbacks`, which does the same for its activities.
//!
//! The invariant a later editor will break: the work only goes to the main thread, and nothing
//! waits for it there. The main thread waits for the app's thread where Android requires it
//! (`activity_events.rs`), so the app's thread waiting for the main thread would be a deadlock.

// An `eventfd` is a C call with no binding in the standard library; that is the `unsafe` here.
#![allow(unsafe_code)]

use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsFd, FromRawFd, OwnedFd};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use ndk::looper::{FdEvent, ThreadLooper};

use crate::log::warn_to_log;

type Work = Box<dyn FnOnce() + Send>;

/// The work asked for and not yet run.
static WORK: Mutex<Vec<Work>> = Mutex::new(Vec::new());
/// The signal the main thread's looper polls, once it serves the work.
static SIGNAL: OnceLock<File> = OnceLock::new();

/// Has the calling thread, Android's main thread, run the work asked for from now on, from its
/// looper. The first call does it; later ones do nothing.
pub(crate) fn serve_here() {
    SIGNAL.get_or_init(|| {
        let signal = signal();
        let looper = ThreadLooper::for_thread().expect("Android's main thread has a looper");
        let added =
            looper.add_fd_with_callback(signal.as_fd(), FdEvent::INPUT, |_signal, _events| {
                run_work();
                true
            });
        if added.is_err() {
            warn_to_log!("waft: Android's main thread will run no work of the app's");
        }
        signal
    });
}

/// Has Android's main thread run `work`, and returns at once.
pub(crate) fn run_on_main_thread(work: impl FnOnce() + Send + 'static) {
    locked().push(Box::new(work));
    if let Some(mut signal) = SIGNAL.get() {
        // The counter adds up; one read clears it, however many works were asked for.
        let _ = signal.write_all(&1u64.to_ne_bytes());
    }
}

/// Runs the work asked for so far, on the main thread: a panic in one is said in the log, and the
/// rest still run.
fn run_work() {
    if let Some(mut signal) = SIGNAL.get() {
        let _ = signal.read(&mut [0; 8]);
    }
    let work = std::mem::take(&mut *locked());
    for work in work {
        if catch_unwind(AssertUnwindSafe(work)).is_err() {
            warn_to_log!("waft: a work on Android's main thread panicked");
        }
    }
}

/// A counter the looper sees readable while it is above zero, which is not read past zero.
fn signal() -> File {
    // SAFETY: a plain system call, which answers a new descriptor or -1.
    let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
    assert!(fd >= 0, "an eventfd: {}", std::io::Error::last_os_error());
    // SAFETY: the descriptor was just made, and nothing else owns it.
    File::from(unsafe { OwnedFd::from_raw_fd(fd) })
}

fn locked() -> MutexGuard<'static, Vec<Work>> {
    WORK.lock().unwrap_or_else(PoisonError::into_inner)
}
