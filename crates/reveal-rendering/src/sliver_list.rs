//! Flutter counterpart: `rendering/sliver_list.dart`.

use std::rc::Rc;

use reveal_embedder::{Matrix4, Offset};
use reveal_foundation::{App, Handle, PRECISION_ERROR_TOLERANCE};

use crate::box_::AnyRenderBox;
use crate::object::{
    AnyRenderObject, ContainerRenderObjectData, ContainerRenderObjectMixin, RenderHandle,
    RenderObject, RenderObjectData,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;
use crate::sliver::{
    RenderSliver, RenderSliverData, RenderSliverHelpers, SliverGeometry, SliverHitTestResult,
};
use crate::sliver_multi_box_adaptor::{
    RenderSliverBoxChildManager, RenderSliverMultiBoxAdaptor, RenderSliverMultiBoxAdaptorData,
    RenderSliverWithKeepAliveMixin, SliverMultiBoxAdaptorParentData,
};

/// A sliver that places multiple box children in a linear array along the main axis.
///
/// Each child is forced to have the [`crate::SliverConstraints::cross_axis_extent`] in the cross
/// axis but determines its own main axis extent.
///
/// [`RenderSliverList`] determines its scroll offset by "dead reckoning" because children outside
/// the visible part of the sliver are not materialized, which means [`RenderSliverList`] cannot
/// learn their main axis extent. Instead, newly materialized children are placed adjacent to
/// existing children.
pub struct RenderSliverList {
    render_object: RenderObjectData,
    render_sliver: RenderSliverData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    adaptor: RenderSliverMultiBoxAdaptorData,
}

impl RenderSliverList {
    /// Creates a sliver that places multiple box children in a linear array along the main axis.
    pub fn new(
        app: &mut App,
        child_manager: Rc<dyn RenderSliverBoxChildManager>,
    ) -> RenderHandle<RenderSliverList> {
        RenderHandle::new_sliver(
            app,
            RenderSliverList {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                container: ContainerRenderObjectData::new(),
                adaptor: RenderSliverMultiBoxAdaptorData::new(child_manager),
            },
        )
    }
}

impl ContainerRenderObjectMixin for RenderSliverList {
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

impl RenderSliverHelpers for RenderSliverList {}

impl RenderSliverWithKeepAliveMixin for RenderSliverList {}

impl RenderSliverMultiBoxAdaptor for RenderSliverList {
    crate::render_sliver_multi_box_adaptor_accessors!();
}

impl RenderObject for RenderSliverList {
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
        let constraints = self.constraints(app);
        let child_manager = self.child_manager(app);
        child_manager.did_start_layout(app);
        child_manager.set_did_underflow(app, false);

        let scroll_offset = constraints.scroll_offset + constraints.cache_origin;
        debug_assert!(scroll_offset >= 0.0);
        let remaining_extent = constraints.remaining_cache_extent;
        debug_assert!(remaining_extent >= 0.0);
        let target_end_scroll_offset = scroll_offset + remaining_extent;
        let child_constraints = constraints.as_box_constraints(0.0, f64::INFINITY, None);
        let mut leading_garbage = 0;
        let mut trailing_garbage = 0;
        let mut reached_end = false;

        // This algorithm in principle is straight-forward: find the first child that overlaps the
        // given scroll_offset, creating more children at the top of the list if necessary, then
        // walk down the list updating and laying out each child and adding more at the end if
        // necessary until we have enough children to cover the entire viewport.
        //
        // It is complicated by one minor issue, which is that any time you update or create a
        // child, it's possible that some of the children that haven't yet been laid out will be
        // removed, leaving the list in an inconsistent state, and requiring that missing nodes be
        // recreated.
        //
        // To keep this mess tractable, this algorithm starts from what is currently the first
        // child, if any, and then walks up and/or down from there, so that the nodes that might
        // get removed are always at the edges of what has already been laid out.

        // Make sure we have at least one child to start from.
        if self.first_child(app).is_none() && !self.add_initial_child(app, 0, 0.0) {
            // There are no children.
            self.set_geometry(app, SliverGeometry::ZERO);
            child_manager.did_finish_layout(app);
            return;
        }

        // We have at least one child.

        // These variables track the range of children that we have laid out. Within this range,
        // the children have consecutive indices. Outside this range, it's possible for a child to
        // get removed without notice.
        let mut leading_child_with_layout: Option<AnyRenderBox> = None;
        let mut trailing_child_with_layout: Option<AnyRenderBox> = None;

        let mut earliest_useful_child = self.first_child(app);

        // A first child with no layout offset is likely a result of children reordering.
        //
        // We rely on the first child to have an accurate layout offset. In the case of no layout
        // offset, we have to find the first child that has a valid layout offset.
        let first_child = self.first_child(app).expect("checked above");
        if RenderSliver::child_scroll_offset(self, app, first_child.as_object()).is_none() {
            let mut leading_children_without_layout_offset = 0;
            while let Some(current) = earliest_useful_child {
                if RenderSliver::child_scroll_offset(self, app, current.as_object()).is_some() {
                    break;
                }
                earliest_useful_child = self.child_after(app, current);
                leading_children_without_layout_offset += 1;
            }
            // We should be able to destroy children with no layout offset safely, because they
            // are likely outside of the viewport.
            self.collect_garbage(app, leading_children_without_layout_offset, 0);
            // If we cannot find a valid layout offset, start from the initial child.
            if self.first_child(app).is_none() && !self.add_initial_child(app, 0, 0.0) {
                // There are no children.
                self.set_geometry(app, SliverGeometry::ZERO);
                child_manager.did_finish_layout(app);
                return;
            }
        }

        // Find the last child that is at or before the scroll offset.
        earliest_useful_child = self.first_child(app);
        loop {
            let current = earliest_useful_child.expect("the child list is not empty");
            let earliest_scroll_offset =
                RenderSliver::child_scroll_offset(self, app, current.as_object())
                    .expect("a laid-out child has a scroll offset");
            if earliest_scroll_offset <= scroll_offset {
                break;
            }
            // We have to add children before the earliest useful child.
            earliest_useful_child =
                self.insert_and_layout_leading_child(app, child_constraints, true);
            let Some(inserted) = earliest_useful_child else {
                let first_child = self.first_child(app).expect("the child list is not empty");
                first_child
                    .as_object()
                    .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(app)
                    .set_layout_offset(Some(0.0));

                if scroll_offset == 0.0 {
                    // insert_and_layout_leading_child only lays out the children before the first
                    // child. In this case, nothing has been laid out. We have to lay out the
                    // first child manually.
                    first_child.layout(app, child_constraints, true);
                    leading_child_with_layout = Some(first_child);
                    trailing_child_with_layout.get_or_insert(first_child);
                    break;
                }
                // We ran out of children before reaching the scroll offset. We must inform our
                // parent that this sliver cannot fulfill its contract and that we need a scroll
                // offset correction.
                self.set_geometry(
                    app,
                    SliverGeometry::new().scroll_offset_correction(-scroll_offset),
                );
                return;
            };

            let first_child = self.first_child(app).expect("the child list is not empty");
            let first_child_scroll_offset =
                earliest_scroll_offset - self.paint_extent_of(app, first_child);
            // first_child_scroll_offset may contain double precision error
            if first_child_scroll_offset < -PRECISION_ERROR_TOLERANCE {
                // Let's assume there is no child before the first child. We will correct it on
                // the next layout if it is not.
                self.set_geometry(
                    app,
                    SliverGeometry::new().scroll_offset_correction(-first_child_scroll_offset),
                );
                first_child
                    .as_object()
                    .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(app)
                    .set_layout_offset(Some(0.0));
                return;
            }

            inserted
                .as_object()
                .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(app)
                .set_layout_offset(Some(first_child_scroll_offset));
            debug_assert_eq!(Some(inserted), self.first_child(app));
            leading_child_with_layout = Some(inserted);
            trailing_child_with_layout.get_or_insert(inserted);
        }

        debug_assert!(
            RenderSliver::child_scroll_offset(
                self,
                app,
                self.first_child(app).expect("not empty").as_object()
            )
            .expect("laid out")
                > -PRECISION_ERROR_TOLERANCE
        );

        // If the scroll offset is at zero, we should make sure we are actually at the beginning
        // of the list.
        if scroll_offset < PRECISION_ERROR_TOLERANCE {
            // We iterate from the first child in case the leading child has a 0 paint extent.
            loop {
                let first_child = self.first_child(app).expect("the child list is not empty");
                if self.index_of(app, first_child) <= 0 {
                    break;
                }
                let earliest_scroll_offset =
                    RenderSliver::child_scroll_offset(self, app, first_child.as_object())
                        .expect("laid out");
                // We correct one child at a time. If there are more children before the earliest
                // useful child, we will correct it once the scroll offset reaches zero again.
                let inserted = self.insert_and_layout_leading_child(app, child_constraints, true);
                debug_assert!(inserted.is_some());
                let first_child = self.first_child(app).expect("the child list is not empty");
                let first_child_scroll_offset =
                    earliest_scroll_offset - self.paint_extent_of(app, first_child);
                first_child
                    .as_object()
                    .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(app)
                    .set_layout_offset(Some(0.0));
                // We only need to correct if the leading child actually has a paint extent.
                if first_child_scroll_offset < -PRECISION_ERROR_TOLERANCE {
                    self.set_geometry(
                        app,
                        SliverGeometry::new().scroll_offset_correction(-first_child_scroll_offset),
                    );
                    return;
                }
            }
        }

        // At this point, the earliest useful child is the first child, and is a child whose
        // scroll offset is at or before the scroll offset, and leading_child_with_layout and
        // trailing_child_with_layout are either None or cover a range of render boxes that we
        // have laid out with the first being the same as the earliest useful child and the last
        // being either at or after the scroll offset.
        let earliest_useful_child = self.first_child(app).expect("the child list is not empty");

        // Make sure we've laid out at least one child. Dart also assigns
        // `leadingChildWithLayout` here; nothing reads it after this point.
        if leading_child_with_layout.is_none() {
            earliest_useful_child.layout(app, child_constraints, true);
            trailing_child_with_layout = Some(earliest_useful_child);
        }

        // Here, the earliest useful child is still the first child, it's got a scroll offset that
        // is at or before our actual scroll offset, and it has been laid out, and is in fact our
        // leading child with layout. It's possible that some children beyond that one have also
        // been laid out.
        let mut in_layout_range = true;
        let mut child = Some(earliest_useful_child);
        let mut index = self.index_of(app, earliest_useful_child);
        let mut end_scroll_offset =
            RenderSliver::child_scroll_offset(self, app, earliest_useful_child.as_object())
                .expect("laid out")
                + self.paint_extent_of(app, earliest_useful_child);

        // Dart's local `advance()`, which is used in two different places below.
        let advance = |app: &mut App,
                       child: &mut Option<AnyRenderBox>,
                       index: &mut i32,
                       end_scroll_offset: &mut f64,
                       in_layout_range: &mut bool,
                       trailing_child_with_layout: &mut Option<AnyRenderBox>|
         -> bool {
            // returns true if we advanced, false if we have no more children
            let current = child.expect("advance is only called with a child");
            if Some(current) == *trailing_child_with_layout {
                *in_layout_range = false;
            }
            *child = self.child_after(app, current);
            if child.is_none() {
                *in_layout_range = false;
            }
            *index += 1;
            if !*in_layout_range {
                if child.is_none_or(|current| self.index_of(app, current) != *index) {
                    // We are missing a child. Insert it (and lay it out) if possible.
                    *child = self.insert_and_layout_child(
                        app,
                        child_constraints,
                        *trailing_child_with_layout,
                        true,
                    );
                    if child.is_none() {
                        // We have run out of children.
                        return false;
                    }
                } else {
                    // Lay out the child.
                    child.expect("checked").layout(app, child_constraints, true);
                }
                *trailing_child_with_layout = *child;
            }
            let current = child.expect("set above");
            let child_parent_data = current
                .as_object()
                .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(app);
            child_parent_data.set_layout_offset(Some(*end_scroll_offset));
            debug_assert_eq!(child_parent_data.index, Some(*index));
            *end_scroll_offset = RenderSliver::child_scroll_offset(self, app, current.as_object())
                .expect("just set")
                + self.paint_extent_of(app, current);
            true
        };

        // Find the first child that ends after the scroll offset.
        while end_scroll_offset < scroll_offset {
            leading_garbage += 1;
            if !advance(
                app,
                &mut child,
                &mut index,
                &mut end_scroll_offset,
                &mut in_layout_range,
                &mut trailing_child_with_layout,
            ) {
                debug_assert_eq!(leading_garbage, self.child_count(app));
                debug_assert!(child.is_none());
                // we want to make sure we keep the last child around so we know the end scroll
                // offset
                self.collect_garbage(app, leading_garbage - 1, 0);
                debug_assert_eq!(self.first_child(app), self.last_child(app));
                let last_child = self.last_child(app).expect("one child is kept");
                let extent = RenderSliver::child_scroll_offset(self, app, last_child.as_object())
                    .expect("laid out")
                    + self.paint_extent_of(app, last_child);
                self.set_geometry(
                    app,
                    SliverGeometry::new()
                        .scroll_extent(extent)
                        .max_paint_extent(extent),
                );
                return;
            }
        }

        // Now find the first child that ends after our end.
        while end_scroll_offset < target_end_scroll_offset {
            if !advance(
                app,
                &mut child,
                &mut index,
                &mut end_scroll_offset,
                &mut in_layout_range,
                &mut trailing_child_with_layout,
            ) {
                reached_end = true;
                break;
            }
        }

        // Finally count up all the remaining children and label them as garbage.
        if let Some(current) = child {
            let mut walker = self.child_after(app, current);
            while let Some(current) = walker {
                trailing_garbage += 1;
                walker = self.child_after(app, current);
            }
        }

        // At this point everything should be good to go, we just have to clean up the garbage and
        // report the geometry.
        self.collect_garbage(app, leading_garbage, trailing_garbage);

        debug_assert!(self.debug_assert_child_list_is_non_empty_and_contiguous(app));
        let first_child = self.first_child(app).expect("not empty");
        let last_child = self.last_child(app).expect("not empty");
        let leading_scroll_offset =
            RenderSliver::child_scroll_offset(self, app, first_child.as_object())
                .expect("laid out");
        let estimated_max_scroll_offset = if reached_end {
            end_scroll_offset
        } else {
            let estimated = child_manager.estimate_max_scroll_offset(
                app,
                constraints,
                Some(self.index_of(app, first_child)),
                Some(self.index_of(app, last_child)),
                Some(leading_scroll_offset),
                Some(end_scroll_offset),
            );
            debug_assert!(estimated >= end_scroll_offset - leading_scroll_offset);
            estimated
        };
        let paint_extent =
            self.calculate_paint_offset(app, constraints, leading_scroll_offset, end_scroll_offset);
        let cache_extent =
            self.calculate_cache_offset(app, constraints, leading_scroll_offset, end_scroll_offset);
        let target_end_scroll_offset_for_paint =
            constraints.scroll_offset + constraints.remaining_paint_extent;
        self.set_geometry(
            app,
            SliverGeometry::new()
                .scroll_extent(estimated_max_scroll_offset)
                .paint_extent(paint_extent)
                .cache_extent(cache_extent)
                .max_paint_extent(estimated_max_scroll_offset)
                // Conservative to avoid flickering away the clip during scroll.
                .has_visual_overflow(
                    end_scroll_offset > target_end_scroll_offset_for_paint
                        || constraints.scroll_offset > 0.0,
                ),
        );

        // We may have started the layout while scrolled to the end, which would not expose a new
        // child.
        if estimated_max_scroll_offset == end_scroll_offset {
            child_manager.set_did_underflow(app, true);
        }
        child_manager.did_finish_layout(app);
    }
}

impl RenderSliver for RenderSliverList {
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

#[cfg(test)]
mod tests {
    use reveal_embedder::Size;
    use reveal_foundation::AppCell;
    use reveal_gestures::HitTestResult;

    use super::*;
    use crate::sliver_multi_box_adaptor::test_support::TestChildManager;
    use crate::sliver_multi_box_adaptor::viewport_test_support::ScrollHarness;

    const VIEWPORT: Size = Size::new(100.0, 200.0);

    fn sliver_list(
        app: &mut App,
        extents: Vec<f64>,
    ) -> (
        RenderHandle<RenderSliverList>,
        Rc<TestChildManager<RenderSliverList>>,
        ScrollHarness,
    ) {
        let manager = TestChildManager::new(extents);
        let list = RenderSliverList::new(app, manager.clone());
        manager.attach(list);
        let harness = ScrollHarness::new(app, list.as_sliver(), VIEWPORT);
        (list, manager, harness)
    }

    /// `sliver_list_test.dart`: children of differing extents are placed one after the other, and
    /// each child's layout offset is the sum of the extents before it.
    #[test]
    fn a_sliver_list_stacks_children_of_differing_extents() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (list, _, _harness) = sliver_list(&mut app, vec![40.0, 60.0, 80.0, 100.0, 120.0]);

        let first = list.first_child(&app).expect("children were created");
        let second = list.child_after(&app, first).expect("two children");
        let third = list.child_after(&app, second).expect("three children");
        assert_eq!(
            RenderSliver::child_scroll_offset(list, &app, first.as_object()),
            Some(0.0)
        );
        assert_eq!(
            RenderSliver::child_scroll_offset(list, &app, second.as_object()),
            Some(40.0)
        );
        assert_eq!(
            RenderSliver::child_scroll_offset(list, &app, third.as_object()),
            Some(100.0)
        );
        assert_eq!(list.geometry(&app).paint_extent, 200.0);
        assert!(list.geometry(&app).has_visual_overflow);
    }

    /// A list that fits inside the viewport reports its exact extent and no overflow.
    #[test]
    fn a_short_sliver_list_reports_its_exact_extent() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (list, manager, _harness) = sliver_list(&mut app, vec![40.0, 60.0]);

        let geometry = list.geometry(&app);
        assert_eq!(geometry.scroll_extent, 100.0);
        assert_eq!(geometry.paint_extent, 100.0);
        assert!(!geometry.has_visual_overflow);
        assert_eq!(list.child_count(&app), 2);
        assert!(manager.did_underflow.get());
    }

    /// Scrolling drops the children that left the cache area and creates the ones that entered.
    #[test]
    fn scrolling_a_sliver_list_replaces_its_children() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (list, _, harness) = sliver_list(&mut app, vec![100.0; 30]);
        assert_eq!(
            list.index_of(&app, list.first_child(&app).expect("built")),
            0
        );

        harness.scroll_to(&mut app, 1500.0);

        let first = list.first_child(&app).expect("children were created");
        // 1500 - 250 of leading cache = 1250, which is inside index 12.
        assert_eq!(list.index_of(&app, first), 12);
        assert!(list.geometry(&app).has_visual_overflow);
    }

    /// A hit inside a reified child lands on the child and the sliver.
    #[test]
    fn a_sliver_list_hit_tests_its_children() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (list, _, _harness) = sliver_list(&mut app, vec![50.0; 20]);

        let mut result = HitTestResult::new();
        let hit = list.hit_test(
            &mut app,
            &mut SliverHitTestResult::wrap(&mut result),
            120.0,
            10.0,
        );

        assert!(hit);
        assert_eq!(result.path().len(), 2);
    }

    /// A kept-alive child is moved to the bucket rather than destroyed when it scrolls away.
    #[test]
    fn a_sliver_list_keeps_a_child_alive() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (list, _, harness) = sliver_list(&mut app, vec![100.0; 30]);
        let first = list.first_child(&app).expect("children were created");
        first
            .as_object()
            .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(&mut app)
            .set_keep_alive(true);

        harness.scroll_to(&mut app, 2000.0);

        assert_eq!(list.keep_alive_children(&app), vec![first]);
        assert!(first.as_object().attached(&app));
    }
}
