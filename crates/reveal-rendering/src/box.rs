//! Flutter counterpart: `rendering/box.dart` (`BoxConstraints` only).
//!
//! `RenderBox` / `_DebugSize` / `BoxHitTestResult` wait on `RenderObject`.

use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};

use reveal_embedder::{Size, ViewConstraints, clamp_double, lerp_double};
use reveal_painting::EdgeInsetsGeometry;

use crate::object::Constraints;

/// Immutable layout constraints for `RenderBox` layout.
///
/// A [`Size`] respects a [`BoxConstraints`] if, and only if, all of the following
/// relations hold:
///
/// * [`min_width`](Self::min_width) <= [`Size::width`] <= [`max_width`](Self::max_width)
/// * [`min_height`](Self::min_height) <= [`Size::height`] <= [`max_height`](Self::max_height)
#[derive(Clone, Copy)]
pub struct BoxConstraints {
    /// The minimum width that satisfies the constraints.
    pub min_width: f64,
    /// The maximum width that satisfies the constraints.
    ///
    /// Might be [`f64::INFINITY`].
    pub max_width: f64,
    /// The minimum height that satisfies the constraints.
    pub min_height: f64,
    /// The maximum height that satisfies the constraints.
    ///
    /// Might be [`f64::INFINITY`].
    pub max_height: f64,
}

impl BoxConstraints {
    /// Creates box constraints with the given constraints.
    pub const fn new() -> BoxConstraints {
        BoxConstraints {
            min_width: 0.0,
            max_width: f64::INFINITY,
            min_height: 0.0,
            max_height: f64::INFINITY,
        }
    }

    /// Sets [`min_width`](Self::min_width).
    pub const fn min_width(mut self, value: f64) -> BoxConstraints {
        self.min_width = value;
        self
    }

    /// Sets [`max_width`](Self::max_width).
    pub const fn max_width(mut self, value: f64) -> BoxConstraints {
        self.max_width = value;
        self
    }

    /// Sets [`min_height`](Self::min_height).
    pub const fn min_height(mut self, value: f64) -> BoxConstraints {
        self.min_height = value;
        self
    }

    /// Sets [`max_height`](Self::max_height).
    pub const fn max_height(mut self, value: f64) -> BoxConstraints {
        self.max_height = value;
        self
    }

    /// Creates box constraints that is respected only by the given size.
    pub fn tight(size: Size) -> BoxConstraints {
        BoxConstraints {
            min_width: size.width(),
            max_width: size.width(),
            min_height: size.height(),
            max_height: size.height(),
        }
    }

    /// Creates box constraints that require the given width or height.
    pub const fn tight_for(width: Option<f64>, height: Option<f64>) -> BoxConstraints {
        BoxConstraints {
            min_width: match width {
                Some(width) => width,
                None => 0.0,
            },
            max_width: match width {
                Some(width) => width,
                None => f64::INFINITY,
            },
            min_height: match height {
                Some(height) => height,
                None => 0.0,
            },
            max_height: match height {
                Some(height) => height,
                None => f64::INFINITY,
            },
        }
    }

    /// Creates box constraints that require the given width or height, except if
    /// they are infinite.
    pub const fn tight_for_finite(width: f64, height: f64) -> BoxConstraints {
        BoxConstraints {
            min_width: if width != f64::INFINITY { width } else { 0.0 },
            max_width: if width != f64::INFINITY {
                width
            } else {
                f64::INFINITY
            },
            min_height: if height != f64::INFINITY { height } else { 0.0 },
            max_height: if height != f64::INFINITY {
                height
            } else {
                f64::INFINITY
            },
        }
    }

    /// Creates box constraints that forbid sizes larger than the given size.
    pub fn loose(size: Size) -> BoxConstraints {
        BoxConstraints {
            min_width: 0.0,
            max_width: size.width(),
            min_height: 0.0,
            max_height: size.height(),
        }
    }

    /// Creates box constraints that expand to fill another box constraints.
    pub const fn expand(width: Option<f64>, height: Option<f64>) -> BoxConstraints {
        BoxConstraints {
            min_width: match width {
                Some(width) => width,
                None => f64::INFINITY,
            },
            max_width: match width {
                Some(width) => width,
                None => f64::INFINITY,
            },
            min_height: match height {
                Some(height) => height,
                None => f64::INFINITY,
            },
            max_height: match height {
                Some(height) => height,
                None => f64::INFINITY,
            },
        }
    }

