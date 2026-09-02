//! Flutter counterpart: `painting/beveled_rectangle_border.dart`.

use std::any::Any;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

use reveal_embedder::{Canvas, FillRule, Path, PathBuilder, RRect};
use reveal_embedder::{Matrix4, Offset, Rect};

use crate::basic_types::TextDirection;
use crate::border_radius::{BorderRadius, BorderRadiusGeometry};
use crate::borders::{
    BorderSide, BorderStyle, OutlinedBorder, ShapeBorder, outlined_border_dimensions,
};
use crate::edge_insets::EdgeInsetsGeometry;

/// A rectangular border with flattened or "beveled" corners.
///
/// The line segments that connect the rectangle's four sides will begin at
/// locations offset by the corresponding border radius, but not farther than
/// the side's center. If all the border radii exceed the sides' half
/// widths/heights the resulting shape is a diamond made by connecting the
/// centers of the sides.
#[derive(Clone, Copy, PartialEq)]
pub struct BeveledRectangleBorder {
    /// The border outline's color and weight.
    pub side: BorderSide,
    /// The radii for each corner.
    ///
    /// Each corner [`Radius`](reveal_embedder::Radius) defines the endpoints of
    /// a line segment that spans the corner. The endpoints are located in the
    /// same place as they would be for `RoundedRectangleBorder`, but they're
    /// connected by a straight line instead of an arc.
    ///
    /// Negative radius values are clamped to 0.0 by [`get_inner_path`](ShapeBorder::get_inner_path)
    /// and [`get_outer_path`](ShapeBorder::get_outer_path).
    pub border_radius: BorderRadiusGeometry,
}

impl BeveledRectangleBorder {
    /// Creates a border like a `RoundedRectangleBorder` except that the corners
    /// are joined by straight lines instead of arcs.
    pub fn new(side: BorderSide, border_radius: BorderRadiusGeometry) -> BeveledRectangleBorder {
        BeveledRectangleBorder {
            side,
            border_radius,
        }
    }

    /// Creates a copy of this border, scaled by the factor `t`.
    pub fn scale(&self, t: f64) -> BeveledRectangleBorder {
        BeveledRectangleBorder::new(self.side.scale(t), self.border_radius * t)
    }

    /// Returns a copy of this BeveledRectangleBorder with the given fields
    /// replaced with the new values.
    pub fn copy_with(
        &self,
        side: Option<BorderSide>,
        border_radius: Option<BorderRadiusGeometry>,
    ) -> BeveledRectangleBorder {
        BeveledRectangleBorder::new(
            side.unwrap_or(self.side),
            border_radius.unwrap_or(self.border_radius),
        )
    }

    fn get_path(rrect: RRect) -> Arc<Path> {
        let center_left = Offset::new(rrect.left, rrect.center().dy());
        let center_right = Offset::new(rrect.right, rrect.center().dy());
        let center_top = Offset::new(rrect.center().dx(), rrect.top);
        let center_bottom = Offset::new(rrect.center().dx(), rrect.bottom);

        let tl_radius_x = rrect.tl_radius_x.max(0.0);
        let tl_radius_y = rrect.tl_radius_y.max(0.0);
        let tr_radius_x = rrect.tr_radius_x.max(0.0);
        let tr_radius_y = rrect.tr_radius_y.max(0.0);
        let bl_radius_x = rrect.bl_radius_x.max(0.0);
        let bl_radius_y = rrect.bl_radius_y.max(0.0);
        let br_radius_x = rrect.br_radius_x.max(0.0);
        let br_radius_y = rrect.br_radius_y.max(0.0);

        let vertices = [
            Offset::new(rrect.left, center_left.dy().min(rrect.top + tl_radius_y)),
            Offset::new(center_top.dx().min(rrect.left + tl_radius_x), rrect.top),
            Offset::new(center_top.dx().max(rrect.right - tr_radius_x), rrect.top),
            Offset::new(rrect.right, center_right.dy().min(rrect.top + tr_radius_y)),
            Offset::new(
                rrect.right,
                center_right.dy().max(rrect.bottom - br_radius_y),
            ),
            Offset::new(
                center_bottom.dx().max(rrect.right - br_radius_x),
                rrect.bottom,
            ),
            Offset::new(
                center_bottom.dx().min(rrect.left + bl_radius_x),
                rrect.bottom,
            ),
            Offset::new(rrect.left, center_left.dy().max(rrect.bottom - bl_radius_y)),
        ];

        let mut path = PathBuilder::new();
        path.move_to(vertices[0]);
        for vertex in &vertices[1..] {
            path.line_to(*vertex);
        }
        path.close();
        path.build()
    }
}

