//! Flutter counterpart: `rendering/box.dart` (`BoxConstraints`, `RenderBox`
//! layout wrapper).
//!
//! `_DebugSize` / `BoxHitTestResult` / `computeDryLayout` wait.

use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};

use std::ops::{Deref, DerefMut};

use reveal_embedder::{Matrix4, Offset, Rect, Size, ViewConstraints, clamp_double, lerp_double};
use reveal_foundation::{App, HandleId};
use reveal_gestures::{HitTestEntry, HitTestResult, HitTestTarget, PointerEvent};
use reveal_painting::EdgeInsetsGeometry;
use reveal_painting::transform_point;

use crate::object::{
    AnyRenderObject, Constraints, RenderHandle, RenderObject, RenderObjectVTable, create, resolve,
};
use crate::pipeline_owner::PipelineOwner;

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

/// Method signature for hit testing a [`RenderBox`].
///
/// Used by [`BoxHitTestResult::add_with_paint_transform`] to hit test children of a
/// [`RenderBox`].
pub type BoxHitTest<'r> = dyn FnOnce(&mut BoxHitTestResult<'r>, Offset) -> bool;

/// The result of performing a hit test on [`RenderBox`]es.
///
/// A view over a [`HitTestResult`]: Dart's `BoxHitTestResult.wrap`. Dart's bare
/// `BoxHitTestResult()` constructor is `wrap` over a fresh `HitTestResult`.
pub struct BoxHitTestResult<'a>(&'a mut HitTestResult);

