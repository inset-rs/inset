//! Flutter counterpart: `widgets/implicit_animations.dart`.
//!
//! The interpolation objects come first, then [`ImplicitlyAnimatedWidget`] and the widgets
//! built on it.

use std::rc::Rc;
use std::time::Duration;

use reveal_animation::{
    Animatable, Animation, AnimationBehavior, AnimationController, AnimationStatusListener,
    AnyAnimation, Curve, CurvedAnimation, Curves, Tween, TweenLerp,
};
use reveal_embedder::{
    Clip, Matrix4, Offset, Rect, TextAlign, TextHeightBehavior, compose, decompose,
};
use reveal_foundation::{App, Handle, Listener};
use reveal_painting::{
    Alignment, AlignmentGeometry, AnyColor, Border, BorderRadius, BoxDecoration, Decoration,
    EdgeInsets, EdgeInsetsGeometry, TextOverflow, TextStyle, TextWidthBasis,
};
use reveal_rendering::{AlignmentGeometryTween, BoxConstraints, Constraints};
use reveal_scheduler::{Ticker, TickerCallback, TickerProviderObject};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::basic::{Align, Directionality, FractionallySizedBox, Padding, Positioned};
use crate::widgets::container::Container;
use crate::widgets::text::DefaultTextStyle;
use crate::widgets::ticker_provider::{
    SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData,
};
use crate::widgets::transitions::{
    FadeTransition, RotationTransition, ScaleTransition, SlideTransition,
};

/// An interpolation between two [`BoxConstraints`].
///
/// This class specializes the interpolation of `Tween<BoxConstraints>` to use
/// [`BoxConstraints::lerp`].
///
/// See `Tween` for a discussion on how to use interpolation objects.
///
/// Unlike the tweens in `reveal_animation`, this one is a value rather than an arena object:
/// the orphan rule leaves no local type in `impl Animatable<BoxConstraints> for
/// Handle<BoxConstraintsTween>`. A driven animation therefore holds a clone, and writing
/// [`begin`](Self::begin) or [`end`](Self::end) afterwards does not reach it.
#[derive(Clone, Debug)]
pub struct BoxConstraintsTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<BoxConstraints>,

    /// The value this variable has at the end of the animation.
    pub end: Option<BoxConstraints>,
}

impl BoxConstraintsTween {
    /// Creates a [`BoxConstraints`] tween.
    ///
    /// The [`begin`](Self::begin) and [`end`](Self::end) properties may be `None`; the `None`
    /// value is treated as a tight constraint of zero size.
    pub fn new(begin: Option<BoxConstraints>, end: Option<BoxConstraints>) -> BoxConstraintsTween {
        BoxConstraintsTween { begin, end }
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> BoxConstraints {
        BoxConstraints::lerp(self.begin, self.end, t)
            .expect("BoxConstraintsTween.begin or end must be set before use")
    }
}

impl Animatable<BoxConstraints> for BoxConstraintsTween {
    fn transform(&self, _app: &App, t: f64) -> BoxConstraints {
        if t == 0.0 {
            return self.begin.expect("Tween.begin must be set before use");
        }
        if t == 1.0 {
            return self.end.expect("Tween.end must be set before use");
        }
        self.lerp(t)
    }
}

/// An interpolation between two [`Decoration`]s.
///
/// This class specializes the interpolation of `Tween<Decoration>` to use
/// `<dyn Decoration>::lerp`.
///
/// See `Tween` for a discussion on how to use interpolation objects.
///
/// Unlike the tweens in `reveal_animation`, this one is a value rather than an arena object:
/// the orphan rule leaves no local type in `impl Animatable<Box<dyn Decoration>> for
/// Handle<DecorationTween>`. A driven animation therefore holds a clone, and writing
/// [`begin`](Self::begin) or [`end`](Self::end) afterwards does not reach it.
#[derive(Debug)]
pub struct DecorationTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<Box<dyn Decoration>>,

    /// The value this variable has at the end of the animation.
    pub end: Option<Box<dyn Decoration>>,
}

impl DecorationTween {
    /// Creates a decoration tween.
    pub fn new(
        begin: Option<Box<dyn Decoration>>,
        end: Option<Box<dyn Decoration>>,
    ) -> DecorationTween {
        DecorationTween { begin, end }
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> Box<dyn Decoration> {
        <dyn Decoration>::lerp(self.begin.as_deref(), self.end.as_deref(), t)
            .expect("DecorationTween.begin or end must be set before use")
    }
}

impl Clone for DecorationTween {
    fn clone(&self) -> DecorationTween {
        DecorationTween {
            begin: self.begin.as_ref().map(|begin| begin.clone_box()),
            end: self.end.as_ref().map(|end| end.clone_box()),
        }
    }
}

impl Animatable<Box<dyn Decoration>> for DecorationTween {
    fn transform(&self, _app: &App, t: f64) -> Box<dyn Decoration> {
        if t == 0.0 {
            return self
                .begin
                .as_ref()
                .expect("Tween.begin must be set before use")
                .clone_box();
        }
        if t == 1.0 {
            return self
                .end
                .as_ref()
                .expect("Tween.end must be set before use")
                .clone_box();
        }
        self.lerp(t)
    }
}

/// An interpolation between two [`EdgeInsets`]s.
///
/// This class specializes the interpolation of `Tween<EdgeInsets>` to use
/// [`EdgeInsets::lerp`].
///
/// See `Tween` for a discussion on how to use interpolation objects.
///
/// Unlike the tweens in `reveal_animation`, this one is a value rather than an arena object:
/// the orphan rule leaves no local type in `impl Animatable<EdgeInsets> for
/// Handle<EdgeInsetsTween>`. A driven animation therefore holds a clone, and writing
/// [`begin`](Self::begin) or [`end`](Self::end) afterwards does not reach it.
///
/// See also:
///
///  * `EdgeInsetsGeometryTween`, which interpolates between two `EdgeInsetsGeometry` objects.
#[derive(Clone, Debug)]
pub struct EdgeInsetsTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<EdgeInsets>,

    /// The value this variable has at the end of the animation.
    pub end: Option<EdgeInsets>,
}

impl EdgeInsetsTween {
    /// Creates an [`EdgeInsets`] tween.
    ///
    /// The [`begin`](Self::begin) and [`end`](Self::end) properties may be `None`; the `None`
    /// value is treated as an [`EdgeInsets`] with no inset.
    pub fn new(begin: Option<EdgeInsets>, end: Option<EdgeInsets>) -> EdgeInsetsTween {
        EdgeInsetsTween { begin, end }
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> EdgeInsets {
        EdgeInsets::lerp(self.begin, self.end, t)
            .expect("EdgeInsetsTween.begin or end must be set before use")
    }
}

impl Animatable<EdgeInsets> for EdgeInsetsTween {
    fn transform(&self, _app: &App, t: f64) -> EdgeInsets {
        if t == 0.0 {
            return self.begin.expect("Tween.begin must be set before use");
        }
        if t == 1.0 {
            return self.end.expect("Tween.end must be set before use");
        }
        self.lerp(t)
    }
}

/// An interpolation between two [`EdgeInsetsGeometry`]s.
///
/// This class specializes the interpolation of `Tween<EdgeInsetsGeometry>` to use
/// [`EdgeInsetsGeometry::lerp`].
///
/// See `Tween` for a discussion on how to use interpolation objects.
///
/// Like [`EdgeInsetsTween`], this one is a value rather than an arena object (see PORTING.md).
///
/// See also:
///
///  * [`EdgeInsetsTween`], which interpolates between two [`EdgeInsets`] objects.
#[derive(Clone, Debug)]
pub struct EdgeInsetsGeometryTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<EdgeInsetsGeometry>,

    /// The value this variable has at the end of the animation.
    pub end: Option<EdgeInsetsGeometry>,
}

impl EdgeInsetsGeometryTween {
    /// Creates an [`EdgeInsetsGeometry`] tween.
    ///
    /// The [`begin`](Self::begin) and [`end`](Self::end) properties may be `None`; the `None`
    /// value is treated as an [`EdgeInsetsGeometry`] with no inset.
    pub fn new(
        begin: Option<EdgeInsetsGeometry>,
        end: Option<EdgeInsetsGeometry>,
    ) -> EdgeInsetsGeometryTween {
        EdgeInsetsGeometryTween { begin, end }
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> EdgeInsetsGeometry {
        EdgeInsetsGeometry::lerp(self.begin, self.end, t)
            .expect("EdgeInsetsGeometryTween.begin or end must be set before use")
    }
}

impl Animatable<EdgeInsetsGeometry> for EdgeInsetsGeometryTween {
    fn transform(&self, _app: &App, t: f64) -> EdgeInsetsGeometry {
        if t == 0.0 {
            return self.begin.expect("Tween.begin must be set before use");
        }
        if t == 1.0 {
            return self.end.expect("Tween.end must be set before use");
        }
        self.lerp(t)
    }
}

/// An interpolation between two [`BorderRadius`]s.
///
/// This class specializes the interpolation of `Tween<Option<BorderRadius>>` to use
/// [`BorderRadius::lerp`].
///
/// See `Tween` for a discussion on how to use interpolation objects.
///
/// Like [`EdgeInsetsTween`], this one is a value rather than an arena object (see PORTING.md).
#[derive(Clone, Debug)]
pub struct BorderRadiusTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<BorderRadius>,

    /// The value this variable has at the end of the animation.
    pub end: Option<BorderRadius>,
}

impl BorderRadiusTween {
    /// Creates a [`BorderRadius`] tween.
    ///
    /// The [`begin`](Self::begin) and [`end`](Self::end) properties may be `None`; the `None`
    /// value is treated as a right angle (no radius).
    pub fn new(begin: Option<BorderRadius>, end: Option<BorderRadius>) -> BorderRadiusTween {
        BorderRadiusTween { begin, end }
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> Option<BorderRadius> {
        BorderRadius::lerp(self.begin, self.end, t)
    }
}

impl Animatable<Option<BorderRadius>> for BorderRadiusTween {
    fn transform(&self, _app: &App, t: f64) -> Option<BorderRadius> {
        if t == 0.0 {
            return self.begin;
        }
        if t == 1.0 {
            return self.end;
        }
        self.lerp(t)
    }
}

/// An interpolation between two [`Border`]s.
///
/// This class specializes the interpolation of `Tween<Option<Border>>` to use
/// [`Border::lerp`].
///
/// See `Tween` for a discussion on how to use interpolation objects.
///
/// Like [`EdgeInsetsTween`], this one is a value rather than an arena object (see PORTING.md).
#[derive(Clone, Debug)]
pub struct BorderTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<Border>,

    /// The value this variable has at the end of the animation.
    pub end: Option<Border>,
}

impl BorderTween {
    /// Creates a [`Border`] tween.
    ///
    /// The [`begin`](Self::begin) and [`end`](Self::end) properties may be `None`; the `None`
    /// value is treated as having no border.
    pub fn new(begin: Option<Border>, end: Option<Border>) -> BorderTween {
        BorderTween { begin, end }
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> Option<Border> {
        Border::lerp(self.begin, self.end, t)
    }
}

impl Animatable<Option<Border>> for BorderTween {
    fn transform(&self, _app: &App, t: f64) -> Option<Border> {
        if t == 0.0 {
            return self.begin;
        }
        if t == 1.0 {
            return self.end;
        }
        self.lerp(t)
    }
}

/// An interpolation between two [`Matrix4`]s.
///
/// This class specializes the interpolation of `Tween<Matrix4>` to be
/// appropriate for transformation matrices.
///
/// Currently this class works only for translations.
///
/// See `Tween` for a discussion on how to use interpolation objects.
///
/// Like [`EdgeInsetsTween`], this one is a value rather than an arena object (see PORTING.md).
#[derive(Clone, Debug)]
pub struct Matrix4Tween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<Matrix4>,

    /// The value this variable has at the end of the animation.
    pub end: Option<Matrix4>,
}

impl Matrix4Tween {
    /// Creates a [`Matrix4`] tween.
    ///
    /// The [`begin`](Self::begin) and [`end`](Self::end) properties must be non-`None`
    /// before the tween is first used, but the arguments can be `None` if the values are
    /// going to be filled in later.
    pub fn new(begin: Option<Matrix4>, end: Option<Matrix4>) -> Matrix4Tween {
        Matrix4Tween { begin, end }
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> Matrix4 {
        let begin = self
            .begin
            .expect("Matrix4Tween.begin must be set before use");
        let end = self.end.expect("Matrix4Tween.end must be set before use");
        let (begin_translation, begin_rotation, begin_scale) = decompose(begin);
        let (end_translation, end_rotation, end_scale) = decompose(end);
        let t = t as f32;
        let lerp_translation = begin_translation * (1.0 - t) + end_translation * t;
        // TODO(alangardner): Implement lerp for constant rotation
        let lerp_rotation = (begin_rotation.scaled(1.0 - t) + end_rotation.scaled(t)).normalized();
        let lerp_scale = begin_scale * (1.0 - t) + end_scale * t;
        compose(lerp_translation, lerp_rotation, lerp_scale)
    }
}

impl Animatable<Matrix4> for Matrix4Tween {
    fn transform(&self, _app: &App, t: f64) -> Matrix4 {
        if t == 0.0 {
            return self.begin.expect("Tween.begin must be set before use");
        }
        if t == 1.0 {
            return self.end.expect("Tween.end must be set before use");
        }
        self.lerp(t)
    }
}

/// An interpolation between two `TextStyle`s.
///
/// This class specializes the interpolation of `Tween<TextStyle>` to use
/// `TextStyle::lerp`.
///
/// This will not work well if the styles don't set the same fields.
///
/// See `Tween` for a discussion on how to use interpolation objects.
///
/// Like [`EdgeInsetsTween`], this one is a value rather than an arena object (see PORTING.md).
#[derive(Clone, Debug)]
pub struct TextStyleTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<TextStyle>,

    /// The value this variable has at the end of the animation.
    pub end: Option<TextStyle>,
}

impl TextStyleTween {
    /// Creates a text style tween.
    pub fn new(begin: Option<TextStyle>, end: Option<TextStyle>) -> TextStyleTween {
        TextStyleTween { begin, end }
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> TextStyle {
        TextStyle::lerp(self.begin.as_ref(), self.end.as_ref(), t)
            .expect("TextStyleTween.begin or end must be set before use")
    }
}

impl Animatable<TextStyle> for TextStyleTween {
    fn transform(&self, _app: &App, t: f64) -> TextStyle {
        if t == 0.0 {
            return self
                .begin
                .clone()
                .expect("Tween.begin must be set before use");
        }
        if t == 1.0 {
            return self.end.clone().expect("Tween.end must be set before use");
        }
        self.lerp(t)
    }
}

/// The fields of Dart's `ImplicitlyAnimatedWidget`, which its subclasses carry.
#[derive(Debug)]
pub struct ImplicitlyAnimatedWidgetData {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The curve to apply when animating the parameters of this container.
    pub curve: Rc<dyn Curve>,

    /// The duration over which to animate the parameters of this container.
    pub duration: Duration,

