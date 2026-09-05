//! Flutter counterpart: `rendering/viewport.dart`.
//!
//! Semantics and debug paint wait; see `PORTING.md`.

use std::any::{Any, TypeId};
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::Curve;
use reveal_embedder::{Clip, Matrix4, Offset, Rect, Size, clamp_double};
use reveal_foundation::{App, HandleId, Listenable, Listener};
use reveal_painting::{Axis, AxisDirection, axis_direction_to_axis, transform_rect};

use crate::box_::{BoxConstraints, BoxHitTestResult, RenderBox, RenderBoxData};
use crate::debug::debug_check_has_bounded_axis;
use crate::object::{
    AnyRenderObject, ContainerRenderObjectData, ContainerRenderObjectMixin, RenderHandle,
    RenderObject, RenderObjectData, debug_checking_intrinsics, resolve, translate,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;
use crate::sliver::{
    AnyRenderSliver, GrowthDirection, SliverConstraints, SliverGeometry, SliverHitTestResult,
    SliverLogicalContainerParentData, SliverPhysicalContainerParentData,
    apply_growth_direction_to_axis_direction, apply_growth_direction_to_scroll_direction,
};
use crate::viewport_offset::AnyViewportOffset;

/// The unit of measurement for a viewport's cache extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheExtentStyle {
    /// Treat the cache extent as logical pixels.
    Pixel,

    /// Treat the cache extent as a multiplier of the main axis extent.
    Viewport,
}

/// The amount of additional content to display and lay out around the viewport.
///
/// The cache area is used to lay out slivers before they are visible on screen. Items that fall
/// in this cache area are laid out even though they are not visible on screen. This allows the
/// viewport to render them ahead of time, enabling a smoother scrolling experience.
///
/// This type encapsulates both the value and the style of the cache extent. It allows the cache
/// extent to be defined either as a fixed number of logical pixels using
/// [`ScrollCacheExtent::Pixels`] or as a multiplier of the viewport's main axis extent using
/// [`ScrollCacheExtent::Viewport`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollCacheExtent {
    /// A cache extent in logical pixels.
    Pixels(f64),

    /// A cache extent as a multiplier of the viewport's main axis extent.
    ///
    /// The main axis extent is the size of the viewport in its main axis. For example, for a
    /// vertically scrolling list, the main axis extent is the height of the viewport. If the
    /// viewport is 600 logical pixels tall, then `ScrollCacheExtent::Viewport(2.0)` results in a
    /// cache extent of 1200 logical pixels.
    Viewport(f64),
}

impl ScrollCacheExtent {
    /// Returns the cache extent in logical pixels for a given `main_axis_extent`.
    fn calculate_cache_offset(self, main_axis_extent: f64) -> f64 {
        match self {
            ScrollCacheExtent::Pixels(pixels) => pixels,
            ScrollCacheExtent::Viewport(value) => value * main_axis_extent,
        }
    }

    /// Returns the style of the cache extent.
    pub fn style(self) -> CacheExtentStyle {
        match self {
            ScrollCacheExtent::Pixels(_) => CacheExtentStyle::Pixel,
            ScrollCacheExtent::Viewport(_) => CacheExtentStyle::Viewport,
        }
    }

    /// Returns the raw value of the cache extent.
    ///
    /// If [`style`](Self::style) is [`CacheExtentStyle::Pixel`], this is the number of pixels to
    /// cache. If it is [`CacheExtentStyle::Viewport`], this is the multiplier of the viewport's
    /// main axis extent to cache.
    pub fn value(self) -> f64 {
        match self {
            ScrollCacheExtent::Pixels(pixels) => pixels,
            ScrollCacheExtent::Viewport(value) => value,
        }
    }
}

/// Specifies an order in which to paint the slivers of a viewport.
///
/// Whichever order the slivers are painted in, they will be hit-tested in the opposite order.
///
/// This can also be thought of as an ordering in the z-direction: whichever sliver is painted
/// last (and hit-tested first) is on top, because it will paint over other slivers if there is
/// overlap. Similarly, whichever sliver is painted first (and hit-tested last) is on the bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SliverPaintOrder {
    /// The first sliver paints on top, and the last sliver on bottom.
    ///
    /// The slivers are painted in the reverse order of the viewport's children, and hit-tested
    /// in the same order as the viewport's children.
    ///
    /// This is the default order.
    #[default]
    FirstIsTop,

    /// The last sliver paints on top, and the first sliver on bottom.
    ///
    /// The slivers are painted in the same order as the viewport's children, and hit-tested in
    /// the reverse order.
    LastIsTop,
}

/// An interface for render objects that are bigger on the inside.
///
/// Some render objects, such as [`RenderViewport`], present a portion of their content, which
/// can be controlled by a [`crate::ViewportOffset`]. This interface lets the framework recognize
/// such render objects and interact with them without having specific knowledge of all the
/// various types of viewports.
///
/// Answer [`RenderObject::interface`] for [`AnyRenderAbstractViewport`] with
/// [`as_abstract_viewport`](Self::as_abstract_viewport) so that
/// [`AnyRenderAbstractViewport::maybe_of`] can find this render object.
pub trait RenderAbstractViewport: RenderObject {
    /// Returns the offset that would be needed to reveal the `target` render object.
    ///
    /// This is used by [`show_in_viewport`], which is itself used by
    /// [`RenderObject::show_on_screen`] for [`RenderViewportBase`].
    ///
    /// The optional `rect` parameter describes which area of that `target` object should be
    /// revealed in the viewport. If `rect` is `None`, the entire `target` (as defined by its
    /// paint bounds) will be revealed. If `rect` is provided it has to be given in the
    /// coordinate system of the `target` object.
    ///
    /// The `alignment` argument describes where the target should be positioned after applying
    /// the returned offset. If `alignment` is 0.0, the child must be positioned as close to the
    /// leading edge of the viewport as possible. If `alignment` is 1.0, the child must be
    /// positioned as close to the trailing edge of the viewport as possible. If `alignment` is
    /// 0.5, the child must be positioned as close to the center of the viewport as possible.
    ///
    /// The `target` might not be a direct child of this viewport but it must be a descendant of
    /// the viewport. Other viewports in between this viewport and the `target` will not be
    /// adjusted.
    ///
    /// This method assumes that the content of the viewport moves linearly, i.e. when the offset
    /// of the viewport is changed by x then `target` also moves by x within the viewport.
    ///
    /// The optional [`Axis`] determines which of two axes to compute an offset for.
    /// One-dimensional viewports ignore it, since there is only one [`Axis`].
    fn get_offset_to_reveal(
        self: RenderHandle<Self>,
        app: &App,
        target: AnyRenderObject,
        alignment: f64,
        rect: Option<Rect>,
        axis: Option<Axis>,
    ) -> RevealedOffset;

    /// The type-erased `RenderAbstractViewport` handle. Free: the vtable is a `const`, and the
    /// id is copied.
    fn as_abstract_viewport(self: RenderHandle<Self>) -> AnyRenderAbstractViewport {
        AnyRenderAbstractViewport {
            id: self.id(),
            vtable: const { &RenderAbstractViewportVTable::of::<Self>() },
        }
    }
}

/// The vtable of an erased [`AnyRenderAbstractViewport`].
struct RenderAbstractViewportVTable {
    get_offset_to_reveal: GetOffsetToRevealFn,
    as_object: fn(&App, HandleId) -> AnyRenderObject,
}

/// The dispatch signature of [`RenderAbstractViewport::get_offset_to_reveal`].
type GetOffsetToRevealFn =
    fn(&App, HandleId, AnyRenderObject, f64, Option<Rect>, Option<Axis>) -> RevealedOffset;

impl RenderAbstractViewportVTable {
    const fn of<T: RenderAbstractViewport>() -> RenderAbstractViewportVTable {
        RenderAbstractViewportVTable {
            get_offset_to_reveal: |app, id, target, alignment, rect, axis| {
                T::get_offset_to_reveal(resolve(id), app, target, alignment, rect, axis)
            },
            as_object: |app, id| resolve::<T>(id).as_render_object(app),
        }
    }
}

/// Erased [`RenderAbstractViewport`]: what a field or parameter Dart types as
/// `RenderAbstractViewport` becomes.
#[derive(Clone, Copy)]
pub struct AnyRenderAbstractViewport {
    id: HandleId,
    vtable: &'static RenderAbstractViewportVTable,
}

impl AnyRenderAbstractViewport {
    /// The default value for the cache extent of a viewport.
    ///
    /// This default assumes [`CacheExtentStyle::Pixel`].
    pub const DEFAULT_CACHE_EXTENT: f64 = 250.0;

    /// Returns the viewport that most tightly encloses the given render object.
    ///
    /// If the object does not have a viewport as an ancestor, this returns `None`.
    ///
    /// Dart's `RenderAbstractViewport.maybeOf`.
    pub fn maybe_of(
        app: &App,
        object: Option<AnyRenderObject>,
    ) -> Option<AnyRenderAbstractViewport> {
        let mut object = object;
        while let Some(current) = object {
            if let Some(viewport) = current.interface::<AnyRenderAbstractViewport>() {
                return Some(viewport);
            }
            object = current.parent(app);
        }
        None
    }

    /// Returns the viewport that most tightly encloses the given render object.
    ///
    /// Dart's `RenderAbstractViewport.of`.
    ///
    /// # Panics
    ///
    /// If the object does not have a viewport as an ancestor.
    pub fn of(app: &App, object: Option<AnyRenderObject>) -> AnyRenderAbstractViewport {
        AnyRenderAbstractViewport::maybe_of(app, object).unwrap_or_else(|| {
            panic!(
                "RenderAbstractViewport::of was called with a render object that was not a \
                 descendant of a RenderAbstractViewport: {object:?}"
            )
        })
    }

    /// The `RenderObject` view of this viewport.
    pub fn as_object(self, app: &App) -> AnyRenderObject {
        (self.vtable.as_object)(app, self.id)
    }

    /// See [`RenderAbstractViewport::get_offset_to_reveal`].
    pub fn get_offset_to_reveal(
        self,
        app: &App,
        target: AnyRenderObject,
        alignment: f64,
        rect: Option<Rect>,
        axis: Option<Axis>,
    ) -> RevealedOffset {
        (self.vtable.get_offset_to_reveal)(app, self.id, target, alignment, rect, axis)
    }
}

impl PartialEq for AnyRenderAbstractViewport {
    fn eq(&self, other: &AnyRenderAbstractViewport) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyRenderAbstractViewport {}

impl Debug for AnyRenderAbstractViewport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyRenderAbstractViewport({:?})", self.id)
    }
}

/// Return value for [`RenderAbstractViewport::get_offset_to_reveal`].
///
/// It indicates the [`offset`](Self::offset) required to reveal an element in a viewport and the
/// [`rect`](Self::rect) position said element would have in the viewport at that offset.
#[derive(Clone, Copy, Debug)]
pub struct RevealedOffset {
    /// Offset for the viewport to reveal a specific element in the viewport.
    pub offset: f64,

    /// The [`Rect`] in the outer coordinate system of the viewport at which the to-be-revealed
    /// element would be located if the viewport's offset is set to [`offset`](Self::offset).
    ///
    /// A viewport usually has two coordinate systems and works as an adapter between the two:
    ///
    /// The inner coordinate system has its origin at the top left corner of the content that
    /// moves inside the viewport. The origin of this coordinate system usually moves around
    /// relative to the leading edge of the viewport when the viewport offset changes.
    ///
    /// The outer coordinate system has its origin at the top left corner of the visible part of
    /// the viewport. This origin stays at the same position regardless of the current viewport
    /// offset.
    pub rect: Rect,
}

impl RevealedOffset {
    /// Instantiates a return value for [`RenderAbstractViewport::get_offset_to_reveal`].
    pub const fn new(offset: f64, rect: Rect) -> RevealedOffset {
        RevealedOffset { offset, rect }
    }

