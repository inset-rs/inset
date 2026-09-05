//! Flutter counterpart: `rendering/sliver_persistent_header.dart`.
//!
//! Semantics wait; see `PORTING.md`.

use std::rc::Rc;
use std::time::Duration;

use reveal_animation::{
    Animatable, AnimationController, AnimationStatus, AnyAnimation, Curve, CurveTween, Curves,
    Tween,
};
use reveal_embedder::{Matrix4, Offset, Rect, clamp_double};
use reveal_foundation::{App, Handle, HandleId, ListenableObject, Listener};
use reveal_painting::{Axis, AxisDirection, transform_rect};
use reveal_scheduler::{Ticker, TickerCallback, TickerProvider, TickerProviderObject};

use crate::box_::{AnyRenderBox, BoxHitTestResult};
use crate::object::{
    AnyRenderObject, RenderHandle, RenderObjectBase, RenderObjectWithChildMixin, resolve,
};
use crate::painting_context::PaintingContext;
use crate::sliver::{
    RenderSliver, RenderSliverHelpers, SliverGeometry, SliverHitTestResult,
    apply_growth_direction_to_axis_direction, calculate_cache_offset,
};
use crate::viewport_offset::ScrollDirection;

/// Trims the specified edges of the given `Rect`, so that they do not exceed the given values.
///
/// Flutter's private `_trim`; the omitted Dart defaults are the unbounded edges.
fn trim(original: Option<Rect>, top: f64, right: f64, bottom: f64, left: f64) -> Option<Rect> {
    original.map(|rect| rect.intersect(Rect::from_ltrb(left, top, right, bottom)))
}

/// An erased [`TickerProvider`] a render object can hold as a field, compared by identity.
///
/// Dart's `TickerProvider? vsync` field.
#[derive(Clone)]
pub struct TickerProviderRef(Rc<dyn TickerProvider>);

impl TickerProviderRef {
    /// Erases an arena object that vends tickers.
    pub fn new<T: TickerProviderObject>(provider: Handle<T>) -> TickerProviderRef {
        TickerProviderRef(Rc::new(provider))
    }
}

impl TickerProvider for TickerProviderRef {
    fn create_ticker(&self, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        self.0.create_ticker(app, on_tick)
    }
}

impl PartialEq for TickerProviderRef {
    fn eq(&self, other: &TickerProviderRef) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for TickerProviderRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TickerProviderRef")
    }
}

/// Specifies how a stretched header is to trigger a callback.
#[derive(Clone)]
pub struct OverScrollHeaderStretchConfiguration {
    /// The offset of overscroll required to trigger the
    /// [`on_stretch_trigger`](Self::on_stretch_trigger).
    pub stretch_trigger_offset: f64,

    /// The callback to be executed when a user over-scrolls to the offset specified by
    /// [`stretch_trigger_offset`](Self::stretch_trigger_offset).
    ///
    /// Dart's `AsyncCallback?`: there is no isolate event loop here, so the callback runs to
    /// completion synchronously.
    pub on_stretch_trigger: Option<Listener>,
}

impl OverScrollHeaderStretchConfiguration {
    /// Creates an object that specifies how a stretched header may activate a callback.
    ///
    /// Dart's default `stretch_trigger_offset` is 100.0.
    pub fn new() -> OverScrollHeaderStretchConfiguration {
        OverScrollHeaderStretchConfiguration {
            stretch_trigger_offset: 100.0,
            on_stretch_trigger: None,
        }
    }

    /// Sets [`stretch_trigger_offset`](Self::stretch_trigger_offset).
    pub fn stretch_trigger_offset(mut self, value: f64) -> OverScrollHeaderStretchConfiguration {
        self.stretch_trigger_offset = value;
        self
    }

    /// Sets [`on_stretch_trigger`](Self::on_stretch_trigger).
    pub fn on_stretch_trigger(mut self, value: Listener) -> OverScrollHeaderStretchConfiguration {
        self.on_stretch_trigger = Some(value);
        self
    }
}

impl Default for OverScrollHeaderStretchConfiguration {
    fn default() -> OverScrollHeaderStretchConfiguration {
        OverScrollHeaderStretchConfiguration::new()
    }
}

/// Specifies how a pinned header or a floating header should react to
/// [`crate::RenderObject::show_on_screen`] calls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PersistentHeaderShowOnScreenConfiguration {
    /// The smallest the floating header can expand to in the main axis direction, in response to a
    /// [`crate::RenderObject::show_on_screen`] call, in addition to its
    /// [`RenderSliverPersistentHeader::min_extent`].
    ///
    /// When a floating persistent header is told to show a [`Rect`] on screen, it may expand
    /// itself to accommodate the rect. The minimum extent that is allowed for such expansion is
    /// either the header's `min_extent` or this value, whichever is larger.
    ///
    /// Defaults to negative infinity, and must be less than or equal to
    /// [`max_show_on_screen_extent`](Self::max_show_on_screen_extent). Has no effect unless the
    /// persistent header is a floating header.
    pub min_show_on_screen_extent: f64,

    /// The biggest the floating header can expand to in the main axis direction, in response to a
    /// [`crate::RenderObject::show_on_screen`] call, in addition to its
    /// [`RenderSliverPersistentHeader::max_extent`].
    ///
    /// Defaults to infinity, and must be greater than or equal to
    /// [`min_show_on_screen_extent`](Self::min_show_on_screen_extent). Has no effect unless the
    /// persistent header is a floating header.
    pub max_show_on_screen_extent: f64,
}

impl PersistentHeaderShowOnScreenConfiguration {
    /// Creates an object that specifies how a pinned or floating persistent header should behave
    /// in response to [`crate::RenderObject::show_on_screen`] calls.
    pub const fn new() -> PersistentHeaderShowOnScreenConfiguration {
        PersistentHeaderShowOnScreenConfiguration {
            min_show_on_screen_extent: f64::NEG_INFINITY,
            max_show_on_screen_extent: f64::INFINITY,
        }
    }

    /// Sets [`min_show_on_screen_extent`](Self::min_show_on_screen_extent).
    pub fn min_show_on_screen_extent(
        mut self,
        value: f64,
    ) -> PersistentHeaderShowOnScreenConfiguration {
        self.min_show_on_screen_extent = value;
        debug_assert!(self.min_show_on_screen_extent <= self.max_show_on_screen_extent);
        self
    }

    /// Sets [`max_show_on_screen_extent`](Self::max_show_on_screen_extent).
    pub fn max_show_on_screen_extent(
        mut self,
        value: f64,
    ) -> PersistentHeaderShowOnScreenConfiguration {
        self.max_show_on_screen_extent = value;
        debug_assert!(self.min_show_on_screen_extent <= self.max_show_on_screen_extent);
        self
    }
}

impl Default for PersistentHeaderShowOnScreenConfiguration {
    fn default() -> PersistentHeaderShowOnScreenConfiguration {
        PersistentHeaderShowOnScreenConfiguration::new()
    }
}

