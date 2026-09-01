//! Flutter counterpart: `animation/tween.dart`.

use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::rc::Rc;

use reveal_embedder::{Color, Rect, Size};
use reveal_foundation::{App, Handle, Listener};

use crate::animation::{Animation, AnimationNode, AnimationStatus, AnimationStatusListener};
use crate::curves::Curve;

/// An object that can produce a value of type `T` given an `Animation<double>`
/// as input.
///
/// Typically, the values of the input animation are nominally in the range 0.0
/// to 1.0. In principle, however, any value could be provided.
///
/// The main implementor of [`Animatable`] is [`Tween`].
pub trait Animatable<T> {
    /// Returns the value of the object at point `t`.
    ///
    /// The value of `t` is nominally a fraction in the range 0.0 to 1.0,
    /// though in practice it may extend outside this range.
    fn transform(&self, app: &App, t: f64) -> T;

    /// The current value of this object for the given animation.
    ///
    /// This function is implemented by deferring to [`transform`]. Implementors
    /// that want to provide custom behavior should override [`transform`], not
    /// [`evaluate`].
    fn evaluate(&self, app: &App, animation: Animation<f64>) -> T
    where
        Self: Sized,
    {
        self.transform(app, animation.value(app))
    }

    /// Returns a new [`Animation`] that is driven by the given animation but
    /// that takes on values determined by this object.
    fn animate(self, app: &mut App, parent: Animation<f64>) -> Animation<T>
    where
        Self: Sized + Clone + 'static,
        T: 'static,
    {
        Animation::from_handle(app.create(AnimatedEvaluation {
            parent,
            evaluatable: self,
            _value: PhantomData,
        }))
    }

    /// Returns a new [`Animatable`] whose value is determined by first
    /// evaluating the given parent and then evaluating this object at the
    /// result.
    fn chain<P>(self, parent: P) -> ChainedEvaluation<P, Self>
    where
        Self: Sized + Clone,
        P: Animatable<f64> + Clone,
    {
        ChainedEvaluation {
            parent,
            evaluatable: self,
        }
    }
}

/// Dart's `_CallbackAnimatable<T>`.
#[derive(Clone)]
pub struct CallbackAnimatable<T> {
    callback: Rc<dyn Fn(f64) -> T>,
}

impl<T> CallbackAnimatable<T> {
    /// Dart's `Animatable.fromCallback`.
    pub fn from_callback(callback: impl Fn(f64) -> T + 'static) -> CallbackAnimatable<T> {
        CallbackAnimatable {
            callback: Rc::new(callback),
        }
    }
}

impl<T> Animatable<T> for CallbackAnimatable<T> {
    fn transform(&self, _app: &App, t: f64) -> T {
        (self.callback)(t)
    }
}

impl<T> Debug for CallbackAnimatable<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CallbackAnimatable")
    }
}

/// Dart's `_AnimatedEvaluation<T>` — the animation returned by
/// [`Animatable::animate`].
struct AnimatedEvaluation<T, A> {
    parent: Animation<f64>,
    evaluatable: A,
    _value: PhantomData<fn() -> T>,
}

impl<T: 'static, A: Animatable<T> + Clone + 'static> AnimationNode<T> for AnimatedEvaluation<T, A> {
    fn add_listener(app: &mut App, this: Handle<Self>, listener: Listener) {
        app.get(this).parent.add_listener(app, listener);
    }

    fn remove_listener(app: &mut App, this: Handle<Self>, listener: &Listener) {
        app.get(this).parent.remove_listener(app, listener);
    }

    fn add_status_listener(app: &mut App, this: Handle<Self>, listener: AnimationStatusListener) {
        app.get(this).parent.add_status_listener(app, listener);
    }

    fn remove_status_listener(
        app: &mut App,
        this: Handle<Self>,
        listener: &AnimationStatusListener,
    ) {
        app.get(this).parent.remove_status_listener(app, listener);
    }

    fn status(app: &App, this: Handle<Self>) -> AnimationStatus {
        app.get(this).parent.status(app)
    }

    fn value(app: &App, this: Handle<Self>) -> T {
        let parent = app.get(this).parent;
        let evaluatable = app.get(this).evaluatable.clone();
        evaluatable.evaluate(app, parent)
    }
}

/// Dart's `_ChainedEvaluation<T>`.
#[derive(Clone)]
pub struct ChainedEvaluation<P, C> {
    parent: P,
    evaluatable: C,
}

