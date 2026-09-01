//! Flutter counterpart: `painting/box_border.dart`.

use std::any::Any;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

use reveal_embedder::{Canvas, Paint, PaintStyle, Path, PathBuilder, Stroke};
use reveal_embedder::{Color, Offset, RRect, Radius, Rect};

use crate::basic_types::TextDirection;
use crate::border_radius::BorderRadius;
use crate::borders::{BorderSide, BorderStyle, ShapeBorder, paint_border};
use crate::draw::{draw_drrect, draw_rrect};
use crate::edge_insets::{EdgeInsets, EdgeInsetsDirectional, EdgeInsetsGeometry};

/// The shape to use when rendering a [`Border`] or `BoxDecoration`.
///
/// Consider using [`ShapeBorder`] subclasses directly (with `ShapeDecoration`),
/// instead of using [`BoxShape`] and [`Border`], if the shapes will need to be
/// interpolated or animated. The [`Border`] class cannot interpolate between
/// different shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxShape {
    /// An axis-aligned rectangle, optionally with rounded corners.
    Rectangle,
    /// A circle centered in the middle of the box.
    Circle,
}

/// Base class for box borders that can paint as rectangles, circles, or rounded
/// rectangles.
///
/// The only API difference that this class introduces over [`ShapeBorder`] is
/// that its [`paint`](Self::paint) method takes additional arguments.
pub trait BoxBorder: ShapeBorder {
    /// The top side of this border.
    fn top(&self) -> BorderSide;

    /// The bottom side of this border.
    fn bottom(&self) -> BorderSide;

    /// Whether all four sides of the border are identical. Uniform borders are
    /// typically more efficient to paint.
    fn is_uniform(&self) -> bool;

    /// A heap clone typed as [`BoxBorder`].
    fn clone_box_border(&self) -> Box<dyn BoxBorder>;

    /// Paints the border within the given [`Rect`] on the given [`Canvas`].
    ///
    /// This is an extension of the [`ShapeBorder::paint`] method. It allows
    /// [`BoxBorder`] borders to be applied to different [`BoxShape`]s and with
    /// different `border_radius` parameters, without changing the [`BoxBorder`]
    /// object itself.
    fn paint(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        text_direction: Option<TextDirection>,
        shape: BoxShape,
        border_radius: Option<BorderRadius>,
    );
}

impl<T: BoxBorder + 'static> From<T> for Box<dyn BoxBorder> {
    fn from(border: T) -> Box<dyn BoxBorder> {
        Box::new(border)
    }
}

impl dyn BoxBorder {
    /// Creates a [`Border`].
    pub fn from_ltrb(
        top: BorderSide,
        right: BorderSide,
        bottom: BorderSide,
        left: BorderSide,
    ) -> Border {
        Border {
            top,
            right,
            bottom,
            left,
        }
    }

    /// A uniform [`Border`] with all sides the same color and width.
    pub fn all(color: Color, width: f64, style: BorderStyle, stroke_align: f64) -> Border {
        Border::all(color, width, style, stroke_align)
    }

    /// Creates a [`Border`] whose sides are all the same.
    pub fn from_border_side(side: BorderSide) -> Border {
        Border::from_border_side(side)
    }

    /// Creates a [`Border`] with symmetrical vertical and horizontal sides.
    pub fn symmetric(vertical: BorderSide, horizontal: BorderSide) -> Border {
        Border::symmetric(vertical, horizontal)
    }

    /// Creates a [`BorderDirectional`].
    pub fn from_steb(
        top: BorderSide,
        start: BorderSide,
        end: BorderSide,
        bottom: BorderSide,
    ) -> BorderDirectional {
        BorderDirectional {
            top,
            start,
            end,
            bottom,
        }
    }

    /// Linearly interpolate between two borders.
    ///
    /// If a border is null, it is treated as having four [`BorderSide::NONE`]
    /// borders.
    pub fn lerp(
        a: Option<&dyn BoxBorder>,
        b: Option<&dyn BoxBorder>,
        t: f64,
    ) -> Option<Box<dyn BoxBorder>> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone_box_border()),
            _ => {}
        }
        let a_is_border = a.is_none()
            || a.and_then(|border| border.as_any().downcast_ref::<Border>())
                .is_some();
        let b_is_border = b.is_none()
            || b.and_then(|border| border.as_any().downcast_ref::<Border>())
                .is_some();
        if a_is_border && b_is_border {
            return Border::lerp(
                a.and_then(|border| border.as_any().downcast_ref::<Border>())
                    .copied(),
                b.and_then(|border| border.as_any().downcast_ref::<Border>())
                    .copied(),
                t,
            )
            .map(|border| Box::new(border) as Box<dyn BoxBorder>);
        }
        let a_is_directional = a.is_none()
            || a.and_then(|border| border.as_any().downcast_ref::<BorderDirectional>())
                .is_some();
        let b_is_directional = b.is_none()
            || b.and_then(|border| border.as_any().downcast_ref::<BorderDirectional>())
                .is_some();
        if a_is_directional && b_is_directional {
            return BorderDirectional::lerp(
                a.and_then(|border| border.as_any().downcast_ref::<BorderDirectional>())
                    .copied(),
                b.and_then(|border| border.as_any().downcast_ref::<BorderDirectional>())
                    .copied(),
                t,
            )
            .map(|border| Box::new(border) as Box<dyn BoxBorder>);
        }
        let (a, b, t) = if b
            .and_then(|border| border.as_any().downcast_ref::<Border>())
            .is_some()
            && a.and_then(|border| border.as_any().downcast_ref::<BorderDirectional>())
                .is_some()
        {
            (b, a, 1.0 - t)
        } else {
            (a, b, t)
        };
        if let (Some(a), Some(b)) = (
            a.and_then(|border| border.as_any().downcast_ref::<Border>())
                .copied(),
            b.and_then(|border| border.as_any().downcast_ref::<BorderDirectional>())
                .copied(),
        ) {
            return Some(lerp_border_to_directional(a, b, t));
        }
        let result = b
            .and_then(|b| b.lerp_from(a.map(|a| a as &dyn ShapeBorder), t))
            .or_else(|| a.and_then(|a| a.lerp_to(b.map(|b| b as &dyn ShapeBorder), t)));
        result
            .and_then(|result| {
                result
                    .as_box_border()
                    .map(|border| border.clone_box_border())
            })
            .or_else(|| {
                if t < 0.5 {
                    a.map(BoxBorder::clone_box_border)
                } else {
                    b.map(BoxBorder::clone_box_border)
                }
            })
    }

    /// Paints a Border with different widths, styles and strokeAligns, on any
    /// borderRadius while using a single color.
    #[allow(clippy::too_many_arguments)]
    pub fn paint_non_uniform_border(
        canvas: &mut Canvas,
        rect: Rect,
        border_radius: Option<BorderRadius>,
        text_direction: Option<TextDirection>,
        shape: BoxShape,
        top: BorderSide,
        right: BorderSide,
        bottom: BorderSide,
        left: BorderSide,
        color: Color,
    ) {
        let border_rect = match shape {
            BoxShape::Rectangle => border_radius
                .unwrap_or(BorderRadius::ZERO)
                .resolve(text_direction)
                .to_rrect(rect),
            BoxShape::Circle => {
                debug_assert!(
                    border_radius.is_none(),
                    "A circle cannot have a border radius. Remove either the shape or the borderRadius argument."
                );
                RRect::from_rect_and_radius(
                    Rect::from_circle(rect.center(), rect.shortest_side() / 2.0),
                    Radius::circular(rect.width()),
                )
            }
        };
        let paint = Paint {
            color: color.into(),
            ..Paint::default()
        };
        let inner = EdgeInsets::from_ltrb(
            left.stroke_inset(),
            top.stroke_inset(),
            right.stroke_inset(),
            bottom.stroke_inset(),
        )
        .deflate_rrect(border_rect);
        let outer = EdgeInsets::from_ltrb(
            left.stroke_outset(),
            top.stroke_outset(),
            right.stroke_outset(),
            bottom.stroke_outset(),
        )
        .inflate_rrect(border_rect);
        draw_drrect(canvas, outer, inner, &paint);
    }
}

