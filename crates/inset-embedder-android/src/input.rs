//! A `MotionEvent` as the framework takes it: the engine's `AndroidTouchProcessor` walk.
//!
//! The engine sends no `Add`, because its own packet converter mints one before a device's
//! first event. The framework's converter here is `gestures/converter.dart`, which has no
//! such step, so the host sends the `Add` itself, as it does for a touch on iOS.

use std::collections::HashMap;
use std::time::Duration;

use android_activity::input::{Axis, MotionAction, MotionEvent, Pointer, ToolType};
use inset_embedder::{
    PointerChange, PointerData, PointerDataPacket, PointerDeviceKind, PointerSignalKind, ViewId,
};

use crate::pointer::{buttons_of, device_id, kind_of_tool, scroll_delta, tool};

/// The devices the framework has been told about, and the numbers its data carries.
#[derive(Default)]
pub(crate) struct Pointers {
    /// Where each device was last reported, so a move carries its delta.
    seen: HashMap<i64, [f64; 2]>,
    /// Flutter's `pointerIdentifier`, one per press, and the per-datum number.
    pressing: HashMap<i64, i64>,
    next_pointer: i64,
    next_embedder: i64,
}

impl Pointers {
    /// The data one motion event makes, in the order the framework takes it.
    pub(crate) fn apply(&mut self, event: &MotionEvent<'_>, view_id: ViewId) -> PointerDataPacket {
        // The NDK reports event times in nanoseconds, on `System.nanoTime`'s scale, where
        // the Java API reports milliseconds. The framework's velocity tracker measures a
        // fling from these, so the unit decides whether a fling happens at all.
        let time_stamp = Duration::from_nanos(event.event_time().max(0) as u64);
        let button_state = event.button_state().0 as i64;
        let mut data = Vec::new();
        match event.action() {
            MotionAction::Down | MotionAction::PointerDown => {
                let pointer = event.pointer_at_index(event.pointer_index());
                self.arrive(&mut data, &pointer, view_id, button_state, time_stamp);
            }
            MotionAction::Up | MotionAction::PointerUp => {
                // An up carries where the other fingers are now, which the engine turns
                // into moves so that position is not lost, and sends before the up.
                let lifted = event.pointer_index();
                for (index, pointer) in event.pointers().enumerate() {
                    if index != lifted && kind_of(&pointer) == PointerDeviceKind::Touch {
                        self.moved(
                            &mut data,
                            &pointer,
                            view_id,
                            button_state,
                            time_stamp,
                            PointerChange::Move,
                        );
                    }
                }
                let pointer = event.pointer_at_index(lifted);
                self.leave(
                    &mut data,
                    &pointer,
                    view_id,
                    button_state,
                    time_stamp,
                    PointerChange::Up,
                );
            }
            MotionAction::Cancel => {
                for pointer in event.pointers() {
                    self.leave(
                        &mut data,
                        &pointer,
                        view_id,
                        button_state,
                        time_stamp,
                        PointerChange::Cancel,
                    );
                }
            }
            MotionAction::Move => {
                for pointer in event.pointers() {
                    self.moved(
                        &mut data,
                        &pointer,
                        view_id,
                        button_state,
                        time_stamp,
                        PointerChange::Move,
                    );
                }
            }
            MotionAction::HoverMove => {
                let pointer = event.pointer_at_index(event.pointer_index());
                self.hover(&mut data, &pointer, view_id, button_state, time_stamp);
            }
            MotionAction::Scroll => {
                let pointer = event.pointer_at_index(event.pointer_index());
                let delta = scroll_delta(
                    pointer.axis_value(Axis::Hscroll),
                    pointer.axis_value(Axis::Vscroll),
                );
                let mut datum = self.datum(
                    &pointer,
                    view_id,
                    PointerChange::Hover,
                    button_state,
                    time_stamp,
                );
                datum.signal_kind = Some(PointerSignalKind::Scroll);
                datum.scroll_delta_x = delta[0];
                datum.scroll_delta_y = delta[1];
                data.push(datum);
            }
            _ => {}
        }
        PointerDataPacket::new(data)
    }

    /// A device arriving: the framework meets it, then it goes down.
    fn arrive(
        &mut self,
        data: &mut Vec<PointerData>,
        pointer: &Pointer<'_>,
        view_id: ViewId,
        button_state: i64,
        time_stamp: Duration,
    ) {
        let device = device_of(pointer);
        if !self.seen.contains_key(&device) {
            data.push(self.datum(pointer, view_id, PointerChange::Add, 0, time_stamp));
        }
        self.next_pointer += 1;
        self.pressing.insert(device, self.next_pointer);
        data.push(self.datum(
            pointer,
            view_id,
            PointerChange::Down,
            button_state,
            time_stamp,
        ));
    }

