//! Flutter counterpart: `engine/src/flutter/lib/ui/geometry.dart`.

use std::fmt::{self, Debug};
use std::ops::{Add, BitAnd, Div, Mul, Neg, Rem, Sub};

use crate::lerp::lerp_double_non_null;

/// An immutable 2D floating-point offset.
///
/// Generally speaking, Offsets can be interpreted in two ways:
///
/// 1. As representing a point in Cartesian space a specified distance from a
///    separately-maintained origin. For example, the top-left position of
///    children in the `RenderBox` protocol is typically represented as an
///    [`Offset`] from the top left of the parent box.
///
/// 2. As a vector that can be applied to coordinates. For example, when painting
///    a `RenderObject`, the parent is passed an [`Offset`] from the screen's
///    origin which it can add to the offsets of its children to find the
///    [`Offset`] from the screen's origin to each of the children.
///
/// Because a particular [`Offset`] can be interpreted as one sense at one time
/// then as the other sense at a later time, the same class is used for both
/// senses.
///
/// See also:
///
///  * [`Size`], which represents a vector describing the size of a rectangle.
#[derive(Clone, Copy, PartialEq)]
pub struct Offset {
    dx: f64,
    dy: f64,
}

impl Offset {
    /// Creates an offset. The first argument sets [`dx`](Offset::dx), the
    /// horizontal component, and the second sets [`dy`](Offset::dy), the
    /// vertical component.
    pub const fn new(dx: f64, dy: f64) -> Offset {
        Offset { dx, dy }
    }

    /// Creates an offset from its [`direction`](Offset::direction) and
    /// [`distance`](Offset::distance).
    ///
    /// The direction is in radians clockwise from the positive x-axis.
    ///
    /// Pass `1.0` for `distance` to create a unit vector.
    pub fn from_direction(direction: f64, distance: f64) -> Offset {
        Offset::new(distance * direction.cos(), distance * direction.sin())
    }

    /// The x component of the offset.
    ///
    /// The y component is given by [`dy`](Offset::dy).
    pub const fn dx(&self) -> f64 {
        self.dx
    }

    /// The y component of the offset.
    ///
    /// The x component is given by [`dx`](Offset::dx).
    pub const fn dy(&self) -> f64 {
        self.dy
    }

    /// Returns true if either component is [`f64::INFINITY`], and false if both
    /// are finite (or negative infinity, or NaN).
    ///
    /// This is different than comparing for equality with an instance that has
    /// _both_ components set to [`f64::INFINITY`].
    ///
    /// See also:
    ///
    ///  * [`is_finite`](Offset::is_finite), which is true if both components are
    ///    finite (and not NaN).
    pub fn is_infinite(&self) -> bool {
        self.dx >= f64::INFINITY || self.dy >= f64::INFINITY
    }

    /// Whether both components are finite (neither infinite nor NaN).
    ///
    /// See also:
    ///
    ///  * [`is_infinite`](Offset::is_infinite), which returns true if either
    ///    component is equal to positive infinity.
    pub fn is_finite(&self) -> bool {
        self.dx.is_finite() && self.dy.is_finite()
    }

    /// Less-than operator. Compares an [`Offset`] or [`Size`] to another
    /// [`Offset`] or [`Size`], and returns true if both the horizontal and
    /// vertical values of the left-hand-side operand are smaller than the
    /// horizontal and vertical values of the right-hand-side operand
    /// respectively. Returns false otherwise.
    ///
    /// This is a partial ordering. It is possible for two values to be neither
    /// less, nor greater than, nor equal to, another.
    pub fn lt(&self, other: &Offset) -> bool {
        self.dx < other.dx && self.dy < other.dy
    }

    /// Less-than-or-equal-to operator. Compares an [`Offset`] or [`Size`] to
    /// another [`Offset`] or [`Size`], and returns true if both the horizontal
    /// and vertical values of the left-hand-side operand are smaller than or
    /// equal to the horizontal and vertical values of the right-hand-side
    /// operand respectively. Returns false otherwise.
    ///
    /// This is a partial ordering. It is possible for two values to be neither
    /// less, nor greater than, nor equal to, another.
    pub fn le(&self, other: &Offset) -> bool {
        self.dx <= other.dx && self.dy <= other.dy
    }

    /// Greater-than operator. Compares an [`Offset`] or [`Size`] to another
    /// [`Offset`] or [`Size`], and returns true if both the horizontal and
    /// vertical values of the left-hand-side operand are bigger than the
    /// horizontal and vertical values of the right-hand-side operand
    /// respectively. Returns false otherwise.
    ///
    /// This is a partial ordering. It is possible for two values to be neither
    /// less, nor greater than, nor equal to, another.
    pub fn gt(&self, other: &Offset) -> bool {
        self.dx > other.dx && self.dy > other.dy
    }

    /// Greater-than-or-equal-to operator. Compares an [`Offset`] or [`Size`] to
    /// another [`Offset`] or [`Size`], and returns true if both the horizontal
    /// and vertical values of the left-hand-side operand are bigger than or
    /// equal to the horizontal and vertical values of the right-hand-side
    /// operand respectively. Returns false otherwise.
    ///
    /// This is a partial ordering. It is possible for two values to be neither
    /// less, nor greater than, nor equal to, another.
    pub fn ge(&self, other: &Offset) -> bool {
        self.dx >= other.dx && self.dy >= other.dy
    }

    /// The magnitude of the offset.
    ///
    /// If you need this value to compare it to another [`Offset`]'s distance,
    /// consider using [`distance_squared`](Offset::distance_squared) instead,
    /// since it is cheaper to compute.
    pub fn distance(&self) -> f64 {
        (self.dx * self.dx + self.dy * self.dy).sqrt()
    }

    /// The square of the magnitude of the offset.
    ///
    /// This is cheaper than computing the [`distance`](Offset::distance) itself.
    pub fn distance_squared(&self) -> f64 {
        self.dx * self.dx + self.dy * self.dy
    }

    /// The angle of this offset as radians clockwise from the positive x-axis,
    /// in the range -pi to pi, assuming positive values of the x-axis go to the
    /// right and positive values of the y-axis go down.
    ///
    /// Zero means that [`dy`](Offset::dy) is zero and [`dx`](Offset::dx) is zero
    /// or positive.
    ///
    /// Values from zero to pi/2 indicate positive values of [`dx`](Offset::dx)
    /// and [`dy`](Offset::dy), the bottom-right quadrant.
    ///
    /// Values from pi/2 to pi indicate negative values of [`dx`](Offset::dx) and
    /// positive values of [`dy`](Offset::dy), the bottom-left quadrant.
    ///
    /// Values from zero to -pi/2 indicate positive values of [`dx`](Offset::dx)
    /// and negative values of [`dy`](Offset::dy), the top-right quadrant.
    ///
    /// Values from -pi/2 to -pi indicate negative values of [`dx`](Offset::dx)
    /// and [`dy`](Offset::dy), the top-left quadrant.
    ///
    /// When [`dy`](Offset::dy) is zero and [`dx`](Offset::dx) is negative, the
    /// direction is pi.
    ///
    /// When [`dx`](Offset::dx) is zero, direction is pi/2 if [`dy`](Offset::dy)
    /// is positive and -pi/2 if [`dy`](Offset::dy) is negative.
    ///
    /// See also:
    ///
    ///  * [`distance`](Offset::distance), to compute the magnitude of the
    ///    vector.
    pub fn direction(&self) -> f64 {
        self.dy.atan2(self.dx)
    }

