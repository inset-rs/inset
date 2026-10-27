//! WinUI ToolTipPositioning's in-window placement and ToolTipService safe zone.

use super::PlacementMode;
use inset_embedder::{Offset, Rect, Size};

/// `QueryRelativePosition`, with `PS_NONE` and the OS menu-alignment fallback.
pub(super) fn relative_position(
    bounds: Rect,
    size: Size,
    target: Rect,
    preferred: PlacementMode,
) -> Rect {
    use PlacementMode::*;
    let width = size.width().min(bounds.width());
    let height = size.height().min(bounds.height());
    let preferred = if preferred == Mouse { Top } else { preferred };
    let sides = match preferred {
        Top => [Top, Bottom, Right, Left],
        Bottom => [Bottom, Top, Right, Left],
        Left => [Left, Right, Top, Bottom],
        Right => [Right, Left, Top, Bottom],
        Mouse => unreachable!(),
    };
    for side in sides {
        let fits = match side {
            Top => target.top - bounds.top >= height,
            Bottom => bounds.bottom - target.bottom >= height,
            Left => target.left - bounds.left >= width,
            Right => bounds.right - target.right >= width,
            Mouse => unreachable!(),
        };
        if fits {
            let (left, top) = match side {
                Top => (target.center().dx() - width / 2.0, target.top - height),
                Bottom => (target.center().dx() - width / 2.0, target.bottom),
                Left => (target.left - width, target.center().dy() - height / 2.0),
                Right => (target.right, target.center().dy() - height / 2.0),
                Mouse => unreachable!(),
            };
            return constrain(bounds, Rect::from_ltwh(left, top, width, height));
        }
    }
    let (left, top) = match preferred {
        Top => (target.center().dx() - width / 2.0, bounds.top),
        Bottom => (target.center().dx() - width / 2.0, bounds.bottom - height),
        Left => (bounds.left, target.center().dy() - height / 2.0),
        Right => (bounds.right - width, target.center().dy() - height / 2.0),
        Mouse => unreachable!(),
    };
    constrain(bounds, Rect::from_ltwh(left, top, width, height))
}

/// Moves a fitting rectangle inside the window without changing its size.
fn constrain(bounds: Rect, rect: Rect) -> Rect {
    Rect::from_ltwh(
        rect.left.clamp(bounds.left, bounds.right - rect.width()),
        rect.top.clamp(bounds.top, bounds.bottom - rect.height()),
        rect.width(),
        rect.height(),
    )
}

/// `PerformMousePlacementWithPopup`: cursor position plus its 11-pixel offset and 2-pixel edge tolerance.
pub(super) fn mouse_position(
    bounds: Rect,
    size: Size,
    point: Offset,
    offset: Offset,
    rtl: bool,
) -> Rect {
    let width = size.width().min(bounds.width());
    let height = size.height().min(bounds.height());
    let x = if rtl {
        bounds.right - point.dx()
    } else {
        point.dx()
    };
    let mut left = (x + offset.dx()).max(bounds.left + 2.0);
    let mut top = (point.dy() + 11.0 + offset.dy()).max(bounds.top + 2.0);
    if left + width > bounds.right {
        left = (bounds.right - width - 2.0).max(bounds.left);
    }
    if top + height > bounds.bottom {
        top = (bounds.bottom - height - 2.0).max(bounds.top);
    }
    if rtl {
        left = bounds.right - left - width;
    }
    constrain(bounds, Rect::from_ltwh(left, top, width, height))
}

/// Tests the convex hull of the owner and popup rectangles, as `IsToolTipInSafeZone` does.
pub(super) fn safe_zone(point: Offset, owner: Rect, popup: Rect) -> bool {
    let corners = |r: Rect| {
        [
            Offset::new(r.left, r.top),
            Offset::new(r.right, r.top),
            Offset::new(r.right, r.bottom),
            Offset::new(r.left, r.bottom),
        ]
    };
    let mut points = corners(owner)
        .into_iter()
        .chain(corners(popup))
        .collect::<Vec<_>>();
    points.sort_by(|a, b| a.dx().total_cmp(&b.dx()).then(a.dy().total_cmp(&b.dy())));
    points.dedup();
    let cross = |a: Offset, b: Offset, c: Offset| {
        (b.dx() - a.dx()) * (c.dy() - a.dy()) - (b.dy() - a.dy()) * (c.dx() - a.dx())
    };
    let mut hull = Vec::new();
    for &p in &points {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
            hull.pop();
        }
        hull.push(p);
    }
    let lower = hull.len();
    for &p in points.iter().rev().skip(1) {
        while hull.len() > lower && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
            hull.pop();
        }
        hull.push(p);
    }
    hull.windows(2)
        .all(|edge| cross(edge[0], edge[1], point) >= 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flips_then_uses_axial_space_and_finally_clamps_oversized_content() {
        let bounds = Rect::from_ltwh(0.0, 0.0, 300.0, 200.0);
        let below = relative_position(
            bounds,
            Size::new(100.0, 40.0),
            Rect::from_ltwh(120.0, 0.0, 20.0, 20.0),
            PlacementMode::Top,
        );
        assert_eq!(below, Rect::from_ltwh(80.0, 20.0, 100.0, 40.0));
        let right = relative_position(
            bounds,
            Size::new(70.0, 160.0),
            Rect::from_ltwh(70.0, 80.0, 20.0, 20.0),
            PlacementMode::Top,
        );
        assert_eq!(right, Rect::from_ltwh(90.0, 10.0, 70.0, 160.0));
        assert_eq!(
            relative_position(bounds, Size::new(500.0, 300.0), bounds, PlacementMode::Top),
            bounds
        );
    }

    #[test]
    fn safe_zone_includes_diagonal_bridge_but_not_its_bounding_box_corners() {
        let owner = Rect::from_ltwh(0.0, 0.0, 20.0, 20.0);
        let popup = Rect::from_ltwh(80.0, 80.0, 20.0, 20.0);
        assert!(safe_zone(Offset::new(50.0, 50.0), owner, popup));
        assert!(!safe_zone(Offset::new(95.0, 5.0), owner, popup));
    }
}
