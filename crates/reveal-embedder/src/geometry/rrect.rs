//! Flutter counterpart: `RRect` / `RSuperellipse` in `engine/src/flutter/lib/ui/geometry.dart`.

#![allow(clippy::too_many_arguments)]

use std::fmt::{self, Debug};

use super::geometry::{Offset, Radius, Rect};
use super::lerp::lerp_double_non_null;

/// Dart's private `_RRectLike` fields. Shared by [`RRect`] and [`RSuperellipse`].
#[derive(Clone, Copy, PartialEq)]
struct Rounded {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    tl_radius_x: f64,
    tl_radius_y: f64,
    tr_radius_x: f64,
    tr_radius_y: f64,
    br_radius_x: f64,
    br_radius_y: f64,
    bl_radius_x: f64,
    bl_radius_y: f64,
}

impl Rounded {
    fn raw(
        left: f64,
        top: f64,
        right: f64,
        bottom: f64,
        tl_radius_x: f64,
        tl_radius_y: f64,
        tr_radius_x: f64,
        tr_radius_y: f64,
        br_radius_x: f64,
        br_radius_y: f64,
        bl_radius_x: f64,
        bl_radius_y: f64,
    ) -> Rounded {
        debug_assert!(tl_radius_x >= 0.0);
        debug_assert!(tl_radius_y >= 0.0);
        debug_assert!(tr_radius_x >= 0.0);
        debug_assert!(tr_radius_y >= 0.0);
        debug_assert!(br_radius_x >= 0.0);
        debug_assert!(br_radius_y >= 0.0);
        debug_assert!(bl_radius_x >= 0.0);
        debug_assert!(bl_radius_y >= 0.0);
        Rounded {
            left,
            top,
            right,
            bottom,
            tl_radius_x,
            tl_radius_y,
            tr_radius_x,
            tr_radius_y,
            br_radius_x,
            br_radius_y,
            bl_radius_x,
            bl_radius_y,
        }
    }

    fn uniform(
        left: f64,
        top: f64,
        right: f64,
        bottom: f64,
        radius_x: f64,
        radius_y: f64,
    ) -> Rounded {
        Rounded::raw(
            left, top, right, bottom, radius_x, radius_y, radius_x, radius_y, radius_x, radius_y,
            radius_x, radius_y,
        )
    }

    fn corners(
        left: f64,
        top: f64,
        right: f64,
        bottom: f64,
        top_left: Radius,
        top_right: Radius,
        bottom_right: Radius,
        bottom_left: Radius,
    ) -> Rounded {
        Rounded::raw(
            left,
            top,
            right,
            bottom,
            top_left.x,
            top_left.y,
            top_right.x,
            top_right.y,
            bottom_right.x,
            bottom_right.y,
            bottom_left.x,
            bottom_left.y,
        )
    }

    fn shift(self, offset: Offset) -> Rounded {
        Rounded::raw(
            self.left + offset.dx(),
            self.top + offset.dy(),
            self.right + offset.dx(),
            self.bottom + offset.dy(),
            self.tl_radius_x,
            self.tl_radius_y,
            self.tr_radius_x,
            self.tr_radius_y,
            self.br_radius_x,
            self.br_radius_y,
            self.bl_radius_x,
            self.bl_radius_y,
        )
    }

    fn inflate(self, delta: f64) -> Rounded {
        Rounded::raw(
            self.left - delta,
            self.top - delta,
            self.right + delta,
            self.bottom + delta,
            (self.tl_radius_x + delta).max(0.0),
            (self.tl_radius_y + delta).max(0.0),
            (self.tr_radius_x + delta).max(0.0),
            (self.tr_radius_y + delta).max(0.0),
            (self.br_radius_x + delta).max(0.0),
            (self.br_radius_y + delta).max(0.0),
            (self.bl_radius_x + delta).max(0.0),
            (self.bl_radius_y + delta).max(0.0),
        )
    }

    fn width(self) -> f64 {
        self.right - self.left
    }

    fn height(self) -> f64 {
        self.bottom - self.top
    }

