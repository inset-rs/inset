//! `TableViewCellsPanel` (`controls/dev/TableView/TableViewCellsPanel.cpp`): the panel that is both the header band's `PART_HeaderHost` and each row's `PART_CellsHost`, one cell per column in column order, each arranged at its column's resolved width, so the header and every row share each column's width by construction.
//!
//! An `Auto` column's cells are measured without a width limit and the widest is recorded in the [`ColumnMeasurements`] the panels of one table share, from which the table resolves the column's width; pixel and star cells are measured at their column's width. The leading frozen prefix is pinned against the horizontal scroll offset and the scrolled cells are clipped beneath it (`ApplyFrozenColumnLayout`), in paint and in hit testing alike. A later editor who moves a cell in paint without moving it in hit testing leaves frozen cells that draw in one place and take presses in another.

use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use waft_embedder::{Clip, Matrix4, Offset, Rect, Size};
use waft_foundation::{App, Handle, HandleId, Listenable, Listener};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, BoxParentData, ClipRectLayer,
    ContainerBoxParentData, ContainerLayer, ContainerParentData, ContainerParentDataMixin,
    ContainerRenderObjectData, ContainerRenderObjectMixin, PaintingContext, ParentData,
    PipelineOwner, RenderBox, RenderBoxContainerDefaultsMixin, RenderBoxData, RenderHandle,
    RenderObject, RenderObjectData,
};
use waft_widgets::*;

/// `c_columnMeasureChangeThreshold`: a cell's measured width must move by more than this to ask for the widths to be resolved again.
const MEASURE_CHANGE_THRESHOLD: f64 = 0.5;

/// What a panel knows of one column: `TableViewColumn`'s `Width` unit, `ActualWidth` and `Visibility`, as the table last resolved them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CellsPanelColumn {
    pub(crate) id: HandleId,
    pub(crate) auto: bool,
    pub(crate) width: f64,
    pub(crate) visible: bool,
}

/// Where a panel sits against the body's horizontal scrolling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CellsPanelPlacement {
    /// A row's cells, inside the scrolling body: the frozen prefix counter-moves by the offset.
    Row,
    /// The header band, outside it: the scrolled cells follow the offset and the frozen prefix stays.
    Header,
}

/// The `Auto` columns' measured widths, per panel, that one table's panels record and the table resolves from (`MeasuredWidthForColumn`, pulled across the header and the realized rows).
pub(crate) struct ColumnMeasurements {
    panels: RefCell<HashMap<u64, HashMap<HandleId, f64>>>,
    next_panel: Cell<u64>,
    /// `RequestColumnWidthResolve`: told when a panel's measurements changed.
    on_change: Rc<dyn Fn(&mut App)>,
}

impl ColumnMeasurements {
    pub(crate) fn new(on_change: Rc<dyn Fn(&mut App)>) -> Rc<Self> {
        Rc::new(Self {
            panels: RefCell::default(),
            next_panel: Cell::new(0),
            on_change,
        })
    }

    /// The widest cell any panel measured for `column`, or 0.
    pub(crate) fn measured(&self, column: HandleId) -> f64 {
        self.panels
            .borrow()
            .values()
            .filter_map(|panel| panel.get(&column))
            .fold(0.0, |widest: f64, width| widest.max(*width))
    }

    fn panel_key(&self) -> u64 {
        let key = self.next_panel.get();
        self.next_panel.set(key + 1);
        key
    }

    /// `RecordAndDetectMeasuredWidthChange`: records a panel's pass, answering whether a width moved past the threshold or a column came or went.
    fn record(&self, panel: u64, widths: HashMap<HandleId, f64>) -> bool {
        let mut panels = self.panels.borrow_mut();
        let changed = match panels.get(&panel) {
            Some(previous) => {
                previous.len() != widths.len()
                    || widths.iter().any(|(column, width)| {
                        previous
                            .get(column)
                            .is_none_or(|old| (old - width).abs() > MEASURE_CHANGE_THRESHOLD)
                    })
            }
            None => !widths.is_empty(),
        };
        panels.insert(panel, widths);
        changed
    }

    /// A panel left the tree, as a recycled row's does (`OnRowElementClearing`).
    fn forget(&self, panel: u64) -> bool {
        self.panels
            .borrow_mut()
            .remove(&panel)
            .is_some_and(|widths| !widths.is_empty())
    }
}

impl fmt::Debug for ColumnMeasurements {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ColumnMeasurements")
            .field("panels", &self.panels.borrow().len())
            .finish_non_exhaustive()
    }
}