    /// Called every time an animation completes.
    ///
    /// This can be useful to trigger additional actions (e.g. another animation)
    /// at the end of the current animation.
    pub on_end: Option<Listener>,
}

impl ImplicitlyAnimatedWidgetData {
    /// The base fields of a widget animating over `duration`, with Dart's defaults.
    pub fn new(duration: Duration) -> ImplicitlyAnimatedWidgetData {
        ImplicitlyAnimatedWidgetData {
            key: None,
            curve: Curves::linear(),
            duration,
            on_end: None,
        }
    }
}

/// The [`ImplicitlyAnimatedWidgetData`] accessors an [`ImplicitlyAnimatedWidget`] implementor
/// writes; the bag lives under the field `implicitly_animated_widget`.
#[macro_export]
macro_rules! implicitly_animated_widget_accessors {
    () => {
        fn implicitly_animated_widget_data(&self) -> &$crate::ImplicitlyAnimatedWidgetData {
            &self.implicitly_animated_widget
        }

        fn implicitly_animated_widget_data_mut(
            &mut self,
        ) -> &mut $crate::ImplicitlyAnimatedWidgetData {
            &mut self.implicitly_animated_widget
        }
    };
}

/// The fluent setters of Dart's `ImplicitlyAnimatedWidget` named arguments, defined on a
/// subclass.
#[macro_export]
macro_rules! implicitly_animated_widget_setters {
    ($name:ident) => {
        /// Dart `key:`.
        pub fn key(mut self, key: $crate::KeyRef) -> $name {
            self.implicitly_animated_widget.key = Some(key);
            self
        }

        /// Dart `curve:`.
        pub fn curve(mut self, curve: ::std::rc::Rc<dyn ::reveal_animation::Curve>) -> $name {
            self.implicitly_animated_widget.curve = curve;
            self
        }

        /// Dart `onEnd:`.
        pub fn on_end(mut self, on_end: ::reveal_foundation::Listener) -> $name {
            self.implicitly_animated_widget.on_end = Some(on_end);
            self
        }
    };
}

/// A widget that animates changes to its properties.
///
/// Widgets of this type will not animate when they are first added to the
/// widget tree. Rather, when they are rebuilt with different values, they will
/// respond to those _changes_ by animating the changes over a specified
/// [`duration`](ImplicitlyAnimatedWidgetData::duration).
///
/// Which properties are animated is left up to the implementor. Their [`State`]s
/// must implement [`ImplicitlyAnimatedWidgetState`] and provide a way to visit the
/// relevant fields to animate.
///
/// ## Relationship to `AnimatedWidget`s
///
/// [`ImplicitlyAnimatedWidget`]s automatically animate changes in their properties whenever
/// they change. For this, they create and manage their own internal
/// [`AnimationController`]s to power the animation. While these widgets are simple to use and
/// don't require you to manually manage the lifecycle of an [`AnimationController`], they
/// are also somewhat limited: besides the target value for the animated property, developers
/// can only choose a [`duration`](ImplicitlyAnimatedWidgetData::duration) and
/// [`curve`](ImplicitlyAnimatedWidgetData::curve) for the animation. If you require more
/// control over the animation (e.g. you want to stop it somewhere in the middle), consider
/// using an `AnimatedWidget`. Those widgets take an `Animation` as an argument, which gives
/// the developer full control over the animation at the cost of requiring you to manually
/// manage the underlying [`AnimationController`].
///
/// ## Common implicitly animated widgets
///
/// A number of implicitly animated widgets ship with the framework. They are
/// usually named `AnimatedFoo`, where `Foo` is the name of the non-animated
/// version of that widget. Commonly used implicitly animated widgets include:
///
///  * [`AnimatedOpacity`], which is an implicitly animated version of `Opacity`.
///  * [`AnimatedPositioned`], which is an implicitly animated version of
///    [`Positioned`].
///  * [`AnimatedPositionedDirectional`], which is an implicitly animated version
///    of `PositionedDirectional`.
///  * [`AnimatedRotation`], which is an implicitly animated version of `Transform.rotate`.
///  * [`AnimatedScale`], which is an implicitly animated version of `Transform.scale`.
///  * [`AnimatedDefaultTextStyle`], which is an implicitly animated version of
///    [`DefaultTextStyle`].
pub trait ImplicitlyAnimatedWidget: StatefulWidget {
    /// The [`ImplicitlyAnimatedWidgetData`] bag
    /// ([`implicitly_animated_widget_accessors!`](crate::implicitly_animated_widget_accessors)).
    fn implicitly_animated_widget_data(&self) -> &ImplicitlyAnimatedWidgetData;

    /// The [`ImplicitlyAnimatedWidgetData`] bag, mutably.
    fn implicitly_animated_widget_data_mut(&mut self) -> &mut ImplicitlyAnimatedWidgetData;
}

/// The `T` of Dart's `Tween<T>` in a slot [`ImplicitlyAnimatedWidgetState::for_each_tween`]
/// visits: Dart's `T extends Object`, with the `==` that decides whether a target value
/// changed.
///
/// `Clone` is not a supertrait: `Box<dyn Decoration>` is a tween value and is duplicated
/// through `Decoration::clone_box`.
pub trait TweenValue: 'static {
    /// Dart's `==` between a widget's target value and a tween endpoint.
    fn same(&self, other: &Self) -> bool;
}

impl TweenValue for f64 {
    fn same(&self, other: &f64) -> bool {
        self == other
    }
}

impl TweenValue for BoxConstraints {
    fn same(&self, other: &BoxConstraints) -> bool {
        self == other
    }
}

impl TweenValue for EdgeInsets {
    fn same(&self, other: &EdgeInsets) -> bool {
        self == other
    }
}

impl TweenValue for EdgeInsetsGeometry {
    fn same(&self, other: &EdgeInsetsGeometry) -> bool {
        self == other
    }
}

impl TweenValue for Matrix4 {
    fn same(&self, other: &Matrix4) -> bool {
        self == other
    }
}

impl TweenValue for Offset {
    fn same(&self, other: &Offset) -> bool {
        self == other
    }
}

impl TweenValue for TextStyle {
    fn same(&self, other: &TextStyle) -> bool {
        self == other
    }
}

impl TweenValue for Option<BorderRadius> {
    fn same(&self, other: &Option<BorderRadius>) -> bool {
        self == other
    }
}

impl TweenValue for Option<Border> {
    fn same(&self, other: &Option<Border>) -> bool {
        self == other
    }
}

impl TweenValue for Option<AlignmentGeometry> {
    fn same(&self, other: &Option<AlignmentGeometry>) -> bool {
        self == other
    }
}

impl TweenValue for Box<dyn Decoration> {
    fn same(&self, other: &Box<dyn Decoration>) -> bool {
        self.eq_decoration(&**other)
    }
}

/// A tween in one of a state's slots: Dart's `Tween<T>` parameter of a [`TweenVisitor`].
///
/// Implemented both by `Handle<Tween<T>>`, the arena tween of `reveal_animation`, and by the
/// tween values this file defines ([`EdgeInsetsTween`] and the rest).
pub trait TweenSlot: Animatable<<Self as TweenSlot>::Value> + Clone + 'static {
    /// The `T` of Dart's `Tween<T>`.
    type Value: TweenValue;

    /// The value this variable has at the beginning of the animation.
    fn begin(&self, app: &App) -> Option<Self::Value>;

    /// Dart's `begin` setter.
    fn set_begin(&mut self, app: &mut App, begin: Option<Self::Value>);

    /// The value this variable has at the end of the animation.
    fn end(&self, app: &App) -> Option<Self::Value>;

    /// Dart's `end` setter.
    fn set_end(&mut self, app: &mut App, end: Option<Self::Value>);

    /// Releases what this tween holds in the arena.
    ///
    /// Dart leaves its tweens to the collector; a slot filled with an arena tween frees it
    /// when the state is disposed.
    fn dispose(&self, app: &mut App) {
        let _ = app;
    }
}

impl<T: TweenLerp + TweenValue> TweenSlot for Handle<Tween<T>> {
    type Value = T;

    fn begin(&self, app: &App) -> Option<T> {
        Tween::begin(*self, app).cloned()
    }

    fn set_begin(&mut self, app: &mut App, begin: Option<T>) {
        Tween::set_begin(*self, app, begin);
    }

    fn end(&self, app: &App) -> Option<T> {
        Tween::end(*self, app).cloned()
    }

    fn set_end(&mut self, app: &mut App, end: Option<T>) {
        Tween::set_end(*self, app, end);
    }

    fn dispose(&self, app: &mut App) {
        app.destroy(*self);
    }
}

/// The slot accessors every tween value in this file shares.
macro_rules! tween_slot_for_value {
    ($tween:ty, $value:ty) => {
        impl TweenSlot for $tween {
            type Value = $value;

            fn begin(&self, _app: &App) -> Option<$value> {
                self.begin.clone()
            }

            fn set_begin(&mut self, _app: &mut App, begin: Option<$value>) {
                self.begin = begin;
            }

            fn end(&self, _app: &App) -> Option<$value> {
                self.end.clone()
            }

            fn set_end(&mut self, _app: &mut App, end: Option<$value>) {
                self.end = end;
            }
        }
    };
}

tween_slot_for_value!(BoxConstraintsTween, BoxConstraints);
tween_slot_for_value!(EdgeInsetsTween, EdgeInsets);
tween_slot_for_value!(EdgeInsetsGeometryTween, EdgeInsetsGeometry);
tween_slot_for_value!(Matrix4Tween, Matrix4);
tween_slot_for_value!(TextStyleTween, TextStyle);

/// The slot accessors of a tween whose Dart value is itself nullable, where `Tween<T?>.end`
/// has type `T??` and Dart flattens it: an unset endpoint and an endpoint set to null are
/// the same thing, which `tween.end ?? tween.begin` relies on.
macro_rules! tween_slot_for_nullable_value {
    ($tween:ty, $value:ty) => {
        impl TweenSlot for $tween {
            type Value = Option<$value>;

            fn begin(&self, _app: &App) -> Option<Option<$value>> {
                self.begin.map(Some)
            }

            fn set_begin(&mut self, _app: &mut App, begin: Option<Option<$value>>) {
                self.begin = begin.flatten();
            }

            fn end(&self, _app: &App) -> Option<Option<$value>> {
                self.end.map(Some)
            }

            fn set_end(&mut self, _app: &mut App, end: Option<Option<$value>>) {
                self.end = end.flatten();
            }
        }
    };
}

tween_slot_for_nullable_value!(BorderRadiusTween, BorderRadius);
tween_slot_for_nullable_value!(BorderTween, Border);
tween_slot_for_nullable_value!(AlignmentGeometryTween, AlignmentGeometry);

impl TweenSlot for DecorationTween {
    type Value = Box<dyn Decoration>;

    fn begin(&self, _app: &App) -> Option<Box<dyn Decoration>> {
        self.begin.as_ref().map(|begin| begin.clone_box())
    }

    fn set_begin(&mut self, _app: &mut App, begin: Option<Box<dyn Decoration>>) {
        self.begin = begin;
    }

    fn end(&self, _app: &App) -> Option<Box<dyn Decoration>> {
        self.end.as_ref().map(|end| end.clone_box())
    }

    fn set_end(&mut self, _app: &mut App, end: Option<Box<dyn Decoration>>) {
        self.end = end;
    }
}

/// Signature for a [`TweenSlot`] factory.
///
/// This is the type of one of the arguments of [`TweenVisitor`], the signature
/// used by [`ImplicitlyAnimatedWidgetState::for_each_tween`].
///
/// Instances of this function are expected to take a value and return a tween
/// beginning at that value.
pub type TweenConstructor<W> = fn(&mut App, &<W as TweenSlot>::Value) -> W;

/// Signature for the callbacks passed to
/// [`ImplicitlyAnimatedWidgetState::for_each_tween`].
///
/// The `tween` argument should contain the current tween value. This will
/// initially be `None` when the state is first initialized.
///
/// The `target` argument should contain the value toward which the state
/// is animating. For instance, if the state is animating its widget's
/// opacity value, then this argument should contain the widget's current
/// opacity value.
///
/// The `constructor` argument should contain a function that takes a value
/// (the widget's value being animated) and returns a tween beginning at that
/// value.
///
/// `for_each_tween` is expected to update its tween value to the return value
/// of this visitor.
///
/// Dart's `TweenVisitor` is a function typed over `dynamic`; a Rust visitor cannot be one
/// closure across the different tween types a state holds, so it is a trait whose one
/// method is generic over the slot.
pub trait TweenVisitor {
    /// Visits one slot; the `W` parameter specifies the tween that is being animated.
    fn visit<W: TweenSlot>(
        &mut self,
        app: &mut App,
        tween: Option<W>,
        target: Option<W::Value>,
        constructor: TweenConstructor<W>,
    ) -> Option<W>;
}

/// Dart's visitor inside `_constructTweens`.
struct ConstructTweens {
    should_start_animation: bool,
}

impl TweenVisitor for ConstructTweens {
    fn visit<W: TweenSlot>(
        &mut self,
        app: &mut App,
        tween: Option<W>,
        target: Option<W::Value>,
        constructor: TweenConstructor<W>,
    ) -> Option<W> {
        let target = target?;
        let mut tween = match tween {
            Some(tween) => tween,
            None => constructor(app, &target),
        };
        let endpoint = tween.end(app).or_else(|| tween.begin(app));
        if !endpoint.is_some_and(|endpoint| target.same(&endpoint)) {
            self.should_start_animation = true;
        } else if tween.end(app).is_none() {
            let begin = tween.begin(app);
            tween.set_end(app, begin);
        }
        Some(tween)
    }
}

/// Dart's visitor inside `didUpdateWidget`, which retargets a running animation from the
/// value it currently shows.
struct RetargetTweens {
    animation: AnyAnimation<f64>,
}

impl TweenVisitor for RetargetTweens {
    fn visit<W: TweenSlot>(
        &mut self,
        app: &mut App,
        tween: Option<W>,
        target: Option<W::Value>,
        _constructor: TweenConstructor<W>,
    ) -> Option<W> {
        let mut tween = tween?;
        let begin = tween.evaluate(app, self.animation);
        tween.set_begin(app, Some(begin));
        tween.set_end(app, target);
        Some(tween)
    }
}

/// Empties every slot at `dispose`, freeing the arena tweens the constructors made.
struct DisposeTweens;

impl TweenVisitor for DisposeTweens {
    fn visit<W: TweenSlot>(
        &mut self,
        app: &mut App,
        tween: Option<W>,
        _target: Option<W::Value>,
        _constructor: TweenConstructor<W>,
    ) -> Option<W> {
        if let Some(tween) = tween {
            tween.dispose(app);
        }
        None
    }
}

/// The fields of Dart's `ImplicitlyAnimatedWidgetState`, which its subclasses carry.
///
/// Both are created in `init_state`, because a state is created without the [`App`].
#[derive(Default)]
pub struct ImplicitlyAnimatedWidgetStateData {
    controller: Option<Handle<AnimationController>>,
    animation: Option<Handle<CurvedAnimation>>,
}

impl ImplicitlyAnimatedWidgetStateData {
    /// The state a fresh [`ImplicitlyAnimatedWidgetState`] starts from.
    pub fn new() -> ImplicitlyAnimatedWidgetStateData {
        ImplicitlyAnimatedWidgetStateData::default()
    }
}

/// The [`ImplicitlyAnimatedWidgetStateData`] accessors an
/// [`ImplicitlyAnimatedWidgetState`] implementor writes; the bag lives under the field
/// `implicitly_animated_widget_state`.
#[macro_export]
macro_rules! implicitly_animated_widget_state_accessors {
    () => {
        fn implicitly_animated_widget_state_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::ImplicitlyAnimatedWidgetStateData {
            &app.get(self).implicitly_animated_widget_state
        }

        fn implicitly_animated_widget_state_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::ImplicitlyAnimatedWidgetStateData {
            &mut app.get_mut(self).implicitly_animated_widget_state
        }
    };
}

/// A base for the [`State`] of widgets with implicit animations.
///
/// [`ImplicitlyAnimatedWidgetState`] requires that implementors respond to the
/// animation themselves. If you would like `set_state` to be called
/// automatically as the animation changes, use [`AnimatedWidgetBaseState`].
///
/// Properties that implementors choose to animate are represented by [`TweenSlot`]
/// fields. They must implement the [`for_each_tween`](Self::for_each_tween) method to allow
/// [`ImplicitlyAnimatedWidgetState`] to iterate through the widget's fields and
/// animate them.
///
/// The bodies Dart inherits are the trait's provided methods; the leaf's `impl State`
/// forwards to them where Dart writes `super.initState()` and the rest.
pub trait ImplicitlyAnimatedWidgetState:
    State + SingleTickerProviderStateMixin + TickerProviderObject