    /// Determines which provided leading or trailing edge of the viewport will be used for
    /// [`show_in_viewport`], accounting for the size and already visible portion of the render
    /// object that is being revealed.
    ///
    /// If the target render object is already fully visible, this returns `None`.
    pub fn clamp_offset(
        leading_edge_offset: RevealedOffset,
        trailing_edge_offset: RevealedOffset,
        current_offset: f64,
    ) -> Option<RevealedOffset> {
        //           scrollOffset
        //                       0 +---------+
        //                         |         |
        //                       _ |         |
        //    viewport position |  |         |
        // with `descendant` at |  |         | _
        //        trailing edge |_ | xxxxxxx |  | viewport position
        //                         |         |  | with `descendant` at
        //                         |         | _| leading edge
        //                         |         |
        //                     800 +---------+
        //
        // `trailing_edge_offset`: Distance from scrollOffset 0 to the start of the viewport on
        //                         the left in the image above.
        // `leading_edge_offset`: Distance from scrollOffset 0 to the start of the viewport on
        //                        the right in the image above.
        //
        // The viewport position on the left is achieved by setting `offset.pixels` to
        // `trailing_edge_offset`, the one on the right by setting it to `leading_edge_offset`.
        let inverted = leading_edge_offset.offset < trailing_edge_offset.offset;
        let (smaller, larger) = if inverted {
            (leading_edge_offset, trailing_edge_offset)
        } else {
            (trailing_edge_offset, leading_edge_offset)
        };
        if current_offset > larger.offset {
            Some(larger)
        } else if current_offset < smaller.offset {
            Some(smaller)
        } else {
            None
        }
    }
}

/// Flutter's `layoutChildSequence`'s `advance` callback: `childBefore` or `childAfter`.
pub type SliverAdvance<T> =
    dyn Fn(RenderHandle<T>, &App, AnyRenderSliver) -> Option<AnyRenderSliver>;

/// Flutter's `RenderViewportBase` fields.
pub struct RenderViewportBaseData {
    axis_direction: AxisDirection,
    cross_axis_direction: AxisDirection,
    offset: AnyViewportOffset,
    scroll_cache_extent: ScrollCacheExtent,
    /// This value is set during layout based on the
    /// [`scroll_cache_extent`](RenderViewportBase::scroll_cache_extent).
    ///
    /// When the style is [`CacheExtentStyle::Viewport`], it is the main axis extent of the
    /// viewport multiplied by the requested cache extent, which is still expressed in pixels.
    calculated_cache_extent: Option<f64>,
    paint_order: SliverPaintOrder,
    clip_behavior: Clip,
}

impl RenderViewportBaseData {
    /// Initializes fields with Flutter's constructor defaults: an [`AxisDirection::Down`] axis,
    /// a [`AnyRenderAbstractViewport::DEFAULT_CACHE_EXTENT`] pixel cache extent,
    /// [`SliverPaintOrder::FirstIsTop`], and [`Clip::HardEdge`].
    pub fn new(
        cross_axis_direction: AxisDirection,
        offset: AnyViewportOffset,
    ) -> RenderViewportBaseData {
        RenderViewportBaseData {
            axis_direction: AxisDirection::Down,
            cross_axis_direction,
            offset,
            scroll_cache_extent: ScrollCacheExtent::Pixels(
                AnyRenderAbstractViewport::DEFAULT_CACHE_EXTENT,
            ),
            calculated_cache_extent: None,
            paint_order: SliverPaintOrder::FirstIsTop,
            clip_behavior: Clip::HardEdge,
        }
    }
}

