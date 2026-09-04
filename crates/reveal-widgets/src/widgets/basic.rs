//! Flutter counterpart: `widgets/basic.dart`.
//!
//! `DecoratedBox` is Flutter's `widgets/container.dart`; it lives here until that file is
//! ported. `WidgetBuilder` is Flutter's `framework.dart` typedef; the framework module does
//! not export one, so it is defined here next to `Builder`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_embedder::{Size, TextDirection};
use reveal_foundation::App;
use reveal_gestures::{
    PointerCancelEventListener, PointerDownEventListener, PointerEnterEventListener,
    PointerExitEventListener, PointerHoverEventListener, PointerMoveEventListener,
    PointerPanZoomEndEventListener, PointerPanZoomStartEventListener,
    PointerPanZoomUpdateEventListener, PointerSignalEventListener, PointerUpEventListener,
};
use reveal_painting::{AlignmentGeometry, Decoration, EdgeInsetsGeometry};
use reveal_rendering::{
    AnyRenderObject, BoxConstraints, DecorationPosition, HitTestBehavior, RenderAligningShiftedBox,
    RenderBox, RenderConstrainedBox, RenderDecoratedBox, RenderHandle, RenderMouseRegion,
    RenderOpacity, RenderPadding, RenderPointerListener, RenderPositionedBox,
    RenderRepaintBoundary,
};
use reveal_services::{MouseCursor, MouseCursorRef};

use crate::framework::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, RenderObjectWidget,
    SingleChildRenderObjectWidget, StatelessWidget, WidgetRef,
};
use crate::widgets::image::create_local_image_configuration;

// BIDIRECTIONAL TEXT SUPPORT

/// A widget that determines the ambient directionality of text and
/// text-direction-sensitive render objects.
///
/// For example, [`Padding`] depends on the [`Directionality`] to resolve
/// `EdgeInsetsDirectional` objects into absolute `EdgeInsets` objects.
///
/// This example uses a right-to-left [`TextDirection`] and draws a blue box with
/// a right margin of 8 pixels.
///
/// ```text
/// Directionality {
///   text_direction: TextDirection::Rtl,
///   child: Container(margin: EdgeInsetsGeometry::directional(8.0, 0.0, 0.0, 0.0), color: blue),
/// }
/// ```
///
/// Flutter's `_UbiquitousInheritedWidget` / `_UbiquitousInheritedElement` is an
/// optimisation that skips per-dependent bookkeeping and notifies by walking the subtree;
/// this is a plain [`InheritedWidget`] with the same observable behaviour.
#[derive(Debug)]
pub struct Directionality {
    pub key: Option<KeyRef>,
    /// The text direction for this subtree.
    pub text_direction: TextDirection,
    pub child: WidgetRef,
}

impl Directionality {
    /// Creates a `Directionality`; Dart's optional named arguments are the setters.
    pub fn new<K>(text_direction: TextDirection, child: impl IntoWidget<K>) -> Directionality {
        Directionality {
            key: None,
            text_direction,
            child: child.into_widget(),
        }
    }

    /// Dart `Directionality(key:)`.
    pub fn key(mut self, key: KeyRef) -> Directionality {
        self.key = Some(key);
        self
    }

    /// The text direction from the closest instance of this class that encloses
    /// the given context.
    ///
    /// If there is no [`Directionality`] ancestor widget in the tree at the given
    /// context, then this will panic with a descriptive message.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let text_direction = Directionality::of(app, context);
    /// ```
    ///
    /// See also:
    ///
    ///  * [`maybe_of`](Self::maybe_of), which will return `None` if no [`Directionality`]
    ///    ancestor widget is in the tree.
    pub fn of(app: &mut App, context: BuildContext) -> TextDirection {
        let widget = context
            .depend_on_inherited_widget_of_exact_type::<Directionality>(app)
            .expect(
                "No Directionality widget found. This widget requires a Directionality widget \
                 ancestor. Typically, the Directionality widget is introduced by the WidgetsApp \
                 widget at the top of your application widget tree. It determines the ambient \
                 reading direction and is used, for example, to determine how to lay out text, \
                 how to interpret \"start\" and \"end\" values, and to resolve \
                 EdgeInsetsDirectional, AlignmentDirectional, and other *Directional objects.",
            );
        widget.text_direction
    }

    /// The text direction from the closest instance of this class that encloses
    /// the given context.
    ///
    /// If there is no [`Directionality`] ancestor widget in the tree at the given
    /// context, then this will return `None`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let text_direction: Option<TextDirection> = Directionality::maybe_of(app, context);
    /// ```
    ///
    /// See also:
    ///
    ///  * [`of`](Self::of), which will panic if no [`Directionality`] ancestor widget is in
    ///    the tree.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<TextDirection> {
        let widget = context.depend_on_inherited_widget_of_exact_type::<Directionality>(app);
        widget.map(|widget| widget.text_direction)
    }
}

impl InheritedWidget for Directionality {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &Directionality) -> bool {
        self.text_direction != old_widget.text_direction
    }
}

// PAINTING NODES

/// A widget that makes its child partially transparent.
///
/// This class paints its child into an intermediate buffer and then blends the
/// child back into the scene partially transparent.
///
/// For values of opacity other than 0.0 and 1.0, this class is relatively
/// expensive because it requires painting the child into an intermediate
/// buffer. For the value 0.0, the child is not painted at all. For the
/// value 1.0, the child is painted immediately without an intermediate buffer.
///
/// The presence of the intermediate buffer which has a transparent background
/// by default may cause some child widgets to behave differently. For example
/// a `BackdropFilter` child will only be able to apply its filter to the content
/// between this widget and the backdrop child and may require adjusting the
/// `BackdropFilter.blendMode` property to produce the desired results.
///
/// This example shows some `Text` when the `visible` member field is true, and
/// hides it when it is false:
///
/// ```text
/// Opacity {
///   opacity: if visible { 1.0 } else { 0.0 },
///   child: Some(Text("Now you see me, now you don't!")),
///   ..Default::default()
/// }
/// ```
///
/// This is more efficient than adding and removing the child widget from the
/// tree on demand.
///
/// ## Performance considerations for opacity animation
///
/// Animating an [`Opacity`] widget directly causes the widget (and possibly its
/// subtree) to rebuild each frame, which is not very efficient. Consider using
/// one of these alternative widgets instead:
///
///  * `AnimatedOpacity`, which uses an animation internally to efficiently
///    animate opacity.
///  * `FadeTransition`, which uses a provided animation to efficiently animate
///    opacity.
///
/// ## Transparent image
///
/// If only a single `Image` or `Color` needs to be composited with an opacity
/// between 0.0 and 1.0, it's much faster to directly use them without [`Opacity`]
/// widgets.
///
/// For example, `Container(color: Color.fromRGBO(255, 0, 0, 0.5))` is much
/// faster than `Opacity(opacity: 0.5, child: Container(color: Colors.red))`.
///
/// Directly drawing an `Image` or `Color` with opacity is faster than using
/// [`Opacity`] on top of them because [`Opacity`] could apply the opacity to a
/// group of widgets and therefore a costly offscreen buffer will be used.
/// Drawing content into the offscreen buffer may also trigger render target
/// switches and such switching is particularly slow in older GPUs.
///
/// ## Hit testing
///
/// Setting the [`opacity`](Self::opacity) to zero does not prevent hit testing from being
/// applied to the descendants of the [`Opacity`] widget. This can be confusing for the
/// user, who may not see anything, and may believe the area of the interface
/// where the [`Opacity`] is hiding a widget to be non-interactive.
///
/// With certain widgets, such as `Flow`, that compute their positions only when
/// they are painted, this can actually lead to bugs (from unexpected geometry
/// to exceptions), because those widgets are not painted by the [`Opacity`]
/// widget at all when the [`opacity`](Self::opacity) is zero.
///
/// To avoid such problems, it is generally a good idea to use an
/// `IgnorePointer` widget when setting the [`opacity`](Self::opacity) to zero. This
/// prevents interactions with any children in the subtree.
///
/// See also:
///
///  * `Visibility`, which can hide a child more efficiently (albeit less
///    subtly, because it is either visible or hidden, rather than allowing
///    fractional opacity values). Specifically, the `Visibility.maintain`
///    constructor is equivalent to using an opacity widget with values of
///    `0.0` or `1.0`.
///  * `ShaderMask`, which can apply more elaborate effects to its child.
///  * `Transform`, which applies an arbitrary transform to its child widget at
///    paint time.
///  * `SliverOpacity`, the sliver version of this widget.
///
/// Flutter's `alwaysIncludeSemantics` waits on accessibility.
#[derive(Debug)]
pub struct Opacity {
    pub key: Option<KeyRef>,
    /// The fraction to scale the child's alpha value.
    ///
    /// An opacity of one is fully opaque. An opacity of zero is fully transparent
    /// (i.e., invisible).
    ///
    /// Values one and zero are painted with a fast path. Other values require
    /// painting the child into an intermediate buffer, which is expensive.
    ///
    /// Must be between zero and one, inclusive.
    pub opacity: f64,
    pub child: Option<WidgetRef>,
}

