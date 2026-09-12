//! Flutter counterpart: `widgets/transitions.dart`.
//!
//! `SliverFadeTransition` waits with the slivers, [`MatrixTransition`]'s `filterQuality`
//! with `RenderTransform`'s, and [`FadeTransition`]'s `alwaysIncludeSemantics` with
//! accessibility.

use std::f64::consts::PI;
use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_animation::{Animatable, AnyAnimation};
use inset_embedder::{Matrix4, Offset, Rect, Size, TextAlign, TextDirection};
use inset_foundation::{App, Handle, Listenable, Listener};
use inset_painting::{Alignment, AlignmentGeometry, Axis, Decoration, TextOverflow, TextStyle};
use inset_rendering::{
    AnyRenderObject, DecorationPosition, RelativeRect, RenderAnimatedOpacity,
    RenderAnimatedOpacityMixin, RenderBox, RenderHandle,
};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, RenderObjectWidget, SingleChildRenderObjectWidget, State,
    StateData, Stateful, StatefulWidget, WidgetRef, downcast_widget,
};
use crate::widgets::basic::{Align, ClipRect, FractionalTranslation, Positioned, Transform};
use crate::widgets::container::DecoratedBox;
use crate::widgets::text::DefaultTextStyle;

/// A widget that rebuilds when the given [`Listenable`] changes value.
///
/// [`AnimatedWidget`] is most commonly used with `Animation` objects, which are
/// [`Listenable`], but it can be used with any [`Listenable`], including
/// `ChangeNotifier` and `ValueNotifier`.
///
/// [`AnimatedWidget`] is most useful for widgets that are otherwise stateless. To
/// use [`AnimatedWidget`], implement this trait with its [`build`](Self::build) function;
/// the value becomes a tree node with `.into_widget()`, like every widget kind.
///
/// ## Relationship to `ImplicitlyAnimatedWidget`s
///
/// [`AnimatedWidget`]s (and their subclasses) take an explicit [`Listenable`] as
/// argument, which is usually an `Animation` derived from an
/// `AnimationController`. In most cases, the lifecycle of that
/// `AnimationController` has to be managed manually by the developer.
/// In contrast to that, `ImplicitlyAnimatedWidget`s (and their subclasses)
/// automatically manage their own internal `AnimationController` making those
/// classes easier to use as no external `Animation` has to be provided by the
/// developer. If you only need to set a target value for the animation and
/// configure its duration/curve, consider using (a subclass of)
/// `ImplicitlyAnimatedWidget`s instead of (a subclass of) this class.
///
/// ## Common animated widgets
///
/// A number of animated widgets ship with the framework. They are usually named
/// `FooTransition`, where `Foo` is the name of the non-animated
/// version of that widget. The subclasses of this class should not be confused
/// with subclasses of `ImplicitlyAnimatedWidget` (see above), which are usually
/// named `AnimatedFoo`. Commonly used animated widgets include:
///
///  * `ListenableBuilder`, which uses a builder pattern that is useful for
///    complex [`Listenable`] use cases.
///  * `AnimatedBuilder`, which uses a builder pattern that is useful for
///    complex `Animation` use cases.
///  * `AlignTransition`, which is an animated version of `Align`.
///  * `DecoratedBoxTransition`, which is an animated version of `DecoratedBox`.
///  * `DefaultTextStyleTransition`, which is an animated version of
///    `DefaultTextStyle`.
///  * `PositionedTransition`, which is an animated version of `Positioned`.
///  * `RelativePositionedTransition`, which is an animated version of
///    `Positioned`.
///  * `RotationTransition`, which animates the rotation of a widget.
///  * `ScaleTransition`, which animates the scale of a widget.
///  * `SizeTransition`, which animates its own size.
///  * `SlideTransition`, which animates the position of a widget relative to
///    its normal position.
///  * [`FadeTransition`], which is an animated version of `Opacity`.
///  * `AnimatedModalBarrier`, which is an animated version of `ModalBarrier`.
pub trait AnimatedWidget: Debug + 'static {
    /// See [`Widget::key`](crate::Widget::key).
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    /// The [`Listenable`] to which this widget is listening.
    ///
    /// Commonly an `Animation` or a `ChangeNotifier`.
    ///
    /// Dart compares the old and new widget's listenable by identity; here that is
    /// `Rc::ptr_eq`, so a widget that keeps one `Rc` and clones it into each rebuilt
    /// value keeps its subscription, and one that wraps a handle anew each call is
    /// re-subscribed on every update (the same listener set either way).
    fn listenable(&self) -> Rc<dyn Listenable>;

    /// Override this method to build widgets that depend on the state of the
    /// listenable (e.g., the current value of the animation).
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef;
}

/// The kind tag of [`IntoWidget`] for an [`AnimatedWidget`].
pub struct AnimatedKind;

/// The [`StatefulWidget`] an [`AnimatedWidget`] becomes; its state is Dart's
/// `_AnimatedState`. Subclasses typically do not override `createState`, so this is the
/// one state every animated widget gets.
pub struct Animated<W: AnimatedWidget>(pub W);

impl<W: AnimatedWidget> IntoWidget<AnimatedKind> for W {
    fn into_widget(self) -> WidgetRef {
        Rc::new(Stateful(Animated(self)))
    }
}

impl<W: AnimatedWidget> StatefulWidget for Animated<W> {
    type State = AnimatedState<W>;

    fn key(&self) -> Option<&KeyRef> {
        self.0.key()
    }

    fn create_state(&self) -> AnimatedState<W> {
        AnimatedState {
            state: StateData::new(),
        }
    }
}

impl<W: AnimatedWidget> Debug for Animated<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Dart's `_AnimatedState`.
pub struct AnimatedState<W: AnimatedWidget> {
    state: StateData<Animated<W>>,
}

impl<W: AnimatedWidget> State for AnimatedState<W> {
    type Widget = Animated<W>;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let listenable = self.widget(app).0.listenable();
        listenable.add_listener(app, Listener::handle_method(self, Self::handle_change));
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Animated<W>) {
        let listenable = self.widget(app).0.listenable();
        let old_listenable = old_widget.0.listenable();
        if !Rc::ptr_eq(&listenable, &old_listenable) {
            old_listenable
                .remove_listener(app, &Listener::handle_method(self, Self::handle_change));
            listenable.add_listener(app, Listener::handle_method(self, Self::handle_change));
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let listenable = self.widget(app).0.listenable();
        listenable.remove_listener(app, &Listener::handle_method(self, Self::handle_change));
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = context.widget(app).clone();
        downcast_widget::<Animated<W>>(&*widget)
            .expect("an AnimatedState builds its AnimatedWidget")
            .0
            .build(app, context)
    }
}

impl<W: AnimatedWidget> AnimatedState<W> {
    fn handle_change(self: Handle<Self>, app: &mut App) {
        if !self.mounted(app) {
            return;
        }
        self.set_state(app, |_state| {
            // The listenable's state is our build state, and it changed already.
        });
    }
}

/// Signature for a builder used to control a page's exit transition.
///
/// When a new route enters the stack, the `animation` argument is typically
/// used to control the enter and exit transition of the topmost route. The exit
/// transition of the route just below the new route is controlled with the
/// `secondary_animation`, which also controls the transition of the old route
/// when the topmost route is popped off the stack.
///
/// Typically used as the argument for `ModalRoute::delegated_transition`.
pub type DelegatedTransitionBuilder = Rc<
    dyn Fn(
        &mut App,
        BuildContext,
        AnyAnimation<f64>,
        AnyAnimation<f64>,
        bool,
        Option<&WidgetRef>,
    ) -> Option<WidgetRef>,
>;

/// Animates the position of a widget relative to its normal position.
///
/// The translation is expressed as an [`Offset`] scaled to the child's size. For
/// example, an [`Offset`] with a `dx` of 0.25 will result in a horizontal
/// translation of one quarter the width of the child.
///
/// By default, the offsets are applied in the coordinate system of the canvas
/// (so positive x offsets move the child towards the right). If a
/// [`text_direction`](Self::text_direction) is provided, then the offsets are applied in the
/// reading direction, so in right-to-left text, positive x offsets move towards the
/// left, and in left-to-right text, positive x offsets move towards the right.
///
/// See also:
///
///  * [`AlignTransition`], an animated version of an `Align` that animates its
///    `Align::alignment` property.
///  * [`PositionedTransition`], a widget that animates its child from a start
///    position to an end position over the lifetime of the animation.
///  * [`RelativePositionedTransition`], a widget that transitions its child's
///    position based on the value of a rectangle relative to a bounding box.
#[derive(Debug)]
pub struct SlideTransition {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The animation that controls the position of the child.
    ///
    /// If the current value of the position animation is `(dx, dy)`, the child
    /// will be translated horizontally by `width * dx` and vertically by
    /// `height * dy`, after applying the [`text_direction`](Self::text_direction) if available.
    pub position: AnyAnimation<Offset>,

