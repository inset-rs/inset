//! The display's refresh, from the system's own signal.
//!
//! `AChoreographer` is the platform's vsync for native code, on the thread that asks for
//! it, which here is the loop's. Flutter's engine takes the same signal for its Android
//! `VsyncWaiter`. A callback fires once, so each one asks for the next.
//!
//! Each refresh carries the time the choreographer gives it, `CLOCK_MONOTONIC`'s, which is the
//! clock Rust's `Instant` reads on Android; it is handed on as an `Instant` (`instant_of`), whose
//! error is the time between reading the two clocks once, well under a microsecond.
//!
//! The rate the panel runs at is the system's own choice. `ANativeWindow_setFrameRate`
//! would ask for one; Flutter's engine does not ask — it reads the rate the display reports
//! and paces to it — so neither does this.

// The choreographer is a C API with no binding in the `ndk` crate, and its clock is read by
// `clock_gettime`.
#![allow(unsafe_code)]

use std::cell::Cell;
use std::ffi::c_void;
use std::rc::Rc;
use std::time::{Duration, Instant};

use ndk::native_window::NativeWindow;

/// The signal for one window's display, stopped when dropped.
///
/// The state a posted callback reads is shared with it by a strong reference the callback
/// takes back, so it outlives this handle while one is still in flight; dropping the
/// handle pauses it, and the callback that arrives then lets the state go rather than
/// asking for another refresh.
pub(crate) struct Vsync {
    shared: Rc<Ticks>,
}

struct Ticks {
    /// The choreographer for the loop's thread, which owns it for that thread's life.
    choreographer: *mut ndk_sys::AChoreographer,
    /// Called at each refresh with the moment its vsync occurred.
    on_tick: Box<dyn Fn(Instant)>,
    /// Whether ticks are wanted; a paused signal asks for nothing.
    paused: Cell<bool>,
    /// Whether a refresh is already asked for, so unpausing does not ask twice.
    posted: Cell<bool>,
}

impl Vsync {
    /// Starts the signal for the display `window` is on. `None` where the thread has no
    /// choreographer, which leaves the caller to pace frames itself.
    pub(crate) fn start(_window: &NativeWindow, on_tick: Box<dyn Fn(Instant)>) -> Option<Vsync> {
        // SAFETY: the instance for the calling thread, which is the loop's own and has the
        // looper a choreographer needs. Null where it has none.
        let choreographer = unsafe { ndk_sys::AChoreographer_getInstance() };
        if choreographer.is_null() {
            return None;
        }
        let shared = Rc::new(Ticks {
            choreographer,
            on_tick,
            paused: Cell::new(false),
            posted: Cell::new(false),
        });
        post(&shared);
        Some(Vsync { shared })
    }

    /// Stops the ticks, or starts them again.
    pub(crate) fn set_paused(&self, paused: bool) {
        self.shared.paused.set(paused);
        if !paused {
            post(&self.shared);
        }
    }
}

impl Drop for Vsync {
    fn drop(&mut self) {
        // A refresh may already be asked for; pausing is what tells its callback to let go
        // rather than ask for another.
        self.shared.paused.set(true);
    }
}

/// Asks for the next refresh, unless one is already asked for. The callback is handed a
/// strong reference, which it takes back when it runs.
fn post(shared: &Rc<Ticks>) {
    if shared.posted.replace(true) {
        return;
    }
    let data = Rc::into_raw(Rc::clone(shared)).cast::<c_void>().cast_mut();
    // SAFETY: the choreographer belongs to this thread, `on_refresh` has the signature the
    // API calls it with, and `data` is a reference the callback takes back exactly once.
    unsafe {
        ndk_sys::AChoreographer_postFrameCallback64(shared.choreographer, Some(on_refresh), data);
    }
}

/// One refresh of the display, whose vsync occurred at `frame_time_nanos`: the tick, and the ask
/// for the next.
unsafe extern "C" fn on_refresh(frame_time_nanos: i64, data: *mut c_void) {
    // SAFETY: the reference `post` handed over, taken back once, on the thread that made it.
    let shared = unsafe { Rc::from_raw(data.cast_const().cast::<Ticks>()) };
    shared.posted.set(false);
    if shared.paused.get() {
        return;
    }
    (shared.on_tick)(instant_of(frame_time_nanos));
    post(&shared);
}

/// The moment `nanos` on `CLOCK_MONOTONIC` is, as an `Instant`: as far before now as the clock
/// reads past it, and now for a moment the clock has not reached, as Flutter's Android waiter
/// clamps a frame time ahead of now (`VsyncWaiterAndroid::OnVsyncFromNDK`).
fn instant_of(nanos: i64) -> Instant {
    let now = Instant::now();
    let mut clock = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: a plain read of a clock every Android has, into a `timespec` of this frame.
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut clock) };
    let clock_nanos = i128::from(clock.tv_sec) * 1_000_000_000 + i128::from(clock.tv_nsec);
    let ago = u64::try_from(clock_nanos - i128::from(nanos)).unwrap_or(0);
    now.checked_sub(Duration::from_nanos(ago)).unwrap_or(now)
}
