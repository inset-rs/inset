//! [`WidgetTester`]: the app a case runs, its view, and the frames and time it pumps.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use waft_embedder::{RestorationMap, RestorationUpdate, Size, TargetPlatform, ViewRef};
use waft_foundation::{App, AppCell};
use waft_rendering::RendererBinding;
use waft_scheduler::{FrameTarget, SchedulerBinding};
use waft_test::{TestPlatform, TestView};

use crate::{Center, Container, IntoWidget, SizedBox, Text, View, WidgetsBinding};

/// Dart's `WidgetTester`: an app on a [`TestPlatform`] with one view, and the frames, time and
/// input a case gives it.
///
/// Dropped after a case that did not panic, it disposes the tree and checks that no timer is
/// still pending, as flutter_test does after each test. A case that caught a panic out of a
/// pump skips both: the frame unwound and left the tree half built (`PORTING.md`).
pub struct WidgetTester {
    /// The app under test, which the case borrows to read and change the tree.
    pub cell: Rc<AppCell>,
    platform: Rc<TestPlatform>,
    view: Rc<TestView>,
    restoration: Option<Rc<TestRestoration>>,
    /// Whether a pump is running, or began and unwound: a pump that panicked never clears it.
    in_pump: bool,
    /// Dart's `_nextPointer`: the pointer the next finder-taking gesture uses.
    next_pointer: std::cell::Cell<i64>,
}

/// The host side of flutter_test's `TestRestorationManager`: it answers `Restoration::get` with
/// the data it was handed and keeps every `Restoration::put`.
struct TestRestoration {
    stored: Option<RestorationUpdate>,
    puts: RefCell<Vec<RestorationMap>>,
}

impl waft_embedder::Restoration for TestRestoration {
    fn get(&self) -> Option<RestorationUpdate> {
        self.stored.clone()
    }

    fn put(&self, data: RestorationMap) {
        self.puts.borrow_mut().push(data);
    }
}

impl WidgetTester {
    /// The logical size of flutter_test's view.
    pub const SIZE: Size = Size::new(800.0, 600.0);

    /// flutter_test's device pixel ratio.
    pub const DEVICE_PIXEL_RATIO: f64 = 3.0;

    /// A tester on Android, the platform flutter_test runs on.
    pub fn new() -> WidgetTester {
        WidgetTester::on(TargetPlatform::Android)
    }

    /// A tester on `platform`: Dart's `debugDefaultTargetPlatformOverride`.
    pub fn on(platform: TargetPlatform) -> WidgetTester {
        WidgetTester::with_platform(TestPlatform::new().on(platform))
    }

    /// A tester on `platform`, which this gives flutter_test's view; `platform` names the target
    /// (Android unless it says otherwise) and whatever the case's host reports.
    pub fn with_platform(platform: TestPlatform) -> WidgetTester {
        WidgetTester::with_view(platform, Self::default_view())
    }

    /// A tester on `platform` with `view` as its one view, for a case whose view is not
    /// flutter_test's: one with a host of the case's own. A case that only sets the view's
    /// metrics sets them on [`view`](Self::view), as Dart's does.
    pub fn with_view(platform: TestPlatform, view: TestView) -> WidgetTester {
        let view = Rc::new(view);
        let platform = Rc::new(platform.with_view(Rc::clone(&view) as ViewRef));
        let cell = AppCell::with_platform(Rc::clone(&platform) as waft_embedder::PlatformRef);
        let mut tester = WidgetTester {
            cell,
            platform,
            view,
            restoration: None,
            in_pump: false,
            next_pointer: std::cell::Cell::new(1),
        };
        tester.report_view_metrics_changes();
        tester.install_flutter_test_fonts();
        tester.pump_pre_test_tree();
        tester
    }

