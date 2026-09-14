//! The winit event loop: the windows the host owns, and where each of winit's events
//! goes.
//!
//! winit hands out its `ActiveEventLoop` only inside these callbacks, so this is where
//! windows are created and dropped, and where the requests the framework left on the
//! platform are served — one turn of the loop drains all of them.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use inset_embedder::{
    EmbedderClient, PlatformRef, PointerChange, TextEditingValue, View, ViewFocusDirection,
    ViewFocusEvent, ViewFocusState, ViewId, WindowError, WindowRef,
};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use crate::gpu::Gpu;
use crate::input::Keyboard;
use crate::os;
use crate::pacing::Pacing;
use crate::platform::{WinitPlatform, brightness_of};
use crate::pointer::Pointer;
use crate::surface::WinitSurface;
use crate::view::{HostedView, WinitView, surface_size, window_metrics};
use crate::windows::{self, WinitWindow};
use crate::{DecodeExecution, ImplicitViewConfig, WinitEmbedder, create_image_loader};

const IMPLICIT_VIEW: ViewId = ViewId(0);

/// Work requests and asynchronous completions both enter through winit's proxy.
#[derive(Clone, Copy, Debug)]
pub(crate) enum HostEvent {
    Requests,
    Wake,
    /// A created window's `close`: dropped when the loop next turns.
    CloseWindow(WindowId),
    /// A refresh of the display, from its display link.
    Vsync,
}

type CreateImageLoader = Box<dyn FnOnce(valo::ImageContext) -> valo_codec::ImageLoader>;

pub(crate) struct WinitApp<C> {
    image_loader_setup: Option<CreateImageLoader>,
    implicit_view_config: Option<ImplicitViewConfig>,
    start: Option<Box<dyn FnOnce(PlatformRef) -> C>>,
    pub(crate) client: Option<C>,
    pub(crate) platform: Rc<WinitPlatform>,
    /// When the framework's frames are drawn: one per refresh of the display.
    pub(crate) pacing: Pacing,
    pub(crate) views: HashMap<WindowId, HostedView>,
    /// One device for every window: decoded images belong to it.
    gpu: Option<Gpu>,
    next_view_id: u64,
    pub(crate) frame_source: Option<WindowId>,
    started: bool,
    /// The mouse, as Flutter's pointer.
    pub(crate) pointer: Pointer,
    pub(crate) keyboard: Keyboard,
}

pub(crate) fn run<C: EmbedderClient + 'static>(
    config: WinitEmbedder,
    start: impl FnOnce(PlatformRef) -> C + 'static,
    image_loader_setup: Option<CreateImageLoader>,
) {
    let event_loop = EventLoop::<HostEvent>::with_user_event()
        .build()
        .expect("create winit event loop");
    let implicit_view = config.implicit_view.is_some().then_some(IMPLICIT_VIEW);
    let platform = Rc::new(WinitPlatform::new(event_loop.create_proxy(), implicit_view));
    let mut host = WinitApp {
        image_loader_setup,
        implicit_view_config: config.implicit_view,
        start: Some(Box::new(start)),
        client: None,
        platform,
        pacing: Pacing::new(),
        views: HashMap::new(),
        gpu: None,
        next_view_id: 1,
        frame_source: None,
        started: false,
        pointer: Pointer::new(),
        keyboard: Keyboard::new(),
    };
    event_loop.run_app(&mut host).expect("run winit event loop");
}

impl<C: EmbedderClient> WinitApp<C> {
    /// Flutter's macOS host honors requests to focus a view; native focus events report the result.
    fn apply_focus_requests(&mut self) {
        for request in self.platform.focus_requests.take() {
            if request.state == ViewFocusState::Focused
                && let Some(view) = self
                    .views
                    .values()
                    .find(|view| view.view.id() == request.view_id)
                && !view.window.has_focus()
            {
                view.window.focus_window();
            }
        }
    }

    fn start_client(&mut self) {
        let start = self.start.take().expect("start runs once");
        self.client = Some(start(self.platform.clone()));
        self.pace();
    }