impl Clone for Box<dyn BoxBorder> {
    fn clone(&self) -> Self {
        self.clone_box_border()
    }
}

fn lerp_border_to_directional(a: Border, b: BorderDirectional, t: f64) -> Box<dyn BoxBorder> {
    if b.start == BorderSide::NONE && b.end == BorderSide::NONE {
        return Box::new(Border {
            top: BorderSide::lerp(a.top, b.top, t),
            right: BorderSide::lerp(a.right, BorderSide::NONE, t),
            bottom: BorderSide::lerp(a.bottom, b.bottom, t),
            left: BorderSide::lerp(a.left, BorderSide::NONE, t),
        });
    }
    if a.left == BorderSide::NONE && a.right == BorderSide::NONE {
        return Box::new(BorderDirectional {
            top: BorderSide::lerp(a.top, b.top, t),
            start: BorderSide::lerp(BorderSide::NONE, b.start, t),
            end: BorderSide::lerp(BorderSide::NONE, b.end, t),
            bottom: BorderSide::lerp(a.bottom, b.bottom, t),
        });
    }
    if t < 0.5 {
        Box::new(Border {
            top: BorderSide::lerp(a.top, b.top, t),
            right: BorderSide::lerp(a.right, BorderSide::NONE, t * 2.0),
            bottom: BorderSide::lerp(a.bottom, b.bottom, t),
            left: BorderSide::lerp(a.left, BorderSide::NONE, t * 2.0),
        })
    } else {
        Box::new(BorderDirectional {
            top: BorderSide::lerp(a.top, b.top, t),
            start: BorderSide::lerp(BorderSide::NONE, b.start, (t - 0.5) * 2.0),
            end: BorderSide::lerp(BorderSide::NONE, b.end, (t - 0.5) * 2.0),
            bottom: BorderSide::lerp(a.bottom, b.bottom, t),
        })
    }
}

fn paint_uniform_border_with_radius(
    canvas: &mut Canvas,
    rect: Rect,
    side: BorderSide,
    border_radius: BorderRadius,
) {
    debug_assert!(side.style != BorderStyle::None);
    let width = side.width;
    if width == 0.0 {
        let paint = Paint {
            color: side.color.into(),
            style: PaintStyle::Stroke(Stroke::new(0.0)),
            ..Paint::default()
        };
        draw_rrect(canvas, border_radius.to_rrect(rect), &paint);
    } else {
        let paint = Paint {
            color: side.color.into(),
            ..Paint::default()
        };
        let border_rect = border_radius.to_rrect(rect);
        let inner = border_rect.deflate(side.stroke_inset());
        let outer = border_rect.inflate(side.stroke_outset());
        draw_drrect(canvas, outer, inner, &paint);
    }
}

fn paint_uniform_border_with_circle(canvas: &mut Canvas, rect: Rect, side: BorderSide) {
    debug_assert!(side.style != BorderStyle::None);
    let radius = (rect.shortest_side() + side.stroke_offset()) / 2.0;
    canvas.draw_circle(rect.center(), radius as f32, &side.to_paint());
}

fn paint_uniform_border_with_rectangle(canvas: &mut Canvas, rect: Rect, side: BorderSide) {
    debug_assert!(side.style != BorderStyle::None);
    canvas.draw_rect(rect.inflate(side.stroke_offset() / 2.0), &side.to_paint());
}

fn rect_path(rect: Rect) -> Arc<Path> {
    let mut path = PathBuilder::new();
    path.rect(rect.into());
    path.build()
}

fn box_get_inner_path(
    dimensions: EdgeInsetsGeometry,
    rect: Rect,
    text_direction: Option<TextDirection>,
) -> Arc<Path> {
    debug_assert!(
        text_direction.is_some(),
        "The textDirection argument to getInnerPath must not be null."
    );
    rect_path(dimensions.resolve(text_direction).deflate_rect(rect))
}

fn box_get_outer_path(rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
    debug_assert!(
        text_direction.is_some(),
        "The textDirection argument to getOuterPath must not be null."
    );
    let _ = text_direction;
    rect_path(rect)
}

/// A border of a box, comprised of four sides: top, right, bottom, left.
#[derive(Clone, Copy, PartialEq)]
pub struct Border {
    /// The top side of this border.
    pub top: BorderSide,
    /// The right side of this border.
    pub right: BorderSide,
    /// The bottom side of this border.
    pub bottom: BorderSide,
    /// The left side of this border.
    pub left: BorderSide,
}

