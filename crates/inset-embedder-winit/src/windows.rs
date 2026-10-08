//! The app's views and their windows: the one registry of them, which the event loop and the
//! platform share, and how they are made.
//!
//! The implicit view is made when the app starts, before it has a window, as Flutter's engine
//! adds it with empty metrics; its window opens at winit's first call, the first moment winit
//! lets one be made. A view the app asks for is made with its window, on a turn of the loop:
//! winit hands out its `ActiveEventLoop` only inside its calls, so the request goes to the loop
//! as a command, and the window arrives through the future the request returned.
//!
//! A window is made with no surface; the embedder gives it one while the system gives the windows
//! theirs (`main_loop/`).

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::sync::Arc;

use inset_embedder::{
    View, ViewId, WindowConfig, WindowError, WindowFuture, WindowLevel, WindowRef, WindowingOwner,
};
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::app_requests::{AppRequest, AppRequests};
use crate::gpu::Gpu;
use crate::host_window::WinitWindow;
use crate::os::{OperatingSystem, ThisOs, WindowChrome};
use crate::reply::{Reply, reply};
use crate::view::WinitView;

/// The implicit view's id, as in Flutter: the views the app asks for are numbered after it.
pub(crate) const IMPLICIT_VIEW: ViewId = ViewId(0);

/// A view with its window, as the registry answers it.
#[derive(Clone)]
pub(crate) struct OpenWindow {
    pub(crate) window: Arc<Window>,
    pub(crate) view: Rc<WinitView>,
    /// The app's handle on a window it asked for; the implicit view's window has none.
    pub(crate) handle: Option<Rc<WinitWindow>>,
}

/// What a window's closing did to its view.
pub(crate) enum ViewAfterClose {
    /// The view went with its window, as every view but the implicit one does.
    Removed(ViewId),
    /// The implicit view stays, with no window, as in Flutter's embedders.
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
    /// Whether the view stays once its window closes: the implicit view.
    implicit: bool,
}

struct ViewWindow {
    window: Arc<Window>,
    handle: Option<Rc<WinitWindow>>,
}

impl Views {
    /// The implicit view, with no window yet.
    pub(crate) fn add_implicit(&self, view: Rc<WinitView>) {
        self.insert(ViewEntry {
            view,
            window: None,
            implicit: true,
        });
    }

    /// A view the app asked for, with its window.
    pub(crate) fn add_with_window(&self, open: OpenWindow) {
        let window = open.window.id();
        let view = open.view.id();
        self.insert(ViewEntry {
            view: open.view,
            window: Some(ViewWindow {
                window: open.window,
                handle: open.handle,
            }),
            implicit: false,
        });
        self.view_of_window.borrow_mut().insert(window, view);
    }

    /// `window` is the implicit view's from now on.
    pub(crate) fn attach_implicit_window(&self, window: Arc<Window>) {
        let id = window.id();
        let mut entries = self.entries.borrow_mut();
        let entry = entries
            .values_mut()
            .find(|entry| entry.implicit)
            .expect("the implicit view was added");
        entry.window = Some(ViewWindow {
            window,
            handle: None,
        });
        self.view_of_window.borrow_mut().insert(id, entry.view.id());
    }

    /// Closes `window`: its view lets go of it, and the window is freed once the caller's copies
    /// of it are dropped. The view goes with it, but for the implicit view, which stays.
    pub(crate) fn close(&self, window: WindowId) -> Option<ViewAfterClose> {
        let view_id = self.view_of_window.borrow_mut().remove(&window)?;
        let (view, after_close) = self.detach_window(view_id)?;
        view.window_closed();
        Some(after_close)
    }

