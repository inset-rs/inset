//! The gallery under a driver that behaves like the winit embedder: frames only when requested, timers only when the dispatcher's `wake_at` deadline is reached or a ready task asks with `wake_now`, the app clock advanced only by frames and wakes.
#![feature(arbitrary_self_types)]
use inset_embedder::valo::Context;
use inset_embedder::{
    Contact, Dispatcher, EmbedderClient, FontSource, Frame, Instant, Offset, Platform,
    PointerDeviceKind, PointerReport, ReportedChange, SystemFontSource, TargetPlatform, View,
    ViewId, ViewRef,
};
use inset_rendering::RenderParagraph;
use inset_shell::Shell;
use inset_widgets::WidgetsBinding;
use inset_winui_test_support::CaptureView;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Records the next turn the framework asks for, so the driver can fire it when that time comes.
struct LoopDispatcher {
    deadline: Mutex<Option<Instant>>,
    /// A ready task asked to be woken, which the next wake clears.
    wake_now: AtomicBool,
}

impl LoopDispatcher {
    fn new() -> Arc<LoopDispatcher> {
        Arc::new(LoopDispatcher {
            deadline: Mutex::new(None),
            wake_now: AtomicBool::new(false),
        })
    }

    /// Whether the framework is to be woken by `now`: a ready task asked, or its timer is due.
    fn take_wake(&self, now: Instant) -> bool {
        let asked = self.wake_now.swap(false, Ordering::SeqCst);
        asked || self.due(now).is_some()
    }

    fn due(&self, now: Instant) -> Option<Instant> {
        let mut deadline = self
            .deadline
            .lock()
            .expect("the deadline is never poisoned");
        match *deadline {
            Some(at) if at <= now => deadline.take(),
            _ => None,
        }
    }

    fn next(&self) -> Option<Instant> {
        *self
            .deadline
            .lock()
            .expect("the deadline is never poisoned")
    }
}

impl Dispatcher for LoopDispatcher {
    fn wake_at(&self, timer_wakeup: Option<Instant>) {
        *self
            .deadline
            .lock()
            .expect("the deadline is never poisoned") = timer_wakeup;
    }

    fn wake_now(&self) {
        self.wake_now.store(true, Ordering::SeqCst);
    }

    fn dispatch(&self, work: Box<dyn FnOnce() + Send>) {
        work();
    }
}

struct LoopPlatform {
    view: Rc<CaptureView>,
    origin: Instant,
    clock: Cell<Duration>,
    frame_requested: Cell<bool>,
    dispatcher: Arc<LoopDispatcher>,
}
impl Platform for LoopPlatform {
    fn target_platform(&self) -> TargetPlatform {
        TargetPlatform::MacOS
    }
    /// The one view's frame, at the driver's next tick.
    fn request_frame(&self, _view: ViewId) {
        self.frame_requested.set(true);
    }
    fn now(&self) -> Instant {
        self.origin + self.clock.get()
    }
    fn dispatcher(&self) -> Arc<dyn Dispatcher> {
        Arc::clone(&self.dispatcher) as Arc<dyn Dispatcher>
    }
    fn views(&self) -> Vec<ViewRef> {
        vec![self.view.clone()]
    }
    fn view(&self, id: ViewId) -> Option<ViewRef> {
        (id == ViewId(0)).then(|| self.view.clone() as ViewRef)
    }
    fn font_source(&self) -> Option<Box<dyn FontSource>> {
        Some(Box::new(SystemFontSource::platform()))
    }
}

struct Driver {
    platform: Rc<LoopPlatform>,
    shell: Shell,
    log: Vec<String>,
}
impl Driver {
    fn new() -> Driver {
        let (device, queue) =
            inset_winui_test_support::gpu::headless_device().expect("GPU required");
        let view = Rc::new(CaptureView {
            size: [900, 2000],
            renderer: RefCell::new(Context::new(device, queue)),
            pixels: RefCell::new(Vec::new()),
        });
        let platform = Rc::new(LoopPlatform {
            view,
            origin: Instant::now(),
            // Real time has passed by the time the embedder's first event arrives.
            clock: Cell::new(Duration::from_millis(1)),
            frame_requested: Cell::new(false),
            dispatcher: LoopDispatcher::new(),
        });
        let shell = Shell::new(platform.clone(), |app| {
            winui_gallery::run_feature(app, winui_gallery::Feature::RepeatButton)
        });
        let mut driver = Driver {
            platform,
            shell,
            log: Vec::new(),
        };
        driver.run_until(Duration::from_millis(500));
        driver
    }

