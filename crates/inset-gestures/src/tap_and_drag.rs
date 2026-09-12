//! Flutter counterpart: `gestures/tap_and_drag.dart`.
//!
//! Leaf objects. `BaseTapAndDragGestureRecognizer`'s fields are the
//! [`TapAndDragData`] bag and its bodies are the [`BaseTapAndDragGestureRecognizer`]
//! trait, blanket-implemented for every tap-and-drag leaf; `super` reads
//! `TapStatusTracker::handle_event(self, ..)` then
//! `BaseTapAndDragGestureRecognizer::handle_event(self, ..)` at Dart's position.
//! The bags above it and their `super` namespaces are in `recognizer.rs`.

use std::collections::HashSet;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use inset_embedder::{Offset, PointerDeviceKind};
use inset_foundation::{App, Handle, Listener, Timer, ValueChanged};

use crate::arena::GestureDisposition;
use crate::constants::{K_DOUBLE_TAP_SLOP, K_DOUBLE_TAP_TIMEOUT, K_PRESS_TIMEOUT};
use crate::events::{
    K_PRIMARY_BUTTON, PointerDownEvent, PointerEvent, PointerPanZoomStartEvent, PointerUpEvent,
    compute_hit_slop, compute_pan_slop, transform_delta_via_positions,
};
use crate::gesture_details::PositionedGestureDetails;
use crate::gesture_settings::DeviceGestureSettings;
use crate::recognizer::{
    DragStartBehavior, GestureRecognizer, GestureRecognizerData, OffsetPair, OneSequenceData,
    OneSequenceGestureRecognizer, OneSequenceLeafData, RecognizerLeaf, RecognizerLeafData,
};
use crate::team::GestureArenaTeam;
use crate::velocity::Velocity;

/// The possible states of a [`BaseTapAndDragGestureRecognizer`].
///
/// The recognizer advances from [`Ready`](Self::Ready) to [`Possible`](Self::Possible) when it starts tracking
/// a pointer in [`BaseTapAndDragGestureRecognizer::add_allowed_pointer`]. Where it advances
/// from there depends on the sequence of pointer events that is tracked by the
/// recognizer, following the initial [`PointerDownEvent`]:
///
/// * If a [`crate::PointerUpEvent`] has not been tracked, the recognizer stays in the [`Possible`](Self::Possible)
///   state as long as it continues to track a pointer.
/// * If a [`crate::PointerMoveEvent`] is tracked that has moved a sufficient global distance
///   from the initial [`PointerDownEvent`] and it came before a [`crate::PointerUpEvent`], then
///   this recognizer moves from the [`Possible`](Self::Possible) state to [`Accepted`](Self::Accepted).
/// * If a [`crate::PointerUpEvent`] is tracked before the pointer has moved a sufficient global
///   distance to be considered a drag, then this recognizer moves from the [`Possible`](Self::Possible)
///   state to [`Ready`](Self::Ready).
/// * If a [`crate::PointerCancelEvent`] is tracked then this recognizer moves from its current
///   state to [`Ready`](Self::Ready).
///
/// Once the recognizer has stopped tracking any remaining pointers, the recognizer
/// returns to the [`Ready`](Self::Ready) state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DragState {
    /// The recognizer is ready to start recognizing a drag.
    Ready,
    /// The sequence of pointer events seen thus far is consistent with a drag but
    /// it has not been accepted definitively.
    Possible,
    /// The sequence of pointer events has been accepted definitively as a drag.
    Accepted,
}

fn get_global_distance(event: &PointerEvent, origin_position: Option<OffsetPair>) -> f64 {
    debug_assert!(origin_position.is_some());
    let offset = event.position() - origin_position.unwrap().global;
    offset.distance()
}

/// Dart's `num.sign`: `-1.0`, `0.0` or `1.0`. `f64::signum` answers `±1.0` for
/// `±0.0`, which would move `global_distance_moved` on a zero primary delta.
fn sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        value
    }
}

/// Signature for [`BaseTapAndDragGestureRecognizer::set_on_tap_down`].
///
/// The consecutive tap count at the time the pointer contacted the
/// screen is given by [`TapDragDownDetails::consecutive_tap_count`].
pub type GestureTapDragDownCallback = ValueChanged<TapDragDownDetails>;

/// Details for [`GestureTapDragDownCallback`], such as the number of
/// consecutive taps.
///
/// See also:
///
///  * [`BaseTapAndDragGestureRecognizer`], which passes this information to its
///    [`BaseTapAndDragGestureRecognizer::set_on_tap_down`] callback.
///  * [`TapDragUpDetails`], the details for [`GestureTapDragUpCallback`].
///  * [`TapDragStartDetails`], the details for [`GestureTapDragStartCallback`].
///  * [`TapDragUpdateDetails`], the details for [`GestureTapDragUpdateCallback`].
///  * [`TapDragEndDetails`], the details for [`GestureTapDragEndCallback`].
#[derive(Clone)]
pub struct TapDragDownDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position at which the pointer contacted the screen.
    pub local_position: Offset,

    /// The kind of the device that initiated the event.
    pub kind: Option<PointerDeviceKind>,

    /// If this tap is in a series of taps, then this value represents
    /// the number in the series this tap is.
    pub consecutive_tap_count: i32,
}

impl TapDragDownDetails {
    /// Creates details for a [`GestureTapDragDownCallback`].
    pub fn new(
        global_position: Offset,
        local_position: Offset,
        kind: Option<PointerDeviceKind>,
        consecutive_tap_count: i32,
    ) -> TapDragDownDetails {
        TapDragDownDetails {
            global_position,
            local_position,
            kind,
            consecutive_tap_count,
        }
    }
}

impl PositionedGestureDetails for TapDragDownDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

impl Debug for TapDragDownDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TapDragDownDetails")
            .field("globalPosition", &self.global_position)
            .field("localPosition", &self.local_position)
            .field("kind", &self.kind)
            .field("consecutiveTapCount", &self.consecutive_tap_count)
            .finish()
    }
}

/// Signature for [`BaseTapAndDragGestureRecognizer::set_on_tap_up`].
///
/// The consecutive tap count at the time the pointer contacted the
/// screen is given by [`TapDragUpDetails::consecutive_tap_count`].
pub type GestureTapDragUpCallback = ValueChanged<TapDragUpDetails>;

/// Details for [`GestureTapDragUpCallback`], such as the number of
/// consecutive taps.
///
/// See also:
///
///  * [`BaseTapAndDragGestureRecognizer`], which passes this information to its
///    [`BaseTapAndDragGestureRecognizer::set_on_tap_up`] callback.
///  * [`TapDragDownDetails`], the details for [`GestureTapDragDownCallback`].
///  * [`TapDragStartDetails`], the details for [`GestureTapDragStartCallback`].
///  * [`TapDragUpdateDetails`], the details for [`GestureTapDragUpdateCallback`].
///  * [`TapDragEndDetails`], the details for [`GestureTapDragEndCallback`].
#[derive(Clone)]
pub struct TapDragUpDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position at which the pointer contacted the screen.
    pub local_position: Offset,

    /// The kind of the device that initiated the event.
    pub kind: PointerDeviceKind,

    /// If this tap is in a series of taps, then this value represents
    /// the number in the series this tap is.
    pub consecutive_tap_count: i32,
}

impl TapDragUpDetails {
    /// Creates details for a [`GestureTapDragUpCallback`].
    pub fn new(
        global_position: Offset,
        local_position: Offset,
        kind: PointerDeviceKind,
        consecutive_tap_count: i32,
    ) -> TapDragUpDetails {
        TapDragUpDetails {
            global_position,
            local_position,
            kind,
            consecutive_tap_count,
        }
    }
}

impl PositionedGestureDetails for TapDragUpDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

impl Debug for TapDragUpDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TapDragUpDetails")
            .field("globalPosition", &self.global_position)
            .field("localPosition", &self.local_position)
            .field("kind", &self.kind)
            .field("consecutiveTapCount", &self.consecutive_tap_count)
            .finish()
    }
}

/// Signature for [`BaseTapAndDragGestureRecognizer::set_on_drag_start`].
///
/// The consecutive tap count at the time the pointer contacted the
/// screen is given by [`TapDragStartDetails::consecutive_tap_count`].
pub type GestureTapDragStartCallback = ValueChanged<TapDragStartDetails>;

/// Details for [`GestureTapDragStartCallback`], such as the number of
/// consecutive taps.
///
/// See also:
///
///  * [`BaseTapAndDragGestureRecognizer`], which passes this information to its
///    [`BaseTapAndDragGestureRecognizer::set_on_drag_start`] callback.
///  * [`TapDragDownDetails`], the details for [`GestureTapDragDownCallback`].
///  * [`TapDragUpDetails`], the details for [`GestureTapDragUpCallback`].
///  * [`TapDragUpdateDetails`], the details for [`GestureTapDragUpdateCallback`].
///  * [`TapDragEndDetails`], the details for [`GestureTapDragEndCallback`].
#[derive(Clone)]
pub struct TapDragStartDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position at which the pointer contacted the screen.
    pub local_position: Offset,

    /// Recorded timestamp of the source pointer event that triggered the drag
    /// event.
    ///
    /// Could be none if triggered from proxied events such as accessibility.
    pub source_time_stamp: Option<Duration>,

    /// The kind of the device that initiated the event.
    pub kind: Option<PointerDeviceKind>,

    /// If this tap is in a series of taps, then this value represents
    /// the number in the series this tap is.
    pub consecutive_tap_count: i32,
}

impl TapDragStartDetails {
    /// Creates details for a [`GestureTapDragStartCallback`].
    pub fn new(
        global_position: Offset,
        local_position: Offset,
        source_time_stamp: Option<Duration>,
        kind: Option<PointerDeviceKind>,
        consecutive_tap_count: i32,
    ) -> TapDragStartDetails {
        TapDragStartDetails {
            global_position,
            local_position,
            source_time_stamp,
            kind,
            consecutive_tap_count,
        }
    }
}

impl PositionedGestureDetails for TapDragStartDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

impl Debug for TapDragStartDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TapDragStartDetails")
            .field("globalPosition", &self.global_position)
            .field("localPosition", &self.local_position)
            .field("sourceTimeStamp", &self.source_time_stamp)
            .field("kind", &self.kind)
            .field("consecutiveTapCount", &self.consecutive_tap_count)
            .finish()
    }
}

/// Signature for [`BaseTapAndDragGestureRecognizer::set_on_drag_update`].
///
/// The consecutive tap count at the time the pointer contacted the
/// screen is given by [`TapDragUpdateDetails::consecutive_tap_count`].
pub type GestureTapDragUpdateCallback = ValueChanged<TapDragUpdateDetails>;