impl Opacity {
    /// Creates a `Opacity`; Dart's optional named arguments are the setters.
    pub fn new(opacity: f64) -> Opacity {
        Opacity {
            key: None,
            opacity,
            child: None,
        }
    }

    /// Dart `Opacity(key:)`.
    pub fn key(mut self, key: KeyRef) -> Opacity {
        self.key = Some(key);
        self
    }

    /// Dart `Opacity(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Opacity {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for Opacity {
    type RenderObject = RenderOpacity;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderOpacity::new(app, self.opacity, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderOpacity>,
    ) {
        render_object.set_opacity(app, self.opacity);
    }
}

impl SingleChildRenderObjectWidget for Opacity {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A widget that paints a [`Decoration`] either before or after its child paints.
///
/// `Container` insets its child by the widths of the borders; this widget does
/// not.
///
/// Commonly used with `BoxDecoration`.
///
/// The [`child`](Self::child) is not clipped. To clip a child to the shape of a particular
/// `ShapeDecoration`, consider using a `ClipPath` widget.
///
/// This sample shows a radial gradient that draws a moon on a night sky:
///
/// ```text
/// DecoratedBox {
///   decoration: Box::new(BoxDecoration::new().gradient(RadialGradient { .. })),
///   ..
/// }
/// ```
///
/// See also:
///
///  * `Ink`, which paints a [`Decoration`] on a `Material`, allowing
///    `InkResponse` and `InkWell` splashes to paint over them.
///  * `DecoratedSliver`, which applies a [`Decoration`] to a sliver.
///  * `DecoratedBoxTransition`, the version of this class that animates on the
///    [`decoration`](Self::decoration).
///  * [`Decoration`], which you can extend to provide other effects with
///    [`DecoratedBox`].
///  * `CustomPaint`, another way to draw custom effects from the widget layer.
///
/// Flutter home: `widgets/container.dart`.
#[derive(Debug)]
pub struct DecoratedBox {
    pub key: Option<KeyRef>,
    /// What decoration to paint.
    ///
    /// Commonly a `BoxDecoration`.
    pub decoration: Box<dyn Decoration>,
    /// Whether to paint the box decoration behind or in front of the child.
    ///
    /// By default the decoration paints behind the child.
    pub position: DecorationPosition,
    pub child: Option<WidgetRef>,
}

impl DecoratedBox {
    /// Creates a `DecoratedBox`; Dart's optional named arguments are the setters.
    pub fn new(decoration: impl Decoration + 'static) -> DecoratedBox {
        DecoratedBox {
            key: None,
            decoration: Box::new(decoration),
            position: DecorationPosition::Background,
            child: None,
        }
    }

    /// Dart `DecoratedBox(key:)`.
    pub fn key(mut self, key: KeyRef) -> DecoratedBox {
        self.key = Some(key);
        self
    }

    /// Dart `DecoratedBox(position:)`.
    pub fn position(mut self, position: DecorationPosition) -> DecoratedBox {
        self.position = position;
        self
    }

    /// Dart `DecoratedBox(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> DecoratedBox {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for DecoratedBox {
    type RenderObject = RenderDecoratedBox;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let configuration = create_local_image_configuration(app, context, None);
        RenderDecoratedBox::new(
            app,
            self.decoration.clone_box(),
            self.position,
            configuration,
            None,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderDecoratedBox>,
    ) {
        render_object.set_decoration(app, self.decoration.clone_box());
        let configuration = create_local_image_configuration(app, context, None);
        render_object.set_configuration(app, configuration);
        render_object.set_position(app, self.position);
    }
}

impl SingleChildRenderObjectWidget for DecoratedBox {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

// POSITIONING AND SIZING NODES

/// A widget that insets its child by the given padding.
///
/// When passing layout constraints to its child, padding shrinks the
/// constraints by the given padding, causing the child to layout at a smaller
/// size. Padding then sizes itself to its child's size, inflated by the
/// padding, effectively creating empty space around the child.
///
/// This snippet creates "Hello World!" `Text` inside a `Card` that is indented
/// by sixteen pixels in each direction.
///
/// ```text
/// Card {
///   child: Padding {
///     padding: EdgeInsetsGeometry::all(16.0),
///     child: Some(Text("Hello World!")),
///     ..Default::default()
///   },
/// }
/// ```
///
/// ## Design discussion
///
/// ### Why use a [`Padding`] widget rather than a `Container` with `Container.padding`?
///
/// There isn't really any difference between the two. If you supply a
/// `Container.padding` argument, `Container` builds a [`Padding`] widget
/// for you.
///
/// `Container` doesn't implement its properties directly. Instead, `Container`
/// combines a number of simpler widgets together into a convenient package. For
/// example, the `Container.padding` property causes the container to build a
/// [`Padding`] widget and the `Container.decoration` property causes the
/// container to build a [`DecoratedBox`] widget. If you find `Container`
/// convenient, feel free to use it. If not, feel free to build these simpler
/// widgets in whatever combination meets your needs.
///
/// In fact, the majority of widgets in Flutter are combinations of other
/// simpler widgets. Composition, rather than inheritance, is the primary
/// mechanism for building up widgets.
///
/// See also:
///
///  * `EdgeInsets`, the class that is used to describe the padding dimensions.
///  * `AnimatedPadding`, which animates changes in [`padding`](Self::padding) over a given
///    duration.
///  * `SliverPadding`, the sliver equivalent of this widget.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug)]
pub struct Padding {
    pub key: Option<KeyRef>,
    /// The amount of space by which to inset the child.
    pub padding: EdgeInsetsGeometry,
    pub child: Option<WidgetRef>,
}

impl Padding {
    /// Creates a `Padding`; Dart's optional named arguments are the setters.
    pub fn new(padding: EdgeInsetsGeometry) -> Padding {
        Padding {
            key: None,
            padding,
            child: None,
        }
    }

    /// Dart `Padding(key:)`.
    pub fn key(mut self, key: KeyRef) -> Padding {
        self.key = Some(key);
        self
    }

    /// Dart `Padding(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Padding {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for Padding {
    type RenderObject = RenderPadding;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::maybe_of(app, context);
        RenderPadding::new(app, self.padding, text_direction, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderPadding>,
    ) {
        render_object.set_padding(app, self.padding);
        let text_direction = Directionality::maybe_of(app, context);
        render_object.set_text_direction(app, text_direction);
    }
}

impl SingleChildRenderObjectWidget for Padding {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A widget that aligns its child within itself and optionally sizes itself
/// based on the child's size.
///
/// For example, to align a box at the bottom right, you would pass this box a
/// tight constraint that is bigger than the child's natural size,
/// with an alignment of `Alignment::BOTTOM_RIGHT`.
///
/// This widget will be as big as possible if its dimensions are constrained and
/// [`width_factor`](Self::width_factor) and [`height_factor`](Self::height_factor) are
/// `None`. If a dimension is unconstrained and the corresponding size factor is `None` then
/// the widget will match its child's size in that dimension. If a size factor is non-`None`
/// then the corresponding dimension of this widget will be the product of the child's
/// dimension and the size factor. For example if `width_factor` is 2.0 then
/// the width of this widget will always be twice its child's width.
///
/// ## How it works
///
/// The [`alignment`](Self::alignment) property describes a point in the `child`'s coordinate
/// system and a different point in the coordinate system of this widget. The [`Align`]
/// widget positions the `child` such that both points are lined up on top of
/// each other.
///
/// See also:
///
///  * `AnimatedAlign`, which animates changes in [`alignment`](Self::alignment) smoothly
///    over a given duration.
///  * `CustomSingleChildLayout`, which uses a delegate to control the layout of
///    a single child.
///  * [`Center`], which is the same as [`Align`] but with the
///    [`alignment`](Self::alignment) always set to `Alignment::CENTER`.
///  * `FractionallySizedBox`, which sizes its child based on a fraction of its
///    own size and positions the child according to an `Alignment` value.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug)]
pub struct Align {
    pub key: Option<KeyRef>,
    /// How to align the child.
    ///
    /// The x and y values of the `Alignment` control the horizontal and vertical
    /// alignment, respectively. An x value of -1.0 means that the left edge of
    /// the child is aligned with the left edge of the parent whereas an x value
    /// of 1.0 means that the right edge of the child is aligned with the right
    /// edge of the parent. Other values interpolate (and extrapolate) linearly.
    /// For example, a value of 0.0 means that the center of the child is aligned
    /// with the center of the parent.
    ///
    /// Defaults to `AlignmentGeometry::CENTER`.
    ///
    /// See also:
    ///
    ///  * `Alignment`, which has more details and some convenience constants for
    ///    common positions.
    ///  * `AlignmentDirectional`, which has a horizontal coordinate orientation
    ///    that depends on the [`TextDirection`].
    pub alignment: AlignmentGeometry,
    /// If non-`None`, sets its width to the child's width multiplied by this factor.
    ///
    /// Can be both greater and less than 1.0 but must be non-negative.
    pub width_factor: Option<f64>,
    /// If non-`None`, sets its height to the child's height multiplied by this factor.
    ///
    /// Can be both greater and less than 1.0 but must be non-negative.
    pub height_factor: Option<f64>,
    pub child: Option<WidgetRef>,
}

impl Align {
    /// Creates a `Align`; Dart's named arguments are the setters.
    pub fn new() -> Align {
        Align::default()
    }

