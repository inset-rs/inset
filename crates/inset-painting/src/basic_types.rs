//! Flutter counterpart: `painting/basic_types.dart`.

pub use inset_embedder::{
    BlendMode, BlurStyle, Canvas, ClipOp, FillRule, Image, MaskBlur, Paint, PaintStyle, Paragraph,
    ParagraphBuilder, ParagraphStyle, Path, PathBuilder, Picture, Stroke,
};
pub use inset_embedder::{
    Clip, Color, ColorSpace, FontFeature, FontStyle, FontVariation, FontWeight, Matrix4, Shadow,
    TextAlign, TextBaseline, TextDecoration, TextDecorationStyle, TextDirection,
    TextHeightBehavior, TextLeadingDistribution,
};

/// The description of the difference between two objects, in the context of how
/// it will affect the rendering.
///
/// Used by `TextSpan.compareTo` and `TextStyle.compareTo`.
///
/// The values in this enum are ordered such that they are in increasing order
/// of cost. A value with index N implies all the values with index less than N.
/// For example, [`RenderComparison::Layout`] (index 3) implies
/// [`RenderComparison::Paint`] (2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderComparison {
    /// The two objects are identical (meaning deeply equal, not necessarily
    /// pointer-identical).
    Identical,

    /// The two objects are identical for the purpose of layout, but may be
    /// different in other ways.
    ///
    /// For example, maybe some event handlers changed.
    Metadata,

    /// The two objects are different but only in ways that affect paint, not
    /// layout.
    ///
    /// For example, only the color is changed.
    ///
    /// `RenderObject.markNeedsPaint` would be necessary to handle this kind of
    /// change in a render object.
    Paint,

    /// The two objects are different in ways that affect layout (and therefore
    /// paint).
    ///
    /// For example, the size is changed.
    ///
    /// This is the most drastic level of change possible.
    ///
    /// `RenderObject.markNeedsLayout` would be necessary to handle this kind of
    /// change in a render object.
    Layout,
}

/// The two cardinal directions in two dimensions.
///
/// The axis is always relative to the current coordinate space. This means, for
/// example, that a [`Axis::Horizontal`] axis might actually be diagonally from
/// top right to bottom left, due to some local `Transform` applied to the
/// scene.
///
/// See also:
///
///  * [`AxisDirection`], which is a directional version of this enum (with
///    values like left and right, rather than just horizontal).
///  * [`TextDirection`], which disambiguates between left-to-right horizontal
///    content and right-to-left horizontal content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Left and right.
    ///
    /// See also:
    ///
    ///  * [`TextDirection`], which disambiguates between left-to-right
    ///    horizontal content and right-to-left horizontal content.
    Horizontal,

    /// Up and down.
    Vertical,
}

/// Returns the opposite of the given [`Axis`].
///
/// Specifically, returns [`Axis::Horizontal`] for [`Axis::Vertical`], and vice
/// versa.
///
/// See also:
///
///  * [`flip_axis_direction`], which does the same thing for [`AxisDirection`]
///    values.
pub fn flip_axis(direction: Axis) -> Axis {
    match direction {
        Axis::Horizontal => Axis::Vertical,
        Axis::Vertical => Axis::Horizontal,
    }
}

/// A direction in which boxes flow vertically.
///
/// This is used by the flex algorithm (e.g. `Column`) to decide in which
/// direction to draw boxes.
///
/// This is also used to disambiguate `start` and `end` values (e.g.
/// `MainAxisAlignment.start` or `CrossAxisAlignment.end`).
///
/// See also:
///
///  * [`TextDirection`], which controls the same thing but horizontally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalDirection {
    /// Boxes should start at the bottom and be stacked vertically towards the
    /// top.
    ///
    /// The "start" is at the bottom, the "end" is at the top.
    Up,

    /// Boxes should start at the top and be stacked vertically towards the
    /// bottom.
    ///
    /// The "start" is at the top, the "end" is at the bottom.
    Down,
}

