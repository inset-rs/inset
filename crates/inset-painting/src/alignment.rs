//! Flutter counterpart: `painting/alignment.dart`.

use std::fmt::{self, Debug};
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

use inset_embedder::{Offset, Rect, Size, lerp_double};

use crate::basic_types::TextDirection;

/// Base class for [`Alignment`] that allows for text-direction aware
/// resolution.
///
/// A property or argument of this type accepts classes created either with
/// [`Alignment`] and its variants, or [`AlignmentDirectional::new`].
///
/// To convert an [`AlignmentGeometry`] object of indeterminate type into an
/// [`Alignment`] object, call the [`resolve`](AlignmentGeometry::resolve)
/// method.
///
/// Non-exhaustive: Flutter may add a subclass.
#[non_exhaustive]
#[derive(Clone, Copy)]
pub enum AlignmentGeometry {
    /// Visual alignment — Dart's `Alignment`.
    Alignment(Alignment),
    /// Directional alignment — Dart's `AlignmentDirectional`.
    Directional(AlignmentDirectional),
    /// A visual and a directional horizontal coordinate at once — Dart's private
    /// `_MixedAlignment`, what combining the two kinds produces.
    Mixed {
        /// The distance fraction in the horizontal direction.
        x: f64,
        /// The distance fraction in the horizontal direction from the start side.
        start: f64,
        /// The distance fraction in the vertical direction.
        y: f64,
    },
}

impl AlignmentGeometry {
    /// Creates an [`Alignment`].
    pub const fn xy(x: f64, y: f64) -> AlignmentGeometry {
        AlignmentGeometry::Alignment(Alignment::new(x, y))
    }

    /// Creates a directional alignment, or [`AlignmentDirectional`].
    pub const fn directional(start: f64, y: f64) -> AlignmentGeometry {
        AlignmentGeometry::Directional(AlignmentDirectional::new(start, y))
    }

    /// The top left corner.
    pub const TOP_LEFT: AlignmentGeometry = AlignmentGeometry::Alignment(Alignment::TOP_LEFT);
    /// The center point along the top edge.
    pub const TOP_CENTER: AlignmentGeometry = AlignmentGeometry::Alignment(Alignment::TOP_CENTER);
    /// The top right corner.
    pub const TOP_RIGHT: AlignmentGeometry = AlignmentGeometry::Alignment(Alignment::TOP_RIGHT);
    /// The top corner on the "start" edge.
    pub const TOP_START: AlignmentGeometry =
        AlignmentGeometry::Directional(AlignmentDirectional::TOP_START);
    /// The top corner on the "end" edge.
    pub const TOP_END: AlignmentGeometry =
        AlignmentGeometry::Directional(AlignmentDirectional::TOP_END);
    /// The center point along the left edge.
    pub const CENTER_LEFT: AlignmentGeometry = AlignmentGeometry::Alignment(Alignment::CENTER_LEFT);
    /// The center point, both horizontally and vertically.
    pub const CENTER: AlignmentGeometry = AlignmentGeometry::Alignment(Alignment::CENTER);
    /// The center point along the right edge.
    pub const CENTER_RIGHT: AlignmentGeometry =
        AlignmentGeometry::Alignment(Alignment::CENTER_RIGHT);
    /// The center point along the "start" edge.
    pub const CENTER_START: AlignmentGeometry =
        AlignmentGeometry::Directional(AlignmentDirectional::CENTER_START);
    /// The center point along the "end" edge.
    pub const CENTER_END: AlignmentGeometry =
        AlignmentGeometry::Directional(AlignmentDirectional::CENTER_END);
    /// The bottom left corner.
    pub const BOTTOM_LEFT: AlignmentGeometry = AlignmentGeometry::Alignment(Alignment::BOTTOM_LEFT);
    /// The center point along the bottom edge.
    pub const BOTTOM_CENTER: AlignmentGeometry =
        AlignmentGeometry::Alignment(Alignment::BOTTOM_CENTER);
    /// The bottom right corner.
    pub const BOTTOM_RIGHT: AlignmentGeometry =
        AlignmentGeometry::Alignment(Alignment::BOTTOM_RIGHT);
    /// The bottom corner on the "start" edge.
    pub const BOTTOM_START: AlignmentGeometry =
        AlignmentGeometry::Directional(AlignmentDirectional::BOTTOM_START);
    /// The bottom corner on the "end" edge.
    pub const BOTTOM_END: AlignmentGeometry =
        AlignmentGeometry::Directional(AlignmentDirectional::BOTTOM_END);

    fn x(&self) -> f64 {
        match self {
            AlignmentGeometry::Alignment(alignment) => alignment.x,
            AlignmentGeometry::Directional(_) => 0.0,
            AlignmentGeometry::Mixed { x, .. } => *x,
        }
    }

    fn start(&self) -> f64 {
        match self {
            AlignmentGeometry::Alignment(_) => 0.0,
            AlignmentGeometry::Directional(alignment) => alignment.start,
            AlignmentGeometry::Mixed { start, .. } => *start,
        }
    }