impl<'a> BoxHitTestResult<'a> {
    /// Wraps `result` to create a [`BoxHitTestResult`] that shares its path.
    pub fn wrap(result: &'a mut HitTestResult) -> BoxHitTestResult<'a> {
        BoxHitTestResult(result)
    }

    /// Transforms `position` to the local coordinate system of a child for hit-testing the
    /// child.
    ///
    /// The actual hit testing of the child needs to be implemented in the provided `hit_test`
    /// callback, which is invoked with the transformed `position` as argument.
    ///
    /// The provided paint `transform` (which describes the transform from the child to the
    /// parent in 3D) is processed by `PointerEvent.removePerspectiveTransform` to remove the
    /// perspective component and inverted before it is used to transform `position` from the
    /// coordinate system of the parent to the system of the child.
    ///
    /// If `transform` is `None` it will be treated as the identity transform and `position` is
    /// provided to the `hit_test` callback as-is. If `transform` cannot be inverted, the
    /// `hit_test` callback is not invoked and false is returned.
    pub fn add_with_paint_transform(
        &mut self,
        transform: Option<Matrix4>,
        position: Offset,
        hit_test: impl FnOnce(&mut BoxHitTestResult<'_>, Offset) -> bool,
    ) -> bool {
        let transform = match transform {
            Some(transform) => match transform.invert() {
                Some(inverted) => Some(inverted),
                None => return false,
            },
            None => None,
        };
        self.add_with_raw_transform(transform, position, hit_test)
    }

    /// Convenience method for hit testing children, that are translated by an [`Offset`].
    ///
    /// The actual hit testing of the child needs to be implemented in the provided `hit_test`
    /// callback, which is invoked with the transformed `position` as argument.
    pub fn add_with_paint_offset(
        &mut self,
        offset: Option<Offset>,
        position: Offset,
        hit_test: impl FnOnce(&mut BoxHitTestResult<'_>, Offset) -> bool,
    ) -> bool {
        let transformed_position = match offset {
            Some(offset) => position - offset,
            None => position,
        };
        if let Some(offset) = offset {
            self.0.push_offset(Offset::ZERO - offset);
        }
        let is_hit = hit_test(self, transformed_position);
        if offset.is_some() {
            self.0.pop_transform();
        }
        is_hit
    }

    /// Transforms `position` to the local coordinate system of a child for hit-testing the
    /// child.
    ///
    /// Unlike [`add_with_paint_transform`](Self::add_with_paint_transform), the provided
    /// `transform` is used as-is to transform `position`: it must describe the transform from
    /// the parent to the child.
    pub fn add_with_raw_transform(
        &mut self,
        transform: Option<Matrix4>,
        position: Offset,
        hit_test: impl FnOnce(&mut BoxHitTestResult<'_>, Offset) -> bool,
    ) -> bool {
        let transformed_position = match transform {
            Some(transform) => transform_point(&transform, position),
            None => position,
        };
        if let Some(transform) = transform {
            self.0.push_transform(transform);
        }
        let is_hit = hit_test(self, transformed_position);
        if transform.is_some() {
            self.0.pop_transform();
        }
        is_hit
    }

    /// Pass-through method for adding a hit test while manually managing the position
    /// transformation logic.
    ///
    /// Exactly one of `paint_offset`, `paint_transform`, or `raw_transform` must be given.
    pub fn add_with_out_of_band_position(
        &mut self,
        paint_offset: Option<Offset>,
        paint_transform: Option<Matrix4>,
        raw_transform: Option<Matrix4>,
        hit_test: impl FnOnce(&mut BoxHitTestResult<'_>) -> bool,
    ) -> bool {
        debug_assert_eq!(
            [
                paint_offset.is_some(),
                paint_transform.is_some(),
                raw_transform.is_some()
            ]
            .iter()
            .filter(|given| **given)
            .count(),
            1,
            "Exactly one transform or offset argument must be provided."
        );
        if let Some(paint_offset) = paint_offset {
            self.0.push_offset(Offset::ZERO - paint_offset);
        } else if let Some(raw_transform) = raw_transform {
            self.0.push_transform(raw_transform);
        } else {
            let paint_transform = paint_transform
                .expect("checked")
                .invert()
                .expect("paint_transform must be invertible.");
            self.0.push_transform(paint_transform);
        }
        let is_hit = hit_test(self);
        self.0.pop_transform();
        is_hit
    }
}

impl Deref for BoxHitTestResult<'_> {
    type Target = HitTestResult;

    fn deref(&self) -> &HitTestResult {
        self.0
    }
}

impl DerefMut for BoxHitTestResult<'_> {
    fn deref_mut(&mut self) -> &mut HitTestResult {
        self.0
    }
}

/// A hit test entry used by [`RenderBox`].
///
/// Dart's `BoxHitTestEntry` subclasses `HitTestEntry`; here it is the entry's target, carrying
/// the box and the position of the hit test in the local coordinates of the box. It becomes a
/// [`HitTestEntry`] with [`From`].
#[derive(Clone, Copy, Debug)]
pub struct BoxHitTestEntry {
    target: AnyRenderBox,
    local_position: Offset,
}

impl BoxHitTestEntry {
    /// Creates a box hit test entry.
    pub fn new(target: AnyRenderBox, local_position: Offset) -> BoxHitTestEntry {
        BoxHitTestEntry {
            target,
            local_position,
        }
    }

    /// The [`RenderBox`] that was hit.
    pub fn target(&self) -> AnyRenderBox {
        self.target
    }

    /// The position of the hit test in the local coordinates of [`target`](Self::target).
    pub fn local_position(&self) -> Offset {
        self.local_position
    }
}

impl HitTestTarget for BoxHitTestEntry {
    fn handle_event(&self, app: &mut App, event: &PointerEvent, _entry: &HitTestEntry) {
        self.target.handle_event(app, event, self);
    }
}

impl From<BoxHitTestEntry> for HitTestEntry {
    fn from(entry: BoxHitTestEntry) -> HitTestEntry {
        HitTestEntry::new(entry)
    }
}

/// Parent data used by [`RenderBox`] and its subclasses.
#[derive(Clone, Copy, Debug)]
pub struct BoxParentData {
    /// The offset at which to paint the child in the parent's coordinate system.
    pub offset: Offset,
}

impl BoxParentData {
    /// Creates box parent data with a zero offset.
    pub const fn new() -> BoxParentData {
        BoxParentData {
            offset: Offset::ZERO,
        }
    }
}

impl Default for BoxParentData {
    fn default() -> BoxParentData {
        BoxParentData::new()
    }
}

impl crate::object::ParentData for BoxParentData {}

impl fmt::Display for BoxParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "offset={:?}", self.offset)
    }
}

/// Flutter's `RenderBox` fields.
pub struct RenderBoxData {
    pub(crate) size: Option<Size>,
    pub(crate) constraints: Option<BoxConstraints>,
}

impl RenderBoxData {
    /// Unlaid-out box state.
    pub fn new() -> RenderBoxData {
        RenderBoxData {
            size: None,
            constraints: None,
        }
    }
}

impl Default for RenderBoxData {
    fn default() -> RenderBoxData {
        RenderBoxData::new()
    }
}

