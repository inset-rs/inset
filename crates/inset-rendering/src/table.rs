//! Flutter counterpart: `rendering/table.dart`.
//!
//! The table's semantics (`describeSemanticsConfiguration`, `assembleSemanticsNode`) wait with
//! the rest of accessibility.

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

use inset_embedder::{Offset, Rect, Size, TextBaseline, TextDirection};
use inset_foundation::{App, PRECISION_ERROR_TOLERANCE};
use inset_painting::{BoxPainter, ClipContext, Decoration, ImageConfiguration};

use crate::box_::{
    AnyRenderBox, BoxConstraints, BoxHitTestResult, BoxParentData, DryLayoutFailure, RenderBox,
    RenderBoxData,
};
use crate::object::{AnyRenderObject, ParentData, RenderHandle, RenderObject, RenderObjectData};
use crate::painting_context::PaintingContext;
use crate::table_border::TableBorder;

/// Parent data used by [`RenderTable`] for its children.
#[derive(Debug, Default)]
pub struct TableCellParentData {
    box_parent_data: BoxParentData,

    /// Where this cell should be placed vertically.
    ///
    /// When using [`TableCellVerticalAlignment::Baseline`], the text baseline must be set as
    /// well.
    pub vertical_alignment: Option<TableCellVerticalAlignment>,

    /// The column that the child was in the last time it was laid out.
    pub x: Option<usize>,

    /// The row that the child was in the last time it was laid out.
    pub y: Option<usize>,
}

impl TableCellParentData {
    /// Creates table cell parent data at offset zero, with the default vertical alignment.
    pub const fn new() -> TableCellParentData {
        TableCellParentData {
            box_parent_data: BoxParentData::new(),
            vertical_alignment: None,
            x: None,
            y: None,
        }
    }
}

impl ParentData for TableCellParentData {
    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<TableCellParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<TableCellParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide_mut(id)
    }
}

impl fmt::Display for TableCellParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.box_parent_data, f)?;
        match self.vertical_alignment {
            None => f.write_str("; default vertical alignment"),
            Some(vertical_alignment) => write!(f, "; {vertical_alignment:?}"),
        }
    }
}

/// A shared, immutable [`TableColumnWidth`] (a Dart object reference). Two references are the
/// same column width only when they point at the same value, as Dart compares these objects.
pub type TableColumnWidthRef = Rc<dyn TableColumnWidth>;

/// Base class to describe how wide a column in a [`RenderTable`] should be.
///
/// To size a column to a specific number of pixels, use a [`FixedColumnWidth`]. This is the
/// cheapest way to size a column.
///
/// Other algorithms that are relatively cheap include [`FlexColumnWidth`], which distributes
/// the space equally among the flexible columns, [`FractionColumnWidth`], which sizes a column
/// based on the size of the table's container.
pub trait TableColumnWidth: fmt::Debug {
    /// The smallest width that the column can have.
    ///
    /// The `cells` argument is an iterable that provides all the cells in the table for this
    /// column. Walking the cells is by definition O(N), so algorithms that do that should be
    /// considered expensive.
    ///
    /// The `container_width` argument is the `max_width` of the incoming constraints for the
    /// table, and might be infinite.
    fn min_intrinsic_width(
        &self,
        app: &mut App,
        cells: &[AnyRenderBox],
        container_width: f64,
    ) -> f64;

    /// The ideal width that the column should have. This must be equal to or greater than the
    /// [`min_intrinsic_width`](Self::min_intrinsic_width). The column might be bigger than this
    /// width, e.g. if the column is flexible or if the table's width ends up being forced to be
    /// bigger than the sum of all the `max_intrinsic_width` values.
    ///
    /// The `cells` argument is an iterable that provides all the cells in the table for this
    /// column. Walking the cells is by definition O(N), so algorithms that do that should be
    /// considered expensive.
    ///
    /// The `container_width` argument is the `max_width` of the incoming constraints for the
    /// table, and might be infinite.
    fn max_intrinsic_width(
        &self,
        app: &mut App,
        cells: &[AnyRenderBox],
        container_width: f64,
    ) -> f64;

    /// The flex factor to apply to the cell if there is any room left over when laying out the
    /// table. The remaining space is distributed to any columns with flex in proportion to
    /// their flex value (higher values get more space).
    ///
    /// The `cells` argument is an iterable that provides all the cells in the table for this
    /// column. Walking the cells is by definition O(N), so algorithms that do that should be
    /// considered expensive.
    fn flex(&self, app: &mut App, cells: &[AnyRenderBox]) -> Option<f64> {
        let _ = (app, cells);
        None
    }

    /// Downcast support for Dart's `is` checks on a column width.
    fn as_any(&self) -> &dyn Any;
}

/// Sizes the column according to the intrinsic dimensions of all the cells in that column.
///
/// This is a very expensive way to size a column.
///
/// A flex value can be provided. If specified (and non-`None`), the column will participate in
/// the distribution of remaining space once all the non-flexible columns have been sized.
pub struct IntrinsicColumnWidth {
    flex: Option<f64>,
}

impl IntrinsicColumnWidth {
    /// Creates a column width based on intrinsic sizing.
    ///
    /// This sizing algorithm is very expensive.
    ///
    /// The `flex` argument specifies the flex factor to apply to the column if there is any
    /// room left over when laying out the table. If `flex` is `None` (the default), the table
    /// will not distribute any extra space to the column.
    pub const fn new(flex: Option<f64>) -> IntrinsicColumnWidth {
        IntrinsicColumnWidth { flex }
    }
}

impl TableColumnWidth for IntrinsicColumnWidth {
    fn min_intrinsic_width(
        &self,
        app: &mut App,
        cells: &[AnyRenderBox],
        _container_width: f64,
    ) -> f64 {
        let mut result: f64 = 0.0;
        for cell in cells {
            result = result.max(cell.get_min_intrinsic_width(app, f64::INFINITY));
        }
        result
    }

    fn max_intrinsic_width(
        &self,
        app: &mut App,
        cells: &[AnyRenderBox],
        _container_width: f64,
    ) -> f64 {
        let mut result: f64 = 0.0;
        for cell in cells {
            result = result.max(cell.get_max_intrinsic_width(app, f64::INFINITY));
        }
        result
    }

    fn flex(&self, _app: &mut App, _cells: &[AnyRenderBox]) -> Option<f64> {
        self.flex
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl fmt::Debug for IntrinsicColumnWidth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.flex {
            Some(flex) => write!(f, "IntrinsicColumnWidth(flex: {flex:.1})"),
            None => f.write_str("IntrinsicColumnWidth(flex: null)"),
        }
    }
}

/// Sizes the column to a specific number of pixels.
///
/// This is the cheapest way to size a column.
pub struct FixedColumnWidth {
    /// The width the column should occupy in logical pixels.
    pub value: f64,
}

impl FixedColumnWidth {
    /// Creates a column width based on a fixed number of logical pixels.
    pub const fn new(value: f64) -> FixedColumnWidth {
        FixedColumnWidth { value }
    }
}

impl TableColumnWidth for FixedColumnWidth {
    fn min_intrinsic_width(
        &self,
        _app: &mut App,
        _cells: &[AnyRenderBox],
        _container_width: f64,
    ) -> f64 {
        self.value
    }

    fn max_intrinsic_width(
        &self,
        _app: &mut App,
        _cells: &[AnyRenderBox],
        _container_width: f64,
    ) -> f64 {
        self.value
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl fmt::Debug for FixedColumnWidth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FixedColumnWidth({:.1})", self.value)
    }
}

/// Sizes the column to a fraction of the table's constraints' `max_width`.
///
/// This is a cheap way to size a column.
pub struct FractionColumnWidth {
    /// The fraction of the table's constraints' `max_width` that this column should occupy.
    pub value: f64,
}

impl FractionColumnWidth {
    /// Creates a column width based on a fraction of the table's constraints' `max_width`.
    pub const fn new(value: f64) -> FractionColumnWidth {
        FractionColumnWidth { value }
    }
}

impl TableColumnWidth for FractionColumnWidth {
    fn min_intrinsic_width(
        &self,
        _app: &mut App,
        _cells: &[AnyRenderBox],
        container_width: f64,
    ) -> f64 {
        if !container_width.is_finite() {
            return 0.0;
        }
        self.value * container_width
    }

    fn max_intrinsic_width(
        &self,
        _app: &mut App,
        _cells: &[AnyRenderBox],
        container_width: f64,
    ) -> f64 {
        if !container_width.is_finite() {
            return 0.0;
        }
        self.value * container_width
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl fmt::Debug for FractionColumnWidth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FractionColumnWidth({})", self.value)
    }
}

/// Sizes the column by taking a part of the remaining space once all the other columns have
/// been laid out.
///
/// For example, if two columns have a [`FlexColumnWidth`], then half the space will go to one
/// and half the space will go to the other.
///
/// This is a cheap way to size a column.
pub struct FlexColumnWidth {
    /// The fraction of the remaining space once all the other columns have been laid out that
    /// this column should occupy.
    pub value: f64,
}

thread_local! {
    static DEFAULT_FLEX_COLUMN_WIDTH: TableColumnWidthRef = Rc::new(FlexColumnWidth::new(1.0));
}