where
    Self::Widget: ImplicitlyAnimatedWidget,
{
    /// The [`ImplicitlyAnimatedWidgetStateData`] bag
    /// ([`implicitly_animated_widget_state_accessors!`](crate::implicitly_animated_widget_state_accessors)).
    fn implicitly_animated_widget_state_data(
        self: Handle<Self>,
        app: &App,
    ) -> &ImplicitlyAnimatedWidgetStateData;

    /// The [`ImplicitlyAnimatedWidgetStateData`] bag, mutably.
    fn implicitly_animated_widget_state_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut ImplicitlyAnimatedWidgetStateData;

    /// The animation controller driving this widget's implicit animations.
    fn controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        self.implicitly_animated_widget_state_data(app)
            .controller
            .expect("created in init_state")
    }

    /// The animation driving this widget's implicit animations.
    fn animation(self: Handle<Self>, app: &App) -> AnyAnimation<f64> {
        curved_animation(self, app).as_animation()
    }

    /// Dart's `ImplicitlyAnimatedWidgetState.initState`.
    fn init_state(self: Handle<Self>, app: &mut App) {
        let duration = self.widget(app).implicitly_animated_widget_data().duration;
        let controller = AnimationController::create(
            app,
            None,
            Some(duration),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        self.implicitly_animated_widget_state_data_mut(app)
            .controller = Some(controller);
        controller.add_status_listener(
            app,
            AnimationStatusListener::new(move |status, app| {
                if status.is_completed() {
                    let on_end = self
                        .widget(app)
                        .implicitly_animated_widget_data()
                        .on_end
                        .clone();
                    if let Some(on_end) = on_end {
                        on_end.call(app);
                    }
                }
            }),
        );
        let animation = create_curve(self, app);
        self.implicitly_animated_widget_state_data_mut(app)
            .animation = Some(animation);
        construct_tweens(self, app);
        self.did_update_tweens(app);
    }

    /// Dart's `ImplicitlyAnimatedWidgetState.didUpdateWidget`.
    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Self::Widget) {
        let curve = Rc::clone(&self.widget(app).implicitly_animated_widget_data().curve);
        if !Rc::ptr_eq(&curve, &old_widget.implicitly_animated_widget_data().curve) {
            let animation = curved_animation(self, app);
            animation.dispose(app);
            app.destroy(animation);
            let animation = create_curve(self, app);
            self.implicitly_animated_widget_state_data_mut(app)
                .animation = Some(animation);
        }
        let duration = self.widget(app).implicitly_animated_widget_data().duration;
        let controller = self.controller(app);
        app.get_mut(controller).duration = Some(duration);
        if construct_tweens(self, app) {
            let animation = self.animation(app);
            self.for_each_tween(app, &mut RetargetTweens { animation });
            controller.forward(app, Some(0.0));
            self.did_update_tweens(app);
        }
    }

    /// Dart's `ImplicitlyAnimatedWidgetState.dispose`.
    fn dispose(self: Handle<Self>, app: &mut App) {
        let animation = curved_animation(self, app);
        animation.dispose(app);
        let controller = self.controller(app);
        controller.dispose(app);
        self.for_each_tween(app, &mut DisposeTweens);
        SingleTickerProviderStateMixin::dispose(self, app);
        let data = self.implicitly_animated_widget_state_data_mut(app);
        data.animation = None;
        data.controller = None;
        app.destroy(animation);
        app.destroy(controller);
    }

    /// Visits each tween controlled by this state with the specified `visitor`.
    ///
    /// ### Implementor responsibility
    ///
    /// Properties to be animated are represented by [`TweenSlot`] fields in the state. For
    /// each such tween, [`for_each_tween`](Self::for_each_tween) implementations are
    /// expected to call [`TweenVisitor::visit`] with the appropriate arguments and store the
    /// result back into the field.
    ///
    /// ### When this method will be called
    ///
    /// [`for_each_tween`](Self::for_each_tween) is initially called during
    /// [`init_state`](Self::init_state). It is expected that the visitor's `tween` argument
    /// will be `None`, causing the visitor to call its `constructor` argument to construct
    /// the tween for the first time. The resulting tween will have its `begin` value set to
    /// the target value and will have its `end` value set to `None`. The animation will not
    /// be started.
    ///
    /// When this state's widget is updated (thus triggering the
    /// [`did_update_widget`](Self::did_update_widget) method to be called),
    /// [`for_each_tween`](Self::for_each_tween) will be called again to check if the target
    /// value has changed. If the target value has changed, signaling that the
    /// [`animation`](Self::animation) should start, then the visitor will update the tween's
    /// `begin` and `end` values accordingly, and the animation will be started.
    ///
    /// ### Other fields
    ///
    /// Implementors that contain properties based on tweens created by
    /// [`for_each_tween`](Self::for_each_tween) should override
    /// [`did_update_tweens`](Self::did_update_tweens) to update those properties. Dependent
    /// properties should not be updated within [`for_each_tween`](Self::for_each_tween).
    fn for_each_tween<V: TweenVisitor>(self: Handle<Self>, app: &mut App, visitor: &mut V);

    /// Optional hook for implementors that runs after all tweens have been updated
    /// via [`for_each_tween`](Self::for_each_tween).
    ///
    /// Any properties that depend upon tweens created by
    /// [`for_each_tween`](Self::for_each_tween) should be updated within
    /// [`did_update_tweens`](Self::did_update_tweens), not within
    /// [`for_each_tween`](Self::for_each_tween).
    ///
    /// This method will be called both:
    ///
    ///  1. After the tweens are _initially_ constructed (by the `constructor` argument to
    ///     the [`TweenVisitor`] that's passed to [`for_each_tween`](Self::for_each_tween)).
    ///     In this case, the tweens are likely to contain only a `begin` value and not an
    ///     `end`.
    ///
    ///  2. When the state's widget is updated, and one or more of the tweens visited by
    ///     [`for_each_tween`](Self::for_each_tween) specifies a target value that's
    ///     different than the widget's current value, thus signaling that the
    ///     [`animation`](Self::animation) should run. In this case, the `begin` value for
    ///     each tween will be an evaluation of the tween against the current
    ///     [`animation`](Self::animation), and the `end` value for each tween will be the
    ///     target value.
    fn did_update_tweens(self: Handle<Self>, app: &mut App) {
        let _ = (self, app);
    }
}

/// Dart's private `_animation`.
fn curved_animation<S>(this: Handle<S>, app: &App) -> Handle<CurvedAnimation>
where
    S: ImplicitlyAnimatedWidgetState,
    S::Widget: ImplicitlyAnimatedWidget,
{
    this.implicitly_animated_widget_state_data(app)
        .animation
        .expect("created in init_state")
}

/// Dart's private `_createCurve`.
fn create_curve<S>(this: Handle<S>, app: &mut App) -> Handle<CurvedAnimation>
where
    S: ImplicitlyAnimatedWidgetState,
    S::Widget: ImplicitlyAnimatedWidget,
{
    let parent = this.controller(app).view();
    let curve = Rc::clone(&this.widget(app).implicitly_animated_widget_data().curve);
    CurvedAnimation::create(app, parent, curve, None)
}

/// Dart's private `_constructTweens`.
fn construct_tweens<S>(this: Handle<S>, app: &mut App) -> bool
where
    S: ImplicitlyAnimatedWidgetState,
    S::Widget: ImplicitlyAnimatedWidget,
{
    let mut visitor = ConstructTweens {
        should_start_animation: false,
    };
    this.for_each_tween(app, &mut visitor);
    visitor.should_start_animation
}

/// A base for widgets with implicit animations that need to rebuild their
/// widget tree as the animation runs.
///
/// This calls [`State::build`] each frame that the animation ticks. For a
/// variant that does not rebuild each frame, implement [`ImplicitlyAnimatedWidgetState`]
/// directly.
///
/// Implementors must write the [`ImplicitlyAnimatedWidgetState::for_each_tween`] method to
/// allow [`AnimatedWidgetBaseState`] to iterate through their widget's fields and animate
/// them.
pub trait AnimatedWidgetBaseState: ImplicitlyAnimatedWidgetState
where
    Self::Widget: ImplicitlyAnimatedWidget,
{
    /// Dart's `AnimatedWidgetBaseState.initState`.
    fn init_state(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::init_state(self, app);
        let controller = self.controller(app);
        controller.add_listener(
            app,
            Listener::handle_method(self, Self::handle_animation_changed),
        );
    }

    /// Dart's private `_handleAnimationChanged`.
    fn handle_animation_changed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {
            // The animation ticked. Rebuild with the new animation value.
        });
    }
}

/// A container that gradually changes its values over a period of time.
///
/// The [`AnimatedContainer`] will automatically animate between the old and
/// new values of properties when they change using the provided curve and
/// duration. Properties that are null are not animated. Its child and
/// descendants are not animated.
///
/// This class is useful for generating simple implicit transitions between
/// different parameters to [`Container`] with its internal
/// [`AnimationController`]. For more complex animations, you'll likely want to
/// use a subclass of `Transition` such as the [`DecoratedBoxTransition`] or use
/// your own [`AnimationController`].
///
/// See also:
///
///  * [`AnimatedPadding`], which is a subset of this widget that only
///    supports animating the [`padding`](Self::padding).
///  * `AnimatedPositioned`, which, as a child of a `Stack`, automatically
///    transitions its child's position over a given duration whenever the given
///    position changes.
///  * [`AnimatedAlign`], which automatically transitions its child's
///    position over a given duration whenever the given
///    [`AnimatedAlign::alignment`] changes.
///
/// [`DecoratedBoxTransition`]: crate::widgets::transitions::DecoratedBoxTransition
#[derive(Debug)]
pub struct AnimatedContainer {
    implicitly_animated_widget: ImplicitlyAnimatedWidgetData,

    /// Align the [`child`](Self::child) within the container.
    ///
    /// If non-`None`, the container will expand to fill its parent and position its
    /// child within itself according to the given value. If the incoming
    /// constraints are unbounded, then the child will be shrink-wrapped instead.
    ///
    /// Ignored if [`child`](Self::child) is `None`.
    pub alignment: Option<AlignmentGeometry>,

    /// Empty space to inscribe inside the [`decoration`](Self::decoration). The
    /// [`child`](Self::child), if any, is placed inside this padding.
    pub padding: Option<EdgeInsetsGeometry>,

    // Dart's `color` constructor argument, which the constructor folds into
    // `decoration`; kept so the two setters can assert they are not both given.
    color: Option<AnyColor>,

    /// The decoration to paint behind the [`child`](Self::child).
    ///
    /// A shorthand for specifying just a solid color is available in
    /// [`color`](Self::color).
    pub decoration: Option<Box<dyn Decoration>>,

    /// The decoration to paint in front of the child.
    pub foreground_decoration: Option<Box<dyn Decoration>>,

    /// Additional constraints to apply to the child.
    ///
    /// The [`width`](Self::width) and [`height`](Self::height) setters are combined with the
    /// given constraints: this is Dart's folded field.
    ///
    /// The [`padding`](Self::padding) goes inside the constraints.
    pub constraints: Option<BoxConstraints>,
    given_constraints: Option<BoxConstraints>,
    width: Option<f64>,
    height: Option<f64>,

    /// Empty space to surround the [`decoration`](Self::decoration) and
    /// [`child`](Self::child).
    pub margin: Option<EdgeInsetsGeometry>,

    /// The transformation matrix to apply before painting the container.
    pub transform: Option<Matrix4>,

    /// The alignment of the origin, relative to the size of the container, if
    /// [`transform`](Self::transform) is specified.
    ///
    /// When [`transform`](Self::transform) is `None`, the value of this property is ignored.
    pub transform_alignment: Option<AlignmentGeometry>,

    /// The [`child`](Self::child) contained by the container.
    ///
    /// If `None`, and if the [`constraints`](Self::constraints) are unbounded or also `None`,
    /// the container will expand to fill all available space in its parent, unless
    /// the parent provides unbounded constraints, in which case the container
    /// will attempt to be as small as possible.
    pub child: Option<WidgetRef>,

    /// The clip behavior when [`decoration`](Self::decoration) is not `None`.
    ///
    /// Defaults to [`Clip::None`]. Must be [`Clip::None`] if
    /// [`decoration`](Self::decoration) is `None`.
    ///
    /// Unlike other properties of [`AnimatedContainer`], changes to this property
    /// apply immediately and have no animation.
    pub clip_behavior: Clip,
}

impl AnimatedContainer {
    /// Creates a container that animates its parameters implicitly.
    pub fn new(duration: Duration) -> AnimatedContainer {
        AnimatedContainer {
            implicitly_animated_widget: ImplicitlyAnimatedWidgetData::new(duration),
            alignment: None,
            padding: None,
            color: None,
            decoration: None,
            foreground_decoration: None,
            constraints: None,
            given_constraints: None,
            width: None,
            height: None,
            margin: None,
            transform: None,
            transform_alignment: None,
            child: None,
            clip_behavior: Clip::None,
        }
    }

    /// Dart `AnimatedContainer(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> AnimatedContainer {
        self.alignment = Some(alignment);
        self
    }

    /// Dart `AnimatedContainer(padding:)`; must be non-negative.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> AnimatedContainer {
        debug_assert!(padding.is_non_negative());
        self.padding = Some(padding);
        self
    }

    /// Dart `AnimatedContainer(color:)`, the shorthand for a solid-color
    /// [`decoration`](Self::decoration); cannot be combined with one.
    pub fn color(mut self, color: impl Into<AnyColor>) -> AnimatedContainer {
        let color = color.into();
        debug_assert!(self.decoration.is_none(), "{CANNOT_PROVIDE_BOTH}");
        self.color = Some(color.clone());
        self.decoration = Some(Box::new(BoxDecoration::new().color(color)));
        self
    }

    /// Dart `AnimatedContainer(decoration:)`; cannot be combined with
    /// [`color`](Self::color).
    pub fn decoration(mut self, decoration: impl Decoration + 'static) -> AnimatedContainer {
        debug_assert!(self.color.is_none(), "{CANNOT_PROVIDE_BOTH}");
        debug_assert!(decoration.debug_assert_is_valid());
        self.decoration = Some(Box::new(decoration));
        self
    }

    /// Dart `AnimatedContainer(foregroundDecoration:)`.
    pub fn foreground_decoration(
        mut self,
        foreground_decoration: impl Decoration + 'static,
    ) -> AnimatedContainer {
        self.foreground_decoration = Some(Box::new(foreground_decoration));
        self
    }

    /// Dart `AnimatedContainer(width:)`: tightens [`constraints`](Self::constraints).
    pub fn width(mut self, width: f64) -> AnimatedContainer {
        self.width = Some(width);
        self.fold_constraints();
        self
    }

    /// Dart `AnimatedContainer(height:)`: tightens [`constraints`](Self::constraints).
    pub fn height(mut self, height: f64) -> AnimatedContainer {
        self.height = Some(height);
        self.fold_constraints();
        self
    }

    /// Dart `AnimatedContainer(constraints:)`; must be valid.
    pub fn constraints(mut self, constraints: BoxConstraints) -> AnimatedContainer {
        debug_assert!(constraints.debug_assert_is_valid(false));
        self.given_constraints = Some(constraints);
        self.fold_constraints();
        self
    }

    /// Dart `AnimatedContainer(margin:)`; must be non-negative.
    pub fn margin(mut self, margin: EdgeInsetsGeometry) -> AnimatedContainer {
        debug_assert!(margin.is_non_negative());
        self.margin = Some(margin);
        self
    }

    /// Dart `AnimatedContainer(transform:)`.
    pub fn transform(mut self, transform: Matrix4) -> AnimatedContainer {
        self.transform = Some(transform);
        self
    }

    /// Dart `AnimatedContainer(transformAlignment:)`.
    pub fn transform_alignment(
        mut self,
        transform_alignment: AlignmentGeometry,
    ) -> AnimatedContainer {
        self.transform_alignment = Some(transform_alignment);
        self
    }

    /// Dart `AnimatedContainer(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> AnimatedContainer {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `AnimatedContainer(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> AnimatedContainer {
        self.clip_behavior = clip_behavior;
        self
    }

    crate::implicitly_animated_widget_setters!(AnimatedContainer);

    /// Dart's constructor line folding `width` and `height` into `constraints`.
    fn fold_constraints(&mut self) {
        self.constraints = if self.width.is_some() || self.height.is_some() {
            Some(match self.given_constraints {
                Some(constraints) => constraints.tighten(self.width, self.height),
                None => BoxConstraints::tight_for(self.width, self.height),
            })
        } else {
            self.given_constraints
        };
    }
}