/// Specifies how a floating header is to be "snapped" (animated) into or out of view.
#[derive(Clone)]
pub struct FloatingHeaderSnapConfiguration {
    /// The snap animation curve.
    pub curve: Rc<dyn Curve>,

    /// The snap animation's duration.
    pub duration: Duration,
}

impl FloatingHeaderSnapConfiguration {
    /// Creates an object that specifies how a floating header is to be snapped into or out of
    /// view.
    ///
    /// Dart's defaults: [`Curves::ease`] and 300 milliseconds.
    pub fn new() -> FloatingHeaderSnapConfiguration {
        FloatingHeaderSnapConfiguration {
            curve: Curves::ease(),
            duration: Duration::from_millis(300),
        }
    }

    /// Sets [`curve`](Self::curve).
    pub fn curve(mut self, value: Rc<dyn Curve>) -> FloatingHeaderSnapConfiguration {
        self.curve = value;
        self
    }

    /// Sets [`duration`](Self::duration).
    pub fn duration(mut self, value: Duration) -> FloatingHeaderSnapConfiguration {
        self.duration = value;
        self
    }
}

impl Default for FloatingHeaderSnapConfiguration {
    fn default() -> FloatingHeaderSnapConfiguration {
        FloatingHeaderSnapConfiguration::new()
    }
}

/// Flutter's `RenderSliverPersistentHeader` fields.
pub struct RenderSliverPersistentHeaderData {
    last_stretch_offset: f64,
    needs_update_child: bool,
    last_shrink_offset: f64,
    last_overlaps_content: bool,
    /// Defines the parameters used to execute a callback when a stretching header over-scrolls.
    ///
    /// If this is `None` then the callback is not triggered.
    pub stretch_configuration: Option<OverScrollHeaderStretchConfiguration>,
}

impl RenderSliverPersistentHeaderData {
    /// Creates the state of a persistent header that has not been laid out.
    pub fn new(
        stretch_configuration: Option<OverScrollHeaderStretchConfiguration>,
    ) -> RenderSliverPersistentHeaderData {
        RenderSliverPersistentHeaderData {
            last_stretch_offset: 0.0,
            needs_update_child: true,
            last_shrink_offset: 0.0,
            last_overlaps_content: false,
            stretch_configuration,
        }
    }
}

/// Implements [`RenderSliverPersistentHeader`] field accessors for a `header` field.
#[macro_export]
macro_rules! render_sliver_persistent_header_accessors {
    () => {
        fn header_data(
            self: $crate::RenderHandle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::RenderSliverPersistentHeaderData {
            &self.get(app).header
        }
        fn header_data_mut(
            self: $crate::RenderHandle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::RenderSliverPersistentHeaderData {
            &mut self.get_mut(app).header
        }
    };
}

/// A sliver that has a box child which scrolls normally, except that when it hits the leading
/// edge (typically the top) of the viewport, it shrinks to a minimum size
/// ([`min_extent`](Self::min_extent)).
///
/// This trait primarily provides helpers for managing the child, in particular:
///
///  * [`layout_child`](Self::layout_child), which applies min and max extents and a scroll offset
///    to lay out the child. This is normally called from `perform_layout`.
///  * [`child_extent`](Self::child_extent), to convert the child's box layout dimensions to the
///    sliver geometry model.
///  * hit testing, painting, and other details of the sliver protocol.
///
/// Implementors must implement `perform_layout`, [`min_extent`](Self::min_extent) and
/// [`max_extent`](Self::max_extent), and typically also will implement
/// [`update_child`](Self::update_child).
pub trait RenderSliverPersistentHeader:
    RenderSliver + RenderObjectWithChildMixin<ChildType = AnyRenderBox> + RenderSliverHelpers
{
    /// Mixin field access.
    fn header_data(self: RenderHandle<Self>, app: &App) -> &RenderSliverPersistentHeaderData;

    /// See [`header_data`](Self::header_data).
    fn header_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderSliverPersistentHeaderData;

    /// The biggest that this render object can become, in the main axis direction.
    ///
    /// This value should not be based on the child. If it changes, call `mark_needs_layout`.
    fn max_extent(self: RenderHandle<Self>, app: &App) -> f64;

    /// The smallest that this render object can become, in the main axis direction.
    ///
    /// If this is based on the intrinsic dimensions of the child, the child should be measured
    /// during [`update_child`](Self::update_child) and the value cached and returned here.
    fn min_extent(self: RenderHandle<Self>, app: &App) -> f64;

    /// The dimension of the child in the main axis.
    fn child_extent(self: RenderHandle<Self>, app: &App) -> f64 {
        let Some(child) = self.child(app) else {
            return 0.0;
        };
        debug_assert!(child.has_size(app));
        match self.constraints(app).axis() {
            Axis::Vertical => child.size(app).height(),
            Axis::Horizontal => child.size(app).width(),
        }
    }

    /// The most recent `shrink_offset` passed to [`update_child`](Self::update_child).
    fn last_shrink_offset(self: RenderHandle<Self>, app: &App) -> f64 {
        self.header_data(app).last_shrink_offset
    }

    /// The most recent `overlaps_content` passed to [`update_child`](Self::update_child).
    fn last_overlaps_content(self: RenderHandle<Self>, app: &App) -> bool {
        self.header_data(app).last_overlaps_content
    }

    /// Defines the parameters used to execute a callback when a stretching header over-scrolls.
    fn stretch_configuration(
        self: RenderHandle<Self>,
        app: &App,
    ) -> Option<OverScrollHeaderStretchConfiguration> {
        self.header_data(app).stretch_configuration.clone()
    }

    /// Sets [`stretch_configuration`](Self::stretch_configuration).
    fn set_stretch_configuration(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<OverScrollHeaderStretchConfiguration>,
    ) {
        self.header_data_mut(app).stretch_configuration = value;
    }

    /// Update the child render object if necessary.
    ///
    /// Called before the first layout, any time `mark_needs_layout` is called, and any time the
    /// scroll offset changes. The `shrink_offset` is the difference between the
    /// [`max_extent`](Self::max_extent) and the current size. Zero means the header is fully
    /// expanded, any greater number up to `max_extent` means that the header has been scrolled by
    /// that much. The `overlaps_content` argument is true if the sliver's leading edge is beyond
    /// its normal place in the viewport contents.
    ///
    /// When this method is called by [`layout_child`](Self::layout_child), the child can be set,
    /// mutated, or replaced. Any time this method would mutate the child, call `mark_needs_layout`.
    fn update_child(
        self: RenderHandle<Self>,
        app: &mut App,
        shrink_offset: f64,
        overlaps_content: bool,
    ) {
        let _ = (self, app, shrink_offset, overlaps_content);
    }

    /// The body of Flutter's `markNeedsLayout` override, run before the inherited body.
    ///
    /// This is automatically called whenever the child's intrinsic dimensions change, at which
    /// point we should remeasure them during the next layout.
    fn mark_needs_layout(self: RenderHandle<Self>, app: &mut App) {
        self.header_data_mut(app).needs_update_child = true;
        RenderSliver::mark_needs_layout(self, app);
    }

    /// Lays out the child.
    ///
    /// This is called by `perform_layout`. It applies the given `scroll_offset` (which need not
    /// match the offset given by the constraints) and the `max_extent` (which need not match the
    /// value returned by [`max_extent`](Self::max_extent)).
    ///
    /// The `overlaps_content` argument is passed to [`update_child`](Self::update_child).
    fn layout_child(
        self: RenderHandle<Self>,
        app: &mut App,
        scroll_offset: f64,
        max_extent: f64,
        overlaps_content: bool,
    ) {
        let shrink_offset = scroll_offset.min(max_extent);
        let header = self.header_data(app);
        if header.needs_update_child
            || header.last_shrink_offset != shrink_offset
            || header.last_overlaps_content != overlaps_content
        {
            self.as_object().invoke_layout_callback(app, |app| {
                self.update_child(app, shrink_offset, overlaps_content);
            });
            let header = self.header_data_mut(app);
            header.last_shrink_offset = shrink_offset;
            header.last_overlaps_content = overlaps_content;
            header.needs_update_child = false;
        }
        debug_assert!(
            self.min_extent(app) <= max_extent,
            "the max_extent for this persistent header is less than its min_extent"
        );
        let constraints = self.constraints(app);
        let stretch_configuration = self.stretch_configuration(app);
        let mut stretch_offset = 0.0;
        if stretch_configuration.is_some() && constraints.scroll_offset == 0.0 {
            stretch_offset += constraints.overlap.abs();
        }

        if let Some(child) = self.child(app) {
            let min_extent = self.min_extent(app);
            let child_constraints = constraints.as_box_constraints(
                0.0,
                min_extent.max(max_extent - shrink_offset) + stretch_offset,
                None,
            );
            child.layout(app, child_constraints, true);
        }

        if let Some(configuration) = &stretch_configuration
            && let Some(on_stretch_trigger) = configuration.on_stretch_trigger.clone()
            && stretch_offset >= configuration.stretch_trigger_offset
            && self.header_data(app).last_stretch_offset <= configuration.stretch_trigger_offset
        {
            on_stretch_trigger.call(app);
        }
        self.header_data_mut(app).last_stretch_offset = stretch_offset;
    }

    /// The body of Flutter's `hitTestChildren` override.
    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut SliverHitTestResult<'_>,
        main_axis_position: f64,
        cross_axis_position: f64,
    ) -> bool {
        debug_assert!(self.geometry(app).hit_test_extent > 0.0);
        let Some(child) = self.child(app) else {
            return false;
        };
        self.hit_test_box_child(
            app,
            &mut BoxHitTestResult::wrap(result),
            child,
            main_axis_position,
            cross_axis_position,
        )
    }

    /// The body of Flutter's `applyPaintTransform` override.
    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        debug_assert_eq!(Some(child), self.child(app).map(AnyRenderBox::as_object));
        let child = child.as_box().expect("the child is a box");
        self.apply_paint_transform_for_box_child(app, child, transform);
    }

    /// The body of Flutter's `paint` override.
    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let Some(child) = self.child(app) else {
            return;
        };
        if !self.geometry(app).visible {
            return;
        }
        let constraints = self.constraints(app);
        let paint_extent = self.geometry(app).paint_extent;
        let child_position = RenderSliver::child_main_axis_position(self, app, child.as_object());
        let child_extent = self.child_extent(app);
        let offset = offset
            + match apply_growth_direction_to_axis_direction(
                constraints.axis_direction,
                constraints.growth_direction,
            ) {
                AxisDirection::Up => Offset::new(0.0, paint_extent - child_position - child_extent),
                AxisDirection::Left => {
                    Offset::new(paint_extent - child_position - child_extent, 0.0)
                }
                AxisDirection::Right => Offset::new(child_position, 0.0),
                AxisDirection::Down => Offset::new(0.0, child_position),
            };
        context.paint_child(app, child.as_object(), offset);
    }
}

