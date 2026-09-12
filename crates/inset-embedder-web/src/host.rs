//! One canvas, rAF, and the client the start closure returns.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use inset_embedder::{
    Brightness, EmbedderClient, Frame, KeyData, KeyEventType, Platform, PlatformRef, PointerChange,
    PointerData, PointerDataPacket, PointerDeviceKind, PointerSignalKind, View, ViewConstraints,
    ViewMetrics, ViewPadding, ViewRef,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use web_sys::{HtmlCanvasElement, KeyboardEvent, PointerEvent, WheelEvent, Window};
use web_time::Instant;

use crate::gpu::Gpu;
use crate::keys;
use crate::platform::{IMPLICIT_VIEW, WebPlatform, WebView};
use crate::pointer;
use crate::text_input::TextInputNotify;

type RafCallback = Closure<dyn FnMut(f64)>;
type TimeoutCallback = Closure<dyn FnMut()>;

/// Browser host: one canvas, `requestAnimationFrame`, WebGPU.
pub struct WebEmbedder {
    canvas_id: String,
}

impl Default for WebEmbedder {
    fn default() -> WebEmbedder {
        WebEmbedder {
            canvas_id: "inset".to_owned(),
        }
    }
}

impl WebEmbedder {
    pub fn canvas_id(mut self, id: impl Into<String>) -> WebEmbedder {
        self.canvas_id = id.into();
        self
    }

    /// Starts GPU setup, then `start`, then the rAF / event loop. Does not return.
    pub fn run<C: EmbedderClient + 'static>(self, start: impl FnOnce(PlatformRef) -> C + 'static) {
        console_error_panic_hook::set_once();
        wasm_bindgen_futures::spawn_local(async move {
            run_async(self.canvas_id, start).await;
        });
    }
}

struct Host<C> {
    window: Window,
    canvas: HtmlCanvasElement,
    platform: Rc<WebPlatform>,
    view: Rc<WebView>,
    client: RefCell<C>,
    raf_held: Cell<bool>,
    raf_callback: RefCell<Option<RafCallback>>,
    timeout_id: Cell<Option<i32>>,
    timeout_callback: RefCell<Option<TimeoutCallback>>,
    cursor: Cell<[f64; 2]>,
    last_cursor: Cell<[f64; 2]>,
    mouse_buttons: Cell<i64>,
    pointer_id: Cell<i64>,
    embedder_id: Cell<i64>,
    pressing: RefCell<HashMap<u64, u64>>,
}

async fn run_async<C: EmbedderClient + 'static>(
    canvas_id: String,
    start: impl FnOnce(PlatformRef) -> C,
) {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let canvas = document
        .get_element_by_id(&canvas_id)
        .unwrap_or_else(|| panic!("canvas #{canvas_id}"))
        .dyn_into::<HtmlCanvasElement>()
        .expect("element is a canvas");
    let _ = canvas.set_attribute("tabindex", "0");
    let _ = canvas.style().set_property("touch-action", "none");
    let brightness = if prefers_dark(&window) {
        Brightness::Dark
    } else {
        Brightness::Light
    };
    let (metrics, size) = fit_canvas(&window, &canvas);
    let gpu = Gpu::attach(canvas.clone(), size).await;
    let images = gpu.context.image_context();
    let platform = Rc::new(WebPlatform::new(canvas.clone(), brightness));
    platform.set_images(images);
    let view = Rc::new(WebView::new(metrics, gpu));
    platform.set_view(Rc::clone(&view) as ViewRef);
    let client = start(Rc::clone(&platform) as PlatformRef);
    let host = Rc::new(Host {
        window: window.clone(),
        canvas: canvas.clone(),
        platform: Rc::clone(&platform),
        view: Rc::clone(&view),
        client: RefCell::new(client),
        raf_held: Cell::new(false),
        raf_callback: RefCell::new(None),
        timeout_id: Cell::new(None),
        timeout_callback: RefCell::new(None),
        cursor: Cell::new([0.0, 0.0]),
        last_cursor: Cell::new([0.0, 0.0]),
        mouse_buttons: Cell::new(0),
        pointer_id: Cell::new(0),
        embedder_id: Cell::new(0),
        pressing: RefCell::new(HashMap::new()),
    });
    install_turn_callbacks(&host);
    let weak = Rc::downgrade(&host);
    platform.set_on_schedule(Rc::new(move || {
        if let Some(host) = weak.upgrade() {
            schedule_next(&host);
        }
    }));
    let weak = Rc::downgrade(&host);
    view.set_text_input_listener(Rc::new(move |event| {
        let Some(host) = weak.upgrade() else {
            return;
        };
        match event {
            TextInputNotify::Value(value) => {
                host.client
                    .borrow_mut()
                    .text_input_editing_value(IMPLICIT_VIEW, value);
            }
            TextInputNotify::Action(action) => {
                host.client
                    .borrow_mut()
                    .text_input_action(IMPLICIT_VIEW, action);
            }
        }
    }));
    bind_events(&host);
    schedule_next(&host);
}

