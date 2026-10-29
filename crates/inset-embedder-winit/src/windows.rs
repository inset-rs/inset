//! The app's views and their windows: the one registry of them, which the event loop and the
//! platform share, and how they are made.
//!
//! Where the system's window is the screen, the screen view is made when the app starts, before
//! it has a window, as Flutter's engine adds it with empty metrics; its window opens at winit's
//! first call, the first moment winit lets one be made. A desktop has no screen view. A view the app asks for is made with its window, on a turn of the loop:
//! winit hands out its `ActiveEventLoop` only inside its calls, so the request goes to the loop
//! as a command, and the window arrives through the future the request returned.
//!
//! A window is made with no surface; the embedder gives it one while the system gives the windows
//! theirs (`main_loop/`).

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::sync::Arc;

use inset_embedder::{RequestId, View, ViewId, ViewRequest, WindowConfig, WindowLevel};
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::app_requests::AppRequests;
use crate::gpu::Gpu;
use crate::host_window::WinitWindow;
use crate::os::{OperatingSystem, SystemDrags, ThisOs, ThisOsVisibilityNotices, WindowChrome};
use crate::view::WinitView;

/// The screen view's id, as in Flutter: the views the app asks for are numbered after it.
pub(crate) const SCREEN_VIEW: ViewId = ViewId(0);

/// A view with its window, as the registry answers it.
#[derive(Clone)]
pub(crate) struct OpenWindow {
    pub(crate) window: Arc<dyn Window>,
    pub(crate) view: Rc<WinitView>,
    /// The app's handle on a window it asked for; the screen view's window has none.
    pub(crate) handle: Option<Rc<WinitWindow>>,
}

/// What a window's closing did to its view.
pub(crate) enum ViewAfterClose {
    /// The view went with its window, as every view but the screen view does.
    Removed(ViewId),
    /// The screen view stays, with no window, as in Flutter's embedders.
    Stays,
}

/// The app's views, each with its window while it has one, in the order they came. Every call
/// borrows the registry for itself alone and answers copies, so a caller may call the client
/// with what it got, and the client may reach the registry through the platform meanwhile.
///
/// The registry owns each window; nothing else holds one for longer than a call, but the
/// surfaces its view draws into, which the view lets go of when the window closes. Closing a
/// window here is what frees it.
#[derive(Default)]
pub(crate) struct Views {
    /// By view: views are numbered as they come, so this is the order they came in.
    entries: RefCell<BTreeMap<ViewId, ViewEntry>>,
    view_of_window: RefCell<HashMap<WindowId, ViewId>>,
}

struct ViewEntry {
    view: Rc<WinitView>,
    window: Option<ViewWindow>,
    /// Whether the view stays once its window closes: the screen view.
    screen: bool,
}

struct ViewWindow {
    window: Arc<dyn Window>,
    handle: Option<Rc<WinitWindow>>,
    /// The system's notices that the window may have been minimized or occluded, for the app;
    /// `None` where winit's events are all the system says, and for the screen view's window.
    _visibility_notices: Option<ThisOsVisibilityNotices>,
}

impl Views {
    /// The screen view, with no window yet.
    pub(crate) fn add_screen_view(&self, view: Rc<WinitView>) {
        self.insert(ViewEntry {
            view,
            window: None,
            screen: true,
        });
    }

    /// A view the app asked for, with its window and the system's notices about the window,
    /// which end as the window closes.
    pub(crate) fn add_with_window(
        &self,
        open: OpenWindow,
        visibility_notices: Option<ThisOsVisibilityNotices>,
    ) {
        let window = open.window.id();
        let view = open.view.id();
        self.insert(ViewEntry {
            view: open.view,
            window: Some(ViewWindow {
                window: open.window,
                handle: open.handle,
                _visibility_notices: visibility_notices,
            }),
            screen: false,
        });
        self.view_of_window.borrow_mut().insert(window, view);
    }

