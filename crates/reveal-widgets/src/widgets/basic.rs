//! Flutter counterpart: `widgets/basic.dart`.
//!
//! `Spacer` is Flutter's `widgets/spacer.dart` and `IndexedStack` is
//! `widgets/indexed_stack.dart`. They live here until those files are ported.
//! `WidgetBuilder` is Flutter's `framework.dart` typedef; the framework module does not export
//! one, so it is defined here next to `Builder`.

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::sync::Arc;

use reveal_embedder::{
    BlendMode, Clip, ImageFilter, Matrix4, Offset, Path, RRect, RSuperellipse, Rect, Size,
    TextBaseline, TextDirection,
};
use reveal_foundation::{App, Handle};
use reveal_gestures::{
    PointerCancelEventListener, PointerDownEventListener, PointerEnterEventListener,
    PointerExitEventListener, PointerHoverEventListener, PointerMoveEventListener,
    PointerPanZoomEndEventListener, PointerPanZoomStartEventListener,
    PointerPanZoomUpdateEventListener, PointerSignalEventListener, PointerUpEventListener,
};
use reveal_painting::{
    Alignment, AlignmentGeometry, AnyColor, Axis, AxisDirection, BorderRadiusGeometry, BoxFit,
    EdgeInsetsGeometry, ShapeBorder, VerticalDirection, flip_axis_direction,
    text_direction_to_axis_direction,
};
use reveal_rendering::{
    AnyRenderObject, BackdropKey, BoxConstraints, BoxConstraintsTransform, ChildLayoutId,
    CrossAxisAlignment, CustomClipper, CustomPainter, FlexFit, FlexParentData, HitTestBehavior,
    ImageFilterConfig, LayerLink, MainAxisAlignment, MainAxisSize, MultiChildLayoutDelegate,
    MultiChildLayoutParentData, OverflowBoxFit, RelativeRect, RenderAbsorbPointer,
    RenderAligningShiftedBox, RenderAspectRatio, RenderBackdropFilter, RenderBaseline, RenderBox,
    RenderClipOval, RenderClipPath, RenderClipRRect, RenderClipRSuperellipse, RenderClipRect,
    RenderColoredBox, RenderConstrainedBox, RenderConstrainedOverflowBox,
    RenderConstraintsTransformBox, RenderCustomClip, RenderCustomMultiChildLayoutBox,
    RenderCustomPaint, RenderCustomSingleChildLayoutBox, RenderFittedBox, RenderFlex,
    RenderFollowerLayer, RenderFractionalTranslation, RenderFractionallySizedOverflowBox,
    RenderHandle, RenderIgnorePointer, RenderIndexedStack, RenderIntrinsicHeight,
    RenderIntrinsicWidth, RenderLeaderLayer, RenderLimitedBox, RenderMetaData, RenderMouseRegion,
    RenderOffstage, RenderOpacity, RenderPadding, RenderPointerListener, RenderPositionedBox,
    RenderRepaintBoundary, RenderSizedOverflowBox, RenderSliver, RenderSliverPadding,
    RenderSliverToBoxAdapter, RenderStack, RenderStackBase, RenderTransform, ShapeBorderClipper,
    SingleChildLayoutDelegate, StackFit, StackParentData,
};
use reveal_services::{MouseCursor, MouseCursorRef};

use crate::framework::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, MultiChildRenderObjectWidget,
    ParentDataWidget, RenderObjectWidget, SingleChildRenderObjectWidget, StatelessWidget,
    WidgetRef,
};
use crate::widgets::focus_scope::ExcludeFocus;
use crate::widgets::overlay::TheaterParentData;

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

/// A widget that can be targeted by a [`CompositedTransformFollower`].
///
/// When this widget is composited during the compositing phase (which comes after the paint
/// phase, as described in `WidgetsBinding.drawFrame`), it updates the [`link`](Self::link)
/// object so that any [`CompositedTransformFollower`] widgets that are subsequently composited
/// in the same frame and were given the same [`LayerLink`] can position themselves at the same
/// screen location.
///
/// A single [`CompositedTransformTarget`] can be followed by multiple
/// [`CompositedTransformFollower`] widgets.
///
/// The [`CompositedTransformTarget`] must come earlier in the paint order than any linked
/// [`CompositedTransformFollower`]s.
#[derive(Debug)]
pub struct CompositedTransformTarget {
    pub key: Option<KeyRef>,
    /// The link object that connects this [`CompositedTransformTarget`] with one or more
    /// [`CompositedTransformFollower`]s.
    pub link: Handle<LayerLink>,
    pub child: Option<WidgetRef>,
}

impl CompositedTransformTarget {
    /// Dart's `CompositedTransformTarget({required link, child})`.
    pub fn new(link: Handle<LayerLink>) -> CompositedTransformTarget {
        CompositedTransformTarget {
            key: None,
            link,
            child: None,
        }
    }

    /// Dart `CompositedTransformTarget(key:)`.
    pub fn key(mut self, key: KeyRef) -> CompositedTransformTarget {
        self.key = Some(key);
        self
    }

    /// Dart `CompositedTransformTarget(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> CompositedTransformTarget {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for CompositedTransformTarget {
    type RenderObject = RenderLeaderLayer;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderLeaderLayer::new(app, self.link, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderLeaderLayer>,
    ) {
        render_object.set_link(app, self.link);
    }
}

impl SingleChildRenderObjectWidget for CompositedTransformTarget {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A widget that follows a [`CompositedTransformTarget`].
///
/// When this widget is composited during the compositing phase (which comes after the paint
/// phase, as described in `WidgetsBinding.drawFrame`), it applies a transformation that brings
/// [`target_anchor`](Self::target_anchor) of the linked [`CompositedTransformTarget`] and
/// [`follower_anchor`](Self::follower_anchor) of this widget together. The two anchor points
/// will have the same global coordinates, unless [`offset`](Self::offset) is not zero, in
/// which case [`follower_anchor`](Self::follower_anchor) will be offset by
/// [`offset`](Self::offset) in the linked [`CompositedTransformTarget`]'s coordinate space.
///
/// The [`LayerLink`] object used as the [`link`](Self::link) must be the same object as that
/// provided to the matching [`CompositedTransformTarget`].
///
/// The [`CompositedTransformTarget`] must come earlier in the paint order than this
/// [`CompositedTransformFollower`].
///
/// Hit testing on descendants of this widget will only work if the target position is within
/// the box that this widget's parent considers to be hittable. If the parent covers the screen,
/// this is trivially achievable, so this widget is usually used as the root of an
/// `OverlayEntry` in an app-wide `Overlay` (e.g. as created by `WidgetsApp`).
#[derive(Debug)]
pub struct CompositedTransformFollower {
    pub key: Option<KeyRef>,
    /// The link object that connects this [`CompositedTransformFollower`] with a
    /// [`CompositedTransformTarget`].
    pub link: Handle<LayerLink>,
    /// Whether to show the widget's contents when there is no corresponding
    /// [`CompositedTransformTarget`] with the same [`link`](Self::link).
    ///
    /// When the widget is linked, the child is positioned such that it has the same global
    /// position as the linked [`CompositedTransformTarget`].
    ///
    /// When the widget is not linked, then: if [`show_when_unlinked`](Self::show_when_unlinked)
    /// is true, the child is visible and not repositioned; if it is false, then the child is
    /// hidden.
    pub show_when_unlinked: bool,
    /// The anchor point on the linked [`CompositedTransformTarget`] that
    /// [`follower_anchor`](Self::follower_anchor) will line up with.
    ///
    /// Defaults to [`Alignment::TOP_LEFT`].
    pub target_anchor: Alignment,
    /// The anchor point on this widget that will line up with
    /// [`target_anchor`](Self::target_anchor) on the linked [`CompositedTransformTarget`].
    ///
    /// Defaults to [`Alignment::TOP_LEFT`].
    pub follower_anchor: Alignment,
    /// The additional offset to apply to the [`target_anchor`](Self::target_anchor) of the
    /// linked [`CompositedTransformTarget`] to obtain this widget's
    /// [`follower_anchor`](Self::follower_anchor) position.
    pub offset: Offset,
    pub child: Option<WidgetRef>,
}

impl CompositedTransformFollower {
    /// Dart's `CompositedTransformFollower({required link, ..})`.
    pub fn new(link: Handle<LayerLink>) -> CompositedTransformFollower {
        CompositedTransformFollower {
            key: None,
            link,
            show_when_unlinked: true,
            target_anchor: Alignment::TOP_LEFT,
            follower_anchor: Alignment::TOP_LEFT,
            offset: Offset::ZERO,
            child: None,
        }
    }

    /// Dart `CompositedTransformFollower(key:)`.
    pub fn key(mut self, key: KeyRef) -> CompositedTransformFollower {
        self.key = Some(key);
        self
    }

    /// Dart `CompositedTransformFollower(showWhenUnlinked:)`.
    pub fn show_when_unlinked(mut self, show_when_unlinked: bool) -> CompositedTransformFollower {
        self.show_when_unlinked = show_when_unlinked;
        self
    }

    /// Dart `CompositedTransformFollower(targetAnchor:)`.
    pub fn target_anchor(mut self, target_anchor: Alignment) -> CompositedTransformFollower {
        self.target_anchor = target_anchor;
        self
    }

    /// Dart `CompositedTransformFollower(followerAnchor:)`.
    pub fn follower_anchor(mut self, follower_anchor: Alignment) -> CompositedTransformFollower {
        self.follower_anchor = follower_anchor;
        self
    }

    /// Dart `CompositedTransformFollower(offset:)`.
    pub fn offset(mut self, offset: Offset) -> CompositedTransformFollower {
        self.offset = offset;
        self
    }

    /// Dart `CompositedTransformFollower(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> CompositedTransformFollower {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for CompositedTransformFollower {
    type RenderObject = RenderFollowerLayer;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderFollowerLayer::new(
            app,
            self.link,
            self.show_when_unlinked,
            self.offset,
            self.target_anchor,
            self.follower_anchor,
            None,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderFollowerLayer>,
    ) {
        render_object.set_link(app, self.link);
        render_object.set_show_when_unlinked(app, self.show_when_unlinked);
        render_object.set_offset(app, self.offset);
        render_object.set_leader_anchor(app, self.target_anchor);
        render_object.set_follower_anchor(app, self.follower_anchor);
    }
}

impl SingleChildRenderObjectWidget for CompositedTransformFollower {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

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

/// A widget that provides a canvas on which to draw during the paint phase.
///
/// When asked to paint, [`CustomPaint`] first asks its [`painter`](Self::painter) to paint on
/// the current canvas, then it paints its child, and then, after painting its child, it asks
/// its [`foreground_painter`](Self::foreground_painter) to paint. The coordinate system of the
/// canvas matches the coordinate system of the [`CustomPaint`] object. The painters are
/// expected to paint within a rectangle starting at the origin and encompassing a region of
/// the given size. (If the painters paint outside those bounds, there might be insufficient
/// memory allocated to rasterize the painting commands and the resulting behavior is
/// undefined.) To enforce painting within those bounds, consider wrapping this
/// [`CustomPaint`] with a [`ClipRect`] widget.
///
/// Painters are implemented by implementing [`CustomPainter`].
///
/// Because custom paint calls its painters during paint, you cannot call
/// `set_state` or `mark_needs_layout` during the callback (the layout for this
/// frame has already happened).
///
/// Custom painters normally size themselves to their [`child`](Self::child). If they do not
/// have a child, they attempt to size themselves to the specified [`size`](Self::size), which
/// defaults to `Size::ZERO`. The parent may enforce constraints on this size.
///
/// The [`is_complex`](Self::is_complex) and [`will_change`](Self::will_change) properties are
/// hints to the compositor's raster cache.
///
/// See also:
///
///  * [`CustomPainter`], the trait to implement when creating custom painters.
///  * `Canvas`, the class that a custom painter uses to paint.
pub struct CustomPaint {
    pub key: Option<KeyRef>,
    /// The painter that paints before the children.
    pub painter: Option<Rc<dyn CustomPainter>>,
    /// The painter that paints after the children.
    pub foreground_painter: Option<Rc<dyn CustomPainter>>,
    /// The size that this [`CustomPaint`] should aim for, given the layout
    /// constraints, if there is no child.
    ///
    /// Defaults to `Size::ZERO`.
    ///
    /// If there's a child, this is ignored, and the size of the child is used
    /// instead.
    pub size: Size,
    /// Whether the painting is complex enough to benefit from caching.
    ///
    /// The compositor contains a raster cache that holds bitmaps of layers in
    /// order to avoid the cost of repeatedly rendering those layers on each
    /// frame. If this flag is not set, then the compositor will apply its own
    /// heuristics to decide whether the layer containing this widget is complex
    /// enough to benefit from caching.
    ///
    /// This flag can't be set to true if both [`painter`](Self::painter) and
    /// [`foreground_painter`](Self::foreground_painter) are `None` because this flag will be
    /// ignored in such case.
    pub is_complex: bool,
    /// Whether the raster cache should be told that this painting is likely
    /// to change in the next frame.
    ///
    /// This hint tells the compositor not to cache the layer containing this
    /// widget because the cache will not be used in the future. If this hint is
    /// not set, the compositor will apply its own heuristics to decide whether
    /// the layer is likely to be reused in the future.
    ///
    /// This flag can't be set to true if both [`painter`](Self::painter) and
    /// [`foreground_painter`](Self::foreground_painter) are `None` because this flag will be
    /// ignored in such case.
    pub will_change: bool,
    pub child: Option<WidgetRef>,
}

impl CustomPaint {
    /// Creates a `CustomPaint`; Dart's named arguments are the setters.
    pub fn new() -> CustomPaint {
        CustomPaint::default()
    }

    /// Dart `CustomPaint(key:)`.
    pub fn key(mut self, key: KeyRef) -> CustomPaint {
        self.key = Some(key);
        self
    }

    /// Dart `CustomPaint(painter:)`.
    pub fn painter(mut self, painter: impl CustomPainter) -> CustomPaint {
        self.painter = Some(Rc::new(painter));
        self
    }

    /// Dart `CustomPaint(foregroundPainter:)`.
    pub fn foreground_painter(mut self, foreground_painter: impl CustomPainter) -> CustomPaint {
        self.foreground_painter = Some(Rc::new(foreground_painter));
        self
    }

    /// Dart `CustomPaint(size:)`.
    pub fn size(mut self, size: Size) -> CustomPaint {
        self.size = size;
        self
    }

    /// Dart `CustomPaint(isComplex:)`.
    pub fn is_complex(mut self, is_complex: bool) -> CustomPaint {
        self.is_complex = is_complex;
        self
    }

    /// Dart `CustomPaint(willChange:)`.
    pub fn will_change(mut self, will_change: bool) -> CustomPaint {
        self.will_change = will_change;
        self
    }

    /// Dart `CustomPaint(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> CustomPaint {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart's constructor assert: a raster cache hint needs a painter to hint about.
    fn debug_check_hints(&self) {
        debug_assert!(
            self.painter.is_some()
                || self.foreground_painter.is_some()
                || (!self.is_complex && !self.will_change)
        );
    }
}

impl Default for CustomPaint {
    fn default() -> CustomPaint {
        CustomPaint {
            key: None,
            painter: None,
            foreground_painter: None,
            size: Size::ZERO,
            is_complex: false,
            will_change: false,
            child: None,
        }
    }
}

impl RenderObjectWidget for CustomPaint {
    type RenderObject = RenderCustomPaint;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        self.debug_check_hints();
        RenderCustomPaint::new(
            app,
            self.painter.clone(),
            self.foreground_painter.clone(),
            self.size,
            self.is_complex,
            self.will_change,
            None,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderCustomPaint>,
    ) {
        self.debug_check_hints();
        render_object.set_painter(app, self.painter.clone());
        render_object.set_foreground_painter(app, self.foreground_painter.clone());
        render_object.set_preferred_size(app, self.size);
        render_object.set_is_complex(app, self.is_complex);
        render_object.set_will_change(app, self.will_change);
    }

    fn did_unmount_render_object(
        &self,
        app: &mut App,
        render_object: RenderHandle<RenderCustomPaint>,
    ) {
        render_object.set_painter(app, None);
        render_object.set_foreground_painter(app, None);
    }
}

impl SingleChildRenderObjectWidget for CustomPaint {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

impl Debug for CustomPaint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomPaint")
            .field("key", &self.key)
            .field("has_painter", &self.painter.is_some())
            .field("has_foreground_painter", &self.foreground_painter.is_some())
            .field("size", &self.size)
            .field("is_complex", &self.is_complex)
            .field("will_change", &self.will_change)
            .field("child", &self.child)
            .finish()
    }
}

/// A widget that clips its child using a rectangle.
///
/// By default, [`ClipRect`] prevents its child from painting outside its
/// bounds, but the size and location of the clip rect can be customized using a
/// custom [`clipper`](Self::clipper).
///
/// [`ClipRect`] is commonly used with these widgets, which commonly paint outside
/// their bounds:
///
///  * [`CustomPaint`]
///  * `CustomSingleChildLayout`
///  * `CustomMultiChildLayout`
///  * [`Align`] and [`Center`] (e.g., if [`Align::width_factor`] or
///    [`Align::height_factor`] is less than 1.0).
///  * `OverflowBox`
///  * `SizedOverflowBox`
///
/// For example, by combining a [`ClipRect`] with an [`Align`], one can show just
/// the top half of an image:
///
/// ```text
/// ClipRect::new().child(
///     Align::new()
///         .alignment(AlignmentGeometry::TOP_CENTER)
///         .height_factor(0.5)
///         .child(avatar),
/// )
/// ```
///
/// See also:
///
///  * [`CustomClipper`], for information about creating custom clips.
///  * [`ClipRRect`], for a clip with rounded corners.
///  * `ClipOval`, for an elliptical clip.
///  * `ClipPath`, for an arbitrarily shaped clip.
pub struct ClipRect {
    pub key: Option<KeyRef>,
    /// If non-`None`, determines which clip to use.
    pub clipper: Option<Rc<dyn CustomClipper<Rect>>>,
    /// Whether and how to clip the child.
    ///
    /// Defaults to [`Clip::HardEdge`].
    pub clip_behavior: Clip,
    pub child: Option<WidgetRef>,
}

impl ClipRect {
    /// Creates a rectangular clip; Dart's named arguments are the setters.
    ///
    /// If [`clipper`](Self::clipper) is `None`, the clip will match the layout size and
    /// position of the child.
    ///
    /// If [`clip_behavior`](Self::clip_behavior) is [`Clip::None`], no clipping will be
    /// applied.
    pub fn new() -> ClipRect {
        ClipRect::default()
    }

    /// Dart `ClipRect(key:)`.
    pub fn key(mut self, key: KeyRef) -> ClipRect {
        self.key = Some(key);
        self
    }

    /// Dart `ClipRect(clipper:)`.
    pub fn clipper(mut self, clipper: impl CustomClipper<Rect>) -> ClipRect {
        self.clipper = Some(Rc::new(clipper));
        self
    }

    /// Dart `ClipRect(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> ClipRect {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `ClipRect(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> ClipRect {
        self.child = Some(child.into_widget());
        self
    }
}

impl Default for ClipRect {
    fn default() -> ClipRect {
        ClipRect {
            key: None,
            clipper: None,
            clip_behavior: Clip::HardEdge,
            child: None,
        }
    }
}

impl RenderObjectWidget for ClipRect {
    type RenderObject = RenderClipRect;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderClipRect::new(app, self.clipper.clone(), self.clip_behavior, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderClipRect>,
    ) {
        render_object.set_clipper(app, self.clipper.clone());
        render_object.set_clip_behavior(app, self.clip_behavior);
    }

    fn did_unmount_render_object(
        &self,
        app: &mut App,
        render_object: RenderHandle<RenderClipRect>,
    ) {
        render_object.set_clipper(app, None);
    }
}

impl SingleChildRenderObjectWidget for ClipRect {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

impl Debug for ClipRect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClipRect")
            .field("key", &self.key)
            .field("has_clipper", &self.clipper.is_some())
            .field("clip_behavior", &self.clip_behavior)
            .field("child", &self.child)
            .finish()
    }
}

/// A widget that clips its child using a rounded rectangle.
///
/// By default, [`ClipRRect`] uses its own bounds as the base rectangle for the
/// clip, but the size and location of the clip can be customized using a custom
/// [`clipper`](Self::clipper).
///
/// ## Troubleshooting
///
/// ### Why doesn't my [`ClipRRect`] child have rounded corners?
///
/// When a [`ClipRRect`] is bigger than the child it contains, its rounded corners
/// could be drawn in unexpected positions. Make sure that [`ClipRRect`] and its child
/// have the same bounds (by shrinking the [`ClipRRect`] with a `FittedBox` or by
/// growing the child).
///
/// See also:
///
///  * [`CustomClipper`], for information about creating custom clips.
///  * [`ClipRect`], for more efficient clips without rounded corners.
///  * `ClipRSuperellipse`, for a similar clipping shape with smoother
///    transitions between the straight sides and the rounded corners.
///  * `ClipOval`, for an elliptical clip.
///  * `ClipPath`, for an arbitrarily shaped clip.
pub struct ClipRRect {
    pub key: Option<KeyRef>,
    /// The border radius of the rounded corners.
    ///
    /// Values are clamped so that horizontal and vertical radii sums do not
    /// exceed width/height.
    ///
    /// This value is ignored if [`clipper`](Self::clipper) is non-`None`.
    pub border_radius: BorderRadiusGeometry,
    /// If non-`None`, determines which clip to use.
    pub clipper: Option<Rc<dyn CustomClipper<RRect>>>,
    /// Whether and how to clip the child.
    ///
    /// Defaults to [`Clip::AntiAlias`].
    pub clip_behavior: Clip,
    pub child: Option<WidgetRef>,
}

impl ClipRRect {
    /// Creates a rounded-rectangular clip; Dart's named arguments are the setters.
    ///
    /// The [`border_radius`](Self::border_radius) defaults to
    /// [`BorderRadiusGeometry::ZERO`], i.e. a rectangle with right-angled corners.
    ///
    /// If [`clipper`](Self::clipper) is non-`None`, then
    /// [`border_radius`](Self::border_radius) is ignored.
    ///
    /// If [`clip_behavior`](Self::clip_behavior) is [`Clip::None`], no clipping will be
    /// applied.
    pub fn new() -> ClipRRect {
        ClipRRect::default()
    }

    /// Dart `ClipRRect(key:)`.
    pub fn key(mut self, key: KeyRef) -> ClipRRect {
        self.key = Some(key);
        self
    }

    /// Dart `ClipRRect(borderRadius:)`.
    pub fn border_radius(mut self, border_radius: BorderRadiusGeometry) -> ClipRRect {
        self.border_radius = border_radius;
        self
    }

    /// Dart `ClipRRect(clipper:)`.
    pub fn clipper(mut self, clipper: impl CustomClipper<RRect>) -> ClipRRect {
        self.clipper = Some(Rc::new(clipper));
        self
    }

    /// Dart `ClipRRect(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> ClipRRect {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `ClipRRect(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> ClipRRect {
        self.child = Some(child.into_widget());
        self
    }
}

impl Default for ClipRRect {
    fn default() -> ClipRRect {
        ClipRRect {
            key: None,
            border_radius: BorderRadiusGeometry::ZERO,
            clipper: None,
            clip_behavior: Clip::AntiAlias,
            child: None,
        }
    }
}

impl RenderObjectWidget for ClipRRect {
    type RenderObject = RenderClipRRect;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::maybe_of(app, context);
        RenderClipRRect::new(
            app,
            self.border_radius,
            self.clipper.clone(),
            self.clip_behavior,
            text_direction,
            None,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderClipRRect>,
    ) {
        render_object.set_border_radius(app, self.border_radius);
        render_object.set_clip_behavior(app, self.clip_behavior);
        render_object.set_clipper(app, self.clipper.clone());
        let text_direction = Directionality::maybe_of(app, context);
        render_object.set_text_direction(app, text_direction);
    }
}

impl SingleChildRenderObjectWidget for ClipRRect {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

impl Debug for ClipRRect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClipRRect")
            .field("key", &self.key)
            .field("border_radius", &self.border_radius)
            .field("has_clipper", &self.clipper.is_some())
            .field("clip_behavior", &self.clip_behavior)
            .field("child", &self.child)
            .finish()
    }
}

/// A widget that clips its child using a rounded superellipse.
///
/// By default, [`ClipRSuperellipse`] uses its own bounds as the base rectangle for the
/// clip, but the size and location of the clip can be customized using a custom
/// [`clipper`](Self::clipper).
///
/// ## Troubleshooting
///
/// ### Why doesn't my [`ClipRSuperellipse`] child have rounded corners?
///
/// When a [`ClipRSuperellipse`] is bigger than the child it contains, its rounded corners
/// could be drawn in unexpected positions. Make sure that [`ClipRSuperellipse`] and its child
/// have the same bounds (by shrinking the [`ClipRSuperellipse`] with a `FittedBox` or by
/// growing the child).
///
/// See also:
///
///  * [`CustomClipper`], for information about creating custom clips.
///  * [`ClipRect`], for more efficient clips without rounded corners.
///  * `ClipRSuperellipse`, for a similar clipping shape with smoother
///    transitions between the straight sides and the rounded corners.
///  * `ClipOval`, for an elliptical clip.
///  * `ClipPath`, for an arbitrarily shaped clip.
pub struct ClipRSuperellipse {
    pub key: Option<KeyRef>,
    /// The border radius of the rounded corners.
    ///
    /// Values are clamped so that horizontal and vertical radii sums do not
    /// exceed width/height.
    ///
    /// This value is ignored if [`clipper`](Self::clipper) is non-`None`.
    pub border_radius: BorderRadiusGeometry,
    /// If non-`None`, determines which clip to use.
    pub clipper: Option<Rc<dyn CustomClipper<RSuperellipse>>>,
    /// Whether and how to clip the child.
    ///
    /// Defaults to [`Clip::AntiAlias`].
    pub clip_behavior: Clip,
    pub child: Option<WidgetRef>,
}

impl ClipRSuperellipse {
    /// Creates a rounded-rectangular clip; Dart's named arguments are the setters.
    ///
    /// The [`border_radius`](Self::border_radius) defaults to
    /// [`BorderRadiusGeometry::ZERO`], i.e. a rectangle with right-angled corners.
    ///
    /// If [`clipper`](Self::clipper) is non-`None`, then
    /// [`border_radius`](Self::border_radius) is ignored.
    ///
    /// If [`clip_behavior`](Self::clip_behavior) is [`Clip::None`], no clipping will be
    /// applied.
    pub fn new() -> ClipRSuperellipse {
        ClipRSuperellipse::default()
    }

