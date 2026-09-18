//! The host capability the framework holds: [`Platform`] and the capabilities it hands
//! out — the clipboard, native popup menus, the mouse cursor — with the image codec
//! behind `open_image_codec`.
//!
//! A request from the framework only writes state here and wakes the event loop, which
//! serves it on its next turn: winit hands out windows, cursors and its `ActiveEventLoop`
//! only inside its own callbacks, and the framework's callbacks are not one of them.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use inset_embedder::{
    Brightness, Clipboard, Dispatcher, FontSource, ImageCodec, ImageCodecFuture, ImageDecodeError,
    ImageFrame, ImageFrameFuture, ImageRepetition, MouseCursor, Platform, PopupMenuEntry,
    PopupMenus, SystemFontSource, SystemMouseCursorKind, TargetPlatform, ViewFocusDirection,
    ViewFocusEvent, ViewFocusState, ViewId, ViewRef, WindowingOwner,
};
use winit::event_loop::EventLoopProxy;

use crate::os;
use crate::window::HostEvent;
use crate::windows::WinitWindowing;

/// The long-lived host capability held by `App`.
///
/// Requests only write state and wake the event loop; they never re-enter the
/// client synchronously.
///
/// Beyond [`Platform`], the host hands out its renderer's [`image_context`], for an image
/// from a texture the app makes itself; an app reaches that with
/// `platform.downcast_ref::<WinitPlatform>()`.
///
/// [`image_context`]: WinitPlatform::image_context
pub struct WinitPlatform {
    /// When to wake the framework for its earliest waiting timer, as it last set it;
    /// `None` when no timer is waiting, or once that wake has been given.
    pub(crate) timer_wakeup: Cell<Option<Instant>>,
    /// A ready task asking for the framework to be woken, cleared by the next wake.
    pub(crate) wake_now_requested: Cell<bool>,
    pub(crate) frame_requested: Cell<bool>,
    views: RefCell<HashMap<ViewId, ViewRef>>,
    implicit_view: Option<ViewId>,
    pub(crate) proxy: EventLoopProxy<HostEvent>,
    /// The window maker handed to the app; its requests are served by the event loop.
    pub(crate) windowing: Rc<WinitWindowing>,
    origin: Instant,
    pub(crate) brightness: Cell<Brightness>,
    /// The latest system cursor request, applied by the event loop.
    pub(crate) cursor_request: Cell<Option<SystemMouseCursorKind>>,
    /// Requests from the framework, drained by the native event loop.
    pub(crate) focus_requests: RefCell<Vec<ViewFocusEvent>>,
    /// The OS pasteboard; created on first use so a missing clipboard host is not fatal.
    clipboard: RefCell<Option<arboard::Clipboard>>,
    /// Decoded images belong to the device and do not retain a window surface.
    pub(crate) image_loader: RefCell<Option<valo_codec::ImageLoader>>,
    /// The renderer's image store, for images the app brings its own pixels or textures
    /// for; set with the loader, once the first window has a device.
    pub(crate) images: RefCell<Option<valo::ImageContext>>,
}

impl WinitPlatform {
    pub(crate) fn new(
        proxy: EventLoopProxy<HostEvent>,
        implicit_view: Option<ViewId>,
    ) -> WinitPlatform {
        WinitPlatform {
            timer_wakeup: Cell::new(None),
            wake_now_requested: Cell::new(false),
            frame_requested: Cell::new(false),
            views: RefCell::new(HashMap::new()),
            implicit_view,
            windowing: Rc::new(WinitWindowing::new(proxy.clone())),
            proxy,
            origin: Instant::now(),
            brightness: Cell::new(Brightness::Light),
            cursor_request: Cell::new(None),
            focus_requests: RefCell::new(Vec::new()),
            image_loader: RefCell::new(None),
            images: RefCell::new(None),
            clipboard: RefCell::new(None),
        }
    }

    /// The renderer's image context, on the device every window draws with: an image made
    /// through it, from pixels or from a texture imported from a buffer only this system
    /// has, is one `RawImage` shows. `None` until the first window has given the host a
    /// device.
    pub fn image_context(&self) -> Option<valo::ImageContext> {
        self.images.borrow().clone()
    }

