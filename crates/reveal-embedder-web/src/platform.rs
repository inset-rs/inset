//! `Platform` and `View` for one canvas.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use reveal_embedder::{
    Brightness, Picture, Platform, SystemMouseCursorKind, TargetPlatform, View, ViewId,
    ViewMetrics, ViewRef,
};
use web_sys::HtmlCanvasElement;
use web_time::Instant;

use crate::gpu::Gpu;

pub const IMPLICIT_VIEW: ViewId = ViewId(0);

pub(crate) type OnSchedule = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

pub struct WebPlatform {
    pub canvas: HtmlCanvasElement,
    pub origin: Instant,
    frame_requested: Rc<Cell<bool>>,
    fonts_changed: Rc<Cell<bool>>,
    deadline: Cell<Option<Instant>>,
    view: RefCell<Option<ViewRef>>,
    brightness: Cell<Brightness>,
    /// Set by the host so `request_frame` / `wake_at` can ask for a turn.
    /// Shared with the font source, which is created before the host installs the callback.
    on_schedule: OnSchedule,
}

impl WebPlatform {
    pub fn new(canvas: HtmlCanvasElement, brightness: Brightness) -> WebPlatform {
        WebPlatform {
            canvas,
            origin: Instant::now(),
            frame_requested: Rc::new(Cell::new(false)),
            fonts_changed: Rc::new(Cell::new(false)),
            deadline: Cell::new(None),
            view: RefCell::new(None),
            brightness: Cell::new(brightness),
            on_schedule: Rc::new(RefCell::new(None)),
        }
    }

    pub fn set_view(&self, view: ViewRef) {
        *self.view.borrow_mut() = Some(view);
    }

    pub fn set_on_schedule(&self, callback: Rc<dyn Fn()>) {
        *self.on_schedule.borrow_mut() = Some(callback);
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

    pub fn next_deadline(&self) -> Option<Instant> {
        self.deadline.get()
    }

    pub fn take_due_wake(&self, now: Instant) -> bool {
        let Some(deadline) = self.deadline.get() else {
            return false;
        };
        if now < deadline {
            return false;
        }
        self.deadline.set(None);
        true
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
}

impl Platform for WebPlatform {
    fn target_platform(&self) -> TargetPlatform {
        target_platform_from_ua()
    }

    fn platform_brightness(&self) -> Brightness {
        self.brightness.get()
    }

    fn request_frame(&self) {
        if !self.frame_requested.replace(true) {
            self.schedule();
        }
    }

    fn font_source(&self) -> Option<Box<dyn reveal_embedder::FontSource>> {
        Some(Box::new(crate::fonts::WebFontSource::new(
            Rc::clone(&self.fonts_changed),
            Rc::clone(&self.frame_requested),
            Rc::clone(&self.on_schedule),
        )))
    }

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn wake_at(&self, deadline: Instant) {
        self.deadline.set(Some(deadline));
        self.schedule();
    }

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

    fn implicit_view(&self) -> Option<ViewRef> {
        self.view.borrow().clone()
    }

    fn activate_system_cursor(&self, _device: i64, kind: SystemMouseCursorKind) {
        let css = cursor_css(kind);
        let _ = self.canvas.style().set_property("cursor", css);
    }

    fn clipboard_set_data(&self, text: &str) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let clipboard = window.navigator().clipboard();
        let _ = clipboard.write_text(text);
    }
}

pub struct WebView {
    metrics: Cell<ViewMetrics>,
    gpu: RefCell<Gpu>,
}

impl WebView {
    pub fn new(metrics: ViewMetrics, gpu: Gpu) -> WebView {
        WebView {
            metrics: Cell::new(metrics),
            gpu: RefCell::new(gpu),
        }
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
        IMPLICIT_VIEW
    }

    fn metrics(&self) -> ViewMetrics {
        self.metrics.get()
    }

    fn present(&self, picture: &Picture) {
        self.gpu.borrow_mut().present(picture);
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