    /// Dart `Align(key:)`.
    pub fn key(mut self, key: KeyRef) -> Align {
        self.key = Some(key);
        self
    }

    /// Dart `Align(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> Align {
        self.alignment = alignment;
        self
    }

    /// Dart `Align(width_factor:)`.
    pub fn width_factor(mut self, width_factor: f64) -> Align {
        self.width_factor = Some(width_factor);
        self
    }

    /// Dart `Align(height_factor:)`.
    pub fn height_factor(mut self, height_factor: f64) -> Align {
        self.height_factor = Some(height_factor);
        self
    }

    /// Dart `Align(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Align {
        self.child = Some(child.into_widget());
        self
    }

    /// `Align.createRenderObject`'s body, shared with [`Center`].
    fn create_positioned_box(
        app: &mut App,
        context: BuildContext,
        alignment: AlignmentGeometry,
        width_factor: Option<f64>,
        height_factor: Option<f64>,
    ) -> AnyRenderObject {
        let text_direction = Directionality::maybe_of(app, context);
        RenderPositionedBox::new(
            app,
            alignment,
            text_direction,
            width_factor,
            height_factor,
            None,
        )
        .as_object()
    }

    /// `Align.updateRenderObject`'s body, shared with [`Center`].
    fn update_positioned_box(
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderPositionedBox>,
        alignment: AlignmentGeometry,
        width_factor: Option<f64>,
        height_factor: Option<f64>,
    ) {
        render_object.set_alignment(app, alignment);
        render_object.set_width_factor(app, width_factor);
        render_object.set_height_factor(app, height_factor);
        let text_direction = Directionality::maybe_of(app, context);
        render_object.set_text_direction(app, text_direction);
    }
}

impl Default for Align {
    fn default() -> Align {
        Align {
            key: None,
            alignment: AlignmentGeometry::CENTER,
            width_factor: None,
            height_factor: None,
            child: None,
        }
    }
}

impl RenderObjectWidget for Align {
    type RenderObject = RenderPositionedBox;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        Align::create_positioned_box(
            app,
            context,
            self.alignment,
            self.width_factor,
            self.height_factor,
        )
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderPositionedBox>,
    ) {
        Align::update_positioned_box(
            app,
            context,
            render_object,
            self.alignment,
            self.width_factor,
            self.height_factor,
        );
    }
}

impl SingleChildRenderObjectWidget for Align {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A widget that centers its child within itself.
///
/// This widget will be as big as possible if its dimensions are constrained and
/// [`width_factor`](Self::width_factor) and [`height_factor`](Self::height_factor) are
/// `None`. If a dimension is unconstrained and the corresponding size factor is `None` then
/// the widget will match its child's size in that dimension. If a size factor is non-`None`
/// then the corresponding dimension of this widget will be the product of the child's
/// dimension and the size factor. For example if `width_factor` is 2.0 then
/// the width of this widget will always be twice its child's width.
///
/// See also:
///
///  * [`Align`], which lets you arbitrarily position a child within itself,
///    rather than just centering it.
///  * `Row`, a widget that displays its children in a horizontal array.
///  * `Column`, a widget that displays its children in a vertical array.
///  * `Container`, a convenience widget that combines common painting,
///    positioning, and sizing widgets.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
///
/// Dart's `class Center extends Align`: a distinct widget type whose behaviour is
/// [`Align`]'s with the alignment fixed at `AlignmentGeometry::CENTER`.
#[derive(Debug, Default)]
pub struct Center {
    pub key: Option<KeyRef>,
    /// See [`Align::width_factor`].
    pub width_factor: Option<f64>,
    /// See [`Align::height_factor`].
    pub height_factor: Option<f64>,
    pub child: Option<WidgetRef>,
}

impl Center {
    /// Creates a `Center`; Dart's named arguments are the setters.
    pub fn new() -> Center {
        Center::default()
    }

    /// Dart `Center(key:)`.
    pub fn key(mut self, key: KeyRef) -> Center {
        self.key = Some(key);
        self
    }

    /// Dart `Center(width_factor:)`.
    pub fn width_factor(mut self, width_factor: f64) -> Center {
        self.width_factor = Some(width_factor);
        self
    }

    /// Dart `Center(height_factor:)`.
    pub fn height_factor(mut self, height_factor: f64) -> Center {
        self.height_factor = Some(height_factor);
        self
    }

    /// Dart `Center(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Center {
        self.child = Some(child.into_widget());
        self
    }

    /// How to align the child: always `AlignmentGeometry::CENTER` (the inherited
    /// [`Align::alignment`]).
    pub fn alignment(&self) -> AlignmentGeometry {
        AlignmentGeometry::CENTER
    }
}

impl RenderObjectWidget for Center {
    type RenderObject = RenderPositionedBox;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        Align::create_positioned_box(
            app,
            context,
            self.alignment(),
            self.width_factor,
            self.height_factor,
        )
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderPositionedBox>,
    ) {
        Align::update_positioned_box(
            app,
            context,
            render_object,
            self.alignment(),
            self.width_factor,
            self.height_factor,
        );
    }
}

impl SingleChildRenderObjectWidget for Center {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A box with a specified size.
///
/// If given a child, this widget forces it to have a specific width and/or height.
/// These values will be ignored if this widget's parent does not permit them.
/// For example, this happens if the parent is the screen (forces the child to
/// be the same size as the parent), or another [`SizedBox`] (forces its child to
/// have a specific width and/or height). This can be remedied by wrapping the
/// child [`SizedBox`] in a widget that does permit it to be any size up to the
/// size of the parent, such as [`Center`] or [`Align`].
///
/// If either the width or height is `None`, this widget will try to size itself to
/// match the child's size in that dimension. If the child's size depends on the
/// size of its parent, the height and width must be provided.
///
/// If not given a child, [`SizedBox`] will try to size itself as close to the
/// specified height and width as possible given the parent's constraints. If
/// `height` or `width` is `None` or unspecified, it will be treated as zero.
///
/// The [`expand`](Self::expand) constructor can be used to make a [`SizedBox`] that sizes
/// itself to fit the parent. It is equivalent to setting `width` and `height` to
/// `f64::INFINITY`.
///
/// This snippet makes the child widget (a `Card` with some `Text`) have the
/// exact size 200x300, parental constraints permitting:
///
/// ```text
/// SizedBox {
///   width: Some(200.0),
///   height: Some(300.0),
///   child: Some(Card(child: Text("Hello World!"))),
///   ..Default::default()
/// }
/// ```
///
/// See also:
///
///  * [`ConstrainedBox`], a more generic version of this class that takes
///    arbitrary [`BoxConstraints`] instead of an explicit width and height.
///  * `UnconstrainedBox`, a container that tries to let its child draw without
///    constraints.
///  * `FractionallySizedBox`, a widget that sizes its child to a fraction of
///    the total available space.
///  * `AspectRatio`, a widget that attempts to fit within the parent's
///    constraints while also sizing its child to match a given aspect ratio.
///  * `FittedBox`, which sizes and positions its child widget to fit the parent
///    according to a given `BoxFit` discipline.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
///
/// Dart's named constructors are the associated functions [`expand`](Self::expand),
/// [`shrink`](Self::shrink), [`from_size`](Self::from_size), and [`square`](Self::square);
/// set a key through struct update syntax: `SizedBox { key: Some(key), ..SizedBox::expand(child) }`.
#[derive(Default)]
pub struct SizedBox {
    pub key: Option<KeyRef>,
    /// If non-`None`, requires the child to have exactly this width.
    pub width: Option<f64>,
    /// If non-`None`, requires the child to have exactly this height.
    pub height: Option<f64>,
    pub child: Option<WidgetRef>,
}

impl SizedBox {
    /// Creates a `SizedBox`; Dart's named arguments are the setters.
    pub fn new() -> SizedBox {
        SizedBox::default()
    }

    /// Dart `SizedBox(key:)`.
    pub fn key(mut self, key: KeyRef) -> SizedBox {
        self.key = Some(key);
        self
    }

    /// Dart `SizedBox(width:)`.
    pub fn width(mut self, width: f64) -> SizedBox {
        self.width = Some(width);
        self
    }

    /// Dart `SizedBox(height:)`.
    pub fn height(mut self, height: f64) -> SizedBox {
        self.height = Some(height);
        self
    }

    /// Dart `SizedBox(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> SizedBox {
        self.child = Some(child.into_widget());
        self
    }

    /// Creates a box that will become as large as its parent allows.
    pub fn expand() -> SizedBox {
        SizedBox {
            key: None,
            width: Some(f64::INFINITY),
            height: Some(f64::INFINITY),
            child: None,
        }
    }

    /// Creates a box that will become as small as its parent allows.
    pub fn shrink() -> SizedBox {
        SizedBox {
            key: None,
            width: Some(0.0),
            height: Some(0.0),
            child: None,
        }
    }

