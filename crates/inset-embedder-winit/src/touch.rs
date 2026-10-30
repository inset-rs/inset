//! Fingers and pens as Flutter's pointers: one device each, the add, down, move, up and remove
//! Flutter's iOS embedder sends for every touch (`FlutterViewController.mm`, `dispatchTouches:`),
//! which winit reports in that order as a touch entering, pressing, moving, releasing and
//! leaving, so the framework meets each finger the way it meets a mouse and lets go of it after.
//! A touch that leaves still pressed is cancelled before it is removed, by the client's converter.
//! A pen is a stylus, as Flutter's iOS and Windows embedders report it, and hovers where the
//! system reports it near the screen.

use std::collections::HashMap;

use inset_embedder::{Contact, PointerDeviceKind, PointerReport, ReportedChange};
use winit::event::{ButtonSource, Force, PointerKind, PointerSource, TabletToolButton};

use crate::pointer::PointerStamp;
use crate::trackpad::PAN_ZOOM_DEVICE;

/// The first device a touch is numbered with, after the mouse's and the trackpad's: each device
/// number is unique across kinds, as Flutter's Windows embedder makes them so that pointers do not
/// clash in the engine's converter (`GetOrCreatePointerState`).
const FIRST_TOUCH_DEVICE: i64 = PAN_ZOOM_DEVICE + 1;

/// A finger or the pen, as winit names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Touch {
    /// A finger, by the system's id for its touch.
    Finger(usize),
    /// The pen, which winit names no further.
    Pen,
}

impl Touch {
    /// The touch a pointer of `kind` is; `None` for the mouse, and for a pointer winit cannot
    /// name, which the mouse stands for.
    pub(crate) fn of(kind: PointerKind) -> Option<Touch> {
        match kind {
            PointerKind::Touch(finger) => Some(Touch::Finger(finger.into_raw())),
            PointerKind::TabletTool(_) => Some(Touch::Pen),
            _ => None,
        }
    }

    /// The touch that moved, with how it meets the screen, from the source of a move.
    pub(crate) fn moved_from(source: &PointerSource) -> Option<(Touch, Contact)> {
        match source {
            PointerSource::Touch { finger_id, force } => Some((
                Touch::Finger(finger_id.into_raw()),
                pressing(finger_pressure(*force)),
            )),
            PointerSource::TabletTool { data, .. } => {
                Some((Touch::Pen, pressing(pen_pressure(data))))
            }
            _ => None,
        }
    }

    /// The touch a button event presses or lifts, with how it meets the screen: a finger, or a
    /// pen's tip. `None` for a mouse's button, and for a pen's other buttons.
    pub(crate) fn pressed_by(button: &ButtonSource) -> Option<(Touch, Contact)> {
        match button {
            ButtonSource::Touch { finger_id, force } => Some((
                Touch::Finger(finger_id.into_raw()),
                pressing(finger_pressure(*force)),
            )),
            ButtonSource::TabletTool {
                button: TabletToolButton::Contact,
                data,
                ..
            } => Some((Touch::Pen, pressing(pen_pressure(data)))),
            _ => None,
        }
    }

    fn kind(self) -> PointerDeviceKind {
        match self {
            Touch::Finger(_) => PointerDeviceKind::Touch,
            Touch::Pen => PointerDeviceKind::Stylus,
        }
    }
}

/// How a finger or a pen meets the screen, pressing by `pressure` from none to full.
fn pressing(pressure: f64) -> Contact {
    Contact {
        pressure,
        pressure_min: 0.0,
        pressure_max: 1.0,
        ..Contact::default()
    }
}

/// How hard a finger presses, from none to full; a screen that cannot tell reports full.
fn finger_pressure(force: Option<Force>) -> f64 {
    force.map_or(1.0, |force| force.normalized(None))
}

/// How hard a pen presses, its force taken along its own axis where it reports its angle.
fn pen_pressure(data: &winit::event::TabletToolData) -> f64 {
    data.force
        .map_or(1.0, |force| force.normalized(data.clone().angle()))
}

/// The touches on the screen, or near it.
#[derive(Default)]
pub(crate) struct Touches {
    touches: HashMap<Touch, TouchState>,
}

