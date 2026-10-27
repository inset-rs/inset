//! FlyoutBase_partial.cpp: placement, ordered fallbacks and ResizeToFit.

use inset_embedder::{Offset, Rect, Size, TextDirection};

/// Preferred side and edge alignment relative to the opening target.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlyoutPlacementMode {
    /// Centered above the target; the source default.
    #[default]
    Top,
    /// Centered below the target.
    Bottom,
    /// Centered on the target's left side in left-to-right layout.
    Left,
    /// Centered on the target's right side in left-to-right layout.
    Right,
    /// Centered in the available window area, filling up to the maximum size.
    Full,
    /// Above with left edges aligned.
    TopEdgeAlignedLeft,
    /// Above with right edges aligned.
    TopEdgeAlignedRight,
    /// Below with left edges aligned.
    BottomEdgeAlignedLeft,
    /// Below with right edges aligned.
    BottomEdgeAlignedRight,
    /// Left with top edges aligned.
    LeftEdgeAlignedTop,
    /// Left with bottom edges aligned.
    LeftEdgeAlignedBottom,
    /// Right with top edges aligned.
    RightEdgeAlignedTop,
    /// Right with bottom edges aligned.
    RightEdgeAlignedBottom,
}

/// Primary direction after source flow-direction adjustment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MajorPlacementMode {
    Top,
    Bottom,
    Left,
    Right,
    Full,
}

impl MajorPlacementMode {
    /// Whether the primary axis is vertical.
    fn vertical(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }

    /// Source fallback order, retaining the requested axis first.
    fn fallbacks(self) -> [Self; 4] {
        use MajorPlacementMode::*;
        match self {
            Top => [Top, Bottom, Left, Right],
            Bottom => [Bottom, Top, Left, Right],
            Left => [Left, Right, Top, Bottom],
            Right => [Right, Left, Top, Bottom],
            Full => unreachable!("full placement does not use fallback sides"),
        }
    }
}

/// PreferredJustification combines Top/Left and Bottom/Right, which share source branches.
#[derive(Clone, Copy, Debug)]
enum PreferredJustification {
    Center,
    Start,
    End,
}

impl FlyoutPlacementMode {
    /// GetMajorPlacementFromPlacement and GetEffectivePlacementMode.
    fn major(self, direction: TextDirection) -> MajorPlacementMode {
        use FlyoutPlacementMode::*;
        let side = match self {
            Top | TopEdgeAlignedLeft | TopEdgeAlignedRight => MajorPlacementMode::Top,
            Bottom | BottomEdgeAlignedLeft | BottomEdgeAlignedRight => MajorPlacementMode::Bottom,
            Left | LeftEdgeAlignedTop | LeftEdgeAlignedBottom => MajorPlacementMode::Left,
            Right | RightEdgeAlignedTop | RightEdgeAlignedBottom => MajorPlacementMode::Right,
            Full => MajorPlacementMode::Full,
        };
        if direction == TextDirection::Rtl {
            match side {
                MajorPlacementMode::Left => MajorPlacementMode::Right,
                MajorPlacementMode::Right => MajorPlacementMode::Left,
                _ => side,
            }
        } else {
            side
        }
    }

    /// GetJustificationFromPlacementMode; flow direction only changes the major side.
    fn justification(self) -> PreferredJustification {
        use FlyoutPlacementMode::*;
        match self {
            TopEdgeAlignedLeft
            | BottomEdgeAlignedLeft
            | LeftEdgeAlignedTop
            | RightEdgeAlignedTop => PreferredJustification::Start,
            TopEdgeAlignedRight
            | BottomEdgeAlignedRight
            | LeftEdgeAlignedBottom
            | RightEdgeAlignedBottom => PreferredJustification::End,
            _ => PreferredJustification::Center,
        }
    }
}

