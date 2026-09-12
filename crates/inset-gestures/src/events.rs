//! Flutter counterpart: `gestures/events.dart`.

use std::time::Duration;

use inset_embedder::{Matrix4, Offset, PointerDeviceKind, ViewId};
use inset_foundation::ValueChanged;

use crate::constants::{
    K_PAN_SLOP, K_PRECISE_POINTER_HIT_SLOP, K_PRECISE_POINTER_PAN_SLOP, K_TOUCH_SLOP,
};
use crate::gesture_settings::DeviceGestureSettings;

/// The bit of [`PointerEvent`] `buttons` that corresponds to a cross-device
/// behavior of "primary operation".
pub const K_PRIMARY_BUTTON: i64 = 0x01;

/// The bit of [`PointerEvent`] `buttons` that corresponds to a cross-device
/// behavior of "secondary operation".
pub const K_SECONDARY_BUTTON: i64 = 0x02;

/// The bit of [`PointerEvent`] `buttons` that corresponds to the primary mouse
/// button.
pub const K_PRIMARY_MOUSE_BUTTON: i64 = K_PRIMARY_BUTTON;

/// The bit of [`PointerEvent`] `buttons` that corresponds to the secondary mouse
/// button.
pub const K_SECONDARY_MOUSE_BUTTON: i64 = K_SECONDARY_BUTTON;

/// The bit of [`PointerEvent`] `buttons` that corresponds to when a stylus
/// contacting the screen.
pub const K_STYLUS_CONTACT: i64 = K_PRIMARY_BUTTON;

/// The bit of [`PointerEvent`] `buttons` that corresponds to the primary stylus
/// button.
pub const K_PRIMARY_STYLUS_BUTTON: i64 = K_SECONDARY_BUTTON;

/// The bit of [`PointerEvent`] `buttons` that corresponds to a cross-device
/// behavior of "tertiary operation".
pub const K_TERTIARY_BUTTON: i64 = 0x04;

/// The bit of [`PointerEvent`] `buttons` that corresponds to the middle mouse
/// button.
pub const K_MIDDLE_MOUSE_BUTTON: i64 = K_TERTIARY_BUTTON;

/// The bit of [`PointerEvent`] `buttons` that corresponds to the secondary
/// stylus button.
pub const K_SECONDARY_STYLUS_BUTTON: i64 = K_TERTIARY_BUTTON;

/// The bit of [`PointerEvent`] `buttons` that corresponds to the back mouse
/// button.
pub const K_BACK_MOUSE_BUTTON: i64 = 0x08;

/// The bit of [`PointerEvent`] `buttons` that corresponds to the forward mouse
/// button.
pub const K_FORWARD_MOUSE_BUTTON: i64 = 0x10;

/// The bit of [`PointerEvent`] `buttons` that corresponds to the pointer
/// contacting a touch screen.
pub const K_TOUCH_CONTACT: i64 = K_PRIMARY_BUTTON;

/// Dart VM `kMaxUnsignedSMI` (`foundation/_bitfield_io.dart`).
const K_MAX_UNSIGNED_SMI: i64 = 0x3FFF_FFFF_FFFF_FFFF;

/// The bit of [`PointerEvent`] `buttons` that corresponds to the nth mouse
/// button.
pub fn nth_mouse_button(number: i64) -> i64 {
    K_PRIMARY_MOUSE_BUTTON.wrapping_shl((number - 1) as u32) & K_MAX_UNSIGNED_SMI
}

/// The bit of [`PointerEvent`] `buttons` that corresponds to the nth stylus
/// button.
pub fn nth_stylus_button(number: i64) -> i64 {
    K_PRIMARY_STYLUS_BUTTON.wrapping_shl((number - 1) as u32) & K_MAX_UNSIGNED_SMI
}

/// Returns the button of `buttons` with the smallest integer.
///
/// It returns zero when `buttons` is zero.
pub fn smallest_button(buttons: i64) -> i64 {
    buttons.isolate_lowest_one()
}

/// Returns whether `buttons` contains one and only one button.
pub fn is_single_button(buttons: i64) -> bool {
    buttons != 0 && smallest_button(buttons) == buttons
}