fn install_turn_callbacks<C: EmbedderClient + 'static>(host: &Rc<Host<C>>) {
    let for_raf = Rc::clone(host);
    *host.raf_callback.borrow_mut() = Some(Closure::new(move |_time: f64| {
        for_raf.raf_held.set(false);
        on_turn(&for_raf);
    }));
    let for_timeout = Rc::clone(host);
    *host.timeout_callback.borrow_mut() = Some(Closure::new(move || {
        for_timeout.timeout_id.set(None);
        on_turn(&for_timeout);
    }));
}

fn prefers_dark(window: &Window) -> bool {
    window
        .match_media("(prefers-color-scheme: dark)")
        .ok()
        .flatten()
        .is_some_and(|query| query.matches())
}

fn fit_canvas(window: &Window, canvas: &HtmlCanvasElement) -> (ViewMetrics, [u32; 2]) {
    let dpr = window.device_pixel_ratio();
    let logical_w = f64::from(canvas.client_width()).max(1.0);
    let logical_h = f64::from(canvas.client_height()).max(1.0);
    let width = (logical_w * dpr).round().max(1.0);
    let height = (logical_h * dpr).round().max(1.0);
    canvas.set_width(width as u32);
    canvas.set_height(height as u32);
    let metrics = ViewMetrics {
        physical_size: [width, height],
        physical_constraints: ViewConstraints::tight(width, height),
        device_pixel_ratio: dpr,
        padding: ViewPadding::ZERO,
        view_padding: ViewPadding::ZERO,
        view_insets: ViewPadding::ZERO,
    };
    (metrics, [width as u32, height as u32])
}

fn on_turn<C: EmbedderClient + 'static>(host: &Rc<Host<C>>) {
    let now = Instant::now();
    if host.platform.take_due_wake(now) {
        host.client.borrow_mut().wake(host.platform.elapsed());
    }
    if host.platform.take_fonts_changed() {
        host.client.borrow_mut().system_fonts_changed();
    }
    if host.platform.take_frame_request() {
        host.client.borrow_mut().frame(Frame {
            elapsed: host.platform.elapsed(),
        });
    }
    schedule_next(host);
}

fn schedule_next<C: EmbedderClient + 'static>(host: &Rc<Host<C>>) {
    if host.platform.has_frame_request() {
        schedule_raf(host);
        return;
    }
    let Some(deadline) = host.platform.next_deadline() else {
        return;
    };
    let now = Instant::now();
    if deadline <= now {
        schedule_raf(host);
    } else {
        schedule_timeout(host, deadline.saturating_duration_since(now));
    }
}

fn schedule_raf<C: EmbedderClient + 'static>(host: &Host<C>) {
    if host.raf_held.replace(true) {
        return;
    }
    let callback = host.raf_callback.borrow();
    let Some(callback) = callback.as_ref() else {
        return;
    };
    let _ = host
        .window
        .request_animation_frame(callback.as_ref().unchecked_ref());
}

fn schedule_timeout<C: EmbedderClient + 'static>(host: &Host<C>, delay: Duration) {
    if let Some(id) = host.timeout_id.take() {
        host.window.clear_timeout_with_handle(id);
    }
    let callback = host.timeout_callback.borrow();
    let Some(callback) = callback.as_ref() else {
        return;
    };
    let ms = i32::try_from(delay.as_millis()).unwrap_or(i32::MAX);
    if let Ok(id) = host
        .window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            ms,
        )
    {
        host.timeout_id.set(Some(id));
    }
}