impl Border {
    /// Creates a border whose sides are all the same.
    pub const fn from_border_side(side: BorderSide) -> Border {
        Border {
            top: side,
            right: side,
            bottom: side,
            left: side,
        }
    }

    /// Creates a border with symmetrical vertical and horizontal sides.
    pub const fn symmetric(vertical: BorderSide, horizontal: BorderSide) -> Border {
        Border {
            left: vertical,
            top: horizontal,
            right: vertical,
            bottom: horizontal,
        }
    }

    /// A uniform border with all sides the same color and width.
    pub fn all(color: Color, width: f64, style: BorderStyle, stroke_align: f64) -> Border {
        Border::from_border_side(BorderSide::new(color, width, style, stroke_align))
    }

    /// Creates a [`Border`] that represents the addition of the two given
    /// [`Border`]s.
    pub fn merge(a: Border, b: Border) -> Border {
        debug_assert!(BorderSide::can_merge(a.top, b.top));
        debug_assert!(BorderSide::can_merge(a.right, b.right));
        debug_assert!(BorderSide::can_merge(a.bottom, b.bottom));
        debug_assert!(BorderSide::can_merge(a.left, b.left));
        Border {
            top: BorderSide::merge(a.top, b.top),
            right: BorderSide::merge(a.right, b.right),
            bottom: BorderSide::merge(a.bottom, b.bottom),
            left: BorderSide::merge(a.left, b.left),
        }
    }

    /// Linearly interpolate between two borders.
    pub fn lerp(a: Option<Border>, b: Option<Border>, t: f64) -> Option<Border> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b.scale(t)),
            (Some(a), None) => Some(a.scale(1.0 - t)),
            (Some(a), Some(b)) => Some(Border {
                top: BorderSide::lerp(a.top, b.top, t),
                right: BorderSide::lerp(a.right, b.right, t),
                bottom: BorderSide::lerp(a.bottom, b.bottom, t),
                left: BorderSide::lerp(a.left, b.left, t),
            }),
        }
    }

    /// Creates a copy of this border, scaled by the factor `t`.
    pub fn scale(self, t: f64) -> Border {
        Border {
            top: self.top.scale(t),
            right: self.right.scale(t),
            bottom: self.bottom.scale(t),
            left: self.left.scale(t),
        }
    }

    fn color_is_uniform(self) -> bool {
        let top_color = self.top.color;
        self.left.color == top_color
            && self.bottom.color == top_color
            && self.right.color == top_color
    }

    fn width_is_uniform(self) -> bool {
        let top_width = self.top.width;
        self.left.width == top_width
            && self.bottom.width == top_width
            && self.right.width == top_width
    }

    fn style_is_uniform(self) -> bool {
        let top_style = self.top.style;
        self.left.style == top_style
            && self.bottom.style == top_style
            && self.right.style == top_style
    }

    fn stroke_align_is_uniform(self) -> bool {
        let top_stroke_align = self.top.stroke_align;
        self.left.stroke_align == top_stroke_align
            && self.bottom.stroke_align == top_stroke_align
            && self.right.stroke_align == top_stroke_align
    }

    fn distinct_visible_colors(self) -> Vec<Color> {
        let mut colors = Vec::new();
        for side in [self.top, self.right, self.bottom, self.left] {
            if side.style != BorderStyle::None && !colors.contains(&side.color) {
                colors.push(side.color);
            }
        }
        colors
    }

    fn has_hairline_border(self) -> bool {
        (self.top.style == BorderStyle::Solid && self.top.width == 0.0)
            || (self.right.style == BorderStyle::Solid && self.right.width == 0.0)
            || (self.bottom.style == BorderStyle::Solid && self.bottom.width == 0.0)
            || (self.left.style == BorderStyle::Solid && self.left.width == 0.0)
    }
}

impl Default for Border {
    fn default() -> Border {
        Border {
            top: BorderSide::NONE,
            right: BorderSide::NONE,
            bottom: BorderSide::NONE,
            left: BorderSide::NONE,
        }
    }
}

impl ShapeBorder for Border {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        EdgeInsets::from_ltrb(
            self.left.stroke_inset(),
            self.top.stroke_inset(),
            self.right.stroke_inset(),
            self.bottom.stroke_inset(),
        )
        .into()
    }

    fn add(&self, other: &dyn ShapeBorder, _reversed: bool) -> Option<Box<dyn ShapeBorder>> {
        other.as_any().downcast_ref::<Border>().and_then(|other| {
            if BorderSide::can_merge(self.top, other.top)
                && BorderSide::can_merge(self.right, other.right)
                && BorderSide::can_merge(self.bottom, other.bottom)
                && BorderSide::can_merge(self.left, other.left)
            {
                Some(Box::new(Border::merge(*self, *other)) as Box<dyn ShapeBorder>)
            } else {
                None
            }
        })
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(Border::scale(*self, t))
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<Border>()) {
            Border::lerp(Some(*a), Some(*self), t).map(|border| Box::new(border) as _)
        } else if a.is_none() {
            Some(Box::new(Border::scale(*self, t)))
        } else {
            None
        }
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<Border>()) {
            Border::lerp(Some(*self), Some(*b), t).map(|border| Box::new(border) as _)
        } else if b.is_none() {
            Some(Box::new(Border::scale(*self, 1.0 - t)))
        } else {
            None
        }
    }

    fn get_outer_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        box_get_outer_path(rect, text_direction)
    }

    fn get_inner_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        box_get_inner_path(self.dimensions(), rect, text_direction)
    }

    fn hit_test(
        &self,
        rect: Rect,
        position: Offset,
        _text_direction: Option<TextDirection>,
    ) -> bool {
        rect.contains(position)
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        _text_direction: Option<TextDirection>,
    ) {
        canvas.draw_rect(rect, paint);
    }

    fn prefer_paint_interior(&self) -> bool {
        true
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect, text_direction: Option<TextDirection>) {
        <Self as BoxBorder>::paint(
            self,
            canvas,
            rect,
            text_direction,
            BoxShape::Rectangle,
            None,
        );
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
            .downcast_ref::<Border>()
            .is_some_and(|other| other == self)
    }
}

impl BoxBorder for Border {
    fn top(&self) -> BorderSide {
        self.top
    }

    fn bottom(&self) -> BorderSide {
        self.bottom
    }

    fn is_uniform(&self) -> bool {
        self.color_is_uniform()
            && self.width_is_uniform()
            && self.style_is_uniform()
            && self.stroke_align_is_uniform()
    }