    fn y(&self) -> f64 {
        match self {
            AlignmentGeometry::Alignment(alignment) => alignment.y,
            AlignmentGeometry::Directional(alignment) => alignment.y,
            AlignmentGeometry::Mixed { y, .. } => *y,
        }
    }

    /// Dart's private `_MixedAlignment`.
    const fn mixed(x: f64, start: f64, y: f64) -> AlignmentGeometry {
        AlignmentGeometry::Mixed { x, start, y }
    }

    /// Returns the sum of two [`AlignmentGeometry`] objects.
    ///
    /// If you know you are adding two [`Alignment`] or two
    /// [`AlignmentDirectional`] objects, consider using the `+` operator
    /// instead, which always returns an object of the same type as the operands,
    /// and is typed accordingly.
    ///
    /// Applied to two objects of the same kind, the result is of that kind;
    /// otherwise it is the mixed arm, which [`resolve`](Self::resolve) turns
    /// into a concrete [`Alignment`].
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, other: AlignmentGeometry) -> AlignmentGeometry {
        match (self, other) {
            (AlignmentGeometry::Alignment(a), AlignmentGeometry::Alignment(b)) => {
                AlignmentGeometry::Alignment(a + b)
            }
            (AlignmentGeometry::Directional(a), AlignmentGeometry::Directional(b)) => {
                AlignmentGeometry::Directional(a + b)
            }
            _ => AlignmentGeometry::mixed(
                self.x() + other.x(),
                self.start() + other.start(),
                self.y() + other.y(),
            ),
        }
    }

    /// Integer divides the [`AlignmentGeometry`] object in each dimension by
    /// the given factor.
    pub fn truncating_div(&self, other: f64) -> AlignmentGeometry {
        match self {
            AlignmentGeometry::Alignment(alignment) => {
                AlignmentGeometry::Alignment(alignment.truncating_div(other))
            }
            AlignmentGeometry::Directional(alignment) => {
                AlignmentGeometry::Directional(alignment.truncating_div(other))
            }
            AlignmentGeometry::Mixed { .. } => AlignmentGeometry::mixed(
                (self.x() / other).trunc(),
                (self.start() / other).trunc(),
                (self.y() / other).trunc(),
            ),
        }
    }

    /// Linearly interpolate between two [`AlignmentGeometry`] objects.
    ///
    /// If either is `None`, this function interpolates from
    /// [`Alignment::CENTER`], and the result is of the same kind as the
    /// non-`None` argument.
    ///
    /// Applied to two objects of the same kind, the result is of that kind;
    /// otherwise it is the mixed arm, which [`resolve`](Self::resolve) turns
    /// into a concrete [`Alignment`].
    pub fn lerp(
        a: Option<AlignmentGeometry>,
        b: Option<AlignmentGeometry>,
        t: f64,
    ) -> Option<AlignmentGeometry> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b * t),
            (Some(a), None) => Some(a * (1.0 - t)),
            (Some(AlignmentGeometry::Alignment(a)), Some(AlignmentGeometry::Alignment(b))) => {
                Alignment::lerp(Some(a), Some(b), t).map(AlignmentGeometry::Alignment)
            }
            (Some(AlignmentGeometry::Directional(a)), Some(AlignmentGeometry::Directional(b))) => {
                AlignmentDirectional::lerp(Some(a), Some(b), t).map(AlignmentGeometry::Directional)
            }
            (Some(a), Some(b)) => Some(AlignmentGeometry::mixed(
                lerp_double(Some(a.x()), Some(b.x()), t).unwrap(),
                lerp_double(Some(a.start()), Some(b.start()), t).unwrap(),
                lerp_double(Some(a.y()), Some(b.y()), t).unwrap(),
            )),
        }
    }

    /// Convert this instance into an [`Alignment`], which uses literal
    /// coordinates (the `x` coordinate being explicitly a distance from the
    /// left).
    ///
    /// See also:
    ///
    ///  * [`Alignment`], for which this is a no-op (returns itself).
    ///  * [`AlignmentDirectional`], which flips the horizontal direction based
    ///    on the `direction` argument.
    pub fn resolve(&self, direction: Option<TextDirection>) -> Alignment {
        match self {
            AlignmentGeometry::Alignment(alignment) => alignment.resolve(direction),
            AlignmentGeometry::Directional(alignment) => alignment.resolve(direction),
            AlignmentGeometry::Mixed { .. } => {
                match direction.expect("AlignmentGeometry.resolve needs a TextDirection") {
                    TextDirection::Rtl => Alignment::new(self.x() - self.start(), self.y()),
                    TextDirection::Ltr => Alignment::new(self.x() + self.start(), self.y()),
                }
            }
        }
    }
}

impl PartialEq for AlignmentGeometry {
    fn eq(&self, other: &AlignmentGeometry) -> bool {
        other.x() == self.x() && other.start() == self.start() && other.y() == self.y()
    }
}

impl From<Alignment> for AlignmentGeometry {
    fn from(alignment: Alignment) -> AlignmentGeometry {
        AlignmentGeometry::Alignment(alignment)
    }
}

