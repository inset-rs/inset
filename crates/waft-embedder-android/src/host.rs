//! The activity's loop: what the system tells the app, and where each of it goes.
//!
//! The glue owns `android_main`'s thread and its looper; this is the loop that runs on it.
//! One pass takes the insets Android applied to the window, drains the system's main events,
//! then the input the activity received, then draws a frame if the display refreshed and the
//! framework asked for one.
//!
//! The framework is never called from inside a looper callback. A refresh and a wake asked
//! for from another thread both only set something down and wake the looper, so every call
//! into the framework happens here, between polls, where nothing else is borrowed.

// Closing the activity is a C call on the pointer the glue owns; that is the `unsafe` here.
#![allow(unsafe_code)]

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use android_activity::input::{KeyAction, Keycode};
use android_activity::{AndroidApp, InputStatus, MainEvent, PollEvent};
use waft_embedder::{
    EmbedderClient, Frame, PlatformRef, ViewFocusDirection, ViewFocusEvent, ViewFocusState,
};

use crate::composition::SurfaceControlCompositor;
use crate::gpu::Gpu;
use crate::open_requests::OpenRequests;
use crate::platform::AndroidPlatform;
use crate::text_input::{AndroidTextInput, TextInputEvent};
use crate::view::{AndroidView, Geometry, VIEW};
use crate::vsync::Vsync;
use crate::window_insets::WindowInsets;

/// Refreshes in a row with nothing to draw before the signal is paused, so an idle app
/// makes no wakes.
const IDLE_TICKS: u8 = 2;

/// Runs an Waft application on the activity the system started.
pub struct AndroidEmbedder {
    app: AndroidApp,
}

impl AndroidEmbedder {
    /// Takes the [`AndroidApp`] the glue handed `android_main`.
    pub fn new(app: AndroidApp) -> AndroidEmbedder {
        AndroidEmbedder { app }
    }

    /// Runs until the activity is destroyed. `start` runs once the system has given the
    /// activity a window, and returns the client the host drives.
    pub fn run<C: EmbedderClient + 'static>(self, start: impl FnOnce(PlatformRef) -> C + 'static) {
        let gpu = Gpu::acquire();
        let compositor = composition_on(&gpu);
        let text_input = AndroidTextInput::new(&self.app);
        let view = Rc::new(AndroidView::new(&gpu, compositor.as_deref(), text_input));
        let platform = Rc::new(AndroidPlatform::new(
            Rc::clone(&view),
            &self.app,
            &gpu,
            compositor,
        ));
        *platform.images.borrow_mut() = Some(view.image_context());
        *platform.image_loader.borrow_mut() = crate::images::loader(view.image_context());
        crate::intents::hear(&self.app);
        crate::window_insets::hear(&self.app);
        let open_requests = OpenRequests::new(self.app.clone());
        let mut host = Host {
            open_requests,
            app: self.app,
            gpu,
            view,
            window_insets: None,
            platform,
            client: None,
            start: Some(Box::new(start)),
            vsync: None,
            refreshed: Rc::new(Cell::new(false)),
            idle_ticks: 0,
            destroyed: false,
        };
        host.run();
    }
}

/// The compositor, where Waft's device imports the hardware buffers its layers show: none on
/// OpenGL ES, whose view draws through a swapchain (`picture.rs`).
fn composition_on(gpu: &Gpu) -> Option<Arc<SurfaceControlCompositor>> {
    if !gpu.imports_hardware_buffers() {
        return None;
    }
    SurfaceControlCompositor::new().map(Arc::new)
}

struct Host<C> {
    app: AndroidApp,
    gpu: Gpu,
    view: Rc<AndroidView>,
    /// What the system covers of the window, as the activity last told (`window_insets.rs`):
    /// none before it told any, and never any under an activity that is not Waft's.
    window_insets: Option<WindowInsets>,
    platform: Rc<AndroidPlatform>,
    client: Option<C>,
    start: Option<Box<dyn FnOnce(PlatformRef) -> C>>,
    /// The display's signal, while the activity has a window.
    vsync: Option<Vsync>,
    /// Set by the refresh callback, read by the loop: the framework is never called from
    /// inside a looper callback.
    refreshed: Rc<Cell<bool>>,
    idle_ticks: u8,
    destroyed: bool,
    /// Opens what the intents that reach the app ask to open (`open_requests.rs`).
    open_requests: OpenRequests,
}

impl<C: EmbedderClient> Host<C> {
    fn run(&mut self) {
        while !self.destroyed {
            let timeout = self.timeout();
            // The closure only records what happened; the work is after the poll.
            let app = self.app.clone();
            let mut events = Vec::new();
            app.poll_events(timeout, |event| {
                if let PollEvent::Main(main) = event {
                    events.push(Recorded::of(&main));
                }
            });
            // Before the main events, so that a window arriving in this pass is measured with
            // the insets applied to it.
            self.follow_window_insets();
            for event in events {
                self.on_main_event(event);
            }
            self.read_input();
            self.serve_requests();
        }
    }

