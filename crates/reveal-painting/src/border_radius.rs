//! Flutter counterpart: `painting/border_radius.dart`.

use std::fmt::{self, Debug};
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

use reveal_geometry::{RRect, RSuperellipse, Radius, Rect};

use crate::basic_types::TextDirection;

/// Base class for [`BorderRadius`] that allows for text-direction aware
/// resolution.
///
/// A property or argument of this type accepts classes created either with
/// [`BorderRadius::only`] and its variants, or
/// [`BorderRadiusDirectional::only`] and its variants.
///
/// To convert a [`BorderRadiusGeometry`] object of indeterminate type into a
/// [`BorderRadius`] object, call the [`resolve`](BorderRadiusGeometry::resolve)
/// method.
///
/// Non-exhaustive: `_MixedBorderRadius` (cross-kind `add`/`subtract`) is a
/// later arm.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq)]
pub enum BorderRadiusGeometry {
    /// Visual corners — Dart's `BorderRadius`.
    BorderRadius(BorderRadius),
    /// Writing-direction corners — Dart's `BorderRadiusDirectional`.
    Directional(BorderRadiusDirectional),
}

impl BorderRadiusGeometry {
    /// Creates a [`BorderRadius`] where all radii are `radius`.
    pub const fn all(radius: Radius) -> BorderRadiusGeometry {
        BorderRadiusGeometry::BorderRadius(BorderRadius::all(radius))
    }

    /// Creates a [`BorderRadius`] where all radii are
    /// [`Radius::circular(radius)`](Radius::circular).
    pub fn circular(radius: f64) -> BorderRadiusGeometry {
        BorderRadiusGeometry::BorderRadius(BorderRadius::circular(radius))
    }

    /// Creates a horizontally symmetrical border radius.
    ///
    /// Utilizing the `left` and `right` properties will return a
    /// [`BorderRadius`], while `start` and `end` will yield a
    /// [`BorderRadiusDirectional`]. These properties cannot be used
    /// interchangeably.
    pub fn horizontal(
        left: Option<Radius>,
        right: Option<Radius>,
        start: Option<Radius>,
        end: Option<Radius>,
    ) -> BorderRadiusGeometry {
        debug_assert!(
            (left.is_none() && right.is_none()) || (start.is_none() && end.is_none()),
            "The left and right values cannot be used in conjunction with start and end."
        );
        if start.is_some() || end.is_some() {
            BorderRadiusGeometry::Directional(BorderRadiusDirectional::horizontal(
                start.unwrap_or(Radius::ZERO),
                end.unwrap_or(Radius::ZERO),
            ))
        } else {
            BorderRadiusGeometry::BorderRadius(BorderRadius::horizontal(
                left.unwrap_or(Radius::ZERO),
                right.unwrap_or(Radius::ZERO),
            ))
        }
    }

    /// Creates a [`BorderRadius`] with only the given non-zero values.
    ///
    /// The other corners will be right angles.
    pub const fn only(
        top_left: Radius,
        top_right: Radius,
        bottom_left: Radius,
        bottom_right: Radius,
    ) -> BorderRadiusGeometry {
        BorderRadiusGeometry::BorderRadius(BorderRadius::only(
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        ))
    }

    /// Creates a [`BorderRadiusDirectional`] with only the given non-zero
    /// values.
    ///
    /// The other corners will be right angles.
    pub const fn directional(
        top_start: Radius,
        top_end: Radius,
        bottom_start: Radius,
        bottom_end: Radius,
    ) -> BorderRadiusGeometry {
        BorderRadiusGeometry::Directional(BorderRadiusDirectional::only(
            top_start,
            top_end,
            bottom_start,
            bottom_end,
        ))
    }

    /// Creates a vertically symmetric [`BorderRadius`] where the top and bottom
    /// sides of the rectangle have the same radii.
    pub const fn vertical(top: Radius, bottom: Radius) -> BorderRadiusGeometry {
        BorderRadiusGeometry::BorderRadius(BorderRadius::vertical(top, bottom))
    }

    /// A [`BorderRadius`] with all zero radii.
    pub const ZERO: BorderRadiusGeometry = BorderRadiusGeometry::BorderRadius(BorderRadius::ZERO);

