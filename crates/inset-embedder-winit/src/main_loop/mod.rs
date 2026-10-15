//! The main loop: the one place that hears every event, winit's and each [`MainLoopEvent`], and
//! the only one that tells the app or asks the OS. Each event goes to the helper that owns its
//! concern, which decides what it means, and the loop delivers what the helper answers. Its
//! reactions live in files by concern: `windows`, `frames`, `input`, `quitting` and `app`; this
//! one holds the loop's state and routes each event.
//!
//! winit hands out its `ActiveEventLoop` only inside these callbacks, so this is where
//! windows are created and dropped, and where what the framework asks through the platform is
//! served: each request (`app_requests.rs`) is served on a turn of its own in the order
//! asked. A frame it asks for goes to the frame scheduler, which the loop owns and which says
//! whether it is drawn now or at the next vsync (`frame_scheduler.rs`); the loop draws it
//! then.
//!
//! # Who calls whom
//!
//! Everything here runs on the main thread. Two things reach the embedder from other threads, and
//! each only sends the loop an event through winit's proxy or runs where it was sent: the
//! dispatcher's wakes and work (`dispatcher.rs`), and native surfaces' producers, which present
//! and ask for vsyncs from their own threads (`os/macos/composition`).
//!
//! winit calls the handler here from AppKit's callbacks, and never again while a call is
//! running: what AppKit reports meanwhile, winit queues and delivers once the call returns. The
//! handler calls the client, and the client calls the platform, its views and its windows from
//! inside that call. None of those calls the client back. What needs the event loop, or the
//! client, is sent to the loop for a turn of its own: a frame, a window, an exit, the cursor, a
//! focus change, handing files to the app's handler. The rest acts at once, inside the
//! framework's call: showing, hiding, moving or retitling a window, and hiding one that closes
//! (the loop lets go of it on its next turn), sliding one in, hiding the application, the
//! clipboard, a view's present and its text input, and what the screens and the pointer are.
//!
//! One framework call runs a loop of AppKit's own: `PopupMenus::show`, while the menu tracks the
//! pointer. winit calls the handler nothing meanwhile, but the display links, which run in the
//! run loop's common modes, keep running, so native surfaces' producers get their vsyncs there,
//! and the main dispatch queue is served.
//!
//! The loop runs two of the app's own handlers directly, from a turn of its own and outside every
//! framework call, so a handler may post to the app: the files to open and the reopen
//! (`system_requests.rs`).
//!
//! What AppKit calls on its own, from its dispatch and possibly inside a winit callback, never
//! reaches the loop's state or the client: the application delegate's methods and the
//! application's lifecycle notices only queue for the loop (`os/macos/app_delegate.rs`,
//! `os/macos/app_lifecycle.rs`), and a window's visibility notices and its screen's display
//! link serve only its native surfaces (`os/macos/composition`).
//!
//! # The app, its views and their surfaces
//!
//! The embedder starts the app before winit's loop runs, as Flutter's engine runs Dart's `main`
//! before its window is ready (`launch.rs`). On a desktop the app opens its windows itself, its
//! first one included, which `run_app` asks for. Where the system's window is the screen, the
//! screen view is made first, with no window and no size, then the app, whose `run_app` builds
//! in that view; the view's window opens at winit's first call, the first moment winit
//! lets one be made, and the app hears its new metrics. Views come and go while the app runs,
//! each with its window: one the app opens reaches it through `view_added`, and one that closes
//! through `view_removed`. The screen view alone outlives its window, as in Flutter's
//! embedders. A window closes only when the app closes it: the user's asking only reports it
//! (`view_close_requested`), and what the last window's closing means is the app's to decide.
//!
//! A view draws on a surface the system gives and takes away: winit's `resumed` gives every view
//! one and asks for a frame, at launch and each time after, and `suspended` takes them all while
//! the app and its views stay. A view made while the system gives surfaces gets its own at once.

mod app;
mod frames;
mod input;
mod quitting;
mod windows;

use std::rc::Rc;
use std::time::Instant;

use inset_embedder::{DropChange, EmbedderClient, ViewId, WindowConfig};
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;

