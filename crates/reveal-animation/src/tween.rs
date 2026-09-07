//! Flutter counterpart: `animation/tween.dart`.

use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::rc::Rc;

use reveal_embedder::{Color, Offset, Rect, Size};
use reveal_foundation::{App, Handle, Listener, RetainedHandle};

use crate::animation::{Animation, AnimationStatus, AnimationStatusListener, AnyAnimation};
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
    ///
    /// [`transform`]: Animatable::transform
    /// [`evaluate`]: Animatable::evaluate
    fn evaluate(&self, app: &App, animation: AnyAnimation<f64>) -> T
    where
        Self: Sized,
    {
        self.transform(app, animation.value(app))
    }

    /// Returns a new [`AnyAnimation`] that is driven by the given animation but
    /// that takes on values determined by this object.
    fn animate(self, app: &mut App, parent: AnyAnimation<f64>) -> AnyAnimation<T>
    where
        Self: Sized + Clone + 'static,
        T: 'static,
    {
        let parent = app.retain(parent);
        app.create(AnimatedEvaluation {
            parent,
            evaluatable: self,
            _value: PhantomData,
        })
        .as_animation()
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
    parent: RetainedHandle<AnyAnimation<f64>>,
    evaluatable: A,
    _value: PhantomData<fn() -> T>,
}

impl<T: 'static, A: Animatable<T> + Clone + 'static> Animation<T> for AnimatedEvaluation<T, A> {
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        app.get(self).parent.add_listener(app, listener);
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        app.get(self).parent.remove_listener(app, listener);
    }

    fn add_status_listener(self: Handle<Self>, app: &mut App, listener: AnimationStatusListener) {
        app.get(self).parent.add_status_listener(app, listener);
    }

    fn remove_status_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &AnimationStatusListener,
    ) {
        app.get(self).parent.remove_status_listener(app, listener);
    }

    fn status(self: Handle<Self>, app: &App) -> AnimationStatus {
        app.get(self).parent.status(app)
    }

    fn value(self: Handle<Self>, app: &App) -> T {
        let parent = app.get(self).parent.get();
        let evaluatable = app.get(self).evaluatable.clone();
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
///
/// Dart's `Tween.lerp` is `begin + (end - begin) * t` on `dynamic`, and its assert rejects a
/// type that does not carry the three operators (`Rect` is the type it names: use
/// [`RectTween`]). This trait is the set of types that do.
pub trait TweenLerp: Clone {
    /// Dart's `Tween.lerp`.
    fn lerp(begin: &Self, end: &Self, t: f64) -> Self;
}

impl TweenLerp for f64 {
    fn lerp(begin: &f64, end: &f64, t: f64) -> f64 {
        begin + (end - begin) * t
    }
}

impl TweenLerp for Offset {
    fn lerp(begin: &Offset, end: &Offset, t: f64) -> Offset {
        *begin + (*end - *begin) * t
    }
}

impl TweenLerp for Size {
    fn lerp(begin: &Size, end: &Size, t: f64) -> Size {
        // Dart: `Size - Size` is an `Offset`, and `Size + Offset` is a `Size`.
        *begin + (*end - *begin) * t
    }
}

/// A linear interpolation between a beginning and ending value.
///
/// Tweens are mutable: [`set_begin`] / [`set_end`] after [`Animatable::animate`]
/// is visible to the driven animation, as in Dart.
///
/// [`set_begin`]: Tween::set_begin
/// [`set_end`]: Tween::set_end
#[derive(Debug)]
pub struct Tween<T> {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<T>,

    /// The value this variable has at the end of the animation.
    pub end: Option<T>,
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
    pub fn new(app: &mut App, begin: Option<T>, end: Option<T>) -> Handle<Tween<T>> {
        app.create(Tween { begin, end })
    }

    /// The value this variable has at the beginning of the animation.
    pub fn begin(self: Handle<Self>, app: &App) -> Option<&T> {
        app.get(self).begin.as_ref()
    }

    /// Dart's `begin` setter.
    pub fn set_begin(self: Handle<Self>, app: &mut App, begin: Option<T>) {
        app.get_mut(self).begin = begin;
    }

    /// The value this variable has at the end of the animation.
    pub fn end(self: Handle<Self>, app: &App) -> Option<&T> {
        app.get(self).end.as_ref()
    }

    /// Dart's `end` setter.
    pub fn set_end(self: Handle<Self>, app: &mut App, end: Option<T>) {
        app.get_mut(self).end = end;
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
    pub fn lerp(self: Handle<Self>, app: &App, t: f64) -> T {
        let tween = app.get(self);
        let begin = tween
            .begin
            .as_ref()
            .expect("Tween.begin must be set before use");
        let end = tween
            .end
            .as_ref()
            .expect("Tween.end must be set before use");
        T::lerp(begin, end, t)
    }
}

impl<T: TweenLerp + 'static> Animatable<T> for Handle<Tween<T>> {
    fn transform(&self, app: &App, t: f64) -> T {
        let this = *self;
        let tween = app.get(this);
        tween_transform(&tween.begin, &tween.end, t, |t| this.lerp(app, t))
    }
}

/// A [`Tween`] that evaluates its [`parent`] in reverse.
///
/// [`parent`]: ReverseTween::parent
pub struct ReverseTween<T: 'static> {
    parent: Handle<Tween<T>>,
}

