//! Where the field's text lies, for UIKit's questions about it, as Flutter's `FlutterTextInputView`
//! answers them from the selection rects the framework sends (`firstRectForRange:`,
//! `caretRectForPosition:`, `selectionRectsForRange:`, `closestPositionToPoint:withinRange:`) and
//! from the editable's transform (`localRectFromFrameworkTransform:`). Every rect is in the field's
//! coordinates, as the framework sent it; positions are UTF-16 offsets.

use waft_embedder::{Offset, Rect};

/// One grapheme's bounds and where it starts: Flutter's `FlutterTextSelectionRect` as the
/// framework sends it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GlyphRect {
    pub(crate) position: usize,
    pub(crate) rect: Rect,
    pub(crate) rtl: bool,
}

/// One rect `selectionRectsForRange:` answers, with what UIKit asks of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RangeRect {
    pub(crate) rect: Rect,
    pub(crate) position: usize,
    pub(crate) contains_start: bool,
    pub(crate) contains_end: bool,
}

/// `localRectFromFrameworkTransform:`: the bounds of `rect` taken through `transform`, Flutter's
/// column-major matrix, with its perspective; `None` where a corner goes to infinity.
pub(crate) fn rect_through(transform: &[f32; 16], rect: Rect) -> Option<Rect> {
    let m = transform.map(f64::from);
    let corners = [
        (rect.left, rect.top),
        (rect.left, rect.bottom),
        (rect.right, rect.top),
        (rect.right, rect.bottom),
    ];
    let mut bounds = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for (x, y) in corners {
        let mut tx = m[0] * x + m[4] * y + m[12];
        let mut ty = m[1] * x + m[5] * y + m[13];
        let w = m[3] * x + m[7] * y + m[15];
        if w == 0.0 {
            return None;
        }
        if w != 1.0 {
            tx /= w;
            ty /= w;
        }
        bounds = (
            bounds.0.min(tx),
            bounds.1.min(ty),
            bounds.2.max(tx),
            bounds.3.max(ty),
        );
    }
    Some(Rect::from_ltrb(bounds.0, bounds.1, bounds.2, bounds.3))
}

/// `firstRectForRange:` from the selection rects, as iOS 17 and later have it: the rect of the
/// range's first line, from the rect of its first character to the last one on that line, `None`
/// where the rects do not cover it. `text_length` is the text's length in UTF-16 units.
pub(crate) fn first_rect(
    rects: &[GlyphRect],
    start: usize,
    end: usize,
    text_length: usize,
) -> Option<Rect> {
    let first = start.min(end);
    let mut start_rect: Option<Rect> = None;
    let mut end_rect: Option<Rect> = None;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for (i, glyph) in rects.iter().enumerate() {
        let starts_on_or_before_start = glyph.position <= first;
        let is_last = i + 1 == rects.len();
        let end_of_text_is_after_start = is_last && text_length > first;
        let next_is_after_start = !is_last && rects[i + 1].position > first;
        if starts_on_or_before_start && (end_of_text_is_after_start || next_is_after_start) {
            start_rect = Some(glyph.rect);
        }
        if start_rect.is_some() {
            min_y = min_y.min(glyph.rect.top);
            max_y = max_y.max(glyph.rect.bottom);
            // `end` is exclusive.
            let ends_on_or_after_end = glyph.position + 1 >= end;
            let next_is_on_next_line = !is_last && {
                let next = rects[i + 1].rect;
                (next.top + next.bottom) / 2.0 > glyph.rect.bottom
            };
            if ends_on_or_after_end || is_last || next_is_on_next_line {
                end_rect = Some(glyph.rect);
                break;
            }
        }
    }
    let (start_rect, end_rect) = (start_rect?, end_rect?);
    let min_x = start_rect.left.min(end_rect.left);
    let max_x = start_rect.right.max(end_rect.right);
    Some(Rect::from_ltrb(min_x, min_y, max_x, max_y))
}

