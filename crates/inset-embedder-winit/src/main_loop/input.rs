//! The main loop's input: the mouse, the fingers, the keyboard, typing into a text input, files
//! dragged over a window, and the cursor the app shows.

use std::path::PathBuf;

use inset_embedder::{DropChange, EmbedderClient, PointerDataPacket, SystemMouseCursorKind, View};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Ime, KeyEvent, Modifiers, MouseButton, MouseScrollDelta, Touch};
use winit::window::WindowId;

use crate::ime::ImeOutcome;
use crate::mouse_cursor;
use crate::os::{PointerQueries, ThisOs};
use crate::pointer::PointerStamp;
use crate::text_input::Typed;

use super::MainLoop;

impl<C: EmbedderClient> MainLoop<C> {
    /// Shows `kind` on the window the pointer was last seen in; before the pointer was seen in
    /// any window, nowhere.
    pub(super) fn show_cursor(&self, kind: SystemMouseCursorKind) {
        let pointer_window = self
            .pointer
            .window()
            .and_then(|id| self.views().by_window(id));
        if let Some(open) = pointer_window {
            mouse_cursor::show(&open.window, kind);
        }
    }

    pub(super) fn on_cursor_entered(&mut self, id: WindowId) {
        // The entry carries no position, and the last one seen may be from
        // before the window moved; the add goes out at where the pointer is.
        let position = self
            .views()
            .by_window(id)
            .and_then(|open| ThisOs::pointer_position_in(&open.window));
        let stamp = self.pointer_stamp(id);
        let packet = self.pointer.entered(id, position, stamp, &mut self.ids);
        self.send_pointer_packets(packet);
    }

    pub(super) fn on_cursor_left(&mut self, id: WindowId) {
        let stamp = self.pointer_stamp(id);
        let packet = self.pointer.left(stamp, &mut self.ids);
        self.send_pointer_packets(packet);
    }

    pub(super) fn on_cursor_moved(&mut self, id: WindowId, position: PhysicalPosition<f64>) {
        let stamp = self.pointer_stamp(id);
        let packets = self.pointer.moved(
            id,
            [position.x, position.y],
            ThisOs::pressed_buttons(),
            stamp,
            &mut self.ids,
        );
        self.send_pointer_packets(packets);
    }

    pub(super) fn on_mouse_input(
        &mut self,
        id: WindowId,
        state: ElementState,
        button: MouseButton,
    ) {
        let stamp = self.pointer_stamp(id);
        let pressed = state == ElementState::Pressed;
        let packet = self.pointer.button(button, pressed, stamp, &mut self.ids);
        self.send_pointer_packets(packet);
    }

    pub(super) fn on_mouse_wheel(&mut self, id: WindowId, delta: MouseScrollDelta) {
        let scale = self
            .views()
            .by_window(id)
            .map(|open| open.view.metrics().device_pixel_ratio)
            .unwrap_or(1.0);
        let stamp = self.pointer_stamp(id);
        let packet = self.pointer.scrolled(delta, scale, stamp, &mut self.ids);
        self.send_pointer_packets(packet);
    }

    /// A native menu ran its own event loop and closed: the presses and releases it kept are
    /// sent by what the system says is held down now, and the client is woken.
    pub(super) fn on_menu_closed(&mut self) {
        if let Some(window) = self.pointer.window() {
            let stamp = self.pointer_stamp(window);
            let packets = self
                .pointer
                .reconciled(ThisOs::pressed_buttons(), stamp, &mut self.ids);
            self.send_pointer_packets(packets);
        }
        self.wake_client();
    }

    pub(super) fn on_touch(&mut self, id: WindowId, touch: &Touch) {
        let Some(view) = self.view_id_of(id) else {
            return;
        };
        let packet = self
            .touches
            .packet(touch, view, self.platform.elapsed(), &mut self.ids);
        self.send_pointer_packets(packet);
    }

    /// The view of `window`, if the embedder has it, and the time a pointer event there happens.
    pub(super) fn pointer_stamp(&self, window: WindowId) -> PointerStamp {
        PointerStamp {
            view: self.view_id_of(window),
            time_stamp: self.platform.elapsed(),
        }
    }

    pub(super) fn send_pointer_packets(
        &mut self,
        packets: impl IntoIterator<Item = PointerDataPacket>,
    ) {
        for packet in packets {
            self.client.pointer_data_packet(packet);
        }
    }

    /// The modifier keys held now, which the next key event and typed text carry.
    pub(super) fn on_modifiers_changed(&mut self, modifiers: Modifiers) {
        self.keyboard.set_modifiers(modifiers.state());
    }

    pub(super) fn on_keyboard_input(&mut self, id: WindowId, event: &KeyEvent, is_synthetic: bool) {
        let Some(data) = self
            .keyboard
            .key_data(event, is_synthetic, self.platform.elapsed())
        else {
            return;
        };
        // Flutter's macOS `FlutterKeyboardManager`: the text input plugin sees a key only
        // after the framework declined it.
        if !self.client.key_data(data) && event.state == ElementState::Pressed {
            self.type_into_text_input(id, event);
        }
    }

    /// `FlutterTextInputPlugin.handleKeyEvent` for the view's active text input, if any.
    pub(super) fn type_into_text_input(&mut self, id: WindowId, event: &KeyEvent) {
        let Some(open) = self.views().by_window(id) else {
            return;
        };
        let view = open.view.id();
        match open.view.type_key(event, self.keyboard.modifiers()) {
            Typed::Nothing => {}
            Typed::Edited(value) => self.client.text_input_editing_value(view, value),
            Typed::Entered { edited, action } => {
                if let Some(value) = edited {
                    self.client.text_input_editing_value(view, value);
                }
                self.client.text_input_action(view, action);
            }
        }
    }

    pub(super) fn on_ime(&mut self, id: WindowId, ime: Ime) {
        let Some(open) = self.views().by_window(id) else {
            return;
        };
        let view_id = open.view.id();
        match open.view.take_ime(&ime) {
            ImeOutcome::None => {}
            ImeOutcome::Closed => self.client.text_input_closed(view_id),
            ImeOutcome::Value(value) => self.client.text_input_editing_value(view_id, value),
        }
    }

    /// One file of a drag that came over `window`, or was let go over it.
    pub(super) fn on_dragged_file(&mut self, window: WindowId, change: DropChange, path: PathBuf) {
        if let Some(view) = self.view_id_of(window) {
            self.drag_reports.add_file(view, change, path);
        }
    }

    /// The drag left `window` without dropping.
    pub(super) fn on_drag_exited(&mut self, window: WindowId) {
        if let Some(view) = self.view_id_of(window) {
            self.drag_reports.add_exit(view);
        }
    }

    /// Reports the turn's drags to the client.
    pub(super) fn report_drags(&mut self) {
        for drop in self.drag_reports.take() {
            self.client.drop_data(drop);
        }
    }
}