/// Implements [`RenderViewportBase`] field accessors for a `viewport` field.
#[macro_export]
macro_rules! render_viewport_base_accessors {
    () => {
        fn viewport_data(
            self: $crate::RenderHandle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::RenderViewportBaseData {
            &self.get(app).viewport
        }
        fn viewport_data_mut(
            self: $crate::RenderHandle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::RenderViewportBaseData {
            &mut self.get_mut(app).viewport
        }
    };
}

/// A base class for render objects that are bigger on the inside.
///
/// This render object provides the shared code for render objects that host [`AnyRenderSliver`]
/// render objects inside a [`RenderBox`]. The viewport establishes an
/// [`axis_direction`](Self::axis_direction), which orients the sliver's coordinate system, which
/// is based on scroll offsets rather than Cartesian coordinates.
///
/// The viewport also listens to an [`offset`](Self::offset), which determines the
/// [`SliverConstraints::scroll_offset`] input to the sliver layout protocol.
///
/// Implementors typically override [`RenderObject::perform_layout`] and call
/// [`layout_child_sequence`](Self::layout_child_sequence), perhaps multiple times.
///
/// Flutter's `RenderViewportBase`; the state lives in a [`RenderViewportBaseData`] field.
pub trait RenderViewportBase:
    RenderBox + RenderAbstractViewport + ContainerRenderObjectMixin<ChildType = AnyRenderSliver>
{
    /// Mixin field access.
    fn viewport_data(self: RenderHandle<Self>, app: &App) -> &RenderViewportBaseData;

    /// See [`viewport_data`](Self::viewport_data).
    fn viewport_data_mut(self: RenderHandle<Self>, app: &mut App) -> &mut RenderViewportBaseData;

    /// The direction in which the [`SliverConstraints::scroll_offset`] increases.
    ///
    /// For example, if the axis direction is [`AxisDirection::Down`], a scroll offset of zero is
    /// at the top of the viewport and increases towards the bottom of the viewport.
    fn axis_direction(self: RenderHandle<Self>, app: &App) -> AxisDirection {
        self.viewport_data(app).axis_direction
    }

    /// Sets [`axis_direction`](Self::axis_direction).
    fn set_axis_direction(self: RenderHandle<Self>, app: &mut App, value: AxisDirection) {
        if value == self.axis_direction(app) {
            return;
        }
        self.viewport_data_mut(app).axis_direction = value;
        self.mark_needs_layout(app);
    }

    /// The direction in which child should be laid out in the cross axis.
    ///
    /// For example, if the [`axis_direction`](Self::axis_direction) is [`AxisDirection::Down`],
    /// this property is typically [`AxisDirection::Left`] if the ambient text direction is
    /// right-to-left and [`AxisDirection::Right`] if it is left-to-right.
    fn cross_axis_direction(self: RenderHandle<Self>, app: &App) -> AxisDirection {
        self.viewport_data(app).cross_axis_direction
    }

    /// Sets [`cross_axis_direction`](Self::cross_axis_direction).
    fn set_cross_axis_direction(self: RenderHandle<Self>, app: &mut App, value: AxisDirection) {
        if value == self.cross_axis_direction(app) {
            return;
        }
        self.viewport_data_mut(app).cross_axis_direction = value;
        self.mark_needs_layout(app);
    }

    /// The axis along which the viewport scrolls.
    ///
    /// For example, if the [`axis_direction`](Self::axis_direction) is [`AxisDirection::Down`],
    /// then the axis is [`Axis::Vertical`] and the viewport scrolls vertically.
    fn axis(self: RenderHandle<Self>, app: &App) -> Axis {
        axis_direction_to_axis(self.axis_direction(app))
    }

    /// Which part of the content inside the viewport should be visible.
    ///
    /// The [`crate::ViewportOffset::pixels`] value determines the scroll offset that the
    /// viewport uses to select which part of its content to display. As the user scrolls the
    /// viewport, this value changes, which changes the content that is displayed.
    fn offset(self: RenderHandle<Self>, app: &App) -> AnyViewportOffset {
        self.viewport_data(app).offset
    }

    /// Sets [`offset`](Self::offset).
    fn set_offset(self: RenderHandle<Self>, app: &mut App, value: AnyViewportOffset) {
        let old = self.offset(app);
        if value == old {
            return;
        }
        if self.attached(app) {
            old.remove_listener(app, &self.mark_needs_layout_listener());
        }
        self.viewport_data_mut(app).offset = value;
        if self.attached(app) {
            value.add_listener(app, self.mark_needs_layout_listener());
        }
        // We need to go through layout even if the new offset has the same pixels value as the
        // old offset so that we will apply our viewport and content dimensions.
        self.mark_needs_layout(app);
    }

    /// Dart's `_offset.addListener(markNeedsLayout)` tear-off.
    fn mark_needs_layout_listener(self: RenderHandle<Self>) -> Listener {
        Listener::handle_method(self.handle(), viewport_offset_changed::<Self>)
    }

    /// The viewport has an area before and after the visible area to cache items that are about
    /// to become visible when the user scrolls.
    ///
    /// Items that fall in this cache area are laid out even though they are not (yet) visible on
    /// screen. The scroll cache extent describes how much the cache area extends before the
    /// leading edge and after the trailing edge of the viewport.
    ///
    /// The total extent, which the viewport will try to cover with children, is the scroll cache
    /// extent before the leading edge + extent of the main axis + the scroll cache extent after
    /// the trailing edge.
    fn scroll_cache_extent(self: RenderHandle<Self>, app: &App) -> ScrollCacheExtent {
        self.viewport_data(app).scroll_cache_extent
    }

    /// Sets [`scroll_cache_extent`](Self::scroll_cache_extent).
    ///
    /// `None` resets the value to [`AnyRenderAbstractViewport::DEFAULT_CACHE_EXTENT`] pixels.
    fn set_scroll_cache_extent(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<ScrollCacheExtent>,
    ) {
        let effective_value = value.unwrap_or(ScrollCacheExtent::Pixels(
            AnyRenderAbstractViewport::DEFAULT_CACHE_EXTENT,
        ));
        if effective_value == self.scroll_cache_extent(app) {
            return;
        }
        self.viewport_data_mut(app).scroll_cache_extent = effective_value;
        self.mark_needs_layout(app);
    }

    /// Deprecated. Use [`scroll_cache_extent`](Self::scroll_cache_extent) instead.
    #[deprecated(note = "Use scroll_cache_extent instead.")]
    fn cache_extent(self: RenderHandle<Self>, app: &App) -> f64 {
        self.scroll_cache_extent(app).value()
    }

    /// Deprecated. Use [`set_scroll_cache_extent`](Self::set_scroll_cache_extent) instead.
    ///
    /// `None` resets the value to [`AnyRenderAbstractViewport::DEFAULT_CACHE_EXTENT`] pixels, in
    /// which case the style becomes [`CacheExtentStyle::Pixel`].
    #[deprecated(note = "Use set_scroll_cache_extent instead.")]
    fn set_cache_extent(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        let current = self.scroll_cache_extent(app);
        if value == Some(current.value()) {
            return;
        }
        let replacement = match value {
            None => ScrollCacheExtent::Pixels(AnyRenderAbstractViewport::DEFAULT_CACHE_EXTENT),
            Some(value) => match current {
                ScrollCacheExtent::Pixels(_) => ScrollCacheExtent::Pixels(value),
                ScrollCacheExtent::Viewport(_) => ScrollCacheExtent::Viewport(value),
            },
        };
        self.viewport_data_mut(app).scroll_cache_extent = replacement;
        self.mark_needs_layout(app);
    }

    /// Deprecated. Use [`scroll_cache_extent`](Self::scroll_cache_extent) instead.
    #[deprecated(note = "Use scroll_cache_extent instead.")]
    fn cache_extent_style(self: RenderHandle<Self>, app: &App) -> CacheExtentStyle {
        self.scroll_cache_extent(app).style()
    }

    /// Deprecated. Use [`set_scroll_cache_extent`](Self::set_scroll_cache_extent) instead.
    #[deprecated(note = "Use set_scroll_cache_extent instead.")]
    fn set_cache_extent_style(self: RenderHandle<Self>, app: &mut App, value: CacheExtentStyle) {
        let current = self.scroll_cache_extent(app);
        if value == current.style() {
            return;
        }
        self.viewport_data_mut(app).scroll_cache_extent = match value {
            CacheExtentStyle::Pixel => ScrollCacheExtent::Pixels(current.value()),
            CacheExtentStyle::Viewport => ScrollCacheExtent::Viewport(current.value()),
        };
        self.mark_needs_layout(app);
    }

    /// The order in which to paint the slivers; equivalently, the order in which to arrange them
    /// in the z-direction.
    ///
    /// Whichever order the slivers are painted in, they will be hit-tested in the opposite
    /// order.
    ///
    /// The default is [`SliverPaintOrder::FirstIsTop`].
    fn paint_order(self: RenderHandle<Self>, app: &App) -> SliverPaintOrder {
        self.viewport_data(app).paint_order
    }

    /// Sets [`paint_order`](Self::paint_order).
    fn set_paint_order(self: RenderHandle<Self>, app: &mut App, value: SliverPaintOrder) {
        if value == self.paint_order(app) {
            return;
        }
        self.viewport_data_mut(app).paint_order = value;
        self.mark_needs_paint(app);
    }

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to [`Clip::HardEdge`].
    fn clip_behavior(self: RenderHandle<Self>, app: &App) -> Clip {
        self.viewport_data(app).clip_behavior
    }

    /// Sets [`clip_behavior`](Self::clip_behavior).
    fn set_clip_behavior(self: RenderHandle<Self>, app: &mut App, value: Clip) {
        if value == self.clip_behavior(app) {
            return;
        }
        self.viewport_data_mut(app).clip_behavior = value;
        self.mark_needs_paint(app);
    }

    /// The body of Flutter's `attach` override: listens to the offset.
    fn did_attach(
        self: RenderHandle<Self>,
        app: &mut App,
        owner: reveal_foundation::Handle<PipelineOwner>,
    ) {
        ContainerRenderObjectMixin::did_attach(self, app, owner);
        self.offset(app)
            .add_listener(app, self.mark_needs_layout_listener());
    }

    /// The body of Flutter's `detach` override: stops listening to the offset.
    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        self.offset(app)
            .remove_listener(app, &self.mark_needs_layout_listener());
        ContainerRenderObjectMixin::did_detach(self, app);
    }

    /// Panics saying that the object does not support returning intrinsic dimensions if, in
    /// debug mode, we are not in the [`debug_checking_intrinsics`] mode.
    ///
    /// This is used by [`compute_min_intrinsic_width`](Self::compute_min_intrinsic_width) et al
    /// because viewports do not generally support returning intrinsic dimensions. See the
    /// discussion at [`RenderBox::compute_min_intrinsic_width`].
    fn debug_throw_if_not_checking_intrinsics(self: RenderHandle<Self>) -> bool {
        let _ = self;
        debug_assert!(
            debug_checking_intrinsics(),
            "{} does not support returning intrinsic dimensions.\n\
             Calculating the intrinsic dimensions would require instantiating every child of \
             the viewport, which defeats the point of viewports being lazy.\n\
             If you are merely trying to shrink-wrap the viewport in the main axis direction, \
             consider a RenderShrinkWrappingViewport render object (ShrinkWrappingViewport \
             widget), which achieves that effect without implementing the intrinsic dimension \
             API.",
            std::any::type_name::<Self>()
        );
        true
    }

    /// Zero; a viewport has no intrinsic dimensions.
    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let _ = (app, height);
        debug_assert!(self.debug_throw_if_not_checking_intrinsics());
        0.0
    }

    /// Zero; a viewport has no intrinsic dimensions.
    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let _ = (app, height);
        debug_assert!(self.debug_throw_if_not_checking_intrinsics());
        0.0
    }

    /// Zero; a viewport has no intrinsic dimensions.
    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let _ = (app, width);
        debug_assert!(self.debug_throw_if_not_checking_intrinsics());
        0.0
    }

    /// Zero; a viewport has no intrinsic dimensions.
    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let _ = (app, width);
        debug_assert!(self.debug_throw_if_not_checking_intrinsics());
        0.0
    }

    /// Determines the size and position of some of the children of the viewport.
    ///
    /// This function is the workhorse of `perform_layout` implementations in implementors.
    ///
    /// Layout starts with `child`, proceeds according to the `advance` callback, and stops once
    /// `advance` returns `None`.
    ///
    ///  * `scroll_offset` is the [`SliverConstraints::scroll_offset`] to pass the first child.
    ///    The scroll offset is adjusted by [`SliverGeometry::scroll_extent`] for subsequent
    ///    children.
    ///  * `overlap` is the [`SliverConstraints::overlap`] to pass the first child. The overlap
    ///    is adjusted by the [`SliverGeometry::paint_origin`] and
    ///    [`SliverGeometry::paint_extent`] for subsequent children.
    ///  * `layout_offset` is the layout offset at which to place the first child. The layout
    ///    offset is updated by the [`SliverGeometry::layout_extent`] for subsequent children.
    ///  * `remaining_paint_extent` is [`SliverConstraints::remaining_paint_extent`] to pass the
    ///    first child. The remaining paint extent is updated by the
    ///    [`SliverGeometry::layout_extent`] for subsequent children.
    ///  * `main_axis_extent` is the [`SliverConstraints::viewport_main_axis_extent`] to pass to
    ///    each child.
    ///  * `cross_axis_extent` is the [`SliverConstraints::cross_axis_extent`] to pass to each
    ///    child.
    ///  * `growth_direction` is the [`SliverConstraints::growth_direction`] to pass to each
    ///    child.
    ///
    /// Returns the first non-zero [`SliverGeometry::scroll_offset_correction`] encountered, if
    /// any. Otherwise returns 0.0. Typical callers will call this function repeatedly until it
    /// returns 0.0.
    #[allow(clippy::too_many_arguments)]
    fn layout_child_sequence(
        self: RenderHandle<Self>,
        app: &mut App,
        child: Option<AnyRenderSliver>,
        scroll_offset: f64,
        overlap: f64,
        layout_offset: f64,
        remaining_paint_extent: f64,
        main_axis_extent: f64,
        cross_axis_extent: f64,
        growth_direction: GrowthDirection,
        advance: &SliverAdvance<Self>,
        remaining_cache_extent: f64,
        cache_origin: f64,
    ) -> f64 {
        debug_assert!(scroll_offset.is_finite());
        debug_assert!(scroll_offset >= 0.0);
        let initial_layout_offset = layout_offset;
        let adjusted_user_scroll_direction = apply_growth_direction_to_scroll_direction(
            self.offset(app).user_scroll_direction(app),
            growth_direction,
        );
        let axis_direction = self.axis_direction(app);
        let cross_axis_direction = self.cross_axis_direction(app);
        let mut max_paint_offset = layout_offset + overlap;
        let mut preceding_scroll_extent = 0.0;
        let mut scroll_offset = scroll_offset;
        let mut layout_offset = layout_offset;
        let mut remaining_cache_extent = remaining_cache_extent;
        let mut cache_origin = cache_origin;
        let mut child = child;

        while let Some(current) = child {
            let sliver_scroll_offset = scroll_offset.max(0.0);
            // If the scrollOffset is too small we adjust the paddedOrigin because it doesn't
            // make sense to ask a sliver for content before its scroll offset.
            let corrected_cache_origin = cache_origin.max(-sliver_scroll_offset);
            let cache_extent_correction = cache_origin - corrected_cache_origin;

            debug_assert!(sliver_scroll_offset >= corrected_cache_origin.abs());
            debug_assert!(corrected_cache_origin <= 0.0);
            debug_assert!(sliver_scroll_offset >= 0.0);
            debug_assert!(cache_extent_correction <= 0.0);

            current.layout(
                app,
                SliverConstraints::new(
                    axis_direction,
                    growth_direction,
                    adjusted_user_scroll_direction,
                    sliver_scroll_offset,
                    preceding_scroll_extent,
                    max_paint_offset - layout_offset,
                    (remaining_paint_extent - layout_offset + initial_layout_offset).max(0.0),
                    cross_axis_extent,
                    cross_axis_direction,
                    main_axis_extent,
                    (remaining_cache_extent + cache_extent_correction).max(0.0),
                    corrected_cache_origin,
                ),
                true,
            );

            let child_layout_geometry = current.geometry(app);
            debug_assert!(child_layout_geometry.debug_assert_is_valid());

            // If there is a correction to apply, we'll have to start over.
            if let Some(correction) = child_layout_geometry.scroll_offset_correction {
                return correction;
            }

            // We use the child's paint origin in our coordinate system as the layoutOffset we
            // store in the child's parent data.
            let effective_layout_offset = layout_offset + child_layout_geometry.paint_origin;

            // `effective_layout_offset` becomes meaningless once we moved past the trailing edge
            // because `child_layout_geometry.layout_extent` is zero. Using the still increasing
            // `scroll_offset` to roughly position these invisible slivers in the right order.
            if child_layout_geometry.visible || scroll_offset > 0.0 {
                self.update_child_layout_offset(
                    app,
                    current,
                    effective_layout_offset,
                    growth_direction,
                );
            } else {
                self.update_child_layout_offset(
                    app,
                    current,
                    -scroll_offset + initial_layout_offset,
                    growth_direction,
                );
            }

            max_paint_offset = (effective_layout_offset + child_layout_geometry.paint_extent)
                .max(max_paint_offset);
            scroll_offset -= child_layout_geometry.scroll_extent;
            preceding_scroll_extent += child_layout_geometry.scroll_extent;
            layout_offset += child_layout_geometry.layout_extent;
            if child_layout_geometry.cache_extent != 0.0 {
                remaining_cache_extent -=
                    child_layout_geometry.cache_extent - cache_extent_correction;
                cache_origin =
                    (corrected_cache_origin + child_layout_geometry.cache_extent).min(0.0);
            }

            self.update_out_of_band_data(app, growth_direction, child_layout_geometry);

            // move on to the next child
            child = advance(self, app, current);
        }

        // we made it without a correction, whee!
        0.0
    }

    /// The body of Flutter's `paint` override.
    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if self.first_child(app).is_none() {
            return;
        }
        let clip_behavior = self.clip_behavior(app);
        if self.has_visual_overflow(app) && clip_behavior != Clip::None {
            let bounds = Offset::ZERO & self.size(app);
            context.push_clip_rect(
                app,
                offset,
                bounds,
                |app, context, offset| self.paint_contents(app, context, offset),
                clip_behavior,
            );
        } else {
            self.paint_contents(app, context, offset);
        }
    }

    /// Flutter's `_paintContents`.
    fn paint_contents(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        for child in self.children_in_paint_order(app) {
            if child.geometry(app).visible {
                let child_offset = offset + self.paint_offset_of(app, child);
                context.paint_child(app, child.as_object(), child_offset);
            }
        }
    }

    /// The body of Flutter's `hitTestChildren` override.
    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let (main_axis_position, cross_axis_position) = match self.axis(app) {
            Axis::Vertical => (position.dy(), position.dx()),
            Axis::Horizontal => (position.dx(), position.dy()),
        };
        for child in self.children_in_hit_test_order(app) {
            if !child.geometry(app).visible {
                continue;
            }
            let mut transform = Matrix4::IDENTITY;
            // must be invertible
            RenderObject::apply_paint_transform(self, app, child.as_object(), &mut transform);
            let child_main_axis_position =
                self.compute_child_main_axis_position(app, child, main_axis_position);
            let is_hit =
                result.add_with_out_of_band_position(None, Some(transform), None, |result| {
                    child.hit_test(
                        app,
                        &mut SliverHitTestResult::wrap(result),
                        child_main_axis_position,
                        cross_axis_position,
                    )
                });
            if is_hit {
                return true;
            }
        }
        false
    }

    /// The body of Flutter's `getOffsetToReveal` override.
    fn get_offset_to_reveal(
        self: RenderHandle<Self>,
        app: &App,
        target: AnyRenderObject,
        alignment: f64,
        rect: Option<Rect>,
        _axis: Option<Axis>,
    ) -> RevealedOffset {
        // One dimensional viewport has only one axis, override if it was provided/may be
        // mismatched.
        let axis = self.axis(app);
        let this = self.as_object();

        // Steps to convert `rect` (from a RenderBox coordinate system) to its scroll offset
        // within this viewport (not in the exact order):
        //
        // 1. Pick the outermost RenderBox (between which, and the viewport, there is nothing but
        // RenderSlivers) as an intermediate reference frame (the `pivot`), convert `rect` to
        // that coordinate space.
        //
        // 2. Convert `rect` from the `pivot` coordinate space to its sliver parent's sliver
        // coordinate system (i.e., to a scroll offset), based on the axis direction and growth
        // direction of the parent.
        //
        // 3. Convert the scroll offset to its sliver parent's coordinate space using
        // `child_scroll_offset`, until we reach the viewport.
        //
        // 4. Make the final conversion from the outmost sliver to the viewport using
        // `scroll_offset_of`.

        let mut leading_scroll_offset = 0.0;
        // Starting at `target` and walking towards the root:
        //  - `child` will be the last object before we reach this viewport, and
        //  - `pivot` will be the last RenderBox before we reach this viewport.
        let mut child = target;
        let mut pivot = None;
        // ... between viewport and `target` (`target` included).
        let mut only_slivers = target.as_sliver().is_some();
        let mut rect = rect;
        while child.parent(app) != Some(this) {
            let parent = child
                .parent(app)
                .expect("target is a descendant of this viewport");
            if let Some(box_child) = child.as_box() {
                pivot = Some(box_child);
            }
            match parent.as_sliver() {
                Some(sliver) => {
                    leading_scroll_offset += sliver
                        .child_scroll_offset(app, child)
                        .expect("a sliver parent reports its child's scroll offset");
                }
                None => {
                    only_slivers = false;
                    leading_scroll_offset = 0.0;
                }
            }
            child = parent;
        }

        // `rect` in the new intermediate coordinate system.
        let rect_local;
        // Our new reference frame render object's main axis extent.
        let pivot_extent;
        let growth_direction;

        // `leading_scroll_offset` is currently the scrollOffset of our new reference frame
        // (`pivot` or `target`), within `child`.
        if let Some(pivot) = pivot {
            debug_assert!(pivot.as_object().parent(app).is_some());
            debug_assert_ne!(pivot.as_object().parent(app), Some(this));
            debug_assert_ne!(pivot.as_object(), this);
            let pivot_parent = pivot
                .as_object()
                .parent(app)
                .and_then(AnyRenderObject::as_sliver)
                .expect("only sliver parents are supported between a pivot and the viewport");
            growth_direction = pivot_parent.constraints(app).growth_direction;
            pivot_extent = match axis {
                Axis::Horizontal => pivot.size(app).width(),
                Axis::Vertical => pivot.size(app).height(),
            };
            let rect = *rect.get_or_insert_with(|| target.paint_bounds(app));
            rect_local =
                transform_rect(&target.get_transform_to(app, Some(pivot.as_object())), rect);
        } else if only_slivers {
            // `pivot` does not exist. We'll have to make up one from `target`, the innermost
            // sliver.
            let target_sliver = target.as_sliver().expect("only_slivers");
            growth_direction = target_sliver.constraints(app).growth_direction;
            pivot_extent = target_sliver.geometry(app).scroll_extent;
            rect_local = *rect.get_or_insert_with(|| {
                let constraints = target_sliver.constraints(app);
                let geometry = target_sliver.geometry(app);
                match axis {
                    Axis::Horizontal => Rect::from_ltwh(
                        0.0,
                        0.0,
                        geometry.scroll_extent,
                        constraints.cross_axis_extent,
                    ),
                    Axis::Vertical => Rect::from_ltwh(
                        0.0,
                        0.0,
                        constraints.cross_axis_extent,
                        geometry.scroll_extent,
                    ),
                }
            });
        } else {
            let rect = rect.expect("a non-sliver target must be given a rect");
            return RevealedOffset::new(self.offset(app).pixels(app), rect);
        }

        debug_assert_eq!(child.parent(app), Some(this));
        let sliver = child
            .as_sliver()
            .expect("a viewport's children are slivers");

        // The scroll offset of `rect` within `child`.
        leading_scroll_offset += match apply_growth_direction_to_axis_direction(
            self.axis_direction(app),
            growth_direction,
        ) {
            AxisDirection::Up => pivot_extent - rect_local.bottom,
            AxisDirection::Left => pivot_extent - rect_local.right,
            AxisDirection::Right => rect_local.left,
            AxisDirection::Down => rect_local.top,
        };

        // So far leading_scroll_offset is the scroll offset of `rect` in the `child` sliver's
        // sliver coordinate system. The sign of this value indicates whether the `rect`
        // protrudes the leading edge of the `child` sliver. When this value is non-negative and
        // `child`'s `max_scroll_obstruction_extent` is greater than 0, we assume `rect` can't be
        // obstructed by the leading edge of the viewport (i.e. it is pinned to the leading edge).
        let is_pinned = sliver.geometry(app).max_scroll_obstruction_extent > 0.0
            && leading_scroll_offset >= 0.0;

        // The scroll offset in the viewport to `rect`.
        leading_scroll_offset = self.scroll_offset_of(app, sliver, leading_scroll_offset);

        // This step assumes the viewport's layout is up-to-date, i.e., if offset.pixels is
        // changed after the last perform_layout, the new scroll position will not be accounted
        // for.
        let transform = target.get_transform_to(app, Some(this));
        let mut target_rect = transform_rect(&transform, rect.expect("set above"));
        let extent_of_pinned_slivers = self.max_scroll_obstruction_extent_before(app, sliver);

        match sliver.constraints(app).growth_direction {
            GrowthDirection::Forward => {
                if is_pinned && alignment <= 0.0 {
                    return RevealedOffset::new(f64::INFINITY, target_rect);
                }
                leading_scroll_offset -= extent_of_pinned_slivers;
            }
            GrowthDirection::Reverse => {
                if is_pinned && alignment >= 1.0 {
                    return RevealedOffset::new(f64::NEG_INFINITY, target_rect);
                }
                // If child's growth direction is reverse, when viewport.offset is
                // `leading_scroll_offset`, it is positioned just outside of the leading edge of
                // the viewport.
                leading_scroll_offset -= match axis {
                    Axis::Vertical => target_rect.height(),
                    Axis::Horizontal => target_rect.width(),
                };
            }
        }

        let size = self.size(app);
        let main_axis_extent_difference = match axis {
            Axis::Horizontal => size.width() - extent_of_pinned_slivers - rect_local.width(),
            Axis::Vertical => size.height() - extent_of_pinned_slivers - rect_local.height(),
        };

        let target_offset = leading_scroll_offset - main_axis_extent_difference * alignment;
        let offset_difference = self.offset(app).pixels(app) - target_offset;

        target_rect = match self.axis_direction(app) {
            AxisDirection::Up => target_rect.translate(0.0, -offset_difference),
            AxisDirection::Down => target_rect.translate(0.0, offset_difference),
            AxisDirection::Left => target_rect.translate(-offset_difference, 0.0),
            AxisDirection::Right => target_rect.translate(offset_difference, 0.0),
        };

        RevealedOffset::new(target_offset, target_rect)
    }

    /// The offset at which the given `child` should be painted.
    ///
    /// The returned offset is from the top left corner of the inside of the viewport to the top
    /// left corner of the paint coordinate system of the `child`.
    ///
    /// See also:
    ///
    ///  * [`paint_offset_of`](Self::paint_offset_of), which uses the layout offset and growth
    ///    direction computed for the child during layout.
    fn compute_absolute_paint_offset(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderSliver,
        layout_offset: f64,
        growth_direction: GrowthDirection,
    ) -> Offset {
        // this is only usable once we have a size
        debug_assert!(self.has_size(app));
        let paint_extent = child.geometry(app).paint_extent;
        let size = self.size(app);
        match apply_growth_direction_to_axis_direction(self.axis_direction(app), growth_direction) {
            AxisDirection::Up => Offset::new(0.0, size.height() - layout_offset - paint_extent),
            AxisDirection::Left => Offset::new(size.width() - layout_offset - paint_extent, 0.0),
            AxisDirection::Right => Offset::new(layout_offset, 0.0),
            AxisDirection::Down => Offset::new(0.0, layout_offset),
        }
    }

    // API TO BE IMPLEMENTED BY SUBCLASSES

    /// Whether the contents of this viewport would paint outside the bounds of the viewport if
    /// [`paint`](Self::paint) did not clip.
    ///
    /// This property enables an optimization whereby [`paint`](Self::paint) can skip applying a
    /// clip if the contents of the viewport are known to paint entirely within its bounds.
    fn has_visual_overflow(self: RenderHandle<Self>, app: &App) -> bool;

    /// Called during [`layout_child_sequence`](Self::layout_child_sequence) for each child.
    ///
    /// Typically used by implementors to update any out-of-band data, such as the max scroll
    /// extent, for each child.
    fn update_out_of_band_data(
        self: RenderHandle<Self>,
        app: &mut App,
        growth_direction: GrowthDirection,
        child_layout_geometry: SliverGeometry,
    );

    /// Called during [`layout_child_sequence`](Self::layout_child_sequence) to store the layout
    /// offset for the given child.
    ///
    /// Different implementors use different representations for their children's layout offset
    /// (e.g., logical or physical coordinates). This function lets them transform the child's
    /// layout offset before storing it in the child's parent data.
    fn update_child_layout_offset(
        self: RenderHandle<Self>,
        app: &mut App,
        child: AnyRenderSliver,
        layout_offset: f64,
        growth_direction: GrowthDirection,
    );

    /// The offset at which the given `child` should be painted.
    ///
    /// The returned offset is from the top left corner of the inside of the viewport to the top
    /// left corner of the paint coordinate system of the `child`.
    ///
    /// See also:
    ///
    ///  * [`compute_absolute_paint_offset`](Self::compute_absolute_paint_offset), which computes
    ///    the paint offset from an explicit layout offset and growth direction instead of using
    ///    the values computed for the child during layout.
    fn paint_offset_of(self: RenderHandle<Self>, app: &App, child: AnyRenderSliver) -> Offset;

    /// Returns the scroll offset within the viewport for the given `scroll_offset_within_child`
    /// within the given `child`.
    ///
    /// The returned value is an estimate that assumes the slivers within the viewport do not
    /// change the layout extent in response to changes in their scroll offset.
    fn scroll_offset_of(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderSliver,
        scroll_offset_within_child: f64,
    ) -> f64;

    /// Returns the total scroll obstruction extent of all slivers in the viewport before
    /// `child`.
    ///
    /// This is the extent by which the actual area in which content can scroll is reduced. For
    /// example, an app bar that is pinned at the top will reduce the area in which content can
    /// actually scroll by the height of the app bar.
    fn max_scroll_obstruction_extent_before(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderSliver,
    ) -> f64;

    /// Converts the `parent_main_axis_position` into the child's coordinate system.
    ///
    /// The `parent_main_axis_position` is a distance from the top edge (for vertical viewports)
    /// or left edge (for horizontal viewports) of the viewport bounds. This describes a line,
    /// perpendicular to the viewport's main axis, heretofore known as the target line.
    ///
    /// The child's coordinate system's origin in the main axis is at the leading edge of the
    /// given child, as given by the child's [`SliverConstraints::axis_direction`] and
    /// [`SliverConstraints::growth_direction`].
    ///
    /// This method returns the distance from the leading edge of the given child to the target
    /// line described above.
    fn compute_child_main_axis_position(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderSliver,
        parent_main_axis_position: f64,
    ) -> f64;

    /// The index of the first child of the viewport relative to the center child.
    ///
    /// For example, the center child has index zero and the first child in the reverse growth
    /// direction has index -1.
    fn index_of_first_child(self: RenderHandle<Self>, app: &App) -> i32;

    /// A short string to identify the child with the given index.
    fn label_for_child(self: RenderHandle<Self>, index: i32) -> String;

    /// Walks the children of the viewport, in the order that they should be painted.
    ///
    /// This is the reverse order of
    /// [`children_in_hit_test_order`](Self::children_in_hit_test_order).
    fn children_in_paint_order(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderSliver> {
        match self.paint_order(app) {
            SliverPaintOrder::FirstIsTop => self.children_last_to_first(app),
            SliverPaintOrder::LastIsTop => self.children_as_list(app),
        }
    }

    /// Walks the children of the viewport, in the order that hit-testing should use.
    ///
    /// This is the reverse order of [`children_in_paint_order`](Self::children_in_paint_order).
    fn children_in_hit_test_order(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderSliver> {
        match self.paint_order(app) {
            SliverPaintOrder::FirstIsTop => self.children_as_list(app),
            SliverPaintOrder::LastIsTop => self.children_last_to_first(app),
        }
    }

    /// Flutter's `_childrenLastToFirst`.
    fn children_last_to_first(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderSliver> {
        let mut children = Vec::new();
        let mut child = self.last_child(app);
        while let Some(current) = child {
            children.push(current);
            child = self.child_before(app, current);
        }
        children
    }

    /// The body of Flutter's `showOnScreen` override.
    fn show_on_screen(
        self: RenderHandle<Self>,
        app: &mut App,
        descendant: Option<AnyRenderObject>,
        rect: Option<Rect>,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) {
        let offset = self.offset(app);
        if !offset.allow_implicit_scrolling(app) {
            return crate::object::RenderObjectBase::show_on_screen(
                self, app, descendant, rect, duration, curve,
            );
        }

        let new_rect = show_in_viewport(
            app,
            descendant,
            rect,
            self.as_abstract_viewport(),
            offset,
            duration,
            Rc::clone(&curve),
        );
        crate::object::RenderObjectBase::show_on_screen(self, app, None, new_rect, duration, curve);
    }
}

/// Dart's `_offset.addListener(markNeedsLayout)` tear-off body: a named function so that the
/// listener added on attach compares equal to the one removed on detach.
fn viewport_offset_changed<T: RenderViewportBase>(
    this: reveal_foundation::Handle<T>,
    app: &mut App,
) {
    RenderHandle::from_handle(this).mark_needs_layout(app);
}

/// Make (a portion of) the given `descendant` of the given `viewport` fully visible in the
/// `viewport` by manipulating the provided [`AnyViewportOffset`] `offset`.
///
/// The optional `rect` parameter describes which area of the `descendant` should be shown in the
/// viewport. If `rect` is `None`, the entire `descendant` will be revealed. The `rect` parameter
/// is interpreted relative to the coordinate system of `descendant`.
///
/// The returned [`Rect`] describes the new location of `descendant` or `rect` in the viewport
/// after it has been revealed. See [`RevealedOffset::rect`] for a full definition of this rect.
///
/// If `descendant` is `None`, this is a no-op and `rect` is returned.
///
/// The `duration` parameter can be set to a non-zero value to animate the target object into the
/// viewport with an animation defined by `curve`.
///
/// Flutter's `RenderViewportBase.showInViewport` static.
pub fn show_in_viewport(
    app: &mut App,
    descendant: Option<AnyRenderObject>,
    rect: Option<Rect>,
    viewport: AnyRenderAbstractViewport,
    offset: AnyViewportOffset,
    duration: Duration,
    curve: Rc<dyn Curve>,
) -> Option<Rect> {
    let Some(descendant) = descendant else {
        return rect;
    };
    let leading_edge_offset = viewport.get_offset_to_reveal(app, descendant, 0.0, rect, None);
    let trailing_edge_offset = viewport.get_offset_to_reveal(app, descendant, 1.0, rect, None);
    let current_offset = offset.pixels(app);
    let target_offset =
        RevealedOffset::clamp_offset(leading_edge_offset, trailing_edge_offset, current_offset);
    let Some(target_offset) = target_offset else {
        // `descendant` is between leading and trailing edge and hence already fully shown on
        // screen. No action necessary.
        let viewport_object = viewport.as_object(app);
        debug_assert!(viewport_object.parent(app).is_some());
        let transform = descendant.get_transform_to(app, viewport_object.parent(app));
        return Some(transform_rect(
            &transform,
            rect.unwrap_or_else(|| descendant.paint_bounds(app)),
        ));
    };

    offset.move_to(app, target_offset.offset, Some(duration), Some(curve), None);
    Some(target_offset.rect)
}

/// A render object that is bigger on the inside.
///
/// [`RenderViewport`] is the visual workhorse of the scrolling machinery. It displays a subset of
/// its children according to its own dimensions and the given
/// [`offset`](RenderViewportBase::offset). As the offset varies, different children are visible
/// through the viewport.
///
/// [`RenderViewport`] hosts a bidirectional list of slivers in a single shared [`Axis`], anchored
/// on a [`center`](Self::center) sliver, which is placed at the zero scroll offset. The center
/// widget is displayed in the viewport according to the [`anchor`](Self::anchor) property.
///
/// Slivers that are earlier in the child list than [`center`](Self::center) are displayed in
/// reverse order in the reverse [`RenderViewportBase::axis_direction`] starting from the center.
///
/// [`RenderViewport`] cannot contain [`RenderBox`] children directly. Instead, use a
/// [`crate::RenderSliverList`], [`crate::RenderSliverFixedExtentList`], or a
/// [`crate::RenderSliverToBoxAdapter`], for example.
pub struct RenderViewport {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderSliver>,
    viewport: RenderViewportBaseData,
    anchor: f64,
    center: Option<AnyRenderSliver>,
    // Out-of-band data computed during layout.
    min_scroll_extent: f64,
    max_scroll_extent: f64,
    has_visual_overflow: bool,
}

impl RenderViewport {
    /// Dart's `_maxLayoutCyclesPerChild`.
    const MAX_LAYOUT_CYCLES_PER_CHILD: usize = 10;

    /// Creates a viewport for [`AnyRenderSliver`] objects.
    ///
    /// If the `center` is not specified, then the first child in the `children` list, if any, is
    /// used.
    ///
    /// For testing purposes, consider passing a [`crate::FixedViewportOffset`] as the `offset`.
    pub fn new(
        app: &mut App,
        cross_axis_direction: AxisDirection,
        offset: AnyViewportOffset,
        children: Option<Vec<AnyRenderSliver>>,
        center: Option<AnyRenderSliver>,
    ) -> RenderHandle<RenderViewport> {
        let this = RenderHandle::new_box(
            app,
            RenderViewport {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                viewport: RenderViewportBaseData::new(cross_axis_direction, offset),
                anchor: 0.0,
                center,
                min_scroll_extent: 0.0,
                max_scroll_extent: 0.0,
                has_visual_overflow: false,
            },
        );
        this.add_all(app, children);
        if center.is_none()
            && let Some(first_child) = this.first_child(app)
        {
            this.get_mut(app).center = Some(first_child);
        }
        this
    }

    /// The relative position of the zero scroll offset.
    ///
    /// For example, if the [`RenderViewportBase::axis_direction`] is [`AxisDirection::Down`] or
    /// [`AxisDirection::Up`] and the anchor is 0.5, then the zero scroll offset is vertically
    /// centered within the viewport. If the anchor is 1.0, and the axis direction is
    /// [`AxisDirection::Right`], then the zero scroll offset is on the left edge of the viewport.
    pub fn anchor(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).anchor
    }

    /// Sets [`anchor`](Self::anchor).
    pub fn set_anchor(self: RenderHandle<Self>, app: &mut App, value: f64) {
        debug_assert!((0.0..=1.0).contains(&value));
        if value == self.get(app).anchor {
            return;
        }
        self.get_mut(app).anchor = value;
        self.mark_needs_layout(app);
    }

    /// The first child in the [`GrowthDirection::Forward`] growth direction.
    ///
    /// This child will be at the position defined by [`anchor`](Self::anchor) when the
    /// [`crate::ViewportOffset::pixels`] of [`RenderViewportBase::offset`] is `0`.
    ///
    /// Children after the center will be placed in the [`RenderViewportBase::axis_direction`]
    /// relative to the center. Children before it will be placed in the opposite direction, and
    /// will have a growth direction of [`GrowthDirection::Reverse`].
    ///
    /// The center must be a direct child of the viewport.
    pub fn center(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderSliver> {
        self.get(app).center
    }

    /// Sets [`center`](Self::center).
    pub fn set_center(self: RenderHandle<Self>, app: &mut App, value: Option<AnyRenderSliver>) {
        if value == self.get(app).center {
            return;
        }
        self.get_mut(app).center = value;
        self.mark_needs_layout(app);
    }

    /// Flutter's `_attemptLayout`.
    fn attempt_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        main_axis_extent: f64,
        cross_axis_extent: f64,
        corrected_offset: f64,
    ) -> f64 {
        debug_assert!(!main_axis_extent.is_nan());
        debug_assert!(main_axis_extent >= 0.0);
        debug_assert!(main_axis_extent.is_finite());
        debug_assert!(cross_axis_extent.is_finite());
        debug_assert!(cross_axis_extent >= 0.0);
        debug_assert!(corrected_offset.is_finite());
        let this = self.get_mut(app);
        this.min_scroll_extent = 0.0;
        this.max_scroll_extent = 0.0;
        this.has_visual_overflow = false;

        // center_offset is the offset from the leading edge of the viewport to the zero scroll
        // offset (the line between the forward slivers and the reverse slivers).
        let anchor = self.get(app).anchor;
        let center_offset = main_axis_extent * anchor - corrected_offset;
        let reverse_direction_remaining_paint_extent =
            clamp_double(center_offset, 0.0, main_axis_extent);
        let forward_direction_remaining_paint_extent =
            clamp_double(main_axis_extent - center_offset, 0.0, main_axis_extent);

        let calculated_cache_extent = self
            .scroll_cache_extent(app)
            .calculate_cache_offset(main_axis_extent);
        self.viewport_data_mut(app).calculated_cache_extent = Some(calculated_cache_extent);

        let full_cache_extent = main_axis_extent + 2.0 * calculated_cache_extent;
        let center_cache_offset = center_offset + calculated_cache_extent;
        let reverse_direction_remaining_cache_extent =
            clamp_double(center_cache_offset, 0.0, full_cache_extent);
        let forward_direction_remaining_cache_extent = clamp_double(
            full_cache_extent - center_cache_offset,
            0.0,
            full_cache_extent,
        );

        let center = self.center(app).expect("checked by perform_layout");
        let leading_negative_child = self.child_before(app, center);

        if let Some(leading_negative_child) = leading_negative_child {
            // negative scroll offsets
            let result = self.layout_child_sequence(
                app,
                Some(leading_negative_child),
                main_axis_extent.max(center_offset) - main_axis_extent,
                0.0,
                forward_direction_remaining_paint_extent,
                reverse_direction_remaining_paint_extent,
                main_axis_extent,
                cross_axis_extent,
                GrowthDirection::Reverse,
                &|this, app, child| this.child_before(app, child),
                reverse_direction_remaining_cache_extent,
                clamp_double(
                    main_axis_extent - center_offset,
                    -calculated_cache_extent,
                    0.0,
                ),
            );
            if result != 0.0 {
                return -result;
            }
        }

        // positive scroll offsets
        self.layout_child_sequence(
            app,
            Some(center),
            (-center_offset).max(0.0),
            if leading_negative_child.is_none() {
                (-center_offset).min(0.0)
            } else {
                0.0
            },
            if center_offset >= main_axis_extent {
                center_offset
            } else {
                reverse_direction_remaining_paint_extent
            },
            forward_direction_remaining_paint_extent,
            main_axis_extent,
            cross_axis_extent,
            GrowthDirection::Forward,
            &|this, app, child| this.child_after(app, child),
            forward_direction_remaining_cache_extent,
            clamp_double(center_offset, -calculated_cache_extent, 0.0),
        )
    }
}

impl ContainerRenderObjectMixin for RenderViewport {
    type ChildType = AnyRenderSliver;
    type ParentDataType = SliverPhysicalContainerParentData;

    fn container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &ContainerRenderObjectData<AnyRenderSliver> {
        &self.get(app).container
    }

    fn container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut ContainerRenderObjectData<AnyRenderSliver> {
        &mut self.get_mut(app).container
    }
}

impl RenderViewportBase for RenderViewport {
    crate::render_viewport_base_accessors!();

    fn has_visual_overflow(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).has_visual_overflow
    }

    fn update_out_of_band_data(
        self: RenderHandle<Self>,
        app: &mut App,
        growth_direction: GrowthDirection,
        child_layout_geometry: SliverGeometry,
    ) {
        let this = self.get_mut(app);
        match growth_direction {
            GrowthDirection::Forward => {
                this.max_scroll_extent += child_layout_geometry.scroll_extent
            }
            GrowthDirection::Reverse => {
                this.min_scroll_extent -= child_layout_geometry.scroll_extent
            }
        }
        if child_layout_geometry.has_visual_overflow {
            this.has_visual_overflow = true;
        }
    }

    fn update_child_layout_offset(
        self: RenderHandle<Self>,
        app: &mut App,
        child: AnyRenderSliver,
        layout_offset: f64,
        growth_direction: GrowthDirection,
    ) {
        let paint_offset =
            self.compute_absolute_paint_offset(app, child, layout_offset, growth_direction);
        child
            .as_object()
            .parent_data_of_mut::<SliverPhysicalContainerParentData>(app)
            .set_paint_offset(paint_offset);
    }

    fn paint_offset_of(self: RenderHandle<Self>, app: &App, child: AnyRenderSliver) -> Offset {
        child
            .as_object()
            .parent_data_of::<SliverPhysicalContainerParentData>(app)
            .paint_offset()
    }

    fn scroll_offset_of(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderSliver,
        scroll_offset_within_child: f64,
    ) -> f64 {
        debug_assert_eq!(child.as_object().parent(app), Some(self.as_object()));
        match child.constraints(app).growth_direction {
            GrowthDirection::Forward => {
                let mut scroll_offset_to_child = 0.0;
                let mut current = self.center(app);
                while current != Some(child) {
                    let sliver = current.expect("the center precedes a forward child");
                    scroll_offset_to_child += sliver.geometry(app).scroll_extent;
                    current = self.child_after(app, sliver);
                }
                scroll_offset_to_child + scroll_offset_within_child
            }
            GrowthDirection::Reverse => {
                let mut scroll_offset_to_child = 0.0;
                let center = self.center(app).expect("a laid-out viewport has a center");
                let mut current = self.child_before(app, center);
                while current != Some(child) {
                    let sliver = current.expect("a reverse child precedes the center");
                    scroll_offset_to_child -= sliver.geometry(app).scroll_extent;
                    current = self.child_before(app, sliver);
                }
                scroll_offset_to_child - scroll_offset_within_child
            }
        }
    }

    fn max_scroll_obstruction_extent_before(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderSliver,
    ) -> f64 {
        debug_assert_eq!(child.as_object().parent(app), Some(self.as_object()));
        let mut pinned_extent = 0.0;
        match child.constraints(app).growth_direction {
            GrowthDirection::Forward => {
                let mut current = self.center(app);
                while current != Some(child) {
                    let sliver = current.expect("the center precedes a forward child");
                    pinned_extent += sliver.geometry(app).max_scroll_obstruction_extent;
                    current = self.child_after(app, sliver);
                }
            }
            GrowthDirection::Reverse => {
                let center = self.center(app).expect("a laid-out viewport has a center");
                let mut current = self.child_before(app, center);
                while current != Some(child) {
                    let sliver = current.expect("a reverse child precedes the center");
                    pinned_extent += sliver.geometry(app).max_scroll_obstruction_extent;
                    current = self.child_before(app, sliver);
                }
            }
        }
        pinned_extent
    }

    fn compute_child_main_axis_position(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderSliver,
        parent_main_axis_position: f64,
    ) -> f64 {
        let paint_offset = child
            .as_object()
            .parent_data_of::<SliverPhysicalContainerParentData>(app)
            .paint_offset();
        let constraints = child.constraints(app);
        let paint_extent = child.geometry(app).paint_extent;
        match apply_growth_direction_to_axis_direction(
            constraints.axis_direction,
            constraints.growth_direction,
        ) {
            AxisDirection::Down => parent_main_axis_position - paint_offset.dy(),
            AxisDirection::Right => parent_main_axis_position - paint_offset.dx(),
            AxisDirection::Up => paint_extent - (parent_main_axis_position - paint_offset.dy()),
            AxisDirection::Left => paint_extent - (parent_main_axis_position - paint_offset.dx()),
        }
    }

    fn index_of_first_child(self: RenderHandle<Self>, app: &App) -> i32 {
        let center = self.center(app).expect("a laid-out viewport has a center");
        debug_assert_eq!(center.as_object().parent(app), Some(self.as_object()));
        debug_assert!(self.first_child(app).is_some());
        let mut count = 0;
        let mut child = Some(center);
        while child != self.first_child(app) {
            count -= 1;
            child = self.child_before(app, child.expect("the first child terminates the walk"));
        }
        count
    }

    fn label_for_child(self: RenderHandle<Self>, index: i32) -> String {
        if index == 0 {
            "center child".to_owned()
        } else {
            format!("child {index}")
        }
    }
}

