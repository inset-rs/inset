//! `TableView` (`controls/dev/TableView`: `TableView.xaml`, `TableView.cpp`, `TableView_Layout.cpp`, `TableView_Columns.cpp`, `TableView_Selection.cpp`, `TableView_Sort.cpp` and `TableView_Keyboard.cpp`): items an application gives as a [`TableViewSource`], shown as rows of cells under a header band, in columns the user can resize and sort, with one row selected.
//!
//! WinUI makes the rows through an `ItemsRepeater` in a `ScrollViewer` that scrolls both ways. Here the rows are Waft's lazy `ListView`, which builds only what is in sight, inside a horizontal viewport as wide as the columns; the vertical bar stays at the table's edge. The header band and every row lay their cells out in a `TableViewCellsPanel` at the widths the table resolves into the columns while it lays itself out, so they share each column's width; an `Auto` column's width comes from the cells measured so far, and is resolved again after the frame in which a measured cell changed. The table keeps the built rows' focus nodes by index, and a later editor must keep that registry exact: keyboard navigation reads it.

mod cells_panel;
mod column;
mod keyboard;
mod layout;
mod row;
mod selection;
mod sort;
mod source;

pub use column::{
    TableViewCellTemplate, TableViewColumn, TableViewFrozenEdge, TableViewSortCycle, Visibility,
};
pub use selection::TableViewSelection;
pub use sort::TableViewSort;
pub use source::{SortDirection, TableViewSource};

use cells_panel::{CellsPanelColumn, CellsPanelPlacement, ColumnMeasurements, TableViewCellsPanel};
use column::{ColumnChange, TableViewCellStyle};
use row::{TableViewRow, TableViewRowEvents, cell_wrapper, row_background};

use crate::FocusState;
use crate::*;
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use waft_embedder::{Clip, Color, FontWeight, Offset, Radius, Rect};
use waft_foundation::{App, Handle, HandleId, ListenableObject, Listener, OptionRetained, Ref};
use waft_painting::{AlignmentGeometry, Axis, BorderRadius, EdgeInsetsGeometry};
use waft_scheduler::FrameCallback;
use waft_services::{HardwareKeyboard, KeyEvent, LogicalKeyboardKey};
use waft_widgets::*;

/// XAML `TableViewHeadersVisibility`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TableViewHeadersVisibility {
    None,
    #[default]
    Column,
}

/// XAML `TableViewGridLinesVisibility`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TableViewGridLinesVisibility {
    #[default]
    All,
    Horizontal,
    None,
    Vertical,
}

impl TableViewGridLinesVisibility {
    /// `WantsHorizontalLines`.
    fn horizontal(self) -> bool {
        matches!(self, Self::All | Self::Horizontal)
    }

    /// `WantsVerticalLines`.
    fn vertical(self) -> bool {
        matches!(self, Self::All | Self::Vertical)
    }
}

/// XAML `TableViewDensity`: the row height and the built-in cell and header padding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TableViewDensity {
    Compact,
    #[default]
    Standard,
    Comfortable,
}

/// XAML `TableViewSelectionMode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TableViewSelectionMode {
    None,
    #[default]
    Single,
}

/// XAML `SelectionChangedEventArgs` of a `TableView`: the items, as their indices in the source's items, that left and joined the selection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TableViewSelectionChangedEventArgs {
    /// `RemovedItems`.
    pub removed_items: Vec<usize>,
    /// `AddedItems`.
    pub added_items: Vec<usize>,
}

/// XAML `TableViewSortingEventArgs`, raised before a sort is applied.
pub struct TableViewSortingEventArgs<T> {
    column: Option<Handle<TableViewColumn<T>>>,
    direction: SortDirection,
    cancel: Cell<bool>,
}

impl<T> TableViewSortingEventArgs<T> {
    /// `Column`: the column to sort; `None` when every sort is being cleared.
    pub fn column(&self) -> Option<Handle<TableViewColumn<T>>> {
        self.column
    }

    /// `Direction`: the direction about to be applied.
    pub fn direction(&self) -> SortDirection {
        self.direction
    }

    /// `Cancel`: whether the table leaves its sort as it is, the application ordering the rows itself.
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }

    /// Sets `Cancel`.
    pub fn set_cancel(&self, cancel: bool) {
        self.cancel.set(cancel);
    }
}

impl<T> fmt::Debug for TableViewSortingEventArgs<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TableViewSortingEventArgs")
            .field("column", &self.column)
            .field("direction", &self.direction)
            .field("cancel", &self.cancel.get())
            .finish()
    }
}

/// XAML `TableViewSortedEventArgs`, raised after a sort was applied.
pub struct TableViewSortedEventArgs<T> {
    column: Option<Handle<TableViewColumn<T>>>,
    direction: SortDirection,
}

impl<T> TableViewSortedEventArgs<T> {
    /// `Column`: the column sorted; `None` when every sort was cleared.
    pub fn column(&self) -> Option<Handle<TableViewColumn<T>>> {
        self.column
    }

    /// `Direction`: the direction now applied to the column.
    pub fn direction(&self) -> SortDirection {
        self.direction
    }
}

impl<T> fmt::Debug for TableViewSortedEventArgs<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TableViewSortedEventArgs")
            .field("column", &self.column)
            .field("direction", &self.direction)
            .finish()
    }
}