    /// Dart `ClipRSuperellipse(key:)`.
    pub fn key(mut self, key: KeyRef) -> ClipRSuperellipse {
        self.key = Some(key);
        self
    }

    /// Dart `ClipRSuperellipse(borderRadius:)`.
    pub fn border_radius(mut self, border_radius: BorderRadiusGeometry) -> ClipRSuperellipse {
        self.border_radius = border_radius;
        self
    }

    /// Dart `ClipRSuperellipse(clipper:)`.
    pub fn clipper(mut self, clipper: impl CustomClipper<RSuperellipse>) -> ClipRSuperellipse {
        self.clipper = Some(Rc::new(clipper));
        self
    }

    /// Dart `ClipRSuperellipse(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> ClipRSuperellipse {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `ClipRSuperellipse(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> ClipRSuperellipse {
        self.child = Some(child.into_widget());
        self
    }
}

impl Default for ClipRSuperellipse {
    fn default() -> ClipRSuperellipse {
        ClipRSuperellipse {
            key: None,
            border_radius: BorderRadiusGeometry::ZERO,
            clipper: None,
            clip_behavior: Clip::AntiAlias,
            child: None,
        }
    }
}

impl RenderObjectWidget for ClipRSuperellipse {
    type RenderObject = RenderClipRSuperellipse;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::maybe_of(app, context);
        RenderClipRSuperellipse::new(
            app,
            self.border_radius,
            self.clipper.clone(),
            self.clip_behavior,
            text_direction,
            None,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderClipRSuperellipse>,
    ) {
        render_object.set_border_radius(app, self.border_radius);
        render_object.set_clip_behavior(app, self.clip_behavior);
        render_object.set_clipper(app, self.clipper.clone());
        let text_direction = Directionality::maybe_of(app, context);
        render_object.set_text_direction(app, text_direction);
    }
}

impl SingleChildRenderObjectWidget for ClipRSuperellipse {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

impl Debug for ClipRSuperellipse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClipRSuperellipse")
            .field("key", &self.key)
            .field("border_radius", &self.border_radius)
            .field("has_clipper", &self.clipper.is_some())
            .field("clip_behavior", &self.clip_behavior)
            .field("child", &self.child)
            .finish()
    }
}

/// A widget that clips its child using an oval.
///
/// By default, inscribes an axis-aligned oval into its layout dimensions and
/// prevents its child from painting outside that oval, but the size and
/// location of the clip oval can be customized using a custom `clipper`.
///
/// See also:
///
///  * `CustomClipper`, for information about creating custom clips.
///  * [`ClipRect`], for more efficient clips without rounded corners.
///  * [`ClipRRect`], for a clip with rounded corners.
///  * [`ClipPath`], for an arbitrarily shaped clip.
pub struct ClipOval {
    pub key: Option<KeyRef>,
    /// If non-`None`, determines which clip to use.
    ///
    /// The delegate returns a rectangle that describes the axis-aligned
    /// bounding box of the oval. The oval's axes will themselves also
    /// be axis-aligned.
    ///
    /// If the `clipper` delegate is `None`, then the oval uses the
    /// widget's bounding box (the layout dimensions of the render
    /// object) instead.
    pub clipper: Option<Rc<dyn CustomClipper<Rect>>>,
    /// Whether and how to clip the child.
    ///
    /// Defaults to [`Clip::AntiAlias`].
    pub clip_behavior: Clip,
    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
}

impl ClipOval {
    /// Creates an oval-shaped clip; Dart's named arguments are the setters.
    pub fn new() -> ClipOval {
        ClipOval::default()
    }

    /// Dart `ClipOval(key:)`.
    pub fn key(mut self, key: KeyRef) -> ClipOval {
        self.key = Some(key);
        self
    }

    /// Dart `ClipOval(clipper:)`.
    pub fn clipper(mut self, clipper: Rc<dyn CustomClipper<Rect>>) -> ClipOval {
        self.clipper = Some(clipper);
        self
    }

    /// Dart `ClipOval(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> ClipOval {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `ClipOval(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> ClipOval {
        self.child = Some(child.into_widget());
        self
    }
}

impl Default for ClipOval {
    fn default() -> ClipOval {
        ClipOval {
            key: None,
            clipper: None,
            clip_behavior: Clip::AntiAlias,
            child: None,
        }
    }
}

impl RenderObjectWidget for ClipOval {
    type RenderObject = RenderClipOval;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderClipOval::new(app, self.clipper.clone(), self.clip_behavior, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderClipOval>,
    ) {
        render_object.set_clipper(app, self.clipper.clone());
        render_object.set_clip_behavior(app, self.clip_behavior);
    }

    fn did_unmount_render_object(
        &self,
        app: &mut App,
        render_object: RenderHandle<RenderClipOval>,
    ) {
        render_object.set_clipper(app, None);
    }
}

impl SingleChildRenderObjectWidget for ClipOval {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

impl Debug for ClipOval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClipOval")
            .field("key", &self.key)
            .field("clipper", &self.clipper.as_ref().map(|_| "..."))
            .field("clip_behavior", &self.clip_behavior)
            .field("child", &self.child)
            .finish()
    }
}

/// A widget that clips its child using a path.
///
/// Calls a callback on a delegate whenever the widget is to be
/// painted. The callback returns a path and the widget prevents the
/// child from painting outside the path.
///
/// Clipping to a path is expensive. Certain shapes have more
/// optimized widgets:
///
///  * To clip to a rectangle, consider [`ClipRect`].
///  * To clip to an oval or circle, consider [`ClipOval`].
///  * To clip to a rounded rectangle, consider [`ClipRRect`].
///
/// To clip to a particular `ShapeBorder`, consider using either the
/// [`ClipPath::shape`] static method or the `ShapeBorderClipper` custom clipper
/// class.
pub struct ClipPath {
    pub key: Option<KeyRef>,
    /// If non-`None`, determines which clip to use.
    ///
    /// The default clip, which is used if this property is `None`, is the
    /// bounding box rectangle of the widget. [`ClipRect`] is a more
    /// efficient way of obtaining that effect.
    pub clipper: Option<Rc<dyn CustomClipper<Arc<Path>>>>,
    /// Whether and how to clip the child.
    ///
    /// Defaults to [`Clip::AntiAlias`].
    pub clip_behavior: Clip,
    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
}

impl ClipPath {
    /// Creates a path clip; Dart's named arguments are the setters.
    pub fn new() -> ClipPath {
        ClipPath::default()
    }

    /// Creates a shape clip.
    ///
    /// Uses a `ShapeBorderClipper` to configure the [`ClipPath`] to clip to the
    /// given `ShapeBorder`. Dart's optional arguments are the setters of the
    /// [`ClipPathShape`] this returns.
    pub fn shape(shape: Box<dyn ShapeBorder>) -> ClipPathShape {
        ClipPathShape {
            key: None,
            shape,
            clip_behavior: Clip::AntiAlias,
            child: None,
        }
    }

    /// Dart `ClipPath(key:)`.
    pub fn key(mut self, key: KeyRef) -> ClipPath {
        self.key = Some(key);
        self
    }

    /// Dart `ClipPath(clipper:)`.
    pub fn clipper(mut self, clipper: Rc<dyn CustomClipper<Arc<Path>>>) -> ClipPath {
        self.clipper = Some(clipper);
        self
    }

    /// Dart `ClipPath(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> ClipPath {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `ClipPath(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> ClipPath {
        self.child = Some(child.into_widget());
        self
    }
}

impl Default for ClipPath {
    fn default() -> ClipPath {
        ClipPath {
            key: None,
            clipper: None,
            clip_behavior: Clip::AntiAlias,
            child: None,
        }
    }
}

impl RenderObjectWidget for ClipPath {
    type RenderObject = RenderClipPath;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderClipPath::new(app, self.clipper.clone(), self.clip_behavior, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderClipPath>,
    ) {
        render_object.set_clipper(app, self.clipper.clone());
        render_object.set_clip_behavior(app, self.clip_behavior);
    }

    fn did_unmount_render_object(
        &self,
        app: &mut App,
        render_object: RenderHandle<RenderClipPath>,
    ) {
        render_object.set_clipper(app, None);
    }
}

impl SingleChildRenderObjectWidget for ClipPath {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

impl Debug for ClipPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClipPath")
            .field("key", &self.key)
            .field("clipper", &self.clipper.as_ref().map(|_| "..."))
            .field("clip_behavior", &self.clip_behavior)
            .field("child", &self.child)
            .finish()
    }
}

/// Dart's `ClipPath.shape(..)` before its optional arguments are known: a [`Builder`] that
/// clips to a `ShapeBorder` resolved against the ambient [`Directionality`].
pub struct ClipPathShape {
    key: Option<KeyRef>,
    shape: Box<dyn ShapeBorder>,
    clip_behavior: Clip,
    child: Option<WidgetRef>,
}

impl ClipPathShape {
    /// Dart `ClipPath.shape(key:)`.
    pub fn key(mut self, key: KeyRef) -> ClipPathShape {
        self.key = Some(key);
        self
    }

    /// Dart `ClipPath.shape(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> ClipPathShape {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `ClipPath.shape(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> ClipPathShape {
        self.child = Some(child.into_widget());
        self
    }
}

/// The kind tag of [`IntoWidget`] for a [`ClipPathShape`].
pub struct ClipPathShapeKind;

impl IntoWidget<ClipPathShapeKind> for ClipPathShape {
    fn into_widget(self) -> WidgetRef {
        let ClipPathShape {
            key,
            shape,
            clip_behavior,
            child,
        } = self;
        let builder = Builder::new(move |app, context| {
            let text_direction = Directionality::maybe_of(app, context);
            ClipPath {
                key: None,
                clipper: Some(Rc::new(ShapeBorderClipper::new(
                    shape.clone_box(),
                    text_direction,
                ))),
                clip_behavior,
                child: child.clone(),
            }
            .into_widget()
        });
        match key {
            Some(key) => builder.key(key).into_widget(),
            None => builder.into_widget(),
        }
    }
}

// POSITIONING AND SIZING NODES

/// A widget that applies a transformation before painting its child.
///
/// Unlike `RotatedBox`, which applies a rotation prior to layout, this object
/// applies its transformation just prior to painting, which means the
/// transformation is not taken into account when calculating how much space
/// this widget's child (and thus this widget) consumes.
///
/// This example rotates and skews an orange box containing text, keeping the
/// top right corner pinned to its original position.
///
/// ```text
/// ColoredBox::new(Color::BLACK).child(
///     Transform::new(skew_y)
///         .alignment(AlignmentGeometry::TOP_RIGHT)
///         .child(Text::new("Apartment for rent!")),
/// )
/// ```
///
/// See also:
///
///  * `RotatedBox`, which rotates the child widget during layout, not just
///    during painting.
///  * [`FractionalTranslation`], which applies a translation to the child
///    that is relative to the child's size.
///  * `FittedBox`, which sizes and positions its child widget to fit the parent
///    according to a given `BoxFit` discipline.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
///
/// Dart's named constructors are the associated functions [`rotate`](Self::rotate),
/// [`translate`](Self::translate), [`scale`](Self::scale), and [`flip`](Self::flip).
/// Flutter's `filterQuality` waits on `RenderTransform.filterQuality`.
#[derive(Debug)]
pub struct Transform {
    pub key: Option<KeyRef>,
    /// The matrix to transform the child by during painting.
    pub transform: Matrix4,
    /// The origin of the coordinate system in which to apply the matrix,
    /// described relative to the point given by [`alignment`](Self::alignment).
    ///
    /// Setting an origin is equivalent to conjugating the transform matrix by a
    /// translation. This property is provided just for convenience.
    ///
    /// This offset is applied in addition to any [`alignment`](Self::alignment)
    /// transformation, so a [`rotate`](Self::rotate) with no origin turns the child about its
    /// center, while `Transform::rotate(PI).origin(Offset::new(75.0, 75.0))` on a 150x150
    /// child turns it about its bottom-right corner.
    pub origin: Option<Offset>,
    /// The alignment of the origin, relative to the size of the box.
    ///
    /// When this and [`origin`](Self::origin) are both `None`, the origin is the upper-left
    /// corner of this render object. The default for this field is `None` for some
    /// constructors, and `AlignmentGeometry::CENTER` for others.
    ///
    /// This is equivalent to setting an origin based on the size of the box.
    /// If it is specified at the same time as the [`origin`](Self::origin), both are applied.
    ///
    /// An `AlignmentDirectional::CENTER_START` value is the same as an `Alignment`
    /// whose `x` value is `-1.0` if [`Directionality::of`] returns
    /// [`TextDirection::Ltr`], and `1.0` if it returns [`TextDirection::Rtl`]. Similarly
    /// `AlignmentDirectional::CENTER_END` is the same as an `Alignment` whose `x` value is
    /// `1.0` if [`Directionality::of`] returns [`TextDirection::Ltr`], and `-1.0` if it
    /// returns [`TextDirection::Rtl`].
    pub alignment: Option<AlignmentGeometry>,
    /// Whether to transform registered hits into the child's resulting coordinate system.
    ///
    /// When `true`, hit coordinates within the parent's bounds are transformed to match
    /// where the child appears visually after any transformation such as translation,
    /// rotation, scaling, or skewing.
    ///
    /// When `false`, hit coordinates are not transformed, potentially causing taps to
    /// register in a different location relative to the child's visual position.
    ///
    /// **Important:** Even when [`transform_hit_tests`](Self::transform_hit_tests) is true,
    /// children cannot receive events outside the parent's bounds. Hit testing always starts
    /// with the parent's own bounds check in `RenderBox::hit_test`. If the pointer is outside
    /// the parent's bounds, `RenderBox::hit_test_children` is not invoked and the children are
    /// not considered for hit testing.
    ///
    /// Defaults to true.
    pub transform_hit_tests: bool,
    pub child: Option<WidgetRef>,
}

impl Transform {
    /// Creates a widget that transforms its child; Dart's optional named arguments are the
    /// setters.
    pub fn new(transform: Matrix4) -> Transform {
        Transform {
            key: None,
            transform,
            origin: None,
            alignment: None,
            transform_hit_tests: true,
            child: None,
        }
    }

    /// Creates a widget that transforms its child using a rotation around the
    /// center.
    ///
    /// The `angle` argument gives the rotation in clockwise radians.
    ///
    /// See also:
    ///
    ///  * `RotationTransition`, which animates changes in rotation smoothly
    ///    over a given duration.
    pub fn rotate(angle: f64) -> Transform {
        Transform::new(Transform::compute_rotation(angle)).alignment(AlignmentGeometry::CENTER)
    }

    /// Creates a widget that transforms its child using a translation.
    ///
    /// The `offset` argument specifies the translation.
    pub fn translate(offset: Offset) -> Transform {
        Transform::new(Matrix4::translation(offset.dx() as f32, offset.dy() as f32))
    }

    /// Creates a widget that scales its child along the 2D plane.
    ///
    /// The `scale_x` argument provides the scalar by which to multiply the `x`
    /// axis, and the `scale_y` argument provides the scalar by which to multiply
    /// the `y` axis. Either may be `None`, in which case the scaling factor for
    /// that axis defaults to 1.0.
    ///
    /// For convenience, to scale the child uniformly, instead of providing
    /// `scale_x` and `scale_y`, the `scale` parameter may be used.
    ///
    /// At least one of `scale`, `scale_x`, and `scale_y` must be non-`None`. If
    /// `scale` is provided, the other two must be `None`.
    ///
    /// The [`alignment`](Self::alignment) controls the origin of the scale; by default, this
    /// is the center of the box.
    ///
    /// See also:
    ///
    ///  * `ScaleTransition`, which animates changes in scale smoothly over a given
    ///    duration.
    pub fn scale(scale: Option<f64>, scale_x: Option<f64>, scale_y: Option<f64>) -> Transform {
        debug_assert!(
            !(scale.is_none() && scale_x.is_none() && scale_y.is_none()),
            "At least one of 'scale', 'scale_x' and 'scale_y' is required to be non-None"
        );
        debug_assert!(
            scale.is_none() || (scale_x.is_none() && scale_y.is_none()),
            "If 'scale' is non-None then 'scale_x' and 'scale_y' must be left None"
        );
        let transform = Matrix4::scale(
            scale.or(scale_x).unwrap_or(1.0) as f32,
            scale.or(scale_y).unwrap_or(1.0) as f32,
        );
        Transform::new(transform).alignment(AlignmentGeometry::CENTER)
    }

    /// Creates a widget that mirrors its child about the widget's center point.
    ///
    /// If `flip_x` is true, the child widget will be flipped horizontally.
    ///
    /// If `flip_y` is true, the child widget will be flipped vertically.
    ///
    /// If both are true, the child widget will be flipped both vertically and
    /// horizontally, equivalent to a 180 degree rotation.
    pub fn flip(flip_x: bool, flip_y: bool) -> Transform {
        let transform = Matrix4::scale(
            if flip_x { -1.0 } else { 1.0 },
            if flip_y { -1.0 } else { 1.0 },
        );
        Transform::new(transform).alignment(AlignmentGeometry::CENTER)
    }

    /// Dart `Transform(key:)`.
    pub fn key(mut self, key: KeyRef) -> Transform {
        self.key = Some(key);
        self
    }

    /// Dart `Transform(origin:)`.
    pub fn origin(mut self, origin: Offset) -> Transform {
        self.origin = Some(origin);
        self
    }

    /// Dart `Transform(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> Transform {
        self.alignment = Some(alignment);
        self
    }

    /// Dart `Transform(transformHitTests:)`.
    pub fn transform_hit_tests(mut self, transform_hit_tests: bool) -> Transform {
        self.transform_hit_tests = transform_hit_tests;
        self
    }

    /// Dart `Transform(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Transform {
        self.child = Some(child.into_widget());
        self
    }

    /// Computes a rotation matrix for an angle in radians, attempting to keep rotations
    /// at integral values for angles of 0, π/2, π, 3π/2.
    fn compute_rotation(radians: f64) -> Matrix4 {
        debug_assert!(
            radians.is_finite(),
            "Cannot compute the rotation matrix for a non-finite angle: {radians}"
        );
        if radians == 0.0 {
            return Matrix4::IDENTITY;
        }
        let sin = radians.sin();
        if sin == 1.0 {
            return Transform::create_z_rotation(1.0, 0.0);
        }
        if sin == -1.0 {
            return Transform::create_z_rotation(-1.0, 0.0);
        }
        let cos = radians.cos();
        if cos == -1.0 {
            return Transform::create_z_rotation(0.0, -1.0);
        }
        Transform::create_z_rotation(sin, cos)
    }

    fn create_z_rotation(sin: f64, cos: f64) -> Matrix4 {
        let (sin, cos) = (sin as f32, cos as f32);
        #[rustfmt::skip]
        let rotation = Matrix4::from_flutter_array(&[
            cos, sin, 0.0, 0.0,
            -sin, cos, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]);
        rotation
    }
}

impl RenderObjectWidget for Transform {
    type RenderObject = RenderTransform;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::maybe_of(app, context);
        RenderTransform::new(
            app,
            self.transform,
            self.origin,
            self.alignment,
            text_direction,
            self.transform_hit_tests,
            None,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderTransform>,
    ) {
        render_object.set_transform(app, self.transform);
        render_object.set_origin(app, self.origin);
        render_object.set_alignment(app, self.alignment);
        let text_direction = Directionality::maybe_of(app, context);
        render_object.set_text_direction(app, text_direction);
        render_object.set_transform_hit_tests(app, self.transform_hit_tests);
    }
}

impl SingleChildRenderObjectWidget for Transform {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Scales and positions its child within itself according to [`fit`](Self::fit).
///
/// See also:
///
/// * [`Transform`], which applies an arbitrary transform to its child widget at
///   paint time.
/// * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug)]
pub struct FittedBox {
    pub key: Option<KeyRef>,
    /// How to inscribe the child into the space allocated during layout.
    pub fit: BoxFit,
    /// How to align the child within its parent's bounds.
    ///
    /// An alignment of (-1.0, -1.0) aligns the child to the top-left corner of its
    /// parent's bounds. An alignment of (1.0, 0.0) aligns the child to the middle
    /// of the right edge of its parent's bounds.
    ///
    /// Defaults to `AlignmentGeometry::CENTER`.
    ///
    /// See also:
    ///
    ///  * `Alignment`, a class with convenient constants typically used to
    ///    specify an `AlignmentGeometry`.
    ///  * `AlignmentDirectional`, like `Alignment` for specifying alignments
    ///    relative to text direction.
    pub alignment: AlignmentGeometry,
    /// How to clip a child that overflows this box.
    ///
    /// Defaults to [`Clip::None`].
    pub clip_behavior: Clip,
    pub child: Option<WidgetRef>,
}

impl FittedBox {
    /// Creates a widget that scales and positions its child within itself according to
    /// [`fit`](Self::fit); Dart's named arguments are the setters.
    pub fn new() -> FittedBox {
        FittedBox::default()
    }

    /// Dart `FittedBox(key:)`.
    pub fn key(mut self, key: KeyRef) -> FittedBox {
        self.key = Some(key);
        self
    }

    /// Dart `FittedBox(fit:)`.
    pub fn fit(mut self, fit: BoxFit) -> FittedBox {
        self.fit = fit;
        self
    }

    /// Dart `FittedBox(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> FittedBox {
        self.alignment = alignment;
        self
    }

    /// Dart `FittedBox(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> FittedBox {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `FittedBox(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> FittedBox {
        self.child = Some(child.into_widget());
        self
    }
}

impl Default for FittedBox {
    fn default() -> FittedBox {
        FittedBox {
            key: None,
            fit: BoxFit::Contain,
            alignment: AlignmentGeometry::CENTER,
            clip_behavior: Clip::None,
            child: None,
        }
    }
}

impl RenderObjectWidget for FittedBox {
    type RenderObject = RenderFittedBox;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::maybe_of(app, context);
        let render_object =
            RenderFittedBox::new(app, self.fit, self.alignment, text_direction, None);
        render_object.set_clip_behavior(app, self.clip_behavior);
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderFittedBox>,
    ) {
        render_object.set_fit(app, self.fit);
        render_object.set_alignment(app, self.alignment);
        let text_direction = Directionality::maybe_of(app, context);
        render_object.set_text_direction(app, text_direction);
        render_object.set_clip_behavior(app, self.clip_behavior);
    }
}

impl SingleChildRenderObjectWidget for FittedBox {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Applies a translation transformation before painting its child.
///
/// The translation is expressed as an [`Offset`] scaled to the child's size. For
/// example, an [`Offset`] with a `dx` of 0.25 will result in a horizontal
/// translation of one quarter the width of the child.
///
/// Hit tests will only be detected inside the bounds of the
/// [`FractionalTranslation`], even if the contents are offset such that
/// they overflow.
///
/// See also:
///
///  * [`Transform`], which applies an arbitrary transform to its child widget at
///    paint time.
///  * [`Transform::translate`], which applies an absolute offset translation
///    transformation instead of an offset scaled to the child.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug)]
pub struct FractionalTranslation {
    pub key: Option<KeyRef>,
    /// The translation to apply to the child, scaled to the child's size.
    ///
    /// For example, an [`Offset`] with a `dx` of 0.25 will result in a horizontal
    /// translation of one quarter the width of the child.
    pub translation: Offset,
    /// Whether to apply the translation when performing hit tests.
    pub transform_hit_tests: bool,
    pub child: Option<WidgetRef>,
}

impl FractionalTranslation {
    /// Creates a widget that translates its child's painting; Dart's optional named
    /// arguments are the setters.
    pub fn new(translation: Offset) -> FractionalTranslation {
        FractionalTranslation {
            key: None,
            translation,
            transform_hit_tests: true,
            child: None,
        }
    }

    /// Dart `FractionalTranslation(key:)`.
    pub fn key(mut self, key: KeyRef) -> FractionalTranslation {
        self.key = Some(key);
        self
    }

    /// Dart `FractionalTranslation(transformHitTests:)`.
    pub fn transform_hit_tests(mut self, transform_hit_tests: bool) -> FractionalTranslation {
        self.transform_hit_tests = transform_hit_tests;
        self
    }

    /// Dart `FractionalTranslation(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> FractionalTranslation {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for FractionalTranslation {
    type RenderObject = RenderFractionalTranslation;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderFractionalTranslation::new(app, self.translation, self.transform_hit_tests, None)
            .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderFractionalTranslation>,
    ) {
        render_object.set_translation(app, self.translation);
        render_object.set_transform_hit_tests(app, self.transform_hit_tests);
    }
}

impl SingleChildRenderObjectWidget for FractionalTranslation {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

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
/// container to build a [`DecoratedBox`](crate::DecoratedBox) widget. If you find `Container`
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

/// A widget that defers the layout of its single child to a delegate.
///
/// The delegate can determine the layout constraints for the child and can decide where to
/// position the child. The delegate can also determine the size of the parent, but the size of
/// the parent cannot depend on the size of the child.
///
/// See also:
///
///  * [`SingleChildLayoutDelegate`], which controls the layout of the child.
///  * [`Align`], which sizes itself based on its child's size and positions the child according
///    to an [`Alignment`] value.
///  * `FractionallySizedBox`, which sizes its child based on a fraction of its own size and
///    positions the child according to an [`Alignment`] value.
///  * [`CustomMultiChildLayout`], which uses a delegate to position multiple children.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug)]
pub struct CustomSingleChildLayout {
    pub key: Option<KeyRef>,
    /// The delegate that controls the layout of the child.
    pub delegate: Rc<dyn SingleChildLayoutDelegate>,
    pub child: Option<WidgetRef>,
}

impl CustomSingleChildLayout {
    /// Creates a custom single child layout; Dart's optional named arguments are the setters.
    pub fn new(delegate: Rc<dyn SingleChildLayoutDelegate>) -> CustomSingleChildLayout {
        CustomSingleChildLayout {
            key: None,
            delegate,
            child: None,
        }
    }

    /// Dart `CustomSingleChildLayout(key:)`.
    pub fn key(mut self, key: KeyRef) -> CustomSingleChildLayout {
        self.key = Some(key);
        self
    }

