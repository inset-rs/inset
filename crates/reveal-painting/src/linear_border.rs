//! Flutter counterpart: `painting/linear_border.dart`.

use std::any::Any;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

use reveal_embedder::{
    Canvas, Color, FillRule, Offset, Paint, PaintStyle, Path, PathBuilder, Rect, Stroke,
    lerp_double,
};

use crate::basic_types::TextDirection;
use crate::borders::{BorderSide, BorderStyle, OutlinedBorder, ShapeBorder};
use crate::edge_insets::{EdgeInsetsDirectional, EdgeInsetsGeometry};

/// Defines the relative size and alignment of one [`LinearBorder`] edge.
///
/// A [`LinearBorder`] defines a box outline as zero to four edges, each of
/// which is rendered as a single line. The width and color of the lines is
/// defined by [`LinearBorder::side`].
///
/// Each line's length is defined by [`size`](Self::size), a value between 0.0
/// and 1.0 (the default) which defines the length as a percentage of the
/// length of a box edge.
///
/// When [`size`](Self::size) is less than 1.0, the line is aligned within the
/// available space according to [`alignment`](Self::alignment), a value
/// between -1.0 and 1.0. The default is 0.0, which means centered, -1.0 means
/// align on the "start" side, and 1.0 means align on the "end" side.
#[derive(Clone, Copy, PartialEq)]
pub struct LinearBorderEdge {
    /// A value between 0.0 and 1.0 that defines the length of the edge as a
    /// percentage of the length of the corresponding box edge. Default is 1.0.
    pub size: f64,
    /// A value between -1.0 and 1.0 that defines how edges for which
    /// [`size`](Self::size) is less than 1.0 are aligned relative to the
    /// corresponding box edge.
    pub alignment: f64,
}

impl LinearBorderEdge {
    /// Defines one side of a [`LinearBorder`].
    ///
    /// The values of [`size`](Self::size) and [`alignment`](Self::alignment)
    /// must be between 0.0 and 1.0, and -1.0 and 1.0 respectively.
    pub fn new(size: f64, alignment: f64) -> LinearBorderEdge {
        debug_assert!(
            (0.0..=1.0).contains(&size),
            "size must be between 0.0 and 1.0"
        );
        LinearBorderEdge { size, alignment }
    }

    /// Linearly interpolates between two [`LinearBorderEdge`]s.
    ///
    /// If both `a` and `b` are None then None is returned. If `a` is None then
    /// we interpolate to `b` varying [`size`](Self::size) from 0.0 to `b.size`.
    /// If `b` is None then we interpolate from `a` varying size from `a.size`
    /// to zero. Otherwise both values are interpolated.
    pub fn lerp(
        a: Option<LinearBorderEdge>,
        b: Option<LinearBorderEdge>,
        t: f64,
    ) -> Option<LinearBorderEdge> {
        if a == b {
            return a;
        }
        let a = match a {
            Some(a) => a,
            None => LinearBorderEdge::new(0.0, b.unwrap().alignment),
        };
        let b = match b {
            Some(b) => b,
            None => LinearBorderEdge::new(0.0, a.alignment),
        };
        Some(LinearBorderEdge::new(
            lerp_double(Some(a.size), Some(b.size), t).unwrap(),
            lerp_double(Some(a.alignment), Some(b.alignment), t).unwrap(),
        ))
    }
}

impl Default for LinearBorderEdge {
    fn default() -> LinearBorderEdge {
        LinearBorderEdge::new(1.0, 0.0)
    }
}

impl Debug for LinearBorderEdge {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut s = String::from("LinearBorderEdge(");
        if self.size != 1.0 {
            s.push_str(&format!("size: {}", self.size));
        }
        if self.alignment != 0.0 {
            let comma = if self.size != 1.0 { ", " } else { "" };
            s.push_str(&format!("{comma}alignment: {}", self.alignment));
        }
        s.push(')');
        write!(f, "{s}")
    }
}

