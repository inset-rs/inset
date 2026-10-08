//! Touches as Flutter's pointers: one device per finger, numbered by the touch id the
//! system gave it, with the add before its down and the remove after its up that Flutter's
//! iOS embedder sends around every touch (`FlutterViewController.mm`, `dispatchTouches:`),
//! so the framework meets each finger the way it meets a mouse and lets go of it after.

use std::collections::HashMap;
use std::time::Duration;

use inset_embedder::{PointerChange, PointerData, PointerDataPacket, PointerDeviceKind, ViewId};
use winit::event::{Touch, TouchPhase};

use crate::pointer::PointerIds;

/// The fingers on the screen: where each was last reported, and the press it is.
#[derive(Default)]
pub(crate) struct Touches {
    fingers: HashMap<u64, Finger>,
}

struct Finger {
    last_position: [f64; 2],
    /// Flutter's `pointerIdentifier` for this press, given at the down.
    pointer_identifier: i64,
}

/// One change of one finger, as the framework takes it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct TouchChange {
    device: i64,
    change: PointerChange,
    position: [f64; 2],
    delta: [f64; 2],
    pointer_identifier: i64,
}

impl Touches {
    /// One finger's change in `view`, as the pointer that finger is: its arrival and departure
    /// go with its down and its up, in one packet. `None` for a change the framework is not
    /// told of.
    pub(crate) fn packet(
        &mut self,
        touch: &Touch,
        view: ViewId,
        time_stamp: Duration,
        ids: &mut PointerIds,
    ) -> Option<PointerDataPacket> {
        let position = [touch.location.x, touch.location.y];
        let pressure = touch.force.map_or(1.0, |force| force.normalized());
        let data: Vec<_> = self
            .apply(touch.id, touch.phase, position, ids)
            .into_iter()
            .map(|change| change.data(view, pressure, time_stamp, ids))
            .collect();
        (!data.is_empty()).then(|| PointerDataPacket::new(data))
    }

    /// The changes one touch event makes, in the order the framework takes them: a finger
    /// arriving is an add then a down, a finger leaving an up (or a cancel) then a remove.
    /// A move of a finger the embedder never saw arrive is dropped, as is a second down of one.
    fn apply(
        &mut self,
        id: u64,
        phase: TouchPhase,
        position: [f64; 2],
        ids: &mut PointerIds,
    ) -> Vec<TouchChange> {
        let device = id as i64;
        match phase {
            TouchPhase::Started => {
                if self.fingers.contains_key(&id) {
                    return Vec::new();
                }
                let pointer_identifier = ids.next_pointer();
                self.fingers.insert(
                    id,
                    Finger {
                        last_position: position,
                        pointer_identifier,
                    },
                );
                vec![
                    TouchChange::arrival(device, position),
                    TouchChange::press(device, position, pointer_identifier),
                ]
            }
            TouchPhase::Moved => {
                let Some(finger) = self.fingers.get_mut(&id) else {
                    return Vec::new();
                };
                let delta = delta_from(finger.last_position, position);
                finger.last_position = position;
                vec![TouchChange::motion(
                    device,
                    position,
                    delta,
                    finger.pointer_identifier,
                )]
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                let Some(finger) = self.fingers.remove(&id) else {
                    return Vec::new();
                };
                let change = if phase == TouchPhase::Ended {
                    PointerChange::Up
                } else {
                    PointerChange::Cancel
                };
                let delta = delta_from(finger.last_position, position);
                vec![
                    TouchChange::release(
                        device,
                        change,
                        position,
                        delta,
                        finger.pointer_identifier,
                    ),
                    TouchChange::departure(device, position),
                ]
            }
        }
    }
}

fn delta_from([last_x, last_y]: [f64; 2], [x, y]: [f64; 2]) -> [f64; 2] {
    [x - last_x, y - last_y]
}

impl TouchChange {
    /// The finger arrived at `at`: the framework adds it as a device, before its press.
    fn arrival(device: i64, at: [f64; 2]) -> TouchChange {
        TouchChange::new(device, PointerChange::Add, at, [0.0, 0.0], 0)
    }

    /// The finger went down at `at`, starting the press numbered `pointer`.
    fn press(device: i64, at: [f64; 2], pointer: i64) -> TouchChange {
        TouchChange::new(device, PointerChange::Down, at, [0.0, 0.0], pointer)
    }

    /// The finger moved to `at`, by `delta`, during the press numbered `pointer`.
    fn motion(device: i64, at: [f64; 2], delta: [f64; 2], pointer: i64) -> TouchChange {
        TouchChange::new(device, PointerChange::Move, at, delta, pointer)
    }