    /// Dart `CustomSingleChildLayout(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> CustomSingleChildLayout {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for CustomSingleChildLayout {
    type RenderObject = RenderCustomSingleChildLayoutBox;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderCustomSingleChildLayoutBox::new(app, Rc::clone(&self.delegate), None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderCustomSingleChildLayoutBox>,
    ) {
        render_object.set_delegate(app, Rc::clone(&self.delegate));
    }
}

impl SingleChildRenderObjectWidget for CustomSingleChildLayout {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Metadata for identifying children in a [`CustomMultiChildLayout`].
///
/// [`MultiChildLayoutChildren::has_child`](reveal_rendering::MultiChildLayoutChildren::has_child),
/// [`layout_child`](reveal_rendering::MultiChildLayoutChildren::layout_child) and
/// [`position_child`](reveal_rendering::MultiChildLayoutChildren::position_child) use these
/// identifiers.
#[derive(Debug)]
pub struct LayoutId {
    pub key: Option<KeyRef>,
    /// An object representing the identity of this child.
    ///
    /// The [`id`](Self::id) needs to be unique among the children that the
    /// [`CustomMultiChildLayout`] manages.
    pub id: ChildLayoutId,
    pub child: WidgetRef,
}

impl LayoutId {
    /// Marks a child with a layout identifier.
    pub fn new<K>(id: ChildLayoutId, child: impl IntoWidget<K>) -> LayoutId {
        LayoutId {
            key: None,
            id,
            child: child.into_widget(),
        }
    }

    /// Dart `LayoutId(key:)`.
    pub fn key(mut self, key: KeyRef) -> LayoutId {
        self.key = Some(key);
        self
    }
}

impl ParentDataWidget for LayoutId {
    type ParentData = MultiChildLayoutParentData;

    /// Dart's `key ?? ValueKey<Object>(id)`; the id is already a key here.
    fn key(&self) -> Option<&KeyRef> {
        Some(self.key.as_ref().unwrap_or(&self.id))
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn apply_parent_data(&self, app: &mut App, render_object: AnyRenderObject) {
        debug_assert!(render_object.parent_data_is::<MultiChildLayoutParentData>(app));
        let parent_data = render_object.parent_data_of_mut::<MultiChildLayoutParentData>(app);
        if parent_data.id.as_ref() != Some(&self.id) {
            parent_data.id = Some(Rc::clone(&self.id));
            if let Some(parent) = render_object.parent(app) {
                parent.mark_needs_layout(app);
            }
        }
    }
}

/// A widget that uses a delegate to size and position multiple children.
///
/// The delegate can determine the layout constraints for each child and can decide where to
/// position each child. The delegate can also determine the size of the parent, but the size of
/// the parent cannot depend on the sizes of the children.
///
/// [`CustomMultiChildLayout`] is appropriate when there are complex relationships between the
/// size and positioning of multiple widgets. For simple cases, such as aligning a widget to one
/// or another edge, the [`Stack`] widget is more appropriate.
///
/// Each child must be wrapped in a [`LayoutId`] widget to identify the widget for the delegate.
///
/// See also:
///
///  * [`MultiChildLayoutDelegate`], for details about how to control the layout of the
///    children.
///  * [`Stack`], which arranges children relative to the edges of the container.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug)]
pub struct CustomMultiChildLayout {
    pub key: Option<KeyRef>,
    /// The delegate that controls the layout of the children.
    pub delegate: Rc<dyn MultiChildLayoutDelegate>,
    pub children: Vec<WidgetRef>,
}

impl CustomMultiChildLayout {
    /// Creates a custom multi-child layout; Dart's optional named arguments are the setters.
    pub fn new(delegate: Rc<dyn MultiChildLayoutDelegate>) -> CustomMultiChildLayout {
        CustomMultiChildLayout {
            key: None,
            delegate,
            children: Vec::new(),
        }
    }

    /// Dart `CustomMultiChildLayout(key:)`.
    pub fn key(mut self, key: KeyRef) -> CustomMultiChildLayout {
        self.key = Some(key);
        self
    }

    /// Dart `CustomMultiChildLayout(children:)`.
    pub fn children(
        mut self,
        children: impl IntoIterator<Item = WidgetRef>,
    ) -> CustomMultiChildLayout {
        self.children = children.into_iter().collect();
        self
    }
}

impl RenderObjectWidget for CustomMultiChildLayout {
    type RenderObject = RenderCustomMultiChildLayoutBox;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderCustomMultiChildLayoutBox::new(app, Rc::clone(&self.delegate)).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderCustomMultiChildLayoutBox>,
    ) {
        render_object.set_delegate(app, Rc::clone(&self.delegate));
    }
}

impl MultiChildRenderObjectWidget for CustomMultiChildLayout {
    fn children(&self) -> &[WidgetRef] {
        &self.children
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

/// A container widget that applies an arbitrary transform to its constraints,
/// and sizes its child using the resulting [`BoxConstraints`], optionally
/// clipping, or treating the overflow as an error.
///
/// This container sizes its child using a [`BoxConstraints`] created by applying
/// [`constraints_transform`](Self::constraints_transform) to its own constraints. This
/// container will then attempt to adopt the same size, within the limits of its own
/// constraints. If it ends up with a different size, it will align the child based on
/// [`alignment`](Self::alignment). If the container cannot expand enough to accommodate the
/// entire child, the child will be clipped if [`clip_behavior`](Self::clip_behavior) is not
/// [`Clip::None`].
///
/// When [`child`](Self::child) is `None`, this widget becomes as small as possible and never
/// overflows.
///
/// This widget can be used to ensure some of the child's natural dimensions are
/// honored. For instance, if the child requires a minimum height to fully display its
/// content, [`constraints_transform`](Self::constraints_transform) can be set to
/// [`max_height_unconstrained`](Self::max_height_unconstrained), so that the child may still
/// grow vertically when the parent fails to provide enough vertical space:
///
/// ```text
/// ConstrainedBox::new(BoxConstraints::new().min_height(40.0).max_height(100.0)).child(
///     ConstraintsTransformBox::new(ConstraintsTransformBox::max_height_unconstrained)
///         .child(Text::new("Hello World!")),
/// )
/// ```
///
/// See also:
///
///  * [`ConstrainedBox`], which renders a box which imposes constraints
///    on its child.
///  * [`OverflowBox`], a widget that imposes additional constraints on its child,
///    and allows the child to overflow itself.
///  * [`UnconstrainedBox`] which allows its children to render themselves
///    unconstrained and expands to fit them.
#[derive(Debug)]
pub struct ConstraintsTransformBox {
    pub key: Option<KeyRef>,
    /// The text direction to use when interpreting the [`alignment`](Self::alignment) if it is
    /// an `AlignmentDirectional`.
    ///
    /// Defaults to `None`, in which case `Directionality::maybe_of` is used to determine the
    /// text direction.
    pub text_direction: Option<TextDirection>,
    /// The alignment to use when laying out the child, if it has a different size
    /// than this widget.
    ///
    /// If this is an `AlignmentDirectional`, then
    /// [`text_direction`](Self::text_direction) must not be `None`.
    ///
    /// See also:
    ///
    ///  * `Alignment` for non-`Directionality`-aware alignments.
    ///  * `AlignmentDirectional` for `Directionality`-aware alignments.
    pub alignment: AlignmentGeometry,
    /// The function used to transform the incoming [`BoxConstraints`], to size
    /// [`child`](Self::child).
    ///
    /// The function must return a [`BoxConstraints`] that is normalized.
    ///
    /// See [`ConstraintsTransformBox`] for predefined common
    /// [`BoxConstraintsTransform`]s.
    pub constraints_transform: BoxConstraintsTransform,
    /// How to clip a child that overflows this box.
    ///
    /// Defaults to [`Clip::None`].
    pub clip_behavior: Clip,
    pub child: Option<WidgetRef>,
}

impl ConstraintsTransformBox {
    /// Creates a widget that uses a function to transform the constraints it
    /// passes to its child; Dart's optional named arguments are the setters.
    pub fn new(constraints_transform: BoxConstraintsTransform) -> ConstraintsTransformBox {
        ConstraintsTransformBox {
            key: None,
            text_direction: None,
            alignment: AlignmentGeometry::CENTER,
            constraints_transform,
            clip_behavior: Clip::None,
            child: None,
        }
    }

    /// Dart `ConstraintsTransformBox(key:)`.
    pub fn key(mut self, key: KeyRef) -> ConstraintsTransformBox {
        self.key = Some(key);
        self
    }

    /// Dart `ConstraintsTransformBox(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> ConstraintsTransformBox {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `ConstraintsTransformBox(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> ConstraintsTransformBox {
        self.alignment = alignment;
        self
    }

    /// Dart `ConstraintsTransformBox(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> ConstraintsTransformBox {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `ConstraintsTransformBox(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> ConstraintsTransformBox {
        self.child = Some(child.into_widget());
        self
    }

    /// A [`BoxConstraintsTransform`] that always returns its argument as-is (i.e.,
    /// it is an identity function).
    ///
    /// The [`ConstraintsTransformBox`] becomes a proxy widget that has no effect on
    /// layout if [`constraints_transform`](Self::constraints_transform) is set to this.
    pub fn unmodified(constraints: BoxConstraints) -> BoxConstraints {
        constraints
    }

    /// A [`BoxConstraintsTransform`] that always returns a [`BoxConstraints`] that
    /// imposes no constraints on either dimension.
    ///
    /// Setting [`constraints_transform`](Self::constraints_transform) to this allows
    /// [`child`](Self::child) to render at its "natural" size (equivalent to an
    /// [`UnconstrainedBox`] with `constrained_axis` set to `None`).
    pub fn unconstrained(_constraints: BoxConstraints) -> BoxConstraints {
        BoxConstraints::new()
    }

    /// A [`BoxConstraintsTransform`] that removes the width constraints from the
    /// input.
    ///
    /// Setting [`constraints_transform`](Self::constraints_transform) to this allows
    /// [`child`](Self::child) to render at its "natural" width (equivalent to an
    /// [`UnconstrainedBox`] with `constrained_axis` set to `Axis::Horizontal`).
    pub fn width_unconstrained(constraints: BoxConstraints) -> BoxConstraints {
        constraints.height_constraints()
    }

    /// A [`BoxConstraintsTransform`] that removes the height constraints from the
    /// input.
    ///
    /// Setting [`constraints_transform`](Self::constraints_transform) to this allows
    /// [`child`](Self::child) to render at its "natural" height (equivalent to an
    /// [`UnconstrainedBox`] with `constrained_axis` set to `Axis::Vertical`).
    pub fn height_unconstrained(constraints: BoxConstraints) -> BoxConstraints {
        constraints.width_constraints()
    }

    /// A [`BoxConstraintsTransform`] that removes the `max_height` constraint from
    /// the input.
    ///
    /// Setting [`constraints_transform`](Self::constraints_transform) to this allows
    /// [`child`](Self::child) to render at its "natural" height or the `min_height` of the
    /// incoming [`BoxConstraints`], whichever is larger.
    pub fn max_height_unconstrained(constraints: BoxConstraints) -> BoxConstraints {
        constraints.copy_with().max_height(f64::INFINITY)
    }

    /// A [`BoxConstraintsTransform`] that removes the `max_width` constraint from
    /// the input.
    ///
    /// Setting [`constraints_transform`](Self::constraints_transform) to this allows
    /// [`child`](Self::child) to render at its "natural" width or the `min_width` of the
    /// incoming [`BoxConstraints`], whichever is larger.
    pub fn max_width_unconstrained(constraints: BoxConstraints) -> BoxConstraints {
        constraints.copy_with().max_width(f64::INFINITY)
    }

    /// A [`BoxConstraintsTransform`] that removes both the `max_width` and the
    /// `max_height` constraints from the input.
    ///
    /// Setting [`constraints_transform`](Self::constraints_transform) to this allows
    /// [`child`](Self::child) to render at least its "natural" size, and grow along an axis if
    /// the incoming [`BoxConstraints`] has a larger minimum constraint on that axis.
    pub fn max_unconstrained(constraints: BoxConstraints) -> BoxConstraints {
        constraints
            .copy_with()
            .max_width(f64::INFINITY)
            .max_height(f64::INFINITY)
    }
}

impl RenderObjectWidget for ConstraintsTransformBox {
    type RenderObject = RenderConstraintsTransformBox;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = self
            .text_direction
            .or_else(|| Directionality::maybe_of(app, context));
        let render_object = RenderConstraintsTransformBox::new(
            app,
            self.alignment,
            text_direction,
            self.constraints_transform,
            None,
        );
        render_object.set_clip_behavior(app, self.clip_behavior);
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderConstraintsTransformBox>,
    ) {
        let text_direction = self
            .text_direction
            .or_else(|| Directionality::maybe_of(app, context));
        render_object.set_text_direction(app, text_direction);
        render_object.set_constraints_transform(app, self.constraints_transform);
        render_object.set_alignment(app, self.alignment);
        render_object.set_clip_behavior(app, self.clip_behavior);
    }
}

impl SingleChildRenderObjectWidget for ConstraintsTransformBox {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A widget that imposes no constraints on its child, allowing it to render
/// at its "natural" size.
///
/// This allows a child to render at the size it would render if it were alone
/// on an infinite canvas with no constraints. This container will then attempt
/// to adopt the same size, within the limits of its own constraints. If it ends
/// up with a different size, it will align the child based on
/// [`alignment`](Self::alignment). If the box cannot expand enough to accommodate the entire
/// child, the child will be clipped.
///
/// See also:
///
///  * [`ConstrainedBox`], for a box which imposes constraints on its child.
///  * [`Align`], which loosens the constraints given to the child rather than
///    removing them entirely.
///  * `Container`, a convenience widget that combines common painting,
///    positioning, and sizing widgets.
///  * [`OverflowBox`], a widget that imposes different constraints on its child
///    than it gets from its parent, possibly allowing the child to overflow
///    the parent.
///  * [`ConstraintsTransformBox`], a widget that sizes its child using a
///    transformed [`BoxConstraints`].
#[derive(Debug)]
pub struct UnconstrainedBox {
    pub key: Option<KeyRef>,
    /// The text direction to use when interpreting the [`alignment`](Self::alignment) if it is
    /// an `AlignmentDirectional`.
    pub text_direction: Option<TextDirection>,
    /// The alignment to use when laying out the child.
    ///
    /// If this is an `AlignmentDirectional`, then
    /// [`text_direction`](Self::text_direction) must not be `None`.
    ///
    /// See also:
    ///
    ///  * `Alignment` for non-`Directionality`-aware alignments.
    ///  * `AlignmentDirectional` for `Directionality`-aware alignments.
    pub alignment: AlignmentGeometry,
    /// The axis to retain constraints on, if any.
    ///
    /// If not set, or set to `None` (the default), neither axis will retain its
    /// constraints. If set to `Axis::Vertical`, then vertical constraints will
    /// be retained, and if set to `Axis::Horizontal`, then horizontal constraints
    /// will be retained.
    pub constrained_axis: Option<Axis>,
    /// How to clip a child that overflows this box.
    ///
    /// Defaults to [`Clip::None`].
    pub clip_behavior: Clip,
    pub child: Option<WidgetRef>,
}

impl UnconstrainedBox {
    /// Creates a widget that imposes no constraints on its child, allowing it to
    /// render at its "natural" size; Dart's named arguments are the setters.
    pub fn new() -> UnconstrainedBox {
        UnconstrainedBox::default()
    }

    /// Dart `UnconstrainedBox(key:)`.
    pub fn key(mut self, key: KeyRef) -> UnconstrainedBox {
        self.key = Some(key);
        self
    }

    /// Dart `UnconstrainedBox(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> UnconstrainedBox {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `UnconstrainedBox(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> UnconstrainedBox {
        self.alignment = alignment;
        self
    }

    /// Dart `UnconstrainedBox(constrainedAxis:)`.
    pub fn constrained_axis(mut self, constrained_axis: Axis) -> UnconstrainedBox {
        self.constrained_axis = Some(constrained_axis);
        self
    }

    /// Dart `UnconstrainedBox(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> UnconstrainedBox {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `UnconstrainedBox(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> UnconstrainedBox {
        self.child = Some(child.into_widget());
        self
    }

    fn axis_to_transform(constrained_axis: Option<Axis>) -> BoxConstraintsTransform {
        match constrained_axis {
            Some(Axis::Horizontal) => ConstraintsTransformBox::height_unconstrained,
            Some(Axis::Vertical) => ConstraintsTransformBox::width_unconstrained,
            None => ConstraintsTransformBox::unconstrained,
        }
    }
}

impl Default for UnconstrainedBox {
    fn default() -> UnconstrainedBox {
        UnconstrainedBox {
            key: None,
            text_direction: None,
            alignment: AlignmentGeometry::CENTER,
            constrained_axis: None,
            clip_behavior: Clip::None,
            child: None,
        }
    }
}

impl StatelessWidget for UnconstrainedBox {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let mut transform_box = ConstraintsTransformBox::new(UnconstrainedBox::axis_to_transform(
            self.constrained_axis,
        ))
        .alignment(self.alignment)
        .clip_behavior(self.clip_behavior);
        if let Some(text_direction) = self.text_direction {
            transform_box = transform_box.text_direction(text_direction);
        }
        if let Some(child) = &self.child {
            transform_box = transform_box.child(child.clone());
        }
        transform_box.into_widget()
    }
}

/// A widget that sizes its child to a fraction of the total available space.
/// For more details about the layout algorithm, see
/// [`RenderFractionallySizedOverflowBox`].
///
/// See also:
///
///  * [`Align`], which sizes itself based on its child's size and positions
///    the child according to an `Alignment` value.
///  * [`OverflowBox`], a widget that imposes different constraints on its child
///    than it gets from its parent, possibly allowing the child to overflow the
///    parent.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug)]
pub struct FractionallySizedBox {
    pub key: Option<KeyRef>,
    /// How to align the child.
    ///
    /// The x and y values of the alignment control the horizontal and vertical
    /// alignment, respectively. An x value of -1.0 means that the left edge of
    /// the child is aligned with the left edge of the parent whereas an x value
    /// of 1.0 means that the right edge of the child is aligned with the right
    /// edge of the parent. Other values interpolate (and extrapolate) linearly.
    /// For example, a value of 0.0 means that the center of the child is aligned
    /// with the center of the parent.
    ///
    /// Defaults to `AlignmentGeometry::CENTER`.
    pub alignment: AlignmentGeometry,
    /// If non-`None`, the fraction of the incoming width given to the child.
    ///
    /// If non-`None`, the child is given a tight width constraint that is the max
    /// incoming width constraint multiplied by this factor.
    ///
    /// If `None`, the incoming width constraints are passed to the child
    /// unmodified.
    pub width_factor: Option<f64>,
    /// If non-`None`, the fraction of the incoming height given to the child.
    ///
    /// If non-`None`, the child is given a tight height constraint that is the max
    /// incoming height constraint multiplied by this factor.
    ///
    /// If `None`, the incoming height constraints are passed to the child
    /// unmodified.
    pub height_factor: Option<f64>,
    pub child: Option<WidgetRef>,
}

impl FractionallySizedBox {
    /// Creates a widget that sizes its child to a fraction of the total available space;
    /// Dart's named arguments are the setters.
    pub fn new() -> FractionallySizedBox {
        FractionallySizedBox::default()
    }

    /// Dart `FractionallySizedBox(key:)`.
    pub fn key(mut self, key: KeyRef) -> FractionallySizedBox {
        self.key = Some(key);
        self
    }

    /// Dart `FractionallySizedBox(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> FractionallySizedBox {
        self.alignment = alignment;
        self
    }

    /// Dart `FractionallySizedBox(widthFactor:)`.
    pub fn width_factor(mut self, width_factor: f64) -> FractionallySizedBox {
        debug_assert!(width_factor >= 0.0);
        self.width_factor = Some(width_factor);
        self
    }

    /// Dart `FractionallySizedBox(heightFactor:)`.
    pub fn height_factor(mut self, height_factor: f64) -> FractionallySizedBox {
        debug_assert!(height_factor >= 0.0);
        self.height_factor = Some(height_factor);
        self
    }

    /// Dart `FractionallySizedBox(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> FractionallySizedBox {
        self.child = Some(child.into_widget());
        self
    }
}

impl Default for FractionallySizedBox {
    fn default() -> FractionallySizedBox {
        FractionallySizedBox {
            key: None,
            alignment: AlignmentGeometry::CENTER,
            width_factor: None,
            height_factor: None,
            child: None,
        }
    }
}

impl RenderObjectWidget for FractionallySizedBox {
    type RenderObject = RenderFractionallySizedOverflowBox;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::maybe_of(app, context);
        let render_object =
            RenderFractionallySizedOverflowBox::new(app, self.alignment, text_direction, None);
        render_object.set_width_factor(app, self.width_factor);
        render_object.set_height_factor(app, self.height_factor);
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderFractionallySizedOverflowBox>,
    ) {
        render_object.set_alignment(app, self.alignment);
        render_object.set_width_factor(app, self.width_factor);
        render_object.set_height_factor(app, self.height_factor);
        let text_direction = Directionality::maybe_of(app, context);
        render_object.set_text_direction(app, text_direction);
    }
}

impl SingleChildRenderObjectWidget for FractionallySizedBox {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A box that limits its size only when it's unconstrained.
///
/// If this widget's maximum width is unconstrained then its child's width is
/// limited to [`max_width`](Self::max_width). Similarly, if this widget's maximum height is
/// unconstrained then its child's height is limited to [`max_height`](Self::max_height).
///
/// This has the effect of giving the child a natural dimension in unbounded
/// environments. For example, by providing a [`max_height`](Self::max_height) to a widget
/// that normally tries to be as big as possible, the widget will normally size
/// itself to fit its parent, but when placed in a vertical list, it will take
/// on the given height.
///
/// This is useful when composing widgets that normally try to match their
/// parents' size, so that they behave reasonably in lists (which are
/// unbounded).
///
/// See also:
///
///  * [`ConstrainedBox`], which applies its constraints in all cases, not just
///    when the incoming constraints are unbounded.
///  * [`SizedBox`], which lets you specify tight constraints by explicitly
///    specifying the height or width.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug)]
pub struct LimitedBox {
    pub key: Option<KeyRef>,
    /// The maximum width limit to apply in the absence of a
    /// [`BoxConstraints::max_width`] constraint.
    pub max_width: f64,
    /// The maximum height limit to apply in the absence of a
    /// [`BoxConstraints::max_height`] constraint.
    pub max_height: f64,
    pub child: Option<WidgetRef>,
}

impl LimitedBox {
    /// Creates a box that limits its size only when it's unconstrained; Dart's named
    /// arguments are the setters.
    pub fn new() -> LimitedBox {
        LimitedBox::default()
    }

    /// Dart `LimitedBox(key:)`.
    pub fn key(mut self, key: KeyRef) -> LimitedBox {
        self.key = Some(key);
        self
    }

    /// Dart `LimitedBox(maxWidth:)`; must not be negative.
    pub fn max_width(mut self, max_width: f64) -> LimitedBox {
        debug_assert!(max_width >= 0.0);
        self.max_width = max_width;
        self
    }

    /// Dart `LimitedBox(maxHeight:)`; must not be negative.
    pub fn max_height(mut self, max_height: f64) -> LimitedBox {
        debug_assert!(max_height >= 0.0);
        self.max_height = max_height;
        self
    }

    /// Dart `LimitedBox(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> LimitedBox {
        self.child = Some(child.into_widget());
        self
    }
}

impl Default for LimitedBox {
    fn default() -> LimitedBox {
        LimitedBox {
            key: None,
            max_width: f64::INFINITY,
            max_height: f64::INFINITY,
            child: None,
        }
    }
}

impl RenderObjectWidget for LimitedBox {
    type RenderObject = RenderLimitedBox;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderLimitedBox::new(app, self.max_width, self.max_height, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderLimitedBox>,
    ) {
        render_object.set_max_width(app, self.max_width);
        render_object.set_max_height(app, self.max_height);
    }
}

impl SingleChildRenderObjectWidget for LimitedBox {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A widget that imposes different constraints on its child than it gets
/// from its parent, possibly allowing the child to overflow the parent.
///
/// See also:
///
///  * [`RenderConstrainedOverflowBox`] for details about how [`OverflowBox`] is
///    rendered.
///  * [`SizedOverflowBox`], a widget that is a specific size but passes its
///    original constraints through to its child, which may then overflow.
///  * [`ConstrainedBox`], a widget that imposes additional constraints on its
///    child.
///  * [`UnconstrainedBox`], a container that tries to let its child draw without
///    constraints.
///  * [`SizedBox`], a box with a specified size.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug)]
pub struct OverflowBox {
    pub key: Option<KeyRef>,
    /// How to align the child.
    ///
    /// The x and y values of the alignment control the horizontal and vertical
    /// alignment, respectively. An x value of -1.0 means that the left edge of
    /// the child is aligned with the left edge of the parent whereas an x value
    /// of 1.0 means that the right edge of the child is aligned with the right
    /// edge of the parent. Other values interpolate (and extrapolate) linearly.
    /// For example, a value of 0.0 means that the center of the child is aligned
    /// with the center of the parent.
    ///
    /// Defaults to `AlignmentGeometry::CENTER`.
    pub alignment: AlignmentGeometry,
    /// The minimum width constraint to give the child. Set this to `None` (the
    /// default) to use the constraint from the parent instead.
    pub min_width: Option<f64>,
    /// The maximum width constraint to give the child. Set this to `None` (the
    /// default) to use the constraint from the parent instead.
    pub max_width: Option<f64>,
    /// The minimum height constraint to give the child. Set this to `None` (the
    /// default) to use the constraint from the parent instead.
    pub min_height: Option<f64>,
    /// The maximum height constraint to give the child. Set this to `None` (the
    /// default) to use the constraint from the parent instead.
    pub max_height: Option<f64>,
    /// The way to size the render object.
    ///
    /// This only affects the scenario where the child does not indeed overflow.
    /// If set to [`OverflowBoxFit::DeferToChild`], the render object will size itself to
    /// match the size of its child within the constraints of its parent or be
    /// as small as the parent allows if no child is set. If set to
    /// [`OverflowBoxFit::Max`] (the default), the render object will size itself
    /// to be as large as the parent allows.
    pub fit: OverflowBoxFit,
    pub child: Option<WidgetRef>,
}

impl OverflowBox {
    /// Creates a widget that lets its child overflow itself; Dart's named arguments are the
    /// setters.
    pub fn new() -> OverflowBox {
        OverflowBox::default()
    }

    /// Dart `OverflowBox(key:)`.
    pub fn key(mut self, key: KeyRef) -> OverflowBox {
        self.key = Some(key);
        self
    }

    /// Dart `OverflowBox(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> OverflowBox {
        self.alignment = alignment;
        self
    }

    /// Dart `OverflowBox(minWidth:)`.
    pub fn min_width(mut self, min_width: f64) -> OverflowBox {
        self.min_width = Some(min_width);
        self
    }

    /// Dart `OverflowBox(maxWidth:)`.
    pub fn max_width(mut self, max_width: f64) -> OverflowBox {
        self.max_width = Some(max_width);
        self
    }

    /// Dart `OverflowBox(minHeight:)`.
    pub fn min_height(mut self, min_height: f64) -> OverflowBox {
        self.min_height = Some(min_height);
        self
    }