/// A direction along either the horizontal or vertical [`Axis`] in which the
/// origin, or zero position, is determined.
///
/// This value relates to the direction in which the scroll offset increases
/// from the origin. This value does not represent the direction of user input
/// that may be modifying the scroll offset, such as from a drag. For the
/// active scrolling direction, see `ScrollDirection`.
///
/// See also:
///
///   * `ScrollDirection`, the direction of active scrolling, relative to the
///     positive scroll offset axis given by an [`AxisDirection`] and a
///     `GrowthDirection`.
///   * `GrowthDirection`, the direction in which slivers and their content are
///     ordered, relative to the scroll offset axis as specified by
///     [`AxisDirection`].
///   * `CustomScrollView.anchor`, the relative position of the zero scroll
///     offset in a viewport and inflection point for [`AxisDirection`]s of the
///     same cardinal [`Axis`].
///   * [`axis_direction_is_reversed`], which returns whether traveling along the
///     given axis direction visits coordinates along that axis in numerically
///     decreasing order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisDirection {
    /// A direction in the [`Axis::Vertical`] where zero is at the bottom and
    /// positive values are above it: `⇈`
    ///
    /// Alphabetical content with a `GrowthDirection.forward` would have the A
    /// at the bottom and the Z at the top.
    ///
    /// For example, the behavior of a `ListView` with `ListView.reverse` set to
    /// true would have this axis direction.
    ///
    /// See also:
    ///
    ///   * [`axis_direction_is_reversed`], which returns whether traveling along
    ///     the given axis direction visits coordinates along that axis in
    ///     numerically decreasing order.
    Up,

    /// A direction in the [`Axis::Horizontal`] where zero is on the left and
    /// positive values are to the right of it: `⇉`
    ///
    /// Alphabetical content with a `GrowthDirection.forward` would have the A
    /// on the left and the Z on the right. This is the ordinary reading order
    /// for a horizontal set of tabs in an English application, for example.
    ///
    /// For example, the behavior of a `ListView` with `ListView.scrollDirection`
    /// set to [`Axis::Horizontal`] would have this axis direction.
    ///
    /// See also:
    ///
    ///   * [`axis_direction_is_reversed`], which returns whether traveling along
    ///     the given axis direction visits coordinates along that axis in
    ///     numerically decreasing order.
    Right,

    /// A direction in the [`Axis::Vertical`] where zero is at the top and
    /// positive values are below it: `⇊`
    ///
    /// Alphabetical content with a `GrowthDirection.forward` would have the A
    /// at the top and the Z at the bottom. This is the ordinary reading order
    /// for a vertical list.
    ///
    /// For example, the default behavior of a `ListView` would have this axis
    /// direction.
    ///
    /// See also:
    ///
    ///   * [`axis_direction_is_reversed`], which returns whether traveling along
    ///     the given axis direction visits coordinates along that axis in
    ///     numerically decreasing order.
    Down,

    /// A direction in the [`Axis::Horizontal`] where zero is to the right and
    /// positive values are to the left of it: `⇇`
    ///
    /// Alphabetical content with a `GrowthDirection.forward` would have the A
    /// at the right and the Z at the left. This is the ordinary reading order
    /// for a horizontal set of tabs in a Hebrew application, for example.
    ///
    /// For example, the behavior of a `ListView` with `ListView.scrollDirection`
    /// set to [`Axis::Horizontal`] and `ListView.reverse` set to true would
    /// have this axis direction.
    ///
    /// See also:
    ///
    ///   * [`axis_direction_is_reversed`], which returns whether traveling along
    ///     the given axis direction visits coordinates along that axis in
    ///     numerically decreasing order.
    Left,
}

/// Returns the [`Axis`] that contains the given [`AxisDirection`].
///
/// Specifically, returns [`Axis::Vertical`] for [`AxisDirection::Up`] and
/// [`AxisDirection::Down`] and returns [`Axis::Horizontal`] for
/// [`AxisDirection::Left`] and [`AxisDirection::Right`].
pub fn axis_direction_to_axis(axis_direction: AxisDirection) -> Axis {
    match axis_direction {
        AxisDirection::Up | AxisDirection::Down => Axis::Vertical,
        AxisDirection::Left | AxisDirection::Right => Axis::Horizontal,
    }
}