    /// Creates box constraints that match the given view constraints.
    pub const fn from_view_constraints(constraints: ViewConstraints) -> BoxConstraints {
        BoxConstraints {
            min_width: constraints.min_width,
            max_width: constraints.max_width,
            min_height: constraints.min_height,
            max_height: constraints.max_height,
        }
    }

    /// Creates a copy of this box constraints. Chain field replacements.
    pub const fn copy_with(self) -> BoxConstraints {
        self
    }

    /// Returns new box constraints that are smaller by the given edge dimensions.
    pub fn deflate(self, edges: EdgeInsetsGeometry) -> BoxConstraints {
        debug_assert!(self.debug_assert_is_valid(false));
        let horizontal = edges.horizontal();
        let vertical = edges.vertical();
        let deflated_min_width = (self.min_width - horizontal).max(0.0);
        let deflated_min_height = (self.min_height - vertical).max(0.0);
        BoxConstraints {
            min_width: deflated_min_width,
            max_width: (self.max_width - horizontal).max(deflated_min_width),
            min_height: deflated_min_height,
            max_height: (self.max_height - vertical).max(deflated_min_height),
        }
    }

    /// Returns new box constraints that remove the minimum width and height requirements.
    pub fn loosen(self) -> BoxConstraints {
        debug_assert!(self.debug_assert_is_valid(false));
        BoxConstraints {
            min_width: 0.0,
            max_width: self.max_width,
            min_height: 0.0,
            max_height: self.max_height,
        }
    }

    /// Returns new box constraints that respect the given constraints while being
    /// as close as possible to the original constraints.
    pub fn enforce(self, constraints: BoxConstraints) -> BoxConstraints {
        BoxConstraints {
            min_width: clamp_double(self.min_width, constraints.min_width, constraints.max_width),
            max_width: clamp_double(self.max_width, constraints.min_width, constraints.max_width),
            min_height: clamp_double(
                self.min_height,
                constraints.min_height,
                constraints.max_height,
            ),
            max_height: clamp_double(
                self.max_height,
                constraints.min_height,
                constraints.max_height,
            ),
        }
    }

    /// Returns new box constraints with a tight width and/or height as close to
    /// the given width and height as possible while still respecting the original
    /// box constraints.
    pub fn tighten(self, width: Option<f64>, height: Option<f64>) -> BoxConstraints {
        BoxConstraints {
            min_width: match width {
                Some(width) => clamp_double(width, self.min_width, self.max_width),
                None => self.min_width,
            },
            max_width: match width {
                Some(width) => clamp_double(width, self.min_width, self.max_width),
                None => self.max_width,
            },
            min_height: match height {
                Some(height) => clamp_double(height, self.min_height, self.max_height),
                None => self.min_height,
            },
            max_height: match height {
                Some(height) => clamp_double(height, self.min_height, self.max_height),
                None => self.max_height,
            },
        }
    }

    /// A box constraints with the width and height constraints flipped.
    pub const fn flipped(self) -> BoxConstraints {
        BoxConstraints {
            min_width: self.min_height,
            max_width: self.max_height,
            min_height: self.min_width,
            max_height: self.max_width,
        }
    }

    /// Returns box constraints with the same width constraints but with
    /// unconstrained height.
    pub const fn width_constraints(self) -> BoxConstraints {
        BoxConstraints {
            min_width: self.min_width,
            max_width: self.max_width,
            min_height: 0.0,
            max_height: f64::INFINITY,
        }
    }

    /// Returns box constraints with the same height constraints but with
    /// unconstrained width.
    pub const fn height_constraints(self) -> BoxConstraints {
        BoxConstraints {
            min_width: 0.0,
            max_width: f64::INFINITY,
            min_height: self.min_height,
            max_height: self.max_height,
        }
    }

    /// Returns the width that both satisfies the constraints and is as close as
    /// possible to the given width.
    pub fn constrain_width(self, width: f64) -> f64 {
        debug_assert!(self.debug_assert_is_valid(false));
        clamp_double(width, self.min_width, self.max_width)
    }

    /// Returns the height that both satisfies the constraints and is as close as
    /// possible to the given height.
    pub fn constrain_height(self, height: f64) -> f64 {
        debug_assert!(self.debug_assert_is_valid(false));
        clamp_double(height, self.min_height, self.max_height)
    }

