//! Flutter counterpart: `painting/borders.dart`.

use std::any::Any;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

use reveal_embedder::{Canvas, FillRule, Paint, PaintStyle, Path, PathBuilder, Stroke};
use reveal_embedder::{Color, Offset, Rect, lerp_double};

use crate::basic_types::TextDirection;
use crate::edge_insets::EdgeInsetsGeometry;

/// The style of line to draw for a [`BorderSide`] in a `Border`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    /// Skip the border.
    None,
    /// Draw the border as a solid line.
    Solid,
}

/// A side of a border of a box.
///
/// A `Border` consists of four [`BorderSide`] objects.
///
/// Setting [`width`](BorderSide::width) to 0.0 will result in hairline rendering;
/// see [`width`](BorderSide::width) for a more involved explanation.
#[derive(Clone, Copy, PartialEq)]
pub struct BorderSide {
    /// The color of this side of the border.
    pub color: Color,
    /// The width of this side of the border, in logical pixels.
    ///
    /// Setting width to 0.0 will result in a hairline border. This means that
    /// the border will have the width of one physical pixel. Hairline
    /// rendering takes shortcuts when the path overlaps a pixel more than once.
    /// This means that it will render faster than otherwise, but it might
    /// double-hit pixels, giving it a slightly darker/lighter result.
    ///
    /// To omit the border entirely, set the [`style`](BorderSide::style) to
    /// [`BorderStyle::None`].
    pub width: f64,
    /// The style of this side of the border.
    ///
    /// To omit a side, set [`style`](BorderSide::style) to [`BorderStyle::None`].
    /// This skips painting the border, but the border still has a
    /// [`width`](BorderSide::width).
    pub style: BorderStyle,
    /// The relative position of the stroke on a [`BorderSide`] in an
    /// `OutlinedBorder` or `Border`.
    ///
    /// Values typically range from -1.0 ([`STROKE_ALIGN_INSIDE`](Self::STROKE_ALIGN_INSIDE), inside border,
    /// default) to 1.0 ([`STROKE_ALIGN_OUTSIDE`](Self::STROKE_ALIGN_OUTSIDE), outside border), without any
    /// bound constraints. A value of 0 ([`STROKE_ALIGN_CENTER`](Self::STROKE_ALIGN_CENTER)) will center the
    /// border on the edge of the widget.
    ///
    /// This property is not honored by [`to_paint`](Self::to_paint) (because the
    /// [`Paint`] object cannot represent it); it is intended that classes that
    /// use [`BorderSide`] objects implement this property when painting borders
    /// by suitably inflating or deflating their regions.
    pub stroke_align: f64,
}

impl BorderSide {
    /// The border is drawn fully inside of the border path.
    pub const STROKE_ALIGN_INSIDE: f64 = -1.0;
    /// The border is drawn on the center of the border path.
    pub const STROKE_ALIGN_CENTER: f64 = 0.0;
    /// The border is drawn on the outside of the border path.
    pub const STROKE_ALIGN_OUTSIDE: f64 = 1.0;

    /// A hairline black border that is not rendered.
    pub const NONE: BorderSide = BorderSide {
        color: Color::new(0xFF000000),
        width: 0.0,
        style: BorderStyle::None,
        stroke_align: Self::STROKE_ALIGN_INSIDE,
    };

    /// Creates the side of a border.
    ///
    /// By default, the border is 1.0 logical pixels wide and solid black.
    pub fn new(color: Color, width: f64, style: BorderStyle, stroke_align: f64) -> BorderSide {
        debug_assert!(width >= 0.0);
        BorderSide {
            color,
            width,
            style,
            stroke_align,
        }
    }

    /// Creates a [`BorderSide`] that represents the addition of the two given
    /// [`BorderSide`]s.
    ///
    /// It is only valid to call this if [`can_merge`](Self::can_merge) returns
    /// true for the two sides.
    ///
    /// If one of the sides is zero-width with [`BorderStyle::None`], then the
    /// other side is returned as-is. If both of the sides are zero-width with
    /// [`BorderStyle::None`], then [`BorderSide::NONE`] is returned.
    pub fn merge(a: BorderSide, b: BorderSide) -> BorderSide {
        debug_assert!(Self::can_merge(a, b));
        let a_is_none = a.style == BorderStyle::None && a.width == 0.0;
        let b_is_none = b.style == BorderStyle::None && b.width == 0.0;
        if a_is_none && b_is_none {
            return BorderSide::NONE;
        }
        if a_is_none {
            return b;
        }
        if b_is_none {
            return a;
        }
        debug_assert_eq!(a.color, b.color);
        debug_assert_eq!(a.style, b.style);
        BorderSide {
            color: a.color, // == b.color
            width: a.width + b.width,
            stroke_align: a.stroke_align.max(b.stroke_align),
            style: a.style, // == b.style
        }
    }

    /// Creates a copy of this border but with the given fields replaced with the new values.
    pub fn copy_with(
        &self,
        color: Option<Color>,
        width: Option<f64>,
        style: Option<BorderStyle>,
        stroke_align: Option<f64>,
    ) -> BorderSide {
        BorderSide {
            color: color.unwrap_or(self.color),
            width: width.unwrap_or(self.width),
            style: style.unwrap_or(self.style),
            stroke_align: stroke_align.unwrap_or(self.stroke_align),
        }
    }

    /// Creates a copy of this border side description but with the width scaled
    /// by the factor `t`.
    ///
    /// Since a zero width is normally painted as a hairline width rather than no
    /// border at all, the zero factor is special-cased to instead change the
    /// style to [`BorderStyle::None`].
    ///
    /// Negative values are treated like zero.
    pub fn scale(&self, t: f64) -> BorderSide {
        BorderSide {
            color: self.color,
            width: (self.width * t).max(0.0),
            style: if t <= 0.0 {
                BorderStyle::None
            } else {
                self.style
            },
            stroke_align: Self::STROKE_ALIGN_INSIDE,
        }
    }

