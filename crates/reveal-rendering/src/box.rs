//! Flutter counterpart: `rendering/box.dart` (`BoxConstraints`, `RenderBox`
//! layout wrapper, intrinsics, dry layout and baselines).
//!
//! `_DebugSize` and `debugAssertDoesMeetConstraints` wait.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};

use std::cell::Cell;
use std::ops::{Add, Deref, DerefMut};

use reveal_embedder::{
    Matrix4, Offset, Rect, Size, TextBaseline, ViewConstraints, clamp_double, lerp_double,
};
use reveal_foundation::{App, Handle, HandleId};
use reveal_gestures::{HitTestEntry, HitTestResult, HitTestTarget, PointerEvent};
use reveal_painting::EdgeInsetsGeometry;
use reveal_painting::transform_point;

use crate::object::{
    AnyRenderObject, Constraints, ContainerParentDataMixin, ContainerRenderObjectMixin, ParentData,
    RenderHandle, RenderObject, RenderObjectVTable, create, debug_checking_intrinsics, resolve,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;

thread_local! {
    static DEBUG_DOING_BASELINE: Cell<bool> = const { Cell::new(false) };
}

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

    fn retained_handle(&self) -> Option<HandleId> {
        Some(self.target.id)
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

impl ParentData for BoxParentData {
    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        (id == TypeId::of::<BoxParentData>()).then_some(self)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        (id == TypeId::of::<BoxParentData>()).then_some(self)
    }
}

impl fmt::Display for BoxParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "offset={:?}", self.offset)
    }
}

/// Parent data for a [`RenderBox`] subclass that uses [`ContainerRenderObjectMixin`].
///
/// Flutter's `ContainerBoxParentData`: a convenience class that mixes
/// [`ContainerParentDataMixin`] into a [`BoxParentData`]. Here it is the pair of accessors a
/// container's parent data provides, one for each half.
pub trait ContainerBoxParentData: ContainerParentDataMixin<ChildType = AnyRenderBox> {
    /// Mixin field access: the [`BoxParentData`] half.
    ///
    /// [`ParentData::provide`](crate::ParentData::provide) must answer [`BoxParentData`] with
    /// the same value, or the box protocol cannot position this child.
    fn box_parent_data(&self) -> &BoxParentData;

    /// See [`box_parent_data`](Self::box_parent_data).
    fn box_parent_data_mut(&mut self) -> &mut BoxParentData;

    /// The offset at which to paint the child in the parent's coordinate system.
    fn offset(&self) -> Offset {
        self.box_parent_data().offset
    }

    /// Sets [`offset`](Self::offset).
    fn set_offset(&mut self, value: Offset) {
        self.box_parent_data_mut().offset = value;
    }
}

/// A wrapper that represents the baseline location of a [`RenderBox`].
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BaselineOffset(pub Option<f64>);

impl BaselineOffset {
    /// A value that indicates that the associated [`RenderBox`] does not have any baselines.
    ///
    /// [`BaselineOffset::NO_BASELINE`] is an identity element in most binary operations
    /// involving two [`BaselineOffset`]s (such as [`min_of`](Self::min_of)), for render objects
    /// with no baselines typically do not contribute to the baseline offset of their parents.
    pub const NO_BASELINE: BaselineOffset = BaselineOffset(None);

    /// The distance from the top of the box, or `None` for no baseline.
    pub fn offset(self) -> Option<f64> {
        self.0
    }

    /// Compares this [`BaselineOffset`] and `other`, and returns whichever is closer to the
    /// origin.
    ///
    /// When both `self` and `other` are [`NO_BASELINE`](Self::NO_BASELINE), this method returns
    /// [`NO_BASELINE`](Self::NO_BASELINE). When one of them is
    /// [`NO_BASELINE`](Self::NO_BASELINE), this method returns the other operand.
    pub fn min_of(self, other: BaselineOffset) -> BaselineOffset {
        match (self.0, other.0) {
            (Some(lhs), Some(rhs)) => {
                if lhs >= rhs {
                    other
                } else {
                    self
                }
            }
            (Some(_), None) => self,
            (None, _) => other,
        }
    }
}

impl Add<f64> for BaselineOffset {
    type Output = BaselineOffset;

    /// Returns a new baseline location that is `offset` pixels further away from the origin than
    /// `self`, or unchanged if `self` is [`NO_BASELINE`](Self::NO_BASELINE).
    fn add(self, offset: f64) -> BaselineOffset {
        BaselineOffset(self.0.map(|value| value + offset))
    }
}

/// Why a box cannot compute a dry layout, for
/// [`RenderBox::debug_cannot_compute_dry_layout`].
///
/// Flutter's `debugCannotComputeDryLayout` takes a `reason` or an `error`, exactly one of them.
#[derive(Clone, Copy, Debug)]
pub enum DryLayoutFailure<'a> {
    /// The box does not implement dry layout; the panic names the box and adds this reason.
    Reason(&'a str),

    /// The box cannot lay out with the given constraints; the panic is this error.
    Error(&'a str),
}

/// Intrinsic dimension calculation that computes the intrinsic width given the max height, or
/// the intrinsic height given the max width.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum IntrinsicDimension {
    MinWidth,
    MaxWidth,
    MinHeight,
    MaxHeight,
}

/// An `f64` used as a map key, as Dart uses a `double`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct DoubleKey(u64);

impl DoubleKey {
    fn new(value: f64) -> DoubleKey {
        DoubleKey(value.to_bits())
    }
}

/// Flutter's `_LayoutCacheStorage`: what one [`RenderBox`] has memoized.
///
/// The layout cache storage is typically cleared in
/// [`RenderBox::mark_needs_layout`], but is usually kept across
/// [`AnyRenderBox::layout`] calls because the incoming [`BoxConstraints`] is always an input of
/// every layout computation.
#[derive(Default)]
struct LayoutCacheStorage {
    cached_intrinsic_dimensions: HashMap<(IntrinsicDimension, DoubleKey), f64>,
    cached_dry_layout_sizes: HashMap<BoxConstraints, Size>,
    cached_alphabetic_baseline: HashMap<BoxConstraints, BaselineOffset>,
    cached_ideo_baseline: HashMap<BoxConstraints, BaselineOffset>,
}

impl LayoutCacheStorage {
    /// Empties the storage, and returns whether it had anything cached.
    fn clear(&mut self) -> bool {
        let has_cache = !self.cached_dry_layout_sizes.is_empty()
            || !self.cached_intrinsic_dimensions.is_empty()
            || !self.cached_alphabetic_baseline.is_empty()
            || !self.cached_ideo_baseline.is_empty();
        if has_cache {
            self.cached_dry_layout_sizes.clear();
            self.cached_intrinsic_dimensions.clear();
            self.cached_alphabetic_baseline.clear();
            self.cached_ideo_baseline.clear();
        }
        has_cache
    }

