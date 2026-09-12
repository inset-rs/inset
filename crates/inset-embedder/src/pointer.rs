//! Flutter counterpart: `engine/src/flutter/lib/ui/pointer.dart`.

use std::fmt::{self, Debug, Display};
use std::time::Duration;

use crate::ViewId;

/// How the pointer has changed since the last report.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerChange {
    /// The input from the pointer is no longer directed towards this receiver.
    Cancel,

    /// The device has started tracking the pointer.
    ///
    /// For example, the pointer might be hovering above the device, having not yet
    /// made contact with the surface of the device.
    Add,

    /// The device is no longer tracking the pointer.
    ///
    /// For example, the pointer might have drifted out of the device's hover
    /// detection range or might have been disconnected from the system entirely.
    Remove,

    /// The pointer has moved with respect to the device while not in contact with
    /// the device.
    Hover,

    /// The pointer has made contact with the device.
    Down,

    /// The pointer has moved with respect to the device while in contact with the
    /// device.
    Move,

    /// The pointer has stopped making contact with the device.
    Up,

    /// A pan/zoom has started on this pointer.
    ///
    /// This type of event will always have kind [`PointerDeviceKind::Trackpad`].
    PanZoomStart,

    /// The pan/zoom on this pointer has updated.
    ///
    /// This type of event will always have kind [`PointerDeviceKind::Trackpad`].
    PanZoomUpdate,

    /// The pan/zoom on this pointer has ended.
    ///
    /// This type of event will always have kind [`PointerDeviceKind::Trackpad`].
    PanZoomEnd,
}

/// The kind of pointer device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerDeviceKind {
    /// A touch-based pointer device.
    ///
    /// The most common case is a touch screen.
    ///
    /// When the user is operating with a trackpad on iOS, clicking will also
    /// dispatch events with kind [`Touch`](PointerDeviceKind::Touch) if
    /// `UIApplicationSupportsIndirectInputEvents` is not present in `Info.plist`
    /// or returns NO.
    ///
    /// See also:
    ///
    ///  * [UIApplicationSupportsIndirectInputEvents](https://developer.apple.com/documentation/bundleresources/information_property_list/uiapplicationsupportsindirectinputevents?language=objc).
    Touch,

    /// A mouse-based pointer device.
    ///
    /// The most common case is a mouse on the desktop or Web.
    ///
    /// When the user is operating with a trackpad on iOS, moving the pointing
    /// cursor will also dispatch events with kind [`Mouse`](PointerDeviceKind::Mouse), and clicking will
    /// dispatch events with kind [`Mouse`](PointerDeviceKind::Mouse) if
    /// `UIApplicationSupportsIndirectInputEvents` is not present in `Info.plist`
    /// or returns NO.
    ///
    /// See also:
    ///
    ///  * [UIApplicationSupportsIndirectInputEvents](https://developer.apple.com/documentation/bundleresources/information_property_list/uiapplicationsupportsindirectinputevents?language=objc).
    Mouse,

    /// A pointer device with a stylus.
    Stylus,

    /// A pointer device with a stylus that has been inverted.
    InvertedStylus,

    /// Gestures from a trackpad.
    ///
    /// A trackpad here is defined as a touch-based pointer device with an
    /// indirect surface (the user operates the screen by touching something that
    /// is not the screen).
    ///
    /// When the user makes zoom, pan, scroll or rotate gestures with a physical
    /// trackpad, supporting platforms dispatch events with kind [`Trackpad`](PointerDeviceKind::Trackpad).
    ///
    /// Events with kind [`Trackpad`](PointerDeviceKind::Trackpad) can only have a [`PointerChange`] of `Add`,
    /// `Remove`, and pan-zoom related values.
    ///
    /// Some platforms don't support (or don't fully support) trackpad
    /// gestures, and might convert trackpad gestures into fake pointer events
    /// that simulate dragging. These events typically have kind [`Touch`](PointerDeviceKind::Touch) or
    /// [`Mouse`](PointerDeviceKind::Mouse) instead of [`Trackpad`](PointerDeviceKind::Trackpad). This includes (but is not limited to) Web,
    /// and iOS when `UIApplicationSupportsIndirectInputEvents` isn't present in
    /// `Info.plist` or returns NO.
    ///
    /// Moving the pointing cursor or clicking with a trackpad typically triggers
    /// [`Touch`](PointerDeviceKind::Touch) or [`Mouse`](PointerDeviceKind::Mouse) events, but never triggers [`Trackpad`](PointerDeviceKind::Trackpad) events.
    ///
    /// See also:
    ///
    ///  * [UIApplicationSupportsIndirectInputEvents](https://developer.apple.com/documentation/bundleresources/information_property_list/uiapplicationsupportsindirectinputevents?language=objc).
    Trackpad,

    /// An unknown pointer device.
    Unknown,
}