macro_rules! pointer_event_struct {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $($(#[$extra_meta:meta])* $extra_vis:vis $extra:ident: $extra_ty:ty = $extra_default:expr),* $(,)?
        }
        down = $down:expr,
        pressure = $pressure:expr,
        distance = $distance:expr,
        buttons = $buttons:expr,
        kind = $kind:expr $(,)?
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug)]
        pub struct $name {
            /// The ID of the `FlutterView` which this event originated from.
            pub view_id: ViewId,
            /// Unique identifier that ties the [`PointerEvent`] to the embedder
            /// event that created it.
            pub embedder_id: i64,
            /// Time of event dispatch, relative to an arbitrary timeline.
            pub time_stamp: Duration,
            /// Unique identifier for the pointer, not reused.
            pub pointer: i64,
            /// The kind of input device for which the event was generated.
            pub kind: PointerDeviceKind,
            /// Unique identifier for the pointing device, reused across
            /// interactions.
            pub device: i64,
            /// Coordinate of the position of the pointer, in logical pixels in
            /// the global coordinate space.
            pub position: Offset,
            /// Distance in logical pixels that the pointer moved since the last
            /// [`PointerMoveEvent`] or [`PointerHoverEvent`].
            pub delta: Offset,
            /// Bit field using the `*_BUTTON` constants.
            pub buttons: i64,
            /// Set if the pointer is currently down.
            pub down: bool,
            /// Set if an application from a different security domain is in any
            /// way obscuring this application's window.
            pub obscured: bool,
            /// The pressure of the touch.
            pub pressure: f64,
            /// The minimum value that [`pressure`](Self::pressure) can return.
            pub pressure_min: f64,
            /// The maximum value that [`pressure`](Self::pressure) can return.
            pub pressure_max: f64,
            /// The distance of the detected object from the input surface.
            pub distance: f64,
            /// The maximum value that [`distance`](Self::distance) can return.
            pub distance_max: f64,
            /// The area of the screen being pressed.
            pub size: f64,
            /// The radius of the contact ellipse along the major axis, in
            /// logical pixels.
            pub radius_major: f64,
            /// The radius of the contact ellipse along the minor axis, in
            /// logical pixels.
            pub radius_minor: f64,
            /// The minimum value that could be reported for radius major/minor.
            pub radius_min: f64,
            /// The maximum value that could be reported for radius major/minor.
            pub radius_max: f64,
            /// The orientation angle of the detected object, in radians.
            pub orientation: f64,
            /// The tilt angle of the detected object, in radians.
            pub tilt: f64,
            /// Opaque platform-specific data associated with the event.
            pub platform_data: i64,
            /// Set if the event was synthesized by Flutter.
            pub synthesized: bool,
            /// The transformation used to transform this event from the global
            /// coordinate space into the coordinate space of the event
            /// receiver.
            pub transform: Option<Matrix4>,
            /// The original un-transformed event before any transforms were
            /// applied.
            pub original: Option<Box<$name>>,
            $($(#[$extra_meta])* $extra_vis $extra: $extra_ty,)*
        }

        impl Default for $name {
            fn default() -> $name {
                $name {
                    view_id: ViewId(0),
                    embedder_id: 0,
                    time_stamp: Duration::ZERO,
                    pointer: 0,
                    kind: $kind,
                    device: 0,
                    position: Offset::ZERO,
                    delta: Offset::ZERO,
                    buttons: $buttons,
                    down: $down,
                    obscured: false,
                    pressure: $pressure,
                    pressure_min: 1.0,
                    pressure_max: 1.0,
                    distance: $distance,
                    distance_max: 0.0,
                    size: 0.0,
                    radius_major: 0.0,
                    radius_minor: 0.0,
                    radius_min: 0.0,
                    radius_max: 0.0,
                    orientation: 0.0,
                    tilt: 0.0,
                    platform_data: 0,
                    synthesized: false,
                    transform: None,
                    original: None,
                    $($extra: $extra_default,)*
                }
            }
        }

        impl $name {
            /// The [`position`](Self::position) transformed into the event
            /// receiver's local coordinate system according to
            /// [`transform`](Self::transform).
            pub fn local_position(&self) -> Offset {
                transform_position(self.transform, self.position)
            }

            /// The [`delta`](Self::delta) transformed into the event receiver's
            /// local coordinate system according to [`transform`](Self::transform).
            pub fn local_delta(&self) -> Offset {
                transform_delta_via_positions(
                    self.position,
                    None,
                    self.delta,
                    self.transform,
                )
            }

            /// The minimum value that [`distance`](Self::distance) can return.
            ///
            /// This value is always 0.0.
            pub fn distance_min(&self) -> f64 {
                0.0
            }

            /// Transforms the event from the global coordinate space into the
            /// coordinate space of an event receiver.
            pub fn transformed(&self, transform: Option<Matrix4>) -> $name {
                if transform.is_none() || self.transform == transform {
                    return self.clone();
                }
                let original = self.original.clone().unwrap_or_else(|| {
                    let mut orig = self.clone();
                    orig.transform = None;
                    orig.original = None;
                    Box::new(orig)
                });
                let mut event = (*original).clone();
                event.original = Some(original);
                event.transform = transform;
                event
            }

            /// Creates a copy of this event. Chain field replacements, then
            /// [`transformed`](Self::transformed) if a transform should stick.
            pub fn copy_with(&self) -> $name {
                self.clone()
            }
        }
    };
}

