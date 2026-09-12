//! Flutter counterpart: `widgets/viewport.dart`.
//!
//! `_ViewportElement.debugVisitOnstageChildren` waits with the other diagnostics; see
//! `PORTING.md`.

use std::fmt::Debug;

use inset_embedder::Clip;
use inset_foundation::{App, Handle};
use inset_painting::{AxisDirection, text_direction_to_axis_direction};
use inset_rendering::{
    AnyRenderObject, AnyRenderSliver, AnyViewportOffset, CacheExtentStyle, ErasedRenderObject,
    RenderBox, RenderHandle, RenderShrinkWrappingViewport, RenderViewport, RenderViewportBase,
    ScrollCacheExtent, SliverPaintOrder,
};

use crate::framework::{
    AnyElement, BuildContext, Element, ElementData, KeyRef, MultiChildRenderObjectElementBase,
    MultiChildRenderObjectElementData, MultiChildRenderObjectWidget, RenderObjectElement,
    RenderObjectElementData, RenderObjectElementWidget, RenderObjectWidget, Slot, WidgetRef,
    downcast_widget, keys_equal,
};
use crate::widgets::basic::Directionality;
use crate::widgets::scroll_notification::ViewportElementMixin;

/// A widget through which a portion of larger content can be viewed, typically in combination
/// with a `Scrollable`.
///
/// [`Viewport`] is the visual workhorse of the scrolling machinery. It displays a subset of its
/// children according to its own dimensions and the given [`offset`](Self::offset). As the offset
/// varies, different children are visible through the viewport.
///
/// [`Viewport`] hosts a bidirectional list of slivers, anchored on a [`center`](Self::center)
/// sliver, which is placed at the zero scroll offset. The center widget is displayed in the
/// viewport according to the [`anchor`](Self::anchor) property.
///
/// Slivers that are earlier in the child list than [`center`](Self::center) are displayed in
/// reverse order in the reverse [`axis_direction`](Self::axis_direction) starting from the
/// [`center`](Self::center). For example, if the axis direction is [`AxisDirection::Down`], the
/// first sliver before the center is placed above the center. The slivers that are later in the
/// child list than the center are placed in order in the axis direction.
///
/// [`Viewport`] cannot contain box children directly. Instead, use a `SliverList`,
/// `SliverFixedExtentList`, or a `SliverToBoxAdapter`, for example.
///
/// See also:
///
///  * `SliverToBoxAdapter`, which allows a box widget to be placed inside a sliver context (the
///    opposite of this widget).
///  * [`ShrinkWrappingViewport`], a variant of [`Viewport`] that shrink-wraps its contents along
///    the main axis.
///  * `ViewportElementMixin`, which should be mixed in to the `Element` type used by
///    viewport-like widgets to correctly handle scroll notifications.
#[derive(Debug)]
pub struct Viewport {
    pub key: Option<KeyRef>,
    /// The direction in which the [`offset`](Self::offset)'s `ViewportOffset::pixels` increases.
    ///
    /// For example, if the axis direction is [`AxisDirection::Down`], a scroll offset of zero is
    /// at the top of the viewport and increases towards the bottom of the viewport.
    pub axis_direction: AxisDirection,
    /// The direction in which child should be laid out in the cross axis.
    ///
    /// If the [`axis_direction`](Self::axis_direction) is [`AxisDirection::Down`] or
    /// [`AxisDirection::Up`], this property defaults to [`AxisDirection::Left`] if the ambient
    /// [`Directionality`] is right-to-left and [`AxisDirection::Right`] if it is left-to-right.
    ///
    /// If the axis direction is [`AxisDirection::Left`] or [`AxisDirection::Right`], this
    /// property defaults to [`AxisDirection::Down`].
    pub cross_axis_direction: Option<AxisDirection>,
    /// The relative position of the zero scroll offset.
    ///
    /// For example, if the anchor is 0.5 and the [`axis_direction`](Self::axis_direction) is
    /// [`AxisDirection::Down`] or [`AxisDirection::Up`], then the zero scroll offset is
    /// vertically centered within the viewport. If the anchor is 1.0, and the axis direction is
    /// [`AxisDirection::Right`], then the zero scroll offset is on the left edge of the viewport.
    pub anchor: f64,
    /// Which part of the content inside the viewport should be visible.
    ///
    /// The `ViewportOffset::pixels` value determines the scroll offset that the viewport uses to
    /// select which part of its content to display. As the user scrolls the viewport, this value
    /// changes, which changes the content that is displayed.
    ///
    /// Typically a `ScrollPosition`.
    pub offset: AnyViewportOffset,
    /// The first child in the `GrowthDirection::Forward` growth direction.
    ///
    /// Children after the center will be placed in the [`axis_direction`](Self::axis_direction)
    /// relative to the center. Children before it will be placed in the opposite of the axis
    /// direction relative to the center.
    ///
    /// The center must be the key of a child of the viewport.
    pub center: Option<KeyRef>,
    /// The viewport has an area before and after the visible area to cache items that are about
    /// to become visible when the user scrolls.
    ///
    /// See also:
    ///
    ///  * [`cache_extent_style`](Self::cache_extent_style), which controls the units of the cache
    ///    extent.
    #[deprecated(note = "Use scroll_cache_extent instead.")]
    pub cache_extent: Option<f64>,
    /// The unit of measurement of [`cache_extent`](Self::cache_extent).
    #[deprecated(note = "Use scroll_cache_extent instead.")]
    pub cache_extent_style: CacheExtentStyle,
    /// The amount of additional content to display and lay out around the viewport.
    pub scroll_cache_extent: Option<ScrollCacheExtent>,
    /// The order in which to paint the slivers.
    ///
    /// Defaults to [`SliverPaintOrder::FirstIsTop`].
    pub paint_order: SliverPaintOrder,
    /// Defaults to [`Clip::HardEdge`].
    pub clip_behavior: Clip,
    /// The slivers to place inside the viewport.
    pub slivers: Vec<WidgetRef>,
}

