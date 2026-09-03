//! Flutter counterpart: `gestures/recognizer.dart`.
//!
//! The `GestureRecognizer` class hierarchy is not here yet — Rust has no
//! inheritance encoding proved for four abstract superclasses plus `Timer`
//! for `deadline`. [`OffsetPair`] and the public enums are the mechanical
//! slice.

use std::fmt;

use reveal_embedder::Offset;

use crate::events::PointerEvent;

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
