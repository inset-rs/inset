//! The one [`EmbedderClient`] the framework provides: owns [`App`] and
//! translates host pushes into binding methods.
//!
//! Dart has no type for this — the engine owns the isolate. [`App`] stays in
//! foundation. This crate sits above scheduler and gestures so the isolate
//! can name both.
#![feature(arbitrary_self_types)]

use reveal_embedder::{EmbedderClient, Frame, PlatformRef, PointerDataPacket, ViewId};
use reveal_foundation::App;
use reveal_gestures::GestureBinding;
use reveal_rendering::RendererBinding;
use reveal_scheduler::SchedulerBinding;

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

    fn view_metrics_changed(&mut self, _id: ViewId) {
        RendererBinding::instance(&mut self.app).handle_metrics_changed(&mut self.app);
    }

    fn view_removed(&mut self, _id: ViewId) {}

    fn pointer_data_packet(&mut self, packet: PointerDataPacket) {
        GestureBinding::instance(&mut self.app).handle_pointer_data_packet(&mut self.app, packet);
        self.app.drain_microtasks();
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use reveal_embedder::{
        EmbedderClient, Frame, Picture, Platform, PointerChange, PointerData, PointerDataPacket,
        PointerDeviceKind, View, ViewId, ViewMetrics, ViewRef,
    };
    use reveal_foundation::App;
    use reveal_gestures::{GestureBinding, PointerRoute};
    use reveal_scheduler::SchedulerBinding;

    use super::Shell;

    struct RecordingPlatform {
        frames: Arc<AtomicUsize>,
        view: Option<ViewRef>,
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
            self.view.iter().cloned().collect()
        }

        fn view(&self, id: ViewId) -> Option<ViewRef> {
            self.view.as_ref().filter(|view| view.id() == id).cloned()
        }

        fn implicit_view(&self) -> Option<ViewRef> {
            self.view.clone()
        }
    }

    struct TestView;

    impl View for TestView {
        fn id(&self) -> ViewId {
            ViewId(0)
        }

        fn metrics(&self) -> ViewMetrics {
            ViewMetrics::default()
        }

        fn present(&self, _picture: &Picture) {}
    }

    #[test]
    fn new_runs_setup_and_frame_runs_both_scheduler_phases() {
        let frames = Arc::new(AtomicUsize::new(0));
        let platform = std::rc::Rc::new(RecordingPlatform {
            frames: Arc::clone(&frames),
            view: None,
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

    #[test]
    fn pointer_data_packet_reaches_gesture_binding() {
        let ran = Rc::new(Cell::new(false));
        let ran_flag = Rc::clone(&ran);
        let platform = std::rc::Rc::new(RecordingPlatform {
            frames: Arc::new(AtomicUsize::new(0)),
            view: Some(Rc::new(TestView)),
        });
        let mut shell = Shell::new(platform, |app| {
            let binding = GestureBinding::instance(app);
            binding.pointer_router(app).add_route(
                app,
                1,
                PointerRoute::new(move |_app, _event| {
                    ran_flag.set(true);
                }),
                None,
            );
        });

        shell.pointer_data_packet(PointerDataPacket::new(vec![PointerData {
            change: PointerChange::Down,
            kind: PointerDeviceKind::Mouse,
            buttons: 1,
            pointer_identifier: 1,
            ..PointerData::default()
        }]));
        assert!(ran.get());
    }
}
