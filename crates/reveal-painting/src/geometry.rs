//! Flutter counterpart: `painting/geometry.dart`.

use reveal_geometry::{clamp_double, Offset, Size};

/// Position a child box within a container box, either above or below a target
/// point.
///
/// The container's size is described by `size`.
///
/// The target point is specified by `target`, as an offset from the top left of
/// the container.
///
/// The child box's size is given by `child_size`.
///
/// The return value is the suggested distance from the top left of the
/// container box to the top left of the child box.
///
/// The suggested position will be above the target point if `prefer_below` is
/// false, and below the target point if it is true, unless it wouldn't fit on
/// the preferred side but would fit on the other side.
///
/// The suggested position will place the nearest side of the child to the
/// target point `vertical_offset` from the target point (even if it cannot fit
/// given that constraint).
///
/// The suggested position will be at least `margin` away from the edge of the
/// container. If possible, the child will be positioned so that its center is
/// aligned with the target point. If the child cannot fit horizontally within
/// the container given the margin, then the child will be centered in the
/// container.
///
/// Used by `Tooltip` to position a tooltip relative to its parent.
pub fn position_dependent_box(
    size: Size,
    child_size: Size,
    target: Offset,
    prefer_below: bool,
    vertical_offset: f64,
    margin: f64,
) -> Offset {
    // VERTICAL DIRECTION
    let fits_below = target.dy() + vertical_offset + child_size.height() <= size.height() - margin;
    let fits_above = target.dy() - vertical_offset - child_size.height() >= margin;
    let tooltip_below = if fits_above == fits_below {
        prefer_below
    } else {
        fits_below
    };
    let y = if tooltip_below {
        (target.dy() + vertical_offset).min(size.height() - margin)
    } else {
        (target.dy() - vertical_offset - child_size.height()).max(margin)
    };
    // HORIZONTAL DIRECTION
    let flexible_space = size.width() - child_size.width();
    let x = if flexible_space <= 2.0 * margin {
        // If there's not enough horizontal space for margin + child, center the
        // child.
        flexible_space / 2.0
    } else {
        clamp_double(
            target.dx() - child_size.width() / 2.0,
            margin,
            flexible_space - margin,
        )
    };
    Offset::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    // geometry_test.dart 'positionDependentBox'.
    #[test]
    fn position_dependent_box_matches_dart() {
        assert_eq!(
            position_dependent_box(
                Size::new(100.0, 100.0),
                Size::new(20.0, 10.0),
                Offset::new(50.0, 50.0),
                false,
                0.0,
                0.0,
            ),
            Offset::new(40.0, 40.0),
        );
        assert_eq!(
            position_dependent_box(
                Size::new(100.0, 100.0),
                Size::new(200.0, 10.0),
                Offset::new(50.0, 50.0),
                false,
                0.0,
                0.0,
            ),
            Offset::new(-50.0, 40.0),
        );
        assert_eq!(
            position_dependent_box(
                Size::new(100.0, 100.0),
                Size::new(200.0, 10.0),
                Offset::new(0.0, 50.0),
                false,
                0.0,
                0.0,
            ),
            Offset::new(-50.0, 40.0),
        );
        assert_eq!(
            position_dependent_box(
                Size::new(100.0, 100.0),
                Size::new(200.0, 10.0),
                Offset::new(100.0, 50.0),
                false,
                0.0,
                0.0,
            ),
            Offset::new(-50.0, 40.0),
        );
        assert_eq!(
            position_dependent_box(
                Size::new(100.0, 100.0),
                Size::new(50.0, 10.0),
                Offset::new(50.0, 50.0),
                false,
                0.0,
                20.0,
            ),
            Offset::new(25.0, 40.0),
        );
        assert_eq!(
            position_dependent_box(
                Size::new(100.0, 100.0),
                Size::new(50.0, 10.0),
                Offset::new(50.0, 50.0),
                false,
                0.0,
                30.0,
            ),
            Offset::new(25.0, 40.0),
        );
        assert_eq!(
            position_dependent_box(
                Size::new(100.0, 100.0),
                Size::new(50.0, 10.0),
                Offset::new(0.0, 50.0),
                false,
                0.0,
                20.0,
            ),
            Offset::new(20.0, 40.0),
        );
        assert_eq!(
            position_dependent_box(
                Size::new(100.0, 100.0),
                Size::new(50.0, 10.0),
                Offset::new(0.0, 50.0),
                false,
                0.0,
                30.0,
            ),
            Offset::new(25.0, 40.0),
        );
        assert_eq!(
            position_dependent_box(
                Size::new(100.0, 100.0),
                Size::new(50.0, 10.0),
                Offset::new(100.0, 50.0),
                false,
                0.0,
                20.0,
            ),
            Offset::new(30.0, 40.0),
        );
        assert_eq!(
            position_dependent_box(
                Size::new(100.0, 100.0),
                Size::new(50.0, 10.0),
                Offset::new(100.0, 50.0),
                false,
                0.0,
                30.0,
            ),
            Offset::new(25.0, 40.0),
        );
    }
}