    /// Create a [`Paint`] object that, if used to stroke a line, will draw the
    /// line in this border's style.
    ///
    /// The [`stroke_align`](Self::stroke_align) property is not reflected in the
    /// [`Paint`]; consumers must implement that directly by inflating or
    /// deflating their region appropriately.
    pub fn to_paint(&self) -> Paint {
        match self.style {
            BorderStyle::Solid => Paint {
                color: self.color.into(),
                style: PaintStyle::Stroke(Stroke::new(self.width as f32)),
                ..Paint::default()
            },
            BorderStyle::None => Paint {
                color: Color::new(0x00000000).into(),
                style: PaintStyle::Stroke(Stroke::new(0.0)),
                ..Paint::default()
            },
        }
    }

    /// Whether the two given [`BorderSide`]s can be merged using [`merge`](Self::merge).
    ///
    /// Two sides can be merged if one or both are zero-width with
    /// [`BorderStyle::None`], or if they both have the same color and style.
    pub fn can_merge(a: BorderSide, b: BorderSide) -> bool {
        if (a.style == BorderStyle::None && a.width == 0.0)
            || (b.style == BorderStyle::None && b.width == 0.0)
        {
            return true;
        }
        a.style == b.style && a.color == b.color
    }

    /// Linearly interpolate between two border sides.
    pub fn lerp(a: BorderSide, b: BorderSide, t: f64) -> BorderSide {
        if a == b {
            return a;
        }
        if t == 0.0 {
            return a;
        }
        if t == 1.0 {
            return b;
        }
        let width = lerp_double(Some(a.width), Some(b.width), t).unwrap();
        if width < 0.0 {
            return BorderSide::NONE;
        }
        if a.style == b.style && a.stroke_align == b.stroke_align {
            return BorderSide {
                color: Color::lerp(Some(a.color), Some(b.color), t).unwrap(),
                width,
                style: a.style,               // == b.style
                stroke_align: a.stroke_align, // == b.stroke_align
            };
        }
        let color_a = match a.style {
            BorderStyle::Solid => a.color,
            BorderStyle::None => a.color.with_alpha(0x00),
        };
        let color_b = match b.style {
            BorderStyle::Solid => b.color,
            BorderStyle::None => b.color.with_alpha(0x00),
        };
        if a.stroke_align != b.stroke_align {
            return BorderSide {
                color: Color::lerp(Some(color_a), Some(color_b), t).unwrap(),
                width,
                stroke_align: lerp_double(Some(a.stroke_align), Some(b.stroke_align), t).unwrap(),
                style: BorderStyle::Solid,
            };
        }
        BorderSide {
            color: Color::lerp(Some(color_a), Some(color_b), t).unwrap(),
            width,
            stroke_align: a.stroke_align, // == b.stroke_align
            style: BorderStyle::Solid,
        }
    }

    /// Get the amount of the stroke width that lies inside of the [`BorderSide`].
    pub fn stroke_inset(&self) -> f64 {
        self.width * (1.0 - (1.0 + self.stroke_align) / 2.0)
    }

    /// Get the amount of the stroke width that lies outside of the [`BorderSide`].
    pub fn stroke_outset(&self) -> f64 {
        self.width * (1.0 + self.stroke_align) / 2.0
    }

    /// The offset of the stroke, taking into account the stroke alignment.
    pub fn stroke_offset(&self) -> f64 {
        self.width * self.stroke_align
    }
}

impl Default for BorderSide {
    fn default() -> BorderSide {
        BorderSide::new(
            Color::new(0xFF000000),
            1.0,
            BorderStyle::Solid,
            Self::STROKE_ALIGN_INSIDE,
        )
    }
}

impl Debug for BorderSide {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "BorderSide(color: {:?}, width: {:.1})",
            self.color, self.width
        )
    }
}

/// Base class for shape outlines.
///
/// This class handles how to add multiple borders together. Subclasses define
/// various shapes, like circles (`CircleBorder`), rounded rectangles
/// (`RoundedRectangleBorder`), continuous rectangles
/// (`ContinuousRectangleBorder`), or beveled rectangles
/// (`BeveledRectangleBorder`).
pub trait ShapeBorder: Any + Debug {
    /// The widths of the sides of this border represented as an [`EdgeInsetsGeometry`].
    fn dimensions(&self) -> EdgeInsetsGeometry;

    /// Attempts to create a new object that represents the amalgamation of
    /// `this` border and the `other` border.
    ///
    /// If the type of the other border isn't known, or the given instance cannot
    /// be reasonably added to this instance, then this should return None.
    ///
    /// The `reversed` argument is true if this object was the right operand of
    /// `plus`, and false if it was the left operand.
    fn add(&self, other: &dyn ShapeBorder, reversed: bool) -> Option<Box<dyn ShapeBorder>> {
        let _ = (other, reversed);
        None
    }

    /// Creates a copy of this border, scaled by the factor `t`.
    fn scale(&self, t: f64) -> Box<dyn ShapeBorder>;

