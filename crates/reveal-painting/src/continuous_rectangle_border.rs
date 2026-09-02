//! Flutter counterpart: `painting/continuous_rectangle_border.dart`.

use std::any::Any;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

use reveal_embedder::{Canvas, FillRule, Path, PathBuilder, RRect};
use reveal_embedder::{Offset, Rect};

use crate::basic_types::TextDirection;
use crate::border_radius::{BorderRadius, BorderRadiusGeometry};
use crate::borders::{BorderSide, BorderStyle, OutlinedBorder, ShapeBorder};
use crate::edge_insets::{EdgeInsets, EdgeInsetsGeometry};

/// A rectangular border with smooth continuous transitions between the straight
/// sides and the rounded corners.
///
/// See also:
///
///  * [`RoundedRectangleBorder`](crate::RoundedRectangleBorder), which creates
///    rectangles with rounded corners, however its straight sides change into a
///    rounded corner with a circular radius in a step function instead of
///    gradually like the [`ContinuousRectangleBorder`].
#[derive(Clone, Copy, PartialEq)]
pub struct ContinuousRectangleBorder {
    /// The border outline's color and weight.
    pub side: BorderSide,
    /// The radius for each corner.
    ///
    /// Negative radius values are clamped to 0.0 by [`get_inner_path`](ShapeBorder::get_inner_path)
    /// and [`get_outer_path`](ShapeBorder::get_outer_path).
    pub border_radius: BorderRadiusGeometry,
}

impl ContinuousRectangleBorder {
    /// Creates a [`ContinuousRectangleBorder`].
    pub fn new(side: BorderSide, border_radius: BorderRadiusGeometry) -> ContinuousRectangleBorder {
        ContinuousRectangleBorder {
            side,
            border_radius,
        }
    }

    /// Creates a copy of this border, scaled by the factor `t`.
    pub fn scale(&self, t: f64) -> ContinuousRectangleBorder {
        ContinuousRectangleBorder::new(self.side.scale(t), self.border_radius * t)
    }

    /// Returns a copy of this ContinuousRectangleBorder with the given fields
    /// replaced with the new values.
    pub fn copy_with(
        &self,
        side: Option<BorderSide>,
        border_radius: Option<BorderRadiusGeometry>,
    ) -> ContinuousRectangleBorder {
        ContinuousRectangleBorder::new(
            side.unwrap_or(self.side),
            border_radius.unwrap_or(self.border_radius),
        )
    }

    fn clamp_to_shortest(rrect: RRect, value: f64) -> f64 {
        if value > rrect.shortest_side() {
            rrect.shortest_side()
        } else {
            value
        }
    }

    fn get_path(rrect: RRect) -> Arc<Path> {
        let left = rrect.left;
        let right = rrect.right;
        let top = rrect.top;
        let bottom = rrect.bottom;
        // Radii will be clamped to the value of the shortest side of rrect to
        // avoid strange tie-fighter shapes.
        let tl_radius_x = Self::clamp_to_shortest(rrect, rrect.tl_radius_x).max(0.0);
        let tl_radius_y = Self::clamp_to_shortest(rrect, rrect.tl_radius_y).max(0.0);
        let tr_radius_x = Self::clamp_to_shortest(rrect, rrect.tr_radius_x).max(0.0);
        let tr_radius_y = Self::clamp_to_shortest(rrect, rrect.tr_radius_y).max(0.0);
        let bl_radius_x = Self::clamp_to_shortest(rrect, rrect.bl_radius_x).max(0.0);
        let bl_radius_y = Self::clamp_to_shortest(rrect, rrect.bl_radius_y).max(0.0);
        let br_radius_x = Self::clamp_to_shortest(rrect, rrect.br_radius_x).max(0.0);
        let br_radius_y = Self::clamp_to_shortest(rrect, rrect.br_radius_y).max(0.0);

        let mut path = PathBuilder::new();
        path.move_to(Offset::new(left, top + tl_radius_x))
            .cubic_to(
                Offset::new(left, top),
                Offset::new(left, top),
                Offset::new(left + tl_radius_y, top),
            )
            .line_to(Offset::new(right - tr_radius_x, top))
            .cubic_to(
                Offset::new(right, top),
                Offset::new(right, top),
                Offset::new(right, top + tr_radius_y),
            )
            .line_to(Offset::new(right, bottom - br_radius_x))
            .cubic_to(
                Offset::new(right, bottom),
                Offset::new(right, bottom),
                Offset::new(right - br_radius_y, bottom),
            )
            .line_to(Offset::new(left + bl_radius_x, bottom))
            .cubic_to(
                Offset::new(left, bottom),
                Offset::new(left, bottom),
                Offset::new(left, bottom - bl_radius_y),
            )
            .close();
        path.build()
    }
}

