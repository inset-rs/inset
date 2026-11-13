//! The app's loop: what the system tells the app, and where each of it goes.
//!
//! The loop runs on the app's thread (`app_thread.rs`) for the rest of the process, waiting on the
//! thread's looper. One pass takes the insets Android applied to the views' windows, then what
//! Android told the activities (`activity_events.rs`), then the input the views received, and
//! then serves what the framework waits for, a frame of the views that asked for one among it, if
//! the display refreshed.
//!
//! Everything reaches the loop about a view, and a later editor must keep it so: what an activity
//! was told is about the view the activity is, input about the view whose queue it arrived in,
//! what a keyboard typed and the insets Android applied carry a view's id, and a frame is drawn for
//! the views that asked; each is looked up in the registry (`views.rs`), never taken to be "the"
//! activity's. A view is made with the first activity Android makes for it, and the activity is
//! attached to it until Android destroys the activity. An activity destroyed finishing removes its
//! view; one destroyed to be made again leaves it, and the one Android makes in its place, which
//! the glue gives the same view (`saved_view.rs`), is attached to it, so the app runs on.
//!
//! At a refresh the input waiting is read before the frame is decided and drawn, as Android's
//! `Choreographer.doFrame` runs its input callbacks before its animation and traversal ones. The
//! input queues and the display's signal are both on the app's looper, which fixes no order
//! between them within a poll, and a touch read after the frame it arrived for shows a refresh
//! late.
//!
//! The framework is never called from inside a looper callback. A refresh and a wake asked
//! for from another thread both only set something down and wake the looper, so every call
//! into the framework happens here, between polls, where nothing else is borrowed.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ndk::configuration::{Configuration, UiModeNight};
use ndk::event::{InputEvent, KeyAction, KeyEvent, Keycode, MotionEvent};
use ndk::looper::ThreadLooper;
use ndk::native_window::NativeWindow;
use waft_embedder::{
    EmbedderClient, Frame, FrameVsync, PlatformRef, View, ViewFocusDirection, ViewFocusEvent,
    ViewFocusState, ViewId,
};

use crate::activity::Activity;
use crate::activity_events::{ActivityEvent, Told};
use crate::app_thread::AndroidApp;
use crate::application::Application;
use crate::composition::SurfaceControlCompositor;
use crate::gpu::Gpu;
use crate::input_queues::InputQueues;
use crate::log::warn_to_log;
use crate::open_requests::OpenRequests;
use crate::platform::AndroidPlatform;
use crate::text_input::TextInputEvent;
use crate::view::{AndroidView, ViewParts};
use crate::views::Views;
use crate::vsync::Vsync;

/// Refreshes in a row with nothing to draw before the signal is paused, so an idle app
/// makes no wakes.
const IDLE_TICKS: u8 = 2;

/// The longest the looper is asked to wait for, which it takes in milliseconds as an `i32`.
const LONGEST_WAIT: Duration = Duration::from_millis(i32::MAX as u64);

/// Runs a Waft application on the activities Android makes for it.
pub struct AndroidEmbedder {
    app: AndroidApp,
}

impl AndroidEmbedder {
    /// Takes the [`AndroidApp`] the app's entry, `android_main`, was handed.
    pub fn new(app: AndroidApp) -> AndroidEmbedder {
        AndroidEmbedder { app }
    }

    /// Runs for the rest of the process. `start` runs once the system has given a view a window,
    /// and returns the client the host drives.
    ///
    /// # Panics
    ///
    /// Off the app's thread, which `android_main` is called on.
    pub fn run<C: EmbedderClient + 'static>(
        self,
        start: impl FnOnce(PlatformRef) -> C + 'static,
    ) -> ! {
        let looper = ThreadLooper::for_thread()
            .expect("AndroidEmbedder::run on the app's thread, which android_main is called on");
        let (first, told) = first_activity(&looper);
        let application = self.app.application().clone();
        let gpu = Gpu::acquire();
        let compositor = composition_on(&gpu);
        let views = Rc::new(Views::default());
        let platform = Rc::new(AndroidPlatform::new(
            Rc::clone(&views),
            &application,
            &first,
            crate::loop_waker::app_loop(),
            &gpu,
            compositor.clone(),
        ));
        crate::intents::hear(&application);
        let mut host = Host {
            open_requests: OpenRequests::new(application.clone()),
            looper,
            application,
            gpu,
            compositor,
            views,
            input_queues: InputQueues::default(),
            platform,
            client: None,
            start: Some(Box::new(start)),
            vsync: None,
            refreshed: Rc::new(Cell::new(None)),
            idle_ticks: 0,
        };
        for told in told {
            host.on_told(told);
        }
        host.run()
    }
}