fn bind_events<C: EmbedderClient + 'static>(host: &Rc<Host<C>>) {
    bind_pointer(host);
    bind_wheel(host);
    bind_keyboard(host);
    bind_resize(host);
    bind_brightness(host);
}

fn bind_pointer<C: EmbedderClient + 'static>(host: &Rc<Host<C>>) {
    for (name, phase) in [
        ("pointerdown", PointerPhase::Down),
        ("pointermove", PointerPhase::Move),
        ("pointerup", PointerPhase::Up),
        ("pointercancel", PointerPhase::Cancel),
    ] {
        let listener = Rc::clone(host);
        let closure = Closure::<dyn FnMut(PointerEvent)>::new(move |event: PointerEvent| {
            event.prevent_default();
            dispatch_pointer(&listener, &event, phase);
        });
        let options = web_sys::AddEventListenerOptions::new();
        options.set_passive(false);
        let _ = host
            .canvas
            .add_event_listener_with_callback_and_add_event_listener_options(
                name,
                closure.as_ref().unchecked_ref(),
                &options,
            );
        closure.forget();
    }
}

#[derive(Clone, Copy)]
enum PointerPhase {
    Down,
    Move,
    Up,
    Cancel,
}

fn dispatch_pointer<C: EmbedderClient + 'static>(
    host: &Host<C>,
    event: &PointerEvent,
    phase: PointerPhase,
) {
    if !event.is_primary() {
        return;
    }
    if matches!(phase, PointerPhase::Down) {
        if !host.view.has_text_input() {
            let _ = host.canvas.focus();
        }
        let _ = host.canvas.set_pointer_capture(event.pointer_id());
    }
    let dpr = host.view.metrics().device_pixel_ratio;
    let x = f64::from(event.offset_x()) * dpr;
    let y = f64::from(event.offset_y()) * dpr;
    host.last_cursor.set(host.cursor.get());
    host.cursor.set([x, y]);
    let kind = if event.pointer_type() == "touch" {
        PointerDeviceKind::Touch
    } else {
        PointerDeviceKind::Mouse
    };
    let mut buttons = pointer::flutter_buttons(event.buttons());
    if kind == PointerDeviceKind::Touch && matches!(phase, PointerPhase::Down | PointerPhase::Move)
    {
        buttons = pointer::PRIMARY;
    }
    let change = match phase {
        PointerPhase::Down => {
            host.mouse_buttons.set(buttons.max(pointer::PRIMARY));
            host.pointer_id.set(host.pointer_id.get() + 1);
            PointerChange::Down
        }
        PointerPhase::Move => {
            host.mouse_buttons.set(buttons);
            if buttons == 0 && kind == PointerDeviceKind::Mouse {
                PointerChange::Hover
            } else {
                PointerChange::Move
            }
        }
        PointerPhase::Up => {
            host.mouse_buttons.set(buttons);
            PointerChange::Up
        }
        PointerPhase::Cancel => PointerChange::Cancel,
    };
    send_pointer(host, change, kind, None);
}

fn bind_wheel<C: EmbedderClient + 'static>(host: &Rc<Host<C>>) {
    let listener = Rc::clone(host);
    let closure = Closure::<dyn FnMut(WheelEvent)>::new(move |event: WheelEvent| {
        event.prevent_default();
        let (dx, dy) = pointer::wheel_to_physical(
            event.delta_x(),
            event.delta_y(),
            event.delta_mode(),
            listener.view.metrics().device_pixel_ratio,
        );
        send_pointer(
            &listener,
            PointerChange::Hover,
            PointerDeviceKind::Mouse,
            Some((dx, dy)),
        );
    });
    let options = web_sys::AddEventListenerOptions::new();
    options.set_passive(false);
    let _ = host
        .canvas
        .add_event_listener_with_callback_and_add_event_listener_options(
            "wheel",
            closure.as_ref().unchecked_ref(),
            &options,
        );
    closure.forget();
}