impl RenderAbstractViewport for RenderViewport {
    fn get_offset_to_reveal(
        self: RenderHandle<Self>,
        app: &App,
        target: AnyRenderObject,
        alignment: f64,
        rect: Option<Rect>,
        axis: Option<Axis>,
    ) -> RevealedOffset {
        RenderViewportBase::get_offset_to_reveal(self, app, target, alignment, rect, axis)
    }
}

impl RenderObject for RenderViewport {
    crate::render_object_accessors!();

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(
        self: RenderHandle<Self>,
        app: &mut App,
        owner: reveal_foundation::Handle<PipelineOwner>,
    ) {
        RenderViewportBase::did_attach(self, app, owner)
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        RenderViewportBase::did_detach(self, app)
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app)
    }

    fn is_repaint_boundary(self: RenderHandle<Self>, _app: &App) -> bool {
        let _ = self;
        true
    }

    fn interface(self: RenderHandle<Self>, id: TypeId) -> Option<Box<dyn Any>> {
        (id == TypeId::of::<AnyRenderAbstractViewport>())
            .then(|| Box::new(self.as_abstract_viewport()) as Box<dyn Any>)
    }

    fn sized_by_parent(self: RenderHandle<Self>, _app: &App) -> bool {
        let _ = self;
        true
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        // Ignore the return value of apply_viewport_dimension because we are doing a layout
        // regardless.
        let offset = self.offset(app);
        let size = self.size(app);
        match self.axis(app) {
            Axis::Vertical => offset.apply_viewport_dimension(app, size.height()),
            Axis::Horizontal => offset.apply_viewport_dimension(app, size.width()),
        };

        let Some(center) = self.center(app) else {
            debug_assert!(self.first_child(app).is_none());
            let this = self.get_mut(app);
            this.min_scroll_extent = 0.0;
            this.max_scroll_extent = 0.0;
            this.has_visual_overflow = false;
            offset.apply_content_dimensions(app, 0.0, 0.0);
            return;
        };
        debug_assert_eq!(center.as_object().parent(app), Some(self.as_object()));

        let (main_axis_extent, cross_axis_extent) = match self.axis(app) {
            Axis::Vertical => (size.height(), size.width()),
            Axis::Horizontal => (size.width(), size.height()),
        };

        let center_offset_adjustment = center.center_offset_adjustment(app);
        let max_layout_cycles = RenderViewport::MAX_LAYOUT_CYCLES_PER_CHILD * self.child_count(app);

        let anchor = self.get(app).anchor;
        let mut count = 0;
        loop {
            let correction = self.attempt_layout(
                app,
                main_axis_extent,
                cross_axis_extent,
                offset.pixels(app) + center_offset_adjustment,
            );
            if correction != 0.0 {
                offset.correct_by(app, correction);
            } else {
                let min_scroll_extent = self.get(app).min_scroll_extent;
                let max_scroll_extent = self.get(app).max_scroll_extent;
                if offset.apply_content_dimensions(
                    app,
                    (min_scroll_extent + main_axis_extent * anchor).min(0.0),
                    (max_scroll_extent - main_axis_extent * (1.0 - anchor)).max(0.0),
                ) {
                    break;
                }
            }
            count += 1;
            if count >= max_layout_cycles {
                break;
            }
        }
        debug_assert!(
            count < max_layout_cycles,
            "A RenderViewport exceeded its maximum number of layout cycles.\n\
             RenderViewport render objects, during layout, can retry if either their slivers or \
             their ViewportOffset decide that the offset should be corrected to take into \
             account information collected during that layout.\n\
             In the case of this RenderViewport object, however, this happened {count} times and \
             still there was no consensus on the scroll offset. This usually indicates a bug."
        );
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderViewportBase::paint(self, app, context, offset)
    }

    fn show_on_screen(
        self: RenderHandle<Self>,
        app: &mut App,
        descendant: Option<AnyRenderObject>,
        rect: Option<Rect>,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) {
        RenderViewportBase::show_on_screen(self, app, descendant, rect, duration, curve)
    }
}