    /// Dart `OverflowBox(maxHeight:)`.
    pub fn max_height(mut self, max_height: f64) -> OverflowBox {
        self.max_height = Some(max_height);
        self
    }

    /// Dart `OverflowBox(fit:)`.
    pub fn fit(mut self, fit: OverflowBoxFit) -> OverflowBox {
        self.fit = fit;
        self
    }

    /// Dart `OverflowBox(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> OverflowBox {
        self.child = Some(child.into_widget());
        self
    }
}

impl Default for OverflowBox {
    fn default() -> OverflowBox {
        OverflowBox {
            key: None,
            alignment: AlignmentGeometry::CENTER,
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            fit: OverflowBoxFit::Max,
            child: None,
        }
    }
}

impl RenderObjectWidget for OverflowBox {
    type RenderObject = RenderConstrainedOverflowBox;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::maybe_of(app, context);
        let render_object =
            RenderConstrainedOverflowBox::new(app, self.alignment, text_direction, None);
        render_object.set_min_width(app, self.min_width);
        render_object.set_max_width(app, self.max_width);
        render_object.set_min_height(app, self.min_height);
        render_object.set_max_height(app, self.max_height);
        render_object.set_fit(app, self.fit);
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderConstrainedOverflowBox>,
    ) {
        render_object.set_alignment(app, self.alignment);
        render_object.set_min_width(app, self.min_width);
        render_object.set_max_width(app, self.max_width);
        render_object.set_min_height(app, self.min_height);
        render_object.set_max_height(app, self.max_height);
        render_object.set_fit(app, self.fit);
        let text_direction = Directionality::maybe_of(app, context);
        render_object.set_text_direction(app, text_direction);
    }
}

impl SingleChildRenderObjectWidget for OverflowBox {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A widget that is a specific size but passes its original constraints
/// through to its child, which may then overflow.
///
/// See also:
///
///  * [`OverflowBox`], a widget that imposes different constraints on its child
///    than it gets from its parent, possibly allowing the child to overflow the
///    parent.
///  * [`ConstrainedBox`], a widget that imposes additional constraints on its
///    child.
///  * [`UnconstrainedBox`], a container that tries to let its child draw without
///    constraints.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug)]
pub struct SizedOverflowBox {
    pub key: Option<KeyRef>,
    /// How to align the child.
    ///
    /// The x and y values of the alignment control the horizontal and vertical
    /// alignment, respectively. An x value of -1.0 means that the left edge of
    /// the child is aligned with the left edge of the parent whereas an x value
    /// of 1.0 means that the right edge of the child is aligned with the right
    /// edge of the parent. Other values interpolate (and extrapolate) linearly.
    /// For example, a value of 0.0 means that the center of the child is aligned
    /// with the center of the parent.
    ///
    /// Defaults to `AlignmentGeometry::CENTER`.
    pub alignment: AlignmentGeometry,
    /// The size this widget should attempt to be.
    pub size: Size,
    pub child: Option<WidgetRef>,
}

impl SizedOverflowBox {
    /// Creates a widget of a given size that lets its child overflow; Dart's optional named
    /// arguments are the setters.
    pub fn new(size: Size) -> SizedOverflowBox {
        SizedOverflowBox {
            key: None,
            alignment: AlignmentGeometry::CENTER,
            size,
            child: None,
        }
    }

    /// Dart `SizedOverflowBox(key:)`.
    pub fn key(mut self, key: KeyRef) -> SizedOverflowBox {
        self.key = Some(key);
        self
    }

    /// Dart `SizedOverflowBox(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> SizedOverflowBox {
        self.alignment = alignment;
        self
    }

    /// Dart `SizedOverflowBox(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> SizedOverflowBox {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for SizedOverflowBox {
    type RenderObject = RenderSizedOverflowBox;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::of(app, context);
        RenderSizedOverflowBox::new(app, self.size, self.alignment, Some(text_direction), None)
            .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderSizedOverflowBox>,
    ) {
        render_object.set_alignment(app, self.alignment);
        render_object.set_requested_size(app, self.size);
        let text_direction = Directionality::of(app, context);
        render_object.set_text_direction(app, Some(text_direction));
    }
}

impl SingleChildRenderObjectWidget for SizedOverflowBox {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A widget that lays the child out as if it was in the tree, but without
/// painting anything, without making the child available for hit testing, and
/// without taking any room in the parent.
///
/// Offstage children are still active: they can receive focus and have keyboard
/// input directed to them.
///
/// Animations continue to run in offstage children, and therefore use battery
/// and CPU time, regardless of whether the animations end up being visible.
///
/// [`Offstage`] can be used to measure the dimensions of a widget without
/// bringing it on screen (yet). To hide a widget from view while it is not
/// needed, prefer removing the widget from the tree entirely rather than
/// keeping it alive in an [`Offstage`] subtree.
///
/// See also:
///
///  * `Visibility`, which can hide a child more efficiently (albeit less
///    subtly).
///  * [`TickerMode`](crate::TickerMode), which can be used to disable animations in a subtree.
///  * `SliverOffstage`, the sliver version of this widget.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
///
/// Flutter's `_OffstageElement` only overrides `debugVisitOnstageChildren`; this uses the
/// plain single-child element.
#[derive(Debug)]
pub struct Offstage {
    pub key: Option<KeyRef>,
    /// Whether the child is hidden from the rest of the tree.
    ///
    /// If true, the child is laid out as if it was in the tree, but without
    /// painting anything, without making the child available for hit testing, and
    /// without taking any room in the parent.
    ///
    /// Offstage children are still active: they can receive focus and have keyboard
    /// input directed to them.
    ///
    /// Animations continue to run in offstage children, and therefore use battery
    /// and CPU time, regardless of whether the animations end up being visible.
    ///
    /// If false, the child is included in the tree as normal.
    pub offstage: bool,
    pub child: Option<WidgetRef>,
}

impl Offstage {
    /// Creates a widget that visually hides its child; Dart's named arguments are the
    /// setters.
    pub fn new() -> Offstage {
        Offstage::default()
    }

    /// Dart `Offstage(key:)`.
    pub fn key(mut self, key: KeyRef) -> Offstage {
        self.key = Some(key);
        self
    }

    /// Dart `Offstage(offstage:)`.
    pub fn offstage(mut self, offstage: bool) -> Offstage {
        self.offstage = offstage;
        self
    }

    /// Dart `Offstage(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Offstage {
        self.child = Some(child.into_widget());
        self
    }
}

impl Default for Offstage {
    fn default() -> Offstage {
        Offstage {
            key: None,
            offstage: true,
            child: None,
        }
    }
}

impl RenderObjectWidget for Offstage {
    type RenderObject = RenderOffstage;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderOffstage::new(app, self.offstage, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderOffstage>,
    ) {
        render_object.set_offstage(app, self.offstage);
    }
}

impl SingleChildRenderObjectWidget for Offstage {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A widget that attempts to size the child to a specific aspect ratio.
///
/// The widget first tries the largest width permitted by the layout
/// constraints. The height of the widget is determined by applying the
/// given aspect ratio to the width, expressed as a ratio of width to height.
///
/// For example, a 16:9 width:height aspect ratio would have a value of
/// 16.0/9.0. If the maximum width is infinite, the initial width is determined
/// by applying the aspect ratio to the maximum height.
///
/// Now consider a second example, this time with an aspect ratio of 2.0 and
/// layout constraints that require the width to be between 0.0 and 100.0 and
/// the height to be between 0.0 and 100.0. We'll select a width of 100.0 (the
/// biggest allowed) and a height of 50.0 (to match the aspect ratio).
///
/// ## Setting the aspect ratio in unconstrained situations
///
/// When using a widget such as [`FittedBox`], the constraints are unbounded. This
/// results in [`AspectRatio`] being unable to find a suitable set of constraints
/// to apply. In that situation, consider explicitly setting a size using
/// [`SizedBox`] instead of setting the aspect ratio using [`AspectRatio`]. The size
/// is then scaled appropriately by the [`FittedBox`].
///
/// See also:
///
///  * [`Align`], a widget that aligns its child within itself and optionally
///    sizes itself based on the child's size.
///  * [`ConstrainedBox`], a widget that imposes additional constraints on its
///    child.
///  * [`UnconstrainedBox`], a container that tries to let its child draw without
///    constraints.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug)]
pub struct AspectRatio {
    pub key: Option<KeyRef>,
    /// The aspect ratio to attempt to use.
    ///
    /// The aspect ratio is expressed as a ratio of width to height. For example,
    /// a 16:9 width:height aspect ratio would have a value of 16.0/9.0.
    pub aspect_ratio: f64,
    pub child: Option<WidgetRef>,
}

impl AspectRatio {
    /// Creates a widget with a specific aspect ratio; Dart's optional named arguments are the
    /// setters.
    ///
    /// The `aspect_ratio` argument must be a finite number greater than zero.
    pub fn new(aspect_ratio: f64) -> AspectRatio {
        debug_assert!(aspect_ratio > 0.0);
        AspectRatio {
            key: None,
            aspect_ratio,
            child: None,
        }
    }

    /// Dart `AspectRatio(key:)`.
    pub fn key(mut self, key: KeyRef) -> AspectRatio {
        self.key = Some(key);
        self
    }

    /// Dart `AspectRatio(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> AspectRatio {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for AspectRatio {
    type RenderObject = RenderAspectRatio;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderAspectRatio::new(app, self.aspect_ratio, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderAspectRatio>,
    ) {
        render_object.set_aspect_ratio(app, self.aspect_ratio);
    }
}

impl SingleChildRenderObjectWidget for AspectRatio {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A widget that sizes its child to the child's maximum intrinsic width.
///
/// This class is useful, for example, when unlimited width is available and
/// you would like a child that would otherwise attempt to expand infinitely to
/// instead size itself to a more reasonable width. Additionally, putting a
/// [`Column`] inside an [`IntrinsicWidth`] will allow all [`Column`] children to be
/// as wide as the widest child.
///
/// The constraints that this widget passes to its child will adhere to the
/// parent's constraints, so if the constraints are not large enough to satisfy
/// the child's maximum intrinsic width, then the child will get less width
/// than it otherwise would. Likewise, if the minimum width constraint is
/// larger than the child's maximum intrinsic width, the child will be given
/// more width than it otherwise would.
///
/// If [`step_width`](Self::step_width) is non-`None`, the child's width will be snapped to a
/// multiple of the [`step_width`](Self::step_width). Similarly, if
/// [`step_height`](Self::step_height) is non-`None`, the child's height will be snapped to a
/// multiple of the [`step_height`](Self::step_height).
///
/// This class is relatively expensive, because it adds a speculative layout
/// pass before the final layout phase. Avoid using it where possible. In the
/// worst case, this widget can result in a layout that is O(N²) in the depth of
/// the tree.
///
/// See also:
///
///  * [`Align`], a widget that aligns its child within itself. This can be used
///    to loosen the constraints passed to the [`RenderIntrinsicWidth`],
///    allowing the [`RenderIntrinsicWidth`]'s child to be smaller than that of
///    its parent.
///  * [`Row`], which when used with `CrossAxisAlignment::Stretch` can be used
///    to loosen just the width constraints that are passed to the
///    [`RenderIntrinsicWidth`], allowing the [`RenderIntrinsicWidth`]'s child's
///    width to be smaller than that of its parent.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug, Default)]
pub struct IntrinsicWidth {
    pub key: Option<KeyRef>,
    /// If non-`None`, force the child's width to be a multiple of this value.
    ///
    /// If `None` or 0.0 the child's width will be the same as its maximum
    /// intrinsic width.
    ///
    /// This value must not be negative.
    ///
    /// See also:
    ///
    ///  * `AnyRenderBox::get_max_intrinsic_width`, which defines a widget's max
    ///    intrinsic width in general.
    pub step_width: Option<f64>,
    /// If non-`None`, force the child's height to be a multiple of this value.
    ///
    /// If `None` or 0.0 the child's height will not be constrained.
    ///
    /// This value must not be negative.
    pub step_height: Option<f64>,
    pub child: Option<WidgetRef>,
}

impl IntrinsicWidth {
    /// Creates a widget that sizes its child to the child's intrinsic width; Dart's named
    /// arguments are the setters.
    ///
    /// This class is relatively expensive. Avoid using it where possible.
    pub fn new() -> IntrinsicWidth {
        IntrinsicWidth::default()
    }

    /// Dart `IntrinsicWidth(key:)`.
    pub fn key(mut self, key: KeyRef) -> IntrinsicWidth {
        self.key = Some(key);
        self
    }

    /// Dart `IntrinsicWidth(stepWidth:)`.
    pub fn step_width(mut self, step_width: f64) -> IntrinsicWidth {
        debug_assert!(step_width >= 0.0);
        self.step_width = Some(step_width);
        self
    }

    /// Dart `IntrinsicWidth(stepHeight:)`.
    pub fn step_height(mut self, step_height: f64) -> IntrinsicWidth {
        debug_assert!(step_height >= 0.0);
        self.step_height = Some(step_height);
        self
    }

    /// Dart `IntrinsicWidth(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> IntrinsicWidth {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart's private `_stepWidth`: a step of 0.0 is no step at all.
    fn effective_step_width(&self) -> Option<f64> {
        self.step_width.filter(|step| *step != 0.0)
    }

    /// Dart's private `_stepHeight`.
    fn effective_step_height(&self) -> Option<f64> {
        self.step_height.filter(|step| *step != 0.0)
    }
}

impl RenderObjectWidget for IntrinsicWidth {
    type RenderObject = RenderIntrinsicWidth;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderIntrinsicWidth::new(
            app,
            self.effective_step_width(),
            self.effective_step_height(),
            None,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderIntrinsicWidth>,
    ) {
        render_object.set_step_width(app, self.effective_step_width());
        render_object.set_step_height(app, self.effective_step_height());
    }
}

impl SingleChildRenderObjectWidget for IntrinsicWidth {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A widget that sizes its child to the child's intrinsic height.
///
/// This class is useful, for example, when unlimited height is available and
/// you would like a child that would otherwise attempt to expand infinitely to
/// instead size itself to a more reasonable height. Additionally, putting a
/// [`Row`] inside an [`IntrinsicHeight`] will allow all [`Row`] children to be as tall
/// as the tallest child.
///
/// The constraints that this widget passes to its child will adhere to the
/// parent's constraints, so if the constraints are not large enough to satisfy
/// the child's maximum intrinsic height, then the child will get less height
/// than it otherwise would. Likewise, if the minimum height constraint is
/// larger than the child's maximum intrinsic height, the child will be given
/// more height than it otherwise would.
///
/// This class is relatively expensive, because it adds a speculative layout
/// pass before the final layout phase. Avoid using it where possible. In the
/// worst case, this widget can result in a layout that is O(N²) in the depth of
/// the tree.
///
/// See also:
///
///  * [`Align`], a widget that aligns its child within itself. This can be used
///    to loosen the constraints passed to the [`RenderIntrinsicHeight`],
///    allowing the [`RenderIntrinsicHeight`]'s child to be smaller than that of
///    its parent.
///  * [`Column`], which when used with `CrossAxisAlignment::Stretch` can be used
///    to loosen just the height constraints that are passed to the
///    [`RenderIntrinsicHeight`], allowing the [`RenderIntrinsicHeight`]'s child's
///    height to be smaller than that of its parent.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug, Default)]
pub struct IntrinsicHeight {
    pub key: Option<KeyRef>,
    pub child: Option<WidgetRef>,
}

impl IntrinsicHeight {
    /// Creates a widget that sizes its child to the child's intrinsic height; Dart's named
    /// arguments are the setters.
    ///
    /// This class is relatively expensive. Avoid using it where possible.
    pub fn new() -> IntrinsicHeight {
        IntrinsicHeight::default()
    }

    /// Dart `IntrinsicHeight(key:)`.
    pub fn key(mut self, key: KeyRef) -> IntrinsicHeight {
        self.key = Some(key);
        self
    }

    /// Dart `IntrinsicHeight(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> IntrinsicHeight {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for IntrinsicHeight {
    type RenderObject = RenderIntrinsicHeight;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderIntrinsicHeight::new(app, None).as_object()
    }
}

impl SingleChildRenderObjectWidget for IntrinsicHeight {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A widget that positions its child according to the child's baseline.
///
/// This widget shifts the child down such that the child's baseline (or the
/// bottom of the child, if the child has no baseline) is
/// [`baseline`](Self::baseline) logical pixels below the top of this box, then sizes this box
/// to contain the child. If [`baseline`](Self::baseline) is less than the distance from the
/// top of the child to the baseline of the child, then the child is top-aligned instead.
///
/// See also:
///
///  * [`Align`], a widget that aligns its child within itself and optionally
///    sizes itself based on the child's size.
///  * [`Center`], a widget that centers its child within itself.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug)]
pub struct Baseline {
    pub key: Option<KeyRef>,
    /// The number of logical pixels from the top of this box at which to position
    /// the child's baseline.
    pub baseline: f64,
    /// The type of baseline to use for positioning the child.
    pub baseline_type: TextBaseline,
    pub child: Option<WidgetRef>,
}

impl Baseline {
    /// Creates a widget that positions its child according to the child's baseline; Dart's
    /// optional named arguments are the setters.
    pub fn new(baseline: f64, baseline_type: TextBaseline) -> Baseline {
        Baseline {
            key: None,
            baseline,
            baseline_type,
            child: None,
        }
    }

    /// Dart `Baseline(key:)`.
    pub fn key(mut self, key: KeyRef) -> Baseline {
        self.key = Some(key);
        self
    }

    /// Dart `Baseline(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Baseline {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for Baseline {
    type RenderObject = RenderBaseline;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderBaseline::new(app, self.baseline, self.baseline_type, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderBaseline>,
    ) {
        render_object.set_baseline(app, self.baseline);
        render_object.set_baseline_type(app, self.baseline_type);
    }
}

impl SingleChildRenderObjectWidget for Baseline {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

// SLIVERS

/// A sliver that contains a single box widget.
///
/// Slivers are special-purpose widgets that can be combined using a `CustomScrollView` to
/// create custom scroll effects. A [`SliverToBoxAdapter`] is a basic sliver that creates a
/// bridge back to one of the usual box-based widgets.
///
/// _To learn more about slivers, see `CustomScrollView::slivers`._
///
/// Rather than using multiple [`SliverToBoxAdapter`] widgets to display multiple box widgets
/// in a `CustomScrollView`, consider using `SliverList`, `SliverFixedExtentList`,
/// `SliverPrototypeExtentList`, or `SliverGrid`, which are more efficient because they
/// instantiate only those children that are actually visible through the scroll view's
/// viewport.
///
/// See also:
///
///  * `CustomScrollView`, which displays a scrollable list of slivers.
///  * `SliverList`, which displays multiple box widgets in a linear array.
///  * `SliverFixedExtentList`, which displays multiple box widgets with the same main-axis
///    extent in a linear array.
///  * `SliverGrid`, which displays multiple box widgets in arbitrary positions.
#[derive(Debug, Default)]
pub struct SliverToBoxAdapter {
    pub key: Option<KeyRef>,
    pub child: Option<WidgetRef>,
}

impl SliverToBoxAdapter {
    /// Creates a sliver that contains a single box widget.
    pub fn new() -> SliverToBoxAdapter {
        SliverToBoxAdapter::default()
    }

    /// Dart `SliverToBoxAdapter(key:)`.
    pub fn key(mut self, key: KeyRef) -> SliverToBoxAdapter {
        self.key = Some(key);
        self
    }

    /// Dart `SliverToBoxAdapter(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> SliverToBoxAdapter {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for SliverToBoxAdapter {
    type RenderObject = RenderSliverToBoxAdapter;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderSliverToBoxAdapter::new(app, None).as_object()
    }
}

impl SingleChildRenderObjectWidget for SliverToBoxAdapter {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A sliver that applies padding on each side of another sliver.
///
/// Slivers are special-purpose widgets that can be combined using a `CustomScrollView` to
/// create custom scroll effects. A [`SliverPadding`] is a basic sliver that insets another
/// sliver by applying padding on each side.
///
/// Applying padding in the main axis of the viewport to a sliver whose
/// `SliverGeometry::scroll_extent` is longer than the viewport is generally undesirable, as
/// the padding is applied to the whole sliver rather than to the visible part of it.
///
/// See also:
///
///  * `CustomScrollView`, which displays a scrollable list of slivers.
///  * [`Padding`], the box version of this widget.
#[derive(Debug)]
pub struct SliverPadding {
    pub key: Option<KeyRef>,
    /// The amount of space by which to inset the child sliver.
    pub padding: EdgeInsetsGeometry,
    pub sliver: Option<WidgetRef>,
}

impl SliverPadding {
    /// Creates a sliver that applies padding on each side of another sliver.
    pub fn new(padding: EdgeInsetsGeometry) -> SliverPadding {
        SliverPadding {
            key: None,
            padding,
            sliver: None,
        }
    }

    /// Dart `SliverPadding(key:)`.
    pub fn key(mut self, key: KeyRef) -> SliverPadding {
        self.key = Some(key);
        self
    }

    /// Dart `SliverPadding(sliver:)`.
    pub fn sliver<K>(mut self, sliver: impl IntoWidget<K>) -> SliverPadding {
        self.sliver = Some(sliver.into_widget());
        self
    }
}

impl RenderObjectWidget for SliverPadding {
    type RenderObject = RenderSliverPadding;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::of(app, context);
        RenderSliverPadding::new(app, self.padding, Some(text_direction), None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderSliverPadding>,
    ) {
        render_object.set_padding(app, self.padding);
        let text_direction = Directionality::of(app, context);
        render_object.set_text_direction(app, Some(text_direction));
    }
}

impl SingleChildRenderObjectWidget for SliverPadding {
    fn child(&self) -> Option<&WidgetRef> {
        self.sliver.as_ref()
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

/// A widget that is invisible during hit testing.
///
/// When [`ignoring`](Self::ignoring) is true, this widget (and its subtree) is invisible
/// to hit testing. It still consumes space during layout and paints its child
/// as usual. It just cannot be the target of located events, because it returns
/// false from `RenderBox::hit_test`.
///
/// See also:
///
///  * [`AbsorbPointer`], which also prevents its children from receiving pointer
///    events but is itself visible to hit testing.
///  * `SliverIgnorePointer`, the sliver version of this widget.
///
/// Flutter's deprecated `ignoringSemantics` waits on accessibility.
#[derive(Debug)]
pub struct IgnorePointer {
    pub key: Option<KeyRef>,
    /// Whether this widget is ignored during hit testing.
    ///
    /// Regardless of whether this widget is ignored during hit testing, it will
    /// still consume space during layout and be visible during painting.
    ///
    /// Defaults to true.
    pub ignoring: bool,
    pub child: Option<WidgetRef>,
}

impl IgnorePointer {
    /// Creates a widget that is invisible to hit testing; Dart's named arguments are the
    /// setters.
    pub fn new() -> IgnorePointer {
        IgnorePointer::default()
    }

    /// Dart `IgnorePointer(key:)`.
    pub fn key(mut self, key: KeyRef) -> IgnorePointer {
        self.key = Some(key);
        self
    }

    /// Dart `IgnorePointer(ignoring:)`.
    pub fn ignoring(mut self, ignoring: bool) -> IgnorePointer {
        self.ignoring = ignoring;
        self
    }

    /// Dart `IgnorePointer(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> IgnorePointer {
        self.child = Some(child.into_widget());
        self
    }
}

impl Default for IgnorePointer {
    fn default() -> IgnorePointer {
        IgnorePointer {
            key: None,
            ignoring: true,
            child: None,
        }
    }
}

impl RenderObjectWidget for IgnorePointer {
    type RenderObject = RenderIgnorePointer;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderIgnorePointer::new(app, self.ignoring, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderIgnorePointer>,
    ) {
        render_object.set_ignoring(app, self.ignoring);
    }
}

impl SingleChildRenderObjectWidget for IgnorePointer {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A widget that absorbs pointers during hit testing.
///
/// When [`absorbing`](Self::absorbing) is true, this widget prevents its subtree from
/// receiving pointer events by terminating hit testing at itself. It still consumes space
/// during layout and paints its child as usual. It just prevents its children
/// from being the target of located events, because it returns true from
/// `RenderBox::hit_test`.
///
/// See also:
///
///  * [`IgnorePointer`], which also prevents its children from receiving pointer
///    events but is itself invisible to hit testing.
///
/// Flutter's deprecated `ignoringSemantics` waits on accessibility.
#[derive(Debug)]
pub struct AbsorbPointer {
    pub key: Option<KeyRef>,
    /// Whether this widget absorbs pointers during hit testing.
    ///
    /// Regardless of whether this render object absorbs pointers during hit
    /// testing, it will still consume space during layout and be visible during
    /// painting.
    ///
    /// Defaults to true.
    pub absorbing: bool,
    pub child: Option<WidgetRef>,
}

impl AbsorbPointer {
    /// Creates a widget that absorbs pointers during hit testing; Dart's named arguments are
    /// the setters.
    pub fn new() -> AbsorbPointer {
        AbsorbPointer::default()
    }

    /// Dart `AbsorbPointer(key:)`.
    pub fn key(mut self, key: KeyRef) -> AbsorbPointer {
        self.key = Some(key);
        self
    }

    /// Dart `AbsorbPointer(absorbing:)`.
    pub fn absorbing(mut self, absorbing: bool) -> AbsorbPointer {
        self.absorbing = absorbing;
        self
    }

    /// Dart `AbsorbPointer(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> AbsorbPointer {
        self.child = Some(child.into_widget());
        self
    }
}

impl Default for AbsorbPointer {
    fn default() -> AbsorbPointer {
        AbsorbPointer {
            key: None,
            absorbing: true,
            child: None,
        }
    }
}

impl RenderObjectWidget for AbsorbPointer {
    type RenderObject = RenderAbsorbPointer;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderAbsorbPointer::new(app, self.absorbing, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderAbsorbPointer>,
    ) {
        render_object.set_absorbing(app, self.absorbing);
    }
}

impl SingleChildRenderObjectWidget for AbsorbPointer {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Holds opaque meta data in the render tree.
///
/// Useful for decorating the render tree with information that will be consumed
/// later. For example, you could store information in the render tree that will
/// be used when the user interacts with the render tree but has no visual impact
/// prior to the interaction.
pub struct MetaData {
    pub key: Option<KeyRef>,
    /// Opaque meta data ignored by the render tree.
    pub meta_data: Option<Rc<dyn Any>>,
    /// How to behave during hit testing.
    pub behavior: HitTestBehavior,
    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
}

impl MetaData {
    /// Creates a widget that hold opaque meta data; Dart's named arguments are the setters.
    ///
    /// The [`behavior`](Self::behavior) defaults to [`HitTestBehavior::DeferToChild`].
    pub fn new() -> MetaData {
        MetaData::default()
    }