    fn top_left(&self) -> Radius {
        match self {
            BorderRadiusGeometry::BorderRadius(radius) => radius.top_left,
            BorderRadiusGeometry::Directional(_) => Radius::ZERO,
        }
    }

    fn top_right(&self) -> Radius {
        match self {
            BorderRadiusGeometry::BorderRadius(radius) => radius.top_right,
            BorderRadiusGeometry::Directional(_) => Radius::ZERO,
        }
    }

    fn bottom_left(&self) -> Radius {
        match self {
            BorderRadiusGeometry::BorderRadius(radius) => radius.bottom_left,
            BorderRadiusGeometry::Directional(_) => Radius::ZERO,
        }
    }

    fn bottom_right(&self) -> Radius {
        match self {
            BorderRadiusGeometry::BorderRadius(radius) => radius.bottom_right,
            BorderRadiusGeometry::Directional(_) => Radius::ZERO,
        }
    }

    fn top_start(&self) -> Radius {
        match self {
            BorderRadiusGeometry::BorderRadius(_) => Radius::ZERO,
            BorderRadiusGeometry::Directional(radius) => radius.top_start,
        }
    }

    fn top_end(&self) -> Radius {
        match self {
            BorderRadiusGeometry::BorderRadius(_) => Radius::ZERO,
            BorderRadiusGeometry::Directional(radius) => radius.top_end,
        }
    }

    fn bottom_start(&self) -> Radius {
        match self {
            BorderRadiusGeometry::BorderRadius(_) => Radius::ZERO,
            BorderRadiusGeometry::Directional(radius) => radius.bottom_start,
        }
    }

    fn bottom_end(&self) -> Radius {
        match self {
            BorderRadiusGeometry::BorderRadius(_) => Radius::ZERO,
            BorderRadiusGeometry::Directional(radius) => radius.bottom_end,
        }
    }

    /// Integer divides the [`BorderRadiusGeometry`] object's corners by the
    /// given factor.
    pub fn truncating_div(&self, other: f64) -> BorderRadiusGeometry {
        match self {
            BorderRadiusGeometry::BorderRadius(radius) => {
                BorderRadiusGeometry::BorderRadius(radius.truncating_div(other))
            }
            BorderRadiusGeometry::Directional(radius) => {
                BorderRadiusGeometry::Directional(radius.truncating_div(other))
            }
        }
    }

    /// Convert this instance into a [`BorderRadius`], so that the radii are
    /// expressed for specific physical corners (top-left, top-right, etc)
    /// rather than in a direction-dependent manner.
    ///
    /// See also:
    ///
    ///  * [`BorderRadius`], for which this is a no-op (returns itself).
    ///  * [`BorderRadiusDirectional`], which flips the horizontal direction
    ///    based on the `direction` argument.
    pub fn resolve(&self, direction: Option<TextDirection>) -> BorderRadius {
        match self {
            BorderRadiusGeometry::BorderRadius(radius) => radius.resolve(direction),
            BorderRadiusGeometry::Directional(radius) => radius.resolve(direction),
        }
    }
}

impl From<BorderRadius> for BorderRadiusGeometry {
    fn from(radius: BorderRadius) -> BorderRadiusGeometry {
        BorderRadiusGeometry::BorderRadius(radius)
    }
}

impl From<BorderRadiusDirectional> for BorderRadiusGeometry {
    fn from(radius: BorderRadiusDirectional) -> BorderRadiusGeometry {
        BorderRadiusGeometry::Directional(radius)
    }
}

impl Neg for BorderRadiusGeometry {
    type Output = BorderRadiusGeometry;

    fn neg(self) -> BorderRadiusGeometry {
        match self {
            BorderRadiusGeometry::BorderRadius(radius) => {
                BorderRadiusGeometry::BorderRadius(-radius)
            }
            BorderRadiusGeometry::Directional(radius) => BorderRadiusGeometry::Directional(-radius),
        }
    }
}

impl Mul<f64> for BorderRadiusGeometry {
    type Output = BorderRadiusGeometry;

    fn mul(self, other: f64) -> BorderRadiusGeometry {
        match self {
            BorderRadiusGeometry::BorderRadius(radius) => {
                BorderRadiusGeometry::BorderRadius(radius * other)
            }
            BorderRadiusGeometry::Directional(radius) => {
                BorderRadiusGeometry::Directional(radius * other)
            }
        }
    }
}

