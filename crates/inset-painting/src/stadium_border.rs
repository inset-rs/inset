//! Flutter counterpart: `painting/stadium_border.dart`.

use std::any::Any;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

use inset_embedder::{Canvas, Offset, Paint, Path, RRect, Radius, Rect, lerp_double};

use crate::basic_types::TextDirection;
use crate::border_radius::{BorderRadius, BorderRadiusGeometry};
use crate::borders::{
    BorderSide, BorderStyle, OutlinedBorder, ShapeBorder, outlined_border_dimensions,
};
use crate::circle_border::rrect_path;
use crate::draw::draw_rrect;
use crate::edge_insets::EdgeInsetsGeometry;
use crate::oval_border::as_circle_border;
use crate::rounded_rectangle_border::RoundedRectangleBorder;

/// A border that fits a stadium-shaped border (a box with semicircles on the ends)
/// within the rectangle of the widget it is applied to.
///
/// Typically used with `ShapeDecoration` to draw a stadium border.
///
/// If the rectangle is taller than it is wide, then the semicircles will be on the
/// top and bottom, and on the left and right otherwise.
#[derive(Clone, Copy, PartialEq)]
pub struct StadiumBorder {
    /// The border outline's color and weight.
    pub side: BorderSide,
}

impl StadiumBorder {
    /// Create a stadium border.
    pub fn new(side: BorderSide) -> StadiumBorder {
        StadiumBorder { side }
    }

    /// Creates a copy of this border, scaled by the factor `t`.
    pub fn scale(&self, t: f64) -> StadiumBorder {
        StadiumBorder::new(self.side.scale(t))
    }

    /// Returns a copy of this [`StadiumBorder`] with the given fields replaced
    /// with the new values.
    pub fn copy_with(&self, side: Option<BorderSide>) -> StadiumBorder {
        StadiumBorder::new(side.unwrap_or(self.side))
    }
}

impl Default for StadiumBorder {
    fn default() -> StadiumBorder {
        StadiumBorder::new(BorderSide::NONE)
    }
}

impl ShapeBorder for StadiumBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        outlined_border_dimensions(self.side)
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(StadiumBorder::scale(self, t))
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<StadiumBorder>()) {
            Some(Box::new(StadiumBorder::new(BorderSide::lerp(
                a.side, self.side, t,
            ))))
        } else if let Some(a) = a.and_then(as_circle_border) {
            Some(Box::new(StadiumToCircleBorder {
                side: BorderSide::lerp(a.side, self.side, t),
                circularity: 1.0 - t,
                eccentricity: a.eccentricity,
            }))
        } else if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<RoundedRectangleBorder>())
        {
            Some(Box::new(StadiumToRoundedRectangleBorder {
                side: BorderSide::lerp(a.side, self.side, t),
                border_radius: a.border_radius,
                rectilinearity: 1.0 - t,
            }))
        } else if a.is_none() {
            Some(ShapeBorder::scale(self, t))
        } else {
            None
        }
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<StadiumBorder>()) {
            Some(Box::new(StadiumBorder::new(BorderSide::lerp(
                self.side, b.side, t,
            ))))
        } else if let Some(b) = b.and_then(as_circle_border) {
            Some(Box::new(StadiumToCircleBorder {
                side: BorderSide::lerp(self.side, b.side, t),
                circularity: t,
                eccentricity: b.eccentricity,
            }))
        } else if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<RoundedRectangleBorder>())
        {
            Some(Box::new(StadiumToRoundedRectangleBorder {
                side: BorderSide::lerp(self.side, b.side, t),
                border_radius: b.border_radius,
                rectilinearity: t,
            }))
        } else if b.is_none() {
            Some(ShapeBorder::scale(self, 1.0 - t))
        } else {
            None
        }
    }

    fn get_inner_path(&self, rect: Rect, _text_direction: Option<TextDirection>) -> Arc<Path> {
        let radius = Radius::circular(rect.shortest_side() / 2.0);
        let border_rect = RRect::from_rect_and_radius(rect, radius);
        rrect_path(border_rect.deflate(self.side.stroke_inset()))
    }

    fn get_outer_path(&self, rect: Rect, _text_direction: Option<TextDirection>) -> Arc<Path> {
        let radius = Radius::circular(rect.shortest_side() / 2.0);
        rrect_path(RRect::from_rect_and_radius(rect, radius))
    }

    fn hit_test(
        &self,
        rect: Rect,
        position: Offset,
        _text_direction: Option<TextDirection>,
    ) -> bool {
        let radius = Radius::circular(rect.shortest_side() / 2.0);
        RRect::from_rect_and_radius(rect, radius).contains(position)
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        _text_direction: Option<TextDirection>,
    ) {
        let radius = Radius::circular(rect.shortest_side() / 2.0);
        draw_rrect(canvas, RRect::from_rect_and_radius(rect, radius), paint);
    }

    fn prefer_paint_interior(&self) -> bool {
        true
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect, _text_direction: Option<TextDirection>) {
        match self.side.style {
            BorderStyle::None => {}
            BorderStyle::Solid => {
                let radius = Radius::circular(rect.shortest_side() / 2.0);
                let border_rect = RRect::from_rect_and_radius(rect, radius);
                draw_rrect(
                    canvas,
                    border_rect.inflate(self.side.stroke_offset() / 2.0),
                    &self.side.to_paint(),
                );
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
            .downcast_ref::<StadiumBorder>()
            .is_some_and(|other| other == self)
    }
}

impl OutlinedBorder for StadiumBorder {
    fn side(&self) -> BorderSide {
        self.side
    }

    fn copy_with(&self, side: Option<BorderSide>) -> Box<dyn OutlinedBorder> {
        Box::new(StadiumBorder::copy_with(self, side))
    }

    fn clone_outlined(&self) -> Box<dyn OutlinedBorder> {
        Box::new(*self)
    }
}

impl Debug for StadiumBorder {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "StadiumBorder({:?})", self.side)
    }
}