    /// Dart `MetaData(key:)`.
    pub fn key(mut self, key: KeyRef) -> MetaData {
        self.key = Some(key);
        self
    }

    /// Dart `MetaData(metaData:)`.
    pub fn meta_data(mut self, meta_data: Rc<dyn Any>) -> MetaData {
        self.meta_data = Some(meta_data);
        self
    }

    /// Dart `MetaData(behavior:)`.
    pub fn behavior(mut self, behavior: HitTestBehavior) -> MetaData {
        self.behavior = behavior;
        self
    }

    /// Dart `MetaData(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> MetaData {
        self.child = Some(child.into_widget());
        self
    }
}

impl Default for MetaData {
    fn default() -> MetaData {
        MetaData {
            key: None,
            meta_data: None,
            behavior: HitTestBehavior::DeferToChild,
            child: None,
        }
    }
}

impl RenderObjectWidget for MetaData {
    type RenderObject = RenderMetaData;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderMetaData::new(app, self.meta_data.clone(), self.behavior, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderMetaData>,
    ) {
        render_object.set_meta_data(app, self.meta_data.clone());
        render_object.set_behavior(app, self.behavior);
    }
}

impl SingleChildRenderObjectWidget for MetaData {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// A widget that groups multiple [`BackdropFilter`] widgets so that they share one backdrop.
///
/// Every [`BackdropFilter::grouped`] descendant of this widget applies its filter to the
/// same snapshot of the scene, at the cost of one filter pass for the whole group. The
/// grouped filters must lie over the same background for the result to look right.
#[derive(Debug)]
pub struct BackdropGroup {
    pub key: Option<KeyRef>,
    /// The key the grouped filters share; a fresh one unless the caller supplies it.
    pub backdrop_key: BackdropKey,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl BackdropGroup {
    /// Creates a group with a fresh [`BackdropKey`]; Dart's named arguments are the setters.
    pub fn new<K>(child: impl IntoWidget<K>) -> BackdropGroup {
        BackdropGroup {
            key: None,
            backdrop_key: BackdropKey::new(),
            child: child.into_widget(),
        }
    }

    /// Dart `BackdropGroup(key:)`.
    pub fn key(mut self, key: KeyRef) -> BackdropGroup {
        self.key = Some(key);
        self
    }

    /// Dart `BackdropGroup(backdropKey:)`.
    pub fn backdrop_key(mut self, backdrop_key: BackdropKey) -> BackdropGroup {
        self.backdrop_key = backdrop_key;
        self
    }

    /// Returns the [`BackdropGroup`] enclosing `context`, if any, creating a dependency on it.
    pub fn of(app: &mut App, context: BuildContext) -> Option<&BackdropGroup> {
        context.depend_on_inherited_widget_of_exact_type::<BackdropGroup>(app)
    }
}

impl InheritedWidget for BackdropGroup {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &BackdropGroup) -> bool {
        old_widget.backdrop_key != self.backdrop_key
    }
}

/// A widget that applies a filter to the existing painted content and then paints `child`.
///
/// The filter will be applied to all the areas of the underlying background that the
/// widget's paint bounds cover, so the filter is bounded to the widget and clipped by the
/// enclosing clips.
///
/// Wrap it in a `ClipRect` to limit the effect; place it under a [`BackdropGroup`] and use
/// [`BackdropFilter::grouped`] to share one blur among several filters.
///
/// This effect is relatively expensive, especially if the filter is non-local, such as a
/// blur.
pub struct BackdropFilter {
    pub key: Option<KeyRef>,
    /// The configuration that resolves the filter against the widget's bounds at paint time.
    pub filter_config: ImageFilterConfig,
    /// The blend mode to use to apply the filtered background content onto the background
    /// surface.
    ///
    /// The default value of this property is [`BlendMode::SrcOver`].
    pub blend_mode: BlendMode,
    /// Whether or not the backdrop filter operation will be applied to the child.
    pub enabled: bool,
    /// A backdrop key that identifies a shared backdrop, when given explicitly.
    pub backdrop_group_key: Option<BackdropKey>,
    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
    /// Whether to look up the backdrop key from a parent [`BackdropGroup`].
    use_shared_key: bool,
}

impl BackdropFilter {
    /// Creates a backdrop filter from its configuration; Dart's named arguments are the
    /// setters.
    pub fn new(filter_config: ImageFilterConfig) -> BackdropFilter {
        BackdropFilter {
            key: None,
            filter_config,
            blend_mode: BlendMode::SrcOver,
            enabled: true,
            backdrop_group_key: None,
            child: None,
            use_shared_key: false,
        }
    }

    /// Dart `BackdropFilter(filter:)`: a filter applied as it is.
    pub fn filter(filter: ImageFilter) -> BackdropFilter {
        BackdropFilter::new(ImageFilterConfig::new(filter))
    }

    /// Creates a backdrop filter that shares the backdrop of the enclosing [`BackdropGroup`].
    pub fn grouped(filter_config: ImageFilterConfig) -> BackdropFilter {
        BackdropFilter {
            use_shared_key: true,
            ..BackdropFilter::new(filter_config)
        }
    }

    /// Dart `BackdropFilter(key:)`.
    pub fn key(mut self, key: KeyRef) -> BackdropFilter {
        self.key = Some(key);
        self
    }

    /// Dart `BackdropFilter(blendMode:)`.
    pub fn blend_mode(mut self, blend_mode: BlendMode) -> BackdropFilter {
        self.blend_mode = blend_mode;
        self
    }

    /// Dart `BackdropFilter(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> BackdropFilter {
        self.enabled = enabled;
        self
    }

    /// Dart `BackdropFilter(backdropGroupKey:)`; not for a [`grouped`](Self::grouped) filter.
    pub fn backdrop_group_key(mut self, backdrop_group_key: BackdropKey) -> BackdropFilter {
        debug_assert!(!self.use_shared_key);
        self.backdrop_group_key = Some(backdrop_group_key);
        self
    }

    /// Dart `BackdropFilter(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> BackdropFilter {
        self.child = Some(child.into_widget());
        self
    }

    fn get_backdrop_group_key(&self, app: &mut App, context: BuildContext) -> Option<BackdropKey> {
        if self.use_shared_key {
            return BackdropGroup::of(app, context).map(|group| group.backdrop_key);
        }
        self.backdrop_group_key
    }
}

impl RenderObjectWidget for BackdropFilter {
    type RenderObject = RenderBackdropFilter;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let backdrop_key = self.get_backdrop_group_key(app, context);
        RenderBackdropFilter::new(
            app,
            self.filter_config.clone(),
            self.blend_mode,
            self.enabled,
            backdrop_key,
            None,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderBackdropFilter>,
    ) {
        render_object.set_filter_config(app, self.filter_config.clone());
        render_object.set_enabled(app, self.enabled);
        render_object.set_blend_mode(app, self.blend_mode);
        let backdrop_key = self.get_backdrop_group_key(app, context);
        render_object.set_backdrop_key(app, backdrop_key);
    }
}

impl SingleChildRenderObjectWidget for BackdropFilter {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

impl Debug for BackdropFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BackdropFilter")
            .field("key", &self.key)
            .field("filter_config", &self.filter_config)
            .field("blend_mode", &self.blend_mode)
            .field("enabled", &self.enabled)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl Debug for MetaData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MetaData")
            .field("key", &self.key)
            .field("meta_data", &self.meta_data.as_ref().map(|_| "..."))
            .field("behavior", &self.behavior)
            .field("child", &self.child)
            .finish()
    }
}

// UTILITY NODES

/// A widget that builds its child.
///
/// Useful for attaching a key to an existing widget.
///
/// Dart's `KeyedSubtree.wrap` and `ensureUniqueKeysForList` wait on a key over `Object`
/// (`child.key ?? childIndex`); see `PORTING.md`.
#[derive(Debug)]
pub struct KeyedSubtree {
    pub key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl KeyedSubtree {
    /// Creates a widget that builds its child while assigning it a key.
    ///
    /// This is useful when you want to preserve the state of a widget when it moves around in
    /// the widget tree by associating it with a consistent key.
    pub fn new<K>(child: impl IntoWidget<K>) -> KeyedSubtree {
        KeyedSubtree {
            key: None,
            child: child.into_widget(),
        }
    }

    /// Dart `KeyedSubtree(key:)`.
    pub fn key(mut self, key: KeyRef) -> KeyedSubtree {
        self.key = Some(key);
        self
    }
}

impl StatelessWidget for KeyedSubtree {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        self.child.clone()
    }
}

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

/// A widget that paints its area with a specified color and then draws its
/// child on top of that color.
///
/// Flutter's private `_RenderColoredBox` is rendering's [`RenderColoredBox`].
#[derive(Debug)]
pub struct ColoredBox {
    pub key: Option<KeyRef>,
    /// The color to paint the background area with.
    pub color: AnyColor,
    /// Whether to apply anti-aliasing when painting the box.
    ///
    /// Defaults to `true`.
    ///
    /// When `true`, the painted box will have smooth edges. This is crucial for
    /// animations and transformations (such as rotation or scaling) where the
    /// widget's edges may not align perfectly with the physical pixel grid.
    /// Anti-aliasing allows for sub-pixel rendering, which prevents a 'jagged'
    /// appearance during motion and ensures visually smooth transitions.
    ///
    /// Set this to `false` for specific use cases where multiple [`ColoredBox`]
    /// widgets are positioned adjacent to each other to form a larger, seamless
    /// area of solid color. With anti-aliasing enabled (`true`), faint seams or
    /// gaps might appear between the boxes due to the semi-transparent pixels at
    /// their edges. Disabling anti-aliasing ensures that the boxes align perfectly
    /// without such visual artifacts.
    ///
    /// See also:
    ///
    ///  * `Paint::is_anti_alias`, the underlying property that this controls.
    pub is_anti_alias: bool,
    pub child: Option<WidgetRef>,
}

impl ColoredBox {
    /// Creates a widget that paints its area with the specified color; Dart's optional named
    /// arguments are the setters.
    pub fn new(color: impl Into<AnyColor>) -> ColoredBox {
        ColoredBox {
            key: None,
            color: color.into(),
            is_anti_alias: true,
            child: None,
        }
    }

    /// Dart `ColoredBox(key:)`.
    pub fn key(mut self, key: KeyRef) -> ColoredBox {
        self.key = Some(key);
        self
    }

    /// Dart `ColoredBox(isAntiAlias:)`.
    pub fn is_anti_alias(mut self, is_anti_alias: bool) -> ColoredBox {
        self.is_anti_alias = is_anti_alias;
        self
    }

    /// Dart `ColoredBox(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> ColoredBox {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for ColoredBox {
    type RenderObject = RenderColoredBox;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderColoredBox::new(app, self.color.color(), self.is_anti_alias, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderColoredBox>,
    ) {
        render_object.set_color(app, self.color.color());
        render_object.set_is_anti_alias(app, self.is_anti_alias);
    }
}

impl SingleChildRenderObjectWidget for ColoredBox {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

// LAYOUT NODES

/// Returns the [`AxisDirection`] in the given [`Axis`] in the current
/// [`Directionality`] (or the reverse if `reverse` is true).
///
/// If `axis` is [`Axis::Vertical`], this function returns [`AxisDirection::Down`]
/// unless `reverse` is true, in which case this function returns
/// [`AxisDirection::Up`].
///
/// If `axis` is [`Axis::Horizontal`], this function checks the current
/// [`Directionality`]. If the current [`Directionality`] is right-to-left, then
/// this function returns [`AxisDirection::Left`] (unless `reverse` is true, in
/// which case it returns [`AxisDirection::Right`]). Similarly, if the current
/// [`Directionality`] is left-to-right, then this function returns
/// [`AxisDirection::Right`] (unless `reverse` is true, in which case it returns
/// [`AxisDirection::Left`]).
///
/// This function is used by a number of scrolling widgets (e.g. `ListView`,
/// `GridView`, `PageView`, and `SingleChildScrollView`) to translate their
/// [`Axis`] and `reverse` properties into a concrete [`AxisDirection`].
pub fn get_axis_direction_from_axis_reverse_and_directionality(
    app: &mut App,
    context: BuildContext,
    axis: Axis,
    reverse: bool,
) -> AxisDirection {
    match axis {
        Axis::Horizontal => {
            let text_direction = Directionality::of(app, context);
            let axis_direction = text_direction_to_axis_direction(text_direction);
            if reverse {
                flip_axis_direction(axis_direction)
            } else {
                axis_direction
            }
        }
        Axis::Vertical => {
            if reverse {
                AxisDirection::Up
            } else {
                AxisDirection::Down
            }
        }
    }
}

/// A widget that positions its children relative to the edges of its box.
///
/// This class is useful if you want to overlap several children in a simple
/// way, for example having some text and an image, overlaid with a gradient and
/// a button attached to the bottom.
///
/// Each child of a [`Stack`] widget is either _positioned_ or _non-positioned_.
/// Positioned children are those wrapped in a [`Positioned`] widget that has at
/// least one non-`None` property. The stack sizes itself to contain all the
/// non-positioned children, which are positioned according to
/// [`alignment`](Self::alignment) (which defaults to the top-left corner in
/// left-to-right environments and the top-right corner in right-to-left
/// environments). The positioned children are then placed relative to the stack
/// according to their top, right, bottom, and left properties.
///
/// The stack paints its children in order with the first child being at the
/// bottom. If you want to change the order in which the children paint, you
/// can rebuild the stack with the children in the new order. If you reorder
/// the children in this way, consider giving the children non-`None` keys.
/// These keys will cause the framework to move the underlying objects for
/// the children to their new locations rather than recreate them at their
/// new location.
///
/// For more details about the stack layout algorithm, see `RenderStack`.
///
/// See also:
///
///  * [`Align`], which sizes itself based on its child's size and positions
///    the child according to an `Alignment` value.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug)]
pub struct Stack {
    pub key: Option<KeyRef>,
    /// How to align the non-positioned and partially-positioned children in the stack.
    ///
    /// The non-positioned children are placed relative to each other such that the points
    /// determined by [`alignment`](Self::alignment) are co-located. For example, if the
    /// alignment is `AlignmentGeometry::TOP_LEFT`, then the top left corner of each
    /// non-positioned child will be located at the same global coordinate.
    ///
    /// Partially-positioned children, those that do not specify an alignment in a particular
    /// axis (e.g. that have neither `top` nor `bottom` set), use the alignment to determine how
    /// they should be positioned in that under-specified axis.
    ///
    /// Defaults to `AlignmentGeometry::TOP_START`.
    pub alignment: AlignmentGeometry,
    /// The text direction with which to resolve [`alignment`](Self::alignment).
    ///
    /// Defaults to the ambient [`Directionality`].
    pub text_direction: Option<TextDirection>,
    /// How to size the non-positioned children in the stack.
    ///
    /// The constraints passed into the [`Stack`] from its parent are either loosened
    /// ([`StackFit::Loose`]) or tightened to their biggest size ([`StackFit::Expand`]).
    pub fit: StackFit,
    /// Stacks only clip children whose _geometry_ overflows the stack. A child that paints
    /// outside its bounds (e.g. a box with a shadow) will not be clipped, regardless of the
    /// value of this property. Similarly, a child that itself has a descendant that overflows
    /// the stack will not be clipped, as only the geometry of the stack's direct children are
    /// considered.
    ///
    /// Even when this is set to [`Clip::None`], the stack itself does not extend its hit-test
    /// region: pointer events that fall outside the stack's own bounds will not reach a child
    /// even if that child is painted in that area.
    ///
    /// Defaults to [`Clip::HardEdge`].
    pub clip_behavior: Clip,
    pub children: Vec<WidgetRef>,
}

impl Stack {
    /// Creates a `Stack`; Dart's named arguments are the setters.
    ///
    /// By default, the non-positioned children of the stack are aligned by their top left
    /// corners.
    pub fn new() -> Stack {
        Stack::default()
    }

    /// Dart `Stack(key:)`.
    pub fn key(mut self, key: KeyRef) -> Stack {
        self.key = Some(key);
        self
    }

    /// Dart `Stack(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> Stack {
        self.alignment = alignment;
        self
    }

    /// Dart `Stack(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> Stack {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `Stack(fit:)`.
    pub fn fit(mut self, fit: StackFit) -> Stack {
        self.fit = fit;
        self
    }

    /// Dart `Stack(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> Stack {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `Stack(children:)`.
    pub fn children(mut self, children: impl IntoIterator<Item = WidgetRef>) -> Stack {
        self.children = children.into_iter().collect();
        self
    }

    /// `Stack.createRenderObject`'s body, shared with [`RawIndexedStack`]'s text direction.
    fn resolved_text_direction(
        text_direction: Option<TextDirection>,
        app: &mut App,
        context: BuildContext,
    ) -> Option<TextDirection> {
        text_direction.or_else(|| Directionality::maybe_of(app, context))
    }
}

impl Default for Stack {
    fn default() -> Stack {
        Stack {
            key: None,
            alignment: AlignmentGeometry::TOP_START,
            text_direction: None,
            fit: StackFit::Loose,
            clip_behavior: Clip::HardEdge,
            children: Vec::new(),
        }
    }
}

impl RenderObjectWidget for Stack {
    type RenderObject = RenderStack;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let render_object = RenderStack::new(app);
        render_object.set_alignment(app, self.alignment);
        let text_direction = Stack::resolved_text_direction(self.text_direction, app, context);
        render_object.set_text_direction(app, text_direction);
        render_object.set_fit(app, self.fit);
        render_object.set_clip_behavior(app, self.clip_behavior);
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderStack>,
    ) {
        render_object.set_alignment(app, self.alignment);
        let text_direction = Stack::resolved_text_direction(self.text_direction, app, context);
        render_object.set_text_direction(app, text_direction);
        render_object.set_fit(app, self.fit);
        render_object.set_clip_behavior(app, self.clip_behavior);
    }
}

impl MultiChildRenderObjectWidget for Stack {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

/// A widget that controls where a child of a [`Stack`] is positioned.
///
/// A [`Positioned`] widget must be a descendant of a [`Stack`], and the path from the
/// [`Positioned`] widget to its enclosing [`Stack`] must contain only stateless or stateful
/// widgets (not other kinds of widgets, like render object widgets).
///
/// If a widget is wrapped in a [`Positioned`], then it is a _positioned_ widget in its
/// [`Stack`]. If the [`top`](Self::top) property is non-`None`, the top edge of this child
/// will be positioned [`top`](Self::top) layout units from the top of the stack widget. The
/// [`right`](Self::right), [`bottom`](Self::bottom), and [`left`](Self::left) properties work
/// analogously.
///
/// If both the [`top`](Self::top) and [`bottom`](Self::bottom) properties are non-`None`, then
/// the child will be forced to have exactly the height required to satisfy both constraints.
/// Similarly, setting the [`right`](Self::right) and [`left`](Self::left) properties to
/// non-`None` values will force the child to have a particular width. Alternatively the
/// [`width`](Self::width) and [`height`](Self::height) properties can be used to give the
/// dimensions, with one corresponding position property (e.g. [`top`](Self::top) and
/// [`height`](Self::height)).
///
/// If all three values on a particular axis are `None`, then the [`Stack::alignment`] property
/// is used to position the child.
///
/// If all six values are `None`, the child is a non-positioned child. The [`Stack`] uses only
/// the non-positioned children to size itself.
///
/// Only two out of the three horizontal values ([`left`](Self::left), [`right`](Self::right),
/// [`width`](Self::width)), and only two out of the three vertical values
/// ([`top`](Self::top), [`bottom`](Self::bottom), [`height`](Self::height)), can be set. In
/// each case, at least one of the three must be `None`.
#[derive(Debug)]
pub struct Positioned {
    pub key: Option<KeyRef>,
    /// The distance that the child's left edge is inset from the left of the stack.
    ///
    /// Only two out of the three horizontal values ([`left`](Self::left),
    /// [`right`](Self::right), [`width`](Self::width)) can be set. The third must be `None`.
    ///
    /// If all three are `None`, the [`Stack::alignment`] is used to position the child
    /// horizontally.
    pub left: Option<f64>,
    /// The distance that the child's top edge is inset from the top of the stack.
    ///
    /// Only two out of the three vertical values ([`top`](Self::top), [`bottom`](Self::bottom),
    /// [`height`](Self::height)) can be set. The third must be `None`.
    ///
    /// If all three are `None`, the [`Stack::alignment`] is used to position the child
    /// vertically.
    pub top: Option<f64>,
    /// The distance that the child's right edge is inset from the right of the stack.
    ///
    /// See [`left`](Self::left).
    pub right: Option<f64>,
    /// The distance that the child's bottom edge is inset from the bottom of the stack.
    ///
    /// See [`top`](Self::top).
    pub bottom: Option<f64>,
    /// The child's width.
    ///
    /// See [`left`](Self::left).
    pub width: Option<f64>,
    /// The child's height.
    ///
    /// See [`top`](Self::top).
    pub height: Option<f64>,
    pub child: WidgetRef,
}

impl Positioned {
    /// Creates a widget that controls where a child of a [`Stack`] is positioned; Dart's
    /// optional named arguments are the setters.
    pub fn new<K>(child: impl IntoWidget<K>) -> Positioned {
        Positioned {
            key: None,
            left: None,
            top: None,
            right: None,
            bottom: None,
            width: None,
            height: None,
            child: child.into_widget(),
        }
    }

    /// Creates a `Positioned` object with the values from the given `Rect`.
    ///
    /// This sets the [`left`](Self::left), [`top`](Self::top), [`width`](Self::width), and
    /// [`height`](Self::height) properties from the given `Rect`. The [`right`](Self::right)
    /// and [`bottom`](Self::bottom) properties are set to `None`.
    pub fn from_rect<K>(rect: Rect, child: impl IntoWidget<K>) -> Positioned {
        Positioned {
            key: None,
            left: Some(rect.left),
            top: Some(rect.top),
            right: None,
            bottom: None,
            width: Some(rect.width()),
            height: Some(rect.height()),
            child: child.into_widget(),
        }
    }

    /// Creates a `Positioned` object with the values from the given [`RelativeRect`].
    ///
    /// This sets the [`left`](Self::left), [`top`](Self::top), [`right`](Self::right), and
    /// [`bottom`](Self::bottom) properties from the given [`RelativeRect`]. The
    /// [`height`](Self::height) and [`width`](Self::width) properties are set to `None`.
    pub fn from_relative_rect<K>(rect: RelativeRect, child: impl IntoWidget<K>) -> Positioned {
        Positioned {
            key: None,
            left: Some(rect.left),
            top: Some(rect.top),
            right: Some(rect.right),
            bottom: Some(rect.bottom),
            width: None,
            height: None,
            child: child.into_widget(),
        }
    }

    /// Creates a `Positioned` object with [`left`](Self::left), [`top`](Self::top),
    /// [`right`](Self::right), and [`bottom`](Self::bottom) set to 0.0 unless a value for them
    /// is passed.
    pub fn fill<K>(child: impl IntoWidget<K>) -> Positioned {
        Positioned {
            key: None,
            left: Some(0.0),
            top: Some(0.0),
            right: Some(0.0),
            bottom: Some(0.0),
            width: None,
            height: None,
            child: child.into_widget(),
        }
    }

    /// Creates a widget that controls where a child of a [`Stack`] is positioned, taking
    /// `start` and `end` rather than [`left`](Self::left) and [`right`](Self::right).
    ///
    /// If `text_direction` is `TextDirection::Rtl`, then the `start` argument is used for the
    /// [`right`](Self::right) property and the `end` argument is used for the
    /// [`left`](Self::left) property. Otherwise, if `text_direction` is `TextDirection::Ltr`,
    /// then the `start` argument is used for the [`left`](Self::left) property and the `end`
    /// argument is used for the [`right`](Self::right) property.
    ///
    /// Dart `Positioned.directional(textDirection:, start:, end:, child:)`; `start` and `end`
    /// are arguments rather than setters because resolving them needs the text direction.
    pub fn directional<K>(
        text_direction: TextDirection,
        start: Option<f64>,
        end: Option<f64>,
        child: impl IntoWidget<K>,
    ) -> Positioned {
        let (left, right) = match text_direction {
            TextDirection::Rtl => (end, start),
            TextDirection::Ltr => (start, end),
        };
        Positioned {
            key: None,
            left,
            top: None,
            right,
            bottom: None,
            width: None,
            height: None,
            child: child.into_widget(),
        }
    }

    /// Dart `Positioned(key:)`.
    pub fn key(mut self, key: KeyRef) -> Positioned {
        self.key = Some(key);
        self
    }

    /// Dart `Positioned(left:)`.
    pub fn left(mut self, left: f64) -> Positioned {
        self.left = Some(left);
        self.debug_check_axes();
        self
    }

    /// Dart `Positioned(top:)`.
    pub fn top(mut self, top: f64) -> Positioned {
        self.top = Some(top);
        self.debug_check_axes();
        self
    }

    /// Dart `Positioned(right:)`.
    pub fn right(mut self, right: f64) -> Positioned {
        self.right = Some(right);
        self.debug_check_axes();
        self
    }

    /// Dart `Positioned(bottom:)`.
    pub fn bottom(mut self, bottom: f64) -> Positioned {
        self.bottom = Some(bottom);
        self.debug_check_axes();
        self
    }

    /// Dart `Positioned(width:)`.
    pub fn width(mut self, width: f64) -> Positioned {
        self.width = Some(width);
        self.debug_check_axes();
        self
    }

    /// Dart `Positioned(height:)`.
    pub fn height(mut self, height: f64) -> Positioned {
        self.height = Some(height);
        self.debug_check_axes();
        self
    }

    /// The constructor asserts: at most two of the three values on each axis.
    fn debug_check_axes(&self) {
        debug_assert!(self.left.is_none() || self.right.is_none() || self.width.is_none());
        debug_assert!(self.top.is_none() || self.bottom.is_none() || self.height.is_none());
    }
}

impl ParentDataWidget for Positioned {
    type ParentData = StackParentData;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn debug_is_valid_render_object(&self, app: &App, render_object: AnyRenderObject) -> bool {
        render_object.parent_data_is::<StackParentData>(app)
            || render_object.parent_data_is::<TheaterParentData>(app)
    }

