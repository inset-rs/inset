//! Flutter counterpart: `painting/edge_insets.dart`.

use std::fmt::{self, Debug};
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

use inset_embedder::{Offset, RRect, Radius, Rect, Size, clamp_double, lerp_double};

use crate::basic_types::{Axis, TextDirection};

/// Base class for [`EdgeInsets`] that allows for text-direction aware
/// resolution.
///
/// A property or argument of this type accepts classes created either with
/// [`EdgeInsets::from_ltrb`] and its variants, or
/// [`EdgeInsetsDirectional::from_steb`] and its variants.
///
/// To convert an [`EdgeInsetsGeometry`] object of indeterminate type into an
/// [`EdgeInsets`] object, call the [`resolve`](EdgeInsetsGeometry::resolve)
/// method.
///
/// See also:
///
///  * `Padding`, a widget that describes margins using [`EdgeInsetsGeometry`].
///
/// Non-exhaustive: Flutter may add a subclass.
#[non_exhaustive]
#[derive(Clone, Copy)]
pub enum EdgeInsetsGeometry {
    /// Insets with fixed visual sides — Dart's `EdgeInsets`.
    Insets(EdgeInsets),
    /// Insets relative to the writing direction — Dart's `EdgeInsetsDirectional`.
    Directional(EdgeInsetsDirectional),
    /// Visual and directional offsets at once — Dart's private `_MixedEdgeInsets`,
    /// what combining the two kinds produces.
    Mixed {
        /// The offset from the left.
        left: f64,
        /// The offset from the right.
        right: f64,
        /// The offset from the start side.
        start: f64,
        /// The offset from the end side.
        end: f64,
        /// The offset from the top.
        top: f64,
        /// The offset from the bottom.
        bottom: f64,
    },
}

