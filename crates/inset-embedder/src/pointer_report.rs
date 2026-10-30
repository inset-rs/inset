//! Flutter counterpart: the embedder API's `FlutterPointerEvent`
//! (`engine/src/flutter/shell/platform/embedder/embedder.h`) and its copy into `PointerData` in
//! `FlutterEngineSendPointerEvent` (`embedder.cc`).

use std::time::Duration;

use crate::{PointerChange, PointerData, PointerDeviceKind, PointerSignalKind, ViewId};

/// What a host saw one pointing device do, as the system reported it.
///
/// It holds only what the system says. What the framework also needs, each press's identifier,
/// how far the device came since the framework last heard of it, and the changes the system left
/// out, the client adds ([`PointerDataPacketConverter`](crate::PointerDataPacketConverter)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerReport {
    /// The view the device is over.
    pub view_id: ViewId,

    /// When the system reported it, on the host's timeline.
    pub time_stamp: Duration,

    /// The device: one number per mouse, finger, pen or trackpad, the same for each of its
    /// presses, and different from every other device the host reports.
    pub device: i64,

    /// The kind of device.
    pub kind: PointerDeviceKind,

    /// Where the device is, in the view's physical pixels.
    pub position: [f64; 2],

    /// What the device did.
    pub change: ReportedChange,

    /// How a finger or a pen meets the surface, where the system says.
    pub contact: Contact,
}

/// How a device changed, with what the framework takes of that change.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReportedChange {
    /// The device started being tracked, such as the mouse coming into the view.
    Add,

    /// The device is no longer tracked.
    Remove,

    /// It moved without touching, with `buttons` held, such as a pen's barrel button.
    Hover { buttons: i64 },

    /// It touched, or its first button went down, with `buttons` held.
    Down { buttons: i64 },

    /// It moved while touching, or the buttons it holds changed, to `buttons`.
    Move { buttons: i64 },

    /// It stopped touching, or its last button was released, with `buttons` still held.
    Up { buttons: i64 },

    /// The system took its press away, with `buttons` held.
    Cancel { buttons: i64 },

    /// A wheel or a trackpad scrolled by `delta`, in physical pixels, positive towards the
    /// content's end.
    Scroll { delta: [f64; 2] },

    /// A touch stopped the system's own scrolling momentum.
    ScrollInertiaCancel,

    /// A zoom by `scale`, such as a browser's for a wheel turned with Control held.
    Scale { scale: f64 },

    /// A trackpad gesture, panning, zooming and rotating together, started.
    PanZoomStart,

    /// The gesture so far, each the total since it started: `pan` in physical pixels, `scale`
    /// from 1.0, and `rotation` in radians, clockwise, from 0.0.
    PanZoomUpdate {
        pan: [f64; 2],
        scale: f64,
        rotation: f64,
    },

    /// The gesture ended.
    PanZoomEnd,
}

/// How a finger or a pen meets the surface: [`PointerData`]'s fields of the same names. The
/// default is a device that detects none of it, such as a mouse, whose pressure reads 1.0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    pub pressure: f64,
    pub pressure_min: f64,
    pub pressure_max: f64,
    pub distance: f64,
    pub distance_max: f64,
    pub size: f64,
    pub radius_major: f64,
    pub radius_minor: f64,
    pub radius_min: f64,
    pub radius_max: f64,
    pub orientation: f64,
    pub tilt: f64,
}

impl Default for Contact {
    fn default() -> Contact {
        Contact {
            pressure: 1.0,
            pressure_min: 1.0,
            pressure_max: 1.0,
            distance: 0.0,
            distance_max: 0.0,
            size: 0.0,
            radius_major: 0.0,
            radius_minor: 0.0,
            radius_min: 0.0,
            radius_max: 0.0,
            orientation: 0.0,
            tilt: 0.0,
        }
    }
}

impl From<PointerReport> for PointerData {
    /// The datum the converter starts from: what the system said, with what only the converter
    /// knows, the press's identifier, the deltas and `synthesized`, left at zero, and no
    /// `embedder_id`, as `embedder.cc` gives none.
    fn from(report: PointerReport) -> PointerData {
        let contact = report.contact;
        let mut data = PointerData {
            view_id: report.view_id,
            time_stamp: report.time_stamp,
            kind: report.kind,
            device: report.device,
            physical_x: report.position[0],
            physical_y: report.position[1],
            pressure: contact.pressure,
            pressure_min: contact.pressure_min,
            pressure_max: contact.pressure_max,
            distance: contact.distance,
            distance_max: contact.distance_max,
            size: contact.size,
            radius_major: contact.radius_major,
            radius_minor: contact.radius_minor,
            radius_min: contact.radius_min,
            radius_max: contact.radius_max,
            orientation: contact.orientation,
            tilt: contact.tilt,
            ..PointerData::default()
        };
        report.change.write_into(&mut data);
        data
    }
}

