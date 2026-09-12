//! Flutter counterpart: `gestures/converter.dart`.

use inset_embedder::{
    Offset, PointerChange, PointerData, PointerDeviceKind, PointerSignalKind, ViewId,
};

use crate::events::{
    K_PRIMARY_BUTTON, PointerAddedEvent, PointerCancelEvent, PointerDownEvent, PointerEvent,
    PointerHoverEvent, PointerMoveEvent, PointerPanZoomEndEvent, PointerPanZoomStartEvent,
    PointerPanZoomUpdateEvent, PointerRemovedEvent, PointerScaleEvent, PointerScrollEvent,
    PointerScrollInertiaCancelEvent, PointerUpEvent,
};

/// Add [`K_PRIMARY_BUTTON`] to `buttons` when a pointer of certain devices is
/// down.
///
/// This patch is supposed to be done by embedders. Patching it in framework is
/// a workaround before [`PointerEventConverter`] is moved to embedders.
/// https://github.com/flutter/flutter/issues/30454
fn synthesise_down_buttons(buttons: i64, kind: PointerDeviceKind) -> i64 {
    match kind {
        PointerDeviceKind::Mouse | PointerDeviceKind::Trackpad => buttons,
        PointerDeviceKind::Touch
        | PointerDeviceKind::Stylus
        | PointerDeviceKind::InvertedStylus
        | PointerDeviceKind::Unknown => {
            if buttons == 0 {
                K_PRIMARY_BUTTON
            } else {
                buttons
            }
        }
    }
}

/// Converts from engine pointer data to framework pointer events.
pub struct PointerEventConverter;

