//! The one [`EmbedderClient`] the framework provides: owns [`App`] and
//! translates host pushes into scheduler phases and retained views.
//!
//! Dart has no type for this — the engine owns the isolate. It lives here
//! because it translates a host frame into scheduler phases; foundation
//! cannot name this crate.

use reveal_embedder::{EmbedderClient, Frame, PlatformRef, PointerDataPacket, ViewId};
use reveal_foundation::App;

use crate::SchedulerBinding;

/// Host-facing isolate: [`App`] plus the methods the embedder pushes.
pub struct Shell {
    app: App,
}

impl Shell {
    /// Builds the heap from `platform`, then runs application setup.
    ///
    /// Setup (later widgets `run_app`) requests the first frame when it
    /// installs work that needs one.
    pub fn new(platform: PlatformRef, setup: impl FnOnce(&mut App)) -> Shell {
        let mut app = App::with_platform(platform);
        setup(&mut app);
        Shell { app }
    }

    pub fn app(&mut self) -> &mut App {
        &mut self.app
    }
}

impl EmbedderClient for Shell {
    fn frame(&mut self, frame: Frame) {
        SchedulerBinding::handle_begin_frame(&mut self.app, Some(frame.elapsed));
        self.app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(&mut self.app);
        self.app.drain_microtasks();
    }

    fn view_added(&mut self, _id: ViewId) {}

    fn view_metrics_changed(&mut self, _id: ViewId) {}

    fn view_removed(&mut self, _id: ViewId) {}

    fn pointer_data_packet(&mut self, _packet: PointerDataPacket) {}
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use reveal_embedder::{EmbedderClient, Frame, Platform, ViewId, ViewRef};
    use reveal_foundation::App;

    use super::Shell;
    use crate::SchedulerBinding;

    struct RecordingPlatform {
        frames: Arc<AtomicUsize>,
    }

    impl Platform for RecordingPlatform {
        fn target_platform(&self) -> reveal_embedder::TargetPlatform {
            reveal_embedder::TargetPlatform::Android
        }

        fn request_frame(&self) {
            self.frames.fetch_add(1, Ordering::SeqCst);
        }

        fn now(&self) -> std::time::Instant {
            std::time::Instant::now()
        }

        fn wake_at(&self, _deadline: std::time::Instant) {}

        fn views(&self) -> Vec<ViewRef> {
            Vec::new()
        }

        fn view(&self, _id: ViewId) -> Option<ViewRef> {
            None
        }

        fn implicit_view(&self) -> Option<ViewRef> {
            None
        }
    }

    #[test]
    fn new_runs_setup_and_frame_runs_both_scheduler_phases() {
        let frames = Arc::new(AtomicUsize::new(0));
        let platform = std::rc::Rc::new(RecordingPlatform {
            frames: Arc::clone(&frames),
        });
        let mut setup_ran = false;
        let mut shell = Shell::new(platform, |_app: &mut App| {
            setup_ran = true;
        });
        assert!(setup_ran);
        assert_eq!(frames.load(Ordering::SeqCst), 0);

        SchedulerBinding::schedule_frame(shell.app());
        assert_eq!(frames.load(Ordering::SeqCst), 1);
        shell.frame(Frame {
            elapsed: Duration::from_millis(16),
        });
    }
}
