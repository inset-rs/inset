//! Shared test scaffolding: a platform with one view to run widgets under, on the target
//! platform a case names, and pumped frames.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use waft_embedder::{TargetPlatform, ViewId, ViewRef};
use waft_foundation::{App, AppCell};
use waft_scheduler::SchedulerBinding;
use waft_test::{TestPlatform, TestView};
use waft_widgets::{BuildContext, Builder, IntoWidget, SizedBox, View, WidgetRef, run_widget};

/// The 800x600 physical view at 2x every test app draws into.
pub(crate) fn test_view() -> ViewRef {
    Rc::new(TestView::with_pixel_ratio(800.0, 600.0, 2.0))
}

/// An app on Android, the platform Flutter's tests run on, with one 2x view.
pub(crate) fn test_cell() -> Rc<AppCell> {
    test_cell_on(TargetPlatform::Android)
}

/// An app on `platform` with one 2x view, so pointer packets find their view.
pub(crate) fn test_cell_on(platform: TargetPlatform) -> Rc<AppCell> {
    let platform = TestPlatform::new().on(platform).with_view(test_view());
    AppCell::with_platform(Rc::new(platform))
}

/// Mounts `child` under the platform's view and runs the first frame.
pub(crate) fn build(cell: &AppCell, child: WidgetRef) {
    {
        let mut app = cell.borrow_mut();
        let view = app
            .platform()
            .views()
            .into_iter()
            .next()
            .expect("an app from test_support::test_cell");
        run_widget(&mut app, View::new(view, child).into_widget());
    }
    // `run_app` attaches the root widget on the next timer turn, as Dart's `Timer.run` does.
    cell.elapse(Duration::ZERO);
    pump(&mut cell.borrow_mut(), Duration::ZERO);
}

/// Runs one frame at `at`, of every view the platform has.
pub(crate) fn pump(app: &mut App, at: Duration) {
    let views: Vec<ViewId> = app
        .platform()
        .views()
        .iter()
        .map(|view| view.id())
        .collect();
    SchedulerBinding::handle_begin_frame(app, &views, Some(at));
    app.drain_microtasks();
    SchedulerBinding::handle_draw_frame(app, &views);
    app.drain_microtasks();
}

/// The stack a case runs on: the 8 MiB a platform's main thread gives the framework, and
/// 1 MiB more, which a debug build's theme animation (`ThemeData::lerp` of two 180 KB themes)
/// came to need once `ThemeData` held a `TabBarThemeData`.
const MAIN_THREAD_STACK_SIZE: usize = 9 * 1024 * 1024;

/// Runs `body` on a thread with a main thread's stack, re-raising its panic.
///
/// A debug build of `ThemeData::new` alone takes over 1 MiB of stack (a `TextStyle` is 1232
/// bytes, and a theme holds 105 of them), so a case that keeps two themes, or lerps them, does
/// not fit the 2 MiB a test thread has.
pub(crate) fn with_main_thread_stack(body: impl FnOnce() + Send) {
    std::thread::scope(|scope| {
        let joined = std::thread::Builder::new()
            .stack_size(MAIN_THREAD_STACK_SIZE)
            .spawn_scoped(scope, body)
            .expect("a thread for the case")
            .join();
        if let Err(panic) = joined {
            std::panic::resume_unwind(panic);
        }
    });
}

/// Builds `wrap(probe)` on `cell` and returns what `read` saw at the probe's context.
pub(crate) fn probe<T: 'static>(
    cell: &AppCell,
    wrap: impl FnOnce(&App, WidgetRef) -> WidgetRef,
    read: impl Fn(&mut App, BuildContext) -> T + 'static,
) -> T {
    let seen = Rc::new(RefCell::new(None));
    let probe = Builder::new({
        let seen = Rc::clone(&seen);
        move |app, context| {
            *seen.borrow_mut() = Some(read(app, context));
            SizedBox::shrink().into_widget()
        }
    })
    .into_widget();
    let root = wrap(&cell.borrow(), probe);
    build(cell, root);
    let value = seen.borrow_mut().take();
    value.expect("the probe built")
}

/// flutter_test's `TestVSync`: vends ordinary tickers, in no view.
pub(crate) struct TestVSync;

impl waft_scheduler::TickerProvider for TestVSync {
    fn create_ticker(
        &self,
        app: &mut App,
        on_tick: waft_scheduler::TickerCallback,
    ) -> waft_foundation::Handle<waft_scheduler::Ticker> {
        waft_scheduler::Ticker::new(app, on_tick, None)
    }
}

/// Dart's `MaterialApp(theme: theme, home: home)`.
pub(crate) fn material_app(theme: crate::theme_data::ThemeData, home: WidgetRef) -> WidgetRef {
    crate::app::MaterialApp::new()
        .theme(theme)
        .home(home)
        .into_widget()
}

/// Dart's `MaterialApp(restorationScopeId: restoration_scope_id, theme: theme, home: home)`.
pub(crate) fn restorable_material_app(
    restoration_scope_id: &str,
    theme: crate::theme_data::ThemeData,
    home: WidgetRef,
) -> WidgetRef {
    crate::app::MaterialApp::new()
        .restoration_scope_id(restoration_scope_id)
        .theme(theme)
        .home(home)
        .into_widget()
}

/// Dart's `MaterialApp(theme: theme, ..)` with the other arguments `configure` gives.
pub(crate) fn material_page_app(
    theme: crate::theme_data::ThemeData,
    configure: impl FnOnce(crate::app::MaterialApp) -> crate::app::MaterialApp,
) -> WidgetRef {
    configure(crate::app::MaterialApp::new().theme(theme)).into_widget()
}

/// Dart's `Text(text)` as Flutter's tests lay it out, in flutter_test's default font,
/// FlutterTest, whose glyphs are squares of the font size: one square per character at the
/// ambient `DefaultTextStyle`'s font size (14 if it gives none), scaled by the ambient text
/// scaler. The test platform has no FlutterTest, so this box stands in where a case measures
/// text.
pub(crate) fn ahem_text(text: &str) -> Builder {
    let characters = text.chars().count() as f64;
    Builder::new(move |app, context| {
        let font_size = waft_widgets::DefaultTextStyle::of(app, context)
            .style
            .font_size
            .unwrap_or(14.0);
        let square = waft_widgets::MediaQuery::text_scaler_of(app, context).scale(font_size);
        SizedBox::new()
            .width(square * characters)
            .height(square)
            .into_widget()
    })
}

/// Dart's `Text(text, textScaler:)` as Flutter's tests lay it out in flutter_test's default
/// font, FlutterTest, where [`ahem_text`] is not enough because a case reads line heights or
/// baselines: one square of the font size per character of the longest line, one line per
/// `\n`, each line the font size times the style's `height` rounded as SkParagraph rounds a
/// line, each glyph followed by the style's `letter_spacing`, and an alphabetic baseline at
/// FlutterTest's 0.75 em ascent, spread over that height (Flutter's proportional leading) or
/// below half the extra height (`TextLeadingDistribution::Even`).
/// The font size is the ambient `DefaultTextStyle`'s (14 if it gives none), scaled by
/// [`text_scaler`](Self::text_scaler) or else the ambient text scaler.
#[derive(Clone, Debug)]
pub(crate) struct AhemText {
    pub(crate) key: Option<waft_widgets::KeyRef>,
    pub(crate) text: String,
    pub(crate) text_scaler: Option<waft_painting::TextScaler>,
}