    fn apply_parent_data(&self, app: &mut App, render_object: AnyRenderObject) {
        // Dart's `_TheaterParentData extends StackParentData`, so an `Overlay` entry and an
        // `OverlayPortal` overlay child can be positioned too.
        let parent_data = if render_object.parent_data_is::<StackParentData>(app) {
            render_object.parent_data_of_mut::<StackParentData>(app)
        } else {
            render_object
                .parent_data_of_mut::<TheaterParentData>(app)
                .stack_mut()
        };
        let mut needs_layout = false;

        if parent_data.left != self.left {
            parent_data.left = self.left;
            needs_layout = true;
        }

        if parent_data.top != self.top {
            parent_data.top = self.top;
            needs_layout = true;
        }

        if parent_data.right != self.right {
            parent_data.right = self.right;
            needs_layout = true;
        }

        if parent_data.bottom != self.bottom {
            parent_data.bottom = self.bottom;
            needs_layout = true;
        }

        if parent_data.width != self.width {
            parent_data.width = self.width;
            needs_layout = true;
        }

        if parent_data.height != self.height {
            parent_data.height = self.height;
            needs_layout = true;
        }

        if needs_layout && let Some(parent) = render_object.parent(app) {
            parent.mark_needs_layout(app);
        }
    }
}

/// The `RenderFlex` configuration a [`Flex`], [`Row`], or [`Column`] pushes: every field Dart's
/// `Flex` declares except the key and the children, which the element manages.
#[derive(Clone, Copy, Debug)]
struct FlexConfiguration {
    direction: Axis,
    main_axis_alignment: MainAxisAlignment,
    main_axis_size: MainAxisSize,
    cross_axis_alignment: CrossAxisAlignment,
    text_direction: Option<TextDirection>,
    vertical_direction: VerticalDirection,
    text_baseline: Option<TextBaseline>,
    clip_behavior: Clip,
    spacing: f64,
}

impl FlexConfiguration {
    /// A flex laid out along `direction`, with every other field at Dart's default.
    fn new(direction: Axis) -> FlexConfiguration {
        FlexConfiguration {
            direction,
            main_axis_alignment: MainAxisAlignment::Start,
            main_axis_size: MainAxisSize::Max,
            cross_axis_alignment: CrossAxisAlignment::Center,
            text_direction: None,
            vertical_direction: VerticalDirection::Down,
            text_baseline: None,
            clip_behavior: Clip::None,
            spacing: 0.0,
        }
    }

    /// Dart's `Flex._needTextDirection`.
    fn need_text_direction(&self) -> bool {
        match self.direction {
            // because it affects the layout order.
            Axis::Horizontal => true,
            Axis::Vertical => {
                self.cross_axis_alignment == CrossAxisAlignment::Start
                    || self.cross_axis_alignment == CrossAxisAlignment::End
            }
        }
    }

    /// Dart's `Flex.getEffectiveTextDirection`.
    fn get_effective_text_direction(
        &self,
        app: &mut App,
        context: BuildContext,
    ) -> Option<TextDirection> {
        self.text_direction.or_else(|| {
            self.need_text_direction()
                .then(|| Directionality::maybe_of(app, context))
                .flatten()
        })
    }

    /// `Flex.createRenderObject`'s body.
    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let render_object = RenderFlex::new(app);
        self.push(app, context, render_object);
        render_object.as_object()
    }

    /// `Flex.updateRenderObject`'s body; also the shape of Dart's constructor cascade.
    fn push(&self, app: &mut App, context: BuildContext, render_object: RenderHandle<RenderFlex>) {
        render_object.set_direction(app, self.direction);
        render_object.set_main_axis_alignment(app, self.main_axis_alignment);
        render_object.set_main_axis_size(app, self.main_axis_size);
        render_object.set_cross_axis_alignment(app, self.cross_axis_alignment);
        let text_direction = self.get_effective_text_direction(app, context);
        render_object.set_text_direction(app, text_direction);
        render_object.set_vertical_direction(app, self.vertical_direction);
        render_object.set_text_baseline(app, self.text_baseline);
        render_object.set_clip_behavior(app, self.clip_behavior);
        render_object.set_spacing(app, self.spacing);
    }
}

/// A widget that displays its children in a one-dimensional array.
///
/// The [`Flex`] widget allows you to control the axis along which the children are placed
/// (horizontal or vertical). This is referred to as the _main axis_. If you know the main axis
/// in advance, then consider using a [`Row`] (if it's horizontal) or [`Column`] (if it's
/// vertical) instead, because that will be less verbose.
///
/// To cause a child to expand to fill the available space in the [`direction`](Self::direction)
/// of this widget's main axis, wrap the child in an [`Expanded`] widget.
///
/// The [`Flex`] widget does not scroll (and in general it is considered an error to have more
/// children in a [`Flex`] than will fit in the available room). If you have some widgets and
/// want them to be able to scroll if there is insufficient room, consider using a `ListView`.
///
/// If you only have one child, then rather than using [`Flex`], [`Row`] or [`Column`], consider
/// using [`Align`] or [`Center`] to position the child.
///
/// ## Layout algorithm
///
/// _This section describes how a [`Flex`] is rendered by the framework._
///
/// Layout for a [`Flex`] proceeds in six steps:
///
/// 1. Layout each child with a `None` or zero flex factor with unbounded main axis constraints
///    and the incoming cross axis constraints. If the [`cross_axis_alignment`](Self::cross_axis_alignment)
///    is `CrossAxisAlignment::Stretch`, instead use tight cross axis constraints that match the
///    incoming max extent in the cross axis.
/// 2. Divide the remaining main axis space among the children with non-zero flex factors
///    according to their flex factor.
/// 3. Layout each of the remaining children with the same cross axis constraints as in step 1,
///    but instead of using unbounded main axis constraints, use max axis constraints based on
///    the amount of space allocated in step 2.
/// 4. The width of the [`Flex`] is determined by the [`main_axis_size`](Self::main_axis_size)
///    property.
/// 5. Determine the position for each child according to the
///    [`main_axis_alignment`](Self::main_axis_alignment) and the
///    [`cross_axis_alignment`](Self::cross_axis_alignment).
///
/// See also:
///
///  * [`Row`], for a version of this widget that is always horizontal.
///  * [`Column`], for a version of this widget that is always vertical.
///  * [`Expanded`], to indicate children that should take all the remaining room.
///  * [`Flexible`], to indicate children that should share the remaining room.
///  * [`Spacer`], a widget that takes up space proportional to its flex value.
#[derive(Debug)]
pub struct Flex {
    pub key: Option<KeyRef>,
    /// The direction to use as the main axis.
    ///
    /// If you know the axis in advance, then consider using a [`Row`] (if it's horizontal) or
    /// [`Column`] (if it's vertical) instead of a [`Flex`], since that will be less verbose.
    pub direction: Axis,
    /// How the children should be placed along the main axis.
    ///
    /// For example, [`MainAxisAlignment::Start`], the default, places the children at the start
    /// (i.e., the left for a [`Row`] or the top for a [`Column`]) of the main axis.
    pub main_axis_alignment: MainAxisAlignment,
    /// How much space should be occupied in the main axis.
    ///
    /// After allocating space to children, there might be some remaining free space. This value
    /// controls whether to maximize or minimize the amount of free space, subject to the
    /// incoming layout constraints.
    pub main_axis_size: MainAxisSize,
    /// How the children should be placed along the cross axis.
    ///
    /// For example, [`CrossAxisAlignment::Center`], the default, centers the children in the
    /// cross axis (e.g., horizontally for a [`Column`]).
    pub cross_axis_alignment: CrossAxisAlignment,
    /// Determines the order to lay children out horizontally and how to interpret `start` and
    /// `end` in the horizontal direction.
    ///
    /// Defaults to the ambient [`Directionality`].
    ///
    /// If the [`direction`](Self::direction) is [`Axis::Horizontal`], this controls the order in
    /// which the children are positioned (left-to-right or right-to-left), and the meaning of
    /// the [`main_axis_alignment`](Self::main_axis_alignment) property's
    /// [`MainAxisAlignment::Start`] and [`MainAxisAlignment::End`] values.
    ///
    /// If the [`direction`](Self::direction) is [`Axis::Vertical`], this controls the meaning of
    /// the [`cross_axis_alignment`](Self::cross_axis_alignment) property's
    /// [`CrossAxisAlignment::Start`] and [`CrossAxisAlignment::End`] values.
    pub text_direction: Option<TextDirection>,
    /// Determines the order to lay children out vertically and how to interpret `start` and
    /// `end` in the vertical direction.
    ///
    /// Defaults to [`VerticalDirection::Down`].
    pub vertical_direction: VerticalDirection,
    /// If aligning items according to their baseline, which baseline to use.
    ///
    /// This must be set if using baseline alignment. There is no default because there is no
    /// way for the framework to know the correct baseline _a priori_.
    pub text_baseline: Option<TextBaseline>,
    /// Defaults to [`Clip::None`].
    pub clip_behavior: Clip,
    /// How much space to place between children in the main axis.
    pub spacing: f64,
    pub children: Vec<WidgetRef>,
}

impl Flex {
    /// Creates a flex layout; Dart's optional named arguments are the setters.
    ///
    /// The [`direction`](Self::direction) is required.
    ///
    /// The [`text_direction`](Self::text_direction) defaults to the ambient [`Directionality`],
    /// if any. If there is no ambient directionality, and a text direction is going to be
    /// necessary to decide which direction to lay the children in or to disambiguate `start` or
    /// `end` values for the main or cross axis directions, the
    /// [`text_direction`](Self::text_direction) must not be `None`.
    pub fn new(direction: Axis) -> Flex {
        let configuration = FlexConfiguration::new(direction);
        Flex {
            key: None,
            direction: configuration.direction,
            main_axis_alignment: configuration.main_axis_alignment,
            main_axis_size: configuration.main_axis_size,
            cross_axis_alignment: configuration.cross_axis_alignment,
            text_direction: configuration.text_direction,
            vertical_direction: configuration.vertical_direction,
            text_baseline: configuration.text_baseline,
            clip_behavior: configuration.clip_behavior,
            spacing: configuration.spacing,
            children: Vec::new(),
        }
    }

    /// Dart `Flex(key:)`.
    pub fn key(mut self, key: KeyRef) -> Flex {
        self.key = Some(key);
        self
    }

    /// Dart `Flex(mainAxisAlignment:)`.
    pub fn main_axis_alignment(mut self, main_axis_alignment: MainAxisAlignment) -> Flex {
        self.main_axis_alignment = main_axis_alignment;
        self
    }

    /// Dart `Flex(mainAxisSize:)`.
    pub fn main_axis_size(mut self, main_axis_size: MainAxisSize) -> Flex {
        self.main_axis_size = main_axis_size;
        self
    }

    /// Dart `Flex(crossAxisAlignment:)`.
    pub fn cross_axis_alignment(mut self, cross_axis_alignment: CrossAxisAlignment) -> Flex {
        self.cross_axis_alignment = cross_axis_alignment;
        self
    }

    /// Dart `Flex(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> Flex {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `Flex(verticalDirection:)`.
    pub fn vertical_direction(mut self, vertical_direction: VerticalDirection) -> Flex {
        self.vertical_direction = vertical_direction;
        self
    }

    /// Dart `Flex(textBaseline:)`.
    pub fn text_baseline(mut self, text_baseline: TextBaseline) -> Flex {
        self.text_baseline = Some(text_baseline);
        self
    }

    /// Dart `Flex(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> Flex {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `Flex(spacing:)`.
    pub fn spacing(mut self, spacing: f64) -> Flex {
        self.spacing = spacing;
        self
    }

    /// Dart `Flex(children:)`.
    pub fn children(mut self, children: impl IntoIterator<Item = WidgetRef>) -> Flex {
        self.children = children.into_iter().collect();
        self
    }

    /// The value to pass to `RenderFlex::set_text_direction`.
    ///
    /// This value is derived from the [`text_direction`](Self::text_direction) property and the
    /// ambient [`Directionality`]. The value is `None` if there is no need to specify the text
    /// direction. In practice there's always a need to specify the direction except for
    /// vertical flexes (e.g. [`Column`]s) whose
    /// [`cross_axis_alignment`](Self::cross_axis_alignment) is not dependent on the text
    /// direction (not `start` or `end`). In particular, a [`Row`] always needs a text direction
    /// because the text direction controls its layout order. (For [`Column`]s, the layout order
    /// is controlled by [`vertical_direction`](Self::vertical_direction), which is always
    /// specified as it does not depend on an inherited widget and defaults to
    /// [`VerticalDirection::Down`].)
    pub fn get_effective_text_direction(
        &self,
        app: &mut App,
        context: BuildContext,
    ) -> Option<TextDirection> {
        self.configuration()
            .get_effective_text_direction(app, context)
    }

    fn configuration(&self) -> FlexConfiguration {
        FlexConfiguration {
            direction: self.direction,
            main_axis_alignment: self.main_axis_alignment,
            main_axis_size: self.main_axis_size,
            cross_axis_alignment: self.cross_axis_alignment,
            text_direction: self.text_direction,
            vertical_direction: self.vertical_direction,
            text_baseline: self.text_baseline,
            clip_behavior: self.clip_behavior,
            spacing: self.spacing,
        }
    }
}

impl RenderObjectWidget for Flex {
    type RenderObject = RenderFlex;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        self.configuration().create_render_object(app, context)
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderFlex>,
    ) {
        self.configuration().push(app, context, render_object);
    }
}

impl MultiChildRenderObjectWidget for Flex {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

/// A widget that displays its children in a horizontal array.
///
/// To cause a child to expand to fill the available horizontal space, wrap the child in an
/// [`Expanded`] widget.
///
/// The [`Row`] widget does not scroll (and in general it is considered an error to have more
/// children in a [`Row`] than will fit in the available room). If you have a line of widgets and
/// want them to be able to scroll if there is insufficient room, consider using a `ListView`.
///
/// For a vertical variant, see [`Column`].
///
/// If you only have one child, then consider using [`Align`] or [`Center`] to position the
/// child.
///
/// See also:
///
///  * [`Flexible`], to indicate children that should share the remaining room.
///  * [`Expanded`], to indicate children that should take all the remaining room.
///  * [`Spacer`], a widget that takes up space proportional to its flex value.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
///
/// Dart's `class Row extends Flex`: a distinct widget type whose behaviour is [`Flex`]'s with
/// the direction fixed at [`Axis::Horizontal`].
#[derive(Debug)]
pub struct Row {
    pub key: Option<KeyRef>,
    /// See [`Flex::main_axis_alignment`].
    pub main_axis_alignment: MainAxisAlignment,
    /// See [`Flex::main_axis_size`].
    pub main_axis_size: MainAxisSize,
    /// See [`Flex::cross_axis_alignment`].
    pub cross_axis_alignment: CrossAxisAlignment,
    /// See [`Flex::text_direction`].
    pub text_direction: Option<TextDirection>,
    /// See [`Flex::vertical_direction`].
    pub vertical_direction: VerticalDirection,
    /// See [`Flex::text_baseline`].
    pub text_baseline: Option<TextBaseline>,
    /// See [`Flex::spacing`].
    pub spacing: f64,
    pub children: Vec<WidgetRef>,
}

impl Row {
    /// Creates a horizontal array of children; Dart's named arguments are the setters.
    pub fn new() -> Row {
        Row::default()
    }

    /// Dart `Row(key:)`.
    pub fn key(mut self, key: KeyRef) -> Row {
        self.key = Some(key);
        self
    }

    /// Dart `Row(mainAxisAlignment:)`.
    pub fn main_axis_alignment(mut self, main_axis_alignment: MainAxisAlignment) -> Row {
        self.main_axis_alignment = main_axis_alignment;
        self
    }

    /// Dart `Row(mainAxisSize:)`.
    pub fn main_axis_size(mut self, main_axis_size: MainAxisSize) -> Row {
        self.main_axis_size = main_axis_size;
        self
    }

    /// Dart `Row(crossAxisAlignment:)`.
    pub fn cross_axis_alignment(mut self, cross_axis_alignment: CrossAxisAlignment) -> Row {
        self.cross_axis_alignment = cross_axis_alignment;
        self
    }

    /// Dart `Row(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> Row {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `Row(verticalDirection:)`.
    pub fn vertical_direction(mut self, vertical_direction: VerticalDirection) -> Row {
        self.vertical_direction = vertical_direction;
        self
    }

    /// Dart `Row(textBaseline:)`.
    pub fn text_baseline(mut self, text_baseline: TextBaseline) -> Row {
        self.text_baseline = Some(text_baseline);
        self
    }

    /// Dart `Row(spacing:)`.
    pub fn spacing(mut self, spacing: f64) -> Row {
        self.spacing = spacing;
        self
    }

    /// Dart `Row(children:)`.
    pub fn children(mut self, children: impl IntoIterator<Item = WidgetRef>) -> Row {
        self.children = children.into_iter().collect();
        self
    }

    /// The direction to use as the main axis: always [`Axis::Horizontal`] (the inherited
    /// [`Flex::direction`]).
    pub fn direction(&self) -> Axis {
        Axis::Horizontal
    }

    /// See [`Flex::get_effective_text_direction`].
    pub fn get_effective_text_direction(
        &self,
        app: &mut App,
        context: BuildContext,
    ) -> Option<TextDirection> {
        self.configuration()
            .get_effective_text_direction(app, context)
    }

    fn configuration(&self) -> FlexConfiguration {
        FlexConfiguration {
            direction: self.direction(),
            main_axis_alignment: self.main_axis_alignment,
            main_axis_size: self.main_axis_size,
            cross_axis_alignment: self.cross_axis_alignment,
            text_direction: self.text_direction,
            vertical_direction: self.vertical_direction,
            text_baseline: self.text_baseline,
            spacing: self.spacing,
            ..FlexConfiguration::new(self.direction())
        }
    }
}

impl Default for Row {
    fn default() -> Row {
        let configuration = FlexConfiguration::new(Axis::Horizontal);
        Row {
            key: None,
            main_axis_alignment: configuration.main_axis_alignment,
            main_axis_size: configuration.main_axis_size,
            cross_axis_alignment: configuration.cross_axis_alignment,
            text_direction: configuration.text_direction,
            vertical_direction: configuration.vertical_direction,
            text_baseline: configuration.text_baseline,
            spacing: configuration.spacing,
            children: Vec::new(),
        }
    }
}

impl RenderObjectWidget for Row {
    type RenderObject = RenderFlex;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        self.configuration().create_render_object(app, context)
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderFlex>,
    ) {
        self.configuration().push(app, context, render_object);
    }
}

impl MultiChildRenderObjectWidget for Row {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

/// A widget that displays its children in a vertical array.
///
/// To cause a child to expand to fill the available vertical space, wrap the child in an
/// [`Expanded`] widget.
///
/// The [`Column`] widget does not scroll (and in general it is considered an error to have more
/// children in a [`Column`] than will fit in the available room). If you have a line of widgets
/// and want them to be able to scroll if there is insufficient room, consider using a
/// `ListView`.
///
/// For a horizontal variant, see [`Row`].
///
/// If you only have one child, then consider using [`Align`] or [`Center`] to position the
/// child.
///
/// See also:
///
///  * [`Flexible`], to indicate children that should share the remaining room.
///  * [`Expanded`], to indicate children that should take all the remaining room.
///  * [`Spacer`], a widget that takes up space proportional to its flex value.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
///
/// Dart's `class Column extends Flex`: a distinct widget type whose behaviour is [`Flex`]'s
/// with the direction fixed at [`Axis::Vertical`].
#[derive(Debug)]
pub struct Column {
    pub key: Option<KeyRef>,
    /// See [`Flex::main_axis_alignment`].
    pub main_axis_alignment: MainAxisAlignment,
    /// See [`Flex::main_axis_size`].
    pub main_axis_size: MainAxisSize,
    /// See [`Flex::cross_axis_alignment`].
    pub cross_axis_alignment: CrossAxisAlignment,
    /// See [`Flex::text_direction`].
    pub text_direction: Option<TextDirection>,
    /// See [`Flex::vertical_direction`].
    pub vertical_direction: VerticalDirection,
    /// See [`Flex::text_baseline`].
    pub text_baseline: Option<TextBaseline>,
    /// See [`Flex::spacing`].
    pub spacing: f64,
    pub children: Vec<WidgetRef>,
}

impl Column {
    /// Creates a vertical array of children; Dart's named arguments are the setters.
    pub fn new() -> Column {
        Column::default()
    }

    /// Dart `Column(key:)`.
    pub fn key(mut self, key: KeyRef) -> Column {
        self.key = Some(key);
        self
    }

    /// Dart `Column(mainAxisAlignment:)`.
    pub fn main_axis_alignment(mut self, main_axis_alignment: MainAxisAlignment) -> Column {
        self.main_axis_alignment = main_axis_alignment;
        self
    }

    /// Dart `Column(mainAxisSize:)`.
    pub fn main_axis_size(mut self, main_axis_size: MainAxisSize) -> Column {
        self.main_axis_size = main_axis_size;
        self
    }

    /// Dart `Column(crossAxisAlignment:)`.
    pub fn cross_axis_alignment(mut self, cross_axis_alignment: CrossAxisAlignment) -> Column {
        self.cross_axis_alignment = cross_axis_alignment;
        self
    }

    /// Dart `Column(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> Column {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `Column(verticalDirection:)`.
    pub fn vertical_direction(mut self, vertical_direction: VerticalDirection) -> Column {
        self.vertical_direction = vertical_direction;
        self
    }

    /// Dart `Column(textBaseline:)`.
    pub fn text_baseline(mut self, text_baseline: TextBaseline) -> Column {
        self.text_baseline = Some(text_baseline);
        self
    }

    /// Dart `Column(spacing:)`.
    pub fn spacing(mut self, spacing: f64) -> Column {
        self.spacing = spacing;
        self
    }

    /// Dart `Column(children:)`.
    pub fn children(mut self, children: impl IntoIterator<Item = WidgetRef>) -> Column {
        self.children = children.into_iter().collect();
        self
    }

    /// The direction to use as the main axis: always [`Axis::Vertical`] (the inherited
    /// [`Flex::direction`]).
    pub fn direction(&self) -> Axis {
        Axis::Vertical
    }

    /// See [`Flex::get_effective_text_direction`].
    pub fn get_effective_text_direction(
        &self,
        app: &mut App,
        context: BuildContext,
    ) -> Option<TextDirection> {
        self.configuration()
            .get_effective_text_direction(app, context)
    }

    fn configuration(&self) -> FlexConfiguration {
        FlexConfiguration {
            direction: self.direction(),
            main_axis_alignment: self.main_axis_alignment,
            main_axis_size: self.main_axis_size,
            cross_axis_alignment: self.cross_axis_alignment,
            text_direction: self.text_direction,
            vertical_direction: self.vertical_direction,
            text_baseline: self.text_baseline,
            spacing: self.spacing,
            ..FlexConfiguration::new(self.direction())
        }
    }
}

impl Default for Column {
    fn default() -> Column {
        let configuration = FlexConfiguration::new(Axis::Vertical);
        Column {
            key: None,
            main_axis_alignment: configuration.main_axis_alignment,
            main_axis_size: configuration.main_axis_size,
            cross_axis_alignment: configuration.cross_axis_alignment,
            text_direction: configuration.text_direction,
            vertical_direction: configuration.vertical_direction,
            text_baseline: configuration.text_baseline,
            spacing: configuration.spacing,
            children: Vec::new(),
        }
    }
}

impl RenderObjectWidget for Column {
    type RenderObject = RenderFlex;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        self.configuration().create_render_object(app, context)
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderFlex>,
    ) {
        self.configuration().push(app, context, render_object);
    }
}

impl MultiChildRenderObjectWidget for Column {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

/// A widget that controls how a child of a [`Row`], [`Column`], or [`Flex`] flexes.
///
/// Using a [`Flexible`] widget gives a child of a [`Row`], [`Column`], or [`Flex`] the
/// flexibility to expand to fill the available space in the main axis (e.g., horizontally for a
/// [`Row`] or vertically for a [`Column`]), but, unlike [`Expanded`], [`Flexible`] does not
/// require the child to fill the available space.
///
/// A [`Flexible`] widget must be a descendant of a [`Row`], [`Column`], or [`Flex`], and the
/// path from the [`Flexible`] widget to its enclosing [`Row`], [`Column`], or [`Flex`] must
/// contain only stateless or stateful widgets (not other kinds of widgets, like render object
/// widgets).
///
/// See also:
///
///  * [`Expanded`], which forces the child to expand to fill the available space.
///  * [`Spacer`], a widget that takes up space proportional to its flex value.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
#[derive(Debug)]
pub struct Flexible {
    pub key: Option<KeyRef>,
    /// The flex factor to use for this child.
    ///
    /// If zero, the child is inflexible and determines its own size. If non-zero, the amount of
    /// space the child can occupy in the main axis is determined by dividing the free space
    /// (after placing the inflexible children) according to the flex factors of the flexible
    /// children.
    pub flex: i32,
    /// How a flexible child is inscribed into the available space.
    ///
    /// If [`flex`](Self::flex) is non-zero, the [`fit`](Self::fit) determines whether the child
    /// fills the space the parent makes available during layout. If the fit is
    /// [`FlexFit::Tight`], the child is required to fill the available space. If the fit is
    /// [`FlexFit::Loose`], the child can be at most as large as the available space (but is
    /// allowed to be smaller).
    pub fit: FlexFit,
    pub child: WidgetRef,
}

impl Flexible {
    /// Creates a widget that controls how a child of a [`Row`], [`Column`], or [`Flex`] flexes;
    /// Dart's optional named arguments are the setters.
    pub fn new<K>(child: impl IntoWidget<K>) -> Flexible {
        Flexible {
            key: None,
            flex: 1,
            fit: FlexFit::Loose,
            child: child.into_widget(),
        }
    }

    /// Dart `Flexible(key:)`.
    pub fn key(mut self, key: KeyRef) -> Flexible {
        self.key = Some(key);
        self
    }

    /// Dart `Flexible(flex:)`.
    pub fn flex(mut self, flex: i32) -> Flexible {
        self.flex = flex;
        self
    }

    /// Dart `Flexible(fit:)`.
    pub fn fit(mut self, fit: FlexFit) -> Flexible {
        self.fit = fit;
        self
    }

