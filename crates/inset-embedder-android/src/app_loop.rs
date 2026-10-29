//! The app's loop, on the app's thread (`app_thread`): what the activities hand on, and where each
//! of it goes.
//!
//! One pass waits on the thread's looper, where the display's refresh arrives, then takes what the
//! activities' threads handed on, view by view, and serves what the framework is waiting for: the
//! intents that reached the app, a wake, and a frame of each view that asked, at the display's
//! refresh.
//!
//! The framework is never called from inside a looper callback. A refresh, a wake asked for from
//! another thread and an activity's event all only set something down and wake the looper, so every
//! call into the framework happens here, between polls, where nothing else is borrowed.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use android_activity::AndroidApp;
use inset_embedder::{
    EmbedderClient, Frame, PlatformRef, PointerDataPacket, ViewFocusDirection, ViewFocusEvent,
    ViewFocusState, ViewId,
};
use ndk::looper::ThreadLooper;

use crate::app_thread::{ActivityEvent, Done, Inbox};
use crate::gpu::Gpu;
use crate::platform::AndroidPlatform;
use crate::view::Geometry;
use crate::vsync::Vsync;

/// Refreshes in a row with nothing to draw before the signal is paused, so an idle app
/// makes no wakes.
const IDLE_TICKS: u8 = 2;

pub(crate) struct AppLoop<C> {
    looper: ThreadLooper,
    inbox: Inbox,
    gpu: Gpu,
    platform: Rc<AndroidPlatform>,
    client: Option<C>,
    start: Option<Box<dyn FnOnce(PlatformRef) -> C>>,
    /// The display's signal, while a view has a window.
    vsync: Option<Vsync>,
    /// Set by the refresh callback, read by the loop: the framework is never called from
    /// inside a looper callback.
    refreshed: Rc<Cell<bool>>,
    idle_ticks: u8,
}

impl<C: EmbedderClient> AppLoop<C> {
    /// The app, with the activity that started it as its first view. `start` runs once a view
    /// first has a window, and returns the client the loop drives.
    pub(crate) fn new(
        looper: ThreadLooper,
        inbox: Inbox,
        first: AndroidApp,
        view: ViewId,
        start: impl FnOnce(PlatformRef) -> C + 'static,
    ) -> AppLoop<C> {
        let platform = Rc::new(AndroidPlatform::new(looper.as_foreign().clone()));
        crate::intents::hear(looper.as_foreign().clone());
        let mut app_loop = AppLoop {
            looper,
            inbox,
            gpu: Gpu::acquire(),
            platform,
            client: None,
            start: Some(Box::new(start)),
            vsync: None,
            refreshed: Rc::new(Cell::new(false)),
            idle_ticks: 0,
        };
        app_loop.view_created(view, first);
        app_loop
    }

    /// Runs for the rest of the process.
    pub(crate) fn run(&mut self) {
        loop {
            self.wait();
            for (view, event) in crate::app_thread::take(&self.inbox) {
                self.on_activity_event(view, event);
            }
            self.serve_requests();
        }
    }