    /// The direction to use for the x offset described by the [`position`](Self::position).
    ///
    /// If [`text_direction`](Self::text_direction) is `None`, the x offset is applied in the
    /// coordinate system of the canvas (so positive x offsets move the child towards the
    /// right).
    ///
    /// If [`text_direction`](Self::text_direction) is [`TextDirection::Rtl`], the x offset is
    /// applied in the reading direction such that x offsets move the child towards the left.
    ///
    /// If [`text_direction`](Self::text_direction) is [`TextDirection::Ltr`], the x offset is
    /// applied in the reading direction such that x offsets move the child towards the right.
    pub text_direction: Option<TextDirection>,

    /// Whether hit testing should be affected by the slide animation.
    ///
    /// If false, hit testing will proceed as if the child was not translated at
    /// all. Setting this value to false is useful for fast animations where you
    /// expect the user to commonly interact with the child widget in its final
    /// location and you want the user to benefit from "muscle memory".
    pub transform_hit_tests: bool,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
}

impl SlideTransition {
    /// Creates a fractional translation transition.
    pub fn new(position: AnyAnimation<Offset>) -> SlideTransition {
        SlideTransition {
            key: None,
            position,
            text_direction: None,
            transform_hit_tests: true,
            child: None,
        }
    }

    /// Dart `SlideTransition(key:)`.
    pub fn key(mut self, key: KeyRef) -> SlideTransition {
        self.key = Some(key);
        self
    }

    /// Dart `SlideTransition(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> SlideTransition {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `SlideTransition(transformHitTests:)`.
    pub fn transform_hit_tests(mut self, transform_hit_tests: bool) -> SlideTransition {
        self.transform_hit_tests = transform_hit_tests;
        self
    }

    /// Dart `SlideTransition(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> SlideTransition {
        self.child = Some(child.into_widget());
        self
    }
}

impl AnimatedWidget for SlideTransition {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn listenable(&self) -> Rc<dyn Listenable> {
        Rc::new(self.position)
    }

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let mut offset = self.position.value(app);
        if self.text_direction == Some(TextDirection::Rtl) {
            offset = Offset::new(-offset.dx(), offset.dy());
        }
        let mut translation =
            FractionalTranslation::new(offset).transform_hit_tests(self.transform_hit_tests);
        if let Some(child) = &self.child {
            translation = translation.child(child.clone());
        }
        translation.into_widget()
    }
}

/// Signature for the callback to [`MatrixTransition::on_transform`].
///
/// Computes a [`Matrix4`] to be used in the [`MatrixTransition`] transformed widget
/// from the [`MatrixTransition::animation`] value.
pub type TransformCallback = Rc<dyn Fn(f64) -> Matrix4>;

/// Animates the [`Matrix4`] of a transformed widget.
///
/// The [`on_transform`](Self::on_transform) callback computes a [`Matrix4`] from the animated
/// value, it is called every time the [`animation`](Self::animation) changes its value.
///
/// See also:
///
///  * [`ScaleTransition`], which animates the scale of a widget, by providing a
///    matrix which scales along the X and Y axis.
///  * [`RotationTransition`], which animates the rotation of a widget, by
///    providing a matrix which rotates along the Z axis.
pub struct MatrixTransition {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The callback to compute a [`Matrix4`] from the [`animation`](Self::animation). It's
    /// called every time [`animation`](Self::animation) changes its value.
    pub on_transform: TransformCallback,

    /// The animation that controls the matrix of the child.
    ///
    /// The matrix will be computed from the animation with the
    /// [`on_transform`](Self::on_transform) callback.
    pub animation: AnyAnimation<f64>,

    /// The alignment of the origin of the coordinate system in which the
    /// transform takes place, relative to the size of the box.
    ///
    /// For example, to set the origin of the transform to bottom middle, you can
    /// use an alignment of (0.0, 1.0).
    pub alignment: Alignment,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
}

impl MatrixTransition {
    /// Creates a matrix transition.
    ///
    /// The [`alignment`](Self::alignment) defaults to [`Alignment::CENTER`].
    pub fn new(
        animation: AnyAnimation<f64>,
        on_transform: impl Fn(f64) -> Matrix4 + 'static,
    ) -> MatrixTransition {
        MatrixTransition {
            key: None,
            on_transform: Rc::new(on_transform),
            animation,
            alignment: Alignment::CENTER,
            child: None,
        }
    }

    /// Dart `MatrixTransition(key:)`.
    pub fn key(mut self, key: KeyRef) -> MatrixTransition {
        self.key = Some(key);
        self
    }

    /// Dart `MatrixTransition(alignment:)`.
    pub fn alignment(mut self, alignment: Alignment) -> MatrixTransition {
        self.alignment = alignment;
        self
    }

    /// Dart `MatrixTransition(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> MatrixTransition {
        self.child = Some(child.into_widget());
        self
    }

    /// `MatrixTransition.build`'s body, shared with [`ScaleTransition`] and
    /// [`RotationTransition`].
    fn build_transform(
        transform: Matrix4,
        alignment: Alignment,
        child: Option<&WidgetRef>,
    ) -> WidgetRef {
        let mut transform =
            Transform::new(transform).alignment(AlignmentGeometry::Alignment(alignment));
        if let Some(child) = child {
            transform = transform.child(child.clone());
        }
        transform.into_widget()
    }
}

impl Debug for MatrixTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MatrixTransition")
            .field("animation", &self.animation)
            .field("alignment", &self.alignment)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl AnimatedWidget for MatrixTransition {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn listenable(&self) -> Rc<dyn Listenable> {
        Rc::new(self.animation)
    }

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let transform = (self.on_transform)(self.animation.value(app));
        MatrixTransition::build_transform(transform, self.alignment, self.child.as_ref())
    }
}

/// Animates the scale of a transformed widget.
///
/// See also:
///
///  * [`PositionedTransition`], a widget that animates its child from a start
///    position to an end position over the lifetime of the animation.
///  * [`RelativePositionedTransition`], a widget that transitions its child's
///    position based on the value of a rectangle relative to a bounding box.
///  * [`SizeTransition`], a widget that animates its own size and clips and
///    aligns its child.
#[derive(Debug)]
pub struct ScaleTransition {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// See [`MatrixTransition::animation`]; read it as
    /// [`scale`](Self::scale).
    pub animation: AnyAnimation<f64>,

    /// See [`MatrixTransition::alignment`].
    pub alignment: Alignment,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
}

impl ScaleTransition {
    /// Creates a scale transition.
    ///
    /// The [`alignment`](Self::alignment) defaults to [`Alignment::CENTER`].
    pub fn new(scale: AnyAnimation<f64>) -> ScaleTransition {
        ScaleTransition {
            key: None,
            animation: scale,
            alignment: Alignment::CENTER,
            child: None,
        }
    }

    /// The animation that controls the scale of the child.
    pub fn scale(&self) -> AnyAnimation<f64> {
        self.animation
    }

    /// Dart `ScaleTransition(key:)`.
    pub fn key(mut self, key: KeyRef) -> ScaleTransition {
        self.key = Some(key);
        self
    }

    /// Dart `ScaleTransition(alignment:)`.
    pub fn alignment(mut self, alignment: Alignment) -> ScaleTransition {
        self.alignment = alignment;
        self
    }

    /// Dart `ScaleTransition(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> ScaleTransition {
        self.child = Some(child.into_widget());
        self
    }

    /// The callback that controls the scale of the child.
    ///
    /// If the current value of the animation is v, the child will be
    /// painted v times its normal size.
    fn handle_scale_matrix(value: f64) -> Matrix4 {
        Matrix4::scale(value as f32, value as f32)
    }
}

impl AnimatedWidget for ScaleTransition {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn listenable(&self) -> Rc<dyn Listenable> {
        Rc::new(self.animation)
    }

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let transform = ScaleTransition::handle_scale_matrix(self.animation.value(app));
        MatrixTransition::build_transform(transform, self.alignment, self.child.as_ref())
    }
}