/// Dart's assert message when both a color and a decoration are supplied.
const CANNOT_PROVIDE_BOTH: &str = "Cannot provide both a color and a decoration\n\
     The color argument is just a shorthand for \"decoration: BoxDecoration(color: color)\".";

impl ImplicitlyAnimatedWidget for AnimatedContainer {
    crate::implicitly_animated_widget_accessors!();
}

impl StatefulWidget for AnimatedContainer {
    type State = AnimatedContainerState;

    fn key(&self) -> Option<&KeyRef> {
        self.implicitly_animated_widget.key.as_ref()
    }

    fn create_state(&self) -> AnimatedContainerState {
        AnimatedContainerState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData::new(),
            alignment: None,
            padding: None,
            decoration: None,
            foreground_decoration: None,
            constraints: None,
            margin: None,
            transform: None,
            transform_alignment: None,
        }
    }
}

/// Dart's `_AnimatedContainerState`.
pub struct AnimatedContainerState {
    state: StateData<AnimatedContainer>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData,
    alignment: Option<AlignmentGeometryTween>,
    padding: Option<EdgeInsetsGeometryTween>,
    decoration: Option<DecorationTween>,
    foreground_decoration: Option<DecorationTween>,
    constraints: Option<BoxConstraintsTween>,
    margin: Option<EdgeInsetsGeometryTween>,
    transform: Option<Matrix4Tween>,
    transform_alignment: Option<AlignmentGeometryTween>,
}

impl SingleTickerProviderStateMixin for AnimatedContainerState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl TickerProviderObject for AnimatedContainerState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ImplicitlyAnimatedWidgetState for AnimatedContainerState {
    crate::implicitly_animated_widget_state_accessors!();

    fn for_each_tween<V: TweenVisitor>(self: Handle<Self>, app: &mut App, visitor: &mut V) {
        let tween = app.get(self).alignment.clone();
        let target = self.widget(app).alignment.map(Some);
        app.get_mut(self).alignment = visitor.visit(app, tween, target, |_app, value| {
            AlignmentGeometryTween::new(*value, None)
        });
        let tween = app.get(self).padding.clone();
        let target = self.widget(app).padding;
        app.get_mut(self).padding = visitor.visit(app, tween, target, |_app, value| {
            EdgeInsetsGeometryTween::new(Some(*value), None)
        });
        let tween = app.get(self).decoration.clone();
        let target = clone_decoration(self.widget(app).decoration.as_deref());
        app.get_mut(self).decoration = visitor.visit(app, tween, target, |_app, value| {
            DecorationTween::new(Some(value.clone_box()), None)
        });
        let tween = app.get(self).foreground_decoration.clone();
        let target = clone_decoration(self.widget(app).foreground_decoration.as_deref());
        app.get_mut(self).foreground_decoration =
            visitor.visit(app, tween, target, |_app, value| {
                DecorationTween::new(Some(value.clone_box()), None)
            });
        let tween = app.get(self).constraints.clone();
        let target = self.widget(app).constraints;
        app.get_mut(self).constraints = visitor.visit(app, tween, target, |_app, value| {
            BoxConstraintsTween::new(Some(*value), None)
        });
        let tween = app.get(self).margin.clone();
        let target = self.widget(app).margin;
        app.get_mut(self).margin = visitor.visit(app, tween, target, |_app, value| {
            EdgeInsetsGeometryTween::new(Some(*value), None)
        });
        let tween = app.get(self).transform.clone();
        let target = self.widget(app).transform;
        app.get_mut(self).transform = visitor.visit(app, tween, target, |_app, value| {
            Matrix4Tween::new(Some(*value), None)
        });
        let tween = app.get(self).transform_alignment.clone();
        let target = self.widget(app).transform_alignment.map(Some);
        app.get_mut(self).transform_alignment = visitor.visit(app, tween, target, |_app, value| {
            AlignmentGeometryTween::new(*value, None)
        });
    }
}

impl AnimatedWidgetBaseState for AnimatedContainerState {}

impl State for AnimatedContainerState {
    type Widget = AnimatedContainer;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        AnimatedWidgetBaseState::init_state(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &AnimatedContainer) {
        ImplicitlyAnimatedWidgetState::did_update_widget(self, app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let animation = self.animation(app);
        let state = app.get(self);
        let (alignment, padding) = (state.alignment.clone(), state.padding.clone());
        let (decoration, foreground_decoration) = (
            state.decoration.clone(),
            state.foreground_decoration.clone(),
        );
        let (constraints, margin) = (state.constraints.clone(), state.margin.clone());
        let (transform, transform_alignment) =
            (state.transform.clone(), state.transform_alignment.clone());
        let widget = self.widget(app);
        let (clip_behavior, child) = (widget.clip_behavior, widget.child.clone());
        let mut container = Container::new().clip_behavior(clip_behavior);
        if let Some(alignment) = alignment.and_then(|tween| tween.evaluate(app, animation)) {
            container = container.alignment(alignment);
        }
        if let Some(padding) = padding {
            container = container.padding(padding.evaluate(app, animation));
        }
        if let Some(constraints) = constraints {
            container = container.constraints(constraints.evaluate(app, animation));
        }
        if let Some(margin) = margin {
            container = container.margin(margin.evaluate(app, animation));
        }
        if let Some(transform) = transform {
            container = container.transform(transform.evaluate(app, animation));
        }
        if let Some(transform_alignment) =
            transform_alignment.and_then(|tween| tween.evaluate(app, animation))
        {
            container = container.transform_alignment(transform_alignment);
        }
        if let Some(child) = child {
            container = container.child(child);
        }
        // An evaluated decoration is a `Box<dyn Decoration>`, which the setter's
        // `impl Decoration` argument cannot take; `DecoratedBoxTransition` writes the
        // field for the same reason. The setter's assert runs here instead.
        container.decoration = decoration.map(|tween| tween.evaluate(app, animation));
        container.foreground_decoration =
            foreground_decoration.map(|tween| tween.evaluate(app, animation));
        debug_assert!(
            container
                .decoration
                .as_ref()
                .is_none_or(|decoration| decoration.debug_assert_is_valid())
        );
        container.into_widget()
    }
}

/// The target a decoration slot hands the visitor; Dart passes `widget.decoration` itself.
fn clone_decoration(decoration: Option<&dyn Decoration>) -> Option<Box<dyn Decoration>> {
    decoration.map(|decoration| decoration.clone_box())
}

/// Animated version of [`Padding`] which automatically transitions the
/// indentation over a given duration whenever the given inset changes.
///
/// See also:
///
///  * [`AnimatedContainer`], which can transition more values at once.
///  * [`AnimatedAlign`], which automatically transitions its child's
///    position over a given duration whenever the given
///    [`AnimatedAlign::alignment`] changes.
#[derive(Debug)]
pub struct AnimatedPadding {
    implicitly_animated_widget: ImplicitlyAnimatedWidgetData,

    /// The amount of space by which to inset the child.
    pub padding: EdgeInsetsGeometry,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
}

impl AnimatedPadding {
    /// Creates a widget that insets its child by a value that animates
    /// implicitly.
    pub fn new(padding: EdgeInsetsGeometry, duration: Duration) -> AnimatedPadding {
        debug_assert!(padding.is_non_negative());
        AnimatedPadding {
            implicitly_animated_widget: ImplicitlyAnimatedWidgetData::new(duration),
            padding,
            child: None,
        }
    }

    /// Dart `AnimatedPadding(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> AnimatedPadding {
        self.child = Some(child.into_widget());
        self
    }

    crate::implicitly_animated_widget_setters!(AnimatedPadding);
}

impl ImplicitlyAnimatedWidget for AnimatedPadding {
    crate::implicitly_animated_widget_accessors!();
}

impl StatefulWidget for AnimatedPadding {
    type State = AnimatedPaddingState;

    fn key(&self) -> Option<&KeyRef> {
        self.implicitly_animated_widget.key.as_ref()
    }

    fn create_state(&self) -> AnimatedPaddingState {
        AnimatedPaddingState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData::new(),
            padding: None,
        }
    }
}

/// Dart's `_AnimatedPaddingState`.
pub struct AnimatedPaddingState {
    state: StateData<AnimatedPadding>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData,
    padding: Option<EdgeInsetsGeometryTween>,
}

impl SingleTickerProviderStateMixin for AnimatedPaddingState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl TickerProviderObject for AnimatedPaddingState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ImplicitlyAnimatedWidgetState for AnimatedPaddingState {
    crate::implicitly_animated_widget_state_accessors!();

    fn for_each_tween<V: TweenVisitor>(self: Handle<Self>, app: &mut App, visitor: &mut V) {
        let tween = app.get(self).padding.clone();
        let target = Some(self.widget(app).padding);
        app.get_mut(self).padding = visitor.visit(app, tween, target, |_app, value| {
            EdgeInsetsGeometryTween::new(Some(*value), None)
        });
    }
}

impl AnimatedWidgetBaseState for AnimatedPaddingState {}

impl State for AnimatedPaddingState {
    type Widget = AnimatedPadding;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        AnimatedWidgetBaseState::init_state(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &AnimatedPadding) {
        ImplicitlyAnimatedWidgetState::did_update_widget(self, app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let animation = self.animation(app);
        let padding = app
            .get(self)
            .padding
            .clone()
            .expect("for_each_tween made the tween");
        let child = self.widget(app).child.clone();
        let padding = padding
            .evaluate(app, animation)
            .clamp(&EdgeInsetsGeometry::ZERO, &EdgeInsetsGeometry::INFINITY);
        let mut widget = Padding::new(padding);
        if let Some(child) = child {
            widget = widget.child(child);
        }
        widget.into_widget()
    }
}

/// Animated version of [`Align`] which automatically transitions the child's
/// position over a given duration whenever the given
/// [`alignment`](Self::alignment) changes.
///
/// For the animation, you can choose a [`curve`](ImplicitlyAnimatedWidgetData::curve) as
/// well as a [`duration`](ImplicitlyAnimatedWidgetData::duration) and the widget will
/// automatically animate to the new target [`alignment`](Self::alignment). If you require
/// more control over the animation (e.g. if you want to stop it mid-animation), consider
/// using an `AlignTransition` instead, which takes a provided `Animation` as argument. While
/// that allows you to fine-tune the animation, it also requires more development overhead as
/// you have to manually manage the lifecycle of the underlying [`AnimationController`].
///
/// See also:
///
///  * [`AnimatedContainer`], which can transition more values at once.
///  * [`AnimatedPadding`], which can animate the padding instead of the
///    alignment.
///  * [`AnimatedSlide`], which can animate the translation of child by a given offset
///    relative to its size.
///  * [`AnimatedPositioned`], which, as a child of a `Stack`, automatically
///    transitions its child's position over a given duration whenever the given
///    position changes.
#[derive(Debug)]
pub struct AnimatedAlign {
    implicitly_animated_widget: ImplicitlyAnimatedWidgetData,

    /// How to align the child.
    ///
    /// The x and y values of the [`Alignment`] control the horizontal and vertical
    /// alignment, respectively. An x value of -1.0 means that the left edge of
    /// the child is aligned with the left edge of the parent whereas an x value
    /// of 1.0 means that the right edge of the child is aligned with the right
    /// edge of the parent. Other values interpolate (and extrapolate) linearly.
    /// For example, a value of 0.0 means that the center of the child is aligned
    /// with the center of the parent.
    pub alignment: AlignmentGeometry,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,

    /// If non-`None`, sets its height to the child's height multiplied by this factor.
    ///
    /// Must be greater than or equal to 0.0, defaults to `None`.
    pub height_factor: Option<f64>,

    /// If non-`None`, sets its width to the child's width multiplied by this factor.
    ///
    /// Must be greater than or equal to 0.0, defaults to `None`.
    pub width_factor: Option<f64>,
}

impl AnimatedAlign {
    /// Creates a widget that positions its child by an alignment that animates
    /// implicitly.
    pub fn new(alignment: AlignmentGeometry, duration: Duration) -> AnimatedAlign {
        AnimatedAlign {
            implicitly_animated_widget: ImplicitlyAnimatedWidgetData::new(duration),
            alignment,
            child: None,
            height_factor: None,
            width_factor: None,
        }
    }

    /// Dart `AnimatedAlign(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> AnimatedAlign {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `AnimatedAlign(heightFactor:)`.
    pub fn height_factor(mut self, height_factor: f64) -> AnimatedAlign {
        debug_assert!(height_factor >= 0.0);
        self.height_factor = Some(height_factor);
        self
    }

    /// Dart `AnimatedAlign(widthFactor:)`.
    pub fn width_factor(mut self, width_factor: f64) -> AnimatedAlign {
        debug_assert!(width_factor >= 0.0);
        self.width_factor = Some(width_factor);
        self
    }

    crate::implicitly_animated_widget_setters!(AnimatedAlign);
}

impl ImplicitlyAnimatedWidget for AnimatedAlign {
    crate::implicitly_animated_widget_accessors!();
}

impl StatefulWidget for AnimatedAlign {
    type State = AnimatedAlignState;

    fn key(&self) -> Option<&KeyRef> {
        self.implicitly_animated_widget.key.as_ref()
    }

    fn create_state(&self) -> AnimatedAlignState {
        AnimatedAlignState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData::new(),
            alignment: None,
            height_factor_tween: None,
            width_factor_tween: None,
        }
    }
}

/// Dart's `_AnimatedAlignState`.
pub struct AnimatedAlignState {
    state: StateData<AnimatedAlign>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData,
    alignment: Option<AlignmentGeometryTween>,
    height_factor_tween: Option<Handle<Tween<f64>>>,
    width_factor_tween: Option<Handle<Tween<f64>>>,
}

impl SingleTickerProviderStateMixin for AnimatedAlignState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl TickerProviderObject for AnimatedAlignState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ImplicitlyAnimatedWidgetState for AnimatedAlignState {
    crate::implicitly_animated_widget_state_accessors!();

    fn for_each_tween<V: TweenVisitor>(self: Handle<Self>, app: &mut App, visitor: &mut V) {
        let tween = app.get(self).alignment.clone();
        let target = Some(Some(self.widget(app).alignment));
        app.get_mut(self).alignment = visitor.visit(app, tween, target, |_app, value| {
            AlignmentGeometryTween::new(*value, None)
        });
        if self.widget(app).height_factor.is_some() {
            let (tween, target) = (
                app.get(self).height_factor_tween,
                self.widget(app).height_factor,
            );
            app.get_mut(self).height_factor_tween = visitor.visit(app, tween, target, double_tween);
        }
        if self.widget(app).width_factor.is_some() {
            let (tween, target) = (
                app.get(self).width_factor_tween,
                self.widget(app).width_factor,
            );
            app.get_mut(self).width_factor_tween = visitor.visit(app, tween, target, double_tween);
        }
    }
}

impl AnimatedWidgetBaseState for AnimatedAlignState {}