impl AhemText {
    /// Dart's `Text(text)`.
    pub(crate) fn new(text: &str) -> AhemText {
        AhemText {
            key: None,
            text: text.to_owned(),
            text_scaler: None,
        }
    }

    /// Dart `Text(textScaler:)`.
    pub(crate) fn text_scaler(mut self, text_scaler: waft_painting::TextScaler) -> AhemText {
        self.text_scaler = Some(text_scaler);
        self
    }
}

impl waft_widgets::StatelessWidget for AhemText {
    fn key(&self) -> Option<&waft_widgets::KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let style = waft_widgets::DefaultTextStyle::of(app, context).style;
        let text_scaler = self
            .text_scaler
            .clone()
            .unwrap_or_else(|| waft_widgets::MediaQuery::text_scaler_of(app, context));
        let font_size = text_scaler.scale(style.font_size.unwrap_or(14.0));
        let height = style.height.unwrap_or(1.0);
        let lines: Vec<&str> = self.text.split('\n').collect();
        let longest = lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0) as f64;
        let line_height = (font_size * height).round();
        // SkParagraph adds the style's letter spacing, which the text scaler leaves alone, after
        // every glyph.
        let letter_spacing = style.letter_spacing.unwrap_or(0.0);
        // The style's `height` grows the line: Flutter's proportional leading scales the
        // ascent and descent, its even leading puts half the extra space above the ascent.
        let (leading_above, ascent_scale) = match style.leading_distribution {
            Some(waft_embedder::TextLeadingDistribution::Even) => {
                ((font_size * height - font_size) / 2.0, 1.0)
            }
            _ => (0.0, height),
        };
        AhemBox {
            size: waft_embedder::Size::new(
                longest * (font_size + letter_spacing),
                line_height * lines.len() as f64,
            ),
            alphabetic_baseline: leading_above + 0.75 * font_size * ascent_scale,
            ideographic_baseline: leading_above + font_size * ascent_scale,
        }
        .into_widget()
    }
}

/// The leaf an [`AhemText`] builds: a box of a fixed size with fixed baselines.
#[derive(Debug)]
struct AhemBox {
    size: waft_embedder::Size,
    alphabetic_baseline: f64,
    ideographic_baseline: f64,
}

impl waft_widgets::RenderObjectWidget for AhemBox {
    type RenderObject = RenderAhemBox;

    fn create_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
    ) -> waft_rendering::AnyRenderObject {
        use waft_rendering::RenderBox;
        waft_rendering::RenderHandle::new_box(
            app,
            RenderAhemBox {
                render_object: waft_rendering::RenderObjectData::new(),
                render_box: waft_rendering::RenderBoxData::new(),
                size: self.size,
                alphabetic_baseline: self.alphabetic_baseline,
                ideographic_baseline: self.ideographic_baseline,
            },
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: waft_rendering::RenderHandle<RenderAhemBox>,
    ) {
        use waft_rendering::RenderBox;
        let render = render_object.get_mut(app);
        render.size = self.size;
        render.alphabetic_baseline = self.alphabetic_baseline;
        render.ideographic_baseline = self.ideographic_baseline;
        render_object.mark_needs_layout(app);
    }
}

impl waft_widgets::LeafRenderObjectWidget for AhemBox {}

struct RenderAhemBox {
    render_object: waft_rendering::RenderObjectData,
    render_box: waft_rendering::RenderBoxData,
    size: waft_embedder::Size,
    alphabetic_baseline: f64,
    ideographic_baseline: f64,
}

impl RenderAhemBox {
    fn baseline(&self, baseline: waft_embedder::TextBaseline) -> f64 {
        match baseline {
            waft_embedder::TextBaseline::Alphabetic => self.alphabetic_baseline,
            waft_embedder::TextBaseline::Ideographic => self.ideographic_baseline,
        }
    }
}

impl waft_rendering::RenderObject for RenderAhemBox {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: waft_rendering::RenderHandle<Self>, app: &mut App) {
        use waft_rendering::RenderBox;
        let size = self.constraints(app).constrain(self.get(app).size);
        self.set_size(app, size);
    }
}

impl waft_rendering::RenderBox for RenderAhemBox {
    waft_rendering::render_box_accessors!();

    fn compute_min_intrinsic_width(
        self: waft_rendering::RenderHandle<Self>,
        app: &mut App,
        _height: f64,
    ) -> f64 {
        self.get(app).size.width()
    }

    fn compute_max_intrinsic_width(
        self: waft_rendering::RenderHandle<Self>,
        app: &mut App,
        _height: f64,
    ) -> f64 {
        self.get(app).size.width()
    }

    fn compute_min_intrinsic_height(
        self: waft_rendering::RenderHandle<Self>,
        app: &mut App,
        _width: f64,
    ) -> f64 {
        self.get(app).size.height()
    }

    fn compute_max_intrinsic_height(
        self: waft_rendering::RenderHandle<Self>,
        app: &mut App,
        _width: f64,
    ) -> f64 {
        self.get(app).size.height()
    }

    fn compute_dry_layout(
        self: waft_rendering::RenderHandle<Self>,
        app: &mut App,
        constraints: waft_rendering::BoxConstraints,
    ) -> waft_embedder::Size {
        constraints.constrain(self.get(app).size)
    }

    fn compute_dry_baseline(
        self: waft_rendering::RenderHandle<Self>,
        app: &mut App,
        _constraints: waft_rendering::BoxConstraints,
        baseline: waft_embedder::TextBaseline,
    ) -> Option<f64> {
        Some(self.get(app).baseline(baseline))
    }

    fn compute_distance_to_actual_baseline(
        self: waft_rendering::RenderHandle<Self>,
        app: &mut App,
        baseline: waft_embedder::TextBaseline,
    ) -> Option<f64> {
        Some(self.get(app).baseline(baseline))
    }
}

/// Dart's `WidgetTester` over an 800x600 logical view at 2x, the size Flutter's tests lay
/// out in: a fake clock that frames and timers share.
pub(crate) struct WidgetTester {
    pub(crate) cell: Rc<AppCell>,
    restoration: Option<Rc<TestRestoration>>,
    now: Duration,
    mounted: bool,
}

/// The host side of flutter_test's `TestRestorationManager`: it answers `Restoration::get` with
/// the data it was handed and keeps every `Restoration::put`.
struct TestRestoration {
    stored: Option<waft_embedder::RestorationUpdate>,
    puts: RefCell<Vec<waft_embedder::RestorationMap>>,
}

impl waft_embedder::Restoration for TestRestoration {
    fn get(&self) -> Option<waft_embedder::RestorationUpdate> {
        self.stored.clone()
    }

    fn put(&self, data: waft_embedder::RestorationMap) {
        self.puts.borrow_mut().push(data);
    }
}

