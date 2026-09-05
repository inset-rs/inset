//! Flutter counterpart: `rendering/sliver_padding.dart`.
//!
//! Debug paint overlays wait; see `PORTING.md`.

use reveal_embedder::{Matrix4, Offset, TextDirection};
use reveal_foundation::App;
use reveal_painting::{Axis, AxisDirection, EdgeInsets, EdgeInsetsGeometry};

use crate::object::{
    AnyRenderObject, RenderHandle, RenderObject, RenderObjectData, RenderObjectWithChildData,
    RenderObjectWithChildMixin,
};
use crate::painting_context::PaintingContext;
use crate::sliver::{
    AnyRenderSliver, RenderSliver, RenderSliverData, SliverGeometry, SliverHitTestResult,
    SliverPhysicalParentData, apply_growth_direction_to_axis_direction, calculate_cache_offset,
    calculate_paint_offset,
};

/// A sliver that insets another sliver by applying resolved padding on each side.
///
/// Flutter's `RenderSliverEdgeInsetsPadding`: the shared bodies of a padding sliver, over the
/// [`resolved_padding`](Self::resolved_padding) the implementor supplies.
pub trait RenderSliverEdgeInsetsPadding:
    RenderSliver + RenderObjectWithChildMixin<ChildType = AnyRenderSliver>
{
    /// The amount to pad the child in each dimension.
    ///
    /// The offsets are specified in terms of visual edges, left, top, right, and bottom. These
    /// values are not affected by the [`TextDirection`].
    ///
    /// Must be `Some` and contain no negative values when `perform_layout` is called.
    fn resolved_padding(self: RenderHandle<Self>, app: &App) -> Option<EdgeInsets>;

    /// The padding in the scroll direction on the side nearest the 0.0 scroll direction.
    ///
    /// Only valid after layout has started, since before layout the render object doesn't know
    /// what direction it will be laid out in.
    fn before_padding(self: RenderHandle<Self>, app: &App) -> f64 {
        let padding = self
            .resolved_padding(app)
            .expect("the padding is resolved before layout");
        let constraints = self.constraints(app);
        match apply_growth_direction_to_axis_direction(
            constraints.axis_direction,
            constraints.growth_direction,
        ) {
            AxisDirection::Up => padding.bottom,
            AxisDirection::Right => padding.left,
            AxisDirection::Down => padding.top,
            AxisDirection::Left => padding.right,
        }
    }

    /// The padding in the scroll direction on the side furthest from the 0.0 scroll offset.
    ///
    /// Only valid after layout has started.
    fn after_padding(self: RenderHandle<Self>, app: &App) -> f64 {
        let padding = self
            .resolved_padding(app)
            .expect("the padding is resolved before layout");
        let constraints = self.constraints(app);
        match apply_growth_direction_to_axis_direction(
            constraints.axis_direction,
            constraints.growth_direction,
        ) {
            AxisDirection::Up => padding.top,
            AxisDirection::Right => padding.right,
            AxisDirection::Down => padding.bottom,
            AxisDirection::Left => padding.left,
        }
    }

    /// The total padding in the [`crate::SliverConstraints::axis_direction`]. (In other words,
    /// for a vertical downwards-growing list, the sum of the padding on the top and bottom.)
    ///
    /// Only valid after layout has started.
    fn main_axis_padding(self: RenderHandle<Self>, app: &App) -> f64 {
        let padding = self
            .resolved_padding(app)
            .expect("the padding is resolved before layout");
        // Dart's `EdgeInsetsGeometry.along(axis)`, which `EdgeInsets` inherits.
        match self.constraints(app).axis() {
            Axis::Horizontal => padding.horizontal(),
            Axis::Vertical => padding.vertical(),
        }
    }

    /// The total padding in the cross-axis direction. (In other words, for a vertical
    /// downwards-growing list, the sum of the padding on the left and right.)
    ///
    /// Only valid after layout has started.
    fn cross_axis_padding(self: RenderHandle<Self>, app: &App) -> f64 {
        let padding = self
            .resolved_padding(app)
            .expect("the padding is resolved before layout");
        match self.constraints(app).axis() {
            Axis::Horizontal => padding.vertical(),
            Axis::Vertical => padding.horizontal(),
        }
    }

    /// The body of Flutter's `setupParentData` override.
    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        let _ = self;
        if !child.parent_data_is::<SliverPhysicalParentData>(app) {
            child.set_parent_data(app, SliverPhysicalParentData::new());
        }
    }

    /// The body of Flutter's `performLayout` override.
    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let paint_offset = |from: f64, to: f64| calculate_paint_offset(constraints, from, to);
        let cache_offset = |from: f64, to: f64| calculate_cache_offset(constraints, from, to);

        let resolved_padding = self
            .resolved_padding(app)
            .expect("the padding is resolved before layout");
        let before_padding = self.before_padding(app);
        let after_padding = self.after_padding(app);
        let main_axis_padding = self.main_axis_padding(app);
        let cross_axis_padding = self.cross_axis_padding(app);
        let Some(child) = self.child(app) else {
            let paint_extent = paint_offset(0.0, main_axis_padding);
            let cache_extent = cache_offset(0.0, main_axis_padding);
            self.set_geometry(
                app,
                SliverGeometry::new()
                    .scroll_extent(main_axis_padding)
                    .paint_extent(paint_extent.min(constraints.remaining_paint_extent))
                    .max_paint_extent(main_axis_padding)
                    .cache_extent(cache_extent),
            );
            return;
        };
        let before_padding_paint_extent = paint_offset(0.0, before_padding);
        let mut overlap = constraints.overlap;
        if overlap > 0.0 {
            overlap = (constraints.overlap - before_padding_paint_extent).max(0.0);
        }
        let child_constraints = constraints
            .copy_with()
            .scroll_offset((constraints.scroll_offset - before_padding).max(0.0))
            .cache_origin((constraints.cache_origin + before_padding).min(0.0))
            .overlap(overlap)
            .remaining_paint_extent(
                constraints.remaining_paint_extent - paint_offset(0.0, before_padding),
            )
            .remaining_cache_extent(
                constraints.remaining_cache_extent - cache_offset(0.0, before_padding),
            )
            .cross_axis_extent((constraints.cross_axis_extent - cross_axis_padding).max(0.0))
            .preceding_scroll_extent(before_padding + constraints.preceding_scroll_extent);
        child.layout(app, child_constraints, true);

        let child_layout_geometry = child.geometry(app);
        if let Some(correction) = child_layout_geometry.scroll_offset_correction {
            self.set_geometry(
                app,
                SliverGeometry::new().scroll_offset_correction(correction),
            );
            return;
        }
        let scroll_extent = child_layout_geometry.scroll_extent;
        let before_padding_cache_extent = cache_offset(0.0, before_padding);
        let after_padding_cache_extent = cache_offset(
            before_padding + scroll_extent,
            main_axis_padding + scroll_extent,
        );
        let after_padding_paint_extent = paint_offset(
            before_padding + scroll_extent,
            main_axis_padding + scroll_extent,
        );
        let main_axis_padding_cache_extent =
            before_padding_cache_extent + after_padding_cache_extent;
        let main_axis_padding_paint_extent =
            before_padding_paint_extent + after_padding_paint_extent;
        let paint_extent = (before_padding_paint_extent
            + child_layout_geometry
                .paint_extent
                .max(child_layout_geometry.layout_extent + after_padding_paint_extent))
        .min(constraints.remaining_paint_extent);
        self.set_geometry(
            app,
            SliverGeometry::new()
                .paint_origin(child_layout_geometry.paint_origin)
                .scroll_extent(main_axis_padding + scroll_extent)
                .paint_extent(paint_extent)
                .layout_extent(
                    (main_axis_padding_paint_extent + child_layout_geometry.layout_extent)
                        .min(paint_extent),
                )
                .cache_extent(
                    (main_axis_padding_cache_extent + child_layout_geometry.cache_extent)
                        .min(constraints.remaining_cache_extent),
                )
                .max_paint_extent(main_axis_padding + child_layout_geometry.max_paint_extent)
                .hit_test_extent(
                    (main_axis_padding_paint_extent + child_layout_geometry.paint_extent)
                        .max(before_padding_paint_extent + child_layout_geometry.hit_test_extent),
                )
                .has_visual_overflow(child_layout_geometry.has_visual_overflow),
        );
        let calculated_offset = match apply_growth_direction_to_axis_direction(
            constraints.axis_direction,
            constraints.growth_direction,
        ) {
            AxisDirection::Up => paint_offset(
                resolved_padding.bottom + scroll_extent,
                resolved_padding.vertical() + scroll_extent,
            ),
            AxisDirection::Left => paint_offset(
                resolved_padding.right + scroll_extent,
                resolved_padding.horizontal() + scroll_extent,
            ),
            AxisDirection::Right => paint_offset(0.0, resolved_padding.left),
            AxisDirection::Down => paint_offset(0.0, resolved_padding.top),
        };
        let child_paint_offset = match constraints.axis() {
            Axis::Horizontal => Offset::new(calculated_offset, resolved_padding.top),
            Axis::Vertical => Offset::new(resolved_padding.left, calculated_offset),
        };
        child
            .as_object()
            .parent_data_of_mut::<SliverPhysicalParentData>(app)
            .paint_offset = child_paint_offset;
        debug_assert_eq!(before_padding, self.before_padding(app));
        debug_assert_eq!(after_padding, self.after_padding(app));
        debug_assert_eq!(main_axis_padding, self.main_axis_padding(app));
        debug_assert_eq!(cross_axis_padding, self.cross_axis_padding(app));
    }

    /// The body of Flutter's `hitTestChildren` override.
    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut SliverHitTestResult<'_>,
        main_axis_position: f64,
        cross_axis_position: f64,
    ) -> bool {
        let Some(child) = self.child(app) else {
            return false;
        };
        if child.geometry(app).hit_test_extent <= 0.0 {
            return false;
        }
        let paint_offset = child
            .as_object()
            .parent_data_of::<SliverPhysicalParentData>(app)
            .paint_offset;
        let main_axis_offset =
            RenderSliverEdgeInsetsPadding::child_main_axis_position(self, app, child.as_object());
        let cross_axis_offset =
            RenderSliverEdgeInsetsPadding::child_cross_axis_position(self, app, child.as_object());
        result.add_with_axis_offset(
            Some(paint_offset),
            main_axis_offset,
            cross_axis_offset,
            main_axis_position,
            cross_axis_position,
            |result, main, cross| child.hit_test(app, result, main, cross),
        )
    }

    /// The body of Flutter's `childMainAxisPosition` override.
    fn child_main_axis_position(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> f64 {
        debug_assert_eq!(Some(child), self.child(app).map(AnyRenderSliver::as_object));
        let constraints = self.constraints(app);
        let before_padding = self.before_padding(app);
        self.calculate_paint_offset(app, constraints, 0.0, before_padding)
    }

    /// The body of Flutter's `childCrossAxisPosition` override.
    fn child_cross_axis_position(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> f64 {
        debug_assert_eq!(Some(child), self.child(app).map(AnyRenderSliver::as_object));
        let padding = self
            .resolved_padding(app)
            .expect("the padding is resolved before layout");
        match self.constraints(app).axis() {
            Axis::Horizontal => padding.top,
            Axis::Vertical => padding.left,
        }
    }

    /// The body of Flutter's `childScrollOffset` override.
    fn child_scroll_offset(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> Option<f64> {
        debug_assert_eq!(child.parent(app), Some(self.as_object()));
        Some(self.before_padding(app))
    }

    /// The body of Flutter's `applyPaintTransform` override.
    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        debug_assert_eq!(Some(child), self.child(app).map(AnyRenderSliver::as_object));
        child
            .parent_data_of::<SliverPhysicalParentData>(app)
            .apply_paint_transform(transform);
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
        if !child.geometry(app).visible {
            return;
        }
        let paint_offset = child
            .as_object()
            .parent_data_of::<SliverPhysicalParentData>(app)
            .paint_offset;
        context.paint_child(app, child.as_object(), offset + paint_offset);
    }
}

/// Insets a [`RenderSliver`], applying padding on each side.
///
/// A [`RenderSliverPadding`] object wraps the [`SliverGeometry::layout_extent`] of its child. Any
/// incoming [`crate::SliverConstraints::overlap`] is ignored and not passed on to the child.
pub struct RenderSliverPadding {
    render_object: RenderObjectData,
    render_sliver: RenderSliverData,
    child: RenderObjectWithChildData<AnyRenderSliver>,
    padding: EdgeInsetsGeometry,
    text_direction: Option<TextDirection>,
    resolved_padding: Option<EdgeInsets>,
}

impl RenderSliverPadding {
    /// Creates a render object that insets its child in a viewport.
    ///
    /// The `padding` argument must have non-negative insets.
    pub fn new(
        app: &mut App,
        padding: EdgeInsetsGeometry,
        text_direction: Option<TextDirection>,
        child: Option<AnyRenderSliver>,
    ) -> RenderHandle<RenderSliverPadding> {
        debug_assert!(padding.is_non_negative());
        let this = RenderHandle::new_sliver(
            app,
            RenderSliverPadding {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                child: RenderObjectWithChildData::new(),
                padding,
                text_direction,
                resolved_padding: None,
            },
        );
        this.set_child(app, child);
        this
    }

    /// Flutter's `_resolve`.
    fn resolve(self: RenderHandle<Self>, app: &mut App) {
        if self.get(app).resolved_padding.is_some() {
            return;
        }
        let this = self.get(app);
        let resolved = this.padding.resolve(this.text_direction);
        debug_assert!(resolved.is_non_negative());
        self.get_mut(app).resolved_padding = Some(resolved);
    }

    /// Flutter's `_markNeedsResolution`.
    fn mark_needs_resolution(self: RenderHandle<Self>, app: &mut App) {
        self.get_mut(app).resolved_padding = None;
        self.mark_needs_layout(app);
    }

    /// The amount to pad the child in each dimension.
    ///
    /// If this is set to an [`EdgeInsetsGeometry::Directional`] value, then
    /// [`text_direction`](Self::text_direction) must not be `None`.
    pub fn padding(self: RenderHandle<Self>, app: &App) -> EdgeInsetsGeometry {
        self.get(app).padding
    }

    /// Sets [`padding`](Self::padding).
    pub fn set_padding(self: RenderHandle<Self>, app: &mut App, value: EdgeInsetsGeometry) {
        debug_assert!(value.is_non_negative());
        if self.get(app).padding == value {
            return;
        }
        self.get_mut(app).padding = value;
        self.mark_needs_resolution(app);
    }

    /// The text direction with which to resolve [`padding`](Self::padding).
    ///
    /// This may be changed to `None`, but only after the padding has been changed to a value that
    /// does not depend on the direction.
    pub fn text_direction(self: RenderHandle<Self>, app: &App) -> Option<TextDirection> {
        self.get(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TextDirection>,
    ) {
        if self.get(app).text_direction == value {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.mark_needs_resolution(app);
    }
}

impl RenderObjectWithChildMixin for RenderSliverPadding {
    type ChildType = AnyRenderSliver;

    fn child_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RenderObjectWithChildData<AnyRenderSliver> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderSliver> {
        &mut self.get_mut(app).child
    }
}

impl RenderSliverEdgeInsetsPadding for RenderSliverPadding {
    fn resolved_padding(self: RenderHandle<Self>, app: &App) -> Option<EdgeInsets> {
        self.get(app).resolved_padding
    }
}

impl RenderObject for RenderSliverPadding {
    crate::render_object_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderSliverEdgeInsetsPadding::setup_parent_data(self, app, child)
    }

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
        RenderSliverEdgeInsetsPadding::apply_paint_transform(self, app, child, transform)
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        self.resolve(app);
        RenderSliverEdgeInsetsPadding::perform_layout(self, app)
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderSliverEdgeInsetsPadding::paint(self, app, context, offset)
    }
}

impl RenderSliver for RenderSliverPadding {
    crate::render_sliver_accessors!();

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut SliverHitTestResult<'_>,
        main_axis_position: f64,
        cross_axis_position: f64,
    ) -> bool {
        RenderSliverEdgeInsetsPadding::hit_test_children(
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
        RenderSliverEdgeInsetsPadding::child_main_axis_position(self, app, child)
    }

    fn child_cross_axis_position(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> f64 {
        RenderSliverEdgeInsetsPadding::child_cross_axis_position(self, app, child)
    }

    fn child_scroll_offset(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> Option<f64> {
        RenderSliverEdgeInsetsPadding::child_scroll_offset(self, app, child)
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::Size;
    use reveal_foundation::AppCell;
    use reveal_foundation::Handle;
    use reveal_gestures::HitTestResult;

    use super::*;
    use crate::box_::{AnyRenderBox, BoxConstraints, RenderBox};
    use crate::pipeline_owner::PipelineOwner;
    use crate::sliver::{RenderSliverToBoxAdapter, SliverPhysicalParentData};
    use crate::sliver_multi_box_adaptor::test_support::hit_testable_box;
    use crate::viewport::RenderViewport;
    use crate::viewport_offset::{FixedViewportOffset, ViewportOffset};

    /// Lays a sliver out in a viewport of the given size, without a `RenderView`.
    fn lay_out(
        app: &mut App,
        sliver: AnyRenderSliver,
        size: Size,
        scroll_offset: f64,
    ) -> (Handle<PipelineOwner>, RenderHandle<RenderViewport>) {
        let offset = FixedViewportOffset::new(app, scroll_offset);
        let viewport = RenderViewport::new(
            app,
            AxisDirection::Right,
            offset.as_viewport_offset(),
            Some(vec![sliver]),
            None,
        );
        let owner = PipelineOwner::new(app, None);
        owner.set_root_node(app, Some(viewport.as_object()));
        viewport
            .as_box()
            .layout(app, BoxConstraints::tight(size), false);
        (owner, viewport)
    }

    fn padded_box(
        app: &mut App,
        padding: EdgeInsetsGeometry,
        height: f64,
    ) -> (RenderHandle<RenderSliverPadding>, AnyRenderBox) {
        let child_box = hit_testable_box(app, height);
        let adapter = RenderSliverToBoxAdapter::new(app, Some(child_box));
        let padding = RenderSliverPadding::new(
            app,
            padding,
            Some(TextDirection::Ltr),
            Some(adapter.as_sliver()),
        );
        (padding, child_box)
    }

    /// `sliver_padding_test.dart`: the padding adds to the sliver's scroll extent, and the child
    /// is offset by the leading padding.
    #[test]
    fn padding_insets_the_child_and_grows_the_scroll_extent() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (padding, child_box) = padded_box(
            &mut app,
            EdgeInsetsGeometry::from_ltrb(10.0, 20.0, 30.0, 40.0),
            100.0,
        );
        lay_out(&mut app, padding.as_sliver(), Size::new(200.0, 400.0), 0.0);

        let geometry = padding.geometry(&app);
        assert_eq!(geometry.scroll_extent, 160.0);
        assert_eq!(geometry.paint_extent, 160.0);
        // The child is squeezed in the cross axis by the horizontal padding.
        assert_eq!(child_box.size(&app), Size::new(160.0, 100.0));
        assert_eq!(padding.before_padding(&app), 20.0);
        assert_eq!(padding.after_padding(&app), 40.0);
        assert_eq!(padding.main_axis_padding(&app), 60.0);
        assert_eq!(padding.cross_axis_padding(&app), 40.0);
        let child = padding.child(&app).expect("has a child");
        assert_eq!(
            child
                .as_object()
                .parent_data_of::<SliverPhysicalParentData>(&app)
                .paint_offset,
            Offset::new(10.0, 20.0)
        );
        assert_eq!(
            RenderSliver::child_scroll_offset(padding, &app, child.as_object()),
            Some(20.0)
        );
    }

    /// Scrolling past the leading padding consumes it before the child starts to scroll.
    #[test]
    fn scrolling_consumes_the_leading_padding_first() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (padding, _) = padded_box(
            &mut app,
            EdgeInsetsGeometry::from_ltrb(0.0, 20.0, 0.0, 0.0),
            100.0,
        );
        lay_out(&mut app, padding.as_sliver(), Size::new(200.0, 400.0), 10.0);

        let child = padding.child(&app).expect("has a child");
        // Only half the leading padding has scrolled off, so the child is still at its start.
        assert_eq!(child.constraints(&app).scroll_offset, 0.0);
        assert_eq!(
            RenderSliverEdgeInsetsPadding::child_main_axis_position(
                padding,
                &app,
                child.as_object()
            ),
            10.0
        );
        assert_eq!(padding.geometry(&app).scroll_extent, 120.0);
    }

    /// A padding sliver with no child is just the padding.
    #[test]
    fn a_childless_padding_is_the_padding_alone() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let padding = RenderSliverPadding::new(
            &mut app,
            EdgeInsetsGeometry::all(15.0),
            Some(TextDirection::Ltr),
            None,
        );
        lay_out(&mut app, padding.as_sliver(), Size::new(200.0, 400.0), 0.0);

        let geometry = padding.geometry(&app);
        assert_eq!(geometry.scroll_extent, 30.0);
        assert_eq!(geometry.paint_extent, 30.0);
        assert_eq!(geometry.max_paint_extent, 30.0);
    }

    /// A hit inside the child, past the leading padding, reaches it.
    #[test]
    fn padding_hit_tests_its_child_at_the_inset_position() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (padding, _) = padded_box(
            &mut app,
            EdgeInsetsGeometry::from_ltrb(10.0, 20.0, 30.0, 40.0),
            100.0,
        );
        lay_out(&mut app, padding.as_sliver(), Size::new(200.0, 400.0), 0.0);

        let mut inside = HitTestResult::new();
        let hit_inside = padding.hit_test(
            &mut app,
            &mut SliverHitTestResult::wrap(&mut inside),
            25.0,
            20.0,
        );
        let mut in_padding = HitTestResult::new();
        let hit_in_padding = padding.hit_test(
            &mut app,
            &mut SliverHitTestResult::wrap(&mut in_padding),
            5.0,
            20.0,
        );

        let child = padding.child(&app).expect("has a child");
        // The cross axis is inset by the left padding.
        assert_eq!(
            RenderSliver::child_cross_axis_position(padding, &app, child.as_object()),
            10.0
        );
        // A hit past the leading padding reaches the box; one inside the padding does not.
        assert!(hit_inside);
        assert_eq!(inside.path().len(), 3);
        assert!(!hit_in_padding);
    }

    /// Changing the padding re-resolves it and marks the sliver for layout.
    #[test]
    fn setting_the_padding_re_resolves_it() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (padding, _) = padded_box(&mut app, EdgeInsetsGeometry::all(10.0), 100.0);
        lay_out(&mut app, padding.as_sliver(), Size::new(200.0, 400.0), 0.0);
        assert_eq!(padding.resolved_padding(&app), Some(EdgeInsets::all(10.0)));

        padding.set_padding(&mut app, EdgeInsetsGeometry::all(25.0));

        assert_eq!(padding.resolved_padding(&app), None);
        assert!(padding.as_object().debug_needs_layout(&app));
    }
}
