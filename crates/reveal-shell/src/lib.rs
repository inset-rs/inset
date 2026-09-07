//! The one [`EmbedderClient`] the framework provides: owns the [`AppCell`] and
//! translates host pushes into binding methods.
//!
//! Dart has no type for this — the engine owns the isolate. [`App`] stays in
//! foundation. This crate sits above scheduler and gestures so the isolate
//! can name both.
#![feature(arbitrary_self_types)]

use std::cell::RefMut;
use std::rc::Rc;
use std::time::{Duration, Instant};

use reveal_embedder::{EmbedderClient, Frame, KeyData, PlatformRef, PointerDataPacket, ViewId};
use reveal_foundation::{App, AppCell};
use reveal_gestures::GestureBinding;
use reveal_painting::PaintingBinding;
use reveal_rendering::RendererBinding;
use reveal_scheduler::SchedulerBinding;
use reveal_services::KeyEventManager;

/// Host-facing isolate: the [`AppCell`] plus the methods the embedder pushes.
///
/// Every push is one turn: the `App` is borrowed for the binding call and released before the
/// cell's checkpoint, where the microtasks and futures the event queued run.
pub struct Shell {
    app: Rc<AppCell>,
    platform: PlatformRef,
    /// The platform time the app clock was last advanced to.
    clock: Duration,
    /// The platform's `now` at its `elapsed` zero, known once a frame has reported its `elapsed`; lets every push, not only a frame or wake, move the app clock to the platform's.
    origin: Option<Instant>,
}

impl Shell {
    /// Builds the heap from `platform`, then runs application setup.
    ///
    /// Setup (later widgets `run_app`) requests the first frame when it
    /// installs work that needs one.
    pub fn new(platform: PlatformRef, setup: impl FnOnce(&mut App)) -> Shell {
        let shell = Shell {
            app: AppCell::with_platform(platform.clone()),
            platform,
            clock: Duration::ZERO,
            origin: None,
        };
        shell.turn(|app| {
            // Flutter's engine collects the platform's fonts before the framework runs.
            PaintingBinding::instance(app).install_platform_fonts(app);
            setup(app);
        });
        shell
    }

    /// The `App`, borrowed until the guard drops. A host that needs a full turn — a borrow, then
    /// the checkpoint — takes [`cell`](Self::cell).
    pub fn app(&self) -> RefMut<'_, App> {
        self.app.borrow_mut()
    }

    pub fn cell(&self) -> &Rc<AppCell> {
        &self.app
    }

    /// One platform event: `f` on the borrowed `App`, then the checkpoint.
    fn turn<R>(&self, f: impl FnOnce(&mut App) -> R) -> R {
        let result = f(&mut self.app.borrow_mut());
        self.app.checkpoint();
        result
    }

    /// An input or platform push: the app clock catches up with the platform's first, as Dart's
    /// timers measure real time from the moment they are created, then the turn runs.
    fn push<R>(&mut self, f: impl FnOnce(&mut App) -> R) -> R {
        if let Some(origin) = self.origin {
            let elapsed = self.platform.now().saturating_duration_since(origin);
            self.advance_clock(elapsed);
        }
        self.turn(f)
    }

    /// Moves the app clock up to the platform's `elapsed`, firing the timers that came due.
    fn advance_clock(&mut self, elapsed: Duration) {
        if elapsed <= self.clock {
            return;
        }
        self.app.elapse(elapsed - self.clock);
        self.clock = elapsed;
    }
}

impl EmbedderClient for Shell {
    fn frame(&mut self, frame: Frame) {
        self.origin = Some(self.platform.now() - frame.elapsed);
        self.advance_clock(frame.elapsed);
        // The engine runs `_beginFrame` and `_drawFrame` as two native tasks: two turns.
        self.turn(|app| SchedulerBinding::handle_begin_frame(app, Some(frame.elapsed)));
        self.turn(SchedulerBinding::handle_draw_frame);
    }

    fn view_added(&mut self, _id: ViewId) {}