impl Viewport {
    /// Creates a widget that is bigger on the inside.
    ///
    /// The viewport listens to the [`offset`](Self::offset), which means you do not need to
    /// rebuild this widget when the offset changes.
    #[expect(deprecated, reason = "the deprecated fields have Dart's defaults")]
    pub fn new(offset: AnyViewportOffset) -> Viewport {
        Viewport {
            key: None,
            axis_direction: AxisDirection::Down,
            cross_axis_direction: None,
            anchor: 0.0,
            offset,
            center: None,
            cache_extent: None,
            cache_extent_style: CacheExtentStyle::Pixel,
            scroll_cache_extent: None,
            paint_order: SliverPaintOrder::FirstIsTop,
            clip_behavior: Clip::HardEdge,
            slivers: Vec::new(),
        }
    }

    /// Dart `Viewport(key:)`.
    pub fn key(mut self, key: KeyRef) -> Viewport {
        self.key = Some(key);
        self
    }

    /// Dart `Viewport(axisDirection:)`.
    pub fn axis_direction(mut self, axis_direction: AxisDirection) -> Viewport {
        self.axis_direction = axis_direction;
        self
    }

    /// Dart `Viewport(crossAxisDirection:)`.
    pub fn cross_axis_direction(mut self, cross_axis_direction: AxisDirection) -> Viewport {
        self.cross_axis_direction = Some(cross_axis_direction);
        self
    }

    /// Dart `Viewport(anchor:)`.
    pub fn anchor(mut self, anchor: f64) -> Viewport {
        self.anchor = anchor;
        self
    }

    /// Dart `Viewport(center:)`.
    pub fn center(mut self, center: KeyRef) -> Viewport {
        self.center = Some(center);
        self.debug_assert_center_is_a_sliver();
        self
    }

    /// Dart `Viewport(cacheExtent:)`.
    #[deprecated(note = "Use scroll_cache_extent instead.")]
    pub fn cache_extent(mut self, cache_extent: f64) -> Viewport {
        #[expect(deprecated, reason = "the deprecated field this setter writes")]
        {
            self.cache_extent = Some(cache_extent);
        }
        self
    }