impl EdgeInsetsGeometry {
    /// Creates insets where all the offsets are `value`.
    pub const fn all(value: f64) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::Insets(EdgeInsets::all(value))
    }

    /// Creates [`EdgeInsets`] with only the given values non-zero.
    pub const fn only(left: f64, top: f64, right: f64, bottom: f64) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::Insets(EdgeInsets::only(left, top, right, bottom))
    }

    /// Creates [`EdgeInsetsDirectional`] with only the given values non-zero.
    pub const fn directional(start: f64, top: f64, end: f64, bottom: f64) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::Directional(EdgeInsetsDirectional::only(start, top, end, bottom))
    }

    /// Creates [`EdgeInsets`] with symmetrical vertical and horizontal offsets.
    pub const fn symmetric(vertical: f64, horizontal: f64) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::Insets(EdgeInsets::symmetric(vertical, horizontal))
    }

    /// Creates [`EdgeInsets`] from offsets from the left, top, right, and
    /// bottom.
    pub const fn from_ltrb(left: f64, top: f64, right: f64, bottom: f64) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::Insets(EdgeInsets::from_ltrb(left, top, right, bottom))
    }

    /// Creates [`EdgeInsetsDirectional`] from offsets from the start, top, end,
    /// and bottom.
    pub const fn from_steb(start: f64, top: f64, end: f64, bottom: f64) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::Directional(EdgeInsetsDirectional::from_steb(start, top, end, bottom))
    }

    /// An [`EdgeInsets`] with zero offsets in each direction.
    pub const ZERO: EdgeInsetsGeometry = EdgeInsetsGeometry::Insets(EdgeInsets::ZERO);

    fn left(&self) -> f64 {
        match self {
            EdgeInsetsGeometry::Insets(insets) => insets.left,
            EdgeInsetsGeometry::Directional(_) => 0.0,
            EdgeInsetsGeometry::Mixed { left, .. } => *left,
        }
    }

    fn right(&self) -> f64 {
        match self {
            EdgeInsetsGeometry::Insets(insets) => insets.right,
            EdgeInsetsGeometry::Directional(_) => 0.0,
            EdgeInsetsGeometry::Mixed { right, .. } => *right,
        }
    }

    fn start(&self) -> f64 {
        match self {
            EdgeInsetsGeometry::Insets(_) => 0.0,
            EdgeInsetsGeometry::Directional(insets) => insets.start,
            EdgeInsetsGeometry::Mixed { start, .. } => *start,
        }
    }

    fn end(&self) -> f64 {
        match self {
            EdgeInsetsGeometry::Insets(_) => 0.0,
            EdgeInsetsGeometry::Directional(insets) => insets.end,
            EdgeInsetsGeometry::Mixed { end, .. } => *end,
        }
    }

    fn top(&self) -> f64 {
        match self {
            EdgeInsetsGeometry::Insets(insets) => insets.top,
            EdgeInsetsGeometry::Directional(insets) => insets.top,
            EdgeInsetsGeometry::Mixed { top, .. } => *top,
        }
    }

    fn bottom(&self) -> f64 {
        match self {
            EdgeInsetsGeometry::Insets(insets) => insets.bottom,
            EdgeInsetsGeometry::Directional(insets) => insets.bottom,
            EdgeInsetsGeometry::Mixed { bottom, .. } => *bottom,
        }
    }

    /// Dart's private `_MixedEdgeInsets.fromLRSETB`.
    const fn from_lrsetb(
        left: f64,
        right: f64,
        start: f64,
        end: f64,
        top: f64,
        bottom: f64,
    ) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::Mixed {
            left,
            right,
            start,
            end,
            top,
            bottom,
        }
    }

    /// An [`EdgeInsetsGeometry`] with infinite offsets in each direction.
    ///
    /// Can be used as an infinite upper bound for [`clamp`](Self::clamp).
    pub const INFINITY: EdgeInsetsGeometry = EdgeInsetsGeometry::from_lrsetb(
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
    );

    /// Whether every dimension is non-negative.
    pub fn is_non_negative(&self) -> bool {
        self.left() >= 0.0
            && self.right() >= 0.0
            && self.start() >= 0.0
            && self.end() >= 0.0
            && self.top() >= 0.0
            && self.bottom() >= 0.0
    }

    /// The total offset in the horizontal direction.
    pub fn horizontal(&self) -> f64 {
        self.left() + self.right() + self.start() + self.end()
    }

    /// The total offset in the vertical direction.
    pub fn vertical(&self) -> f64 {
        self.top() + self.bottom()
    }

    /// The total offset in the given direction.
    pub fn along(&self, axis: Axis) -> f64 {
        match axis {
            Axis::Horizontal => self.horizontal(),
            Axis::Vertical => self.vertical(),
        }
    }

    /// The size that this [`EdgeInsets`] would occupy with an empty interior.
    pub fn collapsed_size(&self) -> Size {
        Size::new(self.horizontal(), self.vertical())
    }

    /// An [`EdgeInsetsGeometry`] with top and bottom, left and right, and start
    /// and end flipped.
    pub fn flipped(&self) -> EdgeInsetsGeometry {
        match self {
            EdgeInsetsGeometry::Insets(insets) => EdgeInsetsGeometry::Insets(insets.flipped()),
            EdgeInsetsGeometry::Directional(insets) => {
                EdgeInsetsGeometry::Directional(insets.flipped())
            }
            EdgeInsetsGeometry::Mixed { .. } => EdgeInsetsGeometry::from_lrsetb(
                self.right(),
                self.left(),
                self.end(),
                self.start(),
                self.bottom(),
                self.top(),
            ),
        }
    }

    /// Returns a new size that is bigger than the given size by the amount of
    /// inset in the horizontal and vertical directions.
    ///
    /// See also:
    ///
    ///  * [`EdgeInsets::inflate_rect`], to inflate a [`Rect`] rather than a
    ///    [`Size`] (for [`EdgeInsetsDirectional`], requires first calling
    ///    [`resolve`](Self::resolve) to establish how the start and end map to
    ///    the left or right).
    ///  * [`deflate_size`](Self::deflate_size), to deflate a [`Size`] rather
    ///    than inflating it.
    pub fn inflate_size(&self, size: Size) -> Size {
        Size::new(
            size.width() + self.horizontal(),
            size.height() + self.vertical(),
        )
    }

    /// Returns a new size that is smaller than the given size by the amount of
    /// inset in the horizontal and vertical directions.
    ///
    /// If the argument is smaller than [`collapsed_size`](Self::collapsed_size),
    /// then the resulting size will have negative dimensions.
    ///
    /// See also:
    ///
    ///  * [`EdgeInsets::deflate_rect`], to deflate a [`Rect`] rather than a
    ///    [`Size`]. (for [`EdgeInsetsDirectional`], requires first calling
    ///    [`resolve`](Self::resolve) to establish how the start and end map to
    ///    the left or right).
    ///  * [`inflate_size`](Self::inflate_size), to inflate a [`Size`] rather
    ///    than deflating it.
    pub fn deflate_size(&self, size: Size) -> Size {
        Size::new(
            size.width() - self.horizontal(),
            size.height() - self.vertical(),
        )
    }

    /// Returns the difference between two [`EdgeInsetsGeometry`] objects.
    ///
    /// If you know you are applying this to two [`EdgeInsets`] or two
    /// [`EdgeInsetsDirectional`] objects, consider using the binary infix `-`
    /// operator instead, which always returns an object of the same type as the
    /// operands, and is typed accordingly.
    ///
    /// Applied to two objects of the same kind, the result is of that kind;
    /// otherwise it is the mixed arm, which [`resolve`](Self::resolve) turns
    /// into a concrete [`EdgeInsets`].
    ///
    /// This method returns the same result as [`add`](Self::add) applied to the
    /// negation of the argument.
    pub fn subtract(self, other: EdgeInsetsGeometry) -> EdgeInsetsGeometry {
        match (self, other) {
            (EdgeInsetsGeometry::Insets(a), EdgeInsetsGeometry::Insets(b)) => {
                EdgeInsetsGeometry::Insets(a - b)
            }
            (EdgeInsetsGeometry::Directional(a), EdgeInsetsGeometry::Directional(b)) => {
                EdgeInsetsGeometry::Directional(a - b)
            }
            _ => EdgeInsetsGeometry::from_lrsetb(
                self.left() - other.left(),
                self.right() - other.right(),
                self.start() - other.start(),
                self.end() - other.end(),
                self.top() - other.top(),
                self.bottom() - other.bottom(),
            ),
        }
    }

    /// Returns the sum of two [`EdgeInsetsGeometry`] objects.
    ///
    /// If you know you are adding two [`EdgeInsets`] or two
    /// [`EdgeInsetsDirectional`] objects, consider using the `+` operator
    /// instead, which always returns an object of the same type as the operands,
    /// and is typed accordingly.
    ///
    /// Applied to two objects of the same kind, the result is of that kind;
    /// otherwise it is the mixed arm, which [`resolve`](Self::resolve) turns
    /// into a concrete [`EdgeInsets`].
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, other: EdgeInsetsGeometry) -> EdgeInsetsGeometry {
        match (self, other) {
            (EdgeInsetsGeometry::Insets(a), EdgeInsetsGeometry::Insets(b)) => {
                EdgeInsetsGeometry::Insets(a + b)
            }
            (EdgeInsetsGeometry::Directional(a), EdgeInsetsGeometry::Directional(b)) => {
                EdgeInsetsGeometry::Directional(a + b)
            }
            _ => EdgeInsetsGeometry::from_lrsetb(
                self.left() + other.left(),
                self.right() + other.right(),
                self.start() + other.start(),
                self.end() + other.end(),
                self.top() + other.top(),
                self.bottom() + other.bottom(),
            ),
        }
    }

    /// Returns a new [`EdgeInsetsGeometry`] object with all values greater than
    /// or equal to `min`, and less than or equal to `max`.
    pub fn clamp(&self, min: &EdgeInsetsGeometry, max: &EdgeInsetsGeometry) -> EdgeInsetsGeometry {
        match self {
            EdgeInsetsGeometry::Insets(insets) => {
                EdgeInsetsGeometry::Insets(insets.clamp(min, max))
            }
            _ => EdgeInsetsGeometry::from_lrsetb(
                clamp_double(self.left(), min.left(), max.left()),
                clamp_double(self.right(), min.right(), max.right()),
                clamp_double(self.start(), min.start(), max.start()),
                clamp_double(self.end(), min.end(), max.end()),
                clamp_double(self.top(), min.top(), max.top()),
                clamp_double(self.bottom(), min.bottom(), max.bottom()),
            ),
        }
    }

    /// Integer divides the [`EdgeInsetsGeometry`] object in each dimension by
    /// the given factor.
    pub fn truncating_div(&self, other: f64) -> EdgeInsetsGeometry {
        match self {
            EdgeInsetsGeometry::Insets(insets) => {
                EdgeInsetsGeometry::Insets(insets.truncating_div(other))
            }
            EdgeInsetsGeometry::Directional(insets) => {
                EdgeInsetsGeometry::Directional(insets.truncating_div(other))
            }
            EdgeInsetsGeometry::Mixed { .. } => EdgeInsetsGeometry::from_lrsetb(
                (self.left() / other).trunc(),
                (self.right() / other).trunc(),
                (self.start() / other).trunc(),
                (self.end() / other).trunc(),
                (self.top() / other).trunc(),
                (self.bottom() / other).trunc(),
            ),
        }
    }

    /// Linearly interpolate between two [`EdgeInsetsGeometry`] objects.
    ///
    /// If either is `None`, this function interpolates from
    /// [`EdgeInsets::ZERO`], and the result is of the same kind as the non-`None`
    /// argument.
    ///
    /// Applied to two objects of the same kind, the result is of that kind;
    /// otherwise it is the mixed arm, which [`resolve`](Self::resolve) turns
    /// into a concrete [`EdgeInsets`].
    pub fn lerp(
        a: Option<EdgeInsetsGeometry>,
        b: Option<EdgeInsetsGeometry>,
        t: f64,
    ) -> Option<EdgeInsetsGeometry> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b * t),
            (Some(a), None) => Some(a * (1.0 - t)),
            (Some(EdgeInsetsGeometry::Insets(a)), Some(EdgeInsetsGeometry::Insets(b))) => {
                EdgeInsets::lerp(Some(a), Some(b), t).map(EdgeInsetsGeometry::Insets)
            }
            (
                Some(EdgeInsetsGeometry::Directional(a)),
                Some(EdgeInsetsGeometry::Directional(b)),
            ) => EdgeInsetsDirectional::lerp(Some(a), Some(b), t)
                .map(EdgeInsetsGeometry::Directional),
            (Some(a), Some(b)) => Some(EdgeInsetsGeometry::from_lrsetb(
                lerp_double(Some(a.left()), Some(b.left()), t).unwrap(),
                lerp_double(Some(a.right()), Some(b.right()), t).unwrap(),
                lerp_double(Some(a.start()), Some(b.start()), t).unwrap(),
                lerp_double(Some(a.end()), Some(b.end()), t).unwrap(),
                lerp_double(Some(a.top()), Some(b.top()), t).unwrap(),
                lerp_double(Some(a.bottom()), Some(b.bottom()), t).unwrap(),
            )),
        }
    }

    /// Convert this instance into an [`EdgeInsets`], which uses literal
    /// coordinates (i.e. the `left` coordinate being explicitly a distance from
    /// the left, and the `right` coordinate being explicitly a distance from
    /// the right).
    ///
    /// See also:
    ///
    ///  * [`EdgeInsets`], for which this is a no-op (returns itself).
    ///  * [`EdgeInsetsDirectional`], which flips the horizontal direction based
    ///    on the `direction` argument.
    pub fn resolve(&self, direction: Option<TextDirection>) -> EdgeInsets {
        match self {
            EdgeInsetsGeometry::Insets(insets) => insets.resolve(direction),
            EdgeInsetsGeometry::Directional(insets) => insets.resolve(direction),
            EdgeInsetsGeometry::Mixed { .. } => {
                match direction.expect("EdgeInsetsGeometry.resolve needs a TextDirection") {
                    TextDirection::Rtl => EdgeInsets::from_ltrb(
                        self.end() + self.left(),
                        self.top(),
                        self.start() + self.right(),
                        self.bottom(),
                    ),
                    TextDirection::Ltr => EdgeInsets::from_ltrb(
                        self.start() + self.left(),
                        self.top(),
                        self.end() + self.right(),
                        self.bottom(),
                    ),
                }
            }
        }
    }
}

