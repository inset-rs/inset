//! Flutter counterpart: `painting/circle_border.dart`.

use std::any::Any;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

use inset_embedder::{Canvas, Paint, Path, PathBuilder, rrect_radii_elliptical};
use inset_embedder::{Offset, RRect, Radius, Rect, clamp_double, lerp_double};

use crate::basic_types::TextDirection;
use crate::borders::{
    BorderSide, BorderStyle, OutlinedBorder, ShapeBorder, outlined_border_dimensions,
};
use crate::draw::draw_oval;
use crate::edge_insets::EdgeInsetsGeometry;

/// A border that fits a circle within the available space.
///
/// Typically used with `ShapeDecoration` to draw a circle.
///
/// The [`dimensions`](ShapeBorder::dimensions) assume that the border is being
/// used in a square space. When applied to a rectangular space, the border
/// paints in the center of the rectangle.
///
/// The `eccentricity` parameter describes how much a circle will deform to
/// fit the rectangle it is a border for. A value of zero implies no
/// deformation (a circle touching at least two sides of the rectangle), a
/// value of one implies full deformation (an oval touching all sides of the
/// rectangle).
#[derive(Clone, Copy, PartialEq)]
pub struct CircleBorder {
    /// The border outline's color and weight.
    pub side: BorderSide,
    /// Defines the ratio (0.0-1.0) from which the border will deform to fit a
    /// rectangle.
    ///
    /// When 0.0, it draws a circle touching at least two sides of the rectangle.
    /// When 1.0, it draws an oval touching all sides of the rectangle.
    pub eccentricity: f64,
}

impl CircleBorder {
    /// Create a circle border.
    pub fn new(side: BorderSide, eccentricity: f64) -> CircleBorder {
        debug_assert!(
            eccentricity >= 0.0,
            "The eccentricity argument {eccentricity} is not greater than or equal to zero."
        );
        debug_assert!(
            eccentricity <= 1.0,
            "The eccentricity argument {eccentricity} is not less than or equal to one."
        );
        CircleBorder { side, eccentricity }
    }

    /// Creates a copy of this border, scaled by the factor `t`.
    pub fn scale(&self, t: f64) -> CircleBorder {
        CircleBorder::new(self.side.scale(t), self.eccentricity)
    }

    /// Returns a copy of this [`CircleBorder`] with the given fields replaced
    /// with the new values.
    pub fn copy_with(&self, side: Option<BorderSide>, eccentricity: Option<f64>) -> CircleBorder {
        CircleBorder::new(
            side.unwrap_or(self.side),
            eccentricity.unwrap_or(self.eccentricity),
        )
    }

    fn adjust_rect(self, rect: Rect) -> Rect {
        if self.eccentricity == 0.0 || rect.width() == rect.height() {
            return Rect::from_circle(rect.center(), rect.shortest_side() / 2.0);
        }
        if rect.width() < rect.height() {
            let delta = (1.0 - self.eccentricity) * (rect.height() - rect.width()) / 2.0;
            Rect::from_ltrb(rect.left, rect.top + delta, rect.right, rect.bottom - delta)
        } else {
            let delta = (1.0 - self.eccentricity) * (rect.width() - rect.height()) / 2.0;
            Rect::from_ltrb(rect.left + delta, rect.top, rect.right - delta, rect.bottom)
        }
    }
}

impl Default for CircleBorder {
    fn default() -> CircleBorder {
        CircleBorder::new(BorderSide::NONE, 0.0)
    }
}

impl ShapeBorder for CircleBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        outlined_border_dimensions(self.side)
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(CircleBorder::scale(self, t))
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<CircleBorder>()) {
            Some(Box::new(CircleBorder::new(
                BorderSide::lerp(a.side, self.side, t),
                clamp_double(
                    lerp_double(Some(a.eccentricity), Some(self.eccentricity), t).unwrap(),
                    0.0,
                    1.0,
                ),
            )))
        } else if a.is_none() {
            Some(ShapeBorder::scale(self, t))
        } else {
            None
        }
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<CircleBorder>()) {
            Some(Box::new(CircleBorder::new(
                BorderSide::lerp(self.side, b.side, t),
                clamp_double(
                    lerp_double(Some(self.eccentricity), Some(b.eccentricity), t).unwrap(),
                    0.0,
                    1.0,
                ),
            )))
        } else if b.is_none() {
            Some(ShapeBorder::scale(self, 1.0 - t))
        } else {
            None
        }
    }

    fn get_inner_path(&self, rect: Rect, _text_direction: Option<TextDirection>) -> Arc<Path> {
        oval_path(self.adjust_rect(rect).deflate(self.side.stroke_inset()))
    }

    fn get_outer_path(&self, rect: Rect, _text_direction: Option<TextDirection>) -> Arc<Path> {
        oval_path(self.adjust_rect(rect))
    }

    fn hit_test(
        &self,
        rect: Rect,
        position: Offset,
        _text_direction: Option<TextDirection>,
    ) -> bool {
        let adjusted_rect = self.adjust_rect(rect);
        RRect::from_rect_and_radius(
            adjusted_rect,
            Radius::elliptical(adjusted_rect.width() / 2.0, adjusted_rect.height() / 2.0),
        )
        .contains(position)
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        _text_direction: Option<TextDirection>,
    ) {
        if self.eccentricity == 0.0 {
            canvas.draw_circle(rect.center(), (rect.shortest_side() / 2.0) as f32, paint);
        } else {
            draw_oval(canvas, self.adjust_rect(rect), paint);
        }
    }

    fn prefer_paint_interior(&self) -> bool {
        true
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect, _text_direction: Option<TextDirection>) {
        match self.side.style {
            BorderStyle::None => {}
            BorderStyle::Solid => {
                if self.eccentricity == 0.0 {
                    canvas.draw_circle(
                        rect.center(),
                        ((rect.shortest_side() + self.side.stroke_offset()) / 2.0) as f32,
                        &self.side.to_paint(),
                    );
                } else {
                    let border_rect = self.adjust_rect(rect);
                    draw_oval(
                        canvas,
                        border_rect.inflate(self.side.stroke_offset() / 2.0),
                        &self.side.to_paint(),
                    );
                }
            }
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
            .downcast_ref::<CircleBorder>()
            .is_some_and(|other| other == self)
    }
}

