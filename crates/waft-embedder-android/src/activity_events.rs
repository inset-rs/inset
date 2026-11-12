//! What Android tells the app of its activities, handed from Android's main thread to the app's.
//!
//! Android calls every activity's callbacks on the process's main thread (`native_activity.rs`);
//! the app runs on a thread of its own (`app_thread.rs`). Each callback hands what it was told to
//! the app's thread as an [`ActivityEvent`] about the view the activity is, set down in one queue
//! for the process, and wakes the app's loop, which takes them in the order they were told
//! ([`take`]).
//!
//! The invariants a later editor will break. The main thread never calls the framework, and it
//! waits for the app only where Android requires the app to be done before the callback returns:
//! when a window or an input queue is taken away, after which Android destroys it. Such an event
//! carries a [`Done`], which the loop gives once it has let go ([`tell_and_wait`]); Flutter's
//! embedding waits at the same point, for its raster thread to let go of the surface
//! (`FlutterSurfaceView.surfaceDestroyed`, `Shell::OnPlatformViewDestroyed`). And once the app's
//! entry has returned nothing is set down ([`close`]), so the main thread never waits for an app
//! that will not answer.

use std::sync::mpsc;
use std::sync::{Mutex, MutexGuard, PoisonError};

use ndk::configuration::Configuration;
use ndk::input_queue::InputQueue;
use ndk::native_window::NativeWindow;
use waft_embedder::ViewId;

use crate::activity::Activity;
use crate::loop_waker::wake_app_loop;

/// What was told since the loop last took it; `None` once the app's entry has returned.
static TOLD: Mutex<Option<Vec<Told>>> = Mutex::new(Some(Vec::new()));

/// One thing Android told an activity, about the view the activity is.
pub(crate) struct Told {
    pub(crate) view: ViewId,
    pub(crate) event: ActivityEvent,
}

/// What Android tells an activity, as the app's thread takes it.
pub(crate) enum ActivityEvent {
    /// Android made the activity, which is the view from now on, with `configuration` its
    /// resources'. `restored`: it was made from the state an activity before it saved, so the
    /// intent it carries is one that activity started with.
    Created {
        activity: Activity,
        configuration: Configuration,
        restored: bool,
    },
    /// The activity's window: `NativeActivity`'s `surfaceCreated`.
    WindowCreated(NativeWindow),
    /// The window's size changed: `surfaceChanged`.
    WindowResized,
    /// The window asks to be drawn again: `surfaceRedrawNeeded`. Android does not wait for it.
    WindowRedrawNeeded,
    /// The window is taken away, and is destroyed once the app has let go of it and given `Done`.
    WindowDestroyed(Done),
    /// The queue the activity's input arrives in.
    InputQueueCreated(InputQueue),
    /// The input queue is taken away, and is destroyed once the app has detached it from its
    /// looper and given `Done`.
    InputQueueDestroyed(Done),
    /// The rectangle the activity laid its content out in: left, top, right and bottom.
    ContentRectChanged([i32; 4]),
    /// The activity's configuration changed, to this, where the manifest says it absorbs the
    /// change.
    ConfigurationChanged(Configuration),
    /// The activity's window gained or lost the focus.
    FocusChanged(bool),
    /// Android destroyed the activity: `finishing` where it is gone for good, as `isFinishing`
    /// says, and otherwise to make it again, as for a change the manifest does not absorb.
    Destroyed { finishing: bool },
}

/// The app's word that it let go of what an event took away, which Android's main thread waits
/// for. Dropping it without giving it lets the main thread go on all the same.
pub(crate) struct Done(mpsc::Sender<()>);

impl Done {
    /// Lets Android's main thread go on.
    pub(crate) fn give(self) {
        let _ = self.0.send(());
    }
}

/// Sets `event` down for the app's loop as told about `view`, and wakes the loop. Android's main
/// thread calls it.
pub(crate) fn tell(view: ViewId, event: ActivityEvent) {
    let mut told = locked();
    let Some(told) = told.as_mut() else {
        return;
    };
    told.push(Told { view, event });
    wake_app_loop();
}

/// Sets down the event `event` makes of a [`Done`], and waits until the app's loop gives it.
/// Android's main thread calls it where Android destroys what the event takes away once the
/// callback returns.
pub(crate) fn tell_and_wait(view: ViewId, event: impl FnOnce(Done) -> ActivityEvent) {
    let (done, given) = mpsc::channel();
    tell(view, event(Done(done)));
    // An error is a `Done` dropped ungiven: the app's entry returned, or the event was about a
    // view the app has no more.
    let _ = given.recv();
}

/// What was told since the last call, in the order it was told.
pub(crate) fn take() -> Vec<Told> {
    locked().as_mut().map(std::mem::take).unwrap_or_default()
}

/// The app's entry returned: what was told and not taken is dropped, its `Done`s with it, and
/// nothing is set down from now on.
pub(crate) fn close() {
    locked().take();
}

fn locked() -> MutexGuard<'static, Option<Vec<Told>>> {
    TOLD.lock().unwrap_or_else(PoisonError::into_inner)
}