impl RenderBox for RenderViewport {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<SliverPhysicalContainerParentData>(app) {
            child.set_parent_data(app, SliverPhysicalContainerParentData::new());
        }
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        // Hit test logic relies on this always providing an invertible matrix.
        child
            .parent_data_of::<SliverPhysicalContainerParentData>(app)
            .apply_paint_transform(transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderViewportBase::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderViewportBase::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderViewportBase::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderViewportBase::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderViewportBase::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        debug_assert!(debug_check_has_bounded_axis(self.axis(app), constraints));
        constraints.biggest()
    }
}

/// A render object that is bigger on the inside and shrink wraps its children in the main axis.
///
/// [`RenderShrinkWrappingViewport`] displays a subset of its children according to its own
/// dimensions and the given [`RenderViewportBase::offset`]. As the offset varies, different
/// children are visible through the viewport.
///
/// It differs from [`RenderViewport`] in that [`RenderViewport`] expands to fill the main axis
/// whereas this shrink-wraps itself to match its children in the main axis. This shrink wrapping
/// behavior is expensive because the children, and hence the viewport, could potentially change
/// size whenever the offset changes (e.g., because of a collapsing header).
///
/// [`RenderShrinkWrappingViewport`] cannot contain [`RenderBox`] children directly. Instead, use
/// a [`crate::RenderSliverList`], [`crate::RenderSliverFixedExtentList`], or a
/// [`crate::RenderSliverToBoxAdapter`], for example.
pub struct RenderShrinkWrappingViewport {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderSliver>,
    viewport: RenderViewportBaseData,
    // Out-of-band data computed during layout.
    max_scroll_extent: f64,
    shrink_wrap_extent: f64,
    has_visual_overflow: bool,
}