/// Waits until Android has made the app's first activity, which the platform is made with, and
/// answers it with everything told so far, its creation among it.
fn first_activity(looper: &ThreadLooper) -> (Activity, Vec<Told>) {
    let mut told = Vec::new();
    loop {
        told.extend(crate::activity_events::take());
        let first = told.iter().find_map(|told| match &told.event {
            ActivityEvent::Created { activity, .. } => Some(activity.clone()),
            _ => None,
        });
        if let Some(first) = first {
            return (first, told);
        }
        if looper.poll_once().is_err() {
            warn_to_log!("waft: the app's looper failed while waiting for its first activity");
        }
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
    /// The app's thread's looper, which the loop waits on.
    looper: ThreadLooper,
    application: Application,
    gpu: Gpu,
    /// What the views' pictures are composited by, where Waft's device imports hardware buffers.
    compositor: Option<Arc<SurfaceControlCompositor>>,
    /// Every view of the app's.
    views: Rc<Views>,
    /// The queue each view's input arrives in.
    input_queues: InputQueues,
    platform: Rc<AndroidPlatform>,
    client: Option<C>,
    start: Option<Box<dyn FnOnce(PlatformRef) -> C>>,
    /// The display's signal, while a view has a window.
    vsync: Option<Vsync>,
    /// When the vsync of the refresh not yet served occurred: set by the refresh callback, read
    /// by the loop, as the framework is never called from inside a looper callback.
    refreshed: Rc<Cell<Option<Instant>>>,
    idle_ticks: u8,
    /// Opens what the intents that reach the app ask to open (`open_requests.rs`).
    open_requests: OpenRequests,
}

impl<C: EmbedderClient> Host<C> {
    fn run(&mut self) -> ! {
        loop {
            self.wait();
            // Before what the activities were told, so that a window arriving in this pass is
            // measured with the insets applied to it.
            self.follow_window_insets();
            for told in crate::activity_events::take() {
                self.on_told(told);
            }
            self.read_input();
            self.serve_requests();
        }
    }

    /// Waits on the looper until something wakes it, or for as long as [`Host::timeout`] says.
    fn wait(&self) {
        let polled = match self.timeout() {
            Some(timeout) => self.looper.poll_once_timeout(timeout.min(LONGEST_WAIT)),
            None => self.looper.poll_once(),
        };
        if polled.is_err() {
            warn_to_log!("waft: the app's looper failed");
        }
    }

    /// How long the loop may wait: no longer than the framework's earliest timer and the
    /// expiry of the app's copy on the clipboard, and not at all while it has work of its own.
    fn timeout(&self) -> Option<Duration> {
        if self.refreshed.get().is_some() || self.platform.wakes.wake_now_requested() {
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

    /// One thing Android told an activity, about the view the activity is.
    fn on_told(&mut self, Told { view, event }: Told) {
        match event {
            ActivityEvent::Created {
                activity,
                configuration,
                restored,
            } => self.activity_created(view, &activity, configuration, restored),
            ActivityEvent::WindowCreated(window) => self.window_arrived(view, window),
            ActivityEvent::WindowDestroyed(done) => {
                self.window_gone(view);
                done.give();
            }
            ActivityEvent::WindowResized => self.sync_geometry(view),
            ActivityEvent::WindowRedrawNeeded => {
                if let Some(view) = self.views.get(view) {
                    view.represent();
                }
            }
            ActivityEvent::ContentRectChanged(rect) => {
                if let Some(known) = self.views.get(view) {
                    known.set_content_rect(rect);
                }
                self.sync_geometry(view);
            }
            // The configuration carries the appearance and the languages, and the screen's
            // size, orientation and density, and a change of it arrives as its own event rather
            // than through the window.
            ActivityEvent::ConfigurationChanged(configuration) => {
                self.configuration_changed(view, configuration)
            }
            ActivityEvent::FocusChanged(focused) => self.on_focus(view, focused),
            ActivityEvent::InputQueueCreated(queue) => {
                self.input_queues
                    .attach(view, queue, self.looper.as_foreign());
            }
            ActivityEvent::InputQueueDestroyed(done) => {
                self.input_queues.detach(view);
                done.give();
            }
            ActivityEvent::Destroyed { finishing } => self.activity_destroyed(view, finishing),
        }
    }

    /// Android made `activity` for the view numbered `id`, with `configuration`: the activity is
    /// attached to the view, which is made if it is new. The launcher's intent is
    /// read for what it asks, unless the activity was `restored`, whose intent was read when it
    /// first came; a document activity's intent is the view's own (`view_requests.rs`).
    fn activity_created(
        &mut self,
        id: ViewId,
        activity: &Activity,
        configuration: Configuration,
        restored: bool,
    ) {
        let view = match self.views.get(id) {
            Some(view) => view,
            None => self.add_view(id, activity, configuration.clone()),
        };
        view.attach(activity, configuration);
        crate::window_insets::hear(&self.application, activity, id);
        if !restored && !activity.is_document() {
            crate::intents::read_launch_intent(activity);
        }
    }

    /// The view numbered `id`, which `activity` arrived as, one of the app's from now on, without
    /// a window yet: what its activity's keyboard types and the insets Android applies to its
    /// window are told as that view's. The client hears of it, once there is one.
    fn add_view(
        &mut self,
        id: ViewId,
        activity: &Activity,
        configuration: Configuration,
    ) -> Rc<AndroidView> {
        let view = AndroidView::new(
            id,
            ViewParts {
                arrival: self.platform.view_requests.arrival(activity),
                configuration,
                application: &self.application,
                app_description: self.platform.app_description.clone(),
                gpu: &self.gpu,
                compositor: self.compositor.as_deref(),
            },
        );
        self.views.add(Rc::clone(&view));
        if let Some(client) = &mut self.client {
            client.view_added(id);
        }
        view
    }

    /// Android destroyed the activity of the view numbered `id`: the view goes with it where the
    /// activity is `finishing`, and otherwise stays, without a window, for the activity Android
    /// makes in its place, as Flutter's engine stays for a configuration change
    /// (`detachFromActivityForConfigChanges`).
    fn activity_destroyed(&mut self, id: ViewId, finishing: bool) {
        let Some(view) = self.views.get(id) else {
            return;
        };
        if let Some(activity) = view.activity() {
            view.detach(&activity);
        }
        if finishing {
            self.remove_view(id);
        }
    }

    /// The view numbered `id` is the app's no more, and the client hears so.
    fn remove_view(&mut self, id: ViewId) {
        self.views.remove(id);
        self.input_queues.detach(id);
        if let Some(client) = &mut self.client {
            client.view_removed(id);
        }
    }

    /// The system gave the activity of the view numbered `id` its window: the first window of the
    /// app's starts the client, and the view's task is no longer minimized.
    fn window_arrived(&mut self, id: ViewId, window: NativeWindow) {
        let Some(view) = self.views.get(id) else {
            return;
        };
        view.window_arrived(&self.gpu, window.clone());
        self.sync_configuration(&view);
        self.sync_screen(&view);
        view.refresh_metrics(view.density(), self.screen_id());
        if self.client.is_none() {
            let start = self.start.take().expect("start runs once");
            self.client = Some(start(Rc::clone(&self.platform) as PlatformRef));
        } else {
            // Back from the background: the framework's last picture belongs to the new
            // window, and its metrics may have changed while the app was away.
            if let Some(client) = &mut self.client {
                client.view_metrics_changed(id);
                client.window_changed(id);
            }
            view.represent();
        }
        if let Some(activity) = view.activity() {
            crate::refresh::follow_the_display(&activity);
        }
        self.start_vsync(&window);
    }

    /// The system took the window of the view numbered `id` away, which minimizes its task; the
    /// display's signal stops with the last window.
    fn window_gone(&mut self, id: ViewId) {
        if let Some(view) = self.views.get(id) {
            view.window_gone();
            if let Some(client) = &mut self.client {
                client.window_changed(id);
            }
        }
        if !self.views.any_has_window() {
            self.vsync = None;
        }
    }

    /// Starts the display's signal, unless a view's window already started it.
    fn start_vsync(&mut self, window: &NativeWindow) {
        if self.vsync.is_some() {
            return;
        }
        let refreshed = Rc::clone(&self.refreshed);
        self.vsync = Vsync::start(window, Box::new(move |vsync| refreshed.set(Some(vsync))));
    }

    /// The screen the app was last told it is on, once the system has described it.
    fn screen_id(&self) -> Option<waft_embedder::ScreenId> {
        self.platform
            .screen
            .borrow()
            .as_ref()
            .map(|screen| screen.id)
    }

    /// The configuration of the activity of the view numbered `id` changed to `configuration`.
    fn configuration_changed(&mut self, id: ViewId, configuration: Configuration) {
        let Some(view) = self.views.get(id) else {
            return;
        };
        view.set_configuration(configuration);
        self.sync_configuration(&view);
        self.sync_screen(&view);
        self.sync_geometry(id);
    }

    /// The size of the window of the view numbered `id` or its content rect changed: the picture
    /// follows the size, and the view's metrics both.
    fn sync_geometry(&mut self, id: ViewId) {
        let Some(view) = self.views.get(id) else {
            return;
        };
        view.resized();
        self.sync_metrics(&view);
    }

    /// `view`'s metrics, read again: the framework hears of them when they changed.
    fn sync_metrics(&mut self, view: &AndroidView) {
        if view.refresh_metrics(view.density(), self.screen_id())
            && let Some(client) = &mut self.client
        {
            client.view_metrics_changed(view.id());
        }
    }

    /// Takes the insets Android applied to the views' windows since the last pass, and has each
    /// view's metrics follow its own.
    fn follow_window_insets(&mut self) {
        for (id, insets) in crate::window_insets::take() {
            let Some(view) = self.views.get(id) else {
                continue;
            };
            view.set_window_insets(insets);
            self.sync_metrics(&view);
        }
    }

    /// The screen `view`'s activity is on, read again: the app hears of it when it differs from
    /// what the app was told. The system says nothing of a change of the refresh rate alone, which
    /// only a display listener would hear.
    fn sync_screen(&mut self, view: &AndroidView) {
        let Some(activity) = view.activity() else {
            return;
        };
        let screen = crate::display::read_screen(&activity, view.density());
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

    /// The appearance and the languages `view`'s configuration carries.
    fn sync_configuration(&mut self, view: &AndroidView) {
        let config = view.configuration();
        let brightness = match config.ui_mode_night() {
            UiModeNight::Yes => waft_embedder::Brightness::Dark,
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

    /// The window of the view numbered `id` gained or lost the focus; one that gains it has the
    /// clipboard looked at, which only the focused app may read, and the app's tasks, which the
    /// user may have swiped away meanwhile.
    fn on_focus(&mut self, id: ViewId, focused: bool) {
        if let Some(view) = self.views.get(id) {
            view.set_focused(focused);
        }
        if let Some(client) = &mut self.client {
            client.view_focus_changed(ViewFocusEvent {
                view_id: id,
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
            self.look_at_tasks();
        }
    }

    /// Removes each view whose activity Android destroyed to make again and whose task is gone
    /// since: the user swiped its card away, which no activity is there to hear. Android keeps
    /// the task of a view whose activity it will make again, so a task missing from the app's is
    /// one that will not come back.
    fn look_at_tasks(&mut self) {
        let kept: Vec<_> = self
            .views
            .all()
            .into_iter()
            .filter(|view| view.activity().is_none())
            .collect();
        if kept.is_empty() {
            return;
        }
        let Some(tasks) = crate::task_window::task_ids(&self.application) else {
            return;
        };
        for view in kept {
            if view.task().task_id().is_some_and(|id| !tasks.contains(&id)) {
                self.remove_view(view.id());
            }
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

    /// The input the views received since the last read.
    fn read_input(&mut self) {
        // The queues are the loop's, and nothing the framework is handed an event in reaches them.
        let queues = std::mem::take(&mut self.input_queues);
        queues.read(|view, event| self.on_input(view, event));
        self.input_queues = queues;
    }

    /// Takes one event of the input of the view numbered `view`, and answers whether it was
    /// handled. A key left unhandled goes on through the window's views, to the text input's when
    /// a field takes text (`text_input.rs`), and to the system's own handling, such as the volume
    /// keys'.
    fn on_input(&mut self, view: ViewId, event: &InputEvent) -> bool {
        match event {
            InputEvent::MotionEvent(motion) => {
                self.on_motion(view, motion);
                true
            }
            InputEvent::KeyEvent(key) => self.on_key(view, key),
            _ => false,
        }
    }

    fn on_motion(&mut self, view: ViewId, motion: &MotionEvent) {
        let reports = crate::input::reports_of(motion, view);
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
    fn on_key(&mut self, view: ViewId, key: &KeyEvent) -> bool {
        if key.key_code() != Keycode::Back {
            return false;
        }
        if key.action() == KeyAction::Up {
            self.go_back(view);
        }
        true
    }

    fn go_back(&mut self, view: ViewId) {
        let went_back = self
            .client
            .as_mut()
            .is_some_and(|client| client.pop_route(view));
        if !went_back {
            self.close(view);
        }
    }

    /// Leaves a Back press nothing handled in the view numbered `id` to Android: Waft's activity
    /// does the system's own with it, on the main thread, `onBackPressed`. Android 12's release
    /// notes say a launcher's root activity is moved to the background; on the emulator's API 37
    /// it was finished, started from the launcher's intent or not. The app runs on either way, and
    /// an activity finished takes its view with it (`activity_destroyed`).
    fn close(&self, id: ViewId) {
        let Some(activity) = self.views.get(id).and_then(|view| view.activity()) else {
            return;
        };
        let asked = activity.with(|env, activity| {
            env.call_method(
                activity,
                jni::jni_str!("backLeftUnhandled"),
                jni::jni_sig!("()V"),
                &[],
            )
            .map(drop)
        });
        if let Err(error) = asked {
            warn_to_log!("the activity was not handed the back press: {error}");
        }
    }

    /// What the framework is waiting for: what the intents that reached the app ask of it, what the
    /// keyboard typed, the clipboard's change once the host cleared a copy of the app's whose time
    /// came, a wake, when one was asked for or its timer wakeup has arrived, and a frame at the
    /// display's refresh.
    fn serve_requests(&mut self) {
        self.hand_on_failed_requests();
        self.remove_views_of_removed_tasks();
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

    /// Tells the client of the requests for views that got none.
    fn hand_on_failed_requests(&mut self) {
        let Some(client) = &mut self.client else {
            return;
        };
        for (request, error) in self.platform.view_requests.take_failed() {
            client.view_request_failed(request, error);
        }
    }

    /// Removes the views whose tasks the app removed while Android had destroyed their activities
    /// (`TaskWindow::close`).
    fn remove_views_of_removed_tasks(&mut self) {
        for view in self.views.all() {
            if view.is_task_removed() {
                self.remove_view(view.id());
            }
        }
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

    /// Hands the client what the keyboards did to the fields that take text, each in its view, in
    /// the order they did it; what was typed into a view that is the app's no more is dropped.
    fn hand_on_typing(&mut self) {
        let typed = crate::text_input::take();
        let Some(client) = &mut self.client else {
            return;
        };
        for (view, event) in typed {
            if self.views.get(view).is_none() {
                continue;
            }
            match event {
                TextInputEvent::Edited(value) => client.text_input_editing_value(view, value),
                TextInputEvent::Action(action) => client.text_input_action(view, action),
            }
        }
    }

    /// The display's refresh, if it came, and its signal kept going while a view waits for a frame
    /// or a producer waits on a refresh; the signal is started again by the next request or wait
    /// after a refresh paused it.
    fn pace(&mut self) {
        if let Some(vsync) = self.refreshed.take() {
            self.refresh(vsync);
        }
        let wanted = self.views.any_frame_due() || self.platform.vsync_waits.any();
        if let Some(vsync) = &self.vsync
            && wanted
        {
            vsync.set_paused(false);
        }
    }

    /// One refresh of the display, whose vsync occurred at `vsync`: the input that arrived for it
    /// read first, then the waits on it served, and a frame of the views that asked for one and
    /// have a window, if any did. The signal is paused after a few refreshes with nothing to draw
    /// and nobody waiting.
    fn refresh(&mut self, vsync: Instant) {
        self.read_input();
        let wanted = self.views.any_frame_due();
        let waited = self.platform.vsync_waits.any();
        if waited {
            self.idle_ticks = 0;
            self.platform.vsync_waits.serve(self.refresh_period());
        }
        if wanted {
            self.idle_ticks = 0;
            self.draw_frame(vsync);
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

    /// The time between two refreshes of the screen, as the system reports its rate; a 60 Hz
    /// panel's where it reports none.
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

    /// Draws one frame, for the vsync at `vsync`, of the views that asked for one and have a
    /// window: the framework builds, lays out and paints, and each view presents.
    fn draw_frame(&mut self, vsync: Instant) {
        let views = self.views.take_frames_due();
        let frame = Frame {
            elapsed: self.platform.elapsed(),
            vsync: Some(self.frame_vsync(vsync)),
        };
        if let Some(client) = &mut self.client {
            client.frame(&views, frame);
        }
    }

    /// The times of the vsync at `vsync` on the platform's clock: its picture is for the next
    /// refresh, one period on, as Flutter's Android waiter targets it.
    fn frame_vsync(&self, vsync: Instant) -> FrameVsync {
        let start = self.platform.elapsed_at(vsync);
        FrameVsync {
            start,
            target: start + self.refresh_period(),
        }
    }
}