fn bind_keyboard<C: EmbedderClient + 'static>(host: &Rc<Host<C>>) {
    for (name, down) in [("keydown", true), ("keyup", false)] {
        let listener = Rc::clone(host);
        let closure = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            if dispatch_key(&listener, &event, down) && !event.is_composing() {
                event.prevent_default();
            }
        });
        let _ = host
            .window
            .add_event_listener_with_callback(name, closure.as_ref().unchecked_ref());
        closure.forget();
    }
}

fn dispatch_key<C: EmbedderClient + 'static>(
    host: &Host<C>,
    event: &KeyboardEvent,
    down: bool,
) -> bool {
    let Some(physical) = keys::physical_key_usage(&event.code()) else {
        return false;
    };
    let Some(logical) = keys::logical_key_id(&event.key(), event.location() as i16) else {
        return false;
    };
    let event_type = if !down {
        KeyEventType::Up
    } else if event.repeat() {
        KeyEventType::Repeat
    } else {
        KeyEventType::Down
    };
    let logical = match event_type {
        KeyEventType::Down => {
            host.pressing.borrow_mut().insert(physical, logical);
            logical
        }
        KeyEventType::Repeat => host
            .pressing
            .borrow()
            .get(&physical)
            .copied()
            .unwrap_or(logical),
        KeyEventType::Up => host
            .pressing
            .borrow_mut()
            .remove(&physical)
            .unwrap_or(logical),
    };
    let character = match event_type {
        KeyEventType::Up => None,
        KeyEventType::Down | KeyEventType::Repeat => keys::character_of(&event.key()),
    };
    host.client.borrow_mut().key_data(KeyData {
        time_stamp: host.platform.elapsed(),
        event_type,
        physical,
        logical,
        character,
        synthesized: false,
        ..KeyData::default()
    })
}

fn bind_resize<C: EmbedderClient + 'static>(host: &Rc<Host<C>>) {
    let listener = Rc::clone(host);
    let closure = Closure::<dyn FnMut()>::new(move || {
        let (metrics, size) = fit_canvas(&listener.window, &listener.canvas);
        listener.view.resize_surface(size);
        listener.view.set_metrics(metrics);
        listener
            .client
            .borrow_mut()
            .view_metrics_changed(IMPLICIT_VIEW);
        listener.platform.request_frame();
        schedule_next(&listener);
    });
    let _ = host
        .window
        .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref());
    closure.forget();
}

fn bind_brightness<C: EmbedderClient + 'static>(host: &Rc<Host<C>>) {
    let Ok(Some(query)) = host.window.match_media("(prefers-color-scheme: dark)") else {
        return;
    };
    let listener = Rc::clone(host);
    let closure = Closure::<dyn FnMut()>::new(move || {
        let brightness = if prefers_dark(&listener.window) {
            Brightness::Dark
        } else {
            Brightness::Light
        };
        listener.platform.set_brightness(brightness);
        listener.client.borrow_mut().platform_brightness_changed();
    });
    let _ = query.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref());
    closure.forget();
}

fn send_pointer<C: EmbedderClient + 'static>(
    host: &Host<C>,
    change: PointerChange,
    kind: PointerDeviceKind,
    scroll: Option<(f64, f64)>,
) {
    let [x, y] = host.cursor.get();
    let [last_x, last_y] = host.last_cursor.get();
    host.embedder_id.set(host.embedder_id.get() + 1);
    let buttons = host.mouse_buttons.get();
    let pointer_identifier = if change == PointerChange::Hover && buttons == 0 {
        0
    } else {
        host.pointer_id.get()
    };
    let (signal_kind, scroll_delta_x, scroll_delta_y) = match scroll {
        Some((dx, dy)) => (Some(PointerSignalKind::Scroll), dx, dy),
        None => (None, 0.0, 0.0),
    };
    let data = PointerData {
        view_id: IMPLICIT_VIEW,
        embedder_id: host.embedder_id.get(),
        time_stamp: host.platform.elapsed(),
        change,
        kind,
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
    host.last_cursor.set([x, y]);
    host.client
        .borrow_mut()
        .pointer_data_packet(PointerDataPacket::new(vec![data]));
}