/// Details for [`GestureTapDragUpdateCallback`], such as the number of
/// consecutive taps.
///
/// See also:
///
///  * [`BaseTapAndDragGestureRecognizer`], which passes this information to its
///    [`BaseTapAndDragGestureRecognizer::set_on_drag_update`] callback.
///  * [`TapDragDownDetails`], the details for [`GestureTapDragDownCallback`].
///  * [`TapDragUpDetails`], the details for [`GestureTapDragUpCallback`].
///  * [`TapDragStartDetails`], the details for [`GestureTapDragStartCallback`].
///  * [`TapDragEndDetails`], the details for [`GestureTapDragEndCallback`].
#[derive(Clone)]
pub struct TapDragUpdateDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position at which the pointer contacted the screen.
    pub local_position: Offset,

    /// Recorded timestamp of the source pointer event that triggered the drag
    /// event.
    ///
    /// Could be none if triggered from proxied events such as accessibility.
    pub source_time_stamp: Option<Duration>,

    /// The amount the pointer has moved in the coordinate space of the event
    /// receiver since the previous update.
    ///
    /// If the [`GestureTapDragUpdateCallback`] is for a one-dimensional drag (e.g.,
    /// a horizontal or vertical drag), then this offset contains only the delta
    /// in that direction (i.e., the coordinate in the other direction is zero).
    ///
    /// Defaults to zero if not specified in the constructor.
    pub delta: Offset,

    /// The amount the pointer has moved along the primary axis in the coordinate
    /// space of the event receiver since the previous
    /// update.
    ///
    /// If the [`GestureTapDragUpdateCallback`] is for a one-dimensional drag (e.g.,
    /// a horizontal or vertical drag), then this value contains the component of
    /// [`delta`](Self::delta) along the primary axis (e.g., horizontal or vertical,
    /// respectively). Otherwise, if the [`GestureTapDragUpdateCallback`] is for a
    /// two-dimensional drag (e.g., a pan), then this value is none.
    ///
    /// Defaults to none if not specified in the constructor.
    pub primary_delta: Option<f64>,

    /// The kind of the device that initiated the event.
    pub kind: Option<PointerDeviceKind>,

    /// A delta offset from the point where the drag initially contacted
    /// the screen to the point where the pointer is currently located in global
    /// coordinates (the present [`global_position`](Self::global_position)) when this callback is triggered.
    ///
    /// When considering a [`crate::GestureRecognizer`] that tracks the number of consecutive taps,
    /// this offset is associated with the most recent [`PointerDownEvent`] that occurred.
    pub offset_from_origin: Offset,

    /// A local delta offset from the point where the drag initially contacted
    /// the screen to the point where the pointer is currently located in local
    /// coordinates (the present [`local_position`](Self::local_position)) when this callback is triggered.
    ///
    /// When considering a [`crate::GestureRecognizer`] that tracks the number of consecutive taps,
    /// this offset is associated with the most recent [`PointerDownEvent`] that occurred.
    pub local_offset_from_origin: Offset,

    /// If this tap is in a series of taps, then this value represents
    /// the number in the series this tap is.
    pub consecutive_tap_count: i32,
}

impl TapDragUpdateDetails {
    /// Creates details for a [`GestureTapDragUpdateCallback`].
    ///
    /// If [`primary_delta`](Self::primary_delta) is non-none, then its value must match one of the
    /// coordinates of [`delta`](Self::delta) and the other coordinate must be zero.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        global_position: Offset,
        local_position: Offset,
        source_time_stamp: Option<Duration>,
        delta: Offset,
        primary_delta: Option<f64>,
        kind: Option<PointerDeviceKind>,
        offset_from_origin: Offset,
        local_offset_from_origin: Offset,
        consecutive_tap_count: i32,
    ) -> TapDragUpdateDetails {
        debug_assert!(
            primary_delta.is_none()
                || (primary_delta == Some(delta.dx()) && delta.dy() == 0.0)
                || (primary_delta == Some(delta.dy()) && delta.dx() == 0.0)
        );
        TapDragUpdateDetails {
            global_position,
            local_position,
            source_time_stamp,
            delta,
            primary_delta,
            kind,
            offset_from_origin,
            local_offset_from_origin,
            consecutive_tap_count,
        }
    }
}

impl PositionedGestureDetails for TapDragUpdateDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

impl Debug for TapDragUpdateDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TapDragUpdateDetails")
            .field("globalPosition", &self.global_position)
            .field("localPosition", &self.local_position)
            .field("sourceTimeStamp", &self.source_time_stamp)
            .field("delta", &self.delta)
            .field("primaryDelta", &self.primary_delta)
            .field("kind", &self.kind)
            .field("offsetFromOrigin", &self.offset_from_origin)
            .field("localOffsetFromOrigin", &self.local_offset_from_origin)
            .field("consecutiveTapCount", &self.consecutive_tap_count)
            .finish()
    }
}

/// Signature for [`BaseTapAndDragGestureRecognizer::set_on_drag_end`].
///
/// The consecutive tap count at the time the pointer contacted the
/// screen is given by [`TapDragEndDetails::consecutive_tap_count`].
pub type GestureTapDragEndCallback = ValueChanged<TapDragEndDetails>;

/// Details for [`GestureTapDragEndCallback`], such as the number of
/// consecutive taps.
///
/// See also:
///
///  * [`BaseTapAndDragGestureRecognizer`], which passes this information to its
///    [`BaseTapAndDragGestureRecognizer::set_on_drag_end`] callback.
///  * [`TapDragDownDetails`], the details for [`GestureTapDragDownCallback`].
///  * [`TapDragUpDetails`], the details for [`GestureTapDragUpCallback`].
///  * [`TapDragStartDetails`], the details for [`GestureTapDragStartCallback`].
///  * [`TapDragUpdateDetails`], the details for [`GestureTapDragUpdateCallback`].
#[derive(Clone)]
pub struct TapDragEndDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position at which the pointer contacted the screen.
    pub local_position: Offset,

    /// The velocity the pointer was moving when it stopped contacting the screen.
    ///
    /// Defaults to zero if not specified in the constructor.
    pub velocity: Velocity,

    /// The velocity the pointer was moving along the primary axis when it stopped
    /// contacting the screen, in logical pixels per second.
    ///
    /// If the [`GestureTapDragEndCallback`] is for a one-dimensional drag (e.g., a
    /// horizontal or vertical drag), then this value contains the component of
    /// [`velocity`](Self::velocity) along the primary axis (e.g., horizontal or vertical,
    /// respectively). Otherwise, if the [`GestureTapDragEndCallback`] is for a
    /// two-dimensional drag (e.g., a pan), then this value is none.
    ///
    /// Defaults to none if not specified in the constructor.
    pub primary_velocity: Option<f64>,

    /// If this tap is in a series of taps, then this value represents
    /// the number in the series this tap is.
    pub consecutive_tap_count: i32,
}

impl TapDragEndDetails {
    /// Creates details for a [`GestureTapDragEndCallback`].
    pub fn new(
        global_position: Offset,
        local_position: Option<Offset>,
        velocity: Velocity,
        primary_velocity: Option<f64>,
        consecutive_tap_count: i32,
    ) -> TapDragEndDetails {
        debug_assert!(
            primary_velocity.is_none()
                || primary_velocity == Some(velocity.pixels_per_second.dx())
                || primary_velocity == Some(velocity.pixels_per_second.dy())
        );
        TapDragEndDetails {
            global_position,
            local_position: local_position.unwrap_or(global_position),
            velocity,
            primary_velocity,
            consecutive_tap_count,
        }
    }
}

impl PositionedGestureDetails for TapDragEndDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

impl Debug for TapDragEndDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TapDragEndDetails")
            .field("globalPosition", &self.global_position)
            .field("localPosition", &self.local_position)
            .field("velocity", &self.velocity)
            .field("primaryVelocity", &self.primary_velocity)
            .field("consecutiveTapCount", &self.consecutive_tap_count)
            .finish()
    }
}

/// Signature for when the pointer that previously triggered a
/// [`GestureTapDragDownCallback`] did not complete.
///
/// Used by [`BaseTapAndDragGestureRecognizer::set_on_cancel`].
pub type GestureCancelCallback = Listener;

/// Field bag for Dart's `_TapStatusTrackerMixin`.
pub struct TapStatusTrackerData {
    down: Option<PointerDownEvent>,
    up: Option<PointerUpEvent>,
    consecutive_tap_count: i32,
    origin_position: Option<OffsetPair>,
    previous_buttons: Option<i64>,
    consecutive_tap_timer: Option<Timer>,
    last_tap_offset: Option<Offset>,
    on_tap_track_start: Option<Listener>,
    on_tap_track_reset: Option<Listener>,
}

impl TapStatusTrackerData {
    /// Initializes the tap-status tracker: no down event seen, consecutive count zero.
    pub fn new() -> TapStatusTrackerData {
        TapStatusTrackerData {
            down: None,
            up: None,
            consecutive_tap_count: 0,
            origin_position: None,
            previous_buttons: None,
            consecutive_tap_timer: None,
            last_tap_offset: None,
            on_tap_track_start: None,
            on_tap_track_reset: None,
        }
    }
}

impl Default for TapStatusTrackerData {
    fn default() -> TapStatusTrackerData {
        TapStatusTrackerData::new()
    }
}

/// Field bag for Dart's `BaseTapAndDragGestureRecognizer`.
pub struct TapAndDragData {
    tap_status_tracker: TapStatusTrackerData,
    drag_start_behavior: DragStartBehavior,
    drag_update_throttle_frequency: Option<Duration>,
    max_consecutive_tap: Option<i32>,
    eager_victory_on_drag: bool,
    on_tap_down: Option<GestureTapDragDownCallback>,
    on_tap_up: Option<GestureTapDragUpCallback>,
    on_drag_start: Option<GestureTapDragStartCallback>,
    on_drag_update: Option<GestureTapDragUpdateCallback>,
    on_drag_end: Option<GestureTapDragEndCallback>,
    on_cancel: Option<GestureCancelCallback>,
    past_slop_tolerance: bool,
    sent_tap_down: bool,
    won_arena_for_primary_pointer: bool,
    primary_pointer: Option<i64>,
    deadline_timer: Option<Timer>,
    deadline: Duration,
    drag_state: DragState,
    start: Option<PointerEvent>,
    initial_position: OffsetPair,
    current_position: OffsetPair,
    global_distance_moved: f64,
    global_distance_moved_all_axes: f64,
    last_drag_update_details: Option<TapDragUpdateDetails>,
    drag_update_throttle_timer: Option<Timer>,
    accepted_active_pointers: HashSet<i64>,
}

impl TapAndDragData {
    /// Initializes the recognizer: no callbacks, [`DragStartBehavior::Start`],
    /// and [`eager_victory_on_drag`](Self::eager_victory_on_drag) true.
    pub fn new() -> TapAndDragData {
        TapAndDragData {
            tap_status_tracker: TapStatusTrackerData::new(),
            drag_start_behavior: DragStartBehavior::Start,
            drag_update_throttle_frequency: None,
            max_consecutive_tap: None,
            eager_victory_on_drag: true,
            on_tap_down: None,
            on_tap_up: None,
            on_drag_start: None,
            on_drag_update: None,
            on_drag_end: None,
            on_cancel: None,
            past_slop_tolerance: false,
            sent_tap_down: false,
            won_arena_for_primary_pointer: false,
            primary_pointer: None,
            deadline_timer: None,
            deadline: K_PRESS_TIMEOUT,
            drag_state: DragState::Ready,
            start: None,
            initial_position: OffsetPair::ZERO,
            current_position: OffsetPair::ZERO,
            global_distance_moved: 0.0,
            global_distance_moved_all_axes: 0.0,
            last_drag_update_details: None,
            drag_update_throttle_timer: None,
            accepted_active_pointers: HashSet::new(),
        }
    }
}

impl Default for TapAndDragData {
    fn default() -> TapAndDragData {
        TapAndDragData::new()
    }
}

