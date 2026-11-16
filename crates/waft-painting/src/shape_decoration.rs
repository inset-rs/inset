//! Flutter counterpart: `painting/shape_decoration.dart`.

use std::any::Any;
use std::fmt::{Debug, Formatter, Result as FmtResult};

use waft_embedder::{BlurStyle, Canvas, ClipOp, FillRule, Matrix4, Offset, Paint};
use waft_embedder::{Path, PathBuilder, Rect, Size};
use waft_foundation::{App, VoidCallback};

use crate::basic_types::TextDirection;
use crate::borders::{BorderSide, BorderStyle, ShapeBorderRef};
use crate::box_border::{Border, BoxShape};
use crate::box_decoration::BoxDecoration;
use crate::box_shadow::BoxShadow;
use crate::circle_border::CircleBorder;
use crate::colors::AnyColor;
use crate::debug::debug_disable_shadows;
use crate::decoration::{BoxPainter, Decoration, DecorationRef};
use crate::edge_insets::EdgeInsetsGeometry;
use crate::gradient::GradientRef;
use crate::image_provider::ImageConfiguration;
use crate::rounded_rectangle_border::RoundedRectangleBorder;

/// An immutable description of how to paint an arbitrary shape.
///
/// [`ShapeDecoration`] draws a [`ShapeBorder`], optionally filling it with a
/// [`color`](Self::color) or a [`gradient`](Self::gradient) and optionally casting a shadow.
///
/// [`image`](https://api.flutter.dev/flutter/painting/ShapeDecoration/image.html)
/// is deferred (see PORTING.md).
#[derive(Clone)]
pub struct ShapeDecoration {
    /// The color to fill in the background of the shape.
    ///
    /// The color is under the image (image paint is deferred).
    ///
    /// If a [`gradient`](Self::gradient) is specified, [`color`](Self::color) must be None.
    pub color: Option<AnyColor>,
    /// A gradient to use when filling the shape.
    ///
    /// The gradient is under the image (image paint is deferred).
    ///
    /// If a [`color`](Self::color) is specified, [`gradient`](Self::gradient) must be None.
    pub gradient: Option<GradientRef>,
    /// A list of shadows cast by the [`shape`](Self::shape).
    pub shadows: Option<Vec<BoxShadow>>,
    /// The shape to fill the [`color`](Self::color) and [`gradient`](Self::gradient) into and
    /// to cast as the [`shadows`](Self::shadows).
    ///
    /// Shapes can be stacked (using `plus`). The color and gradient
    /// are drawn into the inner-most shape specified.
    ///
    /// The [`shape`](Self::shape) property specifies the outline (border) of the
    /// decoration.
    pub shape: ShapeBorderRef,
}

impl ShapeDecoration {
    /// Creates a shape decoration.
    ///
    /// `shape` is required. Chain setters for the fields Dart's constructor
    /// takes as named arguments: `ShapeDecoration::new(shape).color(c)`.
    ///
    /// * If [`color`](Self::color) is None, this decoration does not paint a
    ///   background color.
    /// * If [`gradient`](Self::gradient) is None, this decoration does not paint
    ///   gradients.
    /// * If [`shadows`](Self::shadows) is None, this decoration does not paint a
    ///   shadow.
    ///
    /// The [`color`](Self::color) and [`gradient`](Self::gradient) properties are mutually
    /// exclusive, one (or both) of them must be None.
    pub fn new(shape: impl Into<ShapeBorderRef>) -> ShapeDecoration {
        ShapeDecoration {
            color: None,
            gradient: None,
            shadows: None,
            shape: shape.into(),
        }
    }

    /// The color to fill in the background of the shape.
    pub fn color(mut self, color: impl Into<AnyColor>) -> ShapeDecoration {
        self.color = Some(color.into());
        self.debug_assert_color_or_gradient();
        self
    }

    /// A gradient to use when filling the shape.
    pub fn gradient(mut self, gradient: impl Into<GradientRef>) -> ShapeDecoration {
        self.gradient = Some(gradient.into());
        self.debug_assert_color_or_gradient();
        self
    }