impl From<AlignmentDirectional> for AlignmentGeometry {
    fn from(alignment: AlignmentDirectional) -> AlignmentGeometry {
        AlignmentGeometry::Directional(alignment)
    }
}

impl Neg for AlignmentGeometry {
    type Output = AlignmentGeometry;

    fn neg(self) -> AlignmentGeometry {
        match self {
            AlignmentGeometry::Alignment(alignment) => AlignmentGeometry::Alignment(-alignment),
            AlignmentGeometry::Directional(alignment) => AlignmentGeometry::Directional(-alignment),
            AlignmentGeometry::Mixed { .. } => {
                AlignmentGeometry::mixed(-self.x(), -self.start(), -self.y())
            }
        }
    }
}

impl Mul<f64> for AlignmentGeometry {
    type Output = AlignmentGeometry;

    fn mul(self, other: f64) -> AlignmentGeometry {
        match self {
            AlignmentGeometry::Alignment(alignment) => {
                AlignmentGeometry::Alignment(alignment * other)
            }
            AlignmentGeometry::Directional(alignment) => {
                AlignmentGeometry::Directional(alignment * other)
            }
            AlignmentGeometry::Mixed { .. } => {
                AlignmentGeometry::mixed(self.x() * other, self.start() * other, self.y() * other)
            }
        }
    }
}

impl Div<f64> for AlignmentGeometry {
    type Output = AlignmentGeometry;

    fn div(self, other: f64) -> AlignmentGeometry {
        match self {
            AlignmentGeometry::Alignment(alignment) => {
                AlignmentGeometry::Alignment(alignment / other)
            }
            AlignmentGeometry::Directional(alignment) => {
                AlignmentGeometry::Directional(alignment / other)
            }
            AlignmentGeometry::Mixed { .. } => {
                AlignmentGeometry::mixed(self.x() / other, self.start() / other, self.y() / other)
            }
        }
    }
}

impl Rem<f64> for AlignmentGeometry {
    type Output = AlignmentGeometry;

    fn rem(self, other: f64) -> AlignmentGeometry {
        match self {
            AlignmentGeometry::Alignment(alignment) => {
                AlignmentGeometry::Alignment(alignment % other)
            }
            AlignmentGeometry::Directional(alignment) => {
                AlignmentGeometry::Directional(alignment % other)
            }
            AlignmentGeometry::Mixed { .. } => {
                AlignmentGeometry::mixed(self.x() % other, self.start() % other, self.y() % other)
            }
        }
    }
}

impl Debug for AlignmentGeometry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start() == 0.0 {
            Alignment::stringify(self.x(), self.y(), f)
        } else if self.x() == 0.0 {
            AlignmentDirectional::stringify(self.start(), self.y(), f)
        } else {
            Alignment::stringify(self.x(), self.y(), f)?;
            write!(f, " + ")?;
            AlignmentDirectional::stringify(self.start(), 0.0, f)
        }
    }
}

/// A point within a rectangle.
///
/// `Alignment(0.0, 0.0)` represents the center of the rectangle. The distance
/// from -1.0 to +1.0 is the distance from one side of the rectangle to the
/// other side of the rectangle. Therefore, 2.0 units horizontally (or
/// vertically) is equivalent to the width (or height) of the rectangle.
///
/// `Alignment(-1.0, -1.0)` represents the top left of the rectangle.
///
/// `Alignment(1.0, 1.0)` represents the bottom right of the rectangle.
///
/// `Alignment(0.0, 3.0)` represents a point that is horizontally centered with
/// respect to the rectangle and vertically below the bottom of the rectangle by
/// the height of the rectangle.
///
/// `Alignment(0.0, -0.5)` represents a point that is horizontally centered with
/// respect to the rectangle and vertically half way between the top edge and
/// the center.
///
/// `Alignment(x, y)` in a rectangle with height h and width w describes the
/// point (x * w/2 + w/2, y * h/2 + h/2) in the coordinate system of the
/// rectangle.
///
/// [`Alignment`] uses visual coordinates, which means increasing [`x`](Alignment::x)
/// moves the point from left to right. To support layouts with a right-to-left
/// [`TextDirection`], consider using [`AlignmentDirectional`], in which the
/// direction the point moves when increasing the horizontal value depends on
/// the [`TextDirection`].
///
/// A variety of widgets use [`Alignment`] in their configuration, most
/// notably:
///
///  * `Align` positions a child according to an [`Alignment`].
///
/// See also:
///
///  * [`AlignmentDirectional`], which has a horizontal coordinate orientation
///    that depends on the [`TextDirection`].
///  * [`AlignmentGeometry`], which is an abstract type that is agnostic as to
///    whether the horizontal direction depends on the [`TextDirection`].
#[derive(Clone, Copy, PartialEq)]
pub struct Alignment {
    /// The distance fraction in the horizontal direction.
    ///
    /// A value of -1.0 corresponds to the leftmost edge. A value of 1.0
    /// corresponds to the rightmost edge. Values are not limited to that range;
    /// values less than -1.0 represent positions to the left of the left edge,
    /// and values greater than 1.0 represent positions to the right of the
    /// right edge.
    pub x: f64,
    /// The distance fraction in the vertical direction.
    ///
    /// A value of -1.0 corresponds to the topmost edge. A value of 1.0
    /// corresponds to the bottommost edge. Values are not limited to that range;
    /// values less than -1.0 represent positions above the top, and values
    /// greater than 1.0 represent positions below the bottom.
    pub y: f64,
}