impl FlexColumnWidth {
    /// Creates a column width based on a fraction of the remaining space once all the other
    /// columns have been laid out. Dart's default `value` is 1.0.
    pub const fn new(value: f64) -> FlexColumnWidth {
        FlexColumnWidth { value }
    }

    /// Dart's `const FlexColumnWidth()`, the default column width of a table: one shared value,
    /// as Dart's constant is one object, so a table given the default again is unchanged.
    pub fn default_width() -> TableColumnWidthRef {
        DEFAULT_FLEX_COLUMN_WIDTH.with(Rc::clone)
    }
}

impl TableColumnWidth for FlexColumnWidth {
    fn min_intrinsic_width(
        &self,
        _app: &mut App,
        _cells: &[AnyRenderBox],
        _container_width: f64,
    ) -> f64 {
        0.0
    }

    fn max_intrinsic_width(
        &self,
        _app: &mut App,
        _cells: &[AnyRenderBox],
        _container_width: f64,
    ) -> f64 {
        0.0
    }

    fn flex(&self, _app: &mut App, _cells: &[AnyRenderBox]) -> Option<f64> {
        Some(self.value)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl fmt::Debug for FlexColumnWidth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FlexColumnWidth({:.1})", self.value)
    }
}

/// Sizes the column such that it is the size that is the maximum of two column width
/// specifications.
///
/// For example, to have a column be 10% of the container width or 100px, whichever is bigger,
/// you could use:
///
/// ```ignore
/// MaxColumnWidth::new(Rc::new(FixedColumnWidth::new(100.0)), Rc::new(FractionColumnWidth::new(0.1)))
/// ```
///
/// Both specifications are evaluated, so if either specification is expensive, so is this.
#[derive(Debug)]
pub struct MaxColumnWidth {
    /// A lower bound for the width of this column.
    pub a: TableColumnWidthRef,
    /// Another lower bound for the width of this column.
    pub b: TableColumnWidthRef,
}

impl MaxColumnWidth {
    /// Creates a column width that is the maximum of two other column widths.
    pub fn new(a: TableColumnWidthRef, b: TableColumnWidthRef) -> MaxColumnWidth {
        MaxColumnWidth { a, b }
    }
}

impl TableColumnWidth for MaxColumnWidth {
    fn min_intrinsic_width(
        &self,
        app: &mut App,
        cells: &[AnyRenderBox],
        container_width: f64,
    ) -> f64 {
        self.a
            .min_intrinsic_width(app, cells, container_width)
            .max(self.b.min_intrinsic_width(app, cells, container_width))
    }

    fn max_intrinsic_width(
        &self,
        app: &mut App,
        cells: &[AnyRenderBox],
        container_width: f64,
    ) -> f64 {
        self.a
            .max_intrinsic_width(app, cells, container_width)
            .max(self.b.max_intrinsic_width(app, cells, container_width))
    }

