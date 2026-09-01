//! Flutter counterpart: `animation/curves.dart`.

use std::any::Any;
use std::f64::consts::PI;
use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_embedder::{Offset, clamp_double, lerp_double};

/// An parametric animation easing curve, i.e. a mapping of the unit interval to
/// the unit interval.
///
/// Easing curves are used to adjust the rate of change of an animation over
/// time, allowing them to speed up and slow down, rather than moving at a
/// constant rate.
///
/// A [`Curve`] must map t=0.0 to 0.0 and t=1.0 to 1.0.
///
/// See also:
///
///  * [`Curves`], a collection of common animation easing curves.
///  * `CurveTween`, which can be used to apply a [`Curve`] to an `Animation`.
///  * `Animatable`, for a more flexible interface that maps fractions to
///    arbitrary values.
///
/// # Implementing a curve
///
/// Override [`transform_internal`](Curve::transform_internal). Its default
/// panics, matching the `UnimplementedError` Dart throws, because a curve may
/// instead override [`transform`](Curve::transform) whole — [`Split`] does.
pub trait Curve: Any + Debug {
    /// Returns the value of the curve at point `t`.
    ///
    /// This function must ensure the following:
    /// - The value of `t` must be between 0.0 and 1.0
    /// - Values of `t`=0.0 and `t`=1.0 must be mapped to 0.0 and 1.0,
    ///   respectively.
    ///
    /// It is recommended that implementors override
    /// [`transform_internal`](Curve::transform_internal) instead of this
    /// function, as the above cases are already handled in the default
    /// implementation of [`transform`](Curve::transform), which delegates the
    /// remaining logic to [`transform_internal`](Curve::transform_internal).
    fn transform(&self, t: f64) -> f64 {
        if t == 0.0 || t == 1.0 {
            return t;
        }
        debug_assert!(
            (0.0..=1.0).contains(&t),
            "parametric value {t} is outside of [0, 1] range."
        );
        self.transform_internal(t)
    }

    /// Returns the value of the curve at point `t`.
    ///
    /// The given parametric value `t` will be between 0.0 and 1.0, inclusive.
    fn transform_internal(&self, _t: f64) -> f64 {
        unimplemented!()
    }
}

impl dyn Curve {
    /// Returns a new curve that is the reversed inversion of this one.
    ///
    /// This is often useful with `CurvedAnimation.reverseCurve`.
    ///
    /// See also:
    ///
    ///  * [`FlippedCurve`], the class that is used to implement this getter.
    ///  * `ReverseAnimation`, which reverses an `Animation` rather than a
    ///    [`Curve`].
    ///  * `CurvedAnimation`, which can take a separate curve and reverse curve.
    pub fn flipped(self: Rc<dyn Curve>) -> FlippedCurve {
        FlippedCurve::new(self)
    }
}

/// The identity map over the unit interval.
///
/// See [`Curves::linear`] for an instance of this class.
#[derive(Debug)]
struct Linear;

impl Curve for Linear {
    fn transform_internal(&self, t: f64) -> f64 {
        t
    }
}

/// A sawtooth curve that repeats a given number of times over the unit interval.
///
/// The curve rises linearly from 0.0 to 1.0 and then falls discontinuously back
/// to 0.0 each iteration.
pub struct SawTooth {
    count: i64,
}

impl SawTooth {
    /// Creates a sawtooth curve.
    pub const fn new(count: i64) -> SawTooth {
        SawTooth { count }
    }

    /// The number of repetitions of the sawtooth pattern in the unit interval.
    pub const fn count(&self) -> i64 {
        self.count
    }
}

impl Curve for SawTooth {
    fn transform_internal(&self, t: f64) -> f64 {
        let t = t * self.count as f64;
        t - t.trunc()
    }
}

impl Debug for SawTooth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SawTooth({})", self.count)
    }
}

/// A curve that is 0.0 until [`begin`](Interval::begin), then curved (according
/// to [`curve`](Interval::curve)) from 0.0 at [`begin`](Interval::begin) to 1.0
/// at [`end`](Interval::end), then remains 1.0 past [`end`](Interval::end).
///
/// An [`Interval`] can be used to delay an animation. For example, a six second
/// animation that uses an [`Interval`] with its [`begin`](Interval::begin) set
/// to 0.5 and its [`end`](Interval::end) set to 1.0 will essentially become a
/// three-second animation that starts three seconds later.
pub struct Interval {
    begin: f64,
    end: f64,
    curve: Rc<dyn Curve>,
}

impl Interval {
    /// Creates an interval curve.
    ///
    /// Dart defaults `curve` to [`Curves::linear`].
    pub fn new(begin: f64, end: f64, curve: Rc<dyn Curve>) -> Interval {
        Interval { begin, end, curve }
    }

    /// The largest value for which this interval is 0.0.
    ///
    /// From t=0.0 to t=[`begin`](Interval::begin), the interval's value is 0.0.
    pub const fn begin(&self) -> f64 {
        self.begin
    }

    /// The smallest value for which this interval is 1.0.
    ///
    /// From t=[`end`](Interval::end) to t=1.0, the interval's value is 1.0.
    pub const fn end(&self) -> f64 {
        self.end
    }

    /// The curve to apply between [`begin`](Interval::begin) and
    /// [`end`](Interval::end).
    pub fn curve(&self) -> &Rc<dyn Curve> {
        &self.curve
    }
}

impl Curve for Interval {
    fn transform_internal(&self, t: f64) -> f64 {
        debug_assert!(self.begin >= 0.0);
        debug_assert!(self.begin <= 1.0);
        debug_assert!(self.end >= 0.0);
        debug_assert!(self.end <= 1.0);
        debug_assert!(self.end >= self.begin);
        let t = clamp_double((t - self.begin) / (self.end - self.begin), 0.0, 1.0);
        if t == 0.0 || t == 1.0 {
            return t;
        }
        self.curve.transform(t)
    }
}

impl Debug for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if (&*self.curve as &dyn Any)
            .downcast_ref::<Linear>()
            .is_none()
        {
            return write!(
                f,
                "Interval({:?}\u{22ef}{:?})\u{27a9}{:?}",
                self.begin, self.end, self.curve
            );
        }
        write!(f, "Interval({:?}\u{22ef}{:?})", self.begin, self.end)
    }
}

