//! Flutter counterpart: `rendering/list_body.dart`.

use std::any::{Any, TypeId};
use std::fmt;

use waft_embedder::{Offset, Size, TextBaseline};
use waft_foundation::{App, Handle};
use waft_painting::{Axis, AxisDirection, axis_direction_to_axis};

use crate::box_::{
    AnyRenderBox, BaselineOffset, BoxConstraints, BoxHitTestResult, BoxParentData,
    ContainerBoxParentData, RenderBox, RenderBoxContainerDefaultsMixin, RenderBoxData,
};
use crate::object::{
    AnyRenderObject, ContainerParentData, ContainerParentDataMixin, ContainerRenderObjectData,
    ContainerRenderObjectMixin, ParentData, RenderHandle, RenderObject, RenderObjectData,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;

/// Parent data for use with [`RenderListBody`].
#[derive(Debug, Default)]
pub struct ListBodyParentData {
    box_parent_data: BoxParentData,
    container_parent_data: ContainerParentData<AnyRenderBox>,
}

impl ListBodyParentData {
    /// Creates list body parent data at offset zero, with no siblings.
    pub const fn new() -> ListBodyParentData {
        ListBodyParentData {
            box_parent_data: BoxParentData::new(),
            container_parent_data: ContainerParentData::new(),
        }
    }
}

impl ParentData for ListBodyParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<ListBodyParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<ListBodyParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide_mut(id)
    }
}

impl ContainerParentDataMixin for ListBodyParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container_parent_data
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container_parent_data
    }
}

impl ContainerBoxParentData for ListBodyParentData {
    fn box_parent_data(&self) -> &BoxParentData {
        &self.box_parent_data
    }

    fn box_parent_data_mut(&mut self) -> &mut BoxParentData {
        &mut self.box_parent_data
    }
}

impl fmt::Display for ListBodyParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.box_parent_data, f)
    }
}

/// Displays its children sequentially along a given axis, forcing them to the
/// dimensions of the parent in the other axis.
///
/// This layout algorithm arranges its children linearly along the main axis
/// (either horizontally or vertically). In the cross axis, children are
/// stretched to match the box's cross-axis extent. In the main axis, children
/// are given unlimited space and the box expands its main axis to contain all
/// its children. Because [`RenderListBody`] boxes expand in the main axis, they
/// must be given unlimited space in the main axis, typically by being contained
/// in a viewport with a scrolling direction that matches the box's main axis.
pub struct RenderListBody {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    axis_direction: AxisDirection,
}

impl RenderListBody {
    /// Creates a render object that arranges its children sequentially along a
    /// given axis.
    ///
    /// By default, children are arranged along the vertical axis. Add children with
    /// [`add`](ContainerRenderObjectMixin::add) or [`add_all`](ContainerRenderObjectMixin::add_all);
    /// Dart's constructor takes them.
    pub fn new(app: &mut App, axis_direction: AxisDirection) -> RenderHandle<RenderListBody> {
        RenderHandle::new_box(
            app,
            RenderListBody {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                axis_direction,
            },
        )
    }

    /// The direction in which the children are laid out.
    ///
    /// For example, if the [`axis_direction`](Self::axis_direction) is [`AxisDirection::Down`],
    /// each child will be laid out below the next, vertically.
    pub fn axis_direction(self: RenderHandle<Self>, app: &App) -> AxisDirection {
        self.get(app).axis_direction
    }

    /// Sets [`axis_direction`](Self::axis_direction).
    pub fn set_axis_direction(self: RenderHandle<Self>, app: &mut App, value: AxisDirection) {
        if self.get(app).axis_direction == value {
            return;
        }
        self.get_mut(app).axis_direction = value;
        self.mark_needs_layout(app);
    }

    /// The axis (horizontal or vertical) corresponding to the current
    /// [`axis_direction`](Self::axis_direction).
    pub fn main_axis(self: RenderHandle<Self>, app: &App) -> Axis {
        axis_direction_to_axis(self.axis_direction(app))
    }