    /// `Flexible.applyParentData`'s body, shared with [`Expanded`].
    fn apply_flex_parent_data(
        app: &mut App,
        render_object: AnyRenderObject,
        flex: i32,
        fit: FlexFit,
    ) {
        debug_assert!(render_object.parent_data_is::<FlexParentData>(app));
        let parent_data = render_object.parent_data_of_mut::<FlexParentData>(app);
        let mut needs_layout = false;

        if parent_data.flex != Some(flex) {
            parent_data.flex = Some(flex);
            needs_layout = true;
        }

        if parent_data.fit != Some(fit) {
            parent_data.fit = Some(fit);
            needs_layout = true;
        }

        if needs_layout && let Some(parent) = render_object.parent(app) {
            parent.mark_needs_layout(app);
        }
    }
}

impl ParentDataWidget for Flexible {
    type ParentData = FlexParentData;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn apply_parent_data(&self, app: &mut App, render_object: AnyRenderObject) {
        Flexible::apply_flex_parent_data(app, render_object, self.flex, self.fit);
    }
}

/// A widget that expands a child of a [`Row`], [`Column`], or [`Flex`] so that the child fills
/// the available space.
///
/// Using an [`Expanded`] widget makes a child of a [`Row`], [`Column`], or [`Flex`] expand to
/// fill the available space along the main axis (e.g., horizontally for a [`Row`] or vertically
/// for a [`Column`]). If multiple children are expanded, the available space is divided among
/// them according to the [`flex`](Self::flex) factor.
///
/// An [`Expanded`] widget must be a descendant of a [`Row`], [`Column`], or [`Flex`], and the
/// path from the [`Expanded`] widget to its enclosing [`Row`], [`Column`], or [`Flex`] must
/// contain only stateless or stateful widgets (not other kinds of widgets, like render object
/// widgets).
///
/// See also:
///
///  * [`Flexible`], which does not force the child to fill the available space.
///  * [`Spacer`], a widget that takes up space proportional to its flex value.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
///
/// Dart's `class Expanded extends Flexible`: a distinct widget type whose behaviour is
/// [`Flexible`]'s with the fit fixed at [`FlexFit::Tight`].
#[derive(Debug)]
pub struct Expanded {
    pub key: Option<KeyRef>,
    /// See [`Flexible::flex`].
    pub flex: i32,
    pub child: WidgetRef,
}

impl Expanded {
    /// Creates a widget that expands a child of a [`Row`], [`Column`], or [`Flex`] so that the
    /// child fills the available space along the flex widget's main axis.
    pub fn new<K>(child: impl IntoWidget<K>) -> Expanded {
        Expanded {
            key: None,
            flex: 1,
            child: child.into_widget(),
        }
    }

    /// Dart `Expanded(key:)`.
    pub fn key(mut self, key: KeyRef) -> Expanded {
        self.key = Some(key);
        self
    }

    /// Dart `Expanded(flex:)`.
    pub fn flex(mut self, flex: i32) -> Expanded {
        self.flex = flex;
        self
    }

    /// How the child is inscribed into the available space: always [`FlexFit::Tight`] (the
    /// inherited [`Flexible::fit`]).
    pub fn fit(&self) -> FlexFit {
        FlexFit::Tight
    }
}

impl ParentDataWidget for Expanded {
    type ParentData = FlexParentData;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn apply_parent_data(&self, app: &mut App, render_object: AnyRenderObject) {
        Flexible::apply_flex_parent_data(app, render_object, self.flex, self.fit());
    }
}

/// Creates an adjustable, empty spacer that can be used to tune the spacing between widgets in
/// a [`Flex`] container, like [`Row`] or [`Column`].
///
/// The [`Spacer`] widget will take up any available space, so setting the
/// [`Flex::main_axis_alignment`] on a flex container that contains a [`Spacer`] to
/// [`MainAxisAlignment::SpaceAround`], [`MainAxisAlignment::SpaceBetween`], or
/// [`MainAxisAlignment::SpaceEvenly`] will not have any visible effect: the [`Spacer`] has
/// taken up all of the additional space, therefore there is none left to redistribute.
///
/// See also:
///
///  * [`Row`] and [`Column`], which are the most common containers to use a `Spacer` in.
///  * [`SizedBox`], to create a box with a specific size and an optional child.
///
/// Flutter's `widgets/spacer.dart`; it lives here until that file has a home of its own.
#[derive(Debug)]
pub struct Spacer {
    pub key: Option<KeyRef>,
    /// The flex factor to use in determining how much space to take up.
    ///
    /// The amount of space the [`Spacer`] can occupy in the main axis is determined by dividing
    /// the free space proportionately, after placing the inflexible children, according to the
    /// flex factors of the flexible children.
    ///
    /// Defaults to one.
    pub flex: i32,
}

impl Spacer {
    /// Creates a flexible space to insert into a [`Flexible`] widget; Dart's named arguments
    /// are the setters.
    pub fn new() -> Spacer {
        Spacer::default()
    }

    /// Dart `Spacer(key:)`.
    pub fn key(mut self, key: KeyRef) -> Spacer {
        self.key = Some(key);
        self
    }

    /// Dart `Spacer(flex:)`.
    pub fn flex(mut self, flex: i32) -> Spacer {
        debug_assert!(flex > 0);
        self.flex = flex;
        self
    }
}

impl Default for Spacer {
    fn default() -> Spacer {
        Spacer { key: None, flex: 1 }
    }
}

impl StatelessWidget for Spacer {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        Expanded::new(SizedBox::shrink())
            .flex(self.flex)
            .into_widget()
    }
}

/// A [`Stack`] that shows a single child from a list of children.
///
/// The displayed child is the one with the given [`index`](Self::index). The stack is always as
/// big as the largest child.
///
/// If the index is `None`, then nothing is displayed.
///
/// See also:
///
///  * [`Stack`], for more details about stacks.
///  * The [catalog of layout widgets](https://flutter.dev/widgets/layout/).
///
/// Flutter's `widgets/indexed_stack.dart`; it lives here until that file has a home of its own.
#[derive(Debug)]
pub struct IndexedStack {
    pub key: Option<KeyRef>,
    /// How to align the non-positioned and partially-positioned children in the stack.
    ///
    /// Defaults to `AlignmentGeometry::TOP_START`. See [`Stack::alignment`].
    pub alignment: AlignmentGeometry,
    /// The text direction with which to resolve [`alignment`](Self::alignment).
    ///
    /// Defaults to the ambient [`Directionality`].
    pub text_direction: Option<TextDirection>,
    /// Defaults to [`Clip::HardEdge`].
    pub clip_behavior: Clip,
    /// How to size the non-positioned children in the stack.
    ///
    /// Defaults to [`StackFit::Loose`]. See [`Stack::fit`].
    pub sizing: StackFit,
    /// The index of the child to show.
    ///
    /// If this is `None`, none of the children will be shown.
    pub index: Option<i32>,
    /// The child widgets of the stack.
    ///
    /// Only the child at index [`index`](Self::index) will be shown.
    pub children: Vec<WidgetRef>,
}

impl IndexedStack {
    /// Creates a [`Stack`] widget that paints a single child; Dart's named arguments are the
    /// setters.
    pub fn new() -> IndexedStack {
        IndexedStack::default()
    }

    /// Dart `IndexedStack(key:)`.
    pub fn key(mut self, key: KeyRef) -> IndexedStack {
        self.key = Some(key);
        self
    }

    /// Dart `IndexedStack(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> IndexedStack {
        self.alignment = alignment;
        self
    }

    /// Dart `IndexedStack(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> IndexedStack {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `IndexedStack(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> IndexedStack {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `IndexedStack(sizing:)`.
    pub fn sizing(mut self, sizing: StackFit) -> IndexedStack {
        self.sizing = sizing;
        self
    }

    /// Dart `IndexedStack(index:)`; `None` shows no child, which the default (`Some(0)`) does
    /// not express.
    pub fn index(mut self, index: Option<i32>) -> IndexedStack {
        self.index = index;
        self
    }

    /// Dart `IndexedStack(children:)`.
    pub fn children(mut self, children: impl IntoIterator<Item = WidgetRef>) -> IndexedStack {
        self.children = children.into_iter().collect();
        self
    }
}

impl Default for IndexedStack {
    fn default() -> IndexedStack {
        IndexedStack {
            key: None,
            alignment: AlignmentGeometry::TOP_START,
            text_direction: None,
            clip_behavior: Clip::HardEdge,
            sizing: StackFit::Loose,
            index: Some(0),
            children: Vec::new(),
        }
    }
}

impl StatelessWidget for IndexedStack {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        // Each child is wrapped with a visibility scope (so `Visibility.of` reports the child
        // as hidden when it is not the selected index) and with `ExcludeFocus` (so
        // non-selected children cannot receive focus). Both of these do not introduce any
        // render objects between the child and the enclosing `RenderIndexedStack`.
        //
        // This allows parent data widgets such as `Positioned` to correctly apply their
        // `StackParentData` to the `IndexedStack`'s render object.
        //
        // Painting, hit-testing, and semantics for non-selected children are already handled
        // by `RenderIndexedStack`, so no additional render-object wrappers are needed.
        let wrapped_children = self.children.iter().enumerate().map(|(i, child)| {
            let is_selected = Some(i as i32) == self.index;
            VisibilityScope {
                is_visible: is_selected,
                child: ExcludeFocus::new(child.clone())
                    .excluding(!is_selected)
                    .into_widget(),
            }
            .into_widget()
        });
        RawIndexedStack::new(self.sizing)
            .alignment(self.alignment)
            .index(self.index)
            .children(wrapped_children)
            .clip_behavior(self.clip_behavior)
            .with_text_direction(self.text_direction)
            .into_widget()
    }
}

/// The render object widget that backs [`IndexedStack`].
///
/// Dart's `class _RawIndexedStack extends Stack`: a distinct widget type whose behaviour is
/// [`Stack`]'s over a `RenderIndexedStack`, with the fit taken from `sizing`.
#[derive(Debug)]
struct RawIndexedStack {
    key: Option<KeyRef>,
    /// See [`Stack::alignment`].
    alignment: AlignmentGeometry,
    /// See [`Stack::text_direction`].
    text_direction: Option<TextDirection>,
    /// See [`Stack::clip_behavior`].
    clip_behavior: Clip,
    /// See [`Stack::fit`]; Dart's `super(fit: sizing)`.
    fit: StackFit,
    /// The index of the child to show.
    index: Option<i32>,
    children: Vec<WidgetRef>,
}

impl RawIndexedStack {
    fn new(sizing: StackFit) -> RawIndexedStack {
        let stack = Stack::new();
        RawIndexedStack {
            key: None,
            alignment: stack.alignment,
            text_direction: stack.text_direction,
            clip_behavior: stack.clip_behavior,
            fit: sizing,
            index: Some(0),
            children: Vec::new(),
        }
    }

    fn alignment(mut self, alignment: AlignmentGeometry) -> RawIndexedStack {
        self.alignment = alignment;
        self
    }

    fn with_text_direction(mut self, text_direction: Option<TextDirection>) -> RawIndexedStack {
        self.text_direction = text_direction;
        self
    }

    fn clip_behavior(mut self, clip_behavior: Clip) -> RawIndexedStack {
        self.clip_behavior = clip_behavior;
        self
    }

    fn index(mut self, index: Option<i32>) -> RawIndexedStack {
        self.index = index;
        self
    }

    fn children(mut self, children: impl IntoIterator<Item = WidgetRef>) -> RawIndexedStack {
        self.children = children.into_iter().collect();
        self
    }

    /// The constructor assert: the index must be `None` or within the range of children.
    fn debug_check_index(&self) {
        debug_assert!(
            self.index.is_none_or(|index| {
                (index == 0 && self.children.is_empty())
                    || (index >= 0 && (index as usize) < self.children.len())
            }),
            "The index must be null or within the range of children."
        );
    }
}

impl RenderObjectWidget for RawIndexedStack {
    type RenderObject = RenderIndexedStack;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        self.debug_check_index();
        let render_object = RenderIndexedStack::new(app);
        render_object.set_index(app, self.index);
        render_object.set_fit(app, self.fit);
        render_object.set_clip_behavior(app, self.clip_behavior);
        render_object.set_alignment(app, self.alignment);
        let text_direction = Stack::resolved_text_direction(self.text_direction, app, context);
        render_object.set_text_direction(app, text_direction);
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderIndexedStack>,
    ) {
        self.debug_check_index();
        render_object.set_index(app, self.index);
        render_object.set_fit(app, self.fit);
        render_object.set_clip_behavior(app, self.clip_behavior);
        render_object.set_alignment(app, self.alignment);
        let text_direction = Stack::resolved_text_direction(self.text_direction, app, context);
        render_object.set_text_direction(app, text_direction);
    }
}

impl MultiChildRenderObjectWidget for RawIndexedStack {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

/// Inherited widget that allows descendants to find their visibility status.
///
/// Used by [`Visibility`](crate::Visibility) and [`IndexedStack`] to propagate visibility
/// information to descendants (Dart's `_VisibilityScope`).
#[derive(Debug)]
pub(crate) struct VisibilityScope {
    pub(crate) is_visible: bool,
    pub(crate) child: WidgetRef,
}

impl InheritedWidget for VisibilityScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &VisibilityScope) -> bool {
        self.is_visible != old_widget.is_visible
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::cell::Cell;
    use std::rc::Rc;

    use reveal_embedder::{Color, Offset, Radius, Size};
    use reveal_foundation::{App, AppCell, Handle};
    use reveal_gestures::{HitTestEntry, HitTestResult, PointerDownEvent, PointerEvent};
    use reveal_rendering::{
        AnyRenderBox, BoxHitTestEntry, BoxHitTestResult, BoxParentData, ContainerBoxParentData,
        ContainerRenderObjectMixin, ErasedRenderObject, RenderBox, RenderObject,
        RenderObjectWithChildMixin,
    };
    use reveal_services::SystemMouseCursors;

    use super::*;
    use crate::framework::{
        GlobalKey, IntoWidget, State, StateData, StatefulWidget, downcast_widget,
    };
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
    fn listener_receives_a_pointer_down_through_its_render_object() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
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
    // ---- multi-child widgets ----

    /// The children of a container render object, in list order.
    fn container_children<T: ContainerRenderObjectMixin<ChildType = AnyRenderBox>>(
        parent: RenderHandle<T>,
        app: &App,
    ) -> Vec<AnyRenderBox> {
        let mut children = Vec::new();
        let mut child = parent.first_child(app);
        while let Some(current) = child {
            children.push(current);
            child = parent.child_after(app, current);
        }
        children
    }

    /// The offset a flex gave one of its children.
    fn flex_offset(child: AnyRenderBox, app: &App) -> Offset {
        child
            .as_object()
            .parent_data_of::<FlexParentData>(app)
            .offset()
    }

    /// The offset a stack gave one of its children.
    fn stack_offset(child: AnyRenderBox, app: &App) -> Offset {
        child
            .as_object()
            .parent_data_of::<StackParentData>(app)
            .offset()
    }

    /// The boxes a hit test walked, innermost first.
    fn hit_boxes(result: &HitTestResult) -> Vec<AnyRenderBox> {
        result
            .path()
            .iter()
            .filter_map(|entry: &HitTestEntry| {
                let target: &dyn Any = entry.target();
                target
                    .downcast_ref::<BoxHitTestEntry>()
                    .map(BoxHitTestEntry::target)
            })
            .collect()
    }

    fn ltr(child: WidgetRef) -> WidgetRef {
        Directionality::new(TextDirection::Ltr, child).into_widget()
    }

    /// A box that answers a hit test, so a hit can tell which child was reached.
    fn opaque(width: f64, height: f64) -> WidgetRef {
        Listener {
            behavior: HitTestBehavior::Opaque,
            child: Some(sized(width, height)),
            ..Default::default()
        }
        .into_widget()
    }

    /// The `TileState` a global key names, through the harness's own build owner.
    fn tile_state(harness: &Harness, app: &App, key: &GlobalKey) -> Option<Handle<TileState>> {
        harness
            .owner
            .global_key_element(app, key.identity())
            .and_then(|element| element.state_handle::<TileState>(app))
    }

    /// A stateful leaf that counts its builds, so a move can be told from a rebirth.
    #[derive(Debug)]
    struct Tile {
        key: Option<KeyRef>,
        width: f64,
    }

    impl Tile {
        fn keyed(key: &GlobalKey, width: f64) -> WidgetRef {
            Tile {
                key: Some(Rc::new(key.clone())),
                width,
            }
            .into_widget()
        }
    }

    impl StatefulWidget for Tile {
        type State = TileState;

        fn key(&self) -> Option<&KeyRef> {
            self.key.as_ref()
        }

        fn create_state(&self) -> TileState {
            TileState {
                state: StateData::new(),
                builds: 0,
            }
        }
    }

    struct TileState {
        state: StateData<Tile>,
        builds: u32,
    }