/// Flutter's `RenderSliverScrollingPersistentHeader` field.
#[derive(Debug, Default)]
pub struct RenderSliverScrollingPersistentHeaderData {
    // Distance from our leading edge to the child's leading edge, in the axis direction.
    // Negative if we're scrolled off the top.
    child_position: Option<f64>,
}

impl RenderSliverScrollingPersistentHeaderData {
    /// Creates the state of a scrolling persistent header that has not been laid out.
    pub const fn new() -> RenderSliverScrollingPersistentHeaderData {
        RenderSliverScrollingPersistentHeaderData {
            child_position: None,
        }
    }
}

/// A sliver with a box child which scrolls normally, except that when it hits the leading edge
/// (typically the top) of the viewport, it shrinks to a minimum size before continuing to scroll.
///
/// This sliver makes no effort to avoid overlapping other content.
pub trait RenderSliverScrollingPersistentHeader: RenderSliverPersistentHeader {
    /// Mixin field access.
    fn scrolling_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RenderSliverScrollingPersistentHeaderData;

    /// See [`scrolling_data`](Self::scrolling_data).
    fn scrolling_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderSliverScrollingPersistentHeaderData;

    /// Updates the geometry, and returns the new value for
    /// [`RenderSliver::child_main_axis_position`].
    fn update_geometry(self: RenderHandle<Self>, app: &mut App) -> f64 {
        let constraints = self.constraints(app);
        let mut stretch_offset = 0.0;
        if self.stretch_configuration(app).is_some() {
            stretch_offset += constraints.overlap.abs();
        }
        let max_extent = self.max_extent(app);
        let paint_extent = max_extent - constraints.scroll_offset;
        let cache_extent = calculate_cache_offset(constraints, 0.0, max_extent);

        self.set_geometry(
            app,
            SliverGeometry::new()
                .scroll_extent(max_extent)
                .paint_origin(constraints.overlap.min(0.0))
                .paint_extent(clamp_double(
                    paint_extent,
                    0.0,
                    constraints.remaining_paint_extent,
                ))
                .cache_extent(cache_extent)
                .max_paint_extent(max_extent + stretch_offset)
                // Conservatively say we do have overflow to avoid complexity.
                .has_visual_overflow(true),
        );
        if stretch_offset > 0.0 {
            0.0
        } else {
            (paint_extent - self.child_extent(app)).min(0.0)
        }
    }

    /// The body of Flutter's `performLayout` override.
    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let scroll_offset = self.constraints(app).scroll_offset;
        let max_extent = self.max_extent(app);
        self.layout_child(app, scroll_offset, max_extent, false);
        let child_position = self.update_geometry(app);
        self.scrolling_data_mut(app).child_position = Some(child_position);
    }

    /// The body of Flutter's `childMainAxisPosition` override.
    fn child_main_axis_position(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> f64 {
        debug_assert_eq!(Some(child), self.child(app).map(AnyRenderBox::as_object));
        self.scrolling_data(app)
            .child_position
            .expect("the header has been laid out")
    }
}