    /// `window` is the screen view's from now on.
    pub(crate) fn attach_screen_window(&self, window: Arc<dyn Window>) {
        let id = window.id();
        let mut entries = self.entries.borrow_mut();
        let entry = entries
            .values_mut()
            .find(|entry| entry.screen)
            .expect("the screen view was added");
        entry.window = Some(ViewWindow {
            window,
            handle: None,
            _visibility_notices: None,
        });
        self.view_of_window.borrow_mut().insert(id, entry.view.id());
    }

    /// Closes `window`: its view lets go of it, and the window is freed once the caller's copies
    /// of it are dropped. The view goes with it, but for the screen view, which stays.
    pub(crate) fn close(&self, window: WindowId) -> Option<ViewAfterClose> {
        let view_id = self.view_of_window.borrow_mut().remove(&window)?;
        let (view, after_close) = self.detach_window(view_id)?;
        view.window_closed();
        Some(after_close)
    }

    /// Takes `view_id`'s window from its entry, and the entry too unless it is the screen
    /// view's; answers the view, for the caller to tell once the registry is let go of.
    fn detach_window(&self, view_id: ViewId) -> Option<(Rc<WinitView>, ViewAfterClose)> {
        let mut entries = self.entries.borrow_mut();
        let entry = entries.get_mut(&view_id)?;
        entry.window = None;
        let view = Rc::clone(&entry.view);
        if entry.screen {
            return Some((view, ViewAfterClose::Stays));
        }
        entries.remove(&view_id);
        Some((view, ViewAfterClose::Removed(view_id)))
    }

    pub(crate) fn by_window(&self, window: WindowId) -> Option<OpenWindow> {
        let view = self.view_id_of(window)?;
        self.by_view(view)
    }

    /// The view `view` with its window; `None` for a view without one.
    pub(crate) fn by_view(&self, view: ViewId) -> Option<OpenWindow> {
        self.entries.borrow().get(&view).and_then(ViewEntry::open)
    }

    pub(crate) fn view_id_of(&self, window: WindowId) -> Option<ViewId> {
        self.view_of_window.borrow().get(&window).copied()
    }

    /// Every view with its window, in the order they came.
    pub(crate) fn open_windows(&self) -> Vec<OpenWindow> {
        self.entries
            .borrow()
            .values()
            .filter_map(ViewEntry::open)
            .collect()
    }

    /// The window that came first of those still open.
    pub(crate) fn first_window(&self) -> Option<OpenWindow> {
        self.entries.borrow().values().find_map(ViewEntry::open)
    }

    /// Every view, with a window or not, in the order they came.
    pub(crate) fn views(&self) -> Vec<Rc<WinitView>> {
        self.entries
            .borrow()
            .values()
            .map(|entry| Rc::clone(&entry.view))
            .collect()
    }

    /// The view numbered `id`, with a window or not.
    pub(crate) fn get(&self, id: ViewId) -> Option<Rc<WinitView>> {
        self.entries
            .borrow()
            .get(&id)
            .map(|entry| Rc::clone(&entry.view))
    }

    pub(crate) fn screen_view(&self) -> Option<Rc<WinitView>> {
        self.entries
            .borrow()
            .values()
            .find(|entry| entry.screen)
            .map(|entry| Rc::clone(&entry.view))
    }

    fn insert(&self, entry: ViewEntry) {
        let id = entry.view.id();
        assert!(
            self.entries.borrow_mut().insert(id, entry).is_none(),
            "duplicate view id {id:?}"
        );
    }
}

impl ViewEntry {
    fn open(&self) -> Option<OpenWindow> {
        let window = self.window.as_ref()?;
        Some(OpenWindow {
            window: Arc::clone(&window.window),
            view: Rc::clone(&self.view),
            handle: window.handle.clone(),
        })
    }
}

/// A request for a view, on its way to the loop, which makes a window for it.
pub(crate) struct WindowRequest {
    pub request_id: RequestId,
    pub request: ViewRequest,
}

/// Makes the views, on the one device every view draws with, and their windows.
pub(crate) struct ViewMaker {
    /// One device for every view: decoded images belong to it. A device needs no window.
    gpu: Option<Gpu>,
    /// The number of the next view the app asks for; the screen view is the first.
    next_view_id: u64,
}