struct TouchState {
    /// The device it is reported as: the smallest number from `FIRST_TOUCH_DEVICE` no other
    /// touch has, as Flutter's Windows embedder numbers its touches (`SequentialIdGenerator`).
    device: i64,
    /// Where it was last reported, for a departure winit gives no position for.
    at: [f64; 2],
    /// Whether it presses, which makes its moves drags rather than hovers.
    pressing: bool,
}

impl Touches {
    /// `touch` came to `at`: the framework adds it as a device, before anything else about it.
    /// A second arrival is dropped.
    pub(crate) fn entered(
        &mut self,
        touch: Touch,
        at: [f64; 2],
        stamp: PointerStamp,
    ) -> Option<PointerReport> {
        if self.touches.contains_key(&touch) {
            return None;
        }
        let state = TouchState {
            device: self.free_device(),
            at,
            pressing: false,
        };
        let arrival = report(touch, &state, ReportedChange::Add, pressing(0.0), stamp);
        self.touches.insert(touch, state);
        arrival
    }

    /// `touch` pressed at `at`. A press of a touch the embedder never saw arrive, or of one
    /// already pressing, is dropped.
    pub(crate) fn pressed(
        &mut self,
        touch: Touch,
        at: [f64; 2],
        contact: Contact,
        stamp: PointerStamp,
    ) -> Option<PointerReport> {
        let state = self.touches.get_mut(&touch)?;
        if state.pressing {
            return None;
        }
        state.pressing = true;
        state.at = at;
        let down = ReportedChange::Down { buttons: 0 };
        report(touch, state, down, contact, stamp)
    }

    /// `touch` moved to `at`: a move while it presses, a hover otherwise. A move of a touch the
    /// embedder never saw arrive is dropped.
    pub(crate) fn moved(
        &mut self,
        touch: Touch,
        at: [f64; 2],
        contact: Contact,
        stamp: PointerStamp,
    ) -> Option<PointerReport> {
        let state = self.touches.get_mut(&touch)?;
        state.at = at;
        let motion = if state.pressing {
            ReportedChange::Move { buttons: 0 }
        } else {
            ReportedChange::Hover { buttons: 0 }
        };
        report(touch, state, motion, contact, stamp)
    }

    /// `touch` lifted at `at`; it stays until it leaves. A lift of a touch that was not pressing
    /// is dropped.
    pub(crate) fn released(
        &mut self,
        touch: Touch,
        at: [f64; 2],
        contact: Contact,
        stamp: PointerStamp,
    ) -> Option<PointerReport> {
        let state = self.touches.get_mut(&touch)?;
        if !std::mem::replace(&mut state.pressing, false) {
            return None;
        }
        state.at = at;
        let up = ReportedChange::Up { buttons: 0 };
        report(touch, state, up, contact, stamp)
    }

    /// `touch` left, at `at` where winit says: the framework removes it as a device.
    pub(crate) fn left(
        &mut self,
        touch: Touch,
        at: Option<[f64; 2]>,
        stamp: PointerStamp,
    ) -> Option<PointerReport> {
        let mut state = self.touches.remove(&touch)?;
        if let Some(at) = at {
            state.at = at;
        }
        let departure = ReportedChange::Remove;
        report(touch, &state, departure, pressing(0.0), stamp)
    }

    /// The smallest device number from `FIRST_TOUCH_DEVICE` no touch has.
    fn free_device(&self) -> i64 {
        (FIRST_TOUCH_DEVICE..)
            .find(|device| self.touches.values().all(|state| state.device != *device))
            .expect("the touches are fewer than the numbers")
    }
}

