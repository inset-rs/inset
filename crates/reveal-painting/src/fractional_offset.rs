//! Flutter counterpart: `painting/fractional_offset.dart`.

use std::fmt::{self, Debug};
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

use reveal_embedder::{Offset, Rect, Size, lerp_double};

use crate::alignment::Alignment;

/// An offset that's expressed as a fraction of a [`Size`].
///
/// `FractionalOffset(1.0, 0.0)` represents the top right of the [`Size`].
///
/// `FractionalOffset(0.0, 1.0)` represents the bottom left of the [`Size`].
///
/// `FractionalOffset(0.5, 2.0)` represents a point half way across the [`Size`],
/// below the bottom of the rectangle by the height of the [`Size`].
///
/// The [`FractionalOffset`] class specifies offsets in terms of a distance from
/// the top left, regardless of the [`crate::TextDirection`].
///
/// ## Design discussion
///
/// [`FractionalOffset`] and [`Alignment`] are two different representations of
/// the same information: the location within a rectangle relative to the size
/// of the rectangle. The difference between the two classes is in the
/// coordinate system they use to represent the location.
///
/// [`FractionalOffset`] uses a coordinate system with an origin in the top-left
/// corner of the rectangle whereas [`Alignment`] uses a coordinate system with
/// an origin in the center of the rectangle.
///
/// See also:
///
///  * [`Alignment`], which uses a coordinate system based on the center of the
///    rectangle instead of the top left corner of the rectangle.
#[derive(Clone, Copy, PartialEq)]
pub struct FractionalOffset {
    /// The distance fraction in the horizontal direction in [`Alignment`]
    /// coordinates (`dx * 2 - 1`).
    pub x: f64,
    /// The distance fraction in the vertical direction in [`Alignment`]
    /// coordinates (`dy * 2 - 1`).
    pub y: f64,
}

impl FractionalOffset {
    /// Creates a fractional offset.
    pub const fn new(dx: f64, dy: f64) -> FractionalOffset {
        FractionalOffset {
            x: dx * 2.0 - 1.0,
            y: dy * 2.0 - 1.0,
        }
    }

    /// Creates a fractional offset from a specific offset and size.
    ///
    /// The returned [`FractionalOffset`] describes the position of the
    /// [`Offset`] in the [`Size`], as a fraction of the [`Size`].
    pub fn from_offset_and_size(offset: Offset, size: Size) -> FractionalOffset {
        FractionalOffset::new(offset.dx() / size.width(), offset.dy() / size.height())
    }

    /// Creates a fractional offset from a specific offset and rectangle.
    ///
    /// The offset is assumed to be relative to the same origin as the rectangle.
    ///
    /// If the offset is relative to the top left of the rectangle, use
    /// [`from_offset_and_size`](Self::from_offset_and_size) instead, passing
    /// `rect.size()`.
    ///
    /// The returned [`FractionalOffset`] describes the position of the
    /// [`Offset`] in the [`Rect`], as a fraction of the [`Rect`].
    pub fn from_offset_and_rect(offset: Offset, rect: Rect) -> FractionalOffset {
        FractionalOffset::from_offset_and_size(offset - rect.top_left(), rect.size())
    }

    /// The distance fraction in the horizontal direction.
    ///
    /// A value of 0.0 corresponds to the leftmost edge. A value of 1.0
    /// corresponds to the rightmost edge. Values are not limited to that range;
    /// negative values represent positions to the left of the left edge, and
    /// values greater than 1.0 represent positions to the right of the right
    /// edge.
    pub const fn dx(&self) -> f64 {
        (self.x + 1.0) / 2.0
    }

    /// The distance fraction in the vertical direction.
    ///
    /// A value of 0.0 corresponds to the topmost edge. A value of 1.0
    /// corresponds to the bottommost edge. Values are not limited to that range;
    /// negative values represent positions above the top, and values greater
    /// than 1.0 represent positions below the bottom.
    pub const fn dy(&self) -> f64 {
        (self.y + 1.0) / 2.0
    }