impl Alignment {
    /// Creates an alignment.
    pub const fn new(x: f64, y: f64) -> Alignment {
        Alignment { x, y }
    }

    /// The top left corner.
    pub const TOP_LEFT: Alignment = Alignment::new(-1.0, -1.0);
    /// The center point along the top edge.
    pub const TOP_CENTER: Alignment = Alignment::new(0.0, -1.0);
    /// The top right corner.
    pub const TOP_RIGHT: Alignment = Alignment::new(1.0, -1.0);
    /// The center point along the left edge.
    pub const CENTER_LEFT: Alignment = Alignment::new(-1.0, 0.0);
    /// The center point, both horizontally and vertically.
    pub const CENTER: Alignment = Alignment::new(0.0, 0.0);
    /// The center point along the right edge.
    pub const CENTER_RIGHT: Alignment = Alignment::new(1.0, 0.0);
    /// The bottom left corner.
    pub const BOTTOM_LEFT: Alignment = Alignment::new(-1.0, 1.0);
    /// The center point along the bottom edge.
    pub const BOTTOM_CENTER: Alignment = Alignment::new(0.0, 1.0);
    /// The bottom right corner.
    pub const BOTTOM_RIGHT: Alignment = Alignment::new(1.0, 1.0);

    /// Integer divides the [`Alignment`] in each dimension by the given factor.
    pub fn truncating_div(&self, other: f64) -> Alignment {
        Alignment::new((self.x / other).trunc(), (self.y / other).trunc())
    }

    /// Returns the offset that is this fraction in the direction of the given
    /// offset.
    pub fn along_offset(&self, other: Offset) -> Offset {
        let center_x = other.dx() / 2.0;
        let center_y = other.dy() / 2.0;
        Offset::new(center_x + self.x * center_x, center_y + self.y * center_y)
    }

    /// Returns the offset that is this fraction within the given size.
    pub fn along_size(&self, other: Size) -> Offset {
        let center_x = other.width() / 2.0;
        let center_y = other.height() / 2.0;
        Offset::new(center_x + self.x * center_x, center_y + self.y * center_y)
    }

    /// Returns the point that is this fraction within the given rect.
    pub fn within_rect(&self, rect: Rect) -> Offset {
        let half_width = rect.width() / 2.0;
        let half_height = rect.height() / 2.0;
        Offset::new(
            rect.left + half_width + self.x * half_width,
            rect.top + half_height + self.y * half_height,
        )
    }

    /// Returns a rect of the given size, aligned within given rect as specified
    /// by this alignment.
    ///
    /// For example, a 100×100 size inscribed on a 200×200 rect using
    /// [`Alignment::TOP_LEFT`] would be the 100×100 rect at the top left of the
    /// 200×200 rect.
    pub fn inscribe(&self, size: Size, rect: Rect) -> Rect {
        let half_width_delta = (rect.width() - size.width()) / 2.0;
        let half_height_delta = (rect.height() - size.height()) / 2.0;
        Rect::from_ltwh(
            rect.left + half_width_delta + self.x * half_width_delta,
            rect.top + half_height_delta + self.y * half_height_delta,
            size.width(),
            size.height(),
        )
    }

    /// Linearly interpolate between two [`Alignment`]s.
    ///
    /// If either is null, this function interpolates from [`Alignment::CENTER`].
    pub fn lerp(a: Option<Alignment>, b: Option<Alignment>, t: f64) -> Option<Alignment> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(Alignment::new(
                lerp_double(Some(0.0), Some(b.x), t).unwrap(),
                lerp_double(Some(0.0), Some(b.y), t).unwrap(),
            )),
            (Some(a), None) => Some(Alignment::new(
                lerp_double(Some(a.x), Some(0.0), t).unwrap(),
                lerp_double(Some(a.y), Some(0.0), t).unwrap(),
            )),
            (Some(a), Some(b)) => Some(Alignment::new(
                lerp_double(Some(a.x), Some(b.x), t).unwrap(),
                lerp_double(Some(a.y), Some(b.y), t).unwrap(),
            )),
        }
    }

    /// Convert this instance into an [`Alignment`], which uses literal
    /// coordinates.
    pub fn resolve(&self, _direction: Option<TextDirection>) -> Alignment {
        *self
    }

    fn stringify(x: f64, y: f64, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (x, y) {
            (-1.0, -1.0) => write!(f, "Alignment.topLeft"),
            (0.0, -1.0) => write!(f, "Alignment.topCenter"),
            (1.0, -1.0) => write!(f, "Alignment.topRight"),
            (-1.0, 0.0) => write!(f, "Alignment.centerLeft"),
            (0.0, 0.0) => write!(f, "Alignment.center"),
            (1.0, 0.0) => write!(f, "Alignment.centerRight"),
            (-1.0, 1.0) => write!(f, "Alignment.bottomLeft"),
            (0.0, 1.0) => write!(f, "Alignment.bottomCenter"),
            (1.0, 1.0) => write!(f, "Alignment.bottomRight"),
            _ => write!(f, "Alignment({x:.1}, {y:.1})"),
        }
    }
}