impl PartialEq for EdgeInsetsGeometry {
    fn eq(&self, other: &EdgeInsetsGeometry) -> bool {
        other.left() == self.left()
            && other.right() == self.right()
            && other.start() == self.start()
            && other.end() == self.end()
            && other.top() == self.top()
            && other.bottom() == self.bottom()
    }
}

impl From<EdgeInsets> for EdgeInsetsGeometry {
    fn from(insets: EdgeInsets) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::Insets(insets)
    }
}

impl From<EdgeInsetsDirectional> for EdgeInsetsGeometry {
    fn from(insets: EdgeInsetsDirectional) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::Directional(insets)
    }
}

/// Returns the [`EdgeInsetsGeometry`] object with each dimension negated.
///
/// This is the same as multiplying the object by -1.0.
///
/// This operator returns an object of the same type as the operand.
impl Neg for EdgeInsetsGeometry {
    type Output = EdgeInsetsGeometry;

    fn neg(self) -> EdgeInsetsGeometry {
        match self {
            EdgeInsetsGeometry::Insets(insets) => EdgeInsetsGeometry::Insets(-insets),
            EdgeInsetsGeometry::Directional(insets) => EdgeInsetsGeometry::Directional(-insets),
            EdgeInsetsGeometry::Mixed { .. } => EdgeInsetsGeometry::from_lrsetb(
                -self.left(),
                -self.right(),
                -self.start(),
                -self.end(),
                -self.top(),
                -self.bottom(),
            ),
        }
    }
}

/// Scales the [`EdgeInsetsGeometry`] object in each dimension by the given
/// factor.
///
/// This operator returns an object of the same type as the operand.
impl Mul<f64> for EdgeInsetsGeometry {
    type Output = EdgeInsetsGeometry;

    fn mul(self, other: f64) -> EdgeInsetsGeometry {
        match self {
            EdgeInsetsGeometry::Insets(insets) => EdgeInsetsGeometry::Insets(insets * other),
            EdgeInsetsGeometry::Directional(insets) => {
                EdgeInsetsGeometry::Directional(insets * other)
            }
            EdgeInsetsGeometry::Mixed { .. } => EdgeInsetsGeometry::from_lrsetb(
                self.left() * other,
                self.right() * other,
                self.start() * other,
                self.end() * other,
                self.top() * other,
                self.bottom() * other,
            ),
        }
    }
}

/// Divides the [`EdgeInsetsGeometry`] object in each dimension by the given
/// factor.
///
/// This operator returns an object of the same type as the operand.
impl Div<f64> for EdgeInsetsGeometry {
    type Output = EdgeInsetsGeometry;

    fn div(self, other: f64) -> EdgeInsetsGeometry {
        match self {
            EdgeInsetsGeometry::Insets(insets) => EdgeInsetsGeometry::Insets(insets / other),
            EdgeInsetsGeometry::Directional(insets) => {
                EdgeInsetsGeometry::Directional(insets / other)
            }
            EdgeInsetsGeometry::Mixed { .. } => EdgeInsetsGeometry::from_lrsetb(
                self.left() / other,
                self.right() / other,
                self.start() / other,
                self.end() / other,
                self.top() / other,
                self.bottom() / other,
            ),
        }
    }
}

/// Computes the remainder in each dimension by the given factor.
///
/// This operator returns an object of the same type as the operand.
impl Rem<f64> for EdgeInsetsGeometry {
    type Output = EdgeInsetsGeometry;

    fn rem(self, other: f64) -> EdgeInsetsGeometry {
        match self {
            EdgeInsetsGeometry::Insets(insets) => EdgeInsetsGeometry::Insets(insets % other),
            EdgeInsetsGeometry::Directional(insets) => {
                EdgeInsetsGeometry::Directional(insets % other)
            }
            EdgeInsetsGeometry::Mixed { .. } => EdgeInsetsGeometry::from_lrsetb(
                self.left() % other,
                self.right() % other,
                self.start() % other,
                self.end() % other,
                self.top() % other,
                self.bottom() % other,
            ),
        }
    }
}

impl Debug for EdgeInsetsGeometry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start() == 0.0 && self.end() == 0.0 {
            if self.left() == 0.0
                && self.right() == 0.0
                && self.top() == 0.0
                && self.bottom() == 0.0
            {
                return write!(f, "EdgeInsets.zero");
            }
            if self.left() == self.right()
                && self.right() == self.top()
                && self.top() == self.bottom()
            {
                return write!(f, "EdgeInsets.all({:.1})", self.left());
            }
            return write!(
                f,
                "EdgeInsets({:.1}, {:.1}, {:.1}, {:.1})",
                self.left(),
                self.top(),
                self.right(),
                self.bottom()
            );
        }
        if self.left() == 0.0 && self.right() == 0.0 {
            return write!(
                f,
                "EdgeInsetsDirectional({:.1}, {:.1}, {:.1}, {:.1})",
                self.start(),
                self.top(),
                self.end(),
                self.bottom()
            );
        }
        write!(
            f,
            "EdgeInsets({:.1}, {:.1}, {:.1}, {:.1}) + EdgeInsetsDirectional({:.1}, 0.0, {:.1}, 0.0)",
            self.left(),
            self.top(),
            self.right(),
            self.bottom(),
            self.start(),
            self.end()
        )
    }
}