/// The [`BaseTapAndDragGestureRecognizer`] field bag on every tap-and-drag leaf.
pub trait TapAndDragLeafData: OneSequenceLeafData {
    /// The leaf's [`BaseTapAndDragGestureRecognizer`] bag.
    fn tap_and_drag(&self) -> &TapAndDragData;

    /// The leaf's [`BaseTapAndDragGestureRecognizer`] bag, mutably.
    fn tap_and_drag_mut(&mut self) -> &mut TapAndDragData;
}

/// Virtuals `BaseTapAndDragGestureRecognizer`'s bodies call on its leaves.
pub trait TapAndDragLeaf: RecognizerLeaf + TapAndDragLeafData {
    /// Returns the effective delta that should be considered for the incoming
    /// `delta`.
    fn get_delta_for_details(self: Handle<Self>, app: &App, delta: Offset) -> Offset;

    /// Returns the value for the primary axis from the given `value`.
    ///
    /// Returns `None` if the recognizer does not have a primary axis.
    fn get_primary_value_from_offset(self: Handle<Self>, app: &App, value: Offset) -> Option<f64>;

    /// Whether [`TapAndDragData`]'s global distance moved is big enough to
    /// accept the gesture.
    fn has_sufficient_global_distance_to_accept(
        self: Handle<Self>,
        app: &App,
        pointer_device_kind: PointerDeviceKind,
    ) -> bool;

    /// The consecutive-tap timer's callback. A distinct fn item per leaf.
    fn consecutive_tap_timer_timeout(self: Handle<Self>, app: &mut App) {
        TapStatusTracker::consecutive_tap_timer_timeout(self, app);
    }

    /// The deadline timer's callback. A distinct fn item per leaf.
    fn deadline_fired(self: Handle<Self>, app: &mut App) {
        BaseTapAndDragGestureRecognizer::did_exceed_deadline(self, app);
    }

    /// The drag-update throttle timer's callback. A distinct fn item per leaf.
    fn drag_update_throttled(self: Handle<Self>, app: &mut App) {
        BaseTapAndDragGestureRecognizer::handle_drag_update_throttled(self, app);
    }
}

/// A mixin for [`OneSequenceGestureRecognizer`] that tracks the number of taps
/// that occur in a series of [`PointerEvent`]s.
///
/// A tap is tracked as part of a series of taps if:
///
/// 1. The elapsed time between when a [`crate::PointerUpEvent`] and the subsequent
///    [`PointerDownEvent`] does not exceed [`K_DOUBLE_TAP_TIMEOUT`].
/// 2. The delta between the position tapped in the global coordinate system
///    and the position that was tapped previously must be less than or equal
///    to [`K_DOUBLE_TAP_SLOP`].
///
/// This mixin's state, i.e. the series of taps being tracked is reset when
/// a tap is tracked that does not meet any of the specifications stated above.
pub trait TapStatusTracker: TapAndDragLeaf {
    /// The leaf's [`TapStatusTracker`] bag.
    fn tap_status_tracker_data(self: Handle<Self>, app: &App) -> &TapStatusTrackerData {
        &app.get(self).tap_and_drag().tap_status_tracker
    }

    /// The leaf's [`TapStatusTracker`] bag, mutably.
    fn tap_status_tracker_data_mut(self: Handle<Self>, app: &mut App) -> &mut TapStatusTrackerData {
        &mut app.get_mut(self).tap_and_drag_mut().tap_status_tracker
    }

    /// The [`PointerDownEvent`] that was most recently tracked in [`add_allowed_pointer`](Self::add_allowed_pointer).
    ///
    /// This value will be none if a [`PointerDownEvent`] has not been tracked yet in
    /// [`add_allowed_pointer`](Self::add_allowed_pointer) or the timer between two taps has elapsed.
    fn current_down(self: Handle<Self>, app: &App) -> Option<PointerDownEvent> {
        app.get(self).tap_and_drag().tap_status_tracker.down.clone()
    }

    /// The [`PointerUpEvent`] that was most recently tracked in [`handle_event`](Self::handle_event).
    ///
    /// This value will be none if a [`PointerUpEvent`] has not been tracked yet in
    /// [`handle_event`](Self::handle_event) or the timer between two taps has elapsed.
    fn current_up(self: Handle<Self>, app: &App) -> Option<PointerUpEvent> {
        app.get(self).tap_and_drag().tap_status_tracker.up.clone()
    }

    /// The number of consecutive taps that the most recently tracked [`PointerDownEvent`]
    /// in [`current_down`](Self::current_down) represents.
    ///
    /// This value defaults to zero, meaning a tap series is not currently being tracked.
    fn consecutive_tap_count(self: Handle<Self>, app: &App) -> i32 {
        app.get(self)
            .tap_and_drag()
            .tap_status_tracker
            .consecutive_tap_count
    }

    /// Called when a new tap series begins.
    fn on_tap_track_start(self: Handle<Self>, app: &App) -> Option<Listener> {
        app.get(self)
            .tap_and_drag()
            .tap_status_tracker
            .on_tap_track_start
            .clone()
    }

    /// Sets [`on_tap_track_start`](Self::on_tap_track_start).
    fn set_on_tap_track_start(self: Handle<Self>, app: &mut App, callback: Option<Listener>) {
        app.get_mut(self)
            .tap_and_drag_mut()
            .tap_status_tracker
            .on_tap_track_start = callback;
    }

    /// Called when the tap series being tracked is reset.
    fn on_tap_track_reset(self: Handle<Self>, app: &App) -> Option<Listener> {
        app.get(self)
            .tap_and_drag()
            .tap_status_tracker
            .on_tap_track_reset
            .clone()
    }

    /// Sets [`on_tap_track_reset`](Self::on_tap_track_reset).
    fn set_on_tap_track_reset(self: Handle<Self>, app: &mut App, callback: Option<Listener>) {
        app.get_mut(self)
            .tap_and_drag_mut()
            .tap_status_tracker
            .on_tap_track_reset = callback;
    }

    /// When tracking a tap, the [`consecutive_tap_count`](Self::consecutive_tap_count) is incremented if the given tap
    /// falls under the tolerance specifications and reset to 1 if not.
    fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        OneSequenceGestureRecognizer::add_allowed_pointer(self, app, &event);
        let timer = app
            .get(self)
            .tap_and_drag()
            .tap_status_tracker
            .consecutive_tap_timer;
        if timer.is_some_and(|timer| !timer.is_active(app)) {
            self.tap_tracker_reset(app);
        }
        let max = app.get(self).tap_and_drag().max_consecutive_tap;
        let count = app
            .get(self)
            .tap_and_drag()
            .tap_status_tracker
            .consecutive_tap_count;
        if max == Some(count) {
            self.tap_tracker_reset(app);
        }
        app.get_mut(self).tap_and_drag_mut().tap_status_tracker.up = None;
        let down = app.get(self).tap_and_drag().tap_status_tracker.down.clone();
        if down.is_some() && !self.represents_same_series(app, &event) {
            // The given tap does not match the specifications of the series of taps being tracked,
            // reset the tap count and related state.
            app.get_mut(self)
                .tap_and_drag_mut()
                .tap_status_tracker
                .consecutive_tap_count = 1;
        } else {
            app.get_mut(self)
                .tap_and_drag_mut()
                .tap_status_tracker
                .consecutive_tap_count += 1;
        }
        self.consecutive_tap_timer_stop(app);
        // `_down` must be assigned in this method instead of [handleEvent],
        // because [acceptGesture] might be called before [handleEvent],
        // which may rely on `_down` to initiate a callback.
        self.track_tap(app, event);
    }

    /// Updates consecutive-tap tracking for move, up, and cancel events.
    fn handle_event(self: Handle<Self>, app: &mut App, event: &PointerEvent) {
        if matches!(event, PointerEvent::Move(_)) {
            let computed_slop =
                compute_hit_slop(event.kind(), app.get(self).recognizer().gesture_settings);
            let origin = app
                .get(self)
                .tap_and_drag()
                .tap_status_tracker
                .origin_position;
            let is_slop_past_tolerance = get_global_distance(event, origin) > computed_slop;

            if is_slop_past_tolerance {
                self.consecutive_tap_timer_stop(app);
                let tracker = &mut app.get_mut(self).tap_and_drag_mut().tap_status_tracker;
                tracker.previous_buttons = None;
                tracker.last_tap_offset = None;
            }
        } else if let PointerEvent::Up(up) = event {
            app.get_mut(self).tap_and_drag_mut().tap_status_tracker.up = Some(up.clone());
            if app
                .get(self)
                .tap_and_drag()
                .tap_status_tracker
                .down
                .is_some()
            {
                self.consecutive_tap_timer_stop(app);
                self.consecutive_tap_timer_start(app);
            }
        } else if matches!(event, PointerEvent::Cancel(_)) {
            self.tap_tracker_reset(app);
        }
    }

    /// Resets consecutive-tap tracking when this member loses the arena.
    fn reject_gesture(self: Handle<Self>, app: &mut App, _pointer: i64) {
        self.tap_tracker_reset(app);
    }

    /// Resets consecutive-tap tracking, then runs [`OneSequenceGestureRecognizer::dispose`].
    fn dispose(self: Handle<Self>, app: &mut App) {
        self.tap_tracker_reset(app);
        OneSequenceGestureRecognizer::dispose(self, app);
    }

    /// The consecutive tap timer may time out before a tap down/tap up event is
    /// fired. In this case we should not reset the tap tracker state immediately.
    /// Instead we should reset the tap tracker on the next call to [`add_allowed_pointer`](Self::add_allowed_pointer),
    /// if the timer is no longer active.
    fn consecutive_tap_timer_timeout(self: Handle<Self>, _app: &mut App) {}

    /// Records the down event that starts or continues a tap series.
    fn track_tap(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        {
            let tracker = &mut app.get_mut(self).tap_and_drag_mut().tap_status_tracker;
            tracker.down = Some(event.clone());
            tracker.previous_buttons = Some(event.buttons);
            tracker.last_tap_offset = Some(event.position);
            tracker.origin_position = Some(OffsetPair::new(event.local_position(), event.position));
        }
        if let Some(callback) = app
            .get(self)
            .tap_and_drag()
            .tap_status_tracker
            .on_tap_track_start
            .clone()
        {
            callback.call(app);
        }
    }

    /// Whether `buttons` matches the buttons of the previous tap in the series.
    fn has_same_button(self: Handle<Self>, app: &App, buttons: i64) -> bool {
        let previous = app
            .get(self)
            .tap_and_drag()
            .tap_status_tracker
            .previous_buttons;
        debug_assert!(previous.is_some());
        Some(buttons) == previous
    }

    /// Whether `second_tap_offset` is within [`K_DOUBLE_TAP_SLOP`] of the last tap.
    fn is_within_consecutive_tap_tolerance(
        self: Handle<Self>,
        app: &App,
        second_tap_offset: Offset,
    ) -> bool {
        let Some(last_tap_offset) = app
            .get(self)
            .tap_and_drag()
            .tap_status_tracker
            .last_tap_offset
        else {
            return false;
        };
        let difference = second_tap_offset - last_tap_offset;
        difference.distance() <= K_DOUBLE_TAP_SLOP
    }

    /// Whether `event` belongs to the series currently being tracked.
    fn represents_same_series(self: Handle<Self>, app: &App, event: &PointerDownEvent) -> bool {
        app.get(self)
            .tap_and_drag()
            .tap_status_tracker
            .consecutive_tap_timer
            .is_some()
            && self.is_within_consecutive_tap_tolerance(app, event.position)
            && self.has_same_button(app, event.buttons)
    }

    /// Starts the consecutive-tap timeout if one is not already scheduled.
    fn consecutive_tap_timer_start(self: Handle<Self>, app: &mut App) {
        if app
            .get(self)
            .tap_and_drag()
            .tap_status_tracker
            .consecutive_tap_timer
            .is_some()
        {
            return;
        }
        let timer = Timer::new(
            app,
            K_DOUBLE_TAP_TIMEOUT,
            Listener::handle_method(
                self,
                <Self as TapAndDragLeaf>::consecutive_tap_timer_timeout,
            ),
        );
        app.get_mut(self)
            .tap_and_drag_mut()
            .tap_status_tracker
            .consecutive_tap_timer = Some(timer);
    }

    /// Cancels the consecutive-tap timeout, if one is running.
    fn consecutive_tap_timer_stop(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app
            .get_mut(self)
            .tap_and_drag_mut()
            .tap_status_tracker
            .consecutive_tap_timer
            .take()
        {
            timer.cancel(app);
        }
    }

    /// The timer has timed out, i.e. the time between a [`crate::PointerUpEvent`] and the subsequent
    /// [`PointerDownEvent`] exceeded the duration of [`K_DOUBLE_TAP_TIMEOUT`], so the tap belonging
    /// to the [`PointerDownEvent`] cannot be considered part of the same tap series as the
    /// previous [`crate::PointerUpEvent`].
    fn tap_tracker_reset(self: Handle<Self>, app: &mut App) {
        self.consecutive_tap_timer_stop(app);
        {
            let tracker = &mut app.get_mut(self).tap_and_drag_mut().tap_status_tracker;
            tracker.previous_buttons = None;
            tracker.origin_position = None;
            tracker.last_tap_offset = None;
            tracker.consecutive_tap_count = 0;
            tracker.down = None;
            tracker.up = None;
        }
        if let Some(callback) = app
            .get(self)
            .tap_and_drag()
            .tap_status_tracker
            .on_tap_track_reset
            .clone()
        {
            callback.call(app);
        }
    }
}