/// The kind of pointer signal event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerSignalKind {
    /// The event is not associated with a pointer signal.
    None,

    /// A pointer-generated scroll (e.g., mouse wheel or trackpad scroll).
    Scroll,

    /// A pointer-generated scroll-inertia cancel.
    ScrollInertiaCancel,

    /// A pointer-generated scale event (e.g. trackpad pinch).
    Scale,

    /// An unknown pointer signal kind.
    Unknown,
}

/// Information about the state of a pointer.
#[derive(Clone, Copy, Debug)]
pub struct PointerData {
    /// The ID of the [`crate::View`] this pointer event originated from.
    pub view_id: ViewId,

    /// Unique identifier that ties the pointer event to the embedder
    /// event that created it.
    ///
    /// No two pointer events can have the same [`embedder_id`](Self::embedder_id). This is different
    /// from [`pointer_identifier`](Self::pointer_identifier) - used for hit-testing, whereas [`embedder_id`](Self::embedder_id) is
    /// used to identify the platform event.
    pub embedder_id: i64,

    /// Time of event dispatch, relative to an arbitrary timeline.
    pub time_stamp: Duration,

    /// How the pointer has changed since the last report.
    pub change: PointerChange,

    /// The kind of input device for which the event was generated.
    pub kind: PointerDeviceKind,

    /// The kind of signal for a pointer signal event.
    pub signal_kind: Option<PointerSignalKind>,

    /// Unique identifier for the pointing device, reused across interactions.
    pub device: i64,

    /// Unique identifier for the pointer.
    ///
    /// This field changes for each new pointer down event. Framework uses this
    /// identifier to determine hit test result.
    pub pointer_identifier: i64,

    /// X coordinate of the position of the pointer, in physical pixels in the
    /// global coordinate space.
    pub physical_x: f64,

    /// Y coordinate of the position of the pointer, in physical pixels in the
    /// global coordinate space.
    pub physical_y: f64,

    /// The distance of pointer movement on X coordinate in physical pixels.
    pub physical_delta_x: f64,

    /// The distance of pointer movement on Y coordinate in physical pixels.
    pub physical_delta_y: f64,

    /// Bit field using the *Button constants (primaryMouseButton,
    /// secondaryStylusButton, etc). For example, if this has the value 6 and the
    /// [`kind`](Self::kind) is [`PointerDeviceKind::InvertedStylus`], then this indicates an
    /// upside-down stylus with both its primary and secondary buttons pressed.
    pub buttons: i64,

    /// Set if an application from a different security domain is in any way
    /// obscuring this application's window. (Aspirational; not currently
    /// implemented.)
    pub obscured: bool,

    /// Set if this pointer data was synthesized by pointer data packet converter.
    /// pointer data packet converter will synthesize additional pointer datas if
    /// the input sequence of pointer data is illegal.
    ///
    /// For example, a down pointer data will be synthesized if the converter receives
    /// a move pointer data while the pointer is not previously down.
    pub synthesized: bool,

    /// The pressure of the touch as a number ranging from 0.0, indicating a touch
    /// with no discernible pressure, to 1.0, indicating a touch with "normal"
    /// pressure, and possibly beyond, indicating a stronger touch. For devices
    /// that do not detect pressure (e.g. mice), returns 1.0.
    pub pressure: f64,

    /// The minimum value that [`pressure`](Self::pressure) can return for this pointer. For devices
    /// that do not detect pressure (e.g. mice), returns 1.0. This will always be
    /// a number less than or equal to 1.0.
    pub pressure_min: f64,

    /// The maximum value that [`pressure`](Self::pressure) can return for this pointer. For devices
    /// that do not detect pressure (e.g. mice), returns 1.0. This will always be
    /// a greater than or equal to 1.0.
    pub pressure_max: f64,

    /// The distance of the detected object from the input surface (e.g. the
    /// distance of a stylus or finger from a touch screen), in arbitrary units on
    /// an arbitrary (not necessarily linear) scale. If the pointer is down, this
    /// is 0.0 by definition.
    pub distance: f64,

    /// The maximum value that a distance can return for this pointer. If this
    /// input device cannot detect "hover touch" input events, then this will be
    /// 0.0.
    pub distance_max: f64,