/// Flutter's `RenderSliverPinnedPersistentHeader` field.
#[derive(Clone, Copy, Debug)]
pub struct RenderSliverPinnedPersistentHeaderData {
    /// Specifies the persistent header's behavior when `show_on_screen` is called.
    ///
    /// If set to `None`, the persistent header will delegate the `show_on_screen` call to its
    /// parent render object.
    pub show_on_screen_configuration: Option<PersistentHeaderShowOnScreenConfiguration>,
}

impl RenderSliverPinnedPersistentHeaderData {
    /// Creates the state of a pinned persistent header.
    pub const fn new() -> RenderSliverPinnedPersistentHeaderData {
        RenderSliverPinnedPersistentHeaderData {
            show_on_screen_configuration: Some(PersistentHeaderShowOnScreenConfiguration::new()),
        }
    }
}

impl Default for RenderSliverPinnedPersistentHeaderData {
    fn default() -> RenderSliverPinnedPersistentHeaderData {
        RenderSliverPinnedPersistentHeaderData::new()
    }
}

/// A sliver with a box child which never scrolls off the viewport in the positive scroll
/// direction, and which first scrolls on at a full size but then shrinks as the viewport continues
/// to scroll.
///
/// This sliver avoids overlapping other earlier slivers where possible.
pub trait RenderSliverPinnedPersistentHeader: RenderSliverPersistentHeader {
    /// Mixin field access.
    fn pinned_data(self: RenderHandle<Self>, app: &App) -> &RenderSliverPinnedPersistentHeaderData;

    /// See [`pinned_data`](Self::pinned_data).
    fn pinned_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderSliverPinnedPersistentHeaderData;

    /// The body of Flutter's `performLayout` override.
    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let max_extent = self.max_extent(app);
        let overlaps_content = constraints.overlap > 0.0;
        self.layout_child(app, constraints.scroll_offset, max_extent, overlaps_content);
        let effective_remaining_paint_extent =
            (constraints.remaining_paint_extent - constraints.overlap).max(0.0);
        let layout_extent = clamp_double(
            max_extent - constraints.scroll_offset,
            0.0,
            effective_remaining_paint_extent,
        );
        let stretch_offset = if self.stretch_configuration(app).is_some() {
            constraints.overlap.abs()
        } else {
            0.0
        };
        let child_extent = self.child_extent(app);
        let min_extent = self.min_extent(app);
        self.set_geometry(
            app,
            SliverGeometry::new()
                .scroll_extent(max_extent)
                .paint_origin(constraints.overlap)
                .paint_extent(child_extent.min(effective_remaining_paint_extent))
                .layout_extent(layout_extent)
                .max_paint_extent(max_extent + stretch_offset)
                .max_scroll_obstruction_extent(min_extent)
                .cache_extent(if layout_extent > 0.0 {
                    -constraints.cache_origin + layout_extent
                } else {
                    layout_extent
                })
                // Conservatively say we do have overflow to avoid complexity.
                .has_visual_overflow(true),
        );
    }

    /// The body of Flutter's `childMainAxisPosition` override.
    fn child_main_axis_position(
        self: RenderHandle<Self>,
        _app: &App,
        _child: AnyRenderObject,
    ) -> f64 {
        let _ = self;
        0.0
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
        let local_bounds = match descendant {
            Some(descendant) => Some(transform_rect(
                &descendant.get_transform_to(app, Some(self.as_object())),
                rect.unwrap_or_else(|| descendant.paint_bounds(app)),
            )),
            None => rect,
        };

        let constraints = self.constraints(app);
        let child_extent = self.child_extent(app);
        let new_rect = match apply_growth_direction_to_axis_direction(
            constraints.axis_direction,
            constraints.growth_direction,
        ) {
            AxisDirection::Up => trim(
                local_bounds,
                f64::NEG_INFINITY,
                f64::INFINITY,
                child_extent,
                f64::NEG_INFINITY,
            ),
            AxisDirection::Left => trim(
                local_bounds,
                f64::NEG_INFINITY,
                child_extent,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ),
            AxisDirection::Right => trim(
                local_bounds,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::INFINITY,
                0.0,
            ),
            AxisDirection::Down => trim(
                local_bounds,
                0.0,
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ),
        };

        RenderObjectBase::show_on_screen(
            self,
            app,
            Some(self.as_object()),
            new_rect,
            duration,
            curve,
        );
    }
}

/// Flutter's `RenderSliverFloatingPersistentHeader` fields.
pub struct RenderSliverFloatingPersistentHeaderData {
    controller: Option<Handle<AnimationController>>,
    animation: Option<AnyAnimation<f64>>,
    last_actual_scroll_offset: Option<f64>,
    effective_scroll_offset: Option<f64>,
    // Important for pointer scrolling, which does not have the same concept of a hold and release
    // scroll movement, like dragging. This keeps track of the last ScrollDirection when scrolling
    // started.
    last_started_scroll_direction: Option<ScrollDirection>,
    // Distance from our leading edge to the child's leading edge, in the axis direction.
    // Negative if we're scrolled off the top.
    child_position: Option<f64>,
    vsync: Option<TickerProviderRef>,
    /// Defines the parameters used to snap (animate) the floating header in and out of view.
    ///
    /// If this is `None` then the floating header does not snap.
    pub snap_configuration: Option<FloatingHeaderSnapConfiguration>,
    /// Specifies the persistent header's behavior when `show_on_screen` is called.
    ///
    /// If set to `None`, the persistent header will delegate the `show_on_screen` call to its
    /// parent render object.
    pub show_on_screen_configuration: Option<PersistentHeaderShowOnScreenConfiguration>,
}

impl RenderSliverFloatingPersistentHeaderData {
    /// Creates the state of a floating persistent header that has not been laid out.
    pub fn new(
        vsync: Option<TickerProviderRef>,
        snap_configuration: Option<FloatingHeaderSnapConfiguration>,
        show_on_screen_configuration: Option<PersistentHeaderShowOnScreenConfiguration>,
    ) -> RenderSliverFloatingPersistentHeaderData {
        RenderSliverFloatingPersistentHeaderData {
            controller: None,
            animation: None,
            last_actual_scroll_offset: None,
            effective_scroll_offset: None,
            last_started_scroll_direction: None,
            child_position: None,
            vsync,
            snap_configuration,
            show_on_screen_configuration,
        }
    }
}

/// A sliver with a box child which shrinks and scrolls like a
/// [`RenderSliverScrollingPersistentHeader`], but immediately comes back when the user scrolls in
/// the reverse direction.
///
/// See also:
///
///  * [`RenderSliverFloatingPinnedPersistentHeader`], which is similar but sticks to the start of
///    the viewport rather than scrolling off.
pub trait RenderSliverFloatingPersistentHeader: RenderSliverPersistentHeader {
    /// Mixin field access.
    fn floating_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RenderSliverFloatingPersistentHeaderData;