    fn with_clipboard<T>(&self, f: impl FnOnce(&mut arboard::Clipboard) -> T) -> Option<T> {
        let mut slot = self.clipboard.borrow_mut();
        if slot.is_none() {
            *slot = arboard::Clipboard::new().ok();
        }
        slot.as_mut().map(f)
    }

    fn wake_event_loop(&self) {
        let _ = self.proxy.send_event(HostEvent::Request);
    }

    pub(crate) fn elapsed(&self) -> Duration {
        self.now().saturating_duration_since(self.origin)
    }

    pub(crate) fn add_view(&self, view: ViewRef) {
        let id = view.id();
        assert!(
            self.views.borrow_mut().insert(id, view).is_none(),
            "duplicate view id {id:?}"
        );
    }

    pub(crate) fn remove_view(&self, id: ViewId) {
        assert!(
            self.views.borrow_mut().remove(&id).is_some(),
            "unknown view id {id:?}"
        );
    }
}

impl Platform for WinitPlatform {
    /// The host this binary was built for — the platform adapter is where
    /// host detection belongs, exactly as Flutter's `defaultTargetPlatform`
    /// bottoms out in `dart:io`'s `Platform.isAndroid` chain.
    fn target_platform(&self) -> TargetPlatform {
        if cfg!(target_os = "ios") {
            TargetPlatform::IOS
        } else if cfg!(target_os = "macos") {
            TargetPlatform::MacOS
        } else if cfg!(target_os = "windows") {
            TargetPlatform::Windows
        } else if cfg!(target_os = "fuchsia") {
            TargetPlatform::Fuchsia
        } else {
            TargetPlatform::Linux
        }
    }

    fn platform_brightness(&self) -> Brightness {
        self.brightness.get()
    }

    fn request_frame(&self) {
        if !self.frame_requested.replace(true) {
            self.wake_event_loop();
        }
    }

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn dispatcher(&self) -> Arc<dyn Dispatcher> {
        Arc::new(WinitDispatcher(self.proxy.clone()))
    }

    fn views(&self) -> Vec<ViewRef> {
        self.views.borrow().values().cloned().collect()
    }

    fn view(&self, id: ViewId) -> Option<ViewRef> {
        self.views.borrow().get(&id).cloned()
    }

    fn open_image_codec(&self, bytes: Arc<[u8]>) -> ImageCodecFuture {
        let loader = self.image_loader.borrow().clone();
        Box::pin(async move {
            if bytes.is_empty() {
                return Err(ImageDecodeError::Empty);
            }
            let loader = loader.ok_or(ImageDecodeError::NoDecoder)?;
            let codec = loader
                .open(
                    bytes,
                    valo_codec::DecodeOptions {
                        // Pictures are routinely drawn smaller than they decode, and mip
                        // levels are the difference between a smooth downscale and a
                        // shimmering one.
                        mipmaps: true,
                        ..Default::default()
                    },
                )
                .await
                .map_err(as_decode_error)?;
            Ok(Box::new(WinitImageCodec { codec }) as Box<dyn ImageCodec>)
        })
    }

    fn request_view_focus_change(
        &self,
        view_id: ViewId,
        state: ViewFocusState,
        direction: ViewFocusDirection,
    ) {
        self.focus_requests.borrow_mut().push(ViewFocusEvent {
            view_id,
            state,
            direction,
        });
        self.wake_event_loop();
    }

    fn implicit_view(&self) -> Option<ViewRef> {
        self.implicit_view.and_then(|id| self.view(id))
    }

    /// The OS font database, scanned on request; the shell asks once at start-up.
    fn font_source(&self) -> Option<Box<dyn FontSource>> {
        Some(Box::new(SystemFontSource::platform()))
    }

    fn windowing_owner(&self) -> Option<Rc<dyn WindowingOwner>> {
        Some(Rc::clone(&self.windowing) as Rc<dyn WindowingOwner>)
    }