    fn get_min(min: f64, radius1: f64, radius2: f64, limit: f64) -> f64 {
        let sum = radius1 + radius2;
        if sum > limit && sum != 0.0 {
            return min.min(limit / sum);
        }
        min
    }

    fn scale_radii(self) -> Rounded {
        let mut scale = 1.0;
        scale = Rounded::get_min(scale, self.bl_radius_y, self.tl_radius_y, self.height());
        scale = Rounded::get_min(scale, self.tl_radius_x, self.tr_radius_x, self.width());
        scale = Rounded::get_min(scale, self.tr_radius_y, self.br_radius_y, self.height());
        scale = Rounded::get_min(scale, self.br_radius_x, self.bl_radius_x, self.width());
        debug_assert!(scale >= 0.0);
        if scale < 1.0 {
            Rounded::raw(
                self.left,
                self.top,
                self.right,
                self.bottom,
                self.tl_radius_x * scale,
                self.tl_radius_y * scale,
                self.tr_radius_x * scale,
                self.tr_radius_y * scale,
                self.br_radius_x * scale,
                self.br_radius_y * scale,
                self.bl_radius_x * scale,
                self.bl_radius_y * scale,
            )
        } else {
            self
        }
    }

    fn lerp_to(self, b: Option<Rounded>, t: f64) -> Rounded {
        match b {
            None => {
                let k = 1.0 - t;
                Rounded::raw(
                    self.left * k,
                    self.top * k,
                    self.right * k,
                    self.bottom * k,
                    (self.tl_radius_x * k).max(0.0),
                    (self.tl_radius_y * k).max(0.0),
                    (self.tr_radius_x * k).max(0.0),
                    (self.tr_radius_y * k).max(0.0),
                    (self.br_radius_x * k).max(0.0),
                    (self.br_radius_y * k).max(0.0),
                    (self.bl_radius_x * k).max(0.0),
                    (self.bl_radius_y * k).max(0.0),
                )
            }
            Some(b) => Rounded::raw(
                lerp_double_non_null(self.left, b.left, t),
                lerp_double_non_null(self.top, b.top, t),
                lerp_double_non_null(self.right, b.right, t),
                lerp_double_non_null(self.bottom, b.bottom, t),
                lerp_double_non_null(self.tl_radius_x, b.tl_radius_x, t).max(0.0),
                lerp_double_non_null(self.tl_radius_y, b.tl_radius_y, t).max(0.0),
                lerp_double_non_null(self.tr_radius_x, b.tr_radius_x, t).max(0.0),
                lerp_double_non_null(self.tr_radius_y, b.tr_radius_y, t).max(0.0),
                lerp_double_non_null(self.br_radius_x, b.br_radius_x, t).max(0.0),
                lerp_double_non_null(self.br_radius_y, b.br_radius_y, t).max(0.0),
                lerp_double_non_null(self.bl_radius_x, b.bl_radius_x, t).max(0.0),
                lerp_double_non_null(self.bl_radius_y, b.bl_radius_y, t).max(0.0),
            ),
        }
    }

    fn fmt_debug(self, class_name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rect = format!(
            "{:.1}, {:.1}, {:.1}, {:.1}",
            self.left, self.top, self.right, self.bottom
        );
        let tl = Radius::elliptical(self.tl_radius_x, self.tl_radius_y);
        let tr = Radius::elliptical(self.tr_radius_x, self.tr_radius_y);
        let br = Radius::elliptical(self.br_radius_x, self.br_radius_y);
        let bl = Radius::elliptical(self.bl_radius_x, self.bl_radius_y);
        if tl == tr && tr == br && br == bl {
            if tl.x == tl.y {
                write!(f, "{class_name}.fromLTRBR({rect}, {:.1})", tl.x)
            } else {
                write!(
                    f,
                    "{class_name}.fromLTRBXY({rect}, {:.1}, {:.1})",
                    tl.x, tl.y
                )
            }
        } else {
            write!(
                f,
                "{class_name}.fromLTRBAndCorners({rect}, topLeft: {tl:?}, topRight: {tr:?}, bottomRight: {br:?}, bottomLeft: {bl:?})"
            )
        }
    }
}