pointer_event_struct! {
    /// The device has started tracking the pointer.
    pub struct PointerAddedEvent {}
    down = false,
    pressure = 0.0,
    distance = 0.0,
    buttons = 0,
    kind = PointerDeviceKind::Touch,
}

pointer_event_struct! {
    /// The device is no longer tracking the pointer.
    pub struct PointerRemovedEvent {}
    down = false,
    pressure = 0.0,
    distance = 0.0,
    buttons = 0,
    kind = PointerDeviceKind::Touch,
}

pointer_event_struct! {
    /// The pointer has moved with respect to the device while not in contact
    /// with the device.
    pub struct PointerHoverEvent {}
    down = false,
    pressure = 0.0,
    distance = 0.0,
    buttons = 0,
    kind = PointerDeviceKind::Touch,
}

pointer_event_struct! {
    /// The pointer has moved with respect to the device while not in contact
    /// with the device, and entered a target object.
    ///
    /// Only sent through `MouseTracker` annotation callbacks, never by pointer
    /// routing or hit testing.
    pub struct PointerEnterEvent {}
    down = false,
    pressure = 0.0,
    distance = 0.0,
    buttons = 0,
    kind = PointerDeviceKind::Touch,
}

pointer_event_struct! {
    /// The pointer has moved with respect to the device while not in contact
    /// with the device, and exited a target object.
    ///
    /// Only sent through `MouseTracker` annotation callbacks, never by pointer
    /// routing or hit testing.
    pub struct PointerExitEvent {}
    down = false,
    pressure = 0.0,
    distance = 0.0,
    buttons = 0,
    kind = PointerDeviceKind::Touch,
}

pointer_event_struct! {
    /// The pointer has made contact with the device.
    pub struct PointerDownEvent {}
    down = true,
    pressure = 1.0,
    distance = 0.0,
    buttons = K_PRIMARY_BUTTON,
    kind = PointerDeviceKind::Touch,
}

pointer_event_struct! {
    /// The pointer has moved with respect to the device while in contact with
    /// the device.
    pub struct PointerMoveEvent {}
    down = true,
    pressure = 1.0,
    distance = 0.0,
    buttons = K_PRIMARY_BUTTON,
    kind = PointerDeviceKind::Touch,
}

pointer_event_struct! {
    /// The pointer has stopped making contact with the device.
    pub struct PointerUpEvent {}
    down = false,
    pressure = 0.0,
    distance = 0.0,
    buttons = 0,
    kind = PointerDeviceKind::Touch,
}

pointer_event_struct! {
    /// The input from the pointer is no longer directed towards this receiver.
    pub struct PointerCancelEvent {}
    down = false,
    pressure = 0.0,
    distance = 0.0,
    buttons = 0,
    kind = PointerDeviceKind::Touch,
}

pointer_event_struct! {
    /// The pointer issued a scroll event.
    pub struct PointerScrollEvent {
        /// The amount to scroll, in logical pixels.
        pub scroll_delta: Offset = Offset::ZERO,
    }
    down = false,
    pressure = 1.0,
    distance = 0.0,
    buttons = 0,
    kind = PointerDeviceKind::Touch,
}