impl State for AnimatedAlignState {
    type Widget = AnimatedAlign;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        AnimatedWidgetBaseState::init_state(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &AnimatedAlign) {
        ImplicitlyAnimatedWidgetState::did_update_widget(self, app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let animation = self.animation(app);
        let state = app.get(self);
        let alignment = state
            .alignment
            .clone()
            .expect("for_each_tween made the tween");
        let (height_factor_tween, width_factor_tween) =
            (state.height_factor_tween, state.width_factor_tween);
        let child = self.widget(app).child.clone();
        let alignment = alignment
            .evaluate(app, animation)
            .expect("the tween is built from the widget's alignment");
        let mut align = Align::new().alignment(alignment);
        if let Some(height_factor) = height_factor_tween {
            align = align.height_factor(height_factor.evaluate(app, animation));
        }
        if let Some(width_factor) = width_factor_tween {
            align = align.width_factor(width_factor.evaluate(app, animation));
        }
        if let Some(child) = child {
            align = align.child(child);
        }
        align.into_widget()
    }
}

/// Animated version of [`Positioned`] which automatically transitions the child's
/// position over a given duration whenever the given position changes.
///
/// Only works if it's the child of a `Stack`.
///
/// This widget is a good choice if the _size_ of the child would end up
/// changing as a result of this animation. If the size is intended to remain
/// the same, with only the _position_ changing over time, then consider
/// `SlideTransition` instead. `SlideTransition` only triggers a repaint each
/// frame of the animation, whereas [`AnimatedPositioned`] will trigger a relayout
/// as well.
///
/// For the animation, you can choose a [`curve`](ImplicitlyAnimatedWidgetData::curve) as
/// well as a [`duration`](ImplicitlyAnimatedWidgetData::duration) and the widget will
/// automatically animate to the new target position. If you require more control over the
/// animation (e.g. if you want to stop it mid-animation), consider using a
/// `PositionedTransition` instead.
///
/// See also:
///
///  * [`AnimatedPositionedDirectional`], which adapts to the ambient
///    [`Directionality`] (the same as this widget, but for animating
///    `PositionedDirectional`).
#[derive(Debug)]
pub struct AnimatedPositioned {
    implicitly_animated_widget: ImplicitlyAnimatedWidgetData,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,

    /// The offset of the child's left edge from the left of the stack.
    pub left: Option<f64>,

    /// The offset of the child's top edge from the top of the stack.
    pub top: Option<f64>,

    /// The offset of the child's right edge from the right of the stack.
    pub right: Option<f64>,

    /// The offset of the child's bottom edge from the bottom of the stack.
    pub bottom: Option<f64>,

    /// The child's width.
    ///
    /// Only two out of the three horizontal values ([`left`](Self::left),
    /// [`right`](Self::right), [`width`](Self::width)) can be set. The third must be `None`.
    pub width: Option<f64>,

    /// The child's height.
    ///
    /// Only two out of the three vertical values ([`top`](Self::top),
    /// [`bottom`](Self::bottom), [`height`](Self::height)) can be set. The third must be
    /// `None`.
    pub height: Option<f64>,
}

impl AnimatedPositioned {
    /// Creates a widget that animates its position implicitly.
    ///
    /// Only two out of the three horizontal values ([`left`](Self::left),
    /// [`right`](Self::right), [`width`](Self::width)), and only two out of the three
    /// vertical values ([`top`](Self::top), [`bottom`](Self::bottom),
    /// [`height`](Self::height)), can be set. In each case, at least one of the three must be
    /// `None`.
    pub fn new<K>(child: impl IntoWidget<K>, duration: Duration) -> AnimatedPositioned {
        AnimatedPositioned {
            implicitly_animated_widget: ImplicitlyAnimatedWidgetData::new(duration),
            child: child.into_widget(),
            left: None,
            top: None,
            right: None,
            bottom: None,
            width: None,
            height: None,
        }
    }

    /// Creates a widget that animates the rectangle it occupies implicitly.
    pub fn from_rect<K>(
        child: impl IntoWidget<K>,
        rect: Rect,
        duration: Duration,
    ) -> AnimatedPositioned {
        AnimatedPositioned {
            implicitly_animated_widget: ImplicitlyAnimatedWidgetData::new(duration),
            child: child.into_widget(),
            left: Some(rect.left),
            top: Some(rect.top),
            right: None,
            bottom: None,
            width: Some(rect.width()),
            height: Some(rect.height()),
        }
    }

    /// Dart `AnimatedPositioned(left:)`.
    pub fn left(mut self, left: f64) -> AnimatedPositioned {
        self.left = Some(left);
        self.debug_check_axes();
        self
    }

    /// Dart `AnimatedPositioned(top:)`.
    pub fn top(mut self, top: f64) -> AnimatedPositioned {
        self.top = Some(top);
        self.debug_check_axes();
        self
    }

    /// Dart `AnimatedPositioned(right:)`.
    pub fn right(mut self, right: f64) -> AnimatedPositioned {
        self.right = Some(right);
        self.debug_check_axes();
        self
    }

    /// Dart `AnimatedPositioned(bottom:)`.
    pub fn bottom(mut self, bottom: f64) -> AnimatedPositioned {
        self.bottom = Some(bottom);
        self.debug_check_axes();
        self
    }

    /// Dart `AnimatedPositioned(width:)`.
    pub fn width(mut self, width: f64) -> AnimatedPositioned {
        self.width = Some(width);
        self.debug_check_axes();
        self
    }

    /// Dart `AnimatedPositioned(height:)`.
    pub fn height(mut self, height: f64) -> AnimatedPositioned {
        self.height = Some(height);
        self.debug_check_axes();
        self
    }

    crate::implicitly_animated_widget_setters!(AnimatedPositioned);

    /// Dart's two constructor asserts, on the setters that can violate them.
    fn debug_check_axes(&self) {
        debug_assert!(
            self.left.is_none() || self.right.is_none() || self.width.is_none(),
            "Only two out of `left`, `right` and `width` can be set."
        );
        debug_assert!(
            self.top.is_none() || self.bottom.is_none() || self.height.is_none(),
            "Only two out of `top`, `bottom` and `height` can be set."
        );
    }
}

impl ImplicitlyAnimatedWidget for AnimatedPositioned {
    crate::implicitly_animated_widget_accessors!();
}

impl StatefulWidget for AnimatedPositioned {
    type State = AnimatedPositionedState;

    fn key(&self) -> Option<&KeyRef> {
        self.implicitly_animated_widget.key.as_ref()
    }

    fn create_state(&self) -> AnimatedPositionedState {
        AnimatedPositionedState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData::new(),
            left: None,
            top: None,
            right: None,
            bottom: None,
            width: None,
            height: None,
        }
    }
}

/// Dart's `_AnimatedPositionedState`.
pub struct AnimatedPositionedState {
    state: StateData<AnimatedPositioned>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData,
    left: Option<Handle<Tween<f64>>>,
    top: Option<Handle<Tween<f64>>>,
    right: Option<Handle<Tween<f64>>>,
    bottom: Option<Handle<Tween<f64>>>,
    width: Option<Handle<Tween<f64>>>,
    height: Option<Handle<Tween<f64>>>,
}

impl SingleTickerProviderStateMixin for AnimatedPositionedState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl TickerProviderObject for AnimatedPositionedState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ImplicitlyAnimatedWidgetState for AnimatedPositionedState {
    crate::implicitly_animated_widget_state_accessors!();

    fn for_each_tween<V: TweenVisitor>(self: Handle<Self>, app: &mut App, visitor: &mut V) {
        let (tween, target) = (app.get(self).left, self.widget(app).left);
        app.get_mut(self).left = visitor.visit(app, tween, target, double_tween);
        let (tween, target) = (app.get(self).top, self.widget(app).top);
        app.get_mut(self).top = visitor.visit(app, tween, target, double_tween);
        let (tween, target) = (app.get(self).right, self.widget(app).right);
        app.get_mut(self).right = visitor.visit(app, tween, target, double_tween);
        let (tween, target) = (app.get(self).bottom, self.widget(app).bottom);
        app.get_mut(self).bottom = visitor.visit(app, tween, target, double_tween);
        let (tween, target) = (app.get(self).width, self.widget(app).width);
        app.get_mut(self).width = visitor.visit(app, tween, target, double_tween);
        let (tween, target) = (app.get(self).height, self.widget(app).height);
        app.get_mut(self).height = visitor.visit(app, tween, target, double_tween);
    }
}

impl AnimatedWidgetBaseState for AnimatedPositionedState {}

impl State for AnimatedPositionedState {
    type Widget = AnimatedPositioned;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        AnimatedWidgetBaseState::init_state(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &AnimatedPositioned) {
        ImplicitlyAnimatedWidgetState::did_update_widget(self, app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let animation = self.animation(app);
        let state = app.get(self);
        let (left, top, right) = (state.left, state.top, state.right);
        let (bottom, width, height) = (state.bottom, state.width, state.height);
        let mut positioned = Positioned::new(self.widget(app).child.clone());
        if let Some(left) = left {
            positioned = positioned.left(left.evaluate(app, animation));
        }
        if let Some(top) = top {
            positioned = positioned.top(top.evaluate(app, animation));
        }
        if let Some(right) = right {
            positioned = positioned.right(right.evaluate(app, animation));
        }
        if let Some(bottom) = bottom {
            positioned = positioned.bottom(bottom.evaluate(app, animation));
        }
        if let Some(width) = width {
            positioned = positioned.width(width.evaluate(app, animation));
        }
        if let Some(height) = height {
            positioned = positioned.height(height.evaluate(app, animation));
        }
        positioned.into_widget()
    }
}

/// Animated version of `PositionedDirectional` which automatically transitions
/// the child's position over a given duration whenever the given position
/// changes.
///
/// The ambient [`Directionality`] is used to determine whether
/// [`start`](Self::start) is to the left or to the right.
///
/// Only works if it's the child of a `Stack`.
///
/// See also:
///
///  * [`AnimatedPositioned`], which specifies the widget's position visually (the
///    same as this widget, but for animating [`Positioned`]).
#[derive(Debug)]
pub struct AnimatedPositionedDirectional {
    implicitly_animated_widget: ImplicitlyAnimatedWidgetData,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,

    /// The offset of the child's start edge from the start of the stack.
    pub start: Option<f64>,

    /// The offset of the child's top edge from the top of the stack.
    pub top: Option<f64>,

    /// The offset of the child's end edge from the end of the stack.
    pub end: Option<f64>,

    /// The offset of the child's bottom edge from the bottom of the stack.
    pub bottom: Option<f64>,

    /// The child's width.
    ///
    /// Only two out of the three horizontal values ([`start`](Self::start),
    /// [`end`](Self::end), [`width`](Self::width)) can be set. The third must be `None`.
    pub width: Option<f64>,

    /// The child's height.
    ///
    /// Only two out of the three vertical values ([`top`](Self::top),
    /// [`bottom`](Self::bottom), [`height`](Self::height)) can be set. The third must be
    /// `None`.
    pub height: Option<f64>,
}

impl AnimatedPositionedDirectional {
    /// Creates a widget that animates its position implicitly.
    ///
    /// Only two out of the three horizontal values ([`start`](Self::start),
    /// [`end`](Self::end), [`width`](Self::width)), and only two out of the three vertical
    /// values ([`top`](Self::top), [`bottom`](Self::bottom), [`height`](Self::height)), can
    /// be set. In each case, at least one of the three must be `None`.
    pub fn new<K>(child: impl IntoWidget<K>, duration: Duration) -> AnimatedPositionedDirectional {
        AnimatedPositionedDirectional {
            implicitly_animated_widget: ImplicitlyAnimatedWidgetData::new(duration),
            child: child.into_widget(),
            start: None,
            top: None,
            end: None,
            bottom: None,
            width: None,
            height: None,
        }
    }

    /// Dart `AnimatedPositionedDirectional(start:)`.
    pub fn start(mut self, start: f64) -> AnimatedPositionedDirectional {
        self.start = Some(start);
        self.debug_check_axes();
        self
    }

    /// Dart `AnimatedPositionedDirectional(top:)`.
    pub fn top(mut self, top: f64) -> AnimatedPositionedDirectional {
        self.top = Some(top);
        self.debug_check_axes();
        self
    }

    /// Dart `AnimatedPositionedDirectional(end:)`.
    pub fn end(mut self, end: f64) -> AnimatedPositionedDirectional {
        self.end = Some(end);
        self.debug_check_axes();
        self
    }

    /// Dart `AnimatedPositionedDirectional(bottom:)`.
    pub fn bottom(mut self, bottom: f64) -> AnimatedPositionedDirectional {
        self.bottom = Some(bottom);
        self.debug_check_axes();
        self
    }

    /// Dart `AnimatedPositionedDirectional(width:)`.
    pub fn width(mut self, width: f64) -> AnimatedPositionedDirectional {
        self.width = Some(width);
        self.debug_check_axes();
        self
    }

    /// Dart `AnimatedPositionedDirectional(height:)`.
    pub fn height(mut self, height: f64) -> AnimatedPositionedDirectional {
        self.height = Some(height);
        self.debug_check_axes();
        self
    }

    crate::implicitly_animated_widget_setters!(AnimatedPositionedDirectional);

    /// Dart's two constructor asserts, on the setters that can violate them.
    fn debug_check_axes(&self) {
        debug_assert!(
            self.start.is_none() || self.end.is_none() || self.width.is_none(),
            "Only two out of `start`, `end` and `width` can be set."
        );
        debug_assert!(
            self.top.is_none() || self.bottom.is_none() || self.height.is_none(),
            "Only two out of `top`, `bottom` and `height` can be set."
        );
    }
}

impl ImplicitlyAnimatedWidget for AnimatedPositionedDirectional {
    crate::implicitly_animated_widget_accessors!();
}

impl StatefulWidget for AnimatedPositionedDirectional {
    type State = AnimatedPositionedDirectionalState;

    fn key(&self) -> Option<&KeyRef> {
        self.implicitly_animated_widget.key.as_ref()
    }

    fn create_state(&self) -> AnimatedPositionedDirectionalState {
        AnimatedPositionedDirectionalState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData::new(),
            start: None,
            top: None,
            end: None,
            bottom: None,
            width: None,
            height: None,
        }
    }
}

/// Dart's `_AnimatedPositionedDirectionalState`.
pub struct AnimatedPositionedDirectionalState {
    state: StateData<AnimatedPositionedDirectional>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData,
    start: Option<Handle<Tween<f64>>>,
    top: Option<Handle<Tween<f64>>>,
    end: Option<Handle<Tween<f64>>>,
    bottom: Option<Handle<Tween<f64>>>,
    width: Option<Handle<Tween<f64>>>,
    height: Option<Handle<Tween<f64>>>,
}

impl SingleTickerProviderStateMixin for AnimatedPositionedDirectionalState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl TickerProviderObject for AnimatedPositionedDirectionalState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ImplicitlyAnimatedWidgetState for AnimatedPositionedDirectionalState {
    crate::implicitly_animated_widget_state_accessors!();

    fn for_each_tween<V: TweenVisitor>(self: Handle<Self>, app: &mut App, visitor: &mut V) {
        let (tween, target) = (app.get(self).start, self.widget(app).start);
        app.get_mut(self).start = visitor.visit(app, tween, target, double_tween);
        let (tween, target) = (app.get(self).top, self.widget(app).top);
        app.get_mut(self).top = visitor.visit(app, tween, target, double_tween);
        let (tween, target) = (app.get(self).end, self.widget(app).end);
        app.get_mut(self).end = visitor.visit(app, tween, target, double_tween);
        let (tween, target) = (app.get(self).bottom, self.widget(app).bottom);
        app.get_mut(self).bottom = visitor.visit(app, tween, target, double_tween);
        let (tween, target) = (app.get(self).width, self.widget(app).width);
        app.get_mut(self).width = visitor.visit(app, tween, target, double_tween);
        let (tween, target) = (app.get(self).height, self.widget(app).height);
        app.get_mut(self).height = visitor.visit(app, tween, target, double_tween);
    }
}

impl AnimatedWidgetBaseState for AnimatedPositionedDirectionalState {}

