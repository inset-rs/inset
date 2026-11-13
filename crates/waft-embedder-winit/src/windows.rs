//! The app's windows: the registry of what is a window's, which the event loop and the platform
//! share, and how a window and its view are made. The views themselves, a window's among them, are
//! in the registry of views (`views.rs`).
//!
//! Where the system's window is the screen, the screen view is made when the app starts, before it
//! has a window, as Flutter's engine adds it with empty metrics; its window opens at winit's first
//! call, the first moment winit lets one be made. A desktop has no screen view. A view the app asks
//! for is made with its window, on a turn of the loop: winit hands out its `ActiveEventLoop` only
//! inside its calls, so the request goes to the loop as a command, and the window arrives through
//! the future the request returned.
//!
//! A window holds its view as an app holds a view it adds (`view.rs`): it places in itself the
//! layer its view shows into, which on macOS it makes and keeps filling the window through a hold
//! of its own on it, kept here with the window (`os`'s `ViewLayer` and `WindowHold`); the loop adds
//! the view on that layer with the window's metrics, gives the view the window's metrics again as
//! the window resizes or moves, and removes it as the window closes (`main_loop/windows.rs`).

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::sync::Arc;

use waft_embedder::{
    RequestId, View, ViewConstraints, ViewHost, ViewId, ViewMetrics, ViewPadding, WindowConfig,
    WindowLevel,
};
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::LinuxSettings;
use crate::app_requests::AppRequests;
use crate::gpu::LazyGpu;
use crate::host_window::{WinitWindow, winit_icon};
use crate::os::capabilities::{
    NativeComposition, OperatingSystem, SystemDrags, WindowChrome, WindowGeometry,
};
use crate::os::{System, TrackpadTouchObserver, ViewLayer, VisibilityNotices, WindowHold};
use crate::screens::screen_of;
use crate::view::WinitView;
use crate::view_host::WindowViewHost;
use crate::views::{SCREEN_VIEW, Views};

/// A window with its view, as the registry answers it.
#[derive(Clone)]
pub(crate) struct OpenWindow {
    pub(crate) window: Arc<dyn Window>,
    pub(crate) view: Rc<WinitView>,
    /// The view's host, the window's: it takes the window's keys and input methods.
    pub(crate) host: Rc<WindowViewHost>,
    /// The app's handle on a window it asked for, through which it closes it; the screen view's
    /// window has none, and lasts as long as the app.
    pub(crate) handle: Option<Rc<WinitWindow>>,
}

/// A window just opened, holding a view that is yet to be added on the layer the window placed
/// in itself for it.
pub(crate) struct OpenedWindow {
    pub(crate) open: OpenWindow,
    /// The layer the window placed for its view.
    pub(crate) view_layer: ViewLayer,
    /// The window's hold on that layer.
    pub(crate) hold: WindowHold,
}

/// The app's windows, each with what is a window's, in the order their views came. Every call
/// borrows the registry for itself alone and answers copies, so a caller may call the client
/// with what it got, and the client may reach the registry through the platform meanwhile.
///
/// The registry owns each window; nothing else holds one for longer than a call, but the layer its
/// view shows into where that is the window, which the view lets go of as it is removed. Closing a
/// window here and removing its view is what frees it.
#[derive(Default)]
pub(crate) struct Windows {
    /// By view: views are numbered as they come, so this is the order their windows came in.
    entries: RefCell<BTreeMap<ViewId, WindowEntry>>,
    view_of_window: RefCell<HashMap<WindowId, ViewId>>,
}

struct WindowEntry {
    /// The window's hold on the layer its view shows into, which places the layer in the window
    /// until the window closes. Before the window, so that it is dropped first, while the window
    /// is still open.
    _view_layer: WindowHold,
    /// Before the window, so that it is dropped first: what observes the window lets go of it
    /// while it is still open.
    _observers: WindowObservers,
    window: Arc<dyn Window>,
    view: Rc<WinitView>,
    /// The view's host, which the window's code made with the view.
    host: Rc<WindowViewHost>,
    handle: Option<Rc<WinitWindow>>,
}

/// What the embedder hears of one window from the system beyond winit's events, each kept until
/// the window closes: `None` where winit's events are all the system says, and for the screen
/// view's window.
#[derive(Default)]
#[expect(
    dead_code,
    reason = "each is kept for its drop, which ends the observing"
)]
pub(crate) struct WindowObservers {
    /// The system's notices that the window may have been minimized or occluded, for the app.
    pub(crate) visibility: Option<VisibilityNotices>,
    /// What reports the fingers coming down on a trackpad over the window.
    pub(crate) trackpad_touches: Option<TrackpadTouchObserver>,
}