impl<P: Animatable<f64>, C: Animatable<T>, T> Animatable<T> for ChainedEvaluation<P, C> {
    fn transform(&self, app: &App, t: f64) -> T {
        self.evaluatable
            .transform(app, self.parent.transform(app, t))
    }
}

impl<P: Debug, C: Debug> Debug for ChainedEvaluation<P, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}\u{27A9}{:?}", self.parent, self.evaluatable)
    }
}

/// Types that [`Tween<T>`] can interpolate with `+`, `-`, and `*`.
pub trait TweenLerp: Clone {
    /// Dart's `Tween.lerp`.
    fn lerp(begin: &Self, end: &Self, t: f64) -> Self;
}

impl TweenLerp for f64 {
    fn lerp(begin: &f64, end: &f64, t: f64) -> f64 {
        begin + (end - begin) * t
    }
}

/// A linear interpolation between a beginning and ending value.
///
/// Tweens are mutable: [`set_begin`] / [`set_end`] after [`Animatable::animate`]
/// is visible to the driven animation, as in Dart.
///
/// [`set_begin`]: Tween::set_begin
/// [`set_end`]: Tween::set_end
pub struct Tween<T: 'static>(Handle<TweenData<T>>);

pub(crate) struct TweenData<T> {
    pub begin: Option<T>,
    pub end: Option<T>,
}

impl<T> Clone for Tween<T> {
    fn clone(&self) -> Tween<T> {
        *self
    }
}

impl<T> Copy for Tween<T> {}

impl<T: Debug> Debug for Tween<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Tween").field(&self.0).finish()
    }
}

impl<T: 'static> Tween<T> {
    /// Creates a tween.
    ///
    /// The [`begin`] and [`end`] properties must be non-null before the tween is
    /// first used, but the arguments can be null if the values are going to be
    /// filled in later.
    ///
    /// [`begin`]: Tween::begin
    /// [`end`]: Tween::end
    pub fn new(app: &mut App, begin: Option<T>, end: Option<T>) -> Tween<T> {
        Tween(app.create(TweenData { begin, end }))
    }

    /// The value this variable has at the beginning of the animation.
    pub fn begin(self, app: &App) -> Option<&T> {
        app.get(self.0).begin.as_ref()
    }

    /// Dart's `begin` setter.
    pub fn set_begin(self, app: &mut App, begin: Option<T>) {
        app.get_mut(self.0).begin = begin;
    }

    /// The value this variable has at the end of the animation.
    pub fn end(self, app: &App) -> Option<&T> {
        app.get(self.0).end.as_ref()
    }

    /// Dart's `end` setter.
    pub fn set_end(self, app: &mut App, end: Option<T>) {
        app.get_mut(self.0).end = end;
    }
}

fn tween_transform<T: Clone>(
    begin: &Option<T>,
    end: &Option<T>,
    t: f64,
    lerp: impl FnOnce(f64) -> T,
) -> T {
    if t == 0.0 {
        return begin.clone().expect("Tween.begin must be set before use");
    }
    if t == 1.0 {
        return end.clone().expect("Tween.end must be set before use");
    }
    lerp(t)
}

impl<T: TweenLerp + 'static> Tween<T> {
    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(self, app: &App, t: f64) -> T {
        let data = app.get(self.0);
        let begin = data
            .begin
            .as_ref()
            .expect("Tween.begin must be set before use");
        let end = data.end.as_ref().expect("Tween.end must be set before use");
        T::lerp(begin, end, t)
    }
}

impl<T: TweenLerp + 'static> Animatable<T> for Tween<T> {
    fn transform(&self, app: &App, t: f64) -> T {
        let data = app.get(self.0);
        tween_transform(&data.begin, &data.end, t, |t| self.lerp(app, t))
    }
}

/// A [`Tween`] that evaluates its [`parent`] in reverse.
///
/// [`parent`]: ReverseTween::parent
pub struct ReverseTween<T: 'static>(Handle<ReverseTweenData<T>>);

struct ReverseTweenData<T: 'static> {
    parent: Tween<T>,
}

impl<T> Clone for ReverseTween<T> {
    fn clone(&self) -> ReverseTween<T> {
        *self
    }
}

impl<T> Copy for ReverseTween<T> {}

impl<T: TweenLerp + 'static> ReverseTween<T> {
    /// Construct a [`Tween`] that evaluates its parent in reverse.
    pub fn new(app: &mut App, parent: Tween<T>) -> ReverseTween<T> {
        ReverseTween(app.create(ReverseTweenData { parent }))
    }

    /// This tween's value is the same as the parent's value evaluated in reverse.
    pub fn parent(self, app: &App) -> Tween<T> {
        app.get(self.0).parent
    }
}