impl RenderShrinkWrappingViewport {
    /// Creates a viewport (for [`AnyRenderSliver`] objects) that shrink-wraps its contents.
    ///
    /// For testing purposes, consider passing a [`crate::FixedViewportOffset`] as the `offset`.
    pub fn new(
        app: &mut App,
        cross_axis_direction: AxisDirection,
        offset: AnyViewportOffset,
        children: Option<Vec<AnyRenderSliver>>,
    ) -> RenderHandle<RenderShrinkWrappingViewport> {
        let this = RenderHandle::new_box(
            app,
            RenderShrinkWrappingViewport {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                viewport: RenderViewportBaseData::new(cross_axis_direction, offset),
                max_scroll_extent: 0.0,
                shrink_wrap_extent: 0.0,
                has_visual_overflow: false,
            },
        );
        this.add_all(app, children);
        this
    }

    /// Flutter's `_debugCheckHasBoundedCrossAxis`.
    fn debug_check_has_bounded_cross_axis(self: RenderHandle<Self>, app: &App) -> bool {
        let constraints = self.constraints(app);
        match self.axis(app) {
            Axis::Vertical => debug_assert!(
                constraints.has_bounded_width(),
                "Vertical viewport was given unbounded width.\n\
                 Viewports expand in the cross axis to fill their container and constrain their \
                 children to match their extent in the cross axis. In this case, a vertical \
                 shrinkwrapping viewport was given an unlimited amount of horizontal space in \
                 which to expand."
            ),
            Axis::Horizontal => debug_assert!(
                constraints.has_bounded_height(),
                "Horizontal viewport was given unbounded height.\n\
                 Viewports expand in the cross axis to fill their container and constrain their \
                 children to match their extent in the cross axis. In this case, a horizontal \
                 shrinkwrapping viewport was given an unlimited amount of vertical space in \
                 which to expand."
            ),
        }
        true
    }

    /// Flutter's `_attemptLayout`.
    fn attempt_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        main_axis_extent: f64,
        cross_axis_extent: f64,
        corrected_offset: f64,
    ) -> f64 {
        // We can't assert main_axis_extent is finite, because it could be infinite if it is
        // within a column or row for example. In such a case, there's not even any scrolling to
        // do, although some scroll physics could still temporarily scroll the content in a
        // simulation.
        debug_assert!(!main_axis_extent.is_nan());
        debug_assert!(main_axis_extent >= 0.0);
        debug_assert!(cross_axis_extent.is_finite());
        debug_assert!(cross_axis_extent >= 0.0);
        debug_assert!(corrected_offset.is_finite());
        let this = self.get_mut(app);
        this.max_scroll_extent = 0.0;
        this.shrink_wrap_extent = 0.0;
        // Since the viewport is shrinkwrapped, we know that any negative overscroll into the
        // potentially infinite main_axis_extent will overflow the end of the viewport.
        this.has_visual_overflow = corrected_offset < 0.0;
        let calculated_cache_extent = if main_axis_extent.is_finite() {
            self.scroll_cache_extent(app)
                .calculate_cache_offset(main_axis_extent)
        } else {
            // If main_axis_extent is infinite, it builds everything anyway, so we don't need any
            // extra cache.
            0.0
        };
        self.viewport_data_mut(app).calculated_cache_extent = Some(calculated_cache_extent);

        let first_child = self.first_child(app);
        self.layout_child_sequence(
            app,
            first_child,
            corrected_offset.max(0.0),
            corrected_offset.min(0.0),
            (-corrected_offset).max(0.0),
            main_axis_extent + corrected_offset.min(0.0),
            main_axis_extent,
            cross_axis_extent,
            GrowthDirection::Forward,
            &|this, app, child| this.child_after(app, child),
            main_axis_extent + 2.0 * calculated_cache_extent,
            -calculated_cache_extent,
        )
    }
}

impl ContainerRenderObjectMixin for RenderShrinkWrappingViewport {
    type ChildType = AnyRenderSliver;
    type ParentDataType = SliverLogicalContainerParentData;

