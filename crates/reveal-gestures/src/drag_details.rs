//! Flutter counterpart: `gestures/drag_details.dart`.

use std::fmt::{self, Debug};
use std::time::Duration;

use reveal_embedder::{Offset, PointerDeviceKind};
use reveal_foundation::ValueChanged;

use crate::gesture_details::PositionedGestureDetails;
use crate::velocity::Velocity;

/// Details object for callbacks that use [`GestureDragDownCallback`].
///
/// See also:
///
///  * [`DragGestureRecognizer::set_on_down`](crate::DragGestureRecognizer::set_on_down),
///    which uses [`GestureDragDownCallback`].
///  * [`DragStartDetails`], the details for [`GestureDragStartCallback`].
///  * [`DragUpdateDetails`], the details for [`GestureDragUpdateCallback`].
///  * [`DragEndDetails`], the details for
///    [`GestureDragEndCallback`](crate::GestureDragEndCallback).
#[derive(Clone)]
pub struct DragDownDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position in the coordinate system of the event receiver at
    /// which the pointer contacted the screen.
    pub local_position: Offset,
}

impl DragDownDetails {
    /// Creates details for a [`GestureDragDownCallback`].
    ///
    /// If `local_position` is none, it defaults to the global position.
    pub fn new(global_position: Offset, local_position: Option<Offset>) -> DragDownDetails {
        DragDownDetails {
            global_position,
            local_position: local_position.unwrap_or(global_position),
        }
    }
}

impl Default for DragDownDetails {
    fn default() -> DragDownDetails {
        DragDownDetails::new(Offset::ZERO, None)
    }
}

impl PositionedGestureDetails for DragDownDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

impl Debug for DragDownDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DragDownDetails")
            .field("globalPosition", &self.global_position)
            .field("localPosition", &self.local_position)
            .finish()
    }
}

/// Signature for when a pointer has contacted the screen and might begin to
/// move.
///
/// The `details` object provides the position of the touch.
///
/// See [`DragGestureRecognizer::set_on_down`](crate::DragGestureRecognizer::set_on_down).
pub type GestureDragDownCallback = ValueChanged<DragDownDetails>;

/// Details object for callbacks that use [`GestureDragStartCallback`].
///
/// See also:
///
///  * [`DragGestureRecognizer::set_on_start`](crate::DragGestureRecognizer::set_on_start),
///    which uses [`GestureDragStartCallback`].
///  * [`DragDownDetails`], the details for [`GestureDragDownCallback`].
///  * [`DragUpdateDetails`], the details for [`GestureDragUpdateCallback`].
///  * [`DragEndDetails`], the details for
///    [`GestureDragEndCallback`](crate::GestureDragEndCallback).
#[derive(Clone)]
pub struct DragStartDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position in the coordinate system of the event receiver at
    /// which the pointer contacted the screen.
    pub local_position: Offset,

    /// Recorded timestamp of the source pointer event that triggered the drag
    /// event.
    ///
    /// Could be none if triggered from proxied events such as accessibility.
    pub source_time_stamp: Option<Duration>,

    /// The kind of the device that initiated the event.
    pub kind: Option<PointerDeviceKind>,
}

impl DragStartDetails {
    /// Creates details for a [`GestureDragStartCallback`].
    ///
    /// If `local_position` is none, it defaults to the global position.
    pub fn new(
        global_position: Offset,
        local_position: Option<Offset>,
        source_time_stamp: Option<Duration>,
        kind: Option<PointerDeviceKind>,
    ) -> DragStartDetails {
        DragStartDetails {
            global_position,
            local_position: local_position.unwrap_or(global_position),
            source_time_stamp,
            kind,
        }
    }
}

impl Default for DragStartDetails {
    fn default() -> DragStartDetails {
        DragStartDetails::new(Offset::ZERO, None, None, None)
    }
}

impl PositionedGestureDetails for DragStartDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

impl Debug for DragStartDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DragStartDetails")
            .field("globalPosition", &self.global_position)
            .field("localPosition", &self.local_position)
            .field("sourceTimeStamp", &self.source_time_stamp)
            .field("kind", &self.kind)
            .finish()
    }
}