    /// An offset with zero magnitude.
    ///
    /// This can be used to represent the origin of a coordinate space.
    pub const ZERO: Offset = Offset::new(0.0, 0.0);

    /// An offset with infinite x and y components.
    ///
    /// See also:
    ///
    ///  * [`is_infinite`](Offset::is_infinite), which checks whether either
    ///    component is infinite.
    ///  * [`is_finite`](Offset::is_finite), which checks whether both
    ///    components are finite.
    // This is included for completeness, because [Size.infinite] exists.
    pub const INFINITE: Offset = Offset::new(f64::INFINITY, f64::INFINITY);

    /// Returns a new offset with the x component scaled by `scale_x` and the y
    /// component scaled by `scale_y`.
    ///
    /// If the two scale arguments are the same, consider using the `*` operator
    /// instead:
    ///
    /// ```
    /// # use reveal_geometry::Offset;
    /// let a = Offset::new(10.0, 10.0);
    /// let b = a * 2.0; // same as: a.scale(2.0, 2.0)
    /// # assert_eq!(b, a.scale(2.0, 2.0));
    /// ```
    ///
    /// If the two arguments are -1, consider using the unary `-` operator
    /// instead:
    ///
    /// ```
    /// # use reveal_geometry::Offset;
    /// let a = Offset::new(10.0, 10.0);
    /// let b = -a; // same as: a.scale(-1.0, -1.0)
    /// # assert_eq!(b, a.scale(-1.0, -1.0));
    /// ```
    pub fn scale(&self, scale_x: f64, scale_y: f64) -> Offset {
        Offset::new(self.dx * scale_x, self.dy * scale_y)
    }

    /// Returns a new offset with `translate_x` added to the x component and
    /// `translate_y` added to the y component.
    ///
    /// If the arguments come from another [`Offset`], consider using the `+` or
    /// `-` operators instead:
    ///
    /// ```
    /// # use reveal_geometry::Offset;
    /// let a = Offset::new(10.0, 10.0);
    /// let b = Offset::new(10.0, 10.0);
    /// let c = a + b; // same as: a.translate(b.dx(), b.dy())
    /// let d = a - b; // same as: a.translate(-b.dx(), -b.dy())
    /// # assert_eq!(c, a.translate(b.dx(), b.dy()));
    /// # assert_eq!(d, a.translate(-b.dx(), -b.dy()));
    /// ```
    pub fn translate(&self, translate_x: f64, translate_y: f64) -> Offset {
        Offset::new(self.dx + translate_x, self.dy + translate_y)
    }

    /// Integer (truncating) division operator.
    ///
    /// Returns an offset whose coordinates are the coordinates of this offset
    /// divided by the scalar `operand`, rounded towards zero.
    pub fn truncating_div(&self, operand: f64) -> Offset {
        Offset::new((self.dx / operand).trunc(), (self.dy / operand).trunc())
    }

    /// Linearly interpolate between two offsets.
    ///
    /// If either offset is null, this function interpolates from
    /// [`Offset::ZERO`].
    ///
    /// The `t` argument represents position on the timeline, with 0.0 meaning
    /// that the interpolation has not started, returning `a` (or something
    /// equivalent to `a`), 1.0 meaning that the interpolation has finished,
    /// returning `b` (or something equivalent to `b`), and values in between
    /// meaning that the interpolation is at the relevant point on the timeline
    /// between `a` and `b`. The interpolation can be extrapolated beyond 0.0 and
    /// 1.0, so negative values and values greater than 1.0 are valid (and can
    /// easily be generated by curves such as `Curves.elasticInOut`).
    ///
    /// Values for `t` are usually obtained from an `Animation<double>`, such as
    /// an `AnimationController`.
    pub fn lerp(a: Option<Offset>, b: Option<Offset>, t: f64) -> Option<Offset> {
        match (a, b) {
            (None, None) => None,
            (Some(a), None) => Some(a * (1.0 - t)),
            (None, Some(b)) => Some(b * t),
            (Some(a), Some(b)) => Some(Offset::new(
                lerp_double_non_null(a.dx, b.dx, t),
                lerp_double_non_null(a.dy, b.dy, t),
            )),
        }
    }
}

/// Unary negation operator.
///
/// Returns an offset with the coordinates negated.
///
/// If the [`Offset`] represents an arrow on a plane, this operator returns the
/// same arrow but pointing in the reverse direction.
impl Neg for Offset {
    type Output = Offset;

    fn neg(self) -> Offset {
        Offset::new(-self.dx, -self.dy)
    }
}

/// Binary subtraction operator.
///
/// Returns an offset whose [`dx`](Offset::dx) value is the left-hand-side
/// operand's [`dx`](Offset::dx) minus the right-hand-side operand's
/// [`dx`](Offset::dx) and whose [`dy`](Offset::dy) value is the left-hand-side
/// operand's [`dy`](Offset::dy) minus the right-hand-side operand's
/// [`dy`](Offset::dy).
///
/// See also [`translate`](Offset::translate).
impl Sub for Offset {
    type Output = Offset;

    fn sub(self, other: Offset) -> Offset {
        Offset::new(self.dx - other.dx, self.dy - other.dy)
    }
}

/// Binary addition operator.
///
/// Returns an offset whose [`dx`](Offset::dx) value is the sum of the
/// [`dx`](Offset::dx) values of the two operands, and whose [`dy`](Offset::dy)
/// value is the sum of the [`dy`](Offset::dy) values of the two operands.
///
/// See also [`translate`](Offset::translate).
impl Add for Offset {
    type Output = Offset;

    fn add(self, other: Offset) -> Offset {
        Offset::new(self.dx + other.dx, self.dy + other.dy)
    }
}

/// Multiplication operator.
///
/// Returns an offset whose coordinates are the coordinates of the left-hand-side
/// operand (an Offset) multiplied by the scalar right-hand-side operand (an
/// `f64`).
///
/// See also [`scale`](Offset::scale).
impl Mul<f64> for Offset {
    type Output = Offset;

    fn mul(self, operand: f64) -> Offset {
        Offset::new(self.dx * operand, self.dy * operand)
    }
}

/// Division operator.
///
/// Returns an offset whose coordinates are the coordinates of the left-hand-side
/// operand (an Offset) divided by the scalar right-hand-side operand (an `f64`).
///
/// See also [`scale`](Offset::scale).
impl Div<f64> for Offset {
    type Output = Offset;

    fn div(self, operand: f64) -> Offset {
        Offset::new(self.dx / operand, self.dy / operand)
    }
}