    /// Returns the size that both satisfies the constraints and is as close as
    /// possible to the given size.
    pub fn constrain(self, size: Size) -> Size {
        Size::new(
            self.constrain_width(size.width()),
            self.constrain_height(size.height()),
        )
    }

    /// Returns the size that both satisfies the constraints and is as close as
    /// possible to the given width and height.
    pub fn constrain_dimensions(self, width: f64, height: f64) -> Size {
        Size::new(self.constrain_width(width), self.constrain_height(height))
    }

    /// Returns a size that attempts to meet the following conditions, in order:
    ///
    ///  * The size must satisfy these constraints.
    ///  * The aspect ratio of the returned size matches the aspect ratio of the
    ///    given size.
    ///  * The returned size is as big as possible while still being equal to or
    ///    smaller than the given size.
    pub fn constrain_size_and_attempt_to_preserve_aspect_ratio(self, size: Size) -> Size {
        if self.is_tight() {
            return self.smallest();
        }
        if size.is_empty() {
            return self.constrain(size);
        }
        let mut width = size.width();
        let mut height = size.height();
        let aspect_ratio = width / height;
        if width > self.max_width {
            width = self.max_width;
            height = width / aspect_ratio;
        }
        if height > self.max_height {
            height = self.max_height;
            width = height * aspect_ratio;
        }
        if width < self.min_width {
            width = self.min_width;
            height = width / aspect_ratio;
        }
        if height < self.min_height {
            height = self.min_height;
            width = height * aspect_ratio;
        }
        Size::new(self.constrain_width(width), self.constrain_height(height))
    }

    /// The biggest size that satisfies the constraints.
    pub fn biggest(self) -> Size {
        Size::new(
            self.constrain_width(f64::INFINITY),
            self.constrain_height(f64::INFINITY),
        )
    }

    /// The smallest size that satisfies the constraints.
    pub fn smallest(self) -> Size {
        Size::new(self.constrain_width(0.0), self.constrain_height(0.0))
    }

    /// Whether there is exactly one width value that satisfies the constraints.
    pub fn has_tight_width(self) -> bool {
        self.min_width >= self.max_width
    }

    /// Whether there is exactly one height value that satisfies the constraints.
    pub fn has_tight_height(self) -> bool {
        self.min_height >= self.max_height
    }

    /// Whether there is an upper bound on the maximum width.
    pub fn has_bounded_width(self) -> bool {
        self.max_width < f64::INFINITY
    }

    /// Whether there is an upper bound on the maximum height.
    pub fn has_bounded_height(self) -> bool {
        self.max_height < f64::INFINITY
    }

    /// Whether the width constraint is infinite.
    pub fn has_infinite_width(self) -> bool {
        self.min_width >= f64::INFINITY
    }

    /// Whether the height constraint is infinite.
    pub fn has_infinite_height(self) -> bool {
        self.min_height >= f64::INFINITY
    }

    /// Whether the given size satisfies the constraints.
    pub fn is_satisfied_by(self, size: Size) -> bool {
        debug_assert!(self.debug_assert_is_valid(false));
        self.min_width <= size.width()
            && size.width() <= self.max_width
            && self.min_height <= size.height()
            && size.height() <= self.max_height
    }

    /// Scales each constraint parameter by the inverse of the given factor,
    /// rounded toward zero (Dart `~/`).
    pub fn truncating_div(self, factor: f64) -> BoxConstraints {
        BoxConstraints {
            min_width: (self.min_width / factor).trunc(),
            max_width: (self.max_width / factor).trunc(),
            min_height: (self.min_height / factor).trunc(),
            max_height: (self.max_height / factor).trunc(),
        }
    }

