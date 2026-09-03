//! Flutter counterpart: `gestures/gesture_details.dart`.

use reveal_embedder::Offset;

/// An abstract interface representing gesture details that include positional information.
///
/// This class serve as a common interface for gesture details that involve positional data,
/// such as dragging and tapping. It simplifies gesture handling by enabling the use of shared logic
/// across multiple gesture types, users can create a method to handle a single gesture details
/// with this position information.
pub trait PositionedGestureDetails {
    /// The global position at which the pointer interacts with the screen.
    ///
    /// See also:
    ///
    ///  * [`local_position`](Self::local_position), which is the [`global_position`](Self::global_position) transformed to the
    ///    coordinate space of the event receiver.
    fn global_position(&self) -> Offset;

    /// The local position in the coordinate system of the event receiver at
    /// which the pointer interacts with the screen.
    ///
    /// See also:
    ///
    ///  * [`global_position`](Self::global_position), which is the global position at which the pointer
    ///    interacts with the screen.
    fn local_position(&self) -> Offset;
}