    fn view_metrics_changed(&mut self, _id: ViewId) {
        self.push(|app| RendererBinding::instance(app).handle_metrics_changed(app));
    }

    fn view_removed(&mut self, _id: ViewId) {}

    fn pointer_data_packet(&mut self, packet: PointerDataPacket) {
        self.push(|app| GestureBinding::instance(app).handle_pointer_data_packet(app, packet));
    }

    fn key_data(&mut self, data: KeyData) -> bool {
        self.push(|app| KeyEventManager::instance(app).handle_key_data(app, data))
    }

    fn platform_brightness_changed(&mut self) {
        self.push(|app| {
            let callback = app
                .platform_callbacks()
                .on_platform_brightness_changed
                .clone();
            if let Some(callback) = callback {
                callback.call(app);
            }
        });
    }

    fn locales_changed(&mut self) {
        self.push(|app| {
            let callback = app.platform_callbacks().on_locale_changed.clone();
            if let Some(callback) = callback {
                callback.call(app);
            }
        });
    }

    fn wake(&mut self, elapsed: Duration) {
        self.advance_clock(elapsed);
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
        EmbedderClient, Frame, KeyData, KeyEventType, Picture, Platform, PointerChange,
        PointerData, PointerDataPacket, PointerDeviceKind, View, ViewId, ViewMetrics, ViewRef,
    };
    use reveal_foundation::App;
    use reveal_gestures::{GestureBinding, PointerRoute};
    use reveal_scheduler::SchedulerBinding;
    use reveal_services::{HardwareKeyboard, KeyEvent, LogicalKeyboardKey, PhysicalKeyboardKey};

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

        SchedulerBinding::schedule_frame(&mut shell.app());
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

    #[test]
    fn key_data_reaches_the_hardware_keyboard() {
        let seen = Rc::new(Cell::new(0));
        let seen_flag = Rc::clone(&seen);
        let platform = std::rc::Rc::new(RecordingPlatform {
            frames: Arc::new(AtomicUsize::new(0)),
            view: None,
        });
        let mut shell = Shell::new(platform, |app| {
            HardwareKeyboard::instance(app).add_handler(
                app,
                Rc::new(move |_app: &mut App, _event: &KeyEvent| {
                    seen_flag.set(seen_flag.get() + 1);
                    true
                }),
            );
        });

        let key_a = KeyData {
            event_type: KeyEventType::Down,
            physical: PhysicalKeyboardKey::KEY_A.usb_hid_usage,
            logical: LogicalKeyboardKey::KEY_A.key_id,
            character: Some("a".to_owned()),
            ..KeyData::default()
        };
        assert!(shell.key_data(key_a), "the handler claimed the event");
        assert_eq!(seen.get(), 1);

        let keyboard = HardwareKeyboard::instance(&mut shell.app());
        assert!(keyboard.is_logical_key_pressed(&shell.app(), LogicalKeyboardKey::KEY_A));
    }

    #[test]
    fn wake_and_frame_advance_the_app_clock_and_fire_due_timers() {
        let platform: reveal_embedder::PlatformRef = Rc::new(RecordingPlatform {
            frames: Arc::new(AtomicUsize::new(0)),
            view: None,
        });
        let fired = Rc::new(Cell::new(Vec::new()));
        let mut shell = Shell::new(platform, |app| {
            for (name, delay) in [("early", 10), ("late", 30)] {
                let fired = Rc::clone(&fired);
                reveal_foundation::Timer::new(
                    app,
                    Duration::from_millis(delay),
                    reveal_foundation::Listener::new(move |_app| {
                        let mut log = fired.take();
                        log.push(name);
                        fired.set(log);
                    }),
                );
            }
        });
        assert!(fired.take().is_empty());

        shell.wake(Duration::from_millis(15));
        assert_eq!(fired.take(), vec!["early"]);

        shell.frame(Frame {
            elapsed: Duration::from_millis(32),
        });
        assert_eq!(fired.take(), vec!["late"]);
    }
}