/// Modulo (remainder) operator.
///
/// Returns an offset whose coordinates are the remainder of dividing the
/// coordinates of the left-hand-side operand (an Offset) by the scalar
/// right-hand-side operand (an `f64`).
impl Rem<f64> for Offset {
    type Output = Offset;

    fn rem(self, operand: f64) -> Offset {
        Offset::new(self.dx.rem_euclid(operand), self.dy.rem_euclid(operand))
    }
}

impl Debug for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Offset({:.1}, {:.1})", self.dx, self.dy)
    }
}

/// Holds a 2D floating-point size.
///
/// You can think of this as an [`Offset`] from the origin.
#[derive(Clone, Copy, PartialEq)]
pub struct Size {
    width: f64,
    height: f64,
}

impl Size {
    /// Creates a [`Size`] with the given [`width`](Size::width) and
    /// [`height`](Size::height).
    pub const fn new(width: f64, height: f64) -> Size {
        Size { width, height }
    }

    /// Creates a square [`Size`] whose [`width`](Size::width) and
    /// [`height`](Size::height) are the given dimension.
    ///
    /// See also:
    ///
    ///  * [`from_radius`](Size::from_radius), which is more convenient when the
    ///    available size is the radius of a circle.
    pub const fn square(dimension: f64) -> Size {
        Size::new(dimension, dimension)
    }

    /// Creates a [`Size`] with the given [`width`](Size::width) and an infinite
    /// [`height`](Size::height).
    pub const fn from_width(width: f64) -> Size {
        Size::new(width, f64::INFINITY)
    }

    /// Creates a [`Size`] with the given [`height`](Size::height) and an
    /// infinite [`width`](Size::width).
    pub const fn from_height(height: f64) -> Size {
        Size::new(f64::INFINITY, height)
    }

    /// Creates a square [`Size`] whose [`width`](Size::width) and
    /// [`height`](Size::height) are twice the given dimension.
    ///
    /// This is a square that contains a circle with the given radius.
    ///
    /// See also:
    ///
    ///  * [`square`](Size::square), which creates a square with the given
    ///    dimension.
    pub const fn from_radius(radius: f64) -> Size {
        Size::new(radius * 2.0, radius * 2.0)
    }

    /// The horizontal extent of this size.
    pub const fn width(&self) -> f64 {
        self.width
    }

    /// The vertical extent of this size.
    pub const fn height(&self) -> f64 {
        self.height
    }

    /// Returns true if either component is [`f64::INFINITY`], and false if both
    /// are finite (or negative infinity, or NaN).
    ///
    /// This is different than comparing for equality with an instance that has
    /// _both_ components set to [`f64::INFINITY`].
    ///
    /// See also:
    ///
    ///  * [`is_finite`](Size::is_finite), which is true if both components are
    ///    finite (and not NaN).
    pub fn is_infinite(&self) -> bool {
        self.width >= f64::INFINITY || self.height >= f64::INFINITY
    }

    /// Whether both components are finite (neither infinite nor NaN).
    ///
    /// See also:
    ///
    ///  * [`is_infinite`](Size::is_infinite), which returns true if either
    ///    component is equal to positive infinity.
    pub fn is_finite(&self) -> bool {
        self.width.is_finite() && self.height.is_finite()
    }

    /// Less-than operator. Compares an [`Offset`] or [`Size`] to another
    /// [`Offset`] or [`Size`], and returns true if both the horizontal and
    /// vertical values of the left-hand-side operand are smaller than the
    /// horizontal and vertical values of the right-hand-side operand
    /// respectively. Returns false otherwise.
    ///
    /// This is a partial ordering. It is possible for two values to be neither
    /// less, nor greater than, nor equal to, another.
    pub fn lt(&self, other: &Size) -> bool {
        self.width < other.width && self.height < other.height
    }

    /// Less-than-or-equal-to operator. Compares an [`Offset`] or [`Size`] to
    /// another [`Offset`] or [`Size`], and returns true if both the horizontal
    /// and vertical values of the left-hand-side operand are smaller than or
    /// equal to the horizontal and vertical values of the right-hand-side
    /// operand respectively. Returns false otherwise.
    ///
    /// This is a partial ordering. It is possible for two values to be neither
    /// less, nor greater than, nor equal to, another.
    pub fn le(&self, other: &Size) -> bool {
        self.width <= other.width && self.height <= other.height
    }

    /// Greater-than operator. Compares an [`Offset`] or [`Size`] to another
    /// [`Offset`] or [`Size`], and returns true if both the horizontal and
    /// vertical values of the left-hand-side operand are bigger than the
    /// horizontal and vertical values of the right-hand-side operand
    /// respectively. Returns false otherwise.
    ///
    /// This is a partial ordering. It is possible for two values to be neither
    /// less, nor greater than, nor equal to, another.
    pub fn gt(&self, other: &Size) -> bool {
        self.width > other.width && self.height > other.height
    }

    /// Greater-than-or-equal-to operator. Compares an [`Offset`] or [`Size`] to
    /// another [`Offset`] or [`Size`], and returns true if both the horizontal
    /// and vertical values of the left-hand-side operand are bigger than or
    /// equal to the horizontal and vertical values of the right-hand-side
    /// operand respectively. Returns false otherwise.
    ///
    /// This is a partial ordering. It is possible for two values to be neither
    /// less, nor greater than, nor equal to, another.
    pub fn ge(&self, other: &Size) -> bool {
        self.width >= other.width && self.height >= other.height
    }

    /// The aspect ratio of this size.
    ///
    /// This returns the [`width`](Size::width) divided by the
    /// [`height`](Size::height).
    ///
    /// If the [`width`](Size::width) is zero, the result will be zero. If the
    /// [`height`](Size::height) is zero (and the [`width`](Size::width) is not),
    /// the result will be [`f64::INFINITY`] or [`f64::NEG_INFINITY`] as
    /// determined by the sign of [`width`](Size::width).
    ///
    /// See also:
    ///
    ///  * `AspectRatio`, a widget for giving a child widget a specific aspect
    ///    ratio.
    ///  * `FittedBox`, a widget that (in most modes) attempts to maintain a
    ///    child widget's aspect ratio while changing its size.
    pub fn aspect_ratio(&self) -> f64 {
        if self.height != 0.0 {
            return self.width / self.height;
        }
        if self.width > 0.0 {
            return f64::INFINITY;
        }
        if self.width < 0.0 {
            return f64::NEG_INFINITY;
        }
        0.0
    }

    /// An empty size, one with a zero width and a zero height.
    pub const ZERO: Size = Size::new(0.0, 0.0);

    /// A size whose [`width`](Size::width) and [`height`](Size::height) are
    /// infinite.
    ///
    /// See also:
    ///
    ///  * [`is_infinite`](Size::is_infinite), which checks whether either
    ///    dimension is infinite.
    ///  * [`is_finite`](Size::is_finite), which checks whether both dimensions
    ///    are finite.
    pub const INFINITE: Size = Size::new(f64::INFINITY, f64::INFINITY);

