//! Flutter counterpart: `rendering/sliver_fixed_extent_list.dart`.

use std::rc::Rc;

use reveal_embedder::{Matrix4, Offset};
use reveal_foundation::{App, Handle, PRECISION_ERROR_TOLERANCE};

use crate::box_::{AnyRenderBox, BoxConstraints};
use crate::object::{
    AnyRenderObject, ContainerRenderObjectData, ContainerRenderObjectMixin, RenderHandle,
    RenderObject, RenderObjectData,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;
use crate::sliver::{
    ItemExtentBuilder, RenderSliver, RenderSliverData, RenderSliverHelpers, SliverConstraints,
    SliverGeometry, SliverHitTestResult, SliverLayoutDimensions, calculate_cache_offset,
    calculate_paint_offset,
};
use crate::sliver_multi_box_adaptor::{
    RenderSliverBoxChildManager, RenderSliverMultiBoxAdaptor, RenderSliverMultiBoxAdaptorData,
    RenderSliverWithKeepAliveMixin, SliverMultiBoxAdaptorParentData,
};

/// Flutter's `RenderSliverFixedExtentBoxAdaptor` field.
#[derive(Debug, Default)]
pub struct RenderSliverFixedExtentBoxAdaptorData {
    current_layout_dimensions: Option<SliverLayoutDimensions>,
}

impl RenderSliverFixedExtentBoxAdaptorData {
    /// Creates the state of a fixed-extent box adaptor that has not been laid out.
    pub const fn new() -> RenderSliverFixedExtentBoxAdaptorData {
        RenderSliverFixedExtentBoxAdaptorData {
            current_layout_dimensions: None,
        }
    }
}

/// Implements [`RenderSliverFixedExtentBoxAdaptor`] field accessors for a `fixed_extent` field.
#[macro_export]
macro_rules! render_sliver_fixed_extent_accessors {
    () => {
        fn fixed_extent_data(
            self: $crate::RenderHandle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::RenderSliverFixedExtentBoxAdaptorData {
            &self.get(app).fixed_extent
        }
        fn fixed_extent_data_mut(
            self: $crate::RenderHandle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::RenderSliverFixedExtentBoxAdaptorData {
            &mut self.get_mut(app).fixed_extent
        }
    };
}

/// A sliver that contains multiple box children that have the same extent in the main axis.
///
/// Flutter's `RenderSliverFixedExtentBoxAdaptor`. An implementor answers either
/// [`item_extent`](Self::item_extent) or [`item_extent_builder`](Self::item_extent_builder).
pub trait RenderSliverFixedExtentBoxAdaptor: RenderSliverMultiBoxAdaptor {
    /// Mixin field access.
    fn fixed_extent_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RenderSliverFixedExtentBoxAdaptorData;

    /// See [`fixed_extent_data`](Self::fixed_extent_data).
    fn fixed_extent_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderSliverFixedExtentBoxAdaptorData;

    /// The main-axis extent of each item.
    ///
    /// If this is `Some`, [`item_extent_builder`](Self::item_extent_builder) must be `None`.
    /// If this is `None`, `item_extent_builder` must be `Some`.
    fn item_extent(self: RenderHandle<Self>, app: &App) -> Option<f64>;

    /// The main-axis extent builder of each item.
    ///
    /// If this is `Some`, [`item_extent`](Self::item_extent) must be `None`.
    /// If this is `None`, `item_extent` must be `Some`.
    fn item_extent_builder(self: RenderHandle<Self>, app: &App) -> Option<ItemExtentBuilder> {
        let _ = app;
        let _ = self;
        None
    }

    /// The layout dimensions for the sliver.
    ///
    /// If the sliver has not been laid out yet, this returns a [`SliverLayoutDimensions`] based
    /// on the current constraints.
    fn layout_dimensions(self: RenderHandle<Self>, app: &App) -> SliverLayoutDimensions {
        self.fixed_extent_data(app)
            .current_layout_dimensions
            .unwrap_or_else(|| {
                let constraints = self.constraints(app);
                SliverLayoutDimensions::new(
                    constraints.scroll_offset,
                    constraints.preceding_scroll_extent,
                    constraints.viewport_main_axis_extent,
                    constraints.cross_axis_extent,
                )
            })
    }

    /// The layout offset for the child with the given index.
    ///
    /// This function uses the returned value of
    /// [`item_extent_builder`](Self::item_extent_builder) or the
    /// [`item_extent`](Self::item_extent) to avoid recomputing item size repeatedly during
    /// layout.
    ///
    /// By default, places the children in order, without gaps, starting from layout offset zero.
    fn index_to_layout_offset(self: RenderHandle<Self>, app: &App, index: i32) -> f64 {
        let Some(builder) = self.item_extent_builder(app) else {
            let item_extent = self.item_extent(app).expect("one of the two is set");
            return item_extent * f64::from(index);
        };
        let mut offset = 0.0;
        let dimensions = self.layout_dimensions(app);
        let child_manager = self.child_manager(app);
        for i in 0..index {
            let child_count = child_manager.estimated_child_count(app);
            if child_count.is_some_and(|count| i > count - 1) {
                break;
            }
            let Some(item_extent) = builder(i, dimensions) else {
                break;
            };
            offset += item_extent;
        }
        offset
    }

    /// The minimum child index that is visible at the given scroll offset.
    ///
    /// By default, returns a value consistent with the children being placed in order, without
    /// gaps, starting from layout offset zero.
    fn get_min_child_index_for_scroll_offset(
        self: RenderHandle<Self>,
        app: &App,
        scroll_offset: f64,
    ) -> i32 {
        let Some(builder) = self.item_extent_builder(app) else {
            let item_extent = self.item_extent(app).expect("one of the two is set");
            if item_extent <= 0.0 {
                return 0;
            }
            let actual = scroll_offset / item_extent;
            let round = actual.round();
            if (actual * item_extent - round * item_extent).abs() < PRECISION_ERROR_TOLERANCE {
                return round as i32;
            }
            return actual.floor() as i32;
        };
        self.child_index_for_scroll_offset(app, scroll_offset, &builder)
    }

    /// The maximum child index that is visible at the given scroll offset.
    ///
    /// By default, returns a value consistent with the children being placed in order, without
    /// gaps, starting from layout offset zero.
    fn get_max_child_index_for_scroll_offset(
        self: RenderHandle<Self>,
        app: &App,
        scroll_offset: f64,
    ) -> i32 {
        let Some(builder) = self.item_extent_builder(app) else {
            let item_extent = self.item_extent(app).expect("one of the two is set");
            if item_extent <= 0.0 {
                return 0;
            }
            let actual = scroll_offset / item_extent - 1.0;
            let round = actual.round();
            if (actual * item_extent - round * item_extent).abs() < PRECISION_ERROR_TOLERANCE {
                return (round as i32).max(0);
            }
            return (actual.ceil() as i32).max(0);
        };
        self.child_index_for_scroll_offset(app, scroll_offset, &builder)
    }

    /// Flutter's `_getChildIndexForScrollOffset`.
    fn child_index_for_scroll_offset(
        self: RenderHandle<Self>,
        app: &App,
        scroll_offset: f64,
        builder: &ItemExtentBuilder,
    ) -> i32 {
        if scroll_offset == 0.0 {
            return 0;
        }
        let dimensions = self.layout_dimensions(app);
        let child_manager = self.child_manager(app);
        let mut position = 0.0;
        let mut index = 0;
        while position < scroll_offset {
            let child_count = child_manager.estimated_child_count(app);
            if child_count.is_some_and(|count| index > count - 1) {
                break;
            }
            let Some(item_extent) = builder(index, dimensions) else {
                break;
            };
            position += item_extent;
            index += 1;
        }
        index - 1
    }

    /// Called to estimate the total scrollable extents of this object.
    ///
    /// Must return the total distance from the start of the child with the earliest possible
    /// index to the end of the child with the last possible index.
    ///
    /// By default, defers to [`RenderSliverBoxChildManager::estimate_max_scroll_offset`].
    fn estimate_max_scroll_offset(
        self: RenderHandle<Self>,
        app: &App,
        constraints: SliverConstraints,
        first_index: Option<i32>,
        last_index: Option<i32>,
        leading_scroll_offset: Option<f64>,
        trailing_scroll_offset: Option<f64>,
    ) -> f64 {
        self.child_manager(app).estimate_max_scroll_offset(
            app,
            constraints,
            first_index,
            last_index,
            leading_scroll_offset,
            trailing_scroll_offset,
        )
    }

    /// Called to obtain a precise measure of the total scrollable extents of this object.
    ///
    /// This is used when no child is available for the index corresponding to the current scroll
    /// offset, to determine the precise dimensions of the sliver.
    ///
    /// If [`item_extent_builder`](Self::item_extent_builder) is `None`, multiplies the
    /// [`item_extent`](Self::item_extent) by the number of children reported by
    /// [`RenderSliverBoxChildManager::child_count`]. Otherwise, sums the extents of the first
    /// `child_count` children.
    fn compute_max_scroll_offset(
        self: RenderHandle<Self>,
        app: &mut App,
        _constraints: SliverConstraints,
    ) -> f64 {
        let child_manager = self.child_manager(app);
        let Some(builder) = self.item_extent_builder(app) else {
            let item_extent = self.item_extent(app).expect("one of the two is set");
            return f64::from(child_manager.child_count(app)) * item_extent;
        };
        let dimensions = self.layout_dimensions(app);
        let mut offset = 0.0;
        for i in 0..child_manager.child_count(app) {
            let Some(item_extent) = builder(i, dimensions) else {
                break;
            };
            offset += item_extent;
        }
        offset
    }

    /// Flutter's `_getChildConstraints`.
    fn child_constraints(self: RenderHandle<Self>, app: &App, index: i32) -> BoxConstraints {
        let extent = match self.item_extent_builder(app) {
            None => self.item_extent(app).expect("one of the two is set"),
            Some(builder) => builder(index, self.layout_dimensions(app))
                .expect("the builder must report an extent for a reified child"),
        };
        self.constraints(app)
            .as_box_constraints(extent, extent, None)
    }

    /// The body of Flutter's `paintExtentOf` override.
    fn paint_extent_of(self: RenderHandle<Self>, app: &App, child: AnyRenderBox) -> f64 {
        match self.item_extent_builder(app) {
            None => self.item_extent(app).expect("one of the two is set"),
            Some(builder) => builder(self.index_of(app, child), self.layout_dimensions(app))
                .expect("the builder must report an extent for a reified child"),
        }
    }

    /// The body of Flutter's `performLayout` override.
    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        debug_assert!(
            self.item_extent(app).is_some() != self.item_extent_builder(app).is_some(),
            "exactly one of item_extent and item_extent_builder must be set"
        );
        debug_assert!(
            self.item_extent_builder(app).is_some()
                || self
                    .item_extent(app)
                    .is_some_and(|extent| extent.is_finite() && extent >= 0.0)
        );

        let constraints = self.constraints(app);
        let child_manager = self.child_manager(app);
        child_manager.did_start_layout(app);
        child_manager.set_did_underflow(app, false);

        let scroll_offset = constraints.scroll_offset + constraints.cache_origin;
        debug_assert!(scroll_offset >= 0.0);
        let remaining_extent = constraints.remaining_cache_extent;
        debug_assert!(remaining_extent >= 0.0);
        let target_end_scroll_offset = scroll_offset + remaining_extent;

        self.fixed_extent_data_mut(app).current_layout_dimensions =
            Some(SliverLayoutDimensions::new(
                constraints.scroll_offset,
                constraints.preceding_scroll_extent,
                constraints.viewport_main_axis_extent,
                constraints.cross_axis_extent,
            ));

        let first_index = self.get_min_child_index_for_scroll_offset(app, scroll_offset);
        let target_last_index = target_end_scroll_offset
            .is_finite()
            .then(|| self.get_max_child_index_for_scroll_offset(app, target_end_scroll_offset));

        if self.first_child(app).is_some() {
            let leading_garbage = self.calculate_leading_garbage(app, first_index);
            let trailing_garbage = match target_last_index {
                Some(last_index) => self.calculate_trailing_garbage(app, last_index),
                None => 0,
            };
            self.collect_garbage(app, leading_garbage, trailing_garbage);
        } else {
            self.collect_garbage(app, 0, 0);
        }

        if self.first_child(app).is_none() {
            let layout_offset = self.index_to_layout_offset(app, first_index);
            if !self.add_initial_child(app, first_index, layout_offset) {
                // There are either no children, or we are past the end of all our children.
                let max = if first_index <= 0 {
                    0.0
                } else {
                    self.compute_max_scroll_offset(app, constraints)
                };
                self.set_geometry(
                    app,
                    SliverGeometry::new()
                        .scroll_extent(max)
                        .max_paint_extent(max),
                );
                child_manager.did_finish_layout(app);
                return;
            }
        }

        let mut trailing_child_with_layout: Option<AnyRenderBox> = None;

        let first_child = self.first_child(app).expect("added above");
        let mut index = self.index_of(app, first_child) - 1;
        while index >= first_index {
            let child_constraints = self.child_constraints(app, index);
            let Some(child) = self.insert_and_layout_leading_child(app, child_constraints, false)
            else {
                // Items before the previously first child are no longer present. Reset the scroll
                // offset to offset all items prior and up to the missing item. Let the parent
                // re-lay-out everything.
                let correction = self.index_to_layout_offset(app, index);
                self.set_geometry(
                    app,
                    SliverGeometry::new().scroll_offset_correction(correction),
                );
                return;
            };
            let layout_offset = self.index_to_layout_offset(app, index);
            let child_parent_data = child
                .as_object()
                .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(app);
            child_parent_data.set_layout_offset(Some(layout_offset));
            debug_assert_eq!(child_parent_data.index, Some(index));
            trailing_child_with_layout.get_or_insert(child);
            index -= 1;
        }

        if trailing_child_with_layout.is_none() {
            let first_child = self.first_child(app).expect("the child list is not empty");
            let child_constraints = self.child_constraints(app, self.index_of(app, first_child));
            first_child.layout(app, child_constraints, false);
            let layout_offset = self.index_to_layout_offset(app, first_index);
            first_child
                .as_object()
                .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(app)
                .set_layout_offset(Some(layout_offset));
            trailing_child_with_layout = Some(first_child);
        }

        let mut estimated_max_scroll_offset = f64::INFINITY;
        let mut index = self.index_of(app, trailing_child_with_layout.expect("set above")) + 1;
        while target_last_index.is_none_or(|last_index| index <= last_index) {
            let after = trailing_child_with_layout.expect("set above");
            let mut child = self.child_after(app, after);
            if child.is_none_or(|current| self.index_of(app, current) != index) {
                let child_constraints = self.child_constraints(app, index);
                child = self.insert_and_layout_child(app, child_constraints, Some(after), false);
                if child.is_none() {
                    // We have run out of children.
                    estimated_max_scroll_offset = self.index_to_layout_offset(app, index);
                    break;
                }
            } else {
                let child_constraints = self.child_constraints(app, index);
                child
                    .expect("checked")
                    .layout(app, child_constraints, false);
            }
            let child = child.expect("set above");
            trailing_child_with_layout = Some(child);
            debug_assert_eq!(
                child
                    .as_object()
                    .parent_data_of::<SliverMultiBoxAdaptorParentData>(app)
                    .index,
                Some(index)
            );
            let layout_offset = self.index_to_layout_offset(app, index);
            child
                .as_object()
                .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(app)
                .set_layout_offset(Some(layout_offset));
            index += 1;
        }

        let last_child = self.last_child(app).expect("the child list is not empty");
        let last_index = self.index_of(app, last_child);
        let leading_scroll_offset = self.index_to_layout_offset(app, first_index);
        let trailing_scroll_offset = self.index_to_layout_offset(app, last_index + 1);

        debug_assert!(
            first_index == 0
                || RenderSliver::child_scroll_offset(
                    self,
                    app,
                    self.first_child(app).expect("not empty").as_object()
                )
                .expect("laid out")
                    - scroll_offset
                    <= PRECISION_ERROR_TOLERANCE
        );
        debug_assert!(self.debug_assert_child_list_is_non_empty_and_contiguous(app));
        debug_assert_eq!(
            self.index_of(app, self.first_child(app).expect("not empty")),
            first_index
        );
        debug_assert!(target_last_index.is_none_or(|target| last_index <= target));

        estimated_max_scroll_offset =
            estimated_max_scroll_offset.min(self.estimate_max_scroll_offset(
                app,
                constraints,
                Some(first_index),
                Some(last_index),
                Some(leading_scroll_offset),
                Some(trailing_scroll_offset),
            ));

        let paint_extent =
            calculate_paint_offset(constraints, leading_scroll_offset, trailing_scroll_offset);
        let cache_extent =
            calculate_cache_offset(constraints, leading_scroll_offset, trailing_scroll_offset);

        let target_end_scroll_offset_for_paint =
            constraints.scroll_offset + constraints.remaining_paint_extent;
        let target_last_index_for_paint =
            target_end_scroll_offset_for_paint.is_finite().then(|| {
                self.get_max_child_index_for_scroll_offset(app, target_end_scroll_offset_for_paint)
            });

        self.set_geometry(
            app,
            SliverGeometry::new()
                .scroll_extent(estimated_max_scroll_offset)
                .paint_extent(paint_extent)
                .cache_extent(cache_extent)
                .max_paint_extent(estimated_max_scroll_offset)
                // Conservative to avoid flickering away the clip during scroll.
                .has_visual_overflow(
                    target_last_index_for_paint.is_some_and(|target| last_index >= target)
                        || constraints.scroll_offset > 0.0,
                ),
        );

        // We may have started the layout while scrolled to the end, which would not expose a new
        // child.
        if estimated_max_scroll_offset == trailing_scroll_offset {
            child_manager.set_did_underflow(app, true);
        }
        child_manager.did_finish_layout(app);
    }
}

/// Declares the [`ContainerRenderObjectMixin`], [`RenderObject`] and [`RenderSliver`] impls that
/// a [`RenderSliverFixedExtentBoxAdaptor`] leaf forwards to its base traits.
macro_rules! fixed_extent_box_adaptor_leaf {
    ($leaf:ty) => {
        impl ContainerRenderObjectMixin for $leaf {
            type ChildType = AnyRenderBox;
            type ParentDataType = SliverMultiBoxAdaptorParentData;

            fn container_data(
                self: RenderHandle<Self>,
                app: &App,
            ) -> &ContainerRenderObjectData<AnyRenderBox> {
                &self.get(app).container
            }

            fn container_data_mut(
                self: RenderHandle<Self>,
                app: &mut App,
            ) -> &mut ContainerRenderObjectData<AnyRenderBox> {
                &mut self.get_mut(app).container
            }

            fn insert(
                self: RenderHandle<Self>,
                app: &mut App,
                child: AnyRenderBox,
                after: Option<AnyRenderBox>,
            ) {
                RenderSliverMultiBoxAdaptor::insert(self, app, child, after)
            }

            fn move_child(
                self: RenderHandle<Self>,
                app: &mut App,
                child: AnyRenderBox,
                after: Option<AnyRenderBox>,
            ) {
                RenderSliverMultiBoxAdaptor::move_child(self, app, child, after)
            }

            fn remove(self: RenderHandle<Self>, app: &mut App, child: AnyRenderBox) {
                RenderSliverMultiBoxAdaptor::remove(self, app, child)
            }

            fn remove_all(self: RenderHandle<Self>, app: &mut App) {
                RenderSliverMultiBoxAdaptor::remove_all(self, app)
            }
        }

        impl RenderSliverHelpers for $leaf {}

        impl RenderSliverWithKeepAliveMixin for $leaf {}

        impl RenderSliverMultiBoxAdaptor for $leaf {
            crate::render_sliver_multi_box_adaptor_accessors!();

            fn paint_extent_of(self: RenderHandle<Self>, app: &App, child: AnyRenderBox) -> f64 {
                RenderSliverFixedExtentBoxAdaptor::paint_extent_of(self, app, child)
            }
        }

        impl RenderObject for $leaf {
            crate::render_object_accessors!();

            fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
                RenderSliverMultiBoxAdaptor::setup_parent_data(self, app, child);
                RenderSliverWithKeepAliveMixin::setup_parent_data(self, app, child);
            }

            fn visit_children(
                self: RenderHandle<Self>,
                app: &App,
                visitor: &mut dyn FnMut(AnyRenderObject),
            ) {
                RenderSliverMultiBoxAdaptor::visit_children(self, app, visitor)
            }

            fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
                RenderSliverMultiBoxAdaptor::did_attach(self, app, owner)
            }

            fn did_detach(self: RenderHandle<Self>, app: &mut App) {
                RenderSliverMultiBoxAdaptor::did_detach(self, app)
            }

            fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
                RenderSliverMultiBoxAdaptor::redepth_children(self, app)
            }

            fn apply_paint_transform(
                self: RenderHandle<Self>,
                app: &App,
                child: AnyRenderObject,
                transform: &mut Matrix4,
            ) {
                RenderSliverMultiBoxAdaptor::apply_paint_transform(self, app, child, transform)
            }

            fn paint(
                self: RenderHandle<Self>,
                app: &mut App,
                context: &mut PaintingContext,
                offset: Offset,
            ) {
                RenderSliverMultiBoxAdaptor::paint(self, app, context, offset)
            }

            fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
                RenderSliverFixedExtentBoxAdaptor::perform_layout(self, app)
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
                RenderSliverMultiBoxAdaptor::hit_test_children(
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
                RenderSliverMultiBoxAdaptor::child_main_axis_position(self, app, child)
            }

            fn child_scroll_offset(
                self: RenderHandle<Self>,
                app: &App,
                child: AnyRenderObject,
            ) -> Option<f64> {
                RenderSliverMultiBoxAdaptor::child_scroll_offset(self, app, child)
            }
        }
    };
}