    /// A device leaving: it goes up, and a finger is gone from the screen with it. A mouse
    /// or a stylus stays, since it can hover.
    fn leave(
        &mut self,
        data: &mut Vec<PointerData>,
        pointer: &Pointer<'_>,
        view_id: ViewId,
        button_state: i64,
        time_stamp: Duration,
        change: PointerChange,
    ) {
        let device = device_of(pointer);
        data.push(self.datum(pointer, view_id, change, button_state, time_stamp));
        self.pressing.remove(&device);
        if kind_of(pointer) == PointerDeviceKind::Touch {
            data.push(self.datum(pointer, view_id, PointerChange::Remove, 0, time_stamp));
            self.seen.remove(&device);
        }
    }

    fn moved(
        &mut self,
        data: &mut Vec<PointerData>,
        pointer: &Pointer<'_>,
        view_id: ViewId,
        button_state: i64,
        time_stamp: Duration,
        change: PointerChange,
    ) {
        if !self.seen.contains_key(&device_of(pointer)) {
            return;
        }
        data.push(self.datum(pointer, view_id, change, button_state, time_stamp));
    }

    /// A pointer moving without touching: the framework meets it on its first hover.
    fn hover(
        &mut self,
        data: &mut Vec<PointerData>,
        pointer: &Pointer<'_>,
        view_id: ViewId,
        button_state: i64,
        time_stamp: Duration,
    ) {
        if !self.seen.contains_key(&device_of(pointer)) {
            data.push(self.datum(pointer, view_id, PointerChange::Add, 0, time_stamp));
        }
        data.push(self.datum(
            pointer,
            view_id,
            PointerChange::Hover,
            button_state,
            time_stamp,
        ));
    }

    /// One datum, with the delta from where this device was last reported.
    fn datum(
        &mut self,
        pointer: &Pointer<'_>,
        view_id: ViewId,
        change: PointerChange,
        button_state: i64,
        time_stamp: Duration,
    ) -> PointerData {
        let device = device_of(pointer);
        let kind = kind_of(pointer);
        let position = [f64::from(pointer.x()), f64::from(pointer.y())];
        let last = self.seen.insert(device, position).unwrap_or(position);
        self.next_embedder += 1;
        let stylus = matches!(
            kind,
            PointerDeviceKind::Stylus | PointerDeviceKind::InvertedStylus
        );
        PointerData {
            view_id,
            embedder_id: self.next_embedder,
            time_stamp,
            change,
            kind,
            device,
            // An add or a remove is no press, so it carries no press number.
            pointer_identifier: match change {
                PointerChange::Add | PointerChange::Remove => 0,
                _ => self.pressing.get(&device).copied().unwrap_or(0),
            },
            physical_x: position[0],
            physical_y: position[1],
            physical_delta_x: position[0] - last[0],
            physical_delta_y: position[1] - last[1],
            buttons: buttons_of(button_state, kind),
            pressure: f64::from(pointer.pressure()),
            pressure_min: 0.0,
            pressure_max: 1.0,
            size: f64::from(pointer.size()),
            radius_major: f64::from(pointer.tool_major()),
            radius_minor: f64::from(pointer.tool_minor()),
            orientation: f64::from(pointer.orientation()),
            // Only a stylus reports how far it is from the screen and how it leans.
            tilt: if stylus {
                f64::from(pointer.axis_value(Axis::Tilt))
            } else {
                0.0
            },
            distance: if stylus {
                f64::from(pointer.axis_value(Axis::Distance))
            } else {
                0.0
            },
            ..PointerData::default()
        }
    }
}

fn tool_of(pointer: &Pointer<'_>) -> u32 {
    match pointer.tool_type() {
        ToolType::Finger => tool::FINGER,
        ToolType::Stylus => tool::STYLUS,
        ToolType::Mouse => tool::MOUSE,
        ToolType::Eraser => tool::ERASER,
        // `TOOL_TYPE_UNKNOWN`, and whatever else a device reports.
        _ => 0,
    }
}

fn device_of(pointer: &Pointer<'_>) -> i64 {
    device_id(pointer.pointer_id(), tool_of(pointer))
}

fn kind_of(pointer: &Pointer<'_>) -> PointerDeviceKind {
    kind_of_tool(tool_of(pointer))
}