/// A [`WidgetTester`]'s view: `physical_size` at 2x.
fn tester_view(physical_size: waft_embedder::Size) -> ViewRef {
    Rc::new(TestView::with_pixel_ratio(
        physical_size.width(),
        physical_size.height(),
        2.0,
    ))
}

/// One pointer a [`WidgetTester`] drives, as Dart's `TestGesture`.
#[derive(Clone, Copy)]
pub(crate) struct TestPointer {
    pub(crate) id: i64,
    pub(crate) kind: waft_embedder::PointerDeviceKind,
    pub(crate) buttons: i64,
    pub(crate) position: waft_embedder::Offset,
}

impl WidgetTester {
    /// The logical size of the view.
    pub(crate) const SIZE: waft_embedder::Size = waft_embedder::Size::new(800.0, 600.0);

    /// A tester on `platform`.
    pub(crate) fn on(platform: TargetPlatform) -> WidgetTester {
        WidgetTester::with_platform(TestPlatform::new().on(platform))
    }

    /// A tester on `platform`, which this gives the tester's view.
    pub(crate) fn with_platform(platform: TestPlatform) -> WidgetTester {
        WidgetTester::with_physical_size(
            platform,
            waft_embedder::Size::new(
                WidgetTester::SIZE.width() * 2.0,
                WidgetTester::SIZE.height() * 2.0,
            ),
        )
    }

    /// A tester on `platform` whose 2x view is `physical_size`: Dart's
    /// `tester.view.physicalSize = physical_size`.
    pub(crate) fn with_physical_size(
        platform: TestPlatform,
        physical_size: waft_embedder::Size,
    ) -> WidgetTester {
        WidgetTester::over(Rc::new(platform.with_view(tester_view(physical_size))))
    }

    /// A tester on `platform`, which is handed back so the case can change what the host
    /// reports while the app runs: Dart's `tester.platformDispatcher.*TestValue`.
    pub(crate) fn sharing(platform: TestPlatform) -> (WidgetTester, Rc<TestPlatform>) {
        let physical_size = waft_embedder::Size::new(
            WidgetTester::SIZE.width() * 2.0,
            WidgetTester::SIZE.height() * 2.0,
        );
        let platform = Rc::new(platform.with_view(tester_view(physical_size)));
        (WidgetTester::over(Rc::clone(&platform)), platform)
    }

    fn over(platform: Rc<TestPlatform>) -> WidgetTester {
        WidgetTester {
            cell: AppCell::with_platform(platform),
            restoration: None,
            now: Duration::ZERO,
            mounted: false,
        }
    }