/// Implements [`RenderBox`] field accessors for a `render_box` field.
#[macro_export]
macro_rules! render_box_accessors {
    () => {
        fn render_box_data(
            self: $crate::RenderHandle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::RenderBoxData {
            &self.get(app).render_box
        }
        fn render_box_data_mut(
            self: $crate::RenderHandle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::RenderBoxData {
            &mut self.get_mut(app).render_box
        }
    };
}

/// Flutter's `RenderObjectWithChildMixin` when the child is a box.
pub trait RenderObjectWithChildMixin: RenderBox {
    /// Mixin field access.
    fn child_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &crate::object::RenderObjectWithChildData<AnyRenderBox>;

    /// See [`child_data`](Self::child_data).
    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut crate::object::RenderObjectWithChildData<AnyRenderBox>;

    /// The render object's unique child.
    fn child(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        self.child_data(app).child
    }

    /// Sets the unique child, adopting or dropping as Flutter's setter does.
    fn set_child(self: RenderHandle<Self>, app: &mut App, value: Option<AnyRenderBox>) {
        if let Some(old) = self.child(app) {
            self.drop_child(app, old.as_object());
        }
        self.child_data_mut(app).child = value;
        if let Some(new) = value {
            self.adopt_child(app, new.as_object());
        }
    }
}

/// A render object in a 2D Cartesian coordinate system.
///
/// Flutter's counterpart is `RenderBox`. [`RenderObject::perform_layout`] is the leaf override;
/// [`AnyRenderBox::layout`] is the framework wrapper. `RenderObject`'s tree operations are
/// provided here and forward to [`as_object`](Self::as_object).
pub trait RenderBox: RenderObject {
    /// Mixin field access.
    fn render_box_data(self: RenderHandle<Self>, app: &App) -> &RenderBoxData;

    /// See [`render_box_data`](Self::render_box_data).
    fn render_box_data_mut(self: RenderHandle<Self>, app: &mut App) -> &mut RenderBoxData;

    /// The box constraints most recently supplied by the parent.
    ///
    /// # Panics
    ///
    /// If layout has not yet happened.
    fn constraints(self: RenderHandle<Self>, app: &App) -> BoxConstraints {
        self.render_box_data(app).constraints.unwrap_or_else(|| {
            panic!("A RenderObject does not have any constraints before it has been laid out.")
        })
    }

    /// Installs [`BoxParentData`] unless the child already has it.
    ///
    /// The box default for [`RenderObject::setup_parent_data`]; override it here.
    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<BoxParentData>(app) {
            child.set_parent_data(app, BoxParentData::new());
        }
    }

    /// The size of this box.
    fn size(self: RenderHandle<Self>, app: &App) -> Size {
        self.as_box().size(app)
    }

    /// An estimate of the bounds within which this render object will paint: the box's own size.
    fn paint_bounds(self: RenderHandle<Self>, app: &App) -> Rect {
        Offset::ZERO & self.size(app)
    }

    /// Whether this render object has undergone layout and has a [`size`](Self::size).
    fn has_size(self: RenderHandle<Self>, app: &App) -> bool {
        self.render_box_data(app).size.is_some()
    }

    /// Override this method to handle pointer events that hit this render object.
    ///
    /// For [`RenderBox`] objects, the `entry` argument is a [`BoxHitTestEntry`]. From this
    /// object you can determine the position of the hit test in the local coordinates of the
    /// render object (via `entry.local_position()`).
    fn handle_event(
        self: RenderHandle<Self>,
        app: &mut App,
        event: &PointerEvent,
        entry: &BoxHitTestEntry,
    ) {
        let _ = (self, app, event, entry);
    }

    /// Determines the set of render objects located at the given position.
    ///
    /// Returns true, and adds any render objects that contain the point to the given hit test
    /// result, if this render object or one of its descendants absorbs the hit (preventing
    /// objects below this one from being hit). Returns false if the hit can continue to other
    /// objects below this one.
    ///
    /// The caller is responsible for transforming `position` from global coordinates to its
    /// location relative to the origin of this [`RenderBox`]. This [`RenderBox`] is responsible
    /// for checking whether the given position is within its bounds.
    ///
    /// # Panics
    ///
    /// In debug builds, if this box has not been laid out.
    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        debug_assert!(
            self.has_size(app),
            "Cannot hit test a render box that has never been laid out: {self:?}"
        );
        if self.size(app).contains(position)
            && (self.hit_test_children(app, result, position) || self.hit_test_self(app, position))
        {
            result.add(BoxHitTestEntry::new(self.as_box(), position).into());
            return true;
        }
        false
    }

    /// Override this method if this render object can be hit even if its children were not hit.
    ///
    /// Returns true if the specified `position` should be considered a hit on this render
    /// object.
    fn hit_test_self(self: RenderHandle<Self>, _app: &App, _position: Offset) -> bool {
        let _ = self;
        false
    }

    /// Override this method to check whether any children are located at the given position.
    ///
    /// Subclasses should return true if at least one child reported a hit at the specified
    /// position.
    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let _ = (self, app, result, position);
        false
    }

    /// Sets the size of this box. Call from [`RenderObject::perform_layout`] or
    /// [`RenderObject::perform_resize`].
    fn set_size(self: RenderHandle<Self>, app: &mut App, size: Size) {
        self.as_box().set_size(app, size)
    }

    /// See [`AnyRenderBox::layout`].
    fn layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        parent_uses_size: bool,
    ) {
        self.as_box().layout(app, constraints, parent_uses_size)
    }

    /// The erased `RenderBox` edge. Free: the vtable is a `const`, and the id is copied.
    fn as_box(self: RenderHandle<Self>) -> AnyRenderBox {
        AnyRenderBox {
            id: self.id(),
            vtable: const { &RenderBoxVTable::of::<Self>() },
        }
    }

    /// The erased `RenderObject` edge, through [`as_box`](Self::as_box).
    fn as_object(self: RenderHandle<Self>) -> AnyRenderObject {
        self.as_box().as_object()
    }

    /// See [`AnyRenderObject::adopt_child`].
    fn adopt_child(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        self.as_object().adopt_child(app, child)
    }

    /// See [`AnyRenderObject::drop_child`].
    fn drop_child(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        self.as_object().drop_child(app, child)
    }

    /// See [`AnyRenderObject::mark_needs_layout`].
    fn mark_needs_layout(self: RenderHandle<Self>, app: &mut App) {
        self.as_object().mark_needs_layout(app)
    }

    /// See [`AnyRenderObject::mark_needs_paint`].
    fn mark_needs_paint(self: RenderHandle<Self>, app: &mut App) {
        self.as_object().mark_needs_paint(app)
    }

    /// See [`AnyRenderObject::mark_needs_composited_layer_update`].
    fn mark_needs_composited_layer_update(self: RenderHandle<Self>, app: &mut App) {
        self.as_object().mark_needs_composited_layer_update(app)
    }

    /// See [`AnyRenderObject::schedule_initial_layout`].
    fn schedule_initial_layout(self: RenderHandle<Self>, app: &mut App) {
        self.as_object().schedule_initial_layout(app)
    }

    /// See [`AnyRenderObject::parent`].
    fn parent(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderObject> {
        self.as_object().parent(app)
    }

    /// See [`AnyRenderObject::owner`].
    fn owner(self: RenderHandle<Self>, app: &App) -> Option<PipelineOwner> {
        self.as_object().owner(app)
    }

    /// See [`AnyRenderObject::attached`].
    fn attached(self: RenderHandle<Self>, app: &App) -> bool {
        self.as_object().attached(app)
    }

    /// See [`AnyRenderObject::debug_needs_layout`].
    fn debug_needs_layout(self: RenderHandle<Self>, app: &App) -> bool {
        self.as_object().debug_needs_layout(app)
    }
}