impl State for AnimatedPositionedDirectionalState {
    type Widget = AnimatedPositionedDirectional;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        AnimatedWidgetBaseState::init_state(self, app);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &AnimatedPositionedDirectional,
    ) {
        ImplicitlyAnimatedWidgetState::did_update_widget(self, app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let animation = self.animation(app);
        let state = app.get(self);
        let (start, top, end) = (state.start, state.top, state.end);
        let (bottom, width, height) = (state.bottom, state.width, state.height);
        let text_direction = Directionality::of(app, context);
        let start = start.map(|start| start.evaluate(app, animation));
        let end = end.map(|end| end.evaluate(app, animation));
        let mut positioned =
            Positioned::directional(text_direction, start, end, self.widget(app).child.clone());
        if let Some(top) = top {
            positioned = positioned.top(top.evaluate(app, animation));
        }
        if let Some(bottom) = bottom {
            positioned = positioned.bottom(bottom.evaluate(app, animation));
        }
        if let Some(width) = width {
            positioned = positioned.width(width.evaluate(app, animation));
        }
        if let Some(height) = height {
            positioned = positioned.height(height.evaluate(app, animation));
        }
        positioned.into_widget()
    }
}

/// Animated version of `Transform.scale` which automatically transitions the child's
/// scale over a given duration whenever the given scale changes.
///
/// See also:
///
///  * [`AnimatedRotation`], for animating the rotation of a child.
///  * `ScaleTransition`, an explicitly animated version of this widget, where
///    an `Animation` is provided by the caller instead of being built in.
#[derive(Debug)]
pub struct AnimatedScale {
    implicitly_animated_widget: ImplicitlyAnimatedWidgetData,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,

    /// The target scale.
    pub scale: f64,

    /// The alignment of the origin of the coordinate system in which the scale
    /// takes place, relative to the size of the box.
    ///
    /// For example, to set the origin of the scale to bottom middle, you can use
    /// an alignment of (0.0, 1.0).
    pub alignment: Alignment,
}

impl AnimatedScale {
    /// Creates a widget that animates its scale implicitly.
    pub fn new(scale: f64, duration: Duration) -> AnimatedScale {
        AnimatedScale {
            implicitly_animated_widget: ImplicitlyAnimatedWidgetData::new(duration),
            child: None,
            scale,
            alignment: Alignment::CENTER,
        }
    }

    /// Dart `AnimatedScale(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> AnimatedScale {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `AnimatedScale(alignment:)`.
    pub fn alignment(mut self, alignment: Alignment) -> AnimatedScale {
        self.alignment = alignment;
        self
    }

    crate::implicitly_animated_widget_setters!(AnimatedScale);
}

impl ImplicitlyAnimatedWidget for AnimatedScale {
    crate::implicitly_animated_widget_accessors!();
}

impl StatefulWidget for AnimatedScale {
    type State = AnimatedScaleState;

    fn key(&self) -> Option<&KeyRef> {
        self.implicitly_animated_widget.key.as_ref()
    }

    fn create_state(&self) -> AnimatedScaleState {
        AnimatedScaleState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData::new(),
            scale: None,
            scale_animation: None,
        }
    }
}

/// Dart's `_AnimatedScaleState`.
pub struct AnimatedScaleState {
    state: StateData<AnimatedScale>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData,
    scale: Option<Handle<Tween<f64>>>,
    scale_animation: Option<AnyAnimation<f64>>,
}

impl SingleTickerProviderStateMixin for AnimatedScaleState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl TickerProviderObject for AnimatedScaleState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ImplicitlyAnimatedWidgetState for AnimatedScaleState {
    crate::implicitly_animated_widget_state_accessors!();

    fn for_each_tween<V: TweenVisitor>(self: Handle<Self>, app: &mut App, visitor: &mut V) {
        let (tween, target) = (app.get(self).scale, Some(self.widget(app).scale));
        app.get_mut(self).scale = visitor.visit(app, tween, target, double_tween);
    }

    fn did_update_tweens(self: Handle<Self>, app: &mut App) {
        let scale = app.get(self).scale.expect("for_each_tween made the tween");
        let animation = self.animation(app);
        let scale_animation = animation.drive(app, scale);
        app.get_mut(self).scale_animation = Some(scale_animation);
    }
}

impl State for AnimatedScaleState {
    type Widget = AnimatedScale;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::init_state(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &AnimatedScale) {
        ImplicitlyAnimatedWidgetState::did_update_widget(self, app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let scale = app
            .get(self)
            .scale_animation
            .expect("did_update_tweens drove the animation");
        let widget = self.widget(app);
        let (alignment, child) = (widget.alignment, widget.child.clone());
        let mut transition = ScaleTransition::new(scale).alignment(alignment);
        if let Some(child) = child {
            transition = transition.child(child);
        }
        transition.into_widget()
    }
}

/// Animated version of `Transform.rotate` which automatically transitions the child's
/// rotation over a given duration whenever the given rotation changes.
///
/// See also:
///
///  * [`AnimatedScale`], for animating the scale of a child.
///  * `RotationTransition`, an explicitly animated version of this widget, where
///    an `Animation` is provided by the caller instead of being built in.
#[derive(Debug)]
pub struct AnimatedRotation {
    implicitly_animated_widget: ImplicitlyAnimatedWidgetData,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,

    /// The animation that controls the rotation of the child.
    ///
    /// If the current value of the turns animation is v, the child will be
    /// rotated v * 2 * pi radians before being painted.
    pub turns: f64,

    /// The alignment of the origin of the coordinate system in which the rotation
    /// takes place, relative to the size of the box.
    ///
    /// For example, to set the origin of the rotation to bottom middle, you can use
    /// an alignment of (0.0, 1.0).
    pub alignment: Alignment,
}

impl AnimatedRotation {
    /// Creates a widget that animates its rotation implicitly.
    pub fn new(turns: f64, duration: Duration) -> AnimatedRotation {
        AnimatedRotation {
            implicitly_animated_widget: ImplicitlyAnimatedWidgetData::new(duration),
            child: None,
            turns,
            alignment: Alignment::CENTER,
        }
    }

    /// Dart `AnimatedRotation(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> AnimatedRotation {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `AnimatedRotation(alignment:)`.
    pub fn alignment(mut self, alignment: Alignment) -> AnimatedRotation {
        self.alignment = alignment;
        self
    }

    crate::implicitly_animated_widget_setters!(AnimatedRotation);
}

impl ImplicitlyAnimatedWidget for AnimatedRotation {
    crate::implicitly_animated_widget_accessors!();
}

impl StatefulWidget for AnimatedRotation {
    type State = AnimatedRotationState;

    fn key(&self) -> Option<&KeyRef> {
        self.implicitly_animated_widget.key.as_ref()
    }

    fn create_state(&self) -> AnimatedRotationState {
        AnimatedRotationState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData::new(),
            turns: None,
            turns_animation: None,
        }
    }
}

/// Dart's `_AnimatedRotationState`.
pub struct AnimatedRotationState {
    state: StateData<AnimatedRotation>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData,
    turns: Option<Handle<Tween<f64>>>,
    turns_animation: Option<AnyAnimation<f64>>,
}

impl SingleTickerProviderStateMixin for AnimatedRotationState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl TickerProviderObject for AnimatedRotationState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ImplicitlyAnimatedWidgetState for AnimatedRotationState {
    crate::implicitly_animated_widget_state_accessors!();

    fn for_each_tween<V: TweenVisitor>(self: Handle<Self>, app: &mut App, visitor: &mut V) {
        let (tween, target) = (app.get(self).turns, Some(self.widget(app).turns));
        app.get_mut(self).turns = visitor.visit(app, tween, target, double_tween);
    }

    fn did_update_tweens(self: Handle<Self>, app: &mut App) {
        let turns = app.get(self).turns.expect("for_each_tween made the tween");
        let animation = self.animation(app);
        let turns_animation = animation.drive(app, turns);
        app.get_mut(self).turns_animation = Some(turns_animation);
    }
}

impl State for AnimatedRotationState {
    type Widget = AnimatedRotation;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::init_state(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &AnimatedRotation) {
        ImplicitlyAnimatedWidgetState::did_update_widget(self, app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let turns = app
            .get(self)
            .turns_animation
            .expect("did_update_tweens drove the animation");
        let widget = self.widget(app);
        let (alignment, child) = (widget.alignment, widget.child.clone());
        let mut transition = RotationTransition::new(turns).alignment(alignment);
        if let Some(child) = child {
            transition = transition.child(child);
        }
        transition.into_widget()
    }
}

/// Widget which automatically transitions the child's
/// offset relative to its normal position whenever the given offset changes.
///
/// The translation is expressed as an [`Offset`] scaled to the child's size. For
/// example, an [`Offset`] with a `dx` of 0.25 will result in a horizontal
/// translation of one quarter the width of the child.
///
/// See also:
///
///  * [`AnimatedPositioned`], which, as a child of a `Stack`, automatically
///    transitions its child's position over a given duration whenever the given
///    position changes.
///  * [`AnimatedAlign`], which automatically transitions its child's
///    position over a given duration whenever the given [`AnimatedAlign::alignment`]
///    changes.
#[derive(Debug)]
pub struct AnimatedSlide {
    implicitly_animated_widget: ImplicitlyAnimatedWidgetData,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,

    /// The target offset.
    ///
    /// The child will be translated horizontally by `width * dx` and vertically by
    /// `height * dy`.
    pub offset: Offset,
}

impl AnimatedSlide {
    /// Creates a widget that animates its offset translation implicitly.
    pub fn new(offset: Offset, duration: Duration) -> AnimatedSlide {
        AnimatedSlide {
            implicitly_animated_widget: ImplicitlyAnimatedWidgetData::new(duration),
            child: None,
            offset,
        }
    }

    /// Dart `AnimatedSlide(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> AnimatedSlide {
        self.child = Some(child.into_widget());
        self
    }

    crate::implicitly_animated_widget_setters!(AnimatedSlide);
}

impl ImplicitlyAnimatedWidget for AnimatedSlide {
    crate::implicitly_animated_widget_accessors!();
}

impl StatefulWidget for AnimatedSlide {
    type State = AnimatedSlideState;

    fn key(&self) -> Option<&KeyRef> {
        self.implicitly_animated_widget.key.as_ref()
    }

    fn create_state(&self) -> AnimatedSlideState {
        AnimatedSlideState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData::new(),
            offset: None,
            offset_animation: None,
        }
    }
}

/// Dart's `_AnimatedSlideState`.
pub struct AnimatedSlideState {
    state: StateData<AnimatedSlide>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData,
    offset: Option<Handle<Tween<Offset>>>,
    offset_animation: Option<AnyAnimation<Offset>>,
}

impl SingleTickerProviderStateMixin for AnimatedSlideState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl TickerProviderObject for AnimatedSlideState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ImplicitlyAnimatedWidgetState for AnimatedSlideState {
    crate::implicitly_animated_widget_state_accessors!();

    fn for_each_tween<V: TweenVisitor>(self: Handle<Self>, app: &mut App, visitor: &mut V) {
        let (tween, target) = (app.get(self).offset, Some(self.widget(app).offset));
        app.get_mut(self).offset = visitor.visit(app, tween, target, offset_tween);
    }

    fn did_update_tweens(self: Handle<Self>, app: &mut App) {
        let offset = app.get(self).offset.expect("for_each_tween made the tween");
        let animation = self.animation(app);
        let offset_animation = animation.drive(app, offset);
        app.get_mut(self).offset_animation = Some(offset_animation);
    }
}

impl State for AnimatedSlideState {
    type Widget = AnimatedSlide;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::init_state(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &AnimatedSlide) {
        ImplicitlyAnimatedWidgetState::did_update_widget(self, app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let position = app
            .get(self)
            .offset_animation
            .expect("did_update_tweens drove the animation");
        let child = self.widget(app).child.clone();
        let mut transition = SlideTransition::new(position);
        if let Some(child) = child {
            transition = transition.child(child);
        }
        transition.into_widget()
    }
}

/// Animated version of `Opacity` which automatically transitions the child's
/// opacity over a given duration whenever the given opacity changes.
///
/// Animating an opacity is relatively expensive because it requires painting
/// the child into an intermediate buffer.
///
/// ## Hit testing
///
/// Setting the [`opacity`](Self::opacity) to zero does not prevent hit testing from being
/// applied to the descendants of the [`AnimatedOpacity`] widget. This can be
/// confusing for the user, who may not see anything, and may believe the area
/// of the interface where the [`AnimatedOpacity`] is hiding a widget to be
/// non-interactive.
///
/// To avoid such problems, it is generally a good idea to use an
/// `IgnorePointer` widget when setting the [`opacity`](Self::opacity) to zero. This prevents
/// interactions with any children in the subtree when the [`child`](Self::child) is animating
/// away.
///
/// See also:
///
///  * `FadeTransition`, an explicitly animated version of this widget, where
///    an `Animation` is provided by the caller instead of being built in.
#[derive(Debug)]
pub struct AnimatedOpacity {
    implicitly_animated_widget: ImplicitlyAnimatedWidgetData,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,

    /// The target opacity.
    ///
    /// An opacity of 1.0 is fully opaque. An opacity of 0.0 is fully transparent
    /// (i.e., invisible).
    pub opacity: f64,
}

impl AnimatedOpacity {
    /// Creates a widget that animates its opacity implicitly.
    ///
    /// The [`opacity`](Self::opacity) argument must be between zero and one, inclusive.
    pub fn new(opacity: f64, duration: Duration) -> AnimatedOpacity {
        debug_assert!((0.0..=1.0).contains(&opacity));
        AnimatedOpacity {
            implicitly_animated_widget: ImplicitlyAnimatedWidgetData::new(duration),
            child: None,
            opacity,
        }
    }

    /// Dart `AnimatedOpacity(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> AnimatedOpacity {
        self.child = Some(child.into_widget());
        self
    }

    crate::implicitly_animated_widget_setters!(AnimatedOpacity);
}

impl ImplicitlyAnimatedWidget for AnimatedOpacity {
    crate::implicitly_animated_widget_accessors!();
}

impl StatefulWidget for AnimatedOpacity {
    type State = AnimatedOpacityState;

    fn key(&self) -> Option<&KeyRef> {
        self.implicitly_animated_widget.key.as_ref()
    }

    fn create_state(&self) -> AnimatedOpacityState {
        AnimatedOpacityState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData::new(),
            opacity: None,
            opacity_animation: None,
        }
    }
}

/// Dart's `_AnimatedOpacityState`.
pub struct AnimatedOpacityState {
    state: StateData<AnimatedOpacity>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData,
    opacity: Option<Handle<Tween<f64>>>,
    opacity_animation: Option<AnyAnimation<f64>>,
}

impl SingleTickerProviderStateMixin for AnimatedOpacityState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl TickerProviderObject for AnimatedOpacityState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ImplicitlyAnimatedWidgetState for AnimatedOpacityState {
    crate::implicitly_animated_widget_state_accessors!();

    fn for_each_tween<V: TweenVisitor>(self: Handle<Self>, app: &mut App, visitor: &mut V) {
        let (tween, target) = (app.get(self).opacity, Some(self.widget(app).opacity));
        app.get_mut(self).opacity = visitor.visit(app, tween, target, double_tween);
    }

    fn did_update_tweens(self: Handle<Self>, app: &mut App) {
        let opacity = app
            .get(self)
            .opacity
            .expect("for_each_tween made the tween");
        let animation = self.animation(app);
        let opacity_animation = animation.drive(app, opacity);
        app.get_mut(self).opacity_animation = Some(opacity_animation);
    }
}

impl State for AnimatedOpacityState {
    type Widget = AnimatedOpacity;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::init_state(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &AnimatedOpacity) {
        ImplicitlyAnimatedWidgetState::did_update_widget(self, app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let opacity = app
            .get(self)
            .opacity_animation
            .expect("did_update_tweens drove the animation");
        let child = self.widget(app).child.clone();
        let mut transition = FadeTransition::new(opacity);
        if let Some(child) = child {
            transition = transition.child(child);
        }
        transition.into_widget()
    }
}

/// Animated version of [`DefaultTextStyle`] which automatically transitions the
/// default text style (the text style to apply to descendant `Text` widgets
/// without explicit style) over a given duration whenever the given style
/// changes.
///
/// The [`text_align`](Self::text_align), [`soft_wrap`](Self::soft_wrap),
/// [`overflow`](Self::overflow), [`max_lines`](Self::max_lines),
/// [`text_width_basis`](Self::text_width_basis) and
/// [`text_height_behavior`](Self::text_height_behavior) properties are not animated and take
/// effect immediately when changed.
///
/// For the animation, you can choose a [`curve`](ImplicitlyAnimatedWidgetData::curve) as
/// well as a [`duration`](ImplicitlyAnimatedWidgetData::duration) and the widget will
/// automatically animate to the new default text style. If you require more control over the
/// animation (e.g. if you want to stop it mid-animation), consider using a
/// `DefaultTextStyleTransition` instead.
#[derive(Debug)]
pub struct AnimatedDefaultTextStyle {
    implicitly_animated_widget: ImplicitlyAnimatedWidgetData,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,