impl Div<f64> for BorderRadiusGeometry {
    type Output = BorderRadiusGeometry;

    fn div(self, other: f64) -> BorderRadiusGeometry {
        match self {
            BorderRadiusGeometry::BorderRadius(radius) => {
                BorderRadiusGeometry::BorderRadius(radius / other)
            }
            BorderRadiusGeometry::Directional(radius) => {
                BorderRadiusGeometry::Directional(radius / other)
            }
        }
    }
}

impl Rem<f64> for BorderRadiusGeometry {
    type Output = BorderRadiusGeometry;

    fn rem(self, other: f64) -> BorderRadiusGeometry {
        match self {
            BorderRadiusGeometry::BorderRadius(radius) => {
                BorderRadiusGeometry::BorderRadius(radius % other)
            }
            BorderRadiusGeometry::Directional(radius) => {
                BorderRadiusGeometry::Directional(radius % other)
            }
        }
    }
}

impl Debug for BorderRadiusGeometry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let visual = visual_debug(self);
        let logical = logical_debug(self);
        match (visual, logical) {
            (Some(visual), Some(logical)) => write!(f, "{visual} + {logical}"),
            (Some(visual), None) => write!(f, "{visual}"),
            (None, Some(logical)) => write!(f, "{logical}"),
            (None, None) => write!(f, "BorderRadius.zero"),
        }
    }
}

fn visual_debug(geometry: &BorderRadiusGeometry) -> Option<String> {
    let top_left = geometry.top_left();
    let top_right = geometry.top_right();
    let bottom_left = geometry.bottom_left();
    let bottom_right = geometry.bottom_right();
    if top_left == top_right && top_right == bottom_left && bottom_left == bottom_right {
        if top_left != Radius::ZERO {
            if top_left.x == top_left.y {
                Some(format!("BorderRadius.circular({:.1})", top_left.x))
            } else {
                Some(format!("BorderRadius.all({top_left:?})"))
            }
        } else {
            None
        }
    } else {
        let mut result = String::from("BorderRadius.only(");
        let mut comma = false;
        if top_left != Radius::ZERO {
            result.push_str(&format!("topLeft: {top_left:?}"));
            comma = true;
        }
        if top_right != Radius::ZERO {
            if comma {
                result.push_str(", ");
            }
            result.push_str(&format!("topRight: {top_right:?}"));
            comma = true;
        }
        if bottom_left != Radius::ZERO {
            if comma {
                result.push_str(", ");
            }
            result.push_str(&format!("bottomLeft: {bottom_left:?}"));
            comma = true;
        }
        if bottom_right != Radius::ZERO {
            if comma {
                result.push_str(", ");
            }
            result.push_str(&format!("bottomRight: {bottom_right:?}"));
        }
        result.push(')');
        Some(result)
    }
}

fn logical_debug(geometry: &BorderRadiusGeometry) -> Option<String> {
    let top_start = geometry.top_start();
    let top_end = geometry.top_end();
    let bottom_start = geometry.bottom_start();
    let bottom_end = geometry.bottom_end();
    if top_start == top_end && top_end == bottom_end && bottom_end == bottom_start {
        if top_start != Radius::ZERO {
            if top_start.x == top_start.y {
                Some(format!(
                    "BorderRadiusDirectional.circular({:.1})",
                    top_start.x
                ))
            } else {
                Some(format!("BorderRadiusDirectional.all({top_start:?})"))
            }
        } else {
            None
        }
    } else {
        let mut result = String::from("BorderRadiusDirectional.only(");
        let mut comma = false;
        if top_start != Radius::ZERO {
            result.push_str(&format!("topStart: {top_start:?}"));
            comma = true;
        }
        if top_end != Radius::ZERO {
            if comma {
                result.push_str(", ");
            }
            result.push_str(&format!("topEnd: {top_end:?}"));
            comma = true;
        }
        if bottom_start != Radius::ZERO {
            if comma {
                result.push_str(", ");
            }
            result.push_str(&format!("bottomStart: {bottom_start:?}"));
            comma = true;
        }
        if bottom_end != Radius::ZERO {
            if comma {
                result.push_str(", ");
            }
            result.push_str(&format!("bottomEnd: {bottom_end:?}"));
        }
        result.push(')');
        Some(result)
    }
}

