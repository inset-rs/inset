//! `CGrid` as a render object: the children's placements are parent data, `MeasureOverride` runs against dry layouts, `ArrangeOverride` lays each child out in its cell.

use super::definition::{ColumnDefinition, RowDefinition};
use super::layout::{CellPlacement, GridLayout};
use inset_embedder::{Offset, Size};
use inset_foundation::{App, Handle};
use inset_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, BoxParentData,
    ContainerBoxParentData, ContainerParentData, ContainerParentDataMixin,
    ContainerRenderObjectData, ContainerRenderObjectMixin, PaintingContext, ParentData,
    PipelineOwner, RenderBox, RenderBoxContainerDefaultsMixin, RenderBoxData, RenderHandle,
    RenderObject, RenderObjectData,
};
use std::any::{Any, TypeId};
use std::fmt;

/// The attached `Grid.Row`, `Grid.Column`, `Grid.RowSpan` and `Grid.ColumnSpan` of a child, with its offset in the grid.
#[derive(Debug)]
pub struct GridParentData {
    box_parent_data: BoxParentData,
    container_parent_data: ContainerParentData<AnyRenderBox>,
    pub placement: CellPlacement,
}

impl GridParentData {
    pub fn new() -> GridParentData {
        GridParentData {
            box_parent_data: BoxParentData::new(),
            container_parent_data: ContainerParentData::new(),
            placement: CellPlacement::default(),
        }
    }
}

impl Default for GridParentData {
    fn default() -> GridParentData {
        GridParentData::new()
    }
}

impl ParentData for GridParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<GridParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<GridParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide_mut(id)
    }
}

impl ContainerParentDataMixin for GridParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container_parent_data
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container_parent_data
    }
}

impl ContainerBoxParentData for GridParentData {
    fn box_parent_data(&self) -> &BoxParentData {
        &self.box_parent_data
    }

    fn box_parent_data_mut(&mut self) -> &mut BoxParentData {
        &mut self.box_parent_data
    }
}

impl fmt::Display for GridParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let placement = self.placement;
        write!(
            f,
            "row={} column={} row_span={} column_span={}; {}",
            placement.row,
            placement.column,
            placement.row_span,
            placement.column_span,
            self.box_parent_data
        )
    }
}

/// The render object behind `Grid`.
pub struct RenderGrid {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    layout: GridLayout,
}

impl RenderGrid {
    pub fn new(app: &mut App) -> RenderHandle<RenderGrid> {
        RenderHandle::new_box(
            app,
            RenderGrid {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                layout: GridLayout::new(),
            },
        )
    }

    /// XAML `RowDefinitions` and `ColumnDefinitions`; a change invalidates the definitions (`InvalidateDefinitions`).
    pub fn set_definitions(
        self: RenderHandle<Self>,
        app: &mut App,
        rows: Vec<RowDefinition>,
        columns: Vec<ColumnDefinition>,
    ) {
        let layout = &mut self.get_mut(app).layout;
        if layout.row_definitions == rows && layout.column_definitions == columns {
            return;
        }
        layout.row_definitions = rows;
        layout.column_definitions = columns;
        self.mark_needs_layout(app);
    }

    /// XAML `RowSpacing`.
    pub fn set_row_spacing(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).layout.row_spacing != value {
            self.get_mut(app).layout.row_spacing = value;
            self.mark_needs_layout(app);
        }
    }

    /// XAML `ColumnSpacing`.
    pub fn set_column_spacing(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).layout.column_spacing != value {
            self.get_mut(app).layout.column_spacing = value;
            self.mark_needs_layout(app);
        }
    }

    /// The children in order with their placements: `GetUnsortedChildren` and the layout properties.
    fn cells(self: RenderHandle<Self>, app: &App) -> (Vec<AnyRenderBox>, Vec<CellPlacement>) {
        let mut children = Vec::new();
        let mut placements = Vec::new();
        let mut child = self.first_child(app);
        while let Some(current) = child {
            children.push(current);
            placements.push(
                current
                    .as_object()
                    .parent_data_of::<GridParentData>(app)
                    .placement,
            );
            child = self.child_after(app, current);
        }
        (children, placements)
    }

    /// `MeasureOverride` on a copy of the layout state, measuring the children with `measure_child`.
    fn measure_with(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        mut measure_child: impl FnMut(&mut App, AnyRenderBox, Size) -> Size,
    ) -> Size {
        let (children, placements) = self.cells(app);
        let mut layout = self.get(app).layout.clone();
        let desired = layout.measure(&placements, constraints.biggest(), |index, available| {
            measure_child(app, children[index], available)
        });
        constraints.constrain(desired)
    }

    /// The measure of a child in XAML terms: its size when offered `available`, unbounded where infinite.
    fn dry_layout_child(app: &mut App, child: AnyRenderBox, available: Size) -> Size {
        child.get_dry_layout(app, BoxConstraints::loose(available))
    }
}