macro_rules! rounded_type {
    ($name:ident) => {
        impl $name {
            fn from_inner(inner: Rounded) -> $name {
                $name {
                    left: inner.left,
                    top: inner.top,
                    right: inner.right,
                    bottom: inner.bottom,
                    tl_radius_x: inner.tl_radius_x,
                    tl_radius_y: inner.tl_radius_y,
                    tr_radius_x: inner.tr_radius_x,
                    tr_radius_y: inner.tr_radius_y,
                    br_radius_x: inner.br_radius_x,
                    br_radius_y: inner.br_radius_y,
                    bl_radius_x: inner.bl_radius_x,
                    bl_radius_y: inner.bl_radius_y,
                }
            }

            fn inner(self) -> Rounded {
                Rounded {
                    left: self.left,
                    top: self.top,
                    right: self.right,
                    bottom: self.bottom,
                    tl_radius_x: self.tl_radius_x,
                    tl_radius_y: self.tl_radius_y,
                    tr_radius_x: self.tr_radius_x,
                    tr_radius_y: self.tr_radius_y,
                    br_radius_x: self.br_radius_x,
                    br_radius_y: self.br_radius_y,
                    bl_radius_x: self.bl_radius_x,
                    bl_radius_y: self.bl_radius_y,
                }
            }

            /// Construct from left, top, right, bottom, and the same radii along
            /// the horizontal axis and the vertical axis.
            pub fn from_ltrbxy(
                left: f64,
                top: f64,
                right: f64,
                bottom: f64,
                radius_x: f64,
                radius_y: f64,
            ) -> $name {
                $name::from_inner(Rounded::uniform(
                    left, top, right, bottom, radius_x, radius_y,
                ))
            }

            /// Construct from left, top, right, bottom, and the same radius in
            /// each corner.
            pub fn from_ltrbr(
                left: f64,
                top: f64,
                right: f64,
                bottom: f64,
                radius: Radius,
            ) -> $name {
                $name::from_ltrbxy(left, top, right, bottom, radius.x, radius.y)
            }

            /// Construct from a bounding box and the same radii along each axis.
            pub fn from_rect_xy(rect: Rect, radius_x: f64, radius_y: f64) -> $name {
                $name::from_ltrbxy(
                    rect.left,
                    rect.top,
                    rect.right,
                    rect.bottom,
                    radius_x,
                    radius_y,
                )
            }

            /// Construct from a bounding box and a radius that is the same in
            /// each corner.
            pub fn from_rect_and_radius(rect: Rect, radius: Radius) -> $name {
                $name::from_rect_xy(rect, radius.x, radius.y)
            }

            /// Construct from left, top, right, bottom, and per-corner radii.
            pub fn from_ltrb_and_corners(
                left: f64,
                top: f64,
                right: f64,
                bottom: f64,
                top_left: Radius,
                top_right: Radius,
                bottom_right: Radius,
                bottom_left: Radius,
            ) -> $name {
                $name::from_inner(Rounded::corners(
                    left,
                    top,
                    right,
                    bottom,
                    top_left,
                    top_right,
                    bottom_right,
                    bottom_left,
                ))
            }

            /// Construct from a bounding box and per-corner radii.
            pub fn from_rect_and_corners(
                rect: Rect,
                top_left: Radius,
                top_right: Radius,
                bottom_right: Radius,
                bottom_left: Radius,
            ) -> $name {
                $name::from_ltrb_and_corners(
                    rect.left,
                    rect.top,
                    rect.right,
                    rect.bottom,
                    top_left,
                    top_right,
                    bottom_right,
                    bottom_left,
                )
            }

            /// All values set to zero.
            pub const ZERO: $name = $name {
                left: 0.0,
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                tl_radius_x: 0.0,
                tl_radius_y: 0.0,
                tr_radius_x: 0.0,
                tr_radius_y: 0.0,
                br_radius_x: 0.0,
                br_radius_y: 0.0,
                bl_radius_x: 0.0,
                bl_radius_y: 0.0,
            };

            /// The top-left [`Radius`].
            pub fn tl_radius(self) -> Radius {
                Radius::elliptical(self.tl_radius_x, self.tl_radius_y)
            }

            /// The top-right [`Radius`].
            pub fn tr_radius(self) -> Radius {
                Radius::elliptical(self.tr_radius_x, self.tr_radius_y)
            }

            /// The bottom-right [`Radius`].
            pub fn br_radius(self) -> Radius {
                Radius::elliptical(self.br_radius_x, self.br_radius_y)
            }

            /// The bottom-left [`Radius`].
            pub fn bl_radius(self) -> Radius {
                Radius::elliptical(self.bl_radius_x, self.bl_radius_y)
            }

            /// Returns a clone translated by the given offset.
            pub fn shift(self, offset: Offset) -> $name {
                $name::from_inner(self.inner().shift(offset))
            }

            /// Returns a clone with edges and radii moved outwards by `delta`.
            pub fn inflate(self, delta: f64) -> $name {
                $name::from_inner(self.inner().inflate(delta))
            }

            /// Returns a clone with edges and radii moved inwards by `delta`.
            pub fn deflate(self, delta: f64) -> $name {
                self.inflate(-delta)
            }

            /// The distance between the left and right edges.
            pub fn width(self) -> f64 {
                self.inner().width()
            }

            /// The distance between the top and bottom edges.
            pub fn height(self) -> f64 {
                self.inner().height()
            }

            /// The bounding box of this rounded rectangle.
            pub fn outer_rect(self) -> Rect {
                Rect::from_ltrb(self.left, self.top, self.right, self.bottom)
            }

            /// The non-rounded rectangle constrained by the smaller of the two
            /// diagonals through the middle of the curve corners.
            pub fn safe_inner_rect(self) -> Rect {
                const K_INSET_FACTOR: f64 = 0.29289321881; // 1-cos(pi/4)
                let left_radius = self.bl_radius_x.max(self.tl_radius_x);
                let top_radius = self.tl_radius_y.max(self.tr_radius_y);
                let right_radius = self.tr_radius_x.max(self.br_radius_x);
                let bottom_radius = self.br_radius_y.max(self.bl_radius_y);
                Rect::from_ltrb(
                    self.left + left_radius * K_INSET_FACTOR,
                    self.top + top_radius * K_INSET_FACTOR,
                    self.right - right_radius * K_INSET_FACTOR,
                    self.bottom - bottom_radius * K_INSET_FACTOR,
                )
            }

            /// The rectangle formed from the inner-most centers of the corner
            /// ellipses.
            pub fn middle_rect(self) -> Rect {
                let left_radius = self.bl_radius_x.max(self.tl_radius_x);
                let top_radius = self.tl_radius_y.max(self.tr_radius_y);
                let right_radius = self.tr_radius_x.max(self.br_radius_x);
                let bottom_radius = self.br_radius_y.max(self.bl_radius_y);
                Rect::from_ltrb(
                    self.left + left_radius,
                    self.top + top_radius,
                    self.right - right_radius,
                    self.bottom - bottom_radius,
                )
            }

            /// The biggest rectangle entirely inside with the full width.
            pub fn wide_middle_rect(self) -> Rect {
                let top_radius = self.tl_radius_y.max(self.tr_radius_y);
                let bottom_radius = self.br_radius_y.max(self.bl_radius_y);
                Rect::from_ltrb(
                    self.left,
                    self.top + top_radius,
                    self.right,
                    self.bottom - bottom_radius,
                )
            }

            /// The biggest rectangle entirely inside with the full height.
            pub fn tall_middle_rect(self) -> Rect {
                let left_radius = self.bl_radius_x.max(self.tl_radius_x);
                let right_radius = self.tr_radius_x.max(self.br_radius_x);
                Rect::from_ltrb(
                    self.left + left_radius,
                    self.top,
                    self.right - right_radius,
                    self.bottom,
                )
            }

            /// Whether this rounded rectangle encloses a non-zero area.
            pub fn is_empty(self) -> bool {
                self.left >= self.right || self.top >= self.bottom
            }

            /// Whether all coordinates are finite.
            pub fn is_finite(self) -> bool {
                self.left.is_finite()
                    && self.top.is_finite()
                    && self.right.is_finite()
                    && self.bottom.is_finite()
            }

            /// Whether this is a simple rectangle with zero corner radii.
            pub fn is_rect(self) -> bool {
                (self.tl_radius_x == 0.0 || self.tl_radius_y == 0.0)
                    && (self.tr_radius_x == 0.0 || self.tr_radius_y == 0.0)
                    && (self.bl_radius_x == 0.0 || self.bl_radius_y == 0.0)
                    && (self.br_radius_x == 0.0 || self.br_radius_y == 0.0)
            }

            /// Whether this rounded rectangle has a side with no straight section.
            pub fn is_stadium(self) -> bool {
                self.tl_radius() == self.tr_radius()
                    && self.tr_radius() == self.br_radius()
                    && self.br_radius() == self.bl_radius()
                    && (self.width() <= 2.0 * self.tl_radius_x
                        || self.height() <= 2.0 * self.tl_radius_y)
            }

            /// Whether this rounded rectangle has no side with a straight section.
            pub fn is_ellipse(self) -> bool {
                self.tl_radius() == self.tr_radius()
                    && self.tr_radius() == self.br_radius()
                    && self.br_radius() == self.bl_radius()
                    && self.width() <= 2.0 * self.tl_radius_x
                    && self.height() <= 2.0 * self.tl_radius_y
            }

            /// Whether this rounded rectangle would draw as a circle.
            pub fn is_circle(self) -> bool {
                self.width() == self.height() && self.is_ellipse()
            }

            /// The lesser of the magnitudes of the width and the height.
            pub fn shortest_side(self) -> f64 {
                self.width().abs().min(self.height().abs())
            }

            /// The greater of the magnitudes of the width and the height.
            pub fn longest_side(self) -> f64 {
                self.width().abs().max(self.height().abs())
            }

            /// Whether any of the dimensions are `NaN`.
            pub fn has_nan(self) -> bool {
                self.left.is_nan()
                    || self.top.is_nan()
                    || self.right.is_nan()
                    || self.bottom.is_nan()
                    || self.tr_radius_x.is_nan()
                    || self.tr_radius_y.is_nan()
                    || self.tl_radius_x.is_nan()
                    || self.tl_radius_y.is_nan()
                    || self.br_radius_x.is_nan()
                    || self.br_radius_y.is_nan()
                    || self.bl_radius_x.is_nan()
                    || self.bl_radius_y.is_nan()
            }

            /// The offset halfway between the left and right and the top and
            /// bottom edges.
            pub fn center(self) -> Offset {
                Offset::new(
                    self.left + self.width() / 2.0,
                    self.top + self.height() / 2.0,
                )
            }

            /// Scales all radii so that on each side their sum will not exceed
            /// the width or height.
            pub fn scale_radii(self) -> $name {
                $name::from_inner(self.inner().scale_radii())
            }

            fn lerp_to(self, b: Option<$name>, t: f64) -> $name {
                $name::from_inner(self.inner().lerp_to(b.map($name::inner), t))
            }

            /// Linearly interpolate between two rounded rectangles.
            ///
            /// If either is null, this function substitutes [`ZERO`](Self::ZERO).
            pub fn lerp(a: Option<$name>, b: Option<$name>, t: f64) -> Option<$name> {
                match a {
                    None => b.map(|b| b.lerp_to(None, 1.0 - t)),
                    Some(a) => Some(a.lerp_to(b, t)),
                }
            }
        }
    };
}