    /// See [`floating_data`](Self::floating_data).
    fn floating_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderSliverFloatingPersistentHeaderData;

    /// The body of Flutter's `detach` override: disposes the snap animation controller, which is
    /// lazily recreated if we are reattached.
    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        if let Some(controller) = self.floating_data(app).controller {
            controller.dispose(app);
        }
        self.floating_data_mut(app).controller = None;
    }

    /// A [`TickerProvider`] to use when animating the scroll position.
    fn vsync(self: RenderHandle<Self>, app: &App) -> Option<TickerProviderRef> {
        self.floating_data(app).vsync.clone()
    }

    /// Sets [`vsync`](Self::vsync).
    fn set_vsync(self: RenderHandle<Self>, app: &mut App, value: Option<TickerProviderRef>) {
        if value == self.floating_data(app).vsync {
            return;
        }
        self.floating_data_mut(app).vsync = value.clone();
        match value {
            None => {
                if let Some(controller) = self.floating_data(app).controller {
                    controller.dispose(app);
                }
                self.floating_data_mut(app).controller = None;
            }
            Some(vsync) => {
                if let Some(controller) = self.floating_data(app).controller {
                    controller.resync(app, vsync);
                }
            }
        }
    }

    /// Defines the parameters used to snap (animate) the floating header in and out of view.
    fn snap_configuration(
        self: RenderHandle<Self>,
        app: &App,
    ) -> Option<FloatingHeaderSnapConfiguration> {
        self.floating_data(app).snap_configuration.clone()
    }

    /// Sets [`snap_configuration`](Self::snap_configuration).
    fn set_snap_configuration(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<FloatingHeaderSnapConfiguration>,
    ) {
        self.floating_data_mut(app).snap_configuration = value;
    }

    /// Specifies the persistent header's behavior when `show_on_screen` is called.
    fn show_on_screen_configuration(
        self: RenderHandle<Self>,
        app: &App,
    ) -> Option<PersistentHeaderShowOnScreenConfiguration> {
        self.floating_data(app).show_on_screen_configuration
    }

    /// Sets [`show_on_screen_configuration`](Self::show_on_screen_configuration).
    fn set_show_on_screen_configuration(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<PersistentHeaderShowOnScreenConfiguration>,
    ) {
        self.floating_data_mut(app).show_on_screen_configuration = value;
    }

    /// Flutter's `_effectiveScrollOffset`, the offset the header is laid out at.
    fn effective_scroll_offset(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.floating_data(app).effective_scroll_offset
    }

    /// Updates the geometry, and returns the new value for
    /// [`RenderSliver::child_main_axis_position`].
    fn update_geometry(self: RenderHandle<Self>, app: &mut App) -> f64 {
        let constraints = self.constraints(app);
        let mut stretch_offset = 0.0;
        if self.stretch_configuration(app).is_some() {
            stretch_offset += constraints.overlap.abs();
        }
        let max_extent = self.max_extent(app);
        let paint_extent = max_extent
            - self
                .effective_scroll_offset(app)
                .expect("perform_layout sets the effective scroll offset");
        let layout_extent = max_extent - constraints.scroll_offset;
        self.set_geometry(
            app,
            SliverGeometry::new()
                .scroll_extent(max_extent)
                .paint_origin(constraints.overlap.min(0.0))
                .paint_extent(clamp_double(
                    paint_extent,
                    0.0,
                    constraints.remaining_paint_extent,
                ))
                .layout_extent(clamp_double(
                    layout_extent,
                    0.0,
                    constraints.remaining_paint_extent,
                ))
                .max_paint_extent(max_extent + stretch_offset)
                // Conservatively say we do have overflow to avoid complexity.
                .has_visual_overflow(true),
        );
        if stretch_offset > 0.0 {
            0.0
        } else {
            (paint_extent - self.child_extent(app)).min(0.0)
        }
    }

    /// Flutter's `_updateAnimation`.
    fn update_animation(
        self: RenderHandle<Self>,
        app: &mut App,
        duration: Duration,
        end_value: f64,
        curve: Rc<dyn Curve>,
    ) {
        let vsync = self
            .vsync(app)
            .expect("vsync must be set if the floating header changes size animatedly");
        let controller = match self.floating_data(app).controller {
            Some(controller) => controller,
            None => {
                let controller = AnimationController::create(
                    app,
                    None,
                    Some(duration),
                    None,
                    0.0,
                    1.0,
                    reveal_animation::AnimationBehavior::Normal,
                    vsync,
                );
                self.floating_data_mut(app).controller = Some(controller);
                controller.add_listener(
                    app,
                    Listener::handle_method(self.handle(), snap_tick::<Self>),
                );
                controller
            }
        };
        let begin = self.effective_scroll_offset(app);
        let tween = Tween::new(app, begin, Some(end_value));
        let curve_tween = CurveTween::new(app, curve);
        let animation = controller.drive(app, tween.chain(curve_tween));
        self.floating_data_mut(app).animation = Some(animation);
    }

    /// Update the last known scroll direction when scrolling began.
    fn update_scroll_start_direction(
        self: RenderHandle<Self>,
        app: &mut App,
        direction: ScrollDirection,
    ) {
        self.floating_data_mut(app).last_started_scroll_direction = Some(direction);
    }

    /// If the header isn't already fully exposed, then scroll it into view.
    fn maybe_start_snap_animation(
        self: RenderHandle<Self>,
        app: &mut App,
        direction: ScrollDirection,
    ) {
        let Some(snap) = self.snap_configuration(app) else {
            return;
        };
        let effective_scroll_offset = self
            .effective_scroll_offset(app)
            .expect("the header has been laid out");
        let max_extent = self.max_extent(app);
        if direction == ScrollDirection::Forward && effective_scroll_offset <= 0.0 {
            return;
        }
        if direction == ScrollDirection::Reverse && effective_scroll_offset >= max_extent {
            return;
        }

        let end_value = if direction == ScrollDirection::Forward {
            0.0
        } else {
            max_extent
        };
        self.update_animation(app, snap.duration, end_value, Rc::clone(&snap.curve));
        if let Some(controller) = self.floating_data(app).controller {
            controller.forward(app, Some(0.0));
        }
    }

    /// If a header snap animation or a `show_on_screen` expand animation is underway then stop it.
    fn maybe_stop_snap_animation(
        self: RenderHandle<Self>,
        app: &mut App,
        _direction: ScrollDirection,
    ) {
        if let Some(controller) = self.floating_data(app).controller {
            controller.stop(app, false);
        }
    }

    /// The body of Flutter's `performLayout` override.
    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let max_extent = self.max_extent(app);
        let data = self.floating_data(app);
        let last_actual_scroll_offset = data.last_actual_scroll_offset;
        let effective_scroll_offset = data.effective_scroll_offset;
        let last_started_scroll_direction = data.last_started_scroll_direction;
        // We've laid out at least once to get an initial position, and either we are scrolling
        // back, so should reveal, or some part of it is visible, so should shrink or reveal as
        // appropriate.
        let new_effective_scroll_offset = match last_actual_scroll_offset {
            Some(last_actual_scroll_offset)
                if constraints.scroll_offset < last_actual_scroll_offset
                    || effective_scroll_offset.is_some_and(|offset| offset < max_extent) =>
            {
                let mut delta = last_actual_scroll_offset - constraints.scroll_offset;
                let mut effective_scroll_offset =
                    effective_scroll_offset.expect("laid out at least once");
                let allow_floating_expansion = constraints.user_scroll_direction
                    == ScrollDirection::Forward
                    || last_started_scroll_direction == Some(ScrollDirection::Forward);
                if allow_floating_expansion {
                    if effective_scroll_offset > max_extent {
                        // We're scrolled off-screen, but should reveal, so pretend we're just at
                        // the limit.
                        effective_scroll_offset = max_extent;
                    }
                } else if delta > 0.0 {
                    // Disallow the expansion. (But allow shrinking, i.e. delta < 0.0 is fine.)
                    delta = 0.0;
                }
                clamp_double(
                    effective_scroll_offset - delta,
                    0.0,
                    constraints.scroll_offset,
                )
            }
            _ => constraints.scroll_offset,
        };
        self.floating_data_mut(app).effective_scroll_offset = Some(new_effective_scroll_offset);
        let overlaps_content = new_effective_scroll_offset < constraints.scroll_offset;

        self.layout_child(
            app,
            new_effective_scroll_offset,
            max_extent,
            overlaps_content,
        );
        let child_position = self.update_geometry(app);
        let data = self.floating_data_mut(app);
        data.child_position = Some(child_position);
        data.last_actual_scroll_offset = Some(constraints.scroll_offset);
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
        let Some(show_on_screen) = self.show_on_screen_configuration(app) else {
            return RenderObjectBase::show_on_screen(self, app, descendant, rect, duration, curve);
        };

        let child = self.child(app);
        debug_assert!(child.is_some() || descendant.is_none());
        // We prefer the child's coordinate space (instead of the sliver's) because it's easier for
        // us to convert the target rect into target extents: when the sliver is sitting above the
        // leading edge (not possible with pinned headers), the leading edge of the sliver and the
        // leading edge of the child will not be aligned. The only exception is when the child is
        // `None` (and thus `descendant` is `None`).
        let child_bounds = match descendant {
            Some(descendant) => Some(transform_rect(
                &descendant.get_transform_to(app, child.map(AnyRenderBox::as_object)),
                rect.unwrap_or_else(|| descendant.paint_bounds(app)),
            )),
            None => rect,
        };

        let constraints = self.constraints(app);
        let child_extent = self.child_extent(app);
        let (mut target_extent, target_rect) = match apply_growth_direction_to_axis_direction(
            constraints.axis_direction,
            constraints.growth_direction,
        ) {
            AxisDirection::Up => (
                child_extent - child_bounds.map_or(0.0, |bounds| bounds.top),
                trim(
                    child_bounds,
                    f64::NEG_INFINITY,
                    f64::INFINITY,
                    child_extent,
                    f64::NEG_INFINITY,
                ),
            ),
            AxisDirection::Right => (
                child_bounds.map_or(child_extent, |bounds| bounds.right),
                trim(
                    child_bounds,
                    f64::NEG_INFINITY,
                    f64::INFINITY,
                    f64::INFINITY,
                    0.0,
                ),
            ),
            AxisDirection::Down => (
                child_bounds.map_or(child_extent, |bounds| bounds.bottom),
                trim(
                    child_bounds,
                    0.0,
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                ),
            ),
            AxisDirection::Left => (
                child_extent - child_bounds.map_or(0.0, |bounds| bounds.left),
                trim(
                    child_bounds,
                    f64::NEG_INFINITY,
                    child_extent,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                ),
            ),
        };

        // A stretch header can have a bigger child extent than max extent.
        let max_extent = self.max_extent(app);
        let effective_max_extent = child_extent.max(max_extent);

        target_extent = clamp_double(
            clamp_double(
                target_extent,
                show_on_screen.min_show_on_screen_extent,
                show_on_screen.max_show_on_screen_extent,
            ),
            // Clamp the value back to the valid range after applying additional constraints.
            // Contracting is not allowed.
            child_extent,
            effective_max_extent,
        );

        // Expands the header if needed, with animation.
        let controller_status = self
            .floating_data(app)
            .controller
            .map(|controller| controller.status(app));
        if target_extent > child_extent && controller_status != Some(AnimationStatus::Forward) {
            let target_scroll_offset = max_extent - target_extent;
            debug_assert!(
                self.vsync(app).is_some(),
                "vsync must be set if the floating header changes size animatedly"
            );
            self.update_animation(app, duration, target_scroll_offset, Rc::clone(&curve));
            if let Some(controller) = self.floating_data(app).controller {
                controller.forward(app, Some(0.0));
            }
        }

        let descendant = match descendant {
            None => Some(self.as_object()),
            Some(_) => child.map(AnyRenderBox::as_object),
        };
        RenderObjectBase::show_on_screen(self, app, descendant, target_rect, duration, curve);
    }

    /// The body of Flutter's `childMainAxisPosition` override.
    fn child_main_axis_position(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> f64 {
        debug_assert_eq!(Some(child), self.child(app).map(AnyRenderBox::as_object));
        self.floating_data(app).child_position.unwrap_or(0.0)
    }

    /// The type-erased `RenderSliverFloatingPersistentHeader` handle. Free: the vtable is a
    /// `const`, and the id is copied.
    fn as_floating_persistent_header(
        self: RenderHandle<Self>,
    ) -> AnyRenderSliverFloatingPersistentHeader {
        AnyRenderSliverFloatingPersistentHeader {
            id: self.id(),
            vtable: const { &RenderSliverFloatingPersistentHeaderVTable::of::<Self>() },
        }
    }
}

