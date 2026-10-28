//! `Platform` and `View` for one canvas.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use inset_embedder::{
    Brightness, Clipboard, Dispatcher, FileDialogs, ImageCodecFuture, Matrix4, MouseCursor, Offset,
    Platform, Rect, Screen, Screens, Size, SystemMouseCursorKind, TargetPlatform, TextEditingValue,
    TextInputConfiguration, TextInputHost, View, ViewId, ViewMetrics, ViewRef,
};
use web_sys::HtmlCanvasElement;
use web_time::Instant;

use crate::clipboard::BrowserClipboard;
use crate::dispatcher;
use crate::file_dialogs::BrowserFileDialogs;
use crate::font_fallback::ROBOTO;
use crate::gpu::Gpu;
use crate::text_input::{TextInputListener, WebTextInput};

/// The page's canvas's view, the one view the page gives the app.
pub const CANVAS_VIEW: ViewId = ViewId(0);

pub(crate) type OnSchedule = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

pub struct WebPlatform {
    pub canvas: HtmlCanvasElement,
    pub origin: Instant,
    frame_requested: Rc<Cell<bool>>,
    fonts_changed: Rc<Cell<bool>>,
    /// When to wake for the earliest waiting timer, or `None` when no timer is waiting.
    timer_wakeup: Rc<Cell<Option<Instant>>>,
    /// A ready task asking to be woken. Cleared by the next wake, and kept separate from
    /// the timer wakeup, which belongs to a timer that has not run yet.
    wake_now_requested: Rc<Cell<bool>>,
    view: RefCell<Option<ViewRef>>,
    brightness: Cell<Brightness>,
    /// The page's screen, as the app was last told it (`screen.rs`).
    screen: RefCell<Option<Screen>>,
    /// Set by the host so `request_frame` and `wake_at` can arrange a wake.
    /// Shared with the font source, which is created before the host installs the callback.
    on_schedule: OnSchedule,
    images: RefCell<Option<valo::ImageContext>>,
}

impl WebPlatform {
    pub fn new(
        canvas: HtmlCanvasElement,
        brightness: Brightness,
        screen: Option<Screen>,
    ) -> WebPlatform {
        let platform = WebPlatform {
            canvas,
            origin: Instant::now(),
            frame_requested: Rc::new(Cell::new(false)),
            fonts_changed: Rc::new(Cell::new(false)),
            timer_wakeup: Rc::new(Cell::new(None)),
            wake_now_requested: Rc::new(Cell::new(false)),
            view: RefCell::new(None),
            brightness: Cell::new(brightness),
            screen: RefCell::new(screen),
            on_schedule: Rc::new(RefCell::new(None)),
            images: RefCell::new(None),
        };
        dispatcher::install(
            Rc::clone(&platform.timer_wakeup),
            Rc::clone(&platform.wake_now_requested),
            Rc::clone(&platform.on_schedule),
        );
        platform
    }

    pub fn set_view(&self, view: ViewRef) {
        *self.view.borrow_mut() = Some(view);
    }

    pub fn set_on_schedule(&self, callback: Rc<dyn Fn()>) {
        *self.on_schedule.borrow_mut() = Some(callback);
    }

    pub fn set_images(&self, images: valo::ImageContext) {
        *self.images.borrow_mut() = Some(images);
    }

    pub fn take_frame_request(&self) -> bool {
        self.frame_requested.replace(false)
    }

    pub fn take_fonts_changed(&self) -> bool {
        self.fonts_changed.replace(false)
    }

    pub fn has_frame_request(&self) -> bool {
        self.frame_requested.get()
    }

    pub fn next_timer_wakeup(&self) -> Option<Instant> {
        self.timer_wakeup.get()
    }

    /// Whether a ready task is waiting to be woken, without clearing the request.
    pub fn wake_now_requested(&self) -> bool {
        self.wake_now_requested.get()
    }

    /// Whether the app should be woken now: a ready task asked for it, the timer wakeup has
    /// arrived, or both. A timer wakeup that has arrived is cleared here, and the framework
    /// sets the next one before the wake ends.
    pub fn should_wake(&self, now: Instant) -> bool {
        let asked = self.wake_now_requested.replace(false);
        let timer_came = self.timer_wakeup.get().is_some_and(|wakeup| now >= wakeup);
        if timer_came {
            self.timer_wakeup.set(None);
        }
        asked || timer_came
    }

    pub fn elapsed(&self) -> Duration {
        self.now().saturating_duration_since(self.origin)
    }

    fn schedule(&self) {
        if let Some(callback) = self.on_schedule.borrow().as_ref() {
            callback();
        }
    }

    pub fn set_brightness(&self, brightness: Brightness) {
        self.brightness.set(brightness);
    }