/// A sliver that places multiple box children with the same main axis extent in a linear array.
///
/// [`RenderSliverFixedExtentList`] places its children in a linear array along the main axis
/// starting at offset zero and without gaps. Each child is forced to have the
/// [`item_extent`](Self::item_extent) in the main axis and the
/// [`crate::SliverConstraints::cross_axis_extent`] in the cross axis.
///
/// [`RenderSliverFixedExtentList`] is more efficient than [`crate::RenderSliverList`] because it
/// does not need to perform layout on its children to obtain their extent in the main axis.
pub struct RenderSliverFixedExtentList {
    render_object: RenderObjectData,
    render_sliver: RenderSliverData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    adaptor: RenderSliverMultiBoxAdaptorData,
    fixed_extent: RenderSliverFixedExtentBoxAdaptorData,
    item_extent: f64,
}

impl RenderSliverFixedExtentList {
    /// Creates a sliver that contains multiple box children that have a given extent in the main
    /// axis.
    pub fn new(
        app: &mut App,
        child_manager: Rc<dyn RenderSliverBoxChildManager>,
        item_extent: f64,
    ) -> RenderHandle<RenderSliverFixedExtentList> {
        RenderHandle::new_sliver(
            app,
            RenderSliverFixedExtentList {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                container: ContainerRenderObjectData::new(),
                adaptor: RenderSliverMultiBoxAdaptorData::new(child_manager),
                fixed_extent: RenderSliverFixedExtentBoxAdaptorData::new(),
                item_extent,
            },
        )
    }

