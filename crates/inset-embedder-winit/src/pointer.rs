//! The mouse as Flutter's pointer: where it is, which buttons it holds, and the report each of
//! winit's mouse events becomes. Flutter's macOS embedder sends `kAdd` on `mouseEntered` and
//! `kRemove` on `mouseExited`; a `MouseRegion` exits on the remove.

use std::time::Duration;

use inset_embedder::{Contact, PointerDeviceKind, PointerReport, ReportedChange, ViewId};
use winit::event::{MouseButton, MouseScrollDelta};
use winit::window::WindowId;

/// The buttons the system can be asked about, for reconciling with what it reports: Flutter's
/// `kFlutterPointerButtonMousePrimary`, `Secondary`, `Middle`, `Back` and `Forward`
/// (`embedder.h`).
const KNOWN_BUTTONS: [i64; 5] = [1 << 0, 1 << 1, 1 << 2, 1 << 3, 1 << 4];

/// Flutter's macOS embedder's `kMousePointerDeviceId`.
pub(crate) const MOUSE_DEVICE: i64 = 0;

/// Where and when a pointer event happened: the view whose window it happened in, `None` for a
/// window the embedder no longer has, whose events change the pointer but reach no view; and the
/// time since the embedder started.
#[derive(Clone, Copy)]
pub(crate) struct PointerStamp {
    pub(crate) view: Option<ViewId>,
    pub(crate) time_stamp: Duration,
}

pub(crate) struct Pointer {
    /// The window the mouse was last seen in; system cursor requests go there.
    window: Option<WindowId>,
    /// Where it is, in the window's physical pixels.
    position: [f64; 2],
    /// Flutter `PointerData.buttons`.
    buttons: i64,
    /// Whether the framework has been told the mouse is present: Flutter's
    /// `flutter_state_is_added`. The macOS embedder adds the device before its first hover and
    /// removes it when it leaves the view.
    added: bool,
}

impl Pointer {
    pub(crate) fn new() -> Pointer {
        Pointer {
            window: None,
            position: [0.0, 0.0],
            buttons: 0,
            added: false,
        }
    }

    /// The window the mouse was last seen in.
    pub(crate) fn window(&self) -> Option<WindowId> {
        self.window
    }

    /// Where the mouse was last seen, in its window's physical pixels.
    pub(crate) fn position(&self) -> [f64; 2] {
        self.position
    }

    /// The mouse came into `window` at `position`: the framework is told it is present, before
    /// anything else about it.
    pub(crate) fn entered(
        &mut self,
        window: WindowId,
        position: [f64; 2],
        stamp: PointerStamp,
    ) -> Option<PointerReport> {
        self.window = Some(window);
        self.position = position;
        self.added_report(stamp)
    }

    /// The mouse left its window: the framework is told it is gone, unless a button is held,
    /// since a drag that leaves the window goes on as moves.
    pub(crate) fn left(&mut self, stamp: PointerStamp) -> Option<PointerReport> {
        if self.added && self.buttons == 0 {
            self.added = false;
            return self.report(ReportedChange::Remove, stamp);
        }
        None
    }

    /// The mouse moved in `window` to `position`: a hover, or a drag while a button is held,
    /// after the presses and releases `pressed_now`, what the system says is held down now,
    /// shows were missed.
    pub(crate) fn moved(
        &mut self,
        window: WindowId,
        position: [f64; 2],
        pressed_now: Option<i64>,
        stamp: PointerStamp,
    ) -> Vec<PointerReport> {
        self.window = Some(window);
        self.position = position;
        let mut reports: Vec<_> = self.added_report(stamp).into_iter().collect();
        reports.extend(self.reconciled(pressed_now, stamp));
        reports.extend(self.report(self.motion(), stamp));
        reports
    }

    /// A button pressed or released at `position`: Down when the first goes down, Move while any
    /// remain, Up when the last is released, as Flutter's macOS `dispatchMouseEvent:` splits them.
    pub(crate) fn button(
        &mut self,
        button: MouseButton,
        pressed: bool,
        position: [f64; 2],
        stamp: PointerStamp,
    ) -> Option<PointerReport> {
        let change = self.set_button_bit(flutter_mouse_button(button), pressed)?;
        self.position = position;
        self.report(change, stamp)
    }

    /// The wheel or the trackpad scrolled by `delta`, in a view `scale` physical pixels to the
    /// logical one.
    pub(crate) fn scrolled(
        &mut self,
        delta: MouseScrollDelta,
        scale: f64,
        stamp: PointerStamp,
    ) -> Option<PointerReport> {
        let delta = wheel_to_physical(delta, scale);
        self.report(ReportedChange::Scroll { delta }, stamp)
    }

    /// Brings the record of the buttons in line with `pressed_now`, as the system reports
    /// them, where it can be asked: the presses and releases the embedder missed while a native
    /// menu ran its own event loop. The menu swallows the release of the button that opened
    /// it, which would leave the app holding a button forever: no hovers, no taps.
    pub(crate) fn reconciled(
        &mut self,
        pressed_now: Option<i64>,
        stamp: PointerStamp,
    ) -> Vec<PointerReport> {
        let Some(actual) = pressed_now else {
            return Vec::new();
        };
        let changes = self.missed_button_changes(actual);
        changes
            .into_iter()
            .filter_map(|change| self.report(change, stamp))
            .collect()
    }

