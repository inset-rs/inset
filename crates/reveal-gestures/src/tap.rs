//! Flutter counterpart: `gestures/tap.dart`.
//!
//! Details objects only. `BaseTapGestureRecognizer` / `TapGestureRecognizer`
//! wait on the `GestureRecognizer` hierarchy and a `Timer` for `deadline`.

use reveal_embedder::{Offset, PointerDeviceKind};

use crate::gesture_details::PositionedGestureDetails;

/// Details for `GestureTapDownCallback`, such as position.
///
/// See also:
///
///  * `GestureDetector.onTapDown`, which receives this information.
///  * `TapGestureRecognizer`, which passes this information to one of its callbacks.
pub struct TapDownDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position at which the pointer contacted the screen.
    pub local_position: Offset,

    /// The kind of the device that initiated the event.
    pub kind: Option<PointerDeviceKind>,
}

impl TapDownDetails {
    /// Creates details for a tap-down callback.
    pub fn new(
        global_position: Offset,
        local_position: Option<Offset>,
        kind: Option<PointerDeviceKind>,
    ) -> TapDownDetails {
        TapDownDetails {
            global_position,
            local_position: local_position.unwrap_or(global_position),
            kind,
        }
    }
}

impl Default for TapDownDetails {
    fn default() -> TapDownDetails {
        TapDownDetails {
            global_position: Offset::ZERO,
            local_position: Offset::ZERO,
            kind: None,
        }
    }
}

impl PositionedGestureDetails for TapDownDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

/// Details for `GestureTapUpCallback`, such as position.
///
/// See also:
///
///  * `GestureDetector.onTapUp`, which receives this information.
///  * `TapGestureRecognizer`, which passes this information to one of its callbacks.
pub struct TapUpDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position at which the pointer contacted the screen.
    pub local_position: Offset,

    /// The kind of the device that initiated the event.
    pub kind: PointerDeviceKind,
}

impl TapUpDetails {
    /// Creates a [`TapUpDetails`] data object.
    pub fn new(
        global_position: Offset,
        local_position: Option<Offset>,
        kind: PointerDeviceKind,
    ) -> TapUpDetails {
        TapUpDetails {
            global_position,
            local_position: local_position.unwrap_or(global_position),
            kind,
        }
    }
}

impl PositionedGestureDetails for TapUpDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

/// Details object for callbacks that use `GestureTapMoveCallback`.
///
/// See also:
///
/// * `GestureDetector.onTapMove`, which receives this information.
/// * `TapGestureRecognizer`, which passes this information to one of its callbacks.
pub struct TapMoveDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position at which the pointer contacted the screen.
    pub local_position: Offset,

    /// The kind of the device that initiated the event.
    pub kind: PointerDeviceKind,

    /// The amount the pointer has moved in the coordinate space of the
    /// event receiver since the previous update.
    pub delta: Offset,
}

impl TapMoveDetails {
    /// Creates a [`TapMoveDetails`] data object.
    pub fn new(
        kind: PointerDeviceKind,
        global_position: Offset,
        delta: Offset,
        local_position: Option<Offset>,
    ) -> TapMoveDetails {
        TapMoveDetails {
            kind,
            global_position,
            delta,
            local_position: local_position.unwrap_or(global_position),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_position_defaults_to_global() {
        let down = TapDownDetails::new(Offset::new(3.0, 4.0), None, None);
        assert_eq!(down.local_position, Offset::new(3.0, 4.0));
        let up = TapUpDetails::new(Offset::new(1.0, 2.0), None, PointerDeviceKind::Touch);
        assert_eq!(up.local_position, Offset::new(1.0, 2.0));
        let move_details = TapMoveDetails::new(
            PointerDeviceKind::Touch,
            Offset::new(5.0, 6.0),
            Offset::new(1.0, 0.0),
            None,
        );
        assert_eq!(move_details.local_position, Offset::new(5.0, 6.0));
    }
}