    fn clone_box_border(&self) -> Box<dyn BoxBorder> {
        Box::new(*self)
    }

    fn paint(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        text_direction: Option<TextDirection>,
        shape: BoxShape,
        border_radius: Option<BorderRadius>,
    ) {
        if self.is_uniform() {
            match self.top.style {
                BorderStyle::None => return,
                BorderStyle::Solid => {
                    match shape {
                        BoxShape::Circle => {
                            debug_assert!(
                                border_radius.is_none(),
                                "A circle cannot have a border radius. Remove either the shape or the borderRadius argument."
                            );
                            paint_uniform_border_with_circle(canvas, rect, self.top);
                        }
                        BoxShape::Rectangle => {
                            if let Some(border_radius) = border_radius
                                && border_radius != BorderRadius::ZERO
                            {
                                paint_uniform_border_with_radius(
                                    canvas,
                                    rect,
                                    self.top,
                                    border_radius,
                                );
                                return;
                            }
                            paint_uniform_border_with_rectangle(canvas, rect, self.top);
                        }
                    }
                    return;
                }
            }
        }

        if self.style_is_uniform() && self.top.style == BorderStyle::None {
            return;
        }

        let visible_colors = self.distinct_visible_colors();
        let has_hairline_border = self.has_hairline_border();
        if visible_colors.len() == 1
            && !has_hairline_border
            && (shape == BoxShape::Circle
                || border_radius.is_some_and(|radius| radius != BorderRadius::ZERO))
        {
            <dyn BoxBorder>::paint_non_uniform_border(
                canvas,
                rect,
                border_radius,
                text_direction,
                shape,
                if self.top.style == BorderStyle::None {
                    BorderSide::NONE
                } else {
                    self.top
                },
                if self.right.style == BorderStyle::None {
                    BorderSide::NONE
                } else {
                    self.right
                },
                if self.bottom.style == BorderStyle::None {
                    BorderSide::NONE
                } else {
                    self.bottom
                },
                if self.left.style == BorderStyle::None {
                    BorderSide::NONE
                } else {
                    self.left
                },
                visible_colors[0],
            );
            return;
        }

        if cfg!(debug_assertions) {
            if has_hairline_border {
                debug_assert!(
                    border_radius.is_none() || border_radius == Some(BorderRadius::ZERO),
                    "A hairline border like `BorderSide(width: 0.0, style: BorderStyle.solid)` can only be drawn when BorderRadius is zero or null."
                );
            }
            if border_radius.is_some_and(|radius| radius != BorderRadius::ZERO) {
                panic!("A borderRadius can only be given on borders with uniform colors.");
            }
            if shape != BoxShape::Rectangle {
                panic!("A Border can only be drawn as a circle on borders with uniform colors.");
            }
            if !self.stroke_align_is_uniform()
                || self.top.stroke_align != BorderSide::STROKE_ALIGN_INSIDE
            {
                panic!(
                    "A Border can only draw strokeAlign different than BorderSide.strokeAlignInside on borders with uniform colors."
                );
            }
        }

        paint_border(canvas, rect, self.top, self.right, self.bottom, self.left);
    }
}

impl Debug for Border {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.is_uniform() {
            return write!(f, "Border.all({:?})", self.top);
        }
        let mut arguments = Vec::new();
        if self.top != BorderSide::NONE {
            arguments.push(format!("top: {:?}", self.top));
        }
        if self.right != BorderSide::NONE {
            arguments.push(format!("right: {:?}", self.right));
        }
        if self.bottom != BorderSide::NONE {
            arguments.push(format!("bottom: {:?}", self.bottom));
        }
        if self.left != BorderSide::NONE {
            arguments.push(format!("left: {:?}", self.left));
        }
        write!(f, "Border({})", arguments.join(", "))
    }
}

/// A border of a box, comprised of four sides, the lateral sides of which
/// flip over based on the reading direction.
#[derive(Clone, Copy, PartialEq)]
pub struct BorderDirectional {
    /// The top side of this border.
    pub top: BorderSide,
    /// The start side of this border.
    pub start: BorderSide,
    /// The end side of this border.
    pub end: BorderSide,
    /// The bottom side of this border.
    pub bottom: BorderSide,
}

impl BorderDirectional {
    /// Creates a [`BorderDirectional`] that represents the addition of the two
    /// given [`BorderDirectional`]s.
    pub fn merge(a: BorderDirectional, b: BorderDirectional) -> BorderDirectional {
        debug_assert!(BorderSide::can_merge(a.top, b.top));
        debug_assert!(BorderSide::can_merge(a.start, b.start));
        debug_assert!(BorderSide::can_merge(a.end, b.end));
        debug_assert!(BorderSide::can_merge(a.bottom, b.bottom));
        BorderDirectional {
            top: BorderSide::merge(a.top, b.top),
            start: BorderSide::merge(a.start, b.start),
            end: BorderSide::merge(a.end, b.end),
            bottom: BorderSide::merge(a.bottom, b.bottom),
        }
    }

    /// Linearly interpolate between two borders.
    pub fn lerp(
        a: Option<BorderDirectional>,
        b: Option<BorderDirectional>,
        t: f64,
    ) -> Option<BorderDirectional> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b.scale(t)),
            (Some(a), None) => Some(a.scale(1.0 - t)),
            (Some(a), Some(b)) => Some(BorderDirectional {
                top: BorderSide::lerp(a.top, b.top, t),
                end: BorderSide::lerp(a.end, b.end, t),
                bottom: BorderSide::lerp(a.bottom, b.bottom, t),
                start: BorderSide::lerp(a.start, b.start, t),
            }),
        }
    }

    /// Creates a copy of this border, scaled by the factor `t`.
    pub fn scale(self, t: f64) -> BorderDirectional {
        BorderDirectional {
            top: self.top.scale(t),
            start: self.start.scale(t),
            end: self.end.scale(t),
            bottom: self.bottom.scale(t),
        }
    }

    fn color_is_uniform(self) -> bool {
        let top_color = self.top.color;
        self.start.color == top_color
            && self.bottom.color == top_color
            && self.end.color == top_color
    }

    fn width_is_uniform(self) -> bool {
        let top_width = self.top.width;
        self.start.width == top_width
            && self.bottom.width == top_width
            && self.end.width == top_width
    }

    fn style_is_uniform(self) -> bool {
        let top_style = self.top.style;
        self.start.style == top_style
            && self.bottom.style == top_style
            && self.end.style == top_style
    }

    fn stroke_align_is_uniform(self) -> bool {
        let top_stroke_align = self.top.stroke_align;
        self.start.stroke_align == top_stroke_align
            && self.bottom.stroke_align == top_stroke_align
            && self.end.stroke_align == top_stroke_align
    }

    fn distinct_visible_colors(self) -> Vec<Color> {
        let mut colors = Vec::new();
        for side in [self.top, self.end, self.bottom, self.start] {
            if side.style != BorderStyle::None && !colors.contains(&side.color) {
                colors.push(side.color);
            }
        }
        colors
    }

    fn has_hairline_border(self) -> bool {
        (self.top.style == BorderStyle::Solid && self.top.width == 0.0)
            || (self.end.style == BorderStyle::Solid && self.end.width == 0.0)
            || (self.bottom.style == BorderStyle::Solid && self.bottom.width == 0.0)
            || (self.start.style == BorderStyle::Solid && self.start.width == 0.0)
    }
}