    /// The target text style.
    ///
    /// When this property is changed, the style will be animated over
    /// [`duration`](ImplicitlyAnimatedWidgetData::duration) time.
    pub style: TextStyle,

    /// How the text should be aligned horizontally.
    ///
    /// This property takes effect immediately when changed, it is not animated.
    pub text_align: Option<TextAlign>,

    /// Whether the text should break at soft line breaks.
    ///
    /// This property takes effect immediately when changed, it is not animated.
    ///
    /// See [`DefaultTextStyle::soft_wrap`] for more details.
    pub soft_wrap: bool,

    /// How visual overflow should be handled.
    ///
    /// This property takes effect immediately when changed, it is not animated.
    pub overflow: TextOverflow,

    /// An optional maximum number of lines for the text to span, wrapping if necessary.
    ///
    /// This property takes effect immediately when changed, it is not animated.
    ///
    /// See [`DefaultTextStyle::max_lines`] for more details.
    pub max_lines: Option<i32>,

    /// The strategy to use when calculating the width of the text.
    pub text_width_basis: TextWidthBasis,

    /// How each line of text should be vertically aligned within its line box.
    pub text_height_behavior: Option<TextHeightBehavior>,
}

impl AnimatedDefaultTextStyle {
    /// Creates a widget that animates the default text style implicitly.
    pub fn new<K>(
        child: impl IntoWidget<K>,
        style: TextStyle,
        duration: Duration,
    ) -> AnimatedDefaultTextStyle {
        AnimatedDefaultTextStyle {
            implicitly_animated_widget: ImplicitlyAnimatedWidgetData::new(duration),
            child: child.into_widget(),
            style,
            text_align: None,
            soft_wrap: true,
            overflow: TextOverflow::Clip,
            max_lines: None,
            text_width_basis: TextWidthBasis::Parent,
            text_height_behavior: None,
        }
    }

    /// Dart `AnimatedDefaultTextStyle(textAlign:)`.
    pub fn text_align(mut self, text_align: TextAlign) -> AnimatedDefaultTextStyle {
        self.text_align = Some(text_align);
        self
    }

    /// Dart `AnimatedDefaultTextStyle(softWrap:)`.
    pub fn soft_wrap(mut self, soft_wrap: bool) -> AnimatedDefaultTextStyle {
        self.soft_wrap = soft_wrap;
        self
    }

    /// Dart `AnimatedDefaultTextStyle(overflow:)`.
    pub fn overflow(mut self, overflow: TextOverflow) -> AnimatedDefaultTextStyle {
        self.overflow = overflow;
        self
    }

    /// Dart `AnimatedDefaultTextStyle(maxLines:)`.
    pub fn max_lines(mut self, max_lines: i32) -> AnimatedDefaultTextStyle {
        debug_assert!(max_lines > 0);
        self.max_lines = Some(max_lines);
        self
    }

    /// Dart `AnimatedDefaultTextStyle(textWidthBasis:)`.
    pub fn text_width_basis(
        mut self,
        text_width_basis: TextWidthBasis,
    ) -> AnimatedDefaultTextStyle {
        self.text_width_basis = text_width_basis;
        self
    }

    /// Dart `AnimatedDefaultTextStyle(textHeightBehavior:)`.
    pub fn text_height_behavior(
        mut self,
        text_height_behavior: TextHeightBehavior,
    ) -> AnimatedDefaultTextStyle {
        self.text_height_behavior = Some(text_height_behavior);
        self
    }

    crate::implicitly_animated_widget_setters!(AnimatedDefaultTextStyle);
}

impl ImplicitlyAnimatedWidget for AnimatedDefaultTextStyle {
    crate::implicitly_animated_widget_accessors!();
}

impl StatefulWidget for AnimatedDefaultTextStyle {
    type State = AnimatedDefaultTextStyleState;

    fn key(&self) -> Option<&KeyRef> {
        self.implicitly_animated_widget.key.as_ref()
    }

    fn create_state(&self) -> AnimatedDefaultTextStyleState {
        AnimatedDefaultTextStyleState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData::new(),
            style: None,
        }
    }
}

/// Dart's `_AnimatedDefaultTextStyleState`.
pub struct AnimatedDefaultTextStyleState {
    state: StateData<AnimatedDefaultTextStyle>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData,
    style: Option<TextStyleTween>,
}

impl SingleTickerProviderStateMixin for AnimatedDefaultTextStyleState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl TickerProviderObject for AnimatedDefaultTextStyleState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ImplicitlyAnimatedWidgetState for AnimatedDefaultTextStyleState {
    crate::implicitly_animated_widget_state_accessors!();

    fn for_each_tween<V: TweenVisitor>(self: Handle<Self>, app: &mut App, visitor: &mut V) {
        let tween = app.get(self).style.clone();
        let target = self.widget(app).style.clone();
        app.get_mut(self).style = visitor.visit(app, tween, Some(target), |_app, value| {
            TextStyleTween::new(Some(value.clone()), None)
        });
    }
}

impl AnimatedWidgetBaseState for AnimatedDefaultTextStyleState {}

impl State for AnimatedDefaultTextStyleState {
    type Widget = AnimatedDefaultTextStyle;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        AnimatedWidgetBaseState::init_state(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &AnimatedDefaultTextStyle) {
        ImplicitlyAnimatedWidgetState::did_update_widget(self, app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let animation = self.animation(app);
        let style = app
            .get(self)
            .style
            .clone()
            .expect("for_each_tween made the tween");
        let widget = self.widget(app);
        let (text_align, soft_wrap, overflow) =
            (widget.text_align, widget.soft_wrap, widget.overflow);
        let (max_lines, text_width_basis) = (widget.max_lines, widget.text_width_basis);
        let text_height_behavior = widget.text_height_behavior;
        let child = widget.child.clone();
        let mut default_text_style = DefaultTextStyle::new(style.evaluate(app, animation), child)
            .soft_wrap(soft_wrap)
            .overflow(overflow)
            .text_width_basis(text_width_basis);
        if let Some(text_align) = text_align {
            default_text_style = default_text_style.text_align(text_align);
        }
        if let Some(max_lines) = max_lines {
            default_text_style = default_text_style.max_lines(max_lines);
        }
        if let Some(text_height_behavior) = text_height_behavior {
            default_text_style = default_text_style.text_height_behavior(text_height_behavior);
        }
        default_text_style.into_widget()
    }
}

/// Animated version of [`FractionallySizedBox`] which automatically transitions the
/// child's size over a given duration whenever the given
/// [`width_factor`](Self::width_factor) or [`height_factor`](Self::height_factor) changes,
/// as well as the position whenever the given [`alignment`](Self::alignment) changes.
///
/// For the animation, you can choose a [`curve`](ImplicitlyAnimatedWidgetData::curve) as
/// well as a [`duration`](ImplicitlyAnimatedWidgetData::duration) and the widget will
/// automatically animate to the new target [`width_factor`](Self::width_factor) or
/// [`height_factor`](Self::height_factor).
///
/// See also:
///
///  * [`AnimatedAlign`], which is an implicitly animated version of [`Align`].
///  * [`AnimatedContainer`], which can transition more values at once.
///  * [`AnimatedSlide`], which can animate the translation of child by a given offset
///    relative to its size.
///  * [`AnimatedPositioned`], which, as a child of a `Stack`, automatically
///    transitions its child's position over a given duration whenever the given
///    position changes.
#[derive(Debug)]
pub struct AnimatedFractionallySizedBox {
    implicitly_animated_widget: ImplicitlyAnimatedWidgetData,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,

    /// See [`FractionallySizedBox::height_factor`].
    pub height_factor: Option<f64>,

    /// See [`FractionallySizedBox::width_factor`].
    pub width_factor: Option<f64>,

    /// See [`FractionallySizedBox::alignment`].
    pub alignment: AlignmentGeometry,
}

impl AnimatedFractionallySizedBox {
    /// Creates a widget that sizes its child to a fraction of the total available
    /// space that animates implicitly, and positions its child by an alignment
    /// that animates implicitly.
    pub fn new(duration: Duration) -> AnimatedFractionallySizedBox {
        AnimatedFractionallySizedBox {
            implicitly_animated_widget: ImplicitlyAnimatedWidgetData::new(duration),
            child: None,
            height_factor: None,
            width_factor: None,
            alignment: AlignmentGeometry::CENTER,
        }
    }

    /// Dart `AnimatedFractionallySizedBox(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> AnimatedFractionallySizedBox {
        self.alignment = alignment;
        self
    }

    /// Dart `AnimatedFractionallySizedBox(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> AnimatedFractionallySizedBox {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `AnimatedFractionallySizedBox(heightFactor:)`.
    pub fn height_factor(mut self, height_factor: f64) -> AnimatedFractionallySizedBox {
        debug_assert!(height_factor >= 0.0);
        self.height_factor = Some(height_factor);
        self
    }

    /// Dart `AnimatedFractionallySizedBox(widthFactor:)`.
    pub fn width_factor(mut self, width_factor: f64) -> AnimatedFractionallySizedBox {
        debug_assert!(width_factor >= 0.0);
        self.width_factor = Some(width_factor);
        self
    }

    crate::implicitly_animated_widget_setters!(AnimatedFractionallySizedBox);
}

impl ImplicitlyAnimatedWidget for AnimatedFractionallySizedBox {
    crate::implicitly_animated_widget_accessors!();
}

impl StatefulWidget for AnimatedFractionallySizedBox {
    type State = AnimatedFractionallySizedBoxState;

    fn key(&self) -> Option<&KeyRef> {
        self.implicitly_animated_widget.key.as_ref()
    }

    fn create_state(&self) -> AnimatedFractionallySizedBoxState {
        AnimatedFractionallySizedBoxState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData::new(),
            alignment: None,
            height_factor_tween: None,
            width_factor_tween: None,
        }
    }
}

/// Dart's `_AnimatedFractionallySizedBoxState`.
pub struct AnimatedFractionallySizedBoxState {
    state: StateData<AnimatedFractionallySizedBox>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData,
    alignment: Option<AlignmentGeometryTween>,
    height_factor_tween: Option<Handle<Tween<f64>>>,
    width_factor_tween: Option<Handle<Tween<f64>>>,
}

impl SingleTickerProviderStateMixin for AnimatedFractionallySizedBoxState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl TickerProviderObject for AnimatedFractionallySizedBoxState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ImplicitlyAnimatedWidgetState for AnimatedFractionallySizedBoxState {
    crate::implicitly_animated_widget_state_accessors!();

    fn for_each_tween<V: TweenVisitor>(self: Handle<Self>, app: &mut App, visitor: &mut V) {
        let tween = app.get(self).alignment.clone();
        let target = Some(Some(self.widget(app).alignment));
        app.get_mut(self).alignment = visitor.visit(app, tween, target, |_app, value| {
            AlignmentGeometryTween::new(*value, None)
        });
        if self.widget(app).height_factor.is_some() {
            let (tween, target) = (
                app.get(self).height_factor_tween,
                self.widget(app).height_factor,
            );
            app.get_mut(self).height_factor_tween = visitor.visit(app, tween, target, double_tween);
        }
        if self.widget(app).width_factor.is_some() {
            let (tween, target) = (
                app.get(self).width_factor_tween,
                self.widget(app).width_factor,
            );
            app.get_mut(self).width_factor_tween = visitor.visit(app, tween, target, double_tween);
        }
    }
}

impl AnimatedWidgetBaseState for AnimatedFractionallySizedBoxState {}

impl State for AnimatedFractionallySizedBoxState {
    type Widget = AnimatedFractionallySizedBox;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        AnimatedWidgetBaseState::init_state(self, app);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &AnimatedFractionallySizedBox,
    ) {
        ImplicitlyAnimatedWidgetState::did_update_widget(self, app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let animation = self.animation(app);
        let state = app.get(self);
        let alignment = state
            .alignment
            .clone()
            .expect("for_each_tween made the tween");
        let (height_factor_tween, width_factor_tween) =
            (state.height_factor_tween, state.width_factor_tween);
        let child = self.widget(app).child.clone();
        let alignment = alignment
            .evaluate(app, animation)
            .expect("the tween is built from the widget's alignment");
        let mut box_ = FractionallySizedBox::new().alignment(alignment);
        if let Some(height_factor) = height_factor_tween {
            box_ = box_.height_factor(height_factor.evaluate(app, animation));
        }
        if let Some(width_factor) = width_factor_tween {
            box_ = box_.width_factor(width_factor.evaluate(app, animation));
        }
        if let Some(child) = child {
            box_ = box_.child(child);
        }
        box_.into_widget()
    }
}

/// Dart's `(dynamic value) => Tween<double>(begin: value as double)`, which every state that
/// animates a number passes to the visitor.
fn double_tween(app: &mut App, value: &f64) -> Handle<Tween<f64>> {
    Tween::new(app, Some(*value), None)
}