#[derive(Clone, Copy)]
struct StadiumToCircleBorder {
    side: BorderSide,
    circularity: f64,
    eccentricity: f64,
}

impl StadiumToCircleBorder {
    fn copy_with(
        &self,
        side: Option<BorderSide>,
        circularity: Option<f64>,
        eccentricity: Option<f64>,
    ) -> StadiumToCircleBorder {
        StadiumToCircleBorder {
            side: side.unwrap_or(self.side),
            circularity: circularity.unwrap_or(self.circularity),
            eccentricity: eccentricity.unwrap_or(self.eccentricity),
        }
    }

    fn adjust_rect(self, rect: Rect) -> Rect {
        if self.circularity == 0.0 || rect.width() == rect.height() {
            return rect;
        }
        if rect.width() < rect.height() {
            let partial_delta = (rect.height() - rect.width()) / 2.0;
            let delta = self.circularity * partial_delta * (1.0 - self.eccentricity);
            Rect::from_ltrb(rect.left, rect.top + delta, rect.right, rect.bottom - delta)
        } else {
            let partial_delta = (rect.width() - rect.height()) / 2.0;
            let delta = self.circularity * partial_delta * (1.0 - self.eccentricity);
            Rect::from_ltrb(rect.left + delta, rect.top, rect.right - delta, rect.bottom)
        }
    }

    fn adjust_border_radius(self, rect: Rect) -> BorderRadius {
        let circle_radius = BorderRadius::circular(rect.shortest_side() / 2.0);
        if self.eccentricity != 0.0 {
            if rect.width() < rect.height() {
                BorderRadius::lerp(
                    Some(circle_radius),
                    Some(BorderRadius::all(Radius::elliptical(
                        rect.width() / 2.0,
                        (0.5 + self.eccentricity / 2.0) * rect.height() / 2.0,
                    ))),
                    self.circularity,
                )
                .unwrap()
            } else {
                BorderRadius::lerp(
                    Some(circle_radius),
                    Some(BorderRadius::all(Radius::elliptical(
                        (0.5 + self.eccentricity / 2.0) * rect.width() / 2.0,
                        rect.height() / 2.0,
                    ))),
                    self.circularity,
                )
                .unwrap()
            }
        } else {
            circle_radius
        }
    }
}

