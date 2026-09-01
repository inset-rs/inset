use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reveal_embedder::{
    EmbedderClient, Frame, Picture, Platform, PlatformRef, View, ViewConstraints, ViewId,
    ViewMetrics, ViewPadding, ViewRef,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

use crate::gpu::Gpu;
use crate::{ImplicitViewConfig, WinitEmbedder};

const IMPLICIT_VIEW: ViewId = ViewId(0);

/// The long-lived host capability held by `App`.
///
/// Requests only write state and poke the event loop; they never re-enter the
/// client synchronously.
struct WinitPlatform {
    deadline: Cell<Option<Instant>>,
    frame_requested: Cell<bool>,
    views: RefCell<HashMap<ViewId, ViewRef>>,
    implicit_view: Option<ViewId>,
    proxy: EventLoopProxy<()>,
    origin: Instant,
}

impl WinitPlatform {
    fn new(proxy: EventLoopProxy<()>, implicit_view: Option<ViewId>) -> WinitPlatform {
        WinitPlatform {
            deadline: Cell::new(None),
            frame_requested: Cell::new(false),
            views: RefCell::new(HashMap::new()),
            implicit_view,
            proxy,
            origin: Instant::now(),
        }
    }

    fn poke(&self) {
        let _ = self.proxy.send_event(());
    }

    fn elapsed(&self) -> Duration {
        self.now().saturating_duration_since(self.origin)
    }

    fn add_view(&self, view: ViewRef) {
        let id = view.id();
        assert!(
            self.views.borrow_mut().insert(id, view).is_none(),
            "duplicate view id {id:?}"
        );
    }

    fn remove_view(&self, id: ViewId) {
        assert!(
            self.views.borrow_mut().remove(&id).is_some(),
            "unknown view id {id:?}"
        );
    }
}

impl Platform for WinitPlatform {
    fn request_frame(&self) {
        if !self.frame_requested.replace(true) {
            self.poke();
        }
    }

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn wake_at(&self, deadline: Instant) {
        self.deadline.set(Some(deadline));
        self.poke();
    }

    fn views(&self) -> Vec<ViewRef> {
        self.views.borrow().values().cloned().collect()
    }

    fn view(&self, id: ViewId) -> Option<ViewRef> {
        self.views.borrow().get(&id).cloned()
    }

    fn implicit_view(&self) -> Option<ViewRef> {
        self.implicit_view.and_then(|id| self.view(id))
    }
}

/// Stable view identity, current winit geometry, and the valo present line.
struct WinitView {
    id: ViewId,
    metrics: Cell<ViewMetrics>,
    surface: Arc<Mutex<WinitSurface>>,
}

struct WinitSurface {
    surface: valo::Surface,
    context: valo::Context,
}

impl View for WinitView {
    fn id(&self) -> ViewId {
        self.id
    }

    fn metrics(&self) -> ViewMetrics {
        self.metrics.get()
    }

    fn present(&self, picture: &Picture) {
        let mut state = self.surface.lock().expect("surface lock");
        let Some(surface_frame) = state.surface.acquire() else {
            return;
        };
        state
            .context
            .render(picture, &surface_frame.target(Some(valo::Color::WHITE)));
        state.context.present(surface_frame);
    }
}

struct HostedView {
    window: Arc<Window>,
    view: Rc<WinitView>,
}

struct WinitApp<C> {
    implicit_view_config: Option<ImplicitViewConfig>,
    start: Option<Box<dyn FnOnce(PlatformRef) -> C>>,
    client: Option<C>,
    platform: Rc<WinitPlatform>,
    views: HashMap<WindowId, HostedView>,
    frame_source: Option<WindowId>,
    started: bool,
}

pub(crate) fn run<C: EmbedderClient + 'static>(
    config: WinitEmbedder,
    start: impl FnOnce(PlatformRef) -> C + 'static,
) {
    let event_loop = EventLoop::new().expect("create winit event loop");
    let implicit_view = config.implicit_view.is_some().then_some(IMPLICIT_VIEW);
    let platform = Rc::new(WinitPlatform::new(event_loop.create_proxy(), implicit_view));
    let mut host = WinitApp {
        implicit_view_config: config.implicit_view,
        start: Some(Box::new(start)),
        client: None,
        platform,
        views: HashMap::new(),
        frame_source: None,
        started: false,
    };
    event_loop.run_app(&mut host).expect("run winit event loop");
}

