//! Flutter counterpart: `painting/box_decoration.dart`.

use std::any::Any;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

use reveal_embedder::{BlendMode, Canvas, ClipOp, Paint};
use reveal_embedder::{Color, Offset, Path, PathBuilder, Rect, Size};

use crate::basic_types::TextDirection;
use crate::border_radius::BorderRadiusGeometry;
use crate::borders::{BorderSide, BorderStyle};
use crate::box_border::{Border, BorderDirectional, BoxBorder, BoxShape};
use crate::box_shadow::BoxShadow;
use crate::circle_border::{oval_path, rrect_path};
use crate::debug::debug_disable_shadows;
use crate::decoration::{BoxPainter, Decoration};
use crate::draw::draw_rrect;
use crate::edge_insets::{EdgeInsets, EdgeInsetsGeometry};
use crate::image_provider::ImageConfiguration;

/// An immutable description of how to paint a box.
///
/// The box has a [`border`](Self::border), a body, and may cast a
/// [`box_shadow`](Self::box_shadow).
///
/// [`image`](https://api.flutter.dev/flutter/painting/BoxDecoration/image.html)
/// and [`gradient`](https://api.flutter.dev/flutter/painting/BoxDecoration/gradient.html)
/// are deferred (see PORTING.md).
pub struct BoxDecoration {
    /// The color to fill in the background of the box.
    ///
    /// This is ignored if a gradient is present (gradient paint is deferred).
    pub color: Option<Color>,
    /// A border to draw above the background [`color`](Self::color).
    pub border: Option<Box<dyn BoxBorder>>,
    /// If non-null, the corners of this box are rounded by this
    /// [`BorderRadius`].
    ///
    /// Applies only to boxes with rectangular shapes; ignored if
    /// [`shape`](Self::shape) is not [`BoxShape::Rectangle`].
    pub border_radius: Option<BorderRadiusGeometry>,
    /// A list of shadows cast by this box behind the box.
    pub box_shadow: Option<Vec<BoxShadow>>,
    /// The blend mode applied to the [`color`](Self::color) background of the
    /// box.
    pub background_blend_mode: Option<BlendMode>,
    /// The shape to fill the background [`color`](Self::color) into and to cast
    /// as the [`box_shadow`](Self::box_shadow).
    pub shape: BoxShape,
}

impl BoxDecoration {
    /// Creates a box decoration.
    ///
    /// Chain setters for the fields Dart's constructor takes as named arguments:
    /// `BoxDecoration::new().color(c).border_radius(r)`.
    ///
    /// * If [`color`](Self::color) is None, this decoration does not paint a
    ///   background color.
    /// * If [`border`](Self::border) is None, this decoration does not paint a
    ///   border.
    /// * If [`border_radius`](Self::border_radius) is None, this decoration
    ///   uses more efficient background painting commands. The border radius
    ///   must be None if [`shape`](Self::shape) is [`BoxShape::Circle`].
    /// * If [`box_shadow`](Self::box_shadow) is None, this decoration does not
    ///   paint a shadow.
    /// * If [`background_blend_mode`](Self::background_blend_mode) is None,
    ///   this decoration paints with [`BlendMode::SrcOver`].
    pub fn new() -> BoxDecoration {
        BoxDecoration {
            color: None,
            border: None,
            border_radius: None,
            box_shadow: None,
            background_blend_mode: None,
            shape: BoxShape::Rectangle,
        }
    }

    /// The color to fill in the background of the box.
    pub fn color(mut self, color: Color) -> BoxDecoration {
        self.color = Some(color);
        self
    }

    /// A border to draw above the background [`color`](Self::color).
    pub fn border(mut self, border: impl Into<Box<dyn BoxBorder>>) -> BoxDecoration {
        self.border = Some(border.into());
        self
    }

    /// If non-null, the corners of this box are rounded by this radius.
    ///
    /// Applies only to boxes with rectangular shapes.
    pub fn border_radius(
        mut self,
        border_radius: impl Into<BorderRadiusGeometry>,
    ) -> BoxDecoration {
        self.border_radius = Some(border_radius.into());
        self
    }

    /// A list of shadows cast by this box behind the box.
    pub fn box_shadow(mut self, box_shadow: Vec<BoxShadow>) -> BoxDecoration {
        self.box_shadow = Some(box_shadow);
        self
    }