/// An immutable set of radii for each corner of a rectangle.
///
/// Used by `BoxDecoration` when the shape is a `BoxShape.rectangle`.
///
/// The [`BorderRadius`] class specifies offsets in terms of visual corners,
/// e.g. [`top_left`](BorderRadius::top_left). These values are not affected by
/// the [`TextDirection`]. To support both left-to-right and right-to-left
/// layouts, consider using [`BorderRadiusDirectional`], which is expressed in
/// terms that are relative to a [`TextDirection`] (typically obtained from the
/// ambient `Directionality`).
#[derive(Clone, Copy, PartialEq)]
pub struct BorderRadius {
    /// The top-left [`Radius`].
    pub top_left: Radius,
    /// The top-right [`Radius`].
    pub top_right: Radius,
    /// The bottom-left [`Radius`].
    pub bottom_left: Radius,
    /// The bottom-right [`Radius`].
    pub bottom_right: Radius,
}

impl BorderRadius {
    /// Creates a border radius where all radii are `radius`.
    pub const fn all(radius: Radius) -> BorderRadius {
        BorderRadius::only(radius, radius, radius, radius)
    }

    /// Creates a border radius where all radii are
    /// [`Radius::circular(radius)`](Radius::circular).
    pub fn circular(radius: f64) -> BorderRadius {
        BorderRadius::all(Radius::circular(radius))
    }

    /// Creates a vertically symmetric border radius where the top and bottom
    /// sides of the rectangle have the same radii.
    pub const fn vertical(top: Radius, bottom: Radius) -> BorderRadius {
        BorderRadius::only(top, top, bottom, bottom)
    }

    /// Creates a horizontally symmetrical border radius where the left and
    /// right sides of the rectangle have the same radii.
    pub const fn horizontal(left: Radius, right: Radius) -> BorderRadius {
        BorderRadius::only(left, right, left, right)
    }

    /// Creates a border radius with only the given non-zero values. The other
    /// corners will be right angles.
    pub const fn only(
        top_left: Radius,
        top_right: Radius,
        bottom_left: Radius,
        bottom_right: Radius,
    ) -> BorderRadius {
        BorderRadius {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        }
    }

    /// Returns a copy of this BorderRadius with the given fields replaced with
    /// the new values.
    pub fn copy_with(
        &self,
        top_left: Option<Radius>,
        top_right: Option<Radius>,
        bottom_left: Option<Radius>,
        bottom_right: Option<Radius>,
    ) -> BorderRadius {
        BorderRadius::only(
            top_left.unwrap_or(self.top_left),
            top_right.unwrap_or(self.top_right),
            bottom_left.unwrap_or(self.bottom_left),
            bottom_right.unwrap_or(self.bottom_right),
        )
    }

    /// Creates an [`RRect`] from the current border radius and a [`Rect`].
    ///
    /// If any of the radii have negative values in x or y, those values will be
    /// clamped to zero in order to produce a valid [`RRect`].
    pub fn to_rrect(&self, rect: Rect) -> RRect {
        RRect::from_rect_and_corners(
            rect,
            self.top_left.clamp(Some(Radius::ZERO), None),
            self.top_right.clamp(Some(Radius::ZERO), None),
            self.bottom_right.clamp(Some(Radius::ZERO), None),
            self.bottom_left.clamp(Some(Radius::ZERO), None),
        )
    }

    /// Creates an [`RSuperellipse`] from the current border radius and a [`Rect`].
    ///
    /// If any of the radii have negative values in x or y, those values will be
    /// clamped to zero in order to produce a valid [`RRect`].
    pub fn to_r_superellipse(&self, rect: Rect) -> RSuperellipse {
        RSuperellipse::from_rect_and_corners(
            rect,
            self.top_left.clamp(Some(Radius::ZERO), None),
            self.top_right.clamp(Some(Radius::ZERO), None),
            self.bottom_right.clamp(Some(Radius::ZERO), None),
            self.bottom_left.clamp(Some(Radius::ZERO), None),
        )
    }

    /// A border radius with all zero radii.
    pub const ZERO: BorderRadius = BorderRadius::all(Radius::ZERO);

    /// Integer divides each corner of the [`BorderRadius`] by the given factor.
    pub fn truncating_div(&self, other: f64) -> BorderRadius {
        BorderRadius::only(
            self.top_left.truncating_div(other),
            self.top_right.truncating_div(other),
            self.bottom_left.truncating_div(other),
            self.bottom_right.truncating_div(other),
        )
    }