    /// How long the loop may wait: no longer than the framework's earliest timer and the
    /// expiry of the app's copy on the clipboard, and not at all while it has work of its own.
    fn timeout(&self) -> Option<Duration> {
        if self.refreshed.get() || self.platform.wakes.wake_now_requested() {
            return Some(Duration::ZERO);
        }
        let timer = self.platform.wakes.timer_wakeup();
        let expiry = self.platform.clipboard.expiry();
        let due = match (timer, expiry) {
            (Some(timer), Some(expiry)) => Some(timer.min(expiry)),
            (timer, expiry) => timer.or(expiry),
        };
        due.map(|due| due.saturating_duration_since(Instant::now()))
    }

    fn on_main_event(&mut self, event: Recorded) {
        match event {
            Recorded::WindowArrived => self.window_arrived(),
            Recorded::WindowGone => {
                self.vsync = None;
                self.view.window_gone();
            }
            // The configuration carries the appearance and the languages, and the screen's
            // size, orientation and density, and a change of it arrives as its own event rather
            // than through the window.
            Recorded::ConfigChanged => {
                self.sync_configuration();
                self.sync_screen();
                self.sync_geometry();
            }
            Recorded::GeometryChanged => self.sync_geometry(),
            Recorded::RedrawNeeded => self.view.represent(),
            Recorded::FocusChanged(focused) => self.on_focus(focused),
            Recorded::Destroy => self.destroyed = true,
            Recorded::Ignored => {}
        }
    }

    fn window_arrived(&mut self) {
        let Some(window) = self.app.native_window() else {
            return;
        };
        let size = [window.width() as u32, window.height() as u32];
        self.view.window_arrived(&self.gpu, &window, size);
        self.sync_configuration();
        self.sync_screen();
        self.view.refresh_metrics(self.geometry());
        if self.client.is_none() {
            let start = self.start.take().expect("start runs once");
            self.client = Some(start(Rc::clone(&self.platform) as PlatformRef));
        } else {
            // Back from the background: the framework's last picture belongs to the new
            // window, and its metrics may have changed while the app was away.
            if let Some(client) = &mut self.client {
                client.view_metrics_changed(VIEW);
            }
            self.view.represent();
        }
        self.start_vsync(&window);
    }

    fn start_vsync(&mut self, window: &ndk::native_window::NativeWindow) {
        crate::refresh::follow_the_display(&self.app);
        let refreshed = Rc::clone(&self.refreshed);
        self.vsync = Vsync::start(window, Box::new(move || refreshed.set(true)));
    }

    /// The window's geometry as the system now reports it.
    fn geometry(&self) -> Geometry {
        let size = self
            .app
            .native_window()
            .map(|window| [window.width() as u32, window.height() as u32])
            .unwrap_or([0, 0]);
        Geometry {
            size,
            density: self.density(),
            insets: self.insets_of(size),
            screen: self
                .platform
                .screen
                .borrow()
                .as_ref()
                .map(|screen| screen.id),
        }
    }

    /// What the system covers of the window of `size`: what the activity told, and until it
    /// tells, or where it never does, what its content rect leaves.
    fn insets_of(&self, size: [u32; 2]) -> WindowInsets {
        self.window_insets.unwrap_or_else(|| {
            let rect = self.app.content_rect();
            WindowInsets::around_content(size, [rect.left, rect.top, rect.right, rect.bottom])
        })
    }

    /// Pixels per logical pixel, from the configuration's density.
    fn density(&self) -> f64 {
        self.app
            .config()
            .density()
            .map_or(1.0, |dpi| f64::from(dpi) / 160.0)
    }

    /// The window's size or its content rect changed: the picture follows the size, and the
    /// view's metrics both.
    fn sync_geometry(&mut self) {
        if let Some(window) = self.app.native_window() {
            self.view
                .resized([window.width() as u32, window.height() as u32]);
        }
        self.sync_metrics();
    }

    /// The view's metrics, read again: the framework hears of them when they changed.
    fn sync_metrics(&mut self) {
        if self.view.refresh_metrics(self.geometry())
            && let Some(client) = &mut self.client
        {
            client.view_metrics_changed(VIEW);
        }
    }

    /// Takes the insets Android applied to the window since the last pass, and has the view's
    /// metrics follow them.
    fn follow_window_insets(&mut self) {
        let Some(insets) = crate::window_insets::take() else {
            return;
        };
        self.window_insets = Some(insets);
        self.sync_metrics();
    }