/// TestAndCenterAlignWithinLimits, including its strict available-size comparison.
fn align_within_limits(
    anchor: f64,
    anchor_size: f64,
    extent: f64,
    low: f64,
    high: f64,
    justification: PreferredJustification,
) -> (bool, f64) {
    if high - low > extent && anchor_size >= 0.0 && extent >= 0.0 {
        let position = match justification {
            PreferredJustification::Center => anchor + 0.5 * (anchor_size - extent),
            PreferredJustification::Start => anchor,
            PreferredJustification::End => anchor + anchor_size - extent,
        };
        (true, position.clamp(low, high - extent))
    } else {
        (false, low)
    }
}

/// TestAgainstLimitsAndPlace; computes a clamped position even on failure.
fn place_against_limits(
    anchor: f64,
    anchor_size: f64,
    increasing: bool,
    extent: f64,
    low: f64,
    high: f64,
) -> (bool, f64) {
    let (extra, mut position) = if increasing {
        (high - anchor - anchor_size - extent, anchor + anchor_size)
    } else {
        (anchor - low - extent, anchor - extent)
    };
    if position < low {
        position = low;
    } else if position + extent > high {
        position = high - extent;
    }
    (extra >= 0.0, position)
}

/// TryPlacement computes one side without changing the measured size.
fn try_placement(
    target: Rect,
    control: Size,
    container: Rect,
    side: MajorPlacementMode,
    justification: PreferredJustification,
) -> (bool, Offset) {
    let (fits_primary, primary, fits_secondary, secondary) = if side.vertical() {
        let (p, y) = place_against_limits(
            target.top,
            target.height(),
            side == MajorPlacementMode::Bottom,
            control.height(),
            container.top,
            container.bottom,
        );
        let (s, x) = align_within_limits(
            target.left,
            target.width(),
            control.width(),
            container.left,
            container.right,
            justification,
        );
        (p, y, s, x)
    } else {
        let (p, x) = place_against_limits(
            target.left,
            target.width(),
            side == MajorPlacementMode::Right,
            control.width(),
            container.left,
            container.right,
        );
        let (s, y) = align_within_limits(
            target.top,
            target.height(),
            control.height(),
            container.top,
            container.bottom,
            justification,
        );
        (p, x, s, y)
    };
    (
        fits_primary && fits_secondary,
        if side.vertical() {
            Offset::new(secondary, primary)
        } else {
            Offset::new(primary, secondary)
        },
    )
}

/// CalculateAvailableSpace, clamped to the container's extent on the requested axis.
fn available_space(side: MajorPlacementMode, target: Rect, container: Rect) -> f64 {
    match side {
        MajorPlacementMode::Top => (target.top - container.top).min(container.height()),
        MajorPlacementMode::Bottom => (container.bottom - target.bottom).min(container.height()),
        MajorPlacementMode::Left => (target.left - container.left).min(container.width()),
        MajorPlacementMode::Right => (container.right - target.right).min(container.width()),
        MajorPlacementMode::Full => unreachable!(),
    }
    .max(0.0)
}

/// ResizeToFit keeps the source side unless its opposite can accommodate the minimum.
fn resize_to_fit(
    side: &mut MajorPlacementMode,
    target: Rect,
    container: Rect,
    minimum: Size,
    position: &mut Offset,
    size: &mut Size,
) {
    let vertical = side.vertical();
    let mut available = available_space(*side, target, container);
    let (mut width, mut height) = (size.width(), size.height());
    let (mut x, mut y) = (position.dx(), position.dy());
    let extent = if vertical { height } else { width };
    if extent > available {
        let alternatives = side.fallbacks();
        let opposite = alternatives[1];
        let other_available = available_space(opposite, target, container);
        let minimum = if vertical {
            minimum.height()
        } else {
            minimum.width()
        };
        if other_available > available && other_available >= minimum {
            *side = opposite;
            available = other_available;
        }
        let extent = if extent > available && available >= minimum {
            available
        } else {
            extent
        };
        if vertical {
            height = extent.min(container.height());
            y = if *side == MajorPlacementMode::Top {
                target.top - height
            } else {
                target.bottom
            };
        } else {
            width = extent.min(container.width());
            x = if *side == MajorPlacementMode::Left {
                target.left - width
            } else {
                target.right
            };
        }
    }
    if vertical && width > container.width() {
        width = container.width();
        x = container.left;
    }
    if !vertical && height > container.height() {
        height = container.height();
        y = container.top;
    }
    if x < container.left {
        x = container.left;
    } else if x + width > container.right {
        x = container.right - width;
    }
    if y < container.top {
        y = container.top;
    } else if y + height > container.bottom {
        y = container.bottom - height;
    }
    *position = Offset::new(x, y);
    *size = Size::new(width.max(0.0), height.max(0.0));
}