    /// Linearly interpolate between two [`BorderRadius`] objects.
    ///
    /// If either is null, this function interpolates from [`BorderRadius::ZERO`].
    pub fn lerp(a: Option<BorderRadius>, b: Option<BorderRadius>, t: f64) -> Option<BorderRadius> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b * t),
            (Some(a), None) => Some(a * (1.0 - t)),
            (Some(a), Some(b)) => Some(BorderRadius::only(
                Radius::lerp(Some(a.top_left), Some(b.top_left), t).unwrap(),
                Radius::lerp(Some(a.top_right), Some(b.top_right), t).unwrap(),
                Radius::lerp(Some(a.bottom_left), Some(b.bottom_left), t).unwrap(),
                Radius::lerp(Some(a.bottom_right), Some(b.bottom_right), t).unwrap(),
            )),
        }
    }

    /// Convert this instance into a [`BorderRadius`].
    pub fn resolve(&self, _direction: Option<TextDirection>) -> BorderRadius {
        *self
    }
}

impl Sub for BorderRadius {
    type Output = BorderRadius;

    fn sub(self, other: BorderRadius) -> BorderRadius {
        BorderRadius::only(
            self.top_left - other.top_left,
            self.top_right - other.top_right,
            self.bottom_left - other.bottom_left,
            self.bottom_right - other.bottom_right,
        )
    }
}

impl Add for BorderRadius {
    type Output = BorderRadius;

    fn add(self, other: BorderRadius) -> BorderRadius {
        BorderRadius::only(
            self.top_left + other.top_left,
            self.top_right + other.top_right,
            self.bottom_left + other.bottom_left,
            self.bottom_right + other.bottom_right,
        )
    }
}

impl Neg for BorderRadius {
    type Output = BorderRadius;

    fn neg(self) -> BorderRadius {
        BorderRadius::only(
            -self.top_left,
            -self.top_right,
            -self.bottom_left,
            -self.bottom_right,
        )
    }
}

impl Mul<f64> for BorderRadius {
    type Output = BorderRadius;

    fn mul(self, other: f64) -> BorderRadius {
        BorderRadius::only(
            self.top_left * other,
            self.top_right * other,
            self.bottom_left * other,
            self.bottom_right * other,
        )
    }
}

impl Div<f64> for BorderRadius {
    type Output = BorderRadius;

    fn div(self, other: f64) -> BorderRadius {
        BorderRadius::only(
            self.top_left / other,
            self.top_right / other,
            self.bottom_left / other,
            self.bottom_right / other,
        )
    }
}

impl Rem<f64> for BorderRadius {
    type Output = BorderRadius;

    fn rem(self, other: f64) -> BorderRadius {
        BorderRadius::only(
            self.top_left % other,
            self.top_right % other,
            self.bottom_left % other,
            self.bottom_right % other,
        )
    }
}

impl Debug for BorderRadius {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&BorderRadiusGeometry::from(*self), f)
    }
}

/// An immutable set of radii for each corner of a rectangle, but with the
/// corners specified in a manner dependent on the writing direction.
///
/// This can be used to specify a corner radius on the leading or trailing edge
/// of a box, so that it flips to the other side when the text alignment flips
/// (e.g. being on the top right in English text but the top left in Arabic
/// text).
///
/// See also:
///
///  * [`BorderRadius`], a variant that uses physical labels (`topLeft` and
///    `topRight` instead of `topStart` and `topEnd`).
#[derive(Clone, Copy, PartialEq)]
pub struct BorderRadiusDirectional {
    /// The top-start [`Radius`].
    pub top_start: Radius,
    /// The top-end [`Radius`].
    pub top_end: Radius,
    /// The bottom-start [`Radius`].
    pub bottom_start: Radius,
    /// The bottom-end [`Radius`].
    pub bottom_end: Radius,
}

impl BorderRadiusDirectional {
    /// Creates a border radius where all radii are `radius`.
    pub const fn all(radius: Radius) -> BorderRadiusDirectional {
        BorderRadiusDirectional::only(radius, radius, radius, radius)
    }

    /// Creates a border radius where all radii are
    /// [`Radius::circular(radius)`](Radius::circular).
    pub fn circular(radius: f64) -> BorderRadiusDirectional {
        BorderRadiusDirectional::all(Radius::circular(radius))
    }