    /// Sets [`RenderSliverFixedExtentBoxAdaptor::item_extent`].
    pub fn set_item_extent(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).item_extent == value {
            return;
        }
        self.get_mut(app).item_extent = value;
        self.mark_needs_layout(app);
    }
}

impl RenderSliverFixedExtentBoxAdaptor for RenderSliverFixedExtentList {
    crate::render_sliver_fixed_extent_accessors!();

    fn item_extent(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        Some(self.get(app).item_extent)
    }
}

fixed_extent_box_adaptor_leaf!(RenderSliverFixedExtentList);

/// A sliver that places multiple box children with the corresponding main axis extent in a linear
/// array.
pub struct RenderSliverVariedExtentList {
    render_object: RenderObjectData,
    render_sliver: RenderSliverData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    adaptor: RenderSliverMultiBoxAdaptorData,
    fixed_extent: RenderSliverFixedExtentBoxAdaptorData,
    item_extent_builder: ItemExtentBuilder,
}

impl RenderSliverVariedExtentList {
    /// Creates a sliver that contains multiple box children that have an explicit extent in the
    /// main axis.
    pub fn new(
        app: &mut App,
        child_manager: Rc<dyn RenderSliverBoxChildManager>,
        item_extent_builder: ItemExtentBuilder,
    ) -> RenderHandle<RenderSliverVariedExtentList> {
        RenderHandle::new_sliver(
            app,
            RenderSliverVariedExtentList {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                container: ContainerRenderObjectData::new(),
                adaptor: RenderSliverMultiBoxAdaptorData::new(child_manager),
                fixed_extent: RenderSliverFixedExtentBoxAdaptorData::new(),
                item_extent_builder,
            },
        )
    }