    /// Linearly interpolates from another [`ShapeBorder`] (possibly of another
    /// class) to `this`.
    ///
    /// Return None if this class cannot interpolate from `a`. If `a` is None,
    /// this must not return None.
    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if a.is_none() {
            Some(self.scale(t))
        } else {
            None
        }
    }

    /// Linearly interpolates from `this` to another [`ShapeBorder`] (possibly of
    /// another class).
    ///
    /// Return None if this class cannot interpolate to `b`. If `b` is None,
    /// this must not return None.
    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        if b.is_none() {
            Some(self.scale(1.0 - t))
        } else {
            None
        }
    }

    /// Create a [`Path`] that describes the outer edge of the border.
    fn get_outer_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path>;

    /// Create a [`Path`] that describes the inner edge of the border.
    fn get_inner_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path>;

    /// Tests whether the outer boundary of this border contains `position`.
    fn hit_test(
        &self,
        rect: Rect,
        position: Offset,
        text_direction: Option<TextDirection>,
    ) -> bool {
        self.get_outer_path(rect, text_direction)
            .contains(position.into(), FillRule::NonZero)
    }

    /// Paint a canvas with the appropriate shape.
    ///
    /// On subclasses whose [`prefer_paint_interior`](Self::prefer_paint_interior)
    /// returns true, this should be faster than using `Canvas.draw_path` with
    /// the path provided by [`get_outer_path`](Self::get_outer_path).
    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        text_direction: Option<TextDirection>,
    ) {
        let _ = (canvas, rect, paint, text_direction);
        debug_assert!(
            !self.prefer_paint_interior(),
            "prefer_paint_interior returns true but paint_interior is not implemented."
        );
        debug_assert!(
            false,
            "prefer_paint_interior returns false, so it is an error to call its paint_interior method."
        );
    }

    /// Reports whether [`paint_interior`](Self::paint_interior) is implemented.
    fn prefer_paint_interior(&self) -> bool {
        false
    }

    /// Paints the border within the given [`Rect`] on the given [`Canvas`].
    fn paint(&self, canvas: &mut Canvas, rect: Rect, text_direction: Option<TextDirection>);

    /// A heap clone, so `plus` and lerp can duplicate like Dart.
    fn clone_box(&self) -> Box<dyn ShapeBorder>;

    /// Downcast support for `is` / `as` in Dart.
    fn as_any(&self) -> &dyn Any;

    /// [`OutlinedBorder`] downcast for [`dyn OutlinedBorder::lerp`].
    fn as_outlined_border(&self) -> Option<&dyn OutlinedBorder> {
        None
    }

    /// `BoxBorder` downcast for [`dyn BoxBorder::lerp`].
    fn as_box_border(&self) -> Option<&dyn crate::box_border::BoxBorder> {
        None
    }

    /// Field equality. Dart's default `==` is identity; subclasses that override
    /// `==` implement this.
    fn eq_shape(&self, other: &dyn ShapeBorder) -> bool {
        std::ptr::eq(self.as_any(), other.as_any())
    }
}

impl<T: ShapeBorder + 'static> From<T> for Box<dyn ShapeBorder> {
    fn from(border: T) -> Box<dyn ShapeBorder> {
        Box::new(border)
    }
}

impl dyn ShapeBorder {
    /// Creates a new border consisting of the two borders on either side of the
    /// operator.
    ///
    /// If the borders belong to classes that know how to add themselves, then
    /// this results in a new border that represents the intelligent addition of
    /// those two borders (see [`ShapeBorder::add`]). Otherwise, an object is
    /// returned that merely paints the two borders sequentially, with the left
    /// hand operand on the inside and the right hand operand on the outside.
    pub fn plus(&self, other: &dyn ShapeBorder) -> Box<dyn ShapeBorder> {
        self.add(other, false)
            .or_else(|| other.add(self, true))
            .unwrap_or_else(|| {
                Box::new(CompoundBorder::new(vec![
                    other.clone_box(),
                    self.clone_box(),
                ]))
            })
    }

    /// Linearly interpolates between two [`ShapeBorder`]s.
    pub fn lerp(
        a: Option<&dyn ShapeBorder>,
        b: Option<&dyn ShapeBorder>,
        t: f64,
    ) -> Option<Box<dyn ShapeBorder>> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone_box()),
            _ => {}
        }
        b.and_then(|b| b.lerp_from(a, t))
            .or_else(|| a.and_then(|a| a.lerp_to(b, t)))
            .or_else(|| b.and_then(|b| b.lerp_to(a, 1.0 - t)))
            .or_else(|| a.and_then(|a| a.lerp_from(b, 1.0 - t)))
            .or_else(|| {
                if t < 0.5 {
                    a.map(ShapeBorder::clone_box)
                } else {
                    b.map(ShapeBorder::clone_box)
                }
            })
    }
}

impl Clone for Box<dyn ShapeBorder> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl PartialEq for dyn ShapeBorder {
    fn eq(&self, other: &dyn ShapeBorder) -> bool {
        self.eq_shape(other)
    }
}

/// A [`ShapeBorder`] that draws an outline with the width and color specified
/// by [`side`](Self::side).
pub trait OutlinedBorder: ShapeBorder {
    /// The border outline's color and weight.
    fn side(&self) -> BorderSide;

    /// Returns a copy of this [`OutlinedBorder`] that draws its outline with the
    /// specified [`side`](Self::side), if `side` is Some.
    fn copy_with(&self, side: Option<BorderSide>) -> Box<dyn OutlinedBorder>;

    /// A heap clone typed as [`OutlinedBorder`].
    fn clone_outlined(&self) -> Box<dyn OutlinedBorder>;
}

impl dyn OutlinedBorder {
    /// Linearly interpolates between two [`OutlinedBorder`]s.
    pub fn lerp(
        a: Option<&dyn OutlinedBorder>,
        b: Option<&dyn OutlinedBorder>,
        t: f64,
    ) -> Option<Box<dyn OutlinedBorder>> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone_outlined()),
            _ => {}
        }
        let result = <dyn ShapeBorder>::lerp(
            a.map(|border| border as &dyn ShapeBorder),
            b.map(|border| border as &dyn ShapeBorder),
            t,
        )?;
        if let Some(outlined) = result.as_outlined_border() {
            Some(outlined.clone_outlined())
        } else if t < 0.5 {
            a.map(OutlinedBorder::clone_outlined)
        } else {
            b.map(OutlinedBorder::clone_outlined)
        }
    }
}

impl Clone for Box<dyn OutlinedBorder> {
    fn clone(&self) -> Self {
        self.clone_outlined()
    }
}

/// Dimensions for an [`OutlinedBorder`]: Dart's inherited `dimensions` getter.
pub fn outlined_border_dimensions(side: BorderSide) -> EdgeInsetsGeometry {
    EdgeInsetsGeometry::all(side.stroke_inset().max(0.0))
}

/// Represents the addition of two otherwise-incompatible borders.
///
/// The borders are listed from the outside to the inside.
struct CompoundBorder {
    borders: Vec<Box<dyn ShapeBorder>>,
}

impl CompoundBorder {
    fn new(borders: Vec<Box<dyn ShapeBorder>>) -> CompoundBorder {
        debug_assert!(borders.len() >= 2);
        debug_assert!(
            borders
                .iter()
                .all(|border| border.as_any().downcast_ref::<CompoundBorder>().is_none())
        );
        CompoundBorder { borders }
    }