    /// Creates a vertically symmetric border radius where the top and bottom
    /// sides of the rectangle have the same radii.
    pub const fn vertical(top: Radius, bottom: Radius) -> BorderRadiusDirectional {
        BorderRadiusDirectional::only(top, top, bottom, bottom)
    }

    /// Creates a horizontally symmetrical border radius where the start and end
    /// sides of the rectangle have the same radii.
    pub const fn horizontal(start: Radius, end: Radius) -> BorderRadiusDirectional {
        BorderRadiusDirectional::only(start, end, start, end)
    }

    /// Creates a border radius with only the given non-zero values. The other
    /// corners will be right angles.
    pub const fn only(
        top_start: Radius,
        top_end: Radius,
        bottom_start: Radius,
        bottom_end: Radius,
    ) -> BorderRadiusDirectional {
        BorderRadiusDirectional {
            top_start,
            top_end,
            bottom_start,
            bottom_end,
        }
    }

    /// A border radius with all zero radii.
    ///
    /// Consider using [`BorderRadius::ZERO`] instead, since that object has the
    /// same effect, but will be cheaper to [`resolve`](Self::resolve).
    pub const ZERO: BorderRadiusDirectional = BorderRadiusDirectional::all(Radius::ZERO);

    /// Integer divides each corner of the [`BorderRadiusDirectional`] by the
    /// given factor.
    pub fn truncating_div(&self, other: f64) -> BorderRadiusDirectional {
        BorderRadiusDirectional::only(
            self.top_start.truncating_div(other),
            self.top_end.truncating_div(other),
            self.bottom_start.truncating_div(other),
            self.bottom_end.truncating_div(other),
        )
    }

    /// Linearly interpolate between two [`BorderRadiusDirectional`] objects.
    ///
    /// If either is null, this function interpolates from
    /// [`BorderRadiusDirectional::ZERO`].
    pub fn lerp(
        a: Option<BorderRadiusDirectional>,
        b: Option<BorderRadiusDirectional>,
        t: f64,
    ) -> Option<BorderRadiusDirectional> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b * t),
            (Some(a), None) => Some(a * (1.0 - t)),
            (Some(a), Some(b)) => Some(BorderRadiusDirectional::only(
                Radius::lerp(Some(a.top_start), Some(b.top_start), t).unwrap(),
                Radius::lerp(Some(a.top_end), Some(b.top_end), t).unwrap(),
                Radius::lerp(Some(a.bottom_start), Some(b.bottom_start), t).unwrap(),
                Radius::lerp(Some(a.bottom_end), Some(b.bottom_end), t).unwrap(),
            )),
        }
    }

    /// Convert this instance into a [`BorderRadius`], so that the radii are
    /// expressed for specific physical corners.
    pub fn resolve(&self, direction: Option<TextDirection>) -> BorderRadius {
        match direction.expect("No TextDirection found.") {
            TextDirection::Rtl => BorderRadius::only(
                self.top_end,
                self.top_start,
                self.bottom_end,
                self.bottom_start,
            ),
            TextDirection::Ltr => BorderRadius::only(
                self.top_start,
                self.top_end,
                self.bottom_start,
                self.bottom_end,
            ),
        }
    }
}

impl Sub for BorderRadiusDirectional {
    type Output = BorderRadiusDirectional;

    fn sub(self, other: BorderRadiusDirectional) -> BorderRadiusDirectional {
        BorderRadiusDirectional::only(
            self.top_start - other.top_start,
            self.top_end - other.top_end,
            self.bottom_start - other.bottom_start,
            self.bottom_end - other.bottom_end,
        )
    }
}

impl Add for BorderRadiusDirectional {
    type Output = BorderRadiusDirectional;

    fn add(self, other: BorderRadiusDirectional) -> BorderRadiusDirectional {
        BorderRadiusDirectional::only(
            self.top_start + other.top_start,
            self.top_end + other.top_end,
            self.bottom_start + other.bottom_start,
            self.bottom_end + other.bottom_end,
        )
    }
}

impl Neg for BorderRadiusDirectional {
    type Output = BorderRadiusDirectional;