    fn flex(&self, app: &mut App, cells: &[AnyRenderBox]) -> Option<f64> {
        let a_flex = self.a.flex(app, cells);
        let b_flex = self.b.flex(app, cells);
        match (a_flex, b_flex) {
            (None, _) => b_flex,
            (Some(_), None) => a_flex,
            (Some(a_flex), Some(b_flex)) => Some(a_flex.max(b_flex)),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Sizes the column such that it is the size that is the minimum of two column width
/// specifications.
///
/// For example, to have a column be 10% of the container width but never bigger than 100px,
/// you could use:
///
/// ```ignore
/// MinColumnWidth::new(Rc::new(FixedColumnWidth::new(100.0)), Rc::new(FractionColumnWidth::new(0.1)))
/// ```
///
/// Both specifications are evaluated, so if either specification is expensive, so is this.
#[derive(Debug)]
pub struct MinColumnWidth {
    /// An upper bound for the width of this column.
    pub a: TableColumnWidthRef,
    /// Another upper bound for the width of this column.
    pub b: TableColumnWidthRef,
}

impl MinColumnWidth {
    /// Creates a column width that is the minimum of two other column widths.
    pub fn new(a: TableColumnWidthRef, b: TableColumnWidthRef) -> MinColumnWidth {
        MinColumnWidth { a, b }
    }
}

impl TableColumnWidth for MinColumnWidth {
    fn min_intrinsic_width(
        &self,
        app: &mut App,
        cells: &[AnyRenderBox],
        container_width: f64,
    ) -> f64 {
        self.a
            .min_intrinsic_width(app, cells, container_width)
            .min(self.b.min_intrinsic_width(app, cells, container_width))
    }

    fn max_intrinsic_width(
        &self,
        app: &mut App,
        cells: &[AnyRenderBox],
        container_width: f64,
    ) -> f64 {
        self.a
            .max_intrinsic_width(app, cells, container_width)
            .min(self.b.max_intrinsic_width(app, cells, container_width))
    }

    fn flex(&self, app: &mut App, cells: &[AnyRenderBox]) -> Option<f64> {
        let a_flex = self.a.flex(app, cells);
        let b_flex = self.b.flex(app, cells);
        match (a_flex, b_flex) {
            (None, _) => b_flex,
            (Some(_), None) => a_flex,
            (Some(a_flex), Some(b_flex)) => Some(a_flex.min(b_flex)),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Vertical alignment options for cells in [`RenderTable`] objects.
///
/// This is specified using [`TableCellParentData`] objects on the parent data of the children
/// of the [`RenderTable`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TableCellVerticalAlignment {
    /// Cells with this alignment are placed with their top at the top of the row.
    #[default]
    Top,

    /// Cells with this alignment are vertically centered in the row.
    Middle,

    /// Cells with this alignment are placed with their bottom at the bottom of the row.
    Bottom,

    /// Cells with this alignment are aligned such that they all share the same baseline. Cells
    /// with no baseline are top-aligned instead. The baseline used is specified by
    /// [`RenderTable::text_baseline`]. It is not valid to use the baseline value if
    /// [`RenderTable::text_baseline`] is not specified.
    ///
    /// This vertical alignment is relatively expensive because it causes the table to compute
    /// the baseline for each cell in the row.
    Baseline,

    /// Cells with this alignment are sized to be as tall as the row, then made to fit the row.
    /// If all the cells have this alignment, then the row will have zero height.
    Fill,

    /// Cells with this alignment are sized to be the same height as the tallest cell in the
    /// row.
    IntrinsicHeight,
}

/// A table where the columns and rows are sized to fit the contents of the cells.
pub struct RenderTable {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    // Children are stored in row-major order.
    // children.len() must be rows * columns
    children: Vec<Option<AnyRenderBox>>,
    columns: usize,
    rows: usize,
    column_widths: HashMap<usize, TableColumnWidthRef>,
    default_column_width: TableColumnWidthRef,
    text_direction: TextDirection,
    border: Option<TableBorder>,
    // row_decorations and row_decoration_painters need to be in sync. They have to either both
    // be None or have same length.
    row_decorations: Option<Vec<Option<Box<dyn Decoration>>>>,
    row_decoration_painters: Option<Vec<Option<Box<dyn BoxPainter>>>>,
    configuration: ImageConfiguration,
    default_vertical_alignment: TableCellVerticalAlignment,
    text_baseline: Option<TextBaseline>,
    // cache the table geometry for painting purposes
    row_tops: Vec<f64>,
    column_lefts: Option<Vec<f64>>,
    table_width: f64,
    baseline_distance: Option<f64>,
}

impl RenderTable {
    /// Creates a table render object with no rows or columns.
    ///
    /// Dart's `columns`, `rows` and `children` arguments are [`set_columns`](Self::set_columns),
    /// [`set_rows`](Self::set_rows) and [`set_children`](Self::set_children), and its other named
    /// arguments are the setters: [`set_column_widths`](Self::set_column_widths) (empty),
    /// [`set_default_column_width`](Self::set_default_column_width)
    /// ([`FlexColumnWidth::default_width`]), [`set_border`](Self::set_border),
    /// [`set_row_decorations`](Self::set_row_decorations),
    /// [`set_configuration`](Self::set_configuration) ([`ImageConfiguration::EMPTY`]),
    /// [`set_default_vertical_alignment`](Self::set_default_vertical_alignment)
    /// ([`TableCellVerticalAlignment::Top`]) and [`set_text_baseline`](Self::set_text_baseline).
    pub fn new(app: &mut App, text_direction: TextDirection) -> RenderHandle<RenderTable> {
        RenderHandle::new_box(
            app,
            RenderTable {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                children: Vec::new(),
                columns: 0,
                rows: 0,
                column_widths: HashMap::new(),
                default_column_width: FlexColumnWidth::default_width(),
                text_direction,
                border: None,
                row_decorations: None,
                row_decoration_painters: None,
                configuration: ImageConfiguration::EMPTY,
                default_vertical_alignment: TableCellVerticalAlignment::Top,
                text_baseline: None,
                row_tops: Vec::new(),
                column_lefts: None,
                table_width: 0.0,
                baseline_distance: None,
            },
        )
    }

    /// The number of vertical alignment lines in this table.
    ///
    /// Changing the number of columns will remove any children that no longer fit in the
    /// table.
    ///
    /// Changing the number of columns is an expensive operation because the table needs to
    /// rearrange its internal representation.
    pub fn columns(self: RenderHandle<Self>, app: &App) -> usize {
        self.get(app).columns
    }

    /// Sets [`columns`](Self::columns).
    pub fn set_columns(self: RenderHandle<Self>, app: &mut App, value: usize) {
        if value == self.columns(app) {
            return;
        }
        let old_columns = self.columns(app);
        let rows = self.rows(app);
        let this = self.get_mut(app);
        let old_children = std::mem::take(&mut this.children);
        this.columns = value;
        let columns = value;
        this.children = vec![None; columns * rows];
        let columns_to_copy = columns.min(old_columns);
        for y in 0..rows {
            for x in 0..columns_to_copy {
                this.children[x + y * columns] = old_children[x + y * old_columns];
            }
        }
        if old_columns > columns {
            for y in 0..rows {
                for x in columns..old_columns {
                    let xy = x + y * old_columns;
                    if let Some(old_child) = old_children[xy] {
                        self.drop_child(app, old_child.as_object());
                    }
                }
            }
        }
        self.mark_needs_layout(app);
    }

    /// The number of horizontal alignment lines in this table.
    ///
    /// Changing the number of rows will remove any children that no longer fit in the table.
    pub fn rows(self: RenderHandle<Self>, app: &App) -> usize {
        self.get(app).rows
    }

    /// Sets [`rows`](Self::rows).
    pub fn set_rows(self: RenderHandle<Self>, app: &mut App, value: usize) {
        if value == self.rows(app) {
            return;
        }
        let columns = self.columns(app);
        if self.get(app).rows > value {
            for xy in columns * value..self.get(app).children.len() {
                if let Some(child) = self.get(app).children[xy] {
                    self.drop_child(app, child.as_object());
                }
            }
        }
        let this = self.get_mut(app);
        this.rows = value;
        this.children.resize(columns * value, None);
        self.mark_needs_layout(app);
    }

    /// How the horizontal extents of the columns of this table should be determined.
    ///
    /// If the map has no entry for a given column, the table uses the
    /// [`default_column_width`](Self::default_column_width) instead.
    ///
    /// The layout performance of the table depends critically on which column sizing
    /// algorithms are used here. In particular, [`IntrinsicColumnWidth`] is quite expensive
    /// because it needs to measure each cell in the column to determine the intrinsic size of
    /// the column.
    ///
    /// This property can never return `None`. If it is set to `None`, and the existing map is
    /// not empty, then the value is replaced by an empty map. (If it is set to `None` while the
    /// current value is an empty map, the value is not changed.)
    pub fn column_widths(
        self: RenderHandle<Self>,
        app: &App,
    ) -> HashMap<usize, TableColumnWidthRef> {
        self.get(app).column_widths.clone()
    }

    /// Sets [`column_widths`](Self::column_widths).
    pub fn set_column_widths(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<HashMap<usize, TableColumnWidthRef>>,
    ) {
        if self.get(app).column_widths.is_empty() && value.is_none() {
            return;
        }
        self.get_mut(app).column_widths = value.unwrap_or_default();
        self.mark_needs_layout(app);
    }

    /// Determines how the width of column with the given index is determined.
    pub fn set_column_width(
        self: RenderHandle<Self>,
        app: &mut App,
        column: usize,
        value: TableColumnWidthRef,
    ) {
        if self
            .get(app)
            .column_widths
            .get(&column)
            .is_some_and(|current| Rc::ptr_eq(current, &value))
        {
            return;
        }
        self.get_mut(app).column_widths.insert(column, value);
        self.mark_needs_layout(app);
    }

    /// How to determine with widths of columns that don't have an explicit sizing algorithm.
    ///
    /// Specifically, the [`default_column_width`](Self::default_column_width) is used for
    /// column `i` if `column_widths[i]` is absent.
    pub fn default_column_width(self: RenderHandle<Self>, app: &App) -> TableColumnWidthRef {
        Rc::clone(&self.get(app).default_column_width)
    }

    /// Sets [`default_column_width`](Self::default_column_width).
    pub fn set_default_column_width(
        self: RenderHandle<Self>,
        app: &mut App,
        value: TableColumnWidthRef,
    ) {
        if Rc::ptr_eq(&self.get(app).default_column_width, &value) {
            return;
        }
        self.get_mut(app).default_column_width = value;
        self.mark_needs_layout(app);
    }

    /// The direction in which the columns are ordered.
    pub fn text_direction(self: RenderHandle<Self>, app: &App) -> TextDirection {
        self.get(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(self: RenderHandle<Self>, app: &mut App, value: TextDirection) {
        if self.get(app).text_direction == value {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.mark_needs_layout(app);
    }

    /// The style to use when painting the boundary and interior divisions of the table.
    pub fn border(self: RenderHandle<Self>, app: &App) -> Option<TableBorder> {
        self.get(app).border
    }

    /// Sets [`border`](Self::border).
    pub fn set_border(self: RenderHandle<Self>, app: &mut App, value: Option<TableBorder>) {
        if self.border(app) == value {
            return;
        }
        self.get_mut(app).border = value;
        self.mark_needs_paint(app);
    }

    /// The decorations to use for each row of the table.
    ///
    /// Row decorations fill the horizontal and vertical extent of each row in the table, unlike
    /// decorations for individual cells, which might not fill either.
    pub fn row_decorations(self: RenderHandle<Self>, app: &App) -> &[Option<Box<dyn Decoration>>] {
        self.get(app).row_decorations.as_deref().unwrap_or(&[])
    }

    /// Sets [`row_decorations`](Self::row_decorations).
    pub fn set_row_decorations(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<Vec<Option<Box<dyn Decoration>>>>,
    ) {
        let this = self.get_mut(app);
        this.row_decorations = value;
        if let Some(painters) = this.row_decoration_painters.take() {
            for mut painter in painters.into_iter().flatten() {
                painter.dispose();
            }
        }
        this.row_decoration_painters = this
            .row_decorations
            .as_ref()
            .map(|decorations| decorations.iter().map(|_| None).collect());
    }

    /// The settings to pass to the [`row_decorations`](Self::row_decorations) when painting, so
    /// that they can resolve images appropriately. See [`ImageConfiguration`].
    pub fn configuration(self: RenderHandle<Self>, app: &App) -> &ImageConfiguration {
        &self.get(app).configuration
    }

    /// Sets [`configuration`](Self::configuration).
    pub fn set_configuration(self: RenderHandle<Self>, app: &mut App, value: ImageConfiguration) {
        if value == self.get(app).configuration {
            return;
        }
        self.get_mut(app).configuration = value;
        self.mark_needs_paint(app);
    }

    /// How cells that do not explicitly specify a vertical alignment are aligned vertically.
    pub fn default_vertical_alignment(
        self: RenderHandle<Self>,
        app: &App,
    ) -> TableCellVerticalAlignment {
        self.get(app).default_vertical_alignment
    }

    /// Sets [`default_vertical_alignment`](Self::default_vertical_alignment).
    pub fn set_default_vertical_alignment(
        self: RenderHandle<Self>,
        app: &mut App,
        value: TableCellVerticalAlignment,
    ) {
        if self.get(app).default_vertical_alignment == value {
            return;
        }
        self.get_mut(app).default_vertical_alignment = value;
        self.mark_needs_layout(app);
    }

    /// The text baseline to use when aligning rows using
    /// [`TableCellVerticalAlignment::Baseline`].
    pub fn text_baseline(self: RenderHandle<Self>, app: &App) -> Option<TextBaseline> {
        self.get(app).text_baseline
    }

    /// Sets [`text_baseline`](Self::text_baseline).
    pub fn set_text_baseline(self: RenderHandle<Self>, app: &mut App, value: Option<TextBaseline>) {
        if self.get(app).text_baseline == value {
            return;
        }
        self.get_mut(app).text_baseline = value;
        self.mark_needs_layout(app);
    }

    /// Replaces the children of this table with the given cells.
    ///
    /// The cells are divided into the specified number of columns before replacing the
    /// existing children.
    ///
    /// If the new cells contain any existing children of the table, those children are moved
    /// to their new location in the table rather than removed from the table and re-added.
    pub fn set_flat_children(
        self: RenderHandle<Self>,
        app: &mut App,
        columns: usize,
        cells: Vec<Option<AnyRenderBox>>,
    ) {
        // consider the case of a newly empty table
        if columns == 0 || cells.is_empty() {
            debug_assert!(cells.is_empty());
            self.get_mut(app).columns = columns;
            if self.get(app).children.is_empty() {
                debug_assert_eq!(self.get(app).rows, 0);
                return;
            }
            let old_children = std::mem::take(&mut self.get_mut(app).children);
            for old_child in old_children.into_iter().flatten() {
                self.drop_child(app, old_child.as_object());
            }
            self.get_mut(app).rows = 0;
            self.mark_needs_layout(app);
            return;
        }
        debug_assert!(cells.len().is_multiple_of(columns));
        let (old_columns, old_rows) = (self.get(app).columns, self.get(app).rows);
        // fill a set with the cells that are moving (it's important not to drop_child a child
        // that's remaining with us, because that would clear their parent data field)
        let mut lost_children: HashSet<AnyRenderBox> = HashSet::new();
        for y in 0..old_rows {
            for x in 0..old_columns {
                let xy_old = x + y * old_columns;
                let xy_new = x + y * columns;
                if let Some(old_child) = self.get(app).children[xy_old]
                    && (x >= columns || xy_new >= cells.len() || Some(old_child) != cells[xy_new])
                {
                    lost_children.insert(old_child);
                }
            }
        }
        // adopt cells that are arriving, and cross cells that are just moving off our list of
        // lost_children
        let mut y = 0;
        while y * columns < cells.len() {
            for x in 0..columns {
                let xy_new = x + y * columns;
                let xy_old = x + y * old_columns;
                if let Some(cell) = cells[xy_new]
                    && (x >= old_columns
                        || y >= old_rows
                        || self.get(app).children[xy_old] != Some(cell))
                    && !lost_children.remove(&cell)
                {
                    self.adopt_child(app, cell.as_object());
                }
            }
            y += 1;
        }
        // drop all the lost children
        for child in lost_children {
            self.drop_child(app, child.as_object());
        }
        // update our internal values
        let this = self.get_mut(app);
        this.columns = columns;
        this.rows = cells.len() / columns;
        this.children = cells;
        debug_assert_eq!(this.children.len(), this.rows * this.columns);
        self.mark_needs_layout(app);
    }

    /// Replaces the children of this table with the given cells.
    pub fn set_children(
        self: RenderHandle<Self>,
        app: &mut App,
        cells: Option<Vec<Vec<AnyRenderBox>>>,
    ) {
        // TODO(ianh): Make this smarter, like set_flat_children
        let Some(cells) = cells else {
            self.set_flat_children(app, 0, Vec::new());
            return;
        };
        let old_children = std::mem::take(&mut self.get_mut(app).children);
        for old_child in old_children.into_iter().flatten() {
            self.drop_child(app, old_child.as_object());
        }
        let this = self.get_mut(app);
        this.columns = cells.first().map_or(0, Vec::len);
        this.rows = 0;
        for row in cells {
            self.add_row(app, row.into_iter().map(Some).collect());
        }
        debug_assert_eq!(
            self.get(app).children.len(),
            self.rows(app) * self.columns(app)
        );
    }

    /// Adds a row to the end of the table.
    ///
    /// The newly added children must not already have parents.
    pub fn add_row(self: RenderHandle<Self>, app: &mut App, cells: Vec<Option<AnyRenderBox>>) {
        debug_assert_eq!(cells.len(), self.columns(app));
        debug_assert_eq!(
            self.get(app).children.len(),
            self.rows(app) * self.columns(app)
        );
        let this = self.get_mut(app);
        this.rows += 1;
        this.children.extend(cells.iter().copied());
        for cell in cells.into_iter().flatten() {
            self.adopt_child(app, cell.as_object());
        }
        self.mark_needs_layout(app);
    }

    /// Replaces the child at the given position with the given child.
    ///
    /// If the given child is already located at the given position, this function does not
    /// modify the table. Otherwise, the given child must not already have a parent.
    pub fn set_child(
        self: RenderHandle<Self>,
        app: &mut App,
        x: usize,
        y: usize,
        value: Option<AnyRenderBox>,
    ) {
        debug_assert!(x < self.columns(app) && y < self.rows(app));
        debug_assert_eq!(
            self.get(app).children.len(),
            self.rows(app) * self.columns(app)
        );
        let xy = x + y * self.columns(app);
        let old_child = self.get(app).children[xy];
        if old_child == value {
            return;
        }
        if let Some(old_child) = old_child {
            self.drop_child(app, old_child.as_object());
        }
        self.get_mut(app).children[xy] = value;
        if let Some(value) = value {
            self.adopt_child(app, value.as_object());
        }
    }

    /// Returns the list of [`RenderBox`] objects that are in the given column, in row order,
    /// starting from the first row.
    pub fn column(self: RenderHandle<Self>, app: &App, x: usize) -> Vec<AnyRenderBox> {
        let this = self.get(app);
        (0..this.rows)
            .filter_map(|y| this.children[x + y * this.columns])
            .collect()
    }

    /// Returns the list of [`RenderBox`] objects that are on the given row, in column order,
    /// starting with the first column.
    pub fn row(self: RenderHandle<Self>, app: &App, y: usize) -> Vec<AnyRenderBox> {
        let this = self.get(app);
        let start = y * this.columns;
        let end = (y + 1) * this.columns;
        this.children[start..end]
            .iter()
            .flatten()
            .copied()
            .collect()
    }

    /// The column width the table uses for column `x`.
    fn column_width_for(self: RenderHandle<Self>, app: &App, x: usize) -> TableColumnWidthRef {
        let this = self.get(app);
        Rc::clone(
            this.column_widths
                .get(&x)
                .unwrap_or(&this.default_column_width),
        )
    }

    fn compute_column_widths(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Vec<f64> {
        let columns = self.columns(app);
        debug_assert_eq!(self.get(app).children.len(), self.rows(app) * columns);
        // We apply the constraints to the column widths in the order of
        // least important to most important:
        // 1. apply the ideal widths (max_intrinsic_width)
        // 2. grow the flex columns so that the table has the max_width (if
        //    finite) or the min_width (if not)
        // 3. if there were no flex columns, then grow the table to the
        //    min_width.
        // 4. apply the maximum width of the table, shrinking columns as
        //    necessary, applying minimum column widths as we go

        // 1. apply ideal widths, and collect information we'll need later
        let mut widths = vec![0.0; columns];
        let mut min_widths = vec![0.0; columns];
        let mut flexes: Vec<Option<f64>> = vec![None; columns];
        let mut table_width = 0.0; // running tally of the sum of widths[x] for all x
        let mut unflexed_table_width = 0.0; // sum of the max_intrinsic_widths of any column that has None flex
        let mut total_flex = 0.0;
        for x in 0..columns {
            let column_width = self.column_width_for(app, x);
            let column_cells = self.column(app, x);
            // apply ideal width (max_intrinsic_width)
            let max_intrinsic_width =
                column_width.max_intrinsic_width(app, &column_cells, constraints.max_width);
            debug_assert!(max_intrinsic_width.is_finite());
            debug_assert!(max_intrinsic_width >= 0.0);
            widths[x] = max_intrinsic_width;
            table_width += max_intrinsic_width;
            // collect min width information while we're at it
            let min_intrinsic_width =
                column_width.min_intrinsic_width(app, &column_cells, constraints.max_width);
            debug_assert!(min_intrinsic_width.is_finite());
            debug_assert!(min_intrinsic_width >= 0.0);
            min_widths[x] = min_intrinsic_width;
            debug_assert!(max_intrinsic_width >= min_intrinsic_width);
            // collect flex information while we're at it
            let flex = column_width.flex(app, &column_cells);
            if let Some(flex) = flex {
                debug_assert!(flex.is_finite());
                debug_assert!(flex > 0.0);
                flexes[x] = Some(flex);
                total_flex += flex;
            } else {
                unflexed_table_width += max_intrinsic_width;
            }
        }
        let max_width_constraint = constraints.max_width;
        let min_width_constraint = constraints.min_width;

        // 2. grow the flex columns so that the table has the max_width (if
        //    finite) or the min_width (if not)
        if total_flex > 0.0 {
            // this can only grow the table, but it _will_ grow the table at
            // least as big as the target width.
            let target_width = if max_width_constraint.is_finite() {
                max_width_constraint
            } else {
                min_width_constraint
            };
            if table_width < target_width {
                let remaining_width = target_width - unflexed_table_width;
                debug_assert!(remaining_width.is_finite());
                debug_assert!(remaining_width >= 0.0);
                for x in 0..columns {
                    if let Some(flex) = flexes[x] {
                        let flexed_width = remaining_width * flex / total_flex;
                        debug_assert!(flexed_width.is_finite());
                        debug_assert!(flexed_width >= 0.0);
                        if widths[x] < flexed_width {
                            let delta = flexed_width - widths[x];
                            table_width += delta;
                            widths[x] = flexed_width;
                        }
                    }
                }
                debug_assert!(table_width + PRECISION_ERROR_TOLERANCE >= target_width);
            }
        }
        // step 2 and 3 are mutually exclusive
        // 3. if there were no flex columns, then grow the table to the
        //    min_width.
        else if table_width < min_width_constraint {
            let delta = (min_width_constraint - table_width) / columns as f64;
            for width in &mut widths {
                *width += delta;
            }
            table_width = min_width_constraint;
        }

        // beyond this point, unflexed_table_width is no longer valid

        // 4. apply the maximum width of the table, shrinking columns as
        //    necessary, applying minimum column widths as we go
        if table_width > max_width_constraint {
            let mut deficit = table_width - max_width_constraint;
            // Some columns may have low flex but have all the free space.
            // (Consider a case with a 1px wide column of flex 1000.0 and
            // a 1000px wide column of flex 1.0; the sizes coming from the
            // max_intrinsic_widths. If the maximum table width is 2px, then
            // just applying the flexes to the deficit would result in a
            // table with one column at -998px and one column at 990px,
            // which is wildly unhelpful.)
            // Similarly, some columns may be flexible, but not actually
            // be shrinkable due to a large minimum width. (Consider a
            // case with two columns, one is flex and one isn't, both have
            // 1000px max_intrinsic_widths, but the flex one has 1000px
            // min_intrinsic_width also. The whole deficit will have to come
            // from the non-flex column.)
            // So what we do is we repeatedly iterate through the flexible
            // columns shrinking them proportionally until we have no
            // available columns, then do the same to the non-flexible ones.
            let mut available_columns = columns;
            while deficit > PRECISION_ERROR_TOLERANCE && total_flex > PRECISION_ERROR_TOLERANCE {
                let mut new_total_flex = 0.0;
                for x in 0..columns {
                    if let Some(flex) = flexes[x] {
                        let new_width = widths[x] - deficit * flex / total_flex;
                        debug_assert!(new_width.is_finite());
                        if new_width <= min_widths[x] {
                            // shrank to minimum
                            deficit -= widths[x] - min_widths[x];
                            widths[x] = min_widths[x];
                            flexes[x] = None;
                            available_columns -= 1;
                        } else {
                            deficit -= widths[x] - new_width;
                            widths[x] = new_width;
                            new_total_flex += flex;
                        }
                        debug_assert!(widths[x] >= 0.0);
                    }
                }
                total_flex = new_total_flex;
            }
            while deficit > PRECISION_ERROR_TOLERANCE && available_columns > 0 {
                // Now we have to take out the remaining space from the
                // columns that aren't minimum sized.
                // To make this fair, we repeatedly remove equal amounts from
                // each column, clamped to the minimum width, until we run out
                // of columns that aren't at their min_width.
                let delta = deficit / available_columns as f64;
                debug_assert!(delta != 0.0);
                let mut new_available_columns = 0;
                for x in 0..columns {
                    let available_delta = widths[x] - min_widths[x];
                    if available_delta > 0.0 {
                        if available_delta <= delta {
                            // shrank to minimum
                            deficit -= widths[x] - min_widths[x];
                            widths[x] = min_widths[x];
                        } else {
                            deficit -= delta;
                            widths[x] -= delta;
                            new_available_columns += 1;
                        }
                    }
                }
                available_columns = new_available_columns;
            }
        }
        widths
    }

    /// Returns the position and dimensions of the box that the given row covers, in this render
    /// object's coordinate space (so the left coordinate is always 0.0).
    ///
    /// The row being queried must exist.
    ///
    /// This is only valid after layout.
    pub fn get_row_box(self: RenderHandle<Self>, app: &App, row: usize) -> Rect {
        debug_assert!(row < self.rows(app));
        debug_assert!(!self.debug_needs_layout(app));
        let row_tops = &self.get(app).row_tops;
        Rect::from_ltrb(
            0.0,
            row_tops[row],
            self.size(app).width(),
            row_tops[row + 1],
        )
    }

    /// The vertical alignment of `child`: its own, else the table's default.
    fn vertical_alignment_of(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderBox,
    ) -> TableCellVerticalAlignment {
        child
            .as_object()
            .parent_data_of::<TableCellParentData>(app)
            .vertical_alignment
            .unwrap_or(self.get(app).default_vertical_alignment)
    }

    fn set_child_offset(app: &mut App, child: AnyRenderBox, offset: Offset) {
        child.parent_data_of_mut::<BoxParentData>(app).offset = offset;
    }
}

impl RenderObject for RenderTable {
    crate::render_object_accessors!();

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        let this = self.get_mut(app);
        if let Some(painters) = this.row_decoration_painters.take() {
            for mut painter in painters.into_iter().flatten() {
                painter.dispose();
            }
            let decorations = this.row_decorations.as_ref().map_or(0, Vec::len);
            this.row_decoration_painters = Some((0..decorations).map(|_| None).collect());
        }
        let children = self.get(app).children.clone();
        for child in children.into_iter().flatten() {
            child.as_object().detach(app);
        }
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        let this = self.get(app);
        debug_assert_eq!(this.children.len(), this.rows * this.columns);
        for child in this.children.iter().flatten() {
            visitor(child.as_object());
        }
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let rows = self.rows(app);
        let columns = self.columns(app);
        debug_assert_eq!(self.get(app).children.len(), rows * columns);
        if rows * columns == 0 {
            // TODO(ianh): if columns is zero, this should be zero width
            // TODO(ianh): if columns is not zero, this should be based on the column width
            // specifications
            self.get_mut(app).table_width = 0.0;
            self.set_size(app, constraints.constrain(Size::ZERO));
            return;
        }
        let widths = self.compute_column_widths(app, constraints);
        let mut positions = vec![0.0; columns];
        let (column_lefts, table_width) = match self.text_direction(app) {
            TextDirection::Rtl => {
                positions[columns - 1] = 0.0;
                for x in (0..columns - 1).rev() {
                    positions[x] = positions[x + 1] + widths[x + 1];
                }
                let table_width = positions[0] + widths[0];
                (positions.iter().rev().copied().collect(), table_width)
            }
            TextDirection::Ltr => {
                positions[0] = 0.0;
                for x in 1..columns {
                    positions[x] = positions[x - 1] + widths[x - 1];
                }
                let table_width = positions[columns - 1] + widths[columns - 1];
                (positions.clone(), table_width)
            }
        };
        let this = self.get_mut(app);
        this.column_lefts = Some(column_lefts);
        this.table_width = table_width;
        this.row_tops.clear();
        this.baseline_distance = None;
        // then, lay out each row
        let mut row_top = 0.0;
        for y in 0..rows {
            self.get_mut(app).row_tops.push(row_top);
            let mut row_height: f64 = 0.0;
            let mut have_baseline = false;
            let mut before_baseline_distance: f64 = 0.0;
            let mut after_baseline_distance: f64 = 0.0;
            let mut baselines = vec![0.0; columns];
            for x in 0..columns {
                let xy = x + y * columns;
                let Some(child) = self.get(app).children[xy] else {
                    continue;
                };
                let child_parent_data = child
                    .as_object()
                    .parent_data_of_mut::<TableCellParentData>(app);
                child_parent_data.x = Some(x);
                child_parent_data.y = Some(y);
                match self.vertical_alignment_of(app, child) {
                    TableCellVerticalAlignment::Baseline => {
                        let text_baseline = self.text_baseline(app).expect(
                            "An explicit textBaseline is required when using baseline alignment.",
                        );
                        child.layout(app, BoxConstraints::tight_for(Some(widths[x]), None), true);
                        let child_baseline =
                            child.get_distance_to_baseline(app, text_baseline, true);
                        if let Some(child_baseline) = child_baseline {
                            before_baseline_distance = before_baseline_distance.max(child_baseline);
                            after_baseline_distance = after_baseline_distance
                                .max(child.size(app).height() - child_baseline);
                            baselines[x] = child_baseline;
                            have_baseline = true;
                        } else {
                            row_height = row_height.max(child.size(app).height());
                            RenderTable::set_child_offset(
                                app,
                                child,
                                Offset::new(positions[x], row_top),
                            );
                        }
                    }
                    TableCellVerticalAlignment::Top
                    | TableCellVerticalAlignment::Middle
                    | TableCellVerticalAlignment::Bottom
                    | TableCellVerticalAlignment::IntrinsicHeight => {
                        child.layout(app, BoxConstraints::tight_for(Some(widths[x]), None), true);
                        row_height = row_height.max(child.size(app).height());
                    }
                    TableCellVerticalAlignment::Fill => {}
                }
            }
            if have_baseline {
                if y == 0 {
                    self.get_mut(app).baseline_distance = Some(before_baseline_distance);
                }
                row_height = row_height.max(before_baseline_distance + after_baseline_distance);
            }
            for x in 0..columns {
                let xy = x + y * columns;
                let Some(child) = self.get(app).children[xy] else {
                    continue;
                };
                match self.vertical_alignment_of(app, child) {
                    TableCellVerticalAlignment::Baseline => {
                        RenderTable::set_child_offset(
                            app,
                            child,
                            Offset::new(
                                positions[x],
                                row_top + before_baseline_distance - baselines[x],
                            ),
                        );
                    }
                    TableCellVerticalAlignment::Top => {
                        RenderTable::set_child_offset(
                            app,
                            child,
                            Offset::new(positions[x], row_top),
                        );
                    }
                    TableCellVerticalAlignment::Middle => {
                        let offset = Offset::new(
                            positions[x],
                            row_top + (row_height - child.size(app).height()) / 2.0,
                        );
                        RenderTable::set_child_offset(app, child, offset);
                    }
                    TableCellVerticalAlignment::Bottom => {
                        let offset = Offset::new(
                            positions[x],
                            row_top + row_height - child.size(app).height(),
                        );
                        RenderTable::set_child_offset(app, child, offset);
                    }
                    TableCellVerticalAlignment::Fill
                    | TableCellVerticalAlignment::IntrinsicHeight => {
                        child.layout(
                            app,
                            BoxConstraints::tight_for(Some(widths[x]), Some(row_height)),
                            false,
                        );
                        RenderTable::set_child_offset(
                            app,
                            child,
                            Offset::new(positions[x], row_top),
                        );
                    }
                }
            }
            row_top += row_height;
        }
        self.get_mut(app).row_tops.push(row_top);
        let table_width = self.get(app).table_width;
        self.set_size(app, constraints.constrain(Size::new(table_width, row_top)));
        debug_assert_eq!(self.get(app).row_tops.len(), rows + 1);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let rows = self.rows(app);
        let columns = self.columns(app);
        debug_assert_eq!(self.get(app).children.len(), rows * columns);
        if rows * columns == 0 {
            if let Some(border) = self.border(app) {
                let border_rect =
                    Rect::from_ltwh(offset.dx(), offset.dy(), self.get(app).table_width, 0.0);
                border.paint(context.canvas(), border_rect, &[], &[]);
            }
            return;
        }
        debug_assert_eq!(self.get(app).row_tops.len(), rows + 1);
        if self.get(app).row_decorations.is_some() {
            let size = self.size(app);
            let this = self.get_mut(app);
            let decorations = this.row_decorations.as_ref().expect("checked above");
            let painters = this
                .row_decoration_painters
                .as_mut()
                .expect("in sync with the decorations");
            debug_assert_eq!(decorations.len(), painters.len());
            let canvas = context.canvas();
            for y in 0..rows {
                if decorations.len() <= y {
                    break;
                }
                if let Some(decoration) = &decorations[y] {
                    let painter = painters[y].get_or_insert_with(|| {
                        decoration.create_box_painter(Some(Rc::new(move |app| {
                            self.mark_needs_paint(app)
                        })))
                    });
                    let row_size = Size::new(size.width(), this.row_tops[y + 1] - this.row_tops[y]);
                    painter.paint(
                        canvas,
                        Offset::new(offset.dx(), offset.dy() + this.row_tops[y]),
                        &this
                            .configuration
                            .copy_with(None, None, None, None, Some(row_size), None),
                    );
                }
            }
        }
        let children = self.get(app).children.clone();
        for child in children.into_iter().flatten() {
            let child_offset = child.box_parent_data(app).offset;
            context.paint_child(app, child.as_object(), child_offset + offset);
        }
        debug_assert_eq!(rows, self.get(app).row_tops.len() - 1);
        debug_assert_eq!(
            Some(columns),
            self.get(app).column_lefts.as_ref().map(Vec::len)
        );
        if let Some(border) = self.border(app) {
            // The border rect might not fill the entire height of this render object
            // if the rows underflow. We always force the columns to fill the width of
            // the render object, which means the columns cannot underflow.
            let this = self.get(app);
            let row_tops = &this.row_tops;
            let border_rect = Rect::from_ltwh(
                offset.dx(),
                offset.dy(),
                this.table_width,
                row_tops[row_tops.len() - 1],
            );
            let rows = row_tops[1..row_tops.len() - 1].to_vec();
            let columns = this.column_lefts.as_ref().expect("laid out")[1..].to_vec();
            border.paint(context.canvas(), border_rect, &rows, &columns);
        }
    }
}

impl RenderBox for RenderTable {
    crate::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<TableCellParentData>(app) {
            child.set_parent_data(app, TableCellParentData::new());
        }
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, _height: f64) -> f64 {
        let (rows, columns) = (self.rows(app), self.columns(app));
        debug_assert_eq!(self.get(app).children.len(), rows * columns);
        if rows * columns == 0 {
            return 0.0;
        }
        let mut total_min_width = 0.0;
        for x in 0..columns {
            let column_width = self.column_width_for(app, x);
            let column_cells = self.column(app, x);
            total_min_width += column_width.min_intrinsic_width(app, &column_cells, f64::INFINITY);
        }
        total_min_width
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, _height: f64) -> f64 {
        let (rows, columns) = (self.rows(app), self.columns(app));
        debug_assert_eq!(self.get(app).children.len(), rows * columns);
        if rows * columns == 0 {
            return 0.0;
        }
        let mut total_max_width = 0.0;
        for x in 0..columns {
            let column_width = self.column_width_for(app, x);
            let column_cells = self.column(app, x);
            total_max_width += column_width.max_intrinsic_width(app, &column_cells, f64::INFINITY);
        }
        total_max_width
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        // winner of the 2016 world's most expensive intrinsic dimension function award
        // honorable mention, most likely to improve if taught about memoization award
        let (rows, columns) = (self.rows(app), self.columns(app));
        debug_assert_eq!(self.get(app).children.len(), rows * columns);
        if rows * columns == 0 {
            return 0.0;
        }
        let widths =
            self.compute_column_widths(app, BoxConstraints::tight_for_finite(width, f64::INFINITY));
        let mut row_top = 0.0;
        for y in 0..rows {
            let mut row_height: f64 = 0.0;
            for (x, &width) in widths.iter().enumerate() {
                let xy = x + y * columns;
                if let Some(child) = self.get(app).children[xy] {
                    row_height = row_height.max(child.get_max_intrinsic_height(app, width));
                }
            }
            row_top += row_height;
        }
        row_top
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.as_box().get_min_intrinsic_height(app, width)
    }

    /// Returns the baseline offset of the cell in the first row with the lowest baseline, and
    /// uses [`TableCellVerticalAlignment::Baseline`].
    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        _baseline: TextBaseline,
    ) -> Option<f64> {
        debug_assert!(!self.debug_needs_layout(app));
        self.get(app).baseline_distance
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let columns = self.columns(app);
        if self.rows(app) * columns == 0 {
            return None;
        }
        let widths = self.compute_column_widths(app, constraints);
        let mut baseline_offset: Option<f64> = None;
        for (col, &width) in widths.iter().enumerate() {
            let Some(child) = self.get(app).children[col] else {
                continue;
            };
            let child_constraints = BoxConstraints::tight_for(Some(width), None);
            let child_baseline = match self.vertical_alignment_of(app, child) {
                TableCellVerticalAlignment::Baseline => {
                    child.get_dry_baseline(app, child_constraints, baseline)
                }
                TableCellVerticalAlignment::Top
                | TableCellVerticalAlignment::Middle
                | TableCellVerticalAlignment::Bottom
                | TableCellVerticalAlignment::Fill
                | TableCellVerticalAlignment::IntrinsicHeight => None,
            };
            if let Some(child_baseline) = child_baseline
                && baseline_offset.is_none_or(|offset| offset < child_baseline)
            {
                baseline_offset = Some(child_baseline);
            }
        }
        baseline_offset
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let (rows, columns) = (self.rows(app), self.columns(app));
        if rows * columns == 0 {
            return constraints.constrain(Size::ZERO);
        }
        let widths = self.compute_column_widths(app, constraints);
        let table_width: f64 = widths.iter().sum();
        let mut row_top = 0.0;
        for y in 0..rows {
            let mut row_height: f64 = 0.0;
            for (x, &width) in widths.iter().enumerate() {
                let xy = x + y * columns;
                let Some(child) = self.get(app).children[xy] else {
                    continue;
                };
                match self.vertical_alignment_of(app, child) {
                    TableCellVerticalAlignment::Baseline => {
                        self.debug_cannot_compute_dry_layout(DryLayoutFailure::Reason(
                            "TableCellVerticalAlignment.baseline requires a full layout for \
                             baseline metrics to be available.",
                        ));
                        return Size::ZERO;
                    }
                    TableCellVerticalAlignment::Top
                    | TableCellVerticalAlignment::Middle
                    | TableCellVerticalAlignment::Bottom
                    | TableCellVerticalAlignment::IntrinsicHeight => {
                        let child_size =
                            child.get_dry_layout(app, BoxConstraints::tight_for(Some(width), None));
                        row_height = row_height.max(child_size.height());
                    }
                    TableCellVerticalAlignment::Fill => {}
                }
            }
            row_top += row_height;
        }
        constraints.constrain(Size::new(table_width, row_top))
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        debug_assert_eq!(
            self.get(app).children.len(),
            self.rows(app) * self.columns(app)
        );
        let children = self.get(app).children.clone();
        for child in children.into_iter().rev().flatten() {
            let child_offset = child.box_parent_data(app).offset;
            let is_hit = result.add_with_paint_offset(
                Some(child_offset),
                position,
                |result, transformed| {
                    debug_assert_eq!(transformed, position - child_offset);
                    child.hit_test(app, result, transformed)
                },
            );
            if is_hit {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use inset_embedder::valo::Op;
    use inset_embedder::{Color, Radius};
    use inset_foundation::{AppCell, Handle};
    use inset_painting::{AlignmentGeometry, BorderRadius, BorderStyle};

    use super::*;
    use crate::layer::{ContainerLayer, ErasedLayer, OffsetLayer, PictureLayer};
    use crate::pipeline_owner::PipelineOwner;
    use crate::proxy_box::{RenderConstrainedBox, RenderRepaintBoundary};
    use crate::shifted_box::RenderPositionedBox;

    fn sized_box(app: &mut App, width: f64, height: f64) -> AnyRenderBox {
        RenderConstrainedBox::new(app, BoxConstraints::tight(Size::new(width, height)), None)
            .as_box()
    }

    fn positioned_box(app: &mut App) -> AnyRenderBox {
        RenderPositionedBox::new(app, AlignmentGeometry::CENTER, None, None, None, None).as_box()
    }

    /// `rendering_tester.dart`'s `layout`: attach `root` under a repaint boundary, lay it out
    /// with `constraints`, and paint it. The default constraints are an 800 by 600 screen.
    struct Laid {
        owner: Handle<PipelineOwner>,
        boundary: RenderHandle<RenderRepaintBoundary>,
    }

    fn layout(app: &mut App, root: AnyRenderBox, constraints: BoxConstraints) -> Laid {
        let boundary = RenderRepaintBoundary::new(app, Some(root));
        let owner = PipelineOwner::new(app, None, None);
        owner.set_root_node(app, Some(boundary.as_object()));
        boundary.as_box().layout(app, constraints, false);
        let paint_root = OffsetLayer::new(app, Offset::ZERO);
        paint_root.as_layer().attach(app, boundary.as_object().id());
        boundary
            .as_object()
            .schedule_initial_paint(app, paint_root.as_container_layer());
        pump_frame(app, owner);
        Laid { owner, boundary }
    }

    fn screen() -> BoxConstraints {
        BoxConstraints::tight(Size::new(800.0, 600.0))
    }

    /// `rendering_tester.dart`'s `pumpFrame`: layout, compositing bits and paint.
    fn pump_frame(app: &mut App, owner: Handle<PipelineOwner>) {
        owner.flush_layout(app);
        owner.flush_compositing_bits(app);
        owner.flush_paint(app);
    }

    /// The display list ops the boundary painted, in order.
    fn painted(app: &App, laid: &Laid) -> Vec<Op> {
        let layer = laid.boundary.as_object().debug_layer(app).expect("painted");
        layer
            .depth_first_iterate_children(app)
            .into_iter()
            .filter_map(|child| {
                app.handle::<PictureLayer>(child.id())
                    .and_then(|picture| picture.picture(app).map(|p| p.ops().to_vec()))
            })
            .flatten()
            .collect()
    }

    fn paths_painted(app: &App, laid: &Laid) -> usize {
        painted(app, laid)
            .iter()
            .filter(|op| matches!(op, Op::DrawPath { .. }))
            .count()
    }

    /// `table_test.dart`: "Table control test; tight".
    #[test]
    fn an_empty_table_fills_tight_constraints() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let table = RenderTable::new(&mut app, TextDirection::Ltr);
        layout(&mut app, table.as_box(), screen());
        assert_eq!(table.size(&app), Size::new(800.0, 600.0));
    }

    /// `table_test.dart`: "Table control test; loose".
    #[test]
    fn an_empty_table_is_empty_in_loose_constraints() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let table = RenderTable::new(&mut app, TextDirection::Ltr);
        let center = RenderPositionedBox::new(
            &mut app,
            AlignmentGeometry::CENTER,
            None,
            None,
            None,
            Some(table.as_box()),
        );
        layout(&mut app, center.as_box(), screen());
        assert_eq!(table.size(&app), Size::ZERO);
    }

    /// `table_test.dart`: "Table control test: constrained flex columns".
    #[test]
    fn flex_columns_share_the_width() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let table = RenderTable::new(&mut app, TextDirection::Ltr);
        let children: Vec<AnyRenderBox> = (0..6).map(|_| positioned_box(&mut app)).collect();
        table.set_flat_children(&mut app, 6, children.iter().copied().map(Some).collect());
        layout(
            &mut app,
            table.as_box(),
            BoxConstraints::tight_for(Some(100.0), None),
        );
        let expected_width = 100.0 / 6.0;
        for child in children {
            assert!((child.size(&app).width() - expected_width).abs() < 1e-10);
        }
    }

    /// `table_test.dart`: "Table test: combinations". Boxes have no baseline, so a baseline
    /// row top-aligns them.
    #[test]
    fn intrinsic_columns_fit_their_widest_cell() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let table = RenderTable::new(&mut app, TextDirection::Ltr);
        table.set_columns(&mut app, 5);
        table.set_rows(&mut app, 5);
        table.set_default_column_width(&mut app, Rc::new(IntrinsicColumnWidth::new(None)));
        table.set_default_vertical_alignment(&mut app, TableCellVerticalAlignment::Baseline);
        table.set_text_baseline(&mut app, Some(TextBaseline::Alphabetic));
        let center = RenderPositionedBox::new(
            &mut app,
            AlignmentGeometry::CENTER,
            None,
            None,
            None,
            Some(table.as_box()),
        );
        let laid = layout(&mut app, center.as_box(), screen());
        assert_eq!(table.size(&app), Size::ZERO);

        let tall = sized_box(&mut app, 100.0, 200.0);
        table.set_child(&mut app, 2, 4, Some(tall));
        pump_frame(&mut app, laid.owner);
        assert_eq!(table.size(&app), Size::new(100.0, 200.0));

        let first = sized_box(&mut app, 10.0, 30.0);
        let second = sized_box(&mut app, 20.0, 20.0);
        let third = sized_box(&mut app, 30.0, 10.0);
        table.set_child(&mut app, 0, 0, Some(first));
        table.set_child(&mut app, 1, 0, Some(second));
        table.set_child(&mut app, 2, 0, Some(third));
        pump_frame(&mut app, laid.owner);
        assert_eq!(table.size(&app), Size::new(130.0, 230.0));
        assert_eq!(
            table.get(&app).column_lefts.as_deref(),
            Some(&[0.0, 10.0, 30.0, 130.0, 130.0][..])
        );
        assert_eq!(
            table.get(&app).row_tops,
            [0.0, 30.0, 30.0, 30.0, 30.0, 230.0]
        );
        assert_eq!(third.size(&app), Size::new(100.0, 10.0));
        assert_eq!(tall.box_parent_data(&app).offset, Offset::new(30.0, 30.0));
        let parent_data = third
            .as_object()
            .parent_data_of::<TableCellParentData>(&app);
        assert_eq!((parent_data.x, parent_data.y), (Some(2), Some(0)));
    }

    /// `table_test.dart`: "Table test: removing cells".
    #[test]
    fn fewer_rows_drop_the_cells_below() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let table = RenderTable::new(&mut app, TextDirection::Ltr);
        table.set_columns(&mut app, 5);
        table.set_rows(&mut app, 5);
        let child = sized_box(&mut app, 10.0, 10.0);
        table.set_child(&mut app, 4, 4, Some(child));
        layout(&mut app, table.as_box(), screen());
        assert!(child.as_object().attached(&app));
        table.set_rows(&mut app, 4);
        assert!(!child.as_object().attached(&app));
    }

    /// `table_test.dart`: "Table test: replacing cells". A cell that moves keeps its parent.
    #[test]
    fn moving_cells_keeps_them_in_the_table() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let table = RenderTable::new(&mut app, TextDirection::Ltr);
        let moving: Vec<AnyRenderBox> = (0..3).map(|_| positioned_box(&mut app)).collect();
        let mut cells = Vec::new();
        for child in &moving {
            cells.push(Some(*child));
            cells.push(Some(positioned_box(&mut app)));
        }
        table.set_flat_children(&mut app, 3, cells);
        assert_eq!(table.rows(&app), 2);
        let laid = layout(&mut app, table.as_box(), screen());
        for _ in 0..2 {
            let mut cells = Vec::new();
            for child in &moving {
                cells.push(Some(positioned_box(&mut app)));
                cells.push(Some(*child));
            }
            table.set_flat_children(&mut app, 3, cells);
            pump_frame(&mut app, laid.owner);
        }
        assert_eq!(table.columns(&app), 3);
        assert_eq!(table.rows(&app), 2);
        for child in moving {
            assert_eq!(child.as_object().parent(&app), Some(table.as_object()));
        }
    }

    /// `table_test.dart`: "Table border painting": the inner lines are one path per direction,
    /// after which the outer border paints its four sides.
    #[test]
    fn a_border_paints_its_inner_lines_and_four_sides() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let table = RenderTable::new(&mut app, TextDirection::Rtl);
        table.set_border(
            &mut app,
            Some(TableBorder::all(
                Color::new(0xFF000000),
                1.0,
                BorderStyle::Solid,
                BorderRadius::ZERO,
            )),
        );
        let laid = layout(&mut app, table.as_box(), screen());
        for (columns, cells, paths) in [
            (1, 0, 4),
            (1, 1, 4),
            (1, 2, 5),
            (2, 2, 5),
            (2, 4, 6),
            (3, 6, 6),
        ] {
            let children = (0..cells).map(|_| Some(positioned_box(&mut app))).collect();
            table.set_flat_children(&mut app, columns, children);
            pump_frame(&mut app, laid.owner);
            assert_eq!(
                paths_painted(&app, &laid),
                paths,
                "{columns} columns, {cells} cells"
            );
        }
    }

    /// `table_test.dart`: "Table paints a borderRadius": a uniform rounded border is one ring
    /// after the two inner lines.
    #[test]
    fn a_rounded_uniform_border_paints_one_ring() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let table = RenderTable::new(&mut app, TextDirection::Ltr);
        table.set_border(
            &mut app,
            Some(TableBorder::all(
                Color::new(0xFF000000),
                1.0,
                BorderStyle::Solid,
                BorderRadius::all(Radius::circular(8.0)),
            )),
        );
        let laid = layout(&mut app, table.as_box(), screen());
        let children = (0..4).map(|_| Some(positioned_box(&mut app))).collect();
        table.set_flat_children(&mut app, 2, children);
        pump_frame(&mut app, laid.owner);
        assert_eq!(paths_painted(&app, &laid), 3);
    }

    /// Row decorations paint behind each row, filling it.
    #[test]
    fn row_decorations_paint_behind_their_rows() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let table = RenderTable::new(&mut app, TextDirection::Ltr);
        let rows = vec![
            vec![sized_box(&mut app, 10.0, 10.0)],
            vec![sized_box(&mut app, 10.0, 20.0)],
        ];
        table.set_children(&mut app, Some(rows));
        table.set_row_decorations(
            &mut app,
            Some(vec![
                None,
                Some(Box::new(
                    inset_painting::BoxDecoration::new().color(Color::new(0xFF0000FF)),
                )),
            ]),
        );
        let laid = layout(
            &mut app,
            table.as_box(),
            BoxConstraints::tight_for(Some(100.0), None),
        );
        let rects: Vec<_> = painted(&app, &laid)
            .into_iter()
            .filter_map(|op| match op {
                Op::DrawRect { rect, .. } => Some(rect),
                _ => None,
            })
            .collect();
        assert_eq!(rects.len(), 1, "only the second row is decorated");
        assert_eq!(
            table.get_row_box(&app, 1),
            Rect::from_ltrb(0.0, 10.0, 100.0, 30.0)
        );
    }