/// Builds what an empty table shows: XAML's `EmptyTemplate`.
pub type TableViewEmptyTemplate = Rc<dyn Fn(&mut App, BuildContext) -> WidgetRef>;

/// Receives XAML `SelectionChanged`.
pub type TableViewSelectionChangedHandler =
    Rc<dyn Fn(&mut App, &TableViewSelectionChangedEventArgs)>;

/// Receives XAML `Sorting`.
pub type TableViewSortingHandler<T> = Rc<dyn Fn(&mut App, &TableViewSortingEventArgs<T>)>;

/// Receives XAML `Sorted`.
pub type TableViewSortedHandler<T> = Rc<dyn Fn(&mut App, &TableViewSortedEventArgs<T>)>;

/// XAML `TableView`.
pub struct TableView<T> {
    /// `ItemsSource`: the items and the shape they are shown in.
    pub items_source: Ref<TableViewSource<T>>,
    /// `Columns`.
    pub columns: Vec<Ref<TableViewColumn<T>>>,
    /// `HeadersVisibility`.
    pub headers_visibility: TableViewHeadersVisibility,
    /// `GridLinesVisibility`.
    pub grid_lines_visibility: TableViewGridLinesVisibility,
    /// `CanUserResizeColumns`.
    pub can_user_resize_columns: bool,
    /// `CanUserSortColumns`.
    pub can_user_sort_columns: bool,
    /// `Density`.
    pub density: TableViewDensity,
    /// `RowBackground`.
    pub row_background: Option<Brush>,
    /// `AlternatingRowBackground`.
    pub alternating_row_background: Option<Brush>,
    /// `EmptyTemplate`.
    pub empty_template: Option<TableViewEmptyTemplate>,
    /// `SelectionMode`; selection needs a [`selection`](Self::selection) as well.
    pub selection_mode: TableViewSelectionMode,
    /// The selection, which holds `SelectedIndex` and `SelectedItem`; a table given none selects nothing.
    pub selection: Option<Ref<TableViewSelection>>,
    /// Sorting from code; a header sorts without it.
    pub sort: Option<Ref<TableViewSort>>,
    /// `SelectionChanged`.
    pub selection_changed: Option<TableViewSelectionChangedHandler>,
    /// `Sorting`.
    pub sorting: Option<TableViewSortingHandler<T>>,
    /// `Sorted`.
    pub sorted: Option<TableViewSortedHandler<T>>,
    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl<T> Clone for TableView<T> {
    fn clone(&self) -> Self {
        Self {
            items_source: self.items_source.clone(),
            columns: self.columns.clone(),
            headers_visibility: self.headers_visibility,
            grid_lines_visibility: self.grid_lines_visibility,
            can_user_resize_columns: self.can_user_resize_columns,
            can_user_sort_columns: self.can_user_sort_columns,
            density: self.density,
            row_background: self.row_background,
            alternating_row_background: self.alternating_row_background,
            empty_template: self.empty_template.clone(),
            selection_mode: self.selection_mode,
            selection: self.selection.clone(),
            sort: self.sort.clone(),
            selection_changed: self.selection_changed.clone(),
            sorting: self.sorting.clone(),
            sorted: self.sorted.clone(),
            key: self.key.clone(),
        }
    }
}

impl<T: 'static> TableView<T> {
    /// Creates a table of the items of `items_source` in `columns`.
    pub fn new(
        items_source: Ref<TableViewSource<T>>,
        columns: Vec<Ref<TableViewColumn<T>>>,
    ) -> Self {
        Self {
            items_source,
            columns,
            headers_visibility: TableViewHeadersVisibility::Column,
            grid_lines_visibility: TableViewGridLinesVisibility::All,
            can_user_resize_columns: true,
            can_user_sort_columns: true,
            density: TableViewDensity::Standard,
            row_background: None,
            alternating_row_background: None,
            empty_template: None,
            selection_mode: TableViewSelectionMode::Single,
            selection: None,
            sort: None,
            selection_changed: None,
            sorting: None,
            sorted: None,
            key: None,
        }
    }

    /// Sets the XAML `HeadersVisibility`.
    pub fn headers_visibility(mut self, value: TableViewHeadersVisibility) -> Self {
        self.headers_visibility = value;
        self
    }

    /// Sets the XAML `GridLinesVisibility`.
    pub fn grid_lines_visibility(mut self, value: TableViewGridLinesVisibility) -> Self {
        self.grid_lines_visibility = value;
        self
    }

    /// Sets the XAML `CanUserResizeColumns`.
    pub fn can_user_resize_columns(mut self, value: bool) -> Self {
        self.can_user_resize_columns = value;
        self
    }

    /// Sets the XAML `CanUserSortColumns`.
    pub fn can_user_sort_columns(mut self, value: bool) -> Self {
        self.can_user_sort_columns = value;
        self
    }

    /// Sets the XAML `Density`.
    pub fn density(mut self, value: TableViewDensity) -> Self {
        self.density = value;
        self
    }

    /// Sets the XAML `RowBackground`.
    pub fn row_background(mut self, value: Brush) -> Self {
        self.row_background = Some(value);
        self
    }

    /// Sets the XAML `AlternatingRowBackground`.
    pub fn alternating_row_background(mut self, value: Brush) -> Self {
        self.alternating_row_background = Some(value);
        self
    }