use crate::app_requests::AppRequest;
use crate::dispatcher::Wakeups;
use crate::drag_drop::DragReports;
use crate::frame_scheduler::FrameScheduler;
use crate::images::CreateImageLoader;
use crate::keyboard::Keyboard;
use crate::launch::StartApp;
use crate::lifecycle::{AppLifecycle, LifecycleNotice};
use crate::platform::WinitPlatform;
use crate::pointer::{Pointer, PointerIds};
use crate::state::EmbedderState;
use crate::system_requests::SystemRequest;
use crate::touch::Touches;
use crate::windows::{ViewMaker, Views, screen_window_config};

/// What reaches the loop through winit's proxy: the framework's requests, wakes asked for
/// from other threads, and the system's requests of the application.
pub(crate) enum MainLoopEvent {
    /// Something the framework asked, through the platform, in the order asked: the oldest
    /// request waiting (`app_requests.rs`).
    AppRequest,
    /// A native menu ran its own loop and closed: the buttons it kept are reconciled and the
    /// client woken.
    MenuClosed,
    /// When to wake the framework for its earliest waiting timer, or `None` when no timer
    /// is waiting; served by `about_to_wait` when that time comes.
    TimerWakeup(Option<Instant>),
    /// A ready task asking for the framework to be woken; served by the next wake, and it
    /// never changes the timer wakeup above.
    WakeNow,
    /// A vsync of the first window's display, from its display link.
    Vsync,
    /// Something the system asked of the application as a whole.
    SystemRequest(SystemRequest),
    /// Something the system said of the application as a whole: whether it is active, or
    /// visible.
    Lifecycle(LifecycleNotice),
}

/// The embedder of the running app.
pub(crate) struct MainLoop<C> {
    /// The app, as the embedder drives it.
    client: C,
    platform: Rc<WinitPlatform>,
    /// When the framework's frames are drawn: at the vsync after it asks for one.
    frame_scheduler: FrameScheduler,
    /// When the framework is to be woken, as it asked.
    wakeups: Wakeups,
    /// What the embedder knows, shared with the platform, which answers the app from it.
    state: Rc<EmbedderState>,
    /// Makes the views, on the one device they all draw with, and their windows.
    view_maker: ViewMaker,
    /// The screen view's window, opened at winit's first call; `None` once it is open, and on
    /// a desktop, which has no screen view.
    screen_window: Option<WindowConfig>,
    /// The mouse, as Flutter's pointer.
    pointer: Pointer,
    /// The fingers on the screen, each as a pointer of its own.
    touches: Touches,
    /// The numbers every pointer's data carries, shared so no two devices' presses collide.
    ids: PointerIds,
    keyboard: Keyboard,
    /// Files dragged over the views this turn, reported when the loop is about to wait.
    drag_reports: DragReports,
    /// Whether the application is active and visible, for its lifecycle state.
    lifecycle: AppLifecycle,
    /// Whether the system gives the windows surfaces now: from winit's `resumed` until its
    /// `suspended`.
    surfaces_given: bool,
}

impl<C: EmbedderClient> MainLoop<C> {
    /// Starts the app: the screen view is made, where the system's window is the screen, with
    /// no window yet, and then the app, with that view among its views. The view's window opens
    /// at winit's first call.
    pub(crate) fn start(
        start: StartApp<C>,
        create_image_loader: Option<CreateImageLoader>,
        proxy: EventLoopProxy<MainLoopEvent>,
    ) -> MainLoop<C> {
        let state = Rc::new(EmbedderState::new(proxy.clone(), create_image_loader));
        let platform = Rc::new(WinitPlatform::new(Rc::clone(&state), proxy.clone()));
        let mut view_maker = ViewMaker::new();
        let screen_window = screen_window_config();
        if let Some(window) = &screen_window {
            state.add_screen_view(view_maker.make_screen_view(window));
        }
        let client = start(platform.clone());
        MainLoop {
            client,
            state,
            platform,
            frame_scheduler: FrameScheduler::new(move || {
                let _ = proxy.send_event(MainLoopEvent::Vsync);
            }),
            wakeups: Wakeups::default(),
            view_maker,
            screen_window,
            pointer: Pointer::new(),
            touches: Touches::default(),
            ids: PointerIds::default(),
            keyboard: Keyboard::new(),
            drag_reports: DragReports::default(),
            lifecycle: AppLifecycle::default(),
            surfaces_given: false,
        }
    }

    /// Wakes the client: the futures its app awaits resolve at the checkpoint a wake brings.
    fn wake_client(&mut self) {
        self.client.wake(self.platform.elapsed());
    }