impl PointerEventConverter {
    /// Expand the given packet of pointer data into a sequence of framework
    /// pointer events.
    ///
    /// The `device_pixel_ratio_for_view` is used to obtain the device pixel
    /// ratio for the view a particular event occurred in to convert its data
    /// from physical coordinates to logical pixels.
    pub fn expand(
        data: impl IntoIterator<Item = PointerData>,
        device_pixel_ratio_for_view: impl Fn(ViewId) -> Option<f64>,
    ) -> impl Iterator<Item = PointerEvent> {
        data.into_iter().filter_map(move |datum| {
            if datum.signal_kind == Some(PointerSignalKind::Unknown) {
                return None;
            }
            let device_pixel_ratio = device_pixel_ratio_for_view(datum.view_id)?;
            let position = Offset::new(datum.physical_x, datum.physical_y) / device_pixel_ratio;
            let delta =
                Offset::new(datum.physical_delta_x, datum.physical_delta_y) / device_pixel_ratio;
            let radius_minor = to_logical_pixels(datum.radius_minor, device_pixel_ratio);
            let radius_major = to_logical_pixels(datum.radius_major, device_pixel_ratio);
            let radius_min = to_logical_pixels(datum.radius_min, device_pixel_ratio);
            let radius_max = to_logical_pixels(datum.radius_max, device_pixel_ratio);
            let time_stamp = datum.time_stamp;
            let kind = datum.kind;
            match datum.signal_kind.unwrap_or(PointerSignalKind::None) {
                PointerSignalKind::None => match datum.change {
                    PointerChange::Add => Some(PointerEvent::Added(PointerAddedEvent {
                        view_id: datum.view_id,
                        time_stamp,
                        kind,
                        device: datum.device,
                        position,
                        obscured: datum.obscured,
                        pressure_min: datum.pressure_min,
                        pressure_max: datum.pressure_max,
                        distance: datum.distance,
                        distance_max: datum.distance_max,
                        radius_min,
                        radius_max,
                        orientation: datum.orientation,
                        tilt: datum.tilt,
                        embedder_id: datum.embedder_id,
                        ..PointerAddedEvent::default()
                    })),
                    PointerChange::Hover => Some(PointerEvent::Hover(PointerHoverEvent {
                        view_id: datum.view_id,
                        time_stamp,
                        kind,
                        device: datum.device,
                        position,
                        delta,
                        buttons: datum.buttons,
                        obscured: datum.obscured,
                        pressure_min: datum.pressure_min,
                        pressure_max: datum.pressure_max,
                        distance: datum.distance,
                        distance_max: datum.distance_max,
                        size: datum.size,
                        radius_major,
                        radius_minor,
                        radius_min,
                        radius_max,
                        orientation: datum.orientation,
                        tilt: datum.tilt,
                        synthesized: datum.synthesized,
                        embedder_id: datum.embedder_id,
                        ..PointerHoverEvent::default()
                    })),
                    PointerChange::Down => {
                        debug_assert!(!matches!(kind, PointerDeviceKind::Trackpad));
                        Some(PointerEvent::Down(PointerDownEvent {
                            view_id: datum.view_id,
                            time_stamp,
                            pointer: datum.pointer_identifier,
                            kind,
                            device: datum.device,
                            position,
                            buttons: synthesise_down_buttons(datum.buttons, kind),
                            obscured: datum.obscured,
                            pressure: datum.pressure,
                            pressure_min: datum.pressure_min,
                            pressure_max: datum.pressure_max,
                            distance_max: datum.distance_max,
                            size: datum.size,
                            radius_major,
                            radius_minor,
                            radius_min,
                            radius_max,
                            orientation: datum.orientation,
                            tilt: datum.tilt,
                            embedder_id: datum.embedder_id,
                            ..PointerDownEvent::default()
                        }))
                    }
                    PointerChange::Move => {
                        debug_assert!(!matches!(kind, PointerDeviceKind::Trackpad));
                        Some(PointerEvent::Move(PointerMoveEvent {
                            view_id: datum.view_id,
                            time_stamp,
                            pointer: datum.pointer_identifier,
                            kind,
                            device: datum.device,
                            position,
                            delta,
                            buttons: synthesise_down_buttons(datum.buttons, kind),
                            obscured: datum.obscured,
                            pressure: datum.pressure,
                            pressure_min: datum.pressure_min,
                            pressure_max: datum.pressure_max,
                            distance_max: datum.distance_max,
                            size: datum.size,
                            radius_major,
                            radius_minor,
                            radius_min,
                            radius_max,
                            orientation: datum.orientation,
                            tilt: datum.tilt,
                            platform_data: datum.platform_data,
                            synthesized: datum.synthesized,
                            embedder_id: datum.embedder_id,
                            ..PointerMoveEvent::default()
                        }))
                    }
                    PointerChange::Up => {
                        debug_assert!(!matches!(kind, PointerDeviceKind::Trackpad));
                        Some(PointerEvent::Up(PointerUpEvent {
                            view_id: datum.view_id,
                            time_stamp,
                            pointer: datum.pointer_identifier,
                            kind,
                            device: datum.device,
                            position,
                            buttons: datum.buttons,
                            obscured: datum.obscured,
                            pressure: datum.pressure,
                            pressure_min: datum.pressure_min,
                            pressure_max: datum.pressure_max,
                            distance: datum.distance,
                            distance_max: datum.distance_max,
                            size: datum.size,
                            radius_major,
                            radius_minor,
                            radius_min,
                            radius_max,
                            orientation: datum.orientation,
                            tilt: datum.tilt,
                            embedder_id: datum.embedder_id,
                            ..PointerUpEvent::default()
                        }))
                    }
                    PointerChange::Cancel => {
                        debug_assert!(!matches!(kind, PointerDeviceKind::Trackpad));
                        Some(PointerEvent::Cancel(PointerCancelEvent {
                            view_id: datum.view_id,
                            time_stamp,
                            pointer: datum.pointer_identifier,
                            kind,
                            device: datum.device,
                            position,
                            buttons: datum.buttons,
                            obscured: datum.obscured,
                            pressure_min: datum.pressure_min,
                            pressure_max: datum.pressure_max,
                            distance: datum.distance,
                            distance_max: datum.distance_max,
                            size: datum.size,
                            radius_major,
                            radius_minor,
                            radius_min,
                            radius_max,
                            orientation: datum.orientation,
                            tilt: datum.tilt,
                            embedder_id: datum.embedder_id,
                            ..PointerCancelEvent::default()
                        }))
                    }
                    PointerChange::Remove => Some(PointerEvent::Removed(PointerRemovedEvent {
                        view_id: datum.view_id,
                        time_stamp,
                        kind,
                        device: datum.device,
                        position,
                        obscured: datum.obscured,
                        pressure_min: datum.pressure_min,
                        pressure_max: datum.pressure_max,
                        distance_max: datum.distance_max,
                        radius_min,
                        radius_max,
                        embedder_id: datum.embedder_id,
                        ..PointerRemovedEvent::default()
                    })),
                    PointerChange::PanZoomStart => {
                        Some(PointerEvent::PanZoomStart(PointerPanZoomStartEvent {
                            view_id: datum.view_id,
                            time_stamp,
                            pointer: datum.pointer_identifier,
                            device: datum.device,
                            position,
                            embedder_id: datum.embedder_id,
                            synthesized: datum.synthesized,
                            ..PointerPanZoomStartEvent::default()
                        }))
                    }
                    PointerChange::PanZoomUpdate => {
                        let pan = Offset::new(datum.pan_x, datum.pan_y) / device_pixel_ratio;
                        let pan_delta =
                            Offset::new(datum.pan_delta_x, datum.pan_delta_y) / device_pixel_ratio;
                        Some(PointerEvent::PanZoomUpdate(PointerPanZoomUpdateEvent {
                            view_id: datum.view_id,
                            time_stamp,
                            pointer: datum.pointer_identifier,
                            device: datum.device,
                            position,
                            pan,
                            pan_delta,
                            scale: datum.scale,
                            rotation: datum.rotation,
                            embedder_id: datum.embedder_id,
                            synthesized: datum.synthesized,
                            ..PointerPanZoomUpdateEvent::default()
                        }))
                    }
                    PointerChange::PanZoomEnd => {
                        Some(PointerEvent::PanZoomEnd(PointerPanZoomEndEvent {
                            view_id: datum.view_id,
                            time_stamp,
                            pointer: datum.pointer_identifier,
                            device: datum.device,
                            position,
                            embedder_id: datum.embedder_id,
                            synthesized: datum.synthesized,
                            ..PointerPanZoomEndEvent::default()
                        }))
                    }
                },
                PointerSignalKind::Scroll => {
                    if !datum.scroll_delta_x.is_finite()
                        || !datum.scroll_delta_y.is_finite()
                        || device_pixel_ratio <= 0.0
                    {
                        return None;
                    }
                    let scroll_delta = Offset::new(datum.scroll_delta_x, datum.scroll_delta_y)
                        / device_pixel_ratio;
                    Some(PointerEvent::Scroll(PointerScrollEvent {
                        view_id: datum.view_id,
                        time_stamp,
                        kind,
                        device: datum.device,
                        position,
                        scroll_delta,
                        embedder_id: datum.embedder_id,
                        ..PointerScrollEvent::default()
                    }))
                }
                PointerSignalKind::ScrollInertiaCancel => Some(PointerEvent::ScrollInertiaCancel(
                    PointerScrollInertiaCancelEvent {
                        view_id: datum.view_id,
                        time_stamp,
                        kind,
                        device: datum.device,
                        position,
                        embedder_id: datum.embedder_id,
                        ..PointerScrollInertiaCancelEvent::default()
                    },
                )),
                PointerSignalKind::Scale => Some(PointerEvent::Scale(PointerScaleEvent {
                    view_id: datum.view_id,
                    time_stamp,
                    kind,
                    device: datum.device,
                    position,
                    embedder_id: datum.embedder_id,
                    scale: datum.scale,
                    ..PointerScaleEvent::default()
                })),
                PointerSignalKind::Unknown => None,
            }
        })
    }
}