    fn baselines(
        &mut self,
        baseline: TextBaseline,
    ) -> &mut HashMap<BoxConstraints, BaselineOffset> {
        match baseline {
            TextBaseline::Alphabetic => &mut self.cached_alphabetic_baseline,
            TextBaseline::Ideographic => &mut self.cached_ideo_baseline,
        }
    }
}

/// Flutter's `RenderBox` fields.
pub struct RenderBoxData {
    pub(crate) size: Option<Size>,
    pub(crate) constraints: Option<BoxConstraints>,
    layout_cache: LayoutCacheStorage,
    debug_computing_this_dry_layout: bool,
    debug_computing_this_dry_baseline: bool,
}

impl RenderBoxData {
    /// Unlaid-out box state.
    pub fn new() -> RenderBoxData {
        RenderBoxData {
            size: None,
            constraints: None,
            layout_cache: LayoutCacheStorage::default(),
            debug_computing_this_dry_layout: false,
            debug_computing_this_dry_baseline: false,
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

/// Dart's `child.parentData! as BoxParentData`.
///
/// # Panics
///
/// If the child has no parent data, or its parent data is not a box's.
fn box_parent_data_of(app: &App, child: AnyRenderObject) -> &BoxParentData {
    child
        .parent_data(app)
        .and_then(|parent_data| parent_data.part::<BoxParentData>())
        .expect("parent data is not a BoxParentData")
}

/// A mixin that provides useful default behaviors for boxes with children managed by the
/// [`ContainerRenderObjectMixin`] mixin.
///
/// By convention, this trait doesn't override any members of the supertrait. Instead, it
/// provides helpful functions that render objects can call as appropriate.
///
/// Flutter's `RenderBoxContainerDefaultsMixin`.
pub trait RenderBoxContainerDefaultsMixin:
    ContainerRenderObjectMixin<ChildType = AnyRenderBox, ParentDataType: ContainerBoxParentData>
    + RenderBox
{
    /// Returns the baseline of the first child with a baseline.
    ///
    /// Useful when the children are displayed vertically in the same order they appear in the
    /// child list.
    fn default_compute_distance_to_first_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        debug_assert!(!self.as_object().debug_needs_layout(app));
        let mut child = self.first_child(app);
        while let Some(current) = child {
            let child_parent_data = current
                .as_object()
                .parent_data_of::<Self::ParentDataType>(app);
            let (offset, next_sibling) =
                (child_parent_data.offset(), child_parent_data.next_sibling());
            if let Some(result) = current.get_distance_to_actual_baseline(app, baseline) {
                return Some(result + offset.dy());
            }
            child = next_sibling;
        }
        None
    }

    /// Returns the minimum baseline value among every child.
    ///
    /// Useful when the vertical position of the children isn't determined by the order in the
    /// child list.
    fn default_compute_distance_to_highest_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        debug_assert!(!self.as_object().debug_needs_layout(app));
        let mut min_baseline = BaselineOffset::NO_BASELINE;
        let mut child = self.first_child(app);
        while let Some(current) = child {
            let child_parent_data = current
                .as_object()
                .parent_data_of::<Self::ParentDataType>(app);
            let (offset, next_sibling) =
                (child_parent_data.offset(), child_parent_data.next_sibling());
            let candidate = BaselineOffset(current.get_distance_to_actual_baseline(app, baseline))
                + offset.dy();
            min_baseline = min_baseline.min_of(candidate);
            child = next_sibling;
        }
        min_baseline.offset()
    }

    /// Performs a hit test on each child by walking the child list backwards.
    ///
    /// Stops walking once after the first child reports that it contains the given point.
    /// Returns whether any children contain the given point.
    ///
    /// See also:
    ///
    ///  * [`default_paint`](Self::default_paint), which paints the children appropriate for
    ///    this hit-testing strategy.
    fn default_hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let mut child = self.last_child(app);
        while let Some(current) = child {
            // The x, y parameters have the top left of the node's box as the origin.
            let child_parent_data = current
                .as_object()
                .parent_data_of::<Self::ParentDataType>(app);
            let (offset, previous_sibling) = (
                child_parent_data.offset(),
                child_parent_data.previous_sibling(),
            );
            let is_hit =
                result.add_with_paint_offset(Some(offset), position, |result, transformed| {
                    debug_assert_eq!(transformed, position - offset);
                    current.hit_test(app, result, transformed)
                });
            if is_hit {
                return true;
            }
            child = previous_sibling;
        }
        false
    }

    /// Paints each child by walking the child list forwards.
    ///
    /// See also:
    ///
    ///  * [`default_hit_test_children`](Self::default_hit_test_children), which implements
    ///    hit-testing of the children in a manner appropriate for this painting strategy.
    fn default_paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let mut child = self.first_child(app);
        while let Some(current) = child {
            let child_parent_data = current
                .as_object()
                .parent_data_of::<Self::ParentDataType>(app);
            let (child_offset, next_sibling) =
                (child_parent_data.offset(), child_parent_data.next_sibling());
            context.paint_child(app, current.as_object(), child_offset + offset);
            child = next_sibling;
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

    /// See [`AnyRenderBox::get_min_intrinsic_width`].
    fn get_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        self.as_box().get_min_intrinsic_width(app, height)
    }

