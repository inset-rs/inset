//! Flutter counterpart: `painting/rounded_rectangle_border.dart`.

use std::any::Any;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

use reveal_embedder::{
    Canvas, Offset, Paint, Path, PathBuilder, RSuperellipse, Radius, Rect, lerp_double,
    rsuperellipse_radii_elliptical,
};

use crate::basic_types::TextDirection;
use crate::border_radius::{BorderRadius, BorderRadiusGeometry};
use crate::borders::{
    BorderSide, BorderStyle, OutlinedBorder, ShapeBorder, outlined_border_dimensions,
};
use crate::circle_border::rrect_path;
use crate::draw::{draw_drrect, draw_rrect, draw_rsuperellipse};
use crate::edge_insets::EdgeInsetsGeometry;
use crate::oval_border::as_circle_border;

/// A rectangular border with rounded corners.
///
/// Typically used with `ShapeDecoration` to draw a box with a rounded
/// rectangle.
///
/// This shape can interpolate to and from [`CircleBorder`].
#[derive(Clone, Copy, PartialEq)]
pub struct RoundedRectangleBorder {
    /// The border outline's color and weight.
    pub side: BorderSide,
    /// The radii for each corner.
    pub border_radius: BorderRadiusGeometry,
}

impl RoundedRectangleBorder {
    /// Creates a rounded rectangle border.
    pub fn new(side: BorderSide, border_radius: BorderRadiusGeometry) -> RoundedRectangleBorder {
        RoundedRectangleBorder {
            side,
            border_radius,
        }
    }

    /// Creates a copy of this border, scaled by the factor `t`.
    pub fn scale(&self, t: f64) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(self.side.scale(t), self.border_radius * t)
    }

    /// Returns a copy of this RoundedRectangleBorder with the given fields
    /// replaced with the new values.
    pub fn copy_with(
        &self,
        side: Option<BorderSide>,
        border_radius: Option<BorderRadiusGeometry>,
    ) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(
            side.unwrap_or(self.side),
            border_radius.unwrap_or(self.border_radius),
        )
    }
}

impl Default for RoundedRectangleBorder {
    fn default() -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(BorderSide::NONE, BorderRadius::ZERO.into())
    }
}

impl ShapeBorder for RoundedRectangleBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        outlined_border_dimensions(self.side)
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(RoundedRectangleBorder::scale(self, t))
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<RoundedRectangleBorder>()) {
            Some(Box::new(RoundedRectangleBorder::new(
                BorderSide::lerp(a.side, self.side, t),
                BorderRadiusGeometry::lerp(Some(a.border_radius), Some(self.border_radius), t)
                    .unwrap(),
            )))
        } else if let Some(a) = a.and_then(as_circle_border) {
            Some(Box::new(RoundedRectangleToCircleBorder {
                side: BorderSide::lerp(a.side, self.side, t),
                border_radius: self.border_radius,
                circularity: 1.0 - t,
                eccentricity: a.eccentricity,
            }))
        } else if a.is_none() {
            Some(ShapeBorder::scale(self, t))
        } else {
            None
        }
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<RoundedRectangleBorder>()) {
            Some(Box::new(RoundedRectangleBorder::new(
                BorderSide::lerp(self.side, b.side, t),
                BorderRadiusGeometry::lerp(Some(self.border_radius), Some(b.border_radius), t)
                    .unwrap(),
            )))
        } else if let Some(b) = b.and_then(as_circle_border) {
            Some(Box::new(RoundedRectangleToCircleBorder {
                side: BorderSide::lerp(self.side, b.side, t),
                border_radius: self.border_radius,
                circularity: t,
                eccentricity: b.eccentricity,
            }))
        } else if b.is_none() {
            Some(ShapeBorder::scale(self, 1.0 - t))
        } else {
            None
        }
    }

    fn get_inner_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        let border_rect = self.border_radius.resolve(text_direction).to_rrect(rect);
        rrect_path(border_rect.deflate(self.side.stroke_inset()))
    }

    fn get_outer_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        rrect_path(self.border_radius.resolve(text_direction).to_rrect(rect))
    }

    fn hit_test(
        &self,
        rect: Rect,
        position: Offset,
        text_direction: Option<TextDirection>,
    ) -> bool {
        let resolved_border_radius = self.border_radius.resolve(text_direction);
        if resolved_border_radius == BorderRadius::ZERO {
            return rect.contains(position);
        }
        resolved_border_radius.to_rrect(rect).contains(position)
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        text_direction: Option<TextDirection>,
    ) {
        if self.border_radius == BorderRadius::ZERO.into() {
            canvas.draw_rect(rect, paint);
        } else {
            draw_rrect(
                canvas,
                self.border_radius.resolve(text_direction).to_rrect(rect),
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
                if self.side.width == 0.0 {
                    draw_rrect(
                        canvas,
                        self.border_radius.resolve(text_direction).to_rrect(rect),
                        &self.side.to_paint(),
                    );
                } else {
                    let paint = Paint {
                        color: self.side.color.into(),
                        ..Paint::default()
                    };
                    let border_rect = self.border_radius.resolve(text_direction).to_rrect(rect);
                    let inner = border_rect.deflate(self.side.stroke_inset());
                    let outer = border_rect.inflate(self.side.stroke_outset());
                    draw_drrect(canvas, outer, inner, &paint);
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
            .downcast_ref::<RoundedRectangleBorder>()
            .is_some_and(|other| other == self)
    }
}

impl OutlinedBorder for RoundedRectangleBorder {
    fn side(&self) -> BorderSide {
        self.side
    }

    fn copy_with(&self, side: Option<BorderSide>) -> Box<dyn OutlinedBorder> {
        Box::new(RoundedRectangleBorder::copy_with(self, side, None))
    }

    fn clone_outlined(&self) -> Box<dyn OutlinedBorder> {
        Box::new(*self)
    }
}

impl Debug for RoundedRectangleBorder {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "RoundedRectangleBorder({:?}, {:?})",
            self.side, self.border_radius
        )
    }
}