fn to_logical_pixels(physical_pixels: f64, device_pixel_ratio: f64) -> f64 {
    physical_pixels / device_pixel_ratio
}

#[cfg(test)]
mod tests {
    use super::*;
    use inset_embedder::PointerDataPacket;

    #[test]
    fn expand_mouse_down_keeps_buttons() {
        let datum = PointerData {
            change: PointerChange::Down,
            kind: PointerDeviceKind::Mouse,
            buttons: 0x01,
            physical_x: 20.0,
            physical_y: 40.0,
            ..PointerData::default()
        };
        let events: Vec<_> = PointerEventConverter::expand([datum], |_| Some(2.0)).collect();
        match &events[..] {
            [PointerEvent::Down(event)] => {
                assert_eq!(event.position, Offset::new(10.0, 20.0));
                assert_eq!(event.buttons, 0x01);
                assert!(event.down);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn expand_touch_down_synthesises_primary_button() {
        let datum = PointerData {
            change: PointerChange::Down,
            kind: PointerDeviceKind::Touch,
            buttons: 0,
            ..PointerData::default()
        };
        let events: Vec<_> = PointerEventConverter::expand([datum], |_| Some(1.0)).collect();
        match &events[..] {
            [PointerEvent::Down(event)] => assert_eq!(event.buttons, K_PRIMARY_BUTTON),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn expand_drops_missing_view() {
        let packet = PointerDataPacket {
            data: vec![PointerData {
                change: PointerChange::Hover,
                ..PointerData::default()
            }],
        };
        let events: Vec<_> = PointerEventConverter::expand(packet.data, |_| None).collect();
        assert!(events.is_empty());
    }
}