impl Default for BorderDirectional {
    fn default() -> BorderDirectional {
        BorderDirectional {
            top: BorderSide::NONE,
            start: BorderSide::NONE,
            end: BorderSide::NONE,
            bottom: BorderSide::NONE,
        }
    }
}

impl ShapeBorder for BorderDirectional {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        EdgeInsetsDirectional::from_steb(
            self.start.stroke_inset(),
            self.top.stroke_inset(),
            self.end.stroke_inset(),
            self.bottom.stroke_inset(),
        )
        .into()
    }

    fn add(&self, other: &dyn ShapeBorder, _reversed: bool) -> Option<Box<dyn ShapeBorder>> {
        if let Some(typed_other) = other.as_any().downcast_ref::<BorderDirectional>() {
            if BorderSide::can_merge(self.top, typed_other.top)
                && BorderSide::can_merge(self.start, typed_other.start)
                && BorderSide::can_merge(self.end, typed_other.end)
                && BorderSide::can_merge(self.bottom, typed_other.bottom)
            {
                return Some(Box::new(BorderDirectional::merge(*self, *typed_other)) as _);
            }
            return None;
        }
        if let Some(typed_other) = other.as_any().downcast_ref::<Border>() {
            if !BorderSide::can_merge(typed_other.top, self.top)
                || !BorderSide::can_merge(typed_other.bottom, self.bottom)
            {
                return None;
            }
            if self.start != BorderSide::NONE || self.end != BorderSide::NONE {
                if typed_other.left != BorderSide::NONE || typed_other.right != BorderSide::NONE {
                    return None;
                }
                debug_assert_eq!(typed_other.left, BorderSide::NONE);
                debug_assert_eq!(typed_other.right, BorderSide::NONE);
                return Some(Box::new(BorderDirectional {
                    top: BorderSide::merge(typed_other.top, self.top),
                    start: self.start,
                    end: self.end,
                    bottom: BorderSide::merge(typed_other.bottom, self.bottom),
                }) as _);
            }
            debug_assert_eq!(self.start, BorderSide::NONE);
            debug_assert_eq!(self.end, BorderSide::NONE);
            return Some(Box::new(Border {
                top: BorderSide::merge(typed_other.top, self.top),
                right: typed_other.right,
                bottom: BorderSide::merge(typed_other.bottom, self.bottom),
                left: typed_other.left,
            }) as _);
        }
        None
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(BorderDirectional::scale(*self, t))
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(a) = a.and_then(|a| a.as_any().downcast_ref::<BorderDirectional>()) {
            BorderDirectional::lerp(Some(*a), Some(*self), t).map(|border| Box::new(border) as _)
        } else if a.is_none() {
            Some(Box::new(BorderDirectional::scale(*self, t)))
        } else {
            None
        }
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if let Some(b) = b.and_then(|b| b.as_any().downcast_ref::<BorderDirectional>()) {
            BorderDirectional::lerp(Some(*self), Some(*b), t).map(|border| Box::new(border) as _)
        } else if b.is_none() {
            Some(Box::new(BorderDirectional::scale(*self, 1.0 - t)))
        } else {
            None
        }
    }

    fn get_outer_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        box_get_outer_path(rect, text_direction)
    }

    fn get_inner_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        box_get_inner_path(self.dimensions(), rect, text_direction)
    }

    fn hit_test(
        &self,
        rect: Rect,
        position: Offset,
        _text_direction: Option<TextDirection>,
    ) -> bool {
        rect.contains(position)
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        _text_direction: Option<TextDirection>,
    ) {
        canvas.draw_rect(rect, paint);
    }

    fn prefer_paint_interior(&self) -> bool {
        true
    }

    fn paint(&self, canvas: &mut Canvas, rect: Rect, text_direction: Option<TextDirection>) {
        <Self as BoxBorder>::paint(
            self,
            canvas,
            rect,
            text_direction,
            BoxShape::Rectangle,
            None,
        );
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
            .downcast_ref::<BorderDirectional>()
            .is_some_and(|other| other == self)
    }
}

impl BoxBorder for BorderDirectional {
    fn top(&self) -> BorderSide {
        self.top
    }

    fn bottom(&self) -> BorderSide {
        self.bottom
    }

    fn is_uniform(&self) -> bool {
        self.color_is_uniform()
            && self.width_is_uniform()
            && self.style_is_uniform()
            && self.stroke_align_is_uniform()
    }

    fn clone_box_border(&self) -> Box<dyn BoxBorder> {
        Box::new(*self)
    }