/// Dart's `(dynamic value) => Tween<Offset>(begin: value as Offset)`.
fn offset_tween(app: &mut App, value: &Offset) -> Handle<Tween<Offset>> {
    Tween::new(app, Some(*value), None)
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use reveal_embedder::{Size, TextDirection};
    use reveal_rendering::{RenderAnimatedOpacity, RenderAnimatedOpacityMixin};

    use super::*;
    use crate::binding::WidgetsBinding;
    use crate::framework::GlobalKey;
    use crate::test_harness::{binding_app, binding_mount, binding_pump};
    use crate::view::View;
    use crate::widgets::basic::{Align, Builder, Directionality, SizedBox, Stack};

    const DURATION: Duration = Duration::from_millis(100);

    /// Rebuilds the mounted tree with a new root child and runs the frame that updates it.
    fn rebuild(app: &mut App, child: WidgetRef, at: Duration) {
        let view = app
            .platform()
            .implicit_view()
            .expect("an app from binding_app");
        let root = View::new(view, child).into_widget();
        WidgetsBinding::instance(app).attach_root_widget(app, root);
        binding_pump(app, at);
    }

    fn render_box_of(app: &mut App, key: &GlobalKey) -> reveal_rendering::AnyRenderBox {
        key.current_context(app)
            .expect("the widget is mounted")
            .find_render_object(app)
            .expect("a mounted render object")
            .as_box()
            .expect("a box")
    }

    fn positioned_tree(key: &Rc<GlobalKey>, width: f64, on_end: Option<Listener>) -> WidgetRef {
        let mut positioned = AnimatedPositioned::new(SizedBox::expand(), DURATION)
            .key(key.clone())
            .left(0.0)
            .top(0.0)
            .width(width)
            .height(10.0);
        if let Some(on_end) = on_end {
            positioned = positioned.on_end(on_end);
        }
        Directionality::new(
            TextDirection::Ltr,
            Stack::new().children([positioned.into_widget()]),
        )
        .into_widget()
    }

    #[test]
    fn an_animated_positioned_animates_its_box_and_calls_on_end_once() {
        let mut app = binding_app();
        let key = Rc::new(GlobalKey::new());
        let ends = Rc::new(Cell::new(0));
        let counted = Rc::clone(&ends);
        let on_end = Listener::new(move |_app| counted.set(counted.get() + 1));
        binding_mount(&mut app, positioned_tree(&key, 20.0, Some(on_end.clone())));
        assert_eq!(
            render_box_of(&mut app, &key).size(&app),
            Size::new(20.0, 10.0),
            "the first build does not animate"
        );

        rebuild(
            &mut app,
            positioned_tree(&key, 40.0, Some(on_end)),
            Duration::ZERO,
        );
        assert_eq!(
            render_box_of(&mut app, &key).size(&app).width(),
            20.0,
            "the frame that starts the animation still shows the old width"
        );

        binding_pump(&mut app, Duration::from_millis(50));
        let halfway = render_box_of(&mut app, &key).size(&app).width();
        assert!(
            (halfway - 30.0).abs() < 0.001,
            "halfway between 20 and 40, not {halfway}"
        );
        assert_eq!(ends.get(), 0, "the animation is still running");

        binding_pump(&mut app, Duration::from_millis(100));
        assert_eq!(render_box_of(&mut app, &key).size(&app).width(), 40.0);
        assert_eq!(
            ends.get(),
            0,
            "the controller completes on the first tick past the duration"
        );

        binding_pump(&mut app, Duration::from_millis(150));
        assert_eq!(ends.get(), 1);

        binding_pump(&mut app, Duration::from_millis(200));
        assert_eq!(ends.get(), 1, "on_end fires once per animation");
    }

    fn opacity_tree(key: &Rc<GlobalKey>, opacity: f64) -> WidgetRef {
        AnimatedOpacity::new(opacity, DURATION)
            .key(key.clone())
            .child(SizedBox::square(Some(10.0)))
            .into_widget()
    }

    fn opacity_of(app: &mut App, key: &GlobalKey) -> f64 {
        let render_object = key
            .current_context(app)
            .expect("the widget is mounted")
            .find_render_object(app)
            .expect("a mounted render object")
            .downcast::<RenderAnimatedOpacity>(app)
            .expect("a RenderAnimatedOpacity");
        RenderAnimatedOpacityMixin::opacity(render_object, app).value(app)
    }

    #[test]
    fn an_animated_opacity_reaches_its_target_after_the_duration() {
        let mut app = binding_app();
        let key = Rc::new(GlobalKey::new());
        binding_mount(&mut app, opacity_tree(&key, 1.0));
        assert_eq!(opacity_of(&mut app, &key), 1.0);

        rebuild(&mut app, opacity_tree(&key, 0.0), Duration::ZERO);
        assert_eq!(opacity_of(&mut app, &key), 1.0);

        binding_pump(&mut app, Duration::from_millis(50));
        let halfway = opacity_of(&mut app, &key);
        assert!((halfway - 0.5).abs() < 0.001, "{halfway}");

        binding_pump(&mut app, Duration::from_millis(100));
        assert_eq!(opacity_of(&mut app, &key), 0.0);
    }

    #[test]
    fn a_new_target_mid_flight_retargets_from_the_value_on_screen() {
        let mut app = binding_app();
        let key = Rc::new(GlobalKey::new());
        binding_mount(&mut app, opacity_tree(&key, 0.0));
        rebuild(&mut app, opacity_tree(&key, 1.0), Duration::ZERO);
        binding_pump(&mut app, Duration::from_millis(50));
        let midpoint = opacity_of(&mut app, &key);
        assert!((midpoint - 0.5).abs() < 0.001, "{midpoint}");

        rebuild(&mut app, opacity_tree(&key, 0.0), Duration::from_millis(50));
        assert!(
            (opacity_of(&mut app, &key) - midpoint).abs() < 0.001,
            "the restarted animation still shows the value on screen"
        );

        binding_pump(&mut app, Duration::from_millis(100));
        let quarter = opacity_of(&mut app, &key);
        assert!(
            (quarter - 0.25).abs() < 0.001,
            "the new tween runs from the value on screen, not from 1.0: {quarter}"
        );

        binding_pump(&mut app, Duration::from_millis(150));
        assert_eq!(opacity_of(&mut app, &key), 0.0);
    }

    fn text_style_tree(font_size: f64, seen: &Rc<RefCell<Option<TextStyle>>>) -> WidgetRef {
        let seen = Rc::clone(seen);
        AnimatedDefaultTextStyle::new(
            Builder::new(move |app, context| {
                *seen.borrow_mut() = Some(DefaultTextStyle::of(app, context).style);
                SizedBox::square(Some(10.0)).into_widget()
            }),
            TextStyle::new().font_size(font_size),
            DURATION,
        )
        .into_widget()
    }

    #[test]
    fn an_animated_default_text_style_interpolates_the_style() {
        let mut app = binding_app();
        let seen: Rc<RefCell<Option<TextStyle>>> = Rc::new(RefCell::new(None));
        let font_size = || {
            seen.borrow()
                .as_ref()
                .expect("the builder ran")
                .font_size
                .expect("a font size")
        };
        binding_mount(&mut app, text_style_tree(10.0, &seen));
        assert_eq!(font_size(), 10.0);

        rebuild(&mut app, text_style_tree(20.0, &seen), Duration::ZERO);
        assert_eq!(font_size(), 10.0);

        binding_pump(&mut app, Duration::from_millis(50));
        let halfway = font_size();
        assert!((halfway - 15.0).abs() < 0.001, "{halfway}");

        binding_pump(&mut app, Duration::from_millis(100));
        assert_eq!(font_size(), 20.0);
    }

    #[test]
    fn a_target_that_did_not_change_leaves_the_controller_dismissed() {
        let mut app = binding_app();
        let key = Rc::new(GlobalKey::new());
        binding_mount(&mut app, opacity_tree(&key, 0.5));
        rebuild(&mut app, opacity_tree(&key, 0.5), Duration::ZERO);
        binding_pump(&mut app, Duration::from_millis(50));

        let state = key
            .current_state::<AnimatedOpacityState>(&mut app)
            .expect("the widget is mounted");
        assert!(
            state.controller(&app).is_dismissed(&app),
            "an unchanged target does not start the controller"
        );
        assert_eq!(opacity_of(&mut app, &key), 0.5);
    }

    #[test]
    fn an_animated_positioned_directional_resolves_start_against_the_text_direction() {
        let directional = |text_direction: TextDirection| {
            Directionality::new(
                text_direction,
                Stack::new().children([AnimatedPositionedDirectional::new(
                    SizedBox::expand(),
                    DURATION,
                )
                .start(10.0)
                .top(0.0)
                .width(20.0)
                .height(10.0)
                .into_widget()]),
            )
            .into_widget()
        };
        let mut app = binding_app();
        binding_mount(&mut app, directional(TextDirection::Ltr));
        let positioned = |app: &mut App| {
            let widget = WidgetsBinding::instance(app)
                .root_element(app)
                .expect("a mounted tree");
            let mut element = widget;
            loop {
                let widget = element.widget(app).clone();
                if let Some(positioned) = widget.as_any().downcast_ref::<Positioned>() {
                    return (positioned.left, positioned.right);
                }
                element = element.children(app)[0];
            }
        };
        assert_eq!(positioned(&mut app), (Some(10.0), None));

        rebuild(&mut app, directional(TextDirection::Rtl), Duration::ZERO);
        assert_eq!(positioned(&mut app), (None, Some(10.0)));
    }

    #[test]
    fn dispose_frees_the_arena_tweens_the_constructors_made() {
        let mut app = binding_app();
        let key = Rc::new(GlobalKey::new());
        binding_mount(&mut app, opacity_tree(&key, 0.5));
        let state = key
            .current_state::<AnimatedOpacityState>(&mut app)
            .expect("the widget is mounted");
        let tween = app
            .get(state)
            .opacity
            .expect("for_each_tween made the tween");
        assert!(app.contains(tween));

        rebuild(
            &mut app,
            SizedBox::square(Some(10.0)).into_widget(),
            Duration::ZERO,
        );
        assert!(!app.contains(tween), "dispose freed the arena tween");
    }

    #[test]
    fn a_text_style_tween_interpolates_the_style_fields() {
        let tween = TextStyleTween::new(
            Some(TextStyle::new().font_size(10.0)),
            Some(TextStyle::new().font_size(20.0)),
        );
        assert_eq!(tween.lerp(0.5).font_size, Some(15.0));
        let app = App::new();
        assert_eq!(tween.transform(&app, 0.0).font_size, Some(10.0));
        assert_eq!(tween.transform(&app, 1.0).font_size, Some(20.0));
    }

    #[test]
    fn a_box_constraints_tween_interpolates_the_constraints() {
        let tween = BoxConstraintsTween::new(
            Some(BoxConstraints::tight(Size::new(10.0, 10.0))),
            Some(BoxConstraints::tight(Size::new(20.0, 30.0))),
        );
        let app = App::new();
        let halfway = tween.transform(&app, 0.5);
        assert_eq!(halfway.max_width, 15.0);
        assert_eq!(halfway.max_height, 20.0);
    }

    fn container_tree(key: &Rc<GlobalKey>, width: f64, on_end: Option<Listener>) -> WidgetRef {
        let mut container = AnimatedContainer::new(DURATION)
            .key(key.clone())
            .width(width)
            .height(10.0);
        if let Some(on_end) = on_end {
            container = container.on_end(on_end);
        }
        container.into_widget()
    }

    #[test]
    fn an_animated_container_animates_its_width_and_calls_on_end_once() {
        let mut app = binding_app();
        let key = Rc::new(GlobalKey::new());
        let ends = Rc::new(Cell::new(0));
        let counted = Rc::clone(&ends);
        let on_end = Listener::new(move |_app| counted.set(counted.get() + 1));
        binding_mount(&mut app, container_tree(&key, 20.0, Some(on_end.clone())));
        assert_eq!(
            render_box_of(&mut app, &key).size(&app),
            Size::new(20.0, 10.0),
            "the first build does not animate"
        );

        rebuild(
            &mut app,
            container_tree(&key, 40.0, Some(on_end)),
            Duration::ZERO,
        );
        assert_eq!(
            render_box_of(&mut app, &key).size(&app).width(),
            20.0,
            "the frame that starts the animation still shows the old width"
        );

        binding_pump(&mut app, Duration::from_millis(50));
        let halfway = render_box_of(&mut app, &key).size(&app).width();
        assert!(
            (halfway - 30.0).abs() < 0.001,
            "halfway between 20 and 40, not {halfway}"
        );
        assert_eq!(ends.get(), 0, "the animation is still running");

        binding_pump(&mut app, Duration::from_millis(100));
        assert_eq!(render_box_of(&mut app, &key).size(&app).width(), 40.0);

        binding_pump(&mut app, Duration::from_millis(150));
        assert_eq!(ends.get(), 1);

        binding_pump(&mut app, Duration::from_millis(200));
        assert_eq!(ends.get(), 1, "on_end fires once per animation");
    }

    /// The global offset of the widget the key is on.
    fn offset_of(app: &mut App, key: &GlobalKey) -> Offset {
        render_box_of(app, key).local_to_global(app, Offset::ZERO, None)
    }

    fn padding_tree(key: &Rc<GlobalKey>, text_direction: TextDirection, start: f64) -> WidgetRef {
        Directionality::new(
            text_direction,
            AnimatedPadding::new(
                EdgeInsetsGeometry::from_steb(start, 0.0, 0.0, 0.0),
                DURATION,
            )
            .child(SizedBox::square(Some(10.0)).key(key.clone())),
        )
        .into_widget()
    }

    #[test]
    fn an_animated_padding_interpolates_a_directional_inset() {
        let mut app = binding_app();
        let key = Rc::new(GlobalKey::new());
        binding_mount(&mut app, padding_tree(&key, TextDirection::Ltr, 0.0));
        assert_eq!(offset_of(&mut app, &key), Offset::ZERO);

        rebuild(
            &mut app,
            padding_tree(&key, TextDirection::Ltr, 40.0),
            Duration::ZERO,
        );
        binding_pump(&mut app, Duration::from_millis(50));
        let halfway = offset_of(&mut app, &key).dx();
        assert!((halfway - 20.0).abs() < 0.001, "{halfway}");

        binding_pump(&mut app, Duration::from_millis(100));
        assert_eq!(offset_of(&mut app, &key).dx(), 40.0);

        let mut app = binding_app();
        binding_mount(&mut app, padding_tree(&key, TextDirection::Rtl, 40.0));
        assert_eq!(
            offset_of(&mut app, &key),
            Offset::ZERO,
            "an rtl start is an inset on the right"
        );
    }

    fn align_tree(key: &Rc<GlobalKey>, alignment: AlignmentGeometry) -> WidgetRef {
        AnimatedAlign::new(alignment, DURATION)
            .child(SizedBox::square(Some(10.0)).key(key.clone()))
            .into_widget()
    }

    #[test]
    fn an_animated_align_interpolates_the_alignment() {
        let mut app = binding_app();
        let key = Rc::new(GlobalKey::new());
        binding_mount(&mut app, align_tree(&key, AlignmentGeometry::TOP_LEFT));
        assert_eq!(offset_of(&mut app, &key), Offset::ZERO);

        rebuild(
            &mut app,
            align_tree(&key, AlignmentGeometry::BOTTOM_RIGHT),
            Duration::ZERO,
        );
        binding_pump(&mut app, Duration::from_millis(50));
        assert_eq!(offset_of(&mut app, &key), Offset::new(145.0, 95.0));

        binding_pump(&mut app, Duration::from_millis(100));
        assert_eq!(offset_of(&mut app, &key), Offset::new(290.0, 190.0));
    }

    fn slide_tree(key: &Rc<GlobalKey>, offset: Offset) -> WidgetRef {
        Align::new()
            .alignment(AlignmentGeometry::TOP_LEFT)
            .child(
                AnimatedSlide::new(offset, DURATION)
                    .child(SizedBox::square(Some(10.0)).key(key.clone())),
            )
            .into_widget()
    }

    #[test]
    fn an_animated_slide_moves_the_child_by_a_fraction_of_its_size() {
        let mut app = binding_app();
        let key = Rc::new(GlobalKey::new());
        binding_mount(&mut app, slide_tree(&key, Offset::ZERO));
        assert_eq!(offset_of(&mut app, &key), Offset::ZERO);

        rebuild(
            &mut app,
            slide_tree(&key, Offset::new(2.0, 0.0)),
            Duration::ZERO,
        );
        binding_pump(&mut app, Duration::from_millis(50));
        assert_eq!(offset_of(&mut app, &key), Offset::new(10.0, 0.0));

        binding_pump(&mut app, Duration::from_millis(100));
        assert_eq!(offset_of(&mut app, &key), Offset::new(20.0, 0.0));
    }

    #[test]
    fn a_matrix4_tween_round_trips_a_translation_and_a_scale() {
        let begin = Matrix4::IDENTITY;
        let end = Matrix4::translation(10.0, 20.0).then(&Matrix4::scale(3.0, 3.0));
        let tween = Matrix4Tween::new(Some(begin), Some(end));
        let app = App::new();
        assert_eq!(tween.transform(&app, 0.0), begin);
        assert_eq!(tween.transform(&app, 1.0), end);

        let (translation, _, scale) = decompose(tween.transform(&app, 0.5));
        assert!((translation.x - 5.0).abs() < 1e-5, "{translation:?}");
        assert!((translation.y - 10.0).abs() < 1e-5, "{translation:?}");
        assert!((scale.x - 2.0).abs() < 1e-5, "{scale:?}");
        assert!((scale.y - 2.0).abs() < 1e-5, "{scale:?}");
    }

    #[test]
    fn an_edge_insets_geometry_tween_crosses_the_two_kinds() {
        let tween = EdgeInsetsGeometryTween::new(
            Some(EdgeInsetsGeometry::from_ltrb(10.0, 0.0, 0.0, 0.0)),
            Some(EdgeInsetsGeometry::from_steb(20.0, 0.0, 0.0, 0.0)),
        );
        let app = App::new();
        let halfway = tween.transform(&app, 0.5);
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
    fn a_border_radius_tween_interpolates_the_radius() {
        use reveal_embedder::Radius;
        let tween = BorderRadiusTween::new(
            Some(BorderRadius::all(Radius::circular(0.0))),
            Some(BorderRadius::all(Radius::circular(10.0))),
        );
        let app = App::new();
        let halfway = tween.transform(&app, 0.5).expect("both ends are set");
        assert_eq!(halfway.top_left.x, 5.0);
    }
}