fn window_metrics(window: &Window) -> ViewMetrics {
    let size = window.inner_size();
    let width = f64::from(size.width);
    let height = f64::from(size.height);
    ViewMetrics {
        physical_size: [width, height],
        physical_constraints: ViewConstraints::tight(width, height),
        device_pixel_ratio: window.scale_factor(),
        padding: ViewPadding::ZERO,
        view_padding: ViewPadding::ZERO,
        view_insets: ViewPadding::ZERO,
    }
}

impl<C: EmbedderClient> WinitApp<C> {
    fn flush_frame_request(&mut self) {
        if let Some(window) = self
            .frame_source
            .and_then(|id| self.views.get(&id).map(|view| &view.window))
            && self.platform.frame_requested.replace(false)
        {
            window.request_redraw();
        }
    }

    fn start_client(&mut self) {
        let start = self.start.take().expect("start runs once");
        self.client = Some(start(self.platform.clone()));
        self.flush_frame_request();
    }

    fn create_implicit_view(&mut self, event_loop: &ActiveEventLoop, config: ImplicitViewConfig) {
        let attributes = Window::default_attributes()
            .with_title(config.title)
            .with_inner_size(winit::dpi::LogicalSize::new(
                config.logical_size[0],
                config.logical_size[1],
            ));
        let window = event_loop.create_window(attributes).expect("create window");
        let window = Arc::new(window);
        let window_id = window.id();
        let gpu = Gpu::acquire();
        let size = window.inner_size();
        let surface = valo::Surface::new(
            &gpu.instance,
            &gpu.adapter,
            &gpu.device,
            window.clone(),
            [size.width, size.height],
        )
        .expect("create valo surface");
        let context = valo::Context::new(gpu.device, gpu.queue);
        let view = Rc::new(WinitView {
            id: IMPLICIT_VIEW,
            metrics: Cell::new(window_metrics(&window)),
            surface: Arc::new(Mutex::new(WinitSurface { surface, context })),
        });
        self.platform.add_view(view.clone());
        self.views.insert(window_id, HostedView { window, view });
        self.frame_source = Some(window_id);
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
        }
    }
}

impl<C: EmbedderClient> ApplicationHandler for WinitApp<C> {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        if matches!(cause, winit::event::StartCause::ResumeTimeReached { .. }) {
            self.platform.deadline.set(None);
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
        if self.frame_source.is_some() {
            self.flush_frame_request();
        } else if self.platform.frame_requested.replace(false)
            && let Some(client) = &mut self.client
        {
            client.frame(Frame {
                elapsed: self.platform.elapsed(),
            });
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(match self.platform.deadline.get() {
            Some(deadline) => winit::event_loop::ControlFlow::WaitUntil(deadline),
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

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.remove_view(id);
                if self.views.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(hosted_view) = self.views.get(&id) {
                    let size = hosted_view.window.inner_size();
                    hosted_view
                        .view
                        .surface
                        .lock()
                        .expect("surface lock")
                        .surface
                        .resize([size.width, size.height]);
                    hosted_view
                        .view
                        .metrics
                        .set(window_metrics(&hosted_view.window));
                    if let Some(client) = &mut self.client {
                        client.view_metrics_changed(hosted_view.view.id());
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(client) = &mut self.client {
                    client.frame(Frame {
                        elapsed: self.platform.elapsed(),
                    });
                }
                self.flush_frame_request();
            }
            _ => {}
        }
    }
}