/// Animates the rotation of a widget.
///
/// See also:
///
///  * [`ScaleTransition`], a widget that animates the scale of a transformed
///    widget.
///  * [`SizeTransition`], a widget that animates its own size and clips and
///    aligns its child.
#[derive(Debug)]
pub struct RotationTransition {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// See [`MatrixTransition::animation`]; read it as [`turns`](Self::turns).
    pub animation: AnyAnimation<f64>,

    /// See [`MatrixTransition::alignment`].
    pub alignment: Alignment,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
}

impl RotationTransition {
    /// Creates a rotation transition.
    pub fn new(turns: AnyAnimation<f64>) -> RotationTransition {
        RotationTransition {
            key: None,
            animation: turns,
            alignment: Alignment::CENTER,
            child: None,
        }
    }

    /// The animation that controls the rotation of the child.
    pub fn turns(&self) -> AnyAnimation<f64> {
        self.animation
    }

    /// Dart `RotationTransition(key:)`.
    pub fn key(mut self, key: KeyRef) -> RotationTransition {
        self.key = Some(key);
        self
    }

    /// Dart `RotationTransition(alignment:)`.
    pub fn alignment(mut self, alignment: Alignment) -> RotationTransition {
        self.alignment = alignment;
        self
    }

    /// Dart `RotationTransition(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> RotationTransition {
        self.child = Some(child.into_widget());
        self
    }

    /// The callback that controls the rotation of the child.
    ///
    /// If the current value of the animation is v, the child will be rotated
    /// v * 2 * pi radians before being painted.
    fn handle_turns_matrix(value: f64) -> Matrix4 {
        Matrix4::rotation((value * PI * 2.0) as f32)
    }
}

impl AnimatedWidget for RotationTransition {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn listenable(&self) -> Rc<dyn Listenable> {
        Rc::new(self.animation)
    }

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let transform = RotationTransition::handle_turns_matrix(self.animation.value(app));
        MatrixTransition::build_transform(transform, self.alignment, self.child.as_ref())
    }
}

/// Animates its own size and clips and aligns its child.
///
/// [`SizeTransition`] acts as a `ClipRect` that animates either its width or its
/// height, depending upon the value of [`axis`](Self::axis). The alignment of the child is
/// specified by the [`alignment`](Self::alignment).
///
/// Like most widgets, [`SizeTransition`] will conform to the constraints it is
/// given, so be sure to put it in a context where it can change size. For
/// instance, if you place it into a `Container` with a fixed size, then the
/// [`SizeTransition`] will not be able to change size, and will appear to do
/// nothing.
///
/// See also:
///
///  * `AnimatedCrossFade`, for a widget that automatically animates between
///    the sizes of two children, fading between them.
///  * [`ScaleTransition`], a widget that scales the size of the child instead of
///    clipping it.
///  * [`PositionedTransition`], a widget that animates its child from a start
///    position to an end position over the lifetime of the animation.
///  * [`RelativePositionedTransition`], a widget that transitions its child's
///    position based on the value of a rectangle relative to a bounding box.
#[derive(Debug)]
pub struct SizeTransition {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// [`Axis::Horizontal`] if [`size_factor`](Self::size_factor) modifies the width,
    /// otherwise [`Axis::Vertical`].
    pub axis: Axis,

    /// The animation that controls the (clipped) size of the child.
    ///
    /// The width or height (depending on the [`axis`](Self::axis) value) of this widget will
    /// be its intrinsic width or height multiplied by
    /// [`size_factor`](Self::size_factor)'s value at the current point in the animation.
    ///
    /// If the value of [`size_factor`](Self::size_factor) is less than one, the child will be
    /// clipped in the appropriate axis.
    pub size_factor: AnyAnimation<f64>,

    /// Describes how to align the child along the axis that
    /// [`size_factor`](Self::size_factor) is modifying.
    ///
    /// A value of -1.0 indicates the top when [`axis`](Self::axis) is [`Axis::Vertical`], and
    /// the start when [`axis`](Self::axis) is [`Axis::Horizontal`]. The start is on the left
    /// when the text direction in effect is [`TextDirection::Ltr`] and on the right when it
    /// is [`TextDirection::Rtl`].
    ///
    /// A value of 1.0 indicates the bottom or end, depending upon the [`axis`](Self::axis).
    ///
    /// A value of 0.0 (the default) indicates the center for either [`axis`](Self::axis)
    /// value.
    ///
    /// This property has been deprecated and superseded by
    /// [`alignment`](Self::alignment). Existing usages can be migrated as follows:
    /// - If [`axis`](Self::axis) is [`Axis::Horizontal`], replace with
    ///   `AlignmentGeometry::xy(axis_alignment.unwrap_or(0.0), -1.0)`.
    /// - If [`axis`](Self::axis) is [`Axis::Vertical`], replace with
    ///   `AlignmentGeometry::xy(-1.0, axis_alignment.unwrap_or(0.0))`.
    #[deprecated(
        note = "Use alignment instead. This property provides full control over both axes, \
                which is an improvement over the old axisAlignment. This feature was \
                deprecated after v3.41.0-1.0.pre."
    )]
    pub axis_alignment: Option<f64>,

    /// The alignment of the child within the parent during the transition.
    pub alignment: Option<AlignmentGeometry>,

    /// The factor by which to multiply the cross axis size of the child.
    ///
    /// If the value of [`fixed_cross_axis_size_factor`](Self::fixed_cross_axis_size_factor)
    /// is less than one, the child will be clipped along the appropriate axis.
    ///
    /// If `None` (the default), the cross axis size is as large as the parent.
    pub fixed_cross_axis_size_factor: Option<f64>,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
}

impl SizeTransition {
    /// Creates a size transition.
    ///
    /// The [`axis`](Self::axis) defaults to [`Axis::Vertical`]. The
    /// [`alignment`](Self::alignment) defaults to the center of the modified axis, which
    /// centers the child within the parent during the transition.
    #[allow(deprecated)]
    pub fn new(size_factor: AnyAnimation<f64>) -> SizeTransition {
        SizeTransition {
            key: None,
            axis: Axis::Vertical,
            size_factor,
            axis_alignment: None,
            alignment: None,
            fixed_cross_axis_size_factor: None,
            child: None,
        }
    }

    /// Dart `SizeTransition(key:)`.
    pub fn key(mut self, key: KeyRef) -> SizeTransition {
        self.key = Some(key);
        self
    }

    /// Dart `SizeTransition(axis:)`.
    pub fn axis(mut self, axis: Axis) -> SizeTransition {
        self.axis = axis;
        self
    }

    /// Dart `SizeTransition(axisAlignment:)`.
    #[deprecated(
        note = "Use alignment instead. This property provides full control over both axes, \
                which is an improvement over the old axisAlignment. This feature was \
                deprecated after v3.41.0-1.0.pre."
    )]
    #[allow(deprecated)]
    pub fn axis_alignment(mut self, axis_alignment: f64) -> SizeTransition {
        debug_assert!(
            self.alignment.is_none(),
            "Cannot provide both axisAlignment and alignment as axisAlignment has been \
             deprecated and superseded by alignment."
        );
        self.axis_alignment = Some(axis_alignment);
        self
    }

    /// Dart `SizeTransition(alignment:)`.
    #[allow(deprecated)]
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> SizeTransition {
        debug_assert!(
            self.axis_alignment.is_none(),
            "Cannot provide both axisAlignment and alignment as axisAlignment has been \
             deprecated and superseded by alignment."
        );
        self.alignment = Some(alignment);
        self
    }

    /// Dart `SizeTransition(fixedCrossAxisSizeFactor:)`.
    pub fn fixed_cross_axis_size_factor(
        mut self,
        fixed_cross_axis_size_factor: f64,
    ) -> SizeTransition {
        debug_assert!(fixed_cross_axis_size_factor >= 0.0);
        self.fixed_cross_axis_size_factor = Some(fixed_cross_axis_size_factor);
        self
    }

    /// Dart `SizeTransition(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> SizeTransition {
        self.child = Some(child.into_widget());
        self
    }
}

impl AnimatedWidget for SizeTransition {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn listenable(&self) -> Rc<dyn Listenable> {
        Rc::new(self.size_factor)
    }