    /// The app's views and their windows.
    fn views(&self) -> &Views {
        self.state.views()
    }

    /// Serves the oldest request the framework or the app made that is not served yet.
    fn serve_next_app_request(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(request) = self.state.requests().take_next() {
            self.on_app_request(event_loop, request);
        }
    }

    /// Serves one request the framework or the app made.
    fn on_app_request(&mut self, event_loop: &ActiveEventLoop, request: AppRequest) {
        match request {
            AppRequest::RequestFrame => self.request_frame(),
            AppRequest::OpenWindow(request) => self.open_window(event_loop, request),
            AppRequest::CloseWindow(id) => self.close_window(id),
            AppRequest::Quit => self.quit(event_loop),
            AppRequest::AnswerExit(response) => self.on_exit_answered(event_loop, response),
            AppRequest::ShowCursor(kind) => self.show_cursor(kind),
            AppRequest::ChangeViewFocus(change) => self.change_view_focus(change),
            AppRequest::HandOverFiles => self.hand_over_files(),
        }
    }

    /// The view of `window`, if the embedder has it.
    fn view_id_of(&self, window: WindowId) -> Option<ViewId> {
        self.views().view_id_of(window)
    }
}

impl<C: EmbedderClient> ApplicationHandler<MainLoopEvent> for MainLoop<C> {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        match cause {
            // winit's first call, on every system.
            StartCause::Init => self.open_screen_window(event_loop),
            StartCause::ResumeTimeReached {
                requested_resume, ..
            } => self.on_wait_ended(requested_resume),
            StartCause::WaitCancelled { .. } | StartCause::Poll => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.report_drags();
        // A wake already due is delivered before the loop waits: a task the framework posted
        // to itself, a zero-duration timer, runs ahead of whatever the system delivers next,
        // as Dart's event queue orders them.
        self.wake_if_due(Instant::now());
        event_loop.set_control_flow(
            self.wakeups
                .control_flow(self.frame_scheduler.vsync_timer_due()),
        );
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        self.give_surfaces();
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.take_surfaces();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: MainLoopEvent) {
        match event {
            MainLoopEvent::AppRequest => self.serve_next_app_request(event_loop),
            MainLoopEvent::MenuClosed => self.on_menu_closed(),
            MainLoopEvent::TimerWakeup(wakeup) => self.on_wake_at(wakeup),
            MainLoopEvent::WakeNow => self.on_wake_now(),
            MainLoopEvent::Vsync => self.on_vsync(),
            MainLoopEvent::SystemRequest(request) => self.on_system_request(request),
            MainLoopEvent::Lifecycle(notice) => self.on_lifecycle_notice(notice),
        }
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Focused(focused) => self.on_focused(id, focused),
            WindowEvent::CloseRequested => self.on_close_requested(id),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => self.on_resized(id),
            WindowEvent::Moved(_) => self.on_moved(id),
            // A window seen again is when a picture its surface refused while occluded is
            // presented (wgpu issue 8309, `view.rs`).
            WindowEvent::RedrawRequested | WindowEvent::Occluded(false) => {
                self.on_redraw_requested(id)
            }
            WindowEvent::ThemeChanged(theme) => self.on_theme_changed(theme),
            WindowEvent::CursorEntered { .. } => self.on_cursor_entered(id),
            WindowEvent::CursorLeft { .. } => self.on_cursor_left(id),
            WindowEvent::CursorMoved { position, .. } => self.on_cursor_moved(id, position),
            WindowEvent::ModifiersChanged(modifiers) => self.on_modifiers_changed(modifiers),
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => self.on_keyboard_input(id, &event, is_synthetic),
            WindowEvent::Ime(ime) => self.on_ime(id, ime),
            WindowEvent::MouseWheel { delta, .. } => self.on_mouse_wheel(id, delta),
            WindowEvent::MouseInput { state, button, .. } => self.on_mouse_input(id, state, button),
            WindowEvent::Touch(touch) => self.on_touch(id, &touch),
            WindowEvent::HoveredFile(path) => self.on_dragged_file(id, DropChange::Entered, path),
            WindowEvent::DroppedFile(path) => self.on_dragged_file(id, DropChange::Dropped, path),
            WindowEvent::HoveredFileCancelled => self.on_drag_exited(id),
            _ => {}
        }
    }
}