    /// The winit loop: a requested frame runs at the next 16 ms tick; a reached deadline wakes the client; otherwise time passes.
    fn run_until(&mut self, end: Duration) {
        while self.platform.clock.get() < end {
            let now = self.platform.clock.get();
            if self.platform.frame_requested.replace(false) {
                self.log.push(format!("{now:?} frame"));
                self.shell
                    .frame(&[self.platform.view.id()], Frame { elapsed: now });
                // The display's next vsync.
                self.platform.clock.set(now + Duration::from_millis(16));
            } else if self.platform.dispatcher.take_wake(self.platform.now()) {
                self.log.push(format!("{now:?} wake"));
                self.shell.wake(now);
            } else {
                let next = self
                    .platform
                    .dispatcher
                    .next()
                    .map(|d| d.saturating_duration_since(self.platform.origin))
                    .filter(|d| *d > now)
                    .unwrap_or(end)
                    .min(end)
                    .min(now + Duration::from_millis(16));
                self.platform.clock.set(next);
            }
        }
    }

    fn mouse(&mut self, change: ReportedChange, point: Offset) {
        self.log
            .push(format!("{:?} {change:?}", self.platform.clock.get()));
        self.shell.pointer_reports(vec![PointerReport {
            view_id: ViewId(0),
            time_stamp: self.platform.clock.get(),
            device: 0,
            kind: PointerDeviceKind::Mouse,
            position: [point.dx(), point.dy()],
            change,
            contact: Contact::default(),
        }]);
    }

    fn find(&self, text: &str) -> Option<Offset> {
        let cell = self.shell.cell();
        let mut app = cell.borrow_mut();
        let root = WidgetsBinding::instance(&mut app)
            .root_element(&app)
            .unwrap();
        let mut all = vec![root];
        let mut i = 0;
        while i < all.len() {
            all.extend(all[i].children(&app));
            i += 1;
        }
        for element in all {
            if let Some(object) = element.render_object(&app)
                && let Some(paragraph) = object.downcast::<RenderParagraph>(&app)
                && paragraph.text(&app).to_plain_text(true, true) == text
            {
                let object = object.as_box().unwrap();
                let size = object.size(&app);
                return Some(
                    object.local_to_global(&app, Offset::ZERO, None)
                        + Offset::new(size.width() / 2.0, size.height() / 2.0),
                );
            }
        }
        None
    }

    fn status(&self, prefix: &str) -> String {
        let cell = self.shell.cell();
        let mut app = cell.borrow_mut();
        let root = WidgetsBinding::instance(&mut app)
            .root_element(&app)
            .unwrap();
        let mut all = vec![root];
        let mut i = 0;
        while i < all.len() {
            all.extend(all[i].children(&app));
            i += 1;
        }
        for element in all {
            if let Some(object) = element.render_object(&app)
                && let Some(paragraph) = object.downcast::<RenderParagraph>(&app)
            {
                let t = paragraph.text(&app).to_plain_text(true, true);
                if t.starts_with(prefix) {
                    return t;
                }
            }
        }
        String::new()
    }
}

#[test]
fn repeat_button_repeats_under_the_winit_loop() {
    let mut driver = Driver::new();
    let hold_me = driver.find("Hold me").expect("Hold me");
    driver.mouse(ReportedChange::Add, hold_me - Offset::new(50.0, 50.0));
    driver.mouse(ReportedChange::Hover { buttons: 0 }, hold_me);
    // Idle long enough for the app clock to fall behind the platform's between frames.
    driver.run_until(Duration::from_millis(1000));
    driver.mouse(ReportedChange::Down { buttons: 1 }, hold_me);
    driver.run_until(Duration::from_millis(1400));
    // `Delay` is 500 ms from the press, on the platform's clock.
    assert_eq!(driver.status("RepeatButton:"), "RepeatButton: 1 clicks");
    driver.run_until(Duration::from_millis(1600));
    let after_delay = driver.status("RepeatButton:");
    driver.run_until(Duration::from_millis(2500));
    let held = driver.status("RepeatButton:");
    driver.mouse(ReportedChange::Up { buttons: 0 }, hold_me);
    driver.run_until(Duration::from_millis(3000));
    let released = driver.status("RepeatButton:");
    let clicks = |status: &str| {
        status
            .trim_start_matches("RepeatButton: ")
            .trim_end_matches(" clicks")
            .parse::<u32>()
            .unwrap()
    };
    assert!(
        clicks(&after_delay) >= 3,
        "{after_delay} at 1.6 s; log:\n{}",
        driver.log.join("\n")
    );
    // The delay at 1500 ms, then 1000 ms of repeats at 33 ms: 32, give or take a frame.
    assert!(
        (30..=34).contains(&clicks(&held)),
        "{held} at 2.5 s; log:\n{}",
        driver.log.join("\n")
    );
    assert_eq!(released, held, "no clicks after the release");
}
