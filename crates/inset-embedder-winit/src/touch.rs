//! Fingers and pens as Flutter's pointers: one device each, the add, down, move, up and remove
//! Flutter's iOS embedder sends for every touch (`FlutterViewController.mm`, `dispatchTouches:`),
//! which winit reports in that order as a contact entering, pressing, moving, releasing and
//! leaving, so the framework meets each finger the way it meets a mouse and lets go of it after.
//! A contact that leaves still pressed was cancelled, and is cancelled before it is removed. A pen
//! is a stylus, as Flutter's iOS and Windows embedders report it, and hovers where the system
//! reports it near the screen.

use std::collections::HashMap;

use inset_embedder::{PointerChange, PointerData, PointerDataPacket, PointerDeviceKind, ViewId};
use winit::event::{ButtonSource, Force, PointerKind, PointerSource, TabletToolButton};

use crate::pointer::{PointerIds, PointerStamp};

/// The device of the pen: winit names no pen, and a finger's device, the system's id of the
/// touch, is never negative.
const PEN_DEVICE: i64 = -1;

/// A finger or a pen, as the pointer device it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Contact {
    device: i64,
    kind: PointerDeviceKind,
}

impl Contact {
    /// The contact a pointer of `kind` is; `None` for the mouse, and for a pointer winit cannot
    /// name, which the mouse stands for.
    pub(crate) fn of(kind: PointerKind) -> Option<Contact> {
        match kind {
            PointerKind::Touch(finger) => Some(Contact::finger(finger.into_raw())),
            PointerKind::TabletTool(_) => Some(Contact::pen()),
            _ => None,
        }
    }

    /// The contact that moved, with how hard it presses, from the source of a move.
    pub(crate) fn moved_from(source: &PointerSource) -> Option<(Contact, f64)> {
        match source {
            PointerSource::Touch { finger_id, force } => {
                Some((Contact::finger(finger_id.into_raw()), pressure(*force)))
            }
            PointerSource::TabletTool { data, .. } => Some((Contact::pen(), pen_pressure(data))),
            _ => None,
        }
    }

    /// The contact a button event presses or lifts, with how hard it presses: a finger, or a
    /// pen's tip. `None` for a mouse's button, and for a pen's other buttons.
    pub(crate) fn pressed_by(button: &ButtonSource) -> Option<(Contact, f64)> {
        match button {
            ButtonSource::Touch { finger_id, force } => {
                Some((Contact::finger(finger_id.into_raw()), pressure(*force)))
            }
            ButtonSource::TabletTool {
                button: TabletToolButton::Contact,
                data,
                ..
            } => Some((Contact::pen(), pen_pressure(data))),
            _ => None,
        }
    }

    fn finger(id: usize) -> Contact {
        Contact {
            device: id as i64,
            kind: PointerDeviceKind::Touch,
        }
    }

    fn pen() -> Contact {
        Contact {
            device: PEN_DEVICE,
            kind: PointerDeviceKind::Stylus,
        }
    }
}

/// How hard a finger presses, from none to full; a screen that cannot tell reports full.
fn pressure(force: Option<Force>) -> f64 {
    force.map_or(1.0, |force| force.normalized(None))
}

/// How hard a pen presses, its force taken along its own axis where it reports its angle.
fn pen_pressure(data: &winit::event::TabletToolData) -> f64 {
    data.force
        .map_or(1.0, |force| force.normalized(data.clone().angle()))
}

/// The contacts on the screen, or near it: where each was last reported, and the press it is in.
#[derive(Default)]
pub(crate) struct Touches {
    contacts: HashMap<i64, ContactState>,
}

struct ContactState {
    last_position: [f64; 2],
    /// Flutter's `pointerIdentifier` of the press it is in, given at the down; `None` while it
    /// hovers.
    press: Option<i64>,
}

/// One change of one contact, as the framework takes it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ContactChange {
    device: i64,
    kind: PointerDeviceKind,
    change: PointerChange,
    position: [f64; 2],
    delta: [f64; 2],
    pointer_identifier: i64,
    pressure: f64,
}

impl Touches {
    /// `contact` came to `at`: the framework adds it as a device, before anything else about it.
    /// A second arrival is dropped.
    pub(crate) fn entered(
        &mut self,
        contact: Contact,
        at: [f64; 2],
        stamp: PointerStamp,
        ids: &mut PointerIds,
    ) -> Option<PointerDataPacket> {
        if self.contacts.contains_key(&contact.device) {
            return None;
        }
        self.contacts.insert(
            contact.device,
            ContactState {
                last_position: at,
                press: None,
            },
        );
        let arrival = ContactChange::new(contact, PointerChange::Add, at, [0.0, 0.0], 0, 0.0);
        packet([arrival], stamp, ids)
    }