impl ShapeBorder for StadiumToCircleBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        outlined_border_dimensions(self.side)
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(StadiumToCircleBorder {
            side: self.side.scale(t),
            circularity: t,
            eccentricity: self.eccentricity,
        })
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<StadiumBorder>()) {
            Some(Box::new(StadiumToCircleBorder {
                side: BorderSide::lerp(a.side, self.side, t),
                circularity: self.circularity * t,
                eccentricity: self.eccentricity,
            }))
        } else if let Some(a) = a.and_then(as_circle_border) {
            Some(Box::new(StadiumToCircleBorder {
                side: BorderSide::lerp(a.side, self.side, t),
                circularity: self.circularity + (1.0 - self.circularity) * (1.0 - t),
                eccentricity: a.eccentricity,
            }))
        } else if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<StadiumToCircleBorder>()) {
            Some(Box::new(StadiumToCircleBorder {
                side: BorderSide::lerp(a.side, self.side, t),
                circularity: lerp_double(Some(a.circularity), Some(self.circularity), t).unwrap(),
                eccentricity: lerp_double(Some(a.eccentricity), Some(self.eccentricity), t)
                    .unwrap(),
            }))
        } else if a.is_none() {
            Some(ShapeBorder::scale(self, t))
        } else {
            None
        }
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<StadiumBorder>()) {
            Some(Box::new(StadiumToCircleBorder {
                side: BorderSide::lerp(self.side, b.side, t),
                circularity: self.circularity * (1.0 - t),
                eccentricity: self.eccentricity,
            }))
        } else if let Some(b) = b.and_then(as_circle_border) {
            Some(Box::new(StadiumToCircleBorder {
                side: BorderSide::lerp(self.side, b.side, t),
                circularity: self.circularity + (1.0 - self.circularity) * t,
                eccentricity: b.eccentricity,
            }))
        } else if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<StadiumToCircleBorder>()) {
            Some(Box::new(StadiumToCircleBorder {
                side: BorderSide::lerp(self.side, b.side, t),
                circularity: lerp_double(Some(self.circularity), Some(b.circularity), t).unwrap(),
                eccentricity: lerp_double(Some(self.eccentricity), Some(b.eccentricity), t)
                    .unwrap(),
            }))
        } else if b.is_none() {
            Some(ShapeBorder::scale(self, 1.0 - t))
        } else {
            None
        }
    }

    fn get_inner_path(&self, rect: Rect, _text_direction: Option<TextDirection>) -> Arc<Path> {
        rrect_path(
            self.adjust_border_radius(rect)
                .to_rrect(self.adjust_rect(rect))
                .deflate(self.side.stroke_inset()),
        )
    }

    fn get_outer_path(&self, rect: Rect, _text_direction: Option<TextDirection>) -> Arc<Path> {
        rrect_path(
            self.adjust_border_radius(rect)
                .to_rrect(self.adjust_rect(rect)),
        )
    }

    fn hit_test(
        &self,
        rect: Rect,
        position: Offset,
        _text_direction: Option<TextDirection>,
    ) -> bool {
        self.adjust_border_radius(rect)
            .to_rrect(self.adjust_rect(rect))
            .contains(position)
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        _text_direction: Option<TextDirection>,
    ) {
        draw_rrect(
            canvas,
            self.adjust_border_radius(rect)
                .to_rrect(self.adjust_rect(rect)),
            paint,
        );
    }

    fn prefer_paint_interior(&self) -> bool {
        true
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect, _text_direction: Option<TextDirection>) {
        match self.side.style {
            BorderStyle::None => {}
            BorderStyle::Solid => {
                let border_rect = self
                    .adjust_border_radius(rect)
                    .to_rrect(self.adjust_rect(rect));
                draw_rrect(
                    canvas,
                    border_rect.inflate(self.side.stroke_offset() / 2.0),
                    &self.side.to_paint(),
                );
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
            .downcast_ref::<StadiumToCircleBorder>()
            .is_some_and(|other| other.side == self.side && other.circularity == self.circularity)
    }
}

impl OutlinedBorder for StadiumToCircleBorder {
    fn side(&self) -> BorderSide {
        self.side
    }

    fn copy_with(&self, side: Option<BorderSide>) -> Box<dyn OutlinedBorder> {
        Box::new(StadiumToCircleBorder::copy_with(self, side, None, None))
    }

    fn clone_outlined(&self) -> Box<dyn OutlinedBorder> {
        Box::new(*self)
    }
}

impl Debug for StadiumToCircleBorder {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.eccentricity != 0.0 {
            write!(
                f,
                "StadiumBorder({:?}, {:.1}% of the way to being a CircleBorder that is {:.1}% oval)",
                self.side,
                self.circularity * 100.0,
                self.eccentricity * 100.0
            )
        } else {
            write!(
                f,
                "StadiumBorder({:?}, {:.1}% of the way to being a CircleBorder)",
                self.side,
                self.circularity * 100.0
            )
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct StadiumToRoundedRectangleBorder {
    side: BorderSide,
    border_radius: BorderRadiusGeometry,
    rectilinearity: f64,
}

impl StadiumToRoundedRectangleBorder {
    fn copy_with(
        &self,
        side: Option<BorderSide>,
        border_radius: Option<BorderRadiusGeometry>,
        rectilinearity: Option<f64>,
    ) -> StadiumToRoundedRectangleBorder {
        StadiumToRoundedRectangleBorder {
            side: side.unwrap_or(self.side),
            border_radius: border_radius.unwrap_or(self.border_radius),
            rectilinearity: rectilinearity.unwrap_or(self.rectilinearity),
        }
    }

    fn adjust_border_radius(self, rect: Rect) -> BorderRadiusGeometry {
        BorderRadiusGeometry::lerp(
            Some(self.border_radius),
            Some(BorderRadius::all(Radius::circular(rect.shortest_side() / 2.0)).into()),
            1.0 - self.rectilinearity,
        )
        .unwrap()
    }
}

impl ShapeBorder for StadiumToRoundedRectangleBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        outlined_border_dimensions(self.side)
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(StadiumToRoundedRectangleBorder {
            side: self.side.scale(t),
            border_radius: self.border_radius * t,
            rectilinearity: t,
        })
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<StadiumBorder>()) {
            Some(Box::new(StadiumToRoundedRectangleBorder {
                side: BorderSide::lerp(a.side, self.side, t),
                border_radius: self.border_radius,
                rectilinearity: self.rectilinearity * t,
            }))
        } else if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<RoundedRectangleBorder>())
        {
            Some(Box::new(StadiumToRoundedRectangleBorder {
                side: BorderSide::lerp(a.side, self.side, t),
                border_radius: self.border_radius,
                rectilinearity: self.rectilinearity + (1.0 - self.rectilinearity) * (1.0 - t),
            }))
        } else if let Some(a) =
            a.and_then(|a| a.as_any().downcast_ref::<StadiumToRoundedRectangleBorder>())
        {
            Some(Box::new(StadiumToRoundedRectangleBorder {
                side: BorderSide::lerp(a.side, self.side, t),
                border_radius: BorderRadiusGeometry::lerp(
                    Some(a.border_radius),
                    Some(self.border_radius),
                    t,
                )
                .unwrap(),
                rectilinearity: lerp_double(Some(a.rectilinearity), Some(self.rectilinearity), t)
                    .unwrap(),
            }))
        } else if a.is_none() {
            Some(ShapeBorder::scale(self, t))
        } else {
            None
        }
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<StadiumBorder>()) {
            Some(Box::new(StadiumToRoundedRectangleBorder {
                side: BorderSide::lerp(self.side, b.side, t),
                border_radius: self.border_radius,
                rectilinearity: self.rectilinearity * (1.0 - t),
            }))
        } else if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<RoundedRectangleBorder>())
        {
            Some(Box::new(StadiumToRoundedRectangleBorder {
                side: BorderSide::lerp(self.side, b.side, t),
                border_radius: self.border_radius,
                rectilinearity: self.rectilinearity + (1.0 - self.rectilinearity) * t,
            }))
        } else if let Some(b) =
            b.and_then(|b| b.as_any().downcast_ref::<StadiumToRoundedRectangleBorder>())
        {
            Some(Box::new(StadiumToRoundedRectangleBorder {
                side: BorderSide::lerp(self.side, b.side, t),
                border_radius: BorderRadiusGeometry::lerp(
                    Some(self.border_radius),
                    Some(b.border_radius),
                    t,
                )
                .unwrap(),
                rectilinearity: lerp_double(Some(self.rectilinearity), Some(b.rectilinearity), t)
                    .unwrap(),
            }))
        } else if b.is_none() {
            Some(ShapeBorder::scale(self, 1.0 - t))
        } else {
            None
        }
    }

    fn get_inner_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        let border_rect = self
            .adjust_border_radius(rect)
            .resolve(text_direction)
            .to_rrect(rect);
        let adjusted_rect = border_rect.deflate(
            lerp_double(Some(self.side.width), Some(0.0), self.side.stroke_align).unwrap(),
        );
        rrect_path(adjusted_rect)
    }

    fn get_outer_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        rrect_path(
            self.adjust_border_radius(rect)
                .resolve(text_direction)
                .to_rrect(rect),
        )
    }

    fn hit_test(
        &self,
        rect: Rect,
        position: Offset,
        text_direction: Option<TextDirection>,
    ) -> bool {
        let adjusted_border_radius = self.adjust_border_radius(rect).resolve(text_direction);
        if adjusted_border_radius == BorderRadius::ZERO {
            return rect.contains(position);
        }
        adjusted_border_radius.to_rrect(rect).contains(position)
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        text_direction: Option<TextDirection>,
    ) {
        let adjusted_border_radius = self.adjust_border_radius(rect);
        if adjusted_border_radius == BorderRadius::ZERO.into() {
            canvas.draw_rect(rect, paint);
        } else {
            draw_rrect(
                canvas,
                adjusted_border_radius
                    .resolve(text_direction)
                    .to_rrect(rect),
                paint,
            );
        }
    }

    fn prefer_paint_interior(&self) -> bool {
        true
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect, text_direction: Option<TextDirection>) {
        match self.side.style {
            BorderStyle::None => {}
            BorderStyle::Solid => {
                let adjusted_border_radius = self.adjust_border_radius(rect);
                let border_rect = adjusted_border_radius
                    .resolve(text_direction)
                    .to_rrect(rect);
                draw_rrect(
                    canvas,
                    border_rect.inflate(self.side.stroke_offset() / 2.0),
                    &self.side.to_paint(),
                );
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
            .downcast_ref::<StadiumToRoundedRectangleBorder>()
            .is_some_and(|other| other == self)
    }
}

impl OutlinedBorder for StadiumToRoundedRectangleBorder {
    fn side(&self) -> BorderSide {
        self.side
    }

    fn copy_with(&self, side: Option<BorderSide>) -> Box<dyn OutlinedBorder> {
        Box::new(StadiumToRoundedRectangleBorder::copy_with(
            self, side, None, None,
        ))
    }

    fn clone_outlined(&self) -> Box<dyn OutlinedBorder> {
        Box::new(*self)
    }
}

impl Debug for StadiumToRoundedRectangleBorder {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "StadiumBorder({:?}, {:?}, {:.1}% of the way to being a RoundedRectangleBorder)",
            self.side,
            self.border_radius,
            self.rectilinearity * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inset_embedder::Color;

    fn side_width(width: f64) -> BorderSide {
        BorderSide {
            width,
            ..BorderSide::default()
        }
    }

    #[test]
    fn defaults_copy_with_scale_lerp() {
        let border = StadiumBorder::default();
        assert_eq!(border.side, BorderSide::NONE);
        assert_eq!(border, border.copy_with(None));
        let side = BorderSide {
            width: 10.0,
            color: Color::new(0xFF123456),
            ..BorderSide::default()
        };
        assert_eq!(
            StadiumBorder::default().copy_with(Some(side)),
            StadiumBorder::new(side)
        );

        let c10 = StadiumBorder::new(side_width(10.0));
        let c15 = StadiumBorder::new(side_width(15.0));
        let c20 = StadiumBorder::new(side_width(20.0));
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
    fn stroke_align_dimensions() {
        let center = StadiumBorder::new(BorderSide {
            width: 10.0,
            stroke_align: BorderSide::STROKE_ALIGN_CENTER,
            ..BorderSide::default()
        });
        let outside = StadiumBorder::new(BorderSide {
            width: 10.0,
            stroke_align: BorderSide::STROKE_ALIGN_OUTSIDE,
            ..BorderSide::default()
        });
        assert_eq!(center.dimensions(), EdgeInsetsGeometry::all(5.0));
        assert_eq!(outside.dimensions(), EdgeInsetsGeometry::all(0.0));
    }
}