impl Sub for Alignment {
    type Output = Alignment;

    fn sub(self, other: Alignment) -> Alignment {
        Alignment::new(self.x - other.x, self.y - other.y)
    }
}

impl Add for Alignment {
    type Output = Alignment;

    fn add(self, other: Alignment) -> Alignment {
        Alignment::new(self.x + other.x, self.y + other.y)
    }
}

impl Neg for Alignment {
    type Output = Alignment;

    fn neg(self) -> Alignment {
        Alignment::new(-self.x, -self.y)
    }
}

impl Mul<f64> for Alignment {
    type Output = Alignment;

    fn mul(self, other: f64) -> Alignment {
        Alignment::new(self.x * other, self.y * other)
    }
}

impl Div<f64> for Alignment {
    type Output = Alignment;

    fn div(self, other: f64) -> Alignment {
        Alignment::new(self.x / other, self.y / other)
    }
}

impl Rem<f64> for Alignment {
    type Output = Alignment;

    fn rem(self, other: f64) -> Alignment {
        Alignment::new(self.x.rem_euclid(other), self.y.rem_euclid(other))
    }
}

impl Debug for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Alignment::stringify(self.x, self.y, f)
    }
}

/// An offset that's expressed as a fraction of a [`Size`], but whose horizontal
/// component is dependent on the writing direction.
///
/// This can be used to indicate an offset from the left in [`TextDirection::Ltr`]
/// text and an offset from the right in [`TextDirection::Rtl`] text without
/// having to be aware of the current text direction.
///
/// See also:
///
///  * [`Alignment`], a variant that is defined in physical terms (i.e. whose
///    horizontal component does not depend on the text direction).
#[derive(Clone, Copy, PartialEq)]
pub struct AlignmentDirectional {
    /// The distance fraction in the horizontal direction.
    ///
    /// A value of -1.0 corresponds to the edge on the "start" side, which is
    /// the left side in [`TextDirection::Ltr`] contexts and the right side in
    /// [`TextDirection::Rtl`] contexts. A value of 1.0 corresponds to the
    /// opposite edge, the "end" side. Values are not limited to that range;
    /// values less than -1.0 represent positions beyond the start edge, and
    /// values greater than 1.0 represent positions beyond the end edge.
    ///
    /// This value is normalized into an [`Alignment::x`] value by the
    /// [`resolve`](Self::resolve) method.
    pub start: f64,
    /// The distance fraction in the vertical direction.
    ///
    /// A value of -1.0 corresponds to the topmost edge. A value of 1.0
    /// corresponds to the bottommost edge. Values are not limited to that range;
    /// values less than -1.0 represent positions above the top, and values
    /// greater than 1.0 represent positions below the bottom.
    ///
    /// This value is passed through to [`Alignment::y`] unmodified by the
    /// [`resolve`](Self::resolve) method.
    pub y: f64,
}

impl AlignmentDirectional {
    /// Creates a directional alignment.
    pub const fn new(start: f64, y: f64) -> AlignmentDirectional {
        AlignmentDirectional { start, y }
    }

    /// The top corner on the "start" side.
    pub const TOP_START: AlignmentDirectional = AlignmentDirectional::new(-1.0, -1.0);
    /// The center point along the top edge.
    ///
    /// Consider using [`Alignment::TOP_CENTER`] instead, as it does not need to
    /// be [`resolve`](Self::resolve)d to be used.
    pub const TOP_CENTER: AlignmentDirectional = AlignmentDirectional::new(0.0, -1.0);
    /// The top corner on the "end" side.
    pub const TOP_END: AlignmentDirectional = AlignmentDirectional::new(1.0, -1.0);
    /// The center point along the "start" edge.
    pub const CENTER_START: AlignmentDirectional = AlignmentDirectional::new(-1.0, 0.0);
    /// The center point, both horizontally and vertically.
    ///
    /// Consider using [`Alignment::CENTER`] instead, as it does not need to be
    /// [`resolve`](Self::resolve)d to be used.
    pub const CENTER: AlignmentDirectional = AlignmentDirectional::new(0.0, 0.0);
    /// The center point along the "end" edge.
    pub const CENTER_END: AlignmentDirectional = AlignmentDirectional::new(1.0, 0.0);
    /// The bottom corner on the "start" side.
    pub const BOTTOM_START: AlignmentDirectional = AlignmentDirectional::new(-1.0, 1.0);
    /// The center point along the bottom edge.
    ///
    /// Consider using [`Alignment::BOTTOM_CENTER`] instead, as it does not need
    /// to be [`resolve`](Self::resolve)d to be used.
    pub const BOTTOM_CENTER: AlignmentDirectional = AlignmentDirectional::new(0.0, 1.0);
    /// The bottom corner on the "end" side.
    pub const BOTTOM_END: AlignmentDirectional = AlignmentDirectional::new(1.0, 1.0);

