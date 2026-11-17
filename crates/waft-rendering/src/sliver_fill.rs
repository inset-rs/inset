//! Flutter counterpart: `rendering/sliver_fill.dart`.
//!
//! `RenderSliverFillViewport.visitChildrenForSemantics` and the semantics update its
//! `allowImplicitScrolling` setter asks for wait for accessibility.

use std::rc::Rc;

use waft_embedder::{Matrix4, Offset};
use waft_foundation::App;
use waft_painting::{Axis, AxisDirection};

use crate::box_::AnyRenderBox;
use crate::object::{
    AnyRenderObject, ContainerRenderObjectData, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin,
};
use crate::painting_context::PaintingContext;
use crate::sliver::{
    RenderSliver, RenderSliverData, RenderSliverHelpers, RenderSliverSingleBoxAdapter,
    RenderSliverSingleBoxAdapterBase, SliverConstraints, SliverGeometry, SliverHitTestResult,
    SliverPhysicalParentData, apply_growth_direction_to_axis_direction, calculate_cache_offset,
    calculate_paint_offset,
};
use crate::sliver_fixed_extent_list::{
    RenderSliverFixedExtentBoxAdaptor, RenderSliverFixedExtentBoxAdaptorData,
    fixed_extent_box_adaptor_leaf,
};
use crate::sliver_multi_box_adaptor::{
    RenderSliverBoxChildManager, RenderSliverMultiBoxAdaptorData,
};

/// A sliver that contains multiple box children that each fill the viewport.
///
/// [`RenderSliverFillViewport`] places its children in a linear array along the main axis. Each
/// child is sized to fill the viewport, both in the main and cross axis. A
/// [`viewport_fraction`](Self::viewport_fraction) factor can be provided to size the children to
/// a multiple of the viewport's main axis dimension (typically a fraction less than 1.0).
///
/// See also:
///
///  * [`RenderSliverFillRemaining`], which sizes the children based on the remaining space rather
///    than the viewport itself.
///  * [`crate::RenderSliverFixedExtentList`], which has a configurable item extent.
///  * [`crate::RenderSliverList`], which does not require its children to have the same extent
///    in the main axis.
pub struct RenderSliverFillViewport {
    render_object: RenderObjectData,
    render_sliver: RenderSliverData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    adaptor: RenderSliverMultiBoxAdaptorData,
    fixed_extent: RenderSliverFixedExtentBoxAdaptorData,
    viewport_fraction: f64,
    allow_implicit_scrolling: bool,
}

impl RenderSliverFillViewport {
    /// Creates a sliver that contains multiple box children that each fill the viewport.
    ///
    /// Dart's defaults: `viewport_fraction` 1.0, `allow_implicit_scrolling` true.
    pub fn new(
        app: &mut App,
        child_manager: Rc<dyn RenderSliverBoxChildManager>,
        viewport_fraction: f64,
        allow_implicit_scrolling: bool,
    ) -> RenderHandle<RenderSliverFillViewport> {
        debug_assert!(viewport_fraction > 0.0);
        RenderHandle::new_sliver(
            app,
            RenderSliverFillViewport {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                container: ContainerRenderObjectData::new(),
                adaptor: RenderSliverMultiBoxAdaptorData::new(child_manager),
                fixed_extent: RenderSliverFixedExtentBoxAdaptorData::new(),
                viewport_fraction,
                allow_implicit_scrolling,
            },
        )
    }

    /// The fraction of the viewport that each child should fill in the main axis.
    ///
    /// If this fraction is less than 1.0, more than one child will be visible at once. If this
    /// fraction is greater than 1.0, each child will be larger than the viewport in the main
    /// axis.
    pub fn viewport_fraction(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).viewport_fraction
    }

    /// See [`viewport_fraction`](Self::viewport_fraction).
    pub fn set_viewport_fraction(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).viewport_fraction == value {
            return;
        }
        self.get_mut(app).viewport_fraction = value;
        self.mark_needs_layout(app);
    }

    /// Whether the pages answer implicit scrolling, as `PageView::allow_implicit_scrolling`
    /// describes.
    ///
    /// This is typically used by assistive technologies, such as screen readers.
    pub fn allow_implicit_scrolling(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).allow_implicit_scrolling
    }

    /// See [`allow_implicit_scrolling`](Self::allow_implicit_scrolling).
    pub fn set_allow_implicit_scrolling(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).allow_implicit_scrolling == value {
            return;
        }
        self.get_mut(app).allow_implicit_scrolling = value;
    }
}

impl RenderSliverFixedExtentBoxAdaptor for RenderSliverFillViewport {
    crate::render_sliver_fixed_extent_accessors!();

    fn item_extent(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        Some(self.constraints(app).viewport_main_axis_extent * self.get(app).viewport_fraction)
    }
}