    /// Takes `view_id`'s window from its entry, and the entry too unless it is the implicit
    /// view's; answers the view, for the caller to tell once the registry is let go of.
    fn detach_window(&self, view_id: ViewId) -> Option<(Rc<WinitView>, ViewAfterClose)> {
        let mut entries = self.entries.borrow_mut();
        let entry = entries.get_mut(&view_id)?;
        entry.window = None;
        let view = Rc::clone(&entry.view);
        if entry.implicit {
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

    pub(crate) fn no_window_open(&self) -> bool {
        self.view_of_window.borrow().is_empty()
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

    pub(crate) fn implicit(&self) -> Option<Rc<WinitView>> {
        self.entries
            .borrow()
            .values()
            .find(|entry| entry.implicit)
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

/// A window the app asked for, on its way to the loop.
pub(crate) struct WindowRequest {
    pub config: WindowConfig,
    pub reply: Reply<Result<WindowRef, WindowError>>,
}

/// The platform's window maker: sends each request to the loop, which makes the window.
pub(crate) struct WinitWindowing {
    requests: Rc<AppRequests>,
}

impl WinitWindowing {
    pub(crate) fn new(requests: Rc<AppRequests>) -> WinitWindowing {
        WinitWindowing { requests }
    }
}

impl WindowingOwner for WinitWindowing {
    fn create(&self, config: WindowConfig) -> WindowFuture {
        let (reply, answer) = reply();
        self.requests
            .send(AppRequest::OpenWindow(WindowRequest { config, reply }));
        Box::pin(answer)
    }
}

/// Makes the views, on the one device every view draws with, and their windows.
pub(crate) struct ViewMaker {
    /// One device for every view: decoded images belong to it. A device needs no window.
    gpu: Option<Gpu>,
    /// The number of the next view the app asks for; the implicit view is the first.
    next_view_id: u64,
}

impl ViewMaker {
    pub(crate) fn new() -> ViewMaker {
        ViewMaker {
            gpu: None,
            next_view_id: IMPLICIT_VIEW.0 + 1,
        }
    }

    /// The implicit view, with no window yet, for a window configured as `config`.
    pub(crate) fn make_implicit(&mut self, config: &WindowConfig) -> Rc<WinitView> {
        self.view(IMPLICIT_VIEW, config)
    }

    /// A view the app asked for, with its window configured as `config`, and the handle the app
    /// holds it by, which sends its `close` to the loop through `requests`.
    pub(crate) fn make_requested(
        &mut self,
        event_loop: &ActiveEventLoop,
        config: &WindowConfig,
        requests: Rc<AppRequests>,
    ) -> Result<(OpenWindow, WindowRef), String> {
        let window = open_window(event_loop, config)?;
        let view = self.view(ViewId(self.next_view_id), config);
        self.next_view_id += 1;
        view.attach_window(&window);
        let handle = Rc::new(WinitWindow::new(
            window.id(),
            Arc::downgrade(&window),
            Rc::clone(&view),
            requests,
        ));
        let open = OpenWindow {
            window,
            view,
            handle: Some(Rc::clone(&handle)),
        };
        Ok((open, handle))
    }

    /// The view numbered `id`, on the device every view draws with, for a window configured as
    /// `config`.
    fn view(&mut self, id: ViewId, config: &WindowConfig) -> Rc<WinitView> {
        let gpu = self.gpu.get_or_insert_with(Gpu::acquire);
        Rc::new(WinitView::new(id, gpu, config.background.sees_through()))
    }
}

/// The implicit view's window, configured as `config`, attached to `view`.
pub(crate) fn open_implicit_window(
    event_loop: &ActiveEventLoop,
    config: &WindowConfig,
    view: &WinitView,
) -> Result<Arc<Window>, String> {
    let window = open_window(event_loop, config)?;
    view.attach_window(&window);
    Ok(window)
}

/// A window configured as `config`: winit's attributes, then what the system's module applies
/// beyond them.
fn open_window(event_loop: &ActiveEventLoop, config: &WindowConfig) -> Result<Arc<Window>, String> {
    let window = event_loop
        .create_window(attributes_for(config))
        .map_err(|error| error.to_string())?;
    ThisOs::configure(&window, config);
    Ok(Arc::new(window))
}

/// winit's attributes for a configuration. What winit has no attribute for, the system's
/// module applies after creation. A system whose window is the screen sizes it itself.
fn attributes_for(config: &WindowConfig) -> WindowAttributes {
    let level = match config.level {
        WindowLevel::Normal => winit::window::WindowLevel::Normal,
        WindowLevel::AlwaysOnTop => winit::window::WindowLevel::AlwaysOnTop,
        WindowLevel::AlwaysOnBottom => winit::window::WindowLevel::AlwaysOnBottom,
    };
    let mut attributes = Window::default_attributes();
    if !ThisOs::WINDOW_IS_THE_SCREEN {
        attributes = attributes.with_inner_size(LogicalSize::new(config.size[0], config.size[1]));
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