pointer_event_struct! {
    /// The pointer issued a scroll-inertia cancel event.
    pub struct PointerScrollInertiaCancelEvent {}
    down = false,
    pressure = 1.0,
    distance = 0.0,
    buttons = 0,
    kind = PointerDeviceKind::Touch,
}

pointer_event_struct! {
    /// The pointer issued a scale event.
    pub struct PointerScaleEvent {
        /// The scale implied by the trackpad event.
        pub scale: f64 = 1.0,
    }
    down = false,
    pressure = 1.0,
    distance = 0.0,
    buttons = 0,
    kind = PointerDeviceKind::Touch,
}

pointer_event_struct! {
    /// A pan/zoom has begun on this pointer.
    pub struct PointerPanZoomStartEvent {}
    down = false,
    pressure = 1.0,
    distance = 0.0,
    buttons = 0,
    kind = PointerDeviceKind::Trackpad,
}

pointer_event_struct! {
    /// The active pan/zoom on this pointer has updated.
    pub struct PointerPanZoomUpdateEvent {
        /// The current panning magnitude of the pan/zoom in logical pixels.
        pub pan: Offset = Offset::ZERO,
        /// The difference in panning since the last pan/zoom update, in logical
        /// pixels.
        pub pan_delta: Offset = Offset::ZERO,
        /// The current scale of the pan/zoom (unitless), with 1.0 as the
        /// initial scale.
        pub scale: f64 = 1.0,
        /// The current angle of the pan/zoom in radians, with 0.0 as the
        /// initial angle.
        pub rotation: f64 = 0.0,
    }
    down = false,
    pressure = 1.0,
    distance = 0.0,
    buttons = 0,
    kind = PointerDeviceKind::Trackpad,
}

pointer_event_struct! {
    /// The pan/zoom on this pointer has ended.
    pub struct PointerPanZoomEndEvent {}
    down = false,
    pressure = 1.0,
    distance = 0.0,
    buttons = 0,
    kind = PointerDeviceKind::Trackpad,
}

impl PointerPanZoomUpdateEvent {
    /// The [`pan`](Self::pan) transformed into the event receiver's local
    /// coordinate system according to [`transform`](Self::transform).
    pub fn local_pan(&self) -> Offset {
        transform_position(self.transform, self.pan)
    }

    /// The [`pan_delta`](Self::pan_delta) transformed into the event receiver's
    /// local coordinate system according to [`transform`](Self::transform).
    pub fn local_pan_delta(&self) -> Offset {
        transform_delta_via_positions(
            self.pan,
            Some(self.local_pan()),
            self.pan_delta,
            self.transform,
        )
    }
}

/// Runs `$body` with `$e` bound to whichever event struct the enum holds.
macro_rules! with_variant {
    ($event:expr, $e:ident => $body:expr) => {
        match $event {
            PointerEvent::Added($e) => $body,
            PointerEvent::Removed($e) => $body,
            PointerEvent::Hover($e) => $body,
            PointerEvent::Enter($e) => $body,
            PointerEvent::Exit($e) => $body,
            PointerEvent::Down($e) => $body,
            PointerEvent::Move($e) => $body,
            PointerEvent::Up($e) => $body,
            PointerEvent::Cancel($e) => $body,
            PointerEvent::Scroll($e) => $body,
            PointerEvent::ScrollInertiaCancel($e) => $body,
            PointerEvent::Scale($e) => $body,
            PointerEvent::PanZoomStart($e) => $body,
            PointerEvent::PanZoomUpdate($e) => $body,
            PointerEvent::PanZoomEnd($e) => $body,
        }
    };
}