    /// The blend mode applied to the [`color`](Self::color) background.
    ///
    /// A [`color`](Self::color) must already be set.
    pub fn background_blend_mode(mut self, background_blend_mode: BlendMode) -> BoxDecoration {
        self.background_blend_mode = Some(background_blend_mode);
        self.debug_assert_blend();
        self
    }

    /// The shape to fill the background into and to cast as the box shadow.
    pub fn shape(mut self, shape: BoxShape) -> BoxDecoration {
        self.shape = shape;
        self
    }

    fn from_fields(
        color: Option<Color>,
        border: Option<Box<dyn BoxBorder>>,
        border_radius: Option<BorderRadiusGeometry>,
        box_shadow: Option<Vec<BoxShadow>>,
        background_blend_mode: Option<BlendMode>,
        shape: BoxShape,
    ) -> BoxDecoration {
        let decoration = BoxDecoration {
            color,
            border,
            border_radius,
            box_shadow,
            background_blend_mode,
            shape,
        };
        decoration.debug_assert_blend();
        decoration
    }

    fn debug_assert_blend(&self) {
        debug_assert!(
            self.background_blend_mode.is_none() || self.color.is_some(),
            "backgroundBlendMode applies to BoxDecoration's background color or \
             gradient, but no color or gradient was provided."
        );
    }

    /// Creates a copy of this object. Chain setters to replace fields
    /// (`decoration.copy_with().color(c)`).
    pub fn copy_with(&self) -> BoxDecoration {
        self.clone()
    }

    /// Returns a new box decoration that is scaled by the given factor.
    pub fn scale(&self, factor: f64) -> BoxDecoration {
        BoxDecoration::from_fields(
            Color::lerp(None, self.color, factor),
            <dyn BoxBorder>::lerp(None, self.border.as_deref(), factor),
            BorderRadiusGeometry::lerp(None, self.border_radius, factor),
            BoxShadow::lerp_list(None, self.box_shadow.as_deref(), factor),
            self.background_blend_mode,
            self.shape,
        )
    }

    /// Linearly interpolate between two box decorations.
    ///
    /// Interpolates each parameter of the box decoration separately.
    ///
    /// The [`shape`](Self::shape) is not interpolated.
    pub fn lerp(
        a: Option<&BoxDecoration>,
        b: Option<&BoxDecoration>,
        t: f64,
    ) -> Option<BoxDecoration> {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return Some(a.clone());
        }
        if a.is_none() {
            return b.map(|b| b.scale(t));
        }
        if b.is_none() {
            return a.map(|a| a.scale(1.0 - t));
        }
        if t == 0.0 {
            return a.cloned();
        }
        if t == 1.0 {
            return b.cloned();
        }
        let a = a.unwrap();
        let b = b.unwrap();
        Some(BoxDecoration::from_fields(
            Color::lerp(a.color, b.color, t),
            <dyn BoxBorder>::lerp(a.border.as_deref(), b.border.as_deref(), t),
            BorderRadiusGeometry::lerp(a.border_radius, b.border_radius, t),
            BoxShadow::lerp_list(a.box_shadow.as_deref(), b.box_shadow.as_deref(), t),
            if t < 0.5 {
                a.background_blend_mode
            } else {
                b.background_blend_mode
            },
            if t < 0.5 { a.shape } else { b.shape },
        ))
    }
}

impl Default for BoxDecoration {
    fn default() -> BoxDecoration {
        BoxDecoration::new()
    }
}

impl Clone for BoxDecoration {
    fn clone(&self) -> BoxDecoration {
        BoxDecoration {
            color: self.color,
            border: self.border.as_ref().map(|b| b.clone_box_border()),
            border_radius: self.border_radius,
            box_shadow: self.box_shadow.clone(),
            background_blend_mode: self.background_blend_mode,
            shape: self.shape,
        }
    }
}

impl PartialEq for BoxDecoration {
    fn eq(&self, other: &BoxDecoration) -> bool {
        self.color == other.color
            && borders_eq(self.border.as_deref(), other.border.as_deref())
            && self.border_radius == other.border_radius
            && self.box_shadow == other.box_shadow
            && self.background_blend_mode == other.background_blend_mode
            && self.shape == other.shape
    }
}