impl Windows {
    /// The window of `open` is one of the app's, with its hold on its view's layer and what the
    /// embedder observes of it, which end as it closes.
    pub(crate) fn add(&self, open: OpenWindow, view_layer: WindowHold, observers: WindowObservers) {
        let window = open.window.id();
        let view = open.view.id();
        let entry = WindowEntry {
            _view_layer: view_layer,
            _observers: observers,
            window: open.window,
            view: open.view,
            host: open.host,
            handle: open.handle,
        };
        assert!(
            self.entries.borrow_mut().insert(view, entry).is_none(),
            "a second window for view {view:?}"
        );
        self.view_of_window.borrow_mut().insert(window, view);
    }

    /// Closes `window`: it is freed once the caller's copies of it are dropped and its view, which
    /// this answers for the caller to remove, has let go of its layer.
    pub(crate) fn close(&self, window: WindowId) -> Option<ViewId> {
        let view = self.view_of_window.borrow_mut().remove(&window)?;
        self.entries.borrow_mut().remove(&view)?;
        Some(view)
    }

    pub(crate) fn by_window(&self, window: WindowId) -> Option<OpenWindow> {
        let view = self.view_id_of(window)?;
        self.by_view(view)
    }

    /// The window of view `view`; `None` for a view in no window.
    pub(crate) fn by_view(&self, view: ViewId) -> Option<OpenWindow> {
        self.entries.borrow().get(&view).map(WindowEntry::open)
    }

    pub(crate) fn view_id_of(&self, window: WindowId) -> Option<ViewId> {
        self.view_of_window.borrow().get(&window).copied()
    }

    /// Every open window, in the order their views came.
    pub(crate) fn open_windows(&self) -> Vec<OpenWindow> {
        self.entries
            .borrow()
            .values()
            .map(WindowEntry::open)
            .collect()
    }

    /// The window that came first of those still open.
    pub(crate) fn first_window(&self) -> Option<OpenWindow> {
        self.entries.borrow().values().next().map(WindowEntry::open)
    }
}

impl WindowEntry {
    fn open(&self) -> OpenWindow {
        OpenWindow {
            window: Arc::clone(&self.window),
            view: Rc::clone(&self.view),
            host: Rc::clone(&self.host),
            handle: self.handle.clone(),
        }
    }
}

/// A window the app asked for, on its way to the loop, which makes it: the request, its
/// configuration, and what its view carries when it arrives.
pub(crate) struct RequestedWindow {
    pub request_id: RequestId,
    pub config: WindowConfig,
    /// What the view carries when it arrives.
    pub initial_data: Option<String>,
}

/// The screen view's window, which opens at winit's first call: its configuration, and the view
/// and its host, made with the app.
pub(crate) struct ScreenWindow {
    config: WindowConfig,
    pub(crate) view: Rc<WinitView>,
    host: Rc<WindowViewHost>,
}

/// Makes the windows' views, which share the one device they draw with, their hosts and their
/// windows.
pub(crate) struct ViewMaker {
    /// One device for every view, made at the first draw if no image made it before.
    gpu: LazyGpu,
    /// Where the views' hosts and the windows' handles send their requests, to the loop.
    requests: Rc<AppRequests>,
    /// What the app tells Linux's desktops, its app id among it, which every window carries.
    linux: LinuxSettings,
}

impl ViewMaker {
    pub(crate) fn new(gpu: LazyGpu, requests: Rc<AppRequests>, linux: LinuxSettings) -> ViewMaker {
        ViewMaker {
            gpu,
            requests,
            linux,
        }
    }

    /// The screen view, with its host and its window to open, where the system's window is the
    /// screen, as on iOS. A desktop has none: the app opens every window it has, as Flutter's
    /// multi-window apps do.
    pub(crate) fn make_screen_view(&self) -> Option<ScreenWindow> {
        let config = System::WINDOW_IS_THE_SCREEN.then(WindowConfig::default)?;
        let (view, host) = self.view(SCREEN_VIEW, &config);
        Some(ScreenWindow {
            config,
            view: Rc::new(view),
            host,
        })
    }

    /// The window `window_request` asked for, with its view, numbered by `views`, carrying its
    /// request's id and initial data, the view's host, and the window's handle, which sends its
    /// `close` to the loop.
    pub(crate) fn make_requested(
        &self,
        event_loop: &dyn ActiveEventLoop,
        window_request: &RequestedWindow,
        views: &Views,
    ) -> Result<OpenedWindow, String> {
        let config = &window_request.config;
        let window = self.open_window(event_loop, config)?;
        let (view, host) = self.view(views.take_id(), config);
        let view = Rc::new(view.with_request(
            window_request.request_id,
            window_request.initial_data.clone(),
        ));
        let (view_layer, hold) = hold_view(&window, &host);
        let handle = Rc::new(WinitWindow::new(
            window.id(),
            Arc::downgrade(&window),
            Rc::clone(&view),
            Rc::clone(&self.requests),
        ));
        view.set_window_handle(&handle);
        let open = OpenWindow {
            window,
            view,
            host,
            handle: Some(handle),
        };
        Ok(OpenedWindow {
            open,
            view_layer,
            hold,
        })
    }