    /// The top left corner.
    pub const TOP_LEFT: FractionalOffset = FractionalOffset::new(0.0, 0.0);
    /// The center point along the top edge.
    pub const TOP_CENTER: FractionalOffset = FractionalOffset::new(0.5, 0.0);
    /// The top right corner.
    pub const TOP_RIGHT: FractionalOffset = FractionalOffset::new(1.0, 0.0);
    /// The center point along the left edge.
    pub const CENTER_LEFT: FractionalOffset = FractionalOffset::new(0.0, 0.5);
    /// The center point, both horizontally and vertically.
    pub const CENTER: FractionalOffset = FractionalOffset::new(0.5, 0.5);
    /// The center point along the right edge.
    pub const CENTER_RIGHT: FractionalOffset = FractionalOffset::new(1.0, 0.5);
    /// The bottom left corner.
    pub const BOTTOM_LEFT: FractionalOffset = FractionalOffset::new(0.0, 1.0);
    /// The center point along the bottom edge.
    pub const BOTTOM_CENTER: FractionalOffset = FractionalOffset::new(0.5, 1.0);
    /// The bottom right corner.
    pub const BOTTOM_RIGHT: FractionalOffset = FractionalOffset::new(1.0, 1.0);

    /// Integer divides the [`FractionalOffset`] in each dimension by the given
    /// factor.
    pub fn truncating_div(&self, other: f64) -> FractionalOffset {
        FractionalOffset::new((self.dx() / other).trunc(), (self.dy() / other).trunc())
    }

    /// Linearly interpolate between two [`FractionalOffset`]s.
    ///
    /// If either is null, this function interpolates from
    /// [`FractionalOffset::CENTER`].
    pub fn lerp(
        a: Option<FractionalOffset>,
        b: Option<FractionalOffset>,
        t: f64,
    ) -> Option<FractionalOffset> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(FractionalOffset::new(
                lerp_double(Some(0.5), Some(b.dx()), t).unwrap(),
                lerp_double(Some(0.5), Some(b.dy()), t).unwrap(),
            )),
            (Some(a), None) => Some(FractionalOffset::new(
                lerp_double(Some(a.dx()), Some(0.5), t).unwrap(),
                lerp_double(Some(a.dy()), Some(0.5), t).unwrap(),
            )),
            (Some(a), Some(b)) => Some(FractionalOffset::new(
                lerp_double(Some(a.dx()), Some(b.dx()), t).unwrap(),
                lerp_double(Some(a.dy()), Some(b.dy()), t).unwrap(),
            )),
        }
    }
}

impl From<FractionalOffset> for Alignment {
    fn from(offset: FractionalOffset) -> Alignment {
        Alignment::new(offset.x, offset.y)
    }
}