    /// A tester on Android whose host restores `data`: the app that Dart's
    /// `tester.restartAndRestore()` and `tester.restoreFrom(data)` start again, which the case
    /// pumps its widget into once more.
    pub(crate) fn restoring(data: Option<waft_embedder::RestorationMap>) -> WidgetTester {
        let restoration = Rc::new(TestRestoration {
            stored: Some(waft_embedder::RestorationUpdate {
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

    /// Dart's `tester.getRestorationData()`: what the host holds, which is what the app last
    /// handed it, or else what it restored the app from.
    pub(crate) fn restoration_data(&self) -> Option<waft_embedder::RestorationMap> {
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

    /// Dart's `tester.pumpWidget`: makes `widget` the view's child and runs a frame.
    pub(crate) fn pump_widget(&mut self, widget: WidgetRef) {
        if self.mounted {
            let mut app = self.cell.borrow_mut();
            let view = app.platform().views().into_iter().next().expect("one view");
            let root = View::new(view, widget).into_widget();
            waft_widgets::WidgetsBinding::instance(&mut app).attach_root_widget(&mut app, root);
            drop(app);
            self.pump(Duration::ZERO);
        } else {
            self.mounted = true;
            build(&self.cell, widget);
        }
    }

    /// Dart's `tester.pump(duration)`: fires the timers due by then and runs a frame then.
    pub(crate) fn pump(&mut self, duration: Duration) {
        self.now += duration;
        self.cell.elapse(duration);
        pump(&mut self.cell.borrow_mut(), self.now);
        self.cell.checkpoint();
    }

    /// Dart's `tester.pumpAndSettle()`: 100ms frames until none is scheduled; answers how many
    /// it ran.
    pub(crate) fn pump_and_settle(&mut self) -> u32 {
        self.pump_and_settle_by(Duration::from_millis(100))
    }

    /// Dart's `tester.pumpAndSettle(duration)`: frames `duration` apart, at least one, until
    /// none is scheduled, failing once its default 10 minutes of test time have passed; answers
    /// how many frames it ran.
    pub(crate) fn pump_and_settle_by(&mut self, duration: Duration) -> u32 {
        let end_time = self.now + Duration::from_secs(10 * 60);
        let mut count = 0;
        loop {
            assert!(self.now <= end_time, "pumpAndSettle timed out");
            self.pump(duration);
            count += 1;
            if !SchedulerBinding::has_scheduled_frame(&mut self.cell.borrow_mut()) {
                return count;
            }
        }
    }

    /// A touch pointer with the primary button, as `tester.startGesture` makes.
    pub(crate) fn touch(id: i64) -> TestPointer {
        TestPointer {
            id,
            kind: waft_embedder::PointerDeviceKind::Touch,
            buttons: waft_gestures::K_PRIMARY_BUTTON,
            position: waft_embedder::Offset::ZERO,
        }
    }

    /// A mouse pointer pressing `buttons`, as `tester.createGesture(kind: mouse)` makes.
    pub(crate) fn mouse(id: i64, buttons: i64) -> TestPointer {
        TestPointer {
            id,
            kind: waft_embedder::PointerDeviceKind::Mouse,
            buttons,
            position: waft_embedder::Offset::ZERO,
        }
    }

    /// Sends one packet of `pointer` at its position, in logical coordinates, stamped
    /// `Duration.zero` as Dart's `TestPointer` stamps an event it is given no time stamp for.
    pub(crate) fn send(&mut self, pointer: &TestPointer, change: waft_embedder::PointerChange) {
        self.send_at(pointer, change, waft_embedder::Offset::ZERO, Duration::ZERO);
    }

    /// [`send`](Self::send) with the packet's `delta` from the pointer's last position, in
    /// logical coordinates, and stamped `time_stamp`, as Dart's `TestPointer` gives its events.
    pub(crate) fn send_at(
        &mut self,
        pointer: &TestPointer,
        change: waft_embedder::PointerChange,
        delta: waft_embedder::Offset,
        time_stamp: Duration,
    ) {
        use waft_embedder::{PointerChange, PointerData, PointerDataPacket};
        let buttons = match change {
            PointerChange::Down | PointerChange::Move => pointer.buttons,
            _ => 0,
        };
        let mut app = self.cell.borrow_mut();
        waft_gestures::GestureBinding::instance(&mut app).handle_pointer_data_packet(
            &mut app,
            PointerDataPacket::new(vec![PointerData {
                change,
                kind: pointer.kind,
                time_stamp,
                device: pointer.id,
                pointer_identifier: pointer.id,
                buttons,
                physical_x: pointer.position.dx() * 2.0,
                physical_y: pointer.position.dy() * 2.0,
                physical_delta_x: delta.dx() * 2.0,
                physical_delta_y: delta.dy() * 2.0,
                ..PointerData::default()
            }]),
        );
        app.drain_microtasks();
        drop(app);
        self.cell.checkpoint();
    }

    /// Dart's `startGesture(at)`: a pointer down at `at`.
    pub(crate) fn down(&mut self, pointer: &mut TestPointer, at: waft_embedder::Offset) {
        pointer.position = at;
        if pointer.kind == waft_embedder::PointerDeviceKind::Mouse {
            self.send(pointer, waft_embedder::PointerChange::Hover);
        }
        self.send(pointer, waft_embedder::PointerChange::Down);
    }

    /// Dart's `gesture.moveTo(at)`: a move while down, a hover while up, stamped `Duration.zero`.
    pub(crate) fn move_to(
        &mut self,
        pointer: &mut TestPointer,
        at: waft_embedder::Offset,
        down: bool,
    ) {
        let delta = at - pointer.position;
        pointer.position = at;
        let change = if down {
            waft_embedder::PointerChange::Move
        } else {
            waft_embedder::PointerChange::Hover
        };
        self.send_at(pointer, change, delta, Duration::ZERO);
    }

    /// Dart's `gesture.up()`.
    pub(crate) fn up(&mut self, pointer: &TestPointer) {
        self.send(pointer, waft_embedder::PointerChange::Up);
    }

    /// Dart's `gesture.cancel()`.
    pub(crate) fn cancel(&mut self, pointer: &TestPointer) {
        self.send(pointer, waft_embedder::PointerChange::Cancel);
    }

    /// Dart's `gesture.addPointer()` for a mouse, at `at`.
    pub(crate) fn add_pointer(&mut self, pointer: &mut TestPointer, at: waft_embedder::Offset) {
        pointer.position = at;
        self.send(pointer, waft_embedder::PointerChange::Add);
    }

    /// Dart's `tester.sendKeyEvent(key)`: a key down and a key up of `physical` and `logical`.
    pub(crate) fn send_key_event(
        &mut self,
        physical: waft_services::PhysicalKeyboardKey,
        logical: waft_services::LogicalKeyboardKey,
    ) {
        use waft_embedder::{KeyData, KeyEventDeviceType, KeyEventType};
        let mut app = self.cell.borrow_mut();
        for event_type in [KeyEventType::Down, KeyEventType::Up] {
            waft_services::KeyEventManager::instance(&mut app).handle_key_data(
                &mut app,
                KeyData {
                    time_stamp: Duration::ZERO,
                    event_type,
                    device_type: KeyEventDeviceType::Keyboard,
                    physical: physical.usb_hid_usage,
                    logical: logical.key_id,
                    character: None,
                    synthesized: false,
                },
            );
        }
        app.drain_microtasks();
        drop(app);
        self.cell.checkpoint();
    }

    /// Dart's `tester.tap(at)`: down and up with no frame between.
    pub(crate) fn tap(&mut self, at: waft_embedder::Offset, pointer: i64) {
        let mut touch = WidgetTester::touch(pointer);
        self.down(&mut touch, at);
        self.up(&touch);
    }

    /// Dart's `tester.flingFrom(start, offset, speed)`: a touch down at `start`, 51 moves to
    /// `start + offset` stamped from zero at `speed` logical pixels a second, a frame after
    /// every 16ms of stamps, and an up.
    pub(crate) fn fling_from(
        &mut self,
        start: waft_embedder::Offset,
        offset: waft_embedder::Offset,
        speed: f64,
        pointer: i64,
    ) {
        use waft_embedder::{Offset, PointerChange};
        debug_assert!(offset.distance() > 0.0);
        debug_assert!(speed > 0.0); // speed is pixels/second
        let micros = |time_stamp: f64| Duration::from_micros(time_stamp as u64);
        let mut touch = WidgetTester::touch(pointer);
        const K_MOVE_COUNT: u32 = 50; // Needs to be >= kHistorySize, see _LeastSquaresVelocityTrackerStrategy
        let frame_interval = Duration::from_millis(16).as_micros() as f64;
        let time_stamp_delta = 1000000.0 * offset.distance() / (K_MOVE_COUNT as f64 * speed);
        let mut time_stamp = 0.0_f64;
        let mut last_time_stamp = time_stamp;
        touch.position = start;
        self.send_at(
            &touch,
            PointerChange::Down,
            Offset::ZERO,
            micros(time_stamp.round()),
        );
        for i in 0..=K_MOVE_COUNT {
            let location = start
                + Offset::lerp(
                    Some(Offset::ZERO),
                    Some(offset),
                    f64::from(i) / f64::from(K_MOVE_COUNT),
                )
                .expect("both ends are given");
            let delta = location - touch.position;
            touch.position = location;
            self.send_at(
                &touch,
                PointerChange::Move,
                delta,
                micros(time_stamp.round()),
            );
            time_stamp += time_stamp_delta;
            if time_stamp - last_time_stamp > frame_interval {
                self.pump(micros((time_stamp - last_time_stamp).trunc()));
                last_time_stamp = time_stamp;
            }
        }
        self.send_at(
            &touch,
            PointerChange::Up,
            Offset::ZERO,
            micros(time_stamp.round()),
        );
    }

    /// Dart's `tester.dragFrom(start, offset)`: a touch down at `start`, moves that cross
    /// flutter_test's 20 pixel drag slop box in separate steps, and an up, with no frame
    /// between.
    pub(crate) fn drag_from(
        &mut self,
        start: waft_embedder::Offset,
        offset: waft_embedder::Offset,
        pointer: i64,
    ) {
        use waft_embedder::Offset;
        const K_DRAG_SLOP_DEFAULT: f64 = 20.0;
        let touch_slop_x = K_DRAG_SLOP_DEFAULT;
        let touch_slop_y = K_DRAG_SLOP_DEFAULT;
        let mut gesture = WidgetTester::touch(pointer);
        self.down(&mut gesture, start);
        let mut move_by = |tester: &mut WidgetTester, by: Offset| {
            let at = gesture.position + by;
            tester.move_to(&mut gesture, at, true);
        };

        let x_sign = dart_sign(offset.dx());
        let y_sign = dart_sign(offset.dy());

        let offset_x = offset.dx();
        let offset_y = offset.dy();

        let separate_x = offset.dx().abs() > touch_slop_x && touch_slop_x > 0.0;
        let separate_y = offset.dy().abs() > touch_slop_y && touch_slop_y > 0.0;

        if separate_y || separate_x {
            let offset_slope = offset_y / offset_x;
            let inverse_offset_slope = offset_x / offset_y;
            let slop_slope = touch_slop_y / touch_slop_x;
            let absolute_offset_slope = offset_slope.abs();
            let signed_slop_x = touch_slop_x * x_sign;
            let signed_slop_y = touch_slop_y * y_sign;
            if absolute_offset_slope != slop_slope {
                // The drag goes through one or both of the extents of the edges of the box.
                if absolute_offset_slope < slop_slope {
                    debug_assert!(offset_x.abs() > touch_slop_x);
                    // The drag goes through the vertical edge of the box.
                    // It is guaranteed that the |offsetX| > touchSlopX.
                    let diff_y = offset_slope.abs() * touch_slop_x * y_sign;

                    // The vector from the origin to the vertical edge.
                    move_by(self, Offset::new(signed_slop_x, diff_y));
                    if offset_y.abs() <= touch_slop_y {
                        // The drag ends on or before getting to the horizontal extension of the horizontal edge.
                        move_by(
                            self,
                            Offset::new(offset_x - signed_slop_x, offset_y - diff_y),
                        );
                    } else {
                        let diff_y2 = signed_slop_y - diff_y;
                        let diff_x2 = inverse_offset_slope * diff_y2;

                        // The vector from the edge of the box to the horizontal extension of the horizontal edge.
                        move_by(self, Offset::new(diff_x2, diff_y2));
                        move_by(
                            self,
                            Offset::new(
                                offset_x - diff_x2 - signed_slop_x,
                                offset_y - signed_slop_y,
                            ),
                        );
                    }
                } else {
                    debug_assert!(offset_y.abs() > touch_slop_y);
                    // The drag goes through the horizontal edge of the box.
                    // It is guaranteed that the |offsetY| > touchSlopY.
                    let diff_x = inverse_offset_slope.abs() * touch_slop_y * x_sign;

                    // The vector from the origin to the vertical edge.
                    move_by(self, Offset::new(diff_x, signed_slop_y));
                    if offset_x.abs() <= touch_slop_x {
                        // The drag ends on or before getting to the vertical extension of the vertical edge.
                        move_by(
                            self,
                            Offset::new(offset_x - diff_x, offset_y - signed_slop_y),
                        );
                    } else {
                        let diff_x2 = signed_slop_x - diff_x;
                        let diff_y2 = offset_slope * diff_x2;

                        // The vector from the edge of the box to the vertical extension of the vertical edge.
                        move_by(self, Offset::new(diff_x2, diff_y2));
                        move_by(
                            self,
                            Offset::new(
                                offset_x - signed_slop_x,
                                offset_y - diff_y2 - signed_slop_y,
                            ),
                        );
                    }
                }
            } else {
                // The drag goes through the corner of the box.
                move_by(self, Offset::new(signed_slop_x, signed_slop_y));
                move_by(
                    self,
                    Offset::new(offset_x - signed_slop_x, offset_y - signed_slop_y),
                );
            }
        } else {
            // The drag ends inside the box.
            move_by(self, offset);
        }
        self.up(&gesture);
    }

    /// Dart's `tester.longPressAt(at)`: down, a `kLongPressTimeout + kPressTimeout` wait, and up.
    pub(crate) fn long_press(&mut self, at: waft_embedder::Offset, pointer: i64) {
        let mut touch = WidgetTester::touch(pointer);
        self.down(&mut touch, at);
        self.pump(waft_gestures::K_LONG_PRESS_TIMEOUT + waft_gestures::K_PRESS_TIMEOUT);
        self.up(&touch);
    }

    /// The render object of `key`'s widget.
    pub(crate) fn render_object(
        &self,
        key: &waft_widgets::GlobalKey,
    ) -> waft_rendering::AnyRenderObject {
        let app = self.cell.borrow();
        key.current_context(&app)
            .expect("the keyed widget is mounted")
            .find_render_object(&app)
            .expect("the keyed widget has a render object")
    }

    /// Every render object of type `T` in the view, in paint order.
    pub(crate) fn all<T: waft_rendering::RenderObject>(
        &self,
    ) -> Vec<waft_rendering::RenderHandle<T>> {
        let mut app = self.cell.borrow_mut();
        let binding = waft_rendering::RendererBinding::instance(&mut app);
        let mut found = Vec::new();
        for render_view in binding.render_views(&app) {
            collect(&app, render_view.as_object(), &mut found);
        }
        found
    }

    /// The first render object of type `T` in the view.
    pub(crate) fn first<T: waft_rendering::RenderObject>(&self) -> waft_rendering::RenderHandle<T> {
        self.all::<T>()
            .into_iter()
            .next()
            .expect("a render object of that type")
    }

    /// What the ink features of the first material draw, painted as its `paint` paints them
    /// at the origin: Dart's `paints` matcher on `Material.of(context)`.
    pub(crate) fn ink_list(&self) -> waft_embedder::valo::DisplayList {
        let render = self.first::<crate::material::RenderInkFeatures>();
        let mut canvas = waft_embedder::Canvas::new();
        render.paint_ink_features(
            &mut self.cell.borrow_mut(),
            &mut canvas,
            waft_embedder::Offset::ZERO,
        );
        canvas.build()
    }

    /// The draws of [`ink_list`](Self::ink_list).
    pub(crate) fn ink_drawn(&self) -> Vec<Drawn> {
        Drawn::draws(&self.ink_list())
    }

    /// The centre of `key`'s render box, in the view's coordinates.
    pub(crate) fn center_of(&self, key: &waft_widgets::GlobalKey) -> waft_embedder::Offset {
        let render = self.render_object(key).as_box().expect("a render box");
        let app = self.cell.borrow();
        let size = render.size(&app);
        render.local_to_global(&app, size.center(waft_embedder::Offset::ZERO), None)
    }

    /// Installs the OS fonts, so a `Text` lays out at a size; the test platform has no Ahem,
    /// so glyph sizes are the OS font's, not Flutter's 14 pixel squares.
    pub(crate) fn install_fonts(&self) {
        let mut app = self.cell.borrow_mut();
        let binding = waft_painting::PaintingBinding::instance(&mut app);
        if !binding.has_fonts(&app) {
            binding.install_fonts(&mut app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
    }

    /// Installs the platform's fonts as the shell does at start-up: on a `TestPlatform`,
    /// flutter_test's, where every family is FlutterTest, so a `Text` measures as in Flutter's
    /// tests.
    pub(crate) fn install_platform_fonts(&self) {
        let mut app = self.cell.borrow_mut();
        let binding = waft_painting::PaintingBinding::instance(&mut app);
        if !binding.has_fonts(&app) {
            binding.install_platform_fonts(&mut app);
        }
    }

    /// Every mounted element, depth first with a parent before its children: the order in which
    /// Dart's finders report what they find, so `.first` and `.last` match theirs.
    pub(crate) fn elements(&self) -> Vec<waft_widgets::AnyElement> {
        let mut app = self.cell.borrow_mut();
        let root = waft_widgets::WidgetsBinding::instance(&mut app).root_element(&app);
        let mut all = Vec::new();
        let mut pending: Vec<waft_widgets::AnyElement> = root.into_iter().collect();
        while let Some(element) = pending.pop() {
            all.push(element);
            let mut children = Vec::new();
            element.visit_children(&app, &mut |child| children.push(child));
            pending.extend(children.into_iter().rev());
        }
        all
    }

    /// Dart's `find.byType(W)`: every element whose widget is a `W`.
    pub(crate) fn find_by_type<W: 'static>(&self) -> Vec<waft_widgets::AnyElement> {
        let found = self.elements();
        let app = self.cell.borrow();
        found
            .into_iter()
            .filter(|element| waft_widgets::downcast_widget::<W>(&**element.widget(&app)).is_some())
            .collect()
    }

    /// Dart's `find.text(text)`: every `Text` whose data, or its span's plain text, is `text`.
    pub(crate) fn find_text(&self, text: &str) -> Vec<waft_widgets::AnyElement> {
        let found = self.find_by_type::<waft_widgets::Text>();
        let app = self.cell.borrow();
        found
            .into_iter()
            .filter(|element| {
                let widget =
                    waft_widgets::downcast_widget::<waft_widgets::Text>(&**element.widget(&app))
                        .expect("found by type");
                match (&widget.data, &widget.text_span) {
                    (Some(data), _) => data == text,
                    (None, Some(span)) => span.to_plain_text(true, true) == text,
                    (None, None) => false,
                }
            })
            .collect()
    }

    /// Whether `element` is onstage: in an entry each `Overlay` above it keeps onstage, and
    /// under no `Offstage` that hides it. Dart's finders skip the rest (`skipOffstage: true`).
    pub(crate) fn is_onstage(&self, element: waft_widgets::AnyElement) -> bool {
        use waft_rendering::ContainerRenderObjectMixin as _;
        let app = self.cell.borrow();
        let Some(mut object) = element.find_render_object(&app) else {
            return false;
        };
        while let Some(parent) = object.parent(&app) {
            if let Some(theater) = parent.downcast::<waft_widgets::RenderTheater>(&app) {
                let mut child = theater.first_child(&app);
                for _ in 0..theater.skip_count(&app) {
                    child = child.and_then(|current| theater.child_after(&app, current));
                }
                let mut onstage = false;
                while let Some(current) = child {
                    if current.as_object() == object {
                        onstage = true;
                        break;
                    }
                    child = theater.child_after(&app, current);
                }
                if !onstage {
                    return false;
                }
            }
            if parent
                .downcast::<waft_rendering::RenderOffstage>(&app)
                .is_some_and(|offstage| offstage.offstage(&app))
            {
                return false;
            }
            // Dart's `debugVisitOnstageChildren` of Material's
            // `_TextSelectionToolbarItemsLayoutElement` and Cupertino's
            // `_CupertinoTextSelectionToolbarItemsElement`: a child not painted is offstage.
            // Those two and the trailing-edge align, whose element reports its child as is,
            // are what lays out children with `ToolbarItemsParentData`.
            if parent
                .downcast::<crate::text_selection_toolbar::TextSelectionToolbarTrailingEdgeAlignRenderBox>(&app)
                .is_none()
                && object
                    .parent_data(&app)
                    .and_then(|data| data.part::<waft_widgets::ToolbarItemsParentData>())
                    .is_some_and(|data| !data.should_paint)
            {
                return false;
            }
            // Dart's `SliverMultiBoxAdaptorElement.debugVisitOnstageChildren`: a lazy sliver's
            // child is onstage only while its layout offset falls in the sliver's painted
            // window, which a kept-alive child's does not.
            if let Some(sliver) = parent.as_sliver()
                && let Some(child_parent_data) = object
                    .parent_data(&app)
                    .and_then(|data| data.part::<waft_rendering::SliverMultiBoxAdaptorParentData>())
            {
                let constraints = sliver.constraints(&app);
                let paint_bounds = object.paint_bounds(&app);
                let item_extent = match constraints.axis() {
                    waft_painting::Axis::Horizontal => paint_bounds.width(),
                    waft_painting::Axis::Vertical => paint_bounds.height(),
                };
                let onstage = child_parent_data
                    .layout_offset()
                    .is_some_and(|layout_offset| {
                        layout_offset
                            < constraints.scroll_offset + constraints.remaining_paint_extent
                            && layout_offset + item_extent > constraints.scroll_offset
                    });
                if !onstage {
                    return false;
                }
            }
            object = parent;
        }
        true
    }

    /// Dart's `find.text(text)` with its default `skipOffstage`: [`find_text`](Self::find_text)
    /// without the offstage ones.
    pub(crate) fn find_onstage_text(&self, text: &str) -> Vec<waft_widgets::AnyElement> {
        self.find_text(text)
            .into_iter()
            .filter(|element| self.is_onstage(*element))
            .collect()
    }

    /// Dart's `find.byType(W)` with its default `skipOffstage`:
    /// [`find_by_type`](Self::find_by_type) without the offstage ones.
    pub(crate) fn find_onstage_by_type<W: 'static>(&self) -> Vec<waft_widgets::AnyElement> {
        self.find_by_type::<W>()
            .into_iter()
            .filter(|element| self.is_onstage(*element))
            .collect()
    }

    /// Dart's `find.byKey(key)` with its default `skipOffstage`: every onstage element whose
    /// widget has `key`.
    pub(crate) fn find_onstage_by_key(
        &self,
        key: &waft_widgets::KeyRef,
    ) -> Vec<waft_widgets::AnyElement> {
        let found = self.elements();
        let keyed: Vec<waft_widgets::AnyElement> = {
            let app = self.cell.borrow();
            found
                .into_iter()
                .filter(|element| element.widget(&app).key() == Some(key))
                .collect()
        };
        keyed
            .into_iter()
            .filter(|element| self.is_onstage(*element))
            .collect()
    }

    /// Dart's `find.text(text)` over [`AhemText`]s: every one showing `text`.
    pub(crate) fn find_ahem_text(&self, text: &str) -> Vec<waft_widgets::AnyElement> {
        let found = self.find_by_type::<AhemText>();
        let app = self.cell.borrow();
        found
            .into_iter()
            .filter(|element| {
                waft_widgets::downcast_widget::<AhemText>(&**element.widget(&app))
                    .is_some_and(|ahem| ahem.text == text)
            })
            .collect()
    }

    /// Reads the widget of `element` as a `W`: Dart's `tester.widget<W>(finder)`.
    pub(crate) fn widget<W: 'static, R>(
        &self,
        element: waft_widgets::AnyElement,
        read: impl FnOnce(&W) -> R,
    ) -> R {
        let app = self.cell.borrow();
        read(
            waft_widgets::downcast_widget::<W>(&**element.widget(&app))
                .expect("the element's widget is of that type"),
        )
    }

    /// The ancestors of `element`, nearest first: Dart's `find.ancestor(of:)`.
    pub(crate) fn ancestors(
        &self,
        element: waft_widgets::AnyElement,
    ) -> Vec<waft_widgets::AnyElement> {
        let app = self.cell.borrow();
        std::iter::successors(element.parent(&app), |parent| parent.parent(&app)).collect()
    }

    /// Dart's `find.descendant(of: of, matching: find.byType(W))`: every element under `of`
    /// whose widget is a `W`.
    pub(crate) fn descendants_of_type<W: 'static>(
        &self,
        of: waft_widgets::AnyElement,
    ) -> Vec<waft_widgets::AnyElement> {
        self.find_by_type::<W>()
            .into_iter()
            .filter(|element| self.ancestors(*element).contains(&of))
            .collect()
    }

    /// The nearest ancestor of `element` whose widget is a `W`.
    pub(crate) fn ancestor_of_type<W: 'static>(
        &self,
        element: waft_widgets::AnyElement,
    ) -> waft_widgets::AnyElement {
        let ancestors = self.ancestors(element);
        let app = self.cell.borrow();
        ancestors
            .into_iter()
            .find(|ancestor| waft_widgets::downcast_widget::<W>(&**ancestor.widget(&app)).is_some())
            .expect("an ancestor of that type")
    }

    /// The render box of `element`: Dart's `tester.renderObject(finder)`.
    pub(crate) fn render_box_of(
        &self,
        element: waft_widgets::AnyElement,
    ) -> waft_rendering::AnyRenderBox {
        let app = self.cell.borrow();
        element
            .find_render_object(&app)
            .and_then(|render_object| render_object.as_box())
            .expect("the element has a render box")
    }

    /// The global rectangle of `render`: Dart's `tester.getRect`.
    pub(crate) fn global_rect(&self, render: waft_rendering::AnyRenderBox) -> waft_embedder::Rect {
        let app = self.cell.borrow();
        let top_left = render.local_to_global(&app, waft_embedder::Offset::ZERO, None);
        top_left & render.size(&app)
    }

    /// Dart's `tester.getRect(finder)`: the rect between the global positions of `render`'s
    /// top left and bottom right corners, so a box painted scaled has its scaled size, where
    /// [`global_rect`](Self::global_rect) keeps the box's own size.
    pub(crate) fn get_rect(&self, render: waft_rendering::AnyRenderBox) -> waft_embedder::Rect {
        let app = self.cell.borrow();
        let size = render.size(&app);
        let top_left = render.local_to_global(&app, waft_embedder::Offset::ZERO, None);
        let bottom_right =
            render.local_to_global(&app, size.bottom_right(waft_embedder::Offset::ZERO), None);
        waft_embedder::Rect::from_points(top_left, bottom_right)
    }

    /// Dart's `tester.layerListOf(finder)`: the layer of the nearest render object at or above
    /// `element`'s that has one, and every layer under it, depth first with a parent before its
    /// children.
    pub(crate) fn layer_list_of(
        &self,
        element: waft_widgets::AnyElement,
    ) -> Vec<waft_rendering::AnyLayer> {
        let app = self.cell.borrow();
        let mut current = element
            .find_render_object(&app)
            .expect("the element has a render object");
        let layer = loop {
            if let Some(layer) = current.debug_layer(&app) {
                break layer;
            }
            current = current
                .parent(&app)
                .expect("a layer above the render object");
        };
        let mut layers = Vec::new();
        walk_layers(&app, layer, &mut layers);
        layers
    }

    /// The color of the first `Text` showing `text`, as its paragraph paints it: Dart's
    /// `tester.renderObject<RenderParagraph>(find.text(text)).text.style!.color!`.
    pub(crate) fn text_color(&self, text: &str) -> waft_embedder::Color {
        let element = *self
            .find_text(text)
            .first()
            .expect("a Text showing the text");
        let render = self.render_box_of(element);
        let app = self.cell.borrow();
        let paragraph = render
            .as_object()
            .downcast::<waft_rendering::RenderParagraph>(&app)
            .expect("a Text renders a paragraph");
        paragraph
            .text(&app)
            .style()
            .and_then(|style| style.color.as_deref().copied())
            .expect("the paragraph's text has a color")
    }

    /// The style of the first `Icon`'s glyph: Dart's `tester.widget<RichText>(find.descendant(
    /// of: find.byIcon(icon), matching: find.byType(RichText))).text.style!`.
    pub(crate) fn icon_style(&self) -> waft_painting::TextStyle {
        let icon = *self
            .find_by_type::<waft_widgets::Icon>()
            .first()
            .expect("an Icon");
        self.glyph_style(icon)
    }

    /// Dart's `find.byIcon(icon)`: the first `Icon` showing `icon`.
    pub(crate) fn find_by_icon(&self, icon: &waft_widgets::IconData) -> waft_widgets::AnyElement {
        self.find_by_type::<waft_widgets::Icon>()
            .into_iter()
            .find(|element| {
                self.widget(*element, |widget: &waft_widgets::Icon| {
                    widget.icon.as_ref() == Some(icon)
                })
            })
            .expect("an Icon showing the icon")
    }

    /// The style of the glyph of the `Icon` `icon` is: the `RichText` below it.
    pub(crate) fn glyph_style(&self, icon: waft_widgets::AnyElement) -> waft_painting::TextStyle {
        let rich_text = self
            .find_by_type::<waft_widgets::RichText>()
            .into_iter()
            .find(|rich_text| self.ancestors(*rich_text).contains(&icon))
            .expect("the Icon's RichText");
        self.widget(rich_text, |rich_text: &waft_widgets::RichText| {
            rich_text
                .text
                .style()
                .cloned()
                .expect("an Icon styles its glyph")
        })
    }

    /// The cursor the mouse tracker shows for `device`: Dart's
    /// `RendererBinding.instance.mouseTracker.debugDeviceActiveCursor(device)`.
    pub(crate) fn active_cursor(&self, device: i64) -> waft_services::MouseCursorRef {
        let mut app = self.cell.borrow_mut();
        let tracker = waft_rendering::RendererBinding::instance(&mut app).mouse_tracker(&app);
        tracker
            .debug_device_active_cursor(&app, device)
            .expect("the mouse is over the view")
    }

    /// Dart's `FocusManager.instance.highlightStrategy = strategy`.
    pub(crate) fn set_highlight_strategy(&self, strategy: waft_widgets::FocusHighlightStrategy) {
        let mut app = self.cell.borrow_mut();
        waft_widgets::FocusManager::instance(&mut app).set_highlight_strategy(&mut app, strategy);
    }

    /// What `render` and its subtree draw, painted at the origin: Dart's `paints` matcher.
    pub(crate) fn painted(&self, render: waft_rendering::AnyRenderObject) -> Vec<Drawn> {
        self.pictures(render)
            .iter()
            .flat_map(|picture| Drawn::draws(picture))
            .collect()
    }

    /// Every op `render` and its subtree record, painted at the origin, for a case that reads
    /// a path or a shadow: Dart's `paints` matcher with the calls it names (`drawCircle` is a
    /// path that `as_oval` answers, `drawShadow` an `Op::DrawShadow`).
    pub(crate) fn painted_ops(
        &self,
        render: waft_rendering::AnyRenderObject,
    ) -> Vec<waft_embedder::valo::Op> {
        self.pictures(render)
            .iter()
            .flat_map(|picture| waft_embedder::flattened_ops(picture))
            .collect()
    }

    /// Dart's `paints..drrect(color:, outer:, inner:)`: whether `render` or its subtree paints
    /// the ring between `outer` and `inner` in `color` that a `RoundedRectangleBorder` with a
    /// side paints.
    pub(crate) fn paints_drrect(
        &self,
        render: waft_rendering::AnyRenderObject,
        color: waft_embedder::Color,
        outer: waft_embedder::RRect,
        inner: waft_embedder::RRect,
    ) -> bool {
        use waft_embedder::valo::Op;
        let mut canvas = waft_embedder::Canvas::new();
        let paint = waft_embedder::Paint {
            color: color.into(),
            ..waft_embedder::Paint::default()
        };
        waft_painting::draw_drrect(&mut canvas, outer, inner, &paint);
        let ring = canvas.build();
        let Some(Op::DrawPath {
            path: ring_path,
            fill_rule: ring_fill_rule,
            paint: ring_paint,
            ..
        }) = ring.ops().first()
        else {
            panic!("draw_drrect records a path");
        };
        self.pictures(render).iter().any(|picture| {
            picture.ops().iter().any(|op| {
                matches!(
                    op,
                    Op::DrawPath { path, fill_rule, paint, .. }
                        if path == ring_path
                            && fill_rule == ring_fill_rule
                            && paint.color == ring_paint.color
                )
            })
        })
    }

    /// Dart's `paints..rrect(rrect:, color:)`: whether `render` or its subtree fills `rrect`,
    /// in `color` when it is given.
    pub(crate) fn paints_rrect(
        &self,
        render: waft_rendering::AnyRenderObject,
        rrect: waft_embedder::RRect,
        color: Option<waft_embedder::Color>,
    ) -> bool {
        use waft_embedder::valo::Op;
        let mut canvas = waft_embedder::Canvas::new();
        waft_painting::draw_rrect(&mut canvas, rrect, &waft_embedder::Paint::default());
        let expected = canvas.build();
        let Some(Op::DrawPath {
            path: expected_path,
            fill_rule: expected_fill_rule,
            ..
        }) = expected.ops().first()
        else {
            panic!("draw_rrect records a path");
        };
        let color: Option<waft_embedder::valo::Color> = color.map(Into::into);
        self.pictures(render).iter().any(|picture| {
            picture.ops().iter().any(|op| {
                matches!(
                    op,
                    Op::DrawPath { path, fill_rule, paint, .. }
                        if path == expected_path
                            && fill_rule == expected_fill_rule
                            && color.is_none_or(|color| paint.color == color)
                )
            })
        })
    }

    /// The pictures `render` and its subtree record, painted at the origin.
    pub(crate) fn pictures(
        &self,
        render: waft_rendering::AnyRenderObject,
    ) -> Vec<std::sync::Arc<waft_embedder::Picture>> {
        let mut app = self.cell.borrow_mut();
        assert!(
            !render.is_repaint_boundary(&app),
            "a repaint boundary would move its layer out of the scene"
        );
        use waft_rendering::ContainerLayer;
        let layer = waft_rendering::OffsetLayer::new(&mut app, waft_embedder::Offset::ZERO);
        let container = layer.as_container_layer();
        let bounds = render.paint_bounds(&app);
        let mut context = waft_rendering::PaintingContext::new(container, bounds);
        context.paint_child(&mut app, render, waft_embedder::Offset::ZERO);
        context.stop_recording_if_needed(&mut app);
        let mut pictures = Vec::new();
        let mut child = container.first_child(&app);
        while let Some(layer) = child {
            if let Some(picture) = app
                .handle::<waft_rendering::PictureLayer>(layer.id())
                .and_then(|picture| picture.picture(&app))
            {
                pictures.push(std::sync::Arc::clone(picture));
            }
            child = layer.next_sibling(&app);
        }
        pictures
    }
}

/// Adds `layer` and the layers under it to `layers`, depth first with a parent before its
/// children: flutter_test's `_walkLayers`.
fn walk_layers(
    app: &App,
    layer: waft_rendering::AnyContainerLayer,
    layers: &mut Vec<waft_rendering::AnyLayer>,
) {
    use waft_rendering::ErasedLayer;
    layers.push(layer.as_layer());
    let mut child = layer.first_child(app);
    while let Some(current) = child {
        match current.as_container_layer() {
            Some(container) => walk_layers(app, container, layers),
            None => layers.push(current),
        }
        child = current.next_sibling(app);
    }
}

/// Dart's `isSameColorAs` matcher, with its default threshold.
pub(crate) fn is_same_color_as(a: waft_embedder::Color, b: waft_embedder::Color) -> bool {
    const THRESHOLD: f64 = 0.004;
    (a.a - b.a).abs() <= THRESHOLD
        && (a.r - b.r).abs() <= THRESHOLD
        && (a.g - b.g).abs() <= THRESHOLD
        && (a.b - b.b).abs() <= THRESHOLD
}

/// Dart's `double.sign`: -1.0, 0.0 or 1.0, where Rust's `signum` answers 1.0 for zero.
fn dart_sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        value
    }
}

fn collect<T: waft_rendering::RenderObject>(
    app: &App,
    node: waft_rendering::AnyRenderObject,
    found: &mut Vec<waft_rendering::RenderHandle<T>>,
) {
    if let Some(typed) = node.downcast::<T>(app) {
        found.push(typed);
    }
    node.visit_children(app, &mut |child| collect(app, child, found));
}

/// One recorded canvas op, reduced to what Dart's `paints` matchers compare.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Drawn {
    /// `drawRect`: its colour as `0xAARRGGBB` and its rect.
    Rect(u32, waft_embedder::Rect),
    /// A path (`drawCircle`, `drawRRect`, `drawPath`): its colour and its local bounds.
    Path(u32, waft_embedder::Rect),
    /// `drawPaint`: its colour.
    Paint(u32),
    /// A clip.
    Clip,
    /// `save` / `restore` / a transform.
    State,
}