/// An immutable set of offsets in each of the four cardinal directions.
///
/// Typically used for an offset from each of the four sides of a box. For
/// example, the padding inside a box can be represented using this class.
///
/// The [`EdgeInsets`] class specifies offsets in terms of visual edges, left,
/// top, right, and bottom. These values are not affected by the
/// [`TextDirection`]. To support both left-to-right and right-to-left layouts,
/// consider using [`EdgeInsetsDirectional`], which is expressed in terms of
/// _start_, top, _end_, and bottom, where start and end are resolved in terms
/// of a [`TextDirection`] (typically obtained from the ambient `Directionality`).
///
/// See also:
///
///  * `Padding`, a widget that accepts [`EdgeInsets`] to describe its margins.
///  * [`EdgeInsetsDirectional`], which (for properties and arguments that
///    accept the type [`EdgeInsetsGeometry`]) allows the horizontal insets to
///    be specified in a [`TextDirection`]-aware manner.
#[derive(Clone, Copy, PartialEq, Default)]
pub struct EdgeInsets {
    /// The offset from the left.
    pub left: f64,
    /// The offset from the top.
    pub top: f64,
    /// The offset from the right.
    pub right: f64,
    /// The offset from the bottom.
    pub bottom: f64,
}

impl EdgeInsets {
    /// Creates insets from offsets from the left, top, right, and bottom.
    pub const fn from_ltrb(left: f64, top: f64, right: f64, bottom: f64) -> EdgeInsets {
        EdgeInsets {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Creates insets where all the offsets are `value`.
    pub const fn all(value: f64) -> EdgeInsets {
        EdgeInsets {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }

    /// Creates insets with only the given values non-zero.
    pub const fn only(left: f64, top: f64, right: f64, bottom: f64) -> EdgeInsets {
        EdgeInsets {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Creates insets with symmetrical vertical and horizontal offsets.
    pub const fn symmetric(vertical: f64, horizontal: f64) -> EdgeInsets {
        EdgeInsets {
            left: horizontal,
            top: vertical,
            right: horizontal,
            bottom: vertical,
        }
    }

    /// An [`EdgeInsets`] with zero offsets in each direction.
    pub const ZERO: EdgeInsets = EdgeInsets::only(0.0, 0.0, 0.0, 0.0);

    /// The total offset in the horizontal direction.
    pub fn horizontal(&self) -> f64 {
        self.left + self.right
    }

    /// The total offset in the vertical direction.
    pub fn vertical(&self) -> f64 {
        self.top + self.bottom
    }

    /// Whether every dimension is non-negative.
    pub fn is_non_negative(&self) -> bool {
        self.left >= 0.0 && self.top >= 0.0 && self.right >= 0.0 && self.bottom >= 0.0
    }

    /// An Offset describing the vector from the top left of a rectangle to the
    /// top left of that rectangle inset by this object.
    pub fn top_left(&self) -> Offset {
        Offset::new(self.left, self.top)
    }

    /// An Offset describing the vector from the top right of a rectangle to the
    /// top right of that rectangle inset by this object.
    pub fn top_right(&self) -> Offset {
        Offset::new(-self.right, self.top)
    }

    /// An Offset describing the vector from the bottom left of a rectangle to
    /// the bottom left of that rectangle inset by this object.
    pub fn bottom_left(&self) -> Offset {
        Offset::new(self.left, -self.bottom)
    }

    /// An Offset describing the vector from the bottom right of a rectangle to
    /// the bottom right of that rectangle inset by this object.
    pub fn bottom_right(&self) -> Offset {
        Offset::new(-self.right, -self.bottom)
    }

    /// An [`EdgeInsets`] with top and bottom as well as left and right flipped.
    pub fn flipped(&self) -> EdgeInsets {
        EdgeInsets::from_ltrb(self.right, self.bottom, self.left, self.top)
    }

    /// Returns a new rect that is bigger than the given rect in each direction
    /// by the amount of inset in each direction. Specifically, the left edge of
    /// the rect is moved left by [`left`](EdgeInsets::left), the top edge of
    /// the rect is moved up by [`top`](EdgeInsets::top), the right edge of the
    /// rect is moved right by [`right`](EdgeInsets::right), and the bottom edge
    /// of the rect is moved down by [`bottom`](EdgeInsets::bottom).
    ///
    /// See also:
    ///
    ///  * [`EdgeInsetsGeometry::inflate_size`], to inflate a [`Size`] rather
    ///    than a [`Rect`].
    ///  * [`deflate_rect`](Self::deflate_rect), to deflate a [`Rect`] rather
    ///    than inflating it.
    pub fn inflate_rect(&self, rect: Rect) -> Rect {
        Rect::from_ltrb(
            rect.left - self.left,
            rect.top - self.top,
            rect.right + self.right,
            rect.bottom + self.bottom,
        )
    }

    /// Returns a new [`RRect`] expanded by this [`EdgeInsets`], increasing each
    /// corner's radius by the corresponding per-axis inset amounts (clamped at
    /// zero).
    pub fn inflate_rrect(&self, rect: RRect) -> RRect {
        RRect::from_ltrb_and_corners(
            rect.left - self.left,
            rect.top - self.top,
            rect.right + self.right,
            rect.bottom + self.bottom,
            (rect.tl_radius() + Radius::elliptical(self.left, self.top))
                .clamp(Some(Radius::ZERO), None),
            (rect.tr_radius() + Radius::elliptical(self.right, self.top))
                .clamp(Some(Radius::ZERO), None),
            (rect.br_radius() + Radius::elliptical(self.right, self.bottom))
                .clamp(Some(Radius::ZERO), None),
            (rect.bl_radius() + Radius::elliptical(self.left, self.bottom))
                .clamp(Some(Radius::ZERO), None),
        )
    }

    /// Returns a new [`RRect`] shrunk by this [`EdgeInsets`], decreasing each
    /// corner's radius by the corresponding per-axis inset amounts (clamped at
    /// zero).
    pub fn deflate_rrect(&self, rect: RRect) -> RRect {
        RRect::from_ltrb_and_corners(
            rect.left + self.left,
            rect.top + self.top,
            rect.right - self.right,
            rect.bottom - self.bottom,
            (rect.tl_radius() - Radius::elliptical(self.left, self.top))
                .clamp(Some(Radius::ZERO), None),
            (rect.tr_radius() - Radius::elliptical(self.right, self.top))
                .clamp(Some(Radius::ZERO), None),
            (rect.br_radius() - Radius::elliptical(self.right, self.bottom))
                .clamp(Some(Radius::ZERO), None),
            (rect.bl_radius() - Radius::elliptical(self.left, self.bottom))
                .clamp(Some(Radius::ZERO), None),
        )
    }

    /// Returns a new rect that is smaller than the given rect in each direction
    /// by the amount of inset in each direction. Specifically, the left edge of
    /// the rect is moved right by [`left`](EdgeInsets::left), the top edge of
    /// the rect is moved down by [`top`](EdgeInsets::top), the right edge of
    /// the rect is moved left by [`right`](EdgeInsets::right), and the bottom
    /// edge of the rect is moved up by [`bottom`](EdgeInsets::bottom).
    ///
    /// If the argument's [`Rect`] size is smaller than
    /// [`EdgeInsetsGeometry::collapsed_size`], then the resulting rectangle
    /// will have negative dimensions.
    ///
    /// See also:
    ///
    ///  * [`EdgeInsetsGeometry::deflate_size`], to deflate a [`Size`] rather
    ///    than a [`Rect`].
    ///  * [`inflate_rect`](Self::inflate_rect), to inflate a [`Rect`] rather
    ///    than deflating it.
    pub fn deflate_rect(&self, rect: Rect) -> Rect {
        Rect::from_ltrb(
            rect.left + self.left,
            rect.top + self.top,
            rect.right - self.right,
            rect.bottom - self.bottom,
        )
    }

    /// Returns a new [`EdgeInsets`] object with all values greater than or
    /// equal to `min`, and less than or equal to `max`.
    pub fn clamp(&self, min: &EdgeInsetsGeometry, max: &EdgeInsetsGeometry) -> EdgeInsets {
        EdgeInsets::from_ltrb(
            clamp_double(self.left, min.left(), max.left()),
            clamp_double(self.top, min.top(), max.top()),
            clamp_double(self.right, min.right(), max.right()),
            clamp_double(self.bottom, min.bottom(), max.bottom()),
        )
    }

    /// Integer divides the [`EdgeInsets`] in each dimension by the given
    /// factor.
    pub fn truncating_div(&self, other: f64) -> EdgeInsets {
        EdgeInsets::from_ltrb(
            (self.left / other).trunc(),
            (self.top / other).trunc(),
            (self.right / other).trunc(),
            (self.bottom / other).trunc(),
        )
    }

    /// Linearly interpolate between two [`EdgeInsets`].
    ///
    /// If either is null, this function interpolates from [`EdgeInsets::ZERO`].
    pub fn lerp(a: Option<EdgeInsets>, b: Option<EdgeInsets>, t: f64) -> Option<EdgeInsets> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b * t),
            (Some(a), None) => Some(a * (1.0 - t)),
            (Some(a), Some(b)) => Some(EdgeInsets::from_ltrb(
                lerp_double(Some(a.left), Some(b.left), t).unwrap(),
                lerp_double(Some(a.top), Some(b.top), t).unwrap(),
                lerp_double(Some(a.right), Some(b.right), t).unwrap(),
                lerp_double(Some(a.bottom), Some(b.bottom), t).unwrap(),
            )),
        }
    }

    /// Convert this instance into an [`EdgeInsets`], which uses literal
    /// coordinates.
    pub fn resolve(&self, _direction: Option<TextDirection>) -> EdgeInsets {
        *self
    }

    /// Creates a copy of this EdgeInsets but with the given fields replaced
    /// with the new values.
    pub fn copy_with(
        &self,
        left: Option<f64>,
        top: Option<f64>,
        right: Option<f64>,
        bottom: Option<f64>,
    ) -> EdgeInsets {
        EdgeInsets::only(
            left.unwrap_or(self.left),
            top.unwrap_or(self.top),
            right.unwrap_or(self.right),
            bottom.unwrap_or(self.bottom),
        )
    }
}

/// Returns the difference between two [`EdgeInsets`].
impl Sub for EdgeInsets {
    type Output = EdgeInsets;

    fn sub(self, other: EdgeInsets) -> EdgeInsets {
        EdgeInsets::from_ltrb(
            self.left - other.left,
            self.top - other.top,
            self.right - other.right,
            self.bottom - other.bottom,
        )
    }
}

/// Returns the sum of two [`EdgeInsets`].
impl Add for EdgeInsets {
    type Output = EdgeInsets;

    fn add(self, other: EdgeInsets) -> EdgeInsets {
        EdgeInsets::from_ltrb(
            self.left + other.left,
            self.top + other.top,
            self.right + other.right,
            self.bottom + other.bottom,
        )
    }
}

/// Returns the [`EdgeInsets`] object with each dimension negated.
///
/// This is the same as multiplying the object by -1.0.
impl Neg for EdgeInsets {
    type Output = EdgeInsets;

    fn neg(self) -> EdgeInsets {
        EdgeInsets::from_ltrb(-self.left, -self.top, -self.right, -self.bottom)
    }
}

/// Scales the [`EdgeInsets`] in each dimension by the given factor.
impl Mul<f64> for EdgeInsets {
    type Output = EdgeInsets;

    fn mul(self, other: f64) -> EdgeInsets {
        EdgeInsets::from_ltrb(
            self.left * other,
            self.top * other,
            self.right * other,
            self.bottom * other,
        )
    }
}

/// Divides the [`EdgeInsets`] in each dimension by the given factor.
impl Div<f64> for EdgeInsets {
    type Output = EdgeInsets;

    fn div(self, other: f64) -> EdgeInsets {
        EdgeInsets::from_ltrb(
            self.left / other,
            self.top / other,
            self.right / other,
            self.bottom / other,
        )
    }
}

/// Computes the remainder in each dimension by the given factor.
impl Rem<f64> for EdgeInsets {
    type Output = EdgeInsets;

    fn rem(self, other: f64) -> EdgeInsets {
        EdgeInsets::from_ltrb(
            self.left.rem_euclid(other),
            self.top.rem_euclid(other),
            self.right.rem_euclid(other),
            self.bottom.rem_euclid(other),
        )
    }
}

impl Debug for EdgeInsets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&EdgeInsetsGeometry::from(*self), f)
    }
}