    /// Sets the XAML `EmptyTemplate`.
    pub fn empty_template(
        mut self,
        template: impl Fn(&mut App, BuildContext) -> WidgetRef + 'static,
    ) -> Self {
        self.empty_template = Some(Rc::new(template));
        self
    }

    /// Sets the XAML `SelectionMode`.
    pub fn selection_mode(mut self, value: TableViewSelectionMode) -> Self {
        self.selection_mode = value;
        self
    }

    /// Gives the table the application's selection.
    pub fn selection(mut self, value: Ref<TableViewSelection>) -> Self {
        self.selection = Some(value);
        self
    }

    /// Gives the table the application's sorting from code.
    pub fn sort(mut self, value: Ref<TableViewSort>) -> Self {
        self.sort = Some(value);
        self
    }

    /// Sets the XAML `SelectionChanged` handler.
    pub fn selection_changed(
        mut self,
        handler: impl Fn(&mut App, &TableViewSelectionChangedEventArgs) + 'static,
    ) -> Self {
        self.selection_changed = Some(Rc::new(handler));
        self
    }

    /// Sets the XAML `Sorting` handler.
    pub fn sorting(
        mut self,
        handler: impl Fn(&mut App, &TableViewSortingEventArgs<T>) + 'static,
    ) -> Self {
        self.sorting = Some(Rc::new(handler));
        self
    }

    /// Sets the XAML `Sorted` handler.
    pub fn sorted(
        mut self,
        handler: impl Fn(&mut App, &TableViewSortedEventArgs<T>) + 'static,
    ) -> Self {
        self.sorted = Some(Rc::new(handler));
        self
    }

    /// Sets the identity retained across rebuilds.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

impl<T> fmt::Debug for TableView<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TableView")
            .field("columns", &self.columns.len())
            .field("headers_visibility", &self.headers_visibility)
            .field("grid_lines_visibility", &self.grid_lines_visibility)
            .field("density", &self.density)
            .field("selection_mode", &self.selection_mode)
            .finish_non_exhaustive()
    }
}

impl<T: 'static> StatefulWidget for TableView<T> {
    type State = TableViewState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> TableViewState<T> {
        TableViewState {
            state: StateData::new(),
            vertical: None,
            horizontal: None,
            measurements: None,
            resolve_queued: false,
            viewport: None,
            scale: 1.0,
            source: None,
            selection: None,
            sort: None,
            columns: Vec::new(),
            sorted_column: None,
            source_sort_token: None,
            grippers: HashMap::new(),
            active_resize: None,
            rows: HashMap::new(),
            focus_epoch: 0,
            body_key: GlobalKey::new(),
        }
    }
}

/// `ColumnResizeDragState`: the drag in flight, which Escape can cancel.
struct ColumnResizeDrag<T> {
    column: Handle<TableViewColumn<T>>,
    /// `startValue`: the column's width when the drag began.
    start_value: f64,
    /// `startWidth`: its `Width` as authored, which a canceled drag restores.
    start_width: GridLength,
    /// `didWrite`: the drag has written `Width`.
    did_write: bool,
}

impl<T> Clone for ColumnResizeDrag<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ColumnResizeDrag<T> {}

/// The table's view of its source, selection, sort and columns, its scrolling, the grippers and the built rows.
pub struct TableViewState<T: 'static> {
    state: StateData<TableView<T>>,
    vertical: Option<Ref<ScrollViewportController>>,
    horizontal: Option<Ref<ScrollViewportController>>,
    measurements: Option<Rc<ColumnMeasurements>>,
    /// A width resolution is waiting for the end of the frame.
    resolve_queued: bool,
    /// The body's width at the last layout.
    viewport: Option<f64>,
    /// The device pixel ratio widths are rounded to.
    scale: f64,
    source: Option<(Ref<TableViewSource<T>>, Listener)>,
    selection: Option<(Ref<TableViewSelection>, Listener)>,
    sort: Option<Ref<TableViewSort>>,
    /// The columns listened to, with their listeners.
    columns: Vec<(Ref<TableViewColumn<T>>, Listener)>,
    /// `m_sortedColumns`: the one column sorted, if any.
    sorted_column: Option<Handle<TableViewColumn<T>>>,
    /// `m_tableViewSourceSort.AxisToken`: the source axis the table installed.
    source_sort_token: Option<HandleId>,
    /// Each column's gripper gesture.
    grippers: HashMap<HandleId, Ref<ResizeGripperDrag>>,
    /// `m_activeColumnResizeDrag`.
    active_resize: Option<ColumnResizeDrag<T>>,
    /// The built rows' focus nodes by index.
    rows: HashMap<usize, Handle<FocusNode>>,
    /// Counts focus requests, so a request waiting for its row to be built gives way to a later one.
    focus_epoch: u64,
    /// The body viewport, whose rectangle keyboard navigation scrolls rows into.
    body_key: GlobalKey,
}

/// What every row of one build shares.
struct RowLayout {
    columns: Vec<CellsPanelColumn>,
    frozen: usize,
    vertical_lines: bool,
    horizontal_lines: bool,
    min_height: f64,
    cell_style: TableViewCellStyle,
    row_background: Option<Brush>,
    alternating_row_background: Option<Brush>,
}