    /// Makes the view's metrics changes reach the framework as a host's do: dart:ui's
    /// `onMetricsChanged`, which the framework sets to `RendererBinding.handleMetricsChanged`,
    /// as one platform event, the checkpoint after it.
    fn report_view_metrics_changes(&self) {
        let cell = Rc::downgrade(&self.cell);
        self.view.set_metrics_listener(Rc::new(move |_view| {
            let Some(cell) = cell.upgrade() else {
                return;
            };
            {
                let mut app = cell.borrow_mut();
                RendererBinding::instance(&mut app).handle_metrics_changed(&mut app);
            }
            cell.checkpoint();
        }));
    }

    /// flutter_test's `_runTestBody` before the case: `runApp(Container(key: UniqueKey(), child:
    /// _preTestMessage))` and a pump, which resets the tree to a known state. The case's first
    /// [`pump_widget`](Self::pump_widget) then replaces a tree, building it within its frame, as
    /// Dart's does.
    fn pump_pre_test_tree(&mut self) {
        self.pump_widget(
            Container::new()
                .key(waft_foundation::UniqueKey::new().into())
                .child(
                    Center::new().child(
                        Text::new("Test starting...")
                            .style(
                                waft_painting::TextStyle::new()
                                    .color(waft_embedder::Color::new(0xFF917FFF))
                                    .font_size(40.0),
                            )
                            .text_direction(waft_embedder::TextDirection::Ltr),
                    ),
                ),
        );
    }

    /// A tester on Android whose host restores `data`: the app that Dart's
    /// `tester.restartAndRestore()` and `tester.restoreFrom(data)` start again, which the case
    /// pumps its widget into once more.
    pub fn restoring(data: Option<RestorationMap>) -> WidgetTester {
        let restoration = Rc::new(TestRestoration {
            stored: Some(RestorationUpdate {
                enabled: true,
                data,
            }),
            puts: RefCell::new(Vec::new()),
        });
        let platform = TestPlatform::new()
            .on(TargetPlatform::Android)
            .with_restoration(Rc::clone(&restoration) as Rc<dyn waft_embedder::Restoration>);
        let mut tester = WidgetTester::with_platform(platform);
        tester.restoration = Some(restoration);
        tester
    }

    /// flutter_test's view: 2400x1800 physical pixels at 3.0, which is 800x600 logical.
    fn default_view() -> TestView {
        TestView::with_pixel_ratio(
            WidgetTester::SIZE.width() * WidgetTester::DEVICE_PIXEL_RATIO,
            WidgetTester::SIZE.height() * WidgetTester::DEVICE_PIXEL_RATIO,
            WidgetTester::DEVICE_PIXEL_RATIO,
        )
    }

    /// flutter_test's fonts: every family is FlutterTest.
    fn install_flutter_test_fonts(&self) {
        let mut app = self.cell.borrow_mut();
        let binding = waft_painting::PaintingBinding::instance(&mut app);
        binding.install_platform_fonts(&mut app);
    }

    /// The platform the app runs on, for a case that changes what the host reports while it
    /// runs: Dart's `tester.platformDispatcher.*TestValue`.
    pub fn platform(&self) -> &Rc<TestPlatform> {
        &self.platform
    }

    /// The view the tree is in, whose host records what the framework asks of it and whose
    /// metrics a case sets: Dart's `tester.view`. A metric set reaches the framework at once, so
    /// a case sets it while it does not hold the app.
    pub fn view(&self) -> &Rc<TestView> {
        &self.view
    }

    /// Dart's `tester.binding.setSurfaceSize(size)`: the view's render view is laid out at `size`
    /// logical pixels, or at the view's own size again for `None`, while the view's metrics, and
    /// so the `MediaQuery`, stay as they are.
    pub fn set_surface_size(&self, size: Option<Size>) {
        {
            let mut app = self.cell.borrow_mut();
            let view = waft_embedder::View::id(&*self.view);
            RendererBinding::instance(&mut app).set_surface_size(&mut app, view, size);
        }
        self.cell.checkpoint();
    }