    /// Sets [`RenderSliverFixedExtentBoxAdaptor::item_extent_builder`].
    pub fn set_item_extent_builder(
        self: RenderHandle<Self>,
        app: &mut App,
        value: ItemExtentBuilder,
    ) {
        if Rc::ptr_eq(&self.get(app).item_extent_builder, &value) {
            return;
        }
        self.get_mut(app).item_extent_builder = value;
        self.mark_needs_layout(app);
    }
}

impl RenderSliverFixedExtentBoxAdaptor for RenderSliverVariedExtentList {
    crate::render_sliver_fixed_extent_accessors!();

    fn item_extent(self: RenderHandle<Self>, _app: &App) -> Option<f64> {
        let _ = self;
        None
    }

    fn item_extent_builder(self: RenderHandle<Self>, app: &App) -> Option<ItemExtentBuilder> {
        Some(Rc::clone(&self.get(app).item_extent_builder))
    }
}

fixed_extent_box_adaptor_leaf!(RenderSliverVariedExtentList);

#[cfg(test)]
mod tests {
    use reveal_embedder::Size;
    use reveal_gestures::HitTestResult;

    use super::*;
    use crate::sliver_multi_box_adaptor::test_support::TestChildManager;
    use crate::sliver_multi_box_adaptor::viewport_test_support::ScrollHarness;