    fn create_implicit_view(&mut self, event_loop: &ActiveEventLoop, config: ImplicitViewConfig) {
        let mut attributes = Window::default_attributes().with_title(config.title);
        // A phone's window is the screen; winit on iOS would size the window to
        // the request instead.
        if !cfg!(any(target_os = "ios", target_os = "android")) {
            attributes = attributes.with_inner_size(winit::dpi::LogicalSize::new(
                config.logical_size[0],
                config.logical_size[1],
            ));
        }
        let (window_id, _, _) = self
            .open_window(event_loop, attributes, IMPLICIT_VIEW, false)
            .expect("create window");
        self.frame_source = Some(window_id);
    }

    /// Opens a window on the shared device and registers its view. A window that sees
    /// through gets a surface whose alpha the compositor honours, cleared to nothing, so
    /// what the app leaves unpainted shows what is behind the window.
    fn open_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        attributes: winit::window::WindowAttributes,
        view_id: ViewId,
        sees_through: bool,
    ) -> Result<(WindowId, Arc<Window>, Rc<WinitView>), String> {
        let window = event_loop
            .create_window(attributes)
            .map_err(|error| error.to_string())?;
        let window = Arc::new(window);
        let window_id = window.id();
        let gpu = self.gpu.get_or_insert_with(Gpu::acquire).clone();
        let (alpha, clear) = if sees_through {
            (valo::SurfaceAlpha::Transparent, valo::Color::TRANSPARENT)
        } else {
            (valo::SurfaceAlpha::Opaque, valo::Color::WHITE)
        };
        let surface = valo::Surface::new_with_options(
            &gpu.instance,
            &gpu.adapter,
            &gpu.device,
            window.clone(),
            surface_size(&window),
            valo::SurfaceOptions::default().with_alpha(alpha),
        )
        .map_err(|error| error.to_string())?;
        let mut context = valo::Context::new(gpu.device.clone(), gpu.queue.clone());
        context.set_hide_missing_glyphs(true);
        if self.platform.image_loader.borrow().is_none() {
            let images = context.image_context();
            let loader = match self.image_loader_setup.take() {
                Some(make_loader) => make_loader(images),
                None => create_image_loader(images, DecodeExecution::default())
                    .expect("start the image decode worker"),
            };
            *self.platform.image_loader.borrow_mut() = Some(loader);
            *self.platform.images.borrow_mut() = Some(context.image_context());
        }
        let surface = Arc::new(Mutex::new(WinitSurface {
            surface,
            context,
            in_transaction: false,
        }));
        let view = Rc::new(WinitView {
            id: view_id,
            metrics: Cell::new(window_metrics(&window)),
            surface,
            window: Arc::clone(&window),
            editing_state: RefCell::new(TextEditingValue::EMPTY),
            text_input: Cell::new(None),
            transform: RefCell::new(None),
            clear,
            latest: RefCell::new(None),
            on_screen: Cell::new(false),
            refused: Cell::new(None),
            retries: Cell::new(0),
            resizing: Cell::new(false),
        });
        if let Some(theme) = window.theme() {
            self.platform.brightness.set(brightness_of(theme));
        }
        self.platform.add_view(view.clone());
        self.views.insert(
            window_id,
            HostedView {
                window: Arc::clone(&window),
                view: Rc::clone(&view),
                handle: None,
            },
        );
        Ok((window_id, window, view))
    }

    /// Serves the windows the app asked for since the loop last turned.
    fn open_requested_windows(&mut self, event_loop: &ActiveEventLoop) {
        let requests = self.platform.windowing.take_requests();
        if requests.is_empty() {
            return;
        }
        for request in requests {
            let view_id = ViewId(self.next_view_id);
            self.next_view_id += 1;
            let attributes = windows::attributes_for(&request.config);
            let sees_through = request.config.background.sees_through();
            let opened = self.open_window(event_loop, attributes, view_id, sees_through);
            let result = match opened {
                Ok((window_id, window, view)) => {
                    os::configure(&window, &request.config);
                    let handle = Rc::new(WinitWindow::new(
                        window_id,
                        Arc::downgrade(&window),
                        view,
                        self.platform.proxy.clone(),
                    ));
                    if let Some(hosted) = self.views.get_mut(&window_id) {
                        hosted.handle = Some(Rc::clone(&handle));
                    }
                    if self.frame_source.is_none() {
                        self.frame_source = Some(window_id);
                    }
                    if let Some(client) = &mut self.client {
                        client.view_added(view_id);
                    }
                    let window: WindowRef = handle;
                    Ok(window)
                }
                Err(reason) => Err(WindowError::Failed(reason)),
            };
            request.reply.complete(result);
        }
        // The futures resolve at the app's checkpoint; a wake brings one.
        if let Some(client) = &mut self.client {
            client.wake(self.platform.elapsed());
        }
    }

    fn remove_view(&mut self, window_id: WindowId) {
        let Some(hosted_view) = self.views.remove(&window_id) else {
            return;
        };
        let view_id = hosted_view.view.id();
        self.platform.remove_view(view_id);
        if let Some(client) = &mut self.client {
            client.view_removed(view_id);
        }
        if self.frame_source == Some(window_id) {
            self.frame_source = self.views.keys().next().copied();
            self.pacing.window_gone();
        }
    }

    fn on_focused(&mut self, id: WindowId, focused: bool) {
        if let Some(view) = self.views.get(&id)
            && let Some(client) = &mut self.client
        {
            client.view_focus_changed(ViewFocusEvent {
                view_id: view.view.id(),
                state: if focused {
                    ViewFocusState::Focused
                } else {
                    ViewFocusState::Unfocused
                },
                direction: ViewFocusDirection::Undefined,
            });
        }
    }

    fn on_close_requested(&mut self, event_loop: &ActiveEventLoop, id: WindowId) {
        let handler = self
            .views
            .get(&id)
            .and_then(|hosted| hosted.handle.as_ref())
            .and_then(|handle| handle.close_handler());
        match handler {
            // The app answers a created window's close request itself.
            Some(handler) => handler(),
            None => {
                self.remove_view(id);
                if self.views.is_empty() {
                    event_loop.exit();
                }
            }
        }
    }

    fn on_resized(&mut self, id: WindowId) {
        if let Some(hosted_view) = self.views.get(&id) {
            let metrics = window_metrics(&hosted_view.window);
            if hosted_view.view.metrics.get() == metrics {
                // winit reports a resize for more than one of the system's
                // notices; the same geometry again is nothing to draw for.
                return;
            }
            let size = surface_size(&hosted_view.window);
            hosted_view.view.resized(size);
            hosted_view.view.metrics.set(metrics);
            if let Some(client) = &mut self.client {
                client.view_metrics_changed(hosted_view.view.id());
            }
            // Present the resized layout before the system commits the
            // window geometry, rather than stretching the old frame.
            if os::FRAME_ON_RESIZE && size[0] > 0 && size[1] > 0 {
                let view = Rc::clone(&hosted_view.view);
                self.platform.frame_requested.set(false);
                view.resizing.set(true);
                self.draw_frame();
                view.resizing.set(false);
                self.pacing.drawn();
            }
        }
    }

    /// Only the system asks winit for redraws; the framework's frames come at the
    /// display's refresh. A window seen again, which is the system's word that its surface
    /// takes frames now, or one the system wants repainted, gets its latest picture
    /// presented again if it is not on the surface; otherwise the content is current and
    /// nothing is drawn.
    fn on_redraw_requested(&mut self, id: WindowId) {
        if let Some(hosted) = self.views.get(&id) {
            hosted.view.represent();
        }
    }

    fn on_theme_changed(&mut self, theme: winit::window::Theme) {
        self.platform.brightness.set(brightness_of(theme));
        if let Some(client) = &mut self.client {
            client.platform_brightness_changed();
        }
    }

    fn on_cursor_entered(&mut self, id: WindowId) {
        self.pointer.window = Some(id);
        // The entry carries no position, and the last one seen may be from
        // before the window moved; the add goes out at where the pointer is.
        if let Some(position) = self
            .views
            .get(&id)
            .and_then(|hosted| os::pointer_position(&hosted.window))
        {
            self.pointer.move_to(position);
        }
        self.add_pointer(id);
    }

    fn on_cursor_left(&mut self, id: WindowId) {
        if self.pointer.remove() {
            self.send_pointer(id, PointerChange::Remove);
        }
    }

    fn on_cursor_moved(&mut self, id: WindowId, position: PhysicalPosition<f64>) {
        self.pointer.window = Some(id);
        self.pointer.move_to([position.x, position.y]);
        self.add_pointer(id);
        self.reconcile_buttons(id);
        let motion = self.pointer.motion();
        self.send_pointer(id, motion);
    }

    fn on_mouse_input(&mut self, id: WindowId, state: ElementState, button: MouseButton) {
        if let Some(change) = self
            .pointer
            .set_button(button, state == ElementState::Pressed)
        {
            self.send_pointer(id, change);
        }
    }
}