    /// Dart `Viewport(cacheExtentStyle:)`.
    #[deprecated(note = "Use scroll_cache_extent instead.")]
    pub fn cache_extent_style(mut self, cache_extent_style: CacheExtentStyle) -> Viewport {
        #[expect(deprecated, reason = "the deprecated fields this setter guards")]
        {
            self.cache_extent_style = cache_extent_style;
            debug_assert!(
                self.cache_extent_style != CacheExtentStyle::Viewport
                    || self.cache_extent.is_some()
            );
        }
        self
    }

    /// Dart `Viewport(scrollCacheExtent:)`.
    pub fn scroll_cache_extent(mut self, scroll_cache_extent: ScrollCacheExtent) -> Viewport {
        self.scroll_cache_extent = Some(scroll_cache_extent);
        self
    }

    /// Dart `Viewport(paintOrder:)`.
    pub fn paint_order(mut self, paint_order: SliverPaintOrder) -> Viewport {
        self.paint_order = paint_order;
        self
    }

    /// Dart `Viewport(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> Viewport {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `Viewport(slivers:)`.
    pub fn slivers(mut self, slivers: Vec<WidgetRef>) -> Viewport {
        self.slivers = slivers;
        self.debug_assert_center_is_a_sliver();
        self
    }

    /// Dart's `assert(center == null || slivers.where((child) => child.key == center).length == 1)`.
    fn debug_assert_center_is_a_sliver(&self) {
        if cfg!(debug_assertions)
            && let Some(center) = &self.center
        {
            let matches = self
                .slivers
                .iter()
                .filter(|child| keys_equal(child.key(), Some(center)))
                .count();
            assert!(matches == 1, "the center must be the key of one sliver");
        }
    }

    /// Flutter's `_effectiveScrollCacheExtent`.
    fn effective_scroll_cache_extent(&self) -> Option<ScrollCacheExtent> {
        if self.scroll_cache_extent.is_some() {
            return self.scroll_cache_extent;
        }
        #[expect(deprecated, reason = "the deprecated fields this getter folds in")]
        let (cache_extent, cache_extent_style) = (self.cache_extent, self.cache_extent_style);
        let cache_extent = cache_extent?;
        Some(match cache_extent_style {
            CacheExtentStyle::Pixel => ScrollCacheExtent::Pixels(cache_extent),
            CacheExtentStyle::Viewport => ScrollCacheExtent::Viewport(cache_extent),
        })
    }

    /// Given a [`BuildContext`] and an [`AxisDirection`], determine the correct cross axis
    /// direction.
    ///
    /// This depends on the [`Directionality`] if the `axis_direction` is vertical; otherwise, the
    /// default cross axis direction is downwards.
    pub fn get_default_cross_axis_direction(
        app: &mut App,
        context: BuildContext,
        axis_direction: AxisDirection,
    ) -> AxisDirection {
        match axis_direction {
            AxisDirection::Up | AxisDirection::Down => {
                text_direction_to_axis_direction(Directionality::of(app, context))
            }
            AxisDirection::Right | AxisDirection::Left => AxisDirection::Down,
        }
    }

    fn resolved_cross_axis_direction(&self, app: &mut App, context: BuildContext) -> AxisDirection {
        self.cross_axis_direction.unwrap_or_else(|| {
            Viewport::get_default_cross_axis_direction(app, context, self.axis_direction)
        })
    }
}