impl ReportedChange {
    /// Sets `data`'s change, and the fields this change carries.
    fn write_into(self, data: &mut PointerData) {
        let (change, buttons) = match self {
            ReportedChange::Add => (PointerChange::Add, 0),
            ReportedChange::Remove => (PointerChange::Remove, 0),
            ReportedChange::Hover { buttons } => (PointerChange::Hover, buttons),
            ReportedChange::Down { buttons } => (PointerChange::Down, buttons),
            ReportedChange::Move { buttons } => (PointerChange::Move, buttons),
            ReportedChange::Up { buttons } => (PointerChange::Up, buttons),
            ReportedChange::Cancel { buttons } => (PointerChange::Cancel, buttons),
            // A signal is no change of the device, which the framework reads as a hover.
            ReportedChange::Scroll { delta } => {
                data.signal_kind = Some(PointerSignalKind::Scroll);
                [data.scroll_delta_x, data.scroll_delta_y] = delta;
                (PointerChange::Hover, 0)
            }
            ReportedChange::ScrollInertiaCancel => {
                data.signal_kind = Some(PointerSignalKind::ScrollInertiaCancel);
                (PointerChange::Hover, 0)
            }
            ReportedChange::Scale { scale } => {
                data.signal_kind = Some(PointerSignalKind::Scale);
                data.scale = scale;
                (PointerChange::Hover, 0)
            }
            ReportedChange::PanZoomStart => (PointerChange::PanZoomStart, 0),
            ReportedChange::PanZoomUpdate {
                pan,
                scale,
                rotation,
            } => {
                [data.pan_x, data.pan_y] = pan;
                data.scale = scale;
                data.rotation = rotation;
                (PointerChange::PanZoomUpdate, 0)
            }
            ReportedChange::PanZoomEnd => (PointerChange::PanZoomEnd, 0),
        };
        data.change = change;
        data.buttons = buttons;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(change: ReportedChange) -> PointerReport {
        PointerReport {
            view_id: ViewId(3),
            time_stamp: Duration::from_millis(5),
            device: 2,
            kind: PointerDeviceKind::Mouse,
            position: [10.0, 20.0],
            change,
            contact: Contact::default(),
        }
    }

    #[test]
    fn a_report_is_the_datum_with_what_only_the_converter_knows_left_at_zero() {
        let data = PointerData::from(report(ReportedChange::Down { buttons: 1 }));
        assert_eq!(data.change, PointerChange::Down);
        assert_eq!(data.buttons, 1);
        assert_eq!([data.physical_x, data.physical_y], [10.0, 20.0]);
        assert_eq!((data.view_id, data.device), (ViewId(3), 2));
        assert_eq!(data.embedder_id, 0, "no host numbers its system events");
        assert_eq!(data.pressure, 1.0, "a mouse detects no pressure");
        assert_eq!(data.pointer_identifier, 0);
        assert_eq!([data.physical_delta_x, data.physical_delta_y], [0.0, 0.0]);
        assert!(!data.synthesized);
        assert!(data.signal_kind.is_none());
    }

    #[test]
    fn a_scroll_is_a_signal_on_a_hover() {
        let data = PointerData::from(report(ReportedChange::Scroll { delta: [0.0, 40.0] }));
        assert_eq!(data.change, PointerChange::Hover);
        assert_eq!(data.signal_kind, Some(PointerSignalKind::Scroll));
        assert_eq!([data.scroll_delta_x, data.scroll_delta_y], [0.0, 40.0]);
    }

    #[test]
    fn a_pan_zoom_update_carries_its_totals() {
        let data = PointerData::from(report(ReportedChange::PanZoomUpdate {
            pan: [3.0, 4.0],
            scale: 1.5,
            rotation: 0.25,
        }));
        assert_eq!(data.change, PointerChange::PanZoomUpdate);
        assert_eq!([data.pan_x, data.pan_y], [3.0, 4.0]);
        assert_eq!((data.scale, data.rotation), (1.5, 0.25));
        assert_eq!([data.pan_delta_x, data.pan_delta_y], [0.0, 0.0]);
    }
}