    fn neg(self) -> BorderRadiusDirectional {
        BorderRadiusDirectional::only(
            -self.top_start,
            -self.top_end,
            -self.bottom_start,
            -self.bottom_end,
        )
    }
}

impl Mul<f64> for BorderRadiusDirectional {
    type Output = BorderRadiusDirectional;

    fn mul(self, other: f64) -> BorderRadiusDirectional {
        BorderRadiusDirectional::only(
            self.top_start * other,
            self.top_end * other,
            self.bottom_start * other,
            self.bottom_end * other,
        )
    }
}

impl Div<f64> for BorderRadiusDirectional {
    type Output = BorderRadiusDirectional;

    fn div(self, other: f64) -> BorderRadiusDirectional {
        BorderRadiusDirectional::only(
            self.top_start / other,
            self.top_end / other,
            self.bottom_start / other,
            self.bottom_end / other,
        )
    }
}

impl Rem<f64> for BorderRadiusDirectional {
    type Output = BorderRadiusDirectional;

    fn rem(self, other: f64) -> BorderRadiusDirectional {
        BorderRadiusDirectional::only(
            self.top_start % other,
            self.top_end % other,
            self.bottom_start % other,
            self.bottom_end % other,
        )
    }
}

impl Debug for BorderRadiusDirectional {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&BorderRadiusGeometry::from(*self), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn border_radius_constructors() {
        let radius = Radius::elliptical(5.0, 7.0);
        let all = BorderRadius::all(radius);
        assert_eq!(all.top_left, radius);
        assert_eq!(all.top_right, radius);
        assert_eq!(all.bottom_left, radius);
        assert_eq!(all.bottom_right, radius);

        let circular = BorderRadius::circular(3.0);
        assert_eq!(circular.top_left, Radius::elliptical(3.0, 3.0));

        let radius1 = Radius::elliptical(89.0, 87.0);
        let radius2 = Radius::elliptical(103.0, 107.0);
        let vertical = BorderRadius::vertical(radius1, radius2);
        assert_eq!(vertical.top_left, radius1);
        assert_eq!(vertical.top_right, radius1);
        assert_eq!(vertical.bottom_left, radius2);
        assert_eq!(vertical.bottom_right, radius2);

        let horizontal = BorderRadius::horizontal(radius1, radius2);
        assert_eq!(horizontal.top_left, radius1);
        assert_eq!(horizontal.top_right, radius2);
        assert_eq!(horizontal.bottom_left, radius1);
        assert_eq!(horizontal.bottom_right, radius2);

        assert_eq!(BorderRadius::ZERO.top_left, Radius::ZERO);

        let only = BorderRadius::only(Radius::ZERO, radius1, Radius::ZERO, radius2);
        assert_eq!(only.top_left, Radius::ZERO);
        assert_eq!(only.top_right, radius1);
        assert_eq!(only.bottom_left, Radius::ZERO);
        assert_eq!(only.bottom_right, radius2);
    }

    #[test]
    fn same_kind_operators() {
        assert_eq!(
            BorderRadius::only(
                Radius::elliptical(1.0, 2.0),
                Radius::ZERO,
                Radius::ZERO,
                Radius::ZERO
            ) - BorderRadius::only(
                Radius::elliptical(3.0, 5.0),
                Radius::ZERO,
                Radius::ZERO,
                Radius::ZERO
            ),
            BorderRadius::only(
                Radius::elliptical(-2.0, -3.0),
                Radius::ZERO,
                Radius::ZERO,
                Radius::ZERO
            )
        );
        assert_eq!(
            -BorderRadius::only(
                Radius::elliptical(1.0, 2.0),
                Radius::ZERO,
                Radius::ZERO,
                Radius::ZERO
            ),
            BorderRadius::only(
                Radius::elliptical(-1.0, -2.0),
                Radius::ZERO,
                Radius::ZERO,
                Radius::ZERO
            )
        );
        assert_eq!(
            BorderRadius::all(Radius::circular(15.0)) / 10.0,
            BorderRadius::all(Radius::circular(1.5))
        );
        assert_eq!(
            BorderRadius::all(Radius::circular(15.0)).truncating_div(10.0),
            BorderRadius::all(Radius::circular(1.0))
        );
        assert_eq!(
            BorderRadius::all(Radius::circular(15.0)) % 10.0,
            BorderRadius::all(Radius::circular(5.0))
        );
    }