    #[allow(deprecated)]
    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let axis_alignment = self.axis_alignment.unwrap_or(0.0);
        let alignment = self.alignment.unwrap_or(match self.axis {
            Axis::Horizontal => AlignmentGeometry::directional(axis_alignment, -1.0),
            Axis::Vertical => AlignmentGeometry::directional(-1.0, axis_alignment),
        });
        let size_factor = self.size_factor.value(app).max(0.0);
        let (height_factor, width_factor) = match self.axis {
            Axis::Vertical => (Some(size_factor), self.fixed_cross_axis_size_factor),
            Axis::Horizontal => (self.fixed_cross_axis_size_factor, Some(size_factor)),
        };
        let mut align = Align::new().alignment(alignment);
        if let Some(height_factor) = height_factor {
            align = align.height_factor(height_factor);
        }
        if let Some(width_factor) = width_factor {
            align = align.width_factor(width_factor);
        }
        if let Some(child) = &self.child {
            align = align.child(child.clone());
        }
        ClipRect::new().child(align).into_widget()
    }
}

/// Animates the opacity of a widget.
///
/// For a widget that automatically animates between the sizes of two children,
/// fading between them, see `AnimatedCrossFade`.
///
/// ## Hit testing
///
/// Setting the [`opacity`](Self::opacity) to zero does not prevent hit testing from being
/// applied to the descendants of the [`FadeTransition`] widget. This can be
/// confusing for the user, who may not see anything, and may believe the area
/// of the interface where the [`FadeTransition`] is hiding a widget to be
/// non-interactive.
///
/// With certain widgets, such as `Flow`, that compute their positions only when
/// they are painted, this can actually lead to bugs (from unexpected geometry
/// to exceptions), because those widgets are not painted by the
/// [`FadeTransition`] widget at all when the [`opacity`](Self::opacity) animation reaches
/// zero.
///
/// To avoid such problems, it is generally a good idea to combine this widget
/// with an `IgnorePointer` that one enables when the [`opacity`](Self::opacity) animation
/// reaches zero. This prevents interactions with any children in the subtree
/// when the [`child`](Self::child) is not visible. For performance reasons, when
/// implementing this, care should be taken not to rebuild the relevant widget (e.g. by
/// calling `State::set_state`) except at the transition point.
///
/// See also:
///
///  * `Opacity`, which does not animate changes in opacity.
///  * `AnimatedOpacity`, which animates changes in opacity without taking an
///    explicit `Animation` argument.
#[derive(Debug)]
pub struct FadeTransition {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The animation that controls the opacity of the child.
    ///
    /// If the current value of the opacity animation is v, the child will be
    /// painted with an opacity of v. For example, if v is 0.5, the child will be
    /// blended 50% with its background. Similarly, if v is 0.0, the child will be
    /// completely transparent.
    pub opacity: AnyAnimation<f64>,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
}

impl FadeTransition {
    /// Creates an opacity transition.
    pub fn new(opacity: AnyAnimation<f64>) -> FadeTransition {
        FadeTransition {
            key: None,
            opacity,
            child: None,
        }
    }

    /// Dart `FadeTransition(key:)`.
    pub fn key(mut self, key: KeyRef) -> FadeTransition {
        self.key = Some(key);
        self
    }

    /// Dart `FadeTransition(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> FadeTransition {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for FadeTransition {
    type RenderObject = RenderAnimatedOpacity;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderAnimatedOpacity::new(app, self.opacity, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderAnimatedOpacity>,
    ) {
        render_object.set_opacity(app, self.opacity);
    }
}

impl SingleChildRenderObjectWidget for FadeTransition {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// An interpolation between two relative rects.
///
/// This class specializes the interpolation of `Tween<RelativeRect>` to
/// use [`RelativeRect::lerp`].
///
/// See `Tween` for a discussion on how to use interpolation objects.
///
/// Unlike the tweens in `inset_animation`, this one is a value rather than an arena
/// object: the orphan rule leaves no local type in
/// `impl Animatable<RelativeRect> for Handle<RelativeRectTween>`. A driven animation
/// therefore holds a clone, and writing [`begin`](Self::begin) or [`end`](Self::end)
/// afterwards does not reach it.
#[derive(Clone, Debug)]
pub struct RelativeRectTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<RelativeRect>,

    /// The value this variable has at the end of the animation.
    pub end: Option<RelativeRect>,
}

impl RelativeRectTween {
    /// Creates a [`RelativeRect`] tween.
    ///
    /// The [`begin`](Self::begin) and [`end`](Self::end) properties may be `None`; the `None`
    /// value is treated as [`RelativeRect::FILL`].
    pub fn new(begin: Option<RelativeRect>, end: Option<RelativeRect>) -> RelativeRectTween {
        RelativeRectTween { begin, end }
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> RelativeRect {
        RelativeRect::lerp(self.begin, self.end, t)
            .expect("RelativeRectTween.begin or end must be set before use")
    }
}

impl Animatable<RelativeRect> for RelativeRectTween {
    fn transform(&self, _app: &App, t: f64) -> RelativeRect {
        if t == 0.0 {
            return self.begin.expect("Tween.begin must be set before use");
        }
        if t == 1.0 {
            return self.end.expect("Tween.end must be set before use");
        }
        self.lerp(t)
    }
}

/// Animated version of `Positioned` which takes a specific
/// `Animation<RelativeRect>` to transition the child's position from a start
/// position to an end position over the lifetime of the animation.
///
/// Only works if it's the child of a `Stack`.
///
/// See also:
///
///  * `AnimatedPositioned`, which transitions a child's position without
///    taking an explicit `Animation` argument.
///  * [`RelativePositionedTransition`], a widget that transitions its child's
///    position based on the value of a rectangle relative to a bounding box.
///  * [`SlideTransition`], a widget that animates the position of a widget
///    relative to its normal position.
///  * [`AlignTransition`], an animated version of an `Align` that animates its
///    `Align::alignment` property.
///  * [`ScaleTransition`], a widget that animates the scale of a transformed
///    widget.
///  * [`SizeTransition`], a widget that animates its own size and clips and
///    aligns its child.
#[derive(Debug)]
pub struct PositionedTransition {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The animation that controls the child's size and position.
    pub rect: AnyAnimation<RelativeRect>,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl PositionedTransition {
    /// Creates a transition for `Positioned`.
    pub fn new<K>(
        rect: AnyAnimation<RelativeRect>,
        child: impl IntoWidget<K>,
    ) -> PositionedTransition {
        PositionedTransition {
            key: None,
            rect,
            child: child.into_widget(),
        }
    }

    /// Dart `PositionedTransition(key:)`.
    pub fn key(mut self, key: KeyRef) -> PositionedTransition {
        self.key = Some(key);
        self
    }
}

impl AnimatedWidget for PositionedTransition {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn listenable(&self) -> Rc<dyn Listenable> {
        Rc::new(self.rect)
    }

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        Positioned::from_relative_rect(self.rect.value(app), self.child.clone()).into_widget()
    }
}

/// Animated version of `Positioned` which transitions the child's position
/// based on the value of [`rect`](Self::rect) relative to a bounding box with the
/// specified [`size`](Self::size).
///
/// Only works if it's the child of a `Stack`.
///
/// See also:
///
///  * [`PositionedTransition`], a widget that animates its child from a start
///    position to an end position over the lifetime of the animation.
///  * [`AlignTransition`], an animated version of an `Align` that animates its
///    `Align::alignment` property.
///  * [`ScaleTransition`], a widget that animates the scale of a transformed
///    widget.
///  * [`SizeTransition`], a widget that animates its own size and clips and
///    aligns its child.
///  * [`SlideTransition`], a widget that animates the position of a widget
///    relative to its normal position.
#[derive(Debug)]
pub struct RelativePositionedTransition {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The animation that controls the child's size and position.
    ///
    /// If the animation returns a `None` [`Rect`], the rect is assumed to be
    /// [`Rect::ZERO`].
    ///
    /// See also:
    ///
    ///  * [`size`](Self::size), which gets the size of the box that the `Positioned` widget's
    ///    offsets are relative to.
    pub rect: AnyAnimation<Option<Rect>>,