    /// The area of the screen being pressed, scaled to a value between 0 and 1.
    /// The value of size can be used to determine fat touch events. This value
    /// is only set on Android, and is a device specific approximation within
    /// the range of detectable values. So, for example, the value of 0.1 could
    /// mean a touch with the tip of the finger, 0.2 a touch with full finger,
    /// and 0.3 the full palm.
    pub size: f64,

    /// The radius of the contact ellipse along the major axis, in logical pixels.
    pub radius_major: f64,

    /// The radius of the contact ellipse along the minor axis, in logical pixels.
    pub radius_minor: f64,

    /// The minimum value that could be reported for radiusMajor and radiusMinor
    /// for this pointer, in logical pixels.
    pub radius_min: f64,

    /// The maximum value that could be reported for radiusMajor and radiusMinor
    /// for this pointer, in logical pixels.
    pub radius_max: f64,

    /// For [`PointerDeviceKind::Touch`] events:
    ///
    /// The angle of the contact ellipse, in radius in the range:
    ///
    ///    -pi/2 < orientation <= pi/2
    ///
    /// ...giving the angle of the major axis of the ellipse with the y-axis
    /// (negative angles indicating an orientation along the top-left /
    /// bottom-right diagonal, positive angles indicating an orientation along the
    /// top-right / bottom-left diagonal, and zero indicating an orientation
    /// parallel with the y-axis).
    ///
    /// For [`PointerDeviceKind::Stylus`] and [`PointerDeviceKind::InvertedStylus`] events:
    ///
    /// The angle of the stylus, in radians in the range:
    ///
    ///    -pi < orientation <= pi
    ///
    /// ...giving the angle of the axis of the stylus projected onto the input
    /// surface, relative to the positive y-axis of that surface (thus 0.0
    /// indicates the stylus, if projected onto that surface, would go from the
    /// contact point vertically up in the positive y-axis direction, pi would
    /// indicate that the stylus would go down in the negative y-axis direction;
    /// pi/4 would indicate that the stylus goes up and to the right, -pi/2 would
    /// indicate that the stylus goes to the left, etc).
    pub orientation: f64,

    /// For [`PointerDeviceKind::Stylus`] and [`PointerDeviceKind::InvertedStylus`] events:
    ///
    /// The angle of the stylus, in radians in the range:
    ///
    ///    0 <= tilt <= pi/2
    ///
    /// ...giving the angle of the axis of the stylus, relative to the axis
    /// perpendicular to the input surface (thus 0.0 indicates the stylus is
    /// orthogonal to the plane of the input surface, while pi/2 indicates that
    /// the stylus is flat on that surface).
    pub tilt: f64,

    /// Opaque platform-specific data associated with the event.
    pub platform_data: i64,

    /// For events with signalKind of [`PointerSignalKind::Scroll`]:
    ///
    /// The amount to scroll in the x direction, in physical pixels.
    pub scroll_delta_x: f64,

    /// For events with signalKind of [`PointerSignalKind::Scroll`]:
    ///
    /// The amount to scroll in the y direction, in physical pixels.
    pub scroll_delta_y: f64,

    /// For events with change of [`PointerChange::PanZoomUpdate`]:
    ///
    /// The current panning magnitude of the pan/zoom in the x direction, in
    /// physical pixels.
    pub pan_x: f64,

    /// For events with change of [`PointerChange::PanZoomUpdate`]:
    ///
    /// The current panning magnitude of the pan/zoom in the y direction, in
    /// physical pixels.
    pub pan_y: f64,

    /// For events with change of [`PointerChange::PanZoomUpdate`]:
    ///
    /// The difference in panning of the pan/zoom in the x direction since the
    /// latest panZoomUpdate event, in physical pixels.
    pub pan_delta_x: f64,

    /// For events with change of [`PointerChange::PanZoomUpdate`]:
    ///
    /// The difference in panning of the pan/zoom in the y direction since the
    /// last panZoomUpdate event, in physical pixels.
    pub pan_delta_y: f64,

    /// For events with change of [`PointerChange::PanZoomUpdate`]:
    ///
    /// The current scale of the pan/zoom (unitless), with 1.0 as the initial scale.
    pub scale: f64,

    /// For events with change of [`PointerChange::PanZoomUpdate`]:
    ///
    /// The current angle of the pan/zoom in radians, with 0.0 as the initial angle.
    pub rotation: f64,
}