    fn paint(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        text_direction: Option<TextDirection>,
        shape: BoxShape,
        border_radius: Option<BorderRadius>,
    ) {
        if self.is_uniform() {
            match self.top.style {
                BorderStyle::None => return,
                BorderStyle::Solid => {
                    match shape {
                        BoxShape::Circle => {
                            debug_assert!(
                                border_radius.is_none(),
                                "A circle cannot have a border radius. Remove either the shape or the borderRadius argument."
                            );
                            paint_uniform_border_with_circle(canvas, rect, self.top);
                        }
                        BoxShape::Rectangle => {
                            if let Some(border_radius) = border_radius
                                && border_radius != BorderRadius::ZERO
                            {
                                paint_uniform_border_with_radius(
                                    canvas,
                                    rect,
                                    self.top,
                                    border_radius,
                                );
                                return;
                            }
                            paint_uniform_border_with_rectangle(canvas, rect, self.top);
                        }
                    }
                    return;
                }
            }
        }

        if self.style_is_uniform() && self.top.style == BorderStyle::None {
            return;
        }

        debug_assert!(
            text_direction.is_some(),
            "Non-uniform BorderDirectional objects require a TextDirection when painting."
        );
        let (left, right) = match text_direction.unwrap() {
            TextDirection::Rtl => (self.end, self.start),
            TextDirection::Ltr => (self.start, self.end),
        };

        let visible_colors = self.distinct_visible_colors();
        let has_hairline_border = self.has_hairline_border();
        if visible_colors.len() == 1
            && !has_hairline_border
            && (shape == BoxShape::Circle
                || border_radius.is_some_and(|radius| radius != BorderRadius::ZERO))
        {
            <dyn BoxBorder>::paint_non_uniform_border(
                canvas,
                rect,
                border_radius,
                text_direction,
                shape,
                if self.top.style == BorderStyle::None {
                    BorderSide::NONE
                } else {
                    self.top
                },
                if right.style == BorderStyle::None {
                    BorderSide::NONE
                } else {
                    right
                },
                if self.bottom.style == BorderStyle::None {
                    BorderSide::NONE
                } else {
                    self.bottom
                },
                if left.style == BorderStyle::None {
                    BorderSide::NONE
                } else {
                    left
                },
                visible_colors[0],
            );
            return;
        }

        if has_hairline_border {
            debug_assert!(
                border_radius.is_none() || border_radius == Some(BorderRadius::ZERO),
                "A side like `BorderSide(width: 0.0, style: BorderStyle.solid)` can only be drawn when BorderRadius is zero or null."
            );
        }
        debug_assert!(
            border_radius.is_none(),
            "A borderRadius can only be given for borders with uniform colors."
        );
        debug_assert!(
            shape == BoxShape::Rectangle,
            "A Border can only be drawn as a circle on borders with uniform colors."
        );
        debug_assert!(
            self.stroke_align_is_uniform()
                && self.top.stroke_align == BorderSide::STROKE_ALIGN_INSIDE,
            "A Border can only draw strokeAlign different than strokeAlignInside on borders with uniform colors."
        );

        paint_border(canvas, rect, self.top, right, self.bottom, left);
    }
}