    /// The screen the activity is on, read again: the app hears of it when it differs from what
    /// the app was told. The system says nothing of a change of the refresh rate alone, which only
    /// a display listener would hear.
    fn sync_screen(&mut self) {
        let screen = crate::display::read_screen(&self.app, self.density());
        if *self.platform.screen.borrow() == screen {
            return;
        }
        *self.platform.screen_id.lock().expect("screen lock") =
            screen.as_ref().map(|screen| screen.id);
        *self.platform.screen.borrow_mut() = screen;
        if let Some(client) = &mut self.client {
            client.screens_changed();
        }
    }

    /// The appearance and the languages the configuration carries.
    fn sync_configuration(&mut self) {
        let config = self.app.config();
        let brightness = match config.ui_mode_night() {
            android_activity::ndk::configuration::UiModeNight::Yes => {
                waft_embedder::Brightness::Dark
            }
            _ => waft_embedder::Brightness::Light,
        };
        if self.platform.brightness.replace(brightness) != brightness
            && let Some(client) = &mut self.client
        {
            client.platform_brightness_changed();
        }
        let locale = config.language().map(|language| {
            let locale = waft_embedder::Locale::new(language);
            match config.country() {
                Some(country) => locale.country_code(country),
                None => locale,
            }
        });
        let locales: Vec<_> = locale.into_iter().collect();
        if *self.platform.locales.borrow() != locales {
            *self.platform.locales.borrow_mut() = locales;
            if let Some(client) = &mut self.client {
                client.locales_changed();
            }
        }
    }

    /// The activity gained or lost the focus; one that gains it has the clipboard looked at, which
    /// only the focused app may read.
    fn on_focus(&mut self, focused: bool) {
        if let Some(client) = &mut self.client {
            client.view_focus_changed(ViewFocusEvent {
                view_id: VIEW,
                state: if focused {
                    ViewFocusState::Focused
                } else {
                    ViewFocusState::Unfocused
                },
                direction: ViewFocusDirection::Undefined,
            });
        }
        if focused {
            self.look_at_clipboard();
        }
    }

    /// The app hears the clipboard changed, if it did since the host last looked: Android tells
    /// only a listener Java code provides (`clipboard.rs`).
    fn look_at_clipboard(&mut self) {
        if self.platform.clipboard.has_changed()
            && let Some(client) = &mut self.client
        {
            client.clipboard_changed();
        }
    }

    /// The input the activity received since the last pass.
    fn read_input(&mut self) {
        let app = self.app.clone();
        let Ok(mut events) = app.input_events_iter() else {
            return;
        };
        loop {
            let read = events.next(|event| self.on_input(event));
            if !read {
                break;
            }
        }
    }

    /// Takes one event of the activity's input. A key left unhandled goes on through the window's
    /// views, to the text input's when a field takes text (`text_input.rs`), and to the system's
    /// own handling, such as the volume keys'.
    fn on_input(&mut self, event: &android_activity::input::InputEvent<'_>) -> InputStatus {
        use android_activity::input::InputEvent;
        match event {
            InputEvent::MotionEvent(motion) => {
                self.on_motion(motion);
                InputStatus::Handled
            }
            InputEvent::KeyEvent(key) => self.on_key(key),
            _ => InputStatus::Unhandled,
        }
    }

    fn on_motion(&mut self, motion: &android_activity::input::MotionEvent<'_>) {
        let reports = crate::input::reports_of(motion, VIEW);
        if reports.is_empty() {
            return;
        }
        if let Some(client) = &mut self.client {
            client.pointer_reports(reports);
        }
    }

    /// The back button and the back gesture both arrive as this key. The framework is asked
    /// to go back, and where it has nowhere left to go the activity closes, which is what
    /// Android does for an app that does not handle back itself.
    ///
    /// It is read on release, as Android's own back is: a press that is held and then
    /// cancelled must not go back. Every other key is left unhandled: the framework hears no key
    /// on Android yet.
    fn on_key(&mut self, key: &android_activity::input::KeyEvent<'_>) -> InputStatus {
        if key.key_code() != Keycode::Back {
            return InputStatus::Unhandled;
        }
        if key.action() == KeyAction::Up {
            self.go_back();
        }
        InputStatus::Handled
    }

    fn go_back(&mut self) {
        let went_back = self
            .client
            .as_mut()
            .is_some_and(|client| client.pop_route());
        if !went_back {
            self.close();
        }
    }

    /// Leaves a Back press nothing handled to Android: Waft's activity does the system's own with
    /// it, on the main thread, which since Android 12 moves a launcher's root activity to the
    /// background, the app running on, and finishes any other, the loop then ending at the
    /// `Destroy` that follows.
    fn close(&self) {
        let asked = crate::activity::with_activity(&self.app, |env, activity| {
            env.call_method(
                activity,
                jni::jni_str!("backLeftUnhandled"),
                jni::jni_sig!("()V"),
                &[],
            )
            .map(drop)
        });
        if let Err(error) = asked {
            crate::log::warn_to_log!("the activity was not handed the back press: {error}");
        }
    }