    /// Creates a box with the specified size.
    pub fn from_size(size: Option<Size>) -> SizedBox {
        SizedBox {
            key: None,
            width: size.map(|size| size.width()),
            height: size.map(|size| size.height()),
            child: None,
        }
    }

    /// Creates a box whose [`width`](Self::width) and [`height`](Self::height) are equal.
    pub fn square(dimension: Option<f64>) -> SizedBox {
        SizedBox {
            key: None,
            width: dimension,
            height: dimension,
            child: None,
        }
    }

    fn additional_constraints(&self) -> BoxConstraints {
        BoxConstraints::tight_for(self.width, self.height)
    }

    /// Dart's `toStringShort`: the named constructor this box is equivalent to.
    fn short_name(&self) -> &'static str {
        match (self.width, self.height) {
            (Some(f64::INFINITY), Some(f64::INFINITY)) => "SizedBox.expand",
            (Some(0.0), Some(0.0)) => "SizedBox.shrink",
            _ => "SizedBox",
        }
    }
}

impl RenderObjectWidget for SizedBox {
    type RenderObject = RenderConstrainedBox;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderConstrainedBox::new(app, self.additional_constraints(), None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderConstrainedBox>,
    ) {
        render_object.set_additional_constraints(app, self.additional_constraints());
    }
}

impl SingleChildRenderObjectWidget for SizedBox {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

impl Debug for SizedBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct(self.short_name());
        debug.field("key", &self.key);
        if self.short_name() == "SizedBox" {
            debug.field("width", &self.width);
            debug.field("height", &self.height);
        }
        debug.field("child", &self.child).finish()
    }
}

/// A widget that imposes additional constraints on its child.
///
/// For example, if you wanted [`child`](Self::child) to have a minimum height of 50.0 logical
/// pixels, you could use `BoxConstraints::new().min_height(50.0)` as the
/// [`constraints`](Self::constraints).
///
/// This snippet makes the child widget (a `Card` with some `Text`) fill the
/// parent, by applying `BoxConstraints.expand` constraints:
///
/// ```text
/// ConstrainedBox {
///   constraints: BoxConstraints::expand(),
///   child: Some(Card(child: Text("Hello World!"))),
///   key: None,
/// }
/// ```
///
/// The same behavior can be obtained using the [`SizedBox::expand`] widget.
///
/// See also:
///
///  * [`BoxConstraints`], the class that describes constraints.
///  * `UnconstrainedBox`, a container that tries to let its child draw without
///    constraints.
///  * [`SizedBox`], which lets you specify tight constraints by explicitly
///    specifying the height or width.
///  * `FractionallySizedBox`, which sizes its child based on a fraction of its
///    own size and positions the child according to an `Alignment` value.
///  * `AspectRatio`, a widget that attempts to fit within the parent's
///    constraints while also sizing its child to match a given aspect ratio.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug)]
pub struct ConstrainedBox {
    pub key: Option<KeyRef>,
    /// The additional constraints to impose on the child.
    ///
    /// Must be valid (`BoxConstraints::debug_assert_is_valid`).
    pub constraints: BoxConstraints,
    pub child: Option<WidgetRef>,
}

impl ConstrainedBox {
    /// Creates a `ConstrainedBox`; Dart's optional named arguments are the setters.
    pub fn new(constraints: BoxConstraints) -> ConstrainedBox {
        ConstrainedBox {
            key: None,
            constraints,
            child: None,
        }
    }

    /// Dart `ConstrainedBox(key:)`.
    pub fn key(mut self, key: KeyRef) -> ConstrainedBox {
        self.key = Some(key);
        self
    }

    /// Dart `ConstrainedBox(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> ConstrainedBox {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for ConstrainedBox {
    type RenderObject = RenderConstrainedBox;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderConstrainedBox::new(app, self.constraints, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderConstrainedBox>,
    ) {
        render_object.set_additional_constraints(app, self.constraints);
    }
}

impl SingleChildRenderObjectWidget for ConstrainedBox {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

// EVENT HANDLING

/// A widget that calls callbacks in response to common pointer events.
///
/// It listens to events that can construct gestures, such as when the
/// pointer is pressed, moved, then released or canceled.
///
/// It does not listen to events that are exclusive to mouse, such as when the
/// mouse enters, exits or hovers a region without pressing any buttons. For
/// these events, use [`MouseRegion`].
///
/// Rather than listening for raw pointer events, consider listening for
/// higher-level gestures using `GestureDetector`.
///
/// ## Layout behavior
///
/// _See [`BoxConstraints`] for an introduction to box layout models._
///
/// If it has a child, this widget defers to the child for sizing behavior. If
/// it does not have a child, it grows to fit the parent instead.
///
/// The [`behavior`](Self::behavior) argument defaults to [`HitTestBehavior::DeferToChild`].
pub struct Listener {
    pub key: Option<KeyRef>,
    /// Called when a pointer comes into contact with the screen (for touch
    /// pointers), or has its button pressed (for mouse pointers) at this widget's
    /// location.
    pub on_pointer_down: Option<PointerDownEventListener>,
    /// Called when a pointer that triggered an [`on_pointer_down`](Self::on_pointer_down)
    /// changes position.
    pub on_pointer_move: Option<PointerMoveEventListener>,
    /// Called when a pointer that triggered an [`on_pointer_down`](Self::on_pointer_down) is
    /// no longer in contact with the screen.
    pub on_pointer_up: Option<PointerUpEventListener>,
    /// Called when a pointer that has not triggered an
    /// [`on_pointer_down`](Self::on_pointer_down) changes position.
    ///
    /// This is only fired for pointers which report their location when not down
    /// (e.g. mouse pointers, but not most touch pointers).
    pub on_pointer_hover: Option<PointerHoverEventListener>,
    /// Called when the input from a pointer that triggered an
    /// [`on_pointer_down`](Self::on_pointer_down) is no longer directed towards this receiver.
    pub on_pointer_cancel: Option<PointerCancelEventListener>,
    /// Called when a pan/zoom begins such as from a trackpad gesture.
    pub on_pointer_pan_zoom_start: Option<PointerPanZoomStartEventListener>,
    /// Called when a pan/zoom is updated.
    pub on_pointer_pan_zoom_update: Option<PointerPanZoomUpdateEventListener>,
    /// Called when a pan/zoom finishes.
    pub on_pointer_pan_zoom_end: Option<PointerPanZoomEndEventListener>,
    /// Called when a pointer signal occurs over this object.
    ///
    /// See also:
    ///
    ///  * `PointerSignalEvent`, which goes into more detail on pointer signal
    ///    events.
    pub on_pointer_signal: Option<PointerSignalEventListener>,
    /// How to behave during hit testing.
    pub behavior: HitTestBehavior,
    pub child: Option<WidgetRef>,
}

impl Listener {
    /// Creates a `Listener`; Dart's named arguments are the setters.
    pub fn new() -> Listener {
        Listener::default()
    }

    /// Dart `Listener(key:)`.
    pub fn key(mut self, key: KeyRef) -> Listener {
        self.key = Some(key);
        self
    }

    /// Dart `Listener(on_pointer_down:)`.
    pub fn on_pointer_down(mut self, on_pointer_down: PointerDownEventListener) -> Listener {
        self.on_pointer_down = Some(on_pointer_down);
        self
    }

    /// Dart `Listener(on_pointer_move:)`.
    pub fn on_pointer_move(mut self, on_pointer_move: PointerMoveEventListener) -> Listener {
        self.on_pointer_move = Some(on_pointer_move);
        self
    }

    /// Dart `Listener(on_pointer_up:)`.
    pub fn on_pointer_up(mut self, on_pointer_up: PointerUpEventListener) -> Listener {
        self.on_pointer_up = Some(on_pointer_up);
        self
    }

    /// Dart `Listener(on_pointer_hover:)`.
    pub fn on_pointer_hover(mut self, on_pointer_hover: PointerHoverEventListener) -> Listener {
        self.on_pointer_hover = Some(on_pointer_hover);
        self
    }

    /// Dart `Listener(on_pointer_cancel:)`.
    pub fn on_pointer_cancel(mut self, on_pointer_cancel: PointerCancelEventListener) -> Listener {
        self.on_pointer_cancel = Some(on_pointer_cancel);
        self
    }

    /// Dart `Listener(on_pointer_pan_zoom_start:)`.
    pub fn on_pointer_pan_zoom_start(
        mut self,
        on_pointer_pan_zoom_start: PointerPanZoomStartEventListener,
    ) -> Listener {
        self.on_pointer_pan_zoom_start = Some(on_pointer_pan_zoom_start);
        self
    }

    /// Dart `Listener(on_pointer_pan_zoom_update:)`.
    pub fn on_pointer_pan_zoom_update(
        mut self,
        on_pointer_pan_zoom_update: PointerPanZoomUpdateEventListener,
    ) -> Listener {
        self.on_pointer_pan_zoom_update = Some(on_pointer_pan_zoom_update);
        self
    }

    /// Dart `Listener(on_pointer_pan_zoom_end:)`.
    pub fn on_pointer_pan_zoom_end(
        mut self,
        on_pointer_pan_zoom_end: PointerPanZoomEndEventListener,
    ) -> Listener {
        self.on_pointer_pan_zoom_end = Some(on_pointer_pan_zoom_end);
        self
    }

