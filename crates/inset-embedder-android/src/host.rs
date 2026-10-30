//! The entry point: each activity the system creates attaches to the app, which the first one
//! starts.
//!
//! Android's process outlives its activities, and so does the app: one loop for the process, on a
//! thread of its own (`app_thread`), as Flutter's engine outlives a `FlutterActivity`. Each activity
//! is a view of that app from its creation to its destruction, and its own thread hands on what the
//! system tells it (`activity_thread`).

use android_activity::AndroidApp;
use inset_embedder::{EmbedderClient, PlatformRef};

use crate::activity_thread::ActivityLoop;
use crate::app_thread::AppThread;

/// Runs an Inset application on the activities the system creates.
pub struct AndroidEmbedder {
    app: AndroidApp,
}

impl AndroidEmbedder {
    /// Takes the [`AndroidApp`] the glue handed `android_main`, which is one activity's.
    pub fn new(app: AndroidApp) -> AndroidEmbedder {
        AndroidEmbedder { app }
    }

    /// Attaches the activity to the app as a view, and runs the activity's thread until the
    /// activity is destroyed. The first activity of the process starts the app: `start` runs on
    /// the app's thread once a view has a window, and returns the client the host drives; it is
    /// sent there, and goes unused for every later activity.
    pub fn run<C: EmbedderClient + 'static>(
        self,
        start: impl FnOnce(PlatformRef) -> C + Send + 'static,
    ) {
        let (app_thread, view) = AppThread::attach(self.app.clone(), start);
        ActivityLoop::new(self.app, view, app_thread).run();
    }
}