/// Signature for when a pointer has contacted the screen and has begun to move.
///
/// The `details` object provides the position of the touch when it first
/// touched the surface.
///
/// See [`DragGestureRecognizer::set_on_start`](crate::DragGestureRecognizer::set_on_start).
pub type GestureDragStartCallback = ValueChanged<DragStartDetails>;

/// Details object for callbacks that use [`GestureDragUpdateCallback`].
///
/// See also:
///
///  * [`DragGestureRecognizer::set_on_update`](crate::DragGestureRecognizer::set_on_update),
///    which uses [`GestureDragUpdateCallback`].
///  * [`DragDownDetails`], the details for [`GestureDragDownCallback`].
///  * [`DragStartDetails`], the details for [`GestureDragStartCallback`].
///  * [`DragEndDetails`], the details for
///    [`GestureDragEndCallback`](crate::GestureDragEndCallback).
#[derive(Clone)]
pub struct DragUpdateDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position in the coordinate system of the event receiver at
    /// which the pointer contacted the screen.
    pub local_position: Offset,

    /// Recorded timestamp of the source pointer event that triggered the drag
    /// event.
    ///
    /// Could be none if triggered from proxied events such as accessibility.
    pub source_time_stamp: Option<Duration>,

    /// The amount the pointer has moved in the coordinate space of the event
    /// receiver since the previous update.
    ///
    /// If the [`GestureDragUpdateCallback`] is for a one-dimensional drag (e.g.,
    /// a horizontal or vertical drag), then this offset contains only the delta
    /// in that direction (i.e., the coordinate in the other direction is zero).
    ///
    /// Defaults to zero if not specified in the constructor.
    pub delta: Offset,

    /// The amount the pointer has moved along the primary axis in the coordinate
    /// space of the event receiver since the previous update.
    ///
    /// If the [`GestureDragUpdateCallback`] is for a one-dimensional drag (e.g.,
    /// a horizontal or vertical drag), then this value contains the component of
    /// [`delta`](Self::delta) along the primary axis (e.g., horizontal or
    /// vertical, respectively). Otherwise, if the [`GestureDragUpdateCallback`]
    /// is for a two-dimensional drag (e.g., a pan), then this value is none.
    ///
    /// Defaults to none if not specified in the constructor.
    pub primary_delta: Option<f64>,

    /// The kind of the device that initiated the event.
    pub kind: Option<PointerDeviceKind>,
}

impl DragUpdateDetails {
    /// Creates details for a [`GestureDragUpdateCallback`].
    ///
    /// If [`primary_delta`](Self::primary_delta) is non-none, then its value
    /// must match one of the coordinates of [`delta`](Self::delta) and the other
    /// coordinate must be zero.
    ///
    /// If `local_position` is none, it defaults to the global position.
    pub fn new(
        global_position: Offset,
        local_position: Option<Offset>,
        source_time_stamp: Option<Duration>,
        delta: Offset,
        primary_delta: Option<f64>,
        kind: Option<PointerDeviceKind>,
    ) -> DragUpdateDetails {
        debug_assert!(
            primary_delta.is_none()
                || (primary_delta == Some(delta.dx()) && delta.dy() == 0.0)
                || (primary_delta == Some(delta.dy()) && delta.dx() == 0.0)
        );
        DragUpdateDetails {
            global_position,
            local_position: local_position.unwrap_or(global_position),
            source_time_stamp,
            delta,
            primary_delta,
            kind,
        }
    }
}

impl PositionedGestureDetails for DragUpdateDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

impl Debug for DragUpdateDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DragUpdateDetails")
            .field("globalPosition", &self.global_position)
            .field("localPosition", &self.local_position)
            .field("sourceTimeStamp", &self.source_time_stamp)
            .field("delta", &self.delta)
            .field("primaryDelta", &self.primary_delta)
            .finish()
    }
}

/// Signature for when a pointer that is in contact with the screen and moving
/// has moved again.
///
/// The `details` object provides the position of the touch and the distance it
/// has traveled since the last update.
///
/// See [`DragGestureRecognizer::set_on_update`](crate::DragGestureRecognizer::set_on_update).
pub type GestureDragUpdateCallback = ValueChanged<DragUpdateDetails>;

