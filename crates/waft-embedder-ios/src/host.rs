//! The host: what UIKit tells the app, and where each of it goes.
//!
//! Everything UIKit tells reaches the host as a [`HostEvent`] through [`deliver`], on the main
//! thread, and is served in the order it came. An event delivered while the host serves none is
//! served at once, inside UIKit's call, so a resize draws its frame before UIKit commits the size;
//! one delivered while the host serves another, because UIKit called back into Waft from inside a
//! call the host or the client made, waits in the queue and is served right after. The client is
//! therefore never entered twice, and a later editor must keep every call into the client inside
//! [`Host`]'s handling of an event. A panic there aborts the process once the panic hook has
//! printed it, as UIKit's frames, which every event comes through, were never built to be unwound.
//!
//! The host starts the client as the launch finishes, with the implicit view and no window for it
//! (`launch`); each scene that connects gets a view, the implicit view the first, and shows it in
//! a window of its own (`scene_connected`). The app's lifecycle state follows its scenes'
//! (`lifecycle.rs`), its frames the display link's ticks while a view in sight waits for one, and
//! its wakes the main queue (`wakes.rs`).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Instant;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_foundation::NSURL;
use objc2_ui_kit::UIWindowScene;
use waft_embedder::{
    AppLifecycleState, EmbedderClient, Frame, FrameVsync, KeyData, NotificationActivation,
    PlatformRef, PointerReport, View, ViewFocusDirection, ViewFocusEvent, ViewFocusState, ViewId,
};
use waft_embedder_apple::abort_on_panic;

use crate::display_link::DisplayLink;
use crate::gpu::Gpu;
use crate::keyboard_inset::KeyboardChange;
use crate::lifecycle::{SceneState, ScenesLifecycle};
use crate::platform::IosPlatform;
use crate::view::IosView;
use crate::views::{IMPLICIT_VIEW, Views};

/// Ticks in a row with nothing to draw and nobody waiting before the display link is paused, so an
/// idle app takes no wakes.
const IDLE_TICKS: u8 = 2;

/// What UIKit told the app, in the terms the host serves it in.
pub(crate) enum HostEvent {
    /// A scene connected to `session`, asking to open `urls`.
    SceneConnected {
        scene: Retained<UIWindowScene>,
        session: String,
        urls: Vec<Retained<NSURL>>,
    },
    /// The scene of `session` disconnected: UIKit may connect it again later.
    SceneDisconnected { session: String },
    /// The scene of `session` is in `state` from now on.
    SceneState { session: String, state: SceneState },
    /// The user closed these sessions' scenes for good.
    SessionsDiscarded(Vec<String>),
    /// UIKit asks a connected scene's app to open `urls`.
    UrlsOpened(Vec<Retained<NSURL>>),
    /// The view numbered by this id was laid out again: its size, its safe area or its scale may
    /// have changed.
    LaidOut(ViewId),
    /// The user's appearance or languages may have changed.
    SettingsChanged,
    /// What fingers and pens did on a view.
    Pointer(Vec<PointerReport>),
    /// The keyboard moves, or hides.
    Keyboard(KeyboardChange),
    /// The display refreshed at `at`; the picture drawn now reaches the screen at `target`.
    Vsync { at: Instant, target: Instant },
    /// A wake asked for, of the framework's, whose time may have come.
    Wake,
    /// A wait on the display's refreshes was set down from any thread.
    VsyncWaited,
    /// The pasteboard may have changed.
    ClipboardMayHaveChanged,
    /// The user clicked one of the app's notifications.
    NotificationActivated(NotificationActivation),
}

/// The host as the main thread's callers reach it, whatever its client.
pub(crate) trait Serve {
    /// Serves one event.
    fn serve(&mut self, event: HostEvent);

    /// Hands the client a key a view heard, answering whether it was handled.
    fn key_data(&mut self, data: KeyData) -> bool;
}

thread_local! {
    /// The host, from the launch on. Main thread.
    static HOST: RefCell<Option<Box<dyn Serve>>> = const { RefCell::new(None) };
    /// The events not yet served. Main thread.
    static QUEUE: RefCell<VecDeque<HostEvent>> = const { RefCell::new(VecDeque::new()) };
}

/// Hands the host `event`, served now unless the host is serving another, or is not there yet.
/// Main thread.
pub(crate) fn deliver(event: HostEvent) {
    QUEUE.with_borrow_mut(|queue| queue.push_back(event));
    abort_on_panic(drain);
}

/// Hands the client a key now, after the events waiting; unhandled where the host cannot take it
/// now, being in the middle of another event or not there yet. Main thread.
pub(crate) fn key_data(data: KeyData) -> bool {
    abort_on_panic(|| {
        drain();
        HOST.with(|host| {
            let Ok(mut host) = host.try_borrow_mut() else {
                return false;
            };
            host.as_mut().is_some_and(|host| host.key_data(data))
        })
    })
}

