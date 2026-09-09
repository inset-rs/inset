use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reveal_embedder::{
    Brightness, EmbedderClient, FontSource, Frame, KeyData, KeyEventDeviceType, KeyEventType,
    Matrix4, Offset, Picture, Platform, PlatformRef, PointerChange, PointerData, PointerDataPacket,
    PointerDeviceKind, PointerSignalKind, Rect, Size, SystemFontSource, SystemMouseCursorKind,
    TargetPlatform, TextEditingValue, TextInputConfiguration, View, ViewConstraints,
    ViewFocusDirection, ViewFocusEvent, ViewFocusState, ViewId, ViewMetrics, ViewPadding, ViewRef,
    transform3,
};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, Ime, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::ModifiersState;
use winit::window::{CursorIcon, ImePurpose, Window, WindowId};

use crate::ime::{ImeOutcome, apply_ime};

use crate::gpu::Gpu;
use crate::keys;
use crate::text_input::{ActiveTextInput, TextInputKeyEffect};
use crate::{ImplicitViewConfig, WinitEmbedder};

const IMPLICIT_VIEW: ViewId = ViewId(0);

/// The long-lived host capability held by `App`.
///
/// Requests only write state and wake the event loop; they never re-enter the
/// client synchronously.
struct WinitPlatform {
    deadline: Cell<Option<Instant>>,
    frame_requested: Cell<bool>,
    views: RefCell<HashMap<ViewId, ViewRef>>,
    implicit_view: Option<ViewId>,
    proxy: EventLoopProxy<()>,
    origin: Instant,
    brightness: Cell<Brightness>,
    /// The latest system cursor request, applied by the event loop.
    cursor_request: Cell<Option<SystemMouseCursorKind>>,
    /// Requests from the framework, drained by the native event loop.
    focus_requests: RefCell<Vec<ViewFocusEvent>>,
    /// The OS pasteboard; created on first use so a missing clipboard host is not fatal.
    clipboard: RefCell<Option<arboard::Clipboard>>,
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
            brightness: Cell::new(Brightness::Light),
            cursor_request: Cell::new(None),
            focus_requests: RefCell::new(Vec::new()),
            clipboard: RefCell::new(None),
        }
    }

    fn with_clipboard<T>(&self, f: impl FnOnce(&mut arboard::Clipboard) -> T) -> Option<T> {
        let mut slot = self.clipboard.borrow_mut();
        if slot.is_none() {
            *slot = arboard::Clipboard::new().ok();
        }
        slot.as_mut().map(f)
    }

    fn wake_event_loop(&self) {
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
    /// The host this binary was built for — the platform adapter is where
    /// host detection belongs, exactly as Flutter's `defaultTargetPlatform`
    /// bottoms out in `dart:io`'s `Platform.isAndroid` chain.
    fn target_platform(&self) -> TargetPlatform {
        if cfg!(target_os = "android") {
            TargetPlatform::Android
        } else if cfg!(target_os = "ios") {
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

    fn wake_at(&self, deadline: Instant) {
        self.deadline.set(Some(deadline));
        self.wake_event_loop();
    }

    fn views(&self) -> Vec<ViewRef> {
        self.views.borrow().values().cloned().collect()
    }

    fn view(&self, id: ViewId) -> Option<ViewRef> {
        self.views.borrow().get(&id).cloned()
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

    /// One mouse: the device is not needed to pick the window; the window the pointer was
    /// last seen in shows the cursor.
    fn activate_system_cursor(&self, _device: i64, kind: SystemMouseCursorKind) {
        self.cursor_request.set(Some(kind));
        self.wake_event_loop();
    }

    fn clipboard_set_data(&self, text: &str) {
        let _ = self.with_clipboard(|clipboard| clipboard.set_text(text));
    }

    fn clipboard_get_data(&self) -> Option<String> {
        self.with_clipboard(|clipboard| clipboard.get_text().ok())
            .flatten()
    }

    fn clipboard_has_strings(&self) -> bool {
        self.clipboard_get_data().is_some()
    }
}

/// Stable view identity, current winit geometry, and the valo present line.
struct WinitView {
    id: ViewId,
    metrics: Cell<ViewMetrics>,
    surface: Arc<Mutex<WinitSurface>>,
    /// The window this view draws into, for retrying a present the surface refused.
    window: Arc<Window>,
    editing_state: RefCell<TextEditingValue>,
    /// Set from `start_text_input` to `stop_text_input`; an unhandled key types into it.
    text_input: Cell<Option<ActiveTextInput>>,
    transform: RefCell<Option<Matrix4>>,
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
            // The swapchain has no texture yet (a window just shown, a surface being
            // reconfigured): the scene stays retained, so a redraw presents it next vsync,
            // as Flutter's rasterizer retries a frame it could not draw.
            self.window.request_redraw();
            return;
        };
        state
            .context
            .render(picture, &surface_frame.target(Some(valo::Color::WHITE)));
        state.context.present(surface_frame);
    }

    fn start_text_input(&self, configuration: &TextInputConfiguration) {
        self.text_input
            .set(Some(ActiveTextInput::new(configuration)));
        self.window.set_ime_allowed(true);
        self.window.set_ime_purpose(if configuration.obscure_text {
            ImePurpose::Password
        } else {
            ImePurpose::Normal
        });
    }

    fn stop_text_input(&self) {
        self.text_input.set(None);
        self.window.set_ime_allowed(false);
    }

    fn set_text_input_editing_state(&self, value: &TextEditingValue) {
        *self.editing_state.borrow_mut() = value.clone();
    }

    fn set_text_input_composing_rect(&self, rect: Rect) {
        self.set_ime_area(rect);
    }

    fn set_text_input_caret_rect(&self, rect: Rect) {
        self.set_ime_area(rect);
    }

    fn set_text_input_client_geometry(&self, _size: Size, transform: &Matrix4) {
        *self.transform.borrow_mut() = Some(*transform);
    }
}

impl WinitView {
    fn set_ime_area(&self, rect: Rect) {
        let origin = match *self.transform.borrow() {
            Some(transform) => transform3(transform, Offset::new(rect.left, rect.top)),
            None => Offset::new(rect.left, rect.top),
        };
        let scale = self.metrics.get().device_pixel_ratio;
        self.window.set_ime_cursor_area(
            PhysicalPosition::new(origin.dx() * scale, origin.dy() * scale),
            PhysicalSize::new(rect.width() * scale, rect.height() * scale),
        );
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
    /// The window the mouse pointer was last seen in; system cursor requests go there.
    pointer_window: Option<WindowId>,
    started: bool,
    cursor: [f64; 2],
    last_cursor: [f64; 2],
    /// Flutter `PointerData.buttons` for the mouse.
    mouse_buttons: i64,
    modifiers: ModifiersState,
    pointer_id: i64,
    embedder_id: i64,
    /// The logical key each held physical key went down with, keyed by USB HID usage.
    pressing_records: HashMap<u64, u64>,
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
        pointer_window: None,
        started: false,
        cursor: [0.0, 0.0],
        last_cursor: [0.0, 0.0],
        mouse_buttons: 0,
        modifiers: ModifiersState::empty(),
        pointer_id: 0,
        embedder_id: 0,
        pressing_records: HashMap::new(),
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

fn brightness_of(theme: winit::window::Theme) -> Brightness {
    match theme {
        winit::window::Theme::Light => Brightness::Light,
        winit::window::Theme::Dark => Brightness::Dark,
    }
}

/// Flutter `kFlutterPointerButtonMousePrimary` (`embedder.h`).
const PRIMARY_MOUSE_BUTTON: i64 = 1 << 0;
/// Flutter `kFlutterPointerButtonMouseSecondary`.
const SECONDARY_MOUSE_BUTTON: i64 = 1 << 1;
/// Flutter `kFlutterPointerButtonMouseMiddle`.
const MIDDLE_MOUSE_BUTTON: i64 = 1 << 2;
/// Flutter `kFlutterPointerButtonMouseBack`.
const BACK_MOUSE_BUTTON: i64 = 1 << 3;
/// Flutter `kFlutterPointerButtonMouseForward`.
const FORWARD_MOUSE_BUTTON: i64 = 1 << 4;

/// Flutter engine mouse-button bits for a winit button.
///
/// Named buttons follow `kFlutterPointerButtonMouse*` in `embedder.h`. Extra buttons use
/// `1 << n`, matching the macOS embedder's `otherMouseDown:` (`1 << event.buttonNumber`).
fn flutter_mouse_button(button: MouseButton) -> Option<i64> {
    match button {
        MouseButton::Left => Some(PRIMARY_MOUSE_BUTTON),
        MouseButton::Right => Some(SECONDARY_MOUSE_BUTTON),
        MouseButton::Middle => Some(MIDDLE_MOUSE_BUTTON),
        MouseButton::Back => Some(BACK_MOUSE_BUTTON),
        MouseButton::Forward => Some(FORWARD_MOUSE_BUTTON),
        MouseButton::Other(n) => {
            let shift = u32::from(n);
            (shift < i64::BITS).then_some(1_i64 << shift)
        }
    }
}

/// Down when the first button goes down, Move while any remain, Up when the last is released.
/// Flutter's macOS `dispatchMouseEvent:` uses the same three-way split.
fn pointer_change_for_buttons(previous: i64, next: i64) -> Option<PointerChange> {
    if previous == next {
        return None;
    }
    Some(if next == 0 {
        PointerChange::Up
    } else if previous == 0 {
        PointerChange::Down
    } else {
        PointerChange::Move
    })
}

/// The winit icon for a system cursor kind; `None` hides the cursor
/// ([`SystemMouseCursorKind::None`]). Kinds winit lacks fall back to the default arrow, as
/// the Flutter engine falls back to `basic`.
fn cursor_icon_of(kind: SystemMouseCursorKind) -> Option<CursorIcon> {
    let icon = match kind {
        SystemMouseCursorKind::None => return None,
        SystemMouseCursorKind::Basic | SystemMouseCursorKind::Disappearing => CursorIcon::Default,
        SystemMouseCursorKind::Click => CursorIcon::Pointer,
        SystemMouseCursorKind::Forbidden => CursorIcon::NotAllowed,
        SystemMouseCursorKind::Wait => CursorIcon::Wait,
        SystemMouseCursorKind::Progress => CursorIcon::Progress,
        SystemMouseCursorKind::ContextMenu => CursorIcon::ContextMenu,
        SystemMouseCursorKind::Help => CursorIcon::Help,
        SystemMouseCursorKind::Text => CursorIcon::Text,
        SystemMouseCursorKind::VerticalText => CursorIcon::VerticalText,
        SystemMouseCursorKind::Cell => CursorIcon::Cell,
        SystemMouseCursorKind::Precise => CursorIcon::Crosshair,
        SystemMouseCursorKind::Move => CursorIcon::Move,
        SystemMouseCursorKind::Grab => CursorIcon::Grab,
        SystemMouseCursorKind::Grabbing => CursorIcon::Grabbing,
        SystemMouseCursorKind::NoDrop => CursorIcon::NoDrop,
        SystemMouseCursorKind::Alias => CursorIcon::Alias,
        SystemMouseCursorKind::Copy => CursorIcon::Copy,
        SystemMouseCursorKind::AllScroll => CursorIcon::AllScroll,
        SystemMouseCursorKind::ResizeLeftRight => CursorIcon::EwResize,
        SystemMouseCursorKind::ResizeUpDown => CursorIcon::NsResize,
        SystemMouseCursorKind::ResizeUpLeftDownRight => CursorIcon::NwseResize,
        SystemMouseCursorKind::ResizeUpRightDownLeft => CursorIcon::NeswResize,
        SystemMouseCursorKind::ResizeUp => CursorIcon::NResize,
        SystemMouseCursorKind::ResizeDown => CursorIcon::SResize,
        SystemMouseCursorKind::ResizeLeft => CursorIcon::WResize,
        SystemMouseCursorKind::ResizeRight => CursorIcon::EResize,
        SystemMouseCursorKind::ResizeUpLeft => CursorIcon::NwResize,
        SystemMouseCursorKind::ResizeUpRight => CursorIcon::NeResize,
        SystemMouseCursorKind::ResizeDownLeft => CursorIcon::SwResize,
        SystemMouseCursorKind::ResizeDownRight => CursorIcon::SeResize,
        SystemMouseCursorKind::ResizeColumn => CursorIcon::ColResize,
        SystemMouseCursorKind::ResizeRow => CursorIcon::RowResize,
        SystemMouseCursorKind::ZoomIn => CursorIcon::ZoomIn,
        SystemMouseCursorKind::ZoomOut => CursorIcon::ZoomOut,
    };
    Some(icon)
}

/// Line-based wheels are not pixels. 40 logical px/line is Chromium's
/// convention (shaft-rs-next); convert to physical by the view scale so
/// [`PointerData::scroll_delta_x`] / [`PointerData::scroll_delta_y`] stay
/// physical, matching dart:ui. winit's up-positive deltas are sign-flipped
/// to Flutter's content-forward sign.
fn wheel_to_physical(delta: MouseScrollDelta, scale: f64) -> [f64; 2] {
    const LOGICAL_PIXELS_PER_LINE: f64 = 40.0;
    match delta {
        MouseScrollDelta::LineDelta(dx, dy) => [
            -f64::from(dx) * LOGICAL_PIXELS_PER_LINE * scale,
            -f64::from(dy) * LOGICAL_PIXELS_PER_LINE * scale,
        ],
        MouseScrollDelta::PixelDelta(physical) => [-physical.x, -physical.y],
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

    fn apply_cursor_request(&mut self) {
        let Some(kind) = self.platform.cursor_request.take() else {
            return;
        };
        let Some(window) = self
            .pointer_window
            .and_then(|id| self.views.get(&id).map(|view| &view.window))
        else {
            return;
        };
        match cursor_icon_of(kind) {
            Some(icon) => {
                window.set_cursor(icon);
                window.set_cursor_visible(true);
            }
            None => window.set_cursor_visible(false),
        }
    }

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
        #[cfg(target_os = "macos")]
        let surface = {
            let mut surface = surface;
            surface.set_presents_with_transaction(true);
            surface
        };
        let context = valo::Context::new(gpu.device, gpu.queue);
        let view = Rc::new(WinitView {
            id: IMPLICIT_VIEW,
            metrics: Cell::new(window_metrics(&window)),
            surface: Arc::new(Mutex::new(WinitSurface { surface, context })),
            window: Arc::clone(&window),
            editing_state: RefCell::new(TextEditingValue::EMPTY),
            text_input: Cell::new(None),
            transform: RefCell::new(None),
        });
        if let Some(theme) = window.theme() {
            self.platform.brightness.set(brightness_of(theme));
        }
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

    fn send_pointer(&mut self, window_id: WindowId, change: PointerChange) {
        let Some(packet) = self.pointer_packet(window_id, change, None) else {
            return;
        };
        if let Some(client) = &mut self.client {
            client.pointer_data_packet(packet);
        }
    }

    fn apply_mouse_button(&mut self, window_id: WindowId, button: MouseButton, pressed: bool) {
        let Some(bit) = flutter_mouse_button(button) else {
            return;
        };
        let previous = self.mouse_buttons;
        if pressed {
            self.mouse_buttons |= bit;
        } else {
            self.mouse_buttons &= !bit;
        }
        let Some(change) = pointer_change_for_buttons(previous, self.mouse_buttons) else {
            return;
        };
        self.send_pointer(window_id, change);
    }

    fn send_scroll(&mut self, window_id: WindowId, delta: MouseScrollDelta) {
        let scale = self
            .views
            .get(&window_id)
            .map(|hosted| hosted.view.metrics().device_pixel_ratio)
            .unwrap_or(1.0);
        let [scroll_delta_x, scroll_delta_y] = wheel_to_physical(delta, scale);
        let Some(packet) = self.pointer_packet(
            window_id,
            PointerChange::Hover,
            Some((scroll_delta_x, scroll_delta_y)),
        ) else {
            return;
        };
        if let Some(client) = &mut self.client {
            client.pointer_data_packet(packet);
        }
    }

    fn send_ime(&mut self, window_id: WindowId, ime: Ime) {
        let Some(hosted) = self.views.get(&window_id) else {
            return;
        };
        let view_id = hosted.view.id();
        let outcome = apply_ime(&hosted.view.editing_state.borrow(), &ime);
        match outcome {
            ImeOutcome::None => {}
            ImeOutcome::Closed => {
                if let Some(client) = &mut self.client {
                    client.text_input_closed(view_id);
                }
            }
            ImeOutcome::Value(value) => {
                *hosted.view.editing_state.borrow_mut() = value.clone();
                if let Some(client) = &mut self.client {
                    client.text_input_editing_value(view_id, value);
                }
            }
        }
    }

    fn send_key(&mut self, window_id: WindowId, event: &KeyEvent, is_synthetic: bool) {
        let Some(data) = self.key_data(event, is_synthetic) else {
            return;
        };
        let Some(client) = &mut self.client else {
            return;
        };
        // Flutter's macOS `FlutterKeyboardManager`: the text input plugin sees a key only
        // after the framework declined it.
        if !client.key_data(data) && event.state == ElementState::Pressed {
            self.type_into_text_input(window_id, event);
        }
    }

    /// `FlutterTextInputPlugin.handleKeyEvent` for the view's active text input, if any.
    fn type_into_text_input(&mut self, window_id: WindowId, event: &KeyEvent) {
        let Some(hosted) = self.views.get(&window_id) else {
            return;
        };
        let Some(text_input) = hosted.view.text_input.get() else {
            return;
        };
        let view_id = hosted.view.id();
        match text_input.key_effect(&event.logical_key, event.text.as_deref(), self.modifiers) {
            TextInputKeyEffect::None => {}
            TextInputKeyEffect::Insert(text) => self.send_ime(window_id, Ime::Commit(text)),
            TextInputKeyEffect::Enter { insert, action } => {
                if let Some(text) = insert {
                    self.send_ime(window_id, Ime::Commit(text));
                }
                if let Some(client) = &mut self.client {
                    client.text_input_action(view_id, action);
                }
            }
        }
    }

    /// A winit key event as dart:ui [`KeyData`], or `None` for a key this host
    /// cannot name in Flutter's tables.
    fn key_data(&mut self, event: &KeyEvent, is_synthetic: bool) -> Option<KeyData> {
        let physical = keys::physical_key_usage(event.physical_key)?;
        let event_type = match (event.state, event.repeat) {
            (ElementState::Pressed, false) => KeyEventType::Down,
            (ElementState::Pressed, true) => KeyEventType::Repeat,
            (ElementState::Released, _) => KeyEventType::Up,
        };
        let logical = keys::logical_key_id(&event.logical_key, event.location);
        // Flutter's embedders remember which logical key a physical key went down
        // with, so its repeats and its up report that one even when the modifiers
        // changed in between (`_pressingRecords` in the engine's `KeyboardConverter`).
        let logical = match event_type {
            KeyEventType::Down => {
                let logical = logical?;
                self.pressing_records.insert(physical, logical);
                logical
            }
            KeyEventType::Repeat => self.pressing_records.get(&physical).copied().or(logical)?,
            KeyEventType::Up => self.pressing_records.remove(&physical).or(logical)?,
        };
        let character = match event_type {
            KeyEventType::Up => None,
            KeyEventType::Down | KeyEventType::Repeat => keys::character_of(event.text.as_deref()),
        };
        Some(KeyData {
            time_stamp: self.platform.elapsed(),
            event_type,
            device_type: KeyEventDeviceType::Keyboard,
            physical,
            logical,
            character,
            synthesized: is_synthetic,
        })
    }

    fn pointer_packet(
        &mut self,
        window_id: WindowId,
        change: PointerChange,
        scroll: Option<(f64, f64)>,
    ) -> Option<PointerDataPacket> {
        let hosted = self.views.get(&window_id)?;
        let view_id = hosted.view.id();
        if change == PointerChange::Down {
            self.pointer_id += 1;
        }
        self.embedder_id += 1;
        let [x, y] = self.cursor;
        let [last_x, last_y] = self.last_cursor;
        let buttons = self.mouse_buttons;
        let pointer_identifier = if change == PointerChange::Hover && buttons == 0 {
            0
        } else {
            self.pointer_id
        };
        let (signal_kind, scroll_delta_x, scroll_delta_y) = match scroll {
            Some((dx, dy)) => (Some(PointerSignalKind::Scroll), dx, dy),
            None => (None, 0.0, 0.0),
        };
        let data = PointerData {
            view_id,
            embedder_id: self.embedder_id,
            time_stamp: self.platform.elapsed(),
            change,
            kind: PointerDeviceKind::Mouse,
            signal_kind,
            pointer_identifier,
            physical_x: x,
            physical_y: y,
            physical_delta_x: x - last_x,
            physical_delta_y: y - last_y,
            buttons,
            pressure: 1.0,
            pressure_min: 1.0,
            pressure_max: 1.0,
            scroll_delta_x,
            scroll_delta_y,
            ..PointerData::default()
        };
        self.last_cursor = self.cursor;
        Some(PointerDataPacket::new(vec![data]))
    }
}

impl<C: EmbedderClient> ApplicationHandler for WinitApp<C> {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        if matches!(cause, winit::event::StartCause::ResumeTimeReached { .. }) {
            self.platform.deadline.set(None);
            if let Some(client) = &mut self.client {
                client.wake(self.platform.elapsed());
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
        self.apply_cursor_request();
        self.apply_focus_requests();
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
            WindowEvent::Focused(focused) => {
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
                        // Present the resized layout before AppKit commits the
                        // window geometry, rather than stretching the old frame.
                        #[cfg(target_os = "macos")]
                        if size.width > 0 && size.height > 0 {
                            self.platform.frame_requested.set(false);
                            client.frame(Frame {
                                elapsed: self.platform.elapsed(),
                            });
                        }
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
            WindowEvent::ThemeChanged(theme) => {
                self.platform.brightness.set(brightness_of(theme));
                if let Some(client) = &mut self.client {
                    client.platform_brightness_changed();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer_window = Some(id);
                self.cursor = [position.x, position.y];
                self.send_pointer(
                    id,
                    if self.mouse_buttons != 0 {
                        PointerChange::Move
                    } else {
                        PointerChange::Hover
                    },
                );
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                self.send_key(id, &event, is_synthetic);
            }
            WindowEvent::Ime(ime) => {
                self.send_ime(id, ime);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.send_scroll(id, delta);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.apply_mouse_button(id, button, state == ElementState::Pressed);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use winit::dpi::PhysicalPosition;
    use winit::event::MouseScrollDelta;

    use reveal_embedder::{PointerChange, SystemMouseCursorKind};
    use winit::event::MouseButton;
    use winit::window::CursorIcon;

    use super::{
        BACK_MOUSE_BUTTON, FORWARD_MOUSE_BUTTON, MIDDLE_MOUSE_BUTTON, PRIMARY_MOUSE_BUTTON,
        SECONDARY_MOUSE_BUTTON, cursor_icon_of, flutter_mouse_button, pointer_change_for_buttons,
        wheel_to_physical,
    };

    #[test]
    fn cursor_kinds_map_to_winit_icons() {
        assert_eq!(
            cursor_icon_of(SystemMouseCursorKind::Click),
            Some(CursorIcon::Pointer)
        );
        assert_eq!(
            cursor_icon_of(SystemMouseCursorKind::Basic),
            Some(CursorIcon::Default)
        );
        assert_eq!(
            cursor_icon_of(SystemMouseCursorKind::None),
            None,
            "none hides the cursor"
        );
        assert_eq!(
            cursor_icon_of(SystemMouseCursorKind::Disappearing),
            Some(CursorIcon::Default),
            "a kind winit lacks falls back to the arrow"
        );
    }

    #[test]
    fn line_deltas_scale_to_physical_pixels_with_flutter_sign() {
        assert_eq!(
            wheel_to_physical(MouseScrollDelta::LineDelta(0.0, 1.0), 2.0),
            [0.0, -80.0]
        );
        assert_eq!(
            wheel_to_physical(MouseScrollDelta::LineDelta(0.0, -3.0), 1.0),
            [0.0, 120.0]
        );
    }

    #[test]
    fn pixel_deltas_keep_physical_pixels_and_flip_sign() {
        assert_eq!(
            wheel_to_physical(
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -100.0)),
                2.0
            ),
            [0.0, 100.0]
        );
    }

    #[test]
    fn winit_mouse_buttons_map_to_flutter_bits() {
        assert_eq!(
            flutter_mouse_button(MouseButton::Left),
            Some(PRIMARY_MOUSE_BUTTON)
        );
        assert_eq!(
            flutter_mouse_button(MouseButton::Right),
            Some(SECONDARY_MOUSE_BUTTON)
        );
        assert_eq!(
            flutter_mouse_button(MouseButton::Middle),
            Some(MIDDLE_MOUSE_BUTTON)
        );
        assert_eq!(
            flutter_mouse_button(MouseButton::Back),
            Some(BACK_MOUSE_BUTTON)
        );
        assert_eq!(
            flutter_mouse_button(MouseButton::Forward),
            Some(FORWARD_MOUSE_BUTTON)
        );
        assert_eq!(flutter_mouse_button(MouseButton::Other(5)), Some(1 << 5));
    }

    #[test]
    fn mouse_button_chords_are_down_move_up() {
        assert_eq!(
            pointer_change_for_buttons(0, SECONDARY_MOUSE_BUTTON),
            Some(PointerChange::Down)
        );
        assert_eq!(
            pointer_change_for_buttons(SECONDARY_MOUSE_BUTTON, 0),
            Some(PointerChange::Up)
        );
        assert_eq!(
            pointer_change_for_buttons(
                PRIMARY_MOUSE_BUTTON,
                PRIMARY_MOUSE_BUTTON | SECONDARY_MOUSE_BUTTON
            ),
            Some(PointerChange::Move)
        );
        assert_eq!(
            pointer_change_for_buttons(PRIMARY_MOUSE_BUTTON, PRIMARY_MOUSE_BUTTON),
            None
        );
    }
}