/// A rectangular border with rounded corners following the shape of an
/// [`RSuperellipse`].
#[derive(Clone, Copy, PartialEq)]
pub struct RoundedSuperellipseBorder {
    /// The border outline's color and weight.
    pub side: BorderSide,
    /// The radii for each corner.
    pub border_radius: BorderRadiusGeometry,
}

impl RoundedSuperellipseBorder {
    /// Creates a rounded rectangle border.
    ///
    /// If `border_radius` is None, it defaults to [`BorderRadius::ZERO`].
    pub fn new(
        side: BorderSide,
        border_radius: Option<BorderRadiusGeometry>,
    ) -> RoundedSuperellipseBorder {
        RoundedSuperellipseBorder {
            side,
            border_radius: border_radius.unwrap_or(BorderRadius::ZERO.into()),
        }
    }

    /// Creates a copy of this border, scaled by the factor `t`.
    pub fn scale(&self, t: f64) -> RoundedSuperellipseBorder {
        RoundedSuperellipseBorder::new(self.side.scale(t), Some(self.border_radius * t))
    }

    /// Returns a copy of this RoundedSuperellipseBorder with the given fields
    /// replaced with the new values.
    pub fn copy_with(
        &self,
        side: Option<BorderSide>,
        border_radius: Option<BorderRadiusGeometry>,
    ) -> RoundedSuperellipseBorder {
        RoundedSuperellipseBorder::new(
            side.unwrap_or(self.side),
            Some(border_radius.unwrap_or(self.border_radius)),
        )
    }
}

impl Default for RoundedSuperellipseBorder {
    fn default() -> RoundedSuperellipseBorder {
        RoundedSuperellipseBorder::new(BorderSide::NONE, None)
    }
}