    /// `contact` pressed at `at`, starting a press of its own. A press of a contact the embedder
    /// never saw arrive, or of one already pressing, is dropped.
    pub(crate) fn pressed(
        &mut self,
        contact: Contact,
        at: [f64; 2],
        pressure: f64,
        stamp: PointerStamp,
        ids: &mut PointerIds,
    ) -> Option<PointerDataPacket> {
        let state = self.contacts.get_mut(&contact.device)?;
        if state.press.is_some() {
            return None;
        }
        let press = ids.next_pointer();
        state.press = Some(press);
        let delta = state.moved_to(at);
        let down = ContactChange::new(contact, PointerChange::Down, at, delta, press, pressure);
        packet([down], stamp, ids)
    }

    /// `contact` moved to `at`: a move while it presses, a hover otherwise. A move of a contact
    /// the embedder never saw arrive is dropped.
    pub(crate) fn moved(
        &mut self,
        contact: Contact,
        at: [f64; 2],
        pressure: f64,
        stamp: PointerStamp,
        ids: &mut PointerIds,
    ) -> Option<PointerDataPacket> {
        let state = self.contacts.get_mut(&contact.device)?;
        let delta = state.moved_to(at);
        let motion = match state.press {
            Some(press) => {
                ContactChange::new(contact, PointerChange::Move, at, delta, press, pressure)
            }
            None => ContactChange::new(contact, PointerChange::Hover, at, delta, 0, pressure),
        };
        packet([motion], stamp, ids)
    }

    /// `contact` lifted at `at`, ending its press; it stays until it leaves. A lift of a contact
    /// that was not pressing is dropped.
    pub(crate) fn released(
        &mut self,
        contact: Contact,
        at: [f64; 2],
        pressure: f64,
        stamp: PointerStamp,
        ids: &mut PointerIds,
    ) -> Option<PointerDataPacket> {
        let state = self.contacts.get_mut(&contact.device)?;
        let press = state.press.take()?;
        let delta = state.moved_to(at);
        let up = ContactChange::new(contact, PointerChange::Up, at, delta, press, pressure);
        packet([up], stamp, ids)
    }

    /// `contact` left, at `at` where winit says: the framework removes it as a device, after
    /// cancelling the press it was still in.
    pub(crate) fn left(
        &mut self,
        contact: Contact,
        at: Option<[f64; 2]>,
        stamp: PointerStamp,
        ids: &mut PointerIds,
    ) -> Option<PointerDataPacket> {
        let mut state = self.contacts.remove(&contact.device)?;
        let at = at.unwrap_or(state.last_position);
        let delta = state.moved_to(at);
        let departure = ContactChange::new(contact, PointerChange::Remove, at, [0.0, 0.0], 0, 0.0);
        let changes = match state.press {
            Some(press) => vec![
                ContactChange::new(contact, PointerChange::Cancel, at, delta, press, 0.0),
                departure,
            ],
            None => vec![departure],
        };
        packet(changes, stamp, ids)
    }
}

impl ContactState {
    /// Takes `at` as where the contact is, and answers how far it came since it was last reported.
    fn moved_to(&mut self, [x, y]: [f64; 2]) -> [f64; 2] {
        let [last_x, last_y] = std::mem::replace(&mut self.last_position, [x, y]);
        [x - last_x, y - last_y]
    }
}

/// The packet of `changes` in the view `stamp` names; `None` for a window the embedder no longer
/// has.
fn packet(
    changes: impl IntoIterator<Item = ContactChange>,
    stamp: PointerStamp,
    ids: &mut PointerIds,
) -> Option<PointerDataPacket> {
    let view = stamp.view?;
    let data = changes
        .into_iter()
        .map(|change| change.data(view, stamp, ids))
        .collect();
    Some(PointerDataPacket::new(data))
}

impl ContactChange {
    fn new(
        contact: Contact,
        change: PointerChange,
        position: [f64; 2],
        delta: [f64; 2],
        pointer_identifier: i64,
        pressure: f64,
    ) -> ContactChange {
        ContactChange {
            device: contact.device,
            kind: contact.kind,
            change,
            position,
            delta,
            pointer_identifier,
            pressure,
        }
    }