    const VIEWPORT: Size = Size::new(100.0, 200.0);

    fn fixed_list(
        app: &mut App,
        extents: Vec<f64>,
        item_extent: f64,
    ) -> (
        RenderHandle<RenderSliverFixedExtentList>,
        Rc<TestChildManager<RenderSliverFixedExtentList>>,
        ScrollHarness,
    ) {
        let manager = TestChildManager::new(extents);
        let list = RenderSliverFixedExtentList::new(app, manager.clone(), item_extent);
        manager.attach(list);
        let harness = ScrollHarness::new(app, list.as_sliver(), VIEWPORT);
        (list, manager, harness)
    }

    /// `sliver_fixed_extent_list_test.dart`: only the children that fall in the viewport (plus
    /// the cache area) are reified, and they are placed at multiples of the item extent.
    #[test]
    fn a_fixed_extent_list_reifies_only_the_visible_children() {
        let mut app = App::new();
        let (list, _, _harness) = fixed_list(&mut app, vec![50.0; 40], 50.0);

        assert_eq!(
            list.index_of(&app, list.first_child(&app).expect("built")),
            0
        );
        assert_eq!(list.index_to_layout_offset(&app, 3), 150.0);
        let geometry = list.geometry(&app);
        assert_eq!(geometry.scroll_extent, 2000.0);
        assert_eq!(geometry.paint_extent, 200.0);
        assert!(geometry.has_visual_overflow);
        // 200 pixels of viewport plus 250 of cache, at 50 pixels each, is far short of 40.
        assert!(list.child_count(&app) < 40);
    }