/// The vtable of an erased [`AnyRenderSliverFloatingPersistentHeader`].
struct RenderSliverFloatingPersistentHeaderVTable {
    update_scroll_start_direction: fn(&mut App, HandleId, ScrollDirection),
    maybe_start_snap_animation: fn(&mut App, HandleId, ScrollDirection),
    maybe_stop_snap_animation: fn(&mut App, HandleId, ScrollDirection),
}

impl RenderSliverFloatingPersistentHeaderVTable {
    const fn of<T: RenderSliverFloatingPersistentHeader>()
    -> RenderSliverFloatingPersistentHeaderVTable {
        RenderSliverFloatingPersistentHeaderVTable {
            update_scroll_start_direction: |app, id, direction| {
                T::update_scroll_start_direction(resolve(id), app, direction)
            },
            maybe_start_snap_animation: |app, id, direction| {
                T::maybe_start_snap_animation(resolve(id), app, direction)
            },
            maybe_stop_snap_animation: |app, id, direction| {
                T::maybe_stop_snap_animation(resolve(id), app, direction)
            },
        }
    }
}

/// Erased [`RenderSliverFloatingPersistentHeader`]: what Dart's
/// `findAncestorRenderObjectOfType<RenderSliverFloatingPersistentHeader>()` hands back.
#[derive(Clone, Copy)]
pub struct AnyRenderSliverFloatingPersistentHeader {
    id: HandleId,
    vtable: &'static RenderSliverFloatingPersistentHeaderVTable,
}