impl ShapeBorder for RoundedSuperellipseBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        outlined_border_dimensions(self.side)
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(RoundedSuperellipseBorder::scale(self, t))
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<RoundedSuperellipseBorder>()) {
            Some(Box::new(RoundedSuperellipseBorder::new(
                BorderSide::lerp(a.side, self.side, t),
                BorderRadiusGeometry::lerp(Some(a.border_radius), Some(self.border_radius), t),
            )))
        } else if let Some(a) = a.and_then(as_circle_border) {
            Some(Box::new(RoundedSuperellipseToCircleBorder {
                side: BorderSide::lerp(a.side, self.side, t),
                border_radius: self.border_radius,
                circularity: 1.0 - t,
                eccentricity: a.eccentricity,
            }))
        } else if a.is_none() {
            Some(ShapeBorder::scale(self, t))
        } else {
            None
        }
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<RoundedSuperellipseBorder>()) {
            Some(Box::new(RoundedSuperellipseBorder::new(
                BorderSide::lerp(self.side, b.side, t),
                BorderRadiusGeometry::lerp(Some(self.border_radius), Some(b.border_radius), t),
            )))
        } else if let Some(b) = b.and_then(as_circle_border) {
            Some(Box::new(RoundedSuperellipseToCircleBorder {
                side: BorderSide::lerp(self.side, b.side, t),
                border_radius: self.border_radius,
                circularity: t,
                eccentricity: b.eccentricity,
            }))
        } else if b.is_none() {
            Some(ShapeBorder::scale(self, 1.0 - t))
        } else {
            None
        }
    }

    fn get_inner_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        if self.border_radius == BorderRadius::ZERO.into() {
            rect_path(rect.deflate(self.side.stroke_inset()))
        } else {
            let border_rect = self
                .border_radius
                .resolve(text_direction)
                .to_r_superellipse(rect);
            rsuperellipse_path(border_rect.deflate(self.side.stroke_inset()))
        }
    }

    fn get_outer_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        if self.border_radius == BorderRadius::ZERO.into() {
            rect_path(rect)
        } else {
            rsuperellipse_path(
                self.border_radius
                    .resolve(text_direction)
                    .to_r_superellipse(rect),
            )
        }
    }

    fn hit_test(
        &self,
        rect: Rect,
        position: Offset,
        text_direction: Option<TextDirection>,
    ) -> bool {
        let resolved_border_radius = self.border_radius.resolve(text_direction);
        if resolved_border_radius == BorderRadius::ZERO {
            return rect.contains(position);
        }
        resolved_border_radius
            .to_r_superellipse(rect)
            .contains(position)
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        text_direction: Option<TextDirection>,
    ) {
        if self.border_radius == BorderRadius::ZERO.into() {
            canvas.draw_rect(rect, paint);
        } else {
            draw_rsuperellipse(
                canvas,
                self.border_radius
                    .resolve(text_direction)
                    .to_r_superellipse(rect),
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
                let stroke_offset = (self.side.stroke_outset() - self.side.stroke_inset()) / 2.0;
                if self.border_radius == BorderRadius::ZERO.into() {
                    let base = rect.inflate(stroke_offset);
                    canvas.draw_rect(base, &self.side.to_paint());
                } else {
                    let base = self
                        .border_radius
                        .resolve(text_direction)
                        .to_r_superellipse(rect)
                        .inflate(stroke_offset);
                    draw_rsuperellipse(canvas, base, &self.side.to_paint());
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
            .downcast_ref::<RoundedSuperellipseBorder>()
            .is_some_and(|other| other == self)
    }
}

impl OutlinedBorder for RoundedSuperellipseBorder {
    fn side(&self) -> BorderSide {
        self.side
    }

    fn copy_with(&self, side: Option<BorderSide>) -> Box<dyn OutlinedBorder> {
        Box::new(RoundedSuperellipseBorder::copy_with(self, side, None))
    }

    fn clone_outlined(&self) -> Box<dyn OutlinedBorder> {
        Box::new(*self)
    }
}

impl Debug for RoundedSuperellipseBorder {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "RoundedSuperellipseBorder({:?}, {:?})",
            self.side, self.border_radius
        )
    }
}

fn rect_path(rect: Rect) -> Arc<Path> {
    let mut path = PathBuilder::new();
    path.rect(rect.into());
    path.build()
}

fn rsuperellipse_path(rse: RSuperellipse) -> Arc<Path> {
    let mut path = PathBuilder::new();
    path.rsuperellipse_radii(rse.outer_rect().into(), rsuperellipse_radii_elliptical(rse));
    path.build()
}

fn shape_to_circle_adjust_rect(rect: Rect, circularity: f64, eccentricity: f64) -> Rect {
    if circularity == 0.0 || rect.width() == rect.height() {
        return rect;
    }
    if rect.width() < rect.height() {
        let partial_delta = (rect.height() - rect.width()) / 2.0;
        let delta = circularity * partial_delta * (1.0 - eccentricity);
        Rect::from_ltrb(rect.left, rect.top + delta, rect.right, rect.bottom - delta)
    } else {
        let partial_delta = (rect.width() - rect.height()) / 2.0;
        let delta = circularity * partial_delta * (1.0 - eccentricity);
        Rect::from_ltrb(rect.left + delta, rect.top, rect.right - delta, rect.bottom)
    }
}