impl Debug for BorderDirectional {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut arguments = Vec::new();
        if self.top != BorderSide::NONE {
            arguments.push(format!("top: {:?}", self.top));
        }
        if self.start != BorderSide::NONE {
            arguments.push(format!("start: {:?}", self.start));
        }
        if self.end != BorderSide::NONE {
            arguments.push(format!("end: {:?}", self.end));
        }
        if self.bottom != BorderSide::NONE {
            arguments.push(format!("bottom: {:?}", self.bottom));
        }
        write!(f, "BorderDirectional({})", arguments.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;
    use std::sync::Arc;

    use reveal_embedder::{Color, FillRule};

    use crate::edge_insets::EdgeInsetsGeometry;

    fn width(w: f64) -> BorderSide {
        BorderSide {
            width: w,
            ..BorderSide::default()
        }
    }

    fn color_width(color: u32, w: f64) -> BorderSide {
        BorderSide {
            color: Color::new(color),
            width: w,
            ..BorderSide::default()
        }
    }

    fn border_all(w: f64) -> Border {
        Border::all(
            Color::new(0xFF000000),
            w,
            BorderStyle::Solid,
            BorderSide::STROKE_ALIGN_INSIDE,
        )
    }

    fn none_scaled() -> Border {
        Border::all(
            Color::new(0xFF000000),
            0.0,
            BorderStyle::None,
            BorderSide::STROKE_ALIGN_INSIDE,
        )
    }

    fn plus(a: &dyn ShapeBorder, b: &dyn ShapeBorder) -> Box<dyn ShapeBorder> {
        a.plus(b)
    }

    fn expect_border(got: Option<Box<dyn BoxBorder>>, expected: Border) {
        let got = got.expect("expected a border");
        let got = got
            .as_any()
            .downcast_ref::<Border>()
            .unwrap_or_else(|| panic!("expected Border, got {got:?}"));
        assert_eq!(*got, expected);
    }

    fn expect_directional(got: Option<Box<dyn BoxBorder>>, expected: BorderDirectional) {
        let got = got.expect("expected a border");
        let got = got
            .as_any()
            .downcast_ref::<BorderDirectional>()
            .unwrap_or_else(|| panic!("expected BorderDirectional, got {got:?}"));
        assert_eq!(*got, expected);
    }

    fn contains(path: &Arc<Path>, x: f64, y: f64) -> bool {
        path.contains(Offset::new(x, y).into(), FillRule::NonZero)
    }

    #[derive(Clone, Copy, Debug)]
    struct SillyBorder;

    impl ShapeBorder for SillyBorder {
        fn dimensions(&self) -> EdgeInsetsGeometry {
            EdgeInsetsGeometry::ZERO
        }

        fn scale(&self, _t: f64) -> Box<dyn ShapeBorder> {
            Box::new(*self)
        }

        fn get_outer_path(&self, rect: Rect, _text_direction: Option<TextDirection>) -> Arc<Path> {
            rect_path(rect)
        }

        fn get_inner_path(&self, rect: Rect, _text_direction: Option<TextDirection>) -> Arc<Path> {
            rect_path(rect)
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
            other.as_any().downcast_ref::<SillyBorder>().is_some()
        }
    }

    impl BoxBorder for SillyBorder {
        fn top(&self) -> BorderSide {
            BorderSide::NONE
        }

        fn bottom(&self) -> BorderSide {
            BorderSide::NONE
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

    #[test]
    fn from_border_side_and_symmetric() {
        let side = BorderSide::default();
        let border = Border::from_border_side(side);
        assert_eq!(border.left, side);
        assert_eq!(border.top, side);
        assert_eq!(border.right, side);
        assert_eq!(border.bottom, side);

        let side1 = BorderSide {
            color: Color::new(0xFFFFFFFF),
            ..BorderSide::default()
        };
        let side2 = BorderSide::default();
        let border = Border::symmetric(side1, side2);
        assert_eq!(border.left, side1);
        assert_eq!(border.top, side2);
        assert_eq!(border.right, side1);
        assert_eq!(border.bottom, side2);
    }

    #[test]
    fn merge_add_scale() {
        let magenta3 = color_width(0xFFFF00FF, 3.0);
        let magenta6 = color_width(0xFFFF00FF, 6.0);
        let yellow2 = color_width(0xFFFFFF00, 2.0);
        let yellow_none0 = BorderSide {
            color: Color::new(0xFFFFFF00),
            width: 0.0,
            style: BorderStyle::None,
            ..BorderSide::default()
        };
        assert_eq!(
            Border::merge(
                Border {
                    top: yellow2,
                    ..Border::default()
                },
                Border {
                    right: magenta3,
                    ..Border::default()
                }
            ),
            Border {
                top: yellow2,
                right: magenta3,
                ..Border::default()
            }
        );
        assert_eq!(
            Border::merge(
                Border {
                    bottom: magenta3,
                    ..Border::default()
                },
                Border {
                    bottom: magenta3,
                    ..Border::default()
                }
            ),
            Border {
                bottom: magenta6,
                ..Border::default()
            }
        );
        assert_eq!(
            plus(
                &Border {
                    top: yellow2,
                    ..Border::default()
                },
                &Border {
                    right: magenta3,
                    ..Border::default()
                }
            )
            .as_any()
            .downcast_ref::<Border>()
            .copied()
            .unwrap(),
            Border {
                top: yellow2,
                right: magenta3,
                ..Border::default()
            }
        );
        assert!(
            plus(
                &Border {
                    left: magenta3,
                    ..Border::default()
                },
                &Border {
                    left: yellow2,
                    ..Border::default()
                }
            )
            .as_any()
            .downcast_ref::<Border>()
            .is_none()
        );

        let b3 = Border {
            left: magenta3,
            ..Border::default()
        };
        let b6 = Border {
            left: magenta6,
            ..Border::default()
        };
        assert_eq!(b3.scale(2.0), b6);
        let b_y0 = Border {
            top: yellow_none0,
            ..Border::default()
        };
        assert_eq!(b_y0.scale(3.0), b_y0);
        let b_y2 = Border {
            top: yellow2,
            ..Border::default()
        };
        assert_eq!(b_y2.scale(0.0), b_y0);
    }

    #[test]
    #[should_panic]
    fn merge_conflicting_colors_panics() {
        let magenta3 = color_width(0xFFFF00FF, 3.0);
        let yellow2 = color_width(0xFFFFFF00, 2.0);
        let _ = Border::merge(
            Border {
                left: magenta3,
                ..Border::default()
            },
            Border {
                left: yellow2,
                ..Border::default()
            },
        );
    }

    #[test]
    fn dimensions_and_is_uniform() {
        assert_eq!(
            Border {
                left: width(2.0),
                top: width(3.0),
                bottom: width(5.0),
                right: width(7.0),
            }
            .dimensions(),
            EdgeInsets::from_ltrb(2.0, 3.0, 7.0, 5.0).into()
        );
        assert!(
            !Border {
                left: width(3.0),
                top: width(3.0),
                right: width(3.0),
                bottom: width(3.1),
            }
            .is_uniform()
        );
        assert!(
            Border {
                left: width(3.0),
                top: width(3.0),
                right: width(3.0),
                bottom: width(3.0),
            }
            .is_uniform()
        );
        assert!(Border::default().is_uniform());

        let inside = border_all(10.0);
        assert_eq!(inside.dimensions(), EdgeInsets::all(10.0).into());
        let center = Border::all(
            Color::new(0xFF000000),
            10.0,
            BorderStyle::Solid,
            BorderSide::STROKE_ALIGN_CENTER,
        );
        assert_eq!(center.dimensions(), EdgeInsets::all(5.0).into());
        let outside = Border::all(
            Color::new(0xFF000000),
            10.0,
            BorderStyle::Solid,
            BorderSide::STROKE_ALIGN_OUTSIDE,
        );
        assert_eq!(outside.dimensions(), EdgeInsets::ZERO.into());
    }

    #[test]
    fn lerp() {
        let visual_with_top10 = Border {
            top: width(10.0),
            ..Border::default()
        };
        let at_minus100 = Border {
            left: width(0.0),
            right: width(300.0),
            ..Border::default()
        };
        let at0 = Border {
            left: width(100.0),
            right: width(200.0),
            ..Border::default()
        };
        let at25 = Border {
            left: width(125.0),
            right: width(175.0),
            ..Border::default()
        };
        let at75 = Border {
            left: width(175.0),
            right: width(125.0),
            ..Border::default()
        };
        let at100 = Border {
            left: width(200.0),
            right: width(100.0),
            ..Border::default()
        };
        let at200 = Border {
            left: width(300.0),
            right: width(0.0),
            ..Border::default()
        };

        assert!(Border::lerp(None, None, -1.0).is_none());
        assert_eq!(
            Border::lerp(Some(visual_with_top10), None, -1.0),
            Some(Border {
                top: width(20.0),
                ..Border::default()
            })
        );
        assert_eq!(
            Border::lerp(None, Some(visual_with_top10), -1.0),
            Some(Border::default())
        );
        assert_eq!(
            Border::lerp(Some(at0), Some(at100), -1.0),
            Some(at_minus100)
        );
        assert_eq!(Border::lerp(Some(at0), Some(at100), 0.25), Some(at25));
        assert_eq!(Border::lerp(Some(at0), Some(at100), 0.75), Some(at75));
        assert_eq!(Border::lerp(Some(at0), Some(at100), 2.0), Some(at200));
    }

    #[test]
    #[should_panic(
        expected = "A Border can only draw strokeAlign different than BorderSide.strokeAlignInside on borders with uniform colors."
    )]
    fn paint_nonuniform_stroke_align_panics() {
        let mut canvas = Canvas::new();
        ShapeBorder::paint(
            &Border {
                left: BorderSide {
                    stroke_align: BorderSide::STROKE_ALIGN_CENTER,
                    color: Color::new(0xFF000001),
                    ..BorderSide::default()
                },
                right: BorderSide {
                    stroke_align: BorderSide::STROKE_ALIGN_OUTSIDE,
                    color: Color::new(0xFF000002),
                    ..BorderSide::default()
                },
                ..Border::default()
            },
            &mut canvas,
            Rect::from_ltwh(10.0, 20.0, 30.0, 40.0),
            None,
        );
    }

    #[test]
    fn factories() {
        let side1 = BorderSide::default();
        let side2 = width(2.0);
        let side3 = width(3.0);
        let side4 = width(4.0);
        assert_eq!(
            <dyn BoxBorder>::from_ltrb(side2, side3, side4, side1),
            Border {
                left: side1,
                top: side2,
                right: side3,
                bottom: side4,
            }
        );
        assert_eq!(
            <dyn BoxBorder>::all(
                Color::new(0xFF000000),
                4.0,
                BorderStyle::Solid,
                BorderSide::STROKE_ALIGN_INSIDE,
            ),
            border_all(4.0)
        );
        assert_eq!(
            <dyn BoxBorder>::from_border_side(side3),
            Border::from_border_side(side3)
        );
        assert_eq!(
            <dyn BoxBorder>::symmetric(side3, side2),
            Border::symmetric(side3, side2)
        );
        assert_eq!(
            <dyn BoxBorder>::from_steb(side2, side1, side3, side4),
            BorderDirectional {
                start: side1,
                top: side2,
                end: side3,
                bottom: side4,
            }
        );
    }

    #[test]
    fn box_border_lerp_null_and_mixed() {
        let directional_with_top10 = BorderDirectional {
            top: width(10.0),
            ..BorderDirectional::default()
        };
        let visual_with_top100 = Border {
            top: width(100.0),
            ..Border::default()
        };
        let visual_with_top10 = Border {
            top: width(10.0),
            ..Border::default()
        };
        assert!(<dyn BoxBorder>::lerp(None, None, -1.0).is_none());
        expect_border(
            <dyn BoxBorder>::lerp(Some(&border_all(10.0)), None, -1.0),
            border_all(20.0),
        );
        expect_border(
            <dyn BoxBorder>::lerp(None, Some(&border_all(10.0)), -1.0),
            none_scaled(),
        );
        expect_directional(
            <dyn BoxBorder>::lerp(Some(&directional_with_top10), None, -1.0),
            BorderDirectional {
                top: width(20.0),
                ..BorderDirectional::default()
            },
        );
        expect_border(
            <dyn BoxBorder>::lerp(
                Some(&directional_with_top10),
                Some(&visual_with_top100),
                -1.0,
            ),
            Border::default(),
        );
        expect_border(
            <dyn BoxBorder>::lerp(
                Some(&directional_with_top10),
                Some(&visual_with_top100),
                0.0,
            ),
            visual_with_top10,
        );
        expect_border(
            <dyn BoxBorder>::lerp(
                Some(&directional_with_top10),
                Some(&visual_with_top100),
                0.25,
            ),
            Border {
                top: width(32.5),
                ..Border::default()
            },
        );

        let silly = SillyBorder;
        let empty = Border::default();
        let at_neg = <dyn BoxBorder>::lerp(Some(&silly), Some(&empty), -1.0).unwrap();
        assert!(at_neg.eq_shape(&silly));
        let at_075 = <dyn BoxBorder>::lerp(Some(&silly), Some(&empty), 0.75).unwrap();
        assert!(at_075.eq_shape(&none_scaled()));
    }

    #[test]
    fn directional_merge_add_scale_lerp() {
        let magenta3 = color_width(0xFFFF00FF, 3.0);
        let magenta6 = color_width(0xFFFF00FF, 6.0);
        let yellow2 = color_width(0xFFFFFF00, 2.0);
        assert_eq!(
            BorderDirectional::merge(
                BorderDirectional {
                    top: yellow2,
                    ..BorderDirectional::default()
                },
                BorderDirectional {
                    end: magenta3,
                    ..BorderDirectional::default()
                }
            ),
            BorderDirectional {
                top: yellow2,
                end: magenta3,
                ..BorderDirectional::default()
            }
        );
        assert_eq!(
            plus(
                &Border {
                    left: color_width(0x11111111, 1.0),
                    ..Border::default()
                },
                &BorderDirectional {
                    top: color_width(0x22222222, 1.0),
                    ..BorderDirectional::default()
                }
            )
            .as_any()
            .downcast_ref::<Border>()
            .copied()
            .unwrap(),
            Border {
                left: color_width(0x11111111, 1.0),
                top: color_width(0x22222222, 1.0),
                ..Border::default()
            }
        );

        let b3 = BorderDirectional {
            start: magenta3,
            ..BorderDirectional::default()
        };
        let b6 = BorderDirectional {
            start: magenta6,
            ..BorderDirectional::default()
        };
        assert_eq!(b3.scale(2.0), b6);

        let at0 = BorderDirectional {
            start: width(100.0),
            end: width(200.0),
            ..BorderDirectional::default()
        };
        let at100 = BorderDirectional {
            start: width(200.0),
            end: width(100.0),
            ..BorderDirectional::default()
        };
        assert_eq!(
            BorderDirectional::lerp(Some(at0), Some(at100), 0.25),
            Some(BorderDirectional {
                start: width(125.0),
                end: width(175.0),
                ..BorderDirectional::default()
            })
        );
    }

    #[test]
    fn inner_and_outer_paths() {
        let border = Border {
            top: width(10.0),
            right: width(20.0),
            ..Border::default()
        };
        let rect = Rect::from_ltrb(50.0, 60.0, 110.0, 190.0);
        let outer = border.get_outer_path(rect, Some(TextDirection::Rtl));
        assert!(contains(&outer, 50.0, 60.0));
        assert!(contains(&outer, 110.0, 190.0));
        assert!(!contains(&outer, 40.0, 60.0));
        let inner = border.get_inner_path(rect, Some(TextDirection::Rtl));
        assert!(contains(&inner, 50.0, 70.0));
        assert!(!contains(&inner, 50.0, 60.0));

        let border_directional = BorderDirectional {
            top: width(10.0),
            end: width(20.0),
            ..BorderDirectional::default()
        };
        let inner_rtl = border_directional.get_inner_path(rect, Some(TextDirection::Rtl));
        assert!(contains(&inner_rtl, 70.0, 70.0));
        assert!(!contains(&inner_rtl, 50.0, 70.0));
        let inner_ltr = border_directional.get_inner_path(rect, Some(TextDirection::Ltr));
        assert!(contains(&inner_ltr, 50.0, 70.0));
        assert!(!contains(&inner_ltr, 110.0, 80.0));
    }
}