    fn debug_check_constraints(
        self: RenderHandle<Self>,
        app: &App,
        constraints: BoxConstraints,
    ) -> bool {
        if cfg!(debug_assertions) {
            let unbounded_main_axis = match self.main_axis(app) {
                Axis::Horizontal => !constraints.has_bounded_width(),
                Axis::Vertical => !constraints.has_bounded_height(),
            };
            assert!(
                unbounded_main_axis,
                "RenderListBody must have unlimited space along its main axis.\n\
                 RenderListBody does not clip or resize its children, so it must be placed in a \
                 parent that does not constrain the main axis.\n\
                 You probably want to put the RenderListBody inside a RenderViewport with a \
                 matching main axis."
            );
            let bounded_cross_axis = match self.main_axis(app) {
                Axis::Horizontal => constraints.has_bounded_height(),
                Axis::Vertical => constraints.has_bounded_width(),
            };
            assert!(
                bounded_cross_axis,
                "RenderListBody must have a bounded constraint for its cross axis.\n\
                 RenderListBody forces its children to expand to fit the RenderListBody's \
                 container, so it must be placed in a parent that constrains the cross axis to a \
                 finite dimension.\n\
                 If you are attempting to nest a RenderListBody with one direction inside one of \
                 another direction, you will want to wrap the inner one inside a box that fixes \
                 the dimension in that direction, for example, a RenderIntrinsicWidth or \
                 RenderIntrinsicHeight object. This is relatively expensive, however."
            );
        }
        true
    }

    fn get_intrinsic_cross_axis(
        self: RenderHandle<Self>,
        app: &mut App,
        child_size: impl Fn(&mut App, AnyRenderBox) -> f64,
    ) -> f64 {
        let mut extent: f64 = 0.0;
        let mut child = self.first_child(app);
        while let Some(current) = child {
            extent = extent.max(child_size(app, current));
            child = self.child_after(app, current);
        }
        extent
    }

    fn get_intrinsic_main_axis(
        self: RenderHandle<Self>,
        app: &mut App,
        child_size: impl Fn(&mut App, AnyRenderBox) -> f64,
    ) -> f64 {
        let mut extent = 0.0;
        let mut child = self.first_child(app);
        while let Some(current) = child {
            extent += child_size(app, current);
            child = self.child_after(app, current);
        }
        extent
    }
}

impl ContainerRenderObjectMixin for RenderListBody {
    type ChildType = AnyRenderBox;
    type ParentDataType = ListBodyParentData;

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
}

impl RenderBoxContainerDefaultsMixin for RenderListBody {}

impl RenderObject for RenderListBody {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        debug_assert!(self.debug_check_constraints(app, constraints));
        let mut main_axis_extent = 0.0;
        let mut child = self.first_child(app);
        match self.axis_direction(app) {
            AxisDirection::Right => {
                let inner_constraints =
                    BoxConstraints::tight_for(None, Some(constraints.max_height));
                while let Some(current) = child {
                    current.layout(app, inner_constraints, true);
                    let child_parent_data = current
                        .as_object()
                        .parent_data_of_mut::<ListBodyParentData>(app);
                    child_parent_data.set_offset(Offset::new(main_axis_extent, 0.0));
                    let next_sibling = child_parent_data.next_sibling();
                    main_axis_extent += current.size(app).width();
                    child = next_sibling;
                }
                self.set_size(
                    app,
                    constraints.constrain(Size::new(main_axis_extent, constraints.max_height)),
                );
            }
            AxisDirection::Left => {
                let inner_constraints =
                    BoxConstraints::tight_for(None, Some(constraints.max_height));
                while let Some(current) = child {
                    current.layout(app, inner_constraints, true);
                    main_axis_extent += current.size(app).width();
                    child = self.child_after(app, current);
                }
                let mut position = 0.0;
                child = self.first_child(app);
                while let Some(current) = child {
                    position += current.size(app).width();
                    let child_parent_data = current
                        .as_object()
                        .parent_data_of_mut::<ListBodyParentData>(app);
                    child_parent_data.set_offset(Offset::new(main_axis_extent - position, 0.0));
                    child = child_parent_data.next_sibling();
                }
                self.set_size(
                    app,
                    constraints.constrain(Size::new(main_axis_extent, constraints.max_height)),
                );
            }
            AxisDirection::Down => {
                let inner_constraints =
                    BoxConstraints::tight_for(Some(constraints.max_width), None);
                while let Some(current) = child {
                    current.layout(app, inner_constraints, true);
                    let child_parent_data = current
                        .as_object()
                        .parent_data_of_mut::<ListBodyParentData>(app);
                    child_parent_data.set_offset(Offset::new(0.0, main_axis_extent));
                    let next_sibling = child_parent_data.next_sibling();
                    main_axis_extent += current.size(app).height();
                    child = next_sibling;
                }
                self.set_size(
                    app,
                    constraints.constrain(Size::new(constraints.max_width, main_axis_extent)),
                );
            }
            AxisDirection::Up => {
                let inner_constraints =
                    BoxConstraints::tight_for(Some(constraints.max_width), None);
                while let Some(current) = child {
                    current.layout(app, inner_constraints, true);
                    main_axis_extent += current.size(app).height();
                    child = self.child_after(app, current);
                }
                let mut position = 0.0;
                child = self.first_child(app);
                while let Some(current) = child {
                    position += current.size(app).height();
                    let child_parent_data = current
                        .as_object()
                        .parent_data_of_mut::<ListBodyParentData>(app);
                    child_parent_data.set_offset(Offset::new(0.0, main_axis_extent - position));
                    child = child_parent_data.next_sibling();
                }
                self.set_size(
                    app,
                    constraints.constrain(Size::new(constraints.max_width, main_axis_extent)),
                );
            }
        }
        debug_assert!(self.size(app).is_finite());
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner)
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app)
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app)
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        self.default_paint(app, context, offset);
    }
}