    /// Whether this size encloses a zero area.
    ///
    /// Negative areas are considered empty.
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// Integer (truncating) division operator.
    ///
    /// Returns a [`Size`] whose dimensions are the dimensions of this size
    /// divided by the scalar `operand`, rounded towards zero.
    pub fn truncating_div(&self, operand: f64) -> Size {
        Size::new(
            (self.width / operand).trunc(),
            (self.height / operand).trunc(),
        )
    }

    /// The lesser of the magnitudes of the [`width`](Size::width) and the
    /// [`height`](Size::height).
    pub fn shortest_side(&self) -> f64 {
        self.width.abs().min(self.height.abs())
    }

    /// The greater of the magnitudes of the [`width`](Size::width) and the
    /// [`height`](Size::height).
    pub fn longest_side(&self) -> f64 {
        self.width.abs().max(self.height.abs())
    }

    /// The offset to the intersection of the top and left edges of the
    /// rectangle described by the given [`Offset`] (which is interpreted as the
    /// top-left corner) and this [`Size`].
    ///
    /// See also [`Rect::top_left`].
    pub fn top_left(&self, origin: Offset) -> Offset {
        origin
    }

    /// The offset to the center of the top edge of the rectangle described by
    /// the given offset (which is interpreted as the top-left corner) and this
    /// size.
    ///
    /// See also [`Rect::top_center`].
    pub fn top_center(&self, origin: Offset) -> Offset {
        Offset::new(origin.dx() + self.width / 2.0, origin.dy())
    }

    /// The offset to the intersection of the top and right edges of the
    /// rectangle described by the given offset (which is interpreted as the
    /// top-left corner) and this size.
    ///
    /// See also [`Rect::top_right`].
    pub fn top_right(&self, origin: Offset) -> Offset {
        Offset::new(origin.dx() + self.width, origin.dy())
    }

    /// The offset to the center of the left edge of the rectangle described by
    /// the given offset (which is interpreted as the top-left corner) and this
    /// size.
    ///
    /// See also [`Rect::center_left`].
    pub fn center_left(&self, origin: Offset) -> Offset {
        Offset::new(origin.dx(), origin.dy() + self.height / 2.0)
    }

    /// The offset to the point halfway between the left and right and the top
    /// and bottom edges of the rectangle described by the given offset (which
    /// is interpreted as the top-left corner) and this size.
    ///
    /// See also [`Rect::center`].
    pub fn center(&self, origin: Offset) -> Offset {
        Offset::new(
            origin.dx() + self.width / 2.0,
            origin.dy() + self.height / 2.0,
        )
    }

    /// The offset to the center of the right edge of the rectangle described by
    /// the given offset (which is interpreted as the top-left corner) and this
    /// size.
    ///
    /// See also [`Rect::center_right`].
    pub fn center_right(&self, origin: Offset) -> Offset {
        Offset::new(origin.dx() + self.width, origin.dy() + self.height / 2.0)
    }

    /// The offset to the intersection of the bottom and left edges of the
    /// rectangle described by the given offset (which is interpreted as the
    /// top-left corner) and this size.
    ///
    /// See also [`Rect::bottom_left`].
    pub fn bottom_left(&self, origin: Offset) -> Offset {
        Offset::new(origin.dx(), origin.dy() + self.height)
    }

    /// The offset to the center of the bottom edge of the rectangle described
    /// by the given offset (which is interpreted as the top-left corner) and
    /// this size.
    ///
    /// See also [`Rect::bottom_center`].
    pub fn bottom_center(&self, origin: Offset) -> Offset {
        Offset::new(origin.dx() + self.width / 2.0, origin.dy() + self.height)
    }

    /// The offset to the intersection of the bottom and right edges of the
    /// rectangle described by the given offset (which is interpreted as the
    /// top-left corner) and this size.
    ///
    /// See also [`Rect::bottom_right`].
    pub fn bottom_right(&self, origin: Offset) -> Offset {
        Offset::new(origin.dx() + self.width, origin.dy() + self.height)
    }

    /// Whether the point specified by the given offset (which is assumed to be
    /// relative to the top left of the size) lies between the left and right
    /// and the top and bottom edges of a rectangle of this size.
    ///
    /// Rectangles include their top and left edges but exclude their bottom and
    /// right edges.
    pub fn contains(&self, offset: Offset) -> bool {
        offset.dx() >= 0.0
            && offset.dx() < self.width
            && offset.dy() >= 0.0
            && offset.dy() < self.height
    }

    /// A [`Size`] with the [`width`](Size::width) and [`height`](Size::height)
    /// swapped.
    pub fn flipped(&self) -> Size {
        Size::new(self.height, self.width)
    }

    /// Linearly interpolate between two sizes.
    ///
    /// If either size is null, this function interpolates from [`Size::ZERO`].
    ///
    /// The `t` argument represents position on the timeline, with 0.0 meaning
    /// that the interpolation has not started, returning `a` (or something
    /// equivalent to `a`), 1.0 meaning that the interpolation has finished,
    /// returning `b` (or something equivalent to `b`), and values in between
    /// meaning that the interpolation is at the relevant point on the timeline
    /// between `a` and `b`. The interpolation can be extrapolated beyond 0.0 and
    /// 1.0, so negative values and values greater than 1.0 are valid (and can
    /// easily be generated by curves such as `Curves.elasticInOut`).
    ///
    /// Values for `t` are usually obtained from an `Animation<double>`, such as
    /// an `AnimationController`.
    pub fn lerp(a: Option<Size>, b: Option<Size>, t: f64) -> Option<Size> {
        match (a, b) {
            (None, None) => None,
            (Some(a), None) => Some(a * (1.0 - t)),
            (None, Some(b)) => Some(b * t),
            (Some(a), Some(b)) => Some(Size::new(
                lerp_double_non_null(a.width, b.width, t),
                lerp_double_non_null(a.height, b.height, t),
            )),
        }
    }
}

/// Binary subtraction operator for [`Size`].
///
/// Subtracting a [`Size`] from a [`Size`] returns the [`Offset`] that describes
/// how much bigger the left-hand-side operand is than the right-hand-side
/// operand. Adding that resulting [`Offset`] to the [`Size`] that was the
/// right-hand-side operand would return a [`Size`] equal to the [`Size`] that
/// was the left-hand-side operand. (i.e. if `size_a - size_b -> offset_a`, then
/// `offset_a + size_b -> size_a`)
impl Sub<Size> for Size {
    type Output = Offset;

    fn sub(self, other: Size) -> Offset {
        Offset::new(self.width - other.width, self.height - other.height)
    }
}

/// Subtracting an [`Offset`] from a [`Size`] returns the [`Size`] that is
/// smaller than the [`Size`] operand by the difference given by the [`Offset`]
/// operand. In other words, the returned [`Size`] has a [`width`](Size::width)
/// consisting of the [`width`](Size::width) of the left-hand-side operand minus
/// the [`Offset::dx`] dimension of the right-hand-side operand, and a
/// [`height`](Size::height) consisting of the [`height`](Size::height) of the
/// left-hand-side operand minus the [`Offset::dy`] dimension of the
/// right-hand-side operand.
impl Sub<Offset> for Size {
    type Output = Size;