    /// Linearly interpolate between two box constraints.
    ///
    /// If either is none, this function interpolates from a [`BoxConstraints`]
    /// object whose fields are all set to 0.0.
    pub fn lerp(
        a: Option<BoxConstraints>,
        b: Option<BoxConstraints>,
        t: f64,
    ) -> Option<BoxConstraints> {
        if a == b {
            return a;
        }
        let Some(a) = a else {
            return Some((b.unwrap() * t).normalize());
        };
        let Some(b) = b else {
            return Some((a * (1.0 - t)).normalize());
        };
        debug_assert!(a.debug_assert_is_valid(false));
        debug_assert!(b.debug_assert_is_valid(false));
        debug_assert!(
            (a.min_width.is_finite() && b.min_width.is_finite())
                || (a.min_width == f64::INFINITY && b.min_width == f64::INFINITY),
            "Cannot interpolate between finite constraints and unbounded constraints."
        );
        debug_assert!(
            (a.max_width.is_finite() && b.max_width.is_finite())
                || (a.max_width == f64::INFINITY && b.max_width == f64::INFINITY),
            "Cannot interpolate between finite constraints and unbounded constraints."
        );
        debug_assert!(
            (a.min_height.is_finite() && b.min_height.is_finite())
                || (a.min_height == f64::INFINITY && b.min_height == f64::INFINITY),
            "Cannot interpolate between finite constraints and unbounded constraints."
        );
        debug_assert!(
            (a.max_height.is_finite() && b.max_height.is_finite())
                || (a.max_height == f64::INFINITY && b.max_height == f64::INFINITY),
            "Cannot interpolate between finite constraints and unbounded constraints."
        );
        Some(
            BoxConstraints {
                min_width: if a.min_width.is_finite() {
                    lerp_double(Some(a.min_width), Some(b.min_width), t).unwrap()
                } else {
                    f64::INFINITY
                },
                max_width: if a.max_width.is_finite() {
                    lerp_double(Some(a.max_width), Some(b.max_width), t).unwrap()
                } else {
                    f64::INFINITY
                },
                min_height: if a.min_height.is_finite() {
                    lerp_double(Some(a.min_height), Some(b.min_height), t).unwrap()
                } else {
                    f64::INFINITY
                },
                max_height: if a.max_height.is_finite() {
                    lerp_double(Some(a.max_height), Some(b.max_height), t).unwrap()
                } else {
                    f64::INFINITY
                },
            }
            .normalize(),
        )
    }

    /// Returns a box constraints that [`is_normalized`](Self::is_normalized).
    pub fn normalize(self) -> BoxConstraints {
        if self.is_normalized() {
            return self;
        }
        let min_width = if self.min_width >= 0.0 {
            self.min_width
        } else {
            0.0
        };
        let min_height = if self.min_height >= 0.0 {
            self.min_height
        } else {
            0.0
        };
        BoxConstraints {
            min_width,
            max_width: if min_width > self.max_width {
                min_width
            } else {
                self.max_width
            },
            min_height,
            max_height: if min_height > self.max_height {
                min_height
            } else {
                self.max_height
            },
        }
    }
}

impl Default for BoxConstraints {
    fn default() -> BoxConstraints {
        BoxConstraints::new()
    }
}

impl Constraints for BoxConstraints {
    fn is_tight(&self) -> bool {
        self.has_tight_width() && self.has_tight_height()
    }

    fn is_normalized(&self) -> bool {
        self.min_width >= 0.0
            && self.min_width <= self.max_width
            && self.min_height >= 0.0
            && self.min_height <= self.max_height
    }

    fn debug_assert_is_valid(&self, is_applied_constraint: bool) -> bool {
        if cfg!(debug_assertions) {
            assert!(
                !self.min_width.is_nan()
                    && !self.max_width.is_nan()
                    && !self.min_height.is_nan()
                    && !self.max_height.is_nan(),
                "BoxConstraints has a NaN value."
            );
            assert!(
                self.min_width >= 0.0,
                "BoxConstraints has a negative minimum width."
            );
            assert!(
                self.min_height >= 0.0,
                "BoxConstraints has a negative minimum height."
            );
            assert!(
                self.max_width >= self.min_width,
                "BoxConstraints has non-normalized width constraints."
            );
            assert!(
                self.max_height >= self.min_height,
                "BoxConstraints has non-normalized height constraints."
            );
            if is_applied_constraint {
                assert!(
                    self.min_width.is_finite(),
                    "BoxConstraints forces an infinite width."
                );
                assert!(
                    self.min_height.is_finite(),
                    "BoxConstraints forces an infinite height."
                );
            }
            debug_assert!(self.is_normalized());
        }
        self.is_normalized()
    }
}

impl std::ops::Mul<f64> for BoxConstraints {
    type Output = BoxConstraints;

    fn mul(self, factor: f64) -> BoxConstraints {
        BoxConstraints {
            min_width: self.min_width * factor,
            max_width: self.max_width * factor,
            min_height: self.min_height * factor,
            max_height: self.max_height * factor,
        }
    }
}

impl std::ops::Div<f64> for BoxConstraints {
    type Output = BoxConstraints;

