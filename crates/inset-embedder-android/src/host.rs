//! The activity's loop: what the system tells the app, and where each of it goes.
//!
//! The glue owns `android_main`'s thread and its looper; this is the loop that runs on it.
//! One pass drains the system's main events, then the input the activity received, then
//! draws a frame if the display refreshed and the framework asked for one.
//!
//! The framework is never called from inside a looper callback. A refresh and a wake asked
//! for from another thread both only set something down and wake the looper, so every call
//! into the framework happens here, between polls, where nothing else is borrowed.

// Closing the activity is a C call on the pointer the glue owns; that is the `unsafe` here.
#![allow(unsafe_code)]

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use android_activity::input::{KeyAction, Keycode};
use android_activity::{AndroidApp, InputStatus, MainEvent, PollEvent};
use inset_embedder::{
    EmbedderClient, Frame, PlatformRef, ViewFocusDirection, ViewFocusEvent, ViewFocusState,
};

use crate::gpu::Gpu;
use crate::platform::AndroidPlatform;
use crate::view::{AndroidView, Geometry, VIEW};
use crate::vsync::Vsync;

/// Refreshes in a row with nothing to draw before the signal is paused, so an idle app
/// makes no wakes.
const IDLE_TICKS: u8 = 2;

/// Runs an Inset application on the activity the system started.
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
        let view = Rc::new(AndroidView::new(&gpu));
        let platform = Rc::new(AndroidPlatform::new(Rc::clone(&view), &self.app));
        *platform.images.borrow_mut() = Some(view.image_context());
        *platform.image_loader.borrow_mut() = crate::images::loader(view.image_context());
        let mut host = Host {
            app: self.app,
            gpu,
            view,
            platform,
            client: None,
            start: Some(Box::new(start)),
            vsync: None,
            refreshed: Rc::new(Cell::new(false)),
            idle_ticks: 0,
            destroyed: false,
            pointers: crate::input::Pointers::default(),
        };
        host.run();
    }
}

struct Host<C> {
    app: AndroidApp,
    gpu: Gpu,
    view: Rc<AndroidView>,
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
    /// The devices the framework has been told about.
    pointers: crate::input::Pointers,
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
            for event in events {
                self.on_main_event(event);
            }
            self.read_input();
            self.serve_requests();
        }
    }

    /// How long the loop may wait: no longer than the framework's earliest timer, and not
    /// at all while it has work of its own.
    fn timeout(&self) -> Option<Duration> {
        if self.refreshed.get() || self.platform.wakes.wake_now_requested() {
            return Some(Duration::ZERO);
        }
        self.platform
            .wakes
            .timer_wakeup()
            .map(|wakeup| wakeup.saturating_duration_since(Instant::now()))
    }

    fn on_main_event(&mut self, event: Recorded) {
        match event {
            Recorded::WindowArrived => self.window_arrived(),
            Recorded::WindowGone => {
                self.vsync = None;
                self.view.window_gone();
            }
            // The configuration carries the appearance and the languages, and a change of
            // it arrives as its own event rather than through the window.
            Recorded::ConfigChanged => {
                self.sync_configuration();
                self.sync_metrics();
            }
            Recorded::GeometryChanged => self.sync_metrics(),
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
        let rect = self.app.content_rect();
        Geometry {
            size,
            density: self
                .app
                .config()
                .density()
                .map_or(1.0, |dpi| f64::from(dpi) / 160.0),
            content: [rect.left, rect.top, rect.right, rect.bottom],
        }
    }

    fn sync_metrics(&mut self) {
        let geometry = self.geometry();
        if let Some(window) = self.app.native_window() {
            self.view
                .resized([window.width() as u32, window.height() as u32]);
        }
        if self.view.refresh_metrics(geometry)
            && let Some(client) = &mut self.client
        {
            client.view_metrics_changed(VIEW);
        }
    }

    /// The appearance and the languages the configuration carries.
    fn sync_configuration(&mut self) {
        let config = self.app.config();
        let brightness = match config.ui_mode_night() {
            android_activity::ndk::configuration::UiModeNight::Yes => {
                inset_embedder::Brightness::Dark
            }
            _ => inset_embedder::Brightness::Light,
        };
        if self.platform.brightness.replace(brightness) != brightness
            && let Some(client) = &mut self.client
        {
            client.platform_brightness_changed();
        }
        let locale = config.language().map(|language| {
            let locale = inset_embedder::Locale::new(language);
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
    }

    /// The input the activity received since the last pass.
    fn read_input(&mut self) {
        let app = self.app.clone();
        let Ok(mut events) = app.input_events_iter() else {
            return;
        };
        loop {
            let read = events.next(|event| {
                self.on_input(event);
                InputStatus::Handled
            });
            if !read {
                break;
            }
        }
    }

    fn on_input(&mut self, event: &android_activity::input::InputEvent<'_>) {
        use android_activity::input::InputEvent;
        match event {
            InputEvent::MotionEvent(motion) => self.on_motion(motion),
            InputEvent::KeyEvent(key) => self.on_key(key),
            // The rest of the keyboard follows.
            _ => {}
        }
    }

    fn on_motion(&mut self, motion: &android_activity::input::MotionEvent<'_>) {
        let packet = self.pointers.apply(motion, VIEW);
        if packet.data.is_empty() {
            return;
        }
        if let Some(client) = &mut self.client {
            client.pointer_data_packet(packet);
        }
    }

    /// The back button and the back gesture both arrive as this key. The framework is asked
    /// to go back, and where it has nowhere left to go the activity closes, which is what
    /// Android does for an app that does not handle back itself.
    ///
    /// It is read on release, as Android's own back is: a press that is held and then
    /// cancelled must not go back.
    fn on_key(&mut self, key: &android_activity::input::KeyEvent<'_>) {
        if key.key_code() != Keycode::Back || key.action() != KeyAction::Up {
            // The rest of the keyboard follows.
            return;
        }
        let went_back = self
            .client
            .as_mut()
            .is_some_and(|client| client.pop_route());
        if !went_back {
            self.close();
        }
    }

    /// Closes the activity, as Android closes an app whose back press nothing handled.
    fn close(&self) {
        // SAFETY: the activity this process was started with, as the glue hands it out. The
        // call only asks the system to finish it; the loop ends at the `Destroy` that
        // follows.
        unsafe {
            ndk_sys::ANativeActivity_finish(self.app.activity_as_ptr().cast());
        }
    }

    /// What the framework is waiting for: a wake, when one was asked for or its timer
    /// wakeup has arrived, and a frame at the display's refresh.
    fn serve_requests(&mut self) {
        if self.platform.wakes.should_wake(Instant::now())
            && let Some(client) = &mut self.client
        {
            let elapsed = self.platform.elapsed();
            client.wake(elapsed);
        }
        self.pace();
    }

    /// One refresh of the display: the frame the framework asked for, if it asked. The
    /// signal is paused after a few refreshes with nothing to draw, and started again by
    /// the next request.
    fn pace(&mut self) {
        let wanted = self.platform.frame_requested.get();
        if let Some(vsync) = &self.vsync
            && wanted
        {
            vsync.set_paused(false);
        }
        if !self.refreshed.replace(false) {
            return;
        }
        if wanted {
            self.idle_ticks = 0;
            self.platform.frame_requested.set(false);
            self.draw_frame();
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

    /// Draws one frame: the framework builds, lays out and paints, and the view presents.
    fn draw_frame(&mut self) {
        let elapsed = self.platform.elapsed();
        if let Some(client) = &mut self.client {
            client.frame(Frame { elapsed });
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