fn borders_eq(a: Option<&dyn BoxBorder>, b: Option<&dyn BoxBorder>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.eq_shape(b),
        _ => false,
    }
}

impl Decoration for BoxDecoration {
    fn debug_assert_is_valid(&self) -> bool {
        debug_assert!(
            self.shape != BoxShape::Circle || self.border_radius.is_none(),
            "A circle cannot have a border radius. Remove either the shape or the borderRadius argument."
        );
        true
    }

    fn padding(&self) -> EdgeInsetsGeometry {
        self.border
            .as_ref()
            .map(|border| border.dimensions())
            .unwrap_or(EdgeInsets::ZERO.into())
    }

    fn get_clip_path(&self, rect: Rect, text_direction: TextDirection) -> Arc<Path> {
        match self.shape {
            BoxShape::Circle => {
                let center = rect.center();
                let radius = rect.shortest_side() / 2.0;
                oval_path(Rect::from_circle(center, radius))
            }
            BoxShape::Rectangle => {
                if let Some(border_radius) = self.border_radius {
                    rrect_path(border_radius.resolve(Some(text_direction)).to_rrect(rect))
                } else {
                    let mut path = PathBuilder::new();
                    path.rect(rect.into());
                    path.build()
                }
            }
        }
    }

    fn is_complex(&self) -> bool {
        self.box_shadow.is_some()
    }

    fn lerp_from(&self, a: Option<&dyn Decoration>, t: f64) -> Option<Box<dyn Decoration>> {
        match a {
            None => Some(Box::new(self.scale(t))),
            Some(a) => a.as_any().downcast_ref::<BoxDecoration>().map(|a| {
                Box::new(BoxDecoration::lerp(Some(a), Some(self), t).unwrap())
                    as Box<dyn Decoration>
            }),
        }
    }

    fn lerp_to(&self, b: Option<&dyn Decoration>, t: f64) -> Option<Box<dyn Decoration>> {
        match b {
            None => Some(Box::new(self.scale(1.0 - t))),
            Some(b) => b.as_any().downcast_ref::<BoxDecoration>().map(|b| {
                Box::new(BoxDecoration::lerp(Some(self), Some(b), t).unwrap())
                    as Box<dyn Decoration>
            }),
        }
    }

    fn hit_test(
        &self,
        size: Size,
        position: Offset,
        text_direction: Option<TextDirection>,
    ) -> bool {
        debug_assert!((Offset::ZERO & size).contains(position));
        match self.shape {
            BoxShape::Rectangle => {
                if let Some(border_radius) = self.border_radius {
                    let bounds = border_radius
                        .resolve(text_direction)
                        .to_rrect(Offset::ZERO & size);
                    bounds.contains(position)
                } else {
                    true
                }
            }
            BoxShape::Circle => {
                let center = size.center(Offset::ZERO);
                let radius = size.width().min(size.height()) / 2.0;
                (position - center).distance_squared() <= radius * radius
            }
        }
    }

    fn create_box_painter(&self, on_changed: Option<Box<dyn Fn()>>) -> Box<dyn BoxPainter> {
        Box::new(BoxDecorationPainter::new(self.clone(), on_changed))
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
            .downcast_ref::<BoxDecoration>()
            .is_some_and(|other| other == self)
    }
}

impl Debug for BoxDecoration {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("BoxDecoration")
            .field("color", &self.color)
            .field("border", &self.border)
            .field("border_radius", &self.border_radius)
            .field("box_shadow", &self.box_shadow)
            .field("background_blend_mode", &self.background_blend_mode)
            .field("shape", &self.shape)
            .finish()
    }
}

/// An object that paints a [`BoxDecoration`] into a canvas.
struct BoxDecorationPainter {
    decoration: BoxDecoration,
    on_changed: Option<Box<dyn Fn()>>,
    cached_background_paint: Option<Paint>,
}

impl BoxDecorationPainter {
    fn new(decoration: BoxDecoration, on_changed: Option<Box<dyn Fn()>>) -> BoxDecorationPainter {
        BoxDecorationPainter {
            decoration,
            on_changed,
            cached_background_paint: None,
        }
    }