    fn container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &ContainerRenderObjectData<AnyRenderSliver> {
        &self.get(app).container
    }

    fn container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut ContainerRenderObjectData<AnyRenderSliver> {
        &mut self.get_mut(app).container
    }
}

impl RenderViewportBase for RenderShrinkWrappingViewport {
    crate::render_viewport_base_accessors!();

    fn debug_throw_if_not_checking_intrinsics(self: RenderHandle<Self>) -> bool {
        let _ = self;
        debug_assert!(
            debug_checking_intrinsics(),
            "{} does not support returning intrinsic dimensions.\n\
             Calculating the intrinsic dimensions would require instantiating every child of \
             the viewport, which defeats the point of viewports being lazy.\n\
             If you are merely trying to shrink-wrap the viewport in the main axis direction, \
             you should be able to achieve that effect by just giving the viewport loose \
             constraints, without needing to measure its intrinsic dimensions.",
            std::any::type_name::<Self>()
        );
        true
    }

    fn has_visual_overflow(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).has_visual_overflow
    }

    fn update_out_of_band_data(
        self: RenderHandle<Self>,
        app: &mut App,
        growth_direction: GrowthDirection,
        child_layout_geometry: SliverGeometry,
    ) {
        debug_assert_eq!(growth_direction, GrowthDirection::Forward);
        let this = self.get_mut(app);
        this.max_scroll_extent += child_layout_geometry.scroll_extent;
        if child_layout_geometry.has_visual_overflow {
            this.has_visual_overflow = true;
        }
        this.shrink_wrap_extent += child_layout_geometry.max_paint_extent;
    }

    fn update_child_layout_offset(
        self: RenderHandle<Self>,
        app: &mut App,
        child: AnyRenderSliver,
        layout_offset: f64,
        growth_direction: GrowthDirection,
    ) {
        debug_assert_eq!(growth_direction, GrowthDirection::Forward);
        child
            .as_object()
            .parent_data_of_mut::<SliverLogicalContainerParentData>(app)
            .set_layout_offset(Some(layout_offset));
    }

    fn paint_offset_of(self: RenderHandle<Self>, app: &App, child: AnyRenderSliver) -> Offset {
        let layout_offset = child
            .as_object()
            .parent_data_of::<SliverLogicalContainerParentData>(app)
            .layout_offset()
            .expect("a laid-out child has a layout offset");
        self.compute_absolute_paint_offset(app, child, layout_offset, GrowthDirection::Forward)
    }

    fn scroll_offset_of(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderSliver,
        scroll_offset_within_child: f64,
    ) -> f64 {
        debug_assert_eq!(child.as_object().parent(app), Some(self.as_object()));
        debug_assert_eq!(
            child.constraints(app).growth_direction,
            GrowthDirection::Forward
        );
        let mut scroll_offset_to_child = 0.0;
        let mut current = self.first_child(app);
        while current != Some(child) {
            let sliver = current.expect("child is in the child list");
            scroll_offset_to_child += sliver.geometry(app).scroll_extent;
            current = self.child_after(app, sliver);
        }
        scroll_offset_to_child + scroll_offset_within_child
    }

    fn max_scroll_obstruction_extent_before(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderSliver,
    ) -> f64 {
        debug_assert_eq!(child.as_object().parent(app), Some(self.as_object()));
        debug_assert_eq!(
            child.constraints(app).growth_direction,
            GrowthDirection::Forward
        );
        let mut pinned_extent = 0.0;
        let mut current = self.first_child(app);
        while current != Some(child) {
            let sliver = current.expect("child is in the child list");
            pinned_extent += sliver.geometry(app).max_scroll_obstruction_extent;
            current = self.child_after(app, sliver);
        }
        pinned_extent
    }

    fn compute_child_main_axis_position(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderSliver,
        parent_main_axis_position: f64,
    ) -> f64 {
        debug_assert!(self.has_size(app));
        let layout_offset = child
            .as_object()
            .parent_data_of::<SliverLogicalContainerParentData>(app)
            .layout_offset()
            .expect("a laid-out child has a layout offset");
        let constraints = child.constraints(app);
        let size = self.size(app);
        match apply_growth_direction_to_axis_direction(
            constraints.axis_direction,
            constraints.growth_direction,
        ) {
            AxisDirection::Down | AxisDirection::Right => parent_main_axis_position - layout_offset,
            AxisDirection::Up => size.height() - parent_main_axis_position - layout_offset,
            AxisDirection::Left => size.width() - parent_main_axis_position - layout_offset,
        }
    }

    fn index_of_first_child(self: RenderHandle<Self>, _app: &App) -> i32 {
        let _ = self;
        0
    }

    fn label_for_child(self: RenderHandle<Self>, index: i32) -> String {
        let _ = self;
        format!("child {index}")
    }
}

impl RenderAbstractViewport for RenderShrinkWrappingViewport {
    fn get_offset_to_reveal(
        self: RenderHandle<Self>,
        app: &App,
        target: AnyRenderObject,
        alignment: f64,
        rect: Option<Rect>,
        axis: Option<Axis>,
    ) -> RevealedOffset {
        RenderViewportBase::get_offset_to_reveal(self, app, target, alignment, rect, axis)
    }
}

impl RenderObject for RenderShrinkWrappingViewport {
    crate::render_object_accessors!();

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(
        self: RenderHandle<Self>,
        app: &mut App,
        owner: reveal_foundation::Handle<PipelineOwner>,
    ) {
        RenderViewportBase::did_attach(self, app, owner)
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        RenderViewportBase::did_detach(self, app)
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app)
    }

    fn is_repaint_boundary(self: RenderHandle<Self>, _app: &App) -> bool {
        let _ = self;
        true
    }

    fn interface(self: RenderHandle<Self>, id: TypeId) -> Option<Box<dyn Any>> {
        (id == TypeId::of::<AnyRenderAbstractViewport>())
            .then(|| Box::new(self.as_abstract_viewport()) as Box<dyn Any>)
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let offset = self.offset(app);
        if self.first_child(app).is_none() {
            // Shrinkwrapping viewport only requires the cross axis to be bounded.
            debug_assert!(self.debug_check_has_bounded_cross_axis(app));
            let size = match self.axis(app) {
                Axis::Vertical => Size::new(constraints.max_width, constraints.min_height),
                Axis::Horizontal => Size::new(constraints.min_width, constraints.max_height),
            };
            self.set_size(app, size);
            offset.apply_viewport_dimension(app, 0.0);
            let this = self.get_mut(app);
            this.max_scroll_extent = 0.0;
            this.shrink_wrap_extent = 0.0;
            this.has_visual_overflow = false;
            offset.apply_content_dimensions(app, 0.0, 0.0);
            return;
        }

        // Shrinkwrapping viewport only requires the cross axis to be bounded.
        debug_assert!(self.debug_check_has_bounded_cross_axis(app));
        let (main_axis_extent, cross_axis_extent) = match self.axis(app) {
            Axis::Vertical => (constraints.max_height, constraints.max_width),
            Axis::Horizontal => (constraints.max_width, constraints.max_height),
        };

        let effective_extent;
        loop {
            let corrected_offset = offset.pixels(app);
            let correction =
                self.attempt_layout(app, main_axis_extent, cross_axis_extent, corrected_offset);
            if correction != 0.0 {
                offset.correct_by(app, correction);
                continue;
            }
            let shrink_wrap_extent = self.get(app).shrink_wrap_extent;
            let extent = match self.axis(app) {
                Axis::Vertical => constraints.constrain_height(shrink_wrap_extent),
                Axis::Horizontal => constraints.constrain_width(shrink_wrap_extent),
            };
            let did_accept_viewport_dimension = offset.apply_viewport_dimension(app, extent);
            let max_scroll_extent = self.get(app).max_scroll_extent;
            let did_accept_content_dimension =
                offset.apply_content_dimensions(app, 0.0, (max_scroll_extent - extent).max(0.0));
            if did_accept_viewport_dimension && did_accept_content_dimension {
                effective_extent = extent;
                break;
            }
        }
        let size = match self.axis(app) {
            Axis::Vertical => constraints.constrain_dimensions(cross_axis_extent, effective_extent),
            Axis::Horizontal => {
                constraints.constrain_dimensions(effective_extent, cross_axis_extent)
            }
        };
        self.set_size(app, size);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderViewportBase::paint(self, app, context, offset)
    }

    fn show_on_screen(
        self: RenderHandle<Self>,
        app: &mut App,
        descendant: Option<AnyRenderObject>,
        rect: Option<Rect>,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) {
        RenderViewportBase::show_on_screen(self, app, descendant, rect, duration, curve)
    }
}

impl RenderBox for RenderShrinkWrappingViewport {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<SliverLogicalContainerParentData>(app) {
            child.set_parent_data(app, SliverLogicalContainerParentData::new());
        }
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        // Hit test logic relies on this always providing an invertible matrix.
        let offset = self.paint_offset_of(
            app,
            child
                .as_sliver()
                .expect("a viewport's children are slivers"),
        );
        translate(transform, offset.dx(), offset.dy());
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderViewportBase::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderViewportBase::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderViewportBase::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderViewportBase::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderViewportBase::compute_max_intrinsic_height(self, app, width)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use reveal_foundation::Handle;
    use reveal_gestures::HitTestResult;

    use super::*;
    use crate::box_::BoxConstraints;
    use crate::layer::CompositedLayer;
    use crate::proxy_box::RenderConstrainedBox;
    use crate::sliver::{RenderSliver, RenderSliverToBoxAdapter};
    use crate::viewport_offset::{FixedViewportOffset, ViewportOffset};

    /// A sliver of a fixed scroll extent that records the offsets it was painted at.
    struct TestSliver {
        render_object: RenderObjectData,
        render_sliver: crate::sliver::RenderSliverData,
        extent: f64,
        paint_offsets: Rc<Cell<Vec<Offset>>>,
    }

    impl RenderObject for TestSliver {
        crate::render_object_accessors!();

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            let constraints = self.constraints(app);
            let extent = self.get(app).extent;
            let painted = self.calculate_paint_offset(app, constraints, 0.0, extent);
            let cached = self.calculate_cache_offset(app, constraints, 0.0, extent);
            self.set_geometry(
                app,
                SliverGeometry::new()
                    .scroll_extent(extent)
                    .paint_extent(painted)
                    .cache_extent(cached)
                    .max_paint_extent(extent),
            );
        }