    fn sub(self, other: Offset) -> Size {
        Size::new(self.width - other.dx(), self.height - other.dy())
    }
}

/// Binary addition operator for adding an [`Offset`] to a [`Size`].
///
/// Returns a [`Size`] whose [`width`](Size::width) is the sum of the
/// [`width`](Size::width) of the left-hand-side operand, a [`Size`], and the
/// [`Offset::dx`] dimension of the right-hand-side operand, an [`Offset`], and
/// whose [`height`](Size::height) is the sum of the [`height`](Size::height) of
/// the left-hand-side operand and the [`Offset::dy`] dimension of the
/// right-hand-side operand.
impl Add<Offset> for Size {
    type Output = Size;

    fn add(self, other: Offset) -> Size {
        Size::new(self.width + other.dx(), self.height + other.dy())
    }
}

/// Multiplication operator.
///
/// Returns a [`Size`] whose dimensions are the dimensions of the left-hand-side
/// operand (a [`Size`]) multiplied by the scalar right-hand-side operand (an
/// `f64`).
impl Mul<f64> for Size {
    type Output = Size;

    fn mul(self, operand: f64) -> Size {
        Size::new(self.width * operand, self.height * operand)
    }
}

/// Division operator.
///
/// Returns a [`Size`] whose dimensions are the dimensions of the left-hand-side
/// operand (a [`Size`]) divided by the scalar right-hand-side operand (an
/// `f64`).
impl Div<f64> for Size {
    type Output = Size;

    fn div(self, operand: f64) -> Size {
        Size::new(self.width / operand, self.height / operand)
    }
}

/// Modulo (remainder) operator.
///
/// Returns a [`Size`] whose dimensions are the remainder of dividing the
/// left-hand-side operand (a [`Size`]) by the scalar right-hand-side operand
/// (an `f64`).
impl Rem<f64> for Size {
    type Output = Size;

    fn rem(self, operand: f64) -> Size {
        Size::new(
            self.width.rem_euclid(operand),
            self.height.rem_euclid(operand),
        )
    }
}

/// Rectangle constructor operator.
///
/// Combines an [`Offset`] and a [`Size`] to form a [`Rect`] whose top-left
/// coordinate is the point given by adding this offset, the left-hand-side
/// operand, to the origin, and whose size is the right-hand-side operand.
///
/// ```
/// # use reveal_geometry::{Offset, Rect, Size};
/// let my_rect = Offset::ZERO & Size::new(100.0, 100.0);
/// // same as: Rect::from_ltwh(0.0, 0.0, 100.0, 100.0)
/// # assert_eq!(my_rect, Rect::from_ltwh(0.0, 0.0, 100.0, 100.0));
/// ```
impl BitAnd<Size> for Offset {
    type Output = Rect;

    fn bitand(self, other: Size) -> Rect {
        Rect::from_ltwh(self.dx(), self.dy(), other.width, other.height)
    }
}

impl Debug for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Size({:.1}, {:.1})", self.width, self.height)
    }
}

/// An immutable, 2D, axis-aligned, floating-point rectangle whose coordinates
/// are relative to a given origin.
///
/// A Rect can be created with one of its constructors or from an [`Offset`] and
/// a [`Size`] using the `&` operator:
///
/// ```
/// # use reveal_geometry::{Offset, Rect, Size};
/// let my_rect = Offset::new(1.0, 2.0) & Size::new(3.0, 4.0);
/// # assert_eq!(my_rect, Rect::from_ltwh(1.0, 2.0, 3.0, 4.0));
/// ```
#[derive(Clone, Copy, PartialEq)]
pub struct Rect {
    /// The offset of the left edge of this rectangle from the x axis.
    pub left: f64,

    /// The offset of the top edge of this rectangle from the y axis.
    pub top: f64,

    /// The offset of the right edge of this rectangle from the x axis.
    pub right: f64,

    /// The offset of the bottom edge of this rectangle from the y axis.
    pub bottom: f64,
}