    /// The press numbered `pointer` ended at `at`, by `delta` since the last move: `change` is
    /// an Up, or a Cancel.
    fn release(
        device: i64,
        change: PointerChange,
        at: [f64; 2],
        delta: [f64; 2],
        pointer: i64,
    ) -> TouchChange {
        TouchChange::new(device, change, at, delta, pointer)
    }

    /// The finger left at `at`: the framework removes it as a device, after its release.
    fn departure(device: i64, at: [f64; 2]) -> TouchChange {
        TouchChange::new(device, PointerChange::Remove, at, [0.0, 0.0], 0)
    }

    fn new(
        device: i64,
        change: PointerChange,
        position: [f64; 2],
        delta: [f64; 2],
        pointer_identifier: i64,
    ) -> TouchChange {
        TouchChange {
            device,
            change,
            position,
            delta,
            pointer_identifier,
        }
    }

    /// The datum for this change in `view`, with `pressure` as the screen reports it, from
    /// none to full. Buttons are left at none, as Flutter's Android embedder leaves them:
    /// the framework's converter supplies the primary button a touch in contact means.
    fn data(
        self,
        view_id: ViewId,
        pressure: f64,
        time_stamp: Duration,
        ids: &mut PointerIds,
    ) -> PointerData {
        PointerData {
            view_id,
            embedder_id: ids.next_embedder(),
            time_stamp,
            change: self.change,
            kind: PointerDeviceKind::Touch,
            device: self.device,
            pointer_identifier: self.pointer_identifier,
            physical_x: self.position[0],
            physical_y: self.position[1],
            physical_delta_x: self.delta[0],
            physical_delta_y: self.delta[1],
            pressure,
            pressure_min: 0.0,
            pressure_max: 1.0,
            ..PointerData::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn changes(
        touches: &mut Touches,
        ids: &mut PointerIds,
        id: u64,
        phase: TouchPhase,
        at: [f64; 2],
    ) -> Vec<PointerChange> {
        touches
            .apply(id, phase, at, ids)
            .into_iter()
            .map(|change| change.change)
            .collect()
    }

    #[test]
    fn a_finger_is_added_before_its_down_and_removed_after_its_up() {
        let mut touches = Touches::default();
        let mut ids = PointerIds::default();
        assert_eq!(
            changes(&mut touches, &mut ids, 7, TouchPhase::Started, [1.0, 2.0]),
            [PointerChange::Add, PointerChange::Down]
        );
        assert_eq!(
            changes(&mut touches, &mut ids, 7, TouchPhase::Moved, [3.0, 2.0]),
            [PointerChange::Move]
        );
        assert_eq!(
            changes(&mut touches, &mut ids, 7, TouchPhase::Ended, [3.0, 2.0]),
            [PointerChange::Up, PointerChange::Remove]
        );
        assert!(touches.fingers.is_empty());
    }

    #[test]
    fn a_cancelled_finger_is_cancelled_then_removed() {
        let mut touches = Touches::default();
        let mut ids = PointerIds::default();
        touches.apply(1, TouchPhase::Started, [0.0, 0.0], &mut ids);
        assert_eq!(
            changes(&mut touches, &mut ids, 1, TouchPhase::Cancelled, [0.0, 0.0]),
            [PointerChange::Cancel, PointerChange::Remove]
        );
    }

    #[test]
    fn each_press_gets_its_own_pointer_identifier_and_its_moves_keep_it() {
        let mut touches = Touches::default();
        let mut ids = PointerIds::default();
        let first = touches.apply(1, TouchPhase::Started, [0.0, 0.0], &mut ids);
        let second = touches.apply(2, TouchPhase::Started, [9.0, 9.0], &mut ids);
        assert_eq!(first[0].pointer_identifier, 0, "an add is no press");
        assert_eq!(first[1].pointer_identifier, 1);
        assert_eq!(second[1].pointer_identifier, 2);
        let moved = touches.apply(1, TouchPhase::Moved, [4.0, 0.0], &mut ids);
        assert_eq!(moved[0].pointer_identifier, 1);
        assert_eq!(moved[0].delta, [4.0, 0.0]);
        assert_eq!(moved[0].device, 1);
    }

    #[test]
    fn a_finger_the_host_never_saw_arrive_is_dropped() {
        let mut touches = Touches::default();
        let mut ids = PointerIds::default();
        assert!(
            touches
                .apply(3, TouchPhase::Moved, [0.0, 0.0], &mut ids)
                .is_empty()
        );
        assert!(
            touches
                .apply(3, TouchPhase::Ended, [0.0, 0.0], &mut ids)
                .is_empty()
        );
    }
}