    impl State for TileState {
        type Widget = Tile;
        crate::state_accessors!();

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            app.get_mut(self).builds += 1;
            let width = self.widget(app).width;
            sized(width, 20.0)
        }
    }

    #[test]
    fn row_lays_out_fixed_and_expanded_children() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            ltr(Row::new()
                .children([
                    sized(50.0, 20.0),
                    Expanded::new(sized(10.0, 20.0)).into_widget(),
                    sized(30.0, 20.0),
                ])
                .into_widget()),
        );
        harness.pump(&mut app);

        let row = render_object_under_root::<RenderFlex>(&harness, &app);
        assert_eq!(row.size(&app), Size::new(300.0, 20.0), "mainAxisSize.max");
        let children = container_children(row, &app);
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].size(&app), Size::new(50.0, 20.0));
        assert_eq!(
            children[1].size(&app),
            Size::new(220.0, 20.0),
            "the expanded child takes the free space, tightly"
        );
        assert_eq!(children[2].size(&app), Size::new(30.0, 20.0));
        assert_eq!(flex_offset(children[0], &app), Offset::new(0.0, 0.0));
        assert_eq!(flex_offset(children[1], &app), Offset::new(50.0, 0.0));
        assert_eq!(flex_offset(children[2], &app), Offset::new(270.0, 0.0));
    }

    #[test]
    fn a_loose_flexible_child_keeps_its_own_size() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            ltr(Row::new()
                .children([
                    sized(50.0, 20.0),
                    Flexible::new(sized(10.0, 20.0))
                        .fit(FlexFit::Loose)
                        .into_widget(),
                ])
                .into_widget()),
        );
        harness.pump(&mut app);

        let row = render_object_under_root::<RenderFlex>(&harness, &app);
        let children = container_children(row, &app);
        assert_eq!(
            children[1].size(&app),
            Size::new(10.0, 20.0),
            "FlexFit::Loose only offers the space, it does not force it"
        );
        assert_eq!(flex_offset(children[1], &app), Offset::new(50.0, 0.0));

        harness.set_child(
            &mut app,
            ltr(Row::new()
                .children([
                    sized(50.0, 20.0),
                    Flexible::new(sized(10.0, 20.0))
                        .fit(FlexFit::Tight)
                        .into_widget(),
                ])
                .into_widget()),
        );
        harness.pump(&mut app);
        let children = container_children(row, &app);
        assert_eq!(
            children[1].size(&app),
            Size::new(250.0, 20.0),
            "FlexFit::Tight fills the offered space"
        );
    }

    #[test]
    fn a_column_space_between_spreads_its_children() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            Column::new()
                .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                .children([sized(10.0, 20.0), sized(10.0, 30.0)])
                .into_widget(),
        );
        harness.pump(&mut app);

        let column = render_object_under_root::<RenderFlex>(&harness, &app);
        assert_eq!(
            column.text_direction(&app),
            None,
            "a centred column needs no text direction"
        );
        assert_eq!(column.size(&app), Size::new(10.0, 200.0));
        let children = container_children(column, &app);
        assert_eq!(flex_offset(children[0], &app), Offset::new(0.0, 0.0));
        assert_eq!(flex_offset(children[1], &app), Offset::new(0.0, 170.0));
    }

    #[test]
    fn reordering_keyed_children_moves_elements_and_render_children() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (first, second, third) = (GlobalKey::new(), GlobalKey::new(), GlobalKey::new());
        let row = |order: [&GlobalKey; 3]| {
            let widths = [10.0, 20.0, 30.0];
            let by_key = |key: &GlobalKey| {
                let index = [&first, &second, &third]
                    .iter()
                    .position(|candidate| candidate.identity() == key.identity())
                    .expect("one of the three keys");
                Tile::keyed(key, widths[index])
            };
            ltr(Row::new().children(order.map(by_key)).into_widget())
        };

        let harness = Harness::mount(&mut app, row([&first, &second, &third]));
        harness.pump(&mut app);
        let flex = render_object_under_root::<RenderFlex>(&harness, &app);
        let before = container_children(flex, &app);
        assert_eq!(
            before
                .iter()
                .map(|child| child.size(&app).width())
                .collect::<Vec<f64>>(),
            vec![10.0, 20.0, 30.0]
        );
        let state = tile_state(&harness, &app, &first).expect("the first tile's state");
        assert_eq!(app.get(state).builds, 1);

        harness.set_child(&mut app, row([&third, &first, &second]));
        harness.pump(&mut app);

        assert_eq!(
            tile_state(&harness, &app, &first),
            Some(state),
            "the moved element keeps its State"
        );
        assert_eq!(
            app.get(state).builds,
            2,
            "it was rebuilt in place, not created afresh"
        );
        let after = container_children(flex, &app);
        assert_eq!(
            after,
            vec![before[2], before[0], before[1]],
            "the same render objects, in the new order"
        );
        assert_eq!(flex_offset(after[0], &app), Offset::new(0.0, 0.0));
        assert_eq!(flex_offset(after[1], &app), Offset::new(30.0, 0.0));
        assert_eq!(flex_offset(after[2], &app), Offset::new(40.0, 0.0));
    }

    #[test]
    fn a_child_can_be_added_and_removed_in_the_middle() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (first, middle, last) = (GlobalKey::new(), GlobalKey::new(), GlobalKey::new());
        let ends = || {
            ltr(Row::new()
                .children([Tile::keyed(&first, 10.0), Tile::keyed(&last, 30.0)])
                .into_widget())
        };
        let all_three = || {
            ltr(Row::new()
                .children([
                    Tile::keyed(&first, 10.0),
                    Tile::keyed(&middle, 20.0),
                    Tile::keyed(&last, 30.0),
                ])
                .into_widget())
        };

        let harness = Harness::mount(&mut app, ends());
        harness.pump(&mut app);
        let flex = render_object_under_root::<RenderFlex>(&harness, &app);
        let before = container_children(flex, &app);
        assert_eq!(before.len(), 2);

        harness.set_child(&mut app, all_three());
        harness.pump(&mut app);
        let inserted = container_children(flex, &app);
        assert_eq!(inserted.len(), 3);
        assert_eq!(inserted[0], before[0], "the first child was kept");
        assert_eq!(inserted[2], before[1], "the last child was kept");
        assert_eq!(inserted[1].size(&app).width(), 20.0);
        assert_eq!(flex_offset(inserted[2], &app), Offset::new(30.0, 0.0));

        harness.set_child(&mut app, ends());
        harness.pump(&mut app);
        let removed = container_children(flex, &app);
        assert_eq!(removed, vec![before[0], before[1]]);
        assert_eq!(flex_offset(removed[1], &app), Offset::new(10.0, 0.0));
        assert!(
            tile_state(&harness, &app, &middle).is_none(),
            "the removed child was unmounted"
        );
    }

    #[test]
    fn a_stack_sizes_its_non_positioned_child_by_its_fit() {
        // Loose leaves the child alone, Expand forces the biggest size, Passthrough hands the
        // stack's own constraints down (whose minimum is larger than the child).
        for (fit, child_size, stack_size) in [
            (StackFit::Loose, Size::new(10.0, 5.0), Size::new(20.0, 10.0)),
            (
                StackFit::Expand,
                Size::new(100.0, 80.0),
                Size::new(100.0, 80.0),
            ),
            (
                StackFit::Passthrough,
                Size::new(20.0, 10.0),
                Size::new(20.0, 10.0),
            ),
        ] {
            let cell = AppCell::new();
            let mut app = cell.borrow_mut();
            let harness = Harness::mount(
                &mut app,
                ltr(ConstrainedBox::new(
                    BoxConstraints::new()
                        .min_width(20.0)
                        .max_width(100.0)
                        .min_height(10.0)
                        .max_height(80.0),
                )
                .child(Stack::new().fit(fit).children([
                    sized(10.0, 5.0),
                    Positioned::fill(sized(1.0, 1.0)).into_widget(),
                ]))
                .into_widget()),
            );
            harness.pump(&mut app);

            let constrained = render_object_under_root::<RenderConstrainedBox>(&harness, &app);
            let stack: RenderHandle<RenderStack> = constrained
                .child(&app)
                .expect("a child")
                .as_object()
                .downcast(&app)
                .expect("a RenderStack");
            assert_eq!(stack.size(&app), stack_size, "{fit:?}");
            let children = container_children(stack, &app);
            assert_eq!(children[0].size(&app), child_size, "{fit:?}");
            assert_eq!(stack_offset(children[0], &app), Offset::ZERO, "{fit:?}");
            assert_eq!(
                children[1].size(&app),
                stack_size,
                "Positioned.fill fills the stack ({fit:?})"
            );
            assert_eq!(stack_offset(children[1], &app), Offset::ZERO, "{fit:?}");
        }
    }

    #[test]
    fn an_indexed_stack_lays_out_every_child_but_shows_one() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let stack = |index: i32| {
            ltr(IndexedStack::new()
                .index(Some(index))
                .children([opaque(40.0, 30.0), opaque(20.0, 50.0)])
                .into_widget())
        };
        let harness = Harness::mount(&mut app, stack(1));
        harness.pump(&mut app);

        let indexed = render_object_under_root::<RenderIndexedStack>(&harness, &app);
        assert_eq!(indexed.index(&app), Some(1));
        let children = container_children(indexed, &app);
        assert_eq!(children.len(), 2, "every child is laid out");
        assert_eq!(children[0].size(&app), Size::new(40.0, 30.0));
        assert_eq!(children[1].size(&app), Size::new(20.0, 50.0));
        assert_eq!(
            indexed.size(&app),
            Size::new(40.0, 50.0),
            "as big as the largest child"
        );

        // Only the child at the index answers a hit test: (30, 10) is inside the first child
        // and outside the second.
        let mut result = HitTestResult::new();
        assert!(!indexed.hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            Offset::new(30.0, 10.0)
        ));
        let mut result = HitTestResult::new();
        assert!(indexed.hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            Offset::new(10.0, 10.0)
        ));
        assert_eq!(hit_boxes(&result), vec![children[1], indexed.as_box()]);

        harness.set_child(&mut app, stack(0));
        harness.pump(&mut app);
        assert_eq!(indexed.index(&app), Some(0));
        let mut result = HitTestResult::new();
        assert!(indexed.hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            Offset::new(30.0, 10.0)
        ));
        assert_eq!(hit_boxes(&result), vec![children[0], indexed.as_box()]);
    }

    #[test]
    fn clip_rect_and_clip_rrect_configure_their_render_objects() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let clipped = |behavior: Clip| {
            ClipRect::new()
                .clip_behavior(behavior)
                .child(sized(10.0, 10.0))
                .into_widget()
        };
        let harness = Harness::mount(&mut app, clipped(Clip::HardEdge));
        harness.pump(&mut app);
        let clip = render_object_under_root::<RenderClipRect>(&harness, &app);
        assert_eq!(clip.clip_behavior(&app), Clip::HardEdge, "Dart's default");
        assert!(clip.clipper(&app).is_none());

        harness.set_child(&mut app, clipped(Clip::AntiAliasWithSaveLayer));
        harness.pump(&mut app);
        assert_eq!(
            render_object_under_root::<RenderClipRect>(&harness, &app),
            clip,
            "the same RenderClipRect is reconfigured"
        );
        assert_eq!(clip.clip_behavior(&app), Clip::AntiAliasWithSaveLayer);
        assert_eq!(clip.size(&app), Size::new(10.0, 10.0), "sized to its child");
    }

    #[test]
    fn clip_rrect_resolves_its_radius_through_directionality() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let corner = Radius::circular(4.0);
        let radius = BorderRadiusGeometry::directional(corner, corner, corner, corner);
        let rounded = |text_direction: TextDirection| {
            Directionality::new(
                text_direction,
                ClipRRect::new()
                    .border_radius(radius)
                    .child(sized(10.0, 10.0)),
            )
            .into_widget()
        };
        let harness = Harness::mount(&mut app, rounded(TextDirection::Rtl));
        harness.pump(&mut app);
        let clip = render_object_under_root::<RenderClipRRect>(&harness, &app);
        assert_eq!(clip.clip_behavior(&app), Clip::AntiAlias, "Dart's default");
        assert_eq!(clip.border_radius(&app), radius);
        assert_eq!(clip.text_direction(&app), Some(TextDirection::Rtl));

        harness.set_child(&mut app, rounded(TextDirection::Ltr));
        harness.pump(&mut app);
        assert_eq!(clip.text_direction(&app), Some(TextDirection::Ltr));
    }

    #[test]
    fn transform_named_constructors_match_dart() {
        assert_eq!(Transform::rotate(0.0).transform, Matrix4::IDENTITY);
        assert_eq!(
            Transform::rotate(std::f64::consts::FRAC_PI_2).transform,
            Matrix4::from_flutter_array(&[
                0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ]),
            "a quarter turn is exact"
        );
        assert_eq!(
            Transform::rotate(0.0).alignment,
            Some(AlignmentGeometry::CENTER)
        );

        let translated = Transform::translate(Offset::new(3.0, 4.0));
        assert_eq!(translated.transform, Matrix4::translation(3.0, 4.0));
        assert_eq!(translated.alignment, None, "Dart leaves it unset");
        assert_eq!(translated.origin, None);

        assert_eq!(
            Transform::scale(Some(0.5), None, None).transform,
            Matrix4::scale(0.5, 0.5)
        );
        assert_eq!(
            Transform::scale(None, Some(2.0), None).transform,
            Matrix4::scale(2.0, 1.0),
            "the unset axis stays at 1.0"
        );
        assert_eq!(
            Transform::flip(true, false).transform,
            Matrix4::scale(-1.0, 1.0)
        );
        assert_eq!(
            Transform::flip(true, false).alignment,
            Some(AlignmentGeometry::CENTER)
        );
    }

    #[test]
    fn transform_moves_hit_tests_with_its_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let translated = |transform_hit_tests: bool| {
            Transform::translate(Offset::new(20.0, 0.0))
                .transform_hit_tests(transform_hit_tests)
                .child(opaque(10.0, 10.0))
                .into_widget()
        };
        let harness = Harness::mount(&mut app, translated(true));
        harness.pump(&mut app);
        let transform = render_object_under_root::<RenderTransform>(&harness, &app);
        assert!(transform.transform_hit_tests(&app));

        let mut result = HitTestResult::new();
        assert!(
            transform.hit_test(
                &mut app,
                &mut BoxHitTestResult::wrap(&mut result),
                Offset::new(25.0, 5.0),
            ),
            "the child is hit where it is painted"
        );
        let mut result = HitTestResult::new();
        assert!(!transform.hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            Offset::new(5.0, 5.0),
        ));

        harness.set_child(&mut app, translated(false));
        harness.pump(&mut app);
        assert!(!transform.transform_hit_tests(&app));
        let mut result = HitTestResult::new();
        assert!(
            transform.hit_test(
                &mut app,
                &mut BoxHitTestResult::wrap(&mut result),
                Offset::new(5.0, 5.0),
            ),
            "the child is hit where it was laid out"
        );
    }

    #[test]
    fn fractional_translation_forwards_its_configuration() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            FractionalTranslation::new(Offset::new(0.25, 0.0))
                .transform_hit_tests(false)
                .child(sized(40.0, 10.0))
                .into_widget(),
        );
        harness.pump(&mut app);
        let translation = render_object_under_root::<RenderFractionalTranslation>(&harness, &app);
        assert_eq!(translation.translation(&app), Offset::new(0.25, 0.0));
        assert!(!translation.transform_hit_tests(&app));

        harness.set_child(
            &mut app,
            FractionalTranslation::new(Offset::new(0.5, 0.5))
                .child(sized(40.0, 10.0))
                .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(translation.translation(&app), Offset::new(0.5, 0.5));
        assert!(
            translation.transform_hit_tests(&app),
            "back to Dart's default"
        );
    }

    #[test]
    fn ignore_pointer_and_absorb_pointer_stop_hit_tests_differently() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            IgnorePointer::new().child(opaque(10.0, 10.0)).into_widget(),
        );
        harness.pump(&mut app);
        let ignoring = render_object_under_root::<RenderIgnorePointer>(&harness, &app);
        assert!(ignoring.ignoring(&app), "Dart's default");
        let mut result = HitTestResult::new();
        assert!(!ignoring.hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            Offset::new(5.0, 5.0),
        ));
        assert!(hit_boxes(&result).is_empty());

        harness.set_child(
            &mut app,
            IgnorePointer::new()
                .ignoring(false)
                .child(opaque(10.0, 10.0))
                .into_widget(),
        );
        harness.pump(&mut app);
        assert!(!ignoring.ignoring(&app));
        let mut result = HitTestResult::new();
        assert!(ignoring.hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            Offset::new(5.0, 5.0),
        ));
        assert!(!hit_boxes(&result).is_empty(), "the child answers again");

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            AbsorbPointer::new().child(opaque(10.0, 10.0)).into_widget(),
        );
        harness.pump(&mut app);
        let absorbing = render_object_under_root::<RenderAbsorbPointer>(&harness, &app);
        assert!(absorbing.absorbing(&app), "Dart's default");
        let mut result = HitTestResult::new();
        assert!(
            absorbing.hit_test(
                &mut app,
                &mut BoxHitTestResult::wrap(&mut result),
                Offset::new(5.0, 5.0),
            ),
            "absorbing itself answers the hit"
        );
        assert!(
            hit_boxes(&result).is_empty(),
            "the subtree is not reached, and nothing is added to the path"
        );
    }

    #[test]
    fn limited_box_caps_an_unbounded_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let limited = |max_width: f64| {
            ltr(Row::new()
                .children([LimitedBox::new()
                    .max_width(max_width)
                    .child(SizedBox::expand())
                    .into_widget()])
                .into_widget())
        };
        let harness = Harness::mount(&mut app, limited(50.0));
        harness.pump(&mut app);
        let flex = render_object_under_root::<RenderFlex>(&harness, &app);
        let child = container_children(flex, &app)[0]
            .as_object()
            .downcast::<RenderLimitedBox>(&app)
            .expect("a RenderLimitedBox");
        assert_eq!(child.max_width(&app), 50.0);
        assert_eq!(child.max_height(&app), f64::INFINITY, "Dart's default");
        assert_eq!(
            child.size(&app).width(),
            50.0,
            "the row's unbounded width is capped"
        );

        harness.set_child(&mut app, limited(30.0));
        harness.pump(&mut app);
        assert_eq!(child.max_width(&app), 30.0);
        assert_eq!(child.size(&app).width(), 30.0);
    }

    #[test]
    fn colored_box_forwards_its_color_and_anti_aliasing() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let red = Color::from_argb(255, 255, 0, 0);
        let blue = Color::from_argb(255, 0, 0, 255);
        let harness = Harness::mount(
            &mut app,
            ColoredBox::new(red).child(sized(10.0, 10.0)).into_widget(),
        );
        harness.pump(&mut app);
        let colored = render_object_under_root::<RenderColoredBox>(&harness, &app);
        assert_eq!(colored.color(&app), red);
        assert!(colored.is_anti_alias(&app), "Dart's default");

        harness.set_child(
            &mut app,
            ColoredBox::new(blue)
                .is_anti_alias(false)
                .child(sized(10.0, 10.0))
                .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(colored.color(&app), blue, "the same box is reconfigured");
        assert!(!colored.is_anti_alias(&app));
    }

    #[test]
    fn offstage_takes_no_room_and_hides_its_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let staged = |offstage: bool| {
            Offstage::new()
                .offstage(offstage)
                .child(opaque(10.0, 10.0))
                .into_widget()
        };
        let harness = Harness::mount(&mut app, staged(true));
        harness.pump(&mut app);
        let offstage = render_object_under_root::<RenderOffstage>(&harness, &app);
        assert!(offstage.offstage(&app), "Dart's default");
        assert_eq!(offstage.size(&app), Size::ZERO, "no room in the parent");
        let child = offstage.child(&app).expect("laid out as if in the tree");
        assert_eq!(child.size(&app), Size::new(10.0, 10.0));
        assert!(!offstage.paints_child(&app, child));

        harness.set_child(&mut app, staged(false));
        harness.pump(&mut app);
        assert_eq!(offstage.size(&app), Size::new(10.0, 10.0));
        assert!(offstage.paints_child(&app, child));
    }

    /// Counts its paint calls and records the size it was handed.
    #[derive(Debug)]
    struct RecordingPainter {
        paints: Rc<Cell<u32>>,
        painted: Rc<Cell<Size>>,
    }

    impl CustomPainter for RecordingPainter {
        fn paint(&self, _app: &mut App, _canvas: &mut reveal_embedder::Canvas, size: Size) {
            self.paints.set(self.paints.get() + 1);
            self.painted.set(size);
        }

        fn should_repaint(&self, _app: &App, _old_delegate: &dyn CustomPainter) -> bool {
            true
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn custom_paint_sizes_itself_and_runs_its_painter() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let paints = Rc::new(Cell::new(0));
        let painted = Rc::new(Cell::new(Size::ZERO));
        let painter = || RecordingPainter {
            paints: Rc::clone(&paints),
            painted: Rc::clone(&painted),
        };
        let harness = Harness::mount(
            &mut app,
            CustomPaint::new()
                .painter(painter())
                .size(Size::new(40.0, 30.0))
                .into_widget(),
        );
        harness.pump(&mut app);
        let custom = render_object_under_root::<RenderCustomPaint>(&harness, &app);
        assert_eq!(
            custom.size(&app),
            Size::new(40.0, 30.0),
            "the preferred size, with no child"
        );
        assert_eq!(paints.get(), 1);
        assert_eq!(painted.get(), Size::new(40.0, 30.0));
        assert!(custom.foreground_painter(&app).is_none());

        harness.set_child(
            &mut app,
            CustomPaint::new()
                .foreground_painter(painter())
                .child(sized(10.0, 10.0))
                .into_widget(),
        );
        harness.pump(&mut app);
        assert!(custom.painter(&app).is_none(), "the painter is cleared");
        assert!(custom.foreground_painter(&app).is_some());
        assert_eq!(
            custom.size(&app),
            Size::new(10.0, 10.0),
            "the child's size wins"
        );
        assert_eq!(paints.get(), 2);
        assert_eq!(painted.get(), Size::new(10.0, 10.0));
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "Incorrect use of ParentDataWidget")]
    fn a_flexible_under_a_non_flex_parent_asserts() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            Padding::new(EdgeInsetsGeometry::all(1.0))
                .child(Flexible::new(sized(10.0, 10.0)))
                .into_widget(),
        );
        harness.pump(&mut app);
    }

    #[test]
    fn meta_data_reaches_its_render_object_and_updates() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            MetaData::new()
                .meta_data(Rc::new(String::from("tag")))
                .behavior(HitTestBehavior::Opaque)
                .child(SizedBox::shrink())
                .into_widget(),
        );
        harness.pump(&mut app);
        let render_object = render_object_under_root::<RenderMetaData>(&harness, &app);
        assert_eq!(
            render_object
                .meta_data(&app)
                .and_then(|data| data.downcast::<String>().ok())
                .as_deref(),
            Some(&String::from("tag"))
        );
        harness.set_child(
            &mut app,
            MetaData::new().child(SizedBox::shrink()).into_widget(),
        );
        harness.pump(&mut app);
        assert!(render_object.meta_data(&app).is_none());
    }

    #[test]
    fn a_backdrop_filter_reaches_its_render_object_and_a_grouped_one_shares_the_group_key() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            BackdropFilter::filter(ImageFilter::blur(3.0, 3.0))
                .child(SizedBox::shrink())
                .into_widget(),
        );
        harness.pump(&mut app);
        let render_object = render_object_under_root::<RenderBackdropFilter>(&harness, &app);
        assert_eq!(render_object.filter(&app), ImageFilter::blur(3.0, 3.0));
        assert!(render_object.backdrop_key(&app).is_none());

        let key = BackdropKey::new();
        harness.set_child(
            &mut app,
            BackdropGroup::new(
                BackdropFilter::grouped(ImageFilterConfig::blur().sigma_x(2.0).sigma_y(2.0))
                    .child(SizedBox::shrink()),
            )
            .backdrop_key(key)
            .into_widget(),
        );
        harness.pump(&mut app);
        let inherited = harness
            .render_root(&app)
            .child(&app)
            .expect("the filter under the group")
            .as_object()
            .downcast::<RenderBackdropFilter>(&app)
            .expect("a RenderBackdropFilter");
        assert_eq!(inherited.backdrop_key(&app), Some(key));
    }
    /// `SliverPadding` insets its sliver child on every side: the padded sliver's scroll extent
    /// grows by the main-axis insets, and the child is narrowed by the cross-axis ones.
    #[test]
    fn a_sliver_padding_insets_its_child_on_every_side() {
        use reveal_embedder::TextDirection;
        use reveal_rendering::{
            ContainerRenderObjectMixin, FixedViewportOffset, RenderSliver, RenderSliverPadding,
            RenderSliverToBoxAdapter, RenderViewport, ScrollCacheExtent, ViewportOffset,
        };

        use crate::widgets::viewport::Viewport;

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let offset = FixedViewportOffset::zero(&mut app);
        let tree = Directionality::new(
            TextDirection::Ltr,
            Viewport::new(offset.as_viewport_offset())
                .scroll_cache_extent(ScrollCacheExtent::Pixels(0.0))
                .slivers(vec![
                    SliverPadding::new(EdgeInsetsGeometry::only(10.0, 20.0, 30.0, 40.0))
                        .sliver(SliverToBoxAdapter::new().child(SizedBox::new().height(50.0)))
                        .into_widget(),
                ]),
        )
        .into_widget();
        let harness = Harness::mount(&mut app, tree);
        harness.pump(&mut app);

        let padding = render_object_under_root::<RenderViewport>(&harness, &app)
            .first_child(&app)
            .expect("the padding")
            .as_object()
            .downcast::<RenderSliverPadding>(&app)
            .expect("a RenderSliverPadding");
        // 20 above and 40 below the 50-pixel child.
        assert_eq!(padding.geometry(&app).scroll_extent, 110.0);
        let adapter = padding
            .child(&app)
            .expect("the adapter")
            .as_object()
            .downcast::<RenderSliverToBoxAdapter>(&app)
            .expect("a RenderSliverToBoxAdapter");
        // 10 on the left and 30 on the right of the 300-pixel viewport.
        assert_eq!(
            adapter.child(&app).expect("the box").size(&app).width(),
            260.0
        );
    }

    #[test]
    fn fitted_box_scales_its_child_to_the_space_it_is_given() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            FittedBox::new().child(sized(600.0, 400.0)).into_widget(),
        );
        harness.pump(&mut app);
        let fitted = render_object_under_root::<RenderFittedBox>(&harness, &app);
        assert_eq!(
            fitted.size(&app),
            Size::new(300.0, 200.0),
            "the 3:2 child fills the 300x200 root"
        );
        let child = fitted.child(&app).expect("a child");
        assert_eq!(
            child.size(&app),
            Size::new(600.0, 400.0),
            "the child lays out unconstrained"
        );
        assert_eq!(
            child.local_to_global(&app, Offset::new(600.0, 400.0), Some(fitted.as_object())),
            Offset::new(300.0, 200.0),
            "half scale"
        );
    }

    #[test]
    fn constraints_transform_box_transforms_are_dart_s() {
        let constraints = BoxConstraints::new()
            .min_width(10.0)
            .max_width(20.0)
            .min_height(30.0)
            .max_height(40.0);
        assert_eq!(
            ConstraintsTransformBox::unmodified(constraints),
            constraints
        );
        assert_eq!(
            ConstraintsTransformBox::unconstrained(constraints),
            BoxConstraints::new()
        );
        assert_eq!(
            ConstraintsTransformBox::width_unconstrained(constraints),
            constraints.height_constraints()
        );
        assert_eq!(
            ConstraintsTransformBox::height_unconstrained(constraints),
            constraints.width_constraints()
        );
        assert_eq!(
            ConstraintsTransformBox::max_height_unconstrained(constraints),
            constraints.copy_with().max_height(f64::INFINITY)
        );
        assert_eq!(
            ConstraintsTransformBox::max_width_unconstrained(constraints),
            constraints.copy_with().max_width(f64::INFINITY)
        );
        assert_eq!(
            ConstraintsTransformBox::max_unconstrained(constraints),
            constraints
                .copy_with()
                .max_width(f64::INFINITY)
                .max_height(f64::INFINITY)
        );
    }

    #[test]
    fn unconstrained_box_lets_its_child_size_itself_under_a_tight_parent() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tight = |child: WidgetRef| {
            SizedBox::new()
                .width(50.0)
                .height(50.0)
                .child(child)
                .into_widget()
        };
        let harness = Harness::mount(
            &mut app,
            tight(
                UnconstrainedBox::new()
                    .child(sized(120.0, 30.0))
                    .into_widget(),
            ),
        );
        harness.pump(&mut app);
        let unconstrained = render_object_under_root::<RenderConstrainedBox>(&harness, &app)
            .child(&app)
            .expect("the transform box")
            .as_object()
            .downcast::<RenderConstraintsTransformBox>(&app)
            .expect("a RenderConstraintsTransformBox");
        assert_eq!(
            unconstrained.size(&app),
            Size::new(50.0, 50.0),
            "the box still obeys the tight parent"
        );
        assert_eq!(
            unconstrained.child(&app).expect("a child").size(&app),
            Size::new(120.0, 30.0),
            "the child is laid out unconstrained"
        );

        assert_eq!(
            unconstrained
                .child(&app)
                .expect("a child")
                .constraints(&app),
            BoxConstraints::new(),
            "no axis keeps its constraints"
        );

        harness.set_child(
            &mut app,
            tight(
                UnconstrainedBox::new()
                    .constrained_axis(Axis::Horizontal)
                    .child(sized(120.0, 30.0))
                    .into_widget(),
            ),
        );
        harness.pump(&mut app);
        let horizontal = render_object_under_root::<RenderConstrainedBox>(&harness, &app)
            .child(&app)
            .expect("the transform box")
            .as_object()
            .downcast::<RenderConstraintsTransformBox>(&app)
            .expect("a RenderConstraintsTransformBox");
        assert_eq!(
            horizontal.child(&app).expect("a child").constraints(&app),
            BoxConstraints::new().min_width(50.0).max_width(50.0),
            "the horizontal constraints are retained"
        );
        assert_eq!(
            horizontal.child(&app).expect("a child").size(&app),
            Size::new(50.0, 30.0)
        );
    }

    #[test]
    fn fractionally_sized_box_tightens_its_child_to_a_fraction() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            FractionallySizedBox::new()
                .width_factor(0.5)
                .height_factor(0.25)
                .child(SizedBox::expand())
                .into_widget(),
        );
        harness.pump(&mut app);
        let fraction =
            render_object_under_root::<RenderFractionallySizedOverflowBox>(&harness, &app);
        assert_eq!(
            fraction.child(&app).expect("a child").size(&app),
            Size::new(150.0, 50.0),
            "half the width and a quarter of the height of the root"
        );
        assert_eq!(fraction.size(&app), Size::new(150.0, 50.0));
    }

    #[test]
    fn overflow_box_gives_its_child_its_own_constraints() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            OverflowBox::new()
                .min_width(400.0)
                .max_width(400.0)
                .min_height(300.0)
                .max_height(300.0)
                .child(SizedBox::expand())
                .into_widget(),
        );
        harness.pump(&mut app);
        let overflow = render_object_under_root::<RenderConstrainedOverflowBox>(&harness, &app);
        assert_eq!(
            overflow.size(&app),
            Size::new(300.0, 200.0),
            "the box is as large as the root allows"
        );
        assert_eq!(
            overflow.child(&app).expect("a child").size(&app),
            Size::new(400.0, 300.0),
            "the child overflows"
        );
        assert_eq!(child_offset(overflow, &app), Offset::new(-50.0, -50.0));

        harness.set_child(
            &mut app,
            OverflowBox::new()
                .fit(OverflowBoxFit::DeferToChild)
                .max_width(40.0)
                .max_height(20.0)
                .child(SizedBox::expand())
                .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(
            render_object_under_root::<RenderConstrainedOverflowBox>(&harness, &app).size(&app),
            Size::new(40.0, 20.0),
            "deferring to the child takes the child's size"
        );
    }

    #[test]
    fn sized_overflow_box_takes_its_requested_size() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            Directionality::new(
                TextDirection::Ltr,
                SizedOverflowBox::new(Size::new(40.0, 20.0)).child(sized(100.0, 60.0)),
            )
            .into_widget(),
        );
        harness.pump(&mut app);
        let sized_overflow = render_object_under_root::<RenderSizedOverflowBox>(&harness, &app);
        assert_eq!(sized_overflow.size(&app), Size::new(40.0, 20.0));
        assert_eq!(
            sized_overflow.child(&app).expect("a child").size(&app),
            Size::new(100.0, 60.0),
            "the original constraints reach the child"
        );
    }

    #[test]
    fn aspect_ratio_picks_the_widest_size_that_keeps_the_ratio() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(&mut app, AspectRatio::new(2.0).into_widget());
        harness.pump(&mut app);
        assert_eq!(
            render_object_under_root::<RenderAspectRatio>(&harness, &app).size(&app),
            Size::new(300.0, 150.0)
        );

        harness.set_child(&mut app, AspectRatio::new(0.5).into_widget());
        harness.pump(&mut app);
        assert_eq!(
            render_object_under_root::<RenderAspectRatio>(&harness, &app).size(&app),
            Size::new(100.0, 200.0),
            "the height is clamped and the width follows from it"
        );
    }

    #[test]
    fn intrinsic_width_sizes_to_the_child_s_max_intrinsic_width() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            IntrinsicWidth::new().child(sized(40.0, 10.0)).into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(
            render_object_under_root::<RenderIntrinsicWidth>(&harness, &app).size(&app),
            Size::new(40.0, 10.0)
        );

        harness.set_child(
            &mut app,
            IntrinsicWidth::new()
                .step_width(30.0)
                .child(sized(40.0, 10.0))
                .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(
            render_object_under_root::<RenderIntrinsicWidth>(&harness, &app).size(&app),
            Size::new(60.0, 10.0),
            "the width is snapped up to a multiple of the step"
        );

        harness.set_child(
            &mut app,
            IntrinsicWidth::new()
                .step_width(0.0)
                .child(sized(40.0, 10.0))
                .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(
            render_object_under_root::<RenderIntrinsicWidth>(&harness, &app).step_width(&app),
            None,
            "Dart's `_stepWidth` reads a zero step as no step"
        );
    }

    #[test]
    fn intrinsic_height_sizes_to_the_child_s_max_intrinsic_height() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            IntrinsicHeight::new()
                .child(sized(40.0, 10.0))
                .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(
            render_object_under_root::<RenderIntrinsicHeight>(&harness, &app).size(&app),
            Size::new(40.0, 10.0)
        );
    }

    #[test]
    fn baseline_shifts_its_child_down_to_the_requested_baseline() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            Baseline::new(30.0, TextBaseline::Alphabetic)
                .child(sized(10.0, 10.0))
                .into_widget(),
        );
        harness.pump(&mut app);
        let baseline = render_object_under_root::<RenderBaseline>(&harness, &app);
        assert_eq!(
            baseline.size(&app),
            Size::new(10.0, 30.0),
            "a child with no baseline reports its bottom"
        );
        assert_eq!(child_offset(baseline, &app), Offset::new(0.0, 20.0));
    }

    #[test]
    fn the_clip_widgets_reach_their_render_objects_and_a_shape_clip_resolves_its_direction() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            ClipOval::new().child(SizedBox::shrink()).into_widget(),
        );
        harness.pump(&mut app);
        assert!(
            render_object_under_root::<RenderClipOval>(&harness, &app).clip_behavior(&app)
                == Clip::AntiAlias
        );

        harness.set_child(
            &mut app,
            ClipRSuperellipse::new()
                .border_radius(reveal_painting::BorderRadiusGeometry::circular(8.0))
                .child(SizedBox::shrink())
                .into_widget(),
        );
        harness.pump(&mut app);
        let superellipse = render_object_under_root::<RenderClipRSuperellipse>(&harness, &app);
        assert_eq!(
            superellipse.border_radius(&app),
            reveal_painting::BorderRadiusGeometry::circular(8.0)
        );

        let shape: Box<dyn ShapeBorder> = Box::new(reveal_painting::RoundedRectangleBorder::new(
            reveal_painting::BorderSide::NONE,
            reveal_painting::BorderRadiusGeometry::circular(4.0),
        ));
        harness.set_child(
            &mut app,
            Directionality::new(
                TextDirection::Rtl,
                ClipPath::shape(shape)
                    .clip_behavior(Clip::HardEdge)
                    .child(SizedBox::shrink()),
            )
            .into_widget(),
        );
        harness.pump(&mut app);
        let clip_path = harness
            .render_root(&app)
            .child(&app)
            .expect("the clip under the directionality")
            .as_object()
            .downcast::<RenderClipPath>(&app)
            .expect("a RenderClipPath");
        assert_eq!(clip_path.clip_behavior(&app), Clip::HardEdge);
        let clipper = clip_path.clipper(&app).expect("the shape clipper");
        let clipper = clipper
            .as_any()
            .downcast_ref::<ShapeBorderClipper>()
            .expect("a ShapeBorderClipper");
        assert_eq!(clipper.text_direction, Some(TextDirection::Rtl));
    }
}