    /// Waits on the looper, where the display's refresh is called back, until it is woken or
    /// the framework's next time comes.
    fn wait(&self) {
        let waited = match self.timeout() {
            Some(timeout) => self.looper.poll_once_timeout(timeout),
            None => self.looper.poll_once(),
        };
        if waited.is_err() {
            crate::log::warn_to_log!("inset: the app's looper failed to wait");
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

    fn on_activity_event(&mut self, view: ViewId, event: ActivityEvent) {
        match event {
            ActivityEvent::Created(app) => self.view_created(view, app),
            ActivityEvent::WindowArrived => self.window_arrived(view),
            ActivityEvent::WindowGone(done) => self.window_gone(view, done),
            ActivityEvent::GeometryChanged => self.sync_metrics(view),
            // The configuration carries the appearance and the languages, and the screen's
            // size, orientation and density, and a change of it arrives as its own event rather
            // than through the window.
            ActivityEvent::ConfigChanged => {
                self.sync_configuration(view);
                self.sync_screen(view);
                self.sync_metrics(view);
            }
            ActivityEvent::RedrawNeeded => {
                if let Some(view) = self.platform.android_view(view) {
                    view.represent();
                }
            }
            ActivityEvent::FocusChanged(focused) => self.on_focus(view, focused),
            ActivityEvent::Pointer(packet) => self.on_pointer(packet),
            ActivityEvent::Back => self.on_back(view),
            ActivityEvent::Destroyed(done) => self.view_destroyed(view, done),
        }
    }

    /// An activity the system created: a view of the app's from now on. The first one's view
    /// starts the image loader on its device.
    fn view_created(&mut self, view: ViewId, app: AndroidApp) {
        crate::intents::read_launch_intent(&app);
        let android_view = self.platform.attach(view, app, &self.gpu);
        if self.platform.images.borrow().is_none() {
            *self.platform.images.borrow_mut() = Some(android_view.image_context());
            *self.platform.image_loader.borrow_mut() =
                crate::images::loader(android_view.image_context());
        }
        if let Some(client) = &mut self.client {
            client.view_added(view);
        }
    }

    /// The activity `view` is was destroyed: the view is gone from the app, which goes on.
    fn view_destroyed(&mut self, view: ViewId, done: Done) {
        self.platform.detach(view);
        self.stop_vsync_without_windows();
        if let Some(client) = &mut self.client {
            client.view_removed(view);
        }
        done.answer();
    }

    fn window_arrived(&mut self, view: ViewId) {
        let (Some(app), Some(android_view)) = (
            self.platform.activities.of(view),
            self.platform.android_view(view),
        ) else {
            return;
        };
        let Some(window) = app.native_window() else {
            return;
        };
        let size = [window.width() as u32, window.height() as u32];
        android_view.window_arrived(&self.gpu, &window, size);
        self.sync_configuration(view);
        self.sync_screen(view);
        android_view.refresh_metrics(self.geometry(&app));
        if self.client.is_none() {
            let start = self.start.take().expect("start runs once");
            self.client = Some(start(Rc::clone(&self.platform) as PlatformRef));
        } else {
            // Back from the background: the framework's last picture belongs to the new
            // window, and its metrics may have changed while the activity was away.
            if let Some(client) = &mut self.client {
                client.view_metrics_changed(view);
            }
            android_view.represent();
        }
        crate::refresh::follow_the_display(&app);
        if self.vsync.is_none() {
            self.start_vsync();
        }
    }

    /// The system took `view`'s window away: the view lets go of it before the activity's thread
    /// hears it may go on.
    fn window_gone(&mut self, view: ViewId, done: Done) {
        if let Some(view) = self.platform.android_view(view) {
            view.window_gone();
        }
        self.stop_vsync_without_windows();
        done.answer();
    }

    fn start_vsync(&mut self) {
        let refreshed = Rc::clone(&self.refreshed);
        self.vsync = Vsync::start(Box::new(move || refreshed.set(true)));
    }

    /// The display's signal goes when no view has a window to draw in.
    fn stop_vsync_without_windows(&mut self) {
        if !self.platform.any_view_has_a_native_window() {
            self.vsync = None;
        }
    }

    /// The window's geometry as the system now reports it for the activity `app`.
    fn geometry(&self, app: &AndroidApp) -> Geometry {
        let size = app
            .native_window()
            .map(|window| [window.width() as u32, window.height() as u32])
            .unwrap_or([0, 0]);
        let rect = app.content_rect();
        Geometry {
            size,
            density: density(app),
            content: [rect.left, rect.top, rect.right, rect.bottom],
            screen: self
                .platform
                .screen
                .borrow()
                .as_ref()
                .map(|screen| screen.id),
        }
    }

    fn sync_metrics(&mut self, view: ViewId) {
        let (Some(app), Some(android_view)) = (
            self.platform.activities.of(view),
            self.platform.android_view(view),
        ) else {
            return;
        };
        if let Some(window) = app.native_window() {
            android_view.resized([window.width() as u32, window.height() as u32]);
        }
        if android_view.refresh_metrics(self.geometry(&app))
            && let Some(client) = &mut self.client
        {
            client.view_metrics_changed(view);
        }
    }

    /// The screen `view`'s activity is on, read again: the app hears of it when it differs from
    /// what the app was told. The system says nothing of a change of the refresh rate alone, which
    /// only a display listener would hear.
    fn sync_screen(&mut self, view: ViewId) {
        let Some(app) = self.platform.activities.of(view) else {
            return;
        };
        let screen = crate::display::read_screen(&app, density(&app));
        if *self.platform.screen.borrow() == screen {
            return;
        }
        *self.platform.screen.borrow_mut() = screen;
        if let Some(client) = &mut self.client {
            client.screens_changed();
        }
    }

    /// The appearance and the languages the configuration of `view`'s activity carries.
    fn sync_configuration(&mut self, view: ViewId) {
        let Some(app) = self.platform.activities.of(view) else {
            return;
        };
        let config = app.config();
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

    /// `view`'s activity gained or lost the focus. One that gains it is the one the user turned
    /// to, and has the clipboard looked at, which only the focused app may read.
    fn on_focus(&mut self, view: ViewId, focused: bool) {
        if focused {
            self.platform.activities.bring_to_front(view);
        }
        if let Some(client) = &mut self.client {
            client.view_focus_changed(ViewFocusEvent {
                view_id: view,
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

    fn on_pointer(&mut self, packet: PointerDataPacket) {
        if let Some(client) = &mut self.client {
            client.pointer_data_packet(packet);
        }
    }

    /// The framework is asked to go back, and where it has nowhere left to go `view`'s activity
    /// finishes, which is what Android does for an activity that does not handle back itself.
    fn on_back(&mut self, view: ViewId) {
        let went_back = self
            .client
            .as_mut()
            .is_some_and(|client| client.pop_route());
        if !went_back && let Some(app) = self.platform.activities.of(view) {
            crate::task::finish(&app);
        }
    }

    /// What the framework is waiting for: what the intents that reached the app ask of it, the
    /// clipboard's change once the host cleared a copy of the app's whose time came, a wake, when
    /// one was asked for or its timer wakeup has arrived, and a frame at the display's refresh.
    fn serve_requests(&mut self) {
        self.hand_on_intents();
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
        if let Some(request) = crate::open_requests::request(arrived.viewed) {
            client.open_requested(request);
        }
        for activation in arrived.activations {
            client.notification_activated(activation);
        }
    }

    /// One refresh of the display: a frame of each view with a window that asked for one. The
    /// signal is paused after a few refreshes with nothing to draw, and started again by the next
    /// request.
    fn pace(&mut self) {
        let wanted = self.platform.frames_wanted();
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
            let views = self.platform.take_frame_requests();
            self.draw_frame(&views);
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

    /// Draws one frame of `views`: the framework builds, lays out and paints, and each view
    /// presents.
    fn draw_frame(&mut self, views: &[ViewId]) {
        let elapsed = self.platform.elapsed();
        if let Some(client) = &mut self.client {
            client.frame(views, Frame { elapsed });
        }
    }
}

/// Pixels per logical pixel, from the configuration's density.
fn density(app: &AndroidApp) -> f64 {
    app.config()
        .density()
        .map_or(1.0, |dpi| f64::from(dpi) / 160.0)
}