/// `selectionRectsForRange:`: the rects of the graphemes that start in `start..end`, zero wide
/// where the range is empty, which then takes the grapheme at its end.
pub(crate) fn rects_in_range(
    rects: &[GlyphRect],
    start: usize,
    end: usize,
    text_length: usize,
) -> Vec<RangeRect> {
    rects
        .iter()
        .enumerate()
        .filter(|(_, glyph)| {
            glyph.position >= start
                && (glyph.position < end || (start == end && glyph.position <= end))
        })
        .map(|(i, glyph)| {
            let width = if start == end {
                0.0
            } else {
                glyph.rect.width()
            };
            RangeRect {
                rect: Rect::from_ltwh(glyph.rect.left, glyph.rect.top, width, glyph.rect.height()),
                position: glyph.position,
                contains_start: i == 0,
                contains_end: i == text_length,
            }
        })
        .collect()
}

/// `caretRectForPosition:`: a zero-wide rect at the edge of the grapheme after the caret, where
/// there is none before it or the caret leans forward, else at the edge of the one before it, from
/// `around`, the rects of the graphemes before and after `index`.
pub(crate) fn caret_rect(
    around: &[(Rect, bool)],
    index: usize,
    leans_forward: bool,
) -> Option<Rect> {
    let edge_of = |(rect, rtl): (Rect, bool), leading: bool| {
        let x = if rtl == leading {
            rect.right
        } else {
            rect.left
        };
        Rect::from_ltwh(x, rect.top, 0.0, rect.height())
    };
    let first = *around.first()?;
    if index == 0 {
        return Some(edge_of(first, true));
    }
    if around.len() == 2 && leans_forward {
        return Some(edge_of(around[1], true));
    }
    Some(edge_of(first, false))
}

/// `closestPositionToPoint:withinRange:`: the position in `start..=end` nearest `point`, and
/// whether it is the trailing edge of a grapheme, an upstream position; `None` with no rects.
pub(crate) fn closest_position(
    rects: &[GlyphRect],
    point: Offset,
    start: usize,
    end: usize,
    vertical_precision: f64,
) -> Option<(usize, bool)> {
    let mut closest: Option<usize> = None;
    for (i, glyph) in rects.iter().enumerate() {
        if glyph.position >= start && glyph.position <= end {
            let closer = match closest {
                None => true,
                Some(c) => is_boundary_closer(
                    point,
                    glyph.rect,
                    glyph.rtl,
                    false,
                    rects[c].rect,
                    rects[c].rtl,
                    vertical_precision,
                ),
            };
            if closer {
                closest = Some(i);
            }
        }
    }
    let closest_index = closest.unwrap_or(0);
    let closest_rect = rects.get(closest_index)?;
    let mut position = (closest_rect.position, false);
    let from = closest_index.saturating_sub(1);
    let to = (closest_index + 2).min(rects.len());
    for glyph in &rects[from..to] {
        let after = glyph.position + 1;
        if after >= start
            && after <= end
            && is_boundary_closer(
                point,
                glyph.rect,
                glyph.rtl,
                true,
                closest_rect.rect,
                closest_rect.rtl,
                vertical_precision,
            )
        {
            position = (after, true);
        }
    }
    Some(position)
}