    /// The `Positioned` widget's offsets are relative to a box of this
    /// size whose origin is 0,0.
    pub size: Size,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl RelativePositionedTransition {
    /// Create an animated version of `Positioned`.
    ///
    /// Each frame, the `Positioned` widget will be configured to represent the
    /// current value of the [`rect`](Self::rect) argument assuming that the stack has the
    /// given [`size`](Self::size).
    pub fn new<K>(
        rect: AnyAnimation<Option<Rect>>,
        size: Size,
        child: impl IntoWidget<K>,
    ) -> RelativePositionedTransition {
        RelativePositionedTransition {
            key: None,
            rect,
            size,
            child: child.into_widget(),
        }
    }

    /// Dart `RelativePositionedTransition(key:)`.
    pub fn key(mut self, key: KeyRef) -> RelativePositionedTransition {
        self.key = Some(key);
        self
    }
}

impl AnimatedWidget for RelativePositionedTransition {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn listenable(&self) -> Rc<dyn Listenable> {
        Rc::new(self.rect)
    }

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let offsets =
            RelativeRect::from_size(self.rect.value(app).unwrap_or(Rect::ZERO), self.size);
        Positioned::new(self.child.clone())
            .top(offsets.top)
            .right(offsets.right)
            .bottom(offsets.bottom)
            .left(offsets.left)
            .into_widget()
    }
}

/// Animated version of a [`DecoratedBox`] that animates the different properties
/// of its `Decoration`.
///
/// See also:
///
///  * [`DecoratedBox`], which also draws a `Decoration` but is not animated.
///  * `AnimatedContainer`, a more full-featured container that also animates on
///    decoration using an internal animation.
#[derive(Debug)]
pub struct DecoratedBoxTransition {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// Animation of the decoration to paint.
    ///
    /// Can be created using a `DecorationTween` interpolating typically between
    /// two `BoxDecoration`.
    pub decoration: AnyAnimation<Box<dyn Decoration>>,

    /// Whether to paint the box decoration behind or in front of the child.
    pub position: DecorationPosition,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl DecoratedBoxTransition {
    /// Creates an animated [`DecoratedBox`] whose `Decoration` animation updates
    /// the widget.
    ///
    /// See also:
    ///
    ///  * [`DecoratedBox::new`]
    pub fn new<K>(
        decoration: AnyAnimation<Box<dyn Decoration>>,
        child: impl IntoWidget<K>,
    ) -> DecoratedBoxTransition {
        DecoratedBoxTransition {
            key: None,
            decoration,
            position: DecorationPosition::Background,
            child: child.into_widget(),
        }
    }

    /// Dart `DecoratedBoxTransition(key:)`.
    pub fn key(mut self, key: KeyRef) -> DecoratedBoxTransition {
        self.key = Some(key);
        self
    }

    /// Dart `DecoratedBoxTransition(position:)`.
    pub fn position(mut self, position: DecorationPosition) -> DecoratedBoxTransition {
        self.position = position;
        self
    }
}

impl AnimatedWidget for DecoratedBoxTransition {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn listenable(&self) -> Rc<dyn Listenable> {
        Rc::new(self.decoration)
    }

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        DecoratedBox {
            key: None,
            decoration: self.decoration.value(app),
            position: self.position,
            child: Some(self.child.clone()),
        }
        .into_widget()
    }
}

/// Animated version of an `Align` that animates its `Align::alignment` property.
///
/// See also:
///
///  * `AnimatedAlign`, which animates changes to the [`alignment`](Self::alignment) without
///    taking an explicit `Animation` argument.
///  * [`PositionedTransition`], a widget that animates its child from a start
///    position to an end position over the lifetime of the animation.
///  * [`RelativePositionedTransition`], a widget that transitions its child's
///    position based on the value of a rectangle relative to a bounding box.
///  * [`SizeTransition`], a widget that animates its own size and clips and
///    aligns its child.
///  * [`SlideTransition`], a widget that animates the position of a widget
///    relative to its normal position.
#[derive(Debug)]
pub struct AlignTransition {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The animation that controls the child's alignment.
    pub alignment: AnyAnimation<AlignmentGeometry>,

    /// If non-`None`, the child's width factor, see `Align::width_factor`.
    pub width_factor: Option<f64>,

    /// If non-`None`, the child's height factor, see `Align::height_factor`.
    pub height_factor: Option<f64>,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl AlignTransition {
    /// Creates an animated `Align` whose [`AlignmentGeometry`] animation updates
    /// the widget.
    ///
    /// See also:
    ///
    ///  * `Align::new`.
    pub fn new<K>(
        alignment: AnyAnimation<AlignmentGeometry>,
        child: impl IntoWidget<K>,
    ) -> AlignTransition {
        AlignTransition {
            key: None,
            alignment,
            width_factor: None,
            height_factor: None,
            child: child.into_widget(),
        }
    }

    /// Dart `AlignTransition(key:)`.
    pub fn key(mut self, key: KeyRef) -> AlignTransition {
        self.key = Some(key);
        self
    }

    /// Dart `AlignTransition(widthFactor:)`.
    pub fn width_factor(mut self, width_factor: f64) -> AlignTransition {
        self.width_factor = Some(width_factor);
        self
    }

    /// Dart `AlignTransition(heightFactor:)`.
    pub fn height_factor(mut self, height_factor: f64) -> AlignTransition {
        self.height_factor = Some(height_factor);
        self
    }
}

impl AnimatedWidget for AlignTransition {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn listenable(&self) -> Rc<dyn Listenable> {
        Rc::new(self.alignment)
    }

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let mut align = Align::new().alignment(self.alignment.value(app));
        if let Some(width_factor) = self.width_factor {
            align = align.width_factor(width_factor);
        }
        if let Some(height_factor) = self.height_factor {
            align = align.height_factor(height_factor);
        }
        align.child(self.child.clone()).into_widget()
    }
}

/// Animated version of a [`DefaultTextStyle`] that animates the different properties
/// of its [`TextStyle`].
///
/// See also:
///
///  * `AnimatedDefaultTextStyle`, which animates changes in text style without
///    taking an explicit `Animation` argument.
///  * [`DefaultTextStyle`], which also defines a [`TextStyle`] for its descendants
///    but is not animated.
#[derive(Debug)]
pub struct DefaultTextStyleTransition {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The animation that controls the descendants' text style.
    pub style: AnyAnimation<TextStyle>,

    /// How the text should be aligned horizontally.
    pub text_align: Option<TextAlign>,

    /// Whether the text should break at soft line breaks.
    ///
    /// See [`DefaultTextStyle::soft_wrap`] for more details.
    pub soft_wrap: bool,

    /// How visual overflow should be handled.
    pub overflow: TextOverflow,