impl AnyRenderSliverFloatingPersistentHeader {
    /// See [`RenderSliverFloatingPersistentHeader::update_scroll_start_direction`].
    pub fn update_scroll_start_direction(self, app: &mut App, direction: ScrollDirection) {
        (self.vtable.update_scroll_start_direction)(app, self.id, direction)
    }

    /// See [`RenderSliverFloatingPersistentHeader::maybe_start_snap_animation`].
    pub fn maybe_start_snap_animation(self, app: &mut App, direction: ScrollDirection) {
        (self.vtable.maybe_start_snap_animation)(app, self.id, direction)
    }

    /// See [`RenderSliverFloatingPersistentHeader::maybe_stop_snap_animation`].
    pub fn maybe_stop_snap_animation(self, app: &mut App, direction: ScrollDirection) {
        (self.vtable.maybe_stop_snap_animation)(app, self.id, direction)
    }
}

impl PartialEq for AnyRenderSliverFloatingPersistentHeader {
    fn eq(&self, other: &AnyRenderSliverFloatingPersistentHeader) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyRenderSliverFloatingPersistentHeader {}

/// The snap animation's listener: Dart's closure inside `_updateAnimation`.
fn snap_tick<T: RenderSliverFloatingPersistentHeader>(this: Handle<T>, app: &mut App) {
    let this = RenderHandle::from_handle(this);
    let Some(animation) = this.floating_data(app).animation else {
        return;
    };
    let value = animation.value(app);
    if this.effective_scroll_offset(app) == Some(value) {
        return;
    }
    this.floating_data_mut(app).effective_scroll_offset = Some(value);
    RenderSliverPersistentHeader::mark_needs_layout(this, app);
}

/// A sliver with a box child which shrinks and then remains pinned to the start of the viewport
/// like a [`RenderSliverPinnedPersistentHeader`], but immediately grows when the user scrolls in
/// the reverse direction.
///
/// See also:
///
///  * [`RenderSliverFloatingPersistentHeader`], which is similar but scrolls off the top rather
///    than sticking to it.
pub trait RenderSliverFloatingPinnedPersistentHeader: RenderSliverFloatingPersistentHeader {
    /// The body of Flutter's `updateGeometry` override.
    fn update_geometry(self: RenderHandle<Self>, app: &mut App) -> f64 {
        let constraints = self.constraints(app);
        let min_extent = self.min_extent(app);
        let min_allowed_extent = if constraints.remaining_paint_extent > min_extent {
            min_extent
        } else {
            constraints.remaining_paint_extent
        };
        let max_extent = self.max_extent(app);
        let paint_extent = max_extent
            - self
                .effective_scroll_offset(app)
                .expect("perform_layout sets the effective scroll offset");
        let clamped_paint_extent = clamp_double(
            paint_extent,
            min_allowed_extent,
            constraints.remaining_paint_extent,
        );
        let layout_extent = max_extent - constraints.scroll_offset;
        let stretch_offset = if self.stretch_configuration(app).is_some() {
            constraints.overlap.abs()
        } else {
            0.0
        };
        self.set_geometry(
            app,
            SliverGeometry::new()
                .scroll_extent(max_extent)
                .paint_origin(constraints.overlap.min(0.0))
                .paint_extent(clamped_paint_extent)
                .layout_extent(clamp_double(layout_extent, 0.0, clamped_paint_extent))
                .max_paint_extent(max_extent + stretch_offset)
                .max_scroll_obstruction_extent(min_extent)
                // Conservatively say we do have overflow to avoid complexity.
                .has_visual_overflow(true),
        );
        0.0
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::Size;
    use reveal_foundation::AppCell;

    use super::*;
    use crate::object::{RenderObject, RenderObjectData, RenderObjectWithChildData};
    use crate::sliver::{AnyRenderSliver, RenderSliverData};
    use crate::sliver_multi_box_adaptor::test_support::hit_testable_box;
    use crate::sliver_multi_box_adaptor::viewport_test_support::ScrollHarness;

    /// The shape every persistent header leaf in this test shares: a min and a max extent, and a
    /// box child of the header's current extent.
    macro_rules! persistent_header_leaf {
        ($leaf:ident, $extra:ident, $perform_layout:path, $child_position:path) => {
            struct $leaf {
                render_object: RenderObjectData,
                render_sliver: RenderSliverData,
                child: RenderObjectWithChildData<AnyRenderBox>,
                header: RenderSliverPersistentHeaderData,
                extra: $extra,
                min_extent: f64,
                max_extent: f64,
            }

            impl RenderObjectWithChildMixin for $leaf {
                type ChildType = AnyRenderBox;

                fn child_data(
                    self: RenderHandle<Self>,
                    app: &App,
                ) -> &RenderObjectWithChildData<AnyRenderBox> {
                    &self.get(app).child
                }

                fn child_data_mut(
                    self: RenderHandle<Self>,
                    app: &mut App,
                ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
                    &mut self.get_mut(app).child
                }
            }

            impl RenderSliverHelpers for $leaf {}

            impl RenderSliverPersistentHeader for $leaf {
                crate::render_sliver_persistent_header_accessors!();

                fn max_extent(self: RenderHandle<Self>, app: &App) -> f64 {
                    self.get(app).max_extent
                }

                fn min_extent(self: RenderHandle<Self>, app: &App) -> f64 {
                    self.get(app).min_extent
                }
            }

            impl RenderObject for $leaf {
                crate::render_object_accessors!();

                fn visit_children(
                    self: RenderHandle<Self>,
                    app: &App,
                    visitor: &mut dyn FnMut(AnyRenderObject),
                ) {
                    if let Some(child) = self.child(app) {
                        visitor(child.as_object());
                    }
                }

                fn apply_paint_transform(
                    self: RenderHandle<Self>,
                    app: &App,
                    child: AnyRenderObject,
                    transform: &mut Matrix4,
                ) {
                    RenderSliverPersistentHeader::apply_paint_transform(self, app, child, transform)
                }

                fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
                    $perform_layout(self, app)
                }

                fn paint(
                    self: RenderHandle<Self>,
                    app: &mut App,
                    context: &mut PaintingContext,
                    offset: Offset,
                ) {
                    RenderSliverPersistentHeader::paint(self, app, context, offset)
                }
            }

            impl RenderSliver for $leaf {
                crate::render_sliver_accessors!();

                fn hit_test_children(
                    self: RenderHandle<Self>,
                    app: &mut App,
                    result: &mut SliverHitTestResult<'_>,
                    main_axis_position: f64,
                    cross_axis_position: f64,
                ) -> bool {
                    RenderSliverPersistentHeader::hit_test_children(
                        self,
                        app,
                        result,
                        main_axis_position,
                        cross_axis_position,
                    )
                }

                fn child_main_axis_position(
                    self: RenderHandle<Self>,
                    app: &App,
                    child: AnyRenderObject,
                ) -> f64 {
                    $child_position(self, app, child)
                }
            }
        };
    }

