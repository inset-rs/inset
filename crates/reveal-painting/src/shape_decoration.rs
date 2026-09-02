//! Flutter counterpart: `painting/shape_decoration.dart`.

use std::any::Any;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

use reveal_embedder::{BlurStyle, Canvas, ClipOp, Color, FillRule, Matrix4, Offset, Paint};
use reveal_embedder::{Path, PathBuilder, Rect, Size};

use crate::basic_types::TextDirection;
use crate::borders::{BorderSide, BorderStyle, ShapeBorder};
use crate::box_border::{Border, BoxShape};
use crate::box_decoration::BoxDecoration;
use crate::box_shadow::BoxShadow;
use crate::circle_border::CircleBorder;
use crate::debug::debug_disable_shadows;
use crate::decoration::{BoxPainter, Decoration};
use crate::edge_insets::EdgeInsetsGeometry;
use crate::image_provider::ImageConfiguration;
use crate::rounded_rectangle_border::RoundedRectangleBorder;

/// An immutable description of how to paint an arbitrary shape.
///
/// [`ShapeDecoration`] draws a [`ShapeBorder`], optionally filling it with a
/// [`color`](Self::color) and optionally casting a shadow.
///
/// [`image`](https://api.flutter.dev/flutter/painting/ShapeDecoration/image.html)
/// and [`gradient`](https://api.flutter.dev/flutter/painting/ShapeDecoration/gradient.html)
/// are deferred (see PORTING.md).
pub struct ShapeDecoration {
    /// The color to fill in the background of the shape.
    ///
    /// The color is under the image (image paint is deferred).
    pub color: Option<Color>,
    /// A list of shadows cast by the [`shape`](Self::shape).
    pub shadows: Option<Vec<BoxShadow>>,
    /// The shape to fill the [`color`](Self::color) into and to cast as the
    /// [`shadows`](Self::shadows).
    ///
    /// Shapes can be stacked (using [`plus`](dyn ShapeBorder::plus)). The color
    /// is drawn into the inner-most shape specified.
    ///
    /// The [`shape`](Self::shape) property specifies the outline (border) of the
    /// decoration.
    pub shape: Box<dyn ShapeBorder>,
}

impl ShapeDecoration {
    /// Creates a shape decoration.
    ///
    /// `shape` is required. Chain setters for the fields Dart's constructor
    /// takes as named arguments: `ShapeDecoration::new(shape).color(c)`.
    ///
    /// * If [`color`](Self::color) is None, this decoration does not paint a
    ///   background color.
    /// * If [`shadows`](Self::shadows) is None, this decoration does not paint a
    ///   shadow.
    pub fn new(shape: impl Into<Box<dyn ShapeBorder>>) -> ShapeDecoration {
        ShapeDecoration {
            color: None,
            shadows: None,
            shape: shape.into(),
        }
    }

    /// The color to fill in the background of the shape.
    pub fn color(mut self, color: Color) -> ShapeDecoration {
        self.color = Some(color);
        self
    }

    /// A list of shadows cast by the [`shape`](Self::shape).
    pub fn shadows(mut self, shadows: Vec<BoxShadow>) -> ShapeDecoration {
        self.shadows = Some(shadows);
        self
    }

    /// The shape to fill and to cast as the shadows.
    pub fn shape(mut self, shape: impl Into<Box<dyn ShapeBorder>>) -> ShapeDecoration {
        self.shape = shape.into();
        self
    }

    /// Creates a shape decoration configured to match a [`BoxDecoration`].
    pub fn from_box_decoration(source: &BoxDecoration) -> ShapeDecoration {
        let shape: Box<dyn ShapeBorder> = match source.shape {
            BoxShape::Circle => {
                if let Some(border) = source.border.as_ref() {
                    debug_assert!(
                        border.is_uniform(),
                        "A circular border cannot have non-uniform borders."
                    );
                    Box::new(CircleBorder::new(border.top(), 0.0))
                } else {
                    Box::new(CircleBorder::default())
                }
            }
            BoxShape::Rectangle => {
                if let Some(border_radius) = source.border_radius {
                    debug_assert!(
                        source
                            .border
                            .as_ref()
                            .is_none_or(|border| border.is_uniform()),
                        "A borderRadius cannot be applied to a non-uniform border."
                    );
                    Box::new(RoundedRectangleBorder::new(
                        source
                            .border
                            .as_ref()
                            .map(|border| border.top())
                            .unwrap_or(BorderSide::NONE),
                        border_radius,
                    ))
                } else if let Some(border) = source.border.as_ref() {
                    border.clone_box()
                } else {
                    Box::new(Border::default())
                }
            }
        };
        ShapeDecoration {
            color: source.color,
            shadows: source.box_shadow.clone(),
            shape,
        }
    }