    /// The target platform the case runs on: Dart's `defaultTargetPlatform`, which a
    /// [`TargetPlatformVariant`](super::TargetPlatformVariant) sets.
    pub fn target_platform(&self) -> TargetPlatform {
        waft_embedder::Platform::target_platform(&*self.platform)
    }

    /// The view's device pixel ratio, which turns a case's logical positions into the physical
    /// ones the host reports.
    pub fn device_pixel_ratio(&self) -> f64 {
        waft_embedder::View::metrics(&*self.view).device_pixel_ratio
    }

    /// Dart's `tester.hasRunningAnimations`: whether any transient callback is scheduled, which
    /// a running [`Ticker`](waft_scheduler::Ticker) registers one per frame.
    pub fn has_running_animations(&self) -> bool {
        SchedulerBinding::transient_callback_count(&mut self.cell.borrow_mut()) > 0
    }

    /// Dart's `tester.getRestorationData()`: what the host holds, which is what the app last
    /// handed it, or else what it restored the app from.
    pub fn restoration_data(&self) -> Option<RestorationMap> {
        let restoration = self
            .restoration
            .as_ref()
            .expect("a tester made by WidgetTester::restoring");
        let last_put = restoration.puts.borrow().last().cloned();
        last_put.or_else(|| {
            restoration
                .stored
                .as_ref()
                .and_then(|stored| stored.data.clone())
        })
    }

    /// Dart's `tester.pumpWidget(widget)`: makes `widget` the view's child, schedules a frame
    /// and pumps once.
    pub fn pump_widget<K>(&mut self, widget: impl IntoWidget<K>) {
        self.attach_and_schedule(widget);
        self.pump();
    }

    /// Dart's `binding.attachRootWidget(binding.wrapWithDefaultView(widget))` and
    /// `binding.scheduleFrame()`.
    fn attach_and_schedule<K>(&mut self, widget: impl IntoWidget<K>) {
        self.in_pump = true;
        let mut app = self.cell.borrow_mut();
        let view = Rc::clone(&self.view) as ViewRef;
        let view_id = view.id();
        let root = View::new(view, widget.into_widget()).into_widget();
        WidgetsBinding::instance(&mut app).attach_root_widget(&mut app, root);
        SchedulerBinding::schedule_frame(&mut app, FrameTarget::View(view_id));
    }

    /// Dart's `tester.pumpFrames(target, maxDuration)`: makes `widget` the view's child,
    /// schedules a frame, and pumps frames 16.683ms apart until `max_duration` has passed.
    pub fn pump_frames<K>(&mut self, widget: impl IntoWidget<K>, max_duration: Duration) {
        self.pump_frames_every(widget, max_duration, Duration::from_micros(16_683));
    }

    /// Dart's `tester.pumpFrames(target, maxDuration, interval)`.
    pub fn pump_frames_every<K>(
        &mut self,
        widget: impl IntoWidget<K>,
        max_duration: Duration,
        interval: Duration,
    ) {
        self.attach_and_schedule(widget);
        let mut elapsed = Duration::ZERO;
        while elapsed < max_duration {
            self.pump_by(interval);
            elapsed += interval;
        }
    }

    /// Dart's `tester.nextPointer`: the pointer a finder-taking gesture uses, numbered from 1.
    pub fn next_pointer(&self) -> i64 {
        let pointer = self.next_pointer.get();
        self.next_pointer.set(pointer + 1);
        pointer
    }

    /// Dart's `tester.pump()`: a frame now, if a view asked for one, with no time passing and
    /// so no timer firing.
    pub fn pump(&mut self) {
        self.frame_if_asked();
    }

    /// Dart's `tester.pump(duration)`: `duration` passes on the app's clock, firing the timers
    /// due by then, and then a frame, if a view asked for one.
    pub fn pump_by(&mut self, duration: Duration) {
        self.cell.elapse(duration);
        self.frame_if_asked();
    }

