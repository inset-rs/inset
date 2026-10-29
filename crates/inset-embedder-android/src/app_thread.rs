//! The app's thread: the one the process has for the app, which outlives every activity, as
//! Flutter's engine outlives a `FlutterActivity`.
//!
//! android-activity runs `android_main` on a thread of its own for each activity the system creates,
//! and only that thread may read the activity's events (`activity_thread`). Each such thread hands
//! what its activity hears to this one, where the app lives: the client, the platform and every
//! view (`app_loop`). The first activity starts the thread; it runs for the rest of the process, and
//! an activity created after every other one was destroyed attaches to the same app.
//!
//! What Android waits for before it goes on, a window taken away or an activity destroyed, the
//! activity's thread waits for in turn ([`AppThread::send_and_wait`]): the app lets go of the
//! window or the view and answers, so nothing draws into a window Android has taken back.

use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};

use android_activity::AndroidApp;
use inset_embedder::{EmbedderClient, PlatformRef, PointerDataPacket, ViewId};
use ndk::looper::{ForeignLooper, ThreadLooper};

use crate::app_loop::AppLoop;

/// The app's thread, once the process's first activity has started it.
static APP_THREAD: Mutex<Option<AppThread>> = Mutex::new(None);

/// What an activity's thread hands the app's thread, for the view the activity is.
pub(crate) enum ActivityEvent {
    /// An activity created after the app started, which attaches to it as a view.
    Created(AndroidApp),
    WindowArrived,
    /// Answered once the view no longer draws into the window.
    WindowGone(Done),
    GeometryChanged,
    /// The configuration, which carries the appearance, the languages and the screen.
    ConfigChanged,
    RedrawNeeded,
    FocusChanged(bool),
    Pointer(PointerDataPacket),
    /// The back button or the back gesture, released.
    Back,
    /// Answered once the view is gone from the app.
    Destroyed(Done),
}

/// The app's answer that it did what an event asked, which the activity's thread waits for. Dropped
/// unanswered, it ends the wait too, so an activity never waits for ever.
pub(crate) struct Done(mpsc::Sender<()>);

impl Done {
    pub(crate) fn answer(self) {
        let _ = self.0.send(());
    }
}

/// The events activities' threads handed on, not yet taken.
pub(crate) type Inbox = Arc<Mutex<Vec<(ViewId, ActivityEvent)>>>;

/// The app's thread, as an activity's thread reaches it.
#[derive(Clone)]
pub(crate) struct AppThread {
    inbox: Inbox,
    looper: ForeignLooper,
    /// The number the next view is given.
    next_view: Arc<AtomicU64>,
}

impl AppThread {
    /// Attaches `activity` to the app as a view of its own, starting the app's thread with `start`
    /// when the process has none yet, `start` going unused otherwise. Answers the app's thread and
    /// the view the activity is.
    pub(crate) fn attach<C: EmbedderClient + 'static>(
        activity: AndroidApp,
        start: impl FnOnce(PlatformRef) -> C + Send + 'static,
    ) -> (AppThread, ViewId) {
        let mut running = locked(&APP_THREAD);
        if let Some(app_thread) = running.as_ref() {
            let view = app_thread.new_view();
            app_thread.send(view, ActivityEvent::Created(activity));
            return (app_thread.clone(), view);
        }
        let next_view = Arc::new(AtomicU64::new(0));
        let view = ViewId(next_view.fetch_add(1, Ordering::Relaxed));
        let app_thread = AppThread::start(activity, view, next_view, start);
        *running = Some(app_thread.clone());
        (app_thread, view)
    }

    /// Spawns the thread with the app on it, the activity that started it its first view, and
    /// waits for its looper.
    fn start<C: EmbedderClient + 'static>(
        first: AndroidApp,
        view: ViewId,
        next_view: Arc<AtomicU64>,
        start: impl FnOnce(PlatformRef) -> C + Send + 'static,
    ) -> AppThread {
        let inbox = Inbox::default();
        let (looper_sent, looper) = mpsc::channel();
        let app_inbox = Arc::clone(&inbox);
        std::thread::Builder::new()
            .name("inset-app".into())
            .spawn(move || {
                let looper = ThreadLooper::prepare();
                let _ = looper_sent.send(looper.as_foreign().clone());
                // A panic leaves no app for any activity to attach to.
                let ran = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    AppLoop::new(looper, app_inbox, first, view, start).run()
                }));
                if ran.is_err() {
                    std::process::abort();
                }
            })
            .expect("a thread for the app");
        AppThread {
            inbox,
            looper: looper.recv().expect("the app's thread prepares its looper"),
            next_view,
        }
    }

    fn new_view(&self) -> ViewId {
        ViewId(self.next_view.fetch_add(1, Ordering::Relaxed))
    }

    /// Hands `event` to the app, for `view`, and wakes the app's thread.
    pub(crate) fn send(&self, view: ViewId, event: ActivityEvent) {
        locked(&self.inbox).push((view, event));
        self.looper.wake();
    }

    /// Hands the event `event` makes to the app, for `view`, and waits until the app has done what
    /// it asks.
    pub(crate) fn send_and_wait(&self, view: ViewId, event: impl FnOnce(Done) -> ActivityEvent) {
        let (done, answered) = mpsc::channel();
        self.send(view, event(Done(done)));
        let _ = answered.recv();
    }
}

/// Takes the events handed on since the last call, in the order they came.
pub(crate) fn take(inbox: &Inbox) -> Vec<(ViewId, ActivityEvent)> {
    std::mem::take(&mut *locked(inbox))
}

fn locked<T>(shared: &Mutex<T>) -> MutexGuard<'_, T> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}