    fn from_fields(
        color: Option<Color>,
        shadows: Option<Vec<BoxShadow>>,
        shape: Box<dyn ShapeBorder>,
    ) -> ShapeDecoration {
        ShapeDecoration {
            color,
            shadows,
            shape,
        }
    }

    /// Creates a copy of this object. Chain setters to replace fields
    /// (`decoration.copy_with().color(c)`).
    pub fn copy_with(&self) -> ShapeDecoration {
        self.clone()
    }

    /// Linearly interpolate between two shapes.
    ///
    /// Interpolates each parameter of the decoration separately.
    ///
    /// If both values are None, this returns None. Otherwise, it returns a
    /// non-null value, with null arguments treated like a [`ShapeDecoration`]
    /// whose optional fields are all None.
    pub fn lerp(
        a: Option<&ShapeDecoration>,
        b: Option<&ShapeDecoration>,
        t: f64,
    ) -> Option<ShapeDecoration> {
        if a.is_none() && b.is_none() {
            return None;
        }
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return Some(a.clone());
        }
        if let (Some(a), Some(b)) = (a, b) {
            if t == 0.0 {
                return Some(a.clone());
            }
            if t == 1.0 {
                return Some(b.clone());
            }
        }
        Some(ShapeDecoration::from_fields(
            Color::lerp(a.and_then(|a| a.color), b.and_then(|b| b.color), t),
            BoxShadow::lerp_list(
                a.and_then(|a| a.shadows.as_deref()),
                b.and_then(|b| b.shadows.as_deref()),
                t,
            ),
            <dyn ShapeBorder>::lerp(a.map(|a| a.shape.as_ref()), b.map(|b| b.shape.as_ref()), t)
                .expect("ShapeBorder.lerp of a ShapeDecoration shape"),
        ))
    }
}

impl Clone for ShapeDecoration {
    fn clone(&self) -> ShapeDecoration {
        ShapeDecoration {
            color: self.color,
            shadows: self.shadows.clone(),
            shape: self.shape.clone_box(),
        }
    }
}

impl PartialEq for ShapeDecoration {
    fn eq(&self, other: &ShapeDecoration) -> bool {
        self.color == other.color
            && self.shadows == other.shadows
            && self.shape.eq_shape(other.shape.as_ref())
    }
}

impl Decoration for ShapeDecoration {
    fn padding(&self) -> EdgeInsetsGeometry {
        self.shape.dimensions()
    }

    fn is_complex(&self) -> bool {
        self.shadows.is_some()
    }

    fn lerp_from(&self, a: Option<&dyn Decoration>, t: f64) -> Option<Box<dyn Decoration>> {
        match a {
            Some(a) if a.as_any().downcast_ref::<BoxDecoration>().is_some() => {
                let a = a.as_any().downcast_ref::<BoxDecoration>().unwrap();
                ShapeDecoration::lerp(
                    Some(&ShapeDecoration::from_box_decoration(a)),
                    Some(self),
                    t,
                )
                .map(|decoration| Box::new(decoration) as Box<dyn Decoration>)
            }
            Some(a) if a.as_any().downcast_ref::<ShapeDecoration>().is_some() => {
                let a = a.as_any().downcast_ref::<ShapeDecoration>().unwrap();
                ShapeDecoration::lerp(Some(a), Some(self), t)
                    .map(|decoration| Box::new(decoration) as Box<dyn Decoration>)
            }
            None => ShapeDecoration::lerp(None, Some(self), t)
                .map(|decoration| Box::new(decoration) as Box<dyn Decoration>),
            Some(_) => None,
        }
    }