/// XAML `TableViewCellsPanel`: one cell per entry of `columns`, in order.
#[derive(Debug)]
pub(crate) struct TableViewCellsPanel {
    pub(crate) columns: Vec<CellsPanelColumn>,
    /// How many columns, from the first, are in the leading frozen prefix.
    pub(crate) frozen: usize,
    pub(crate) placement: CellsPanelPlacement,
    /// The body's horizontal scrolling, whose offset pins the frozen prefix.
    pub(crate) horizontal: ScrollControllerRef,
    pub(crate) measurements: Rc<ColumnMeasurements>,
    pub(crate) children: Vec<WidgetRef>,
}

impl RenderObjectWidget for TableViewCellsPanel {
    type RenderObject = RenderTableViewCellsPanel;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        let panel_key = self.measurements.panel_key();
        let render = RenderHandle::new_box(
            app,
            RenderTableViewCellsPanel {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                columns: self.columns.clone(),
                frozen: self.frozen,
                placement: self.placement,
                horizontal: self.horizontal.clone(),
                measurements: self.measurements.clone(),
                panel_key,
                scroll_listener: None,
            },
        );
        render.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render: RenderHandle<RenderTableViewCellsPanel>,
    ) {
        render.set_layout(app, self.columns.clone(), self.frozen, self.placement);
        if render.get(app).horizontal.handle() != self.horizontal.handle() {
            let attached = render.get(app).scroll_listener.is_some();
            render.stop_listening(app);
            render.get_mut(app).horizontal = self.horizontal.clone();
            if attached {
                render.listen(app);
            }
            render.mark_needs_paint(app);
        }
    }
}

impl MultiChildRenderObjectWidget for TableViewCellsPanel {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

#[derive(Debug, Default)]
pub(crate) struct CellsPanelParentData {
    box_parent_data: BoxParentData,
    container_parent_data: ContainerParentData<AnyRenderBox>,
}

impl ParentData for CellsPanelParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<CellsPanelParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<CellsPanelParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide_mut(id)
    }
}

impl ContainerParentDataMixin for CellsPanelParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container_parent_data
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container_parent_data
    }
}

impl ContainerBoxParentData for CellsPanelParentData {
    fn box_parent_data(&self) -> &BoxParentData {
        &self.box_parent_data
    }

    fn box_parent_data_mut(&mut self) -> &mut BoxParentData {
        &mut self.box_parent_data
    }
}

impl fmt::Display for CellsPanelParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.box_parent_data)
    }
}

/// The render object behind [`TableViewCellsPanel`].
pub(crate) struct RenderTableViewCellsPanel {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    columns: Vec<CellsPanelColumn>,
    frozen: usize,
    placement: CellsPanelPlacement,
    horizontal: ScrollControllerRef,
    measurements: Rc<ColumnMeasurements>,
    panel_key: u64,
    scroll_listener: Option<Listener>,
}