/// An immutable rounded rectangle with the custom radii for all four corners.
#[derive(Clone, Copy, PartialEq)]
pub struct RRect {
    /// The offset of the left edge of this rectangle from the x axis.
    pub left: f64,
    /// The offset of the top edge of this rectangle from the y axis.
    pub top: f64,
    /// The offset of the right edge of this rectangle from the x axis.
    pub right: f64,
    /// The offset of the bottom edge of this rectangle from the y axis.
    pub bottom: f64,
    /// The top-left horizontal radius.
    pub tl_radius_x: f64,
    /// The top-left vertical radius.
    pub tl_radius_y: f64,
    /// The top-right horizontal radius.
    pub tr_radius_x: f64,
    /// The top-right vertical radius.
    pub tr_radius_y: f64,
    /// The bottom-right horizontal radius.
    pub br_radius_x: f64,
    /// The bottom-right vertical radius.
    pub br_radius_y: f64,
    /// The bottom-left horizontal radius.
    pub bl_radius_x: f64,
    /// The bottom-left vertical radius.
    pub bl_radius_y: f64,
}

rounded_type!(RRect);

impl RRect {
    /// Whether the point specified by the given offset (which is assumed to be
    /// relative to the origin) lies inside the rounded rectangle.
    pub fn contains(self, point: Offset) -> bool {
        if point.dx() < self.left
            || point.dx() >= self.right
            || point.dy() < self.top
            || point.dy() >= self.bottom
        {
            return false;
        }
        let scaled = self.scale_radii();
        let (x, y, radius_x, radius_y);
        if point.dx() < scaled.left + scaled.tl_radius_x
            && point.dy() < scaled.top + scaled.tl_radius_y
        {
            x = point.dx() - scaled.left - scaled.tl_radius_x;
            y = point.dy() - scaled.top - scaled.tl_radius_y;
            radius_x = scaled.tl_radius_x;
            radius_y = scaled.tl_radius_y;
        } else if point.dx() > scaled.right - scaled.tr_radius_x
            && point.dy() < scaled.top + scaled.tr_radius_y
        {
            x = point.dx() - scaled.right + scaled.tr_radius_x;
            y = point.dy() - scaled.top - scaled.tr_radius_y;
            radius_x = scaled.tr_radius_x;
            radius_y = scaled.tr_radius_y;
        } else if point.dx() > scaled.right - scaled.br_radius_x
            && point.dy() > scaled.bottom - scaled.br_radius_y
        {
            x = point.dx() - scaled.right + scaled.br_radius_x;
            y = point.dy() - scaled.bottom + scaled.br_radius_y;
            radius_x = scaled.br_radius_x;
            radius_y = scaled.br_radius_y;
        } else if point.dx() < scaled.left + scaled.bl_radius_x
            && point.dy() > scaled.bottom - scaled.bl_radius_y
        {
            x = point.dx() - scaled.left - scaled.bl_radius_x;
            y = point.dy() - scaled.bottom + scaled.bl_radius_y;
            radius_x = scaled.bl_radius_x;
            radius_y = scaled.bl_radius_y;
        } else {
            return true;
        }
        let x = x / radius_x;
        let y = y / radius_y;
        x * x + y * y <= 1.0
    }
}