impl<T: TweenLerp + 'static> Animatable<T> for ReverseTween<T> {
    fn transform(&self, app: &App, t: f64) -> T {
        self.parent(app).lerp(app, 1.0 - t)
    }
}

/// An interpolation between two colors.
pub struct ColorTween(Handle<TweenData<Color>>);

impl Clone for ColorTween {
    fn clone(&self) -> ColorTween {
        *self
    }
}

impl Copy for ColorTween {}

impl ColorTween {
    /// Creates a [`Color`] tween.
    ///
    /// The begin and end properties may be null; the null value is treated as
    /// transparent.
    pub fn new(app: &mut App, begin: Option<Color>, end: Option<Color>) -> ColorTween {
        ColorTween(app.create(TweenData { begin, end }))
    }

    /// The value this variable has at the beginning of the animation.
    pub fn begin(self, app: &App) -> Option<Color> {
        app.get(self.0).begin
    }

    /// Dart's `begin` setter.
    pub fn set_begin(self, app: &mut App, begin: Option<Color>) {
        app.get_mut(self.0).begin = begin;
    }

    /// The value this variable has at the end of the animation.
    pub fn end(self, app: &App) -> Option<Color> {
        app.get(self.0).end
    }

    /// Dart's `end` setter.
    pub fn set_end(self, app: &mut App, end: Option<Color>) {
        app.get_mut(self.0).end = end;
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(self, app: &App, t: f64) -> Option<Color> {
        let data = app.get(self.0);
        Color::lerp(data.begin, data.end, t)
    }
}

impl Animatable<Option<Color>> for ColorTween {
    fn transform(&self, app: &App, t: f64) -> Option<Color> {
        if t == 0.0 {
            return self.begin(app);
        }
        if t == 1.0 {
            return self.end(app);
        }
        self.lerp(app, t)
    }
}

/// An interpolation between two sizes.
pub struct SizeTween(Handle<TweenData<Size>>);

impl Clone for SizeTween {
    fn clone(&self) -> SizeTween {
        *self
    }
}

impl Copy for SizeTween {}

impl SizeTween {
    /// Creates a [`Size`] tween.
    pub fn new(app: &mut App, begin: Option<Size>, end: Option<Size>) -> SizeTween {
        SizeTween(app.create(TweenData { begin, end }))
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(self, app: &App, t: f64) -> Option<Size> {
        let data = app.get(self.0);
        Size::lerp(data.begin, data.end, t)
    }
}

impl Animatable<Option<Size>> for SizeTween {
    fn transform(&self, app: &App, t: f64) -> Option<Size> {
        let data = app.get(self.0);
        if t == 0.0 {
            return data.begin;
        }
        if t == 1.0 {
            return data.end;
        }
        self.lerp(app, t)
    }
}

/// An interpolation between two rectangles.
pub struct RectTween(Handle<TweenData<Rect>>);

impl Clone for RectTween {
    fn clone(&self) -> RectTween {
        *self
    }
}

impl Copy for RectTween {}

impl RectTween {
    /// Creates a [`Rect`] tween.
    pub fn new(app: &mut App, begin: Option<Rect>, end: Option<Rect>) -> RectTween {
        RectTween(app.create(TweenData { begin, end }))
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(self, app: &App, t: f64) -> Option<Rect> {
        let data = app.get(self.0);
        Rect::lerp(data.begin, data.end, t)
    }
}

impl Animatable<Option<Rect>> for RectTween {
    fn transform(&self, app: &App, t: f64) -> Option<Rect> {
        let data = app.get(self.0);
        if t == 0.0 {
            return data.begin;
        }
        if t == 1.0 {
            return data.end;
        }
        self.lerp(app, t)
    }
}

/// An interpolation between two integers that rounds.
pub struct IntTween(Handle<TweenData<i64>>);

impl Clone for IntTween {
    fn clone(&self) -> IntTween {
        *self
    }
}

impl Copy for IntTween {}

impl IntTween {
    /// Creates an int tween.
    pub fn new(app: &mut App, begin: Option<i64>, end: Option<i64>) -> IntTween {
        IntTween(app.create(TweenData { begin, end }))
    }