    /// Dart `Listener(on_pointer_signal:)`.
    pub fn on_pointer_signal(mut self, on_pointer_signal: PointerSignalEventListener) -> Listener {
        self.on_pointer_signal = Some(on_pointer_signal);
        self
    }

    /// Dart `Listener(behavior:)`.
    pub fn behavior(mut self, behavior: HitTestBehavior) -> Listener {
        self.behavior = behavior;
        self
    }

    /// Dart `Listener(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Listener {
        self.child = Some(child.into_widget());
        self
    }

    /// Copies the callbacks onto the render object; Dart's constructor and the setters
    /// share this list.
    fn apply_listeners(&self, app: &mut App, render_object: RenderHandle<RenderPointerListener>) {
        render_object.set_on_pointer_down(app, self.on_pointer_down.clone());
        render_object.set_on_pointer_move(app, self.on_pointer_move.clone());
        render_object.set_on_pointer_up(app, self.on_pointer_up.clone());
        render_object.set_on_pointer_hover(app, self.on_pointer_hover.clone());
        render_object.set_on_pointer_cancel(app, self.on_pointer_cancel.clone());
        render_object.set_on_pointer_pan_zoom_start(app, self.on_pointer_pan_zoom_start.clone());
        render_object.set_on_pointer_pan_zoom_update(app, self.on_pointer_pan_zoom_update.clone());
        render_object.set_on_pointer_pan_zoom_end(app, self.on_pointer_pan_zoom_end.clone());
        render_object.set_on_pointer_signal(app, self.on_pointer_signal.clone());
    }

    /// Dart's `debugFillProperties` `listeners` list: the callbacks that are set.
    fn listener_names(&self) -> Vec<&'static str> {
        let listeners = [
            (self.on_pointer_down.is_some(), "down"),
            (self.on_pointer_move.is_some(), "move"),
            (self.on_pointer_up.is_some(), "up"),
            (self.on_pointer_hover.is_some(), "hover"),
            (self.on_pointer_cancel.is_some(), "cancel"),
            (self.on_pointer_pan_zoom_start.is_some(), "panZoomStart"),
            (self.on_pointer_pan_zoom_update.is_some(), "panZoomUpdate"),
            (self.on_pointer_pan_zoom_end.is_some(), "panZoomEnd"),
            (self.on_pointer_signal.is_some(), "signal"),
        ];
        listeners
            .into_iter()
            .filter_map(|(is_set, name)| is_set.then_some(name))
            .collect()
    }
}

impl Default for Listener {
    fn default() -> Listener {
        Listener {
            key: None,
            on_pointer_down: None,
            on_pointer_move: None,
            on_pointer_up: None,
            on_pointer_hover: None,
            on_pointer_cancel: None,
            on_pointer_pan_zoom_start: None,
            on_pointer_pan_zoom_update: None,
            on_pointer_pan_zoom_end: None,
            on_pointer_signal: None,
            behavior: HitTestBehavior::DeferToChild,
            child: None,
        }
    }
}

impl RenderObjectWidget for Listener {
    type RenderObject = RenderPointerListener;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        let render_object = RenderPointerListener::new(app, self.behavior, None);
        self.apply_listeners(app, render_object);
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderPointerListener>,
    ) {
        self.apply_listeners(app, render_object);
        render_object.set_behavior(app, self.behavior);
    }
}

impl SingleChildRenderObjectWidget for Listener {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

impl Debug for Listener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Listener")
            .field("key", &self.key)
            .field("listeners", &self.listener_names())
            .field("behavior", &self.behavior)
            .field("child", &self.child)
            .finish()
    }
}

/// A widget that tracks the movement of mice.
///
/// [`MouseRegion`] is used
/// when it is needed to compare the list of objects that a mouse pointer is
/// hovering over between this frame and the last frame. This means entering
/// events, exiting events, and mouse cursors.
///
/// To listen to general pointer events, use [`Listener`], or more preferably,
/// `GestureDetector`.
///
/// ## Layout behavior
///
/// _See [`BoxConstraints`] for an introduction to box layout models._
///
/// If it has a child, this widget defers to the child for sizing behavior. If
/// it does not have a child, it grows to fit the parent instead.
///
/// See also:
///
///  * [`Listener`], a similar widget that tracks pointer events when the pointer
///    has buttons pressed.
///
/// By default, all callbacks are empty, [`cursor`](Self::cursor) is `MouseCursor.defer`, and
/// [`opaque`](Self::opaque) is true.
pub struct MouseRegion {
    pub key: Option<KeyRef>,
    /// Triggered when a mouse pointer has entered this widget.
    ///
    /// This callback is triggered when the pointer, with or without buttons
    /// pressed, has started to be contained by the region of this widget. More
    /// specifically, the callback is triggered by the following cases:
    ///
    ///  * This widget has appeared under a pointer.
    ///  * This widget has moved to under a pointer.
    ///  * A new pointer has been added to somewhere within this widget.
    ///  * An existing pointer has moved into this widget.
    ///
    /// This callback is not always matched by an [`on_exit`](Self::on_exit). If the
    /// [`MouseRegion`] is unmounted while being hovered by a pointer, the
    /// [`on_exit`](Self::on_exit) of the widget callback will never called. For more details,
    /// see [`on_exit`](Self::on_exit).
    ///
    /// The time that this callback is triggered is always between frames: either
    /// during the post-frame callbacks, or during the callback of a pointer
    /// event.
    ///
    /// See also:
    ///
    ///  * [`on_exit`](Self::on_exit), which is triggered when a mouse pointer exits the region.
    ///  * `MouseTrackerAnnotation.onEnter`, which is how this callback is
    ///    internally implemented.
    pub on_enter: Option<PointerEnterEventListener>,
    /// Triggered when a pointer moves into a position within this widget without
    /// buttons pressed.
    ///
    /// Usually this is only fired for pointers which report their location when
    /// not down (e.g. mouse pointers). Certain devices also fire this event on
    /// single taps in accessibility mode.
    ///
    /// This callback is not triggered by the movement of the widget.
    ///
    /// The time that this callback is triggered is during the callback of a
    /// pointer event, which is always between frames.
    ///
    /// See also:
    ///
    ///  * [`Listener::on_pointer_hover`], which does the same job. Prefer using
    ///    [`Listener::on_pointer_hover`], since hover events are similar to other regular
    ///    events.
    pub on_hover: Option<PointerHoverEventListener>,
    /// Triggered when a mouse pointer has exited this widget when the widget is
    /// still mounted.
    ///
    /// This callback is triggered when the pointer, with or without buttons
    /// pressed, has stopped being contained by the region of this widget, except
    /// when the exit is caused by the disappearance of this widget. More
    /// specifically, this callback is triggered by the following cases:
    ///
    ///  * A pointer that is hovering this widget has moved away.
    ///  * A pointer that is hovering this widget has been removed.
    ///  * This widget, which is being hovered by a pointer, has moved away.
    ///
    /// And is __not__ triggered by the following case:
    ///
    ///  * This widget, which is being hovered by a pointer, has disappeared.
    ///
    /// This means that a [`MouseRegion::on_exit`] might not be matched by a
    /// [`MouseRegion::on_enter`].
    ///
    /// This restriction aims to prevent a common misuse: if `State::set_state` is
    /// called during [`MouseRegion::on_exit`] without checking whether the widget is
    /// still mounted, an exception will occur. This is because the callback is
    /// triggered during the post-frame phase, at which point the widget has been
    /// unmounted. Since `State::set_state` is exclusive to widgets, the restriction
    /// is specific to [`MouseRegion`], and does not apply to its lower-level
    /// counterparts, [`RenderMouseRegion`] and `MouseTrackerAnnotation`.
    ///
    /// There are a few ways to mitigate this restriction:
    ///
    ///  * If the hover state is completely contained within a widget that
    ///    unconditionally creates this [`MouseRegion`], then this will not be a
    ///    concern, since after the [`MouseRegion`] is unmounted the state is no
    ///    longer used.
    ///  * Otherwise, the outer widget very likely has access to the variable that
    ///    controls whether this [`MouseRegion`] is present. If so, call
    ///    [`on_exit`](Self::on_exit) at the event that turns the condition from true to false.
    ///  * In cases where the solutions above won't work, you can always
    ///    override `State::dispose` and call [`on_exit`](Self::on_exit), or create your own
    ///    widget using [`RenderMouseRegion`].
    ///
    /// See also:
    ///
    ///  * [`on_enter`](Self::on_enter), which is triggered when a mouse pointer enters the
    ///    region.
    ///  * [`RenderMouseRegion`] and `MouseTrackerAnnotation.onExit`, which are how
    ///    this callback is internally implemented, but without the restriction.
    pub on_exit: Option<PointerExitEventListener>,
    /// The mouse cursor for mouse pointers that are hovering over the region.
    ///
    /// When a mouse enters the region, its cursor will be changed to the
    /// [`cursor`](Self::cursor). When the mouse leaves the region, the cursor will be decided
    /// by the region found at the new location.
    ///
    /// The [`cursor`](Self::cursor) defaults to `MouseCursor.defer`, deferring the choice of
    /// cursor to the next region behind it in hit-test order.
    pub cursor: MouseCursorRef,
    /// Whether this widget should prevent other [`MouseRegion`]s visually behind it
    /// from detecting the pointer.
    ///
    /// This changes the list of regions that a pointer hovers, thus affecting how
    /// their [`on_hover`](Self::on_hover), [`on_enter`](Self::on_enter),
    /// [`on_exit`](Self::on_exit), and [`cursor`](Self::cursor) behave.
    ///
    /// If [`opaque`](Self::opaque) is true, this widget will absorb the mouse pointer and
    /// prevent this widget's siblings (or any other widgets that are not
    /// ancestors or descendants of this widget) from detecting the mouse
    /// pointer even when the pointer is within their areas.
    ///
    /// If [`opaque`](Self::opaque) is false, this object will not affect how [`MouseRegion`]s
    /// behind it behave, which will detect the mouse pointer as long as the
    /// pointer is within their areas.
    ///
    /// This defaults to true.
    pub opaque: bool,
    /// How to behave during hit testing.
    ///
    /// This defaults to [`HitTestBehavior::Opaque`] if `None`.
    pub hit_test_behavior: Option<HitTestBehavior>,
    pub child: Option<WidgetRef>,
}