    /// Takes the page's screen as it is now; answers whether it differs from what the app was
    /// told, which the app is then to hear.
    pub fn set_screen(&self, screen: Option<Screen>) -> bool {
        if *self.screen.borrow() == screen {
            return false;
        }
        *self.screen.borrow_mut() = screen;
        true
    }
}

impl Platform for WebPlatform {
    fn target_platform(&self) -> TargetPlatform {
        target_platform_from_ua()
    }

    fn platform_brightness(&self) -> Brightness {
        self.brightness.get()
    }

    /// The page's canvas is its one view: a frame of any other is no frame of the page's.
    fn request_frame(&self, view: ViewId) {
        if view == CANVAS_VIEW && !self.frame_requested.replace(true) {
            self.schedule();
        }
    }

    fn font_source(&self) -> Option<Box<dyn inset_embedder::FontSource>> {
        Some(Box::new(crate::fonts::WebFontSource::new(
            Rc::clone(&self.fonts_changed),
            Rc::clone(&self.frame_requested),
            Rc::clone(&self.on_schedule),
        )))
    }

    /// Roboto, the family this host ships, on whatever system the page runs: Flutter web's
    /// `setDefaultFontFamilies(['Roboto'])`.
    fn default_font_families(&self) -> Vec<String> {
        vec![String::from(ROBOTO)]
    }

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn dispatcher(&self) -> Arc<dyn Dispatcher> {
        Arc::new(dispatcher::WebDispatcher)
    }

    /// The page's canvas, the one view the page gives the app, once it is there.
    fn views(&self) -> Vec<ViewRef> {
        self.view.borrow().iter().cloned().collect()
    }

    fn view(&self, id: ViewId) -> Option<ViewRef> {
        self.view
            .borrow()
            .as_ref()
            .filter(|view| view.id() == id)
            .cloned()
    }

    /// The browser's asynchronous clipboard (`clipboard.rs`).
    fn clipboard(&self) -> Option<&dyn Clipboard> {
        Some(&BrowserClipboard)
    }

    /// The browser's pickers (`file_dialogs.rs`).
    fn file_dialogs(&self) -> Option<&dyn FileDialogs> {
        Some(&BrowserFileDialogs)
    }

    /// The page's one screen (`screen.rs`).
    fn screens(&self) -> Option<&dyn Screens> {
        Some(self)
    }

    fn mouse_cursor(&self) -> Option<&dyn MouseCursor> {
        Some(self)
    }

    fn open_image_codec(&self, bytes: std::sync::Arc<[u8]>) -> ImageCodecFuture {
        let Some(images) = self.images.borrow().clone() else {
            return Box::pin(std::future::ready(Err(
                inset_embedder::ImageDecodeError::NoDecoder,
            )));
        };
        crate::images::open_codec(
            bytes,
            images,
            Rc::clone(&self.frame_requested),
            Rc::clone(&self.on_schedule),
        )
    }
}

/// The page's screen, and no pointer: the page sees the pointer only over itself, in its own
/// coordinates.
impl Screens for WebPlatform {
    fn screens(&self) -> Vec<Screen> {
        self.screen.borrow().iter().cloned().collect()
    }

    fn pointer_location(&self) -> Option<Offset> {
        None
    }
}

impl MouseCursor for WebPlatform {
    /// One canvas, one cursor: the view and the device do not pick anything.
    fn activate_system_cursor(&self, _view: ViewId, _device: i64, kind: SystemMouseCursorKind) {
        let css = cursor_css(kind);
        let _ = self.canvas.style().set_property("cursor", css);
    }
}

pub struct WebView {
    metrics: Cell<ViewMetrics>,
    gpu: RefCell<Gpu>,
    text_input: RefCell<Option<WebTextInput>>,
    notify_text_input: RefCell<Option<TextInputListener>>,
}

impl WebView {
    pub fn new(metrics: ViewMetrics, gpu: Gpu) -> WebView {
        WebView {
            metrics: Cell::new(metrics),
            gpu: RefCell::new(gpu),
            text_input: RefCell::new(None),
            notify_text_input: RefCell::new(None),
        }
    }

    pub fn set_text_input_listener(&self, listener: TextInputListener) {
        *self.notify_text_input.borrow_mut() = Some(listener);
    }

    pub fn has_text_input(&self) -> bool {
        self.text_input.borrow().is_some()
    }

    /// Whether `target` is the field the view edits text in now.
    pub(crate) fn is_text_input_field(&self, target: &web_sys::EventTarget) -> bool {
        self.text_input
            .borrow()
            .as_ref()
            .is_some_and(|input| input.is_field(target))
    }

    pub fn set_metrics(&self, metrics: ViewMetrics) {
        self.metrics.set(metrics);
    }

    pub fn resize_surface(&self, size: [u32; 2]) {
        self.gpu.borrow_mut().resize(size);
    }
}

impl View for WebView {
    fn id(&self) -> ViewId {
        CANVAS_VIEW
    }