/// Like [`with_variant!`], re-wrapping the result in the same variant.
macro_rules! map_variant {
    ($event:expr, $e:ident => $body:expr) => {
        match $event {
            PointerEvent::Added($e) => PointerEvent::Added($body),
            PointerEvent::Removed($e) => PointerEvent::Removed($body),
            PointerEvent::Hover($e) => PointerEvent::Hover($body),
            PointerEvent::Enter($e) => PointerEvent::Enter($body),
            PointerEvent::Exit($e) => PointerEvent::Exit($body),
            PointerEvent::Down($e) => PointerEvent::Down($body),
            PointerEvent::Move($e) => PointerEvent::Move($body),
            PointerEvent::Up($e) => PointerEvent::Up($body),
            PointerEvent::Cancel($e) => PointerEvent::Cancel($body),
            PointerEvent::Scroll($e) => PointerEvent::Scroll($body),
            PointerEvent::ScrollInertiaCancel($e) => PointerEvent::ScrollInertiaCancel($body),
            PointerEvent::Scale($e) => PointerEvent::Scale($body),
            PointerEvent::PanZoomStart($e) => PointerEvent::PanZoomStart($body),
            PointerEvent::PanZoomUpdate($e) => PointerEvent::PanZoomUpdate($body),
            PointerEvent::PanZoomEnd($e) => PointerEvent::PanZoomEnd($body),
        }
    };
}

/// Flutter's `PointerEnterEvent.fromMouseEvent` / `PointerExitEvent.fromMouseEvent`: the
/// fields every event carries, copied into a new event, then transformed like the source.
macro_rules! from_mouse_event {
    ($target:ident, $event:expr) => {
        with_variant!($event, e => $target {
            view_id: e.view_id,
            time_stamp: e.time_stamp,
            pointer: e.pointer,
            kind: e.kind,
            device: e.device,
            position: e.position,
            delta: e.delta,
            buttons: e.buttons,
            obscured: e.obscured,
            pressure_min: e.pressure_min,
            pressure_max: e.pressure_max,
            distance: e.distance,
            distance_max: e.distance_max,
            size: e.size,
            radius_major: e.radius_major,
            radius_minor: e.radius_minor,
            radius_min: e.radius_min,
            radius_max: e.radius_max,
            orientation: e.orientation,
            tilt: e.tilt,
            down: e.down,
            synthesized: e.synthesized,
            ..$target::default()
        })
        .transformed($event.transform())
    };
}

impl PointerEnterEvent {
    /// Creates an enter event from a [`PointerEvent`].
    ///
    /// This is used by the `MouseTracker` to synthesize enter events.
    pub fn from_mouse_event(event: &PointerEvent) -> PointerEnterEvent {
        from_mouse_event!(PointerEnterEvent, event)
    }
}

impl PointerExitEvent {
    /// Creates an exit event from a [`PointerEvent`].
    ///
    /// This is used by the `MouseTracker` to synthesize exit events.
    pub fn from_mouse_event(event: &PointerEvent) -> PointerExitEvent {
        from_mouse_event!(PointerExitEvent, event)
    }
}

/// Base class for touch, stylus, or mouse events.
///
/// Pairing enum over the public event classes. `_Transformed*` subclasses are
/// the same variant with [`transform`](PointerAddedEvent::transform) set.
#[derive(Clone, Debug)]
pub enum PointerEvent {
    /// [`PointerAddedEvent`].
    Added(PointerAddedEvent),
    /// [`PointerRemovedEvent`].
    Removed(PointerRemovedEvent),
    /// [`PointerHoverEvent`].
    Hover(PointerHoverEvent),
    /// [`PointerEnterEvent`].
    Enter(PointerEnterEvent),
    /// [`PointerExitEvent`].
    Exit(PointerExitEvent),
    /// [`PointerDownEvent`].
    Down(PointerDownEvent),
    /// [`PointerMoveEvent`].
    Move(PointerMoveEvent),
    /// [`PointerUpEvent`].
    Up(PointerUpEvent),
    /// [`PointerCancelEvent`].
    Cancel(PointerCancelEvent),
    /// [`PointerScrollEvent`].
    Scroll(PointerScrollEvent),
    /// [`PointerScrollInertiaCancelEvent`].
    ScrollInertiaCancel(PointerScrollInertiaCancelEvent),
    /// [`PointerScaleEvent`].
    Scale(PointerScaleEvent),
    /// [`PointerPanZoomStartEvent`].
    PanZoomStart(PointerPanZoomStartEvent),
    /// [`PointerPanZoomUpdateEvent`].
    PanZoomUpdate(PointerPanZoomUpdateEvent),
    /// [`PointerPanZoomEndEvent`].
    PanZoomEnd(PointerPanZoomEndEvent),
}

impl PointerEvent {
    /// The ID of the `FlutterView` which this event originated from.
    pub fn view_id(&self) -> ViewId {
        with_variant!(self, e => e.view_id)
    }