impl OutlinedBorder for CircleBorder {
    fn side(&self) -> BorderSide {
        self.side
    }

    fn copy_with(&self, side: Option<BorderSide>) -> Box<dyn OutlinedBorder> {
        Box::new(CircleBorder::copy_with(self, side, None))
    }

    fn clone_outlined(&self) -> Box<dyn OutlinedBorder> {
        Box::new(*self)
    }
}

impl Debug for CircleBorder {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.eccentricity != 0.0 {
            write!(
                f,
                "CircleBorder({:?}, eccentricity: {})",
                self.side, self.eccentricity
            )
        } else {
            write!(f, "CircleBorder({:?})", self.side)
        }
    }
}

pub(crate) fn oval_path(rect: Rect) -> Arc<Path> {
    let mut path = PathBuilder::new();
    let rx = (rect.width() / 2.0) as f32;
    let ry = (rect.height() / 2.0) as f32;
    path.rrect_radii_elliptical(rect, [[rx, ry]; 4]);
    path.build()
}

pub(crate) fn rrect_path(rrect: RRect) -> Arc<Path> {
    let mut path = PathBuilder::new();
    path.rrect_radii_elliptical(rrect.outer_rect(), rrect_radii_elliptical(rrect));
    path.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use inset_embedder::{Color, FillRule};

    fn side_width(width: f64) -> BorderSide {
        BorderSide {
            width,
            ..BorderSide::default()
        }
    }

    fn contains(path: &Arc<Path>, x: f64, y: f64) -> bool {
        path.contains(Offset::new(x, y).into(), FillRule::NonZero)
    }

    #[test]
    fn defaults() {
        let border = CircleBorder::default();
        assert_eq!(border.side, BorderSide::NONE);
        assert_eq!(border.eccentricity, 0.0);
    }

    #[test]
    fn copy_with_eq() {
        assert_eq!(
            CircleBorder::default(),
            CircleBorder::default().copy_with(None, None)
        );
        assert_eq!(
            CircleBorder::new(BorderSide::NONE, 0.5),
            CircleBorder::default().copy_with(None, Some(0.5))
        );
        let side = BorderSide {
            width: 10.0,
            color: Color::new(0xFF123456),
            ..BorderSide::default()
        };
        assert_eq!(
            CircleBorder::default().copy_with(Some(side), None),
            CircleBorder::new(side, 0.0)
        );
    }

    #[test]
    fn scale_lerp_dimensions() {
        let c10 = CircleBorder::new(side_width(10.0), 0.0);
        let c15 = CircleBorder::new(side_width(15.0), 0.0);
        let c20 = CircleBorder::new(side_width(20.0), 0.0);
        assert_eq!(c10.dimensions(), EdgeInsetsGeometry::all(10.0));
        assert_eq!(c10.scale(2.0), c20);
        assert_eq!(c20.scale(0.5), c10);
        let as10: &dyn ShapeBorder = &c10;
        let as20: &dyn ShapeBorder = &c20;
        assert!(
            <dyn ShapeBorder>::lerp(Some(as10), Some(as20), 0.0)
                .unwrap()
                .eq_shape(&c10)
        );
        assert!(
            <dyn ShapeBorder>::lerp(Some(as10), Some(as20), 0.5)
                .unwrap()
                .eq_shape(&c15)
        );
        assert!(
            <dyn ShapeBorder>::lerp(Some(as10), Some(as20), 1.0)
                .unwrap()
                .eq_shape(&c20)
        );
    }

    #[test]
    fn paths_for_square_and_eccentric_oval() {
        let circle_rect = Rect::from_ltwh(50.0, 0.0, 100.0, 100.0);
        let rect = Rect::from_ltwh(0.0, 0.0, 200.0, 100.0);
        let bounds = CircleBorder::default().get_outer_path(rect, None).bounds();
        assert!((bounds.x - circle_rect.left as f32).abs() < 0.01);
        assert!((bounds.y - circle_rect.top as f32).abs() < 0.01);
        assert!((bounds.width - circle_rect.width() as f32).abs() < 0.01);
        assert!((bounds.height - circle_rect.height() as f32).abs() < 0.01);

        let oval = CircleBorder::new(BorderSide::NONE, 1.0);
        let outer = oval.get_outer_path(rect, None).bounds();
        assert!((outer.width - 200.0).abs() < 0.01);
        assert!((outer.height - 100.0).abs() < 0.01);
    }

    #[test]
    fn hit_test_uses_the_adjusted_circle() {
        let border = CircleBorder::default();
        let rect = Rect::from_ltwh(0.0, 0.0, 100.0, 20.0);
        assert!(border.hit_test(rect, Offset::new(50.0, 10.0), None));
        assert!(!border.hit_test(rect, Offset::new(1.0, 1.0), None));
        assert!(!contains(&border.get_outer_path(rect, None), 30.0, 10.0));
        assert!(contains(&border.get_outer_path(rect, None), 50.0, 10.0));
    }
}