/// `IsSelectionRectBoundaryCloserToPoint`: whether `point` is nearer a boundary of `rect`, its
/// trailing one where `use_trailing` and else its leading one, than the leading boundary of
/// `other`, by Flutter's measured rule: the vertically nearer wins; at the same height, the
/// horizontally nearer above the line's bottom, and the farther along the line below it.
fn is_boundary_closer(
    point: Offset,
    rect: Rect,
    rtl: bool,
    use_trailing: bool,
    other: Rect,
    other_rtl: bool,
    vertical_precision: f64,
) -> bool {
    let half_left = rect.left
        + if use_trailing ^ rtl {
            0.5 * rect.width()
        } else {
            0.0
        };
    let half = Rect::from_ltwh(half_left, rect.top, 0.5 * rect.width(), rect.height());
    if half.contains(point) {
        return true;
    }
    let boundary_x = rect.left
        + if rtl ^ use_trailing {
            rect.width()
        } else {
            0.0
        };
    let boundary_y = rect.top + rect.height() * 0.5;
    let y_dist = (boundary_y - point.dy()).abs();
    let x_dist = (boundary_x - point.dx()).abs();
    let other_x = other.left + if other_rtl { other.width() } else { 0.0 };
    let other_y = other.top + other.height() * 0.5;
    let y_dist_other = (other_y - point.dy()).abs();
    let x_dist_other = (other_x - point.dx()).abs();

    let is_closer_vertically = y_dist < y_dist_other - vertical_precision;
    let is_equal_vertically = (y_dist - y_dist_other).abs() <= vertical_precision;
    let is_above_bottom = point.dy() <= rect.bottom;
    let is_closer_horizontally = x_dist < x_dist_other;
    let is_below_bottom = point.dy() > rect.bottom;
    let is_farther = if rtl {
        rect.left < other.left
    } else {
        rect.left + if use_trailing { rect.width() } else { 0.0 } > other.left
    };
    is_closer_vertically
        || (is_equal_vertically
            && ((is_above_bottom && is_closer_horizontally) || (is_below_bottom && is_farther)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "Text1" in 14-point squares on one line, as Flutter's scribble test lays it out.
    fn line() -> Vec<GlyphRect> {
        (0..5)
            .map(|i| GlyphRect {
                position: i,
                rect: Rect::from_ltwh(14.0 * i as f64, 0.0, 14.0, 14.0),
                rtl: false,
            })
            .collect()
    }

    #[test]
    fn the_first_rect_spans_the_range_on_its_line() {
        assert_eq!(
            first_rect(&line(), 1, 3, 5),
            Some(Rect::from_ltrb(14.0, 0.0, 42.0, 14.0))
        );
        assert_eq!(first_rect(&[], 0, 1, 5), None);
    }

    #[test]
    fn the_first_rect_stops_at_the_end_of_the_line() {
        let mut rects = line();
        rects[3].rect = Rect::from_ltwh(0.0, 20.0, 14.0, 14.0);
        rects[4].rect = Rect::from_ltwh(14.0, 20.0, 14.0, 14.0);
        assert_eq!(
            first_rect(&rects, 1, 5, 5),
            Some(Rect::from_ltrb(14.0, 0.0, 42.0, 14.0))
        );
    }

    #[test]
    fn an_empty_range_has_the_zero_wide_rect_of_the_grapheme_at_it() {
        let in_range = rects_in_range(&line(), 2, 2, 5);
        assert_eq!(in_range.len(), 1);
        assert_eq!(in_range[0].rect, Rect::from_ltwh(28.0, 0.0, 0.0, 14.0));
        assert_eq!(rects_in_range(&line(), 1, 3, 5).len(), 2);
    }

    #[test]
    fn the_caret_sits_at_the_edge_its_affinity_says() {
        let before = (Rect::from_ltwh(14.0, 0.0, 14.0, 14.0), false);
        let after = (Rect::from_ltwh(28.0, 0.0, 14.0, 14.0), false);
        assert_eq!(
            caret_rect(&[before, after], 2, true),
            Some(Rect::from_ltwh(28.0, 0.0, 0.0, 14.0))
        );
        assert_eq!(
            caret_rect(&[before, after], 2, false),
            Some(Rect::from_ltwh(28.0, 0.0, 0.0, 14.0))
        );
        assert_eq!(
            caret_rect(&[before], 5, false),
            Some(Rect::from_ltwh(28.0, 0.0, 0.0, 14.0))
        );
        assert_eq!(
            caret_rect(&[after], 0, false),
            Some(Rect::from_ltwh(28.0, 0.0, 0.0, 14.0))
        );
        assert_eq!(caret_rect(&[], 3, true), None);
    }

    #[test]
    fn a_point_takes_the_nearest_boundary() {
        assert_eq!(
            closest_position(&line(), Offset::new(15.0, 7.0), 0, 5, 1.0),
            Some((1, false))
        );
        assert_eq!(
            closest_position(&line(), Offset::new(68.0, 7.0), 0, 5, 1.0),
            Some((5, true))
        );
    }

    #[test]
    fn a_rect_goes_through_the_editables_transform() {
        let mut moved = [0.0f32; 16];
        moved[0] = 1.0;
        moved[5] = 1.0;
        moved[10] = 1.0;
        moved[15] = 1.0;
        moved[12] = 10.0;
        moved[13] = 20.0;
        assert_eq!(
            rect_through(&moved, Rect::from_ltwh(1.0, 2.0, 3.0, 4.0)),
            Some(Rect::from_ltwh(11.0, 22.0, 3.0, 4.0))
        );
        let flat = [0.0f32; 16];
        assert_eq!(
            rect_through(&flat, Rect::from_ltwh(1.0, 2.0, 3.0, 4.0)),
            None
        );
    }
}