    /// Dart's constructor `assert(!(color != null && gradient != null))`; it can only fail on
    /// the second of the two setters, so it holds whichever order they come in.
    fn debug_assert_color_or_gradient(&self) {
        debug_assert!(!(self.color.is_some() && self.gradient.is_some()));
    }

    /// A list of shadows cast by the [`shape`](Self::shape).
    pub fn shadows(mut self, shadows: Vec<BoxShadow>) -> ShapeDecoration {
        self.shadows = Some(shadows);
        self
    }

    /// The shape to fill and to cast as the shadows.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> ShapeDecoration {
        self.shape = shape.into();
        self
    }

    /// Creates a shape decoration configured to match a [`BoxDecoration`].
    pub fn from_box_decoration(source: &BoxDecoration) -> ShapeDecoration {
        let shape: ShapeBorderRef = match source.shape {
            BoxShape::Circle => {
                if let Some(border) = source.border.as_ref() {
                    debug_assert!(
                        border.is_uniform(),
                        "A circular border cannot have non-uniform borders."
                    );
                    ShapeBorderRef::new(CircleBorder::new(border.top(), 0.0))
                } else {
                    ShapeBorderRef::new(CircleBorder::default())
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
                    ShapeBorderRef::new(RoundedRectangleBorder::new(
                        source
                            .border
                            .as_ref()
                            .map(|border| border.top())
                            .unwrap_or(BorderSide::NONE),
                        border_radius,
                    ))
                } else if let Some(border) = source.border.as_ref() {
                    ShapeBorderRef::from(border.clone())
                } else {
                    ShapeBorderRef::new(Border::default())
                }
            }
        };
        ShapeDecoration::from_fields(
            source.color.clone(),
            source.gradient.clone(),
            source.box_shadow.clone(),
            shape,
        )
    }

    fn from_fields(
        color: Option<AnyColor>,
        gradient: Option<GradientRef>,
        shadows: Option<Vec<BoxShadow>>,
        shape: ShapeBorderRef,
    ) -> ShapeDecoration {
        let decoration = ShapeDecoration {
            color,
            gradient,
            shadows,
            shape,
        };
        decoration.debug_assert_color_or_gradient();
        decoration
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
        // A ShapeDecoration's color and gradient are mutually exclusive (see the
        // assert in the constructor). When interpolating between a decoration that
        // uses a color and one that uses a gradient, represent the color as a
        // uniform-color gradient (built from the other side's gradient) so the two
        // can be interpolated smoothly as gradients, rather than producing both a
        // color and a gradient at the same time.
        // See https://github.com/flutter/flutter/issues/93953
        let mut a_gradient = a.and_then(|a| a.gradient.clone());
        let mut b_gradient = b.and_then(|b| b.gradient.clone());
        if let (None, Some(b_gradient), Some(a_color)) =
            (&a_gradient, &b_gradient, a.and_then(|a| a.color.as_ref()))
        {
            a_gradient = Some(b_gradient.from_color(a_color.color()));
        } else if let (None, Some(a_gradient), Some(b_color)) =
            (&b_gradient, &a_gradient, b.and_then(|b| b.color.as_ref()))
        {
            b_gradient = Some(a_gradient.from_color(b_color.color()));
        }
        let gradient = GradientRef::lerp(a_gradient.as_ref(), b_gradient.as_ref(), t);
        Some(ShapeDecoration::from_fields(
            // color and gradient are mutually exclusive, so only fall back to a color
            // when the interpolation did not produce a gradient.
            if gradient.is_none() {
                AnyColor::lerp(
                    a.and_then(|a| a.color.as_ref()),
                    b.and_then(|b| b.color.as_ref()),
                    t,
                )
                .map(AnyColor::from)
            } else {
                None
            },
            gradient,
            BoxShadow::lerp_list(
                a.and_then(|a| a.shadows.as_deref()),
                b.and_then(|b| b.shadows.as_deref()),
                t,
            ),
            ShapeBorderRef::lerp(a.map(|a| &a.shape), b.map(|b| &b.shape), t)
                .expect("ShapeBorder.lerp of a ShapeDecoration shape"),
        ))
    }
}

