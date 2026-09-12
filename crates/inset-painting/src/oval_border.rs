//! Flutter counterpart: `painting/oval_border.dart`.

use std::any::Any;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

use inset_embedder::{Canvas, Paint, Path};
use inset_embedder::{Offset, Rect, clamp_double, lerp_double};

use crate::basic_types::TextDirection;
use crate::borders::{BorderSide, OutlinedBorder, ShapeBorder, outlined_border_dimensions};
use crate::circle_border::CircleBorder;
use crate::edge_insets::EdgeInsetsGeometry;

/// A border that fits an elliptical shape.
///
/// Typically used with `ShapeDecoration` to draw an oval. Instead of centering
/// the `Border` to a square, like [`CircleBorder`], it fills the available space,
/// such that it touches the edges of the box. There is no difference between
/// `CircleBorder` with eccentricity 1.0 and [`OvalBorder`]. [`OvalBorder`] works
/// as an alias for users to discover this feature.
#[derive(Clone, Copy, PartialEq)]
pub struct OvalBorder {
    /// The border outline's color and weight.
    pub side: BorderSide,
    /// See [`CircleBorder::eccentricity`]. Defaults to 1.0.
    pub eccentricity: f64,
}

impl OvalBorder {
    /// Create an oval border.
    pub fn new(side: BorderSide, eccentricity: f64) -> OvalBorder {
        OvalBorder {
            side,
            eccentricity: CircleBorder::new(side, eccentricity).eccentricity,
        }
    }

    /// Creates a copy of this border, scaled by the factor `t`.
    pub fn scale(&self, t: f64) -> OvalBorder {
        OvalBorder::new(self.side.scale(t), self.eccentricity)
    }

    /// Returns a copy of this [`OvalBorder`] with the given fields replaced
    /// with the new values.
    pub fn copy_with(&self, side: Option<BorderSide>, eccentricity: Option<f64>) -> OvalBorder {
        OvalBorder::new(
            side.unwrap_or(self.side),
            eccentricity.unwrap_or(self.eccentricity),
        )
    }

    fn as_circle(&self) -> CircleBorder {
        CircleBorder::new(self.side, self.eccentricity)
    }
}

/// Dart `border is CircleBorder`, including [`OvalBorder`] (`extends CircleBorder`).
pub(crate) fn as_circle_border(border: &dyn ShapeBorder) -> Option<CircleBorder> {
    if let Some(circle) = border.as_any().downcast_ref::<CircleBorder>() {
        Some(*circle)
    } else {
        border
            .as_any()
            .downcast_ref::<OvalBorder>()
            .map(|oval| oval.as_circle())
    }
}

impl Default for OvalBorder {
    fn default() -> OvalBorder {
        OvalBorder::new(BorderSide::NONE, 1.0)
    }
}

impl ShapeBorder for OvalBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        outlined_border_dimensions(self.side)
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(OvalBorder::scale(self, t))
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<OvalBorder>()) {
            Some(Box::new(OvalBorder::new(
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
            self.as_circle().lerp_from(a, t)
        }
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<OvalBorder>()) {
            Some(Box::new(OvalBorder::new(
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
            self.as_circle().lerp_to(b, t)
        }
    }

    fn get_inner_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        self.as_circle().get_inner_path(rect, text_direction)
    }

    fn get_outer_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        self.as_circle().get_outer_path(rect, text_direction)
    }

    fn hit_test(
        &self,
        rect: Rect,
        position: Offset,
        text_direction: Option<TextDirection>,
    ) -> bool {
        self.as_circle().hit_test(rect, position, text_direction)
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        text_direction: Option<TextDirection>,
    ) {
        self.as_circle()
            .paint_interior(canvas, rect, paint, text_direction);
    }

    fn prefer_paint_interior(&self) -> bool {
        true
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect, text_direction: Option<TextDirection>) {
        self.as_circle().paint(canvas, rect, text_direction);
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
            .downcast_ref::<OvalBorder>()
            .is_some_and(|other| other == self)
    }
}

impl OutlinedBorder for OvalBorder {
    fn side(&self) -> BorderSide {
        self.side
    }

    fn copy_with(&self, side: Option<BorderSide>) -> Box<dyn OutlinedBorder> {
        Box::new(OvalBorder::copy_with(self, side, None))
    }

    fn clone_outlined(&self) -> Box<dyn OutlinedBorder> {
        Box::new(*self)
    }
}

impl Debug for OvalBorder {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.eccentricity != 1.0 {
            write!(
                f,
                "OvalBorder({:?}, eccentricity: {})",
                self.side, self.eccentricity
            )
        } else {
            write!(f, "OvalBorder({:?})", self.side)
        }
    }
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
        assert_eq!(OvalBorder::default().side, BorderSide::NONE);
        assert_eq!(OvalBorder::default().eccentricity, 1.0);
    }

    #[test]
    fn copy_with_eq() {
        assert_eq!(
            OvalBorder::default(),
            OvalBorder::default().copy_with(None, None)
        );
        let side = BorderSide {
            width: 10.0,
            color: Color::new(0xFF123456),
            ..BorderSide::default()
        };
        assert_eq!(
            OvalBorder::default().copy_with(Some(side), None),
            OvalBorder::new(side, 1.0)
        );
    }

    #[test]
    fn scale_lerp_dimensions() {
        let c10 = OvalBorder::new(side_width(10.0), 1.0);
        let c15 = OvalBorder::new(side_width(15.0), 1.0);
        let c20 = OvalBorder::new(side_width(20.0), 1.0);
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
    fn inner_and_outer_paths() {
        let c10 = OvalBorder::new(side_width(10.0), 1.0);
        let inner = c10.get_inner_path(Rect::from_ltwh(0.0, 0.0, 100.0, 40.0), None);
        assert!(contains(&inner, 12.0, 19.0));
        assert!(contains(&inner, 50.0, 10.0));
        assert!(contains(&inner, 88.0, 19.0));
        assert!(contains(&inner, 50.0, 29.0));
        assert!(!contains(&inner, 17.0, 26.0));
        assert!(!contains(&inner, 15.0, 15.0));
        assert!(!contains(&inner, 74.0, 10.0));
        assert!(!contains(&inner, 76.0, 28.0));

        let outer = c10.get_outer_path(Rect::from_ltwh(0.0, 0.0, 100.0, 20.0), None);
        assert!(contains(&outer, 2.0, 9.0));
        assert!(contains(&outer, 50.0, 0.0));
        assert!(contains(&outer, 98.0, 9.0));
        assert!(contains(&outer, 50.0, 19.0));
        assert!(!contains(&outer, 7.0, 16.0));
        assert!(!contains(&outer, 10.0, 2.0));
        assert!(!contains(&outer, 84.0, 1.0));
        assert!(!contains(&outer, 86.0, 18.0));
    }
}