impl<T: TweenLerp + 'static> ReverseTween<T> {
    /// Construct a [`Tween`] that evaluates its parent in reverse.
    pub fn new(app: &mut App, parent: Handle<Tween<T>>) -> Handle<ReverseTween<T>> {
        app.create(ReverseTween { parent })
    }

    /// This tween's value is the same as the parent's value evaluated in reverse.
    pub fn parent(self: Handle<Self>, app: &App) -> Handle<Tween<T>> {
        app.get(self).parent
    }
}

impl<T: TweenLerp + 'static> Animatable<T> for Handle<ReverseTween<T>> {
    fn transform(&self, app: &App, t: f64) -> T {
        let this = *self;
        this.parent(app).lerp(app, 1.0 - t)
    }
}

/// An interpolation between two colors.
pub struct ColorTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<Color>,

    /// The value this variable has at the end of the animation.
    pub end: Option<Color>,
}

impl ColorTween {
    /// Creates a [`Color`] tween.
    ///
    /// The begin and end properties may be null; the null value is treated as
    /// transparent.
    pub fn new(app: &mut App, begin: Option<Color>, end: Option<Color>) -> Handle<ColorTween> {
        app.create(ColorTween { begin, end })
    }

    /// The value this variable has at the beginning of the animation.
    pub fn begin(self: Handle<Self>, app: &App) -> Option<Color> {
        app.get(self).begin
    }

    /// Dart's `begin` setter.
    pub fn set_begin(self: Handle<Self>, app: &mut App, begin: Option<Color>) {
        app.get_mut(self).begin = begin;
    }

    /// The value this variable has at the end of the animation.
    pub fn end(self: Handle<Self>, app: &App) -> Option<Color> {
        app.get(self).end
    }

    /// Dart's `end` setter.
    pub fn set_end(self: Handle<Self>, app: &mut App, end: Option<Color>) {
        app.get_mut(self).end = end;
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(self: Handle<Self>, app: &App, t: f64) -> Option<Color> {
        let tween = app.get(self);
        Color::lerp(tween.begin, tween.end, t)
    }
}

impl Animatable<Option<Color>> for Handle<ColorTween> {
    fn transform(&self, app: &App, t: f64) -> Option<Color> {
        let this = *self;
        if t == 0.0 {
            return this.begin(app);
        }
        if t == 1.0 {
            return this.end(app);
        }
        this.lerp(app, t)
    }
}

/// An interpolation between two sizes.
pub struct SizeTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<Size>,

    /// The value this variable has at the end of the animation.
    pub end: Option<Size>,
}

impl SizeTween {
    /// Creates a [`Size`] tween.
    pub fn new(app: &mut App, begin: Option<Size>, end: Option<Size>) -> Handle<SizeTween> {
        app.create(SizeTween { begin, end })
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(self: Handle<Self>, app: &App, t: f64) -> Option<Size> {
        let tween = app.get(self);
        Size::lerp(tween.begin, tween.end, t)
    }
}

impl Animatable<Option<Size>> for Handle<SizeTween> {
    fn transform(&self, app: &App, t: f64) -> Option<Size> {
        let this = *self;
        let tween = app.get(this);
        if t == 0.0 {
            return tween.begin;
        }
        if t == 1.0 {
            return tween.end;
        }
        this.lerp(app, t)
    }
}

/// An interpolation between two rectangles.
pub struct RectTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<Rect>,

    /// The value this variable has at the end of the animation.
    pub end: Option<Rect>,
}

impl RectTween {
    /// Creates a [`Rect`] tween.
    pub fn new(app: &mut App, begin: Option<Rect>, end: Option<Rect>) -> Handle<RectTween> {
        app.create(RectTween { begin, end })
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(self: Handle<Self>, app: &App, t: f64) -> Option<Rect> {
        let tween = app.get(self);
        Rect::lerp(tween.begin, tween.end, t)
    }
}

impl Animatable<Option<Rect>> for Handle<RectTween> {
    fn transform(&self, app: &App, t: f64) -> Option<Rect> {
        let this = *self;
        let tween = app.get(this);
        if t == 0.0 {
            return tween.begin;
        }
        if t == 1.0 {
            return tween.end;
        }
        this.lerp(app, t)
    }
}

/// An interpolation between two integers that rounds.
pub struct IntTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<i64>,

    /// The value this variable has at the end of the animation.
    pub end: Option<i64>,
}

impl IntTween {
    /// Creates an int tween.
    pub fn new(app: &mut App, begin: Option<i64>, end: Option<i64>) -> Handle<IntTween> {
        app.create(IntTween { begin, end })
    }

    /// Returns the interpolated integer, rounded.
    pub fn lerp(self: Handle<Self>, app: &App, t: f64) -> i64 {
        let tween = app.get(self);
        let begin = tween.begin.expect("IntTween.begin must be set before use");
        let end = tween.end.expect("IntTween.end must be set before use");
        (begin as f64 + (end - begin) as f64 * t).round() as i64
    }
}