    /// Integer divides the [`AlignmentDirectional`] in each dimension by the
    /// given factor.
    pub fn truncating_div(&self, other: f64) -> AlignmentDirectional {
        AlignmentDirectional::new((self.start / other).trunc(), (self.y / other).trunc())
    }

    /// Linearly interpolate between two [`AlignmentDirectional`]s.
    ///
    /// If either is null, this function interpolates from
    /// [`AlignmentDirectional::CENTER`].
    pub fn lerp(
        a: Option<AlignmentDirectional>,
        b: Option<AlignmentDirectional>,
        t: f64,
    ) -> Option<AlignmentDirectional> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(AlignmentDirectional::new(
                lerp_double(Some(0.0), Some(b.start), t).unwrap(),
                lerp_double(Some(0.0), Some(b.y), t).unwrap(),
            )),
            (Some(a), None) => Some(AlignmentDirectional::new(
                lerp_double(Some(a.start), Some(0.0), t).unwrap(),
                lerp_double(Some(a.y), Some(0.0), t).unwrap(),
            )),
            (Some(a), Some(b)) => Some(AlignmentDirectional::new(
                lerp_double(Some(a.start), Some(b.start), t).unwrap(),
                lerp_double(Some(a.y), Some(b.y), t).unwrap(),
            )),
        }
    }

    /// Convert this instance into an [`Alignment`], which uses literal
    /// coordinates.
    pub fn resolve(&self, direction: Option<TextDirection>) -> Alignment {
        match direction.expect("No TextDirection found.") {
            TextDirection::Rtl => Alignment::new(-self.start, self.y),
            TextDirection::Ltr => Alignment::new(self.start, self.y),
        }
    }

    fn stringify(start: f64, y: f64, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (start, y) {
            (-1.0, -1.0) => write!(f, "AlignmentDirectional.topStart"),
            (0.0, -1.0) => write!(f, "AlignmentDirectional.topCenter"),
            (1.0, -1.0) => write!(f, "AlignmentDirectional.topEnd"),
            (-1.0, 0.0) => write!(f, "AlignmentDirectional.centerStart"),
            (0.0, 0.0) => write!(f, "AlignmentDirectional.center"),
            (1.0, 0.0) => write!(f, "AlignmentDirectional.centerEnd"),
            (-1.0, 1.0) => write!(f, "AlignmentDirectional.bottomStart"),
            (0.0, 1.0) => write!(f, "AlignmentDirectional.bottomCenter"),
            (1.0, 1.0) => write!(f, "AlignmentDirectional.bottomEnd"),
            _ => write!(f, "AlignmentDirectional({start:.1}, {y:.1})"),
        }
    }
}

impl Sub for AlignmentDirectional {
    type Output = AlignmentDirectional;

    fn sub(self, other: AlignmentDirectional) -> AlignmentDirectional {
        AlignmentDirectional::new(self.start - other.start, self.y - other.y)
    }
}

impl Add for AlignmentDirectional {
    type Output = AlignmentDirectional;

    fn add(self, other: AlignmentDirectional) -> AlignmentDirectional {
        AlignmentDirectional::new(self.start + other.start, self.y + other.y)
    }
}

impl Neg for AlignmentDirectional {
    type Output = AlignmentDirectional;

    fn neg(self) -> AlignmentDirectional {
        AlignmentDirectional::new(-self.start, -self.y)
    }
}

impl Mul<f64> for AlignmentDirectional {
    type Output = AlignmentDirectional;

    fn mul(self, other: f64) -> AlignmentDirectional {
        AlignmentDirectional::new(self.start * other, self.y * other)
    }
}

impl Div<f64> for AlignmentDirectional {
    type Output = AlignmentDirectional;

    fn div(self, other: f64) -> AlignmentDirectional {
        AlignmentDirectional::new(self.start / other, self.y / other)
    }
}

impl Rem<f64> for AlignmentDirectional {
    type Output = AlignmentDirectional;

    fn rem(self, other: f64) -> AlignmentDirectional {
        AlignmentDirectional::new(self.start.rem_euclid(other), self.y.rem_euclid(other))
    }
}

impl Debug for AlignmentDirectional {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        AlignmentDirectional::stringify(self.start, self.y, f)
    }
}

/// The vertical alignment of text within an input box.
///
/// A single [`y`](TextAlignVertical::y) value that can range from -1.0 to 1.0.
/// -1.0 aligns to the top of an input box so that the top of the first line of
/// text fits within the box and its padding. 0.0 aligns to the center of the
/// box. 1.0 aligns so that the bottom of the last line of text aligns with the
/// bottom interior edge of the input box.
///
/// See also:
///
///  * `TextField.textAlignVertical`, which is passed on to the
///    `InputDecorator`.
///  * `CupertinoTextField.textAlignVertical`, which behaves in the same way as
///    the parameter in TextField.
///  * `InputDecorator.textAlignVertical`, which defines the alignment of
///    prefix, input, and suffix within an `InputDecorator`.
#[derive(Clone, Copy)]
pub struct TextAlignVertical {
    /// A value ranging from -1.0 to 1.0 that defines the topmost and bottommost
    /// locations of the top and bottom of the input box.
    pub y: f64,
}