impl<L: TapAndDragLeaf> TapStatusTracker for L {}

/// A base class for gesture recognizers that recognize taps and movements.
///
/// Takes on the responsibilities of [`crate::TapGestureRecognizer`] and
/// [`crate::DragGestureRecognizer`] in one [`crate::GestureRecognizer`].
///
/// ### Gesture arena behavior
///
/// [`BaseTapAndDragGestureRecognizer`] competes on the pointer events of
/// [`K_PRIMARY_BUTTON`] only when it has at least one non-null `on_tap*`
/// or `on_drag*` callback.
///
/// It will declare defeat if it determines that a gesture is not a
/// tap (e.g. if the pointer is dragged too far while it's contacting the
/// screen) or a drag (e.g. if the pointer was not dragged far enough to
/// be considered a drag).
///
/// This recognizer will not immediately declare victory for every tap that it
/// recognizes, but it declares victory for every drag.
///
/// The recognizer will declare victory when all other recognizer's in
/// the arena have lost, if the timer of [`K_PRESS_TIMEOUT`] elapses and a tap
/// series greater than 1 is being tracked, or until the pointer has moved
/// a sufficient global distance from the origin to be considered a drag.
///
/// If this recognizer loses the arena (either by declaring defeat or by
/// another recognizer declaring victory) while the pointer is contacting the
/// screen, it will fire [`set_on_cancel`](Self::set_on_cancel) instead of [`set_on_tap_up`](Self::set_on_tap_up) or [`set_on_drag_end`](Self::set_on_drag_end).
///
/// ### When competing with `TapGestureRecognizer` and `DragGestureRecognizer`
///
/// Similar to [`crate::TapGestureRecognizer`] and [`crate::DragGestureRecognizer`],
/// [`BaseTapAndDragGestureRecognizer`] will not aggressively declare victory when
/// it detects a tap, so when it is competing with those gesture recognizers and
/// others it has a chance of losing. Similarly, when `eager_victory_on_drag` is set
/// to `false`, this recognizer will not aggressively declare victory when it
/// detects a drag. By default, `eager_victory_on_drag` is set to `true`, so this
/// recognizer will aggressively declare victory when it detects a drag.
///
/// When competing against [`crate::TapGestureRecognizer`], if the pointer does not move past the tap
/// tolerance, then the recognizer that entered the arena first will win. In this case the
/// gesture detected is a tap. If the pointer does travel past the tap tolerance then this
/// recognizer will be declared winner by default. The gesture detected in this case is a drag.
///
/// When competing against [`crate::DragGestureRecognizer`], if the pointer does not move a sufficient
/// global distance to be considered a drag, the recognizers will tie in the arena. If the
/// pointer does travel enough distance then the recognizer that entered the arena
/// first will win. The gesture detected in this case is a drag.
///
/// See also:
///
///  * [`TapAndPanGestureRecognizer`], for a similar recognizer that accepts a drag
///    on any axis regardless if the recognizer has won the arena for the primary
///    pointer being tracked.
///  * [`TapAndHorizontalDragGestureRecognizer`], for a similar recognizer that
///    only accepts horizontal drags before it has won the arena for the primary
///    pointer being tracked.
pub trait BaseTapAndDragGestureRecognizer: TapAndDragLeaf {
    /// Configure the behavior of offsets passed to [`set_on_drag_start`](Self::set_on_drag_start).
    ///
    /// If set to [`DragStartBehavior::Start`], the [`set_on_drag_start`](Self::set_on_drag_start) callback will be called
    /// with the position of the pointer at the time this gesture recognizer won
    /// the arena. If [`DragStartBehavior::Down`], [`set_on_drag_start`](Self::set_on_drag_start) will be called with
    /// the position of the first detected down event for the pointer. When there
    /// are no other gestures competing with this gesture in the arena, there's
    /// no difference in behavior between the two settings.
    ///
    /// By default, the drag start behavior is [`DragStartBehavior::Start`].
    fn drag_start_behavior(self: Handle<Self>, app: &App) -> DragStartBehavior {
        app.get(self).tap_and_drag().drag_start_behavior
    }

    /// Sets [`drag_start_behavior`](Self::drag_start_behavior).
    fn set_drag_start_behavior(self: Handle<Self>, app: &mut App, value: DragStartBehavior) {
        app.get_mut(self).tap_and_drag_mut().drag_start_behavior = value;
    }

    /// The frequency at which the [`set_on_drag_update`](Self::set_on_drag_update) callback is called.
    ///
    /// The value defaults to none, meaning there is no delay for the update callback.
    fn drag_update_throttle_frequency(self: Handle<Self>, app: &App) -> Option<Duration> {
        app.get(self).tap_and_drag().drag_update_throttle_frequency
    }

    /// Sets [`drag_update_throttle_frequency`](Self::drag_update_throttle_frequency).
    fn set_drag_update_throttle_frequency(
        self: Handle<Self>,
        app: &mut App,
        value: Option<Duration>,
    ) {
        app.get_mut(self)
            .tap_and_drag_mut()
            .drag_update_throttle_frequency = value;
    }

    /// An upper bound for the amount of taps that can belong to one tap series.
    ///
    /// When this limit is reached the series of taps being tracked by this
    /// recognizer will be reset.
    fn max_consecutive_tap(self: Handle<Self>, app: &App) -> Option<i32> {
        app.get(self).tap_and_drag().max_consecutive_tap
    }

    /// Sets [`max_consecutive_tap`](Self::max_consecutive_tap).
    fn set_max_consecutive_tap(self: Handle<Self>, app: &mut App, value: Option<i32>) {
        app.get_mut(self).tap_and_drag_mut().max_consecutive_tap = value;
    }

    /// Whether this recognizer eagerly declares victory when it has detected
    /// a drag.
    ///
    /// When this value is `false`, this recognizer will wait until it is the last
    /// recognizer in the gesture arena before declaring victory on a drag.
    ///
    /// Defaults to `true`.
    fn eager_victory_on_drag(self: Handle<Self>, app: &App) -> bool {
        app.get(self).tap_and_drag().eager_victory_on_drag
    }

    /// Sets [`eager_victory_on_drag`](Self::eager_victory_on_drag).
    fn set_eager_victory_on_drag(self: Handle<Self>, app: &mut App, value: bool) {
        app.get_mut(self).tap_and_drag_mut().eager_victory_on_drag = value;
    }

    /// A pointer has contacted the screen at a particular location with a
    /// primary button, which might be the start of a tap.
    ///
    /// This triggers after the down event, once a short timeout ([`K_PRESS_TIMEOUT`]) has
    /// elapsed, or once the gestures has won the arena, whichever comes first.
    fn on_tap_down(self: Handle<Self>, app: &App) -> Option<GestureTapDragDownCallback> {
        app.get(self).tap_and_drag().on_tap_down.clone()
    }

    /// Sets [`on_tap_down`](Self::on_tap_down).
    fn set_on_tap_down(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureTapDragDownCallback>,
    ) {
        app.get_mut(self).tap_and_drag_mut().on_tap_down = callback;
    }

    /// A pointer has stopped contacting the screen at a particular location,
    /// which is recognized as a tap of a primary button.
    ///
    /// This triggers on the up event, if the recognizer wins the arena with it
    /// or has previously won.
    fn on_tap_up(self: Handle<Self>, app: &App) -> Option<GestureTapDragUpCallback> {
        app.get(self).tap_and_drag().on_tap_up.clone()
    }

    /// Sets [`on_tap_up`](Self::on_tap_up).
    fn set_on_tap_up(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureTapDragUpCallback>,
    ) {
        app.get_mut(self).tap_and_drag_mut().on_tap_up = callback;
    }

    /// A pointer has contacted the screen with a primary button and has begun to
    /// move.
    ///
    /// The position of the pointer is provided in the callback's `details`
    /// argument, which is a [`TapDragStartDetails`] object. The
    /// [`drag_start_behavior`](Self::drag_start_behavior) determines this position.
    fn on_drag_start(self: Handle<Self>, app: &App) -> Option<GestureTapDragStartCallback> {
        app.get(self).tap_and_drag().on_drag_start.clone()
    }

    /// Sets [`on_drag_start`](Self::on_drag_start).
    fn set_on_drag_start(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureTapDragStartCallback>,
    ) {
        app.get_mut(self).tap_and_drag_mut().on_drag_start = callback;
    }

    /// A pointer that is in contact with the screen with a primary button and
    /// moving has moved again.
    ///
    /// The distance traveled by the pointer since the last update is provided in
    /// the callback's `details` argument, which is a [`TapDragUpdateDetails`] object.
    fn on_drag_update(self: Handle<Self>, app: &App) -> Option<GestureTapDragUpdateCallback> {
        app.get(self).tap_and_drag().on_drag_update.clone()
    }

    /// Sets [`on_drag_update`](Self::on_drag_update).
    fn set_on_drag_update(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureTapDragUpdateCallback>,
    ) {
        app.get_mut(self).tap_and_drag_mut().on_drag_update = callback;
    }

    /// A pointer that was previously in contact with the screen with a primary
    /// button and moving is no longer in contact with the screen.
    ///
    /// The velocity is provided in the callback's `details` argument, which is a
    /// [`TapDragEndDetails`] object.
    fn on_drag_end(self: Handle<Self>, app: &App) -> Option<GestureTapDragEndCallback> {
        app.get(self).tap_and_drag().on_drag_end.clone()
    }