fn shape_to_circle_adjust_border_radius(
    rect: Rect,
    text_direction: Option<TextDirection>,
    border_radius: BorderRadiusGeometry,
    circularity: f64,
    eccentricity: f64,
) -> BorderRadius {
    let resolved_radius = border_radius.resolve(text_direction);
    if circularity == 0.0 {
        return resolved_radius;
    }
    if eccentricity != 0.0 {
        if rect.width() < rect.height() {
            BorderRadius::lerp(
                Some(resolved_radius),
                Some(BorderRadius::all(Radius::elliptical(
                    rect.width() / 2.0,
                    (0.5 + eccentricity / 2.0) * rect.height() / 2.0,
                ))),
                circularity,
            )
            .unwrap()
        } else {
            BorderRadius::lerp(
                Some(resolved_radius),
                Some(BorderRadius::all(Radius::elliptical(
                    (0.5 + eccentricity / 2.0) * rect.width() / 2.0,
                    rect.height() / 2.0,
                ))),
                circularity,
            )
            .unwrap()
        }
    } else {
        BorderRadius::lerp(
            Some(resolved_radius),
            Some(BorderRadius::circular(rect.shortest_side() / 2.0)),
            circularity,
        )
        .unwrap()
    }
}

#[derive(Clone, Copy)]
struct RoundedRectangleToCircleBorder {
    side: BorderSide,
    border_radius: BorderRadiusGeometry,
    circularity: f64,
    eccentricity: f64,
}

impl RoundedRectangleToCircleBorder {
    fn copy_with(
        &self,
        side: Option<BorderSide>,
        border_radius: Option<BorderRadiusGeometry>,
        circularity: Option<f64>,
        eccentricity: Option<f64>,
    ) -> RoundedRectangleToCircleBorder {
        RoundedRectangleToCircleBorder {
            side: side.unwrap_or(self.side),
            border_radius: border_radius.unwrap_or(self.border_radius),
            circularity: circularity.unwrap_or(self.circularity),
            eccentricity: eccentricity.unwrap_or(self.eccentricity),
        }
    }

    fn draw_shape(
        canvas: &mut Canvas,
        rect: Rect,
        radius: BorderRadius,
        paint: &Paint,
        inflation: Option<f64>,
    ) {
        let mut rrect = radius.to_rrect(rect);
        if let Some(inflation) = inflation {
            rrect = rrect.inflate(inflation);
        }
        draw_rrect(canvas, rrect, paint);
    }

    fn build_path(rect: Rect, radius: BorderRadius, inflation: Option<f64>) -> Arc<Path> {
        let mut rrect = radius.to_rrect(rect);
        if let Some(inflation) = inflation {
            rrect = rrect.inflate(inflation);
        }
        rrect_path(rrect)
    }
}