impl TextAlignVertical {
    /// Creates a TextAlignVertical from any y value between -1.0 and 1.0.
    pub fn new(y: f64) -> TextAlignVertical {
        debug_assert!((-1.0..=1.0).contains(&y));
        TextAlignVertical { y }
    }

    /// Aligns a TextField's input Text with the topmost location within a
    /// TextField's input box.
    pub const TOP: TextAlignVertical = TextAlignVertical { y: -1.0 };
    /// Aligns a TextField's input Text to the center of the TextField.
    pub const CENTER: TextAlignVertical = TextAlignVertical { y: 0.0 };
    /// Aligns a TextField's input Text with the bottommost location within a
    /// TextField.
    pub const BOTTOM: TextAlignVertical = TextAlignVertical { y: 1.0 };
}

impl Debug for TextAlignVertical {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TextAlignVertical(y: {})", self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_control_test() {
        let alignment = Alignment::new(0.5, 0.25);
        assert_eq!(alignment / 2.0, Alignment::new(0.25, 0.125));
        assert_eq!(alignment.truncating_div(2.0), Alignment::CENTER);
        assert_eq!(alignment % 5.0, Alignment::new(0.5, 0.25));
    }

    #[test]
    fn alignment_lerp() {
        let a = Alignment::TOP_LEFT;
        let b = Alignment::TOP_CENTER;
        assert_eq!(
            Alignment::lerp(Some(a), Some(b), 0.25),
            Some(Alignment::new(-0.75, -1.0))
        );
        assert_eq!(Alignment::lerp(None, None, 0.25), None);
        assert_eq!(
            Alignment::lerp(None, Some(b), 0.25),
            Some(Alignment::new(0.0, -0.25))
        );
        assert_eq!(
            Alignment::lerp(Some(a), None, 0.25),
            Some(Alignment::new(-0.75, -0.75))
        );
        assert_eq!(
            Alignment::lerp(Some(Alignment::TOP_LEFT), Some(Alignment::TOP_LEFT), 0.5),
            Some(Alignment::TOP_LEFT)
        );
    }

    #[test]
    fn resolve_flips_start_in_rtl() {
        assert_eq!(
            AlignmentDirectional::new(0.25, 0.3).resolve(Some(TextDirection::Ltr)),
            Alignment::new(0.25, 0.3)
        );
        assert_eq!(
            AlignmentDirectional::new(0.25, 0.3).resolve(Some(TextDirection::Rtl)),
            Alignment::new(-0.25, 0.3)
        );
        assert_eq!(
            AlignmentDirectional::CENTER.resolve(Some(TextDirection::Ltr)),
            Alignment::CENTER
        );
        assert_eq!(
            AlignmentDirectional::CENTER.resolve(Some(TextDirection::Rtl)),
            Alignment::CENTER
        );
        assert_eq!(
            AlignmentDirectional::BOTTOM_END.resolve(Some(TextDirection::Ltr)),
            Alignment::BOTTOM_RIGHT
        );
        assert_eq!(
            AlignmentDirectional::BOTTOM_END.resolve(Some(TextDirection::Rtl)),
            Alignment::BOTTOM_LEFT
        );
        assert_eq!(
            AlignmentDirectional::CENTER_START.resolve(Some(TextDirection::Ltr)),
            AlignmentDirectional::CENTER_END.resolve(Some(TextDirection::Rtl))
        );
    }

    #[test]
    fn zero_horizontal_alignments_compare_equal_across_kinds() {
        assert_eq!(
            AlignmentGeometry::from(Alignment::CENTER),
            AlignmentGeometry::from(AlignmentDirectional::CENTER)
        );
        assert_ne!(
            AlignmentGeometry::from(Alignment::TOP_LEFT),
            AlignmentGeometry::from(AlignmentDirectional::TOP_START)
        );
        assert_eq!(
            AlignmentGeometry::from(AlignmentDirectional::TOP_END * 0.0),
            AlignmentGeometry::from(Alignment::CENTER)
        );
    }

    #[test]
    fn same_kind_operators() {
        assert_eq!(
            Alignment::new(1.0, 2.0) + Alignment::new(3.0, 5.0),
            Alignment::new(4.0, 7.0)
        );
        assert_eq!(
            Alignment::new(1.0, 2.0) - Alignment::new(3.0, 5.0),
            Alignment::new(-2.0, -3.0)
        );
        assert_eq!(
            AlignmentDirectional::new(1.0, 2.0) * 2.0,
            AlignmentDirectional::new(2.0, 4.0)
        );
        assert_eq!(
            AlignmentDirectional::new(1.0, 2.0) / 2.0,
            AlignmentDirectional::new(0.5, 1.0)
        );
        assert_eq!(
            AlignmentDirectional::new(1.0, 2.0) % 2.0,
            AlignmentDirectional::CENTER_END
        );
        assert_eq!(
            AlignmentDirectional::new(1.0, 2.0).truncating_div(2.0),
            AlignmentDirectional::BOTTOM_CENTER
        );
        assert_eq!(Alignment::new(1.0, 2.0) * 2.0, Alignment::new(2.0, 4.0));
        assert_eq!(Alignment::new(1.0, 2.0) % 2.0, Alignment::CENTER_RIGHT);
        assert_eq!(
            Alignment::new(1.0, 2.0).truncating_div(2.0),
            Alignment::BOTTOM_CENTER
        );
    }

    #[test]
    fn debug_matches_darts_to_string() {
        assert_eq!(
            format!("{:?}", Alignment::new(1.0001, 2.0001)),
            "Alignment(1.0, 2.0)"
        );
        assert_eq!(format!("{:?}", Alignment::CENTER), "Alignment.center");
        assert_eq!(
            format!("{:?}", Alignment::new(0.0001, 0.0001)),
            "Alignment(0.0, 0.0)"
        );
        assert_eq!(
            format!("{:?}", AlignmentDirectional::CENTER),
            "AlignmentDirectional.center"
        );
    }

    #[test]
    fn factories_and_static_members() {
        assert_eq!(
            AlignmentGeometry::xy(4.0, 5.0),
            AlignmentGeometry::from(Alignment::new(4.0, 5.0))
        );
        assert_eq!(
            AlignmentGeometry::directional(4.0, 5.0),
            AlignmentGeometry::from(AlignmentDirectional::new(4.0, 5.0))
        );
        assert_eq!(
            AlignmentGeometry::TOP_LEFT,
            AlignmentGeometry::from(Alignment::TOP_LEFT)
        );
        assert_eq!(
            AlignmentGeometry::TOP_START,
            AlignmentGeometry::from(AlignmentDirectional::TOP_START)
        );
        assert_eq!(
            AlignmentGeometry::CENTER,
            AlignmentGeometry::from(Alignment::CENTER)
        );
        assert_eq!(
            AlignmentGeometry::BOTTOM_END,
            AlignmentGeometry::from(AlignmentDirectional::BOTTOM_END)
        );
    }

    #[test]
    fn a_cross_kind_lerp_resolves_per_direction() {
        let a = AlignmentGeometry::xy(1.0, 0.0);
        let b = AlignmentGeometry::directional(1.0, 0.0);
        let halfway = AlignmentGeometry::lerp(Some(a), Some(b), 0.5).expect("both ends are set");
        assert!(matches!(halfway, AlignmentGeometry::Mixed { .. }));
        assert_eq!(
            halfway.resolve(Some(TextDirection::Ltr)),
            Alignment::new(1.0, 0.0)
        );
        assert_eq!(
            halfway.resolve(Some(TextDirection::Rtl)),
            Alignment::new(0.0, 0.0)
        );
    }

    #[test]
    fn a_same_kind_lerp_keeps_the_kind() {
        let lerp = |a, b, t| AlignmentGeometry::lerp(Some(a), Some(b), t).expect("both ends");
        assert_eq!(
            lerp(
                AlignmentGeometry::TOP_LEFT,
                AlignmentGeometry::BOTTOM_LEFT,
                0.5
            ),
            AlignmentGeometry::Alignment(Alignment::CENTER_LEFT)
        );
        assert_eq!(
            lerp(
                AlignmentGeometry::TOP_START,
                AlignmentGeometry::BOTTOM_START,
                0.5
            ),
            AlignmentGeometry::Directional(AlignmentDirectional::CENTER_START)
        );
    }

    #[test]
    fn a_lerp_from_none_starts_at_the_center() {
        let end = AlignmentGeometry::directional(1.0, 1.0);
        assert_eq!(
            AlignmentGeometry::lerp(None, Some(end), 0.5),
            Some(AlignmentGeometry::directional(0.5, 0.5))
        );
    }

    #[test]
    fn a_cross_kind_add_is_the_mixed_arm() {
        let sum = AlignmentGeometry::xy(1.0, 2.0).add(AlignmentGeometry::directional(3.0, 4.0));
        assert_eq!(
            sum.resolve(Some(TextDirection::Ltr)),
            Alignment::new(4.0, 6.0)
        );
        assert_eq!(
            sum.resolve(Some(TextDirection::Rtl)),
            Alignment::new(-2.0, 6.0)
        );
    }

    #[test]
    fn inscribe_puts_the_size_in_the_named_corner() {
        let rect = Rect::from_ltwh(0.0, 0.0, 200.0, 200.0);
        let size = Size::new(100.0, 100.0);
        assert_eq!(
            Alignment::TOP_LEFT.inscribe(size, rect),
            Rect::from_ltwh(0.0, 0.0, 100.0, 100.0)
        );
        assert_eq!(
            Alignment::CENTER.inscribe(size, rect),
            Rect::from_ltwh(50.0, 50.0, 100.0, 100.0)
        );
    }
}