impl PartialEq<Alignment> for FractionalOffset {
    fn eq(&self, other: &Alignment) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl PartialEq<FractionalOffset> for Alignment {
    fn eq(&self, other: &FractionalOffset) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl Sub for FractionalOffset {
    type Output = FractionalOffset;

    fn sub(self, other: FractionalOffset) -> FractionalOffset {
        FractionalOffset::new(self.dx() - other.dx(), self.dy() - other.dy())
    }
}

impl Sub<Alignment> for FractionalOffset {
    type Output = Alignment;

    fn sub(self, other: Alignment) -> Alignment {
        Alignment::from(self) - other
    }
}

impl Add for FractionalOffset {
    type Output = FractionalOffset;

    fn add(self, other: FractionalOffset) -> FractionalOffset {
        FractionalOffset::new(self.dx() + other.dx(), self.dy() + other.dy())
    }
}

impl Add<Alignment> for FractionalOffset {
    type Output = Alignment;

    fn add(self, other: Alignment) -> Alignment {
        Alignment::from(self) + other
    }
}

impl Neg for FractionalOffset {
    type Output = FractionalOffset;

    fn neg(self) -> FractionalOffset {
        FractionalOffset::new(-self.dx(), -self.dy())
    }
}

impl Mul<f64> for FractionalOffset {
    type Output = FractionalOffset;

    fn mul(self, other: f64) -> FractionalOffset {
        FractionalOffset::new(self.dx() * other, self.dy() * other)
    }
}

impl Div<f64> for FractionalOffset {
    type Output = FractionalOffset;

    fn div(self, other: f64) -> FractionalOffset {
        FractionalOffset::new(self.dx() / other, self.dy() / other)
    }
}

impl Rem<f64> for FractionalOffset {
    type Output = FractionalOffset;

    fn rem(self, other: f64) -> FractionalOffset {
        FractionalOffset::new(self.dx() % other, self.dy() % other)
    }
}

impl Debug for FractionalOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Dart `toStringAsFixed(1)` rounds half away from zero.
        let dx = (self.dx() * 10.0).round() / 10.0;
        let dy = (self.dy() * 10.0).round() / 10.0;
        write!(f, "FractionalOffset({dx:.1}, {dy:.1})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // fractional_offset_test.dart 'FractionalOffset control test'.
    #[test]
    fn fractional_offset_control_test() {
        let a = FractionalOffset::new(0.5, 0.25);
        let b = FractionalOffset::new(1.25, 0.75);

        assert_eq!(format!("{a:?}"), "FractionalOffset(0.5, 0.3)");

        assert_eq!(-a, FractionalOffset::new(-0.5, -0.25));
        assert_eq!(a - b, FractionalOffset::new(-0.75, -0.5));
        assert_eq!(a + b, FractionalOffset::new(1.75, 1.0));
        assert_eq!(a * 2.0, FractionalOffset::CENTER_RIGHT);
        assert_eq!(a / 2.0, FractionalOffset::new(0.25, 0.125));
        assert_eq!(a.truncating_div(2.0), FractionalOffset::TOP_LEFT);
        assert_eq!(a % 5.0, FractionalOffset::new(0.5, 0.25));
    }

    // fractional_offset_test.dart 'FractionalOffset.lerp()'.
    #[test]
    fn fractional_offset_lerp() {
        let a = FractionalOffset::TOP_LEFT;
        let b = FractionalOffset::TOP_CENTER;
        assert_eq!(
            FractionalOffset::lerp(Some(a), Some(b), 0.25),
            Some(FractionalOffset::new(0.125, 0.0))
        );
        assert_eq!(FractionalOffset::lerp(None, None, 0.25), None);
        assert_eq!(
            FractionalOffset::lerp(None, Some(b), 0.25),
            Some(FractionalOffset::new(0.5, 0.5 - 0.125))
        );
        assert_eq!(
            FractionalOffset::lerp(Some(a), None, 0.25),
            Some(FractionalOffset::new(0.125, 0.125))
        );
    }

    // fractional_offset_test.dart 'FractionalOffset.lerp identical a,b'.
    #[test]
    fn fractional_offset_lerp_identical() {
        assert_eq!(FractionalOffset::lerp(None, None, 0.0), None);
        let decoration = FractionalOffset::new(1.0, 2.0);
        assert_eq!(
            FractionalOffset::lerp(Some(decoration), Some(decoration), 0.5),
            Some(decoration)
        );
    }

    // fractional_offset_test.dart 'FractionalOffset.fromOffsetAndSize()'.
    #[test]
    fn from_offset_and_size() {
        let a = FractionalOffset::from_offset_and_size(
            Offset::new(100.0, 100.0),
            Size::new(200.0, 400.0),
        );
        assert_eq!(a, FractionalOffset::new(0.5, 0.25));
    }

    // fractional_offset_test.dart 'FractionalOffset.fromOffsetAndRect()'.
    #[test]
    fn from_offset_and_rect() {
        let a = FractionalOffset::from_offset_and_rect(
            Offset::new(150.0, 120.0),
            Rect::from_ltwh(50.0, 20.0, 200.0, 400.0),
        );
        assert_eq!(a, FractionalOffset::new(0.5, 0.25));
    }

    #[test]
    fn converts_to_alignment() {
        assert_eq!(Alignment::from(FractionalOffset::CENTER), Alignment::CENTER);
        assert_eq!(FractionalOffset::TOP_LEFT, Alignment::TOP_LEFT);
    }
}