    type ScrollingExtra = RenderSliverScrollingPersistentHeaderData;
    type PinnedExtra = RenderSliverPinnedPersistentHeaderData;

    persistent_header_leaf!(
        TestScrollingHeader,
        ScrollingExtra,
        RenderSliverScrollingPersistentHeader::perform_layout,
        RenderSliverScrollingPersistentHeader::child_main_axis_position
    );

    impl RenderSliverScrollingPersistentHeader for TestScrollingHeader {
        fn scrolling_data(
            self: RenderHandle<Self>,
            app: &App,
        ) -> &RenderSliverScrollingPersistentHeaderData {
            &self.get(app).extra
        }

        fn scrolling_data_mut(
            self: RenderHandle<Self>,
            app: &mut App,
        ) -> &mut RenderSliverScrollingPersistentHeaderData {
            &mut self.get_mut(app).extra
        }
    }

    persistent_header_leaf!(
        TestPinnedHeader,
        PinnedExtra,
        RenderSliverPinnedPersistentHeader::perform_layout,
        RenderSliverPinnedPersistentHeader::child_main_axis_position
    );

    impl RenderSliverPinnedPersistentHeader for TestPinnedHeader {
        fn pinned_data(
            self: RenderHandle<Self>,
            app: &App,
        ) -> &RenderSliverPinnedPersistentHeaderData {
            &self.get(app).extra
        }

        fn pinned_data_mut(
            self: RenderHandle<Self>,
            app: &mut App,
        ) -> &mut RenderSliverPinnedPersistentHeaderData {
            &mut self.get_mut(app).extra
        }
    }

    fn scrolling_header(
        app: &mut App,
        min_extent: f64,
        max_extent: f64,
    ) -> RenderHandle<TestScrollingHeader> {
        let child = hit_testable_box(app, max_extent);
        let header = RenderHandle::new_sliver(
            app,
            TestScrollingHeader {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                child: RenderObjectWithChildData::new(),
                header: RenderSliverPersistentHeaderData::new(None),
                extra: RenderSliverScrollingPersistentHeaderData::new(),
                min_extent,
                max_extent,
            },
        );
        header.set_child(app, Some(child));
        header
    }

    fn pinned_header(
        app: &mut App,
        min_extent: f64,
        max_extent: f64,
    ) -> RenderHandle<TestPinnedHeader> {
        let child = hit_testable_box(app, max_extent);
        let header = RenderHandle::new_sliver(
            app,
            TestPinnedHeader {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                child: RenderObjectWithChildData::new(),
                header: RenderSliverPersistentHeaderData::new(None),
                extra: RenderSliverPinnedPersistentHeaderData::new(),
                min_extent,
                max_extent,
            },
        );
        header.set_child(app, Some(child));
        header
    }

    /// A header's `layout_child` runs inside a layout callback, so layout has to go through the
    /// owner's layout phase.
    fn lay_out(app: &mut App, sliver: AnyRenderSliver, scroll_offset: f64) -> ScrollHarness {
        let harness = ScrollHarness::new(app, sliver, Size::new(200.0, 400.0));
        harness.scroll_to(app, scroll_offset);
        harness
    }

    /// `sliver_persistent_header_test.dart`: a scrolling header shrinks its paint extent as it
    /// scrolls off, keeping its full scroll extent.
    #[test]
    fn a_scrolling_header_shrinks_as_it_scrolls_off() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let header = scrolling_header(&mut app, 40.0, 120.0);
        lay_out(&mut app, header.as_sliver(), 50.0);

        let geometry = header.geometry(&app);
        assert_eq!(geometry.scroll_extent, 120.0);
        assert_eq!(geometry.paint_extent, 70.0);
        assert_eq!(geometry.max_scroll_obstruction_extent, 0.0);
        // The child shrinks to what is left of the maximum extent, floored at the minimum.
        assert_eq!(header.child_extent(&app), 70.0);
        assert_eq!(header.last_shrink_offset(&app), 50.0);
        let child = header.child(&app).expect("has a child");
        assert_eq!(
            RenderSliver::child_main_axis_position(header, &app, child.as_object()),
            0.0
        );
    }

    /// A pinned header stays at the leading edge and reports its minimum extent as the scroll
    /// obstruction.
    #[test]
    fn a_pinned_header_stays_at_the_leading_edge() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let header = pinned_header(&mut app, 40.0, 120.0);
        lay_out(&mut app, header.as_sliver(), 100.0);

        let geometry = header.geometry(&app);
        assert_eq!(geometry.scroll_extent, 120.0);
        assert_eq!(geometry.max_scroll_obstruction_extent, 40.0);
        // Shrunk to the minimum extent, still painting at the leading edge.
        assert_eq!(header.child_extent(&app), 40.0);
        assert_eq!(geometry.paint_extent, 40.0);
        assert_eq!(geometry.layout_extent, 20.0);
        let child = header.child(&app).expect("has a child");
        assert_eq!(
            RenderSliver::child_main_axis_position(header, &app, child.as_object()),
            0.0
        );
    }

    /// An unscrolled pinned header is at its maximum extent.
    #[test]
    fn an_unscrolled_pinned_header_is_at_its_maximum_extent() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let header = pinned_header(&mut app, 40.0, 120.0);
        lay_out(&mut app, header.as_sliver(), 0.0);

        let geometry = header.geometry(&app);
        assert_eq!(geometry.paint_extent, 120.0);
        assert_eq!(geometry.layout_extent, 120.0);
        assert_eq!(header.last_shrink_offset(&app), 0.0);
        assert!(!header.last_overlaps_content(&app));
    }

    /// `layout_child` re-runs `update_child` only when the shrink offset or overlap changes.
    #[test]
    fn layout_child_reports_the_shrink_offset() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let header = scrolling_header(&mut app, 40.0, 120.0);
        lay_out(&mut app, header.as_sliver(), 80.0);

        assert_eq!(header.last_shrink_offset(&app), 80.0);
        // The child is laid out with the remaining extent, floored at the minimum extent.
        assert_eq!(header.child_extent(&app), 40.0);
    }

    /// The show-on-screen configuration defaults to the full unbounded range.
    #[test]
    fn the_show_on_screen_configuration_defaults_to_unbounded() {
        let configuration = PersistentHeaderShowOnScreenConfiguration::new();
        assert_eq!(configuration.min_show_on_screen_extent, f64::NEG_INFINITY);
        assert_eq!(configuration.max_show_on_screen_extent, f64::INFINITY);
        let snap = FloatingHeaderSnapConfiguration::new();
        assert_eq!(snap.duration, Duration::from_millis(300));
    }
}