impl MouseRegion {
    /// Creates a `MouseRegion`; Dart's named arguments are the setters.
    pub fn new() -> MouseRegion {
        MouseRegion::default()
    }

    /// Dart `MouseRegion(key:)`.
    pub fn key(mut self, key: KeyRef) -> MouseRegion {
        self.key = Some(key);
        self
    }

    /// Dart `MouseRegion(on_enter:)`.
    pub fn on_enter(mut self, on_enter: PointerEnterEventListener) -> MouseRegion {
        self.on_enter = Some(on_enter);
        self
    }

    /// Dart `MouseRegion(on_hover:)`.
    pub fn on_hover(mut self, on_hover: PointerHoverEventListener) -> MouseRegion {
        self.on_hover = Some(on_hover);
        self
    }

    /// Dart `MouseRegion(on_exit:)`.
    pub fn on_exit(mut self, on_exit: PointerExitEventListener) -> MouseRegion {
        self.on_exit = Some(on_exit);
        self
    }

    /// Dart `MouseRegion(cursor:)`.
    pub fn cursor(mut self, cursor: MouseCursorRef) -> MouseRegion {
        self.cursor = cursor;
        self
    }

    /// Dart `MouseRegion(opaque:)`.
    pub fn opaque(mut self, opaque: bool) -> MouseRegion {
        self.opaque = opaque;
        self
    }

    /// Dart `MouseRegion(hit_test_behavior:)`.
    pub fn hit_test_behavior(mut self, hit_test_behavior: HitTestBehavior) -> MouseRegion {
        self.hit_test_behavior = Some(hit_test_behavior);
        self
    }

    /// Dart `MouseRegion(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> MouseRegion {
        self.child = Some(child.into_widget());
        self
    }

    /// Copies the configuration onto the render object; Dart's constructor and the setters
    /// share this list.
    fn apply_configuration(&self, app: &mut App, render_object: RenderHandle<RenderMouseRegion>) {
        render_object.set_on_enter(app, self.on_enter.clone());
        render_object.set_on_hover(app, self.on_hover.clone());
        render_object.set_on_exit(app, self.on_exit.clone());
        render_object.set_cursor(app, Rc::clone(&self.cursor));
        render_object.set_opaque(app, self.opaque);
        render_object.set_hit_test_behavior(app, self.hit_test_behavior);
    }

    /// Dart's `debugFillProperties` `listeners` list: the callbacks that are set.
    fn listener_names(&self) -> Vec<&'static str> {
        let listeners = [
            (self.on_enter.is_some(), "enter"),
            (self.on_exit.is_some(), "exit"),
            (self.on_hover.is_some(), "hover"),
        ];
        listeners
            .into_iter()
            .filter_map(|(is_set, name)| is_set.then_some(name))
            .collect()
    }
}

impl Default for MouseRegion {
    fn default() -> MouseRegion {
        MouseRegion {
            key: None,
            on_enter: None,
            on_hover: None,
            on_exit: None,
            cursor: <dyn MouseCursor>::defer(),
            opaque: true,
            hit_test_behavior: None,
            child: None,
        }
    }
}

impl RenderObjectWidget for MouseRegion {
    type RenderObject = RenderMouseRegion;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        let render_object = RenderMouseRegion::new(app, true, None);
        self.apply_configuration(app, render_object);
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderMouseRegion>,
    ) {
        self.apply_configuration(app, render_object);
    }
}

impl SingleChildRenderObjectWidget for MouseRegion {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

impl Debug for MouseRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MouseRegion")
            .field("key", &self.key)
            .field("listeners", &self.listener_names())
            .field("cursor", &self.cursor)
            .field("opaque", &self.opaque)
            .field("child", &self.child)
            .finish()
    }
}

/// A widget that creates a separate display list for its child.
///
/// This widget creates a separate display list for its child, which
/// can improve performance if the subtree repaints at different times than
/// the surrounding parts of the tree.
///
/// This is useful since `RenderObject::paint` may be triggered even if its
/// associated `Widget` instances did not change or rebuild. A `RenderObject`
/// will repaint whenever any `RenderObject` that shares the same `Layer` is
/// marked as being dirty and needing paint (see `RenderObject::mark_needs_paint`),
/// such as when an ancestor scrolls or when an ancestor or descendant animates.
///
/// Containing `RenderObject::paint` to parts of the render subtree that are
/// actually visually changing using [`RepaintBoundary`] explicitly or implicitly
/// is therefore critical to minimizing redundant work and improving the app's
/// performance.
///
/// When a `RenderObject` is flagged as needing to paint via
/// `RenderObject::mark_needs_paint`, the nearest ancestor `RenderObject` with
/// `RenderObject::is_repaint_boundary`, up to possibly the root of the application,
/// is requested to repaint. That nearest ancestor's `RenderObject::paint` method
/// will cause _all_ of its descendant `RenderObject`s to repaint in the same
/// layer.
///
/// [`RepaintBoundary`] is therefore used, both while propagating the
/// `mark_needs_paint` flag up the render tree and while traversing down the
/// render tree via `PaintingContext::paint_child`, to strategically contain
/// repaints to the render subtree that visually changed for performance. This
/// is done because the [`RepaintBoundary`] widget creates a `RenderObject` that
/// always has a `Layer`, decoupling ancestor render objects from the descendant
/// render objects.
///
/// [`RepaintBoundary`] has the further side-effect of possibly hinting to the
/// engine that it should further optimize animation performance if the render
/// subtree behind the [`RepaintBoundary`] is sufficiently complex and is static
/// while the surrounding tree changes frequently. In those cases, the engine
/// may choose to pay a one time cost of rasterizing and caching the pixel
/// values of the subtree for faster future GPU re-rendering speed.
///
/// Several framework widgets insert [`RepaintBoundary`] widgets to mark natural
/// separation points in applications. For instance, contents in Material Design
/// drawers typically don't change while the drawer opens and closes, so
/// repaints are automatically contained to regions inside or outside the drawer
/// when using the `Drawer` widget during transitions.
///
/// See also:
///
///  * `debugRepaintRainbowEnabled`, a debugging flag to help visually monitor
///    render tree repaints in a running app.
///  * `debugProfilePaintsEnabled`, a debugging flag to show render tree
///    repaints in Flutter DevTools' timeline view.
///
/// Dart's `RepaintBoundary.wrap` and `wrapAll` wait on a key over `Object`
/// (`child.key ?? childIndex`); their callers are the sliver child delegates.
#[derive(Debug, Default)]
pub struct RepaintBoundary {
    pub key: Option<KeyRef>,
    pub child: Option<WidgetRef>,
}

impl RepaintBoundary {
    /// Creates a `RepaintBoundary`; Dart's named arguments are the setters.
    pub fn new() -> RepaintBoundary {
        RepaintBoundary::default()
    }

    /// Dart `RepaintBoundary(key:)`.
    pub fn key(mut self, key: KeyRef) -> RepaintBoundary {
        self.key = Some(key);
        self
    }

    /// Dart `RepaintBoundary(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> RepaintBoundary {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for RepaintBoundary {
    type RenderObject = RenderRepaintBoundary;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderRepaintBoundary::new(app, None).as_object()
    }
}

