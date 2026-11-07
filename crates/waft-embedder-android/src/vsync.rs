//! The display's refresh, from the system's own signal.
//!
//! `AChoreographer` is the platform's vsync for native code, on the thread that asks for
//! it, which here is the loop's. Flutter's engine takes the same signal for its Android
//! `VsyncWaiter`. A callback fires once, so each one asks for the next.
//!
//! The rate the panel runs at is the system's own choice. `ANativeWindow_setFrameRate`
//! would ask for one, but it is an API 30 symbol and this library loads on API 24, where
//! the loader cannot find it and the whole library fails to open; asking would mean
//! resolving it at run time. Flutter's engine does not ask either — it reads the rate the
//! display reports and paces to it — so neither does this.

// The choreographer is a C API with no binding in the `ndk` crate.
#![allow(unsafe_code)]

use std::cell::Cell;
use std::ffi::c_void;
use std::rc::Rc;

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
    on_tick: Box<dyn Fn()>,
    /// Whether ticks are wanted; a paused signal asks for nothing.
    paused: Cell<bool>,
    /// Whether a refresh is already asked for, so unpausing does not ask twice.
    posted: Cell<bool>,
}

impl Vsync {
    /// Starts the signal for the display `window` is on. `None` where the thread has no
    /// choreographer, which leaves the caller to pace frames itself.
    pub(crate) fn start(_window: &NativeWindow, on_tick: Box<dyn Fn()>) -> Option<Vsync> {
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
        ndk_sys::AChoreographer_postFrameCallback(shared.choreographer, Some(on_refresh), data);
    }
}

/// One refresh of the display: the tick, and the ask for the next.
unsafe extern "C" fn on_refresh(_frame_time_nanos: std::os::raw::c_long, data: *mut c_void) {
    // SAFETY: the reference `post` handed over, taken back once, on the thread that made it.
    let shared = unsafe { Rc::from_raw(data.cast_const().cast::<Ticks>()) };
    shared.posted.set(false);
    if shared.paused.get() {
        return;
    }
    (shared.on_tick)();
    post(&shared);
}