    fn div(self, factor: f64) -> BoxConstraints {
        BoxConstraints {
            min_width: self.min_width / factor,
            max_width: self.max_width / factor,
            min_height: self.min_height / factor,
            max_height: self.max_height / factor,
        }
    }
}

impl std::ops::Rem<f64> for BoxConstraints {
    type Output = BoxConstraints;

    fn rem(self, value: f64) -> BoxConstraints {
        BoxConstraints {
            min_width: self.min_width % value,
            max_width: self.max_width % value,
            min_height: self.min_height % value,
            max_height: self.max_height % value,
        }
    }
}

impl PartialEq for BoxConstraints {
    fn eq(&self, other: &BoxConstraints) -> bool {
        debug_assert!(self.debug_assert_is_valid(false));
        debug_assert!(other.debug_assert_is_valid(false));
        self.min_width == other.min_width
            && self.max_width == other.max_width
            && self.min_height == other.min_height
            && self.max_height == other.max_height
    }
}

impl Eq for BoxConstraints {}

impl Hash for BoxConstraints {
    fn hash<H: Hasher>(&self, state: &mut H) {
        debug_assert!(self.debug_assert_is_valid(false));
        self.min_width.to_bits().hash(state);
        self.max_width.to_bits().hash(state);
        self.min_height.to_bits().hash(state);
        self.max_height.to_bits().hash(state);
    }
}

impl Display for BoxConstraints {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let annotation = if self.is_normalized() {
            ""
        } else {
            "; NOT NORMALIZED"
        };
        if self.min_width == f64::INFINITY && self.min_height == f64::INFINITY {
            return write!(f, "BoxConstraints(biggest{annotation})");
        }
        if self.min_width == 0.0
            && self.max_width == f64::INFINITY
            && self.min_height == 0.0
            && self.max_height == f64::INFINITY
        {
            return write!(f, "BoxConstraints(unconstrained{annotation})");
        }
        fn describe(min: f64, max: f64, dim: &str) -> String {
            if min == max {
                format!("{dim}={min:.1}")
            } else {
                format!("{min:.1}<={dim}<={max:.1}")
            }
        }
        write!(
            f,
            "BoxConstraints({}, {}{annotation})",
            describe(self.min_width, self.max_width, "w"),
            describe(self.min_height, self.max_height, "h")
        )
    }
}

impl Debug for BoxConstraints {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_string_marks_biggest_unconstrained_and_tight() {
        assert!(
            BoxConstraints::expand(None, None)
                .to_string()
                .contains("biggest")
        );
        assert!(BoxConstraints::new().to_string().contains("unconstrained"));
        assert!(
            BoxConstraints::tight_for(Some(50.0), None)
                .to_string()
                .contains("w=50")
        );
    }

    #[test]
    fn copy_with_replaces_fields() {
        let constraints = BoxConstraints::new()
            .min_width(3.0)
            .max_width(7.0)
            .min_height(11.0)
            .max_height(17.0);
        let copy = constraints.copy_with();
        assert_eq!(copy, constraints);
        let copy = constraints
            .copy_with()
            .min_width(13.0)
            .max_width(17.0)
            .min_height(111.0)
            .max_height(117.0);
        assert_eq!(copy.min_width, 13.0);
        assert_eq!(copy.max_width, 17.0);
        assert_eq!(copy.min_height, 111.0);
        assert_eq!(copy.max_height, 117.0);
        assert_ne!(copy, constraints);
        let mut copy_hasher = std::collections::hash_map::DefaultHasher::new();
        let mut original_hasher = std::collections::hash_map::DefaultHasher::new();
        copy.hash(&mut copy_hasher);
        constraints.hash(&mut original_hasher);
        assert_ne!(copy_hasher.finish(), original_hasher.finish());
    }

    #[test]
    fn operators() {
        let constraints = BoxConstraints::new()
            .min_width(3.0)
            .max_width(7.0)
            .min_height(11.0)
            .max_height(17.0);
        let copy = constraints * 2.0;
        assert_eq!(copy.min_width, 6.0);
        assert_eq!(copy.max_width, 14.0);
        assert_eq!(copy.min_height, 22.0);
        assert_eq!(copy.max_height, 34.0);
        assert_eq!(copy / 2.0, constraints);
        let copy = constraints.truncating_div(2.0);
        assert_eq!(copy.min_width, 1.0);
        assert_eq!(copy.max_width, 3.0);
        assert_eq!(copy.min_height, 5.0);
        assert_eq!(copy.max_height, 8.0);
        let copy = constraints % 3.0;
        assert_eq!(copy.min_width, 0.0);
        assert_eq!(copy.max_width, 1.0);
        assert_eq!(copy.min_height, 2.0);
        assert_eq!(copy.max_height, 2.0);
    }