    /// Unique identifier for the pointing device, reused across interactions.
    pub fn device(&self) -> i64 {
        with_variant!(self, e => e.device)
    }

    /// The transformation used to transform this event from the global
    /// coordinate space into the coordinate space of the event receiver.
    pub fn transform(&self) -> Option<Matrix4> {
        with_variant!(self, e => e.transform)
    }

    /// Coordinate of the position of the pointer, in logical pixels in the
    /// global coordinate space.
    pub fn position(&self) -> Offset {
        with_variant!(self, e => e.position)
    }

    /// The [`position`](Self::position) transformed into the event receiver's
    /// local coordinate system.
    pub fn local_position(&self) -> Offset {
        with_variant!(self, e => e.local_position())
    }

    /// The [`delta`](Self::delta) transformed into the event receiver's local
    /// coordinate system.
    pub fn local_delta(&self) -> Offset {
        with_variant!(self, e => e.local_delta())
    }

    /// Distance in logical pixels that the pointer moved since the last move
    /// or hover.
    pub fn delta(&self) -> Offset {
        with_variant!(self, e => e.delta)
    }

    /// Unique identifier for the pointer, not reused.
    pub fn pointer(&self) -> i64 {
        with_variant!(self, e => e.pointer)
    }

    /// The kind of input device for which the event was generated.
    pub fn kind(&self) -> PointerDeviceKind {
        with_variant!(self, e => e.kind)
    }

    /// Bit field using the `*_BUTTON` constants.
    pub fn buttons(&self) -> i64 {
        with_variant!(self, e => e.buttons)
    }

    /// The pressure of the touch.
    ///
    /// This value is a number ranging from 0.0, indicating a touch with no
    /// discernible pressure, to 1.0, indicating a touch with "normal" pressure,
    /// and possibly beyond, indicating a stronger touch. For devices that do not
    /// detect pressure (e.g. mice), returns 1.0.
    pub fn pressure(&self) -> f64 {
        with_variant!(self, e => e.pressure)
    }

    /// The minimum value that [`pressure`](Self::pressure) can return for this
    /// pointer.
    ///
    /// For devices that do not detect pressure (e.g. mice), returns 1.0. This
    /// will always be a number less than or equal to 1.0.
    pub fn pressure_min(&self) -> f64 {
        with_variant!(self, e => e.pressure_min)
    }

    /// The maximum value that [`pressure`](Self::pressure) can return for this
    /// pointer.
    ///
    /// For devices that do not detect pressure (e.g. mice), returns 1.0. This
    /// will always be a greater than or equal to 1.0.
    pub fn pressure_max(&self) -> f64 {
        with_variant!(self, e => e.pressure_max)
    }

    /// Time of event dispatch, relative to an arbitrary timeline.
    pub fn time_stamp(&self) -> Duration {
        with_variant!(self, e => e.time_stamp)
    }

    /// Set if the event was synthesized.
    pub fn synthesized(&self) -> bool {
        with_variant!(self, e => e.synthesized)
    }

    /// Set if the pointer is currently down.
    pub fn down(&self) -> bool {
        with_variant!(self, e => e.down)
    }

    /// Transforms the event from the global coordinate space into the
    /// coordinate space of an event receiver.
    pub fn transformed(&self, transform: Option<Matrix4>) -> PointerEvent {
        map_variant!(self, e => e.transformed(transform))
    }
}

/// Returns the transformation of `position` into the coordinate system
/// described by `transform`.
///
/// The z-value of `position` is assumed to be 0.0. If `transform` is None,
/// `position` is returned as-is.
pub fn transform_position(transform: Option<Matrix4>, position: Offset) -> Offset {
    match transform {
        None => position,
        Some(transform) => Offset::from(transform.map_point(position.into())),
    }
}

/// Transforms `untransformed_delta` into the coordinate system described by
/// `transform`.
pub fn transform_delta_via_positions(
    untransformed_end_position: Offset,
    transformed_end_position: Option<Offset>,
    untransformed_delta: Offset,
    transform: Option<Matrix4>,
) -> Offset {
    if transform.is_none() {
        return untransformed_delta;
    }
    let transformed_end_position = transformed_end_position
        .unwrap_or_else(|| transform_position(transform, untransformed_end_position));
    let transformed_start_position =
        transform_position(transform, untransformed_end_position - untransformed_delta);
    transformed_end_position - transformed_start_position
}