    /// Opens the screen view's window, `screen`, holding the screen view.
    pub(crate) fn open_screen_window(
        &self,
        event_loop: &dyn ActiveEventLoop,
        screen: ScreenWindow,
    ) -> Result<OpenedWindow, String> {
        let window = self.open_window(event_loop, &screen.config)?;
        let (view_layer, hold) = hold_view(&window, &screen.host);
        let open = OpenWindow {
            window,
            view: screen.view,
            host: screen.host,
            handle: None,
        };
        Ok(OpenedWindow {
            open,
            view_layer,
            hold,
        })
    }

    /// A window configured as `config`, of this app: winit's attributes, then what the system's
    /// module applies beyond them, and the drags it takes beyond winit's.
    fn open_window(
        &self,
        event_loop: &dyn ActiveEventLoop,
        config: &WindowConfig,
    ) -> Result<Arc<dyn Window>, String> {
        let window = event_loop
            .create_window(attributes_for(event_loop, config, &self.linux))
            .map_err(|error| error.to_string())?;
        System::configure(&*window, config);
        System::accept_drags(&*window);
        Ok(Arc::from(window))
    }

    /// The view numbered `id`, drawing with the device every view shares, for a window
    /// configured as `config`, with the host the window's code makes for it.
    fn view(&self, id: ViewId, config: &WindowConfig) -> (WinitView, Rc<WindowViewHost>) {
        let host = Rc::new(WindowViewHost::new(id, Rc::clone(&self.requests)));
        let view = WinitView::new(
            id,
            &self.gpu,
            config.background.sees_through(),
            Rc::clone(&host) as Rc<dyn ViewHost>,
        );
        (view, host)
    }
}

/// `window` holds the view whose host is `host` from now on: it makes the layer the view is to
/// draw into and places it in itself, and its input methods are the host's. Answers the layer and
/// the window's hold on it.
fn hold_view(window: &Arc<dyn Window>, host: &WindowViewHost) -> (ViewLayer, WindowHold) {
    let layer_and_hold = System::view_layer_in(window);
    host.attach(window);
    layer_and_hold
}

/// The metrics of `window` as its view takes them: its surface's size, its scale and safe area,
/// and the screen it is on.
pub(crate) fn window_metrics(window: &dyn Window) -> ViewMetrics {
    let [width, height] = System::surface_size(window).map(f64::from);
    let padding = System::view_padding(window);
    ViewMetrics {
        physical_size: [width, height],
        physical_constraints: ViewConstraints::tight(width, height),
        device_pixel_ratio: window.scale_factor(),
        // No system winit serves reports a keyboard, so the safe area is the whole of
        // the padding and nothing covers it.
        padding,
        view_padding: padding,
        view_insets: ViewPadding::ZERO,
        // Flutter's desktop and iOS embedders report no system gesture insets.
        system_gesture_insets: ViewPadding::ZERO,
        screen: screen_of(window),
    }
}

/// winit's attributes for a configuration, the system's own among them for this app's `linux`
/// settings. What winit has no attribute for, the system's module applies after creation. A
/// system whose window is the screen sizes it itself.
fn attributes_for(
    event_loop: &dyn ActiveEventLoop,
    config: &WindowConfig,
    linux: &LinuxSettings,
) -> WindowAttributes {
    let level = match config.level {
        WindowLevel::Normal => winit::window::WindowLevel::Normal,
        WindowLevel::AlwaysOnTop => winit::window::WindowLevel::AlwaysOnTop,
        WindowLevel::AlwaysOnBottom => winit::window::WindowLevel::AlwaysOnBottom,
    };
    let mut attributes = WindowAttributes::default();
    if !System::WINDOW_IS_THE_SCREEN {
        attributes = attributes.with_surface_size(LogicalSize::new(config.size[0], config.size[1]));
    }
    attributes = attributes
        .with_title(&config.title)
        .with_window_icon(config.icon.as_ref().and_then(winit_icon))
        .with_decorations(config.decorations)
        .with_resizable(config.resizable)
        .with_transparent(config.background.sees_through())
        .with_window_level(level)
        .with_visible(config.visible)
        .with_active(config.activating);
    if let Some([x, y]) = config.position {
        attributes = attributes.with_position(LogicalPosition::new(x, y));
    }
    System::extend_attributes(event_loop, attributes, config, linux)
}