    /// Scrolling collects the children that left the viewport and creates the ones that entered.
    #[test]
    fn scrolling_collects_and_creates_children() {
        let mut app = App::new();
        let (list, _, harness) = fixed_list(&mut app, vec![50.0; 40], 50.0);
        assert_eq!(
            list.index_of(&app, list.first_child(&app).expect("built")),
            0
        );

        harness.scroll_to(&mut app, 600.0);

        let first = list.first_child(&app).expect("children were created");
        // 600 - 250 of leading cache = 350, which is index 7.
        assert_eq!(list.index_of(&app, first), 7);
        assert_eq!(
            RenderSliver::child_scroll_offset(list, &app, first.as_object()),
            Some(350.0)
        );
        assert_eq!(
            RenderSliver::child_main_axis_position(list, &app, first.as_object()),
            -250.0
        );
    }

    /// A hit inside a reified child lands on the child and the sliver.
    #[test]
    fn a_fixed_extent_list_hit_tests_its_children() {
        let mut app = App::new();
        let (list, _, _harness) = fixed_list(&mut app, vec![50.0; 40], 50.0);

        let mut result = HitTestResult::new();
        let hit = list.hit_test(
            &mut app,
            &mut SliverHitTestResult::wrap(&mut result),
            75.0,
            20.0,
        );

        assert!(hit);
        assert_eq!(result.path().len(), 2);
    }