    /// The change a move is: a drag while a button is held, a hover otherwise.
    fn motion(&self) -> ReportedChange {
        if self.buttons != 0 {
            ReportedChange::Move {
                buttons: self.buttons,
            }
        } else {
            ReportedChange::Hover { buttons: 0 }
        }
    }

    /// The Add the framework is told once, before anything else about the mouse.
    fn added_report(&mut self, stamp: PointerStamp) -> Option<PointerReport> {
        if std::mem::replace(&mut self.added, true) {
            return None;
        }
        self.report(ReportedChange::Add, stamp)
    }

    fn set_button_bit(&mut self, bit: i64, pressed: bool) -> Option<ReportedChange> {
        let previous = self.buttons;
        if pressed {
            self.buttons |= bit;
        } else {
            self.buttons &= !bit;
        }
        change_for_buttons(previous, self.buttons)
    }

    /// The presses and releases missed, which bring the record of the buttons in line with
    /// `actual`; the record takes them as it answers them.
    fn missed_button_changes(&mut self, actual: i64) -> Vec<ReportedChange> {
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

    /// The mouse's report of `change` where it is, in the view `stamp` names; `None` for a window
    /// the embedder no longer has.
    fn report(&self, change: ReportedChange, stamp: PointerStamp) -> Option<PointerReport> {
        Some(PointerReport {
            view_id: stamp.view?,
            time_stamp: stamp.time_stamp,
            device: MOUSE_DEVICE,
            kind: PointerDeviceKind::Mouse,
            position: self.position,
            change,
            contact: Contact::default(),
        })
    }
}

/// Flutter engine mouse-button bits for a winit button: `1 << n` for the button the system
/// numbers `n`, as the macOS embedder's `otherMouseDown:` takes `1 << event.buttonNumber`. winit
/// numbers the five named ones as `kFlutterPointerButtonMouse*` in `embedder.h` does.
fn flutter_mouse_button(button: MouseButton) -> i64 {
    1 << button as u8
}

/// Down when the first button goes down, Move while any remain, Up when the last is released,
/// each with the buttons held after it. Flutter's macOS `dispatchMouseEvent:` uses the same
/// three-way split.
fn change_for_buttons(previous: i64, next: i64) -> Option<ReportedChange> {
    if previous == next {
        return None;
    }
    Some(if next == 0 {
        ReportedChange::Up { buttons: next }
    } else if previous == 0 {
        ReportedChange::Down { buttons: next }
    } else {
        ReportedChange::Move { buttons: next }
    })
}

/// Line-based wheels are not pixels. 40 logical px/line is Chromium's
/// convention; convert to physical by the view scale so a
/// [`ReportedChange::Scroll`] delta stays physical, matching dart:ui.
/// winit's up-positive deltas are sign-flipped to Flutter's
/// content-forward sign.
fn wheel_to_physical(delta: MouseScrollDelta, scale: f64) -> [f64; 2] {
    const LOGICAL_PIXELS_PER_LINE: f64 = 40.0;
    match delta {
        MouseScrollDelta::LineDelta(dx, dy) => [
            -f64::from(dx) * LOGICAL_PIXELS_PER_LINE * scale,
            -f64::from(dy) * LOGICAL_PIXELS_PER_LINE * scale,
        ],
        MouseScrollDelta::PixelDelta(physical) => [-physical.x, -physical.y],
        // A kind of delta winit adds later scrolls nothing until it is ported.
        _ => [0.0, 0.0],
    }
}

#[cfg(test)]
mod tests {
    use winit::dpi::PhysicalPosition;

    use super::*;

    const PRIMARY_MOUSE_BUTTON: i64 = 1 << 0;
    const SECONDARY_MOUSE_BUTTON: i64 = 1 << 1;

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
            PRIMARY_MOUSE_BUTTON
        );
        assert_eq!(
            flutter_mouse_button(MouseButton::Right),
            SECONDARY_MOUSE_BUTTON
        );
        assert_eq!(flutter_mouse_button(MouseButton::Middle), 1 << 2);
        assert_eq!(flutter_mouse_button(MouseButton::Back), 1 << 3);
        assert_eq!(flutter_mouse_button(MouseButton::Forward), 1 << 4);
        assert_eq!(flutter_mouse_button(MouseButton::Button6), 1 << 5);
    }

    #[test]
    fn mouse_button_chords_are_down_move_up() {
        assert_eq!(
            change_for_buttons(0, SECONDARY_MOUSE_BUTTON),
            Some(ReportedChange::Down {
                buttons: SECONDARY_MOUSE_BUTTON
            })
        );
        assert_eq!(
            change_for_buttons(SECONDARY_MOUSE_BUTTON, 0),
            Some(ReportedChange::Up { buttons: 0 })
        );
        assert_eq!(
            change_for_buttons(
                PRIMARY_MOUSE_BUTTON,
                PRIMARY_MOUSE_BUTTON | SECONDARY_MOUSE_BUTTON
            ),
            Some(ReportedChange::Move {
                buttons: PRIMARY_MOUSE_BUTTON | SECONDARY_MOUSE_BUTTON
            })
        );
        assert_eq!(
            change_for_buttons(PRIMARY_MOUSE_BUTTON, PRIMARY_MOUSE_BUTTON),
            None
        );
    }
}
