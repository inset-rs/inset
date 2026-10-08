//! The embedder capability the framework holds: [`Platform`], whose answers are each handed to
//! the module that owns them — the clipboard (`clipboard.rs`), native popup menus
//! (`popup_menus.rs`), the mouse cursor (`mouse_cursor.rs`), quitting (`app_exit.rs`), images
//! (`images.rs`), files the system asks the app to open (`system_requests.rs`), the screens
//! (the system's own, `os/`) — and what only macOS offers beyond `Platform` (`macos.rs`).
//!
//! No call here calls the client. What needs the event loop is sent to it as a request, and the
//! rest acts at once: `main_loop/` says which is which.

use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use inset_embedder::{
    AppExit, Brightness, Clipboard, Compositor, Dispatcher, FileOpening, FontSource,
    ImageCodecFuture, MouseCursor, Platform, PopupMenus, Screens, SystemFontSource, TargetPlatform,
    ViewFocusDirection, ViewFocusEvent, ViewFocusState, ViewId, ViewRef, WindowingOwner,
};
use winit::event_loop::EventLoopProxy;

use crate::app_requests::AppRequest;
use crate::clipboard::Pasteboard;
use crate::dispatcher::WinitDispatcher;
use crate::macos::MacOs;
use crate::main_loop::MainLoopEvent;
use crate::os::{NativeComposition, OperatingSystem, Screens as _, SystemRequests, ThisOs};
use crate::popup_menus::NativePopupMenus;
use crate::state::EmbedderState;
use crate::windows::WinitWindowing;

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
    /// Where the dispatcher's wakes and a closed menu's word go, from any thread.
    proxy: EventLoopProxy<MainLoopEvent>,
    /// The window maker handed to the app; its requests are served by the main loop.
    windowing: Rc<WinitWindowing>,
    /// The system's compositor for native surfaces, where it has one.
    compositor: Option<Arc<dyn Compositor>>,
    origin: Instant,
    pasteboard: Pasteboard,
    popup_menus: NativePopupMenus,
    /// What only macOS offers; `None` elsewhere.
    macos: Option<MacOs>,
}

impl WinitPlatform {
    pub(crate) fn new(
        state: Rc<EmbedderState>,
        proxy: EventLoopProxy<MainLoopEvent>,
    ) -> WinitPlatform {
        WinitPlatform {
            windowing: Rc::new(WinitWindowing::new(Rc::clone(state.requests()))),
            macos: cfg!(target_os = "macos").then(|| MacOs::new(Rc::clone(&state))),
            state,
            compositor: ThisOs::compositor(),
            origin: Instant::now(),
            pasteboard: Pasteboard::default(),
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

    fn request_frame(&self) {
        self.send(AppRequest::RequestFrame);
    }

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn dispatcher(&self) -> Arc<dyn Dispatcher> {
        Arc::new(WinitDispatcher(self.proxy.clone()))
    }

    /// Every view, with a window or not, in the order they came: the implicit view first.
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

    /// The same view for the app's whole life: there before its window opens, and still there,
    /// drawing nowhere, once its window closes.
    fn implicit_view(&self) -> Option<ViewRef> {
        self.state.views().implicit().map(|view| view as ViewRef)
    }

    /// The OS font database, scanned on request; the shell asks once at start-up.
    fn font_source(&self) -> Option<Box<dyn FontSource>> {
        Some(Box::new(SystemFontSource::platform()))
    }

    fn compositor(&self) -> Option<Arc<dyn Compositor>> {
        self.compositor.clone()
    }

    /// The embedder's window maker where the app may open windows: none on iOS.
    fn windowing_owner(&self) -> Option<Rc<dyn WindowingOwner>> {
        ThisOs::APP_MAY_OPEN_WINDOWS.then(|| Rc::clone(&self.windowing) as Rc<dyn WindowingOwner>)
    }

    /// The system's own answers, where it is asked outside winit's callbacks: on macOS.
    fn screens(&self) -> Option<&dyn Screens> {
        ThisOs::screens()
    }

    fn import_pixels(&self, pixels: valo::PixelBuffer) -> Option<valo::Image> {
        self.image_context()?.upload_pixels(pixels, false).ok()
    }

    /// The system pasteboard, where the embedder can reach one.
    fn clipboard(&self) -> Option<&dyn Clipboard> {
        Some(&self.pasteboard)
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