    fn lerp_to(&self, b: Option<&dyn Decoration>, t: f64) -> Option<Box<dyn Decoration>> {
        match b {
            Some(b) if b.as_any().downcast_ref::<BoxDecoration>().is_some() => {
                let b = b.as_any().downcast_ref::<BoxDecoration>().unwrap();
                ShapeDecoration::lerp(
                    Some(self),
                    Some(&ShapeDecoration::from_box_decoration(b)),
                    t,
                )
                .map(|decoration| Box::new(decoration) as Box<dyn Decoration>)
            }
            Some(b) if b.as_any().downcast_ref::<ShapeDecoration>().is_some() => {
                let b = b.as_any().downcast_ref::<ShapeDecoration>().unwrap();
                ShapeDecoration::lerp(Some(self), Some(b), t)
                    .map(|decoration| Box::new(decoration) as Box<dyn Decoration>)
            }
            None => ShapeDecoration::lerp(Some(self), None, t)
                .map(|decoration| Box::new(decoration) as Box<dyn Decoration>),
            Some(_) => None,
        }
    }

    fn hit_test(
        &self,
        size: Size,
        position: Offset,
        text_direction: Option<TextDirection>,
    ) -> bool {
        self.shape
            .hit_test(Offset::ZERO & size, position, text_direction)
    }

    fn create_box_painter(&self, on_changed: Option<Box<dyn Fn()>>) -> Box<dyn BoxPainter> {
        Box::new(ShapeDecorationPainter::new(self.clone(), on_changed))
    }

    fn get_clip_path(&self, rect: Rect, text_direction: TextDirection) -> Arc<Path> {
        self.shape.get_outer_path(rect, Some(text_direction))
    }

    fn clone_box(&self) -> Box<dyn Decoration> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_decoration(&self, other: &dyn Decoration) -> bool {
        other
            .as_any()
            .downcast_ref::<ShapeDecoration>()
            .is_some_and(|other| other == self)
    }
}

impl Debug for ShapeDecoration {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("ShapeDecoration")
            .field("color", &self.color)
            .field("shadows", &self.shadows)
            .field("shape", &self.shape)
            .finish()
    }
}

/// An object that paints a [`ShapeDecoration`] into a canvas.
struct ShapeDecorationPainter {
    decoration: ShapeDecoration,
    on_changed: Option<Box<dyn Fn()>>,
    last_rect: Option<Rect>,
    last_text_direction: Option<TextDirection>,
    outer_path: Option<Arc<Path>>,
    interior_paint: Option<Paint>,
    shadow_count: Option<usize>,
    shadow_bounds: Vec<Rect>,
    shadow_paths: Vec<Arc<Path>>,
    shadow_paints: Vec<Paint>,
}

impl ShapeDecorationPainter {
    fn new(
        decoration: ShapeDecoration,
        on_changed: Option<Box<dyn Fn()>>,
    ) -> ShapeDecorationPainter {
        ShapeDecorationPainter {
            decoration,
            on_changed,
            last_rect: None,
            last_text_direction: None,
            outer_path: None,
            interior_paint: None,
            shadow_count: None,
            shadow_bounds: Vec::new(),
            shadow_paths: Vec::new(),
            shadow_paints: Vec::new(),
        }
    }

    fn precache(&mut self, rect: Rect, text_direction: Option<TextDirection>) {
        if self.last_rect == Some(rect) && self.last_text_direction == text_direction {
            return;
        }

        if self.interior_paint.is_none() && self.decoration.color.is_some() {
            let mut paint = Paint::default();
            if let Some(color) = self.decoration.color {
                paint.color = color.into();
            }
            self.interior_paint = Some(paint);
        }
        if let Some(shadows) = self.decoration.shadows.as_ref() {
            if self.shadow_count.is_none() {
                self.shadow_count = Some(shadows.len());
                self.shadow_paints = shadows.iter().map(BoxShadow::to_paint).collect();
            }
            if self.decoration.shape.prefer_paint_interior() {
                self.shadow_bounds = shadows
                    .iter()
                    .map(|shadow| rect.shift(shadow.offset).inflate(shadow.spread_radius))
                    .collect();
            } else {
                self.shadow_paths = shadows
                    .iter()
                    .map(|shadow| {
                        self.decoration.shape.get_outer_path(
                            rect.shift(shadow.offset).inflate(shadow.spread_radius),
                            text_direction,
                        )
                    })
                    .collect();
            }
        }
        if !self.decoration.shape.prefer_paint_interior()
            && (self.interior_paint.is_some() || self.shadow_count.is_some())
        {
            self.outer_path = Some(self.decoration.shape.get_outer_path(rect, text_direction));
        }

        self.last_rect = Some(rect);
        self.last_text_direction = text_direction;
    }