    /// An optional maximum number of lines for the text to span, wrapping if necessary.
    ///
    /// See [`DefaultTextStyle::max_lines`] for more details.
    pub max_lines: Option<i32>,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl DefaultTextStyleTransition {
    /// Creates an animated [`DefaultTextStyle`] whose [`TextStyle`] animation updates
    /// the widget.
    pub fn new<K>(
        style: AnyAnimation<TextStyle>,
        child: impl IntoWidget<K>,
    ) -> DefaultTextStyleTransition {
        DefaultTextStyleTransition {
            key: None,
            style,
            text_align: None,
            soft_wrap: true,
            overflow: TextOverflow::Clip,
            max_lines: None,
            child: child.into_widget(),
        }
    }

    /// Dart `DefaultTextStyleTransition(key:)`.
    pub fn key(mut self, key: KeyRef) -> DefaultTextStyleTransition {
        self.key = Some(key);
        self
    }

    /// Dart `DefaultTextStyleTransition(textAlign:)`.
    pub fn text_align(mut self, text_align: TextAlign) -> DefaultTextStyleTransition {
        self.text_align = Some(text_align);
        self
    }

    /// Dart `DefaultTextStyleTransition(softWrap:)`.
    pub fn soft_wrap(mut self, soft_wrap: bool) -> DefaultTextStyleTransition {
        self.soft_wrap = soft_wrap;
        self
    }

    /// Dart `DefaultTextStyleTransition(overflow:)`.
    pub fn overflow(mut self, overflow: TextOverflow) -> DefaultTextStyleTransition {
        self.overflow = overflow;
        self
    }

    /// Dart `DefaultTextStyleTransition(maxLines:)`.
    pub fn max_lines(mut self, max_lines: i32) -> DefaultTextStyleTransition {
        self.max_lines = Some(max_lines);
        self
    }
}

impl AnimatedWidget for DefaultTextStyleTransition {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn listenable(&self) -> Rc<dyn Listenable> {
        Rc::new(self.style)
    }

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let mut default_text_style =
            DefaultTextStyle::new(self.style.value(app), self.child.clone())
                .soft_wrap(self.soft_wrap)
                .overflow(self.overflow);
        if let Some(text_align) = self.text_align {
            default_text_style = default_text_style.text_align(text_align);
        }
        if let Some(max_lines) = self.max_lines {
            default_text_style = default_text_style.max_lines(max_lines);
        }
        default_text_style.into_widget()
    }
}

/// A builder that builds a widget given a child.
///
/// The child is passed into the builder to allow the builder to reuse a widget that does
/// not depend on the listenable, as [`ListenableBuilder`] and `AnimatedBuilder` do.
pub type TransitionBuilder = Rc<dyn Fn(&mut App, BuildContext, Option<&WidgetRef>) -> WidgetRef>;

/// A general-purpose widget for building a widget subtree when a `Listenable` changes.
///
/// [`ListenableBuilder`] is useful for more complex widgets that wish to listen to changes in
/// other objects as part of a larger build function. To use [`ListenableBuilder`], construct
/// the widget and pass it a [`builder`](Self::builder) function.
///
/// Any subtype of `Listenable` (such as a `ChangeNotifier`, `ValueNotifier`, or `Animation`)
/// can be used with a [`ListenableBuilder`] to rebuild only certain parts of a widget when
/// the `Listenable` notifies its listeners. Although they have identical implementations, if
/// an `Animation` is being listened to, consider using an `AnimatedBuilder` instead for
/// better readability.
///
/// ## Performance optimizations
///
/// If the [`builder`](Self::builder) function contains a subtree that does not depend on the
/// [`listenable`](Self::listenable), it is more efficient to build that subtree once instead
/// of rebuilding it on every change of the listenable.
///
/// Performance is optimized by building the subtree once and passing it as the
/// [`child`](Self::child); the builder receives it as its third argument.
pub struct ListenableBuilder {
    pub key: Option<KeyRef>,
    /// The `Listenable` supplied to the constructor.
    ///
    /// Also accessible through `animation` for an `AnimatedBuilder`.
    pub listenable: Rc<dyn Listenable>,
    /// Called every time the [`listenable`](Self::listenable) notifies about a change.
    ///
    /// The child given to the builder should typically be part of the returned widget tree.
    pub builder: TransitionBuilder,
    /// The child widget to pass to the [`builder`](Self::builder).
    ///
    /// If a [`builder`](Self::builder) callback's return value contains a subtree that does
    /// not depend on the listenable, it's more efficient to build that subtree once instead
    /// of rebuilding it on every change of the listenable.
    ///
    /// If the pre-built subtree is passed as the [`child`](Self::child) parameter, the
    /// [`ListenableBuilder`] will pass it back to the [`builder`](Self::builder) function so
    /// that it can be incorporated into the build.
    ///
    /// Using this pre-built child is entirely optional, but can improve performance
    /// significantly in some cases and is therefore a good practice.
    pub child: Option<WidgetRef>,
}

/// Dart's `class AnimatedBuilder extends ListenableBuilder`, which only renames the
/// listenable `animation`: the same widget.
pub type AnimatedBuilder = ListenableBuilder;

impl ListenableBuilder {
    /// Creates a builder that responds to changes in `listenable`.
    pub fn new(
        listenable: Rc<dyn Listenable>,
        builder: impl Fn(&mut App, BuildContext, Option<&WidgetRef>) -> WidgetRef + 'static,
    ) -> ListenableBuilder {
        ListenableBuilder {
            key: None,
            listenable,
            builder: Rc::new(builder),
            child: None,
        }
    }

    /// Dart `ListenableBuilder(key:)`.
    pub fn key(mut self, key: KeyRef) -> ListenableBuilder {
        self.key = Some(key);
        self
    }

    /// Dart `ListenableBuilder(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> ListenableBuilder {
        self.child = Some(child.into_widget());
        self
    }

    /// The `Listenable` supplied to the constructor (Dart's `AnimatedBuilder.animation`).
    pub fn animation(&self) -> Rc<dyn Listenable> {
        Rc::clone(&self.listenable)
    }
}