impl RenderTableViewCellsPanel {
    fn set_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        columns: Vec<CellsPanelColumn>,
        frozen: usize,
        placement: CellsPanelPlacement,
    ) {
        let panel = self.get_mut(app);
        if panel.columns == columns && panel.frozen == frozen && panel.placement == placement {
            return;
        }
        panel.columns = columns;
        panel.frozen = frozen;
        panel.placement = placement;
        self.mark_needs_layout(app);
    }

    fn listen(self: RenderHandle<Self>, app: &mut App) {
        let listener = Listener::new(move |app| {
            if app.contains(self.handle()) {
                self.mark_needs_paint(app);
            }
        });
        let horizontal = self.get(app).horizontal.clone();
        horizontal.add_listener(app, listener.clone());
        self.get_mut(app).scroll_listener = Some(listener);
    }

    fn stop_listening(self: RenderHandle<Self>, app: &mut App) {
        if let Some(listener) = self.get_mut(app).scroll_listener.take() {
            let horizontal = self.get(app).horizontal.clone();
            horizontal.remove_listener(app, &listener);
        }
    }

    fn children(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox> {
        let mut children = Vec::new();
        let mut child = self.first_child(app);
        while let Some(current) = child {
            children.push(current);
            child = self.child_after(app, current);
        }
        children
    }

    /// The column a child is the cell of: children are in column order.
    fn column(self: RenderHandle<Self>, app: &App, index: usize) -> Option<CellsPanelColumn> {
        self.get(app).columns.get(index).copied()
    }

    /// The width a cell is arranged at: its column's resolved width, nothing for a collapsed column.
    fn cell_width(column: Option<CellsPanelColumn>, child: AnyRenderBox, app: &App) -> f64 {
        match column {
            Some(column) if column.visible => column.width.max(0.0),
            Some(_) => 0.0,
            None => child.size(app).width(),
        }
    }

    fn horizontal_offset(self: RenderHandle<Self>, app: &App) -> f64 {
        let horizontal = self.get(app).horizontal.clone();
        if horizontal.has_clients(app) {
            horizontal.offset(app)
        } else {
            0.0
        }
    }

    /// `ComputeLeadingFrozenWidth`.
    fn frozen_width(self: RenderHandle<Self>, app: &App) -> f64 {
        let panel = self.get(app);
        panel.columns[..panel.frozen.min(panel.columns.len())]
            .iter()
            .filter(|column| column.visible)
            .map(|column| column.width.max(0.0))
            .sum()
    }

    /// How far the scrolled cells move from their arranged place, and how far the frozen ones move beyond that: `ApplyFrozenColumnLayout`'s translation, for the panel's placement.
    fn shifts(self: RenderHandle<Self>, app: &App) -> (f64, f64) {
        let offset = self.horizontal_offset(app);
        match self.get(app).placement {
            CellsPanelPlacement::Row => (0.0, offset),
            CellsPanelPlacement::Header => (-offset, offset),
        }
    }

    /// How far the child at `index` moves from its arranged place when painted.
    fn child_shift(self: RenderHandle<Self>, app: &App, index: usize) -> f64 {
        let (scrolled_shift, frozen_shift) = self.shifts(app);
        if index < self.get(app).frozen {
            scrolled_shift + frozen_shift
        } else {
            scrolled_shift
        }
    }

    /// `MeasureOverride`: each cell's height at its column's width, an `Auto` cell's natural width recorded.
    fn measure(self: RenderHandle<Self>, app: &mut App, max_height: f64) -> (f64, f64) {
        let children = self.children(app);
        let mut measured = HashMap::new();
        let mut width = 0.0;
        let mut height: f64 = 0.0;
        for (index, child) in children.iter().copied().enumerate() {
            let Some(column) = self.column(app, index) else {
                child.layout(
                    app,
                    BoxConstraints::loose(Size::new(f64::INFINITY, max_height)),
                    true,
                );
                height = height.max(child.size(app).height());
                width += child.size(app).width();
                continue;
            };
            if !column.visible {
                child.layout(app, BoxConstraints::loose(Size::new(0.0, max_height)), true);
                continue;
            }
            let column_width = column.width.max(0.0);
            if column.auto {
                child.layout(
                    app,
                    BoxConstraints::loose(Size::new(f64::INFINITY, max_height)),
                    true,
                );
                let desired = child.size(app);
                measured.insert(column.id, desired.width().max(0.0));
                height = height.max(desired.height());
                if column_width > 0.0 && column_width < desired.width() {
                    child.layout(
                        app,
                        BoxConstraints::loose(Size::new(column_width, max_height)),
                        true,
                    );
                    height = height.max(child.size(app).height());
                }
            } else {
                let limit = if column_width > 0.0 {
                    column_width
                } else {
                    f64::INFINITY
                };
                child.layout(
                    app,
                    BoxConstraints::loose(Size::new(limit, max_height)),
                    true,
                );
                height = height.max(child.size(app).height());
            }
            width += column_width;
        }
        let measurements = self.get(app).measurements.clone();
        if measurements.record(self.get(app).panel_key, measured) {
            (measurements.on_change)(app);
        }
        (width, height)
    }

    /// Paints the cells `frozen` selects, each moved by `shift`.
    fn paint_cells(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        frozen: bool,
        shift: f64,
    ) {
        let frozen_count = self.get(app).frozen;
        for (index, child) in self.children(app).into_iter().enumerate() {
            if (index < frozen_count) != frozen {
                continue;
            }
            let child_offset = child
                .as_object()
                .parent_data_of::<CellsPanelParentData>(app)
                .offset();
            context.paint_child(
                app,
                child.as_object(),
                offset + child_offset + Offset::new(shift, 0.0),
            );
        }
    }
}