impl Default for BeveledRectangleBorder {
    fn default() -> BeveledRectangleBorder {
        BeveledRectangleBorder::new(BorderSide::NONE, BorderRadius::ZERO.into())
    }
}

impl ShapeBorder for BeveledRectangleBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        outlined_border_dimensions(self.side)
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(BeveledRectangleBorder::scale(self, t))
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<BeveledRectangleBorder>()) {
            Some(Box::new(BeveledRectangleBorder::new(
                BorderSide::lerp(a.side, self.side, t),
                BorderRadiusGeometry::lerp(Some(a.border_radius), Some(self.border_radius), t)
                    .unwrap(),
            )))
        } else if a.is_none() {
            Some(ShapeBorder::scale(self, t))
        } else {
            None
        }
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<BeveledRectangleBorder>()) {
            Some(Box::new(BeveledRectangleBorder::new(
                BorderSide::lerp(self.side, b.side, t),
                BorderRadiusGeometry::lerp(Some(self.border_radius), Some(b.border_radius), t)
                    .unwrap(),
            )))
        } else if b.is_none() {
            Some(ShapeBorder::scale(self, 1.0 - t))
        } else {
            None
        }
    }

    fn get_inner_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        Self::get_path(
            self.border_radius
                .resolve(text_direction)
                .to_rrect(rect)
                .deflate(self.side.stroke_inset()),
        )
    }

    fn get_outer_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        Self::get_path(self.border_radius.resolve(text_direction).to_rrect(rect))
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect, text_direction: Option<TextDirection>) {
        if rect.is_empty() {
            return;
        }
        match self.side.style {
            BorderStyle::None => {}
            BorderStyle::Solid => {
                let border_rect = self.border_radius.resolve(text_direction).to_rrect(rect);
                let adjusted_rect = border_rect.inflate(self.side.stroke_outset());
                let mut path = PathBuilder::new();
                path.append(&Self::get_path(adjusted_rect), &Matrix4::IDENTITY);
                path.append(
                    &self.get_inner_path(rect, text_direction),
                    &Matrix4::IDENTITY,
                );
                canvas.draw_path(&path.build(), FillRule::NonZero, &self.side.to_paint());
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
            .downcast_ref::<BeveledRectangleBorder>()
            .is_some_and(|other| other == self)
    }
}

impl OutlinedBorder for BeveledRectangleBorder {
    fn side(&self) -> BorderSide {
        self.side
    }

    fn copy_with(&self, side: Option<BorderSide>) -> Box<dyn OutlinedBorder> {
        Box::new(BeveledRectangleBorder::copy_with(self, side, None))
    }

    fn clone_outlined(&self) -> Box<dyn OutlinedBorder> {
        Box::new(*self)
    }
}

impl Debug for BeveledRectangleBorder {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "BeveledRectangleBorder({:?}, {:?})",
            self.side, self.border_radius
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reveal_embedder::{Color, Radius};

    use crate::border_radius::BorderRadiusDirectional;

    fn contains(path: &Arc<Path>, x: f64, y: f64) -> bool {
        path.contains(Offset::new(x, y).into(), FillRule::NonZero)
    }

    fn side_width(width: f64) -> BorderSide {
        BorderSide {
            width,
            ..BorderSide::default()
        }
    }

    #[test]
    fn defaults() {
        let border = BeveledRectangleBorder::default();
        assert_eq!(border.side, BorderSide::NONE);
        assert_eq!(border.border_radius, BorderRadius::ZERO.into());
    }

    #[test]
    fn copy_with_eq() {
        assert_eq!(
            BeveledRectangleBorder::default(),
            BeveledRectangleBorder::default().copy_with(None, None)
        );
        let side = BorderSide {
            width: 10.0,
            color: Color::new(0xFF123456),
            ..BorderSide::default()
        };
        let radius = BorderRadius::all(Radius::circular(16.0));
        let directional = BorderRadiusDirectional::all(Radius::circular(16.0));
        assert_eq!(
            BeveledRectangleBorder::default().copy_with(Some(side), Some(radius.into())),
            BeveledRectangleBorder::new(side, radius.into())
        );
        assert_eq!(
            BeveledRectangleBorder::default().copy_with(Some(side), Some(directional.into())),
            BeveledRectangleBorder::new(side, directional.into())
        );
    }

    #[test]
    fn scale_lerp_dimensions() {
        let c10 = BeveledRectangleBorder::new(
            side_width(10.0),
            BorderRadius::all(Radius::circular(100.0)).into(),
        );
        let c15 = BeveledRectangleBorder::new(
            side_width(15.0),
            BorderRadius::all(Radius::circular(150.0)).into(),
        );
        let c20 = BeveledRectangleBorder::new(
            side_width(20.0),
            BorderRadius::all(Radius::circular(200.0)).into(),
        );
        assert_eq!(c10.dimensions(), EdgeInsetsGeometry::all(10.0));
        assert_eq!(c10.scale(2.0), c20);
        assert_eq!(c20.scale(0.5), c10);
        assert!(
            <dyn ShapeBorder>::lerp(Some(&c10), Some(&c20), 0.0)
                .unwrap()
                .eq_shape(&c10)
        );
        assert!(
            <dyn ShapeBorder>::lerp(Some(&c10), Some(&c20), 0.5)
                .unwrap()
                .eq_shape(&c15)
        );
        assert!(
            <dyn ShapeBorder>::lerp(Some(&c10), Some(&c20), 1.0)
                .unwrap()
                .eq_shape(&c20)
        );
    }

    #[test]
    fn border_radius_zero() {
        let rect1 = Rect::from_ltrb(10.0, 20.0, 30.0, 40.0);
        let border = BeveledRectangleBorder::default();
        let outer = border.get_outer_path(rect1, None);
        let inner = border.get_inner_path(rect1, None);
        assert!(contains(&outer, 10.0, 20.0));
        assert!(contains(&outer, 20.0, 30.0));
        assert!(!contains(&outer, 9.0, 19.0));
        assert!(!contains(&outer, 31.0, 41.0));
        assert!(contains(&inner, 10.0, 20.0));
        assert!(contains(&inner, 20.0, 30.0));

        let side = side_width(4.0);
        let with_side = BeveledRectangleBorder::new(side, BorderRadius::ZERO.into());
        let outer = with_side.get_outer_path(rect1, None);
        let inner = with_side.get_inner_path(rect1, None);
        assert!(contains(&outer, 10.0, 20.0));
        assert!(contains(&inner, 14.0, 24.0));
        assert!(contains(&inner, 16.0, 26.0));
        assert!(!contains(&inner, 9.0, 23.0));
        assert!(!contains(&inner, 27.0, 37.0));
    }

    #[test]
    fn non_zero_border_radius() {
        let rect = Rect::from_ltrb(10.0, 20.0, 30.0, 40.0);
        let border = BeveledRectangleBorder::new(
            BorderSide::NONE,
            BorderRadius::all(Radius::circular(5.0)).into(),
        );
        let path = border.get_outer_path(rect, None);
        assert!(contains(&path, 15.0, 25.0));
        assert!(contains(&path, 20.0, 30.0));
        assert!(!contains(&path, 10.0, 20.0));
        assert!(!contains(&path, 30.0, 40.0));
    }

    #[test]
    fn border_radius_directional() {
        let rect = Rect::from_ltrb(10.0, 20.0, 30.0, 40.0);
        let border = BeveledRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusDirectional::only(
                Radius::circular(5.0),
                Radius::ZERO,
                Radius::circular(5.0),
                Radius::ZERO,
            )
            .into(),
        );
        let ltr = border.get_outer_path(rect, Some(TextDirection::Ltr));
        assert!(contains(&ltr, 15.0, 25.0));
        assert!(contains(&ltr, 20.0, 30.0));
        assert!(!contains(&ltr, 10.0, 20.0));
        assert!(!contains(&ltr, 10.0, 40.0));
        let rtl = border.get_outer_path(rect, Some(TextDirection::Rtl));
        assert!(contains(&rtl, 25.0, 35.0));
        assert!(contains(&rtl, 25.0, 25.0));
        assert!(!contains(&rtl, 30.0, 20.0));
        assert!(!contains(&rtl, 30.0, 40.0));
    }

    #[test]
    fn stroke_align_dimensions() {
        let border_radius = BorderRadius::all(Radius::circular(10.0));
        let inside = BeveledRectangleBorder::new(side_width(10.0), border_radius.into());
        let center = BeveledRectangleBorder::new(
            BorderSide {
                width: 10.0,
                stroke_align: BorderSide::STROKE_ALIGN_CENTER,
                ..BorderSide::default()
            },
            border_radius.into(),
        );
        let outside = BeveledRectangleBorder::new(
            BorderSide {
                width: 10.0,
                stroke_align: BorderSide::STROKE_ALIGN_OUTSIDE,
                ..BorderSide::default()
            },
            border_radius.into(),
        );
        assert_eq!(inside.dimensions(), EdgeInsetsGeometry::all(10.0));
        assert_eq!(center.dimensions(), EdgeInsetsGeometry::all(5.0));
        assert_eq!(outside.dimensions(), EdgeInsetsGeometry::all(0.0));
    }
}