    fn lerp(a: Option<&dyn ShapeBorder>, b: Option<&dyn ShapeBorder>, t: f64) -> CompoundBorder {
        debug_assert!(
            a.is_some_and(|border| border.as_any().downcast_ref::<CompoundBorder>().is_some())
                || b.is_some_and(|border| border
                    .as_any()
                    .downcast_ref::<CompoundBorder>()
                    .is_some())
        );
        let a_list = compound_or_single(a);
        let b_list = compound_or_single(b);
        let mut results = Vec::new();
        let length = a_list.len().max(b_list.len());
        for index in 0..length {
            let local_a = a_list.get(index).and_then(|slot| slot.as_deref());
            let local_b = b_list.get(index).and_then(|slot| slot.as_deref());
            if let (Some(local_a), Some(local_b)) = (local_a, local_b)
                && let Some(local_result) = local_a
                    .lerp_to(Some(local_b), t)
                    .or_else(|| local_b.lerp_from(Some(local_a), t))
            {
                results.push(local_result);
                continue;
            }
            if let Some(local_b) = local_b {
                results.push(local_b.scale(t));
            }
            if let Some(local_a) = local_a {
                results.push(local_a.scale(1.0 - t));
            }
        }
        CompoundBorder::new(results)
    }
}

fn compound_or_single(border: Option<&dyn ShapeBorder>) -> Vec<Option<Box<dyn ShapeBorder>>> {
    match border.and_then(|border| border.as_any().downcast_ref::<CompoundBorder>()) {
        Some(compound) => compound
            .borders
            .iter()
            .map(|b| Some(b.clone_box()))
            .collect(),
        None => vec![border.map(ShapeBorder::clone_box)],
    }
}

impl ShapeBorder for CompoundBorder {
    fn dimensions(&self) -> EdgeInsetsGeometry {
        self.borders
            .iter()
            .fold(EdgeInsetsGeometry::ZERO, |previous, border| {
                previous.add(border.dimensions())
            })
    }

    fn add(&self, other: &dyn ShapeBorder, reversed: bool) -> Option<Box<dyn ShapeBorder>> {
        if other.as_any().downcast_ref::<CompoundBorder>().is_none() {
            let ours = if reversed {
                &*self.borders[self.borders.len() - 1]
            } else {
                &*self.borders[0]
            };
            let merged = ours
                .add(other, reversed)
                .or_else(|| other.add(ours, !reversed));
            if let Some(merged) = merged {
                let mut result: Vec<Box<dyn ShapeBorder>> =
                    self.borders.iter().map(|b| b.clone_box()).collect();
                let index = if reversed { result.len() - 1 } else { 0 };
                result[index] = merged;
                return Some(Box::new(CompoundBorder::new(result)));
            }
        }
        let mut merged_borders = Vec::new();
        if reversed {
            merged_borders.extend(self.borders.iter().map(|b| b.clone_box()));
        }
        if let Some(other_compound) = other.as_any().downcast_ref::<CompoundBorder>() {
            merged_borders.extend(other_compound.borders.iter().map(|b| b.clone_box()));
        } else {
            merged_borders.push(other.clone_box());
        }
        if !reversed {
            merged_borders.extend(self.borders.iter().map(|b| b.clone_box()));
        }
        Some(Box::new(CompoundBorder::new(merged_borders)))
    }

    fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
        Box::new(CompoundBorder::new(
            self.borders.iter().map(|border| border.scale(t)).collect(),
        ))
    }

    fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        Some(Box::new(CompoundBorder::lerp(a, Some(self), t)))
    }

    fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
        Some(Box::new(CompoundBorder::lerp(Some(self), b, t)))
    }

    fn get_inner_path(&self, mut rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        for border in &self.borders[..self.borders.len() - 1] {
            rect = border
                .dimensions()
                .resolve(text_direction)
                .deflate_rect(rect);
        }
        self.borders[self.borders.len() - 1].get_inner_path(rect, text_direction)
    }

    fn get_outer_path(&self, rect: Rect, text_direction: Option<TextDirection>) -> Arc<Path> {
        self.borders[0].get_outer_path(rect, text_direction)
    }

    fn hit_test(
        &self,
        rect: Rect,
        position: Offset,
        text_direction: Option<TextDirection>,
    ) -> bool {
        self.borders[0].hit_test(rect, position, text_direction)
    }

    fn paint_interior(
        &self,
        canvas: &mut Canvas,
        rect: Rect,
        paint: &Paint,
        text_direction: Option<TextDirection>,
    ) {
        self.borders[0].paint_interior(canvas, rect, paint, text_direction);
    }

    fn prefer_paint_interior(&self) -> bool {
        self.borders
            .iter()
            .all(|border| border.prefer_paint_interior())
    }

    fn paint(&self, canvas: &mut Canvas, mut rect: Rect, text_direction: Option<TextDirection>) {
        for border in &self.borders {
            border.paint(canvas, rect, text_direction);
            rect = border
                .dimensions()
                .resolve(text_direction)
                .deflate_rect(rect);
        }
    }

    fn clone_box(&self) -> Box<dyn ShapeBorder> {
        Box::new(CompoundBorder::new(
            self.borders.iter().map(|b| b.clone_box()).collect(),
        ))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_shape(&self, other: &dyn ShapeBorder) -> bool {
        other
            .as_any()
            .downcast_ref::<CompoundBorder>()
            .is_some_and(|other| {
                self.borders.len() == other.borders.len()
                    && self
                        .borders
                        .iter()
                        .zip(other.borders.iter())
                        .all(|(a, b)| a.eq_shape(&**b))
            })
    }
}

impl Debug for CompoundBorder {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut first = true;
        for border in self.borders.iter().rev() {
            if !first {
                write!(f, " + ")?;
            }
            first = false;
            write!(f, "{border:?}")?;
        }
        Ok(())
    }
}