impl RenderBox for RenderListBody {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<ListBodyParentData>(app) {
            child.set_parent_data(app, ListBodyParentData::new());
        }
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        debug_assert!(self.debug_check_constraints(app, constraints));
        let mut child;
        let next_child: fn(RenderHandle<Self>, &App, AnyRenderBox) -> Option<AnyRenderBox>;
        match self.axis_direction(app) {
            AxisDirection::Right | AxisDirection::Left => {
                let child_constraints =
                    BoxConstraints::tight_for(None, Some(constraints.max_height));
                let mut baseline_offset = BaselineOffset::NO_BASELINE;
                child = self.first_child(app);
                while let Some(current) = child {
                    baseline_offset = baseline_offset.min_of(BaselineOffset(
                        current.get_dry_baseline(app, child_constraints, baseline),
                    ));
                    child = self.child_after(app, current);
                }
                return baseline_offset.offset();
            }
            AxisDirection::Up => {
                child = self.last_child(app);
                next_child = |this, app, child| this.child_before(app, child);
            }
            AxisDirection::Down => {
                child = self.first_child(app);
                next_child = |this, app, child| this.child_after(app, child);
            }
        }
        let child_constraints = BoxConstraints::tight_for(Some(constraints.max_width), None);
        let mut main_axis_extent = 0.0;
        while let Some(current) = child {
            let child_baseline = current.get_dry_baseline(app, child_constraints, baseline);
            if let Some(child_baseline) = child_baseline {
                return Some(child_baseline + main_axis_extent);
            }
            main_axis_extent += current.get_dry_layout(app, child_constraints).height();
            child = next_child(self, app, current);
        }
        None
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        debug_assert!(self.debug_check_constraints(app, constraints));
        let mut main_axis_extent = 0.0;
        let mut child = self.first_child(app);
        match self.axis_direction(app) {
            AxisDirection::Right | AxisDirection::Left => {
                let inner_constraints =
                    BoxConstraints::tight_for(None, Some(constraints.max_height));
                while let Some(current) = child {
                    let child_size = current.get_dry_layout(app, inner_constraints);
                    main_axis_extent += child_size.width();
                    child = self.child_after(app, current);
                }
                constraints.constrain(Size::new(main_axis_extent, constraints.max_height))
            }
            AxisDirection::Up | AxisDirection::Down => {
                let inner_constraints =
                    BoxConstraints::tight_for(Some(constraints.max_width), None);
                while let Some(current) = child {
                    let child_size = current.get_dry_layout(app, inner_constraints);
                    main_axis_extent += child_size.height();
                    child = self.child_after(app, current);
                }
                constraints.constrain(Size::new(constraints.max_width, main_axis_extent))
            }
        }
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let child_size =
            |app: &mut App, child: AnyRenderBox| child.get_min_intrinsic_width(app, height);
        match self.main_axis(app) {
            Axis::Horizontal => self.get_intrinsic_main_axis(app, child_size),
            Axis::Vertical => self.get_intrinsic_cross_axis(app, child_size),
        }
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let child_size =
            |app: &mut App, child: AnyRenderBox| child.get_max_intrinsic_width(app, height);
        match self.main_axis(app) {
            Axis::Horizontal => self.get_intrinsic_main_axis(app, child_size),
            Axis::Vertical => self.get_intrinsic_cross_axis(app, child_size),
        }
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let child_size =
            |app: &mut App, child: AnyRenderBox| child.get_min_intrinsic_height(app, width);
        match self.main_axis(app) {
            Axis::Horizontal => self.get_intrinsic_main_axis(app, child_size),
            Axis::Vertical => self.get_intrinsic_cross_axis(app, child_size),
        }
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let child_size =
            |app: &mut App, child: AnyRenderBox| child.get_max_intrinsic_height(app, width);
        match self.main_axis(app) {
            Axis::Horizontal => self.get_intrinsic_main_axis(app, child_size),
            Axis::Vertical => self.get_intrinsic_cross_axis(app, child_size),
        }
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        self.default_compute_distance_to_first_actual_baseline(app, baseline)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        self.default_hit_test_children(app, result, position)
    }
}