impl Debug for ListenableBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListenableBuilder")
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl AnimatedWidget for ListenableBuilder {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn listenable(&self) -> Rc<dyn Listenable> {
        Rc::clone(&self.listenable)
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        (self.builder)(app, context, self.child.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::Cell;
    use std::time::Duration;

    use inset_animation::{AnimationBehavior, AnimationController, CallbackAnimatable};
    use inset_embedder::Color;
    use inset_foundation::{ChangeNotifier, ValueNotifier};
    use inset_painting::{AnyColor, BoxDecoration};
    use inset_rendering::{
        BoxConstraints, ContainerRenderObjectMixin, RenderAligningShiftedBox, RenderConstrainedBox,
        RenderFractionalTranslation, RenderObjectWithChildMixin, RenderTransform,
    };
    use inset_scheduler::{Ticker, TickerCallback, TickerProvider};

    use super::*;
    use crate::framework::LeafRenderObjectWidget;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Builder, Directionality, SizedBox, Stack};

    /// flutter_test's `TestVSync`: vends ordinary tickers.
    struct TestVSync;

    impl TickerProvider for TestVSync {
        fn create_ticker(&self, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
            Ticker::new(app, on_tick)
        }
    }

    /// `SizedBox` in miniature.
    #[derive(Debug)]
    struct Sized {
        size: Size,
    }

    impl RenderObjectWidget for Sized {
        type RenderObject = RenderConstrainedBox;

        fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
            RenderConstrainedBox::new(app, BoxConstraints::tight(self.size), None).as_object()
        }

        fn update_render_object(
            &self,
            app: &mut App,
            _context: BuildContext,
            render_object: RenderHandle<RenderConstrainedBox>,
        ) {
            render_object.set_additional_constraints(app, BoxConstraints::tight(self.size));
        }
    }

    impl LeafRenderObjectWidget for Sized {}

    fn new_controller(app: &mut App) -> Handle<AnimationController> {
        AnimationController::create(
            app,
            None,
            Some(Duration::from_millis(100)),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            TestVSync,
        )
    }

    fn fade(opacity: AnyAnimation<f64>) -> WidgetRef {
        FadeTransition::new(opacity)
            .child(Sized {
                size: Size::new(10.0, 10.0),
            })
            .into_widget()
    }

    fn fade_under_root(harness: &Harness, app: &App) -> RenderHandle<RenderAnimatedOpacity> {
        harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .as_object()
            .downcast::<RenderAnimatedOpacity>(app)
            .expect("a RenderAnimatedOpacity")
    }

    fn root_child_size(harness: &Harness, app: &App) -> Size {
        harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .size(app)
    }

    /// The render object directly under the test root, as `T`.
    fn under_root<T: inset_rendering::RenderObject>(
        harness: &Harness,
        app: &App,
    ) -> RenderHandle<T> {
        harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .as_object()
            .downcast::<T>(app)
            .expect("the expected render object")
    }

    /// Under a [`Directionality`], for the widgets that resolve a directional alignment.
    fn ltr<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Directionality::new(TextDirection::Ltr, child).into_widget()
    }

    fn ten_by_ten() -> Sized {
        Sized {
            size: Size::new(10.0, 10.0),
        }
    }

    fn assert_close(actual: Offset, expected: Offset) {
        assert!(
            (actual.dx() - expected.dx()).abs() < 1e-4
                && (actual.dy() - expected.dy()).abs() < 1e-4,
            "{actual:?} is not {expected:?}"
        );
    }

    // transitions_test.dart 'SlideTransition transformHitTests'
    #[test]
    fn a_slide_transition_translates_by_the_animation_and_flips_in_rtl() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = new_controller(&mut app);
        let position = controller.drive(
            &mut app,
            CallbackAnimatable::from_callback(|t| Offset::new(t, t * 2.0)),
        );
        let slide = |text_direction: Option<TextDirection>| {
            let mut transition = SlideTransition::new(position)
                .transform_hit_tests(false)
                .child(ten_by_ten());
            if let Some(text_direction) = text_direction {
                transition = transition.text_direction(text_direction);
            }
            transition.into_widget()
        };
        let harness = Harness::mount(&mut app, slide(None));
        harness.pump(&mut app);
        controller.set_value(&mut app, 0.5);
        harness.pump(&mut app);
        let translation: RenderHandle<RenderFractionalTranslation> = under_root(&harness, &app);
        assert_eq!(translation.translation(&app), Offset::new(0.5, 1.0));
        assert!(!translation.transform_hit_tests(&app));

        harness.set_child(&mut app, slide(Some(TextDirection::Rtl)));
        harness.pump(&mut app);
        assert_eq!(translation.translation(&app), Offset::new(-0.5, 1.0));

        harness.set_child(&mut app, slide(Some(TextDirection::Ltr)));
        harness.pump(&mut app);
        assert_eq!(translation.translation(&app), Offset::new(0.5, 1.0));

        controller.dispose(&mut app);
    }

    // transitions_test.dart 'MatrixTransition animates'
    #[test]
    fn a_matrix_transition_paints_the_matrix_its_callback_returns() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = new_controller(&mut app);
        let widget = MatrixTransition::new(controller.view(), |value| {
            Matrix4::translation(value as f32 * 100.0, 0.0)
        })
        .alignment(Alignment::TOP_LEFT)
        .child(ten_by_ten())
        .into_widget();
        let harness = Harness::mount(&mut app, widget);
        harness.pump(&mut app);
        let transform: RenderHandle<RenderTransform> = under_root(&harness, &app);
        assert_eq!(
            transform.alignment(&app),
            Some(AlignmentGeometry::Alignment(Alignment::TOP_LEFT))
        );

        controller.set_value(&mut app, 0.5);
        harness.pump(&mut app);
        let child = transform.child(&app).expect("the sized child");
        assert_close(
            child.local_to_global(&app, Offset::ZERO, None),
            Offset::new(50.0, 0.0),
        );

        controller.dispose(&mut app);
    }

    // transitions_test.dart 'ScaleTransition animates'
    #[test]
    fn a_scale_transition_scales_about_its_alignment() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = new_controller(&mut app);
        let widget = ScaleTransition::new(controller.view())
            .child(ten_by_ten())
            .into_widget();
        let harness = Harness::mount(&mut app, widget);
        controller.set_value(&mut app, 0.5);
        harness.pump(&mut app);

        let transform: RenderHandle<RenderTransform> = under_root(&harness, &app);
        assert_eq!(
            transform.alignment(&app),
            Some(AlignmentGeometry::Alignment(Alignment::CENTER))
        );
        let child = transform.child(&app).expect("the sized child");
        // Half size about the 10x10 child's centre: its top-left lands at (2.5, 2.5).
        assert_close(
            child.local_to_global(&app, Offset::ZERO, None),
            Offset::new(2.5, 2.5),
        );

        controller.set_value(&mut app, 1.0);
        harness.pump(&mut app);
        assert_close(
            child.local_to_global(&app, Offset::ZERO, None),
            Offset::ZERO,
        );

        controller.dispose(&mut app);
    }

    // transitions_test.dart 'RotationTransition animates'
    #[test]
    fn a_rotation_transition_turns_a_full_circle_per_unit() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = new_controller(&mut app);
        let widget = RotationTransition::new(controller.view())
            .child(ten_by_ten())
            .into_widget();
        let harness = Harness::mount(&mut app, widget);
        controller.set_value(&mut app, 0.25);
        harness.pump(&mut app);

        let transform: RenderHandle<RenderTransform> = under_root(&harness, &app);
        let child = transform.child(&app).expect("the sized child");
        // A quarter turn clockwise about the 10x10 child's centre.
        assert_close(
            child.local_to_global(&app, Offset::ZERO, None),
            Offset::new(10.0, 0.0),
        );

        controller.set_value(&mut app, 0.5);
        harness.pump(&mut app);
        assert_close(
            child.local_to_global(&app, Offset::ZERO, None),
            Offset::new(10.0, 10.0),
        );

        controller.dispose(&mut app);
    }

    // transitions_test.dart 'SizeTransition clamps negative size factors'
    #[test]
    fn a_size_transition_clips_its_child_to_the_size_factor() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = new_controller(&mut app);
        let widget = SizeTransition::new(controller.view())
            .child(ten_by_ten())
            .into_widget();
        let harness = Harness::mount(&mut app, ltr(widget));
        harness.pump(&mut app);
        let clip: RenderHandle<inset_rendering::RenderClipRect> = under_root(&harness, &app);
        assert_eq!(clip.size(&app).height(), 0.0);

        controller.set_value(&mut app, 0.5);
        harness.pump(&mut app);
        assert_eq!(clip.size(&app).height(), 5.0);
        assert_eq!(
            clip.size(&app).width(),
            crate::test_harness::VIEW_WIDTH,
            "the cross axis is as large as the parent"
        );

        controller.set_value(&mut app, 1.0);
        harness.pump(&mut app);
        assert_eq!(clip.size(&app).height(), 10.0);

        controller.dispose(&mut app);
    }

    #[test]
    fn a_horizontal_size_transition_fixes_its_cross_axis_factor() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = new_controller(&mut app);
        let widget = SizeTransition::new(controller.view())
            .axis(Axis::Horizontal)
            .fixed_cross_axis_size_factor(1.0)
            .child(ten_by_ten())
            .into_widget();
        let harness = Harness::mount(&mut app, ltr(widget));
        controller.set_value(&mut app, 0.5);
        harness.pump(&mut app);
        let clip: RenderHandle<inset_rendering::RenderClipRect> = under_root(&harness, &app);
        assert_eq!(clip.size(&app), Size::new(5.0, 10.0));

        controller.dispose(&mut app);
    }

    // transitions_test.dart 'PositionedTransition animates'
    #[test]
    fn a_positioned_transition_places_its_child_by_the_relative_rect() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = new_controller(&mut app);
        let tween = RelativeRectTween::new(
            Some(RelativeRect::from_ltrb(0.0, 0.0, 200.0, 100.0)),
            Some(RelativeRect::from_ltrb(100.0, 50.0, 0.0, 0.0)),
        );
        let rect = controller.drive(&mut app, tween);
        let widget = Stack::new()
            .children([PositionedTransition::new(rect, ten_by_ten()).into_widget()])
            .into_widget();
        let harness = Harness::mount(&mut app, ltr(widget));
        controller.set_value(&mut app, 0.5);
        harness.pump(&mut app);

        let stack: RenderHandle<inset_rendering::RenderStack> = under_root(&harness, &app);
        let child = stack.first_child(&app).expect("the positioned child");
        assert_close(
            child.local_to_global(&app, Offset::ZERO, None),
            Offset::new(50.0, 25.0),
        );
        assert_eq!(child.size(&app), Size::new(150.0, 125.0));

        controller.dispose(&mut app);
    }

    // transitions_test.dart 'RelativePositionedTransition animates'
    #[test]
    fn a_relative_positioned_transition_places_its_child_inside_the_given_size() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = new_controller(&mut app);
        let rect = controller.drive(
            &mut app,
            CallbackAnimatable::from_callback(|t| {
                Some(Rect::from_ltwh(t * 40.0, t * 20.0, 30.0, 15.0))
            }),
        );
        let widget = Stack::new()
            .children([RelativePositionedTransition::new(
                rect,
                Size::new(100.0, 100.0),
                ten_by_ten(),
            )
            .into_widget()])
            .into_widget();
        let harness = Harness::mount(&mut app, ltr(widget));
        controller.set_value(&mut app, 0.5);
        harness.pump(&mut app);

        let stack: RenderHandle<inset_rendering::RenderStack> = under_root(&harness, &app);
        let child = stack.first_child(&app).expect("the positioned child");
        // The rect (20, 10, 50, 25) inside a 100x100 box, applied to the 300x200 stack.
        assert_close(
            child.local_to_global(&app, Offset::ZERO, None),
            Offset::new(20.0, 10.0),
        );
        assert_eq!(
            child.size(&app),
            Size::new(
                crate::test_harness::VIEW_WIDTH - 20.0 - 50.0,
                crate::test_harness::VIEW_HEIGHT - 10.0 - 75.0,
            )
        );

        controller.dispose(&mut app);
    }

    /// A decoration that fades black in; `CallbackAnimatable`'s derived `Clone` asks for
    /// `Box<dyn Decoration>: Clone`, which no trait object can be.
    #[derive(Clone)]
    struct FadingBlack;

    impl Animatable<Box<dyn Decoration>> for FadingBlack {
        fn transform(&self, _app: &App, t: f64) -> Box<dyn Decoration> {
            Box::new(BoxDecoration::new().color(Color::from_argb(
                (t * 255.0).round() as i32,
                0,
                0,
                0,
            )))
        }
    }

    // transitions_test.dart 'DecoratedBoxTransition test'
    #[test]
    fn a_decorated_box_transition_hands_the_animated_decoration_to_its_render_object() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = new_controller(&mut app);
        let decoration = controller.drive(&mut app, FadingBlack);
        let widget = DecoratedBoxTransition::new(decoration, ten_by_ten())
            .position(DecorationPosition::Foreground)
            .into_widget();
        let harness = Harness::mount(&mut app, widget);
        controller.set_value(&mut app, 0.5);
        harness.pump(&mut app);

        let decorated: RenderHandle<inset_rendering::RenderDecoratedBox> =
            under_root(&harness, &app);
        assert_eq!(decorated.position(&app), DecorationPosition::Foreground);
        let painted = (decorated.decoration(&app) as &dyn std::any::Any)
            .downcast_ref::<BoxDecoration>()
            .expect("a BoxDecoration");
        assert_eq!(
            painted.color.as_ref().map(AnyColor::color),
            Some(Color::from_argb(128, 0, 0, 0))
        );

        controller.dispose(&mut app);
    }

    // transitions_test.dart 'AlignTransition animates'
    #[test]
    fn an_align_transition_hands_the_animated_alignment_to_its_render_object() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = new_controller(&mut app);
        let alignment = controller.drive(
            &mut app,
            CallbackAnimatable::from_callback(|t| AlignmentGeometry::xy(t, -t)),
        );
        let widget = AlignTransition::new(alignment, ten_by_ten())
            .width_factor(2.0)
            .height_factor(3.0)
            .into_widget();
        let harness = Harness::mount(&mut app, widget);
        controller.set_value(&mut app, 0.5);
        harness.pump(&mut app);

        let positioned: RenderHandle<inset_rendering::RenderPositionedBox> =
            under_root(&harness, &app);
        assert_eq!(
            RenderAligningShiftedBox::alignment(positioned, &app),
            AlignmentGeometry::xy(0.5, -0.5)
        );
        assert_eq!(positioned.width_factor(&app), Some(2.0));
        assert_eq!(positioned.height_factor(&app), Some(3.0));
        assert_eq!(positioned.size(&app), Size::new(20.0, 30.0));

        controller.dispose(&mut app);
    }

    // transitions_test.dart 'DefaultTextStyleTransition test'
    #[test]
    fn a_default_text_style_transition_publishes_the_animated_style() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = new_controller(&mut app);
        let style = controller.drive(
            &mut app,
            CallbackAnimatable::from_callback(|t| TextStyle::new().font_size(10.0 + t * 10.0)),
        );
        let seen: Rc<Cell<f64>> = Rc::new(Cell::new(0.0));
        let child = Builder {
            key: None,
            builder: Rc::new({
                let seen = Rc::clone(&seen);
                move |app, context| {
                    let default = DefaultTextStyle::of(app, context);
                    seen.set(default.style.font_size.expect("a font size"));
                    ten_by_ten().into_widget()
                }
            }),
        };
        let widget = DefaultTextStyleTransition::new(style, child)
            .max_lines(3)
            .overflow(TextOverflow::Ellipsis)
            .soft_wrap(false)
            .into_widget();
        let harness = Harness::mount(&mut app, widget);
        controller.set_value(&mut app, 0.5);
        harness.pump(&mut app);
        assert_eq!(seen.get(), 15.0);

        controller.set_value(&mut app, 1.0);
        harness.pump(&mut app);
        assert_eq!(seen.get(), 20.0);

        controller.dispose(&mut app);
    }

    // transitions_test.dart 'FadeTransition animates'
    #[test]
    fn a_fade_transition_hands_its_animation_to_the_render_object() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = new_controller(&mut app);
        let harness = Harness::mount(&mut app, fade(controller.view()));
        harness.pump(&mut app);
        let render_object = fade_under_root(&harness, &app);
        assert_eq!(render_object.opacity(&app), controller.view());
        assert_eq!(render_object.opacity(&app).value(&app), 0.0);
        assert!(
            !render_object.as_object().is_repaint_boundary(&app),
            "fully transparent"
        );

        for value in [0.25, 0.5, 0.75, 1.0] {
            controller.set_value(&mut app, value);
            harness.pump(&mut app);
            assert_eq!(render_object.opacity(&app).value(&app), value);
        }
        assert!(
            render_object.as_object().is_repaint_boundary(&app),
            "the render object heard the controller"
        );

        let other = new_controller(&mut app);
        harness.set_child(&mut app, fade(other.view()));
        harness.pump(&mut app);
        assert_eq!(
            fade_under_root(&harness, &app),
            render_object,
            "reconfigured in place"
        );
        assert_eq!(render_object.opacity(&app), other.view());

        controller.dispose(&mut app);
        other.dispose(&mut app);
    }

    /// An animated widget sized by a notifier's value.
    #[derive(Debug)]
    struct Grower {
        notifier: Handle<ValueNotifier<f64>>,
        builds: Rc<Cell<u32>>,
    }

    impl AnimatedWidget for Grower {
        fn listenable(&self) -> Rc<dyn Listenable> {
            Rc::new(self.notifier)
        }

        fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
            self.builds.set(self.builds.get() + 1);
            let width = *app.get(self.notifier).value();
            Sized {
                size: Size::new(width, 10.0),
            }
            .into_widget()
        }
    }

    fn has_listeners(app: &App, notifier: Handle<ValueNotifier<f64>>) -> bool {
        app.get(notifier).change_notifier_data().has_listeners()
    }

    #[test]
    fn an_animated_widget_rebuilds_when_its_listenable_notifies() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let notifier = app.create(ValueNotifier::new(10.0));
        let builds = Rc::new(Cell::new(0));
        let harness = Harness::mount(
            &mut app,
            Grower {
                notifier,
                builds: Rc::clone(&builds),
            }
            .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(builds.get(), 1);
        assert!(has_listeners(&app, notifier));
        assert_eq!(root_child_size(&harness, &app), Size::new(10.0, 10.0));

        notifier.set_value(&mut app, 20.0);
        assert_eq!(builds.get(), 1, "rebuilt at the next frame, not inline");
        harness.pump(&mut app);
        assert_eq!(builds.get(), 2);
        assert_eq!(root_child_size(&harness, &app), Size::new(20.0, 10.0));
    }

    #[test]
    fn an_animated_widget_follows_a_new_listenable_and_unsubscribes_on_unmount() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let first = app.create(ValueNotifier::new(10.0));
        let second = app.create(ValueNotifier::new(30.0));
        let builds = Rc::new(Cell::new(0));
        let grower = |notifier| {
            Grower {
                notifier,
                builds: Rc::clone(&builds),
            }
            .into_widget()
        };
        let harness = Harness::mount(&mut app, grower(first));
        harness.pump(&mut app);

        harness.set_child(&mut app, grower(second));
        harness.pump(&mut app);
        assert!(
            !has_listeners(&app, first),
            "did_update_widget left the old listenable"
        );
        assert!(has_listeners(&app, second));
        assert_eq!(root_child_size(&harness, &app), Size::new(30.0, 10.0));

        harness.set_child(
            &mut app,
            Sized {
                size: Size::new(5.0, 5.0),
            }
            .into_widget(),
        );
        harness.pump(&mut app);
        assert!(!has_listeners(&app, second), "dispose removed the listener");
        let builds_before = builds.get();
        second.set_value(&mut app, 40.0);
        harness.pump(&mut app);
        assert_eq!(
            builds.get(),
            builds_before,
            "a disposed state never builds again"
        );
    }

    #[test]
    fn a_listenable_builder_rebuilds_on_notify_and_hands_back_its_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let notifier = app.create(ValueNotifier::new(0i32));
        let builds = Rc::new(Cell::new(0));
        let child: WidgetRef = SizedBox::square(Some(4.0)).into_widget();
        let same_child = Rc::new(Cell::new(true));
        let widget = AnimatedBuilder::new(Rc::new(notifier), {
            let builds = Rc::clone(&builds);
            let same_child = Rc::clone(&same_child);
            let child = child.clone();
            move |_app, _context, passed| {
                builds.set(builds.get() + 1);
                same_child.set(passed.is_some_and(|passed| Rc::ptr_eq(passed, &child)));
                passed.expect("the child is passed back").clone()
            }
        })
        .child(child.clone());
        let harness = Harness::mount(&mut app, widget.into_widget());
        harness.pump(&mut app);
        assert_eq!(builds.get(), 1);
        assert!(same_child.get());

        notifier.set_value(&mut app, 1);
        harness.pump(&mut app);
        assert_eq!(builds.get(), 2);
    }
}
