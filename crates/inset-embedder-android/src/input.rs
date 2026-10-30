//! A `MotionEvent` as the framework takes it: the engine's `AndroidTouchProcessor` walk. As
//! there, a device's add, each press's identifier and the deltas are the converter's
//! (`PointerDataPacketConverter`), which the client runs.

use std::time::Duration;

use android_activity::input::{Axis, MotionAction, MotionEvent, Pointer, ToolType};
use inset_embedder::{Contact, PointerDeviceKind, PointerReport, ReportedChange, ViewId};

use crate::pointer::{buttons_of, device_id, kind_of_tool, scroll_delta, tool};

/// The reports one motion event makes, in the order the framework takes them.
pub(crate) fn reports_of(event: &MotionEvent<'_>, view_id: ViewId) -> Vec<PointerReport> {
    // The NDK reports event times in nanoseconds, on `System.nanoTime`'s scale, where
    // the Java API reports milliseconds. The framework's velocity tracker measures a
    // fling from these, so the unit decides whether a fling happens at all.
    let time_stamp = Duration::from_nanos(event.event_time().max(0) as u64);
    let button_state = event.button_state().0 as i64;
    let buttons = |pointer: &Pointer<'_>| buttons_of(button_state, kind_of(pointer));
    let mut reports = Vec::new();
    match event.action() {
        MotionAction::Down | MotionAction::PointerDown => {
            let pointer = event.pointer_at_index(event.pointer_index());
            let down = ReportedChange::Down {
                buttons: buttons(&pointer),
            };
            reports.push(report(&pointer, view_id, down, time_stamp));
        }
        MotionAction::Up | MotionAction::PointerUp => {
            // An up carries where the other fingers are now, which the engine turns
            // into moves so that position is not lost, and sends before the up.
            let lifted = event.pointer_index();
            for (index, pointer) in event.pointers().enumerate() {
                if index != lifted && kind_of(&pointer) == PointerDeviceKind::Touch {
                    let moved = ReportedChange::Move {
                        buttons: buttons(&pointer),
                    };
                    reports.push(report(&pointer, view_id, moved, time_stamp));
                }
            }
            let pointer = event.pointer_at_index(lifted);
            let up = ReportedChange::Up {
                buttons: buttons(&pointer),
            };
            leave(&mut reports, &pointer, view_id, up, time_stamp);
        }
        MotionAction::Cancel => {
            for pointer in event.pointers() {
                let cancel = ReportedChange::Cancel {
                    buttons: buttons(&pointer),
                };
                leave(&mut reports, &pointer, view_id, cancel, time_stamp);
            }
        }
        MotionAction::Move => {
            for pointer in event.pointers() {
                let moved = ReportedChange::Move {
                    buttons: buttons(&pointer),
                };
                reports.push(report(&pointer, view_id, moved, time_stamp));
            }
        }
        MotionAction::HoverMove => {
            let pointer = event.pointer_at_index(event.pointer_index());
            let hover = ReportedChange::Hover {
                buttons: buttons(&pointer),
            };
            reports.push(report(&pointer, view_id, hover, time_stamp));
        }
        MotionAction::Scroll => {
            let pointer = event.pointer_at_index(event.pointer_index());
            let delta = scroll_delta(
                pointer.axis_value(Axis::Hscroll),
                pointer.axis_value(Axis::Vscroll),
            );
            let scroll = ReportedChange::Scroll { delta };
            reports.push(report(&pointer, view_id, scroll, time_stamp));
        }
        _ => {}
    }
    reports
}

/// A device leaving with `change`, its up or its cancel: a finger is gone from the screen
/// with it, and is removed. A mouse or a stylus stays, since it can hover.
fn leave(
    reports: &mut Vec<PointerReport>,
    pointer: &Pointer<'_>,
    view_id: ViewId,
    change: ReportedChange,
    time_stamp: Duration,
) {
    reports.push(report(pointer, view_id, change, time_stamp));
    if kind_of(pointer) == PointerDeviceKind::Touch {
        reports.push(report(pointer, view_id, ReportedChange::Remove, time_stamp));
    }
}

/// `pointer`'s report of `change`, where it is and how it meets the screen.
fn report(
    pointer: &Pointer<'_>,
    view_id: ViewId,
    change: ReportedChange,
    time_stamp: Duration,
) -> PointerReport {
    PointerReport {
        view_id,
        time_stamp,
        device: device_of(pointer),
        kind: kind_of(pointer),
        position: [f64::from(pointer.x()), f64::from(pointer.y())],
        change,
        contact: contact_of(pointer),
    }
}

/// How `pointer` meets the screen.
fn contact_of(pointer: &Pointer<'_>) -> Contact {
    let stylus = matches!(
        kind_of(pointer),
        PointerDeviceKind::Stylus | PointerDeviceKind::InvertedStylus
    );
    Contact {
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
        ..Contact::default()
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