impl RenderObjectWidget for Viewport {
    type RenderObject = RenderViewport;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let cross_axis_direction = self.resolved_cross_axis_direction(app, context);
        let viewport = RenderViewport::new(app, cross_axis_direction, self.offset, None, None);
        viewport.set_axis_direction(app, self.axis_direction);
        viewport.set_anchor(app, self.anchor);
        viewport.set_scroll_cache_extent(app, self.effective_scroll_cache_extent());
        viewport.set_paint_order(app, self.paint_order);
        viewport.set_clip_behavior(app, self.clip_behavior);
        viewport.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderViewport>,
    ) {
        let cross_axis_direction = self.resolved_cross_axis_direction(app, context);
        render_object.set_axis_direction(app, self.axis_direction);
        render_object.set_cross_axis_direction(app, cross_axis_direction);
        render_object.set_anchor(app, self.anchor);
        render_object.set_offset(app, self.offset);
        render_object.set_scroll_cache_extent(app, self.effective_scroll_cache_extent());
        render_object.set_paint_order(app, self.paint_order);
        render_object.set_clip_behavior(app, self.clip_behavior);
    }
}

impl MultiChildRenderObjectWidget for Viewport {
    fn children(&self) -> &[WidgetRef] {
        &self.slivers
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        ViewportElement::create(app, this).as_element()
    }
}

/// Dart's `_ViewportElement`: the [`Viewport`]'s element, which keeps the render object's
/// `center` in step with the child list.
pub struct ViewportElement {
    element: ElementData,
    render_object_element: RenderObjectElementData,
    multi_child: MultiChildRenderObjectElementData,
    doing_mount_or_update: bool,
    center_slot_index: Option<usize>,
}

impl ViewportElement {
    /// Creates an element that uses the given widget as its configuration.
    pub fn create(app: &mut App, widget: WidgetRef) -> Handle<ViewportElement> {
        debug_assert!(downcast_widget::<Viewport>(&*widget).is_some());
        app.create(ViewportElement {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            multi_child: MultiChildRenderObjectElementData::default(),
            doing_mount_or_update: false,
            center_slot_index: None,
        })
    }

    /// The viewport this element manages.
    pub fn render_object(self: Handle<Self>, app: &App) -> RenderHandle<RenderViewport> {
        self.typed_render_object(app)
    }

    /// Flutter's `_updateCenter`.
    fn update_center(self: Handle<Self>, app: &mut App) {
        let center = Self::widget_of(self.as_element().widget(app))
            .center
            .clone();
        let children = MultiChildRenderObjectElementBase::children(self, app);
        let render_object = self.render_object(app);
        match center {
            Some(center) => {
                let mut element_index = 0;
                for child in &children {
                    if keys_equal(child.widget(app).key(), Some(&center)) {
                        let sliver = child.render_object(app).map(AnyRenderSliver::from_object);
                        render_object.set_center(app, sliver);
                        break;
                    }
                    element_index += 1;
                }
                debug_assert!(element_index < children.len());
                app.get_mut(self).center_slot_index = Some(element_index);
            }
            None if !children.is_empty() => {
                let sliver = children[0]
                    .render_object(app)
                    .map(AnyRenderSliver::from_object);
                render_object.set_center(app, sliver);
                app.get_mut(self).center_slot_index = Some(0);
            }
            None => {
                render_object.set_center(app, None);
                app.get_mut(self).center_slot_index = None;
            }
        }
    }
}

impl RenderObjectElementWidget for ViewportElement {
    type Widget = Viewport;

    fn widget_of(widget: &WidgetRef) -> &Viewport {
        downcast_widget::<Viewport>(&**widget).expect("a ViewportElement holds its Viewport")
    }
}

impl RenderObjectElement for ViewportElement {
    fn render_object_element_data(self: Handle<Self>, app: &App) -> &RenderObjectElementData {
        &app.get(self).render_object_element
    }

    fn render_object_element_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectElementData {
        &mut app.get_mut(self).render_object_element
    }
}

impl MultiChildRenderObjectElementBase for ViewportElement {
    fn multi_child_data(self: Handle<Self>, app: &App) -> &MultiChildRenderObjectElementData {
        &app.get(self).multi_child
    }

    fn multi_child_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut MultiChildRenderObjectElementData {
        &mut app.get_mut(self).multi_child
    }
}

impl ViewportElementMixin for ViewportElement {}

impl Element for ViewportElement {
    crate::element_accessors!();

    const IS_RENDER_OBJECT_ELEMENT: bool = true;

    fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        self.render_object_element_data(app).render_object()
    }

    fn render_object_attaching_child(self: Handle<Self>, _app: &App) -> Option<AnyElement> {
        None
    }

    fn debug_doing_build(self: Handle<Self>, app: &App) -> bool {
        self.render_object_element_data(app).debug_doing_build()
    }

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        MultiChildRenderObjectElementBase::visit_children(self, app, visitor);
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        MultiChildRenderObjectElementBase::forget_child(self, app, child);
    }

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        debug_assert!(!app.get(self).doing_mount_or_update);
        app.get_mut(self).doing_mount_or_update = true;
        MultiChildRenderObjectElementBase::mount(self, app, parent, new_slot);
        self.update_center(app);
        debug_assert!(app.get(self).doing_mount_or_update);
        app.get_mut(self).doing_mount_or_update = false;
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        debug_assert!(!app.get(self).doing_mount_or_update);
        app.get_mut(self).doing_mount_or_update = true;
        MultiChildRenderObjectElementBase::update(self, app, new_widget);
        self.update_center(app);
        debug_assert!(app.get(self).doing_mount_or_update);
        app.get_mut(self).doing_mount_or_update = false;
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::perform_rebuild(self, app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::deactivate(self, app);
    }

    fn unmount(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::unmount(self, app);
    }

    fn update_parent_data(self: Handle<Self>, app: &mut App, parent_data_element: AnyElement) {
        RenderObjectElement::update_parent_data(self, app, parent_data_element);
    }

    fn update_slot(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::update_slot(self, app, new_slot);
    }

    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::attach_render_object(self, app, new_slot);
    }

    fn detach_render_object(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::detach_render_object(self, app);
    }

    fn insert_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        MultiChildRenderObjectElementBase::insert_render_object_child(
            self,
            app,
            child,
            slot.clone(),
        );
        // Once `mount` / `update` are done, the render object's center will be updated in
        // `update_center`.
        let index = Self::indexed_slot(slot.as_ref()).index;
        if !app.get(self).doing_mount_or_update && Some(index) == app.get(self).center_slot_index {
            let center = AnyRenderSliver::from_object(child);
            self.render_object(app).set_center(app, Some(center));
        }
    }

    fn move_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        old_slot: Option<Slot>,
        new_slot: Option<Slot>,
    ) {
        MultiChildRenderObjectElementBase::move_render_object_child(
            self, app, child, old_slot, new_slot,
        );
        debug_assert!(app.get(self).doing_mount_or_update);
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        MultiChildRenderObjectElementBase::remove_render_object_child(self, app, child, slot);
        let render_object = self.render_object(app);
        if !app.get(self).doing_mount_or_update
            && render_object.center(app) == Some(AnyRenderSliver::from_object(child))
        {
            render_object.set_center(app, None);
        }
    }

    fn attach_notification_tree(self: Handle<Self>, app: &mut App) {
        ViewportElementMixin::attach_notification_tree(self, app);
    }

    fn on_notification(
        self: Handle<Self>,
        app: &mut App,
        notification: &dyn crate::framework::Notification,
    ) -> bool {
        ViewportElementMixin::on_notification(self, app, notification)
    }
}