    #[test]
    fn lerp_from_null_and_between() {
        assert!(BoxConstraints::lerp(None, None, 0.5).is_none());
        let constraints = BoxConstraints::new()
            .min_width(3.0)
            .max_width(7.0)
            .min_height(11.0)
            .max_height(17.0);
        let copy = BoxConstraints::lerp(None, Some(constraints), 0.5).unwrap();
        assert!((copy.min_width - 1.5).abs() < 1e-9);
        assert!((copy.max_width - 3.5).abs() < 1e-9);
        assert!((copy.min_height - 5.5).abs() < 1e-9);
        assert!((copy.max_height - 8.5).abs() < 1e-9);
        let copy = BoxConstraints::lerp(Some(constraints), None, 0.5).unwrap();
        assert!((copy.min_width - 1.5).abs() < 1e-9);
        let copy = BoxConstraints::lerp(
            Some(
                BoxConstraints::new()
                    .min_width(13.0)
                    .max_width(17.0)
                    .min_height(111.0)
                    .max_height(117.0),
            ),
            Some(constraints),
            0.2,
        )
        .unwrap();
        assert!((copy.min_width - 11.0).abs() < 1e-9);
        assert!((copy.max_width - 15.0).abs() < 1e-9);
        assert!((copy.min_height - 91.0).abs() < 1e-9);
        assert!((copy.max_height - 97.0).abs() < 1e-9);
    }

    #[test]
    fn lerp_unbounded_axes() {
        let constraints1 = BoxConstraints::new()
            .min_width(f64::INFINITY)
            .min_height(10.0)
            .max_height(20.0);
        let constraints2 = BoxConstraints::new()
            .min_width(f64::INFINITY)
            .min_height(20.0)
            .max_height(30.0);
        let constraints3 = BoxConstraints::new()
            .min_width(f64::INFINITY)
            .min_height(15.0)
            .max_height(25.0);
        assert_eq!(
            BoxConstraints::lerp(Some(constraints1), Some(constraints2), 0.5),
            Some(constraints3)
        );
    }

    #[test]
    fn lerp_overshoot_is_normalized() {
        assert_eq!(
            BoxConstraints::lerp(
                Some(BoxConstraints::tight_for(None, Some(25.0))),
                Some(BoxConstraints::tight_for(None, Some(0.0))),
                1.2,
            ),
            Some(BoxConstraints::tight_for(None, Some(0.0)))
        );
        let lerped = BoxConstraints::lerp(
            Some(BoxConstraints::tight(Size::new(10.0, 10.0))),
            Some(BoxConstraints::new().max_width(100.0).max_height(100.0)),
            -0.1,
        )
        .unwrap();
        assert!(lerped.is_normalized());
        assert_eq!(lerped, BoxConstraints::tight(Size::new(11.0, 11.0)));
    }

    #[test]
    fn normalize_raises_inverted_max() {
        let constraints = BoxConstraints::new()
            .min_width(3.0)
            .max_width(2.0)
            .min_height(11.0)
            .max_height(18.0);
        let copy = constraints.normalize();
        assert_eq!(copy.min_width, 3.0);
        assert_eq!(copy.max_width, 3.0);
        assert_eq!(copy.min_height, 11.0);
        assert_eq!(copy.max_height, 18.0);
    }

    #[test]
    fn from_view_constraints() {
        let unconstrained = BoxConstraints::from_view_constraints(ViewConstraints::default());
        assert_eq!(unconstrained, BoxConstraints::new());
        let constraints =
            BoxConstraints::from_view_constraints(ViewConstraints::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(
            constraints,
            BoxConstraints::new()
                .min_width(1.0)
                .max_width(2.0)
                .min_height(3.0)
                .max_height(4.0)
        );
    }

    #[test]
    fn constrain_size_and_attempt_to_preserve_aspect_ratio_empty_size() {
        let constraints = BoxConstraints::new()
            .min_width(10.0)
            .max_width(20.0)
            .min_height(10.0)
            .max_height(20.0);
        let constrained =
            constraints.constrain_size_and_attempt_to_preserve_aspect_ratio(Size::new(15.0, 0.0));
        assert_eq!(constrained, Size::new(15.0, 10.0));
    }
}