/// A curve that progresses according to [`begin_curve`](Split::begin_curve)
/// until [`split`](Split::split), then according to
/// [`end_curve`](Split::end_curve).
///
/// Split curves are useful in situations where a widget must track the user's
/// finger (which requires a linear animation), but can also be flung using a
/// curve specified with the [`end_curve`](Split::end_curve) argument, after the
/// finger is released. In such a case, the value of [`split`](Split::split)
/// would be the progress of the animation at the time when the finger was
/// released.
///
/// For example, if [`split`](Split::split) is set to 0.5,
/// [`begin_curve`](Split::begin_curve) is [`Curves::linear`], and
/// [`end_curve`](Split::end_curve) is [`Curves::ease_out_cubic`], then the
/// bottom-left quarter of the curve will be a straight line, and the top-right
/// quarter will contain the entire [`Curves::ease_out_cubic`] curve.
pub struct Split {
    split: f64,
    begin_curve: Rc<dyn Curve>,
    end_curve: Rc<dyn Curve>,
}

impl Split {
    /// Creates a split curve.
    ///
    /// Dart defaults `begin_curve` to [`Curves::linear`] and `end_curve` to
    /// [`Curves::ease_out_cubic`].
    pub fn new(split: f64, begin_curve: Rc<dyn Curve>, end_curve: Rc<dyn Curve>) -> Split {
        Split {
            split,
            begin_curve,
            end_curve,
        }
    }

    /// The progress value separating [`begin_curve`](Split::begin_curve) from
    /// [`end_curve`](Split::end_curve).
    ///
    /// The value before which the curve progresses according to
    /// [`begin_curve`](Split::begin_curve) and after which the curve progresses
    /// according to [`end_curve`](Split::end_curve).
    ///
    /// When t is exactly `split`, the curve has the value `split`.
    ///
    /// Must be between 0 and 1.0, inclusively.
    pub const fn split(&self) -> f64 {
        self.split
    }

    /// The curve to use before [`split`](Split::split) is reached.
    pub fn begin_curve(&self) -> &Rc<dyn Curve> {
        &self.begin_curve
    }

    /// The curve to use after [`split`](Split::split) is reached.
    pub fn end_curve(&self) -> &Rc<dyn Curve> {
        &self.end_curve
    }
}

impl Curve for Split {
    fn transform(&self, t: f64) -> f64 {
        debug_assert!((0.0..=1.0).contains(&t));
        debug_assert!((0.0..=1.0).contains(&self.split));

        if t == 0.0 || t == 1.0 {
            return t;
        }

        if t == self.split {
            return self.split;
        }

        if t < self.split {
            let curve_progress = t / self.split;
            let transformed = self.begin_curve.transform(curve_progress);
            lerp_double(Some(0.0), Some(self.split), transformed).unwrap()
        } else {
            let curve_progress = (t - self.split) / (1.0 - self.split);
            let transformed = self.end_curve.transform(curve_progress);
            lerp_double(Some(self.split), Some(1.0), transformed).unwrap()
        }
    }
}

impl Debug for Split {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Dart leads with `describeIdentity`, which appends an identity hash we
        // have no counterpart for: see `PORTING.md`.
        write!(
            f,
            "Split({:?}, {:?}, {:?})",
            self.split, self.begin_curve, self.end_curve
        )
    }
}

/// A curve that is 0.0 until it hits the threshold, then it jumps to 1.0.
pub struct Threshold {
    threshold: f64,
}

impl Threshold {
    /// Creates a threshold curve.
    pub const fn new(threshold: f64) -> Threshold {
        Threshold { threshold }
    }

    /// The value before which the curve is 0.0 and after which the curve is 1.0.
    ///
    /// When t is exactly [`threshold`](Threshold::threshold), the curve has the
    /// value 1.0.
    pub const fn threshold(&self) -> f64 {
        self.threshold
    }
}

impl Curve for Threshold {
    fn transform_internal(&self, t: f64) -> f64 {
        debug_assert!(self.threshold >= 0.0);
        debug_assert!(self.threshold <= 1.0);
        if t < self.threshold { 0.0 } else { 1.0 }
    }
}

impl Debug for Threshold {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Dart declares no `toString`, so it inherits `ParametricCurve`'s, which
        // prints the runtime type alone and never the threshold.
        f.write_str("Threshold")
    }
}

/// A cubic polynomial mapping of the unit interval.
///
/// The [`Curves`] class contains some commonly used cubic curves, such as
/// [`Curves::ease`], [`Curves::ease_in`], [`Curves::ease_out`] and
/// [`Curves::ease_in_out`].
///
/// The [`Cubic`] class implements third-order Bézier curves.
///
/// See also:
///
///  * [`Curves`], where many more predefined curves are available.
///  * `CatmullRomCurve`, a curve which passes through specific values.
pub struct Cubic {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
}

impl Cubic {
    /// Creates a cubic curve.
    ///
    /// Rather than creating a new instance, consider using one of the common
    /// cubic curves in [`Curves`].
    pub const fn new(a: f64, b: f64, c: f64, d: f64) -> Cubic {
        Cubic { a, b, c, d }
    }

    /// The x coordinate of the first control point.
    ///
    /// The line through the point (0, 0) and the first control point is tangent
    /// to the curve at the point (0, 0).
    pub const fn a(&self) -> f64 {
        self.a
    }

    /// The y coordinate of the first control point.
    ///
    /// The line through the point (0, 0) and the first control point is tangent
    /// to the curve at the point (0, 0).
    pub const fn b(&self) -> f64 {
        self.b
    }

    /// The x coordinate of the second control point.
    ///
    /// The line through the point (1, 1) and the second control point is tangent
    /// to the curve at the point (1, 1).
    pub const fn c(&self) -> f64 {
        self.c
    }

    /// The y coordinate of the second control point.
    ///
    /// The line through the point (1, 1) and the second control point is tangent
    /// to the curve at the point (1, 1).
    pub const fn d(&self) -> f64 {
        self.d
    }

