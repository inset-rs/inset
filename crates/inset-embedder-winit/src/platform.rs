//! The embedder capability the framework holds: [`Platform`], whose answers are each handed to
//! the module that owns them — the clipboard (`clipboard.rs`), native popup menus
//! (`popup_menus.rs`), the mouse cursor (`mouse_cursor.rs`), quitting (`app_exit.rs`), images
//! (`images.rs`), files the system asks the app to open (`system_requests.rs`), the screens
//! (`screens.rs`), the user's languages (`locales.rs`) — and what only macOS offers beyond
//! `Platform` (`macos.rs`).
//!
//! No call here calls the client. What needs the event loop is sent to it as a request, and the
//! rest acts at once: `main_loop/` says which is which.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use inset_embedder::{
    AppExit, Brightness, Clipboard, Compositor, Dispatcher, FileOpening, FontSource,
    ImageCodecFuture, Locale, MouseCursor, Platform, PopupMenus, RequestId, Screens,
    SystemFontSource, TargetPlatform, ViewError, ViewFocusDirection, ViewFocusEvent,
    ViewFocusState, ViewId, ViewRef, ViewRequest, VsyncWaiter,
};
use winit::event_loop::EventLoopProxy;

use crate::app_requests::AppRequest;
use crate::clipboard::ClipboardChanges;
use crate::dispatcher::WinitDispatcher;
use crate::macos::MacOs;
use crate::main_loop::MainLoopEvent;
use crate::os::{
    FramePresentation, NativeComposition, OperatingSystem, SystemClipboard, SystemRequests, ThisOs,
    ThisOsClipboard,
};
use crate::popup_menus::NativePopupMenus;
use crate::state::EmbedderState;
use crate::windows::WindowRequest;

/// The long-lived embedder capability held by `App`.
///
/// No call on it calls the client: it answers from what the embedder knows (`state.rs`), and
/// what needs the main loop is sent to it as a request (`main_loop/` says which calls those
/// are).
///
/// Beyond [`Platform`], the embedder hands out its renderer's [`image_context`], for an image
/// from a texture the app makes itself, and what only macOS offers, through [`macos`]. An app
/// reaches these with `platform.downcast_ref::<WinitPlatform>()`.
///
/// [`image_context`]: WinitPlatform::image_context
/// [`macos`]: WinitPlatform::macos
pub struct WinitPlatform {
    /// What the embedder knows, shared with the main loop, which changes it.
    state: Rc<EmbedderState>,
    /// Where the dispatcher's wakes, the waits on a screen's vsync and a closed menu's word go,
    /// from any thread.
    proxy: EventLoopProxy<MainLoopEvent>,
    /// The id of the next request for a view.
    next_request_id: Cell<u64>,
    /// The system's compositor for native surfaces, where it has one.
    compositor: Option<Arc<dyn Compositor>>,
    /// Where any thread waits for a screen's next vsync.
    vsync_waiter: Arc<dyn VsyncWaiter>,
    origin: Instant,
    clipboard: ThisOsClipboard,
    popup_menus: NativePopupMenus,
    /// What only macOS offers; `None` elsewhere.
    macos: Option<MacOs>,
}

impl WinitPlatform {
    pub(crate) fn new(
        state: Rc<EmbedderState>,
        proxy: EventLoopProxy<MainLoopEvent>,
    ) -> WinitPlatform {
        let clipboard = ThisOs::clipboard(Box::new({
            let requests = Rc::clone(state.requests());
            move || requests.send(AppRequest::LookAtClipboard)
        }));
        WinitPlatform {
            next_request_id: Cell::new(1),
            macos: cfg!(target_os = "macos").then(|| MacOs::new(Rc::clone(&state))),
            state,
            compositor: ThisOs::compositor(),
            vsync_waiter: ThisOs::vsync_waiter(&proxy),
            origin: Instant::now(),
            clipboard,
            popup_menus: NativePopupMenus::new(proxy.clone()),
            proxy,
        }
    }

    /// The renderer's image context, on the device every window draws with: an image made
    /// through it, from pixels or from a texture imported from a buffer only this system
    /// has, is one `RawImage` shows. `None` until the first view has given the embedder a
    /// device.
    pub fn image_context(&self) -> Option<valo::ImageContext> {
        self.state.images().store()
    }

    /// What only macOS offers an app: hearing a reopen, a window that slides in, and hiding the
    /// app. `None` on every other system.
    pub fn macos(&self) -> Option<&MacOs> {
        self.macos.as_ref()
    }

    /// Sends the main loop a request, which it serves after those sent before.
    pub(crate) fn send(&self, request: AppRequest) {
        self.state.requests().send(request);
    }

    pub(crate) fn elapsed(&self) -> Duration {
        self.now().saturating_duration_since(self.origin)
    }

    /// Whether the system's clipboard changed since this was last asked.
    pub(crate) fn clipboard_has_changed(&self) -> bool {
        self.clipboard.has_changed()
    }