impl ContainerRenderObjectMixin for RenderGrid {
    type ChildType = AnyRenderBox;
    type ParentDataType = GridParentData;

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

impl RenderBoxContainerDefaultsMixin for RenderGrid {}

impl RenderObject for RenderGrid {
    inset_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let (children, placements) = self.cells(app);
        let mut layout = std::mem::take(&mut self.get_mut(app).layout);

        // Measure: every child's desired size against what its cell offers.
        let mut offered = vec![Size::ZERO; children.len()];
        let desired = layout.measure(&placements, constraints.biggest(), |index, available| {
            let child = children[index];
            offered[index] = available;
            child.layout(app, BoxConstraints::loose(available), true);
            child.size(app)
        });
        let size = constraints.constrain(desired);

        // Arrange: every child fills its cell (XAML's `Stretch`), offered at least the cell and
        // at most what its measure offered rather than exactly the cell. A child last given an
        // exact size stops telling its parent when it needs layout, so the measure would not
        // run again when a child an Auto track follows changes its desired size.
        let rects = layout.arrange(&placements, size);
        for ((child, rect), offered) in children.iter().zip(rects).zip(offered) {
            let cell = rect.size();
            let constraints = BoxConstraints::new()
                .min_width(cell.width())
                .max_width(cell.width().max(offered.width()))
                .min_height(cell.height())
                .max_height(cell.height().max(offered.height()));
            child.layout(app, constraints, true);
            child
                .parent_data_of_mut::<GridParentData>(app)
                .set_offset(rect.top_left());
        }

        self.get_mut(app).layout = layout;
        self.set_size(app, size);
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
        self.default_paint(app, context, offset)
    }
}

impl RenderBox for RenderGrid {
    inset_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<GridParentData>(app) {
            child.set_parent_data(app, GridParentData::new());
        }
    }

    // XAML has one measure; the four intrinsics run it with the children's intrinsic sizes for
    // the space each cell offers.
    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let constraints = BoxConstraints::loose(Size::new(f64::INFINITY, height));
        self.measure_with(app, constraints, |app, child, available| {
            let width = child.get_min_intrinsic_width(app, available.height());
            Size::new(width, child.get_min_intrinsic_height(app, width))
        })
        .width()
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let constraints = BoxConstraints::loose(Size::new(f64::INFINITY, height));
        self.measure_with(app, constraints, |app, child, available| {
            let width = child.get_max_intrinsic_width(app, available.height());
            Size::new(width, child.get_max_intrinsic_height(app, width))
        })
        .width()
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let constraints = BoxConstraints::loose(Size::new(width, f64::INFINITY));
        self.measure_with(app, constraints, |app, child, available| {
            let height = child.get_min_intrinsic_height(app, available.width());
            Size::new(child.get_min_intrinsic_width(app, height), height)
        })
        .height()
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let constraints = BoxConstraints::loose(Size::new(width, f64::INFINITY));
        self.measure_with(app, constraints, |app, child, available| {
            let height = child.get_max_intrinsic_height(app, available.width());
            Size::new(child.get_max_intrinsic_width(app, height), height)
        })
        .height()
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.measure_with(app, constraints, RenderGrid::dry_layout_child)
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