/// Makes `host` the host, and serves what waited for it. Main thread.
pub(crate) fn install(host: Box<dyn Serve>) {
    HOST.with_borrow_mut(|installed| *installed = Some(host));
    abort_on_panic(drain);
}

/// Serves the events waiting, unless the host is serving one already, which serves them as it
/// returns to its loop.
fn drain() {
    HOST.with(|host| {
        let Ok(mut host) = host.try_borrow_mut() else {
            return;
        };
        let Some(host) = host.as_mut() else {
            return;
        };
        while let Some(event) = QUEUE.with_borrow_mut(VecDeque::pop_front) {
            host.serve(event);
        }
    });
}

/// The app's host: its client, the platform the client holds, its views and its scenes.
pub(crate) struct Host<C> {
    client: C,
    platform: Rc<IosPlatform>,
    views: Rc<Views>,
    gpu: Gpu,
    display_link: Rc<DisplayLink>,
    scenes: ScenesLifecycle,
    /// The state the client was last told the app is in.
    reported_state: AppLifecycleState,
    idle_ticks: u8,
}

impl<C: EmbedderClient> Host<C> {
    /// The host of an app launched now: the device every view draws with, the platform, and the
    /// implicit view, with no window yet, which the client is started with.
    pub(crate) fn launch(
        main_thread: MainThreadMarker,
        start: impl FnOnce(PlatformRef) -> C,
    ) -> Host<C> {
        let gpu = Gpu::acquire();
        let views = Rc::new(Views::default());
        let display_link = Rc::new(DisplayLink::new(main_thread));
        let platform = Rc::new(IosPlatform::new(
            Rc::clone(&views),
            &gpu,
            Rc::clone(&display_link),
        ));
        views.add(IosView::new(views.take_id(), &gpu));
        crate::keyboard_notices::observe(main_thread);
        let client = start(Rc::clone(&platform) as PlatformRef);
        Host {
            client,
            platform,
            views,
            gpu,
            display_link,
            scenes: ScenesLifecycle::default(),
            reported_state: AppLifecycleState::Detached,
            idle_ticks: 0,
        }
    }

    /// A scene connected to `session`: it shows the view that session had, the implicit view for
    /// the first, or a view of its own, which the client hears of, in a window of its own.
    fn scene_connected(&mut self, scene: &UIWindowScene, session: String) {
        let view = match self.views.for_session(&session) {
            Some(view) => view,
            None => self.add_view(),
        };
        view.attach(scene, session.clone());
        self.scenes.set(&session, SceneState::Background);
        self.read_settings();
        self.sync_metrics(&view);
        self.client.window_changed(view.id());
    }

    /// A view of its own for a scene that connects when every view has had one.
    fn add_view(&mut self) -> Rc<IosView> {
        let view = IosView::new(self.views.take_id(), &self.gpu);
        self.views.add(Rc::clone(&view));
        self.client.view_added(view.id());
        view
    }

    /// The scene of `session` disconnected: its view keeps its tree and its last picture for the
    /// scene UIKit may connect again by the same session, and draws nothing meanwhile.
    fn scene_disconnected(&mut self, session: &str) {
        if let Some(view) = self.views.of_session(session) {
            view.detach();
            self.client.window_changed(view.id());
        }
        self.scenes.remove(session);
        self.report_lifecycle();
    }

    /// The scene of `session` is in `state` from now on: its view is in sight in the foreground,
    /// has the focus while active, and the app's state follows the scenes'. A scene coming active
    /// has the pasteboard and the user's settings looked at again, which may have changed while it
    /// was away.
    fn scene_state(&mut self, session: &str, state: SceneState) {
        self.scenes.set(session, state);
        if let Some(view) = self.views.of_session(session) {
            let was_focused = view.is_focused();
            view.set_state(state);
            if view.is_focused() != was_focused {
                self.client.view_focus_changed(ViewFocusEvent {
                    view_id: view.id(),
                    state: if view.is_focused() {
                        ViewFocusState::Focused
                    } else {
                        ViewFocusState::Unfocused
                    },
                    direction: ViewFocusDirection::Undefined,
                });
            }
            if view.is_in_sight() {
                view.represent();
            }
        }
        if state == SceneState::Active {
            self.look_at_clipboard();
            self.read_settings();
        }
        self.report_lifecycle();
    }

    /// The client hears the app's state when its scenes' states make another.
    fn report_lifecycle(&mut self) {
        let state = self.scenes.app_state();
        if state != self.reported_state {
            self.reported_state = state;
            self.client.app_lifecycle_state_changed(state);
        }
    }

    /// The user closed these sessions' scenes for good: their views go, but the implicit view,
    /// which the app was given at launch and never hears removed, as in Flutter; it waits for the
    /// next scene that connects.
    fn sessions_discarded(&mut self, sessions: &[String]) {
        for session in sessions {
            let Some(view) = self.views.of_session(session) else {
                continue;
            };
            if view.id() == IMPLICIT_VIEW {
                view.forget_session();
                continue;
            }
            self.views.remove(view.id());
            self.client.view_removed(view.id());
        }
    }