    /// What the framework is waiting for: what the intents that reached the app ask of it, what the
    /// keyboard typed, the clipboard's change once the host cleared a copy of the app's whose time
    /// came, a wake, when one was asked for or its timer wakeup has arrived, and a frame at the
    /// display's refresh.
    fn serve_requests(&mut self) {
        self.hand_on_intents();
        self.hand_on_typing();
        if self.platform.clipboard.clear_expired(Instant::now())
            && let Some(client) = &mut self.client
        {
            client.clipboard_changed();
        }
        if self.platform.wakes.should_wake(Instant::now())
            && let Some(client) = &mut self.client
        {
            let elapsed = self.platform.elapsed();
            client.wake(elapsed);
        }
        self.pace();
    }

    /// Hands the client what the intents that arrived ask: the files and links to open, and the
    /// taps on the app's notifications. They wait while there is no client yet.
    fn hand_on_intents(&mut self) {
        let Some(client) = &mut self.client else {
            return;
        };
        let arrived = crate::intents::take();
        if let Some(request) = self.open_requests.request(arrived.viewed) {
            client.open_requested(request);
        }
        for activation in arrived.activations {
            client.notification_activated(activation);
        }
    }

    /// Hands the client what the keyboard did to the field that takes text, in the order it did
    /// it.
    fn hand_on_typing(&mut self) {
        let typed = crate::text_input::take();
        let Some(client) = &mut self.client else {
            return;
        };
        for event in typed {
            match event {
                TextInputEvent::Edited(value) => client.text_input_editing_value(VIEW, value),
                TextInputEvent::Action(action) => client.text_input_action(VIEW, action),
            }
        }
    }

    /// One refresh of the display: the waits on it served, and the frame the framework asked for,
    /// if it asked. The signal is paused after a few refreshes with nothing to draw and nobody
    /// waiting, and started again by the next request or wait.
    fn pace(&mut self) {
        let wanted = self.platform.frame_requested.get();
        let waited = self.platform.vsync_waits.any();
        if let Some(vsync) = &self.vsync
            && (wanted || waited)
        {
            vsync.set_paused(false);
        }
        if !self.refreshed.replace(false) {
            return;
        }
        if waited {
            self.idle_ticks = 0;
            self.platform.vsync_waits.serve(self.refresh_period());
        }
        if wanted {
            self.idle_ticks = 0;
            self.platform.frame_requested.set(false);
            self.draw_frame();
            return;
        }
        if waited {
            return;
        }
        self.idle_ticks += 1;
        if self.idle_ticks >= IDLE_TICKS
            && let Some(vsync) = &self.vsync
        {
            self.idle_ticks = 0;
            vsync.set_paused(true);
        }
    }

    /// The time between two refreshes of the activity's screen, as the system reports its rate;
    /// a 60 Hz panel's where it reports none.
    fn refresh_period(&self) -> Duration {
        let rate = self
            .platform
            .screen
            .borrow()
            .as_ref()
            .and_then(|screen| screen.refresh_rate)
            .filter(|rate| *rate > 0.0)
            .unwrap_or(60.0);
        Duration::from_secs_f64(1.0 / rate)
    }

    /// Draws one frame: the framework builds, lays out and paints, and the view presents.
    fn draw_frame(&mut self) {
        let elapsed = self.platform.elapsed();
        if let Some(client) = &mut self.client {
            client.frame(&[VIEW], Frame { elapsed });
        }
    }
}

/// What a main event means to this host, recorded inside the poll so that nothing calls
/// the framework while the glue holds its own locks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Recorded {
    WindowArrived,
    WindowGone,
    GeometryChanged,
    ConfigChanged,
    RedrawNeeded,
    FocusChanged(bool),
    Destroy,
    Ignored,
}

impl Recorded {
    fn of(event: &MainEvent<'_>) -> Recorded {
        match event {
            MainEvent::InitWindow { .. } => Recorded::WindowArrived,
            MainEvent::TerminateWindow { .. } => Recorded::WindowGone,
            MainEvent::WindowResized { .. } | MainEvent::ContentRectChanged { .. } => {
                Recorded::GeometryChanged
            }
            MainEvent::InsetsChanged { .. } => Recorded::GeometryChanged,
            MainEvent::ConfigChanged { .. } => Recorded::ConfigChanged,
            MainEvent::RedrawNeeded { .. } => Recorded::RedrawNeeded,
            MainEvent::GainedFocus => Recorded::FocusChanged(true),
            MainEvent::LostFocus => Recorded::FocusChanged(false),
            MainEvent::Destroy => Recorded::Destroy,
            _ => Recorded::Ignored,
        }
    }
}