/// CalculatePlacementPrivate for a popup constrained to the application window.
#[expect(
    clippy::too_many_arguments,
    reason = "keeps the source placement inputs explicit"
)]
pub(super) fn calculate_placement(
    placement: FlyoutPlacementMode,
    direction: TextDirection,
    target: Rect,
    measured: Size,
    minimum: Size,
    maximum: Size,
    container: Rect,
    allow_fallbacks: bool,
) -> Rect {
    let mut side = placement.major(direction);
    if side == MajorPlacementMode::Full {
        let width = container.width().min(maximum.width());
        let height = container.height().min(maximum.height());
        return Rect::from_ltwh(
            container.left + (container.width() - width) / 2.0,
            container.top + (container.height() - height) / 2.0,
            width,
            height,
        );
    }
    let order = side.fallbacks();
    let justification = placement.justification();
    let (mut fits, mut position) = try_placement(target, measured, container, side, justification);
    if !fits && allow_fallbacks {
        for candidate in order.into_iter().skip(1) {
            let (candidate_fits, candidate_position) =
                try_placement(target, measured, container, candidate, justification);
            if candidate_fits {
                fits = true;
                side = candidate;
                position = candidate_position;
                break;
            }
        }
    }
    let mut size = measured;
    if !fits {
        resize_to_fit(
            &mut side,
            target,
            container,
            minimum,
            &mut position,
            &mut size,
        );
    }
    // FlyoutMargin is applied after fitting, only along the chosen direction.
    position = position
        + match side {
            MajorPlacementMode::Top => Offset::new(0.0, -4.0),
            MajorPlacementMode::Bottom => Offset::new(0.0, 4.0),
            MajorPlacementMode::Left => Offset::new(-4.0, 0.0),
            MajorPlacementMode::Right => Offset::new(4.0, 0.0),
            MajorPlacementMode::Full => unreachable!(),
        };
    Rect::from_ltwh(
        position.dx().max(0.0),
        position.dy().max(0.0),
        size.width(),
        size.height(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(
        mode: FlyoutPlacementMode,
        direction: TextDirection,
        target: Rect,
        size: Size,
    ) -> Rect {
        calculate_placement(
            mode,
            direction,
            target,
            size,
            Size::new(96.0, 40.0),
            Size::new(456.0, 758.0),
            Rect::from_ltwh(0.0, 0.0, 500.0, 400.0),
            true,
        )
    }

    #[test]
    fn top_falls_back_to_bottom_before_horizontal_sides() {
        assert_eq!(
            place(
                FlyoutPlacementMode::Top,
                TextDirection::Ltr,
                Rect::from_ltwh(200.0, 10.0, 100.0, 30.0),
                Size::new(120.0, 80.0)
            ),
            Rect::from_ltwh(190.0, 44.0, 120.0, 80.0),
        );
    }

    #[test]
    fn rtl_changes_left_side_to_right_without_changing_vertical_alignment() {
        assert_eq!(
            place(
                FlyoutPlacementMode::LeftEdgeAlignedBottom,
                TextDirection::Rtl,
                Rect::from_ltwh(200.0, 200.0, 100.0, 30.0),
                Size::new(120.0, 80.0)
            ),
            Rect::from_ltwh(304.0, 150.0, 120.0, 80.0),
        );
    }

    #[test]
    fn full_placement_uses_container_and_presenter_maximum() {
        assert_eq!(
            place(
                FlyoutPlacementMode::Full,
                TextDirection::Ltr,
                Rect::ZERO,
                Size::new(120.0, 80.0)
            ),
            Rect::from_ltwh(22.0, 0.0, 456.0, 400.0),
        );
    }

    #[test]
    fn equal_cross_axis_space_does_not_count_as_fitting() {
        assert_eq!(
            align_within_limits(
                25.0,
                10.0,
                100.0,
                0.0,
                100.0,
                PreferredJustification::Center
            ),
            (false, 0.0)
        );
    }

    #[test]
    fn no_side_fits_so_resize_uses_more_space_on_the_original_axis() {
        let result = place(
            FlyoutPlacementMode::Top,
            TextDirection::Ltr,
            Rect::from_ltwh(200.0, 60.0, 100.0, 30.0),
            Size::new(450.0, 350.0),
        );
        assert_eq!(result, Rect::from_ltwh(25.0, 94.0, 450.0, 310.0));
    }

    #[test]
    fn menu_points_exclude_the_opener_in_both_directions() {
        let available = Rect::from_ltwh(0.0, 0.0, 600.0, 400.0);
        let size = Size::new(200.0, 80.0);
        for direction in [TextDirection::Ltr, TextDirection::Rtl] {
            let x = if direction == TextDirection::Ltr {
                400.0
            } else {
                460.0
            };
            for (top, kind, expected_top) in [
                (180.0, PointPlacementKind::Menu, 212.0),
                (180.0, PointPlacementKind::TouchMenu, 100.0),
                (4.0, PointPlacementKind::TouchMenu, 36.0),
            ] {
                let result = calculate_point_placement(
                    FlyoutPlacementMode::Bottom,
                    direction,
                    Offset::new(x, top + 32.0),
                    size,
                    Rect::from_ltwh(400.0, top, 60.0, 32.0),
                    available,
                    kind,
                );
                assert_eq!(result.top, expected_top);
                assert_eq!(
                    result.left,
                    if direction == TextDirection::Ltr {
                        x
                    } else {
                        x - 200.0
                    }
                );
            }
        }
    }
}

/// Point placement differs for ordinary flyouts and menus opened by touch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum PointPlacementKind {
    /// Ordinary flyouts align their chosen side around the point.
    #[default]
    Flyout,
    /// Menus start at the point without the ordinary flyout alignment offset.
    Menu,
    /// Touch menus prefer above the point, then fall back below when necessary.
    TouchMenu,
}

/// UpdateTargetPosition for a point-positioned Flyout within the application window.
pub(super) fn calculate_point_placement(
    placement: FlyoutPlacementMode,
    direction: TextDirection,
    point: Offset,
    size: Size,
    exclusion: Rect,
    available: Rect,
    kind: PointPlacementKind,
) -> Rect {
    use FlyoutPlacementMode::*;
    let width = size.width();
    let height = size.height();
    let mut x = point.dx().clamp(available.left, available.right);
    let mut y = point.dy().clamp(available.top, available.bottom);
    if kind == PointPlacementKind::Flyout {
        match placement {
            Top => {
                x -= width / 2.0;
                y -= height;
            }
            Bottom => x -= width / 2.0,
            Left => {
                x -= width;
                y -= height / 2.0;
            }
            Right => y -= height / 2.0,
            TopEdgeAlignedLeft | RightEdgeAlignedBottom => y -= height,
            TopEdgeAlignedRight | LeftEdgeAlignedBottom => {
                x -= width;
                y -= height;
            }
            BottomEdgeAlignedLeft | RightEdgeAlignedTop | Full => {}
            BottomEdgeAlignedRight | LeftEdgeAlignedTop => x -= width,
        }
    }
    let prefer_top = kind == PointPlacementKind::TouchMenu;
    if prefer_top {
        y -= height;
    }
    let rtl = direction == TextDirection::Rtl;
    let mut side = if prefer_top {
        MajorPlacementMode::Top
    } else {
        placement.major(direction)
    };
    let original_side = side;
    let account_for_exclusion = |side, x: &mut f64, y: &mut f64| {
        // RectUtil::AreDisjoint counts touching edges as an intersection.
        let disjoint = exclusion.right < *x
            || exclusion.left > *x + width
            || exclusion.bottom < *y
            || exclusion.top > *y + height;
        if !exclusion.is_empty() && !disjoint {
            match side {
                MajorPlacementMode::Top => *y = exclusion.top - height,
                MajorPlacementMode::Bottom => *y = exclusion.bottom,
                MajorPlacementMode::Left => *x = exclusion.left - width,
                MajorPlacementMode::Right => *x = exclusion.right,
                MajorPlacementMode::Full => {}
            }
            if rtl && matches!(side, MajorPlacementMode::Left | MajorPlacementMode::Right) {
                *x += width;
            }
        }
    };
    account_for_exclusion(side, &mut x, &mut y);
    let mut target = Offset::new(x, y);
    if !rtl && target.dx() + width > available.right {
        x -= width.min(x);
        if side == MajorPlacementMode::Right {
            side = MajorPlacementMode::Left;
        }
    } else if rtl && target.dx() - available.left < width {
        x += width.min(available.right - x);
        if side == MajorPlacementMode::Left {
            side = MajorPlacementMode::Right;
        }
    }
    if prefer_top && target.dy() < available.top {
        y += height;
        target = target + Offset::new(0.0, height);
        if side == MajorPlacementMode::Top {
            side = MajorPlacementMode::Bottom;
        }
    }
    if target.dy() + height > available.bottom {
        let mut bottom_alignment_y = y;
        if matches!(side, MajorPlacementMode::Left | MajorPlacementMode::Right)
            && !exclusion.is_empty()
            && (y - exclusion.top).abs() <= 1.0
        {
            bottom_alignment_y = exclusion.bottom;
        }
        y = bottom_alignment_y - height.min(bottom_alignment_y);
        if side == MajorPlacementMode::Bottom {
            side = MajorPlacementMode::Top;
        }
    }
    y = y.max(available.top);
    account_for_exclusion(side, &mut x, &mut y);
    let offscreen_horizontally = if rtl {
        x - width < available.left || x > available.right
    } else {
        x < available.left || x + width > available.right
    };
    let offscreen_vertically = y < available.top || y + height > available.bottom;
    if (width <= available.width() && offscreen_horizontally)
        || (height <= available.height() && offscreen_vertically)
    {
        account_for_exclusion(original_side, &mut x, &mut y);
    }
    if width <= available.width() {
        x = if rtl {
            x.clamp(available.left + width, available.right)
        } else {
            x.clamp(available.left, available.right - width)
        };
    }
    if height <= available.height() {
        y = y.clamp(available.top, available.bottom - height);
    }
    Rect::from_ltwh(if rtl { x - width } else { x }, y, width, height)
}

/// CascadingMenuHelper::GetPositionAndDirection, including its four-pixel overlap.
pub(super) fn calculate_submenu_placement(
    target: Rect,
    size: Size,
    direction: TextDirection,
    available: Rect,
) -> Rect {
    let width = size.width();
    let height = size.height();
    let x = if direction == TextDirection::Ltr {
        let right_space = available.right - target.right;
        if width > right_space {
            if width < target.left - available.left {
                target.left - width + 4.0
            } else {
                available.right - width
            }
        } else {
            target.right - 4.0
        }
    } else {
        let left_space = target.left - available.left;
        if width > left_space {
            if width < available.right - target.right {
                target.right - 4.0
            } else {
                available.left
            }
        } else {
            target.left - width + 4.0
        }
    };
    let bottom_space = available.bottom - target.top;
    let y = if height > bottom_space {
        let top_space = target.bottom - available.top;
        if top_space >= height {
            target.bottom - height
        } else {
            available.top
        }
    } else {
        target.top
    };
    Rect::from_ltwh(x, y, width, height)
}