    /// Sets [`on_drag_end`](Self::on_drag_end).
    fn set_on_drag_end(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureTapDragEndCallback>,
    ) {
        app.get_mut(self).tap_and_drag_mut().on_drag_end = callback;
    }

    /// The pointer that previously triggered [`set_on_tap_down`](Self::set_on_tap_down) did not complete.
    fn on_cancel(self: Handle<Self>, app: &App) -> Option<GestureCancelCallback> {
        app.get(self).tap_and_drag().on_cancel.clone()
    }

    /// Sets [`on_cancel`](Self::on_cancel).
    fn set_on_cancel(self: Handle<Self>, app: &mut App, callback: Option<GestureCancelCallback>) {
        app.get_mut(self).tap_and_drag_mut().on_cancel = callback;
    }

    /// Checks whether or not a pointer is allowed to be tracked by this
    /// recognizer.
    fn is_pointer_allowed(self: Handle<Self>, app: &App, event: &PointerDownEvent) -> bool {
        let data = app.get(self).tap_and_drag();
        if data.primary_pointer.is_none() {
            match event.buttons {
                K_PRIMARY_BUTTON => {
                    if data.on_tap_down.is_none()
                        && data.on_drag_start.is_none()
                        && data.on_drag_update.is_none()
                        && data.on_drag_end.is_none()
                        && data.on_tap_up.is_none()
                        && data.on_cancel.is_none()
                    {
                        return false;
                    }
                }
                _ => return false,
            }
        } else if Some(event.pointer) != data.primary_pointer {
            return false;
        }
        GestureRecognizer::is_pointer_allowed(self, app, event)
    }