impl Rect {
    /// Construct a rectangle from its left, top, right, and bottom edges.
    pub const fn from_ltrb(left: f64, top: f64, right: f64, bottom: f64) -> Rect {
        Rect {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Construct a rectangle from its left and top edges, its width, and its
    /// height.
    ///
    /// To construct a [`Rect`] from an [`Offset`] and a [`Size`], you can use
    /// the rectangle constructor operator `&`. See [`BitAnd`] for [`Offset`].
    pub const fn from_ltwh(left: f64, top: f64, width: f64, height: f64) -> Rect {
        Rect::from_ltrb(left, top, left + width, top + height)
    }

    /// Construct a rectangle that bounds the given circle.
    ///
    /// The `center` argument is assumed to be an offset from the origin.
    pub fn from_circle(center: Offset, radius: f64) -> Rect {
        Rect::from_center(center, radius * 2.0, radius * 2.0)
    }

    /// Constructs a rectangle from its center point, width, and height.
    ///
    /// The `center` argument is assumed to be an offset from the origin.
    pub fn from_center(center: Offset, width: f64, height: f64) -> Rect {
        Rect::from_ltrb(
            center.dx() - width / 2.0,
            center.dy() - height / 2.0,
            center.dx() + width / 2.0,
            center.dy() + height / 2.0,
        )
    }

    /// Construct the smallest rectangle that encloses the given offsets,
    /// treating them as vectors from the origin.
    pub fn from_points(a: Offset, b: Offset) -> Rect {
        Rect::from_ltrb(
            a.dx().min(b.dx()),
            a.dy().min(b.dy()),
            a.dx().max(b.dx()),
            a.dy().max(b.dy()),
        )
    }

    /// The distance between the left and right edges of this rectangle.
    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    /// The distance between the top and bottom edges of this rectangle.
    pub fn height(&self) -> f64 {
        self.bottom - self.top
    }

    /// The distance between the upper-left corner and the lower-right corner of
    /// this rectangle.
    pub fn size(&self) -> Size {
        Size::new(self.width(), self.height())
    }

    /// Whether any of the dimensions are `NaN`.
    pub fn has_nan(&self) -> bool {
        self.left.is_nan() || self.top.is_nan() || self.right.is_nan() || self.bottom.is_nan()
    }

    /// A rectangle with left, top, right, and bottom edges all at zero.
    pub const ZERO: Rect = Rect::from_ltrb(0.0, 0.0, 0.0, 0.0);

    const GIANT_SCALAR: f64 = 1.0E+9; // matches kGiantRect from layer.h

    /// A rectangle that covers the entire coordinate space.
    ///
    /// This covers the space from -1e9,-1e9 to 1e9,1e9. This is the space over
    /// which graphics operations are valid.
    pub const LARGEST: Rect = Rect::from_ltrb(
        -Rect::GIANT_SCALAR,
        -Rect::GIANT_SCALAR,
        Rect::GIANT_SCALAR,
        Rect::GIANT_SCALAR,
    );

    /// Whether any of the coordinates of this rectangle are equal to positive
    /// infinity.
    // included for consistency with Offset and Size
    pub fn is_infinite(&self) -> bool {
        self.left >= f64::INFINITY
            || self.top >= f64::INFINITY
            || self.right >= f64::INFINITY
            || self.bottom >= f64::INFINITY
    }

    /// Whether all coordinates of this rectangle are finite.
    pub fn is_finite(&self) -> bool {
        self.left.is_finite()
            && self.top.is_finite()
            && self.right.is_finite()
            && self.bottom.is_finite()
    }

    /// Whether this rectangle encloses a non-zero area. Negative areas are
    /// considered empty.
    pub fn is_empty(&self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }

    /// Returns a new rectangle translated by the given offset.
    ///
    /// To translate a rectangle by separate x and y components rather than by
    /// an [`Offset`], consider [`translate`](Rect::translate).
    pub fn shift(&self, offset: Offset) -> Rect {
        Rect::from_ltrb(
            self.left + offset.dx(),
            self.top + offset.dy(),
            self.right + offset.dx(),
            self.bottom + offset.dy(),
        )
    }

    /// Returns a new rectangle with translateX added to the x components and
    /// translateY added to the y components.
    ///
    /// To translate a rectangle by an [`Offset`] rather than by separate x and
    /// y components, consider [`shift`](Rect::shift).
    pub fn translate(&self, translate_x: f64, translate_y: f64) -> Rect {
        Rect::from_ltrb(
            self.left + translate_x,
            self.top + translate_y,
            self.right + translate_x,
            self.bottom + translate_y,
        )
    }

    /// Returns a new rectangle with edges moved outwards by the given delta.
    pub fn inflate(&self, delta: f64) -> Rect {
        Rect::from_ltrb(
            self.left - delta,
            self.top - delta,
            self.right + delta,
            self.bottom + delta,
        )
    }

    /// Returns a new rectangle with edges moved inwards by the given delta.
    pub fn deflate(&self, delta: f64) -> Rect {
        self.inflate(-delta)
    }

    /// Returns a new rectangle that is the intersection of the given rectangle
    /// and this rectangle. The two rectangles must overlap for this to be
    /// meaningful. If the two rectangles do not overlap, then the resulting
    /// Rect will have a negative width or height.
    pub fn intersect(&self, other: Rect) -> Rect {
        Rect::from_ltrb(
            self.left.max(other.left),
            self.top.max(other.top),
            self.right.min(other.right),
            self.bottom.min(other.bottom),
        )
    }

    /// Returns a new rectangle which is the bounding box containing this
    /// rectangle and the given rectangle.
    pub fn expand_to_include(&self, other: Rect) -> Rect {
        Rect::from_ltrb(
            self.left.min(other.left),
            self.top.min(other.top),
            self.right.max(other.right),
            self.bottom.max(other.bottom),
        )
    }

    /// Whether `other` has a nonzero area of overlap with this rectangle.
    pub fn overlaps(&self, other: Rect) -> bool {
        if self.right <= other.left || other.right <= self.left {
            return false;
        }
        if self.bottom <= other.top || other.bottom <= self.top {
            return false;
        }
        true
    }

    /// The lesser of the magnitudes of the [`width`](Rect::width) and the
    /// [`height`](Rect::height) of this rectangle.
    pub fn shortest_side(&self) -> f64 {
        self.width().abs().min(self.height().abs())
    }

    /// The greater of the magnitudes of the [`width`](Rect::width) and the
    /// [`height`](Rect::height) of this rectangle.
    pub fn longest_side(&self) -> f64 {
        self.width().abs().max(self.height().abs())
    }

    /// The offset to the intersection of the top and left edges of this
    /// rectangle.
    ///
    /// See also [`Size::top_left`].
    pub fn top_left(&self) -> Offset {
        Offset::new(self.left, self.top)
    }

    /// The offset to the center of the top edge of this rectangle.
    ///
    /// See also [`Size::top_center`].
    pub fn top_center(&self) -> Offset {
        Offset::new(self.left + self.width() / 2.0, self.top)
    }

    /// The offset to the intersection of the top and right edges of this
    /// rectangle.
    ///
    /// See also [`Size::top_right`].
    pub fn top_right(&self) -> Offset {
        Offset::new(self.right, self.top)
    }

    /// The offset to the center of the left edge of this rectangle.
    ///
    /// See also [`Size::center_left`].
    pub fn center_left(&self) -> Offset {
        Offset::new(self.left, self.top + self.height() / 2.0)
    }

    /// The offset to the point halfway between the left and right and the top
    /// and bottom edges of this rectangle.
    ///
    /// See also [`Size::center`].
    pub fn center(&self) -> Offset {
        Offset::new(
            self.left + self.width() / 2.0,
            self.top + self.height() / 2.0,
        )
    }

    /// The offset to the center of the right edge of this rectangle.
    ///
    /// See also [`Size::center_right`].
    pub fn center_right(&self) -> Offset {
        Offset::new(self.right, self.top + self.height() / 2.0)
    }

    /// The offset to the intersection of the bottom and left edges of this
    /// rectangle.
    ///
    /// See also [`Size::bottom_left`].
    pub fn bottom_left(&self) -> Offset {
        Offset::new(self.left, self.bottom)
    }

    /// The offset to the center of the bottom edge of this rectangle.
    ///
    /// See also [`Size::bottom_center`].
    pub fn bottom_center(&self) -> Offset {
        Offset::new(self.left + self.width() / 2.0, self.bottom)
    }

    /// The offset to the intersection of the bottom and right edges of this
    /// rectangle.
    ///
    /// See also [`Size::bottom_right`].
    pub fn bottom_right(&self) -> Offset {
        Offset::new(self.right, self.bottom)
    }

    /// Whether the point specified by the given offset (which is assumed to be
    /// relative to the origin) lies between the left and right and the top and
    /// bottom edges of this rectangle.
    ///
    /// Rectangles include their top and left edges but exclude their bottom and
    /// right edges.
    pub fn contains(&self, offset: Offset) -> bool {
        offset.dx() >= self.left
            && offset.dx() < self.right
            && offset.dy() >= self.top
            && offset.dy() < self.bottom
    }

    /// Linearly interpolate between two rectangles.
    ///
    /// If either rect is null, [`Rect::ZERO`] is used as a substitute.
    ///
    /// The `t` argument represents position on the timeline, with 0.0 meaning
    /// that the interpolation has not started, returning `a` (or something
    /// equivalent to `a`), 1.0 meaning that the interpolation has finished,
    /// returning `b` (or something equivalent to `b`), and values in between
    /// meaning that the interpolation is at the relevant point on the timeline
    /// between `a` and `b`. The interpolation can be extrapolated beyond 0.0 and
    /// 1.0, so negative values and values greater than 1.0 are valid (and can
    /// easily be generated by curves such as `Curves.elasticInOut`).
    ///
    /// Values for `t` are usually obtained from an `Animation<double>`, such as
    /// an `AnimationController`.
    pub fn lerp(a: Option<Rect>, b: Option<Rect>, t: f64) -> Option<Rect> {
        match (a, b) {
            (None, None) => None,
            (Some(a), None) => {
                let k = 1.0 - t;
                Some(Rect::from_ltrb(
                    a.left * k,
                    a.top * k,
                    a.right * k,
                    a.bottom * k,
                ))
            }
            (None, Some(b)) => Some(Rect::from_ltrb(
                b.left * t,
                b.top * t,
                b.right * t,
                b.bottom * t,
            )),
            (Some(a), Some(b)) => Some(Rect::from_ltrb(
                lerp_double_non_null(a.left, b.left, t),
                lerp_double_non_null(a.top, b.top, t),
                lerp_double_non_null(a.right, b.right, t),
                lerp_double_non_null(a.bottom, b.bottom, t),
            )),
        }
    }
}

impl Debug for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Rect.fromLTRB({:.1}, {:.1}, {:.1}, {:.1})",
            self.left, self.top, self.right, self.bottom
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_offset_is_equal_to_another_with_the_same_components() {
        assert_eq!(Offset::new(1.0, 2.0), Offset::new(1.0, 2.0));
        assert_ne!(Offset::new(1.0, 2.0), Offset::new(2.0, 1.0));
    }

    #[test]
    fn arithmetic_operators_work_component_wise() {
        let a = Offset::new(10.0, 20.0);
        let b = Offset::new(1.0, 2.0);

        assert_eq!(a + b, Offset::new(11.0, 22.0));
        assert_eq!(a - b, Offset::new(9.0, 18.0));
        assert_eq!(a * 2.0, Offset::new(20.0, 40.0));
        assert_eq!(a / 2.0, Offset::new(5.0, 10.0));
        assert_eq!(-a, Offset::new(-10.0, -20.0));
        assert_eq!(a.scale(2.0, 3.0), Offset::new(20.0, 60.0));
        assert_eq!(a.translate(1.0, -1.0), Offset::new(11.0, 19.0));
    }

    #[test]
    fn remainder_follows_dart_and_stays_non_negative() {
        assert_eq!(Offset::new(-5.0, 5.0) % 3.0, Offset::new(1.0, 2.0));
        assert_eq!(-5.0_f64 % 3.0, -2.0);
    }

    #[test]
    fn truncating_division_rounds_towards_zero() {
        assert_eq!(
            Offset::new(-5.0, 5.0).truncating_div(3.0),
            Offset::new(-1.0, 1.0)
        );
    }

    #[test]
    fn distance_and_direction_describe_the_same_vector_as_the_components() {
        let offset = Offset::new(3.0, 4.0);
        assert_eq!(offset.distance(), 5.0);
        assert_eq!(offset.distance_squared(), 25.0);

        let rebuilt = Offset::from_direction(offset.direction(), offset.distance());
        assert!((rebuilt.dx() - 3.0).abs() < 1e-12);
        assert!((rebuilt.dy() - 4.0).abs() < 1e-12);
    }

    #[test]
    fn comparison_is_component_wise_and_only_partial() {
        let a = Offset::new(1.0, 2.0);
        let b = Offset::new(2.0, 3.0);
        assert!(a.lt(&b));
        assert!(a.le(&b));
        assert!(b.gt(&a));
        assert!(b.ge(&a));

        let c = Offset::new(1.0, 3.0);
        let d = Offset::new(2.0, 2.0);
        assert!(!c.lt(&d) && !c.le(&d) && !c.gt(&d) && !c.ge(&d));

        let e = Offset::new(1.0, 2.0);
        let f = Offset::new(1.0, 3.0);
        assert!(e.le(&f));
        assert!(!e.lt(&f));
        assert_ne!(e, f);
    }

    #[test]
    fn finiteness_asks_about_either_component_or_both() {
        assert!(Offset::INFINITE.is_infinite());
        assert!(Offset::new(0.0, f64::INFINITY).is_infinite());
        assert!(!Offset::new(0.0, f64::NEG_INFINITY).is_infinite());
        assert!(!Offset::new(0.0, f64::NAN).is_infinite());

        assert!(Offset::ZERO.is_finite());
        assert!(!Offset::new(0.0, f64::NAN).is_finite());
    }

    #[test]
    fn lerp_defaults_a_missing_end_to_zero() {
        let a = Offset::new(0.0, 10.0);
        let b = Offset::new(10.0, 0.0);

        assert_eq!(
            Offset::lerp(Some(a), Some(b), 0.5),
            Some(Offset::new(5.0, 5.0))
        );
        assert_eq!(
            Offset::lerp(Some(a), None, 0.5),
            Some(Offset::new(0.0, 5.0))
        );
        assert_eq!(
            Offset::lerp(None, Some(b), 0.5),
            Some(Offset::new(5.0, 0.0))
        );
        assert_eq!(Offset::lerp(None, None, 0.5), None);
    }

    #[test]
    fn debug_matches_flutter_to_string() {
        assert_eq!(format!("{:?}", Offset::new(1.0, 2.5)), "Offset(1.0, 2.5)");
    }

    // engine geometry_test.dart 'Size.aspectRatio'.
    #[test]
    fn size_aspect_ratio_covers_the_zero_and_infinite_arms() {
        assert_eq!(Size::new(0.0, 0.0).aspect_ratio(), 0.0);
        assert_eq!(Size::new(-0.0, 0.0).aspect_ratio(), 0.0);
        assert_eq!(Size::new(1.0, 0.0).aspect_ratio(), f64::INFINITY);
        assert_eq!(Size::new(-1.0, 0.0).aspect_ratio(), f64::NEG_INFINITY);
        assert_eq!(Size::new(3.0, 2.0).aspect_ratio(), 1.5);
        assert_eq!(Size::new(3.0, -2.0).aspect_ratio(), -1.5);
        assert_eq!(Size::new(-3.0, 2.0).aspect_ratio(), -1.5);
        assert_eq!(Size::new(0.0, -1.0).aspect_ratio(), -0.0);
    }

    #[test]
    fn size_shortest_and_longest_side_use_magnitudes() {
        let size = Size::new(-9.0, 4.0);
        assert_eq!(size.shortest_side(), 4.0);
        assert_eq!(size.longest_side(), 9.0);
    }

    #[test]
    fn subtracting_sizes_gives_the_offset_between_their_corners() {
        let offset: Offset = Size::new(5.0, 7.0) - Size::new(3.0, 2.0);
        assert_eq!(offset, Offset::new(2.0, 5.0));

        let size: Size = Size::new(5.0, 7.0) - Offset::new(3.0, 2.0);
        assert_eq!(size, Size::new(2.0, 5.0));

        assert_eq!(
            Size::new(5.0, 7.0) + Offset::new(3.0, 2.0),
            Size::new(8.0, 9.0)
        );
    }

    #[test]
    fn size_comparison_operators_are_component_wise() {
        let small = Size::new(1.0, 2.0);
        let large = Size::new(3.0, 4.0);
        let mixed = Size::new(5.0, 1.0);

        assert!(small.lt(&large));
        assert!(small.le(&small));
        assert!(large.gt(&small));
        assert!(!small.lt(&mixed), "only one component is smaller");
        assert!(!mixed.gt(&small));
    }

    #[test]
    fn size_lerp_matches_darts_null_arms() {
        assert_eq!(Size::lerp(None, None, 0.5), None);
        assert_eq!(
            Size::lerp(Some(Size::new(4.0, 8.0)), None, 0.25),
            Some(Size::new(3.0, 6.0))
        );
        assert_eq!(
            Size::lerp(None, Some(Size::new(4.0, 8.0)), 0.25),
            Some(Size::new(1.0, 2.0))
        );
        assert_eq!(
            Size::lerp(Some(Size::ZERO), Some(Size::new(4.0, 8.0)), 0.5),
            Some(Size::new(2.0, 4.0))
        );
    }

    #[test]
    fn size_contains_includes_top_left_and_excludes_bottom_right() {
        let size = Size::new(2.0, 2.0);
        assert!(size.contains(Offset::ZERO));
        assert!(size.contains(Offset::new(1.9, 1.9)));
        assert!(!size.contains(Offset::new(2.0, 1.0)));
        assert!(!size.contains(Offset::new(-0.1, 1.0)));
    }

    // engine geometry_test.dart 'Rect accessors': fromLTRB(1,3,5,7).
    #[test]
    fn rect_accessors() {
        let rect = Rect::from_ltrb(1.0, 3.0, 5.0, 7.0);
        assert_eq!(rect.left, 1.0);
        assert_eq!(rect.top, 3.0);
        assert_eq!(rect.right, 5.0);
        assert_eq!(rect.bottom, 7.0);
        assert_eq!(rect.width(), 4.0);
        assert_eq!(rect.height(), 4.0);
        assert_eq!(rect.size(), Size::new(4.0, 4.0));
        assert_eq!(rect.center(), Offset::new(3.0, 5.0));
    }

    // engine geometry_test.dart 'Rect created by width and height'.
    #[test]
    fn rect_from_ltwh_matches_ltrb() {
        assert_eq!(
            Rect::from_ltwh(1.0, 3.0, 4.0, 4.0),
            Rect::from_ltrb(1.0, 3.0, 5.0, 7.0)
        );
    }

    // engine geometry_test.dart 'Rect created from center': 5x7 about (1, 3),
    // plus the NaN case.
    #[test]
    fn rect_from_center() {
        let rect = Rect::from_center(Offset::new(1.0, 3.0), 5.0, 7.0);
        assert_eq!(rect, Rect::from_ltrb(-1.5, -0.5, 3.5, 6.5));

        let nan = Rect::from_center(Offset::new(f64::NAN, 3.0), 5.0, 7.0);
        assert!(nan.has_nan());

        assert_eq!(
            Rect::from_points(Offset::new(5.0, 7.0), Offset::new(1.0, 3.0)),
            Rect::from_ltrb(1.0, 3.0, 5.0, 7.0)
        );
    }

    // engine geometry_test.dart 'Rect.toString'.
    #[test]
    fn rect_debug_matches_darts_to_string() {
        assert_eq!(
            format!("{:?}", Rect::from_ltrb(1.0, 3.0, 5.0, 7.0)),
            "Rect.fromLTRB(1.0, 3.0, 5.0, 7.0)"
        );
        assert_eq!(format!("{:?}", Size::new(2.0, 3.0)), "Size(2.0, 3.0)");
    }

    #[test]
    fn an_offset_and_a_size_make_a_rect() {
        let from_origin = Offset::ZERO & Size::new(100.0, 100.0);
        assert_eq!(from_origin, Rect::from_ltwh(0.0, 0.0, 100.0, 100.0));

        let rect = Offset::new(1.0, 2.0) & Size::new(3.0, 4.0);
        assert_eq!(rect, Rect::from_ltwh(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn rect_intersect_and_overlaps_agree_on_the_shared_edge() {
        let a = Rect::from_ltrb(0.0, 0.0, 10.0, 10.0);
        let b = Rect::from_ltrb(5.0, 5.0, 15.0, 15.0);
        assert_eq!(a.intersect(b), Rect::from_ltrb(5.0, 5.0, 10.0, 10.0));
        assert!(a.overlaps(b));

        let touching = Rect::from_ltrb(10.0, 0.0, 20.0, 10.0);
        assert!(!a.overlaps(touching), "a shared edge is not an overlap");
        assert!(a.intersect(touching).is_empty());

        assert_eq!(
            a.expand_to_include(touching),
            Rect::from_ltrb(0.0, 0.0, 20.0, 10.0)
        );

        let zero_area = Rect::from_ltrb(-5.0, 2.0, -5.0, 2.0);
        assert_eq!(
            a.expand_to_include(zero_area),
            Rect::from_ltrb(-5.0, 0.0, 10.0, 10.0)
        );
    }

    #[test]
    fn rect_contains_includes_top_left_and_excludes_bottom_right() {
        let rect = Rect::from_ltrb(1.0, 1.0, 2.0, 2.0);
        assert!(rect.contains(Offset::new(1.0, 1.0)));
        assert!(!rect.contains(Offset::new(2.0, 2.0)));
        assert!(!rect.contains(Offset::new(2.0, 1.5)));
    }

    #[test]
    fn rect_lerp_matches_darts_null_arms() {
        let rect = Rect::from_ltrb(4.0, 8.0, 12.0, 16.0);
        assert_eq!(Rect::lerp(None, None, 0.5), None);
        assert_eq!(
            Rect::lerp(Some(rect), None, 0.5),
            Some(Rect::from_ltrb(2.0, 4.0, 6.0, 8.0))
        );
        assert_eq!(
            Rect::lerp(None, Some(rect), 0.5),
            Some(Rect::from_ltrb(2.0, 4.0, 6.0, 8.0))
        );
        assert_eq!(
            Rect::lerp(Some(Rect::ZERO), Some(rect), 0.5),
            Some(Rect::from_ltrb(2.0, 4.0, 6.0, 8.0))
        );
    }

    #[test]
    fn rect_shift_translate_inflate_deflate() {
        let rect = Rect::from_ltrb(0.0, 0.0, 4.0, 4.0);
        assert_eq!(
            rect.shift(Offset::new(1.0, 2.0)),
            Rect::from_ltrb(1.0, 2.0, 5.0, 6.0)
        );
        assert_eq!(rect.translate(1.0, 2.0), rect.shift(Offset::new(1.0, 2.0)));
        assert_eq!(rect.inflate(1.0), Rect::from_ltrb(-1.0, -1.0, 5.0, 5.0));
        assert_eq!(rect.inflate(1.0).deflate(1.0), rect);
    }
}