    #[test]
    fn lerp_invariants() {
        let a = BorderRadius::all(Radius::circular(10.0));
        let b = BorderRadius::all(Radius::circular(20.0));
        assert_eq!(BorderRadius::lerp(Some(a), Some(b), 0.25), Some(a * 1.25));
        assert_eq!(BorderRadius::lerp(None, None, 0.25), None);
        assert_eq!(BorderRadius::lerp(None, Some(b), 0.25), Some(b * 0.25));
        assert_eq!(BorderRadius::lerp(Some(a), None, 0.25), Some(a * 0.75));
    }

    #[test]
    fn lerp_crazy() {
        let a = BorderRadius::only(
            Radius::elliptical(10.0, 20.0),
            Radius::elliptical(30.0, 40.0),
            Radius::elliptical(50.0, 60.0),
            Radius::ZERO,
        );
        let b = BorderRadius::only(
            Radius::ZERO,
            Radius::elliptical(100.0, 110.0),
            Radius::elliptical(120.0, 130.0),
            Radius::elliptical(140.0, 150.0),
        );
        let c = BorderRadius::only(
            Radius::elliptical(5.0, 10.0),
            Radius::elliptical(65.0, 75.0),
            Radius::elliptical(85.0, 95.0),
            Radius::elliptical(70.0, 75.0),
        );
        assert_eq!(BorderRadius::lerp(Some(a), Some(b), 0.5), Some(c));
    }

    #[test]
    fn directional_resolve_ltr_rtl() {
        let radius1 = Radius::elliptical(89.0, 87.0);
        let radius2 = Radius::elliptical(103.0, 107.0);
        let horizontal = BorderRadiusDirectional::horizontal(radius1, radius2);
        assert_eq!(
            horizontal.resolve(Some(TextDirection::Ltr)),
            BorderRadius::only(radius1, radius2, radius1, radius2)
        );
        assert_eq!(
            horizontal.resolve(Some(TextDirection::Rtl)),
            BorderRadius::only(radius2, radius1, radius2, radius1)
        );

        let only = BorderRadiusDirectional::only(Radius::ZERO, radius1, Radius::ZERO, radius2);
        assert_eq!(
            only.resolve(Some(TextDirection::Ltr)),
            BorderRadius::only(Radius::ZERO, radius1, Radius::ZERO, radius2)
        );
        assert_eq!(
            only.resolve(Some(TextDirection::Rtl)),
            BorderRadius::only(radius1, Radius::ZERO, radius2, Radius::ZERO)
        );
    }

    #[test]
    fn dart_runtime_type_keeps_zero_kinds_unequal() {
        assert_ne!(
            BorderRadiusGeometry::from(BorderRadius::ZERO),
            BorderRadiusGeometry::from(BorderRadiusDirectional::ZERO)
        );
    }

    #[test]
    fn copy_with_replaces_given_fields() {
        let radius = Radius::circular(10.0);
        let border_radius = BorderRadius::all(radius);
        assert_eq!(
            border_radius
                .copy_with(Some(Radius::ZERO), None, None, None)
                .top_left,
            Radius::ZERO
        );
        assert_eq!(
            border_radius
                .copy_with(Some(Radius::ZERO), None, None, None)
                .copy_with(Some(radius), None, None, None),
            border_radius
        );
    }

    #[test]
    fn geometry_factories() {
        let radius5 = Radius::circular(5.0);
        let radius10 = Radius::circular(10.0);
        assert_eq!(
            BorderRadiusGeometry::all(radius10),
            BorderRadiusGeometry::from(BorderRadius::all(radius10))
        );
        assert_eq!(
            BorderRadiusGeometry::circular(10.0),
            BorderRadiusGeometry::from(BorderRadius::circular(10.0))
        );
        assert_eq!(
            BorderRadiusGeometry::horizontal(Some(radius5), Some(radius10), None, None),
            BorderRadiusGeometry::from(BorderRadius::horizontal(radius5, radius10))
        );
        assert_eq!(
            BorderRadiusGeometry::horizontal(None, None, Some(radius5), Some(radius10)),
            BorderRadiusGeometry::from(BorderRadiusDirectional::horizontal(radius5, radius10))
        );
        assert_eq!(
            BorderRadiusGeometry::ZERO,
            BorderRadiusGeometry::from(BorderRadius::ZERO)
        );
    }
}