        fn paint(
            self: RenderHandle<Self>,
            app: &mut App,
            _context: &mut PaintingContext,
            offset: Offset,
        ) {
            let recorded = self.get(app).paint_offsets.clone();
            let mut offsets = recorded.take();
            offsets.push(offset);
            recorded.set(offsets);
        }
    }

    impl RenderSliver for TestSliver {
        crate::render_sliver_accessors!();

        fn hit_test_self(
            self: RenderHandle<Self>,
            _app: &App,
            _main_axis_position: f64,
            _cross_axis_position: f64,
        ) -> bool {
            let _ = self;
            true
        }
    }

    fn test_sliver(app: &mut App, extent: f64) -> (AnyRenderSliver, Rc<Cell<Vec<Offset>>>) {
        let paint_offsets = Rc::new(Cell::new(Vec::new()));
        let sliver = RenderHandle::new_sliver(
            app,
            TestSliver {
                render_object: RenderObjectData::new(),
                render_sliver: crate::sliver::RenderSliverData::new(),
                extent,
                paint_offsets: paint_offsets.clone(),
            },
        )
        .as_sliver();
        (sliver, paint_offsets)
    }

    /// The test binding's first frame: root the tree, lay it out, and flush paint.
    fn first_frame(
        app: &mut App,
        root: crate::box_::AnyRenderBox,
        constraints: BoxConstraints,
    ) -> Handle<PipelineOwner> {
        let owner = PipelineOwner::new(app, None);
        owner.set_root_node(app, Some(root.as_object()));
        root.layout(app, constraints, false);
        root.as_object()
            .schedule_initial_paint(app, CompositedLayer::default());
        owner.flush_paint(app);
        owner
    }

    fn vertical_viewport(
        app: &mut App,
        offset: Handle<FixedViewportOffset>,
        children: Vec<AnyRenderSliver>,
    ) -> RenderHandle<RenderViewport> {
        RenderViewport::new(
            app,
            AxisDirection::Right,
            offset.as_viewport_offset(),
            Some(children),
            None,
        )
    }

    /// `viewport_test.dart`: a viewport does not support intrinsics, because answering would
    /// mean instantiating every child.
    #[test]
    #[should_panic(expected = "does not support returning intrinsic dimensions")]
    fn a_viewport_has_no_intrinsic_dimensions() {
        let mut app = App::new();
        let offset = FixedViewportOffset::zero(&mut app);
        let viewport = vertical_viewport(&mut app, offset, vec![]);
        viewport.as_box().get_min_intrinsic_width(&mut app, 0.0);
    }

    /// While the framework is checking intrinsics, a viewport answers zero instead of panicking.
    #[test]
    fn a_viewport_reports_zero_intrinsics_while_checking_them() {
        let mut app = App::new();
        let offset = FixedViewportOffset::zero(&mut app);
        let viewport = vertical_viewport(&mut app, offset, vec![]);
        crate::object::set_debug_checking_intrinsics(true);
        let width = viewport.as_box().get_min_intrinsic_width(&mut app, 0.0);
        let height = viewport.as_box().get_max_intrinsic_height(&mut app, 0.0);
        crate::object::set_debug_checking_intrinsics(false);
        assert_eq!((width, height), (0.0, 0.0));
    }

    /// A viewport is sized by its parent: its dry layout is the biggest size the constraints
    /// allow, and that is the size it takes.
    #[test]
    fn a_viewport_dry_layout_is_the_biggest_size() {
        let mut app = App::new();
        let (sliver, _paints) = test_sliver(&mut app, 100.0);
        let offset = FixedViewportOffset::zero(&mut app);
        let viewport = vertical_viewport(&mut app, offset, vec![sliver]);
        let constraints = BoxConstraints::new().max_width(200.0).max_height(400.0);
        let dry = viewport.as_box().get_dry_layout(&mut app, constraints);
        first_frame(&mut app, viewport.as_box(), constraints);
        assert_eq!(dry, Size::new(200.0, 400.0));
        assert_eq!(dry, viewport.size(&app));
    }

    /// `viewport_test.dart`: the slivers are laid out one after the other from the leading edge,
    /// and the viewport reports the total scroll extent to its offset.
    #[test]
    fn a_viewport_lays_its_slivers_out_in_order() {
        let mut app = App::new();
        let (first, first_paints) = test_sliver(&mut app, 100.0);
        let (second, second_paints) = test_sliver(&mut app, 300.0);
        let offset = FixedViewportOffset::zero(&mut app);
        let viewport = vertical_viewport(&mut app, offset, vec![first, second]);

        first_frame(
            &mut app,
            viewport.as_box(),
            BoxConstraints::tight(Size::new(200.0, 400.0)),
        );

        assert_eq!(viewport.size(&app), Size::new(200.0, 400.0));
        assert_eq!(viewport.center(&app), Some(first));
        assert_eq!(first.geometry(&app).paint_extent, 100.0);
        assert_eq!(second.geometry(&app).paint_extent, 300.0);
        assert_eq!(viewport.paint_offset_of(&app, first), Offset::ZERO);
        assert_eq!(
            viewport.paint_offset_of(&app, second),
            Offset::new(0.0, 100.0)
        );
        assert_eq!(first_paints.take(), vec![Offset::ZERO]);
        assert_eq!(second_paints.take(), vec![Offset::new(0.0, 100.0)]);
    }

    /// A non-zero offset scrolls the content: the first sliver is partly scrolled off and the
    /// second moves up by the same amount.
    #[test]
    fn a_scrolled_viewport_moves_its_slivers() {
        let mut app = App::new();
        let (first, _) = test_sliver(&mut app, 100.0);
        let (second, _) = test_sliver(&mut app, 300.0);
        let offset = FixedViewportOffset::new(&mut app, 60.0);
        let viewport = vertical_viewport(&mut app, offset, vec![first, second]);

        first_frame(
            &mut app,
            viewport.as_box(),
            BoxConstraints::tight(Size::new(200.0, 400.0)),
        );

        assert_eq!(first.constraints(&app).scroll_offset, 60.0);
        assert_eq!(first.geometry(&app).paint_extent, 40.0);
        assert_eq!(viewport.paint_offset_of(&app, first), Offset::ZERO);
        assert_eq!(
            viewport.paint_offset_of(&app, second),
            Offset::new(0.0, 40.0)
        );
        assert_eq!(viewport.scroll_offset_of(&app, second, 0.0), 100.0);
    }

    /// A hit lands on the sliver that covers the position, in the viewport's coordinate system.
    #[test]
    fn a_viewport_hit_tests_the_sliver_under_the_position() {
        let mut app = App::new();
        let (first, _) = test_sliver(&mut app, 100.0);
        let (second, _) = test_sliver(&mut app, 300.0);
        let offset = FixedViewportOffset::zero(&mut app);
        let viewport = vertical_viewport(&mut app, offset, vec![first, second]);
        first_frame(
            &mut app,
            viewport.as_box(),
            BoxConstraints::tight(Size::new(200.0, 400.0)),
        );

        let mut result = HitTestResult::new();
        let hit = viewport.hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            Offset::new(20.0, 150.0),
        );

        assert!(hit);
        assert_eq!(result.path().len(), 2);
        // The viewport and the second sliver, which starts at 100.
        assert_eq!(
            viewport.compute_child_main_axis_position(&app, second, 150.0),
            50.0
        );
    }

    /// The cache extent widens the region slivers are asked to lay out for.
    #[test]
    fn the_cache_extent_widens_the_laid_out_region() {
        let mut app = App::new();
        let (sliver, _) = test_sliver(&mut app, 1000.0);
        let offset = FixedViewportOffset::zero(&mut app);
        let viewport = vertical_viewport(&mut app, offset, vec![sliver]);
        viewport.set_scroll_cache_extent(&mut app, Some(ScrollCacheExtent::Pixels(100.0)));

        first_frame(
            &mut app,
            viewport.as_box(),
            BoxConstraints::tight(Size::new(200.0, 400.0)),
        );

        assert_eq!(sliver.constraints(&app).remaining_cache_extent, 500.0);
        assert_eq!(sliver.geometry(&app).cache_extent, 500.0);
        assert_eq!(sliver.geometry(&app).paint_extent, 400.0);

        viewport.set_scroll_cache_extent(&mut app, Some(ScrollCacheExtent::Viewport(0.5)));
        viewport.as_box().layout(
            &mut app,
            BoxConstraints::tight(Size::new(200.0, 400.0)),
            false,
        );
        assert_eq!(sliver.constraints(&app).remaining_cache_extent, 600.0);
    }

    /// `RenderShrinkWrappingViewport` sizes itself to the total extent of its slivers, up to the
    /// incoming constraints.
    #[test]
    fn a_shrink_wrapping_viewport_sizes_itself_to_its_slivers() {
        let mut app = App::new();
        let (first, _) = test_sliver(&mut app, 60.0);
        let (second, _) = test_sliver(&mut app, 90.0);
        let offset = FixedViewportOffset::zero(&mut app);
        let viewport = RenderShrinkWrappingViewport::new(
            &mut app,
            AxisDirection::Right,
            offset.as_viewport_offset(),
            Some(vec![first, second]),
        );

        first_frame(
            &mut app,
            viewport.as_box(),
            BoxConstraints::new()
                .max_width(200.0)
                .min_width(200.0)
                .max_height(400.0),
        );

        assert_eq!(viewport.size(&app), Size::new(200.0, 150.0));
        assert_eq!(
            viewport.paint_offset_of(&app, second),
            Offset::new(0.0, 60.0)
        );
        assert_eq!(viewport.index_of_first_child(&app), 0);
        assert_eq!(viewport.label_for_child(1), "child 1");
    }

    /// A box adapter inside a viewport reaches the box protocol: the box is laid out with the
    /// viewport's cross axis extent and painted at the sliver's paint offset.
    #[test]
    fn a_viewport_hosts_a_box_through_the_adapter() {
        let mut app = App::new();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight_for(None, Some(80.0)), None);
        let adapter = RenderSliverToBoxAdapter::new(&mut app, Some(child.as_box()));
        let offset = FixedViewportOffset::zero(&mut app);
        let viewport = vertical_viewport(&mut app, offset, vec![adapter.as_sliver()]);

        first_frame(
            &mut app,
            viewport.as_box(),
            BoxConstraints::tight(Size::new(200.0, 400.0)),
        );

        assert_eq!(child.size(&app), Size::new(200.0, 80.0));
        assert_eq!(adapter.geometry(&app).scroll_extent, 80.0);
        assert_eq!(
            child.as_box().local_to_global(&app, Offset::ZERO, None),
            Offset::ZERO
        );
    }

    /// The viewport is the `RenderAbstractViewport` that `maybe_of` finds from a descendant.
    #[test]
    fn maybe_of_finds_the_enclosing_viewport() {
        let mut app = App::new();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight_for(None, Some(80.0)), None);
        let adapter = RenderSliverToBoxAdapter::new(&mut app, Some(child.as_box()));
        let offset = FixedViewportOffset::zero(&mut app);
        let viewport = vertical_viewport(&mut app, offset, vec![adapter.as_sliver()]);
        first_frame(
            &mut app,
            viewport.as_box(),
            BoxConstraints::tight(Size::new(200.0, 400.0)),
        );

        let found = AnyRenderAbstractViewport::maybe_of(&app, Some(child.as_object()));
        assert_eq!(found, Some(viewport.as_abstract_viewport()));
        assert!(AnyRenderAbstractViewport::maybe_of(&app, None).is_none());
    }

    /// `get_offset_to_reveal` reports the offset that brings a descendant box to the leading edge.
    #[test]
    fn get_offset_to_reveal_reports_the_leading_edge_offset() {
        let mut app = App::new();
        let leading =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight_for(None, Some(100.0)), None);
        let first = RenderSliverToBoxAdapter::new(&mut app, Some(leading.as_box()));
        let target =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight_for(None, Some(50.0)), None);
        let second = RenderSliverToBoxAdapter::new(&mut app, Some(target.as_box()));
        let offset = FixedViewportOffset::zero(&mut app);
        let viewport = vertical_viewport(
            &mut app,
            offset,
            vec![first.as_sliver(), second.as_sliver()],
        );
        first_frame(
            &mut app,
            viewport.as_box(),
            BoxConstraints::tight(Size::new(200.0, 400.0)),
        );

        let revealed = RenderAbstractViewport::get_offset_to_reveal(
            viewport,
            &app,
            target.as_object(),
            0.0,
            None,
            None,
        );
        assert_eq!(revealed.offset, 100.0);
        // At that offset the target sits at the leading edge of the viewport.
        assert_eq!(revealed.rect.top, 0.0);
        assert_eq!(revealed.rect.bottom, 50.0);
    }

    /// `RevealedOffset::clamp_offset` picks the nearer edge, and `None` when already visible.
    #[test]
    fn clamp_offset_picks_the_nearer_edge() {
        let leading = RevealedOffset::new(100.0, Rect::ZERO);
        let trailing = RevealedOffset::new(20.0, Rect::ZERO);
        assert_eq!(
            RevealedOffset::clamp_offset(leading, trailing, 200.0).map(|offset| offset.offset),
            Some(100.0)
        );
        assert_eq!(
            RevealedOffset::clamp_offset(leading, trailing, 0.0).map(|offset| offset.offset),
            Some(20.0)
        );
        assert!(RevealedOffset::clamp_offset(leading, trailing, 50.0).is_none());
    }

    /// The paint order decides which sliver paints last and which is hit-tested first.
    #[test]
    fn the_paint_order_reverses_the_hit_test_order() {
        let mut app = App::new();
        let (first, _) = test_sliver(&mut app, 100.0);
        let (second, _) = test_sliver(&mut app, 300.0);
        let offset = FixedViewportOffset::zero(&mut app);
        let viewport = vertical_viewport(&mut app, offset, vec![first, second]);
        first_frame(
            &mut app,
            viewport.as_box(),
            BoxConstraints::tight(Size::new(200.0, 400.0)),
        );

        assert_eq!(viewport.children_in_paint_order(&app), vec![second, first]);
        assert_eq!(
            viewport.children_in_hit_test_order(&app),
            vec![first, second]
        );

        viewport.set_paint_order(&mut app, SliverPaintOrder::LastIsTop);
        assert_eq!(viewport.children_in_paint_order(&app), vec![first, second]);
        assert_eq!(
            viewport.children_in_hit_test_order(&app),
            vec![second, first]
        );
    }
}