    fn get_background_paint(&mut self) -> &Paint {
        if self.cached_background_paint.is_none() {
            let mut paint = Paint::default();
            if let Some(blend_mode) = self.decoration.background_blend_mode {
                paint.blend_mode = blend_mode;
            }
            if let Some(color) = self.decoration.color {
                paint.color = color.into();
            }
            self.cached_background_paint = Some(paint);
        }
        self.cached_background_paint.as_ref().unwrap()
    }

    fn paint_box(
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        shape: BoxShape,
        border_radius: Option<BorderRadiusGeometry>,
        text_direction: Option<TextDirection>,
    ) {
        match shape {
            BoxShape::Circle => {
                debug_assert!(
                    border_radius.is_none(),
                    "A circle cannot have a border radius. Remove either the shape or the borderRadius argument."
                );
                let center = rect.center();
                let radius = rect.shortest_side() / 2.0;
                canvas.draw_circle(center, radius as f32, paint);
            }
            BoxShape::Rectangle => match border_radius {
                None | Some(BorderRadiusGeometry::ZERO) => {
                    canvas.draw_rect(rect, paint);
                }
                Some(border_radius) => {
                    draw_rrect(
                        canvas,
                        border_radius.resolve(text_direction).to_rrect(rect),
                        paint,
                    );
                }
            },
        }
    }

    fn paint_shadows(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        text_direction: Option<TextDirection>,
    ) {
        let Some(box_shadow) = self.decoration.box_shadow.as_ref() else {
            return;
        };
        for box_shadow in box_shadow {
            let paint = box_shadow.to_paint();
            let bounds = rect
                .shift(box_shadow.offset)
                .inflate(box_shadow.spread_radius);
            if cfg!(debug_assertions)
                && debug_disable_shadows()
                && box_shadow.blur_style == reveal_embedder::BlurStyle::Outer
            {
                canvas.save();
                canvas.clip_rect(bounds, ClipOp::Intersect);
            }
            Self::paint_box(
                canvas,
                bounds,
                &paint,
                self.decoration.shape,
                self.decoration.border_radius,
                text_direction,
            );
            if cfg!(debug_assertions)
                && debug_disable_shadows()
                && box_shadow.blur_style == reveal_embedder::BlurStyle::Outer
            {
                canvas.restore();
            }
        }
    }

    fn paint_background_color(
        &mut self,
        canvas: &mut Canvas,
        rect: Rect,
        text_direction: Option<TextDirection>,
    ) {
        if self.decoration.color.is_some() {
            let adjusted_rect = self.adjusted_rect_on_outlined_border(rect, text_direction);
            let shape = self.decoration.shape;
            let border_radius = self.decoration.border_radius;
            let paint = self.get_background_paint().clone();
            Self::paint_box(
                canvas,
                adjusted_rect,
                &paint,
                shape,
                border_radius,
                text_direction,
            );
        }
    }

    fn calculate_adjusted_side(side: BorderSide) -> f64 {
        if (side.color.a * 255.0).round() == 255.0 && side.style == BorderStyle::Solid {
            side.stroke_inset()
        } else {
            0.0
        }
    }

    fn adjusted_rect_on_outlined_border(
        &self,
        rect: Rect,
        text_direction: Option<TextDirection>,
    ) -> Rect {
        let Some(border) = self.decoration.border.as_ref() else {
            return rect;
        };

        if let Some(border) = border.as_any().downcast_ref::<Border>() {
            let insets = EdgeInsets::from_ltrb(
                Self::calculate_adjusted_side(border.left),
                Self::calculate_adjusted_side(border.top),
                Self::calculate_adjusted_side(border.right),
                Self::calculate_adjusted_side(border.bottom),
            ) / 2.0;
            return Rect::from_ltrb(
                rect.left + insets.left,
                rect.top + insets.top,
                rect.right - insets.right,
                rect.bottom - insets.bottom,
            );
        }

        if let (Some(border), Some(text_direction)) = (
            border.as_any().downcast_ref::<BorderDirectional>(),
            text_direction,
        ) {
            let left_side = if text_direction == TextDirection::Rtl {
                border.end
            } else {
                border.start
            };
            let right_side = if text_direction == TextDirection::Rtl {
                border.start
            } else {
                border.end
            };
            let insets = EdgeInsets::from_ltrb(
                Self::calculate_adjusted_side(left_side),
                Self::calculate_adjusted_side(border.top),
                Self::calculate_adjusted_side(right_side),
                Self::calculate_adjusted_side(border.bottom),
            ) / 2.0;
            return Rect::from_ltrb(
                rect.left + insets.left,
                rect.top + insets.top,
                rect.right - insets.right,
                rect.bottom - insets.bottom,
            );
        }
        rect
    }
}