/// Details object for callbacks that use
/// [`GestureDragEndCallback`](crate::GestureDragEndCallback).
///
/// See also:
///
///  * [`DragGestureRecognizer::set_on_end`](crate::DragGestureRecognizer::set_on_end),
///    which uses [`GestureDragEndCallback`](crate::GestureDragEndCallback).
///  * [`DragDownDetails`], the details for [`GestureDragDownCallback`].
///  * [`DragStartDetails`], the details for [`GestureDragStartCallback`].
///  * [`DragUpdateDetails`], the details for [`GestureDragUpdateCallback`].
#[derive(Clone)]
pub struct DragEndDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position in the coordinate system of the event receiver at
    /// which the pointer contacted the screen.
    pub local_position: Offset,

    /// The velocity the pointer was moving when it stopped contacting the
    /// screen.
    ///
    /// Defaults to zero if not specified in the constructor.
    pub velocity: Velocity,

    /// The velocity the pointer was moving along the primary axis when it
    /// stopped contacting the screen, in logical pixels per second.
    ///
    /// If the [`GestureDragEndCallback`](crate::GestureDragEndCallback) is for a
    /// one-dimensional drag (e.g., a horizontal or vertical drag), then this
    /// value contains the component of [`velocity`](Self::velocity) along the
    /// primary axis (e.g., horizontal or vertical, respectively). Otherwise, if
    /// the callback is for a two-dimensional drag (e.g., a pan), then this value
    /// is none.
    ///
    /// Defaults to none if not specified in the constructor.
    pub primary_velocity: Option<f64>,
}

impl DragEndDetails {
    /// Creates details for a [`GestureDragEndCallback`](crate::GestureDragEndCallback).
    ///
    /// If [`primary_velocity`](Self::primary_velocity) is non-none, its value
    /// must match one of the coordinates of `velocity.pixels_per_second` and the
    /// other coordinate must be zero.
    ///
    /// If `local_position` is none, it defaults to the global position.
    pub fn new(
        global_position: Offset,
        local_position: Option<Offset>,
        velocity: Velocity,
        primary_velocity: Option<f64>,
    ) -> DragEndDetails {
        debug_assert!(
            primary_velocity.is_none()
                || (primary_velocity == Some(velocity.pixels_per_second.dx())
                    && velocity.pixels_per_second.dy() == 0.0)
                || (primary_velocity == Some(velocity.pixels_per_second.dy())
                    && velocity.pixels_per_second.dx() == 0.0)
        );
        DragEndDetails {
            global_position,
            local_position: local_position.unwrap_or(global_position),
            velocity,
            primary_velocity,
        }
    }
}

impl Default for DragEndDetails {
    fn default() -> DragEndDetails {
        DragEndDetails::new(Offset::ZERO, None, Velocity::ZERO, None)
    }
}

impl PositionedGestureDetails for DragEndDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

impl Debug for DragEndDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DragEndDetails")
            .field("globalPosition", &self.global_position)
            .field("localPosition", &self.local_position)
            .field("velocity", &self.velocity)
            .field("primaryVelocity", &self.primary_velocity)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_position_defaults_to_global() {
        let down = DragDownDetails::new(Offset::new(3.0, 4.0), None);
        assert_eq!(down.local_position, Offset::new(3.0, 4.0));
        let start = DragStartDetails::new(Offset::new(1.0, 2.0), None, None, None);
        assert_eq!(start.local_position, Offset::new(1.0, 2.0));
        let update = DragUpdateDetails::new(
            Offset::new(5.0, 6.0),
            None,
            None,
            Offset::new(0.0, 1.0),
            Some(1.0),
            None,
        );
        assert_eq!(update.local_position, Offset::new(5.0, 6.0));
        assert_eq!(update.primary_delta, Some(1.0));
        let end = DragEndDetails::new(
            Offset::new(7.0, 8.0),
            None,
            Velocity::new(Offset::new(0.0, 9.0)),
            Some(9.0),
        );
        assert_eq!(end.local_position, Offset::new(7.0, 8.0));
        assert_eq!(end.primary_velocity, Some(9.0));
    }
}