impl PartialEq for ShapeDecoration {
    fn eq(&self, other: &ShapeDecoration) -> bool {
        self.color == other.color
            && match (&self.gradient, &other.gradient) {
                (None, None) => true,
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
            && self.shadows == other.shadows
            && self.shape == other.shape
    }
}

impl Decoration for ShapeDecoration {
    fn padding(&self) -> EdgeInsetsGeometry {
        self.shape.dimensions()
    }

    fn is_complex(&self) -> bool {
        self.shadows.is_some()
    }

    fn lerp_from(&self, a: Option<&dyn Decoration>, t: f64) -> Option<DecorationRef> {
        match a {
            Some(a) if a.as_any().downcast_ref::<BoxDecoration>().is_some() => {
                let a = a.as_any().downcast_ref::<BoxDecoration>().unwrap();
                ShapeDecoration::lerp(
                    Some(&ShapeDecoration::from_box_decoration(a)),
                    Some(self),
                    t,
                )
                .map(DecorationRef::new)
            }
            Some(a) if a.as_any().downcast_ref::<ShapeDecoration>().is_some() => {
                let a = a.as_any().downcast_ref::<ShapeDecoration>().unwrap();
                ShapeDecoration::lerp(Some(a), Some(self), t).map(DecorationRef::new)
            }
            None => ShapeDecoration::lerp(None, Some(self), t).map(DecorationRef::new),
            Some(_) => None,
        }
    }