/// An immutable set of offsets in each of the four cardinal directions, but
/// whose horizontal components are dependent on the writing direction.
///
/// This can be used to indicate padding from the left in [`TextDirection::Ltr`]
/// text and padding from the right in [`TextDirection::Rtl`] text without
/// having to be aware of the current text direction.
///
/// See also:
///
///  * [`EdgeInsets`], a variant that uses physical labels (left and right
///    instead of start and end).
#[derive(Clone, Copy, PartialEq, Default)]
pub struct EdgeInsetsDirectional {
    /// The offset from the start side, the side from which the user will start
    /// reading text.
    ///
    /// This value is normalized into an [`EdgeInsets::left`] or
    /// [`EdgeInsets::right`] value by the [`resolve`](Self::resolve) method.
    pub start: f64,
    /// The offset from the top.
    ///
    /// This value is passed through to [`EdgeInsets::top`] unmodified by the
    /// [`resolve`](Self::resolve) method.
    pub top: f64,
    /// The offset from the end side, the side on which the user ends reading
    /// text.
    ///
    /// This value is normalized into an [`EdgeInsets::left`] or
    /// [`EdgeInsets::right`] value by the [`resolve`](Self::resolve) method.
    pub end: f64,
    /// The offset from the bottom.
    ///
    /// This value is passed through to [`EdgeInsets::bottom`] unmodified by the
    /// [`resolve`](Self::resolve) method.
    pub bottom: f64,
}

impl EdgeInsetsDirectional {
    /// Creates insets from offsets from the start, top, end, and bottom.
    pub const fn from_steb(start: f64, top: f64, end: f64, bottom: f64) -> EdgeInsetsDirectional {
        EdgeInsetsDirectional {
            start,
            top,
            end,
            bottom,
        }
    }

    /// Creates insets with only the given values non-zero.
    pub const fn only(start: f64, top: f64, end: f64, bottom: f64) -> EdgeInsetsDirectional {
        EdgeInsetsDirectional {
            start,
            top,
            end,
            bottom,
        }
    }