/// Determine the appropriate hit slop pixels based on the `kind` of pointer.
pub fn compute_hit_slop(kind: PointerDeviceKind, settings: Option<DeviceGestureSettings>) -> f64 {
    match kind {
        PointerDeviceKind::Mouse => K_PRECISE_POINTER_HIT_SLOP,
        PointerDeviceKind::Stylus
        | PointerDeviceKind::InvertedStylus
        | PointerDeviceKind::Unknown
        | PointerDeviceKind::Touch
        | PointerDeviceKind::Trackpad => settings
            .and_then(|settings| settings.touch_slop)
            .unwrap_or(K_TOUCH_SLOP),
    }
}

/// Determine the appropriate pan slop pixels based on the `kind` of pointer.
pub fn compute_pan_slop(kind: PointerDeviceKind, settings: Option<DeviceGestureSettings>) -> f64 {
    match kind {
        PointerDeviceKind::Mouse => K_PRECISE_POINTER_PAN_SLOP,
        PointerDeviceKind::Stylus
        | PointerDeviceKind::InvertedStylus
        | PointerDeviceKind::Unknown
        | PointerDeviceKind::Touch
        | PointerDeviceKind::Trackpad => settings
            .and_then(|settings| settings.pan_slop())
            .unwrap_or(K_PAN_SLOP),
    }
}

/// Signature for listening to [`PointerDownEvent`] events.
pub type PointerDownEventListener = ValueChanged<PointerDownEvent>;

/// Signature for listening to [`PointerMoveEvent`] events.
pub type PointerMoveEventListener = ValueChanged<PointerMoveEvent>;

/// Signature for listening to [`PointerUpEvent`] events.
pub type PointerUpEventListener = ValueChanged<PointerUpEvent>;

/// Signature for listening to [`PointerHoverEvent`] events.
pub type PointerHoverEventListener = ValueChanged<PointerHoverEvent>;

/// Signature for listening to [`PointerEnterEvent`] events.
pub type PointerEnterEventListener = ValueChanged<PointerEnterEvent>;

/// Signature for listening to [`PointerExitEvent`] events.
pub type PointerExitEventListener = ValueChanged<PointerExitEvent>;

/// Signature for listening to [`PointerCancelEvent`] events.
pub type PointerCancelEventListener = ValueChanged<PointerCancelEvent>;

/// Signature for listening to [`PointerPanZoomStartEvent`] events.
pub type PointerPanZoomStartEventListener = ValueChanged<PointerPanZoomStartEvent>;

/// Signature for listening to [`PointerPanZoomUpdateEvent`] events.
pub type PointerPanZoomUpdateEventListener = ValueChanged<PointerPanZoomUpdateEvent>;

/// Signature for listening to [`PointerPanZoomEndEvent`] events.
pub type PointerPanZoomEndEventListener = ValueChanged<PointerPanZoomEndEvent>;

/// Signature for listening to pointer signal events: [`PointerEvent::Scroll`],
/// [`PointerEvent::ScrollInertiaCancel`], and [`PointerEvent::Scale`].
pub type PointerSignalEventListener = ValueChanged<PointerEvent>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smallest_button_picks_the_least_significant_set_bit() {
        assert_eq!(smallest_button(0x01), 0x01);
        assert_eq!(smallest_button(0x11), 0x01);
        assert_eq!(smallest_button(0x10), 0x10);
        assert_eq!(smallest_button(0), 0);
    }

    #[test]
    fn is_single_button_rejects_zero_and_multi() {
        assert!(is_single_button(0x1));
        assert!(!is_single_button(0x11));
        assert!(!is_single_button(0));
    }

    #[test]
    fn transformed_down_keeps_global_position() {
        let event = PointerDownEvent {
            position: Offset::new(10.0, 20.0),
            ..PointerDownEvent::default()
        };
        let transformed = event.transformed(Some(Matrix4::translation(5.0, 7.0)));
        assert_eq!(transformed.position, Offset::new(10.0, 20.0));
        assert_eq!(transformed.local_position(), Offset::new(15.0, 27.0));
        assert!(transformed.original.is_some());
    }
}