/// The density's metrics: `GetDensityRowMinHeight`, `GetDensityCellPadding` and `GetDensityHeaderCellPadding`.
fn density_metrics(r: &ThemeResources, density: TableViewDensity) -> (f64, [f64; 4], [f64; 4]) {
    match density {
        TableViewDensity::Compact => (
            r.table_view_row_min_height_compact,
            r.table_view_cell_padding_compact,
            r.table_view_header_cell_padding_compact,
        ),
        TableViewDensity::Standard => (
            r.table_view_row_min_height,
            r.table_view_cell_padding,
            r.table_view_header_cell_padding,
        ),
        TableViewDensity::Comfortable => (
            r.table_view_row_min_height_comfortable,
            r.table_view_cell_padding_comfortable,
            r.table_view_header_cell_padding_comfortable,
        ),
    }
}

// MARK: Source and columns

impl<T: 'static> TableViewState<T> {
    fn id_of(self: Handle<Self>) -> HandleId {
        self.id()
    }

    fn source_handle(self: Handle<Self>, app: &App) -> Handle<TableViewSource<T>> {
        self.widget(app).items_source.handle()
    }

    fn row_count(self: Handle<Self>, app: &App) -> usize {
        self.source_handle(app).view(app).len()
    }

    fn selection_handle(self: Handle<Self>, app: &App) -> Option<Handle<TableViewSelection>> {
        self.widget(app).selection.handle()
    }

    /// The columns the table owns, in order; a column another table holds is skipped, as `IsColumnOwnedBy` skips it.
    fn owned_columns(self: Handle<Self>, app: &App) -> Vec<Handle<TableViewColumn<T>>> {
        let id = self.id_of();
        self.widget(app)
            .columns
            .iter()
            .map(|column| column.handle())
            .filter(|column| column.owner(app) == Some(id))
            .collect()
    }

    fn alive(self: Handle<Self>, app: &App) -> bool {
        app.contains(self) && self.mounted(app)
    }

    /// Runs `work` once the frame is over, if the table is still mounted.
    fn after_frame(self: Handle<Self>, app: &mut App, work: impl Fn(&mut App) + 'static) {
        let view = View::maybe_id_of(app, self.context(app));
        after_next_frame(
            app,
            view,
            FrameCallback::new(move |app, _| {
                if self.alive(app) {
                    work(app);
                }
            }),
        );
    }

    fn sync(self: Handle<Self>, app: &mut App) {
        self.sync_source(app);
        self.sync_columns(app);
        self.sync_selection(app);
        self.sync_sort(app);
    }

    /// `AdoptItemsSource`: listens to the source and owns it; a different source drops the sort the old one carried (`ResetSortStateForNewItemsSource`).
    fn sync_source(self: Handle<Self>, app: &mut App) {
        let source = self.widget(app).items_source.clone();
        let held = app.get(self).source.as_ref().map(|(held, _)| held.handle());
        if held == Some(source.handle()) {
            return;
        }
        if let Some((old, listener)) = app.get_mut(self).source.take() {
            old.remove_listener(app, &listener);
            old.set_owner(app, None);
            self.reset_sort_state_for_new_items_source(app);
        }
        source.set_owner(app, Some(self.id_of()));
        let listener = Listener::new(move |app| self.source_changed(app));
        source.add_listener(app, listener.clone());
        app.get_mut(self).source = Some((source, listener));
        if let Some(selection) = self.selection_handle(app) {
            let count = self.row_count(app);
            selection.set_source(app, count);
        }
    }

    /// The source changed its view: the selection follows the rows the change moved, or starts over on a rebuilt view (`SelectionNode::OnSourceListChanged`), and the rows are rebuilt.
    fn source_changed(self: Handle<Self>, app: &mut App) {
        if !self.alive(app) {
            return;
        }
        if let Some(selection) = self.selection_handle(app) {
            let changes = self.source_handle(app).changes(app).to_vec();
            let count = self.row_count(app);
            selection.source_changed(app, &changes, count);
        }
        self.set_state(app, |_| {});
    }

    /// `TrackColumnsFromVector` and `DetachAllColumnOwners`: the table owns and listens to its columns; a column that left gives up its sort (`PurgeColumnFromSortState`).
    fn sync_columns(self: Handle<Self>, app: &mut App) {
        let id = self.id_of();
        let columns = self.widget(app).columns.clone();
        let held = std::mem::take(&mut app.get_mut(self).columns);
        let mut kept = Vec::with_capacity(columns.len());
        let mut purged = false;
        for (column, listener) in held {
            if columns
                .iter()
                .any(|current| current.handle() == column.handle())
            {
                kept.push((column, listener));
                continue;
            }
            column.remove_listener(app, &listener);
            column.set_owner(app, None);
            column.set_sort_state(app, SortDirection::None);
            if app.get(self).sorted_column == Some(column.handle()) {
                app.get_mut(self).sorted_column = None;
                purged = true;
            }
            if let Some(gripper) = app.get_mut(self).grippers.remove(&column.handle().id()) {
                gripper.dispose(app);
            }
        }
        for column in columns {
            if kept
                .iter()
                .any(|(held, _)| held.handle() == column.handle())
            {
                continue;
            }
            if !column.set_owner(app, Some(id)) {
                continue;
            }
            let handle = column.handle();
            let listener = Listener::new(move |app| self.column_changed(app, handle));
            column.add_listener(app, listener.clone());
            kept.push((column, listener));
        }
        app.get_mut(self).columns = kept;
        if purged {
            // `QueueClearSortAfterColumnRemoval`: the reshape waits until the columns settle.
            self.after_frame(app, move |app| self.recompute_sort_and_raise(app, None));
        }
    }

    /// `TableViewColumn::OnPropertyChanged`'s calls into the table.
    fn column_changed(self: Handle<Self>, app: &mut App, column: Handle<TableViewColumn<T>>) {
        if !self.alive(app) || !app.contains(column) {
            return;
        }
        match column.change(app) {
            ColumnChange::None | ColumnChange::Other => return,
            // `OnColumnCanSortChanged`: a column that opted out drops the sort it holds.
            ColumnChange::CanSort => {
                if !column.can_sort(app) && column.sort_direction(app) != SortDirection::None {
                    self.sort_by_column(app, column, SortDirection::None);
                }
            }
            // The order the column is sorted in belongs to the comparer it was sorted with.
            ColumnChange::CustomSortComparer => {
                let direction = column.sort_direction(app);
                if direction != SortDirection::None {
                    self.sort_by_column(app, column, SortDirection::None);
                    self.sort_by_column(app, column, direction);
                }
            }
            _ => {}
        }
        self.set_state(app, |_| {});
    }
}