    /// Creates insets with symmetric vertical and horizontal offsets.
    ///
    /// This is equivalent to [`EdgeInsets::symmetric`], since the inset is the
    /// same with either [`TextDirection`]. This constructor is just a
    /// convenience for type compatibility.
    pub const fn symmetric(horizontal: f64, vertical: f64) -> EdgeInsetsDirectional {
        EdgeInsetsDirectional {
            start: horizontal,
            end: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }

    /// Creates insets where all the offsets are `value`.
    pub const fn all(value: f64) -> EdgeInsetsDirectional {
        EdgeInsetsDirectional {
            start: value,
            top: value,
            end: value,
            bottom: value,
        }
    }

    /// An [`EdgeInsetsDirectional`] with zero offsets in each direction.
    ///
    /// Consider using [`EdgeInsets::ZERO`] instead, since that object has the
    /// same effect, but will be cheaper to [`resolve`](Self::resolve).
    pub const ZERO: EdgeInsetsDirectional = EdgeInsetsDirectional::only(0.0, 0.0, 0.0, 0.0);

    /// Whether every dimension is non-negative.
    pub fn is_non_negative(&self) -> bool {
        self.start >= 0.0 && self.top >= 0.0 && self.end >= 0.0 && self.bottom >= 0.0
    }

    /// An [`EdgeInsetsDirectional`] with [`top`](Self::top) and
    /// [`bottom`](Self::bottom) as well as [`start`](Self::start) and
    /// [`end`](Self::end) flipped.
    pub fn flipped(&self) -> EdgeInsetsDirectional {
        EdgeInsetsDirectional::from_steb(self.end, self.bottom, self.start, self.top)
    }

    /// Integer divides the [`EdgeInsetsDirectional`] object in each dimension
    /// by the given factor.
    pub fn truncating_div(&self, other: f64) -> EdgeInsetsDirectional {
        EdgeInsetsDirectional::from_steb(
            (self.start / other).trunc(),
            (self.top / other).trunc(),
            (self.end / other).trunc(),
            (self.bottom / other).trunc(),
        )
    }

    /// Linearly interpolate between two [`EdgeInsetsDirectional`].
    ///
    /// If either is null, this function interpolates from
    /// [`EdgeInsetsDirectional::ZERO`].
    ///
    /// To interpolate between two [`EdgeInsetsGeometry`] objects of arbitrary
    /// type (either [`EdgeInsets`] or [`EdgeInsetsDirectional`]), consider the
    /// `EdgeInsetsGeometry.lerp` static method.
    pub fn lerp(
        a: Option<EdgeInsetsDirectional>,
        b: Option<EdgeInsetsDirectional>,
        t: f64,
    ) -> Option<EdgeInsetsDirectional> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b * t),
            (Some(a), None) => Some(a * (1.0 - t)),
            (Some(a), Some(b)) => Some(EdgeInsetsDirectional::from_steb(
                lerp_double(Some(a.start), Some(b.start), t).unwrap(),
                lerp_double(Some(a.top), Some(b.top), t).unwrap(),
                lerp_double(Some(a.end), Some(b.end), t).unwrap(),
                lerp_double(Some(a.bottom), Some(b.bottom), t).unwrap(),
            )),
        }
    }

    /// Convert this instance into an [`EdgeInsets`], which uses literal
    /// coordinates.
    pub fn resolve(&self, direction: Option<TextDirection>) -> EdgeInsets {
        match direction.expect("No TextDirection found.") {
            TextDirection::Rtl => {
                EdgeInsets::from_ltrb(self.end, self.top, self.start, self.bottom)
            }
            TextDirection::Ltr => {
                EdgeInsets::from_ltrb(self.start, self.top, self.end, self.bottom)
            }
        }
    }

    /// Creates a copy of this EdgeInsetsDirectional but with the given fields
    /// replaced with the new values.
    pub fn copy_with(
        &self,
        start: Option<f64>,
        top: Option<f64>,
        end: Option<f64>,
        bottom: Option<f64>,
    ) -> EdgeInsetsDirectional {
        EdgeInsetsDirectional::only(
            start.unwrap_or(self.start),
            top.unwrap_or(self.top),
            end.unwrap_or(self.end),
            bottom.unwrap_or(self.bottom),
        )
    }
}

/// Returns the difference between two [`EdgeInsetsDirectional`] objects.
impl Sub for EdgeInsetsDirectional {
    type Output = EdgeInsetsDirectional;

    fn sub(self, other: EdgeInsetsDirectional) -> EdgeInsetsDirectional {
        EdgeInsetsDirectional::from_steb(
            self.start - other.start,
            self.top - other.top,
            self.end - other.end,
            self.bottom - other.bottom,
        )
    }
}

/// Returns the sum of two [`EdgeInsetsDirectional`] objects.
impl Add for EdgeInsetsDirectional {
    type Output = EdgeInsetsDirectional;

    fn add(self, other: EdgeInsetsDirectional) -> EdgeInsetsDirectional {
        EdgeInsetsDirectional::from_steb(
            self.start + other.start,
            self.top + other.top,
            self.end + other.end,
            self.bottom + other.bottom,
        )
    }
}

/// Returns the [`EdgeInsetsDirectional`] object with each dimension negated.
///
/// This is the same as multiplying the object by -1.0.
impl Neg for EdgeInsetsDirectional {
    type Output = EdgeInsetsDirectional;

    fn neg(self) -> EdgeInsetsDirectional {
        EdgeInsetsDirectional::from_steb(-self.start, -self.top, -self.end, -self.bottom)
    }
}

/// Scales the [`EdgeInsetsDirectional`] object in each dimension by the given
/// factor.
impl Mul<f64> for EdgeInsetsDirectional {
    type Output = EdgeInsetsDirectional;

    fn mul(self, other: f64) -> EdgeInsetsDirectional {
        EdgeInsetsDirectional::from_steb(
            self.start * other,
            self.top * other,
            self.end * other,
            self.bottom * other,
        )
    }
}

/// Divides the [`EdgeInsetsDirectional`] object in each dimension by the given
/// factor.
impl Div<f64> for EdgeInsetsDirectional {
    type Output = EdgeInsetsDirectional;

    fn div(self, other: f64) -> EdgeInsetsDirectional {
        EdgeInsetsDirectional::from_steb(
            self.start / other,
            self.top / other,
            self.end / other,
            self.bottom / other,
        )
    }
}

/// Computes the remainder in each dimension by the given factor.
impl Rem<f64> for EdgeInsetsDirectional {
    type Output = EdgeInsetsDirectional;

    fn rem(self, other: f64) -> EdgeInsetsDirectional {
        EdgeInsetsDirectional::from_steb(
            self.start.rem_euclid(other),
            self.top.rem_euclid(other),
            self.end.rem_euclid(other),
            self.bottom.rem_euclid(other),
        )
    }
}

impl Debug for EdgeInsetsDirectional {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&EdgeInsetsGeometry::from(*self), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_insets_constructors() {
        let ltrb = EdgeInsets::from_ltrb(10.0, 20.0, 30.0, 40.0);
        assert_eq!(ltrb.resolve(Some(TextDirection::Ltr)), ltrb);
        assert_eq!(ltrb.resolve(Some(TextDirection::Rtl)), ltrb);

        let all = EdgeInsets::all(10.0);
        assert_eq!(
            all.resolve(Some(TextDirection::Ltr)),
            EdgeInsets::from_ltrb(10.0, 10.0, 10.0, 10.0)
        );
        assert_eq!(
            all.resolve(Some(TextDirection::Rtl)),
            EdgeInsets::from_ltrb(10.0, 10.0, 10.0, 10.0)
        );