/// `touch`'s report of `change` where `state` has it, in the view `stamp` names; `None` for a
/// window the embedder no longer has. Buttons are left at none, as Flutter's Android embedder
/// leaves them: the framework's converter supplies the primary button a touch pressing means.
fn report(
    touch: Touch,
    state: &TouchState,
    change: ReportedChange,
    contact: Contact,
    stamp: PointerStamp,
) -> Option<PointerReport> {
    Some(PointerReport {
        view_id: stamp.view?,
        time_stamp: stamp.time_stamp,
        device: state.device,
        kind: touch.kind(),
        position: state.at,
        change,
        contact,
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use inset_embedder::ViewId;

    use super::*;
    use crate::pointer::MOUSE_DEVICE;

    const FINGER: Touch = Touch::Finger(7);

    fn stamp() -> PointerStamp {
        PointerStamp {
            view: Some(ViewId(1)),
            time_stamp: Duration::ZERO,
        }
    }

    fn change(report: Option<PointerReport>) -> Option<ReportedChange> {
        report.map(|report| report.change)
    }

    #[test]
    fn a_finger_is_added_pressed_moved_lifted_and_removed() {
        let mut touches = Touches::default();
        let at = [1.0, 2.0];
        let full = pressing(1.0);
        assert_eq!(
            change(touches.entered(FINGER, at, stamp())),
            Some(ReportedChange::Add)
        );
        assert_eq!(
            change(touches.pressed(FINGER, at, full, stamp())),
            Some(ReportedChange::Down { buttons: 0 })
        );
        let moved = touches.moved(FINGER, [3.0, 2.0], full, stamp()).unwrap();
        assert_eq!(moved.change, ReportedChange::Move { buttons: 0 });
        assert_eq!(moved.position, [3.0, 2.0]);
        assert_eq!(moved.kind, PointerDeviceKind::Touch);
        assert_eq!(
            change(touches.released(FINGER, [3.0, 2.0], full, stamp())),
            Some(ReportedChange::Up { buttons: 0 })
        );
        assert_eq!(
            change(touches.left(FINGER, Some([3.0, 2.0]), stamp())),
            Some(ReportedChange::Remove)
        );
        assert!(touches.touches.is_empty());
    }

    #[test]
    fn a_finger_that_leaves_while_pressing_is_removed_where_it_was_last_seen() {
        let mut touches = Touches::default();
        touches.entered(FINGER, [0.0, 0.0], stamp());
        touches.pressed(FINGER, [4.0, 0.0], pressing(1.0), stamp());
        let departure = touches.left(FINGER, None, stamp()).unwrap();
        assert_eq!(departure.change, ReportedChange::Remove);
        assert_eq!(departure.position, [4.0, 0.0]);
    }

    #[test]
    fn each_touch_is_a_device_of_its_own_numbered_apart_from_the_mouse_and_the_trackpad() {
        let mut touches = Touches::default();
        let device = |report: Option<PointerReport>| report.unwrap().device;
        let first = device(touches.entered(Touch::Finger(0), [0.0, 0.0], stamp()));
        let second = device(touches.entered(Touch::Finger(1), [9.0, 9.0], stamp()));
        assert_eq!([first, second], [2, 3]);
        assert!(![MOUSE_DEVICE, PAN_ZOOM_DEVICE].contains(&first));
        touches.left(Touch::Finger(0), None, stamp());
        let third = device(touches.entered(Touch::Finger(5), [0.0, 0.0], stamp()));
        assert_eq!(third, 2, "a number is free again once its touch leaves");
    }

    #[test]
    fn a_pen_hovers_until_it_touches_and_is_a_stylus() {
        let mut touches = Touches::default();
        touches.entered(Touch::Pen, [0.0, 0.0], stamp());
        let hover = touches
            .moved(Touch::Pen, [1.0, 0.0], pressing(0.0), stamp())
            .unwrap();
        assert_eq!(hover.change, ReportedChange::Hover { buttons: 0 });
        assert_eq!(hover.kind, PointerDeviceKind::Stylus);
        touches.pressed(Touch::Pen, [1.0, 0.0], pressing(0.5), stamp());
        let drawn = touches
            .moved(Touch::Pen, [2.0, 0.0], pressing(0.5), stamp())
            .unwrap();
        assert_eq!(drawn.change, ReportedChange::Move { buttons: 0 });
        assert_eq!(drawn.contact.pressure, 0.5);
    }

    #[test]
    fn a_touch_the_host_never_saw_arrive_is_dropped() {
        let mut touches = Touches::default();
        let full = pressing(1.0);
        assert!(touches.moved(FINGER, [0.0, 0.0], full, stamp()).is_none());
        assert!(touches.pressed(FINGER, [0.0, 0.0], full, stamp()).is_none());
        assert!(touches.left(FINGER, None, stamp()).is_none());
    }
}