impl SingleChildRenderObjectWidget for RepaintBoundary {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

// UTILITY NODES

/// Signature for a function that creates a widget, e.g. [`StatelessWidget::build`]
/// or `State::build`.
///
/// Used by [`Builder::builder`], `OverlayEntry.builder`, etc.
///
/// See also:
///
///  * `IndexedWidgetBuilder`, which is similar but also takes an index.
///  * `TransitionBuilder`, which is similar but also takes a child.
///  * `ValueWidgetBuilder`, which is similar but takes a value and a child.
///
/// Flutter's `Widget Function(BuildContext context)` from `framework.dart`; receives
/// [`App`] because a Rust callback cannot capture what it mutates.
pub type WidgetBuilder = Rc<dyn Fn(&mut App, BuildContext) -> WidgetRef>;

/// A stateless utility widget whose [`build`](StatelessWidget::build) method uses its
/// [`builder`](Self::builder) callback to create the widget's child.
///
/// The [`Builder`] widget is essentially a fancy way of calling a closure that returns a
/// widget, but with two key differences:
///
/// 1. It creates a new [`BuildContext`]. This means that any [`BuildContext`]-dependent
///    calls inside it, such as `Theme.of` or `Directionality.of`, will use the
///    context of the [`Builder`] rather than the enclosing widget's.
/// 2. It lets you avoid creating a separate widget class for what may be a very
///    simple nested build.
///
/// ## Why is the [`Builder`] widget necessary?
///
/// The [`Builder`] widget is useful when you need to access a [`BuildContext`]
/// that is a descendant of a particular widget, for example an inherited widget
/// introduced by the same `build` method.
///
/// See also:
///
///  * `StatefulBuilder`, A stateful utility widget whose build method uses its
///    builder callback to create the widget's child.
pub struct Builder {
    pub key: Option<KeyRef>,
    /// Called to obtain the child widget.
    ///
    /// This function is called whenever this widget is included in its parent's
    /// build and the old widget (if any) that it synchronizes with has a distinct
    /// object identity. Typically the parent's build method will construct
    /// a new tree of widgets and so a new Builder child will not be identical
    /// to the corresponding old one.
    pub builder: WidgetBuilder,
}

impl Builder {
    /// Creates a `Builder`; Dart's optional named arguments are the setters.
    pub fn new(builder: impl Fn(&mut App, BuildContext) -> WidgetRef + 'static) -> Builder {
        Builder {
            key: None,
            builder: Rc::new(builder),
        }
    }

    /// Dart `Builder(key:)`.
    pub fn key(mut self, key: KeyRef) -> Builder {
        self.key = Some(key);
        self
    }
}

impl StatelessWidget for Builder {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        (self.builder)(app, context)
    }
}