impl<C: EmbedderClient> ApplicationHandler<HostEvent> for WinitApp<C> {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        let winit::event::StartCause::ResumeTimeReached {
            requested_resume, ..
        } = cause
        else {
            return;
        };
        if self.pacing.tick_is_due(requested_resume) {
            self.tick();
        }
        if self
            .platform
            .wake_due
            .get()
            .is_some_and(|due| due <= requested_resume)
        {
            self.platform.wake_due.set(None);
            if let Some(client) = &mut self.client {
                client.wake(self.platform.elapsed());
            }
            self.pace();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // A wake already due is delivered before the loop waits: a task the framework posted
        // to itself, a zero-duration timer, runs ahead of whatever the system delivers next,
        // as Dart's event queue orders them.
        if self
            .platform
            .wake_due
            .get()
            .is_some_and(|due| due <= Instant::now())
        {
            self.platform.wake_due.set(None);
            if let Some(client) = &mut self.client {
                client.wake(self.platform.elapsed());
            }
            self.pace();
        }
        let resume_at = match (self.platform.wake_due.get(), self.pacing.tick_due()) {
            (Some(wake), Some(tick)) => Some(wake.min(tick)),
            (wake, tick) => wake.or(tick),
        };
        event_loop.set_control_flow(match resume_at {
            Some(resume_at) => winit::event_loop::ControlFlow::WaitUntil(resume_at),
            None => winit::event_loop::ControlFlow::Wait,
        });
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.started {
            return;
        }
        self.started = true;

        if let Some(config) = self.implicit_view_config.take() {
            self.create_implicit_view(event_loop, config);
        }
        self.start_client();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: HostEvent) {
        if let HostEvent::CloseWindow(window_id) = event {
            self.remove_view(window_id);
            if self.views.is_empty() {
                event_loop.exit();
            }
            return;
        }
        if matches!(event, HostEvent::Wake) {
            if let Some(window_id) = self.pointer.window {
                self.reconcile_buttons(window_id);
            }
            if let Some(client) = &mut self.client {
                client.wake(self.platform.elapsed());
            }
        }
        if matches!(event, HostEvent::Vsync) {
            self.tick();
        }
        self.open_requested_windows(event_loop);
        self.apply_cursor_request();
        self.apply_focus_requests();
        self.pace();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Focused(focused) => self.on_focused(id, focused),
            WindowEvent::CloseRequested => self.on_close_requested(event_loop, id),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => self.on_resized(id),
            WindowEvent::RedrawRequested | WindowEvent::Occluded(false) => {
                self.on_redraw_requested(id)
            }
            WindowEvent::ThemeChanged(theme) => self.on_theme_changed(theme),
            // Flutter's macOS embedder sends `kAdd` on `mouseEntered` and `kRemove` on
            // `mouseExited`; a `MouseRegion` exits on the remove. A drag that leaves the
            // window keeps its button and goes on as moves.
            WindowEvent::CursorEntered { .. } => self.on_cursor_entered(id),
            WindowEvent::CursorLeft { .. } => self.on_cursor_left(id),
            WindowEvent::CursorMoved { position, .. } => self.on_cursor_moved(id, position),
            WindowEvent::ModifiersChanged(modifiers) => self.keyboard.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => self.send_key(id, &event, is_synthetic),
            WindowEvent::Ime(ime) => self.send_ime(id, ime),
            WindowEvent::MouseWheel { delta, .. } => self.send_scroll(id, delta),
            WindowEvent::MouseInput { state, button, .. } => self.on_mouse_input(id, state, button),
            _ => {}
        }
    }
}