    fn debug_handle_disabled_shadow_start(
        canvas: &mut Canvas,
        box_shadow: BoxShadow,
        path: &Arc<Path>,
    ) {
        if debug_disable_shadows() && box_shadow.blur_style == BlurStyle::Outer {
            canvas.save();
            let mut clip_path = PathBuilder::new();
            clip_path.rect(Rect::LARGEST.into());
            clip_path.append(path, &Matrix4::IDENTITY);
            canvas.clip_path(&clip_path.build(), FillRule::EvenOdd, ClipOp::Intersect);
        }
    }

    fn debug_handle_disabled_shadow_end(canvas: &mut Canvas, box_shadow: BoxShadow) {
        if debug_disable_shadows() && box_shadow.blur_style == BlurStyle::Outer {
            canvas.restore();
        }
    }

    fn paint_shadows(&self, canvas: &mut Canvas, text_direction: Option<TextDirection>) {
        let Some(shadow_count) = self.shadow_count else {
            return;
        };
        if self.decoration.shape.prefer_paint_interior() {
            for index in 0..shadow_count {
                if cfg!(debug_assertions) {
                    Self::debug_handle_disabled_shadow_start(
                        canvas,
                        self.decoration.shadows.as_ref().unwrap()[index],
                        &self
                            .decoration
                            .shape
                            .get_outer_path(self.shadow_bounds[index], text_direction),
                    );
                }
                self.decoration.shape.paint_interior(
                    canvas,
                    self.shadow_bounds[index],
                    &self.shadow_paints[index],
                    text_direction,
                );
                if cfg!(debug_assertions) {
                    Self::debug_handle_disabled_shadow_end(
                        canvas,
                        self.decoration.shadows.as_ref().unwrap()[index],
                    );
                }
            }
        } else {
            for index in 0..shadow_count {
                if cfg!(debug_assertions) {
                    Self::debug_handle_disabled_shadow_start(
                        canvas,
                        self.decoration.shadows.as_ref().unwrap()[index],
                        &self.shadow_paths[index],
                    );
                }
                canvas.draw_path(
                    &self.shadow_paths[index],
                    FillRule::NonZero,
                    &self.shadow_paints[index],
                );
                if cfg!(debug_assertions) {
                    Self::debug_handle_disabled_shadow_end(
                        canvas,
                        self.decoration.shadows.as_ref().unwrap()[index],
                    );
                }
            }
        }
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        text_direction: Option<TextDirection>,
    ) {
        let Some(interior_paint) = self.interior_paint.as_ref() else {
            return;
        };
        if self.decoration.shape.prefer_paint_interior() {
            let adjusted_rect = self.adjusted_rect_on_outlined_border(rect);
            self.decoration.shape.paint_interior(
                canvas,
                adjusted_rect,
                interior_paint,
                text_direction,
            );
        } else {
            canvas.draw_path(
                self.outer_path.as_ref().unwrap(),
                FillRule::NonZero,
                interior_paint,
            );
        }
    }

    fn adjusted_rect_on_outlined_border(&self, rect: Rect) -> Rect {
        if self.decoration.color.is_some()
            && let Some(outlined) = self.decoration.shape.as_outlined_border()
        {
            let side = outlined.side();
            if (side.color.a * 255.0).round() == 255.0 && side.style == BorderStyle::Solid {
                return rect.deflate(side.stroke_inset() / 2.0);
            }
        }
        rect
    }
}

impl BoxPainter for ShapeDecorationPainter {
    fn paint(&mut self, canvas: &mut Canvas, offset: Offset, configuration: &ImageConfiguration) {
        debug_assert!(
            configuration.size.is_some(),
            "The ImageConfiguration object passed as the third argument must, at a minimum, have a non-null Size."
        );
        let rect = offset & configuration.size.unwrap();
        let text_direction = configuration.text_direction;
        self.precache(rect, text_direction);
        self.paint_shadows(canvas, text_direction);
        self.paint_interior(canvas, rect, text_direction);
        self.decoration.shape.paint(canvas, rect, text_direction);
    }