/// The vtable of an [`AnyRenderBox`]: the object vtable plus the box accessors.
pub(crate) struct RenderBoxVTable {
    pub object: RenderObjectVTable,
    pub box_data: fn(&App, HandleId) -> &RenderBoxData,
    pub box_data_mut: fn(&mut App, HandleId) -> &mut RenderBoxData,
    pub hit_test: fn(&mut App, HandleId, &mut BoxHitTestResult<'_>, Offset) -> bool,
    pub handle_event: fn(&mut App, HandleId, &PointerEvent, &BoxHitTestEntry),
}

impl RenderBoxVTable {
    const fn of<T: RenderBox>() -> RenderBoxVTable {
        RenderBoxVTable {
            object: RenderObjectVTable::of::<T>(
                |app, id, child| <T as RenderBox>::setup_parent_data(resolve(id), app, child),
                |app, id| T::paint_bounds(resolve(id), app),
                Some(|| const { &RenderBoxVTable::of::<T>() }),
                None,
            ),
            box_data: |app, id| T::render_box_data(resolve(id), app),
            box_data_mut: |app, id| T::render_box_data_mut(resolve(id), app),
            hit_test: |app, id, result, position| T::hit_test(resolve(id), app, result, position),
            handle_event: |app, id, event, entry| T::handle_event(resolve(id), app, event, entry),
        }
    }
}

impl<T: RenderBox> RenderHandle<T> {
    /// Creates a box-protocol render object in `app`.
    pub fn new_box(app: &mut App, object: T) -> RenderHandle<T> {
        let this = create(app, object);
        // Flutter's `RenderObject()` constructor: `_wasRepaintBoundary = isRepaintBoundary`.
        let is_repaint_boundary = this.is_repaint_boundary(app);
        this.render_object_data_mut(app).was_repaint_boundary = is_repaint_boundary;
        this
    }
}

/// Erased `RenderBox`.
#[derive(Clone, Copy)]
pub struct AnyRenderBox {
    id: HandleId,
    vtable: &'static RenderBoxVTable,
}

impl PartialEq for AnyRenderBox {
    fn eq(&self, other: &AnyRenderBox) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyRenderBox {}

impl std::hash::Hash for AnyRenderBox {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Debug for AnyRenderBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyRenderBox({:?})", self.id)
    }
}

impl AnyRenderBox {
    pub(crate) fn from_vtable(id: HandleId, vtable: &'static RenderBoxVTable) -> AnyRenderBox {
        AnyRenderBox { id, vtable }
    }