// MARK: Resizing a column

impl<T: 'static> TableViewState<T> {
    /// The gripper gesture of `column`, made the first time its header shows one.
    fn gripper(
        self: Handle<Self>,
        app: &mut App,
        column: Handle<TableViewColumn<T>>,
    ) -> Ref<ResizeGripperDrag> {
        if let Some(gripper) = app.get(self).grippers.get(&column.id()) {
            return gripper.clone();
        }
        let gripper = ResizeGripperDrag::new(
            app,
            Axis::Horizontal,
            ResizeGripperEvents {
                drag_started: Rc::new(move |app| self.resize_started(app, column)),
                drag_delta: Rc::new(move |app, _, total| self.resize_delta(app, column, total)),
                drag_completed: Rc::new(move |app, _, canceled| {
                    self.resize_completed(app, column, canceled)
                }),
            },
        );
        app.get_mut(self)
            .grippers
            .insert(column.id(), gripper.clone());
        gripper
    }

    /// `DragStarted`: one resize at a time; the column's width and authored `Width` are the anchor.
    fn resize_started(self: Handle<Self>, app: &mut App, column: Handle<TableViewColumn<T>>) {
        if !app.contains(self) || !app.contains(column) {
            return;
        }
        if let Some(active) = app.get(self).active_resize
            && active.column != column
            && let Some(other) = app.get(self).grippers.get(&active.column.id()).cloned()
            && other.is_dragging(app)
        {
            if let Some(own) = app.get(self).grippers.get(&column.id()).cloned() {
                own.end_drag(app, true);
            }
            return;
        }
        app.get_mut(self).active_resize = Some(ColumnResizeDrag {
            column,
            start_value: column.actual_width(app),
            start_width: column.width(app),
            did_write: false,
        });
    }

    /// `DragDelta`: the anchor plus the offset, clamped to the column's bounds, written as a pixel `Width`.
    fn resize_delta(
        self: Handle<Self>,
        app: &mut App,
        column: Handle<TableViewColumn<T>>,
        total: f64,
    ) {
        if !app.contains(self) || !app.contains(column) {
            return;
        }
        let Some(active) = app
            .get(self)
            .active_resize
            .filter(|active| active.column == column)
        else {
            return;
        };
        let min = column.min_width(app);
        let lo = if min.is_finite() && min >= 0.0 {
            min
        } else {
            0.0
        };
        let max = column.max_width(app);
        let hi = if max.is_finite() && max >= 0.0 {
            lo.max(max)
        } else {
            f64::INFINITY
        };
        let next = (active.start_value + total).clamp(lo, hi);
        let current = column.width(app);
        if current.unit == GridUnitType::Pixel && (current.value - next).abs() < 0.0001 {
            return;
        }
        column.set_width(app, GridLength::pixel(next));
        if let Some(active) = &mut app.get_mut(self).active_resize {
            active.did_write = true;
        }
    }

    /// `DragCompleted`: a canceled drag restores the authored `Width`.
    fn resize_completed(
        self: Handle<Self>,
        app: &mut App,
        column: Handle<TableViewColumn<T>>,
        canceled: bool,
    ) {
        if !app.contains(self) {
            return;
        }
        let Some(active) = app
            .get(self)
            .active_resize
            .filter(|active| active.column == column)
        else {
            return;
        };
        app.get_mut(self).active_resize = None;
        if canceled && active.did_write && app.contains(column) {
            column.set_width(app, active.start_width);
        }
    }

    /// `CancelColumnResizeDrag`: Escape ends the drag in flight. Answers whether there was one.
    fn cancel_column_resize_drag(self: Handle<Self>, app: &mut App) -> bool {
        let Some(active) = app.get(self).active_resize else {
            return false;
        };
        match app.get(self).grippers.get(&active.column.id()).cloned() {
            Some(gripper) if gripper.is_dragging(app) => gripper.end_drag(app, true),
            _ => app.get_mut(self).active_resize = None,
        }
        true
    }

    /// `TryHandleHeaderColumnResizeKey`: Left and Right resize the column whose header has the focus, Shift the larger step; Alt is reserved.
    fn header_key(
        self: Handle<Self>,
        app: &mut App,
        column: Handle<TableViewColumn<T>>,
        event: &KeyEvent,
    ) -> KeyEventResult {
        if matches!(event, KeyEvent::Up(_)) {
            return KeyEventResult::Ignored;
        }
        let key = event.logical_key();
        if key != LogicalKeyboardKey::ARROW_LEFT && key != LogicalKeyboardKey::ARROW_RIGHT {
            return KeyEventResult::Ignored;
        }
        let keyboard = HardwareKeyboard::instance(app);
        if keyboard.is_alt_pressed(app) {
            return KeyEventResult::Ignored;
        }
        if !self.widget(app).can_user_resize_columns || !column.can_resize(app) {
            return KeyEventResult::Ignored;
        }
        let shift = keyboard.is_shift_pressed(app);
        let gripper = self.gripper(app, column);
        if gripper.try_keyboard_step(app, key, false, shift) {
            KeyEventResult::Handled
        } else {
            KeyEventResult::Ignored
        }
    }
}