/// A widget that is bigger on the inside and shrink wraps its children in the main axis.
///
/// [`ShrinkWrappingViewport`] displays a subset of its children according to its own dimensions
/// and the given [`offset`](Self::offset). As the offset varies, different children are visible
/// through the viewport.
///
/// [`ShrinkWrappingViewport`] differs from [`Viewport`] in that [`Viewport`] expands to fill the
/// main axis whereas [`ShrinkWrappingViewport`] sizes itself to match its children in the main
/// axis. This shrink wrapping behavior is expensive because the children, and hence the viewport,
/// could potentially change size whenever the offset changes (e.g., because of a collapsing
/// header).
///
/// [`ShrinkWrappingViewport`] cannot contain box children directly. Instead, use a `SliverList`,
/// `SliverFixedExtentList`, or a `SliverToBoxAdapter`, for example.
///
/// See also:
///
///  * `SliverToBoxAdapter`, which allows a box widget to be placed inside a sliver context (the
///    opposite of this widget).
///  * [`Viewport`], a viewport that does not shrink-wrap its contents.
#[derive(Debug)]
pub struct ShrinkWrappingViewport {
    pub key: Option<KeyRef>,
    /// The direction in which the [`offset`](Self::offset)'s `ViewportOffset::pixels` increases.
    pub axis_direction: AxisDirection,
    /// The direction in which child should be laid out in the cross axis.
    pub cross_axis_direction: Option<AxisDirection>,
    /// Which part of the content inside the viewport should be visible.
    ///
    /// Typically a `ScrollPosition`.
    pub offset: AnyViewportOffset,
    /// The order in which to paint the slivers.
    ///
    /// Defaults to [`SliverPaintOrder::FirstIsTop`].
    pub paint_order: SliverPaintOrder,
    /// Defaults to [`Clip::HardEdge`].
    pub clip_behavior: Clip,
    /// The viewport has an area before and after the visible area to cache items that are about
    /// to become visible when the user scrolls.
    #[deprecated(note = "Use scroll_cache_extent instead.")]
    pub cache_extent: Option<f64>,
    /// The unit of measurement of [`cache_extent`](Self::cache_extent).
    #[deprecated(note = "Use scroll_cache_extent instead.")]
    pub cache_extent_style: CacheExtentStyle,
    /// The amount of additional content to display and lay out around the viewport.
    pub scroll_cache_extent: Option<ScrollCacheExtent>,
    /// The slivers to place inside the viewport.
    pub slivers: Vec<WidgetRef>,
}

impl ShrinkWrappingViewport {
    /// Creates a widget that is bigger on the inside and shrink wraps its children in the main
    /// axis.
    ///
    /// The viewport listens to the [`offset`](Self::offset), which means you do not need to
    /// rebuild this widget when the offset changes.
    #[expect(deprecated, reason = "the deprecated fields have Dart's defaults")]
    pub fn new(offset: AnyViewportOffset) -> ShrinkWrappingViewport {
        ShrinkWrappingViewport {
            key: None,
            axis_direction: AxisDirection::Down,
            cross_axis_direction: None,
            offset,
            paint_order: SliverPaintOrder::FirstIsTop,
            clip_behavior: Clip::HardEdge,
            cache_extent: None,
            cache_extent_style: CacheExtentStyle::Pixel,
            scroll_cache_extent: None,
            slivers: Vec::new(),
        }
    }

    /// Dart `ShrinkWrappingViewport(key:)`.
    pub fn key(mut self, key: KeyRef) -> ShrinkWrappingViewport {
        self.key = Some(key);
        self
    }

    /// Dart `ShrinkWrappingViewport(axisDirection:)`.
    pub fn axis_direction(mut self, axis_direction: AxisDirection) -> ShrinkWrappingViewport {
        self.axis_direction = axis_direction;
        self
    }

    /// Dart `ShrinkWrappingViewport(crossAxisDirection:)`.
    pub fn cross_axis_direction(
        mut self,
        cross_axis_direction: AxisDirection,
    ) -> ShrinkWrappingViewport {
        self.cross_axis_direction = Some(cross_axis_direction);
        self
    }

    /// Dart `ShrinkWrappingViewport(paintOrder:)`.
    pub fn paint_order(mut self, paint_order: SliverPaintOrder) -> ShrinkWrappingViewport {
        self.paint_order = paint_order;
        self
    }

    /// Dart `ShrinkWrappingViewport(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> ShrinkWrappingViewport {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `ShrinkWrappingViewport(cacheExtent:)`.
    #[deprecated(note = "Use scroll_cache_extent instead.")]
    pub fn cache_extent(mut self, cache_extent: f64) -> ShrinkWrappingViewport {
        #[expect(deprecated, reason = "the deprecated field this setter writes")]
        {
            self.cache_extent = Some(cache_extent);
        }
        self
    }