    fn metrics(&self) -> ViewMetrics {
        self.metrics.get()
    }

    fn present(&self, scene: std::sync::Arc<inset_embedder::Scene>) {
        self.gpu.borrow_mut().present(&scene.picture);
    }

    fn text_input(&self) -> Option<&dyn TextInputHost> {
        Some(self)
    }
}

impl TextInputHost for WebView {
    fn start(&self, configuration: &TextInputConfiguration) {
        if let Some(input) = self.text_input.borrow().as_ref() {
            input.apply_configuration(configuration);
            input.set_dpr(self.metrics.get().device_pixel_ratio);
            return;
        }
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let Some(notify) = self.notify_text_input.borrow().clone() else {
            return;
        };
        let input = WebTextInput::start(&document, configuration, notify);
        input.set_dpr(self.metrics.get().device_pixel_ratio);
        *self.text_input.borrow_mut() = Some(input);
    }

    fn stop(&self) {
        if let Some(input) = self.text_input.borrow_mut().take() {
            input.stop();
        }
    }

    fn set_editing_state(&self, value: &TextEditingValue) {
        if let Some(input) = self.text_input.borrow().as_ref() {
            input.set_editing_state(value);
        }
    }

    fn set_composing_rect(&self, rect: Rect) {
        self.set_caret_rect(rect);
    }

    fn set_caret_rect(&self, rect: Rect) {
        if let Some(input) = self.text_input.borrow().as_ref() {
            input.set_dpr(self.metrics.get().device_pixel_ratio);
            input.set_caret_rect(rect);
        }
    }

    fn set_client_geometry(&self, size: Size, transform: &Matrix4) {
        if let Some(input) = self.text_input.borrow().as_ref() {
            input.set_client_geometry(size, transform);
        }
    }
}

fn target_platform_from_ua() -> TargetPlatform {
    let Some(window) = web_sys::window() else {
        return TargetPlatform::Linux;
    };
    let Ok(ua) = window.navigator().user_agent() else {
        return TargetPlatform::Linux;
    };
    let ua = ua.to_ascii_lowercase();
    if ua.contains("iphone") || ua.contains("ipad") || ua.contains("ipod") {
        TargetPlatform::IOS
    } else if ua.contains("android") {
        TargetPlatform::Android
    } else if ua.contains("mac") {
        TargetPlatform::MacOS
    } else if ua.contains("win") {
        TargetPlatform::Windows
    } else {
        TargetPlatform::Linux
    }
}

fn cursor_css(kind: SystemMouseCursorKind) -> &'static str {
    match kind {
        SystemMouseCursorKind::None => "none",
        SystemMouseCursorKind::Basic | SystemMouseCursorKind::Disappearing => "default",
        SystemMouseCursorKind::Click => "pointer",
        SystemMouseCursorKind::Forbidden => "not-allowed",
        SystemMouseCursorKind::Wait => "wait",
        SystemMouseCursorKind::Progress => "progress",
        SystemMouseCursorKind::ContextMenu => "context-menu",
        SystemMouseCursorKind::Help => "help",
        SystemMouseCursorKind::Text => "text",
        SystemMouseCursorKind::VerticalText => "vertical-text",
        SystemMouseCursorKind::Cell => "cell",
        SystemMouseCursorKind::Precise => "crosshair",
        SystemMouseCursorKind::Move => "move",
        SystemMouseCursorKind::Grab => "grab",
        SystemMouseCursorKind::Grabbing => "grabbing",
        SystemMouseCursorKind::NoDrop => "no-drop",
        SystemMouseCursorKind::Alias => "alias",
        SystemMouseCursorKind::Copy => "copy",
        SystemMouseCursorKind::AllScroll => "all-scroll",
        SystemMouseCursorKind::ResizeLeftRight => "ew-resize",
        SystemMouseCursorKind::ResizeUpDown => "ns-resize",
        SystemMouseCursorKind::ResizeUpLeftDownRight => "nwse-resize",
        SystemMouseCursorKind::ResizeUpRightDownLeft => "nesw-resize",
        SystemMouseCursorKind::ResizeUp => "n-resize",
        SystemMouseCursorKind::ResizeDown => "s-resize",
        SystemMouseCursorKind::ResizeLeft => "w-resize",
        SystemMouseCursorKind::ResizeRight => "e-resize",
        SystemMouseCursorKind::ResizeUpLeft => "nw-resize",
        SystemMouseCursorKind::ResizeUpRight => "ne-resize",
        SystemMouseCursorKind::ResizeDownLeft => "sw-resize",
        SystemMouseCursorKind::ResizeDownRight => "se-resize",
        SystemMouseCursorKind::ResizeColumn => "col-resize",
        SystemMouseCursorKind::ResizeRow => "row-resize",
        SystemMouseCursorKind::ZoomIn => "zoom-in",
        SystemMouseCursorKind::ZoomOut => "zoom-out",
    }
}
