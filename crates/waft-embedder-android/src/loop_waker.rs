//! What wakes the app's loop from any thread: the framework asking for a frame or a wake, a wait
//! on the display's refresh, and Android's main thread setting down what a keyboard typed, the
//! insets it applied to a window or an intent that arrived.
//!
//! There is one loop for the app, so one waker, however many activities set things down for it: a
//! later editor must not give an activity a waker of its own. Today the loop is the one
//! android-activity's glue runs for the activity, and the waker is the glue's.
//!
//! A native method Android's main thread calls arrives with no state of the host's, so the waker
//! it wakes is kept for the process ([`LoopWaker::serve_native_methods`], [`wake_app_loop`]).

use std::sync::{Mutex, PoisonError};

use android_activity::AndroidAppWaker;

/// The waker the native methods wake: the loop's that started last, since the process starts
/// another loop once an activity is finished and Android starts the next.
static APP_LOOP: Mutex<Option<LoopWaker>> = Mutex::new(None);

/// Wakes the app's loop.
#[derive(Clone)]
pub(crate) struct LoopWaker {
    glue: AndroidAppWaker,
}

impl LoopWaker {
    /// The waker of the loop the glue runs, from the waker the glue makes for it.
    pub(crate) fn of(glue: AndroidAppWaker) -> LoopWaker {
        LoopWaker { glue }
    }

    /// Has the loop make a pass, if it is waiting.
    pub(crate) fn wake(&self) {
        self.glue.wake();
    }

    /// The loop this wakes is the one [`wake_app_loop`] wakes from now on.
    pub(crate) fn serve_native_methods(&self) {
        *APP_LOOP.lock().unwrap_or_else(PoisonError::into_inner) = Some(self.clone());
    }
}

/// Wakes the app's loop, for a native method that set something down for it; nothing before a
/// loop started.
pub(crate) fn wake_app_loop() {
    let app_loop = APP_LOOP.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(waker) = app_loop.as_ref() {
        waker.wake();
    }
}