impl Debug for Builder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Builder")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use reveal_embedder::{Color, Offset, Size};
    use reveal_foundation::App;
    use reveal_gestures::{PointerDownEvent, PointerEvent};
    use reveal_painting::BoxDecoration;
    use reveal_rendering::{
        BoxHitTestEntry, BoxParentData, RenderBox, RenderObject, RenderObjectWithChildMixin,
    };
    use reveal_services::SystemMouseCursors;

    use super::*;
    use crate::framework::{IntoWidget, downcast_widget};
    use crate::test_harness::{Harness, padding_under_root};

    /// The render object directly under the test root, typed.
    fn render_object_under_root<T: RenderObject>(harness: &Harness, app: &App) -> RenderHandle<T> {
        harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .as_object()
            .downcast::<T>(app)
            .expect("the widget's render object")
    }

    /// The offset a shifted box gave its child.
    fn child_offset<T: RenderObject + RenderObjectWithChildMixin>(
        parent: RenderHandle<T>,
        app: &App,
    ) -> Offset {
        parent
            .child(app)
            .expect("a child")
            .as_object()
            .parent_data_of::<BoxParentData>(app)
            .offset
    }

    fn sized(width: f64, height: f64) -> WidgetRef {
        SizedBox {
            width: Some(width),
            height: Some(height),
            ..Default::default()
        }
        .into_widget()
    }

    /// Reads `Directionality::of`, counting builds and recording what it saw.
    #[derive(Debug)]
    struct DirectionReader {
        builds: Rc<Cell<u32>>,
        seen: Rc<Cell<Option<TextDirection>>>,
    }

    impl StatelessWidget for DirectionReader {
        fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
            self.builds.set(self.builds.get() + 1);
            self.seen.set(Some(Directionality::of(app, context)));
            sized(10.0, 10.0)
        }
    }

    #[test]
    fn padding_insets_its_child_and_updates_in_place() {
        let mut app = App::new();
        let padded = |inset: f64| {
            Padding {
                key: None,
                padding: EdgeInsetsGeometry::all(inset),
                child: Some(sized(10.0, 10.0)),
            }
            .into_widget()
        };
        let harness = Harness::mount(&mut app, padded(4.0));
        harness.pump(&mut app);
        let before = padding_under_root(&harness, &app);
        assert_eq!(before.size(&app), Size::new(18.0, 18.0));
        assert_eq!(
            before.text_direction(&app),
            None,
            "no Directionality ancestor"
        );

        harness.set_child(&mut app, padded(6.0));
        harness.pump(&mut app);
        let after = padding_under_root(&harness, &app);
        assert_eq!(after, before, "the same RenderPadding is reconfigured");
        assert_eq!(after.padding(&app), EdgeInsetsGeometry::all(6.0));
        assert_eq!(after.size(&app), Size::new(22.0, 22.0));
    }

    #[test]
    fn padding_resolves_directional_insets_through_directionality() {
        let mut app = App::new();
        let directional = |text_direction: TextDirection| {
            Directionality {
                key: None,
                text_direction,
                child: Padding {
                    key: None,
                    padding: EdgeInsetsGeometry::directional(8.0, 0.0, 0.0, 0.0),
                    child: Some(sized(10.0, 10.0)),
                }
                .into_widget(),
            }
            .into_widget()
        };
        let harness = Harness::mount(&mut app, directional(TextDirection::Rtl));
        harness.pump(&mut app);
        let padding = padding_under_root(&harness, &app);
        assert_eq!(padding.text_direction(&app), Some(TextDirection::Rtl));
        assert_eq!(
            child_offset(padding, &app),
            Offset::new(0.0, 0.0),
            "start is the right"
        );

        harness.set_child(&mut app, directional(TextDirection::Ltr));
        harness.pump(&mut app);
        assert_eq!(padding.text_direction(&app), Some(TextDirection::Ltr));
        assert_eq!(
            child_offset(padding, &app),
            Offset::new(8.0, 0.0),
            "start is the left"
        );
    }

    #[test]
    fn center_positions_its_child_in_the_middle() {
        let mut app = App::new();
        let harness = Harness::mount(
            &mut app,
            Center {
                child: Some(sized(10.0, 10.0)),
                ..Default::default()
            }
            .into_widget(),
        );
        harness.pump(&mut app);
        let positioned = render_object_under_root::<RenderPositionedBox>(&harness, &app);
        assert_eq!(positioned.alignment(&app), AlignmentGeometry::CENTER);
        assert_eq!(
            positioned.size(&app),
            Size::new(300.0, 200.0),
            "fills the root"
        );
        assert_eq!(child_offset(positioned, &app), Offset::new(145.0, 95.0));
    }

    #[test]
    fn align_positions_its_child_and_updates_in_place() {
        let mut app = App::new();
        let aligned = |alignment: AlignmentGeometry, width_factor: Option<f64>| {
            Align {
                alignment,
                width_factor,
                child: Some(sized(10.0, 10.0)),
                ..Default::default()
            }
            .into_widget()
        };
        let harness = Harness::mount(&mut app, aligned(AlignmentGeometry::BOTTOM_RIGHT, None));
        harness.pump(&mut app);
        let positioned = render_object_under_root::<RenderPositionedBox>(&harness, &app);
        assert_eq!(child_offset(positioned, &app), Offset::new(290.0, 190.0));

        harness.set_child(&mut app, aligned(AlignmentGeometry::TOP_LEFT, Some(2.0)));
        harness.pump(&mut app);
        let same = render_object_under_root::<RenderPositionedBox>(&harness, &app);
        assert_eq!(
            same, positioned,
            "the same RenderPositionedBox is reconfigured"
        );
        assert_eq!(
            positioned.size(&app),
            Size::new(20.0, 200.0),
            "twice the child's width"
        );
        assert_eq!(child_offset(positioned, &app), Offset::new(0.0, 0.0));
    }

    #[test]
    fn sized_box_constrains_its_child() {
        let mut app = App::new();
        let harness = Harness::mount(&mut app, sized(50.0, 20.0));
        harness.pump(&mut app);
        let constrained = render_object_under_root::<RenderConstrainedBox>(&harness, &app);
        assert_eq!(
            constrained.additional_constraints(&app),
            BoxConstraints::tight_for(Some(50.0), Some(20.0))
        );
        assert_eq!(constrained.size(&app), Size::new(50.0, 20.0));

        harness.set_child(&mut app, SizedBox::expand().into_widget());
        harness.pump(&mut app);
        assert_eq!(
            constrained.size(&app),
            Size::new(300.0, 200.0),
            "as large as the root allows"
        );

        harness.set_child(&mut app, SizedBox::shrink().into_widget());
        harness.pump(&mut app);
        assert_eq!(constrained.size(&app), Size::new(0.0, 0.0));
    }

    #[test]
    fn sized_box_named_constructors_match_dart() {
        let square = SizedBox::square(Some(7.0));
        assert_eq!((square.width, square.height), (Some(7.0), Some(7.0)));
        let from_size = SizedBox::from_size(Some(Size::new(3.0, 4.0)));
        assert_eq!((from_size.width, from_size.height), (Some(3.0), Some(4.0)));
        let unspecified = SizedBox::from_size(None);
        assert_eq!((unspecified.width, unspecified.height), (None, None));
        assert!(format!("{:?}", SizedBox::expand()).starts_with("SizedBox.expand"));
        assert!(format!("{:?}", SizedBox::shrink()).starts_with("SizedBox.shrink"));
        assert!(format!("{square:?}").starts_with("SizedBox {"));
    }

    #[test]
    fn constrained_box_imposes_its_constraints() {
        let mut app = App::new();
        let harness = Harness::mount(
            &mut app,
            ConstrainedBox {
                key: None,
                constraints: BoxConstraints::new().min_width(120.0).min_height(30.0),
                child: Some(sized(10.0, 10.0)),
            }
            .into_widget(),
        );
        harness.pump(&mut app);
        let constrained = render_object_under_root::<RenderConstrainedBox>(&harness, &app);
        assert_eq!(constrained.size(&app), Size::new(120.0, 30.0));
    }

    #[test]
    fn decorated_box_paints_its_decoration_and_updates_in_place() {
        let mut app = App::new();
        let red = Color::from_argb(255, 255, 0, 0);
        let blue = Color::from_argb(255, 0, 0, 255);
        let decorated = |color: Color| {
            DecoratedBox {
                key: None,
                decoration: Box::new(BoxDecoration::new().color(color)),
                position: DecorationPosition::Background,
                child: Some(sized(10.0, 10.0)),
            }
            .into_widget()
        };
        let harness = Harness::mount(&mut app, decorated(red));
        harness.pump(&mut app);
        let render_object = render_object_under_root::<RenderDecoratedBox>(&harness, &app);
        assert!(!render_object.as_object().debug_needs_paint(&app));
        let mut canvas = reveal_embedder::Canvas::new();
        harness
            .render_root(&app)
            .as_object()
            .debug_layer(&app)
            .expect("the root painted")
            .add_to_scene(&app, &mut canvas);
        let ops = canvas.build().ops().to_vec();
        assert!(
            ops.iter()
                .any(|op| matches!(op, reveal_embedder::valo::Op::DrawDisplayList { .. })),
            "{ops:?}"
        );

        harness.set_child(&mut app, decorated(blue));
        harness.pump(&mut app);
        let same = render_object_under_root::<RenderDecoratedBox>(&harness, &app);
        assert_eq!(
            same, render_object,
            "the same RenderDecoratedBox is reconfigured"
        );
        let decoration = render_object
            .decoration(&app)
            .as_any()
            .downcast_ref::<BoxDecoration>()
            .expect("a BoxDecoration");
        assert_eq!(decoration.color, Some(blue.into()));
    }

    #[test]
    fn listener_receives_a_pointer_down_through_its_render_object() {
        let mut app = App::new();
        let downs = Rc::new(Cell::new(0));
        let seen_at = Rc::new(Cell::new(Offset::ZERO));
        let harness = Harness::mount(
            &mut app,
            Listener {
                on_pointer_down: Some(Rc::new({
                    let downs = Rc::clone(&downs);
                    let seen_at = Rc::clone(&seen_at);
                    move |_app: &mut App, event: PointerDownEvent| {
                        downs.set(downs.get() + 1);
                        seen_at.set(event.position);
                    }
                })),
                behavior: HitTestBehavior::Opaque,
                child: Some(sized(10.0, 10.0)),
                ..Default::default()
            }
            .into_widget(),
        );
        harness.pump(&mut app);
        let listener = render_object_under_root::<RenderPointerListener>(&harness, &app);
        assert!(listener.on_pointer_down(&app).is_some());
        assert!(listener.on_pointer_up(&app).is_none());

        let event = PointerEvent::Down(PointerDownEvent {
            position: Offset::new(3.0, 4.0),
            ..PointerDownEvent::default()
        });
        let entry = BoxHitTestEntry::new(listener.as_box(), Offset::new(3.0, 4.0));
        listener.as_box().handle_event(&mut app, &event, &entry);
        assert_eq!(downs.get(), 1);
        assert_eq!(seen_at.get(), Offset::new(3.0, 4.0));

        harness.set_child(
            &mut app,
            Listener {
                child: Some(sized(10.0, 10.0)),
                ..Default::default()
            }
            .into_widget(),
        );
        harness.pump(&mut app);
        assert!(
            listener.on_pointer_down(&app).is_none(),
            "the callback is cleared in place"
        );
        listener.as_box().handle_event(&mut app, &event, &entry);
        assert_eq!(downs.get(), 1);
    }

    #[test]
    fn mouse_region_forwards_its_configuration() {
        let mut app = App::new();
        let harness = Harness::mount(
            &mut app,
            MouseRegion {
                on_enter: Some(Rc::new(|_app, _event| {})),
                cursor: Rc::new(SystemMouseCursors::CLICK),
                opaque: false,
                hit_test_behavior: Some(HitTestBehavior::Translucent),
                child: Some(sized(10.0, 10.0)),
                ..Default::default()
            }
            .into_widget(),
        );
        harness.pump(&mut app);
        let region = render_object_under_root::<RenderMouseRegion>(&harness, &app);
        assert!(region.on_enter(&app).is_some());
        assert!(region.on_hover(&app).is_none());
        assert!(!region.cursor(&app).is_defer());
        assert!(!region.opaque(&app));
        assert_eq!(
            region.hit_test_behavior(&app),
            Some(HitTestBehavior::Translucent)
        );

        harness.set_child(
            &mut app,
            MouseRegion {
                child: Some(sized(10.0, 10.0)),
                ..Default::default()
            }
            .into_widget(),
        );
        harness.pump(&mut app);
        assert!(region.on_enter(&app).is_none());
        assert!(region.cursor(&app).is_defer());
        assert!(region.opaque(&app));
        assert_eq!(
            region.hit_test_behavior(&app),
            Some(HitTestBehavior::Opaque)
        );
    }

    #[test]
    fn opacity_forwards_its_value() {
        let mut app = App::new();
        let faded = |opacity: f64| {
            Opacity {
                key: None,
                opacity,
                child: Some(sized(10.0, 10.0)),
            }
            .into_widget()
        };
        let harness = Harness::mount(&mut app, faded(0.5));
        harness.pump(&mut app);
        let opacity = render_object_under_root::<RenderOpacity>(&harness, &app);
        assert_eq!(opacity.opacity(&app), 0.5);

        harness.set_child(&mut app, faded(0.25));
        harness.pump(&mut app);
        assert_eq!(opacity.opacity(&app), 0.25);
    }

    #[test]
    fn repaint_boundary_isolates_repaints() {
        let mut app = App::new();
        let harness = Harness::mount(
            &mut app,
            RepaintBoundary {
                key: None,
                child: Some(sized(10.0, 10.0)),
            }
            .into_widget(),
        );
        harness.pump(&mut app);
        let boundary = render_object_under_root::<RenderRepaintBoundary>(&harness, &app);
        assert!(boundary.as_object().is_repaint_boundary(&app));
        assert_eq!(boundary.size(&app), Size::new(10.0, 10.0));
    }

    #[test]
    fn directionality_of_resolves_through_the_tree_and_notifies_on_change() {
        let mut app = App::new();
        let builds = Rc::new(Cell::new(0));
        let seen = Rc::new(Cell::new(None));
        // One reader instance, reused like a `const` widget, so only the Directionality
        // decides rebuilds.
        let reader: WidgetRef = DirectionReader {
            builds: Rc::clone(&builds),
            seen: Rc::clone(&seen),
        }
        .into_widget();
        let directional = |text_direction: TextDirection| {
            Directionality {
                key: None,
                text_direction,
                child: reader.clone(),
            }
            .into_widget()
        };
        let harness = Harness::mount(&mut app, directional(TextDirection::Ltr));
        harness.pump(&mut app);
        assert_eq!(builds.get(), 1);
        assert_eq!(seen.get(), Some(TextDirection::Ltr));

        harness.set_child(&mut app, directional(TextDirection::Ltr));
        harness.pump(&mut app);
        assert_eq!(builds.get(), 1, "an equal direction does not notify");

        harness.set_child(&mut app, directional(TextDirection::Rtl));
        harness.pump(&mut app);
        assert_eq!(
            builds.get(),
            2,
            "a changed direction rebuilds the dependent"
        );
        assert_eq!(seen.get(), Some(TextDirection::Rtl));
    }

    #[test]
    fn builder_builds_with_its_context() {
        let mut app = App::new();
        let harness = Harness::mount(
            &mut app,
            Builder {
                key: None,
                builder: Rc::new(|_app, context| {
                    let widget = context.widget(_app).clone();
                    assert!(
                        downcast_widget::<Builder>(&*widget).is_some(),
                        "the context is the Builder's own element"
                    );
                    Padding {
                        key: None,
                        padding: EdgeInsetsGeometry::all(2.0),
                        child: Some(sized(10.0, 10.0)),
                    }
                    .into_widget()
                }),
            }
            .into_widget(),
        );
        harness.pump(&mut app);
        let padding = padding_under_root(&harness, &app);
        assert_eq!(padding.padding(&app), EdgeInsetsGeometry::all(2.0));
        assert_eq!(padding.size(&app), Size::new(14.0, 14.0));
    }
}