impl ContainerRenderObjectMixin for RenderTableViewCellsPanel {
    type ChildType = AnyRenderBox;
    type ParentDataType = CellsPanelParentData;

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

impl RenderBoxContainerDefaultsMixin for RenderTableViewCellsPanel {}

impl RenderObject for RenderTableViewCellsPanel {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let (content_width, measured_height) = self.measure(app, constraints.max_height);
        let height = measured_height.max(constraints.min_height);
        // `ArrangeOverride`: every cell at its column's width and the panel's height.
        let mut x = 0.0;
        for (index, child) in self.children(app).into_iter().enumerate() {
            let column = self.column(app, index);
            let width = Self::cell_width(column, child, app);
            child.layout(app, BoxConstraints::tight(Size::new(width, height)), false);
            child
                .parent_data_of_mut::<CellsPanelParentData>(app)
                .set_offset(Offset::new(x, 0.0));
            x += width;
        }
        let width = match self.get(app).placement {
            CellsPanelPlacement::Row => content_width,
            // The header band is as wide as the body's viewport and scrolls its cells inside it.
            CellsPanelPlacement::Header if constraints.max_width.is_finite() => {
                constraints.max_width
            }
            CellsPanelPlacement::Header => content_width,
        };
        self.set_size(app, constraints.constrain(Size::new(width, height)));
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner);
        self.listen(app);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app);
        self.stop_listening(app);
        let measurements = self.get(app).measurements.clone();
        if measurements.forget(self.get(app).panel_key) {
            (measurements.on_change)(app);
        }
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
        let (scrolled_shift, frozen_shift) = self.shifts(app);
        let frozen_width = self.frozen_width(app);
        let size = self.size(app);
        let header = self.get(app).placement == CellsPanelPlacement::Header;
        if frozen_width > 0.0 || header {
            // The scrolled cells are hidden where the frozen band sits, and a header's beyond its bounds.
            let clip_left = if frozen_width > 0.0 {
                frozen_width + scrolled_shift + frozen_shift
            } else {
                0.0
            };
            let clip = Rect::from_ltrb(clip_left, 0.0, size.width().max(clip_left), size.height());
            let old = self.as_object().layer_as::<ClipRectLayer>(app);
            let layer = context.push_clip_rect(
                app,
                self.as_object().needs_compositing(app),
                offset,
                clip,
                |app, context, offset| {
                    self.paint_cells(app, context, offset, false, scrolled_shift)
                },
                Clip::HardEdge,
                old,
            );
            self.as_object()
                .set_layer(app, layer.map(|layer| layer.as_container_layer()));
        } else {
            self.paint_cells(app, context, offset, false, scrolled_shift);
            self.as_object().set_layer(app, None);
        }
        self.paint_cells(app, context, offset, true, scrolled_shift + frozen_shift);
    }
}

impl RenderBox for RenderTableViewCellsPanel {
    waft_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<CellsPanelParentData>(app) {
            child.set_parent_data(app, CellsPanelParentData::default());
        }
    }

    /// The cell's arranged place and its pin, as it is painted.
    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        let index = self
            .children(app)
            .iter()
            .position(|candidate| candidate.as_object() == child)
            .unwrap_or(usize::MAX);
        let offset = child.parent_data_of::<CellsPanelParentData>(app).offset()
            + Offset::new(self.child_shift(app, index), 0.0);
        *transform = transform.then(&Matrix4::translation(
            offset.dx() as f32,
            offset.dy() as f32,
        ));
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        _app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        constraints.smallest()
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let (scrolled_shift, frozen_shift) = self.shifts(app);
        let frozen_count = self.get(app).frozen;
        let clip_left = {
            let frozen_width = self.frozen_width(app);
            if frozen_width > 0.0 {
                frozen_width + scrolled_shift + frozen_shift
            } else {
                f64::NEG_INFINITY
            }
        };
        let header = self.get(app).placement == CellsPanelPlacement::Header;
        if header && (position.dx() < 0.0 || position.dx() > self.size(app).width()) {
            return false;
        }
        let children = self.children(app);
        // The frozen cells are painted above the scrolled ones, so they are hit first.
        let order = children
            .iter()
            .enumerate()
            .filter(|(index, _)| *index < frozen_count)
            .chain(
                children
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index >= frozen_count),
            );
        for (index, child) in order {
            let frozen = index < frozen_count;
            if !frozen && position.dx() < clip_left {
                continue;
            }
            let shift = if frozen {
                scrolled_shift + frozen_shift
            } else {
                scrolled_shift
            };
            let child_offset = child
                .as_object()
                .parent_data_of::<CellsPanelParentData>(app)
                .offset()
                + Offset::new(shift, 0.0);
            let hit = result.add_with_paint_offset(
                Some(child_offset),
                position,
                |result, transformed| child.hit_test(app, result, transformed),
            );
            if hit {
                return true;
            }
        }
        false
    }
}