    /// The view numbered `id` was laid out again: its drawing follows its bounds and scale, and,
    /// where its metrics changed, the client hears so and draws its new layout at once, before UIKit
    /// commits the new size, rather than the old picture stretched over it.
    fn laid_out(&mut self, id: ViewId) {
        let Some(view) = self.views.get(id) else {
            return;
        };
        view.laid_out();
        if self.sync_metrics(&view) && view.is_in_sight() && view.has_area() {
            let frame = Frame {
                elapsed: self.platform.elapsed(),
                vsync: None,
            };
            self.client.frame(&[id], frame);
        }
    }

    /// `view`'s metrics, read again: the client hears of them when they changed, which this
    /// answers.
    fn sync_metrics(&mut self, view: &IosView) -> bool {
        let changed = view.refresh_metrics();
        if changed {
            self.client.view_metrics_changed(view.id());
        }
        changed
    }

    /// The keyboard moves: each view in a scene takes what it covers of it from now on.
    fn keyboard_moved(&mut self, change: KeyboardChange) {
        for view in self.views.all() {
            if view.follow_keyboard(change) {
                self.sync_metrics(&view);
            }
        }
    }

    /// The user's appearance and languages, read again: the client hears of each that changed.
    fn read_settings(&mut self) {
        let appearance = self.views.all().iter().find_map(|view| view.brightness());
        if let Some(brightness) = appearance
            && self.platform.set_brightness(brightness)
        {
            self.client.platform_brightness_changed();
        }
        if self.platform.read_locales() {
            self.client.locales_changed();
        }
    }

    /// The client hears the pasteboard changed, if another app changed it since the host last
    /// looked.
    fn look_at_clipboard(&mut self) {
        if self.platform.clipboard_has_changed() {
            self.client.clipboard_changed();
        }
    }

    /// What `urls` ask the app to open, which the client hears together.
    fn open(&mut self, urls: &[Retained<NSURL>]) {
        let request = crate::open_requests::request_for(urls);
        if !request.is_empty() {
            self.client.open_requested(request);
        }
    }

    /// A wake, if one asked for has come: a ready task, or the framework's timer.
    fn wake(&mut self) {
        if self.platform.wakes().should_wake(Instant::now()) {
            let elapsed = self.platform.elapsed();
            self.client.wake(elapsed);
        }
    }

    /// One refresh of the display, at `at`, its picture reaching the screen at `target`: the waits
    /// on it served, and a frame of the views in sight that asked for one, if any did. The display
    /// link is paused after a few ticks with nothing to draw and nobody waiting.
    fn refresh(&mut self, at: Instant, target: Instant) {
        let waited = self.platform.vsync_waits().any();
        if waited {
            self.platform.vsync_waits().serve(at, target);
        }
        let due = self.views.take_frames_due();
        if !due.is_empty() {
            self.idle_ticks = 0;
            let frame = Frame {
                elapsed: self.platform.elapsed(),
                vsync: Some(FrameVsync {
                    start: self.platform.elapsed_at(at),
                    target: self.platform.elapsed_at(target),
                }),
            };
            self.client.frame(&due, frame);
            return;
        }
        if waited {
            self.idle_ticks = 0;
            return;
        }
        self.idle_ticks += 1;
        if self.idle_ticks >= IDLE_TICKS {
            self.idle_ticks = 0;
            self.display_link.set_paused(true);
        }
    }

    /// Keeps the display link going while a view in sight waits for a frame or a producer waits
    /// on a refresh, after whatever the last event changed.
    fn keep_pace(&self) {
        if self.views.any_frame_due() || self.platform.vsync_waits().any() {
            self.display_link.set_paused(false);
        }
    }
}

impl<C: EmbedderClient> Serve for Host<C> {
    fn serve(&mut self, event: HostEvent) {
        match event {
            HostEvent::SceneConnected {
                scene,
                session,
                urls,
            } => {
                self.scene_connected(&scene, session);
                self.open(&urls);
            }
            HostEvent::SceneDisconnected { session } => self.scene_disconnected(&session),
            HostEvent::SceneState { session, state } => self.scene_state(&session, state),
            HostEvent::SessionsDiscarded(sessions) => self.sessions_discarded(&sessions),
            HostEvent::UrlsOpened(urls) => self.open(&urls),
            HostEvent::LaidOut(id) => self.laid_out(id),
            HostEvent::SettingsChanged => self.read_settings(),
            HostEvent::Pointer(reports) => self.client.pointer_reports(reports),
            HostEvent::Keyboard(change) => self.keyboard_moved(change),
            HostEvent::Vsync { at, target } => self.refresh(at, target),
            HostEvent::Wake => self.wake(),
            HostEvent::VsyncWaited => {}
            HostEvent::ClipboardMayHaveChanged => self.look_at_clipboard(),
            HostEvent::NotificationActivated(activation) => {
                self.client.notification_activated(activation)
            }
        }
        self.keep_pace();
    }

    fn key_data(&mut self, data: KeyData) -> bool {
        self.client.key_data(data)
    }
}