fixed_extent_box_adaptor_leaf!(RenderSliverFillViewport);

/// Declares the fields, the constructor and the [`RenderObjectWithChildMixin`],
/// [`RenderSliverHelpers`] and [`RenderSliver`] impls that every [`RenderSliverSingleBoxAdapter`]
/// leaf of this file shares; each leaf writes its own `impl RenderObject` for its layout.
macro_rules! single_box_adapter_leaf {
    ($(#[$doc:meta])* $leaf:ident, $(#[$new_doc:meta])* new) => {
        $(#[$doc])*
        pub struct $leaf {
            render_object: RenderObjectData,
            render_sliver: RenderSliverData,
            child: RenderObjectWithChildData<AnyRenderBox>,
        }

        impl $leaf {
            $(#[$new_doc])*
            pub fn new(app: &mut App, child: Option<AnyRenderBox>) -> RenderHandle<$leaf> {
                let this = RenderHandle::new_sliver(
                    app,
                    $leaf {
                        render_object: RenderObjectData::new(),
                        render_sliver: RenderSliverData::new(),
                        child: RenderObjectWithChildData::new(),
                    },
                );
                this.set_child(app, child);
                this
            }
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

        impl RenderSliver for $leaf {
            crate::render_sliver_accessors!();

            fn hit_test_children(
                self: RenderHandle<Self>,
                app: &mut App,
                result: &mut SliverHitTestResult<'_>,
                main_axis_position: f64,
                cross_axis_position: f64,
            ) -> bool {
                RenderSliverSingleBoxAdapter::hit_test_children(
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
                RenderSliverSingleBoxAdapter::child_main_axis_position(self, app, child)
            }
        }
    };
}

/// The [`RenderObject`] members a [`RenderSliverSingleBoxAdapter`] leaf of this file inherits,
/// other than its layout. Write it inside the leaf's `impl RenderObject`.
macro_rules! single_box_adapter_object_overrides {
    () => {
        crate::render_object_accessors!();

        fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
            RenderSliverSingleBoxAdapter::setup_parent_data(self, app, child)
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
            RenderSliverSingleBoxAdapter::apply_paint_transform(self, app, child, transform)
        }

        fn paint(
            self: RenderHandle<Self>,
            app: &mut App,
            context: &mut PaintingContext,
            offset: Offset,
        ) {
            RenderSliverSingleBoxAdapter::paint(self, app, context, offset)
        }
    };
}

/// The child's extent in the main axis when it lays out at its maximum intrinsic size.
fn max_intrinsic_main_axis_extent(
    app: &mut App,
    child: AnyRenderBox,
    constraints: SliverConstraints,
) -> f64 {
    match constraints.axis() {
        Axis::Horizontal => child.get_max_intrinsic_width(app, constraints.cross_axis_extent),
        Axis::Vertical => child.get_max_intrinsic_height(app, constraints.cross_axis_extent),
    }
}

single_box_adapter_leaf!(
    /// A sliver that contains a single box child that contains a scrollable and fills the
    /// viewport.
    ///
    /// [`RenderSliverFillRemainingWithScrollable`] sizes its child to fill the viewport in the
    /// cross axis and to fill the remaining space in the viewport in the main axis.
    ///
    /// Typically this will be the last sliver in a viewport, since (by definition) there is
    /// never any room for anything beyond this sliver.
    ///
    /// See also:
    ///
    ///  * `NestedScrollView`, which uses this sliver for the inner scrollable.
    ///  * [`RenderSliverFillRemaining`], which lays out its non-scrollable child slightly
    ///    different than this widget.
    ///  * [`RenderSliverFillRemainingAndOverscroll`], which incorporates the overscroll into the
    ///    remaining space to fill.
    ///  * [`RenderSliverFillViewport`], which sizes its children based on the size of the
    ///    viewport, regardless of what else is in the scroll view.
    ///  * [`crate::RenderSliverList`], which shows a list of variable-sized children in a
    ///    viewport.
    RenderSliverFillRemainingWithScrollable,
    /// Creates a [`RenderSliver`] that wraps a scrollable [`crate::RenderBox`] which is sized to
    /// fit the remaining space in the viewport.
    new
);

impl RenderSliverSingleBoxAdapter for RenderSliverFillRemainingWithScrollable {}

impl RenderObject for RenderSliverFillRemainingWithScrollable {
    single_box_adapter_object_overrides!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let extent = constraints.remaining_paint_extent - constraints.overlap.min(0.0);

        let cache_extent =
            calculate_cache_offset(constraints, 0.0, constraints.viewport_main_axis_extent);
        if let Some(child) = self.child(app) {
            let mut max_extent = extent;

            // If sliver has no extent, but is within viewport's cacheExtent, use the
            // sliver's cacheExtent as the maxExtent so that it does not get dropped
            // from the semantic tree.
            if extent == 0.0 && cache_extent > 0.0 {
                max_extent = cache_extent;
            }
            child.layout(
                app,
                constraints.as_box_constraints(extent, max_extent, None),
                false,
            );
        }

        let painted_child_size = calculate_paint_offset(constraints, 0.0, extent);
        debug_assert!(painted_child_size.is_finite());
        debug_assert!(painted_child_size >= 0.0);

        let geometry = SliverGeometry::builder()
            .scroll_extent(constraints.viewport_main_axis_extent)
            .paint_extent(painted_child_size)
            .max_paint_extent(painted_child_size)
            .has_visual_overflow(
                extent > constraints.remaining_paint_extent || constraints.scroll_offset > 0.0,
            )
            .cache_extent(cache_extent)
            .build();
        self.set_geometry(app, geometry);
        if let Some(child) = self.child(app) {
            RenderSliverSingleBoxAdapter::set_child_parent_data(
                self,
                app,
                child.as_object(),
                constraints,
                geometry,
            );
        }
    }
}

single_box_adapter_leaf!(
    /// A sliver that contains a single box child that is non-scrollable and fills the remaining
    /// space in the viewport.
    ///
    /// [`RenderSliverFillRemaining`] sizes its child to fill the viewport in the cross axis and
    /// to fill the remaining space in the viewport in the main axis.
    ///
    /// Typically this will be the last sliver in a viewport, since (by definition) there is
    /// never any room for anything beyond this sliver.
    ///
    /// See also:
    ///
    ///  * [`RenderSliverFillRemainingWithScrollable`], which lays out its scrollable child
    ///    slightly different than this widget.
    ///  * [`RenderSliverFillRemainingAndOverscroll`], which incorporates the overscroll into the
    ///    remaining space to fill.
    ///  * [`RenderSliverFillViewport`], which sizes its children based on the size of the
    ///    viewport, regardless of what else is in the scroll view.
    ///  * [`crate::RenderSliverList`], which shows a list of variable-sized children in a
    ///    viewport.
    RenderSliverFillRemaining,
    /// Creates a [`RenderSliver`] that wraps a non-scrollable [`crate::RenderBox`] which is sized
    /// to fit the remaining space in the viewport.
    new
);

impl RenderSliverSingleBoxAdapter for RenderSliverFillRemaining {}

impl RenderObject for RenderSliverFillRemaining {
    single_box_adapter_object_overrides!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        // The remaining space in the viewportMainAxisExtent. Can be <= 0 if we have
        // scrolled beyond the extent of the screen.
        let mut extent =
            constraints.viewport_main_axis_extent - constraints.preceding_scroll_extent;

        if let Some(child) = self.child(app) {
            let child_extent = max_intrinsic_main_axis_extent(app, child, constraints);

            // If the childExtent is greater than the computed extent, we want to use
            // that instead of potentially cutting off the child. This allows us to
            // safely specify a maxExtent.
            extent = extent.max(child_extent);
            child.layout(
                app,
                constraints.as_box_constraints(extent, extent, None),
                false,
            );
        }

        debug_assert!(
            extent.is_finite(),
            "The calculated extent for the child of SliverFillRemaining is not finite. This can \
             happen if the child is a scrollable, in which case, the hasScrollBody property of \
             SliverFillRemaining should not be set to false."
        );
        let painted_child_size = calculate_paint_offset(constraints, 0.0, extent);
        debug_assert!(painted_child_size.is_finite());
        debug_assert!(painted_child_size >= 0.0);

        let cache_extent = calculate_cache_offset(constraints, 0.0, extent);
        let geometry = SliverGeometry::builder()
            .scroll_extent(extent)
            .paint_extent(painted_child_size)
            .max_paint_extent(painted_child_size)
            .has_visual_overflow(
                extent > constraints.remaining_paint_extent || constraints.scroll_offset > 0.0,
            )
            .cache_extent(cache_extent)
            .build();
        self.set_geometry(app, geometry);
        if let Some(child) = self.child(app) {
            RenderSliverSingleBoxAdapter::set_child_parent_data(
                self,
                app,
                child.as_object(),
                constraints,
                geometry,
            );
        }
    }
}

single_box_adapter_leaf!(
    /// A sliver that contains a single box child that is non-scrollable and fills the remaining
    /// space in the viewport including any overscrolled area.
    ///
    /// [`RenderSliverFillRemainingAndOverscroll`] sizes its child to fill the viewport in the
    /// cross axis and to fill the remaining space in the viewport in the main axis with the
    /// overscroll area included.
    ///
    /// Typically this will be the last sliver in a viewport, since (by definition) there is
    /// never any room for anything beyond this sliver.
    ///
    /// See also:
    ///
    ///  * [`RenderSliverFillRemainingWithScrollable`], which lays out its scrollable child
    ///    without overscroll.
    ///  * [`RenderSliverFillRemaining`], which lays out its non-scrollable child without
    ///    overscroll.
    ///  * [`RenderSliverFillViewport`], which sizes its children based on the size of the
    ///    viewport, regardless of what else is in the scroll view.
    ///  * [`crate::RenderSliverList`], which shows a list of variable-sized children in a
    ///    viewport.
    RenderSliverFillRemainingAndOverscroll,
    /// Creates a [`RenderSliver`] that wraps a non-scrollable [`crate::RenderBox`] which is sized
    /// to fit the remaining space plus any overscroll in the viewport.
    new
);

impl RenderSliverSingleBoxAdapter for RenderSliverFillRemainingAndOverscroll {
    fn set_child_parent_data(
        self: RenderHandle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        constraints: SliverConstraints,
        geometry: SliverGeometry,
    ) {
        RenderSliverSingleBoxAdapterBase::set_child_parent_data(
            self,
            app,
            child,
            constraints,
            geometry,
        );
        let Some(child_box) = child.as_box().filter(|child_box| child_box.has_size(app)) else {
            return;
        };
        // A child stretched into the overscroll area is larger than the
        // scrollExtent, so for reversed axis directions it must be anchored by its
        // actual size to keep its leading edge glued to the viewport's edge.
        let child_size = child_box.size(app);
        let paint_offset = match apply_growth_direction_to_axis_direction(
            constraints.axis_direction,
            constraints.growth_direction,
        ) {
            AxisDirection::Up => Offset::new(
                0.0,
                geometry.paint_extent + constraints.scroll_offset - child_size.height(),
            ),
            AxisDirection::Left => Offset::new(
                geometry.paint_extent + constraints.scroll_offset - child_size.width(),
                0.0,
            ),
            AxisDirection::Right | AxisDirection::Down => return,
        };
        child
            .parent_data_of_mut::<SliverPhysicalParentData>(app)
            .paint_offset = paint_offset;
    }
}

impl RenderObject for RenderSliverFillRemainingAndOverscroll {
    single_box_adapter_object_overrides!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        // The remaining space in the viewportMainAxisExtent. Can be <= 0 if we have
        // scrolled beyond the extent of the screen.
        let mut extent =
            constraints.viewport_main_axis_extent - constraints.preceding_scroll_extent;
        // The maxExtent includes any overscrolled area. Can be < 0 if we have
        // overscroll in the opposite direction, away from the end of the list.
        //
        // When overscrolling past this sliver, the viewport clamps
        // remainingPaintExtent to the viewport extent and delivers the
        // overscrolled distance as this sliver's scrollOffset, so the
        // scrollOffset term is needed for the stretched child to still reach the
        // viewport's trailing edge.
        let mut max_extent = constraints.remaining_paint_extent - constraints.overlap.min(0.0)
            + constraints.scroll_offset;

        if let Some(child) = self.child(app) {
            let child_extent = max_intrinsic_main_axis_extent(app, child, constraints);

            // If the childExtent is greater than the computed extent, we want to use
            // that instead of potentially cutting off the child. This allows us to
            // safely specify a maxExtent.
            extent = extent.max(child_extent);
            // The extent could be larger than the maxExtent due to a larger child
            // size or overscrolling at the top of the scrollable (rather than at the
            // end where this sliver is).
            max_extent = extent.max(max_extent);
            child.layout(
                app,
                constraints.as_box_constraints(extent, max_extent, None),
                true,
            );
        }

        debug_assert!(
            extent.is_finite(),
            "The calculated extent for the child of SliverFillRemaining is not finite. This can \
             happen if the child is a scrollable, in which case, the hasScrollBody property of \
             SliverFillRemaining should not be set to false."
        );
        let painted_child_size = calculate_paint_offset(constraints, 0.0, extent);
        debug_assert!(painted_child_size.is_finite());
        debug_assert!(painted_child_size >= 0.0);

        let cache_extent = calculate_cache_offset(constraints, 0.0, extent);
        let geometry = SliverGeometry::builder()
            .scroll_extent(extent)
            .paint_extent(max_extent.min(constraints.remaining_paint_extent))
            .max_paint_extent(max_extent)
            .has_visual_overflow(
                extent > constraints.remaining_paint_extent || constraints.scroll_offset > 0.0,
            )
            .cache_extent(cache_extent)
            .build();
        self.set_geometry(app, geometry);
        if let Some(child) = self.child(app) {
            RenderSliverSingleBoxAdapter::set_child_parent_data(
                self,
                app,
                child.as_object(),
                constraints,
                geometry,
            );
        }
    }
}