impl Default for ContinuousRectangleBorder {
    fn default() -> ContinuousRectangleBorder {
        ContinuousRectangleBorder::new(BorderSide::NONE, BorderRadius::ZERO.into())
    }
}

impl ShapeBorder for ContinuousRectangleBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        EdgeInsets::all(self.side.width).into()
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(ContinuousRectangleBorder::scale(self, t))
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<ContinuousRectangleBorder>()) {
            Some(Box::new(ContinuousRectangleBorder::new(
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
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<ContinuousRectangleBorder>()) {
            Some(Box::new(ContinuousRectangleBorder::new(
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
                .deflate(self.side.width),
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
                canvas.draw_path(
                    &self.get_outer_path(rect, text_direction),
                    FillRule::NonZero,
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
            .downcast_ref::<ContinuousRectangleBorder>()
            .is_some_and(|other| other == self)
    }
}

impl OutlinedBorder for ContinuousRectangleBorder {
    fn side(&self) -> BorderSide {
        self.side
    }

    fn copy_with(&self, side: Option<BorderSide>) -> Box<dyn OutlinedBorder> {
        Box::new(ContinuousRectangleBorder::copy_with(self, side, None))
    }

    fn clone_outlined(&self) -> Box<dyn OutlinedBorder> {
        Box::new(*self)
    }
}

impl Debug for ContinuousRectangleBorder {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "ContinuousRectangleBorder({:?}, {:?})",
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
        let border = ContinuousRectangleBorder::default();
        assert_eq!(border.side, BorderSide::NONE);
        assert_eq!(border.border_radius, BorderRadius::ZERO.into());
    }

    #[test]
    fn copy_with_eq() {
        assert_eq!(
            ContinuousRectangleBorder::default(),
            ContinuousRectangleBorder::default().copy_with(None, None)
        );
        let side = BorderSide {
            width: 10.0,
            color: Color::new(0xFF123456),
            ..BorderSide::default()
        };
        let radius = BorderRadius::all(Radius::circular(16.0));
        let directional = BorderRadiusDirectional::all(Radius::circular(16.0));
        assert_eq!(
            ContinuousRectangleBorder::default().copy_with(Some(side), Some(radius.into())),
            ContinuousRectangleBorder::new(side, radius.into())
        );
        assert_eq!(
            ContinuousRectangleBorder::default().copy_with(Some(side), Some(directional.into())),
            ContinuousRectangleBorder::new(side, directional.into())
        );
    }

    #[test]
    fn scale_lerp_dimensions() {
        let c10 = ContinuousRectangleBorder::new(
            side_width(10.0),
            BorderRadius::all(Radius::circular(100.0)).into(),
        );
        let c15 = ContinuousRectangleBorder::new(
            side_width(15.0),
            BorderRadius::all(Radius::circular(150.0)).into(),
        );
        let c20 = ContinuousRectangleBorder::new(
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
        let border = ContinuousRectangleBorder::default();
        let outer = border.get_outer_path(rect1, None);
        assert!(contains(&outer, 10.0, 20.0));
        assert!(contains(&outer, 20.0, 30.0));
        assert!(!contains(&outer, 9.0, 19.0));
        assert!(!contains(&outer, 31.0, 41.0));

        let with_side = ContinuousRectangleBorder::new(side_width(4.0), BorderRadius::ZERO.into());
        let inner = with_side.get_inner_path(rect1, None);
        assert!(contains(&inner, 14.0, 24.0));
        assert!(contains(&inner, 16.0, 26.0));
        assert!(!contains(&inner, 9.0, 23.0));
        assert!(!contains(&inner, 27.0, 37.0));
    }

    #[test]
    fn non_zero_border_radius() {
        let rect = Rect::from_ltrb(10.0, 20.0, 30.0, 40.0);
        let border = ContinuousRectangleBorder::new(
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
        let border = ContinuousRectangleBorder::new(
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
}
