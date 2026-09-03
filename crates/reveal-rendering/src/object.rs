//! Flutter counterpart: `rendering/object.dart` (`Constraints` only).
//!
//! `RenderObject` / `PipelineOwner` / `PaintingContext` wait — Handle tree,
//! layout pipeline, and skipped `SemanticsBinding` are not a mechanical copy.

/// Immutable layout constraints.
///
/// Flutter's counterpart is the abstract `Constraints` class.
pub trait Constraints {
    /// Whether there is exactly one size possible given these constraints.
    fn is_tight(&self) -> bool;

    /// Whether the constraint is expressed in a consistent manner.
    fn is_normalized(&self) -> bool;

    /// Asserts that the constraints are valid.
    ///
    /// Returns the same as [`is_normalized`](Self::is_normalized) if asserts
    /// are disabled.
    fn debug_assert_is_valid(&self, is_applied_constraint: bool) -> bool {
        let _ = is_applied_constraint;
        debug_assert!(self.is_normalized());
        self.is_normalized()
    }
}