    /// Registers a new pointer that is allowed by this recognizer.
    fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        if app.get(self).tap_and_drag().drag_state == DragState::Ready {
            TapStatusTracker::add_allowed_pointer(self, app, event.clone());
            let deadline = app.get(self).tap_and_drag().deadline;
            let timer = Timer::new(
                app,
                deadline,
                Listener::handle_method(self, Self::deadline_fired),
            );
            let data = app.get_mut(self).tap_and_drag_mut();
            data.primary_pointer = Some(event.pointer);
            data.global_distance_moved = 0.0;
            data.global_distance_moved_all_axes = 0.0;
            data.drag_state = DragState::Possible;
            data.initial_position = OffsetPair::new(event.local_position(), event.position);
            data.current_position = data.initial_position;
            data.deadline_timer = Some(timer);
        }
    }

    /// Handles a pointer being added that's not allowed by this recognizer.
    fn handle_non_allowed_pointer(self: Handle<Self>, app: &mut App, event: &PointerDownEvent) {
        // There can be multiple drags simultaneously. Their effects are combined.
        if event.buttons != K_PRIMARY_BUTTON
            && !app.get(self).tap_and_drag().won_arena_for_primary_pointer
        {
            OneSequenceGestureRecognizer::handle_non_allowed_pointer(self, app);
        }
    }

    /// Called when this member wins the arena for the given pointer id.
    fn accept_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        if Some(pointer) != app.get(self).tap_and_drag().primary_pointer {
            return;
        }

        self.stop_deadline_timer(app);

        debug_assert!(
            !app.get(self)
                .tap_and_drag()
                .accepted_active_pointers
                .contains(&pointer)
        );
        app.get_mut(self)
            .tap_and_drag_mut()
            .accepted_active_pointers
            .insert(pointer);

        // Called when this recognizer is accepted by the [GestureArena].
        if let Some(down) = TapStatusTracker::current_down(self, app) {
            self.check_tap_down(app, &down);
        }

        app.get_mut(self)
            .tap_and_drag_mut()
            .won_arena_for_primary_pointer = true;

        // resolve(GestureDisposition.accepted) will be called when the [PointerMoveEvent]
        // has moved a sufficient global distance to be considered a drag and
        // `eagerVictoryOnDrag` is set to `true`.
        let start = app.get(self).tap_and_drag().start.clone();
        let eager = app.get(self).tap_and_drag().eager_victory_on_drag;
        if let Some(start) = start
            && eager
        {
            debug_assert_eq!(app.get(self).tap_and_drag().drag_state, DragState::Accepted);
            debug_assert!(TapStatusTracker::current_up(self, app).is_none());
            self.accept_drag(app, &start);
        }

        // This recognizer will wait until it is the last one in the gesture arena
        // before accepting a drag when `eagerVictoryOnDrag` is set to `false`.
        let start = app.get(self).tap_and_drag().start.clone();
        let eager = app.get(self).tap_and_drag().eager_victory_on_drag;
        if let Some(start) = start
            && !eager
        {
            debug_assert_eq!(app.get(self).tap_and_drag().drag_state, DragState::Possible);
            debug_assert!(TapStatusTracker::current_up(self, app).is_none());
            app.get_mut(self).tap_and_drag_mut().drag_state = DragState::Accepted;
            self.accept_drag(app, &start);
        }

        if let Some(up) = TapStatusTracker::current_up(self, app) {
            self.check_tap_up(app, &up);
        }
    }

    /// Called when the number of pointers this recognizer is tracking changes
    /// from one to zero.
    fn did_stop_tracking_last_pointer(self: Handle<Self>, app: &mut App, pointer: i64) {
        match app.get(self).tap_and_drag().drag_state {
            DragState::Ready => {
                self.check_cancel(app);
                self.resolve(app, GestureDisposition::Rejected);
            }
            DragState::Possible => {
                if app.get(self).tap_and_drag().past_slop_tolerance {
                    // This means the pointer was not accepted as a tap.
                    if app.get(self).tap_and_drag().won_arena_for_primary_pointer {
                        // If the recognizer has already won the arena for the primary pointer being tracked
                        // but the pointer has exceeded the tap tolerance, then the pointer is accepted as a
                        // drag gesture.
                        if let Some(down) = TapStatusTracker::current_down(self, app) {
                            if !app
                                .get_mut(self)
                                .tap_and_drag_mut()
                                .accepted_active_pointers
                                .remove(&pointer)
                            {
                                OneSequenceGestureRecognizer::resolve_pointer(
                                    self,
                                    app,
                                    pointer,
                                    GestureDisposition::Rejected,
                                );
                            }
                            app.get_mut(self).tap_and_drag_mut().drag_state = DragState::Accepted;
                            self.accept_drag(app, &PointerEvent::Down(down));
                            self.check_drag_end(app);
                        }
                    } else {
                        self.check_cancel(app);
                        self.resolve(app, GestureDisposition::Rejected);
                    }
                } else {
                    // The pointer is accepted as a tap.
                    if let Some(up) = TapStatusTracker::current_up(self, app) {
                        self.check_tap_up(app, &up);
                    }
                }
            }
            DragState::Accepted => {
                // For the case when the pointer has been accepted as a drag.
                // Meaning [_checkTapDown] and [_checkDragStart] have already ran.
                self.check_drag_end(app);
            }
        }

        self.stop_deadline_timer(app);
        let data = app.get_mut(self).tap_and_drag_mut();
        data.start = None;
        data.drag_state = DragState::Ready;
        data.past_slop_tolerance = false;
    }

    /// Called when a pointer event is routed to this recognizer.
    fn handle_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        if Some(event.pointer()) != app.get(self).tap_and_drag().primary_pointer {
            return;
        }
        TapStatusTracker::handle_event(self, app, &event);
        if matches!(event, PointerEvent::Move(_)) {
            // Receiving a [PointerMoveEvent], does not automatically mean the pointer
            // being tracked is doing a drag gesture. There is some drift that can happen
            // between the initial [PointerDownEvent] and subsequent [PointerMoveEvent]s.
            // Accessing [_pastSlopTolerance] lets us know if our tap has moved past the
            // acceptable tolerance. If the pointer does not move past this tolerance than
            // it is not considered a drag.
            //
            // To be recognized as a drag, the [PointerMoveEvent] must also have moved
            // a sufficient global distance from the initial [PointerDownEvent] to be
            // accepted as a drag. This logic is handled in [_hasSufficientGlobalDistanceToAccept].
            //
            // The recognizer will also detect the gesture as a drag when the pointer
            // has been accepted and it has moved past the [slopTolerance] but has not moved
            // a sufficient global distance from the initial position to be considered a drag.
            // In this case since the gesture cannot be a tap, it defaults to a drag.
            let computed_slop =
                compute_hit_slop(event.kind(), app.get(self).recognizer().gesture_settings);
            let initial = app.get(self).tap_and_drag().initial_position;
            let past = app.get(self).tap_and_drag().past_slop_tolerance
                || get_global_distance(&event, Some(initial)) > computed_slop;
            app.get_mut(self).tap_and_drag_mut().past_slop_tolerance = past;

            let drag_state = app.get(self).tap_and_drag().drag_state;
            if drag_state == DragState::Accepted {
                app.get_mut(self).tap_and_drag_mut().current_position =
                    OffsetPair::from_event_position(&event);
                self.check_drag_update(app, &event, None);
            } else if drag_state == DragState::Possible {
                if app.get(self).tap_and_drag().start.is_none() {
                    // Only check for a drag if the start of a drag was not already identified.
                    self.check_drag(app, &event);
                }

                // This can occur when the recognizer is accepted before a [PointerMoveEvent] has been
                // received that moves the pointer a sufficient global distance to be considered a drag.
                if app.get(self).tap_and_drag().start.is_some()
                    && app.get(self).tap_and_drag().won_arena_for_primary_pointer
                {
                    app.get_mut(self).tap_and_drag_mut().drag_state = DragState::Accepted;
                    let start = app.get(self).tap_and_drag().start.clone().unwrap();
                    self.accept_drag(app, &start);
                }
            }
        } else if matches!(event, PointerEvent::Up(_)) {
            match app.get(self).tap_and_drag().drag_state {
                DragState::Possible => {
                    // The drag has not been accepted before a [PointerUpEvent], therefore the recognizer
                    // attempts to recognize a tap.
                    OneSequenceGestureRecognizer::stop_tracking_if_pointer_no_longer_down(
                        self, app, &event,
                    );
                }
                DragState::Accepted => {
                    self.give_up_pointer(app, event.pointer());
                }
                DragState::Ready => {}
            }
        } else if matches!(event, PointerEvent::Cancel(_)) {
            app.get_mut(self).tap_and_drag_mut().drag_state = DragState::Ready;
            self.give_up_pointer(app, event.pointer());
        }
    }

    /// Called when this member loses the arena for the given pointer id.
    fn reject_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        if Some(pointer) != app.get(self).tap_and_drag().primary_pointer {
            return;
        }
        TapStatusTracker::reject_gesture(self, app, pointer);

        self.stop_deadline_timer(app);
        self.give_up_pointer(app, pointer);
        self.reset_taps(app);
        self.reset_drag_update_throttle(app);
    }

    /// Releases any resources used by the object.
    fn dispose(self: Handle<Self>, app: &mut App) {
        self.stop_deadline_timer(app);
        self.reset_drag_update_throttle(app);
        TapStatusTracker::dispose(self, app);
    }

    /// Drag updates may require throttling to avoid excessive updating, such as for text layouts in text
    /// fields. The frequency of invocations is controlled by the [`drag_update_throttle_frequency`](Self::drag_update_throttle_frequency).
    ///
    /// Once the drag gesture ends, any pending drag update will be fired
    /// immediately. See [`check_drag_end`](Self::check_drag_end).
    fn handle_drag_update_throttled(self: Handle<Self>, app: &mut App) {
        debug_assert!(
            app.get(self)
                .tap_and_drag()
                .last_drag_update_details
                .is_some()
        );
        if let Some(callback) = app.get(self).tap_and_drag().on_drag_update.clone() {
            let details = app
                .get(self)
                .tap_and_drag()
                .last_drag_update_details
                .clone()
                .unwrap();
            GestureRecognizer::invoke_callback(self, app, "onDragUpdate", |app| {
                callback(app, details);
            });
        }
        let data = app.get_mut(self).tap_and_drag_mut();
        data.drag_update_throttle_timer = None;
        data.last_drag_update_details = None;
    }

    /// Accepts the pointer as a drag and fires start (and a first update if the
    /// local delta is non-zero).
    fn accept_drag(self: Handle<Self>, app: &mut App, event: &PointerEvent) {
        debug_assert_eq!(app.get(self).tap_and_drag().drag_state, DragState::Accepted);

        if !app.get(self).tap_and_drag().won_arena_for_primary_pointer {
            return;
        }

        if app.get(self).tap_and_drag().drag_start_behavior == DragStartBehavior::Start {
            let delta = OffsetPair::new(event.local_delta(), event.delta());
            let data = app.get_mut(self).tap_and_drag_mut();
            data.initial_position = data.initial_position + delta;
            data.current_position = data.initial_position;
        }
        self.check_drag_start(app, event);
        let local_delta = event.local_delta();
        if local_delta != Offset::ZERO {
            app.get_mut(self).tap_and_drag_mut().current_position =
                OffsetPair::from_event_position(event);
            let initial_local = app.get(self).tap_and_drag().initial_position.local;
            let corrected_local_position = initial_local + local_delta;
            let local_to_global_transform =
                event.transform().and_then(|transform| transform.invert());
            let global_update_delta = transform_delta_via_positions(
                corrected_local_position,
                None,
                local_delta,
                local_to_global_transform,
            );
            let update_delta = OffsetPair::new(local_delta, global_update_delta);
            // Only adds delta for down behaviour
            let corrected = app.get(self).tap_and_drag().initial_position + update_delta;
            self.check_drag_update(app, event, Some(corrected));
        }
    }

    /// Accumulates travel and, if past the axis slop, records the drag start.
    fn check_drag(self: Handle<Self>, app: &mut App, event: &PointerEvent) {
        let local_to_global_transform = event.transform().and_then(|transform| transform.invert());
        let moved_locally = TapAndDragLeaf::get_delta_for_details(self, app, event.local_delta());
        let primary = TapAndDragLeaf::get_primary_value_from_offset(self, app, moved_locally);
        let moved = transform_delta_via_positions(
            event.local_position(),
            None,
            moved_locally,
            local_to_global_transform,
        )
        .distance()
            * sign(primary.unwrap_or(1.0));
        app.get_mut(self).tap_and_drag_mut().global_distance_moved += moved;
        let all_axes = transform_delta_via_positions(
            event.local_position(),
            None,
            event.local_delta(),
            local_to_global_transform,
        )
        .distance()
            * sign(1.0);
        app.get_mut(self)
            .tap_and_drag_mut()
            .global_distance_moved_all_axes += all_axes;
        let sufficient =
            TapAndDragLeaf::has_sufficient_global_distance_to_accept(self, app, event.kind());
        let won = app.get(self).tap_and_drag().won_arena_for_primary_pointer;
        let all_axes_abs = app
            .get(self)
            .tap_and_drag()
            .global_distance_moved_all_axes
            .abs();
        let pan_slop = compute_pan_slop(event.kind(), app.get(self).recognizer().gesture_settings);
        if sufficient || (won && all_axes_abs > pan_slop) {
            app.get_mut(self).tap_and_drag_mut().start = Some(event.clone());
            if app.get(self).tap_and_drag().eager_victory_on_drag {
                app.get_mut(self).tap_and_drag_mut().drag_state = DragState::Accepted;
                if !app.get(self).tap_and_drag().won_arena_for_primary_pointer {
                    self.resolve(app, GestureDisposition::Accepted);
                }
            }
        }
    }

    /// Fires [`set_on_tap_down`](Self::set_on_tap_down) once for the current primary pointer.
    fn check_tap_down(self: Handle<Self>, app: &mut App, event: &PointerDownEvent) {
        if app.get(self).tap_and_drag().sent_tap_down {
            return;
        }

        let details = TapDragDownDetails::new(
            event.position,
            event.local_position(),
            Some(GestureRecognizer::kind_for_pointer(
                self,
                app,
                event.pointer,
            )),
            TapStatusTracker::consecutive_tap_count(self, app),
        );

        if let Some(callback) = app.get(self).tap_and_drag().on_tap_down.clone() {
            GestureRecognizer::invoke_callback(self, app, "onTapDown", |app| {
                callback(app, details);
            });
        }

        app.get_mut(self).tap_and_drag_mut().sent_tap_down = true;
    }

    /// Fires [`set_on_tap_up`](Self::set_on_tap_up) if this recognizer has won the arena.
    fn check_tap_up(self: Handle<Self>, app: &mut App, event: &PointerUpEvent) {
        if !app.get(self).tap_and_drag().won_arena_for_primary_pointer {
            return;
        }

        let up_details = TapDragUpDetails::new(
            event.position,
            event.local_position(),
            event.kind,
            TapStatusTracker::consecutive_tap_count(self, app),
        );

        if let Some(callback) = app.get(self).tap_and_drag().on_tap_up.clone() {
            GestureRecognizer::invoke_callback(self, app, "onTapUp", |app| {
                callback(app, up_details);
            });
        }

        self.reset_taps(app);
        if !app
            .get_mut(self)
            .tap_and_drag_mut()
            .accepted_active_pointers
            .remove(&event.pointer)
        {
            OneSequenceGestureRecognizer::resolve_pointer(
                self,
                app,
                event.pointer,
                GestureDisposition::Rejected,
            );
        }
    }

    /// Fires [`set_on_drag_start`](Self::set_on_drag_start).
    fn check_drag_start(self: Handle<Self>, app: &mut App, event: &PointerEvent) {
        if let Some(callback) = app.get(self).tap_and_drag().on_drag_start.clone() {
            let initial = app.get(self).tap_and_drag().initial_position;
            let details = TapDragStartDetails::new(
                initial.global,
                initial.local,
                Some(event.time_stamp()),
                Some(GestureRecognizer::kind_for_pointer(
                    self,
                    app,
                    event.pointer(),
                )),
                TapStatusTracker::consecutive_tap_count(self, app),
            );

            GestureRecognizer::invoke_callback(self, app, "onDragStart", |app| {
                callback(app, details);
            });
        }

        app.get_mut(self).tap_and_drag_mut().start = None;
    }

    /// Fires [`set_on_drag_update`](Self::set_on_drag_update), or schedules it if throttling is enabled.
    fn check_drag_update(
        self: Handle<Self>,
        app: &mut App,
        event: &PointerEvent,
        corrected: Option<OffsetPair>,
    ) {
        let global_position = corrected
            .map(|pair| pair.global)
            .unwrap_or_else(|| event.position());
        let local_position = corrected
            .map(|pair| pair.local)
            .unwrap_or_else(|| event.local_position());
        let initial = app.get(self).tap_and_drag().initial_position;

        let details = TapDragUpdateDetails::new(
            global_position,
            local_position,
            Some(event.time_stamp()),
            event.local_delta(),
            None,
            Some(GestureRecognizer::kind_for_pointer(
                self,
                app,
                event.pointer(),
            )),
            global_position - initial.global,
            local_position - initial.local,
            TapStatusTracker::consecutive_tap_count(self, app),
        );

        if let Some(frequency) = app.get(self).tap_and_drag().drag_update_throttle_frequency {
            app.get_mut(self)
                .tap_and_drag_mut()
                .last_drag_update_details = Some(details);
            // Only schedule a new timer if there's not one pending.
            if app
                .get(self)
                .tap_and_drag()
                .drag_update_throttle_timer
                .is_none()
            {
                let timer = Timer::new(
                    app,
                    frequency,
                    Listener::handle_method(self, Self::drag_update_throttled),
                );
                app.get_mut(self)
                    .tap_and_drag_mut()
                    .drag_update_throttle_timer = Some(timer);
            }
        } else if let Some(callback) = app.get(self).tap_and_drag().on_drag_update.clone() {
            GestureRecognizer::invoke_callback(self, app, "onDragUpdate", |app| {
                callback(app, details);
            });
        }
    }

    /// Fires a pending throttled update, then [`set_on_drag_end`](Self::set_on_drag_end).
    fn check_drag_end(self: Handle<Self>, app: &mut App) {
        let global_position = app.get(self).tap_and_drag().current_position.global;
        let local_position = app.get(self).tap_and_drag().current_position.local;

        if app
            .get(self)
            .tap_and_drag()
            .drag_update_throttle_timer
            .is_some()
        {
            // If there's already an update scheduled, trigger it immediately and
            // cancel the timer.
            if let Some(timer) = app
                .get_mut(self)
                .tap_and_drag_mut()
                .drag_update_throttle_timer
                .take()
            {
                timer.cancel(app);
            }
            self.handle_drag_update_throttled(app);
        }

        let end_details = TapDragEndDetails::new(
            global_position,
            Some(local_position),
            Velocity::ZERO,
            Some(0.0),
            TapStatusTracker::consecutive_tap_count(self, app),
        );

        if let Some(callback) = app.get(self).tap_and_drag().on_drag_end.clone() {
            GestureRecognizer::invoke_callback(self, app, "onDragEnd", |app| {
                callback(app, end_details);
            });
        }

        self.reset_taps(app);
        self.reset_drag_update_throttle(app);
    }

    /// Fires [`set_on_cancel`](Self::set_on_cancel) if tap-down was sent.
    fn check_cancel(self: Handle<Self>, app: &mut App) {
        if !app.get(self).tap_and_drag().sent_tap_down {
            // Do not fire tap cancel if [onTapDown] was never called.
            return;
        }
        if let Some(callback) = app.get(self).tap_and_drag().on_cancel.clone() {
            GestureRecognizer::invoke_callback(self, app, "onCancel", |app| callback.call(app));
        }
        self.reset_drag_update_throttle(app);
        self.reset_taps(app);
    }

    /// Fires tap-down after [`K_PRESS_TIMEOUT`], and accepts a consecutive tap
    /// series greater than one so a long-press recognizer cannot win.
    fn did_exceed_deadline(self: Handle<Self>, app: &mut App) {
        if let Some(down) = TapStatusTracker::current_down(self, app) {
            self.check_tap_down(app, &down);

            if TapStatusTracker::consecutive_tap_count(self, app) > 1 {
                // If our consecutive tap count is greater than 1, i.e. is a double tap or greater,
                // then this recognizer declares victory to prevent the [LongPressGestureRecognizer]
                // from declaring itself the winner if a double tap is held for too long.
                self.resolve(app, GestureDisposition::Accepted);
            }
        }
    }

    /// Stops tracking `pointer` and rejects it if it was never accepted.
    fn give_up_pointer(self: Handle<Self>, app: &mut App, pointer: i64) {
        OneSequenceGestureRecognizer::stop_tracking_pointer(self, app, pointer);
        // If the pointer was never accepted, then it is rejected since this recognizer is no longer
        // interested in winning the gesture arena for it.
        if !app
            .get_mut(self)
            .tap_and_drag_mut()
            .accepted_active_pointers
            .remove(&pointer)
        {
            OneSequenceGestureRecognizer::resolve_pointer(
                self,
                app,
                pointer,
                GestureDisposition::Rejected,
            );
        }
    }

    /// Clears tap-down / arena-won / primary-pointer state for the next series.
    fn reset_taps(self: Handle<Self>, app: &mut App) {
        let data = app.get_mut(self).tap_and_drag_mut();
        data.sent_tap_down = false;
        data.won_arena_for_primary_pointer = false;
        data.primary_pointer = None;
    }

    /// Cancels a pending throttled drag update.
    fn reset_drag_update_throttle(self: Handle<Self>, app: &mut App) {
        if app
            .get(self)
            .tap_and_drag()
            .drag_update_throttle_frequency
            .is_none()
        {
            return;
        }
        app.get_mut(self)
            .tap_and_drag_mut()
            .last_drag_update_details = None;
        if let Some(timer) = app
            .get_mut(self)
            .tap_and_drag_mut()
            .drag_update_throttle_timer
            .take()
        {
            timer.cancel(app);
        }
    }

    /// Cancels the press-timeout timer, if one is running.
    fn stop_deadline_timer(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).tap_and_drag_mut().deadline_timer.take() {
            timer.cancel(app);
        }
    }
}

