//! Reusable application fixture with simulated time and input.

use crate::{CaptureView, gpu};
use inset_embedder::valo::Context;
use inset_embedder::{
    Offset, PointerChange, PointerData, PointerDataPacket, PointerDeviceKind, SystemFontSource,
    TargetPlatform, View,
};
use inset_foundation::{App, AppCell};
use inset_gestures::GestureBinding;
use inset_rendering::RenderParagraph;
use inset_scheduler::SchedulerBinding;
use inset_test::{MockClipboard, TestPlatform};
use inset_widgets::{AnyElement, WidgetRef, WidgetsBinding};
use std::{cell::RefCell, rc::Rc, time::Duration};

/// Mounts an arbitrary application with simulated time and pointer input.
pub struct Fixture {
    /// Application arena and scheduler checkpoints.
    pub cell: Rc<AppCell>,
    /// Headless rendered output.
    pub view: Rc<CaptureView>,
    /// Simulated event timestamp.
    pub at: Duration,
    /// Gesture identifiers increase on each down, as in the engine converter.
    next_pointer: i64,
    /// Identifier of the current touch gesture, shared with manually constructed packets.
    pub touch_pointer: i64,
    /// Identifier of the current mouse gesture, shared with manually constructed packets.
    pub mouse_pointer: i64,
    /// The platform's clipboard, in memory.
    clipboard: MockClipboard,
}
impl Fixture {
    /// Mounts whatever `run` starts, for a test of one control.
    pub fn with_root(size: [u32; 2], run: impl FnOnce(&mut App)) -> Self {
        let (device, queue) = gpu::headless_device().expect("GPU required");
        let view = Rc::new(CaptureView {
            size,
            renderer: RefCell::new(Context::new(device, queue)),
            pixels: RefCell::new(Vec::new()),
        });
        // Images made from pixels go to the capture view's renderer, which draws them.
        let renderer = view.clone();
        let clipboard = MockClipboard::new();
        let platform = TestPlatform::new()
            .on(TargetPlatform::MacOS)
            .with_view(view.clone())
            .with_clipboard(Rc::new(clipboard.clone()))
            .with_font_source(|| Box::new(SystemFontSource::platform()))
            .with_pixel_import(move |pixels| {
                let renderer = renderer.renderer.borrow();
                renderer.image_context().upload_pixels(pixels, false).ok()
            });
        let cell = AppCell::with_platform(Rc::new(platform));
        {
            let mut app = cell.borrow_mut();
            inset_painting::PaintingBinding::instance(&mut app).install_fonts(&mut app, |fonts| {
                fonts.add_source(SystemFontSource::platform());
            });
            run(&mut app);
        }
        cell.elapse(Duration::ZERO);
        let mut result = Self {
            cell,
            view,
            at: Duration::ZERO,
            next_pointer: 0,
            touch_pointer: 0,
            mouse_pointer: 0,
            clipboard,
        };
        result.pump();
        result
    }
    /// Runs ten 20 ms refreshes of the view.
    pub fn pump(&mut self) {
        for _ in 0..10 {
            self.frame_after(Duration::from_millis(20));
        }
    }

    /// Runs one refresh of the view, `elapsed` after the last, processing microtasks between its
    /// phases.
    pub fn frame_after(&mut self, elapsed: Duration) {
        self.at += elapsed;
        let views = [self.view.id()];
        SchedulerBinding::handle_begin_frame(&mut self.cell.borrow_mut(), &views, Some(self.at));
        self.cell.checkpoint();
        SchedulerBinding::handle_draw_frame(&mut self.cell.borrow_mut(), &views);
        self.cell.checkpoint();
    }
    /// The text on the platform's clipboard.
    pub fn clipboard_text(&self) -> Option<String> {
        self.clipboard.clipboard_data()
    }

    /// Enumerates all mounted elements, including retained offstage subtrees.
    pub fn elements(&self) -> Vec<AnyElement> {
        let mut app = self.cell.borrow_mut();
        let root = WidgetsBinding::instance(&mut app)
            .root_element(&app)
            .unwrap();
        let mut all = vec![root];
        let mut i = 0;
        while i < all.len() {
            all.extend(all[i].children(&app));
            i += 1;
        }
        all
    }
    /// Enumerates painted subtrees while excluding descendants of Offstage widgets.
    fn onstage_elements(&self) -> Vec<AnyElement> {
        let mut app = self.cell.borrow_mut();
        let root = WidgetsBinding::instance(&mut app)
            .root_element(&app)
            .unwrap();
        let mut elements = vec![root];
        let mut index = 0;
        while index < elements.len() {
            let element = elements[index];
            let offstage = inset_widgets::downcast_widget::<inset_widgets::Offstage>(
                element.widget(&app).as_ref(),
            )
            .is_some_and(|widget| widget.offstage);
            if !offstage {
                elements.extend(element.children(&app));
            }
            index += 1;
        }
        elements
    }