// MARK: Building

impl<T: 'static> TableViewState<T> {
    fn vertical(self: Handle<Self>, app: &App) -> Handle<ScrollViewportController> {
        app.get(self)
            .vertical
            .handle()
            .expect("made in `init_state`")
    }

    fn horizontal(self: Handle<Self>, app: &App) -> Handle<ScrollViewportController> {
        app.get(self)
            .horizontal
            .handle()
            .expect("made in `init_state`")
    }

    /// The template inside the border, at the width the table is given: the widths are resolved here, then `PART_HeaderRow` over `PART_BodyScroller`.
    fn template(self: Handle<Self>, app: &mut App, context: BuildContext, width: f64) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let widget = self.widget(app).clone();
        let scale = MediaQuery::maybe_device_pixel_ratio_of(app, context).unwrap_or(1.0);
        {
            let state = app.get_mut(self);
            state.viewport = Some(width).filter(|width| width.is_finite());
            state.scale = scale;
        }
        let columns = self.owned_columns(app);
        self.resolve_column_widths(app, &columns);
        let (min_height, cell_padding, header_padding) = density_metrics(&r, widget.density);
        let panel_columns: Vec<CellsPanelColumn> = columns
            .iter()
            .map(|&column| CellsPanelColumn {
                id: column.id(),
                auto: column.width(app).unit == GridUnitType::Auto,
                width: column.actual_width(app),
                visible: column.visibility(app) == Visibility::Visible,
            })
            .collect();
        // Only the contiguous prefix from the first column is frozen.
        let frozen = columns
            .iter()
            .take_while(|column| column.frozen_edge(app) == TableViewFrozenEdge::Leading)
            .count();
        let content_width: f64 = panel_columns
            .iter()
            .filter(|column| column.visible)
            .map(|column| column.width)
            .sum();
        let layout = Rc::new(RowLayout {
            columns: panel_columns,
            frozen,
            vertical_lines: widget.grid_lines_visibility.vertical(),
            horizontal_lines: widget.grid_lines_visibility.horizontal(),
            min_height,
            cell_style: TableViewCellStyle {
                padding: cell_padding,
                font_size: r.table_view_cell_font_size,
            },
            row_background: widget.row_background,
            alternating_row_background: widget.alternating_row_background,
        });
        let mut children = Vec::new();
        if widget.headers_visibility == TableViewHeadersVisibility::Column {
            children.push(self.header_row(app, &r, &layout, &columns, header_padding));
        }
        children.push(
            Expanded::new(self.body(app, context, &layout, content_width.max(width))).into_widget(),
        );
        Column::new().children(children).into_widget()
    }

    /// PART_HeaderRow > PART_HeaderScroller > PART_HeaderHost.
    fn header_row(
        self: Handle<Self>,
        app: &mut App,
        r: &ThemeResources,
        layout: &RowLayout,
        columns: &[Handle<TableViewColumn<T>>],
        header_padding: [f64; 4],
    ) -> WidgetRef {
        let cells: Vec<WidgetRef> = columns
            .iter()
            .map(|&column| self.header_cell(app, r, layout, column, header_padding))
            .collect();
        let host = TableViewCellsPanel {
            columns: layout.columns.clone(),
            frozen: layout.frozen,
            placement: CellsPanelPlacement::Header,
            horizontal: self.horizontal(app).native_controller(app),
            measurements: self.measurements(app),
            children: cells,
        };
        ControlBorder::new(
            Brush::Solid(r.tabular_surface_header_background_brush),
            Brush::Solid(r.tabular_surface_grid_line_brush),
        )
        .border_thickness_ltrb(if layout.horizontal_lines {
            [0.0, 0.0, 0.0, 1.0]
        } else {
            [0.0; 4]
        })
        .child(host)
        .into_widget()
    }

    /// One header cell (`RebuildHeaders`): the header content, the vertical grid line, the sort chevron and the gripper, on the cell's trailing edge in that order.
    fn header_cell(
        self: Handle<Self>,
        app: &mut App,
        r: &ThemeResources,
        layout: &RowLayout,
        column: Handle<TableViewColumn<T>>,
        header_padding: [f64; 4],
    ) -> WidgetRef {
        let widget = self.widget(app).clone();
        let [left, top, right, bottom] = header_padding;
        let content = Padding::new(EdgeInsetsGeometry::from_ltrb(left, top, right, bottom)).child(
            Align::new()
                .alignment(AlignmentGeometry::CENTER_LEFT)
                .child(
                    Text::new(column.header(app).to_string())
                        .style(control_text_style(
                            r,
                            r.table_view_header_font_size,
                            FontWeight::W600,
                            r.tabular_surface_row_foreground_brush,
                        ))
                        .max_lines(1)
                        .soft_wrap(false),
                ),
        );
        let mut children = vec![content.into_widget()];
        if layout.vertical_lines {
            // TableViewHeaderGridLine
            children.push(
                Positioned::new(
                    IgnorePointer::new().child(ColoredBox::new(r.tabular_surface_grid_line_brush)),
                )
                .right(0.0)
                .top(0.0)
                .bottom(0.0)
                .width(1.0)
                .into_widget(),
            );
        }
        let sortable = widget.can_user_sort_columns && column.can_sort(app);
        if sortable {
            let direction = match column.sort_direction(app) {
                SortDirection::Ascending => SortIndicatorDirection::Ascending,
                SortDirection::Descending => SortIndicatorDirection::Descending,
                SortDirection::None => SortIndicatorDirection::None,
            };
            // TableViewSortIndicator in its host, centred on the trailing edge.
            children.push(
                Positioned::new(Center::new().child(SortIndicator::new(direction)))
                    .right(0.0)
                    .top(0.0)
                    .bottom(0.0)
                    .into_widget(),
            );
        }
        let resizable = widget.can_user_resize_columns && column.can_resize(app);
        if resizable {
            let gripper_width = Some(r.table_view_resize_gripper_width)
                .filter(|width| width.is_finite() && *width > 0.0)
                .unwrap_or(8.0);
            let gripper = self.gripper(app, column);
            children.push(
                Positioned::new(ResizeGripper::new(gripper))
                    .right(0.0)
                    .top(0.0)
                    .bottom(0.0)
                    .width(gripper_width)
                    .into_widget(),
            );
        }
        let mut cell: WidgetRef =
            ConstrainedBox::new(BoxConstraints::new().min_height(layout.min_height))
                .child(ColoredBox::new(Color::new(0)).child(Stack::new().children(children)))
                .into_widget();
        if sortable {
            cell = GestureDetector::new()
                .behavior(waft_rendering::HitTestBehavior::Opaque)
                .on_tap(Listener::new(move |app| {
                    if self.alive(app) && app.contains(column) {
                        self.toggle_sort_direction(app, column);
                    }
                }))
                .child(cell)
                .into_widget();
        }
        if let Some(tip) = column.header_tool_tip(app).filter(|tip| !tip.is_empty()) {
            cell =
                ToolTipService::new(cell, ToolTip::new(Text::new(tip.to_string()))).into_widget();
        }
        HeaderCell {
            child: cell,
            is_tab_stop: resizable,
            key_handler: Rc::new(move |app, event| {
                if self.alive(app) && app.contains(column) {
                    self.header_key(app, column, event)
                } else {
                    KeyEventResult::Ignored
                }
            }),
        }
        .into_widget()
    }

    /// PART_BodyScroller > PART_BodyContent: PART_RowsRepeater, or PART_EmptyStatePresenter when there are no rows and an `EmptyTemplate`.
    fn body(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        layout: &Rc<RowLayout>,
        rows_width: f64,
    ) -> WidgetRef {
        let widget = self.widget(app).clone();
        let count = self.row_count(app);
        if count == 0
            && let Some(template) = widget.empty_template.clone()
        {
            return Center::new().child(template(app, context)).into_widget();
        }
        let layout = layout.clone();
        let list = ListView::builder(move |app, context, index| {
            Some(self.row(app, context, index as usize, &layout))
        })
        .item_count(count as i32)
        .build()
        .controller(self.vertical(app).native_controller(app));
        let rows = SizedBox::new().width(rows_width).child(list);
        let horizontal = ScrollBarViewport::new(
            app.get(self)
                .horizontal
                .clone()
                .expect("made in `init_state`"),
            rows,
        )
        .axis(Axis::Horizontal);
        let vertical = ScrollBarViewport::scroller(
            app.get(self)
                .vertical
                .clone()
                .expect("made in `init_state`"),
            horizontal,
        )
        .depth(1);
        KeyedSubtree::new(vertical.into_widget())
            .key(app.get(self).body_key.clone().into())
            .into_widget()
    }

    /// One `TableViewRow`: a cell wrapper per column, from `GenerateElement` for the row's item.
    fn row(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        index: usize,
        layout: &RowLayout,
    ) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let widget = self.widget(app).clone();
        let source = self.source_handle(app);
        let Some(&item) = source.view(app).get(index) else {
            return SizedBox::new().into_widget();
        };
        let item = source.shared_item(app, item);
        let columns = self.owned_columns(app);
        let cells: Vec<WidgetRef> = columns
            .iter()
            .map(|&column| {
                let element = column.generate_element(app, context, &item, layout.cell_style);
                cell_wrapper(&r, layout.vertical_lines, layout.min_height, element)
            })
            .collect();
        let panel = TableViewCellsPanel {
            columns: layout.columns.clone(),
            frozen: layout.frozen,
            placement: CellsPanelPlacement::Row,
            horizontal: self.horizontal(app).native_controller(app),
            measurements: self.measurements(app),
            children: cells,
        };
        TableViewRow {
            index,
            cells: panel.into_widget(),
            background: row_background(
                &r,
                index,
                layout.row_background,
                layout.alternating_row_background,
            ),
            horizontal_lines: layout.horizontal_lines,
            min_height: layout.min_height,
            selection: widget.selection.clone(),
            events: Rc::new(TableViewRowEvents {
                pointer_select: Rc::new(move |app, index| {
                    if self.alive(app) {
                        self.select_row_from_interaction(app, index);
                    }
                }),
                register: Rc::new(move |app, index, node| self.register_row(app, index, node)),
                unregister: Rc::new(move |app, index, node| self.unregister_row(app, index, node)),
            }),
        }
        .into_widget()
    }
}