    /// When the main loop is to clear a copy of the app's that expires (`clipboard/`).
    pub(crate) fn clipboard_expiry(&self) -> Option<Instant> {
        self.clipboard.expiry()
    }

    /// Clears the app's copy whose time has come by `now`; answers whether it cleared one.
    pub(crate) fn clear_expired_clipboard(&self, now: Instant) -> bool {
        self.clipboard.clear_expired(now)
    }

    /// What the embedder knows, for the parts of the platform in other files.
    pub(crate) fn state(&self) -> &EmbedderState {
        &self.state
    }
}

impl Platform for WinitPlatform {
    /// The embedder this binary was built for — the platform adapter is where embedder detection
    /// belongs, as Flutter's `defaultTargetPlatform` bottoms out in `dart:io`.
    fn target_platform(&self) -> TargetPlatform {
        ThisOs::TARGET_PLATFORM
    }

    fn platform_brightness(&self) -> Brightness {
        self.state.appearance().brightness()
    }

    /// The system's list, as the app was last told it (`locales.rs`).
    fn locales(&self) -> Vec<Locale> {
        self.state.locales().locales()
    }

    /// Drawn at the next vsync of the screen the view's window is on, and held while the window
    /// is minimized or occluded (`frame_scheduler.rs`).
    fn request_frame(&self, view: ViewId) {
        self.send(AppRequest::RequestFrame(view));
    }

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn dispatcher(&self) -> Arc<dyn Dispatcher> {
        Arc::new(WinitDispatcher(self.proxy.clone()))
    }

    /// Every view, with a window or not, in the order they came: the screen view first, where
    /// there is one.
    fn views(&self) -> Vec<ViewRef> {
        self.state
            .views()
            .views()
            .into_iter()
            .map(|view| view as ViewRef)
            .collect()
    }

    fn view(&self, id: ViewId) -> Option<ViewRef> {
        self.state.views().get(id).map(|view| view as ViewRef)
    }

    fn open_image_codec(&self, bytes: Arc<[u8]>) -> ImageCodecFuture {
        self.state.images().open_codec(bytes)
    }

    fn request_view_focus_change(
        &self,
        view_id: ViewId,
        state: ViewFocusState,
        direction: ViewFocusDirection,
    ) {
        self.send(AppRequest::ChangeViewFocus(ViewFocusEvent {
            view_id,
            state,
            direction,
        }));
    }

    /// The OS font database, scanned on request; the shell asks once at start-up.
    fn font_source(&self) -> Option<Box<dyn FontSource>> {
        Some(Box::new(SystemFontSource::platform()))
    }

    fn compositor(&self) -> Option<Arc<dyn Compositor>> {
        self.compositor.clone()
    }

    /// Served at the vsyncs the frames of the views on the screen are drawn at: at the ticks of
    /// its display link, in the run loop's every mode, on macOS (`os/macos/vsync_waiter.rs`), and
    /// by the main loop elsewhere (`vsync_waiter.rs`).
    fn vsync_waiter(&self) -> Option<Arc<dyn VsyncWaiter>> {
        Some(Arc::clone(&self.vsync_waiter))
    }

    /// A window for the view, made on a turn of the main loop, where the app may open windows:
    /// none on iOS.
    fn request_view(&self, request: ViewRequest) -> Result<RequestId, ViewError> {
        if !ThisOs::APP_MAY_OPEN_WINDOWS {
            return Err(ViewError::Unsupported);
        }
        let request_id = RequestId(self.next_request_id.get());
        self.next_request_id.set(request_id.0 + 1);
        self.send(AppRequest::OpenWindow(WindowRequest {
            request_id,
            request,
        }));
        Ok(request_id)
    }

    /// On every system: the system's own answers where it has them, and otherwise winit's
    /// monitors, while the app has a window to list them through (`screens.rs`).
    fn screens(&self) -> Option<&dyn Screens> {
        Some(self)
    }

    fn import_pixels(&self, pixels: valo::PixelBuffer) -> Option<valo::Image> {
        self.image_context()?.upload_pixels(pixels, false).ok()
    }

    /// The system's clipboard: on macOS its pasteboard, elsewhere its plain text
    /// (`clipboard/`).
    fn clipboard(&self) -> Option<&dyn Clipboard> {
        Some(&self.clipboard)
    }

    fn popup_menus(&self) -> Option<&dyn PopupMenus> {
        Some(&self.popup_menus)
    }

    fn mouse_cursor(&self) -> Option<&dyn MouseCursor> {
        Some(self)
    }

    fn app_exit(&self) -> Option<&dyn AppExit> {
        ThisOs::APP_MAY_QUIT.then_some(self)
    }

    fn file_opening(&self) -> Option<&dyn FileOpening> {
        ThisOs::ASKS_TO_OPEN_FILES.then_some(self)
    }
}
