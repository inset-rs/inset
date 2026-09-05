//! Flutter counterpart: `rendering/layout_helper.dart` (`ChildLayouter`,
//! `ChildBaselineGetter`, `ChildLayoutHelper`).

use reveal_embedder::{Size, TextBaseline};
use reveal_foundation::App;

use crate::box_::{AnyRenderBox, BoxConstraints};

/// Signature for a function that takes a [`AnyRenderBox`] and returns the [`Size`] that the box
/// would have if it were laid out with the given [`BoxConstraints`].
///
/// [`ChildLayoutHelper::dry_layout_child`] and [`ChildLayoutHelper::layout_child`] adhere to
/// this signature.
pub type ChildLayouter = fn(&mut App, AnyRenderBox, BoxConstraints) -> Size;

/// Signature for a function that takes a [`AnyRenderBox`] and returns the baseline offset this
/// box would have if it were laid out with the given [`BoxConstraints`].
///
/// [`ChildLayoutHelper::get_dry_baseline`] and [`ChildLayoutHelper::get_baseline`] adhere to
/// this signature.
pub type ChildBaselineGetter =
    fn(&mut App, AnyRenderBox, BoxConstraints, TextBaseline) -> Option<f64>;

/// A collection of functions to lay a box child out with the given set of [`BoxConstraints`].
///
/// All of the functions adhere to the [`ChildLayouter`] or [`ChildBaselineGetter`] signature.
pub struct ChildLayoutHelper;

impl ChildLayoutHelper {
    /// Returns the [`Size`] that the box would have if it were to be laid out with the given
    /// [`BoxConstraints`].
    ///
    /// This method calls [`AnyRenderBox::get_dry_layout`] on the given box.
    ///
    /// This method should only be called by the parent of the provided box child as it binds
    /// parent and child together (if the child is marked as dirty, the child will also be marked
    /// as dirty).
    pub fn dry_layout_child(
        app: &mut App,
        child: AnyRenderBox,
        constraints: BoxConstraints,
    ) -> Size {
        child.get_dry_layout(app, constraints)
    }

    /// Lays out the box with the given constraints and returns its [`Size`].
    ///
    /// This method calls [`AnyRenderBox::layout`] on the given box with `parent_uses_size` set
    /// to true to receive its [`Size`].
    pub fn layout_child(app: &mut App, child: AnyRenderBox, constraints: BoxConstraints) -> Size {
        child.layout(app, constraints, true);
        child.size(app)
    }

    /// Convenience function that calls [`AnyRenderBox::get_dry_baseline`].
    pub fn get_dry_baseline(
        app: &mut App,
        child: AnyRenderBox,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        child.get_dry_baseline(app, constraints, baseline)
    }

    /// Convenience function that calls [`AnyRenderBox::get_distance_to_baseline`].
    ///
    /// The given `child` must be already laid out with `constraints`.
    pub fn get_baseline(
        app: &mut App,
        child: AnyRenderBox,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        debug_assert!(!child.as_object().debug_needs_layout(app));
        debug_assert!(child.constraints(app) == constraints);
        child.get_distance_to_baseline(app, baseline, true)
    }
}
