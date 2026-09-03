//! Flutter counterpart: `gestures/recognizer.dart`.
//!
//! Superclass field bags live here. The leaf Handle and `super` namespaces
//! are in [`tap`](crate::tap) until a second leaf needs the generic form.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use reveal_embedder::{Offset, PointerDeviceKind};
use reveal_foundation::Timer;

use crate::arena::GestureArenaEntry;
use crate::events::PointerEvent;
use crate::gesture_settings::DeviceGestureSettings;
use crate::team::GestureArenaTeam;

/// Signature for [`GestureRecognizerData::allowed_buttons_filter`].
///
/// Used to filter the input buttons of incoming pointer events.
/// The parameter `buttons` comes from `PointerEvent.buttons`.
pub type AllowedButtonsFilter = Rc<dyn Fn(i64) -> bool>;

/// `-1` is used as a sentinel value to indicate no touch slop was specified.
pub(crate) const UNSET_TOUCH_SLOP: f64 = -1.0;

/// The default value for `allowedButtonsFilter`.
/// Accept any input.
pub(crate) fn default_button_accept_behavior(_buttons: i64) -> bool {
    true
}

/// Configuration of offset passed to `DragStartDetails`.
///
/// See also:
///
///  * `DragGestureRecognizer.dragStartBehavior`, which gives an example for the
///    different behaviors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragStartBehavior {
    /// Set the initial offset at the position where the first down event was
    /// detected.
    Down,

    /// Set the initial position at the position where this gesture recognizer
    /// won the arena.
    Start,
}

/// Configuration of multi-finger drag strategy on multi-touch devices.
///
/// When dragging with only one finger, there's no difference in behavior
/// between all the settings.
///
/// Used by `DragGestureRecognizer.multitouchDragStrategy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultitouchDragStrategy {
    /// Only the latest active pointer is tracked by the recognizer.
    ///
    /// If the tracked pointer is released, the first accepted of the remaining active
    /// pointers will continue to be tracked.
    ///
    /// This is the behavior typically seen on Android.
    LatestPointer,

    /// All active pointers will be tracked, and the result is computed from
    /// the boundary pointers.
    ///
    /// The scrolling offset is determined by the maximum deltas of both directions.
    ///
    /// This is the behavior typically seen on iOS.
    AverageBoundaryPointers,

    /// All active pointers will be tracked together. The scrolling offset
    /// is the sum of the offsets of all active pointers.
    SumAllPointers,
}

/// The possible states of a `PrimaryPointerGestureRecognizer`.
///
/// The recognizer advances from [`Ready`](Self::Ready) to [`Possible`](Self::Possible) when it starts tracking a
/// primary pointer. Where it advances from there depends on how the gesture is
/// resolved for that pointer:
///
///  * If the primary pointer is resolved by the gesture winning the arena, the
///    recognizer stays in the [`Possible`](Self::Possible) state as long as it continues to track
///    a pointer.
///  * If the primary pointer is resolved by the gesture being rejected and
///    losing the arena, the recognizer's state advances to [`Defunct`](Self::Defunct).
///
/// Once the recognizer has stopped tracking any remaining pointers, the
/// recognizer returns to [`Ready`](Self::Ready).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureRecognizerState {
    /// The recognizer is ready to start recognizing a gesture.
    Ready,

    /// The sequence of pointer events seen thus far is consistent with the
    /// gesture the recognizer is attempting to recognize but the gesture has not
    /// been accepted definitively.
    Possible,

    /// Further pointer events cannot cause this recognizer to recognize the
    /// gesture until the recognizer returns to the [`Ready`](Self::Ready) state (typically when
    /// all the pointers the recognizer is tracking are removed from the screen).
    Defunct,
}

/// Data associated with a pointer event.
///
/// This is stored by `GestureRecognizer`s on a per-pointer basis.
#[derive(Clone, Copy)]
pub(crate) struct RecognizerEventData {
    pub kind: PointerDeviceKind,
    pub buttons: i64,
}

/// Field bag for Dart's `GestureRecognizer`.
pub(crate) struct GestureRecognizerData {
    pub gesture_settings: Option<DeviceGestureSettings>,
    pub supported_devices: Option<HashSet<PointerDeviceKind>>,
    pub allowed_buttons_filter: AllowedButtonsFilter,
    pub pointer_to_event_data: HashMap<i64, RecognizerEventData>,
}

impl GestureRecognizerData {
    pub(crate) fn new() -> GestureRecognizerData {
        GestureRecognizerData {
            gesture_settings: None,
            supported_devices: None,
            allowed_buttons_filter: Rc::new(default_button_accept_behavior),
            pointer_to_event_data: HashMap::new(),
        }
    }
}

/// Field bag for Dart's `OneSequenceGestureRecognizer`.
pub(crate) struct OneSequenceData {
    pub entries: HashMap<i64, GestureArenaEntry>,
    pub tracked_pointers: HashSet<i64>,
    pub team: Option<GestureArenaTeam>,
}

impl OneSequenceData {
    pub(crate) fn new() -> OneSequenceData {
        OneSequenceData {
            entries: HashMap::new(),
            tracked_pointers: HashSet::new(),
            team: None,
        }
    }
}