impl Debug for RRect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner().fmt_debug("RRect", f)
    }
}

/// An immutable rounded superellipse.
///
/// A rounded superellipse (not to be confused with a standard superellipse) is
/// a shape formed by replacing the four curved corners of a superellipse with
/// circular arcs.
#[derive(Clone, Copy, PartialEq)]
pub struct RSuperellipse {
    /// The offset of the left edge of this rectangle from the x axis.
    pub left: f64,
    /// The offset of the top edge of this rectangle from the y axis.
    pub top: f64,
    /// The offset of the right edge of this rectangle from the x axis.
    pub right: f64,
    /// The offset of the bottom edge of this rectangle from the y axis.
    pub bottom: f64,
    /// The top-left horizontal radius.
    pub tl_radius_x: f64,
    /// The top-left vertical radius.
    pub tl_radius_y: f64,
    /// The top-right horizontal radius.
    pub tr_radius_x: f64,
    /// The top-right vertical radius.
    pub tr_radius_y: f64,
    /// The bottom-right horizontal radius.
    pub br_radius_x: f64,
    /// The bottom-right vertical radius.
    pub br_radius_y: f64,
    /// The bottom-left horizontal radius.
    pub bl_radius_x: f64,
    /// The bottom-left vertical radius.
    pub bl_radius_y: f64,
}