    fn import_pixels(&self, pixels: valo::PixelBuffer) -> Option<valo::Image> {
        self.image_context()?.upload_pixels(pixels, false).ok()
    }

    /// The system pasteboard, where the host can reach one.
    fn clipboard(&self) -> Option<&dyn Clipboard> {
        Some(self)
    }

    fn popup_menus(&self) -> Option<&dyn PopupMenus> {
        Some(self)
    }

    fn mouse_cursor(&self) -> Option<&dyn MouseCursor> {
        Some(self)
    }
}

impl Clipboard for WinitPlatform {
    fn set_text(&self, text: &str) {
        self.with_clipboard(|clipboard| {
            let _ = clipboard.set_text(text);
        });
    }

    fn text(&self) -> Option<String> {
        self.with_clipboard(|clipboard| clipboard.get_text().ok())
            .flatten()
    }

    fn has_strings(&self) -> bool {
        self.text().is_some()
    }
}

impl PopupMenus for WinitPlatform {
    fn show(&self, entries: &[PopupMenuEntry]) -> Option<usize> {
        let chosen = os::popup_menu(entries);
        // The menu ran its own event loop and kept the release of the button that opened
        // it; the next pass reconciles the buttons.
        let _ = self.proxy.send_event(HostEvent::MenuClosed);
        chosen
    }
}

impl MouseCursor for WinitPlatform {
    /// One mouse: the device is not needed to pick the window; the window the pointer was
    /// last seen in shows the cursor.
    fn activate_system_cursor(&self, _device: i64, kind: SystemMouseCursorKind) {
        self.cursor_request.set(Some(kind));
        self.wake_event_loop();
    }
}

/// The host's threads as any thread reaches them: a wake is an event through winit's
/// proxy, which the loop keeps as its earliest due wake and serves when the time comes; work
/// goes to the system's queue, or a thread where the system has none.
struct WinitDispatcher(EventLoopProxy<HostEvent>);

impl Dispatcher for WinitDispatcher {
    fn wake_at(&self, timer_wakeup: Option<Instant>) {
        let _ = self.0.send_event(HostEvent::TimerWakeup(timer_wakeup));
    }

    fn wake_now(&self) {
        let _ = self.0.send_event(HostEvent::WakeNow);
    }

    fn dispatch(&self, work: Box<dyn FnOnce() + Send>) {
        os::run_off_main(work);
    }
}

/// Adapts Valo's drawable frames to the framework's codec contract.
struct WinitImageCodec {
    codec: valo_codec::Codec,
}

impl ImageCodec for WinitImageCodec {
    fn frame_count(&self) -> u32 {
        self.codec.info().frame_count
    }

    fn repetition(&self) -> ImageRepetition {
        match self.codec.info().repetition {
            valo_codec::Repetition::Once => ImageRepetition::Once,
            valo_codec::Repetition::Times(times) => ImageRepetition::Times(times),
            valo_codec::Repetition::Forever => ImageRepetition::Forever,
        }
    }

    fn next_frame(&mut self) -> ImageFrameFuture<'_> {
        let frame = self.codec.next_frame();
        Box::pin(async move {
            let frame = frame.await.map_err(as_decode_error)?;
            Ok(ImageFrame {
                image: frame.image,
                duration: frame.duration,
            })
        })
    }
}

/// Preserves decoder diagnostics at the framework's existing error boundary.
fn as_decode_error(error: valo_codec::DecodeError) -> ImageDecodeError {
    match error {
        valo_codec::DecodeError::NoDecoder => ImageDecodeError::NoDecoder,
        valo_codec::DecodeError::Unsupported(_) => ImageDecodeError::UnknownFormat,
        valo_codec::DecodeError::InvalidData(reason) => ImageDecodeError::Damaged(reason),
        other => ImageDecodeError::Failed(other.to_string()),
    }
}

pub(crate) fn brightness_of(theme: winit::window::Theme) -> Brightness {
    match theme {
        winit::window::Theme::Light => Brightness::Light,
        winit::window::Theme::Dark => Brightness::Dark,
    }
}