    const CUBIC_ERROR_BOUND: f64 = 0.001;

    fn evaluate_cubic(a: f64, b: f64, m: f64) -> f64 {
        3.0 * a * (1.0 - m) * (1.0 - m) * m + 3.0 * b * (1.0 - m) * m * m + m * m * m
    }
}

impl Curve for Cubic {
    fn transform_internal(&self, t: f64) -> f64 {
        assert!(!t.is_nan(), "t: must not be NaN");
        if t <= 0.0 {
            return 0.0;
        }
        if t >= 1.0 {
            return 1.0;
        }
        let mut start = 0.0;
        let mut end = 1.0;
        loop {
            let midpoint = (start + end) / 2.0;
            let estimate = Cubic::evaluate_cubic(self.a, self.c, midpoint);
            if (t - estimate).abs() < Cubic::CUBIC_ERROR_BOUND {
                return Cubic::evaluate_cubic(self.b, self.d, midpoint);
            }
            if estimate < t {
                start = midpoint;
            } else {
                end = midpoint;
            }
        }
    }
}

impl Debug for Cubic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Cubic({:.2}, {:.2}, {:.2}, {:.2})",
            self.a, self.b, self.c, self.d
        )
    }
}

/// A cubic polynomial composed of two curves that share a common center point.
///
/// The curve runs through three points: (0,0), the
/// [`midpoint`](ThreePointCubic::midpoint), and (1,1).
///
/// The [`Curves`] class contains a curve defined with this class:
/// [`Curves::ease_in_out_cubic_emphasized`].
///
/// The [`ThreePointCubic`] class implements third-order Bézier curves, where two
/// curves share an interior [`midpoint`](ThreePointCubic::midpoint) that the
/// curve passes through. If the control points surrounding the middle point
/// ([`b1`](ThreePointCubic::b1), and [`a2`](ThreePointCubic::a2)) are not
/// collinear with the middle point, then the curve's derivative will have a
/// discontinuity (a cusp) at the shared middle point.
///
/// See also:
///
///  * [`Curves`], where many more predefined curves are available.
///  * [`Cubic`], which defines a single cubic polynomial.
///  * `CatmullRomCurve`, a curve which passes through specific values.
pub struct ThreePointCubic {
    a1: Offset,
    b1: Offset,
    midpoint: Offset,
    a2: Offset,
    b2: Offset,
}

impl ThreePointCubic {
    /// Creates two cubic curves that share a common control point.
    ///
    /// Rather than creating a new instance, consider using one of the common
    /// three-point cubic curves in [`Curves`].
    ///
    /// The arguments correspond to the control points for the two curves,
    /// including the [`midpoint`](ThreePointCubic::midpoint), but do not include
    /// the two implied end points at (0,0) and (1,1), which are fixed.
    pub const fn new(
        a1: Offset,
        b1: Offset,
        midpoint: Offset,
        a2: Offset,
        b2: Offset,
    ) -> ThreePointCubic {
        ThreePointCubic {
            a1,
            b1,
            midpoint,
            a2,
            b2,
        }
    }

    /// The coordinates of the first control point of the first curve.
    ///
    /// The line through the point (0, 0) and this control point is tangent to
    /// the curve at the point (0, 0).
    pub const fn a1(&self) -> Offset {
        self.a1
    }

    /// The coordinates of the second control point of the first curve.
    ///
    /// The line through the [`midpoint`](ThreePointCubic::midpoint) and this
    /// control point is tangent to the curve approaching the
    /// [`midpoint`](ThreePointCubic::midpoint).
    pub const fn b1(&self) -> Offset {
        self.b1
    }

    /// The coordinates of the middle shared point.
    ///
    /// The curve will go through this point. If the control points surrounding
    /// this middle point ([`b1`](ThreePointCubic::b1), and
    /// [`a2`](ThreePointCubic::a2)) are not colinear with this point, then the
    /// curve's derivative will have a discontinuity (a cusp) at this point.
    pub const fn midpoint(&self) -> Offset {
        self.midpoint
    }

    /// The coordinates of the first control point of the second curve.
    ///
    /// The line through the [`midpoint`](ThreePointCubic::midpoint) and this
    /// control point is tangent to the curve approaching the
    /// [`midpoint`](ThreePointCubic::midpoint).
    pub const fn a2(&self) -> Offset {
        self.a2
    }

    /// The coordinates of the second control point of the second curve.
    ///
    /// The line through the point (1, 1) and this control point is tangent to
    /// the curve at (1, 1).
    pub const fn b2(&self) -> Offset {
        self.b2
    }
}

impl Curve for ThreePointCubic {
    fn transform_internal(&self, t: f64) -> f64 {
        let first_curve = t < self.midpoint.dx();
        let scale_x = if first_curve {
            self.midpoint.dx()
        } else {
            1.0 - self.midpoint.dx()
        };
        let scale_y = if first_curve {
            self.midpoint.dy()
        } else {
            1.0 - self.midpoint.dy()
        };
        let scaled_t = (t - if first_curve { 0.0 } else { self.midpoint.dx() }) / scale_x;
        if first_curve {
            Cubic::new(
                self.a1.dx() / scale_x,
                self.a1.dy() / scale_y,
                self.b1.dx() / scale_x,
                self.b1.dy() / scale_y,
            )
            .transform(scaled_t)
                * scale_y
        } else {
            Cubic::new(
                (self.a2.dx() - self.midpoint.dx()) / scale_x,
                (self.a2.dy() - self.midpoint.dy()) / scale_y,
                (self.b2.dx() - self.midpoint.dx()) / scale_x,
                (self.b2.dy() - self.midpoint.dy()) / scale_y,
            )
            .transform(scaled_t)
                * scale_y
                + self.midpoint.dy()
        }
    }
}

impl Debug for ThreePointCubic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `curves.dart:522` passes the whole interpolated string as
        // `objectRuntimeType`'s release fallback rather than as the result, so
        // the control points never reach a debug build's output — only the
        // runtime type does, followed by the trailing space in the Dart.
        f.write_str("ThreePointCubic ")
    }
}