impl ViewMaker {
    pub(crate) fn new() -> ViewMaker {
        ViewMaker {
            gpu: None,
            next_view_id: SCREEN_VIEW.0 + 1,
        }
    }

    /// The screen view, with no window yet, for a window configured as `config`.
    pub(crate) fn make_screen_view(&mut self, config: &WindowConfig) -> Rc<WinitView> {
        Rc::new(self.view(SCREEN_VIEW, config))
    }

    /// The view `window_request` asked for, carrying its request's id and initial data, with
    /// its window and the window's handle, which sends its `close` to the loop through
    /// `requests`.
    pub(crate) fn make_requested(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_request: &WindowRequest,
        requests: Rc<AppRequests>,
    ) -> Result<OpenWindow, String> {
        let config = &window_request.request.config;
        let window = open_window(event_loop, config)?;
        let view = self.view(ViewId(self.next_view_id), config).with_request(
            window_request.request_id,
            window_request.request.initial_data.clone(),
        );
        let view = Rc::new(view);
        self.next_view_id += 1;
        view.attach_window(&window);
        let handle = Rc::new(WinitWindow::new(
            window.id(),
            Arc::downgrade(&window),
            Rc::clone(&view),
            requests,
        ));
        view.set_window_handle(&handle);
        Ok(OpenWindow {
            window,
            view,
            handle: Some(handle),
        })
    }

    /// The view numbered `id`, on the device every view draws with, for a window configured as
    /// `config`.
    fn view(&mut self, id: ViewId, config: &WindowConfig) -> WinitView {
        let gpu = self.gpu.get_or_insert_with(Gpu::acquire);
        WinitView::new(id, gpu, config.background.sees_through())
    }
}

/// The screen view's window, where the system's window is the screen, as on iOS. A desktop
/// has none: the app opens every window it has, as Flutter's multi-window apps do.
pub(crate) fn screen_window_config() -> Option<WindowConfig> {
    ThisOs::WINDOW_IS_THE_SCREEN.then(WindowConfig::default)
}

/// The screen view's window, configured as `config`, attached to `view`.
pub(crate) fn open_screen_window(
    event_loop: &dyn ActiveEventLoop,
    config: &WindowConfig,
    view: &WinitView,
) -> Result<Arc<dyn Window>, String> {
    let window = open_window(event_loop, config)?;
    view.attach_window(&window);
    Ok(window)
}

/// A window configured as `config`: winit's attributes, then what the system's module applies
/// beyond them, and the drags it takes beyond winit's.
fn open_window(
    event_loop: &dyn ActiveEventLoop,
    config: &WindowConfig,
) -> Result<Arc<dyn Window>, String> {
    let window = event_loop
        .create_window(attributes_for(config))
        .map_err(|error| error.to_string())?;
    ThisOs::configure(&*window, config);
    ThisOs::accept_drags(&*window);
    Ok(Arc::from(window))
}

/// winit's attributes for a configuration. What winit has no attribute for, the system's
/// module applies after creation. A system whose window is the screen sizes it itself.
fn attributes_for(config: &WindowConfig) -> WindowAttributes {
    let level = match config.level {
        WindowLevel::Normal => winit::window::WindowLevel::Normal,
        WindowLevel::AlwaysOnTop => winit::window::WindowLevel::AlwaysOnTop,
        WindowLevel::AlwaysOnBottom => winit::window::WindowLevel::AlwaysOnBottom,
    };
    let mut attributes = WindowAttributes::default();
    if !ThisOs::WINDOW_IS_THE_SCREEN {
        attributes = attributes.with_surface_size(LogicalSize::new(config.size[0], config.size[1]));
    }
    attributes = attributes
        .with_title(&config.title)
        .with_decorations(config.decorations)
        .with_resizable(config.resizable)
        .with_transparent(config.background.sees_through())
        .with_window_level(level)
        .with_visible(config.visible)
        .with_active(config.activating);
    if let Some([x, y]) = config.position {
        attributes = attributes.with_position(LogicalPosition::new(x, y));
    }
    ThisOs::extend_attributes(attributes, config)
}
