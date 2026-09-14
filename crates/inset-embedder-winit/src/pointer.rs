//! The mouse as Flutter's pointer: where it is, which buttons it holds, and the
//! `PointerData` each change becomes, numbered as Flutter's embedders number them.

use std::time::Duration;

use inset_embedder::{
    PointerChange, PointerData, PointerDataPacket, PointerDeviceKind, PointerSignalKind, ViewId,
};
use winit::event::{MouseButton, MouseScrollDelta};
use winit::window::WindowId;

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
/// The buttons the system can be asked about, for reconciling with what it reports.
const KNOWN_BUTTONS: [i64; 5] = [
    PRIMARY_MOUSE_BUTTON,
    SECONDARY_MOUSE_BUTTON,
    MIDDLE_MOUSE_BUTTON,
    BACK_MOUSE_BUTTON,
    FORWARD_MOUSE_BUTTON,
];

pub(crate) struct Pointer {
    /// The window the mouse was last seen in; system cursor requests go there.
    pub(crate) window: Option<WindowId>,
    /// Where it is, in the window's physical pixels.
    position: [f64; 2],
    /// Where the last packet said it was, for the delta of the next.
    last_position: [f64; 2],
    /// Flutter `PointerData.buttons`.
    buttons: i64,
    /// Whether the framework has been told the mouse is present. Flutter's embedders add
    /// the device before its first hover and remove it when it leaves the view.
    added: bool,
    /// Flutter's `pointerIdentifier`: a new one for each press.
    pointer_id: i64,
    embedder_id: i64,
}

impl Pointer {
    pub(crate) fn new() -> Pointer {
        Pointer {
            window: None,
            position: [0.0, 0.0],
            last_position: [0.0, 0.0],
            buttons: 0,
            added: false,
            pointer_id: 0,
            embedder_id: 0,
        }
    }

    pub(crate) fn move_to(&mut self, position: [f64; 2]) {
        self.position = position;
    }

    /// The change a move is: a drag while a button is held, a hover otherwise.
    pub(crate) fn motion(&self) -> PointerChange {
        if self.buttons != 0 {
            PointerChange::Move
        } else {
            PointerChange::Hover
        }
    }

    /// The mouse arriving: whether the framework is still to be told, once, before
    /// anything else about it.
    pub(crate) fn add(&mut self) -> bool {
        !std::mem::replace(&mut self.added, true)
    }

    /// The mouse leaving a window: whether the framework is to be told, which it is not
    /// while a button is held, since a drag that leaves the window goes on as moves.
    pub(crate) fn remove(&mut self) -> bool {
        if self.added && self.buttons == 0 {
            self.added = false;
            return true;
        }
        false
    }

    /// A button pressed or released, and the change it makes: Down when the first goes
    /// down, Move while any remain, Up when the last is released, as Flutter's macOS
    /// `dispatchMouseEvent:` splits them.
    pub(crate) fn set_button(
        &mut self,
        button: MouseButton,
        pressed: bool,
    ) -> Option<PointerChange> {
        self.set_button_bit(flutter_mouse_button(button)?, pressed)
    }

    fn set_button_bit(&mut self, bit: i64, pressed: bool) -> Option<PointerChange> {
        let previous = self.buttons;
        if pressed {
            self.buttons |= bit;
        } else {
            self.buttons &= !bit;
        }
        pointer_change_for_buttons(previous, self.buttons)
    }

    /// Brings the record of the buttons in line with `actual`, as the system reports
    /// them, answering the changes that makes. A native menu opened from a press runs
    /// its own event loop and swallows the release, which would leave the app holding a
    /// button forever: no hovers, no taps.
    pub(crate) fn reconcile(&mut self, actual: i64) -> Vec<PointerChange> {
        let mut changes = Vec::new();
        for bit in KNOWN_BUTTONS {
            let held = actual & bit != 0;
            if held != (self.buttons & bit != 0)
                && let Some(change) = self.set_button_bit(bit, held)
            {
                changes.push(change);
            }
        }
        changes
    }

    /// The packet for one change of the pointer in `view`, with a scroll's deltas in
    /// physical pixels when the change is one.
    pub(crate) fn packet(
        &mut self,
        view_id: ViewId,
        change: PointerChange,
        scroll: Option<[f64; 2]>,
        time_stamp: Duration,
    ) -> PointerDataPacket {
        if change == PointerChange::Down {
            self.pointer_id += 1;
        }
        self.embedder_id += 1;
        let [x, y] = self.position;
        let [last_x, last_y] = self.last_position;
        let pointer_identifier = if self.buttons == 0
            && matches!(
                change,
                PointerChange::Hover | PointerChange::Add | PointerChange::Remove
            ) {
            0
        } else {
            self.pointer_id
        };
        let (signal_kind, [scroll_delta_x, scroll_delta_y]) = match scroll {
            Some(delta) => (Some(PointerSignalKind::Scroll), delta),
            None => (None, [0.0, 0.0]),
        };
        let data = PointerData {
            view_id,
            embedder_id: self.embedder_id,
            time_stamp,
            change,
            kind: PointerDeviceKind::Mouse,
            signal_kind,
            pointer_identifier,
            physical_x: x,
            physical_y: y,
            physical_delta_x: x - last_x,
            physical_delta_y: y - last_y,
            buttons: self.buttons,
            pressure: 1.0,
            pressure_min: 1.0,
            pressure_max: 1.0,
            scroll_delta_x,
            scroll_delta_y,
            ..PointerData::default()
        };
        self.last_position = self.position;
        PointerDataPacket::new(vec![data])
    }
}

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

/// Line-based wheels are not pixels. 40 logical px/line is Chromium's
/// convention (shaft-rs-next); convert to physical by the view scale so
/// [`PointerData::scroll_delta_x`] / [`PointerData::scroll_delta_y`] stay
/// physical, matching dart:ui. winit's up-positive deltas are sign-flipped
/// to Flutter's content-forward sign.
pub(crate) fn wheel_to_physical(delta: MouseScrollDelta, scale: f64) -> [f64; 2] {
    const LOGICAL_PIXELS_PER_LINE: f64 = 40.0;
    match delta {
        MouseScrollDelta::LineDelta(dx, dy) => [
            -f64::from(dx) * LOGICAL_PIXELS_PER_LINE * scale,
            -f64::from(dy) * LOGICAL_PIXELS_PER_LINE * scale,
        ],
        MouseScrollDelta::PixelDelta(physical) => [-physical.x, -physical.y],
    }
}

#[cfg(test)]
mod tests {
    use winit::dpi::PhysicalPosition;

    use super::*;

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