fn argb(color: waft_embedder::valo::Color) -> u32 {
    let channel = |value: f32| (value * 255.0).round() as u32;
    (channel(color.a) << 24) | (channel(color.r) << 16) | (channel(color.g) << 8) | channel(color.b)
}

fn rect_of(rect: waft_embedder::valo::Rect) -> waft_embedder::Rect {
    waft_embedder::Rect::from_ltwh(
        f64::from(rect.x),
        f64::from(rect.y),
        f64::from(rect.width),
        f64::from(rect.height),
    )
}

impl Drawn {
    /// The ops of `list`, reduced.
    pub(crate) fn all(list: &waft_embedder::valo::DisplayList) -> Vec<Drawn> {
        use waft_embedder::valo::Op;
        list.ops()
            .iter()
            .filter_map(|op| match op {
                Op::DrawRect { paint, rect, .. } => {
                    Some(Drawn::Rect(argb(paint.color), rect_of(*rect)))
                }
                Op::DrawPath {
                    paint,
                    content_bounds,
                    ..
                } => Some(Drawn::Path(argb(paint.color), rect_of(*content_bounds))),
                Op::DrawPaint { paint, .. } => Some(Drawn::Paint(argb(paint.color))),
                Op::ClipPath { .. } => Some(Drawn::Clip),
                Op::Save | Op::Restore | Op::Transform(_) => Some(Drawn::State),
                _ => None,
            })
            .collect()
    }

    /// The draws of `list`, without clips and canvas state.
    pub(crate) fn draws(list: &waft_embedder::valo::DisplayList) -> Vec<Drawn> {
        Drawn::all(list)
            .into_iter()
            .filter(|drawn| !matches!(drawn, Drawn::Clip | Drawn::State))
            .collect()
    }
}