/// Paints a border around the given rectangle on the canvas.
///
/// The four sides can be independently specified. They are painted in the order
/// top, right, bottom, left. This is only notable if the widths of the borders
/// and the size of the given rectangle are such that the border sides will
/// overlap each other. No effort is made to optimize the rendering of uniform
/// borders (where all the borders have the same configuration); to render a
/// uniform border, consider using `Canvas.draw_rect` directly.
pub fn paint_border(
    canvas: &mut Canvas,
    rect: Rect,
    top: BorderSide,
    right: BorderSide,
    bottom: BorderSide,
    left: BorderSide,
) {
    // We draw the borders as filled shapes, unless the borders are hairline
    // borders, in which case we use a stroke, with the stroke width specified
    // here.
    let mut paint = Paint {
        style: PaintStyle::Stroke(Stroke::new(0.0)),
        ..Paint::default()
    };

    match top.style {
        BorderStyle::Solid => {
            paint.color = top.color.into();
            let mut path = PathBuilder::new();
            path.move_to(Offset::new(rect.left, rect.top));
            path.line_to(Offset::new(rect.right, rect.top));
            if top.width == 0.0 {
                paint.style = PaintStyle::Stroke(Stroke::new(0.0));
            } else {
                paint.style = PaintStyle::Fill;
                path.line_to(Offset::new(rect.right - right.width, rect.top + top.width));
                path.line_to(Offset::new(rect.left + left.width, rect.top + top.width));
            }
            canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
        }
        BorderStyle::None => {}
    }

    match right.style {
        BorderStyle::Solid => {
            paint.color = right.color.into();
            let mut path = PathBuilder::new();
            path.move_to(Offset::new(rect.right, rect.top));
            path.line_to(Offset::new(rect.right, rect.bottom));
            if right.width == 0.0 {
                paint.style = PaintStyle::Stroke(Stroke::new(0.0));
            } else {
                paint.style = PaintStyle::Fill;
                path.line_to(Offset::new(
                    rect.right - right.width,
                    rect.bottom - bottom.width,
                ));
                path.line_to(Offset::new(rect.right - right.width, rect.top + top.width));
            }
            canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
        }
        BorderStyle::None => {}
    }

    match bottom.style {
        BorderStyle::Solid => {
            paint.color = bottom.color.into();
            let mut path = PathBuilder::new();
            path.move_to(Offset::new(rect.right, rect.bottom));
            path.line_to(Offset::new(rect.left, rect.bottom));
            if bottom.width == 0.0 {
                paint.style = PaintStyle::Stroke(Stroke::new(0.0));
            } else {
                paint.style = PaintStyle::Fill;
                path.line_to(Offset::new(
                    rect.left + left.width,
                    rect.bottom - bottom.width,
                ));
                path.line_to(Offset::new(
                    rect.right - right.width,
                    rect.bottom - bottom.width,
                ));
            }
            canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
        }
        BorderStyle::None => {}
    }

    match left.style {
        BorderStyle::Solid => {
            paint.color = left.color.into();
            let mut path = PathBuilder::new();
            path.move_to(Offset::new(rect.left, rect.bottom));
            path.line_to(Offset::new(rect.left, rect.top));
            if left.width == 0.0 {
                paint.style = PaintStyle::Stroke(Stroke::new(0.0));
            } else {
                paint.style = PaintStyle::Fill;
                path.line_to(Offset::new(rect.left + left.width, rect.top + top.width));
                path.line_to(Offset::new(
                    rect.left + left.width,
                    rect.bottom - bottom.width,
                ));
            }
            canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
        }
        BorderStyle::None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side(color: Color, width: f64) -> BorderSide {
        BorderSide {
            color,
            width,
            ..BorderSide::default()
        }
    }

    #[test]
    fn merging() {
        let blue = BorderSide {
            color: Color::new(0xFF0000FF),
            ..BorderSide::default()
        };
        let blue2 = BorderSide {
            color: Color::new(0xFF0000FF),
            width: 2.0,
            ..BorderSide::default()
        };
        let green = BorderSide {
            color: Color::new(0xFF00FF00),
            ..BorderSide::default()
        };
        let green2 = BorderSide {
            color: Color::new(0xFF00FF00),
            width: 2.0,
            ..BorderSide::default()
        };
        let green3 = BorderSide {
            color: Color::new(0xFF00FF00),
            width: 3.0,
            ..BorderSide::default()
        };
        let green5 = BorderSide {
            color: Color::new(0xFF00FF00),
            width: 5.0,
            ..BorderSide::default()
        };
        let none = BorderSide {
            style: BorderStyle::None,
            ..BorderSide::default()
        };
        let none2 = BorderSide {
            color: Color::new(0xFF0000FF),
            width: 2.0,
            style: BorderStyle::None,
            ..BorderSide::default()
        };
        let none3 = BorderSide {
            style: BorderStyle::None,
            width: 3.0,
            ..BorderSide::default()
        };
        let side2 = BorderSide {
            width: 2.0,
            ..BorderSide::default()
        };
        let side3 = BorderSide {
            width: 3.0,
            ..BorderSide::default()
        };
        let side5 = BorderSide {
            width: 5.0,
            ..BorderSide::default()
        };
        let solid = BorderSide::default();
        let yellow_none = BorderSide {
            style: BorderStyle::None,
            color: Color::new(0xFFFFFF00),
            width: 0.0,
            ..BorderSide::default()
        };

        assert!(BorderSide::can_merge(BorderSide::NONE, BorderSide::NONE));
        assert!(BorderSide::can_merge(BorderSide::NONE, side2));
        assert!(BorderSide::can_merge(BorderSide::NONE, yellow_none));
        assert!(!BorderSide::can_merge(green, blue));
        assert!(!BorderSide::can_merge(green2, blue2));
        assert!(BorderSide::can_merge(green2, green3));
        assert!(!BorderSide::can_merge(green2, none2));
        assert!(BorderSide::can_merge(none3, BorderSide::NONE));
        assert!(!BorderSide::can_merge(none3, side2));
        assert!(BorderSide::can_merge(none3, yellow_none));
        assert!(BorderSide::can_merge(side2, BorderSide::NONE));
        assert!(!BorderSide::can_merge(side2, none3));
        assert!(BorderSide::can_merge(side2, side3));
        assert!(BorderSide::can_merge(side2, yellow_none));
        assert!(BorderSide::can_merge(side3, side2));
        assert!(!BorderSide::can_merge(solid, none));
        assert!(BorderSide::can_merge(yellow_none, side2));
        assert!(BorderSide::can_merge(yellow_none, yellow_none));

        assert_eq!(
            BorderSide::merge(BorderSide::NONE, BorderSide::NONE),
            BorderSide::NONE
        );
        assert_eq!(BorderSide::merge(BorderSide::NONE, side2), side2);
        assert_eq!(
            BorderSide::merge(BorderSide::NONE, yellow_none),
            BorderSide::NONE
        );
        assert_eq!(BorderSide::merge(green2, green3), green5);
        assert_eq!(BorderSide::merge(none3, BorderSide::NONE), none3);
        assert_eq!(BorderSide::merge(none3, yellow_none), none3);
        assert_eq!(BorderSide::merge(side2, BorderSide::NONE), side2);
        assert_eq!(BorderSide::merge(side2, side3), side5);
        assert_eq!(BorderSide::merge(side2, yellow_none), side2);
        assert_eq!(BorderSide::merge(side3, side2), side5);
        assert_eq!(BorderSide::merge(yellow_none, side2), side2);
        assert_eq!(
            BorderSide::merge(yellow_none, yellow_none),
            BorderSide::NONE
        );
    }

    #[test]
    fn copy_with_and_scale() {
        let green2 = BorderSide {
            color: Color::new(0xFF00FF00),
            width: 2.0,
            ..BorderSide::default()
        };
        let blue3 = BorderSide {
            color: Color::new(0xFF0000FF),
            width: 3.0,
            ..BorderSide::default()
        };
        let blue2 = BorderSide {
            color: Color::new(0xFF0000FF),
            width: 2.0,
            ..BorderSide::default()
        };
        let green3 = BorderSide {
            color: Color::new(0xFF00FF00),
            width: 3.0,
            ..BorderSide::default()
        };
        let none2 = BorderSide {
            color: Color::new(0xFF00FF00),
            width: 2.0,
            style: BorderStyle::None,
            ..BorderSide::default()
        };
        assert_eq!(
            green2.copy_with(Some(Color::new(0xFF0000FF)), Some(3.0), None, None),
            blue3
        );
        assert_eq!(green2.copy_with(None, Some(3.0), None, None), green3);
        assert_eq!(
            green2.copy_with(Some(Color::new(0xFF0000FF)), None, None, None),
            blue2
        );
        assert_eq!(
            green2.copy_with(None, None, Some(BorderStyle::None), None),
            none2
        );

        let side3 = BorderSide {
            width: 3.0,
            color: Color::new(0xFF0000FF),
            ..BorderSide::default()
        };
        let side6 = BorderSide {
            width: 6.0,
            color: Color::new(0xFF0000FF),
            ..BorderSide::default()
        };
        let none = BorderSide {
            style: BorderStyle::None,
            width: 0.0,
            color: Color::new(0xFF0000FF),
            ..BorderSide::default()
        };
        assert_eq!(side3.scale(2.0), side6);
        assert_eq!(side6.scale(0.5), side3);
        assert_eq!(side6.scale(0.0), none);
        assert_eq!(side6.scale(-1.0), none);
        assert_eq!(none.scale(2.0), none);
    }

    #[test]
    fn to_paint_stroke_and_none() {
        let paint1 = BorderSide {
            width: 2.5,
            color: Color::new(0xFFFFFF00),
            ..BorderSide::default()
        }
        .to_paint();
        assert_eq!(paint1.style, PaintStyle::Stroke(Stroke::new(2.5)));
        assert_eq!(paint1.color, Color::new(0xFFFFFF00).into());
        assert_eq!(paint1.blend_mode, reveal_embedder::BlendMode::SrcOver);

        let paint2 = BorderSide {
            width: 2.5,
            color: Color::new(0xFFFFFF00),
            style: BorderStyle::None,
            ..BorderSide::default()
        }
        .to_paint();
        assert_eq!(paint2.style, PaintStyle::Stroke(Stroke::new(0.0)));
        assert_eq!(paint2.color, Color::new(0x00000000).into());
        assert_eq!(paint2.blend_mode, reveal_embedder::BlendMode::SrcOver);
    }

    #[test]
    fn lerp_identical_and_negative_width() {
        let border = BorderSide::default();
        assert_eq!(BorderSide::lerp(border, border, 0.5), border);

        let side0 = BorderSide {
            width: 0.0,
            ..BorderSide::default()
        };
        let side1 = BorderSide::default();
        let side2 = BorderSide {
            width: 2.0,
            ..BorderSide::default()
        };
        assert_eq!(BorderSide::lerp(side2, side1, 10.0), BorderSide::NONE);
        assert_eq!(BorderSide::lerp(side1, side2, -10.0), BorderSide::NONE);
        assert_eq!(BorderSide::lerp(side0, side1, 2.0), side2);
        assert_eq!(BorderSide::lerp(side1, side0, 2.0), BorderSide::NONE);
        assert_eq!(BorderSide::lerp(side2, side1, 2.0), side0);
    }

    #[test]
    fn lerp_stroke_align() {
        let side0 = BorderSide {
            width: 2.0,
            ..BorderSide::default()
        };
        let side1 = BorderSide {
            width: 2.0,
            stroke_align: BorderSide::STROKE_ALIGN_OUTSIDE,
            ..BorderSide::default()
        };
        assert_eq!(
            BorderSide::lerp(side0, side1, 0.0),
            BorderSide {
                width: 2.0,
                ..BorderSide::default()
            }
        );
        assert_eq!(
            BorderSide::lerp(side0, side1, 0.5),
            BorderSide {
                width: 2.0,
                stroke_align: BorderSide::STROKE_ALIGN_CENTER,
                ..BorderSide::default()
            }
        );
        assert_eq!(
            BorderSide::lerp(side0, side1, 1.0),
            BorderSide {
                width: 2.0,
                stroke_align: BorderSide::STROKE_ALIGN_OUTSIDE,
                ..BorderSide::default()
            }
        );

        let side2 = BorderSide {
            width: 2.0,
            ..BorderSide::default()
        };
        let side3 = BorderSide {
            width: 2.0,
            stroke_align: BorderSide::STROKE_ALIGN_CENTER,
            ..BorderSide::default()
        };
        assert_eq!(
            BorderSide::lerp(side2, side3, 0.5),
            BorderSide {
                width: 2.0,
                stroke_align: -0.5,
                ..BorderSide::default()
            }
        );
    }

    #[test]
    fn paint_border_records_one_path_per_solid_side() {
        let mut canvas = Canvas::new();
        paint_border(
            &mut canvas,
            Rect::from_ltrb(0.0, 0.0, 10.0, 10.0),
            side(Color::new(0xFFFF0000), 1.0),
            side(Color::new(0xFF00FF00), 1.0),
            side(Color::new(0xFF0000FF), 1.0),
            side(Color::new(0xFFFFFF00), 1.0),
        );
        assert_eq!(canvas.build().draw_count(), 4);
    }

    fn rect_path(rect: Rect) -> Arc<Path> {
        let mut path = PathBuilder::new();
        path.rect(rect.into());
        path.build()
    }

    /// Flutter `shape_border_test.dart` `_LerpBorder`.
    #[derive(Clone, Copy, Debug)]
    struct LerpBorder {
        t: Option<f64>,
        side: BorderSide,
    }

    impl LerpBorder {
        fn new(t: Option<f64>, side: BorderSide) -> LerpBorder {
            LerpBorder { t, side }
        }
    }

    impl ShapeBorder for LerpBorder {
        fn dimensions(&self) -> EdgeInsetsGeometry {
            outlined_border_dimensions(self.side)
        }

        fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
            Box::new(LerpBorder::new(Some(t), self.side.scale(t)))
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

        fn as_outlined_border(&self) -> Option<&dyn OutlinedBorder> {
            Some(self)
        }

        fn eq_shape(&self, other: &dyn ShapeBorder) -> bool {
            other
                .as_any()
                .downcast_ref::<LerpBorder>()
                .is_some_and(|other| other.t == self.t && other.side == self.side)
        }
    }

    impl OutlinedBorder for LerpBorder {
        fn side(&self) -> BorderSide {
            self.side
        }

        fn copy_with(&self, side: Option<BorderSide>) -> Box<dyn OutlinedBorder> {
            Box::new(LerpBorder::new(self.t, side.unwrap_or(self.side)))
        }

        fn clone_outlined(&self) -> Box<dyn OutlinedBorder> {
            Box::new(*self)
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct ReverseLerpToBorder;

    impl ShapeBorder for ReverseLerpToBorder {
        fn dimensions(&self) -> EdgeInsetsGeometry {
            outlined_border_dimensions(BorderSide::NONE)
        }

        fn lerp_to(&self, b: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
            if b.is_some_and(|b| b.as_any().downcast_ref::<LerpBorder>().is_some()) {
                Some(Box::new(LerpBorder::new(Some(t), BorderSide::NONE)))
            } else if b.is_none() {
                Some(self.scale(1.0 - t))
            } else {
                None
            }
        }

        fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
            Box::new(LerpBorder::new(Some(t), BorderSide::NONE.scale(t)))
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

        fn as_outlined_border(&self) -> Option<&dyn OutlinedBorder> {
            Some(self)
        }
    }

    impl OutlinedBorder for ReverseLerpToBorder {
        fn side(&self) -> BorderSide {
            BorderSide::NONE
        }

        fn copy_with(&self, _side: Option<BorderSide>) -> Box<dyn OutlinedBorder> {
            Box::new(*self)
        }

        fn clone_outlined(&self) -> Box<dyn OutlinedBorder> {
            Box::new(*self)
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct ReverseLerpFromBorder;

    impl ShapeBorder for ReverseLerpFromBorder {
        fn dimensions(&self) -> EdgeInsetsGeometry {
            outlined_border_dimensions(BorderSide::NONE)
        }

        fn lerp_from(&self, a: Option<&dyn ShapeBorder>, t: f64) -> Option<Box<dyn ShapeBorder>> {
            if a.is_some_and(|a| a.as_any().downcast_ref::<LerpBorder>().is_some()) {
                Some(Box::new(LerpBorder::new(Some(t), BorderSide::NONE)))
            } else if a.is_none() {
                Some(self.scale(t))
            } else {
                None
            }
        }

        fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
            Box::new(LerpBorder::new(Some(t), BorderSide::NONE.scale(t)))
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

        fn as_outlined_border(&self) -> Option<&dyn OutlinedBorder> {
            Some(self)
        }
    }

    impl OutlinedBorder for ReverseLerpFromBorder {
        fn side(&self) -> BorderSide {
            BorderSide::NONE
        }

        fn copy_with(&self, _side: Option<BorderSide>) -> Box<dyn OutlinedBorder> {
            Box::new(*self)
        }

        fn clone_outlined(&self) -> Box<dyn OutlinedBorder> {
            Box::new(*self)
        }
    }

    /// A named width so compound `+` / scale / dimensions can be tested without
    /// `Border` / `BoxBorder`.
    #[derive(Clone, Copy)]
    struct NamedWidthBorder {
        name: &'static str,
        width: f64,
    }

    impl Debug for NamedWidthBorder {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            write!(f, "NamedWidthBorder({}, {:.1})", self.name, self.width)
        }
    }

    impl ShapeBorder for NamedWidthBorder {
        fn dimensions(&self) -> EdgeInsetsGeometry {
            EdgeInsetsGeometry::all(self.width)
        }

        fn scale(&self, t: f64) -> Box<dyn ShapeBorder> {
            Box::new(NamedWidthBorder {
                name: self.name,
                width: (self.width * t).max(0.0),
            })
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

        fn eq_shape(&self, other: &dyn ShapeBorder) -> bool {
            other
                .as_any()
                .downcast_ref::<NamedWidthBorder>()
                .is_some_and(|other| other.name == self.name && other.width == self.width)
        }
    }

    #[test]
    fn shape_border_lerp_identical_a_b() {
        assert!(<dyn ShapeBorder>::lerp(None, None, 0.0).is_none());
        let border = LerpBorder::new(None, BorderSide::NONE);
        let as_shape: &dyn ShapeBorder = &border;
        let lerped = <dyn ShapeBorder>::lerp(Some(as_shape), Some(as_shape), 0.5).unwrap();
        assert!(lerped.eq_shape(as_shape));
    }

    #[test]
    fn outlined_border_lerp_identical_a_b() {
        assert!(<dyn OutlinedBorder>::lerp(None, None, 0.0).is_none());
        let border = LerpBorder::new(None, BorderSide::NONE);
        let as_outlined: &dyn OutlinedBorder = &border;
        let lerped = <dyn OutlinedBorder>::lerp(Some(as_outlined), Some(as_outlined), 0.5).unwrap();
        assert!(lerped.eq_shape(as_outlined as &dyn ShapeBorder));
    }

    #[test]
    fn shape_border_lerp_tries_equivalent_reverse_interpolation() {
        let a = LerpBorder::new(None, BorderSide::NONE);
        let b = ReverseLerpToBorder;
        let got = <dyn ShapeBorder>::lerp(Some(&a), Some(&b), 0.25).unwrap();
        assert!(got.eq_shape(&LerpBorder::new(Some(0.75), BorderSide::NONE)));

        let a = ReverseLerpFromBorder;
        let b = LerpBorder::new(None, BorderSide::NONE);
        let got = <dyn ShapeBorder>::lerp(Some(&a), Some(&b), 0.25).unwrap();
        assert!(got.eq_shape(&LerpBorder::new(Some(0.75), BorderSide::NONE)));
    }

    #[test]
    fn outlined_border_lerp_tries_equivalent_reverse_interpolation() {
        let a = LerpBorder::new(None, BorderSide::NONE);
        let b = ReverseLerpToBorder;
        let got = <dyn OutlinedBorder>::lerp(Some(&a), Some(&b), 0.25).unwrap();
        assert!(got.eq_shape(&LerpBorder::new(Some(0.75), BorderSide::NONE)));

        let a = ReverseLerpFromBorder;
        let b = LerpBorder::new(None, BorderSide::NONE);
        let got = <dyn OutlinedBorder>::lerp(Some(&a), Some(&b), 0.25).unwrap();
        assert!(got.eq_shape(&LerpBorder::new(Some(0.75), BorderSide::NONE)));
    }

    #[test]
    fn compound_borders() {
        let b1 = NamedWidthBorder {
            name: "green",
            width: 1.0,
        };
        let b2 = NamedWidthBorder {
            name: "blue",
            width: 1.0,
        };
        let compound = (&b1 as &dyn ShapeBorder).plus(&b2);
        assert_eq!(
            format!("{compound:?}"),
            "NamedWidthBorder(green, 1.0) + NamedWidthBorder(blue, 1.0)"
        );
        assert_eq!(compound.dimensions(), EdgeInsetsGeometry::all(2.0));

        let scaled = compound.scale(3.0);
        assert_eq!(
            format!("{scaled:?}"),
            "NamedWidthBorder(green, 3.0) + NamedWidthBorder(blue, 3.0)"
        );

        let left = (&b1 as &dyn ShapeBorder).plus(&b2);
        let right = (&b1 as &dyn ShapeBorder).plus(&b2);
        assert!(left.eq_shape(&*right));

        let b2b2 = (&b2 as &dyn ShapeBorder).plus(&b2);
        let inner = (&b1 as &dyn ShapeBorder).plus(&*b2b2);
        let outer = (&b1 as &dyn ShapeBorder).plus(&b2).plus(&b2);
        assert!(inner.eq_shape(&*outer));
        assert_eq!(inner.dimensions(), EdgeInsetsGeometry::all(3.0));
    }

    #[test]
    fn shape_border_hit_test_uses_the_outer_path() {
        let border = NamedWidthBorder {
            name: "hit",
            width: 1.0,
        };
        let rect = Rect::from_ltrb(0.0, 0.0, 10.0, 10.0);
        assert!(border.hit_test(rect, Offset::new(5.0, 5.0), None));
        assert!(!border.hit_test(rect, Offset::new(-1.0, 5.0), None));
    }

    #[derive(Clone, Copy, Debug)]
    struct ShapeWithInterior;

    impl ShapeBorder for ShapeWithInterior {
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

        fn paint_interior(
            &self,
            _canvas: &mut Canvas,
            _rect: Rect,
            _paint: &Paint,
            _text_direction: Option<TextDirection>,
        ) {
        }

        fn prefer_paint_interior(&self) -> bool {
            true
        }

        fn paint(&self, _canvas: &mut Canvas, _rect: Rect, _text_direction: Option<TextDirection>) {
        }

        fn clone_box(&self) -> Box<dyn ShapeBorder> {
            Box::new(*self)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct ShapeWithoutInterior;

    impl ShapeBorder for ShapeWithoutInterior {
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
    }

    #[test]
    fn compound_borders_with_differing_prefer_paint_interiors() {
        assert!(ShapeWithInterior.prefer_paint_interior());
        assert!(!ShapeWithoutInterior.prefer_paint_interior());
        assert!(
            (&ShapeWithInterior as &dyn ShapeBorder)
                .plus(&ShapeWithInterior)
                .prefer_paint_interior()
        );
        assert!(
            !(&ShapeWithInterior as &dyn ShapeBorder)
                .plus(&ShapeWithoutInterior)
                .prefer_paint_interior()
        );
        assert!(
            !(&ShapeWithoutInterior as &dyn ShapeBorder)
                .plus(&ShapeWithInterior)
                .prefer_paint_interior()
        );
        assert!(
            !(&ShapeWithoutInterior as &dyn ShapeBorder)
                .plus(&ShapeWithoutInterior)
                .prefer_paint_interior()
        );
    }
}