impl<L: TapAndDragLeaf> BaseTapAndDragGestureRecognizer for L {}

/// The members every tap-and-drag leaf inherits verbatim: the constructor, the
/// superclass field accessors, and the `RecognizerLeaf` overrides that forward
/// to [`BaseTapAndDragGestureRecognizer`].
macro_rules! tap_and_drag_gesture_recognizer_leaf {
    ($name:ident, $description:literal) => {
        impl $name {
            /// Create a gesture recognizer.
            pub fn new(app: &mut App) -> Handle<$name> {
                app.create($name {
                    recognizer: GestureRecognizerData::new(),
                    one_sequence: OneSequenceData::new(),
                    tap_and_drag: TapAndDragData::new(),
                })
            }

            /// The kind of devices that are allowed to be recognized.
            pub fn supported_devices(
                self: Handle<Self>,
                app: &mut App,
                devices: impl IntoIterator<Item = PointerDeviceKind>,
            ) -> Handle<$name> {
                app.get_mut(self).recognizer.supported_devices =
                    Some(devices.into_iter().collect());
                self
            }

            /// Sets the kind of devices that are allowed to be recognized; `None`
            /// tracks and recognizes events from all device kinds.
            pub fn set_supported_devices(
                self: Handle<Self>,
                app: &mut App,
                devices: Option<std::collections::HashSet<PointerDeviceKind>>,
            ) {
                app.get_mut(self).recognizer.supported_devices = devices;
            }

            /// Called when interaction starts. Limits buttons this recognizer
            /// accepts.
            pub fn allowed_buttons_filter(
                self: Handle<Self>,
                app: &mut App,
                filter: impl Fn(i64) -> bool + 'static,
            ) -> Handle<$name> {
                app.get_mut(self).recognizer.allowed_buttons_filter = Rc::new(filter);
                self
            }

            /// Optional device specific configuration that takes precedence over
            /// framework defaults.
            pub fn gesture_settings(
                self: Handle<Self>,
                app: &App,
            ) -> Option<DeviceGestureSettings> {
                app.get(self).recognizer.gesture_settings
            }

            /// Sets [`gesture_settings`](Self::gesture_settings).
            pub fn set_gesture_settings(
                self: Handle<Self>,
                app: &mut App,
                settings: Option<DeviceGestureSettings>,
            ) {
                app.get_mut(self).recognizer.gesture_settings = settings;
            }

            /// The team that this recognizer belongs to, if any.
            pub fn team(self: Handle<Self>, app: &App) -> Option<Handle<GestureArenaTeam>> {
                app.get(self).one_sequence.team
            }

            /// The [`team`](Self::team) can only be set once.
            pub fn set_team(self: Handle<Self>, app: &mut App, value: Handle<GestureArenaTeam>) {
                debug_assert!(app.get(self).one_sequence.entries.is_empty());
                debug_assert!(app.get(self).one_sequence.tracked_pointers.is_empty());
                debug_assert!(app.get(self).one_sequence.team.is_none());
                app.get_mut(self).one_sequence.team = Some(value);
            }

            /// Registers a new pointer that might be relevant to this gesture
            /// detector.
            pub fn add_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
                GestureRecognizer::add_pointer(self, app, event);
            }

            /// Registers a new pointer pan/zoom that might be relevant to this
            /// gesture detector.
            pub fn add_pointer_pan_zoom(
                self: Handle<Self>,
                app: &mut App,
                event: PointerPanZoomStartEvent,
            ) {
                GestureRecognizer::add_pointer_pan_zoom(self, app, event);
            }

            /// Releases any resources used by the object.
            pub fn dispose(self: Handle<Self>, app: &mut App) {
                <Self as RecognizerLeaf>::dispose(self, app);
            }

            /// Returns a very short pretty description of the gesture that the
            /// recognizer looks for.
            pub fn debug_description(self: Handle<Self>) -> &'static str {
                <Self as RecognizerLeaf>::debug_description(self)
            }
        }

        impl RecognizerLeafData for $name {
            fn recognizer(&self) -> &GestureRecognizerData {
                &self.recognizer
            }
            fn recognizer_mut(&mut self) -> &mut GestureRecognizerData {
                &mut self.recognizer
            }
        }

        impl OneSequenceLeafData for $name {
            fn one_sequence(&self) -> &OneSequenceData {
                &self.one_sequence
            }
            fn one_sequence_mut(&mut self) -> &mut OneSequenceData {
                &mut self.one_sequence
            }
        }

        impl TapAndDragLeafData for $name {
            fn tap_and_drag(&self) -> &TapAndDragData {
                &self.tap_and_drag
            }
            fn tap_and_drag_mut(&mut self) -> &mut TapAndDragData {
                &mut self.tap_and_drag
            }
        }

        impl RecognizerLeaf for $name {
            fn handle_non_allowed_pointer(
                self: Handle<Self>,
                app: &mut App,
                event: &PointerDownEvent,
            ) {
                BaseTapAndDragGestureRecognizer::handle_non_allowed_pointer(self, app, event);
            }

            fn is_pointer_allowed(self: Handle<Self>, app: &App, event: &PointerDownEvent) -> bool {
                BaseTapAndDragGestureRecognizer::is_pointer_allowed(self, app, event)
            }

            fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
                BaseTapAndDragGestureRecognizer::add_allowed_pointer(self, app, event);
            }

            fn handle_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
                BaseTapAndDragGestureRecognizer::handle_event(self, app, event);
            }

            fn accept_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
                BaseTapAndDragGestureRecognizer::accept_gesture(self, app, pointer);
            }

            fn reject_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
                BaseTapAndDragGestureRecognizer::reject_gesture(self, app, pointer);
            }

            fn did_stop_tracking_last_pointer(self: Handle<Self>, app: &mut App, pointer: i64) {
                BaseTapAndDragGestureRecognizer::did_stop_tracking_last_pointer(self, app, pointer);
            }

            fn dispose(self: Handle<Self>, app: &mut App) {
                BaseTapAndDragGestureRecognizer::dispose(self, app);
            }

            fn debug_description(self: Handle<Self>) -> &'static str {
                $description
            }
        }
    };
}

/// Recognizes taps along with movement in the horizontal direction.
///
/// Before this recognizer has won the arena for the primary pointer being tracked,
/// it will only accept a drag on the horizontal axis. If a drag is detected after
/// this recognizer has won the arena then it will accept a drag on any axis.
///
/// See also:
///
///  * [`BaseTapAndDragGestureRecognizer`], for the class that provides the main
///    implementation details of this recognizer.
///  * [`TapAndPanGestureRecognizer`], for a similar recognizer that accepts a drag
///    on any axis regardless if the recognizer has won the arena for the primary
///    pointer being tracked.
///  * [`crate::HorizontalDragGestureRecognizer`], for a similar recognizer that only recognizes
///    horizontal movement.
pub struct TapAndHorizontalDragGestureRecognizer {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
    tap_and_drag: TapAndDragData,
}

tap_and_drag_gesture_recognizer_leaf!(
    TapAndHorizontalDragGestureRecognizer,
    "tap and horizontal drag"
);

impl TapAndDragLeaf for TapAndHorizontalDragGestureRecognizer {
    fn has_sufficient_global_distance_to_accept(
        self: Handle<Self>,
        app: &App,
        pointer_device_kind: PointerDeviceKind,
    ) -> bool {
        app.get(self).tap_and_drag.global_distance_moved.abs()
            > compute_hit_slop(
                pointer_device_kind,
                app.get(self).recognizer.gesture_settings,
            )
    }

    fn get_delta_for_details(self: Handle<Self>, _app: &App, delta: Offset) -> Offset {
        Offset::new(delta.dx(), 0.0)
    }

    fn get_primary_value_from_offset(self: Handle<Self>, _app: &App, value: Offset) -> Option<f64> {
        Some(value.dx())
    }
}

/// Recognizes taps along with both horizontal and vertical movement.
///
/// This recognizer will accept a drag on any axis, regardless if it has won the
/// arena for the primary pointer being tracked.
///
/// See also:
///
///  * [`BaseTapAndDragGestureRecognizer`], for the class that provides the main
///    implementation details of this recognizer.
///  * [`TapAndHorizontalDragGestureRecognizer`], for a similar recognizer that
///    only accepts horizontal drags before it has won the arena for the primary
///    pointer being tracked.
///  * [`crate::PanGestureRecognizer`], for a similar recognizer that only recognizes
///    movement.
pub struct TapAndPanGestureRecognizer {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
    tap_and_drag: TapAndDragData,
}

tap_and_drag_gesture_recognizer_leaf!(TapAndPanGestureRecognizer, "tap and pan");

impl TapAndDragLeaf for TapAndPanGestureRecognizer {
    fn has_sufficient_global_distance_to_accept(
        self: Handle<Self>,
        app: &App,
        pointer_device_kind: PointerDeviceKind,
    ) -> bool {
        app.get(self).tap_and_drag.global_distance_moved.abs()
            > compute_pan_slop(
                pointer_device_kind,
                app.get(self).recognizer.gesture_settings,
            )
    }

    fn get_delta_for_details(self: Handle<Self>, _app: &App, delta: Offset) -> Offset {
        delta
    }

    fn get_primary_value_from_offset(
        self: Handle<Self>,
        _app: &App,
        _value: Offset,
    ) -> Option<f64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::time::Duration;

    use inset_foundation::AppCell;

    use super::*;
    use crate::binding::GestureBinding;
    use crate::events::{PointerMoveEvent, PointerUpEvent};
    use crate::monodrag::{DragGestureRecognizer, VerticalDragGestureRecognizer};

    /// Anything longer than [`K_DOUBLE_TAP_TIMEOUT`] will reset the consecutive tap count.
    const K_CONSECUTIVE_TAP_DELAY: Duration = Duration::from_millis(150);