    /// Dart `ShrinkWrappingViewport(cacheExtentStyle:)`.
    #[deprecated(note = "Use scroll_cache_extent instead.")]
    pub fn cache_extent_style(
        mut self,
        cache_extent_style: CacheExtentStyle,
    ) -> ShrinkWrappingViewport {
        #[expect(deprecated, reason = "the deprecated field this setter writes")]
        {
            self.cache_extent_style = cache_extent_style;
        }
        self
    }

    /// Dart `ShrinkWrappingViewport(scrollCacheExtent:)`.
    pub fn scroll_cache_extent(
        mut self,
        scroll_cache_extent: ScrollCacheExtent,
    ) -> ShrinkWrappingViewport {
        self.scroll_cache_extent = Some(scroll_cache_extent);
        self
    }

    /// Dart `ShrinkWrappingViewport(slivers:)`.
    pub fn slivers(mut self, slivers: Vec<WidgetRef>) -> ShrinkWrappingViewport {
        self.slivers = slivers;
        self
    }

    /// Flutter's `_effectiveScrollCacheExtent`.
    fn effective_scroll_cache_extent(&self) -> Option<ScrollCacheExtent> {
        if self.scroll_cache_extent.is_some() {
            return self.scroll_cache_extent;
        }
        #[expect(deprecated, reason = "the deprecated fields this getter folds in")]
        let (cache_extent, cache_extent_style) = (self.cache_extent, self.cache_extent_style);
        let cache_extent = cache_extent?;
        Some(match cache_extent_style {
            CacheExtentStyle::Pixel => ScrollCacheExtent::Pixels(cache_extent),
            CacheExtentStyle::Viewport => ScrollCacheExtent::Viewport(cache_extent),
        })
    }

    fn resolved_cross_axis_direction(&self, app: &mut App, context: BuildContext) -> AxisDirection {
        self.cross_axis_direction.unwrap_or_else(|| {
            Viewport::get_default_cross_axis_direction(app, context, self.axis_direction)
        })
    }
}

impl RenderObjectWidget for ShrinkWrappingViewport {
    type RenderObject = RenderShrinkWrappingViewport;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let cross_axis_direction = self.resolved_cross_axis_direction(app, context);
        let viewport =
            RenderShrinkWrappingViewport::new(app, cross_axis_direction, self.offset, None);
        viewport.set_axis_direction(app, self.axis_direction);
        viewport.set_paint_order(app, self.paint_order);
        viewport.set_clip_behavior(app, self.clip_behavior);
        viewport.set_scroll_cache_extent(app, self.effective_scroll_cache_extent());
        viewport.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderShrinkWrappingViewport>,
    ) {
        let cross_axis_direction = self.resolved_cross_axis_direction(app, context);
        render_object.set_axis_direction(app, self.axis_direction);
        render_object.set_cross_axis_direction(app, cross_axis_direction);
        render_object.set_offset(app, self.offset);
        render_object.set_paint_order(app, self.paint_order);
        render_object.set_clip_behavior(app, self.clip_behavior);
        render_object.set_scroll_cache_extent(app, self.effective_scroll_cache_extent());
    }
}

impl MultiChildRenderObjectWidget for ShrinkWrappingViewport {
    fn children(&self) -> &[WidgetRef] {
        &self.slivers
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::RefCell;
    use std::rc::Rc;

    use inset_embedder::TextDirection;
    use inset_rendering::{
        ContainerRenderObjectMixin, FixedViewportOffset, RenderObjectWithChildMixin,
        RenderPositionedBox, RenderSliverList, SliverMultiBoxAdaptorParentData, ViewportOffset,
    };

    use super::*;
    use crate::framework::IntoWidget;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Center, Directionality, SizedBox, SliverToBoxAdapter};
    use crate::widgets::sliver::SliverList;

    /// The indices the list's item builder was asked for, in order.
    type Built = Rc<RefCell<Vec<i32>>>;

    fn item_list(built: &Built, item_count: i32) -> WidgetRef {
        let built = Rc::clone(built);
        SliverList::builder(move |_app, _context, index| {
            built.borrow_mut().push(index);
            Some(SizedBox::new().height(40.0).into_widget())
        })
        .item_count(item_count)
        .into_widget()
    }

