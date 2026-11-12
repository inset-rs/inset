//! What wakes the app's loop from any thread: the framework asking for a frame or a wake, a wait
//! on the display's refresh, and Android's main thread setting down what an activity was told, what
//! a keyboard typed, the insets it applied to a window or an intent that arrived.
//!
//! The loop is the app's thread's (`app_thread.rs`), one for the process, so there is one waker,
//! its looper's, however many activities set things down for it: a later editor must not give an
//! activity a waker of its own. A native method Android's main thread calls arrives with no state
//! of the host's, so the waker is kept for the process ([`wake_app_loop`]).

use std::sync::OnceLock;

use ndk::looper::{ForeignLooper, ThreadLooper};

/// The app's loop's waker, once its thread has a looper.
static APP_LOOP: OnceLock<LoopWaker> = OnceLock::new();

/// Wakes the app's loop.
#[derive(Clone)]
pub(crate) struct LoopWaker {
    looper: ForeignLooper,
}

impl LoopWaker {
    /// Has the loop make a pass, if it is waiting.
    pub(crate) fn wake(&self) {
        self.looper.wake();
    }
}

/// `looper`, the app's thread's, is the one the app's loop waits on, for the rest of the process.
pub(crate) fn serve(looper: &ThreadLooper) {
    let waker = LoopWaker {
        looper: looper.as_foreign().clone(),
    };
    assert!(
        APP_LOOP.set(waker).is_ok(),
        "the app's loop is served once a process"
    );
}

/// The app's loop's waker.
///
/// # Panics
///
/// Before the app's thread has a looper, which it has before the app's entry runs.
pub(crate) fn app_loop() -> LoopWaker {
    APP_LOOP
        .get()
        .cloned()
        .expect("the app's thread has its looper")
}

/// Wakes the app's loop, for what was set down for it from another thread; nothing before its
/// thread has a looper, whose loop takes what was set down before its first wait.
pub(crate) fn wake_app_loop() {
    if let Some(waker) = APP_LOOP.get() {
        waker.wake();
    }
}