impl ShapeBorder for RoundedRectangleToCircleBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        outlined_border_dimensions(self.side)
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(self.copy_with(
            Some(self.side.scale(t)),
            Some(self.border_radius * t),
            Some(t),
            None,
        ))
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<RoundedRectangleBorder>()) {
            Some(Box::new(self.copy_with(
                Some(BorderSide::lerp(a.side, self.side, t)),
                BorderRadiusGeometry::lerp(Some(a.border_radius), Some(self.border_radius), t),
                Some(self.circularity * t),
                None,
            )))
        } else if let Some(a) = a.and_then(as_circle_border) {
            Some(Box::new(self.copy_with(
                Some(BorderSide::lerp(a.side, self.side, t)),
                Some(self.border_radius),
                Some(self.circularity + (1.0 - self.circularity) * (1.0 - t)),
                Some(a.eccentricity),
            )))
        } else if let Some(a) =
            a.and_then(|a| a.as_any().downcast_ref::<RoundedRectangleToCircleBorder>())
        {
            Some(Box::new(self.copy_with(
                Some(BorderSide::lerp(a.side, self.side, t)),
                BorderRadiusGeometry::lerp(Some(a.border_radius), Some(self.border_radius), t),
                lerp_double(Some(a.circularity), Some(self.circularity), t),
                None,
            )))
        } else if a.is_none() {
            Some(ShapeBorder::scale(self, t))
        } else {
            None
        }
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<RoundedRectangleBorder>()) {
            Some(Box::new(self.copy_with(
                Some(BorderSide::lerp(self.side, b.side, t)),
                BorderRadiusGeometry::lerp(Some(self.border_radius), Some(b.border_radius), t),
                Some(self.circularity * (1.0 - t)),
                None,
            )))
        } else if let Some(b) = b.and_then(as_circle_border) {
            Some(Box::new(self.copy_with(
                Some(BorderSide::lerp(self.side, b.side, t)),
                Some(self.border_radius),
                Some(self.circularity + (1.0 - self.circularity) * t),
                Some(b.eccentricity),
            )))
        } else if let Some(b) =
            b.and_then(|b| b.as_any().downcast_ref::<RoundedRectangleToCircleBorder>())
        {
            Some(Box::new(self.copy_with(
                Some(BorderSide::lerp(self.side, b.side, t)),
                BorderRadiusGeometry::lerp(Some(self.border_radius), Some(b.border_radius), t),
                lerp_double(Some(self.circularity), Some(b.circularity), t),
                None,
            )))
        } else if b.is_none() {
            Some(ShapeBorder::scale(self, 1.0 - t))
        } else {
            None
        }
    }

    fn get_inner_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        Self::build_path(
            shape_to_circle_adjust_rect(rect, self.circularity, self.eccentricity),
            shape_to_circle_adjust_border_radius(
                rect,
                text_direction,
                self.border_radius,
                self.circularity,
                self.eccentricity,
            ),
            Some(-lerp_double(Some(self.side.width), Some(0.0), self.side.stroke_align).unwrap()),
        )
    }

    fn get_outer_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        Self::build_path(
            shape_to_circle_adjust_rect(rect, self.circularity, self.eccentricity),
            shape_to_circle_adjust_border_radius(
                rect,
                text_direction,
                self.border_radius,
                self.circularity,
                self.eccentricity,
            ),
            None,
        )
    }

    fn hit_test(
        &self,
        rect: Rect,
        position: Offset,
        text_direction: Option<TextDirection>,
    ) -> bool {
        let adjusted_rect = shape_to_circle_adjust_rect(rect, self.circularity, self.eccentricity);
        let adjusted_border_radius = shape_to_circle_adjust_border_radius(
            rect,
            text_direction,
            self.border_radius,
            self.circularity,
            self.eccentricity,
        );
        if adjusted_border_radius == BorderRadius::ZERO {
            return adjusted_rect.contains(position);
        }
        adjusted_border_radius
            .to_rrect(adjusted_rect)
            .contains(position)
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        text_direction: Option<TextDirection>,
    ) {
        let adjusted_border_radius = shape_to_circle_adjust_border_radius(
            rect,
            text_direction,
            self.border_radius,
            self.circularity,
            self.eccentricity,
        );
        if adjusted_border_radius == BorderRadius::ZERO {
            canvas.draw_rect(
                shape_to_circle_adjust_rect(rect, self.circularity, self.eccentricity),
                paint,
            );
        } else {
            Self::draw_shape(
                canvas,
                shape_to_circle_adjust_rect(rect, self.circularity, self.eccentricity),
                adjusted_border_radius,
                paint,
                None,
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
                Self::draw_shape(
                    canvas,
                    shape_to_circle_adjust_rect(rect, self.circularity, self.eccentricity),
                    shape_to_circle_adjust_border_radius(
                        rect,
                        text_direction,
                        self.border_radius,
                        self.circularity,
                        self.eccentricity,
                    ),
                    &self.side.to_paint(),
                    Some(self.side.stroke_offset() / 2.0),
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
            .downcast_ref::<RoundedRectangleToCircleBorder>()
            .is_some_and(|other| {
                other.side == self.side
                    && other.border_radius == self.border_radius
                    && other.circularity == self.circularity
            })
    }
}

impl OutlinedBorder for RoundedRectangleToCircleBorder {
    fn side(&self) -> BorderSide {
        self.side
    }

    fn copy_with(&self, side: Option<BorderSide>) -> Box<dyn OutlinedBorder> {
        Box::new(RoundedRectangleToCircleBorder::copy_with(
            self, side, None, None, None,
        ))
    }

    fn clone_outlined(&self) -> Box<dyn OutlinedBorder> {
        Box::new(*self)
    }
}

impl Debug for RoundedRectangleToCircleBorder {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.eccentricity != 0.0 {
            write!(
                f,
                "RoundedRectangleBorder({:?}, {:?}, {:.1}% of the way to being a CircleBorder that is {:.1}% oval)",
                self.side,
                self.border_radius,
                self.circularity * 100.0,
                self.eccentricity * 100.0
            )
        } else {
            write!(
                f,
                "RoundedRectangleBorder({:?}, {:?}, {:.1}% of the way to being a CircleBorder)",
                self.side,
                self.border_radius,
                self.circularity * 100.0
            )
        }
    }
}

#[derive(Clone, Copy)]
struct RoundedSuperellipseToCircleBorder {
    side: BorderSide,
    border_radius: BorderRadiusGeometry,
    circularity: f64,
    eccentricity: f64,
}

impl RoundedSuperellipseToCircleBorder {
    fn copy_with(
        &self,
        side: Option<BorderSide>,
        border_radius: Option<BorderRadiusGeometry>,
        circularity: Option<f64>,
        eccentricity: Option<f64>,
    ) -> RoundedSuperellipseToCircleBorder {
        RoundedSuperellipseToCircleBorder {
            side: side.unwrap_or(self.side),
            border_radius: border_radius.unwrap_or(self.border_radius),
            circularity: circularity.unwrap_or(self.circularity),
            eccentricity: eccentricity.unwrap_or(self.eccentricity),
        }
    }

    fn draw_shape(
        canvas: &mut Canvas,
        rect: Rect,
        radius: BorderRadius,
        paint: &Paint,
        inflation: Option<f64>,
    ) {
        let mut rsuperellipse = radius.to_r_superellipse(rect);
        if let Some(inflation) = inflation {
            rsuperellipse = rsuperellipse.inflate(inflation);
        }
        draw_rsuperellipse(canvas, rsuperellipse, paint);
    }

    fn build_path(rect: Rect, radius: BorderRadius, inflation: Option<f64>) -> Arc<Path> {
        let mut rsuperellipse = radius.to_r_superellipse(rect);
        if let Some(inflation) = inflation {
            rsuperellipse = rsuperellipse.inflate(inflation);
        }
        rsuperellipse_path(rsuperellipse)
    }
}

impl ShapeBorder for RoundedSuperellipseToCircleBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        outlined_border_dimensions(self.side)
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(self.copy_with(
            Some(self.side.scale(t)),
            Some(self.border_radius * t),
            Some(t),
            None,
        ))
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<RoundedSuperellipseBorder>()) {
            Some(Box::new(self.copy_with(
                Some(BorderSide::lerp(a.side, self.side, t)),
                BorderRadiusGeometry::lerp(Some(a.border_radius), Some(self.border_radius), t),
                Some(self.circularity * t),
                None,
            )))
        } else if let Some(a) = a.and_then(as_circle_border) {
            Some(Box::new(self.copy_with(
                Some(BorderSide::lerp(a.side, self.side, t)),
                Some(self.border_radius),
                Some(self.circularity + (1.0 - self.circularity) * (1.0 - t)),
                Some(a.eccentricity),
            )))
        } else if let Some(a) = a.and_then(|a| {
            a.as_any()
                .downcast_ref::<RoundedSuperellipseToCircleBorder>()
        }) {
            Some(Box::new(self.copy_with(
                Some(BorderSide::lerp(a.side, self.side, t)),
                BorderRadiusGeometry::lerp(Some(a.border_radius), Some(self.border_radius), t),
                lerp_double(Some(a.circularity), Some(self.circularity), t),
                None,
            )))
        } else if a.is_none() {
            Some(ShapeBorder::scale(self, t))
        } else {
            None
        }
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<RoundedSuperellipseBorder>()) {
            Some(Box::new(self.copy_with(
                Some(BorderSide::lerp(self.side, b.side, t)),
                BorderRadiusGeometry::lerp(Some(self.border_radius), Some(b.border_radius), t),
                Some(self.circularity * (1.0 - t)),
                None,
            )))
        } else if let Some(b) = b.and_then(as_circle_border) {
            Some(Box::new(self.copy_with(
                Some(BorderSide::lerp(self.side, b.side, t)),
                Some(self.border_radius),
                Some(self.circularity + (1.0 - self.circularity) * t),
                Some(b.eccentricity),
            )))
        } else if let Some(b) = b.and_then(|b| {
            b.as_any()
                .downcast_ref::<RoundedSuperellipseToCircleBorder>()
        }) {
            Some(Box::new(self.copy_with(
                Some(BorderSide::lerp(self.side, b.side, t)),
                BorderRadiusGeometry::lerp(Some(self.border_radius), Some(b.border_radius), t),
                lerp_double(Some(self.circularity), Some(b.circularity), t),
                None,
            )))
        } else if b.is_none() {
            Some(ShapeBorder::scale(self, 1.0 - t))
        } else {
            None
        }
    }

    fn get_inner_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        Self::build_path(
            shape_to_circle_adjust_rect(rect, self.circularity, self.eccentricity),
            shape_to_circle_adjust_border_radius(
                rect,
                text_direction,
                self.border_radius,
                self.circularity,
                self.eccentricity,
            ),
            Some(-lerp_double(Some(self.side.width), Some(0.0), self.side.stroke_align).unwrap()),
        )
    }

    fn get_outer_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        Self::build_path(
            shape_to_circle_adjust_rect(rect, self.circularity, self.eccentricity),
            shape_to_circle_adjust_border_radius(
                rect,
                text_direction,
                self.border_radius,
                self.circularity,
                self.eccentricity,
            ),
            None,
        )
    }

    fn hit_test(
        &self,
        rect: Rect,
        position: Offset,
        text_direction: Option<TextDirection>,
    ) -> bool {
        let adjusted_rect = shape_to_circle_adjust_rect(rect, self.circularity, self.eccentricity);
        let adjusted_border_radius = shape_to_circle_adjust_border_radius(
            rect,
            text_direction,
            self.border_radius,
            self.circularity,
            self.eccentricity,
        );
        if adjusted_border_radius == BorderRadius::ZERO {
            return adjusted_rect.contains(position);
        }
        adjusted_border_radius
            .to_r_superellipse(adjusted_rect)
            .contains(position)
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        text_direction: Option<TextDirection>,
    ) {
        let adjusted_border_radius = shape_to_circle_adjust_border_radius(
            rect,
            text_direction,
            self.border_radius,
            self.circularity,
            self.eccentricity,
        );
        if adjusted_border_radius == BorderRadius::ZERO {
            canvas.draw_rect(
                shape_to_circle_adjust_rect(rect, self.circularity, self.eccentricity),
                paint,
            );
        } else {
            Self::draw_shape(
                canvas,
                shape_to_circle_adjust_rect(rect, self.circularity, self.eccentricity),
                adjusted_border_radius,
                paint,
                None,
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
                Self::draw_shape(
                    canvas,
                    shape_to_circle_adjust_rect(rect, self.circularity, self.eccentricity),
                    shape_to_circle_adjust_border_radius(
                        rect,
                        text_direction,
                        self.border_radius,
                        self.circularity,
                        self.eccentricity,
                    ),
                    &self.side.to_paint(),
                    Some(self.side.stroke_offset() / 2.0),
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
            .downcast_ref::<RoundedSuperellipseToCircleBorder>()
            .is_some_and(|other| {
                other.side == self.side
                    && other.border_radius == self.border_radius
                    && other.circularity == self.circularity
            })
    }
}

impl OutlinedBorder for RoundedSuperellipseToCircleBorder {
    fn side(&self) -> BorderSide {
        self.side
    }

    fn copy_with(&self, side: Option<BorderSide>) -> Box<dyn OutlinedBorder> {
        Box::new(RoundedSuperellipseToCircleBorder::copy_with(
            self, side, None, None, None,
        ))
    }

    fn clone_outlined(&self) -> Box<dyn OutlinedBorder> {
        Box::new(*self)
    }
}

impl Debug for RoundedSuperellipseToCircleBorder {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.eccentricity != 0.0 {
            write!(
                f,
                "RoundedSuperellipseBorder({:?}, {:?}, {:.1}% of the way to being a CircleBorder that is {:.1}% oval)",
                self.side,
                self.border_radius,
                self.circularity * 100.0,
                self.eccentricity * 100.0
            )
        } else {
            write!(
                f,
                "RoundedSuperellipseBorder({:?}, {:?}, {:.1}% of the way to being a CircleBorder)",
                self.side,
                self.border_radius,
                self.circularity * 100.0
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reveal_embedder::{Color, FillRule, Radius};

    use crate::border_radius::BorderRadiusDirectional;
    use crate::circle_border::CircleBorder;

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
    fn rounded_rectangle_defaults_copy_with_scale_lerp() {
        let border = RoundedRectangleBorder::default();
        assert_eq!(border.side, BorderSide::NONE);
        assert_eq!(border.border_radius, BorderRadius::ZERO.into());
        assert_eq!(border, border.copy_with(None, None));

        let side = BorderSide {
            width: 10.0,
            color: Color::new(0xFF123456),
            ..BorderSide::default()
        };
        let radius = BorderRadius::all(Radius::circular(16.0));
        let directional = BorderRadiusDirectional::all(Radius::circular(16.0));
        assert_eq!(
            RoundedRectangleBorder::default().copy_with(Some(side), Some(radius.into())),
            RoundedRectangleBorder::new(side, radius.into())
        );
        assert_eq!(
            RoundedRectangleBorder::default().copy_with(Some(side), Some(directional.into())),
            RoundedRectangleBorder::new(side, directional.into())
        );

        let c10 = RoundedRectangleBorder::new(
            side_width(10.0),
            BorderRadius::all(Radius::circular(100.0)).into(),
        );
        let c15 = RoundedRectangleBorder::new(
            side_width(15.0),
            BorderRadius::all(Radius::circular(150.0)).into(),
        );
        let c20 = RoundedRectangleBorder::new(
            side_width(20.0),
            BorderRadius::all(Radius::circular(200.0)).into(),
        );
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
    fn rounded_rectangle_lerps_with_circle() {
        let r = RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadius::all(Radius::circular(10.0)).into(),
        );
        let c = CircleBorder::default();
        let rect = Rect::from_ltwh(0.0, 0.0, 100.0, 20.0);
        let r_path = r.get_outer_path(rect, None);
        assert!(contains(&r_path, 30.0, 10.0));
        assert!(contains(&r_path, 50.0, 10.0));
        assert!(!contains(&r_path, 1.0, 1.0));
        assert!(!contains(&r_path, 99.0, 19.0));

        let c_path = c.get_outer_path(rect, None);
        assert!(contains(&c_path, 50.0, 10.0));
        assert!(!contains(&c_path, 1.0, 1.0));
        assert!(!contains(&c_path, 30.0, 10.0));
        assert!(!contains(&c_path, 99.0, 19.0));

        let at_01 = <dyn ShapeBorder>::lerp(Some(&r), Some(&c), 0.1)
            .unwrap()
            .get_outer_path(rect, None);
        assert!(contains(&at_01, 30.0, 10.0));
        let at_09 = <dyn ShapeBorder>::lerp(Some(&r), Some(&c), 0.9)
            .unwrap()
            .get_outer_path(rect, None);
        assert!(contains(&at_09, 50.0, 10.0));
        assert!(!contains(&at_09, 30.0, 10.0));
    }

    #[test]
    fn rounded_superellipse_defaults_copy_with_scale_lerp() {
        let border = RoundedSuperellipseBorder::default();
        assert_eq!(border.side, BorderSide::NONE);
        assert_eq!(border.border_radius, BorderRadius::ZERO.into());
        assert_eq!(border, border.copy_with(None, None));

        let c10 = RoundedSuperellipseBorder::new(
            side_width(10.0),
            Some(BorderRadius::all(Radius::circular(100.0)).into()),
        );
        let c15 = RoundedSuperellipseBorder::new(
            side_width(15.0),
            Some(BorderRadius::all(Radius::circular(150.0)).into()),
        );
        let c20 = RoundedSuperellipseBorder::new(
            side_width(20.0),
            Some(BorderRadius::all(Radius::circular(200.0)).into()),
        );
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
}