/// A curve that is the reversed inversion of its given curve.
///
/// This curve evaluates the given curve in reverse (i.e. from 1.0 to 0.0 as t
/// increases from 0.0 to 1.0) and returns the inverse of the given curve's
/// value (i.e. 1.0 minus the given curve's value).
pub struct FlippedCurve {
    curve: Rc<dyn Curve>,
}

impl FlippedCurve {
    /// Creates a flipped curve.
    pub fn new(curve: Rc<dyn Curve>) -> FlippedCurve {
        FlippedCurve { curve }
    }

    /// The curve that is being flipped.
    pub fn curve(&self) -> &Rc<dyn Curve> {
        &self.curve
    }
}

impl Curve for FlippedCurve {
    fn transform_internal(&self, t: f64) -> f64 {
        1.0 - self.curve.transform(1.0 - t)
    }
}

impl Debug for FlippedCurve {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FlippedCurve({:?})", self.curve)
    }
}

/// A curve where the rate of change starts out quickly and then decelerates; an
/// upside-down `f(t) = t²` parabola.
///
/// This is equivalent to the Android `DecelerateInterpolator` class with a unit
/// factor (the default factor).
///
/// See [`Curves::decelerate`] for an instance of this class.
#[derive(Debug)]
struct DecelerateCurve;

impl Curve for DecelerateCurve {
    fn transform_internal(&self, t: f64) -> f64 {
        // Intended to match the behavior of:
        // https://android.googlesource.com/platform/frameworks/base/+/main/core/java/android/view/animation/DecelerateInterpolator.java
        // ...as of December 2016.
        let t = 1.0 - t;
        1.0 - t * t
    }
}

// BOUNCE CURVES

fn bounce(t: f64) -> f64 {
    if t < 1.0 / 2.75 {
        return 7.5625 * t * t;
    } else if t < 2.0 / 2.75 {
        let t = t - 1.5 / 2.75;
        return 7.5625 * t * t + 0.75;
    } else if t < 2.5 / 2.75 {
        let t = t - 2.25 / 2.75;
        return 7.5625 * t * t + 0.9375;
    }
    let t = t - 2.625 / 2.75;
    7.5625 * t * t + 0.984375
}

/// An oscillating curve that grows in magnitude.
///
/// See [`Curves::bounce_in`] for an instance of this class.
#[derive(Debug)]
struct BounceInCurve;

impl Curve for BounceInCurve {
    fn transform_internal(&self, t: f64) -> f64 {
        1.0 - bounce(1.0 - t)
    }
}

/// An oscillating curve that shrink in magnitude.
///
/// See [`Curves::bounce_out`] for an instance of this class.
#[derive(Debug)]
struct BounceOutCurve;

impl Curve for BounceOutCurve {
    fn transform_internal(&self, t: f64) -> f64 {
        bounce(t)
    }
}

/// An oscillating curve that first grows and then shrink in magnitude.
///
/// See [`Curves::bounce_in_out`] for an instance of this class.
#[derive(Debug)]
struct BounceInOutCurve;

impl Curve for BounceInOutCurve {
    fn transform_internal(&self, t: f64) -> f64 {
        if t < 0.5 {
            (1.0 - bounce(1.0 - t * 2.0)) * 0.5
        } else {
            bounce(t * 2.0 - 1.0) * 0.5 + 0.5
        }
    }
}

// ELASTIC CURVES

/// An oscillating curve that grows in magnitude while overshooting its bounds.
///
/// An instance of this class using the default period of 0.4 is available as
/// [`Curves::elastic_in`].
pub struct ElasticInCurve {
    period: f64,
}

impl ElasticInCurve {
    /// Creates an elastic-in curve.
    ///
    /// Rather than creating a new instance, consider using
    /// [`Curves::elastic_in`]. Dart defaults `period` to 0.4.
    pub const fn new(period: f64) -> ElasticInCurve {
        ElasticInCurve { period }
    }

    /// The duration of the oscillation.
    pub const fn period(&self) -> f64 {
        self.period
    }
}

impl Curve for ElasticInCurve {
    fn transform_internal(&self, t: f64) -> f64 {
        let s = self.period / 4.0;
        let t = t - 1.0;
        -(2.0_f64.powf(10.0 * t)) * ((t - s) * (PI * 2.0) / self.period).sin()
    }
}

impl Debug for ElasticInCurve {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ElasticInCurve({:?})", self.period)
    }
}

/// An oscillating curve that shrinks in magnitude while overshooting its bounds.
///
/// An instance of this class using the default period of 0.4 is available as
/// [`Curves::elastic_out`].
pub struct ElasticOutCurve {
    period: f64,
}

impl ElasticOutCurve {
    /// Creates an elastic-out curve.
    ///
    /// Rather than creating a new instance, consider using
    /// [`Curves::elastic_out`]. Dart defaults `period` to 0.4.
    pub const fn new(period: f64) -> ElasticOutCurve {
        ElasticOutCurve { period }
    }

    /// The duration of the oscillation.
    pub const fn period(&self) -> f64 {
        self.period
    }
}

impl Curve for ElasticOutCurve {
    fn transform_internal(&self, t: f64) -> f64 {
        let s = self.period / 4.0;
        2.0_f64.powf(-10.0 * t) * ((t - s) * (PI * 2.0) / self.period).sin() + 1.0
    }
}

impl Debug for ElasticOutCurve {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ElasticOutCurve({:?})", self.period)
    }
}

/// An oscillating curve that grows and then shrinks in magnitude while
/// overshooting its bounds.
///
/// An instance of this class using the default period of 0.4 is available as
/// [`Curves::elastic_in_out`].
pub struct ElasticInOutCurve {
    period: f64,
}

impl ElasticInOutCurve {
    /// Creates an elastic-in-out curve.
    ///
    /// Rather than creating a new instance, consider using
    /// [`Curves::elastic_in_out`]. Dart defaults `period` to 0.4.
    pub const fn new(period: f64) -> ElasticInOutCurve {
        ElasticInOutCurve { period }
    }

    /// The duration of the oscillation.
    pub const fn period(&self) -> f64 {
        self.period
    }
}