    /// A child whose parent data asks to be kept alive moves to the keep-alive bucket instead of
    /// being destroyed, and comes back out of it when it is needed again.
    #[test]
    fn a_kept_alive_child_survives_leaving_the_viewport() {
        let mut app = App::new();
        let (list, manager, harness) = fixed_list(&mut app, vec![50.0; 40], 50.0);
        let first = list.first_child(&app).expect("children were created");
        first
            .as_object()
            .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(&mut app)
            .set_keep_alive(true);

        harness.scroll_to(&mut app, 1000.0);

        // The child is out of the child list but still attached, in the keep-alive bucket.
        assert_eq!(list.keep_alive_children(&app), vec![first]);
        assert!(
            first
                .as_object()
                .parent_data_of::<SliverMultiBoxAdaptorParentData>(&app)
                .kept_alive()
        );
        assert!(!list.paints_child(&app, first));
        let created_before = manager.created.borrow().len();

        harness.scroll_to(&mut app, 0.0);

        // Index 0 came back from the bucket, so the manager was not asked to create it again.
        assert_eq!(list.first_child(&app), Some(first));
        assert!(list.keep_alive_children(&app).is_empty());
        assert!(list.paints_child(&app, first));
        assert!(
            !manager.created.borrow()[created_before..].contains(&0),
            "the kept-alive child was rebuilt"
        );
    }