    /// Returns the interpolated integer, rounded.
    pub fn lerp(self, app: &App, t: f64) -> i64 {
        let data = app.get(self.0);
        let begin = data.begin.expect("IntTween.begin must be set before use");
        let end = data.end.expect("IntTween.end must be set before use");
        (begin as f64 + (end - begin) as f64 * t).round() as i64
    }
}

impl Animatable<i64> for IntTween {
    fn transform(&self, app: &App, t: f64) -> i64 {
        let data = app.get(self.0);
        tween_transform(&data.begin, &data.end, t, |t| self.lerp(app, t))
    }
}

/// An interpolation between two integers that floors.
pub struct StepTween(Handle<TweenData<i64>>);

impl Clone for StepTween {
    fn clone(&self) -> StepTween {
        *self
    }
}

impl Copy for StepTween {}

impl StepTween {
    /// Creates an [`i64`] tween that floors.
    pub fn new(app: &mut App, begin: Option<i64>, end: Option<i64>) -> StepTween {
        StepTween(app.create(TweenData { begin, end }))
    }

    /// Returns the interpolated integer, floored.
    pub fn lerp(self, app: &App, t: f64) -> i64 {
        let data = app.get(self.0);
        let begin = data.begin.expect("StepTween.begin must be set before use");
        let end = data.end.expect("StepTween.end must be set before use");
        (begin as f64 + (end - begin) as f64 * t).floor() as i64
    }
}

impl Animatable<i64> for StepTween {
    fn transform(&self, app: &App, t: f64) -> i64 {
        let data = app.get(self.0);
        tween_transform(&data.begin, &data.end, t, |t| self.lerp(app, t))
    }
}

/// A tween with a constant value.
pub struct ConstantTween<T: 'static>(Tween<T>);

impl<T> Clone for ConstantTween<T> {
    fn clone(&self) -> ConstantTween<T> {
        *self
    }
}

impl<T> Copy for ConstantTween<T> {}

impl<T: Clone + 'static> ConstantTween<T> {
    /// Create a tween whose begin and end values equal `value`.
    pub fn new(app: &mut App, value: T) -> ConstantTween<T> {
        ConstantTween(Tween::new(app, Some(value.clone()), Some(value)))
    }
}

impl<T: Clone + 'static> Animatable<T> for ConstantTween<T> {
    fn transform(&self, app: &App, _t: f64) -> T {
        self.0
            .begin(app)
            .cloned()
            .expect("ConstantTween.begin must be set")
    }
}

/// Transforms the value of the given animation by the given curve.
pub struct CurveTween(Handle<CurveTweenData>);

struct CurveTweenData {
    pub curve: std::rc::Rc<dyn Curve>,
}

impl Clone for CurveTween {
    fn clone(&self) -> CurveTween {
        *self
    }
}

impl Copy for CurveTween {}

impl Debug for CurveTween {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CurveTween").field(&self.0).finish()
    }
}

impl CurveTween {
    /// Creates a curve tween.
    pub fn new(app: &mut App, curve: std::rc::Rc<dyn Curve>) -> CurveTween {
        CurveTween(app.create(CurveTweenData { curve }))
    }

    /// The curve to use when transforming the value of the animation.
    pub fn curve(self, app: &App) -> std::rc::Rc<dyn Curve> {
        std::rc::Rc::clone(&app.get(self.0).curve)
    }

    /// Dart's `curve` setter.
    pub fn set_curve(self, app: &mut App, curve: std::rc::Rc<dyn Curve>) {
        app.get_mut(self.0).curve = curve;
    }
}

impl Animatable<f64> for CurveTween {
    fn transform(&self, app: &App, t: f64) -> f64 {
        if t == 0.0 || t == 1.0 {
            debug_assert_eq!(app.get(self.0).curve.transform(t).round(), t);
            return t;
        }
        app.get(self.0).curve.transform(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::Animation;
    use crate::animations::AlwaysStoppedAnimation;

    #[test]
    fn a_driven_tween_sees_end_assigned_after_animate() {
        let mut app = App::new();
        let tween = Tween::new(&mut app, Some(0.0), Some(1.0));
        let parent = Animation::from_handle(app.create(AlwaysStoppedAnimation::new(1.0)));
        let driven = tween.animate(&mut app, parent);

        assert_eq!(driven.value(&app), 1.0);
        tween.set_end(&mut app, Some(0.4));
        assert_eq!(driven.value(&app), 0.4);
    }

    #[test]
    fn lerp_walks_the_line() {
        let mut app = App::new();
        let tween = Tween::new(&mut app, Some(0.0), Some(10.0));
        assert_eq!(tween.transform(&app, 0.0), 0.0);
        assert_eq!(tween.transform(&app, 0.25), 2.5);
        assert_eq!(tween.transform(&app, 1.0), 10.0);
    }
}