        let only = EdgeInsets::only(10.0, 20.0, 30.0, 40.0);
        assert_eq!(
            only.resolve(Some(TextDirection::Ltr)),
            EdgeInsets::from_ltrb(10.0, 20.0, 30.0, 40.0)
        );

        let symmetric = EdgeInsets::symmetric(20.0, 10.0);
        assert_eq!(
            symmetric.resolve(Some(TextDirection::Ltr)),
            EdgeInsets::from_ltrb(10.0, 20.0, 10.0, 20.0)
        );
        assert_eq!(
            symmetric.resolve(Some(TextDirection::Rtl)),
            EdgeInsets::from_ltrb(10.0, 20.0, 10.0, 20.0)
        );
    }

    #[test]
    fn edge_insets_directional_constructors() {
        let steb = EdgeInsetsDirectional::from_steb(10.0, 20.0, 30.0, 40.0);
        assert_eq!(
            steb.resolve(Some(TextDirection::Ltr)),
            EdgeInsets::from_ltrb(10.0, 20.0, 30.0, 40.0)
        );
        assert_eq!(
            steb.resolve(Some(TextDirection::Rtl)),
            EdgeInsets::from_ltrb(30.0, 20.0, 10.0, 40.0)
        );

        let all = EdgeInsetsDirectional::all(10.0);
        assert_eq!(
            all.resolve(Some(TextDirection::Ltr)),
            EdgeInsets::from_ltrb(10.0, 10.0, 10.0, 10.0)
        );
        assert_eq!(
            all.resolve(Some(TextDirection::Rtl)),
            EdgeInsets::from_ltrb(10.0, 10.0, 10.0, 10.0)
        );

        let directional = EdgeInsetsDirectional::only(10.0, 20.0, 30.0, 40.0);
        assert_eq!(
            directional.resolve(Some(TextDirection::Ltr)),
            EdgeInsets::from_ltrb(10.0, 20.0, 30.0, 40.0)
        );
        assert_eq!(
            directional.resolve(Some(TextDirection::Rtl)),
            EdgeInsets::from_ltrb(30.0, 20.0, 10.0, 40.0)
        );

        let symmetric = EdgeInsetsDirectional::symmetric(10.0, 20.0);
        assert_eq!(
            symmetric.resolve(Some(TextDirection::Ltr)),
            EdgeInsets::from_ltrb(10.0, 20.0, 10.0, 20.0)
        );
        assert_eq!(
            symmetric.resolve(Some(TextDirection::Rtl)),
            EdgeInsets::from_ltrb(10.0, 20.0, 10.0, 20.0)
        );
    }

    #[test]
    fn zero_horizontal_insets_compare_equal_across_kinds() {
        let visual = EdgeInsetsGeometry::from(EdgeInsets::only(0.0, 5.0, 0.0, 7.0));
        let directional = EdgeInsetsGeometry::from(EdgeInsetsDirectional::only(0.0, 5.0, 0.0, 7.0));
        assert_eq!(visual, directional);
        assert_ne!(
            visual,
            EdgeInsetsGeometry::from(EdgeInsetsDirectional::only(5.0, 0.0, 0.0, 0.0))
        );
        assert_ne!(
            EdgeInsetsGeometry::from(EdgeInsets::only(5.0, 0.0, 0.0, 0.0)),
            EdgeInsetsGeometry::from(EdgeInsetsDirectional::only(5.0, 0.0, 0.0, 0.0))
        );
        assert_ne!(
            EdgeInsetsGeometry::from(EdgeInsets::only(0.0, 0.0, 5.0, 0.0)),
            EdgeInsetsGeometry::from(EdgeInsetsDirectional::only(0.0, 0.0, 5.0, 0.0))
        );
    }