    /// Finds the center of the first onstage text label.
    pub fn find(&self, text: &str) -> Offset {
        let elements = self.onstage_elements();
        let app = self.cell.borrow();
        for element in elements {
            if let Some(object) = element.render_object(&app)
                && let Some(paragraph) = object.downcast::<RenderParagraph>(&app)
                && paragraph.text(&app).to_plain_text(true, true) == text
            {
                let object = object.as_box().unwrap();
                let size = object.size(&app);
                return object.local_to_global(
                    &app,
                    Offset::new(size.width() / 2.0, size.height() / 2.0),
                    None,
                );
            }
        }
        panic!("missing label {text}")
    }
    /// Brings an already mounted label into view before a test sends pointer input.
    pub fn ensure_visible(&mut self, text: &str) {
        let elements = self.onstage_elements();
        let mut app = self.cell.borrow_mut();
        let context = elements
            .into_iter()
            .find(|element| {
                element
                    .render_object(&app)
                    .and_then(|object| object.downcast::<RenderParagraph>(&app))
                    .is_some_and(|paragraph| paragraph.text(&app).to_plain_text(true, true) == text)
            })
            .unwrap_or_else(|| panic!("missing label {text}"));
        drop(inset_widgets::Scrollable::ensure_visible(
            &mut app,
            context,
            0.5,
            Duration::ZERO,
            inset_animation::Curves::linear(),
            inset_widgets::ScrollPositionAlignmentPolicy::Explicit,
        ));
        drop(app);
        self.cell.checkpoint();
        self.pump();
    }

    /// Sends a touch event at the fixture timestamp.
    pub fn send(&mut self, change: PointerChange, point: Offset) {
        if change == PointerChange::Down {
            self.next_pointer += 1;
            self.touch_pointer = self.next_pointer;
        }
        let mut app = self.cell.borrow_mut();
        GestureBinding::instance(&mut app).handle_pointer_data_packet(
            &mut app,
            PointerDataPacket::new(vec![PointerData {
                change,
                kind: PointerDeviceKind::Touch,
                time_stamp: self.at,
                pointer_identifier: self.touch_pointer,
                physical_x: point.dx(),
                physical_y: point.dy(),
                ..Default::default()
            }]),
        );
        drop(app);
        self.cell.checkpoint();
    }
    /// A mouse event: hover moves have no buttons, a press and its moves carry the primary button.
    pub fn send_mouse(&mut self, change: PointerChange, point: Offset, buttons: i64) {
        if change == PointerChange::Down {
            self.next_pointer += 1;
            self.mouse_pointer = self.next_pointer;
        }
        let mut app = self.cell.borrow_mut();
        GestureBinding::instance(&mut app).handle_pointer_data_packet(
            &mut app,
            PointerDataPacket::new(vec![PointerData {
                change,
                kind: PointerDeviceKind::Mouse,
                time_stamp: self.at,
                pointer_identifier: self.mouse_pointer,
                physical_x: point.dx(),
                physical_y: point.dy(),
                buttons,
                ..Default::default()
            }]),
        );
        drop(app);
        self.cell.checkpoint();
    }

    /// Traverses with Tab presses until the named control owns keyboard focus.
    pub fn focus(&mut self, text: &str) {
        use inset_services::{
            HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey,
            PhysicalKeyboardKey,
        };
        let elements = self.onstage_elements();
        let mut app = self.cell.borrow_mut();
        let context = elements
            .into_iter()
            .find(|element| {
                element
                    .render_object(&app)
                    .and_then(|object| object.downcast::<RenderParagraph>(&app))
                    .is_some_and(|paragraph| paragraph.text(&app).to_plain_text(true, true) == text)
            })
            .unwrap_or_else(|| panic!("missing label {text}"));
        let target = inset_widgets::Focus::of(&mut app, context, false, false);
        drop(app);
        for _ in 0..100 {
            if inset_widgets::primary_focus(&mut self.cell.borrow_mut()) == Some(target) {
                return;
            }
            for event in [
                KeyEvent::Down(KeyDownEvent::new(
                    PhysicalKeyboardKey::TAB,
                    LogicalKeyboardKey::TAB,
                    self.at,
                )),
                KeyEvent::Up(KeyUpEvent::new(
                    PhysicalKeyboardKey::TAB,
                    LogicalKeyboardKey::TAB,
                    self.at,
                )),
            ] {
                let mut app = self.cell.borrow_mut();
                HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
                drop(app);
                self.cell.checkpoint();
            }
            self.pump();
        }
        panic!("Tab traversal did not reach {text}");
    }

    /// Taps a text label and pumps the resulting frames.
    pub fn tap(&mut self, text: &str) {
        let p = self.find(text);
        self.send(PointerChange::Down, p);
        self.at += Duration::from_millis(20);
        self.send(PointerChange::Up, p);
        self.pump();
    }
    /// Checks the frame and optionally writes it under WINUI_CAPTURE_DIR.
    pub fn capture(&self, name: &str) {
        let pixels = self.view.pixels.borrow();
        assert_eq!(
            pixels.len(),
            (self.view.size[0] * self.view.size[1] * 4) as usize
        );
        assert!(pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
        if let Ok(dir) = std::env::var("WINUI_CAPTURE_DIR") {
            let dir = std::path::Path::new(&dir);
            std::fs::create_dir_all(dir).unwrap();
            gpu::write_png(&dir.join(format!("{name}.png")), self.view.size, &pixels);
        }
    }
}

/// `inset_widgets::run_app` for a widget built once the app exists.
pub fn mount(root: impl FnOnce(&mut App) -> WidgetRef + 'static) -> impl FnOnce(&mut App) {
    move |app| {
        let widget = root(app);
        inset_widgets::run_app(app, widget);
    }
}