    fn lerp_to(&self, b: Option<&dyn Decoration>, t: f64) -> Option<DecorationRef> {
        match b {
            Some(b) if b.as_any().downcast_ref::<BoxDecoration>().is_some() => {
                let b = b.as_any().downcast_ref::<BoxDecoration>().unwrap();
                ShapeDecoration::lerp(
                    Some(self),
                    Some(&ShapeDecoration::from_box_decoration(b)),
                    t,
                )
                .map(DecorationRef::new)
            }
            Some(b) if b.as_any().downcast_ref::<ShapeDecoration>().is_some() => {
                let b = b.as_any().downcast_ref::<ShapeDecoration>().unwrap();
                ShapeDecoration::lerp(Some(self), Some(b), t).map(DecorationRef::new)
            }
            None => ShapeDecoration::lerp(Some(self), None, t).map(DecorationRef::new),
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

    fn create_box_painter(&self, on_changed: Option<VoidCallback>) -> Box<dyn BoxPainter> {
        Box::new(ShapeDecorationPainter::new(self.clone(), on_changed))
    }

    fn get_clip_path(&self, rect: Rect, text_direction: TextDirection) -> Path {
        self.shape.get_outer_path(rect, Some(text_direction))
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
            .field("gradient", &self.gradient)
            .field("shadows", &self.shadows)
            .field("shape", &self.shape)
            .finish()
    }
}

/// An object that paints a [`ShapeDecoration`] into a canvas.
struct ShapeDecorationPainter {
    decoration: ShapeDecoration,
    on_changed: Option<VoidCallback>,
    last_rect: Option<Rect>,
    last_text_direction: Option<TextDirection>,
    outer_path: Option<Path>,
    interior_paint: Option<Paint>,
    shadow_count: Option<usize>,
    shadow_bounds: Vec<Rect>,
    shadow_paths: Vec<Path>,
    shadow_paints: Vec<Paint>,
}

impl ShapeDecorationPainter {
    fn new(
        decoration: ShapeDecoration,
        on_changed: Option<VoidCallback>,
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

        // We reach here in two cases:
        //  - the very first time we paint, in which case everything except decoration is None
        //  - subsequent times, if the rect has changed, in which case we only need to update
        //    the features that depend on the actual rect.
        if self.interior_paint.is_none()
            && (self.decoration.color.is_some() || self.decoration.gradient.is_some())
        {
            let mut paint = Paint::default();
            if let Some(color) = &self.decoration.color {
                paint.color = color.color().into();
            }
            self.interior_paint = Some(paint);
        }
        if let Some(gradient) = &self.decoration.gradient {
            self.interior_paint.as_mut().unwrap().shader =
                Some(gradient.create_shader(rect, text_direction));
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

    fn debug_handle_disabled_shadow_start(canvas: &mut Canvas, box_shadow: BoxShadow, path: &Path) {
        if debug_disable_shadows() && box_shadow.blur_style == BlurStyle::Outer {
            canvas.save();
            let mut clip_path = PathBuilder::new();
            clip_path.rect(Rect::LARGEST.into());
            clip_path.append(path.as_valo(), &Matrix4::IDENTITY);
            canvas.clip_path(
                &clip_path.build(),
                FillRule::EvenOdd,
                ClipOp::Intersect,
                true,
            );
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
                let shadow_path = &self.shadow_paths[index];
                canvas.draw_path(
                    shadow_path.as_valo(),
                    shadow_path.fill_type,
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
            let outer_path = self.outer_path.as_ref().unwrap();
            canvas.draw_path(outer_path.as_valo(), outer_path.fill_type, interior_paint);
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

    fn on_changed(&self) -> Option<&dyn Fn(&mut App)> {
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
    use waft_embedder::Color;

    use waft_embedder::Radius;

    use crate::border_radius::BorderRadiusDirectional;
    use crate::box_border::Border;
    use crate::circle_border::CircleBorder;
    use crate::gradient::LinearGradient;
    use crate::oval_border::OvalBorder;
    use crate::rounded_rectangle_border::RoundedRectangleBorder;

    fn contains(path: &Path, x: f64, y: f64) -> bool {
        path.contains(Offset::new(x, y))
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
    #[should_panic]
    fn a_shape_decoration_cannot_have_both_a_colour_and_a_gradient() {
        let color_r = Color::new(0xffff0000);
        let color_g = Color::new(0xff00ff00);
        let _ = ShapeDecoration::new(Border::default())
            .color(color_r)
            .gradient(LinearGradient::new(vec![color_r, color_g]));
    }

    #[test]
    fn a_shape_decoration_lerps_smoothly_between_a_gradient_and_a_colour() {
        let color_r = Color::new(0xffff0000);
        let color_g = Color::new(0xff00ff00);
        let color_decoration = ShapeDecoration::new(CircleBorder::default()).color(color_r);
        let gradient_decoration = ShapeDecoration::new(RoundedRectangleBorder::default())
            .gradient(LinearGradient::new(vec![color_r, color_g]));

        // The end points are returned unchanged.
        assert_eq!(
            ShapeDecoration::lerp(Some(&color_decoration), Some(&gradient_decoration), 0.0),
            Some(color_decoration.clone())
        );
        assert_eq!(
            ShapeDecoration::lerp(Some(&color_decoration), Some(&gradient_decoration), 1.0),
            Some(gradient_decoration.clone())
        );

        // In between, the color is represented as a uniform-color gradient, so the
        // result is always a gradient (and never both a color and a gradient, which
        // would otherwise throw the constructor's assertion).
        let is_linear = |decoration: &ShapeDecoration| {
            decoration
                .gradient
                .as_ref()
                .is_some_and(|gradient| gradient.as_any().is::<LinearGradient>())
        };
        for t in [0.1, 0.25, 0.49, 0.5, 0.51, 0.75, 0.9] {
            let forward =
                ShapeDecoration::lerp(Some(&color_decoration), Some(&gradient_decoration), t)
                    .unwrap();
            assert!(forward.color.is_none());
            assert!(is_linear(&forward), "{t}: {forward:?}");

            // The reverse direction (gradient -> color) behaves the same way.
            let reverse =
                ShapeDecoration::lerp(Some(&gradient_decoration), Some(&color_decoration), t)
                    .unwrap();
            assert!(reverse.color.is_none());
            assert!(is_linear(&reverse), "{t}: {reverse:?}");
        }

        // Close to the color end, the interpolated gradient is (almost) the uniform
        // start color.
        let near_color =
            ShapeDecoration::lerp(Some(&color_decoration), Some(&gradient_decoration), 0.001)
                .unwrap();
        let gradient_near_color = near_color.gradient.as_ref().unwrap().colors();
        assert_eq!(gradient_near_color.len(), 2);
        for color in gradient_near_color {
            assert!((color.r - color_r.r).abs() <= 0.05);
            assert!((color.g - color_r.g).abs() <= 0.05);
            assert!((color.b - color_r.b).abs() <= 0.05);
        }
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
        let a = DecorationRef::new(ShapeDecoration::new(CircleBorder::default()));
        let b = DecorationRef::new(ShapeDecoration::new(RoundedRectangleBorder::default()));
        assert!(
            DecorationRef::lerp(Some(&a), None, 0.0)
                .unwrap()
                .eq_decoration(&*a)
        );
        assert!(
            DecorationRef::lerp(None, Some(&b), 0.0)
                .unwrap()
                .eq_decoration(&*b)
        );
        assert!(DecorationRef::lerp(None, None, 0.0).is_none());
    }

    #[test]
    fn lerp_and_hit_test() {
        let a = DecorationRef::new(ShapeDecoration::new(CircleBorder::default()));
        let b = DecorationRef::new(ShapeDecoration::new(RoundedRectangleBorder::default()));
        let c = DecorationRef::new(ShapeDecoration::new(OvalBorder::default()));
        assert!(
            DecorationRef::lerp(Some(&a), Some(&b), 0.0)
                .unwrap()
                .eq_decoration(&*a)
        );
        assert!(
            DecorationRef::lerp(Some(&a), Some(&b), 1.0)
                .unwrap()
                .eq_decoration(&*b)
        );
        assert!(
            DecorationRef::lerp(Some(&a), Some(&c), 0.0)
                .unwrap()
                .eq_decoration(&*a)
        );
        assert!(
            DecorationRef::lerp(Some(&a), Some(&c), 1.0)
                .unwrap()
                .eq_decoration(&*c)
        );
        assert!(
            DecorationRef::lerp(Some(&b), Some(&c), 0.0)
                .unwrap()
                .eq_decoration(&*b)
        );
        assert!(
            DecorationRef::lerp(Some(&b), Some(&c), 1.0)
                .unwrap()
                .eq_decoration(&*c)
        );
        let size = Size::new(200.0, 100.0);
        assert!(!a.hit_test(size, Offset::new(20.0, 50.0), None));
        assert!(!c.hit_test(size, Offset::new(50.0, 5.0), None));
        assert!(!c.hit_test(size, Offset::new(5.0, 30.0), None));
        assert!(
            !DecorationRef::lerp(Some(&a), Some(&b), 0.1)
                .unwrap()
                .hit_test(size, Offset::new(20.0, 50.0), None)
        );
        assert!(
            !DecorationRef::lerp(Some(&a), Some(&b), 0.5)
                .unwrap()
                .hit_test(size, Offset::new(20.0, 50.0), None)
        );
        assert!(
            DecorationRef::lerp(Some(&a), Some(&b), 0.9)
                .unwrap()
                .hit_test(size, Offset::new(20.0, 50.0), None)
        );
        assert!(
            !DecorationRef::lerp(Some(&a), Some(&c), 0.1)
                .unwrap()
                .hit_test(size, Offset::new(30.0, 50.0), None)
        );
        assert!(
            DecorationRef::lerp(Some(&a), Some(&c), 0.5)
                .unwrap()
                .hit_test(size, Offset::new(30.0, 50.0), None)
        );
        assert!(
            DecorationRef::lerp(Some(&a), Some(&c), 0.9)
                .unwrap()
                .hit_test(size, Offset::new(30.0, 50.0), None)
        );
        assert!(
            DecorationRef::lerp(Some(&b), Some(&c), 0.1)
                .unwrap()
                .hit_test(size, Offset::new(45.0, 10.0), None)
        );
        assert!(
            DecorationRef::lerp(Some(&b), Some(&c), 0.5)
                .unwrap()
                .hit_test(size, Offset::new(30.0, 10.0), None)
        );
        assert!(
            DecorationRef::lerp(Some(&b), Some(&c), 0.9)
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