    /// A list shorter than the viewport reports underflow and its precise extent.
    #[test]
    fn a_short_list_reports_underflow() {
        let mut app = App::new();
        let (list, manager, _harness) = fixed_list(&mut app, vec![50.0; 2], 50.0);

        assert_eq!(list.geometry(&app).scroll_extent, 100.0);
        assert_eq!(list.geometry(&app).paint_extent, 100.0);
        assert!(manager.did_underflow.get());
    }

    /// `RenderSliverVariedExtentList` asks its builder for each item's extent.
    #[test]
    fn a_varied_extent_list_uses_its_builder() {
        let mut app = App::new();
        let manager = TestChildManager::<RenderSliverVariedExtentList>::new(vec![40.0; 10]);
        let builder: ItemExtentBuilder =
            Rc::new(|index, _| Some(if index % 2 == 0 { 40.0 } else { 20.0 }));
        let list = RenderSliverVariedExtentList::new(&mut app, manager.clone(), builder);
        manager.attach(list);
        let _harness = ScrollHarness::new(&mut app, list.as_sliver(), VIEWPORT);

        // 40 + 20 + 40 + 20 = 120.
        assert_eq!(list.index_to_layout_offset(&app, 4), 120.0);
        assert_eq!(list.item_extent(&app), None);
        assert_eq!(list.geometry(&app).paint_extent, 200.0);
        // The manager estimates the total extent of its ten items.
        assert_eq!(list.geometry(&app).scroll_extent, 400.0);
    }

    /// The minimum and maximum child indices bracket the visible window.
    #[test]
    fn the_child_index_range_brackets_the_scroll_offset() {
        let mut app = App::new();
        let (list, _, _harness) = fixed_list(&mut app, vec![50.0; 40], 50.0);
        let constraints = list.constraints(&app);

        assert_eq!(list.get_min_child_index_for_scroll_offset(&app, 120.0), 2);
        assert_eq!(list.get_max_child_index_for_scroll_offset(&app, 120.0), 2);
        assert_eq!(list.get_max_child_index_for_scroll_offset(&app, 0.0), 0);
        assert_eq!(
            list.compute_max_scroll_offset(&mut app, constraints),
            2000.0
        );
    }
}