/// Returns the [`AxisDirection`] in which reading occurs in the given
/// [`TextDirection`].
///
/// Specifically, returns [`AxisDirection::Left`] for [`TextDirection::Rtl`] and
/// [`AxisDirection::Right`] for [`TextDirection::Ltr`].
pub fn text_direction_to_axis_direction(text_direction: TextDirection) -> AxisDirection {
    match text_direction {
        TextDirection::Rtl => AxisDirection::Left,
        TextDirection::Ltr => AxisDirection::Right,
    }
}

/// Returns the opposite of the given [`AxisDirection`].
///
/// Specifically, returns [`AxisDirection::Up`] for [`AxisDirection::Down`] (and
/// vice versa), as well as [`AxisDirection::Left`] for [`AxisDirection::Right`]
/// (and vice versa).
///
/// See also:
///
///  * [`flip_axis`], which does the same thing for [`Axis`] values.
pub fn flip_axis_direction(axis_direction: AxisDirection) -> AxisDirection {
    match axis_direction {
        AxisDirection::Up => AxisDirection::Down,
        AxisDirection::Right => AxisDirection::Left,
        AxisDirection::Down => AxisDirection::Up,
        AxisDirection::Left => AxisDirection::Right,
    }
}

/// Returns whether traveling along the given axis direction visits coordinates
/// along that axis in numerically decreasing order.
///
/// Specifically, returns true for [`AxisDirection::Up`] and
/// [`AxisDirection::Left`] and false for [`AxisDirection::Down`] and
/// [`AxisDirection::Right`].
pub fn axis_direction_is_reversed(axis_direction: AxisDirection) -> bool {
    match axis_direction {
        AxisDirection::Up | AxisDirection::Left => true,
        AxisDirection::Down | AxisDirection::Right => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flip_axis_swaps_the_two_cardinals() {
        assert_eq!(flip_axis(Axis::Horizontal), Axis::Vertical);
        assert_eq!(flip_axis(Axis::Vertical), Axis::Horizontal);
    }

    #[test]
    fn axis_direction_helpers_match_dart() {
        assert_eq!(axis_direction_to_axis(AxisDirection::Up), Axis::Vertical);
        assert_eq!(axis_direction_to_axis(AxisDirection::Down), Axis::Vertical);
        assert_eq!(
            axis_direction_to_axis(AxisDirection::Left),
            Axis::Horizontal
        );
        assert_eq!(
            axis_direction_to_axis(AxisDirection::Right),
            Axis::Horizontal
        );

        assert_eq!(
            text_direction_to_axis_direction(TextDirection::Rtl),
            AxisDirection::Left
        );
        assert_eq!(
            text_direction_to_axis_direction(TextDirection::Ltr),
            AxisDirection::Right
        );

        assert_eq!(flip_axis_direction(AxisDirection::Up), AxisDirection::Down);
        assert_eq!(
            flip_axis_direction(AxisDirection::Right),
            AxisDirection::Left
        );
        assert_eq!(flip_axis_direction(AxisDirection::Down), AxisDirection::Up);
        assert_eq!(
            flip_axis_direction(AxisDirection::Left),
            AxisDirection::Right
        );

        assert!(axis_direction_is_reversed(AxisDirection::Up));
        assert!(axis_direction_is_reversed(AxisDirection::Left));
        assert!(!axis_direction_is_reversed(AxisDirection::Down));
        assert!(!axis_direction_is_reversed(AxisDirection::Right));
    }

    #[test]
    fn render_comparison_is_ordered_by_cost() {
        assert!(RenderComparison::Identical < RenderComparison::Metadata);
        assert!(RenderComparison::Metadata < RenderComparison::Paint);
        assert!(RenderComparison::Paint < RenderComparison::Layout);
    }

    #[test]
    fn text_direction_lists_rtl_first() {
        assert_eq!(TextDirection::Rtl as u8, 0);
        assert_eq!(TextDirection::Ltr as u8, 1);
    }
}