impl<T: 'static> State for TableViewState<T> {
    type Widget = TableView<T>;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        InputDevice::initialize(app);
        let vertical = ScrollViewportController::new(app);
        let horizontal = ScrollViewportController::new(app);
        let measurements = ColumnMeasurements::new(Rc::new(move |app| self.request_resolve(app)));
        let state = app.get_mut(self);
        state.vertical = Some(vertical);
        state.horizontal = Some(horizontal);
        state.measurements = Some(measurements);
        self.sync(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &TableView<T>) {
        self.sync(app);
        // `OnCanUserSortColumnsPropertyChanged`: turning sorting off drops the sort it made.
        if old.can_user_sort_columns && !self.widget(app).can_user_sort_columns {
            self.clear_sort(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let id = self.id_of();
        if let Some((source, listener)) = app.get_mut(self).source.take() {
            source.remove_listener(app, &listener);
            source.set_owner(app, None);
        }
        if let Some((selection, listener)) = app.get_mut(self).selection.take() {
            selection.remove_listener(app, &listener);
            selection.unbind(app, id);
        }
        if let Some(sort) = app.get_mut(self).sort.take() {
            sort.unbind(app, id);
        }
        for (column, listener) in std::mem::take(&mut app.get_mut(self).columns) {
            column.remove_listener(app, &listener);
            column.set_owner(app, None);
        }
        for (_, gripper) in std::mem::take(&mut app.get_mut(self).grippers) {
            gripper.dispose(app);
        }
        for controller in [
            app.get_mut(self).vertical.take(),
            app.get_mut(self).horizontal.take(),
        ]
        .into_iter()
        .flatten()
        {
            controller.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let corner = r.control_corner_radius;
        let template = LayoutBuilder::new(move |app, context, constraints| {
            // The border's 1 pixel on each side is outside the template.
            self.template(app, context, constraints.constraints().max_width)
        });
        let [top_left, top_right, bottom_right, bottom_left] =
            corner.map(|radius| Radius::circular((radius - 1.0).max(0.0)));
        let clipped = ClipRRect::new()
            .border_radius(
                BorderRadius::only(top_left, top_right, bottom_left, bottom_right).into(),
            )
            .clip_behavior(Clip::AntiAlias)
            .child(template.into_widget());
        let border = ControlBorder::new(
            Brush::Solid(r.tabular_surface_row_background_brush),
            Brush::Solid(r.tabular_surface_grid_line_brush),
        )
        .border_thickness(1.0)
        .corner_radius_corners(corner)
        .child(DefaultTextStyle::new(
            control_text_style(
                &r,
                r.table_view_cell_font_size,
                FontWeight::NORMAL,
                r.tabular_surface_row_foreground_brush,
            ),
            clipped,
        ));
        Focus::new(border)
            .can_request_focus(false)
            .skip_traversal(true)
            .on_key_event(Rc::new(move |app, _, event| self.key(app, event)))
            .into_widget()
    }
}

/// A header cell's key handler.
type HeaderKeyHandler = Rc<dyn Fn(&mut App, &KeyEvent) -> KeyEventResult>;

/// A header cell's focus: a tab stop with the system focus visual only when it can resize, and its Left and Right keys.
#[derive(Clone)]
struct HeaderCell {
    child: WidgetRef,
    is_tab_stop: bool,
    key_handler: HeaderKeyHandler,
}

impl fmt::Debug for HeaderCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HeaderCell")
            .field("is_tab_stop", &self.is_tab_stop)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for HeaderCell {
    type State = HeaderCellState;

    fn create_state(&self) -> HeaderCellState {
        HeaderCellState {
            state: StateData::new(),
            focused: false,
        }
    }
}

struct HeaderCellState {
    state: StateData<HeaderCell>,
    focused: bool,
}

impl State for HeaderCellState {
    type Widget = HeaderCell;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let widget = self.widget(app).clone();
        let focused = app.get(self).focused && FocusState::current(app).shows_focus_visual();
        let visual = FocusVisual::new(widget.child.clone(), r.theme)
            .visible(focused && widget.is_tab_stop)
            .corner_radius(0.0);
        let handler = widget.key_handler.clone();
        Focus::new(visual)
            .can_request_focus(widget.is_tab_stop)
            .skip_traversal(!widget.is_tab_stop)
            .on_focus_change(move |app, focused| {
                self.set_state(app, |state| state.focused = focused)
            })
            .on_key_event(Rc::new(move |app, _, event| handler(app, event)))
            .into_widget()
    }
}