impl Curve for ElasticInOutCurve {
    fn transform_internal(&self, t: f64) -> f64 {
        let s = self.period / 4.0;
        let t = 2.0 * t - 1.0;
        if t < 0.0 {
            -0.5 * 2.0_f64.powf(10.0 * t) * ((t - s) * (PI * 2.0) / self.period).sin()
        } else {
            2.0_f64.powf(-10.0 * t) * ((t - s) * (PI * 2.0) / self.period).sin() * 0.5 + 1.0
        }
    }
}

impl Debug for ElasticInOutCurve {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ElasticInOutCurve({:?})", self.period)
    }
}

// PREDEFINED CURVES

/// A collection of common animation curves.
///
/// See also:
///
///  * [`Curve`], the interface implemented by the curves available from the
///    [`Curves`] class.
///
/// # Why these are functions
///
/// Dart's are `static const` fields, which the language canonicalizes: every
/// mention of `Curves.easeIn` is the *same* object, which is what makes
/// Flutter's `widget.curve != oldWidget.curve` checks answer "unchanged".
/// Rust has no const canonicalization, so each accessor hands out a clone of one
/// process-local [`Rc`] and [`Rc::ptr_eq`] reproduces that comparison.
pub enum Curves {}

macro_rules! predefined_curves {
    ($( $(#[$doc:meta])* $name:ident = $value:expr; )*) => {
        impl Curves {
            $(
                $(#[$doc])*
                pub fn $name() -> Rc<dyn Curve> {
                    thread_local! {
                        static CURVE: Rc<dyn Curve> = Rc::new($value);
                    }
                    CURVE.with(Rc::clone)
                }
            )*
        }
    };
}

predefined_curves! {
    /// A linear animation curve.
    ///
    /// This is the identity map over the unit interval: its
    /// [`Curve::transform`] method returns its input unmodified. This is useful
    /// as a default curve for cases where a [`Curve`] is required but no actual
    /// curve is desired.
    linear = Linear;

    /// A curve where the rate of change starts out quickly and then
    /// decelerates; an upside-down `f(t) = t²` parabola.
    ///
    /// This is equivalent to the Android `DecelerateInterpolator` class with a
    /// unit factor (the default factor).
    decelerate = DecelerateCurve;

    /// A curve that is very steep and linear at the beginning, but quickly
    /// flattens out and very slowly eases in.
    ///
    /// By default is the curve used to animate pages on iOS back to their
    /// original position if a swipe gesture is ended midway through a swipe.
    fast_linear_to_slow_ease_in = Cubic::new(0.18, 1.0, 0.04, 1.0);

    /// A curve that starts slowly, speeds up very quickly, and then ends slowly.
    ///
    /// This curve is used by default to animate page transitions used by
    /// `CupertinoPageRoute`.
    ///
    /// It has been derived from plots of native iOS 16.3 animation frames on
    /// iPhone 14 Pro Max. Specifically, transition animation positions were
    /// measured every frame and plotted against time. Then, a cubic curve was
    /// strictly fit to the measured data points.
    fast_ease_in_to_slow_ease_out = ThreePointCubic::new(
        Offset::new(0.056, 0.024),
        Offset::new(0.108, 0.3085),
        Offset::new(0.198, 0.541),
        Offset::new(0.3655, 1.0),
        Offset::new(0.5465, 0.989),
    );

    /// A cubic animation curve that speeds up quickly and ends slowly.
    ///
    /// This is the same as the CSS easing function `ease`.
    ease = Cubic::new(0.25, 0.1, 0.25, 1.0);

    /// A cubic animation curve that starts slowly and ends quickly.
    ///
    /// This is the same as the CSS easing function `ease-in`.
    ease_in = Cubic::new(0.42, 0.0, 1.0, 1.0);

    /// A cubic animation curve that starts slowly and ends linearly.
    ///
    /// The symmetric animation to [`Curves::linear_to_ease_out`].
    ease_in_to_linear = Cubic::new(0.67, 0.03, 0.65, 0.09);

    /// A cubic animation curve that starts slowly and ends quickly. This is
    /// similar to [`Curves::ease_in`], but with sinusoidal easing for a slightly
    /// less abrupt beginning and end. Nonetheless, the result is quite gentle
    /// and is hard to distinguish from [`Curves::linear`] at a glance.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_in_sine = Cubic::new(0.47, 0.0, 0.745, 0.715);

    /// A cubic animation curve that starts slowly and ends quickly. Based on a
    /// quadratic equation where `f(t) = t²`, this is effectively the inverse of
    /// [`Curves::decelerate`].
    ///
    /// Compared to [`Curves::ease_in_sine`], this curve is slightly steeper.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_in_quad = Cubic::new(0.55, 0.085, 0.68, 0.53);

    /// A cubic animation curve that starts slowly and ends quickly. This curve
    /// is based on a cubic equation where `f(t) = t³`. The result is a safe
    /// sweet spot when choosing a curve for widgets animating off the viewport.
    ///
    /// Compared to [`Curves::ease_in_quad`], this curve is slightly steeper.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_in_cubic = Cubic::new(0.55, 0.055, 0.675, 0.19);

    /// A cubic animation curve that starts slowly and ends quickly. This curve
    /// is based on a quartic equation where `f(t) = t⁴`.
    ///
    /// Animations using this curve or steeper curves will benefit from a longer
    /// duration to avoid motion feeling unnatural.
    ///
    /// Compared to [`Curves::ease_in_cubic`], this curve is slightly steeper.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_in_quart = Cubic::new(0.895, 0.03, 0.685, 0.22);

    /// A cubic animation curve that starts slowly and ends quickly. This curve
    /// is based on a quintic equation where `f(t) = t⁵`.
    ///
    /// Compared to [`Curves::ease_in_quart`], this curve is slightly steeper.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_in_quint = Cubic::new(0.755, 0.05, 0.855, 0.06);

    /// A cubic animation curve that starts slowly and ends quickly. This curve
    /// is based on an exponential equation where `f(t) = 2¹⁰⁽ᵗ⁻¹⁾`.
    ///
    /// Using this curve can give your animations extra flare, but a longer
    /// duration may need to be used to compensate for the steepness of the
    /// curve.
    ///
    /// Compared to [`Curves::ease_in_quint`], this curve is slightly steeper.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_in_expo = Cubic::new(0.95, 0.05, 0.795, 0.035);

    /// A cubic animation curve that starts slowly and ends quickly. This curve
    /// is effectively the bottom-right quarter of a circle.
    ///
    /// Like [`Curves::ease_in_expo`], this curve is fairly dramatic and will
    /// reduce the clarity of an animation if not given a longer duration.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_in_circ = Cubic::new(0.6, 0.04, 0.98, 0.335);

    /// A cubic animation curve that starts slowly and ends quickly. This curve
    /// is similar to `Curves.elasticIn` in that it overshoots its bounds before
    /// reaching its end. Instead of repeated swinging motions before ascending,
    /// though, this curve overshoots once, then continues to ascend.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_in_back = Cubic::new(0.6, -0.28, 0.735, 0.045);

    /// A cubic animation curve that starts quickly and ends slowly.
    ///
    /// This is the same as the CSS easing function `ease-out`.
    ease_out = Cubic::new(0.0, 0.0, 0.58, 1.0);

    /// A cubic animation curve that starts linearly and ends slowly.
    ///
    /// A symmetric animation to [`Curves::ease_in_to_linear`].
    linear_to_ease_out = Cubic::new(0.35, 0.91, 0.33, 0.97);

    /// A cubic animation curve that starts quickly and ends slowly. This is
    /// similar to [`Curves::ease_out`], but with sinusoidal easing for a
    /// slightly less abrupt beginning and end. Nonetheless, the result is quite
    /// gentle and is hard to distinguish from [`Curves::linear`] at a glance.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_out_sine = Cubic::new(0.39, 0.575, 0.565, 1.0);

    /// A cubic animation curve that starts quickly and ends slowly. This is
    /// effectively the same as [`Curves::decelerate`], only simulated using a
    /// cubic bezier function.
    ///
    /// Compared to [`Curves::ease_out_sine`], this curve is slightly steeper.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_out_quad = Cubic::new(0.25, 0.46, 0.45, 0.94);

    /// A cubic animation curve that starts quickly and ends slowly. This curve
    /// is a flipped version of [`Curves::ease_in_cubic`].
    ///
    /// The result is a safe sweet spot when choosing a curve for animating a
    /// widget's position entering or already inside the viewport.
    ///
    /// Compared to [`Curves::ease_out_quad`], this curve is slightly steeper.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_out_cubic = Cubic::new(0.215, 0.61, 0.355, 1.0);

    /// A cubic animation curve that starts quickly and ends slowly. This curve
    /// is a flipped version of [`Curves::ease_in_quart`].
    ///
    /// Animations using this curve or steeper curves will benefit from a longer
    /// duration to avoid motion feeling unnatural.
    ///
    /// Compared to [`Curves::ease_out_cubic`], this curve is slightly steeper.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_out_quart = Cubic::new(0.165, 0.84, 0.44, 1.0);

    /// A cubic animation curve that starts quickly and ends slowly. This curve
    /// is a flipped version of [`Curves::ease_in_quint`].
    ///
    /// Compared to [`Curves::ease_out_quart`], this curve is slightly steeper.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_out_quint = Cubic::new(0.23, 1.0, 0.32, 1.0);

    /// A cubic animation curve that starts quickly and ends slowly. This curve
    /// is a flipped version of [`Curves::ease_in_expo`]. Using this curve can
    /// give your animations extra flare, but a longer duration may need to be
    /// used to compensate for the steepness of the curve.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_out_expo = Cubic::new(0.19, 1.0, 0.22, 1.0);

    /// A cubic animation curve that starts quickly and ends slowly. This curve
    /// is effectively the top-left quarter of a circle.
    ///
    /// Like [`Curves::ease_out_expo`], this curve is fairly dramatic and will
    /// reduce the clarity of an animation if not given a longer duration.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_out_circ = Cubic::new(0.075, 0.82, 0.165, 1.0);

    /// A cubic animation curve that starts quickly and ends slowly. This curve
    /// is similar to `Curves.elasticOut` in that it overshoots its bounds before
    /// reaching its end. Instead of repeated swinging motions after ascending,
    /// though, this curve only overshoots once.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_out_back = Cubic::new(0.175, 0.885, 0.32, 1.275);

    /// A cubic animation curve that starts slowly, speeds up, and then ends
    /// slowly.
    ///
    /// This is the same as the CSS easing function `ease-in-out`.
    ease_in_out = Cubic::new(0.42, 0.0, 0.58, 1.0);

    /// A cubic animation curve that starts slowly, speeds up, and then ends
    /// slowly. This is similar to [`Curves::ease_in_out`], but with sinusoidal
    /// easing for a slightly less abrupt beginning and end.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_in_out_sine = Cubic::new(0.445, 0.05, 0.55, 0.95);

    /// A cubic animation curve that starts slowly, speeds up, and then ends
    /// slowly. This curve can be imagined as [`Curves::ease_in_quad`] as the
    /// first half, and [`Curves::ease_out_quad`] as the second.
    ///
    /// Compared to [`Curves::ease_in_out_sine`], this curve is slightly steeper.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_in_out_quad = Cubic::new(0.455, 0.03, 0.515, 0.955);

    /// A cubic animation curve that starts slowly, speeds up, and then ends
    /// slowly. This curve can be imagined as [`Curves::ease_in_cubic`] as the
    /// first half, and [`Curves::ease_out_cubic`] as the second.
    ///
    /// The result is a safe sweet spot when choosing a curve for a widget whose
    /// initial and final positions are both within the viewport.
    ///
    /// Compared to [`Curves::ease_in_out_quad`], this curve is slightly steeper.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_in_out_cubic = Cubic::new(0.645, 0.045, 0.355, 1.0);

    /// A cubic animation curve that starts slowly, speeds up shortly
    /// thereafter, and then ends slowly. This curve can be imagined as a steeper
    /// version of [`Curves::ease_in_out_cubic`].
    ///
    /// The result is a more emphasized eased curve when choosing a curve for a
    /// widget whose initial and final positions are both within the viewport.
    ///
    /// Compared to [`Curves::ease_in_out_cubic`], this curve is slightly
    /// steeper.
    ease_in_out_cubic_emphasized = ThreePointCubic::new(
        Offset::new(0.05, 0.0),
        Offset::new(0.133333, 0.06),
        Offset::new(0.166666, 0.4),
        Offset::new(0.208333, 0.82),
        Offset::new(0.25, 1.0),
    );

    /// A cubic animation curve that starts slowly, speeds up, and then ends
    /// slowly. This curve can be imagined as [`Curves::ease_in_quart`] as the
    /// first half, and [`Curves::ease_out_quart`] as the second.
    ///
    /// Animations using this curve or steeper curves will benefit from a longer
    /// duration to avoid motion feeling unnatural.
    ///
    /// Compared to [`Curves::ease_in_out_cubic`], this curve is slightly
    /// steeper.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_in_out_quart = Cubic::new(0.77, 0.0, 0.175, 1.0);

    /// A cubic animation curve that starts slowly, speeds up, and then ends
    /// slowly. This curve can be imagined as [`Curves::ease_in_quint`] as the
    /// first half, and [`Curves::ease_out_quint`] as the second.
    ///
    /// Compared to [`Curves::ease_in_out_quart`], this curve is slightly
    /// steeper.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_in_out_quint = Cubic::new(0.86, 0.0, 0.07, 1.0);

    /// A cubic animation curve that starts slowly, speeds up, and then ends
    /// slowly.
    ///
    /// Since this curve is arrived at with an exponential function, the midpoint
    /// is exceptionally steep. Extra consideration should be taken when
    /// designing an animation using this.
    ///
    /// Compared to [`Curves::ease_in_out_quint`], this curve is slightly
    /// steeper.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_in_out_expo = Cubic::new(1.0, 0.0, 0.0, 1.0);

    /// A cubic animation curve that starts slowly, speeds up, and then ends
    /// slowly. This curve can be imagined as [`Curves::ease_in_circ`] as the
    /// first half, and [`Curves::ease_out_circ`] as the second.
    ///
    /// Like [`Curves::ease_in_out_expo`], this curve is fairly dramatic and will
    /// reduce the clarity of an animation if not given a longer duration.
    ///
    /// Compared to [`Curves::ease_in_out_expo`], this curve is slightly steeper.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_in_out_circ = Cubic::new(0.785, 0.135, 0.15, 0.86);

    /// A cubic animation curve that starts slowly, speeds up, and then ends
    /// slowly. This curve can be imagined as [`Curves::ease_in_back`] as the
    /// first half, and [`Curves::ease_out_back`] as the second.
    ///
    /// Since two curves are used as a basis for this curve, the resulting
    /// animation will overshoot its bounds twice before reaching its end - first
    /// by exceeding its lower bound, then exceeding its upper bound and finally
    /// descending to its final position.
    ///
    /// Derived from Robert Penner’s easing functions.
    ease_in_out_back = Cubic::new(0.68, -0.55, 0.265, 1.55);

    /// A curve that starts quickly and eases into its final position.
    ///
    /// Over the course of the animation, the object spends more time near its
    /// final destination. As a result, the user isn’t left waiting for the
    /// animation to finish, and the negative effects of motion are minimized.
    ///
    /// See also:
    ///
    ///  * `Easing.legacy`, the name for this curve in the Material
    ///    specification.
    fast_out_slow_in = Cubic::new(0.4, 0.0, 0.2, 1.0);

    /// A cubic animation curve that starts quickly, slows down, and then ends
    /// quickly.
    slow_middle = Cubic::new(0.15, 0.85, 0.85, 0.15);

    /// An oscillating curve that grows in magnitude.
    bounce_in = BounceInCurve;

    /// An oscillating curve that first grows and then shrink in magnitude.
    bounce_out = BounceOutCurve;

    /// An oscillating curve that first grows and then shrink in magnitude.
    bounce_in_out = BounceInOutCurve;

    /// An oscillating curve that grows in magnitude while overshooting its
    /// bounds.
    elastic_in = ElasticInCurve::new(0.4);

    /// An oscillating curve that shrinks in magnitude while overshooting its
    /// bounds.
    elastic_out = ElasticOutCurve::new(0.4);

    /// An oscillating curve that grows and then shrinks in magnitude while
    /// overshooting its bounds.
    elastic_in_out = ElasticInOutCurve::new(0.4);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flutter's `curves_test.dart` helper of the same name.
    fn assert_maps_zero_to_zero_and_one_to_one(curve: &dyn Curve) {
        assert_eq!(curve.transform(0.0), 0.0, "{curve:?}");
        assert_eq!(curve.transform(1.0), 1.0, "{curve:?}");
    }

    #[test]
    fn every_predefined_curve_maps_the_endpoints_to_themselves() {
        for curve in [
            Curves::linear(),
            Curves::decelerate(),
            Curves::fast_linear_to_slow_ease_in(),
            Curves::fast_ease_in_to_slow_ease_out(),
            Curves::ease(),
            Curves::ease_in(),
            Curves::ease_in_to_linear(),
            Curves::ease_in_sine(),
            Curves::ease_in_quad(),
            Curves::ease_in_cubic(),
            Curves::ease_in_quart(),
            Curves::ease_in_quint(),
            Curves::ease_in_expo(),
            Curves::ease_in_circ(),
            Curves::ease_in_back(),
            Curves::ease_out(),
            Curves::linear_to_ease_out(),
            Curves::ease_out_sine(),
            Curves::ease_out_quad(),
            Curves::ease_out_cubic(),
            Curves::ease_out_quart(),
            Curves::ease_out_quint(),
            Curves::ease_out_expo(),
            Curves::ease_out_circ(),
            Curves::ease_out_back(),
            Curves::ease_in_out(),
            Curves::ease_in_out_sine(),
            Curves::ease_in_out_quad(),
            Curves::ease_in_out_cubic(),
            Curves::ease_in_out_cubic_emphasized(),
            Curves::ease_in_out_quart(),
            Curves::ease_in_out_quint(),
            Curves::ease_in_out_expo(),
            Curves::ease_in_out_circ(),
            Curves::ease_in_out_back(),
            Curves::fast_out_slow_in(),
            Curves::slow_middle(),
            Curves::bounce_in(),
            Curves::bounce_out(),
            Curves::bounce_in_out(),
            Curves::elastic_in(),
            Curves::elastic_out(),
            Curves::elastic_in_out(),
        ] {
            assert_maps_zero_to_zero_and_one_to_one(&*curve);
        }
    }

    #[test]
    fn a_predefined_curve_is_one_object_so_two_reads_compare_equal() {
        assert!(Rc::ptr_eq(&Curves::ease_in(), &Curves::ease_in()));
        assert!(!Rc::ptr_eq(&Curves::ease_in(), &Curves::ease_out()));
        // What a freshly built curve gives, matching Dart's non-const case.
        assert!(!Rc::ptr_eq(
            &(Rc::new(Cubic::new(0.42, 0.0, 1.0, 1.0)) as Rc<dyn Curve>),
            &Curves::ease_in()
        ));
    }

    #[test]
    fn linear_returns_its_input() {
        let linear = Curves::linear();
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert_eq!(linear.transform(t), t);
        }
    }

    #[test]
    fn a_sawtooth_repeats_and_drops_back_to_zero() {
        let curve = SawTooth::new(3);
        assert_eq!(curve.transform(0.0), 0.0);
        assert!((curve.transform(0.5 / 3.0) - 0.5).abs() < 1e-9);
        assert!(curve.transform(0.999 / 3.0) > 0.99);
        assert!(curve.transform(1.001 / 3.0) < 0.01);
        assert_eq!(curve.transform(1.0), 1.0);
    }

    #[test]
    fn an_interval_is_flat_outside_its_bounds() {
        let curve = Interval::new(0.25, 0.75, Curves::linear());
        assert_eq!(curve.transform(0.0), 0.0);
        assert_eq!(curve.transform(0.25), 0.0);
        assert_eq!(curve.transform(0.5), 0.5);
        assert_eq!(curve.transform(0.75), 1.0);
        assert_eq!(curve.transform(1.0), 1.0);
    }

    #[test]
    fn a_split_runs_each_half_over_its_own_curve() {
        let curve = Split::new(0.5, Curves::linear(), Curves::linear());
        assert_eq!(curve.transform(0.0), 0.0);
        assert_eq!(curve.transform(0.25), 0.25);
        assert_eq!(curve.transform(0.5), 0.5);
        assert_eq!(curve.transform(0.75), 0.75);
        assert_eq!(curve.transform(1.0), 1.0);
    }

    #[test]
    fn a_threshold_jumps_once() {
        let curve = Threshold::new(0.5);
        assert_eq!(curve.transform(0.0), 0.0);
        assert_eq!(curve.transform(0.25), 0.0);
        assert_eq!(curve.transform(0.5), 1.0);
        assert_eq!(curve.transform(0.75), 1.0);
        assert_eq!(curve.transform(1.0), 1.0);
    }

    #[test]
    fn a_flipped_curve_reverses_and_inverts_its_curve() {
        let decelerate = Curves::decelerate();
        let flipped = FlippedCurve::new(Rc::clone(&decelerate));
        for t in [0.1, 0.3, 0.5, 0.7, 0.9] {
            let expected = 1.0 - decelerate.transform(1.0 - t);
            assert!((flipped.transform(t) - expected).abs() < 1e-12);
        }
    }

    #[test]
    fn flipped_on_an_erased_curve_wraps_it() {
        let flipped = Curves::decelerate().flipped();
        assert!(
            (flipped.transform(0.25) - (1.0 - Curves::decelerate().transform(0.75))).abs() < 1e-12
        );
    }

    #[test]
    fn a_cubic_stays_inside_the_unit_interval() {
        let curve = Curves::ease_in_out();
        let mut previous = 0.0;
        for step in 0..=100 {
            let value = curve.transform(step as f64 / 100.0);
            assert!((0.0..=1.0).contains(&value), "{value} at step {step}");
            assert!(value >= previous, "{value} < {previous} at step {step}");
            previous = value;
        }
    }

    #[test]
    fn a_three_point_cubic_passes_through_its_midpoint() {
        let curve = ThreePointCubic::new(
            Offset::new(0.05, 0.0),
            Offset::new(0.133333, 0.06),
            Offset::new(0.166666, 0.4),
            Offset::new(0.208333, 0.82),
            Offset::new(0.25, 1.0),
        );
        assert!((curve.transform(0.166666) - 0.4).abs() < 0.01);
    }

    #[test]
    fn the_elastic_curves_overshoot_between_the_endpoints() {
        // One period is 0.4, so the first swing past the bound is a quarter of
        // that either side of the end it starts from.
        assert!(Curves::elastic_out().transform(0.15) > 1.0);
        assert!(Curves::elastic_in().transform(0.85) < 0.0);
    }

    #[test]
    fn debug_matches_flutter_to_string() {
        assert_eq!(format!("{:?}", SawTooth::new(3)), "SawTooth(3)");
        assert_eq!(
            format!("{:?}", Cubic::new(0.42, 0.0, 1.0, 1.0)),
            "Cubic(0.42, 0.00, 1.00, 1.00)"
        );
        assert_eq!(
            format!("{:?}", Interval::new(0.0, 0.5, Curves::linear())),
            "Interval(0.0\u{22ef}0.5)"
        );
        assert_eq!(
            format!("{:?}", Interval::new(0.0, 0.5, Curves::ease())),
            "Interval(0.0\u{22ef}0.5)\u{27a9}Cubic(0.25, 0.10, 0.25, 1.00)"
        );
        assert_eq!(
            format!("{:?}", ElasticInCurve::new(0.4)),
            "ElasticInCurve(0.4)"
        );
        // Dart's is `_Linear`; the skill drops the underscore from the name.
        assert_eq!(
            format!("{:?}", FlippedCurve::new(Curves::linear())),
            "FlippedCurve(Linear)"
        );
        // Neither prints its fields, because neither Dart class declares a
        // `toString` that reaches them.
        assert_eq!(format!("{:?}", Threshold::new(0.5)), "Threshold");
        assert_eq!(
            format!("{:?}", Curves::ease_in_out_cubic_emphasized()),
            "ThreePointCubic "
        );
    }
}