impl Default for PointerData {
    fn default() -> PointerData {
        PointerData {
            view_id: ViewId(0),
            embedder_id: 0,
            time_stamp: Duration::ZERO,
            change: PointerChange::Cancel,
            kind: PointerDeviceKind::Touch,
            signal_kind: None,
            device: 0,
            pointer_identifier: 0,
            physical_x: 0.0,
            physical_y: 0.0,
            physical_delta_x: 0.0,
            physical_delta_y: 0.0,
            buttons: 0,
            obscured: false,
            synthesized: false,
            pressure: 0.0,
            pressure_min: 0.0,
            pressure_max: 0.0,
            distance: 0.0,
            distance_max: 0.0,
            size: 0.0,
            radius_major: 0.0,
            radius_minor: 0.0,
            radius_min: 0.0,
            radius_max: 0.0,
            orientation: 0.0,
            tilt: 0.0,
            platform_data: 0,
            scroll_delta_x: 0.0,
            scroll_delta_y: 0.0,
            pan_x: 0.0,
            pan_y: 0.0,
            pan_delta_x: 0.0,
            pan_delta_y: 0.0,
            scale: 0.0,
            rotation: 0.0,
        }
    }
}

impl PointerData {
    /// Returns a complete textual description of the information in this object.
    pub fn to_string_full(&self) -> String {
        format!(
            "PointerData(\
             embedderId: {}, \
             timeStamp: {:?}, \
             change: {:?}, \
             kind: {:?}, \
             signalKind: {:?}, \
             device: {}, \
             pointerIdentifier: {}, \
             physicalX: {}, \
             physicalY: {}, \
             physicalDeltaX: {}, \
             physicalDeltaY: {}, \
             buttons: {}, \
             synthesized: {}, \
             pressure: {}, \
             pressureMin: {}, \
             pressureMax: {}, \
             distance: {}, \
             distanceMax: {}, \
             size: {}, \
             radiusMajor: {}, \
             radiusMinor: {}, \
             radiusMin: {}, \
             radiusMax: {}, \
             orientation: {}, \
             tilt: {}, \
             platformData: {}, \
             scrollDeltaX: {}, \
             scrollDeltaY: {}, \
             panX: {}, \
             panY: {}, \
             panDeltaX: {}, \
             panDeltaY: {}, \
             scale: {}, \
             rotation: {}, \
             viewId: {:?}\
             )",
            self.embedder_id,
            self.time_stamp,
            self.change,
            self.kind,
            self.signal_kind,
            self.device,
            self.pointer_identifier,
            self.physical_x,
            self.physical_y,
            self.physical_delta_x,
            self.physical_delta_y,
            self.buttons,
            self.synthesized,
            self.pressure,
            self.pressure_min,
            self.pressure_max,
            self.distance,
            self.distance_max,
            self.size,
            self.radius_major,
            self.radius_minor,
            self.radius_min,
            self.radius_max,
            self.orientation,
            self.tilt,
            self.platform_data,
            self.scroll_delta_x,
            self.scroll_delta_y,
            self.pan_x,
            self.pan_y,
            self.pan_delta_x,
            self.pan_delta_y,
            self.scale,
            self.rotation,
            self.view_id,
        )
    }
}

impl Display for PointerData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PointerData(viewId: {}, x: {}, y: {})",
            self.view_id.0, self.physical_x, self.physical_y
        )
    }
}

/// A sequence of reports about the state of pointers.
#[derive(Clone, Debug, Default)]
pub struct PointerDataPacket {
    /// Data about the individual pointers in this packet.
    ///
    /// This list might contain multiple pieces of data about the same pointer.
    pub data: Vec<PointerData>,
}

impl PointerDataPacket {
    /// Creates a packet of pointer data reports.
    pub fn new(data: Vec<PointerData>) -> PointerDataPacket {
        PointerDataPacket { data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_data_defaults_match_dart() {
        let data = PointerData::default();
        assert_eq!(data.view_id, ViewId(0));
        assert_eq!(data.change, PointerChange::Cancel);
        assert_eq!(data.kind, PointerDeviceKind::Touch);
        assert!(data.signal_kind.is_none());
        assert_eq!(data.time_stamp, Duration::ZERO);
    }

    #[test]
    fn display_matches_dart_to_string() {
        let data = PointerData {
            view_id: ViewId(2),
            physical_x: 10.0,
            physical_y: 20.0,
            ..PointerData::default()
        };
        assert_eq!(format!("{data}"), "PointerData(viewId: 2, x: 10, y: 20)");
    }
}