    fn viewport_tree(offset: Handle<FixedViewportOffset>, built: &Built) -> WidgetRef {
        Directionality::new(
            TextDirection::Ltr,
            Viewport::new(offset.as_viewport_offset())
                .scroll_cache_extent(ScrollCacheExtent::Pixels(0.0))
                .slivers(vec![
                    SliverToBoxAdapter::new()
                        .child(SizedBox::new().height(50.0))
                        .into_widget(),
                    item_list(built, 20),
                ]),
        )
        .into_widget()
    }

    fn render_viewport(harness: &Harness, app: &App) -> RenderHandle<RenderViewport> {
        harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .as_object()
            .downcast::<RenderViewport>(app)
            .expect("a RenderViewport")
    }

    fn sliver_list(
        viewport: RenderHandle<RenderViewport>,
        app: &App,
    ) -> RenderHandle<RenderSliverList> {
        let adapter = viewport.first_child(app).expect("the box adapter");
        viewport
            .child_after(app, adapter)
            .expect("the list")
            .as_object()
            .downcast::<RenderSliverList>(app)
            .expect("a RenderSliverList")
    }

    /// The indices of the list's reified children, in child order.
    fn child_indices(list: RenderHandle<RenderSliverList>, app: &App) -> Vec<i32> {
        let mut indices = Vec::new();
        let mut child = list.first_child(app);
        while let Some(current) = child {
            indices.push(
                current
                    .as_object()
                    .parent_data_of::<SliverMultiBoxAdaptorParentData>(app)
                    .index
                    .expect("a reified child has an index"),
            );
            child = list.child_after(app, current);
        }
        indices
    }

    #[test]
    fn a_viewport_builds_only_the_visible_children_and_more_when_the_offset_moves() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let offset = FixedViewportOffset::zero(&mut app);
        let built: Built = Rc::default();
        let harness = Harness::mount(&mut app, viewport_tree(offset, &built));
        harness.pump(&mut app);

        let viewport = render_viewport(&harness, &app);
        let list = sliver_list(viewport, &app);
        // The box adapter takes the first 50 pixels; the 200-pixel viewport fits four more
        // 40-pixel children with no cache extent.
        assert_eq!(child_indices(list, &app), vec![0, 1, 2, 3]);
        assert_eq!(*built.borrow(), vec![0, 1, 2, 3]);

        offset.correct_by(&mut app, 200.0);
        viewport.mark_needs_layout(&mut app);
        harness.pump(&mut app);

        // The window 200..400 covers the list items that start at 50: indices 3 through 8.
        assert_eq!(child_indices(list, &app), vec![3, 4, 5, 6, 7, 8]);
        // The children that scrolled off were destroyed; only the new ones were built.
        assert_eq!(*built.borrow(), vec![0, 1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn a_shrink_wrapping_viewport_sizes_itself_to_its_content() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let offset = FixedViewportOffset::zero(&mut app);
        let built: Built = Rc::default();
        let tree = Directionality::new(
            TextDirection::Ltr,
            Center::new().child(
                ShrinkWrappingViewport::new(offset.as_viewport_offset()).slivers(vec![
                    SliverToBoxAdapter::new()
                        .child(SizedBox::new().height(50.0))
                        .into_widget(),
                    item_list(&built, 2),
                ]),
            ),
        )
        .into_widget();
        let harness = Harness::mount(&mut app, tree);
        harness.pump(&mut app);

        let center = harness
            .render_root(&app)
            .child(&app)
            .expect("a child")
            .as_object()
            .downcast::<RenderPositionedBox>(&app)
            .expect("a RenderPositionedBox");
        let viewport = center
            .child(&app)
            .expect("the viewport")
            .as_object()
            .downcast::<RenderShrinkWrappingViewport>(&app)
            .expect("a RenderShrinkWrappingViewport");
        // 50 for the box adapter plus two 40-pixel list items.
        assert_eq!(viewport.size(&app).height(), 130.0);
    }
}