rounded_type!(RSuperellipse);

impl RSuperellipse {
    /// Whether the point specified by the given offset (which is assumed to be
    /// relative to the origin) lies inside the rounded superellipse.
    ///
    /// Dart's method is engine FFI (Impeller). Ours uses valo's Impeller-port
    /// [`valo_geometry::RoundSuperellipse::contains`].
    pub fn contains(self, point: Offset) -> bool {
        valo_geometry::RoundSuperellipse::new(
            self.outer_rect().into(),
            super::rsuperellipse_radii_elliptical(self),
        )
        .contains(point.into())
    }
}

impl Debug for RSuperellipse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner().fmt_debug("RSuperellipse", f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // engine geometry_test.dart 'RRect.fromRectXY'.
    #[test]
    fn from_rect_xy() {
        let base_rect = Rect::from_ltwh(1.0, 3.0, 5.0, 7.0);
        let r = RRect::from_rect_xy(base_rect, 1.0, 1.0);
        assert_eq!(r.left, 1.0);
        assert_eq!(r.top, 3.0);
        assert_eq!(r.right, 6.0);
        assert_eq!(r.bottom, 10.0);
        assert_eq!(r.shortest_side(), 5.0);
        assert_eq!(r.longest_side(), 7.0);
    }

    fn sample_rrect() -> RRect {
        RRect::from_rect_and_corners(
            Rect::from_ltrb(1.0, 1.0, 2.0, 2.0),
            Radius::circular(0.5),
            Radius::circular(0.25),
            Radius::elliptical(0.25, 0.75),
            Radius::ZERO,
        )
    }

    // engine geometry_test.dart 'RRect.contains()'.
    #[test]
    fn contains_points() {
        let rrect = sample_rrect();
        assert!(!rrect.contains(Offset::new(1.0, 1.0)));
        assert!(!rrect.contains(Offset::new(1.1, 1.1)));
        assert!(rrect.contains(Offset::new(1.15, 1.15)));
        assert!(!rrect.contains(Offset::new(2.0, 1.0)));
        assert!(!rrect.contains(Offset::new(1.93, 1.07)));
        assert!(!rrect.contains(Offset::new(1.97, 1.7)));
        assert!(rrect.contains(Offset::new(1.7, 1.97)));
        assert!(rrect.contains(Offset::new(1.0, 1.99)));
    }

    // engine geometry_test.dart 'RRect.contains() large radii'.
    #[test]
    fn contains_large_radii() {
        let rrect = RRect::from_rect_and_corners(
            Rect::from_ltrb(1.0, 1.0, 2.0, 2.0),
            Radius::circular(5000.0),
            Radius::circular(2500.0),
            Radius::elliptical(2500.0, 7500.0),
            Radius::ZERO,
        );
        assert!(!rrect.contains(Offset::new(1.0, 1.0)));
        assert!(!rrect.contains(Offset::new(1.1, 1.1)));
        assert!(rrect.contains(Offset::new(1.15, 1.15)));
        assert!(!rrect.contains(Offset::new(2.0, 1.0)));
        assert!(!rrect.contains(Offset::new(1.93, 1.07)));
        assert!(!rrect.contains(Offset::new(1.97, 1.7)));
        assert!(rrect.contains(Offset::new(1.7, 1.97)));
        assert!(rrect.contains(Offset::new(1.0, 1.99)));
    }

    fn assert_scaled_corners(rrect: RRect) {
        assert_eq!(rrect.left, 1.0);
        assert_eq!(rrect.top, 1.0);
        assert_eq!(rrect.right, 2.0);
        assert_eq!(rrect.bottom, 2.0);
        assert_eq!(rrect.tl_radius_x, 0.5);
        assert_eq!(rrect.tl_radius_y, 0.5);
        assert_eq!(rrect.tr_radius_x, 0.25);
        assert_eq!(rrect.tr_radius_y, 0.25);
        assert_eq!(rrect.bl_radius_x, 0.0);
        assert_eq!(rrect.bl_radius_y, 0.0);
        assert_eq!(rrect.br_radius_x, 0.25);
        assert_eq!(rrect.br_radius_y, 0.75);
    }

    // engine geometry_test.dart 'RRect.scaleRadii() properly constrained radii should remain unchanged'.
    #[test]
    fn scale_radii_leaves_valid_radii_unchanged() {
        assert_scaled_corners(sample_rrect().scale_radii());
    }

    // engine geometry_test.dart 'RRect.scaleRadii() sum of radii that exceed side length should properly scale'.
    #[test]
    fn scale_radii_when_sum_exceeds_side() {
        let rrect = RRect::from_rect_and_corners(
            Rect::from_ltrb(1.0, 1.0, 2.0, 2.0),
            Radius::circular(5000.0),
            Radius::circular(2500.0),
            Radius::elliptical(2500.0, 7500.0),
            Radius::ZERO,
        )
        .scale_radii();
        assert_scaled_corners(rrect);
    }

    // engine geometry_test.dart 'RRect asserts when corner radii are negative'.
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn negative_corner_radii_panic_in_debug() {
        let _ = RRect::from_rect_and_corners(
            Rect::from_ltrb(10.0, 20.0, 30.0, 40.0),
            Radius::circular(-1.0),
            Radius::ZERO,
            Radius::ZERO,
            Radius::ZERO,
        );
    }

    // engine geometry_test.dart 'RRect.inflate clamps when deflating past zero'.
    #[test]
    fn inflate_clamps_when_deflating_past_zero() {
        let mut rrect = RRect::from_rect_and_corners(
            Rect::from_ltrb(10.0, 20.0, 30.0, 40.0),
            Radius::circular(1.0),
            Radius::circular(2.0),
            Radius::circular(4.0),
            Radius::circular(3.0),
        )
        .inflate(-1.0);
        assert_eq!(rrect.tl_radius_x, 0.0);
        assert_eq!(rrect.tl_radius_y, 0.0);
        assert_eq!(rrect.tr_radius_x, 1.0);
        assert_eq!(rrect.tr_radius_y, 1.0);
        assert_eq!(rrect.bl_radius_x, 2.0);
        assert_eq!(rrect.bl_radius_y, 2.0);
        assert_eq!(rrect.br_radius_x, 3.0);
        assert_eq!(rrect.br_radius_y, 3.0);

        rrect = rrect.inflate(-1.0);
        assert_eq!(rrect.tl_radius_x, 0.0);
        assert_eq!(rrect.tr_radius_x, 0.0);
        assert_eq!(rrect.bl_radius_x, 1.0);
        assert_eq!(rrect.br_radius_x, 2.0);

        rrect = rrect.inflate(-1.0);
        assert_eq!(rrect.bl_radius_x, 0.0);
        assert_eq!(rrect.br_radius_x, 1.0);

        rrect = rrect.inflate(-1.0);
        assert_eq!(rrect.br_radius_x, 0.0);
        assert_eq!(rrect.br_radius_y, 0.0);
    }

    #[test]
    fn rrect_and_rsuperellipse_are_unequal_at_the_same_floats() {
        let rect = Rect::from_ltrb(0.0, 0.0, 10.0, 10.0);
        let rrect = RRect::from_rect_and_radius(rect, Radius::circular(2.0));
        let rse = RSuperellipse::from_rect_and_radius(rect, Radius::circular(2.0));
        assert_eq!(rrect.left, rse.left);
        assert!(format!("{rrect:?}").starts_with("RRect"));
        assert!(format!("{rse:?}").starts_with("RSuperellipse"));
    }

    #[test]
    fn rsuperellipse_contains_the_center_and_rejects_the_outside() {
        let rse = RSuperellipse::from_rect_and_radius(
            Rect::from_ltrb(0.0, 0.0, 10.0, 10.0),
            Radius::circular(2.0),
        );
        assert!(rse.contains(Offset::new(5.0, 5.0)));
        assert!(!rse.contains(Offset::new(-1.0, 5.0)));
    }
}