    /// The `RenderObject` view of this box. Free: points into the nested table.
    pub fn as_object(self) -> AnyRenderObject {
        AnyRenderObject::from_vtable(self.id, &self.vtable.object)
    }

    /// See [`RenderBox::hit_test`].
    pub fn hit_test(
        self,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        (self.vtable.hit_test)(app, self.id, result, position)
    }

    /// See [`RenderBox::handle_event`].
    pub fn handle_event(self, app: &mut App, event: &PointerEvent, entry: &BoxHitTestEntry) {
        (self.vtable.handle_event)(app, self.id, event, entry)
    }

    fn box_data(self, app: &App) -> &RenderBoxData {
        (self.vtable.box_data)(app, self.id)
    }

    fn box_data_mut(self, app: &mut App) -> &mut RenderBoxData {
        (self.vtable.box_data_mut)(app, self.id)
    }

    /// Compute the layout for this box.
    pub fn layout(self, app: &mut App, constraints: BoxConstraints, parent_uses_size: bool) {
        debug_assert!(constraints.debug_assert_is_valid(true));
        let same = self.box_data(app).constraints == Some(constraints);
        self.as_object()
            .run_layout(app, parent_uses_size, constraints.is_tight(), same, |app| {
                self.box_data_mut(app).constraints = Some(constraints)
            });
    }

    /// The size of this box.
    ///
    /// # Panics
    ///
    /// If this box has not been laid out.
    pub fn size(self, app: &App) -> Size {
        self.box_data(app)
            .size
            .unwrap_or_else(|| panic!("RenderBox was not laid out: {self:?}"))
    }

    /// Sets the size of this box.
    pub fn set_size(self, app: &mut App, size: Size) {
        self.box_data_mut(app).size = Some(size);
    }

    /// [`BoxParentData`] stored on this child by its box parent.
    pub fn box_parent_data(self, app: &App) -> &BoxParentData {
        self.as_object().parent_data_of(app)
    }

    /// See [`AnyRenderObject::parent_data_of_mut`].
    pub fn parent_data_of_mut<P: crate::object::ParentData + 'static>(
        self,
        app: &mut App,
    ) -> &mut P {
        self.as_object().parent_data_of_mut(app)
    }

    /// See [`AnyRenderObject::parent_data_is`].
    pub fn parent_data_is<P: crate::object::ParentData + 'static>(self, app: &App) -> bool {
        self.as_object().parent_data_is::<P>(app)
    }
}