    struct TestPointer {
        pointer: i64,
        kind: PointerDeviceKind,
        location: Offset,
    }

    impl TestPointer {
        fn new(pointer: i64) -> TestPointer {
            TestPointer {
                pointer,
                kind: PointerDeviceKind::Touch,
                location: Offset::ZERO,
            }
        }

        fn down(&mut self, position: Offset) -> PointerDownEvent {
            self.location = position;
            PointerDownEvent {
                pointer: self.pointer,
                kind: self.kind,
                position,
                buttons: K_PRIMARY_BUTTON,
                ..PointerDownEvent::default()
            }
        }

        fn move_to(&mut self, position: Offset) -> PointerMoveEvent {
            let delta = position - self.location;
            self.location = position;
            PointerMoveEvent {
                pointer: self.pointer,
                kind: self.kind,
                position,
                delta,
                down: true,
                buttons: K_PRIMARY_BUTTON,
                ..PointerMoveEvent::default()
            }
        }

        fn up(&mut self) -> PointerUpEvent {
            PointerUpEvent {
                pointer: self.pointer,
                kind: self.kind,
                position: self.location,
                ..PointerUpEvent::default()
            }
        }
    }

    fn close_arena(app: &mut App, pointer: i64) {
        GestureBinding::instance(app)
            .gesture_arena(app)
            .close(app, pointer);
    }

    fn route(app: &mut App, event: PointerEvent) {
        GestureBinding::instance(app)
            .pointer_router(app)
            .route(app, event);
        app.drain_microtasks();
    }

    fn sweep(app: &mut App, pointer: i64) {
        GestureBinding::instance(app)
            .gesture_arena(app)
            .sweep(app, pointer);
    }

    fn down(pointer: i64, position: Offset) -> PointerDownEvent {
        PointerDownEvent {
            pointer,
            position,
            buttons: K_PRIMARY_BUTTON,
            ..PointerDownEvent::default()
        }
    }

    fn up(pointer: i64, position: Offset) -> PointerUpEvent {
        PointerUpEvent {
            pointer,
            position,
            ..PointerUpEvent::default()
        }
    }

    fn wire_pan(
        tap_and_drag: Handle<TapAndPanGestureRecognizer>,
        app: &mut App,
        events: &Rc<RefCell<Vec<String>>>,
    ) {
        tap_and_drag.set_drag_start_behavior(app, DragStartBehavior::Down);
        tap_and_drag.set_max_consecutive_tap(app, Some(3));
        let down_events = Rc::clone(events);
        tap_and_drag.set_on_tap_down(
            app,
            Some(Rc::new(move |_app, details: TapDragDownDetails| {
                down_events
                    .borrow_mut()
                    .push(format!("down#{}", details.consecutive_tap_count));
            })),
        );
        let up_events = Rc::clone(events);
        tap_and_drag.set_on_tap_up(
            app,
            Some(Rc::new(move |_app, details: TapDragUpDetails| {
                up_events
                    .borrow_mut()
                    .push(format!("up#{}", details.consecutive_tap_count));
            })),
        );
        let start_events = Rc::clone(events);
        tap_and_drag.set_on_drag_start(
            app,
            Some(Rc::new(move |_app, details: TapDragStartDetails| {
                start_events
                    .borrow_mut()
                    .push(format!("panstart#{}", details.consecutive_tap_count));
            })),
        );
        let update_events = Rc::clone(events);
        tap_and_drag.set_on_drag_update(
            app,
            Some(Rc::new(move |_app, details: TapDragUpdateDetails| {
                update_events
                    .borrow_mut()
                    .push(format!("panupdate#{}", details.consecutive_tap_count));
            })),
        );
        let end_events = Rc::clone(events);
        tap_and_drag.set_on_drag_end(
            app,
            Some(Rc::new(move |_app, details: TapDragEndDetails| {
                end_events
                    .borrow_mut()
                    .push(format!("panend#{}", details.consecutive_tap_count));
            })),
        );
        let cancel_events = Rc::clone(events);
        tap_and_drag.set_on_cancel(
            app,
            Some(Listener::new(move |_app| {
                cancel_events.borrow_mut().push("cancel".to_string());
            })),
        );
    }

    fn wire_horizontal(
        tap_and_drag: Handle<TapAndHorizontalDragGestureRecognizer>,
        app: &mut App,
        events: &Rc<RefCell<Vec<String>>>,
    ) {
        tap_and_drag.set_drag_start_behavior(app, DragStartBehavior::Down);
        tap_and_drag.set_max_consecutive_tap(app, Some(3));
        let down_events = Rc::clone(events);
        tap_and_drag.set_on_tap_down(
            app,
            Some(Rc::new(move |_app, details: TapDragDownDetails| {
                down_events
                    .borrow_mut()
                    .push(format!("down#{}", details.consecutive_tap_count));
            })),
        );
        let up_events = Rc::clone(events);
        tap_and_drag.set_on_tap_up(
            app,
            Some(Rc::new(move |_app, details: TapDragUpDetails| {
                up_events
                    .borrow_mut()
                    .push(format!("up#{}", details.consecutive_tap_count));
            })),
        );
        let start_events = Rc::clone(events);
        tap_and_drag.set_on_drag_start(
            app,
            Some(Rc::new(move |_app, details: TapDragStartDetails| {
                start_events.borrow_mut().push(format!(
                    "horizontaldragstart#{}",
                    details.consecutive_tap_count
                ));
            })),
        );
        let update_events = Rc::clone(events);
        tap_and_drag.set_on_drag_update(
            app,
            Some(Rc::new(move |_app, details: TapDragUpdateDetails| {
                update_events.borrow_mut().push(format!(
                    "horizontaldragupdate#{}",
                    details.consecutive_tap_count
                ));
            })),
        );
        let end_events = Rc::clone(events);
        tap_and_drag.set_on_drag_end(
            app,
            Some(Rc::new(move |_app, details: TapDragEndDetails| {
                end_events.borrow_mut().push(format!(
                    "horizontaldragend#{}",
                    details.consecutive_tap_count
                ));
            })),
        );
        let cancel_events = Rc::clone(events);
        tap_and_drag.set_on_cancel(
            app,
            Some(Listener::new(move |_app| {
                cancel_events.borrow_mut().push("cancel".to_string());
            })),
        );
    }

    #[test]
    fn recognizes_consecutive_taps() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tap_and_drag = TapAndPanGestureRecognizer::new(&mut app);
        let events = Rc::new(RefCell::new(Vec::new()));
        wire_pan(tap_and_drag, &mut app, &events);

        let down1 = down(1, Offset::new(10.0, 10.0));
        let up1 = up(1, Offset::new(11.0, 9.0));
        tap_and_drag.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up1));
        sweep(&mut app, 1);
        assert_eq!(*events.borrow(), ["down#1", "up#1"]);

        events.borrow_mut().clear();
        drop(app);
        cell.elapse(K_CONSECUTIVE_TAP_DELAY);
        let mut app = cell.borrow_mut();
        let down2 = down(2, Offset::new(12.0, 12.0));
        let up2 = up(2, Offset::new(13.0, 11.0));
        tap_and_drag.add_pointer(&mut app, down2.clone());
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down2));
        route(&mut app, PointerEvent::Up(up2));
        sweep(&mut app, 2);
        assert_eq!(*events.borrow(), ["down#2", "up#2"]);

        tap_and_drag.dispose(&mut app);
    }

    #[test]
    fn resets_if_times_out_in_between_taps() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tap_and_drag = TapAndPanGestureRecognizer::new(&mut app);
        let events = Rc::new(RefCell::new(Vec::new()));
        wire_pan(tap_and_drag, &mut app, &events);

        let down1 = down(1, Offset::new(10.0, 10.0));
        let up1 = up(1, Offset::new(11.0, 9.0));
        tap_and_drag.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up1));
        sweep(&mut app, 1);
        assert_eq!(*events.borrow(), ["down#1", "up#1"]);

        events.borrow_mut().clear();
        drop(app);
        cell.elapse(Duration::from_millis(1000));
        let mut app = cell.borrow_mut();
        let down2 = down(2, Offset::new(12.0, 12.0));
        let up2 = up(2, Offset::new(13.0, 11.0));
        tap_and_drag.add_pointer(&mut app, down2.clone());
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down2));
        route(&mut app, PointerEvent::Up(up2));
        sweep(&mut app, 2);
        assert_eq!(*events.borrow(), ["down#1", "up#1"]);

        tap_and_drag.dispose(&mut app);
    }

    #[test]
    fn should_recognize_drag() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tap_and_drag = TapAndPanGestureRecognizer::new(&mut app);
        let events = Rc::new(RefCell::new(Vec::new()));
        wire_pan(tap_and_drag, &mut app, &events);

        let mut pointer = TestPointer::new(5);
        let down_event = pointer.down(Offset::new(10.0, 10.0));
        tap_and_drag.add_pointer(&mut app, down_event.clone());
        close_arena(&mut app, 5);
        route(&mut app, PointerEvent::Down(down_event));
        route(
            &mut app,
            PointerEvent::Move(pointer.move_to(Offset::new(40.0, 45.0))),
        );
        route(&mut app, PointerEvent::Up(pointer.up()));
        sweep(&mut app, 5);
        assert_eq!(
            *events.borrow(),
            ["down#1", "panstart#1", "panupdate#1", "panend#1"]
        );

        tap_and_drag.dispose(&mut app);
    }

    #[test]
    fn tap_and_horizontal_drag_loses_to_vertical_drag() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tap_and_drag = TapAndHorizontalDragGestureRecognizer::new(&mut app);
        let events = Rc::new(RefCell::new(Vec::new()));
        wire_horizontal(tap_and_drag, &mut app, &events);

        let vertical = VerticalDragGestureRecognizer::new(&mut app);
        let vertical_events = Rc::clone(&events);
        vertical.set_on_start(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                vertical_events
                    .borrow_mut()
                    .push("verticalstart".to_string());
            })),
        );
        let vertical_events = Rc::clone(&events);
        vertical.set_on_update(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                vertical_events
                    .borrow_mut()
                    .push("verticalupdate".to_string());
            })),
        );
        let vertical_events = Rc::clone(&events);
        vertical.set_on_end(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                vertical_events.borrow_mut().push("verticalend".to_string());
            })),
        );
        let vertical_events = Rc::clone(&events);
        vertical.set_on_cancel(
            &mut app,
            Some(Listener::new(move |_app| {
                vertical_events
                    .borrow_mut()
                    .push("verticalcancel".to_string());
            })),
        );

        let mut pointer = TestPointer::new(5);
        let down_event = pointer.down(Offset::new(10.0, 10.0));
        tap_and_drag.add_pointer(&mut app, down_event.clone());
        vertical.add_pointer(&mut app, down_event.clone());
        close_arena(&mut app, 5);
        route(&mut app, PointerEvent::Down(down_event));
        route(
            &mut app,
            PointerEvent::Move(pointer.move_to(Offset::new(10.0, 45.0))),
        );
        route(
            &mut app,
            PointerEvent::Move(pointer.move_to(Offset::new(10.0, 100.0))),
        );
        route(&mut app, PointerEvent::Up(pointer.up()));
        assert_eq!(
            *events.borrow(),
            ["verticalstart", "verticalupdate", "verticalend"]
        );

        tap_and_drag.dispose(&mut app);
        DragGestureRecognizer::dispose(vertical, &mut app);
    }
}