    /// Computes the value returned by [`get_min_intrinsic_width`](Self::get_min_intrinsic_width).
    /// Do not call this function directly, instead, call
    /// [`get_min_intrinsic_width`](Self::get_min_intrinsic_width).
    ///
    /// Override in subclasses that implement [`RenderObject::perform_layout`]. This method
    /// should return the minimum width that this box could be without failing to correctly
    /// paint its contents within itself, without clipping.
    ///
    /// If the layout algorithm is independent of the context (e.g. it always tries to be a
    /// particular size), or if the layout algorithm is width-in-height-out, or if the layout
    /// algorithm uses both the incoming width and height constraints (e.g. it always sizes
    /// itself to [`BoxConstraints::biggest`]), then the `height` argument should be ignored.
    ///
    /// If the layout algorithm is strictly height-in-width-out, or is height-in-width-out when
    /// the width is unconstrained, then the height argument is the height to use.
    ///
    /// The `height` argument will never be negative. It may be infinite.
    ///
    /// If this algorithm depends on the intrinsic dimensions of a child, the intrinsic
    /// dimensions of that child should be obtained using the functions whose names start with
    /// `get_`, not `compute_`.
    ///
    /// This function should never return a negative or infinite value.
    ///
    /// # When the intrinsic dimensions cannot be known
    ///
    /// There are cases where render objects do not have an efficient way to compute their
    /// intrinsic dimensions. For example, it may be prohibitively expensive to reify and
    /// measure every child of a lazy viewport, or the dimensions may be computed by a callback
    /// about which the render object cannot reason.
    ///
    /// In such cases the intrinsic functions should panic when
    /// [`debug_checking_intrinsics`](crate::debug_checking_intrinsics) is false and debug
    /// assertions are enabled, and return 0.0 otherwise. See
    /// [`RenderViewportBase::debug_throw_if_not_checking_intrinsics`](crate::RenderViewportBase::debug_throw_if_not_checking_intrinsics).
    ///
    /// # Aspect-ratio-driven boxes
    ///
    /// Some boxes always return a fixed size based on the constraints. For these boxes, the
    /// intrinsic functions should return the appropriate size when the incoming `height` or
    /// `width` argument is finite, treating that as a tight constraint in the respective
    /// direction and treating the other direction's constraints as unbounded. When the incoming
    /// argument is not finite, then they should return the actual intrinsic dimensions based on
    /// the contents, as any other box would.
    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let _ = (self, app, height);
        0.0
    }

    /// See [`AnyRenderBox::get_max_intrinsic_width`].
    fn get_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        self.as_box().get_max_intrinsic_width(app, height)
    }

    /// Computes the value returned by [`get_max_intrinsic_width`](Self::get_max_intrinsic_width).
    /// Do not call this function directly, instead, call
    /// [`get_max_intrinsic_width`](Self::get_max_intrinsic_width).
    ///
    /// Override in subclasses that implement [`RenderObject::perform_layout`]. This should
    /// return the smallest width beyond which increasing the width never decreases the preferred
    /// height. The preferred height is the value that would be returned by
    /// [`compute_min_intrinsic_height`](Self::compute_min_intrinsic_height) for that width.
    ///
    /// If the layout algorithm is strictly height-in-width-out, or is height-in-width-out when
    /// the width is unconstrained, then this should return the same value as
    /// [`compute_min_intrinsic_width`](Self::compute_min_intrinsic_width) for the same height.
    ///
    /// Otherwise, the height argument should be ignored, and the returned value should be equal
    /// to or bigger than the value returned by
    /// [`compute_min_intrinsic_width`](Self::compute_min_intrinsic_width).
    ///
    /// The value returned by this method might not match the size that the object would actually
    /// take. For example, a box that always exactly sizes itself using
    /// [`BoxConstraints::biggest`] might well size itself bigger than its max intrinsic size.
    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let _ = (self, app, height);
        0.0
    }

    /// See [`AnyRenderBox::get_min_intrinsic_height`].
    fn get_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.as_box().get_min_intrinsic_height(app, width)
    }

    /// Computes the value returned by
    /// [`get_min_intrinsic_height`](Self::get_min_intrinsic_height). Do not call this function
    /// directly, instead, call [`get_min_intrinsic_height`](Self::get_min_intrinsic_height).
    ///
    /// Override in subclasses that implement [`RenderObject::perform_layout`]. Should return the
    /// minimum height that this box could be without failing to correctly paint its contents
    /// within itself, without clipping.
    ///
    /// If the layout algorithm is independent of the context, or if the layout algorithm is
    /// height-in-width-out, or if the layout algorithm uses both the incoming height and width
    /// constraints, then the `width` argument should be ignored.
    ///
    /// If the layout algorithm is strictly width-in-height-out, or is width-in-height-out when
    /// the height is unconstrained, then the width argument is the width to use.
    ///
    /// The `width` argument will never be negative. It may be infinite.
    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let _ = (self, app, width);
        0.0
    }

    /// See [`AnyRenderBox::get_max_intrinsic_height`].
    fn get_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.as_box().get_max_intrinsic_height(app, width)
    }

    /// Computes the value returned by
    /// [`get_max_intrinsic_height`](Self::get_max_intrinsic_height). Do not call this function
    /// directly, instead, call [`get_max_intrinsic_height`](Self::get_max_intrinsic_height).
    ///
    /// Override in subclasses that implement [`RenderObject::perform_layout`]. Should return the
    /// smallest height beyond which increasing the height never decreases the preferred width.
    /// The preferred width is the value that would be returned by
    /// [`compute_min_intrinsic_width`](Self::compute_min_intrinsic_width) for that height.
    ///
    /// If the layout algorithm is strictly width-in-height-out, or is width-in-height-out when
    /// the height is unconstrained, then this should return the same value as
    /// [`compute_min_intrinsic_height`](Self::compute_min_intrinsic_height) for the same width.
    ///
    /// Otherwise, the width argument should be ignored, and the returned value should be equal
    /// to or bigger than the value returned by
    /// [`compute_min_intrinsic_height`](Self::compute_min_intrinsic_height).
    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let _ = (self, app, width);
        0.0
    }

    /// See [`AnyRenderBox::get_dry_layout`].
    fn get_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.as_box().get_dry_layout(app, constraints)
    }

    /// Computes the value returned by [`get_dry_layout`](Self::get_dry_layout). Do not call this
    /// function directly, instead, call [`get_dry_layout`](Self::get_dry_layout).
    ///
    /// Override in subclasses that implement [`RenderObject::perform_layout`] or
    /// [`perform_resize`](Self::perform_resize), or when setting
    /// [`RenderObject::sized_by_parent`] to true without overriding
    /// [`perform_resize`](Self::perform_resize). This method should return the [`Size`] that
    /// this box would like to be given the provided [`BoxConstraints`].
    ///
    /// The size returned by this method must match the [`size`](Self::size) that the box will
    /// compute for itself in [`RenderObject::perform_layout`] (or
    /// [`perform_resize`](Self::perform_resize), if
    /// [`RenderObject::sized_by_parent`] is true).
    ///
    /// If this algorithm depends on the size of a child, the size of that child should be
    /// obtained using its [`get_dry_layout`](Self::get_dry_layout) method.
    ///
    /// # When the size cannot be known
    ///
    /// There are cases where render objects do not have an efficient way to compute their size.
    /// For example, the size may be computed by a callback about which the render object cannot
    /// reason. In such cases, the function should call
    /// [`debug_cannot_compute_dry_layout`](Self::debug_cannot_compute_dry_layout) and return
    /// [`Size::ZERO`].
    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let _ = (app, constraints);
        self.debug_cannot_compute_dry_layout(DryLayoutFailure::Reason(
            "It does not implement RenderBox::compute_dry_layout.",
        ));
        Size::ZERO
    }

    /// See [`AnyRenderBox::get_dry_baseline`].
    fn get_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        self.as_box().get_dry_baseline(app, constraints, baseline)
    }

    /// Computes the value returned by [`get_dry_baseline`](Self::get_dry_baseline).
    ///
    /// This method is for overriding only and shouldn't be called directly. To get this box's
    /// speculative baseline location for the given `constraints`, call
    /// [`get_dry_baseline`](Self::get_dry_baseline) instead.
    ///
    /// The "dry" in the method name means the implementation must not produce observable side
    /// effects when called. For example, it must not change the [`size`](Self::size) of the box,
    /// or its children's paint offsets. Moreover, accessing the current layout of this box or a
    /// child box usually indicates a bug in the implementation, as the current layout is
    /// typically calculated using a set of [`BoxConstraints`] that's different from the
    /// `constraints` given as the first parameter. To get the size of this box or a child box in
    /// this method's implementation, use [`get_dry_layout`](Self::get_dry_layout) instead.
    ///
    /// The implementation must return a value that represents the distance from the top of the
    /// box to the first baseline of the box's contents, for the given `constraints`, or `None`
    /// if the box has no baselines. It's the same exact value
    /// [`compute_distance_to_actual_baseline`](Self::compute_distance_to_actual_baseline) would
    /// return, when this box was laid out at `constraints` in the same exact state.
    ///
    /// Not all boxes support dry baseline computation. In such cases the box must call
    /// [`debug_cannot_compute_dry_layout`](Self::debug_cannot_compute_dry_layout) and return a
    /// dummy baseline offset value (such as `None`).
    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let _ = (app, constraints, baseline);
        self.debug_cannot_compute_dry_layout(DryLayoutFailure::Reason(
            "It does not implement RenderBox::compute_dry_baseline.",
        ));
        None
    }

    /// Called from [`compute_dry_layout`](Self::compute_dry_layout) or
    /// [`compute_dry_baseline`](Self::compute_dry_baseline) if this box does not support
    /// calculating a dry layout.
    ///
    /// When debug assertions are enabled and
    /// [`debug_checking_intrinsics`](crate::debug_checking_intrinsics) is not true, this method
    /// panics with the given [`DryLayoutFailure`].
    fn debug_cannot_compute_dry_layout(self: RenderHandle<Self>, failure: DryLayoutFailure<'_>) {
        let _ = self;
        if !cfg!(debug_assertions) || debug_checking_intrinsics() {
            return;
        }
        match failure {
            DryLayoutFailure::Reason(reason) => panic!(
                "The {} class does not support dry layout. {reason}",
                std::any::type_name::<Self>()
            ),
            DryLayoutFailure::Error(error) => panic!("{error}"),
        }
    }

    /// See [`AnyRenderBox::get_distance_to_baseline`].
    fn get_distance_to_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
        only_real: bool,
    ) -> Option<f64> {
        self.as_box()
            .get_distance_to_baseline(app, baseline, only_real)
    }

    /// See [`AnyRenderBox::get_distance_to_actual_baseline`].
    fn get_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        self.as_box().get_distance_to_actual_baseline(app, baseline)
    }

    /// Returns the distance from the y-coordinate of the position of the box to the y-coordinate
    /// of the first given baseline in the box's contents, if any, or `None` otherwise.
    ///
    /// Do not call this function directly. If you need to know the baseline of a child from an
    /// invocation of [`RenderObject::perform_layout`] or [`RenderObject::paint`], call
    /// [`get_distance_to_baseline`](Self::get_distance_to_baseline).
    ///
    /// Subclasses should override this method to supply the distances to their baselines. When
    /// implementing this method, there are generally three strategies:
    ///
    ///  * For classes that use the [`ContainerRenderObjectMixin`] child model, consider
    ///    implementing [`RenderBoxContainerDefaultsMixin`] and using
    ///    [`RenderBoxContainerDefaultsMixin::default_compute_distance_to_first_actual_baseline`].
    ///
    ///  * For classes that define a particular baseline themselves, return that value directly.
    ///
    ///  * For classes that have a child to which they wish to defer the computation, call
    ///    [`get_distance_to_actual_baseline`](Self::get_distance_to_actual_baseline) on the
    ///    child.
    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let _ = (self, app, baseline);
        debug_assert!(
            DEBUG_DOING_BASELINE.get(),
            "Please see the documentation for compute_distance_to_actual_baseline for the \
             required calling conventions of this method."
        );
        None
    }

    /// Mark this render object's layout information as dirty.
    ///
    /// Flutter's `RenderBox.markNeedsLayout`: it also clears the intrinsics, dry layout and
    /// baseline caches, and defers to the parent when they held anything.
    fn mark_needs_layout(self: RenderHandle<Self>, app: &mut App) {
        // If the cache was not empty, then this box's layout is used by the parent's layout
        // algorithm (it's possible that the parent only used the intrinsics for paint, but
        // there's no good way to detect that so we conservatively assume it's a layout
        // dependency).
        //
        // A render object's perform_layout implementation may depend on the baseline location or
        // the intrinsic dimensions of a descendant, even when there are relayout boundaries
        // between them.
        //
        // Some calculations may fail (dry baseline, for example). The layout dependency is still
        // established, but only from the box that failed to compute the dry baseline to the
        // ancestor that queried the dry baseline.
        if self.render_box_data_mut(app).layout_cache.clear() && self.parent(app).is_some() {
            self.as_object().mark_parent_needs_layout(app);
            return;
        }
        crate::object::RenderObjectBase::mark_needs_layout(self, app);
    }

    /// Updates the box's size using only the constraints.
    ///
    /// By default this method sets [`size`](Self::size) to the result of
    /// [`compute_dry_layout`](Self::compute_dry_layout) called with the current
    /// [`constraints`](Self::constraints). Instead of overriding this method, consider
    /// overriding [`compute_dry_layout`](Self::compute_dry_layout).
    fn perform_resize(self: RenderHandle<Self>, app: &mut App) {
        RenderBoxBase::perform_resize(self, app);
    }

    /// The size of this box.
    fn size(self: RenderHandle<Self>, app: &App) -> Size {
        self.as_box().size(app)
    }

    /// Multiply the transform from the parent's coordinate system to this box's coordinate
    /// system into the given transform.
    ///
    /// This function is used to convert coordinate systems between boxes. Subclasses that
    /// apply transforms during painting should override this function to factor those
    /// transforms into the calculation.
    ///
    /// The [`RenderBox`] implementation takes care of adjusting the matrix for the position
    /// of the given child as determined during layout and stored on the child's parent data.
    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        debug_assert!(child.parent(app).map(AnyRenderObject::id) == Some(self.id()));
        // Dart asserts the child's parent data is a `BoxParentData` with a message naming
        // this type; `box_parent_data` panics the same way.
        let offset = box_parent_data_of(app, child).offset;
        crate::object::translate(transform, offset.dx(), offset.dy());
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
        RenderBoxBase::hit_test(self, app, result, position)
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

    /// The type-erased `RenderBox` handle. Free: the vtable is a `const`, and the id is copied.
    fn as_box(self: RenderHandle<Self>) -> AnyRenderBox {
        AnyRenderBox {
            id: self.id(),
            vtable: const { &RenderBoxVTable::of::<Self>() },
        }
    }

    /// The type-erased `RenderObject` handle, through [`as_box`](Self::as_box).
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

    /// See [`AnyRenderObject::mark_needs_paint`].
    fn mark_needs_paint(self: RenderHandle<Self>, app: &mut App) {
        self.as_object().mark_needs_paint(app)
    }

    /// See [`AnyRenderObject::mark_needs_composited_layer_update`].
    fn mark_needs_composited_layer_update(self: RenderHandle<Self>, app: &mut App) {
        self.as_object().mark_needs_composited_layer_update(app)
    }

    /// See [`AnyRenderObject::mark_needs_compositing_bits_update`].
    fn mark_needs_compositing_bits_update(self: RenderHandle<Self>, app: &mut App) {
        self.as_object().mark_needs_compositing_bits_update(app)
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
    fn owner(self: RenderHandle<Self>, app: &App) -> Option<Handle<PipelineOwner>> {
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

/// Flutter's `RenderBox` bodies that an override calls through `super`.
///
/// A trait default cannot call `super`, and a leaf's [`RenderBox::hit_test`] shadows the default
/// it would call; this sibling trait, blanket-implemented for every box, carries that body (the
/// shape `ElementBase` has for `Element`).
pub(crate) trait RenderBoxBase: RenderBox {
    /// Flutter's `RenderBox.performResize`.
    fn perform_resize(self: RenderHandle<Self>, app: &mut App) {
        // Default behavior for subclasses that have sized_by_parent = true.
        let constraints = RenderBox::constraints(self, app);
        let size = RenderBox::compute_dry_layout(self, app, constraints);
        debug_assert!(size.width().is_finite() && size.height().is_finite());
        self.set_size(app, size);
    }

    /// Flutter's `RenderBox.hitTest`.
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
            && (RenderBox::hit_test_children(self, app, result, position)
                || RenderBox::hit_test_self(self, app, position))
        {
            result.add(BoxHitTestEntry::new(self.as_box(), position).into());
            return true;
        }
        false
    }
}

impl<T: RenderBox> RenderBoxBase for T {}

/// The vtable of an [`AnyRenderBox`]: the object vtable plus the box accessors.
pub(crate) struct RenderBoxVTable {
    pub object: RenderObjectVTable,
    pub box_data: fn(&App, HandleId) -> &RenderBoxData,
    pub box_data_mut: fn(&mut App, HandleId) -> &mut RenderBoxData,
    pub hit_test: fn(&mut App, HandleId, &mut BoxHitTestResult<'_>, Offset) -> bool,
    pub handle_event: fn(&mut App, HandleId, &PointerEvent, &BoxHitTestEntry),
    pub compute_min_intrinsic_width: fn(&mut App, HandleId, f64) -> f64,
    pub compute_max_intrinsic_width: fn(&mut App, HandleId, f64) -> f64,
    pub compute_min_intrinsic_height: fn(&mut App, HandleId, f64) -> f64,
    pub compute_max_intrinsic_height: fn(&mut App, HandleId, f64) -> f64,
    pub compute_dry_layout: fn(&mut App, HandleId, BoxConstraints) -> Size,
    pub compute_dry_baseline: fn(&mut App, HandleId, BoxConstraints, TextBaseline) -> Option<f64>,
    pub compute_distance_to_actual_baseline: fn(&mut App, HandleId, TextBaseline) -> Option<f64>,
}

impl RenderBoxVTable {
    const fn of<T: RenderBox>() -> RenderBoxVTable {
        RenderBoxVTable {
            object: RenderObjectVTable::of::<T>(
                |app, id, child| <T as RenderBox>::setup_parent_data(resolve(id), app, child),
                |app, id| T::paint_bounds(resolve(id), app),
                |app, id, child, transform| {
                    <T as RenderBox>::apply_paint_transform(resolve(id), app, child, transform)
                },
                |app, id| <T as RenderBox>::perform_resize(resolve(id), app),
                |app, id| <T as RenderBox>::mark_needs_layout(resolve(id), app),
                Some(|| const { &RenderBoxVTable::of::<T>() }),
                None,
            ),
            box_data: |app, id| T::render_box_data(resolve(id), app),
            box_data_mut: |app, id| T::render_box_data_mut(resolve(id), app),
            hit_test: |app, id, result, position| T::hit_test(resolve(id), app, result, position),
            handle_event: |app, id, event, entry| T::handle_event(resolve(id), app, event, entry),
            compute_min_intrinsic_width: |app, id, height| {
                T::compute_min_intrinsic_width(resolve(id), app, height)
            },
            compute_max_intrinsic_width: |app, id, height| {
                T::compute_max_intrinsic_width(resolve(id), app, height)
            },
            compute_min_intrinsic_height: |app, id, width| {
                T::compute_min_intrinsic_height(resolve(id), app, width)
            },
            compute_max_intrinsic_height: |app, id, width| {
                T::compute_max_intrinsic_height(resolve(id), app, width)
            },
            compute_dry_layout: |app, id, constraints| {
                T::compute_dry_layout(resolve(id), app, constraints)
            },
            compute_dry_baseline: |app, id, constraints, baseline| {
                T::compute_dry_baseline(resolve(id), app, constraints, baseline)
            },
            compute_distance_to_actual_baseline: |app, id, baseline| {
                T::compute_distance_to_actual_baseline(resolve(id), app, baseline)
            },
        }
    }
}

impl<T: RenderBox> RenderHandle<T> {
    /// Creates a box-protocol render object in `app`.
    pub fn new_box(app: &mut App, object: T) -> RenderHandle<T> {
        let this = create(app, object);
        let data = this.render_object_data_mut(app);
        data.object_vtable = Some(&const { RenderBoxVTable::of::<T>() }.object);
        // Flutter's `RenderObject()` constructor:
        // `_needsCompositing = isRepaintBoundary || alwaysNeedsCompositing` and
        // `_wasRepaintBoundary = isRepaintBoundary`.
        let is_repaint_boundary = this.is_repaint_boundary(app);
        let always_needs_compositing = this.always_needs_compositing(app);
        let data = this.render_object_data_mut(app);
        data.was_repaint_boundary = is_repaint_boundary;
        data.needs_compositing = is_repaint_boundary || always_needs_compositing;
        this
    }
}

/// Flutter's `RenderBox._debugSetDoingBaseline(true)` and the `finally` that clears it.
struct DebugDoingBaseline;

impl DebugDoingBaseline {
    fn enter() -> DebugDoingBaseline {
        DEBUG_DOING_BASELINE.set(cfg!(debug_assertions));
        DebugDoingBaseline
    }
}

impl Drop for DebugDoingBaseline {
    fn drop(&mut self) {
        DEBUG_DOING_BASELINE.set(false);
    }
}

/// Flutter's `RenderBox._computeIntrinsics` preamble: whether the result may be memoized.
fn should_cache_intrinsics(this: AnyRenderBox, app: &App) -> bool {
    // perform_resize should not depend on anything except the incoming constraints.
    debug_assert!(debug_checking_intrinsics() || !this.as_object().debug_doing_this_resize(app));
    // We don't want the debug-mode intrinsic tests to affect who gets marked dirty, etc.
    !cfg!(debug_assertions) || !debug_checking_intrinsics()
}

/// Flutter's `_IntrinsicDimension.memoize`.
fn memoize_intrinsic(
    this: AnyRenderBox,
    app: &mut App,
    dimension: IntrinsicDimension,
    input: f64,
    computer: fn(&mut App, HandleId, f64) -> f64,
) -> f64 {
    if !should_cache_intrinsics(this, app) {
        return computer(app, this.id, input);
    }
    let key = (dimension, DoubleKey::new(input));
    if let Some(cached) = this.layout_cache(app).cached_intrinsic_dimensions.get(&key) {
        return *cached;
    }
    let result = computer(app, this.id, input);
    this.layout_cache_mut(app)
        .cached_intrinsic_dimensions
        .insert(key, result);
    result
}

/// Flutter's `_Baseline.memoize`.
fn memoize_baseline(
    this: AnyRenderBox,
    app: &mut App,
    constraints: BoxConstraints,
    baseline: TextBaseline,
    computer: impl FnOnce(AnyRenderBox, &mut App) -> BaselineOffset,
) -> BaselineOffset {
    if !should_cache_intrinsics(this, app) {
        return computer(this, app);
    }
    if let Some(cached) = this
        .layout_cache_mut(app)
        .baselines(baseline)
        .get(&constraints)
    {
        return *cached;
    }
    let result = computer(this, app);
    this.layout_cache_mut(app)
        .baselines(baseline)
        .insert(constraints, result);
    result
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
    /// Whether this render object has undergone layout and has a size.
    pub fn has_size(self, app: &App) -> bool {
        self.box_data(app).size.is_some()
    }

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
        debug_assert!(
            !app.is_disposed(self.id),
            "layout on a disposed render object"
        );
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

    /// The box constraints most recently supplied by the parent.
    ///
    /// # Panics
    ///
    /// If layout has not yet happened.
    pub fn constraints(self, app: &App) -> BoxConstraints {
        self.box_data(app).constraints.unwrap_or_else(|| {
            panic!("A RenderObject does not have any constraints before it has been laid out.")
        })
    }

    /// Returns the minimum width that this box could be without failing to correctly paint its
    /// contents within itself, without clipping.
    ///
    /// The height argument may give a specific height to assume. The given height can be
    /// infinite, meaning that the intrinsic width in an unconstrained environment is being
    /// requested. The given height should never be negative.
    ///
    /// This function should only be called on one's children. Calling this function couples the
    /// child with the parent so that when the child's layout changes, the parent is notified
    /// (via [`RenderBox::mark_needs_layout`]).
    ///
    /// Calling this function is expensive as it can result in O(N^2) behavior.
    ///
    /// Do not override this method. Instead, implement
    /// [`RenderBox::compute_min_intrinsic_width`].
    pub fn get_min_intrinsic_width(self, app: &mut App, height: f64) -> f64 {
        debug_assert!(
            height >= 0.0,
            "The height argument to get_min_intrinsic_width was negative. If you perform \
             computations on another height before passing it to get_min_intrinsic_width, \
             consider using f64::max or clamp_double to force the value into the valid range."
        );
        memoize_intrinsic(
            self,
            app,
            IntrinsicDimension::MinWidth,
            height,
            self.vtable.compute_min_intrinsic_width,
        )
    }

    /// Returns the smallest width beyond which increasing the width never decreases the
    /// preferred height. The preferred height is the value that would be returned by
    /// [`get_min_intrinsic_height`](Self::get_min_intrinsic_height) for that width.
    ///
    /// Do not override this method. Instead, implement
    /// [`RenderBox::compute_max_intrinsic_width`].
    pub fn get_max_intrinsic_width(self, app: &mut App, height: f64) -> f64 {
        debug_assert!(
            height >= 0.0,
            "The height argument to get_max_intrinsic_width was negative."
        );
        memoize_intrinsic(
            self,
            app,
            IntrinsicDimension::MaxWidth,
            height,
            self.vtable.compute_max_intrinsic_width,
        )
    }

    /// Returns the minimum height that this box could be without failing to correctly paint its
    /// contents within itself, without clipping.
    ///
    /// Do not override this method. Instead, implement
    /// [`RenderBox::compute_min_intrinsic_height`].
    pub fn get_min_intrinsic_height(self, app: &mut App, width: f64) -> f64 {
        debug_assert!(
            width >= 0.0,
            "The width argument to get_min_intrinsic_height was negative."
        );
        memoize_intrinsic(
            self,
            app,
            IntrinsicDimension::MinHeight,
            width,
            self.vtable.compute_min_intrinsic_height,
        )
    }

    /// Returns the smallest height beyond which increasing the height never decreases the
    /// preferred width. The preferred width is the value that would be returned by
    /// [`get_min_intrinsic_width`](Self::get_min_intrinsic_width) for that height.
    ///
    /// Do not override this method. Instead, implement
    /// [`RenderBox::compute_max_intrinsic_height`].
    pub fn get_max_intrinsic_height(self, app: &mut App, width: f64) -> f64 {
        debug_assert!(
            width >= 0.0,
            "The width argument to get_max_intrinsic_height was negative."
        );
        memoize_intrinsic(
            self,
            app,
            IntrinsicDimension::MaxHeight,
            width,
            self.vtable.compute_max_intrinsic_height,
        )
    }

    /// Returns the [`Size`] that this box would like to be given the provided
    /// [`BoxConstraints`].
    ///
    /// The size returned by this method is guaranteed to be the same size that this box computes
    /// for itself during layout given the same constraints.
    ///
    /// This function should only be called on one's children. Calling this function couples the
    /// child with the parent so that when the child's layout changes, the parent is notified
    /// (via [`RenderBox::mark_needs_layout`]).
    ///
    /// This layout is called "dry" layout as opposed to the regular "wet" layout run performed
    /// by [`RenderObject::perform_layout`] because it computes the desired size for the given
    /// constraints without changing any internal state.
    ///
    /// Calling this function is expensive as it can result in O(N^2) behavior.
    ///
    /// Do not override this method. Instead, implement [`RenderBox::compute_dry_layout`].
    pub fn get_dry_layout(self, app: &mut App, constraints: BoxConstraints) -> Size {
        if !should_cache_intrinsics(self, app) {
            return self.compute_dry_layout(app, constraints);
        }
        if let Some(cached) = self
            .layout_cache(app)
            .cached_dry_layout_sizes
            .get(&constraints)
        {
            return *cached;
        }
        let result = self.compute_dry_layout(app, constraints);
        self.layout_cache_mut(app)
            .cached_dry_layout_sizes
            .insert(constraints, result);
        result
    }

    /// Flutter's `RenderBox._computeDryLayout`: the re-entrancy guard around the virtual.
    fn compute_dry_layout(self, app: &mut App, constraints: BoxConstraints) -> Size {
        if cfg!(debug_assertions) {
            assert!(!self.box_data(app).debug_computing_this_dry_layout);
            self.box_data_mut(app).debug_computing_this_dry_layout = true;
        }
        let result = (self.vtable.compute_dry_layout)(app, self.id, constraints);
        if cfg!(debug_assertions) {
            self.box_data_mut(app).debug_computing_this_dry_layout = false;
        }
        result
    }

    /// Returns the distance from the top of the box to the first baseline of the box's contents
    /// for the given `constraints`, or `None` if this box does not have any baselines.
    ///
    /// This method calls [`RenderBox::compute_dry_baseline`] under the hood and caches the
    /// result. Boxes typically don't override
    /// [`get_dry_baseline`](Self::get_dry_baseline). Instead, consider overriding
    /// [`RenderBox::compute_dry_baseline`] such that it returns a baseline location that is
    /// consistent with [`get_distance_to_actual_baseline`](Self::get_distance_to_actual_baseline).
    ///
    /// This method is usually called by the [`RenderBox::compute_dry_baseline`] or the
    /// [`RenderBox::compute_dry_layout`] implementation of a parent box to get the baseline
    /// location of a box child. Unlike
    /// [`get_distance_to_baseline`](Self::get_distance_to_baseline), this method takes a
    /// [`BoxConstraints`] as an argument and computes the baseline location as if the box was
    /// laid out by the parent using that [`BoxConstraints`].
    pub fn get_dry_baseline(
        self,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let baseline_offset = memoize_baseline(self, app, constraints, baseline, |this, app| {
            this.compute_dry_baseline(app, constraints, baseline)
        })
        .offset();
        // This assert makes sure compute_dry_baseline always gets called in debug mode, in case
        // the compute_dry_baseline implementation invokes debug_cannot_compute_dry_layout. The
        // check is skipped when debug_checking_intrinsics is true to avoid slowing down the app
        // significantly.
        if cfg!(debug_assertions) && !debug_checking_intrinsics() {
            assert_eq!(
                baseline_offset,
                (self.vtable.compute_dry_baseline)(app, self.id, constraints, baseline)
            );
        }
        baseline_offset
    }

    /// Flutter's `RenderBox._computeDryBaseline`: the re-entrancy guard around the virtual.
    fn compute_dry_baseline(
        self,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> BaselineOffset {
        if cfg!(debug_assertions) {
            assert!(!self.box_data(app).debug_computing_this_dry_baseline);
            self.box_data_mut(app).debug_computing_this_dry_baseline = true;
        }
        let result = BaselineOffset((self.vtable.compute_dry_baseline)(
            app,
            self.id,
            constraints,
            baseline,
        ));
        if cfg!(debug_assertions) {
            self.box_data_mut(app).debug_computing_this_dry_baseline = false;
        }
        result
    }

    /// Returns the distance from the y-coordinate of the position of the box to the y-coordinate
    /// of the first given baseline in the box's contents.
    ///
    /// Used by certain layout models to align adjacent boxes on a common baseline, regardless of
    /// padding, font size differences, etc. If there is no baseline, this function returns the
    /// distance from the y-coordinate of the position of the box to the y-coordinate of the
    /// bottom of the box (i.e., the height of the box) unless the caller passes true for
    /// `only_real`, in which case the function returns `None`.
    ///
    /// Only call this function after calling [`layout`](Self::layout) on this box. You are only
    /// allowed to call this from the parent of this box during that parent's
    /// [`RenderObject::perform_layout`] or [`RenderObject::paint`] functions.
    ///
    /// To override the baseline computation, override
    /// [`RenderBox::compute_distance_to_actual_baseline`].
    pub fn get_distance_to_baseline(
        self,
        app: &mut App,
        baseline: TextBaseline,
        only_real: bool,
    ) -> Option<f64> {
        debug_assert!(
            !DEBUG_DOING_BASELINE.get(),
            "Please see the documentation for compute_distance_to_actual_baseline for the \
             required calling conventions of this method."
        );
        debug_assert!(!self.as_object().debug_needs_layout(app) || debug_checking_intrinsics());
        let result = {
            let _doing_baseline = DebugDoingBaseline::enter();
            self.get_distance_to_actual_baseline(app, baseline)
        };
        match result {
            None if !only_real => Some(self.size(app).height()),
            result => result,
        }
    }

    /// Calls [`RenderBox::compute_distance_to_actual_baseline`] and caches the result.
    ///
    /// This function must only be called from
    /// [`get_distance_to_baseline`](Self::get_distance_to_baseline) and
    /// [`RenderBox::compute_distance_to_actual_baseline`].
    pub fn get_distance_to_actual_baseline(
        self,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        debug_assert!(
            DEBUG_DOING_BASELINE.get() || !cfg!(debug_assertions),
            "Please see the documentation for compute_distance_to_actual_baseline for the \
             required calling conventions of this method."
        );
        let constraints = self
            .box_data(app)
            .constraints
            .expect("a RenderBox has constraints once it has been laid out");
        memoize_baseline(self, app, constraints, baseline, |this, app| {
            BaselineOffset((this.vtable.compute_distance_to_actual_baseline)(
                app, this.id, baseline,
            ))
        })
        .offset()
    }

    fn layout_cache(self, app: &App) -> &LayoutCacheStorage {
        &self.box_data(app).layout_cache
    }

    fn layout_cache_mut(self, app: &mut App) -> &mut LayoutCacheStorage {
        &mut self.box_data_mut(app).layout_cache
    }

    /// [`BoxParentData`] stored on this child by its box parent.
    ///
    /// Dart's `child.parentData! as BoxParentData`: a subclass answers with its own
    /// [`BoxParentData`] half through [`ParentData::provide`].
    pub fn box_parent_data(self, app: &App) -> &BoxParentData {
        box_parent_data_of(app, self.as_object())
    }

    /// Convert the given point from the global coordinate system in logical pixels to the
    /// local coordinate system for this box.
    ///
    /// This method will un-project the point from the screen onto the widget, which makes it
    /// different from `MatrixUtils.transformPoint`.
    ///
    /// If the transform from global coordinates to local coordinates is degenerate, this
    /// function returns `Offset::ZERO`.
    ///
    /// If `ancestor` is non-null, this function converts the given point from the coordinate
    /// system of `ancestor` (which must be an ancestor of this render object) instead of from
    /// the global coordinate system.
    ///
    /// This method is implemented in terms of [`AnyRenderObject::get_transform_to`].
    pub fn global_to_local(
        self,
        app: &App,
        point: Offset,
        ancestor: Option<AnyRenderObject>,
    ) -> Offset {
        // We want to find the local point that corresponds to the given point on
        // the screen, but that also physically resides on this RenderBox's local
        // render plane, so that it is useful for visually accurate gesture
        // processing in the local space. For that, we cannot simply transform the
        // 2D screen point to the 3D local space since the screen space lacks the
        // depth component |z|, and so there are many 3D points that correspond to
        // the screen point. We must first unproject the screen point onto the local
        // render plane to find the true 3D point that corresponds to the screen
        // point.
        //
        // We do orthogonal unprojection after undoing perspective, in local space.
        // The local render plane is the XY plane with normal vector <0, 0, 1>.
        // Unprojection is done by finding the intersection of the view vector with
        // the local XY plane at z = 0.
        let transform = self.as_object().get_transform_to(app, ancestor);
        let Some(transform) = transform.invert() else {
            return Offset::ZERO;
        };

        // Two points with the same screen x and y but different depths define the
        // view direction in local coordinates.
        let n = [0.0, 0.0, 1.0];
        let i = crate::object::perspective_transform(&transform, [0.0, 0.0, 0.0]);
        let d = subtract(
            crate::object::perspective_transform(&transform, [0.0, 0.0, 1.0]),
            i,
        );
        let s = crate::object::perspective_transform(&transform, [point.dx(), point.dy(), 0.0]);
        // Project the screen point onto the local render plane.
        let p = subtract(s, scale(d, dot(n, s) / dot(n, d)));
        Offset::new(p[0], p[1])
    }

    /// Convert the given point from the local coordinate system for this box to the global
    /// coordinate system in logical pixels.
    ///
    /// If `ancestor` is non-null, this function converts the given point to the coordinate
    /// system of `ancestor` (which must be an ancestor of this render object) instead of to
    /// the global coordinate system.
    ///
    /// This method is implemented in terms of [`AnyRenderObject::get_transform_to`]. If the
    /// transform matrix puts the given `point` on the line at infinity (for instance, when
    /// the transform matrix is the zero matrix), this method returns (NaN, NaN).
    pub fn local_to_global(
        self,
        app: &App,
        point: Offset,
        ancestor: Option<AnyRenderObject>,
    ) -> Offset {
        reveal_painting::transform_point(&self.as_object().get_transform_to(app, ancestor), point)
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

impl crate::object::ErasedRenderObject for AnyRenderBox {
    fn as_object(self) -> AnyRenderObject {
        AnyRenderBox::as_object(self)
    }

    fn from_object(object: AnyRenderObject) -> AnyRenderBox {
        object
            .as_box()
            .expect("the render object is not a RenderBox")
    }
}

impl From<AnyRenderBox> for AnyRenderObject {
    fn from(box_: AnyRenderBox) -> AnyRenderObject {
        box_.as_object()
    }
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f64; 3], factor: f64) -> [f64; 3] {
    [a[0] * factor, a[1] * factor, a[2] * factor]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
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

#[cfg(test)]
mod intrinsics_tests {
    use reveal_foundation::AppCell;
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::object::RenderObjectData;

    /// A leaf whose minimum intrinsic width is `100.0 + height`, counting how often it is
    /// actually computed.
    struct CountingBox {
        render_object: RenderObjectData,
        render_box: RenderBoxData,
        computations: Rc<Cell<u32>>,
    }

    impl RenderObject for CountingBox {
        crate::render_object_accessors!();

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            let size = self.constraints(app).smallest();
            self.set_size(app, size);
        }
    }

    impl RenderBox for CountingBox {
        crate::render_box_accessors!();

        fn compute_min_intrinsic_width(
            self: RenderHandle<Self>,
            app: &mut App,
            height: f64,
        ) -> f64 {
            let computations = self.get(app).computations.clone();
            computations.set(computations.get() + 1);
            100.0 + height
        }
    }

    /// `box_test.dart`: `Intrinsics cache' and `Intrinsics cache is cleared when the render
    /// object is marked as needing layout`.
    #[test]
    fn the_intrinsic_cache_is_reused_and_cleared_by_mark_needs_layout() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let computations = Rc::new(Cell::new(0));
        let box_ = RenderHandle::new_box(
            &mut app,
            CountingBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                computations: computations.clone(),
            },
        );

        assert_eq!(box_.as_box().get_min_intrinsic_width(&mut app, 0.0), 100.0);
        assert_eq!(computations.get(), 1);

        assert_eq!(box_.as_box().get_min_intrinsic_width(&mut app, 0.0), 100.0);
        assert_eq!(computations.get(), 1, "the memoized value is reused");

        assert_eq!(box_.as_box().get_min_intrinsic_width(&mut app, 10.0), 110.0);
        assert_eq!(computations.get(), 2, "another height is another entry");

        box_.mark_needs_layout(&mut app);
        assert_eq!(box_.as_box().get_min_intrinsic_width(&mut app, 0.0), 100.0);
        assert_eq!(computations.get(), 3, "mark_needs_layout cleared the cache");
    }

    /// Flutter's `RenderBox.markNeedsLayout`: a box whose intrinsics the parent read defers to
    /// the parent, because the parent's layout depends on them.
    #[test]
    fn clearing_a_cached_intrinsic_marks_the_parent() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let computations = Rc::new(Cell::new(0));
        let child = RenderHandle::new_box(
            &mut app,
            CountingBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                computations,
            },
        );
        let parent = crate::proxy_box::RenderConstrainedBox::new(
            &mut app,
            BoxConstraints::new(),
            Some(child.as_box()),
        );
        parent.layout(&mut app, BoxConstraints::new().max_width(50.0), false);
        assert!(!parent.debug_needs_layout(&app));

        child.as_box().get_min_intrinsic_width(&mut app, 0.0);
        child.mark_needs_layout(&mut app);
        assert!(
            parent.debug_needs_layout(&app),
            "the parent read an intrinsic, so it has to lay out again"
        );
    }
}