impl Animatable<i64> for Handle<IntTween> {
    fn transform(&self, app: &App, t: f64) -> i64 {
        let this = *self;
        let tween = app.get(this);
        tween_transform(&tween.begin, &tween.end, t, |t| this.lerp(app, t))
    }
}

/// An interpolation between two integers that floors.
pub struct StepTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<i64>,

    /// The value this variable has at the end of the animation.
    pub end: Option<i64>,
}

impl StepTween {
    /// Creates an [`i64`] tween that floors.
    pub fn new(app: &mut App, begin: Option<i64>, end: Option<i64>) -> Handle<StepTween> {
        app.create(StepTween { begin, end })
    }

    /// Returns the interpolated integer, floored.
    pub fn lerp(self: Handle<Self>, app: &App, t: f64) -> i64 {
        let tween = app.get(self);
        let begin = tween.begin.expect("StepTween.begin must be set before use");
        let end = tween.end.expect("StepTween.end must be set before use");
        (begin as f64 + (end - begin) as f64 * t).floor() as i64
    }
}

impl Animatable<i64> for Handle<StepTween> {
    fn transform(&self, app: &App, t: f64) -> i64 {
        let this = *self;
        let tween = app.get(this);
        tween_transform(&tween.begin, &tween.end, t, |t| this.lerp(app, t))
    }
}

/// A tween with a constant value.
pub struct ConstantTween<T> {
    // Dart: `class ConstantTween<T> extends Tween<T>` — the superclass's
    // fields, with `begin` and `end` both set to the value.
    tween: Tween<T>,
}

impl<T: Clone + 'static> ConstantTween<T> {
    /// Create a tween whose begin and end values equal `value`.
    pub fn new(app: &mut App, value: T) -> Handle<ConstantTween<T>> {
        app.create(ConstantTween {
            tween: Tween {
                begin: Some(value.clone()),
                end: Some(value),
            },
        })
    }
}

impl<T: Clone + 'static> Animatable<T> for Handle<ConstantTween<T>> {
    fn transform(&self, app: &App, _t: f64) -> T {
        app.get(*self)
            .tween
            .begin
            .clone()
            .expect("ConstantTween.begin must be set")
    }
}

/// Transforms the value of the given animation by the given curve.
#[derive(Debug)]
pub struct CurveTween {
    /// The curve to use when transforming the value of the animation.
    pub curve: Rc<dyn Curve>,
}

impl CurveTween {
    /// Creates a curve tween.
    pub fn new(app: &mut App, curve: Rc<dyn Curve>) -> Handle<CurveTween> {
        app.create(CurveTween { curve })
    }

    /// The curve to use when transforming the value of the animation.
    pub fn curve(self: Handle<Self>, app: &App) -> Rc<dyn Curve> {
        Rc::clone(&app.get(self).curve)
    }

    /// Dart's `curve` setter.
    pub fn set_curve(self: Handle<Self>, app: &mut App, curve: Rc<dyn Curve>) {
        app.get_mut(self).curve = curve;
    }
}

impl Animatable<f64> for Handle<CurveTween> {
    fn transform(&self, app: &App, t: f64) -> f64 {
        let curve = &app.get(*self).curve;
        if t == 0.0 || t == 1.0 {
            debug_assert_eq!(curve.transform(t).round(), t);
            return t;
        }
        curve.transform(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::Animation;
    use crate::animations::AlwaysStoppedAnimation;
    use reveal_foundation::AppCell;

    #[test]
    fn a_driven_tween_sees_end_assigned_after_animate() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tween = Tween::new(&mut app, Some(0.0), Some(1.0));
        let parent = app.create(AlwaysStoppedAnimation::new(1.0)).as_animation();
        let driven = tween.animate(&mut app, parent);

        assert_eq!(driven.value(&app), 1.0);
        tween.set_end(&mut app, Some(0.4));
        assert_eq!(driven.value(&app), 0.4);
    }

    #[test]
    fn lerp_walks_the_line() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tween = Tween::new(&mut app, Some(0.0), Some(10.0));
        assert_eq!(tween.transform(&app, 0.0), 0.0);
        assert_eq!(tween.transform(&app, 0.25), 2.5);
        assert_eq!(tween.transform(&app, 1.0), 10.0);
    }

    #[test]
    fn an_offset_tween_walks_both_axes() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tween = Tween::new(
            &mut app,
            Some(Offset::new(0.0, 10.0)),
            Some(Offset::new(4.0, -10.0)),
        );
        assert_eq!(tween.transform(&app, 0.0), Offset::new(0.0, 10.0));
        assert_eq!(tween.transform(&app, 0.25), Offset::new(1.0, 5.0));
        assert_eq!(tween.transform(&app, 1.0), Offset::new(4.0, -10.0));
    }

    #[test]
    fn a_size_tween_walks_both_dimensions() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tween = Tween::new(
            &mut app,
            Some(Size::new(10.0, 20.0)),
            Some(Size::new(20.0, 0.0)),
        );
        assert_eq!(tween.transform(&app, 0.5), Size::new(15.0, 10.0));
    }
}