impl BoxPainter for BoxDecorationPainter {
    fn paint(&mut self, canvas: &mut Canvas, offset: Offset, configuration: &ImageConfiguration) {
        debug_assert!(
            configuration.size.is_some(),
            "The ImageConfiguration object passed as the third argument must, at a minimum, have a non-null Size."
        );
        let rect = offset & configuration.size.unwrap();
        let text_direction = configuration.text_direction;
        self.paint_shadows(canvas, rect, text_direction);
        self.paint_background_color(canvas, rect, text_direction);
        if let Some(border) = self.decoration.border.as_ref() {
            BoxBorder::paint(
                border.as_ref(),
                canvas,
                rect,
                configuration.text_direction,
                self.decoration.shape,
                self.decoration
                    .border_radius
                    .map(|radius| radius.resolve(text_direction)),
            );
        }
    }

    fn on_changed(&self) -> Option<&dyn Fn()> {
        self.on_changed.as_deref()
    }
}

impl Debug for BoxDecorationPainter {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "BoxPainter for {:?}", self.decoration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reveal_embedder::{FillRule, Radius, lerp_double};

    use crate::border_radius::{BorderRadius, BorderRadiusDirectional};
    use crate::borders::ShapeBorder;
    use crate::box_border::BoxBorder;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct TestBoxBorder {
        width: f64,
    }

    impl ShapeBorder for TestBoxBorder {
        fn dimensions(&self) -> EdgeInsetsGeometry {
            EdgeInsets::all(self.width).into()
        }

        fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
            Box::new(TestBoxBorder {
                width: self.width * t,
            })
        }

        fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
            if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<TestBoxBorder>()) {
                Some(Box::new(TestBoxBorder {
                    width: lerp_double(Some(a.width), Some(self.width), t).unwrap(),
                }))
            } else if a.is_none() {
                Some(self.scale(t))
            } else {
                None
            }
        }

        fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
            if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<TestBoxBorder>()) {
                Some(Box::new(TestBoxBorder {
                    width: lerp_double(Some(self.width), Some(b.width), t).unwrap(),
                }))
            } else if b.is_none() {
                Some(self.scale(1.0 - t))
            } else {
                None
            }
        }

        fn get_outer_path(&self, rect: Rect, _text_direction: Option<TextDirection>) -> Arc<Path> {
            let mut path = PathBuilder::new();
            path.rect(rect.into());
            path.build()
        }

        fn get_inner_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
            self.get_outer_path(rect, text_direction)
        }

        fn paint(&self, _canvas: &mut Canvas, _rect: Rect, _text_direction: Option<TextDirection>) {
        }

        fn clone_box(&self) -> Box<dyn ShapeBorder> {
            Box::new(*self)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_box_border(&self) -> Option<&dyn BoxBorder> {
            Some(self)
        }

        fn eq_shape(&self, other: &dyn ShapeBorder) -> bool {
            other
                .as_any()
                .downcast_ref::<TestBoxBorder>()
                .is_some_and(|other| other.width == self.width)
        }
    }

    impl BoxBorder for TestBoxBorder {
        fn top(&self) -> BorderSide {
            BorderSide {
                width: self.width,
                ..BorderSide::default()
            }
        }

        fn bottom(&self) -> BorderSide {
            BorderSide {
                width: self.width,
                ..BorderSide::default()
            }
        }

        fn is_uniform(&self) -> bool {
            true
        }

        fn clone_box_border(&self) -> Box<dyn BoxBorder> {
            Box::new(*self)
        }

        fn paint(
            &self,
            _canvas: &mut Canvas,
            _rect: Rect,
            _text_direction: Option<TextDirection>,
            _shape: BoxShape,
            _border_radius: Option<BorderRadius>,
        ) {
        }
    }

    fn contains(path: &Arc<Path>, x: f64, y: f64) -> bool {
        path.contains(Offset::new(x, y).into(), FillRule::NonZero)
    }

    #[test]
    fn lerp_identical_a_b() {
        assert!(BoxDecoration::lerp(None, None, 0.0).is_none());
        let decoration = BoxDecoration::default();
        assert_eq!(
            BoxDecoration::lerp(Some(&decoration), Some(&decoration), 0.5).unwrap(),
            decoration
        );
    }

    #[test]
    fn lerp_supports_custom_box_border_subclasses() {
        let a = BoxDecoration::new().border(TestBoxBorder { width: 2.0 });
        let b = BoxDecoration::new().border(TestBoxBorder { width: 6.0 });
        let decoration = BoxDecoration::lerp(Some(&a), Some(&b), 0.25).unwrap();
        assert!(
            decoration
                .border
                .as_ref()
                .unwrap()
                .as_any()
                .downcast_ref::<TestBoxBorder>()
                .is_some_and(|border| border.width == 3.0)
        );
    }

    #[test]
    fn scale_supports_custom_box_border_subclasses() {
        let decoration = BoxDecoration::new()
            .border(TestBoxBorder { width: 8.0 })
            .scale(0.25);
        assert!(
            decoration
                .border
                .as_ref()
                .unwrap()
                .as_any()
                .downcast_ref::<TestBoxBorder>()
                .is_some_and(|border| border.width == 2.0)
        );
    }

    #[test]
    fn border_radius_directional_hit_test() {
        let decoration = BoxDecoration::new()
            .color(Color::new(0xFF000000))
            .border_radius(BorderRadiusDirectional::only(
                Radius::circular(100.0),
                Radius::ZERO,
                Radius::ZERO,
                Radius::ZERO,
            ));
        let size = Size::new(1000.0, 1000.0);
        assert!(decoration.hit_test(size, Offset::new(10.0, 10.0), Some(TextDirection::Rtl)));
        assert!(!decoration.hit_test(size, Offset::new(990.0, 10.0), Some(TextDirection::Rtl)));
        assert!(!decoration.hit_test(size, Offset::new(10.0, 10.0), Some(TextDirection::Ltr)));
        assert!(decoration.hit_test(size, Offset::new(990.0, 10.0), Some(TextDirection::Ltr)));
    }

    #[test]
    fn get_clip_path_with_border_radius() {
        let radius = 10.0;
        let decoration =
            BoxDecoration::new().border_radius(BorderRadius::all(Radius::circular(radius)));
        let rect = Rect::from_ltwh(0.0, 0.0, 100.0, 20.0);
        let clip_path = decoration.get_clip_path(rect, TextDirection::Ltr);
        assert!(contains(&clip_path, 30.0, 10.0));
        assert!(contains(&clip_path, 50.0, 10.0));
        assert!(!contains(&clip_path, 1.0, 1.0));
        assert!(!contains(&clip_path, 99.0, 19.0));
    }

    #[test]
    fn get_clip_path_with_shape_circle() {
        let decoration = BoxDecoration::new().shape(BoxShape::Circle);
        let rect = Rect::from_ltwh(0.0, 0.0, 100.0, 20.0);
        let clip_path = decoration.get_clip_path(rect, TextDirection::Ltr);
        assert!(contains(&clip_path, 50.0, 0.0));
        assert!(contains(&clip_path, 40.0, 10.0));
        assert!(!contains(&clip_path, 40.0, 0.0));
        assert!(!contains(&clip_path, 10.0, 10.0));
    }

    #[test]
    fn hit_test_with_shape_circle() {
        let decoration = BoxDecoration::new().shape(BoxShape::Circle);
        let size = Size::new(100.0, 20.0);
        assert!(decoration.hit_test(size, Offset::new(50.0, 0.0), None));
        assert!(decoration.hit_test(size, Offset::new(40.0, 10.0), None));
        assert!(!decoration.hit_test(size, Offset::new(40.0, 0.0), None));
        assert!(!decoration.hit_test(size, Offset::new(10.0, 10.0), None));
    }

    #[test]
    fn different_blend_modes_are_not_equal() {
        let one = BoxDecoration::new()
            .color(Color::new(0x00000000))
            .background_blend_mode(BlendMode::Color);
        let two = BoxDecoration::new()
            .color(Color::new(0x00000000))
            .background_blend_mode(BlendMode::Difference);
        assert_ne!(one, two);
    }
}