    fn on_changed(&self) -> Option<&dyn Fn()> {
        self.on_changed.as_deref()
    }
}

impl Debug for ShapeDecorationPainter {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "BoxPainter for {:?}", self.decoration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use reveal_embedder::{FillRule, Radius};

    use crate::border_radius::BorderRadiusDirectional;
    use crate::box_border::Border;
    use crate::circle_border::CircleBorder;
    use crate::oval_border::OvalBorder;
    use crate::rounded_rectangle_border::RoundedRectangleBorder;

    fn contains(path: &Arc<Path>, x: f64, y: f64) -> bool {
        path.contains(Offset::new(x, y).into(), FillRule::NonZero)
    }

    #[test]
    fn constructor_and_from_box_decoration() {
        let color_r = Color::new(0xFFFF0000);
        let color_g = Color::new(0xFF00FF00);
        assert_eq!(
            ShapeDecoration::new(Border::default()),
            ShapeDecoration::new(Border::default())
        );
        assert_eq!(
            ShapeDecoration::from_box_decoration(&BoxDecoration::new().shape(BoxShape::Circle)),
            ShapeDecoration::new(CircleBorder::default())
        );
        assert_eq!(
            ShapeDecoration::from_box_decoration(
                &BoxDecoration::new()
                    .border_radius(BorderRadiusDirectional::all(Radius::circular(100.0))),
            ),
            ShapeDecoration::new(RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusDirectional::all(Radius::circular(100.0)).into(),
            ))
        );
        assert_eq!(
            ShapeDecoration::from_box_decoration(
                &BoxDecoration::new()
                    .shape(BoxShape::Circle)
                    .border(Border::all(
                        color_g,
                        1.0,
                        BorderStyle::Solid,
                        BorderSide::STROKE_ALIGN_INSIDE
                    ),),
            ),
            ShapeDecoration::new(CircleBorder::new(
                BorderSide {
                    color: color_g,
                    ..BorderSide::default()
                },
                0.0,
            ))
        );
        assert_eq!(
            ShapeDecoration::from_box_decoration(&BoxDecoration::new().border(Border::all(
                color_r,
                1.0,
                BorderStyle::Solid,
                BorderSide::STROKE_ALIGN_INSIDE,
            )),),
            ShapeDecoration::new(Border::all(
                color_r,
                1.0,
                BorderStyle::Solid,
                BorderSide::STROKE_ALIGN_INSIDE,
            ))
        );
        assert_eq!(
            ShapeDecoration::from_box_decoration(&BoxDecoration::new().border(
                crate::box_border::BorderDirectional {
                    start: BorderSide::default(),
                    ..crate::box_border::BorderDirectional::default()
                }
            )),
            ShapeDecoration::new(crate::box_border::BorderDirectional {
                start: BorderSide::default(),
                ..crate::box_border::BorderDirectional::default()
            })
        );
    }

    #[test]
    fn lerp_identical_a_b() {
        assert!(ShapeDecoration::lerp(None, None, 0.0).is_none());
        let shape = ShapeDecoration::new(CircleBorder::default());
        assert_eq!(
            ShapeDecoration::lerp(Some(&shape), Some(&shape), 0.5).unwrap(),
            shape
        );
    }

    #[test]
    fn lerp_null_a_b() {
        let a: &dyn Decoration = &ShapeDecoration::new(CircleBorder::default());
        let b: &dyn Decoration = &ShapeDecoration::new(RoundedRectangleBorder::default());
        assert!(
            <dyn Decoration>::lerp(Some(a), None, 0.0)
                .unwrap()
                .eq_decoration(a)
        );
        assert!(
            <dyn Decoration>::lerp(None, Some(b), 0.0)
                .unwrap()
                .eq_decoration(b)
        );
        assert!(<dyn Decoration>::lerp(None, None, 0.0).is_none());
    }

    #[test]
    fn lerp_and_hit_test() {
        let a: &dyn Decoration = &ShapeDecoration::new(CircleBorder::default());
        let b: &dyn Decoration = &ShapeDecoration::new(RoundedRectangleBorder::default());
        let c: &dyn Decoration = &ShapeDecoration::new(OvalBorder::default());
        assert!(
            <dyn Decoration>::lerp(Some(a), Some(b), 0.0)
                .unwrap()
                .eq_decoration(a)
        );
        assert!(
            <dyn Decoration>::lerp(Some(a), Some(b), 1.0)
                .unwrap()
                .eq_decoration(b)
        );
        assert!(
            <dyn Decoration>::lerp(Some(a), Some(c), 0.0)
                .unwrap()
                .eq_decoration(a)
        );
        assert!(
            <dyn Decoration>::lerp(Some(a), Some(c), 1.0)
                .unwrap()
                .eq_decoration(c)
        );
        assert!(
            <dyn Decoration>::lerp(Some(b), Some(c), 0.0)
                .unwrap()
                .eq_decoration(b)
        );
        assert!(
            <dyn Decoration>::lerp(Some(b), Some(c), 1.0)
                .unwrap()
                .eq_decoration(c)
        );
        let size = Size::new(200.0, 100.0);
        assert!(!a.hit_test(size, Offset::new(20.0, 50.0), None));
        assert!(!c.hit_test(size, Offset::new(50.0, 5.0), None));
        assert!(!c.hit_test(size, Offset::new(5.0, 30.0), None));
        assert!(
            !<dyn Decoration>::lerp(Some(a), Some(b), 0.1)
                .unwrap()
                .hit_test(size, Offset::new(20.0, 50.0), None)
        );
        assert!(
            !<dyn Decoration>::lerp(Some(a), Some(b), 0.5)
                .unwrap()
                .hit_test(size, Offset::new(20.0, 50.0), None)
        );
        assert!(
            <dyn Decoration>::lerp(Some(a), Some(b), 0.9)
                .unwrap()
                .hit_test(size, Offset::new(20.0, 50.0), None)
        );
        assert!(
            !<dyn Decoration>::lerp(Some(a), Some(c), 0.1)
                .unwrap()
                .hit_test(size, Offset::new(30.0, 50.0), None)
        );
        assert!(
            <dyn Decoration>::lerp(Some(a), Some(c), 0.5)
                .unwrap()
                .hit_test(size, Offset::new(30.0, 50.0), None)
        );
        assert!(
            <dyn Decoration>::lerp(Some(a), Some(c), 0.9)
                .unwrap()
                .hit_test(size, Offset::new(30.0, 50.0), None)
        );
        assert!(
            <dyn Decoration>::lerp(Some(b), Some(c), 0.1)
                .unwrap()
                .hit_test(size, Offset::new(45.0, 10.0), None)
        );
        assert!(
            <dyn Decoration>::lerp(Some(b), Some(c), 0.5)
                .unwrap()
                .hit_test(size, Offset::new(30.0, 10.0), None)
        );
        assert!(
            <dyn Decoration>::lerp(Some(b), Some(c), 0.9)
                .unwrap()
                .hit_test(size, Offset::new(10.0, 30.0), None)
        );
        assert!(b.hit_test(size, Offset::new(20.0, 50.0), None));
    }

    #[test]
    fn get_clip_path() {
        let decoration = ShapeDecoration::new(CircleBorder::default());
        let rect = Rect::from_ltwh(0.0, 0.0, 100.0, 20.0);
        let clip_path = decoration.get_clip_path(rect, TextDirection::Ltr);
        assert!(contains(&clip_path, 50.0, 10.0));
        assert!(!contains(&clip_path, 1.0, 1.0));
        assert!(!contains(&clip_path, 30.0, 10.0));
        assert!(!contains(&clip_path, 99.0, 19.0));
    }

    #[test]
    fn get_clip_path_for_oval() {
        let decoration = ShapeDecoration::new(OvalBorder::default());
        let rect = Rect::from_ltwh(0.0, 0.0, 100.0, 50.0);
        let clip_path = decoration.get_clip_path(rect, TextDirection::Ltr);
        assert!(contains(&clip_path, 50.0, 10.0));
        assert!(!contains(&clip_path, 1.0, 1.0));
        assert!(!contains(&clip_path, 15.0, 1.0));
        assert!(!contains(&clip_path, 99.0, 19.0));
    }
}