/// An [`OutlinedBorder`] like `BoxBorder` that allows one to define a
/// rectangular (box) border in terms of zero to four [`LinearBorderEdge`]s,
/// each of which is rendered as a single line.
///
/// This class resolves itself against the current [`TextDirection`]. Start and
/// end values resolve to left and right for [`TextDirection::Ltr`] and to
/// right and left for [`TextDirection::Rtl`].
#[derive(Clone, Copy, PartialEq)]
pub struct LinearBorder {
    /// The border outline's color and weight.
    pub side: BorderSide,
    /// Defines the left edge for [`TextDirection::Ltr`] or the right for
    /// [`TextDirection::Rtl`].
    pub start: Option<LinearBorderEdge>,
    /// Defines the right edge for [`TextDirection::Ltr`] or the left for
    /// [`TextDirection::Rtl`].
    pub end: Option<LinearBorderEdge>,
    /// Defines the top edge.
    pub top: Option<LinearBorderEdge>,
    /// Defines the bottom edge.
    pub bottom: Option<LinearBorderEdge>,
}

impl LinearBorder {
    /// No border.
    pub const NONE: LinearBorder = LinearBorder {
        side: BorderSide::NONE,
        start: None,
        end: None,
        top: None,
        bottom: None,
    };

    /// Creates a rectangular box border that's rendered as zero to four lines.
    pub fn new() -> LinearBorder {
        LinearBorder::NONE
    }

    /// The color and width of each line.
    pub fn side(mut self, side: BorderSide) -> LinearBorder {
        self.side = side;
        self
    }

    /// Defines the left edge for [`TextDirection::Ltr`] or the right for
    /// [`TextDirection::Rtl`].
    pub fn start(mut self, start: LinearBorderEdge) -> LinearBorder {
        self.start = Some(start);
        self
    }

    /// Defines the right edge for [`TextDirection::Ltr`] or the left for
    /// [`TextDirection::Rtl`].
    pub fn end(mut self, end: LinearBorderEdge) -> LinearBorder {
        self.end = Some(end);
        self
    }

    /// Defines the top edge.
    pub fn top(mut self, top: LinearBorderEdge) -> LinearBorder {
        self.top = Some(top);
        self
    }

    /// Defines the bottom edge.
    pub fn bottom(mut self, bottom: LinearBorderEdge) -> LinearBorder {
        self.bottom = Some(bottom);
        self
    }

    /// Creates a rectangular box border with an edge on the left for
    /// [`TextDirection::Ltr`] or on the right for [`TextDirection::Rtl`].
    pub fn start_side(side: BorderSide, alignment: f64, size: f64) -> LinearBorder {
        LinearBorder::new()
            .side(side)
            .start(LinearBorderEdge::new(size, alignment))
    }

    /// Creates a rectangular box border with an edge on the right for
    /// [`TextDirection::Ltr`] or on the left for [`TextDirection::Rtl`].
    pub fn end_side(side: BorderSide, alignment: f64, size: f64) -> LinearBorder {
        LinearBorder::new()
            .side(side)
            .end(LinearBorderEdge::new(size, alignment))
    }

    /// Creates a rectangular box border with an edge on the top.
    pub fn top_side(side: BorderSide, alignment: f64, size: f64) -> LinearBorder {
        LinearBorder::new()
            .side(side)
            .top(LinearBorderEdge::new(size, alignment))
    }

    /// Creates a rectangular box border with an edge on the bottom.
    pub fn bottom_side(side: BorderSide, alignment: f64, size: f64) -> LinearBorder {
        LinearBorder::new()
            .side(side)
            .bottom(LinearBorderEdge::new(size, alignment))
    }

    /// Creates a copy of this border, scaled by the factor `t`.
    pub fn scale(&self, t: f64) -> LinearBorder {
        LinearBorder::new().side(self.side.scale(t))
    }

    /// Creates a copy of this object. Chain setters to replace fields
    /// (`border.copy_with().side(s).start(e)`).
    pub fn copy_with(&self) -> LinearBorder {
        *self
    }

    fn from_fields(
        side: BorderSide,
        start: Option<LinearBorderEdge>,
        end: Option<LinearBorderEdge>,
        top: Option<LinearBorderEdge>,
        bottom: Option<LinearBorderEdge>,
    ) -> LinearBorder {
        LinearBorder {
            side,
            start,
            end,
            top,
            bottom,
        }
    }
}

impl Default for LinearBorder {
    fn default() -> LinearBorder {
        LinearBorder::NONE
    }
}