impl From<AnyRenderBox> for AnyRenderObject {
    fn from(box_: AnyRenderBox) -> AnyRenderObject {
        box_.as_object()
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

#[cfg(test)]
mod hit_test_tests {
    use super::*;

    #[derive(Debug)]
    struct DummyHitTestTarget;

    impl HitTestTarget for DummyHitTestTarget {
        fn handle_event(&self, _app: &mut App, _event: &PointerEvent, _entry: &HitTestEntry) {}
    }

    /// `box_test.dart`: `BoxHitTestResult wrapping HitTestResult`.
    #[test]
    fn wrapping_shares_the_path() {
        let transform = Matrix4::translation(40.0, 150.0);
        let mut wrapped = HitTestResult::new();
        wrapped.push_transform(transform);
        wrapped.add(HitTestEntry::new(DummyHitTestTarget));
        assert_eq!(wrapped.path().len(), 1);
        assert_eq!(wrapped.path()[0].transform(), Some(transform));

        let mut wrapping = BoxHitTestResult::wrap(&mut wrapped);
        wrapping.add(HitTestEntry::new(DummyHitTestTarget));
        assert_eq!(wrapping.path().len(), 2);
        assert_eq!(wrapping.path()[1].transform(), Some(transform));

        wrapped.add(HitTestEntry::new(DummyHitTestTarget));
        assert_eq!(wrapped.path().len(), 3);
        assert_eq!(wrapped.path()[2].transform(), Some(transform));
    }

    /// `box_test.dart`: `addWithPaintTransform`.
    #[test]
    fn add_with_paint_transform() {
        let mut base = HitTestResult::new();
        let mut result = BoxHitTestResult::wrap(&mut base);
        let mut positions = Vec::new();

        let is_hit = result.add_with_paint_transform(None, Offset::ZERO, |_, position| {
            positions.push(position);
            true
        });
        assert!(is_hit);
        assert_eq!(positions, [Offset::ZERO]);
        positions.clear();

        let is_hit = result.add_with_paint_transform(
            Some(Matrix4::translation(20.0, 30.0)),
            Offset::ZERO,
            |_, position| {
                positions.push(position);
                true
            },
        );
        assert!(is_hit);
        assert_eq!(positions, [Offset::new(-20.0, -30.0)]);
        positions.clear();

        let position = Offset::new(3.0, 4.0);
        let is_hit = result.add_with_paint_transform(None, position, |_, position| {
            positions.push(position);
            false
        });
        assert!(!is_hit);
        assert_eq!(positions, [position]);
        positions.clear();

        let is_hit = result.add_with_paint_transform(
            Some(Matrix4::translation(20.0, 30.0)),
            position,
            |_, position| {
                positions.push(position);
                true
            },
        );
        assert!(is_hit);
        assert_eq!(positions, [position - Offset::new(20.0, 30.0)]);
        positions.clear();

        // A transform that cannot be inverted.
        let is_hit = result.add_with_paint_transform(
            Some(Matrix4::scale(0.0, 0.0)),
            position,
            |_, position| {
                positions.push(position);
                true
            },
        );
        assert!(!is_hit);
        assert!(positions.is_empty());
    }

    /// `box_test.dart`: `addWithPaintOffset`.
    #[test]
    fn add_with_paint_offset() {
        let mut base = HitTestResult::new();
        let mut result = BoxHitTestResult::wrap(&mut base);
        let mut positions = Vec::new();

        let is_hit = result.add_with_paint_offset(None, Offset::ZERO, |_, position| {
            positions.push(position);
            true
        });
        assert!(is_hit);
        assert_eq!(positions, [Offset::ZERO]);
        positions.clear();

        let is_hit = result.add_with_paint_offset(
            Some(Offset::new(55.0, 32.0)),
            Offset::ZERO,
            |_, position| {
                positions.push(position);
                true
            },
        );
        assert!(is_hit);
        assert_eq!(positions, [Offset::new(-55.0, -32.0)]);
        positions.clear();

        let position = Offset::new(3.0, 4.0);
        let is_hit = result.add_with_paint_offset(
            Some(Offset::new(55.0, 32.0)),
            position,
            |result, position| {
                result.add(HitTestEntry::new(DummyHitTestTarget));
                positions.push(position);
                true
            },
        );
        assert!(is_hit);
        assert_eq!(positions, [position - Offset::new(55.0, 32.0)]);
        assert_eq!(
            result.path()[0].transform(),
            Some(Matrix4::translation(-55.0, -32.0)),
            "the entry records the transform to the child"
        );
    }

    /// `box_test.dart`: `addWithRawTransform`.
    #[test]
    fn add_with_raw_transform() {
        let mut base = HitTestResult::new();
        let mut result = BoxHitTestResult::wrap(&mut base);
        let mut positions = Vec::new();

        let is_hit = result.add_with_raw_transform(
            Some(Matrix4::translation(20.0, 30.0)),
            Offset::ZERO,
            |_, position| {
                positions.push(position);
                true
            },
        );
        assert!(is_hit);
        assert_eq!(positions, [Offset::new(20.0, 30.0)]);
        positions.clear();

        let position = Offset::new(3.0, 4.0);
        let is_hit =
            result.add_with_raw_transform(Some(Matrix4::IDENTITY), position, |_, position| {
                positions.push(position);
                true
            });
        assert!(is_hit);
        assert_eq!(positions, [position]);
    }
}