/// Field bag for Dart's `PrimaryPointerGestureRecognizer`.
pub(crate) struct PrimaryPointerData {
    pub deadline: Option<Duration>,
    pub pre_accept_slop_tolerance: Option<f64>,
    pub post_accept_slop_tolerance: Option<f64>,
    pub state: GestureRecognizerState,
    pub primary_pointer: Option<i64>,
    pub initial_position: Option<OffsetPair>,
    pub gesture_accepted: bool,
    pub timer: Option<Timer>,
    pub deadline_event: Option<crate::events::PointerDownEvent>,
}

impl PrimaryPointerData {
    pub(crate) fn new(
        deadline: Option<Duration>,
        pre_accept_slop_tolerance: Option<f64>,
        post_accept_slop_tolerance: Option<f64>,
    ) -> PrimaryPointerData {
        debug_assert!(
            pre_accept_slop_tolerance == Some(UNSET_TOUCH_SLOP)
                || pre_accept_slop_tolerance.is_none()
                || pre_accept_slop_tolerance.is_some_and(|value| value >= 0.0),
            "The preAcceptSlopTolerance must be unspecified, positive, or null"
        );
        debug_assert!(
            post_accept_slop_tolerance == Some(UNSET_TOUCH_SLOP)
                || post_accept_slop_tolerance.is_none()
                || post_accept_slop_tolerance.is_some_and(|value| value >= 0.0),
            "The postAcceptSlopTolerance must be unspecified, positive, or null"
        );
        PrimaryPointerData {
            deadline,
            pre_accept_slop_tolerance,
            post_accept_slop_tolerance,
            state: GestureRecognizerState::Ready,
            primary_pointer: None,
            initial_position: None,
            gesture_accepted: false,
            timer: None,
            deadline_event: None,
        }
    }
}

/// A container for a [`local`](Self::local) and [`global`](Self::global) [`Offset`] pair.
///
/// Usually, the [`global`](Self::global) [`Offset`] is in the coordinate space of the screen
/// after conversion to logical pixels and the [`local`](Self::local) offset is the same
/// [`Offset`], but transformed to a local coordinate space.
#[derive(Clone, Copy, Debug)]
pub struct OffsetPair {
    /// The [`Offset`] in the local coordinate space.
    pub local: Offset,

    /// The [`Offset`] in the global coordinate space after conversion to logical
    /// pixels.
    pub global: Offset,
}

impl OffsetPair {
    /// A [`OffsetPair`] where both [`Offset`]s are [`Offset::ZERO`].
    pub const ZERO: OffsetPair = OffsetPair {
        local: Offset::ZERO,
        global: Offset::ZERO,
    };

    /// Creates a [`OffsetPair`] combining a [`local`](Self::local) and [`global`](Self::global) [`Offset`].
    pub const fn new(local: Offset, global: Offset) -> OffsetPair {
        OffsetPair { local, global }
    }

    /// Creates a [`OffsetPair`] from [`PointerEvent::local_position`] and
    /// [`PointerEvent::position`].
    pub fn from_event_position(event: &PointerEvent) -> OffsetPair {
        OffsetPair {
            local: event.local_position(),
            global: event.position(),
        }
    }

    /// Creates a [`OffsetPair`] from [`PointerEvent::local_delta`] and
    /// [`PointerEvent::delta`].
    pub fn from_event_delta(event: &PointerEvent) -> OffsetPair {
        OffsetPair {
            local: event.local_delta(),
            global: event.delta(),
        }
    }
}

impl std::ops::Add for OffsetPair {
    type Output = OffsetPair;

    fn add(self, other: OffsetPair) -> OffsetPair {
        OffsetPair {
            local: self.local + other.local,
            global: self.global + other.global,
        }
    }
}

impl std::ops::Sub for OffsetPair {
    type Output = OffsetPair;

    fn sub(self, other: OffsetPair) -> OffsetPair {
        OffsetPair {
            local: self.local - other.local,
            global: self.global - other.global,
        }
    }
}

impl fmt::Display for OffsetPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OffsetPair(local: {:?}, global: {:?})",
            self.local, self.global
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reveal_embedder::Offset;

    use crate::events::{PointerDownEvent, PointerEvent};

    #[test]
    fn from_event_position_and_delta_and_arithmetic() {
        let event = PointerEvent::Down(PointerDownEvent {
            position: Offset::new(10.0, 20.0),
            delta: Offset::new(1.0, 2.0),
            ..PointerDownEvent::default()
        });
        let position = OffsetPair::from_event_position(&event);
        assert_eq!(position.global, Offset::new(10.0, 20.0));
        assert_eq!(position.local, Offset::new(10.0, 20.0));
        let delta = OffsetPair::from_event_delta(&event);
        assert_eq!(delta.global, Offset::new(1.0, 2.0));
        let sum = position + delta;
        assert_eq!(sum.global, Offset::new(11.0, 22.0));
        assert_eq!((sum - delta).global, position.global);
        assert_eq!(OffsetPair::ZERO.global, Offset::ZERO);
    }
}
