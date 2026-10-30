//! The main loop's input: the mouse, the fingers and pens, the keyboard, typing into a text
//! input, what is dragged over a window, and the cursor the app shows.
//!
//! winit reports every pointer through one set of events, entered, moved, pressed or released and
//! left, naming which pointer it is: the mouse goes to the pointer that is the mouse
//! (`pointer.rs`), a finger or a pen to the contacts (`touch.rs`). A scroll is the mouse's signal
//! when a wheel turns, and a trackpad's fingers pan as Flutter's pan-zoom pointer, with their
//! pinches and rotations (`trackpad.rs`).

use std::time::Instant;

use inset_embedder::{EmbedderClient, Offset, PointerDataPacket, SystemMouseCursorKind, View};
use winit::data_transfer::{DataTransferId, TypedData};
use winit::dpi::PhysicalPosition;
use winit::event::{
    ButtonSource, ElementState, Ime, KeyEvent, Modifiers, MouseScrollDelta, PointerKind,
    PointerSource, ScrollSource, TouchPhase,
};
use winit::event_loop::{ActiveEventLoop, AsyncRequestSerial, DndAction};
use winit::window::WindowId;

use crate::drag_drop::Carried;
use crate::ime::ImeOutcome;
use crate::mouse_cursor;
use crate::os::{NativeMenus, PointerQueries, ThisOs};
use crate::pointer::PointerStamp;
use crate::text_input::Typed;
use crate::touch::Contact;

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
            mouse_cursor::show(&*open.window, kind);
        }
    }

    /// A pointer came into `window` at `position`: the mouse, or a finger or a pen arriving.
    pub(super) fn on_pointer_entered(
        &mut self,
        window: WindowId,
        position: PhysicalPosition<f64>,
        kind: PointerKind,
    ) {
        let stamp = self.pointer_stamp(window);
        let at = [position.x, position.y];
        let packet = match Contact::of(kind) {
            Some(contact) => self.touches.entered(contact, at, stamp, &mut self.ids),
            None => self.pointer.entered(window, at, stamp, &mut self.ids),
        };
        self.send_pointer_packets(packet);
    }

    /// A pointer left `window`, at `position` where winit says: the mouse, or a finger or a pen
    /// leaving.
    pub(super) fn on_pointer_left(
        &mut self,
        window: WindowId,
        position: Option<PhysicalPosition<f64>>,
        kind: PointerKind,
    ) {
        let stamp = self.pointer_stamp(window);
        let packet = match Contact::of(kind) {
            Some(contact) => {
                let at = position.map(|position| [position.x, position.y]);
                self.touches.left(contact, at, stamp, &mut self.ids)
            }
            None => self.pointer.left(stamp, &mut self.ids),
        };
        self.send_pointer_packets(packet);
    }

    /// A pointer moved in `window` to `position`.
    pub(super) fn on_pointer_moved(
        &mut self,
        window: WindowId,
        position: PhysicalPosition<f64>,
        source: PointerSource,
    ) {
        let stamp = self.pointer_stamp(window);
        let at = [position.x, position.y];
        match Contact::moved_from(&source) {
            Some((contact, pressure)) => {
                let packet = self
                    .touches
                    .moved(contact, at, pressure, stamp, &mut self.ids);
                self.send_pointer_packets(packet);
            }
            None => {
                let packets =
                    self.pointer
                        .moved(window, at, ThisOs::pressed_buttons(), stamp, &mut self.ids);
                self.send_pointer_packets(packets);
            }
        }
    }

    /// A pointer pressed or lifted one of its buttons in `window` at `position`: a mouse's
    /// button, a finger, or a pen's tip.
    pub(super) fn on_pointer_button(
        &mut self,
        window: WindowId,
        state: ElementState,
        position: PhysicalPosition<f64>,
        button: ButtonSource,
    ) {
        let stamp = self.pointer_stamp(window);
        let at = [position.x, position.y];
        let pressed = state == ElementState::Pressed;
        let packet = if let Some((contact, pressure)) = Contact::pressed_by(&button) {
            if pressed {
                self.touches
                    .pressed(contact, at, pressure, stamp, &mut self.ids)
            } else {
                self.touches
                    .released(contact, at, pressure, stamp, &mut self.ids)
            }
        } else if let ButtonSource::Mouse(button) = button {
            self.pointer
                .button(button, pressed, at, stamp, &mut self.ids)
        } else {
            None
        };
        self.send_pointer_packets(packet);
    }

    /// A scroll in `window`, by what drives it, as Flutter's macOS embedder takes a scroll event
    /// (`dispatchGestureEvent:`): fingers on a trackpad pan; the system's momentum after they lift
    /// is left out, since the framework makes its own fling, and only marks when a touch stops
    /// that fling; a wheel's turn, or a scroll winit cannot name the source of, is the mouse's
    /// scroll signal.
    pub(super) fn on_mouse_wheel(
        &mut self,
        window: WindowId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
        source: ScrollSource,
    ) {
        let scale = self.scale_of(window);
        let stamp = self.pointer_stamp(window);
        let packet = match source {
            ScrollSource::Finger => {
                let at = self.pointer.position();
                self.trackpad
                    .scrolled(phase, delta, scale, at, stamp, &mut self.ids)
            }
            ScrollSource::Momentum => {
                self.trackpad.momentum(phase, Instant::now());
                None
            }
            _ => self.pointer.scrolled(delta, scale, stamp, &mut self.ids),
        };
        self.send_pointer_packets(packet);
    }

    /// Fingers came down on the trackpad over `window` at `touched_at`: soon after the system's
    /// momentum, the framework's fling stops.
    pub(super) fn on_trackpad_touched(&mut self, window: WindowId, touched_at: Instant) {
        let stamp = self.pointer_stamp(window);
        let at = self.pointer.position();
        let packet = self.trackpad.touched(touched_at, at, stamp, &mut self.ids);
        self.send_pointer_packets(packet);
    }

    /// Fingers pinched on the trackpad over `window`, which scales the pan-zoom.
    pub(super) fn on_pinch(&mut self, window: WindowId, magnification: f64, phase: TouchPhase) {
        let stamp = self.pointer_stamp(window);
        let at = self.pointer.position();
        let packet = self
            .trackpad
            .pinched(phase, magnification, at, stamp, &mut self.ids);
        self.send_pointer_packets(packet);
    }

    /// Fingers rotated on the trackpad over `window` by `degrees`, which rotates the pan-zoom.
    pub(super) fn on_rotation(&mut self, window: WindowId, degrees: f32, phase: TouchPhase) {
        let stamp = self.pointer_stamp(window);
        let at = self.pointer.position();
        let packet = self
            .trackpad
            .rotated(phase, degrees, at, stamp, &mut self.ids);
        self.send_pointer_packets(packet);
    }

    /// How many physical pixels `window`'s view has to the logical one; one for a window the
    /// embedder no longer has.
    fn scale_of(&self, window: WindowId) -> f64 {
        self.views()
            .by_window(window)
            .map(|open| open.view.metrics().device_pixel_ratio)
            .unwrap_or(1.0)
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
        // Flutter's macOS `FlutterKeyboardManager`: the system's menus see a key only after the
        // framework declined it, as a key equivalent it redispatches, and the text input plugin
        // after those. The key's own event is kept for an app that declines it later.
        ThisOs::keep_key_event(&data);
        if !self.client.key_data(data)
            && event.state == ElementState::Pressed
            && !ThisOs::perform_key_equivalent()
        {
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

    /// A drag came over `window` at `position`: the app hears it with what it carries, which is
    /// accepted as a copy, or it is refused when it carries nothing the app could read.
    pub(super) fn on_drag_entered(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window: WindowId,
        drag: DataTransferId,
        position: Option<PhysicalPosition<f64>>,
    ) {
        let Some(view) = self.view_id_of(window) else {
            return;
        };
        let carried = Carried::by(event_loop, drag);
        let report = self
            .drags
            .entered(drag, view, position.map(offset_of), carried);
        let accepted: &[DndAction] = if report.is_some() {
            &[DndAction::Copy]
        } else {
            &[]
        };
        let _ = event_loop.set_valid_dnd_actions(drag, accepted);
        if let Some(report) = report {
            self.client.drag_data(report);
        }
    }

    /// The drag moved to `position` over its window.
    pub(super) fn on_drag_moved(&mut self, drag: DataTransferId, position: PhysicalPosition<f64>) {
        if let Some(report) = self.drags.moved(drag, offset_of(position)) {
            self.client.drag_data(report);
        }
    }

    /// The drag was let go over its window: the app hears it, and what it reads then is fetched
    /// before winit lets the drop's data go.
    pub(super) fn on_drag_dropped(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        drag: DataTransferId,
    ) {
        let client = &mut self.client;
        self.drags.dropped(
            drag,
            |report| client.drag_data(report),
            |form| event_loop.fetch_data_transfer(drag, form),
        );
    }

    /// The drag left its window without dropping.
    pub(super) fn on_drag_left(&mut self, drag: DataTransferId) {
        if let Some(report) = self.drags.left(drag) {
            self.client.drag_data(report);
        }
    }

    /// winit answered the fetch `fetch` of a dropped drag's form with `value`.
    pub(super) fn on_drag_fetched(&mut self, fetch: AsyncRequestSerial, value: &dyn TypedData) {
        self.drags.fetched(fetch, value);
    }
}

/// A position in a window's physical pixels, as the framework takes one.
fn offset_of(position: PhysicalPosition<f64>) -> Offset {
    Offset::new(position.x, position.y)
}