impl ShapeBorder for LinearBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        let width = self.side.width;
        EdgeInsetsDirectional::from_steb(
            if self.start.is_none() { 0.0 } else { width },
            if self.top.is_none() { 0.0 } else { width },
            if self.end.is_none() { 0.0 } else { width },
            if self.bottom.is_none() { 0.0 } else { width },
        )
        .into()
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(LinearBorder::scale(self, t))
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<LinearBorder>()) {
            Some(Box::new(LinearBorder::from_fields(
                BorderSide::lerp(a.side, self.side, t),
                LinearBorderEdge::lerp(a.start, self.start, t),
                LinearBorderEdge::lerp(a.end, self.end, t),
                LinearBorderEdge::lerp(a.top, self.top, t),
                LinearBorderEdge::lerp(a.bottom, self.bottom, t),
            )))
        } else if a.is_none() {
            Some(ShapeBorder::scale(self, t))
        } else {
            None
        }
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<LinearBorder>()) {
            Some(Box::new(LinearBorder::from_fields(
                BorderSide::lerp(self.side, b.side, t),
                LinearBorderEdge::lerp(self.start, b.start, t),
                LinearBorderEdge::lerp(self.end, b.end, t),
                LinearBorderEdge::lerp(self.top, b.top, t),
                LinearBorderEdge::lerp(self.bottom, b.bottom, t),
            )))
        } else if b.is_none() {
            Some(ShapeBorder::scale(self, 1.0 - t))
        } else {
            None
        }
    }

    fn get_inner_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        let adjusted_rect = self.dimensions().resolve(text_direction).deflate_rect(rect);
        let mut path = PathBuilder::new();
        path.rect(adjusted_rect.into());
        path.build()
    }

    fn get_outer_path(&self, rect: Rect, _text_direction: Option<TextDirection>) -> Arc<Path> {
        let mut path = PathBuilder::new();
        path.rect(rect.into());
        path.build()
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect, text_direction: Option<TextDirection>) {
        let insets = self.dimensions().resolve(text_direction);
        let rtl = text_direction == Some(TextDirection::Rtl);

        fn draw_edge(canvas: &mut Canvas, rect: Rect, color: Color) {
            let mut paint = Paint {
                color: color.into(),
                ..Paint::default()
            };
            let mut path = PathBuilder::new();
            path.move_to(Offset::new(rect.left, rect.top));
            if rect.width() == 0.0 {
                paint.style = PaintStyle::Stroke(Stroke::new(0.0));
                path.line_to(Offset::new(rect.left, rect.bottom));
            } else if rect.height() == 0.0 {
                paint.style = PaintStyle::Stroke(Stroke::new(0.0));
                path.line_to(Offset::new(rect.right, rect.top));
            } else {
                paint.style = PaintStyle::Fill;
                path.line_to(Offset::new(rect.right, rect.top));
                path.line_to(Offset::new(rect.right, rect.bottom));
                path.line_to(Offset::new(rect.left, rect.bottom));
            }
            canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
        }

        if let Some(start) = self.start
            && start.size != 0.0
            && self.side.style != BorderStyle::None
        {
            let inset_rect = Rect::from_ltwh(
                rect.left,
                rect.top + insets.top,
                rect.width(),
                rect.height() - insets.vertical(),
            );
            let x = if rtl {
                rect.right - insets.right
            } else {
                rect.left
            };
            let width = if rtl { insets.right } else { insets.left };
            let height = inset_rect.height() * start.size;
            let y = (inset_rect.height() - height) * ((start.alignment + 1.0) / 2.0);
            draw_edge(
                canvas,
                Rect::from_ltwh(x, y, width, height),
                self.side.color,
            );
        }

        if let Some(end) = self.end
            && end.size != 0.0
            && self.side.style != BorderStyle::None
        {
            let inset_rect = Rect::from_ltwh(
                rect.left,
                rect.top + insets.top,
                rect.width(),
                rect.height() - insets.vertical(),
            );
            let x = if rtl {
                rect.left
            } else {
                rect.right - insets.right
            };
            let width = if rtl { insets.left } else { insets.right };
            let height = inset_rect.height() * end.size;
            let y = (inset_rect.height() - height) * ((end.alignment + 1.0) / 2.0);
            draw_edge(
                canvas,
                Rect::from_ltwh(x, y, width, height),
                self.side.color,
            );
        }

        if let Some(top) = self.top
            && top.size != 0.0
            && self.side.style != BorderStyle::None
        {
            let width = rect.width() * top.size;
            let start_x = (rect.width() - width) * ((top.alignment + 1.0) / 2.0);
            let x = if rtl {
                rect.width() - start_x - width
            } else {
                start_x
            };
            draw_edge(
                canvas,
                Rect::from_ltwh(x, rect.top, width, insets.top),
                self.side.color,
            );
        }

        if let Some(bottom) = self.bottom
            && bottom.size != 0.0
            && self.side.style != BorderStyle::None
        {
            let width = rect.width() * bottom.size;
            let start_x = (rect.width() - width) * ((bottom.alignment + 1.0) / 2.0);
            let x = if rtl {
                rect.width() - start_x - width
            } else {
                start_x
            };
            draw_edge(
                canvas,
                Rect::from_ltwh(x, rect.bottom - insets.bottom, width, self.side.width),
                self.side.color,
            );
        }
    }

    fn clone_box(&self) -> Box<dyn ShapeBorder> {
        Box::new(*self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_outlined_border(&self) -> Option<&dyn OutlinedBorder> {
        Some(self)
    }

    fn eq_shape(&self, other: &dyn ShapeBorder) -> bool {
        other
            .as_any()
            .downcast_ref::<LinearBorder>()
            .is_some_and(|other| other == self)
    }
}

impl OutlinedBorder for LinearBorder {
    fn side(&self) -> BorderSide {
        self.side
    }

    fn copy_with(&self, side: Option<BorderSide>) -> Box<dyn OutlinedBorder> {
        Box::new(self.copy_with().side(side.unwrap_or(self.side)))
    }

    fn clone_outlined(&self) -> Box<dyn OutlinedBorder> {
        Box::new(*self)
    }
}

impl Debug for LinearBorder {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self == &LinearBorder::NONE {
            return write!(f, "LinearBorder.none");
        }
        write!(f, "LinearBorder(side: {:?}", self.side)?;
        if let Some(start) = self.start {
            write!(f, ", start: {start:?}")?;
        }
        if let Some(end) = self.end {
            write!(f, ", end: {end:?}")?;
        }
        if let Some(top) = self.top {
            write!(f, ", top: {top:?}")?;
        }
        if let Some(bottom) = self.bottom {
            write!(f, ", bottom: {bottom:?}")?;
        }
        write!(f, ")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge_insets::EdgeInsets;

    #[test]
    fn edge_defaults() {
        let edge = LinearBorderEdge::default();
        assert_eq!(edge.size, 1.0);
        assert_eq!(edge.alignment, 0.0);
    }

    #[test]
    fn defaults() {
        fn expect_empty(border: LinearBorder) {
            assert_eq!(border.side, BorderSide::NONE);
            assert_eq!(border.dimensions(), EdgeInsets::ZERO.into());
            assert!(!border.prefer_paint_interior());
            assert!(border.start.is_none());
            assert!(border.end.is_none());
            assert!(border.top.is_none());
            assert!(border.bottom.is_none());
        }
        expect_empty(LinearBorder::NONE);
        let start = LinearBorder::start_side(BorderSide::NONE, 0.0, 1.0);
        assert_eq!(start.side, BorderSide::NONE);
        assert_eq!(start.start, Some(LinearBorderEdge::default()));
        assert!(start.end.is_none());
        let end = LinearBorder::end_side(BorderSide::NONE, 0.0, 1.0);
        assert_eq!(end.end, Some(LinearBorderEdge::default()));
        let top = LinearBorder::top_side(BorderSide::NONE, 0.0, 1.0);
        assert_eq!(top.top, Some(LinearBorderEdge::default()));
        let bottom = LinearBorder::bottom_side(BorderSide::NONE, 0.0, 1.0);
        assert_eq!(bottom.bottom, Some(LinearBorderEdge::default()));
    }

    #[test]
    fn copy_with_eq() {
        assert_eq!(LinearBorder::NONE, LinearBorder::NONE.copy_with());
        let side = BorderSide {
            width: 10.0,
            color: Color::new(0xFF123456),
            ..BorderSide::default()
        };
        assert_eq!(
            LinearBorder::NONE.copy_with().side(side),
            LinearBorder::new().side(side)
        );
    }

    #[test]
    fn lerp_identical() {
        assert!(<dyn OutlinedBorder>::lerp(None, None, 0.0).is_none());
        let border = LinearBorder::NONE;
        assert!(
            <dyn OutlinedBorder>::lerp(Some(&border), Some(&border), 0.5)
                .unwrap()
                .eq_shape(&border)
        );
        assert!(LinearBorderEdge::lerp(None, None, 0.0).is_none());
        let edge = LinearBorderEdge::default();
        assert_eq!(
            LinearBorderEdge::lerp(Some(edge), Some(edge), 0.5),
            Some(edge)
        );
    }
}