    #[test]
    fn resolve_covers_only_variants() {
        assert_eq!(
            EdgeInsetsDirectional::only(963.25, 0.0, 0.0, 0.0).resolve(Some(TextDirection::Ltr)),
            EdgeInsets::from_ltrb(963.25, 0.0, 0.0, 0.0)
        );
        assert_eq!(
            EdgeInsetsDirectional::only(0.0, 963.25, 0.0, 0.0).resolve(Some(TextDirection::Ltr)),
            EdgeInsets::from_ltrb(0.0, 963.25, 0.0, 0.0)
        );
        assert_eq!(
            EdgeInsetsDirectional::only(0.0, 0.0, 963.25, 0.0).resolve(Some(TextDirection::Ltr)),
            EdgeInsets::from_ltrb(0.0, 0.0, 963.25, 0.0)
        );
        assert_eq!(
            EdgeInsetsDirectional::only(0.0, 0.0, 0.0, 963.25).resolve(Some(TextDirection::Ltr)),
            EdgeInsets::from_ltrb(0.0, 0.0, 0.0, 963.25)
        );
        assert_eq!(
            EdgeInsetsDirectional::only(963.25, 0.0, 0.0, 0.0).resolve(Some(TextDirection::Rtl)),
            EdgeInsets::from_ltrb(0.0, 0.0, 963.25, 0.0)
        );
        assert_eq!(
            EdgeInsetsDirectional::only(0.0, 0.0, 963.25, 0.0).resolve(Some(TextDirection::Rtl)),
            EdgeInsets::from_ltrb(963.25, 0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn control_test_insets_geometry() {
        let insets = EdgeInsets::from_ltrb(5.0, 7.0, 11.0, 13.0);
        assert_eq!(insets.top_left(), Offset::new(5.0, 7.0));
        assert_eq!(insets.top_right(), Offset::new(-11.0, 7.0));
        assert_eq!(insets.bottom_left(), Offset::new(5.0, -13.0));
        assert_eq!(insets.bottom_right(), Offset::new(-11.0, -13.0));
        assert_eq!(
            EdgeInsetsGeometry::from(insets).collapsed_size(),
            Size::new(16.0, 20.0)
        );
        assert_eq!(
            insets.flipped(),
            EdgeInsets::from_ltrb(11.0, 13.0, 5.0, 7.0)
        );
        assert_eq!(
            EdgeInsetsGeometry::from(insets).along(Axis::Horizontal),
            16.0
        );
        assert_eq!(EdgeInsetsGeometry::from(insets).along(Axis::Vertical), 20.0);
        assert_eq!(
            insets.inflate_rect(Rect::from_ltrb(23.0, 32.0, 124.0, 143.0)),
            Rect::from_ltrb(18.0, 25.0, 135.0, 156.0)
        );
        assert_eq!(
            insets.deflate_rect(Rect::from_ltrb(23.0, 32.0, 124.0, 143.0)),
            Rect::from_ltrb(28.0, 39.0, 113.0, 130.0)
        );
        assert_eq!(
            EdgeInsetsGeometry::from(insets).inflate_size(Size::new(100.0, 125.0)),
            Size::new(116.0, 145.0)
        );
        assert_eq!(
            EdgeInsetsGeometry::from(insets).deflate_size(Size::new(100.0, 125.0)),
            Size::new(84.0, 105.0)
        );
        assert_eq!(insets / 2.0, EdgeInsets::from_ltrb(2.5, 3.5, 5.5, 6.5));
        assert_eq!(
            insets.truncating_div(2.0),
            EdgeInsets::from_ltrb(2.0, 3.0, 5.0, 6.0)
        );
        assert_eq!(insets % 5.0, EdgeInsets::from_ltrb(0.0, 2.0, 1.0, 3.0));
    }

    #[test]
    fn lerp_same_kind() {
        let a = EdgeInsets::all(10.0);
        let b = EdgeInsets::all(20.0);
        assert_eq!(EdgeInsets::lerp(Some(a), Some(b), 0.25), Some(a * 1.25));
        assert_eq!(EdgeInsets::lerp(None, None, 0.25), None);
        assert_eq!(EdgeInsets::lerp(None, Some(b), 0.25), Some(b * 0.25));
        assert_eq!(EdgeInsets::lerp(Some(a), None, 0.25), Some(a * 0.75));
        assert_eq!(
            EdgeInsets::lerp(Some(EdgeInsets::ZERO), Some(EdgeInsets::ZERO), 0.5),
            Some(EdgeInsets::ZERO)
        );
    }

    #[test]
    fn copy_with_replaces_given_fields() {
        let source = EdgeInsets::only(1.0, 2.0, 4.0, 3.0);
        let copy = source.copy_with(Some(5.0), Some(6.0), None, None);
        assert_eq!(copy, EdgeInsets::only(5.0, 6.0, 4.0, 3.0));

        let directional = EdgeInsetsDirectional::only(1.0, 2.0, 4.0, 3.0);
        let directional_copy = directional.copy_with(Some(5.0), Some(6.0), None, None);
        assert_eq!(
            directional_copy,
            EdgeInsetsDirectional::only(5.0, 6.0, 4.0, 3.0)
        );
    }

    #[test]
    fn same_kind_operators() {
        let a = EdgeInsets::from_ltrb(1.0, 2.0, 3.0, 5.0);
        assert_eq!(a * 2.0, EdgeInsets::from_ltrb(2.0, 4.0, 6.0, 10.0));
        assert_eq!(a / 2.0, EdgeInsets::from_ltrb(0.5, 1.0, 1.5, 2.5));
        assert_eq!(a % 2.0, EdgeInsets::from_ltrb(1.0, 0.0, 1.0, 1.0));
        assert_eq!(
            a.truncating_div(2.0),
            EdgeInsets::from_ltrb(0.0, 1.0, 1.0, 2.0)
        );
        assert_eq!(a + a, a * 2.0);
        assert_eq!(a - a, EdgeInsets::ZERO);

        let d = EdgeInsetsDirectional::from_steb(1.0, 2.0, 3.0, 5.0);
        assert_eq!(
            d * 2.0,
            EdgeInsetsDirectional::from_steb(2.0, 4.0, 6.0, 10.0)
        );
        assert_eq!(d + d, d * 2.0);
        assert_eq!(d - d, EdgeInsetsDirectional::ZERO);
    }

    #[test]
    fn debug_matches_darts_to_string() {
        assert_eq!(format!("{:?}", EdgeInsets::ZERO), "EdgeInsets.zero");
        assert_eq!(
            format!("{:?}", EdgeInsets::only(1.01, 1.01, 1.01, 1.01)),
            "EdgeInsets.all(1.0)"
        );
        assert_eq!(
            format!("{:?}", EdgeInsetsDirectional::only(1.01, 1.01, 1.01, 1.01)),
            "EdgeInsetsDirectional(1.0, 1.0, 1.0, 1.0)"
        );
    }

    #[test]
    fn a_cross_kind_lerp_resolves_per_direction() {
        let a = EdgeInsetsGeometry::from_ltrb(10.0, 0.0, 0.0, 0.0);
        let b = EdgeInsetsGeometry::from_steb(20.0, 0.0, 0.0, 0.0);
        let halfway = EdgeInsetsGeometry::lerp(Some(a), Some(b), 0.5).expect("both ends are set");
        assert!(matches!(halfway, EdgeInsetsGeometry::Mixed { .. }));
        assert_eq!(
            halfway.resolve(Some(TextDirection::Ltr)),
            EdgeInsets::from_ltrb(15.0, 0.0, 0.0, 0.0)
        );
        assert_eq!(
            halfway.resolve(Some(TextDirection::Rtl)),
            EdgeInsets::from_ltrb(5.0, 0.0, 10.0, 0.0)
        );
    }

    #[test]
    fn a_same_kind_lerp_keeps_the_kind() {
        let lerp = |a, b, t| EdgeInsetsGeometry::lerp(Some(a), Some(b), t).expect("both ends");
        assert_eq!(
            lerp(
                EdgeInsetsGeometry::all(10.0),
                EdgeInsetsGeometry::all(20.0),
                0.5
            ),
            EdgeInsetsGeometry::Insets(EdgeInsets::all(15.0))
        );
        assert_eq!(
            lerp(
                EdgeInsetsGeometry::from_steb(10.0, 0.0, 0.0, 0.0),
                EdgeInsetsGeometry::from_steb(20.0, 0.0, 0.0, 0.0),
                0.5
            ),
            EdgeInsetsGeometry::Directional(EdgeInsetsDirectional::from_steb(15.0, 0.0, 0.0, 0.0))
        );
    }

    #[test]
    fn clamping_against_infinity_keeps_a_non_negative_inset() {
        let padding = EdgeInsetsGeometry::from_steb(-5.0, 4.0, 0.0, 0.0);
        let clamped = padding.clamp(&EdgeInsetsGeometry::ZERO, &EdgeInsetsGeometry::INFINITY);
        assert_eq!(
            clamped.resolve(Some(TextDirection::Ltr)),
            EdgeInsets::from_ltrb(0.0, 4.0, 0.0, 0.0)
        );
    }

    #[test]
    fn a_cross_kind_add_is_the_mixed_arm() {
        let sum = EdgeInsetsGeometry::from_ltrb(1.0, 2.0, 3.0, 4.0)
            .add(EdgeInsetsGeometry::from_steb(10.0, 0.0, 20.0, 0.0));
        assert_eq!(
            sum.resolve(Some(TextDirection::Ltr)),
            EdgeInsets::from_ltrb(11.0, 2.0, 23.0, 4.0)
        );
        assert_eq!(
            sum.resolve(Some(TextDirection::Rtl)),
            EdgeInsets::from_ltrb(21.0, 2.0, 13.0, 4.0)
        );
    }

    #[test]
    fn a_cross_kind_subtract_negates_the_argument() {
        let a = EdgeInsetsGeometry::from_ltrb(1.0, 2.0, 3.0, 4.0);
        let b = EdgeInsetsGeometry::from_steb(10.0, 0.0, 20.0, 0.0);
        assert_eq!(a.subtract(b), a.add(-b));
    }

    #[test]
    fn geometry_factories_wrap_the_concrete_type() {
        assert_eq!(
            EdgeInsetsGeometry::all(10.0),
            EdgeInsetsGeometry::from(EdgeInsets::all(10.0))
        );
        assert_eq!(
            EdgeInsetsGeometry::from_steb(10.0, 20.0, 20.0, 40.0),
            EdgeInsetsGeometry::from(EdgeInsetsDirectional::from_steb(10.0, 20.0, 20.0, 40.0))
        );
        assert_eq!(
            EdgeInsetsGeometry::ZERO,
            EdgeInsetsGeometry::from(EdgeInsets::ZERO)
        );
    }
}