    /// `table_test.dart`: "MaxColumnWidth.flex returns the correct result" and
    /// "MinColumnWidth.flex returns the correct result".
    #[test]
    fn max_and_min_widths_take_the_flex_of_either() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let fixed: TableColumnWidthRef = Rc::new(FixedColumnWidth::new(100.0));
        let flex: TableColumnWidthRef = Rc::new(FlexColumnWidth::new(1.0));
        for width in [
            Rc::new(MaxColumnWidth::new(Rc::clone(&fixed), Rc::clone(&flex)))
                as TableColumnWidthRef,
            Rc::new(MaxColumnWidth::new(Rc::clone(&flex), Rc::clone(&fixed))),
            Rc::new(MinColumnWidth::new(Rc::clone(&fixed), Rc::clone(&flex))),
            Rc::new(MinColumnWidth::new(Rc::clone(&flex), Rc::clone(&fixed))),
        ] {
            assert_eq!(width.flex(&mut app, &[]), Some(1.0), "{width:?}");
        }
    }

    /// Fixed and fractional columns take their width; a flexible one takes the rest.
    #[test]
    fn fixed_fraction_and_flex_columns_divide_the_width() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let table = RenderTable::new(&mut app, TextDirection::Ltr);
        let children: Vec<AnyRenderBox> = (0..3).map(|_| positioned_box(&mut app)).collect();
        table.set_flat_children(&mut app, 3, children.iter().copied().map(Some).collect());
        let mut widths: HashMap<usize, TableColumnWidthRef> = HashMap::new();
        widths.insert(0, Rc::new(FixedColumnWidth::new(50.0)));
        widths.insert(1, Rc::new(FractionColumnWidth::new(0.25)));
        table.set_column_widths(&mut app, Some(widths));
        layout(
            &mut app,
            table.as_box(),
            BoxConstraints::tight_for(Some(400.0), None),
        );
        let widths: Vec<f64> = children
            .iter()
            .map(|child| child.size(&app).width())
            .collect();
        assert_eq!(widths, [50.0, 100.0, 250.0]);
    }

    /// Too little room shrinks no column below its minimum: the flexible column stops at its
    /// fixed lower bound, and the table is cut to its constraints.
    #[test]
    fn no_column_shrinks_below_its_minimum() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let table = RenderTable::new(&mut app, TextDirection::Ltr);
        let children: Vec<AnyRenderBox> = (0..2).map(|_| positioned_box(&mut app)).collect();
        table.set_flat_children(&mut app, 2, children.iter().copied().map(Some).collect());
        let mut widths: HashMap<usize, TableColumnWidthRef> = HashMap::new();
        widths.insert(0, Rc::new(FixedColumnWidth::new(150.0)));
        widths.insert(
            1,
            Rc::new(MaxColumnWidth::new(
                Rc::new(FixedColumnWidth::new(80.0)),
                Rc::new(FlexColumnWidth::new(1.0)),
            )),
        );
        table.set_column_widths(&mut app, Some(widths));
        layout(
            &mut app,
            table.as_box(),
            BoxConstraints::tight_for(Some(200.0), None),
        );
        let widths: Vec<f64> = children
            .iter()
            .map(|child| child.size(&app).width())
            .collect();
        assert_eq!(widths, [150.0, 80.0]);
        assert_eq!(table.size(&app).width(), 200.0);
    }

    /// `table_test.dart`: "TableRows with different constraints, but vertically with
    /// intrinsicHeight": every cell takes the height of the tallest.
    #[test]
    fn intrinsic_height_cells_match_the_tallest() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let short = RenderConstrainedBox::new(
            &mut app,
            BoxConstraints::tight_for(Some(100.0), Some(100.0)),
            None,
        );
        let tall = RenderConstrainedBox::new(
            &mut app,
            BoxConstraints::tight_for(Some(200.0), Some(200.0)),
            None,
        );
        let table = RenderTable::new(&mut app, TextDirection::Rtl);
        table.set_default_vertical_alignment(&mut app, TableCellVerticalAlignment::IntrinsicHeight);
        table.set_children(&mut app, Some(vec![vec![short.as_box(), tall.as_box()]]));
        layout(
            &mut app,
            table.as_box(),
            BoxConstraints::tight(Size::new(300.0, 300.0)),
        );
        assert_eq!(table.size(&app), Size::new(300.0, 300.0));
        assert_eq!(short.size(&app).height(), tall.size(&app).height());
    }

    /// `table_test.dart`: "Empty table intrinsic dimensions should not crash".
    #[test]
    fn an_empty_table_has_no_intrinsic_size() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let table = RenderTable::new(&mut app, TextDirection::Ltr).as_box();
        for extent in [100.0, f64::INFINITY] {
            assert_eq!(table.get_min_intrinsic_width(&mut app, extent), 0.0);
            assert_eq!(table.get_max_intrinsic_width(&mut app, extent), 0.0);
            assert_eq!(table.get_min_intrinsic_height(&mut app, extent), 0.0);
            assert_eq!(table.get_max_intrinsic_height(&mut app, extent), 0.0);
        }
    }

    /// The intrinsic widths sum the columns, and the intrinsic height at a width sums the
    /// tallest cell of each row; the dry layout matches the layout.
    #[test]
    fn intrinsics_and_dry_layout_follow_the_cells() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let table = RenderTable::new(&mut app, TextDirection::Ltr);
        table.set_default_column_width(&mut app, Rc::new(IntrinsicColumnWidth::new(None)));
        let rows = vec![
            vec![
                sized_box(&mut app, 10.0, 30.0),
                sized_box(&mut app, 40.0, 5.0),
            ],
            vec![
                sized_box(&mut app, 20.0, 10.0),
                sized_box(&mut app, 5.0, 15.0),
            ],
        ];
        table.set_children(&mut app, Some(rows));
        let table = table.as_box();
        assert_eq!(table.get_min_intrinsic_width(&mut app, f64::INFINITY), 60.0);
        assert_eq!(table.get_max_intrinsic_width(&mut app, f64::INFINITY), 60.0);
        assert_eq!(table.get_min_intrinsic_height(&mut app, 60.0), 45.0);
        assert_eq!(table.get_max_intrinsic_height(&mut app, 60.0), 45.0);
        let loose = BoxConstraints::new().max_width(500.0).max_height(500.0);
        let dry = table.get_dry_layout(&mut app, loose);
        layout(&mut app, table, loose);
        assert_eq!(dry, Size::new(60.0, 45.0));
        assert_eq!(table.size(&app), dry);
    }

    /// What the shell does at start-up: the app-wide fonts, here the OS fonts.
    fn install_fonts(app: &mut App) {
        let binding = inset_painting::PaintingBinding::instance(app);
        if !binding.has_fonts(app) {
            binding.install_fonts(app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
    }

    /// Baseline cells share the row's baseline, and the table's baseline is the first row's.
    #[test]
    fn baseline_cells_share_the_first_rows_baseline() {
        use inset_painting::{TextSpan, TextStyle};

        use crate::paragraph::RenderParagraph;

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        let paragraph = |app: &mut App, font_size: f64| {
            let text = TextSpan::new()
                .text("Hg")
                .style(TextStyle::new().font_size(font_size))
                .into_span();
            RenderParagraph::new(app, text, TextDirection::Ltr, None).as_box()
        };
        let small = paragraph(&mut app, 10.0);
        let large = paragraph(&mut app, 30.0);
        let table = RenderTable::new(&mut app, TextDirection::Ltr);
        table.set_default_vertical_alignment(&mut app, TableCellVerticalAlignment::Baseline);
        table.set_text_baseline(&mut app, Some(TextBaseline::Alphabetic));
        table.set_children(&mut app, Some(vec![vec![small, large]]));
        layout(
            &mut app,
            table.as_box(),
            BoxConstraints::tight_for(Some(400.0), None),
        );

        let baseline_of = |app: &mut App, child: AnyRenderBox| {
            child.box_parent_data(app).offset.dy()
                + child
                    .get_distance_to_baseline(app, TextBaseline::Alphabetic, true)
                    .expect("text has a baseline")
        };
        let small_baseline = baseline_of(&mut app, small);
        let large_baseline = baseline_of(&mut app, large);
        assert!((small_baseline - large_baseline).abs() < 1e-9);
        assert!(
            small.box_parent_data(&app).offset.dy() > 0.0,
            "the small text is lowered"
        );
        assert_eq!(
            table
                .as_box()
                .get_distance_to_baseline(&mut app, TextBaseline::Alphabetic, true),
            Some(large_baseline)
        );
    }
}