    /// Dart's `tester.pumpAndSettle()`: 100ms frames until none is scheduled; answers how many
    /// it ran.
    pub fn pump_and_settle(&mut self) -> u32 {
        self.pump_and_settle_by(Duration::from_millis(100))
    }

    /// Dart's `tester.pumpAndSettle(duration)`: frames `duration` apart, at least one, until
    /// none is scheduled, failing once its default 10 minutes of test time have passed; answers
    /// how many frames it ran.
    pub fn pump_and_settle_by(&mut self, duration: Duration) -> u32 {
        let end_time = self.cell.borrow().clock() + Duration::from_secs(10 * 60);
        let mut count = 0;
        loop {
            assert!(
                self.cell.borrow().clock() <= end_time,
                "pumpAndSettle timed out"
            );
            self.pump_by(duration);
            count += 1;
            if !SchedulerBinding::has_scheduled_frame(&mut self.cell.borrow_mut()) {
                return count;
            }
        }
    }

    /// A frame at the app's clock of every view that asked for one since the last, pumped as the
    /// shell runs one: a checkpoint before the frame, between its two halves and after it,
    /// standing in for flutter_test's microtask flush, so an awaited continuation resumes in the
    /// frame it would in a running app.
    fn frame_if_asked(&mut self) {
        self.in_pump = true;
        let (views, now) = {
            let app = self.cell.borrow();
            (self.platform.take_frame_requests(), app.clock())
        };
        self.cell.checkpoint();
        if !views.is_empty() {
            SchedulerBinding::handle_begin_frame(&mut self.cell.borrow_mut(), &views, Some(now));
            self.cell.checkpoint();
            SchedulerBinding::handle_draw_frame(&mut self.cell.borrow_mut(), &views);
        }
        self.cell.checkpoint();
        self.in_pump = false;
    }

    /// What Flutter's leak tracking checks: `cycle` (say, opening and closing a route) leaves no
    /// object behind in the arena. The first run warms up what is made once and kept, such as a
    /// singleton or a theme's cached value; the second is checked with
    /// [`waft_test::assert_no_new_objects`], which names each object left behind with its type
    /// and creation site.
    #[cfg(debug_assertions)]
    pub fn assert_leaves_nothing_behind(&mut self, mut cycle: impl FnMut(&mut WidgetTester)) {
        cycle(self);
        self.cell.checkpoint();
        let snapshot = waft_test::object_snapshot(&self.cell.borrow());
        cycle(self);
        self.cell.checkpoint();
        waft_test::assert_no_new_objects(&self.cell.borrow(), &snapshot);
    }

    /// Runs `body` with the app borrowed: a case's reads and changes that are not a pump.
    pub fn update<R>(&self, body: impl FnOnce(&mut App) -> R) -> R {
        body(&mut self.cell.borrow_mut())
    }
}

impl Default for WidgetTester {
    fn default() -> WidgetTester {
        WidgetTester::new()
    }
}

/// flutter_test's checks after a test: the tree is disposed, and then no timer may be pending.
impl Drop for WidgetTester {
    fn drop(&mut self) {
        if std::thread::panicking() || self.in_pump {
            return;
        }
        self.pump_widget(SizedBox::shrink());
        assert!(
            !self.cell.borrow().has_pending_timers(),
            "A Timer is still pending even after the widget tree was disposed."
        );
    }
}

/// flutter_test's `TestVSync`: vends ordinary tickers, in no view.
pub struct TestVSync;

impl waft_scheduler::TickerProvider for TestVSync {
    fn create_ticker(
        &self,
        app: &mut App,
        on_tick: waft_scheduler::TickerCallback,
    ) -> waft_foundation::Ref<waft_scheduler::Ticker> {
        waft_scheduler::Ticker::new(app, on_tick, None)
    }
}
