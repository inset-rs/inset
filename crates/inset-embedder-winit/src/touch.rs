//! Touches as Flutter's pointers: one device per finger, numbered by the touch id the
//! system gave it, with the add before its down and the remove after its up that Flutter's
//! iOS embedder sends around every touch (`FlutterViewController.mm`, `dispatchTouches:`),
//! so the framework meets each finger the way it meets a mouse and lets go of it after.

use std::collections::HashMap;
use std::time::Duration;

use inset_embedder::{PointerChange, PointerData, PointerDeviceKind, ViewId};
use winit::event::TouchPhase;

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
pub(crate) struct TouchChange {
    pub(crate) device: i64,
    pub(crate) change: PointerChange,
    pub(crate) position: [f64; 2],
    pub(crate) delta: [f64; 2],
    pub(crate) pointer_identifier: i64,
}

impl Touches {
    /// The changes one touch event makes, in the order the framework takes them: a finger
    /// arriving is an add then a down, a finger leaving an up (or a cancel) then a remove.
    /// A move of a finger the host never saw arrive is dropped, as is a second down of one.
    pub(crate) fn apply(
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
                    TouchChange {
                        device,
                        change: PointerChange::Add,
                        position,
                        delta: [0.0, 0.0],
                        pointer_identifier: 0,
                    },
                    TouchChange {
                        device,
                        change: PointerChange::Down,
                        position,
                        delta: [0.0, 0.0],
                        pointer_identifier,
                    },
                ]
            }
            TouchPhase::Moved => {
                let Some(finger) = self.fingers.get_mut(&id) else {
                    return Vec::new();
                };
                let delta = delta_from(finger.last_position, position);
                finger.last_position = position;
                vec![TouchChange {
                    device,
                    change: PointerChange::Move,
                    position,
                    delta,
                    pointer_identifier: finger.pointer_identifier,
                }]
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
                    TouchChange {
                        device,
                        change,
                        position,
                        delta,
                        pointer_identifier: finger.pointer_identifier,
                    },
                    TouchChange {
                        device,
                        change: PointerChange::Remove,
                        position,
                        delta: [0.0, 0.0],
                        pointer_identifier: 0,
                    },
                ]
            }
        }
    }
}

fn delta_from([last_x, last_y]: [f64; 2], [x, y]: [f64; 2]) -> [f64; 2] {
    [x - last_x, y - last_y]
}

impl TouchChange {
    /// The datum for this change in `view`, with `pressure` as the screen reports it, from
    /// none to full. Buttons are left at none, as Flutter's Android embedder leaves them:
    /// the framework's converter supplies the primary button a touch in contact means.
    pub(crate) fn data(
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