    /// The datum for this change in `view`. Buttons are left at none, as Flutter's Android
    /// embedder leaves them: the framework's converter supplies the primary button a contact
    /// pressing means.
    fn data(self, view_id: ViewId, stamp: PointerStamp, ids: &mut PointerIds) -> PointerData {
        PointerData {
            view_id,
            embedder_id: ids.next_embedder(),
            time_stamp: stamp.time_stamp,
            change: self.change,
            kind: self.kind,
            device: self.device,
            pointer_identifier: self.pointer_identifier,
            physical_x: self.position[0],
            physical_y: self.position[1],
            physical_delta_x: self.delta[0],
            physical_delta_y: self.delta[1],
            pressure: self.pressure,
            pressure_min: 0.0,
            pressure_max: 1.0,
            ..PointerData::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    const FINGER: Contact = Contact {
        device: 7,
        kind: PointerDeviceKind::Touch,
    };

    fn stamp() -> PointerStamp {
        PointerStamp {
            view: Some(ViewId(1)),
            time_stamp: Duration::ZERO,
        }
    }

    fn data(packet: Option<PointerDataPacket>) -> Vec<PointerData> {
        packet.map(|packet| packet.data).unwrap_or_default()
    }

    fn changes(packet: Option<PointerDataPacket>) -> Vec<PointerChange> {
        data(packet).iter().map(|datum| datum.change).collect()
    }

    #[test]
    fn a_finger_is_added_pressed_moved_lifted_and_removed() {
        let mut touches = Touches::default();
        let mut ids = PointerIds::default();
        let at = [1.0, 2.0];
        assert_eq!(
            changes(touches.entered(FINGER, at, stamp(), &mut ids)),
            [PointerChange::Add]
        );
        assert_eq!(
            changes(touches.pressed(FINGER, at, 1.0, stamp(), &mut ids)),
            [PointerChange::Down]
        );
        let moved = data(touches.moved(FINGER, [3.0, 2.0], 1.0, stamp(), &mut ids));
        assert_eq!(moved[0].change, PointerChange::Move);
        assert_eq!(moved[0].physical_delta_x, 2.0);
        assert_eq!(
            changes(touches.released(FINGER, [3.0, 2.0], 1.0, stamp(), &mut ids)),
            [PointerChange::Up]
        );
        assert_eq!(
            changes(touches.left(FINGER, Some([3.0, 2.0]), stamp(), &mut ids)),
            [PointerChange::Remove]
        );
        assert!(touches.contacts.is_empty());
    }

    #[test]
    fn a_finger_that_leaves_while_pressing_is_cancelled_then_removed() {
        let mut touches = Touches::default();
        let mut ids = PointerIds::default();
        touches.entered(FINGER, [0.0, 0.0], stamp(), &mut ids);
        touches.pressed(FINGER, [0.0, 0.0], 1.0, stamp(), &mut ids);
        assert_eq!(
            changes(touches.left(FINGER, None, stamp(), &mut ids)),
            [PointerChange::Cancel, PointerChange::Remove]
        );
    }

    #[test]
    fn each_press_gets_its_own_pointer_identifier_and_its_moves_keep_it() {
        let mut touches = Touches::default();
        let mut ids = PointerIds::default();
        let second = Contact {
            device: 8,
            ..FINGER
        };
        let added = data(touches.entered(FINGER, [0.0, 0.0], stamp(), &mut ids));
        assert_eq!(added[0].pointer_identifier, 0, "an add is no press");
        touches.entered(second, [9.0, 9.0], stamp(), &mut ids);
        let first_down = data(touches.pressed(FINGER, [0.0, 0.0], 1.0, stamp(), &mut ids));
        let second_down = data(touches.pressed(second, [9.0, 9.0], 1.0, stamp(), &mut ids));
        assert_eq!(first_down[0].pointer_identifier, 1);
        assert_eq!(second_down[0].pointer_identifier, 2);
        let moved = data(touches.moved(FINGER, [4.0, 0.0], 1.0, stamp(), &mut ids));
        assert_eq!(moved[0].pointer_identifier, 1);
        assert_eq!(moved[0].device, 7);
    }

    #[test]
    fn a_pen_hovers_until_it_touches_and_is_a_stylus() {
        let mut touches = Touches::default();
        let mut ids = PointerIds::default();
        let pen = Contact::pen();
        touches.entered(pen, [0.0, 0.0], stamp(), &mut ids);
        let hover = data(touches.moved(pen, [1.0, 0.0], 0.0, stamp(), &mut ids));
        assert_eq!(hover[0].change, PointerChange::Hover);
        assert_eq!(hover[0].kind, PointerDeviceKind::Stylus);
        assert_eq!(hover[0].pointer_identifier, 0);
        touches.pressed(pen, [1.0, 0.0], 0.5, stamp(), &mut ids);
        let drawn = data(touches.moved(pen, [2.0, 0.0], 0.5, stamp(), &mut ids));
        assert_eq!(drawn[0].change, PointerChange::Move);
        assert_eq!(drawn[0].pressure, 0.5);
    }

    #[test]
    fn a_contact_the_host_never_saw_arrive_is_dropped() {
        let mut touches = Touches::default();
        let mut ids = PointerIds::default();
        assert!(
            touches
                .moved(FINGER, [0.0, 0.0], 1.0, stamp(), &mut ids)
                .is_none()
        );
        assert!(
            touches
                .pressed(FINGER, [0.0, 0.0], 1.0, stamp(), &mut ids)
                .is_none()
        );
        assert!(touches.left(FINGER, None, stamp(), &mut ids).is_none());
    }
}
