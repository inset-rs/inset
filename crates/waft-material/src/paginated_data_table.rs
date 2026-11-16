//! Flutter counterpart: material_ui's `paginated_data_table.dart`.

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use waft_embedder::Color;
use waft_foundation::{App, Handle, Listenable, Listener, ValueChanged};
use waft_gestures::DragStartBehavior;
use waft_painting::Axis;
use waft_painting::{
    AlignmentGeometry, BoxDecoration, EdgeInsets, EdgeInsetsDirectional, FontWeight,
};
use waft_rendering::{BoxConstraints, CrossAxisAlignment, MainAxisAlignment};
use waft_widgets::{
    Align, BuildContext, Column, ConstrainedBox, DefaultTextStyle, Expanded, GlobalKey, IconTheme,
    IconThemeData, IntoWidget, KeyRef, LayoutBuilder, Padding, PageStorage, Row,
    ScrollControllerRef, Semantics, SingleChildScrollView, SizedBox, State, StateData,
    StatefulWidget, Text, WidgetRef, WidgetStatePropertyRef,
};

use crate::card::Card;
use crate::constants::K_MIN_INTERACTIVE_DIMENSION;
use crate::data_table::{DataCell, DataColumn, DataRow, DataTable};
use crate::data_table_source::DataTableSourceRef;
use crate::data_table_theme::DataTableSortIconBuilder;
use crate::debug::debug_check_has_material_localizations;
use crate::dropdown::{DropdownButton, DropdownButtonHideUnderline, DropdownMenuItem};
use crate::icon_button::IconButton;
use crate::icons::Icons;
use crate::ink_decoration::Ink;
use crate::material_localizations::MaterialLocalizations;
use crate::progress_indicator::CircularProgressIndicator;
use crate::theme::Theme;

/// A table that follows the Material 2 design guidelines, with pagination.
///
/// A paginated data table shows [`rows_per_page`](Self::rows_per_page) rows of data per page
/// and provides controls for showing other pages.
///
/// Data is addressed by index (0 is the top row). The [`columns`](Self::columns) define the
/// columns of the table, and the [`source`](Self::source) provides the rows (see
/// [`DataTableSource`](crate::DataTableSource)).
///
/// The [`PaginatedDataTable`] scrolls horizontally if it is too narrow for its columns.
///
/// See also:
///
///  * [`DataTable`], which is not paginated.
///  * <https://material.io/go/design-data-tables#data-tables-tables-within-cards>
#[derive(Clone)]
pub struct PaginatedDataTable {
    pub key: Option<KeyRef>,

    /// The table card's optional header.
    ///
    /// This is typically a `Text` widget, but can also be a `Row` of `TextButton`s. To show
    /// icon buttons at the top end side of the table with a header, set the
    /// [`actions`](Self::actions) property.
    ///
    /// If items in the table are selectable, then, when the selection is not empty, the header
    /// is replaced by a count of the selected items. The [`actions`](Self::actions) are still
    /// visible when items are selected.
    pub header: Option<WidgetRef>,

    /// Icon buttons to show at the top end side of the table. The [`header`](Self::header)
    /// must not be null to show the actions.
    ///
    /// Typically, the exact actions included in this list will vary based on whether any rows
    /// are selected or not.
    ///
    /// These should be size 24.0 with default padding (8.0).
    pub actions: Option<Vec<WidgetRef>>,

    /// The configuration and labels for the columns in the table.
    pub columns: Vec<DataColumn>,

    /// The current primary sort key's column.
    ///
    /// See [`DataTable::sort_column_index`].
    pub sort_column_index: Option<usize>,

    /// Whether the column mentioned in [`sort_column_index`](Self::sort_column_index), if any,
    /// is sorted in ascending order.
    ///
    /// See [`DataTable::sort_ascending`] for details.
    pub sort_ascending: bool,

    /// Invoked when the user selects or unselects every row, using the checkbox in the heading
    /// row.
    ///
    /// See [`DataTable::on_select_all`].
    pub on_select_all: Option<ValueChanged<Option<bool>>>,

    /// The minimum height of each row (excluding the row that contains column headings).
    ///
    /// This value is optional and defaults to `K_MIN_INTERACTIVE_DIMENSION` if not specified.
    pub data_row_min_height: Option<f64>,

    /// The maximum height of each row (excluding the row that contains column headings).
    ///
    /// This value is optional and defaults to `K_MIN_INTERACTIVE_DIMENSION` if not specified.
    pub data_row_max_height: Option<f64>,

    /// The height of the heading row.
    ///
    /// This value is optional and defaults to 56.0 if not specified.
    pub heading_row_height: f64,

    /// The horizontal margin between the edges of the table and the content in the first and
    /// last cells of each row.
    ///
    /// When a checkbox is displayed, it is also the margin between the checkbox the content in
    /// the first data column.
    ///
    /// This value defaults to 24.0 to adhere to the Material Design specifications.
    pub horizontal_margin: f64,

    /// The horizontal margin between the contents of each data column.
    ///
    /// This value defaults to 56.0 to adhere to the Material Design specifications.
    pub column_spacing: f64,

    /// Whether the widget should display checkboxes for selectable rows.
    ///
    /// See [`DataTable::show_checkbox_column`].
    pub show_checkbox_column: bool,

    /// Whether to display buttons to go to the first and last pages.
    ///
    /// Defaults to false.
    pub show_first_last_buttons: bool,

    /// The index of the first row to display when the widget is first created.
    pub initial_first_row_index: Option<usize>,

    /// The thickness of the divider between rows.
    ///
    /// This value is optional and defaults to 1.0 if not specified.
    pub divider_thickness: Option<f64>,

    /// Invoked when the user switches to another page.
    ///
    /// The value is the index of the first row on the currently displayed page.
    pub on_page_changed: Option<ValueChanged<usize>>,

    /// The number of rows to show on each page.
    ///
    /// See also:
    ///
    ///  * [`on_rows_per_page_changed`](Self::on_rows_per_page_changed)
    ///  * [`available_rows_per_page`](Self::available_rows_per_page)
    pub rows_per_page: usize,

    /// The options to offer for the rowsPerPage.
    ///
    /// The current [`rows_per_page`](Self::rows_per_page) must be a value in this list.
    ///
    /// The values in this list should be sorted in ascending order.
    pub available_rows_per_page: Vec<usize>,

    /// Invoked when the user selects a different number of rows per page.
    ///
    /// If this is null, then the value given by [`rows_per_page`](Self::rows_per_page) will be
    /// used and no affordance will be provided to change the value.
    pub on_rows_per_page_changed: Option<ValueChanged<Option<usize>>>,

    /// The data source which provides data to show in each row.
    ///
    /// This object should generally have a lifetime longer than the [`PaginatedDataTable`]
    /// widget itself; it should be reused each time the [`PaginatedDataTable`] constructor is
    /// called.
    pub source: DataTableSourceRef,

    /// {@macro flutter.widgets.scrollable.dragStartBehavior}
    pub drag_start_behavior: DragStartBehavior,

    /// Horizontal margin around the checkbox, if it is displayed.
    pub checkbox_horizontal_margin: Option<f64>,

    /// Defines the color of the arrow heads in the footer.
    pub arrow_head_color: Option<Color>,

    /// Controls the scrolling of the horizontal table, which is only needed when the
    /// [`PaginatedDataTable`] is too narrow.
    ///
    /// If null, a scroll controller is created internally by the scroll view.
    pub controller: Option<ScrollControllerRef>,

    /// Whether this is the primary scroll view associated with the parent
    /// `PrimaryScrollController`.
    ///
    /// When true, the scroll view will use the ambient primary scroll controller.
    pub primary: Option<bool>,

    /// The background color for the heading row.
    ///
    /// See [`DataTable::heading_row_color`].
    pub heading_row_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// Whether to display empty rows when the data source runs out of rows on the last page.
    ///
    /// Defaults to true.
    pub show_empty_rows: bool,

    /// Overrides the default sort indicator.
    ///
    /// See [`DataTable::sort_icon_builder`].
    pub sort_icon_builder: Option<DataTableSortIconBuilder>,
}

impl PaginatedDataTable {
    /// The default value for [`rows_per_page`](Self::rows_per_page).
    ///
    /// Defaults to 10, arbitrarily.
    pub const DEFAULT_ROWS_PER_PAGE: usize = 10;

    /// Creates a widget describing a paginated [`DataTable`] on a [`Card`]; Dart's optional
    /// named arguments are the setters.
    ///
    /// The [`available_rows_per_page`](Self::available_rows_per_page) list must have at least
    /// one member and must all be greater than zero.
    pub fn new(columns: Vec<DataColumn>, source: DataTableSourceRef) -> PaginatedDataTable {
        PaginatedDataTable {
            key: None,
            header: None,
            actions: None,
            columns,
            sort_column_index: None,
            sort_ascending: true,
            on_select_all: None,
            data_row_min_height: None,
            data_row_max_height: None,
            heading_row_height: 56.0,
            horizontal_margin: 24.0,
            column_spacing: 56.0,
            show_checkbox_column: true,
            show_first_last_buttons: false,
            initial_first_row_index: Some(0),
            divider_thickness: None,
            on_page_changed: None,
            rows_per_page: Self::DEFAULT_ROWS_PER_PAGE,
            available_rows_per_page: vec![
                Self::DEFAULT_ROWS_PER_PAGE,
                Self::DEFAULT_ROWS_PER_PAGE * 2,
                Self::DEFAULT_ROWS_PER_PAGE * 5,
                Self::DEFAULT_ROWS_PER_PAGE * 10,
            ],
            on_rows_per_page_changed: None,
            source,
            drag_start_behavior: DragStartBehavior::Start,
            checkbox_horizontal_margin: None,
            arrow_head_color: None,
            controller: None,
            primary: None,
            heading_row_color: None,
            show_empty_rows: true,
            sort_icon_builder: None,
        }
    }

    /// Dart `PaginatedDataTable(key:)`.
    pub fn key(mut self, key: KeyRef) -> PaginatedDataTable {
        self.key = Some(key);
        self
    }

    /// Dart `PaginatedDataTable(header:)`.
    pub fn header<K>(mut self, header: impl IntoWidget<K>) -> PaginatedDataTable {
        self.header = Some(header.into_widget());
        self
    }

    /// Dart `PaginatedDataTable(actions:)`.
    pub fn actions(mut self, actions: Vec<WidgetRef>) -> PaginatedDataTable {
        self.actions = Some(actions);
        self
    }

    /// Dart `PaginatedDataTable(sortColumnIndex:)`.
    pub fn sort_column_index(mut self, sort_column_index: usize) -> PaginatedDataTable {
        self.sort_column_index = Some(sort_column_index);
        self
    }

    /// Dart `PaginatedDataTable(sortAscending:)`.
    pub fn sort_ascending(mut self, sort_ascending: bool) -> PaginatedDataTable {
        self.sort_ascending = sort_ascending;
        self
    }

    /// Dart `PaginatedDataTable(onSelectAll:)`.
    pub fn on_select_all(
        mut self,
        on_select_all: impl Fn(&mut App, Option<bool>) + 'static,
    ) -> PaginatedDataTable {
        self.on_select_all = Some(Rc::new(on_select_all));
        self
    }

    /// Dart's deprecated `PaginatedDataTable(dataRowHeight:)`: it sets
    /// [`data_row_min_height`](Self::data_row_min_height) and
    /// [`data_row_max_height`](Self::data_row_max_height) to one value.
    ///
    /// Dart asserts that the argument is not given with either of them; a fluent chain has no
    /// one call to compare, so a later setter replaces what an earlier one wrote.
    #[deprecated = "Migrate to use data_row_min_height and data_row_max_height instead. This feature was deprecated after v3.7.0-5.0.pre."]
    pub fn data_row_height(mut self, data_row_height: f64) -> PaginatedDataTable {
        self.data_row_min_height = Some(data_row_height);
        self.data_row_max_height = Some(data_row_height);
        self
    }

    /// Dart `PaginatedDataTable(dataRowMinHeight:)`.
    pub fn data_row_min_height(mut self, data_row_min_height: f64) -> PaginatedDataTable {
        self.data_row_min_height = Some(data_row_min_height);
        self
    }

    /// Dart `PaginatedDataTable(dataRowMaxHeight:)`.
    pub fn data_row_max_height(mut self, data_row_max_height: f64) -> PaginatedDataTable {
        self.data_row_max_height = Some(data_row_max_height);
        self
    }

    /// Dart's deprecated `dataRowHeight` getter: the row height, if the minimum and the maximum
    /// are the same.
    #[deprecated = "Migrate to use data_row_min_height and data_row_max_height instead. This feature was deprecated after v3.7.0-5.0.pre."]
    pub fn data_row_height_value(&self) -> Option<f64> {
        if self.data_row_min_height == self.data_row_max_height {
            self.data_row_min_height
        } else {
            None
        }
    }

    /// Dart `PaginatedDataTable(headingRowHeight:)`.
    pub fn heading_row_height(mut self, heading_row_height: f64) -> PaginatedDataTable {
        self.heading_row_height = heading_row_height;
        self
    }

    /// Dart `PaginatedDataTable(horizontalMargin:)`.
    pub fn horizontal_margin(mut self, horizontal_margin: f64) -> PaginatedDataTable {
        self.horizontal_margin = horizontal_margin;
        self
    }

    /// Dart `PaginatedDataTable(columnSpacing:)`.
    pub fn column_spacing(mut self, column_spacing: f64) -> PaginatedDataTable {
        self.column_spacing = column_spacing;
        self
    }

    /// Dart `PaginatedDataTable(showCheckboxColumn:)`.
    pub fn show_checkbox_column(mut self, show_checkbox_column: bool) -> PaginatedDataTable {
        self.show_checkbox_column = show_checkbox_column;
        self
    }

    /// Dart `PaginatedDataTable(showFirstLastButtons:)`.
    pub fn show_first_last_buttons(mut self, show_first_last_buttons: bool) -> PaginatedDataTable {
        self.show_first_last_buttons = show_first_last_buttons;
        self
    }

    /// Dart `PaginatedDataTable(initialFirstRowIndex:)`; `None` is Dart's `null`.
    pub fn initial_first_row_index(
        mut self,
        initial_first_row_index: Option<usize>,
    ) -> PaginatedDataTable {
        self.initial_first_row_index = initial_first_row_index;
        self
    }

    /// Dart `PaginatedDataTable(dividerThickness:)`.
    pub fn divider_thickness(mut self, divider_thickness: f64) -> PaginatedDataTable {
        self.divider_thickness = Some(divider_thickness);
        self
    }

    /// Dart `PaginatedDataTable(onPageChanged:)`.
    pub fn on_page_changed(
        mut self,
        on_page_changed: impl Fn(&mut App, usize) + 'static,
    ) -> PaginatedDataTable {
        self.on_page_changed = Some(Rc::new(on_page_changed));
        self
    }

    /// Dart `PaginatedDataTable(rowsPerPage:)`.
    pub fn rows_per_page(mut self, rows_per_page: usize) -> PaginatedDataTable {
        self.rows_per_page = rows_per_page;
        self
    }

    /// Dart `PaginatedDataTable(availableRowsPerPage:)`.
    pub fn available_rows_per_page(
        mut self,
        available_rows_per_page: Vec<usize>,
    ) -> PaginatedDataTable {
        self.available_rows_per_page = available_rows_per_page;
        self
    }

    /// Dart `PaginatedDataTable(onRowsPerPageChanged:)`.
    pub fn on_rows_per_page_changed(
        mut self,
        on_rows_per_page_changed: impl Fn(&mut App, Option<usize>) + 'static,
    ) -> PaginatedDataTable {
        self.on_rows_per_page_changed = Some(Rc::new(on_rows_per_page_changed));
        self
    }

    /// Dart `PaginatedDataTable(dragStartBehavior:)`.
    pub fn drag_start_behavior(
        mut self,
        drag_start_behavior: DragStartBehavior,
    ) -> PaginatedDataTable {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `PaginatedDataTable(checkboxHorizontalMargin:)`.
    pub fn checkbox_horizontal_margin(
        mut self,
        checkbox_horizontal_margin: f64,
    ) -> PaginatedDataTable {
        self.checkbox_horizontal_margin = Some(checkbox_horizontal_margin);
        self
    }

    /// Dart `PaginatedDataTable(arrowHeadColor:)`.
    pub fn arrow_head_color(mut self, arrow_head_color: Color) -> PaginatedDataTable {
        self.arrow_head_color = Some(arrow_head_color);
        self
    }

    /// Dart `PaginatedDataTable(controller:)`.
    pub fn controller(mut self, controller: ScrollControllerRef) -> PaginatedDataTable {
        self.controller = Some(controller);
        self
    }

    /// Dart `PaginatedDataTable(primary:)`.
    pub fn primary(mut self, primary: bool) -> PaginatedDataTable {
        self.primary = Some(primary);
        self
    }

    /// Dart `PaginatedDataTable(headingRowColor:)`.
    pub fn heading_row_color(
        mut self,
        heading_row_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> PaginatedDataTable {
        self.heading_row_color = Some(heading_row_color);
        self
    }

    /// Dart `PaginatedDataTable(showEmptyRows:)`.
    pub fn show_empty_rows(mut self, show_empty_rows: bool) -> PaginatedDataTable {
        self.show_empty_rows = show_empty_rows;
        self
    }

    /// Dart `PaginatedDataTable(sortIconBuilder:)`.
    pub fn sort_icon_builder(
        mut self,
        sort_icon_builder: impl Fn(&mut App, BuildContext, bool, bool) -> WidgetRef + 'static,
    ) -> PaginatedDataTable {
        self.sort_icon_builder = Some(Rc::new(sort_icon_builder));
        self
    }
}

impl fmt::Debug for PaginatedDataTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PaginatedDataTable")
            .field("columns", &self.columns)
            .field("rows_per_page", &self.rows_per_page)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for PaginatedDataTable {
    type State = PaginatedDataTableState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(self.actions.is_none() || self.header.is_some());
        debug_assert!(!self.columns.is_empty());
        debug_assert!(
            self.sort_column_index
                .is_none_or(|index| index < self.columns.len())
        );
        debug_assert!(match (self.data_row_min_height, self.data_row_max_height) {
            (Some(min), Some(max)) => max >= min,
            _ => true,
        });
        debug_assert!(self.rows_per_page > 0);
        debug_assert!(
            self.divider_thickness
                .is_none_or(|thickness| thickness >= 0.0)
        );
        if self.on_rows_per_page_changed.is_some() {
            debug_assert!(self.available_rows_per_page.contains(&self.rows_per_page));
        }
        debug_assert!(
            !(self.controller.is_some() && self.primary.unwrap_or(false)),
            "Primary ScrollViews obtain their ScrollController via inheritance from a \
             PrimaryScrollController widget. You cannot both set primary to true and pass an \
             explicit controller."
        );
    }

    fn create_state(&self) -> PaginatedDataTableState {
        PaginatedDataTableState {
            state: StateData::new(),
            first_row_index: 0,
            row_count: 0,
            row_count_approximate: false,
            selected_row_count: 0,
            rows: HashMap::new(),
            table_key: GlobalKey::new(),
        }
    }
}

/// The state of a [`PaginatedDataTable`]; Dart's `PaginatedDataTableState`.
pub struct PaginatedDataTableState {
    state: StateData<PaginatedDataTable>,
    first_row_index: usize,
    row_count: usize,
    row_count_approximate: bool,
    selected_row_count: usize,
    rows: HashMap<usize, Option<DataRow>>,
    table_key: GlobalKey,
}

impl PaginatedDataTableState {
    fn handle_data_source_changed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {});
        self.update_caches(app);
    }

    fn update_caches(self: Handle<Self>, app: &mut App) {
        let source = self.widget(app).source.clone();
        let row_count = source.handle().row_count(app);
        let row_count_approximate = source.handle().is_row_count_approximate(app);
        let selected_row_count = source.handle().selected_row_count(app);
        let state = app.get_mut(self);
        state.row_count = row_count;
        state.row_count_approximate = row_count_approximate;
        state.selected_row_count = selected_row_count;
        state.rows.clear();
    }

    /// Ensures that the given row is visible.
    pub fn page_to(self: Handle<Self>, app: &mut App, row_index: usize) {
        let old_first_row_index = app.get(self).first_row_index;
        let rows_per_page = self.widget(app).rows_per_page;
        self.set_state(app, |state| {
            state.first_row_index = (row_index / rows_per_page) * rows_per_page;
        });
        let first_row_index = app.get(self).first_row_index;
        let on_page_changed = self.widget(app).on_page_changed.clone();
        if let Some(on_page_changed) = on_page_changed
            && old_first_row_index != first_row_index
        {
            on_page_changed(app, first_row_index);
        }
    }

    fn get_blank_row_for(self: Handle<Self>, app: &App, index: usize) -> DataRow {
        DataRow::by_index(
            Some(index as i32),
            self.widget(app)
                .columns
                .iter()
                .map(|_column| DataCell::empty())
                .collect(),
        )
    }

    fn get_progress_indicator_row_for(self: Handle<Self>, app: &App, index: usize) -> DataRow {
        let mut have_progress_indicator = false;
        let mut cells: Vec<DataCell> = self
            .widget(app)
            .columns
            .iter()
            .map(|column| {
                if !column.numeric {
                    have_progress_indicator = true;
                    return DataCell::new(CircularProgressIndicator::new());
                }
                DataCell::empty()
            })
            .collect();
        if !have_progress_indicator {
            cells[0] = DataCell::new(CircularProgressIndicator::new());
        }
        DataRow::by_index(Some(index as i32), cells)
    }

    fn get_rows(
        self: Handle<Self>,
        app: &mut App,
        first_row_index: usize,
        rows_per_page: usize,
    ) -> Vec<DataRow> {
        let mut result = Vec::new();
        let next_page_first_row_index = first_row_index + rows_per_page;
        let mut have_progress_indicator = false;
        let source = self.widget(app).source.clone();
        let show_empty_rows = self.widget(app).show_empty_rows;
        for index in first_row_index..next_page_first_row_index {
            let mut row: Option<DataRow> = None;
            if index < app.get(self).row_count || app.get(self).row_count_approximate {
                if !app.get(self).rows.contains_key(&index) {
                    let new_row = source.handle().get_row(app, index);
                    app.get_mut(self).rows.insert(index, new_row);
                }
                row = app.get(self).rows[&index].clone();
                if row.is_none() && !have_progress_indicator {
                    row = Some(self.get_progress_indicator_row_for(app, index));
                    have_progress_indicator = true;
                }
            }

            if show_empty_rows && row.is_none() {
                row = Some(self.get_blank_row_for(app, index));
            }

            if let Some(row) = row {
                result.push(row);
            }
        }
        result
    }

    fn handle_first(self: Handle<Self>, app: &mut App) {
        self.page_to(app, 0);
    }

    fn handle_previous(self: Handle<Self>, app: &mut App) {
        let rows_per_page = self.widget(app).rows_per_page;
        let first_row_index = app.get(self).first_row_index;
        self.page_to(app, first_row_index.saturating_sub(rows_per_page));
    }

    fn handle_next(self: Handle<Self>, app: &mut App) {
        let rows_per_page = self.widget(app).rows_per_page;
        let first_row_index = app.get(self).first_row_index;
        self.page_to(app, first_row_index + rows_per_page);
    }

    fn handle_last(self: Handle<Self>, app: &mut App) {
        let rows_per_page = self.widget(app).rows_per_page;
        let row_count = app.get(self).row_count;
        self.page_to(
            app,
            (row_count.saturating_sub(1) / rows_per_page) * rows_per_page,
        );
    }

    fn is_next_page_unavailable(self: Handle<Self>, app: &App) -> bool {
        let state = app.get(self);
        !state.row_count_approximate
            && state.first_row_index + self.widget(app).rows_per_page >= state.row_count
    }
}

impl State for PaginatedDataTableState {
    type Widget = PaginatedDataTable;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let stored = PageStorage::maybe_of(app, context)
            .and_then(|bucket| bucket.read_state(app, context, None))
            .and_then(|state| state.downcast_ref::<usize>().copied());
        app.get_mut(self).first_row_index = stored
            .or(self.widget(app).initial_first_row_index)
            .unwrap_or(0);
        let source = self.widget(app).source.clone();
        source.add_listener(
            app,
            Listener::handle_method(self, Self::handle_data_source_changed),
        );
        self.handle_data_source_changed_in_init(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &PaginatedDataTable) {
        let source = self.widget(app).source.clone();
        if old_widget.source != source {
            old_widget.source.remove_listener(
                app,
                &Listener::handle_method(self, Self::handle_data_source_changed),
            );
            source.add_listener(
                app,
                Listener::handle_method(self, Self::handle_data_source_changed),
            );
            self.update_caches(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let source = self.widget(app).source.clone();
        source.remove_listener(
            app,
            &Listener::handle_method(self, Self::handle_data_source_changed),
        );
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        // TODO(ianh): This whole build function doesn't handle RTL yet.
        debug_assert!(debug_check_has_material_localizations(app, context));
        let theme_data = Theme::of(app, context);
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let widget = self.widget(app).clone();
        let (first_row_index, row_count, row_count_approximate, selected_row_count) = {
            let state = app.get(self);
            (
                state.first_row_index,
                state.row_count,
                state.row_count_approximate,
                state.selected_row_count,
            )
        };
        // HEADER
        let mut header_widgets: Vec<WidgetRef> = Vec::new();
        if selected_row_count == 0 && widget.header.is_some() {
            header_widgets
                .push(Expanded::new(widget.header.clone().expect("checked above")).into_widget());
        } else if widget.header.is_some() {
            header_widgets.push(
                Expanded::new(Text::new(
                    localizations.selected_row_count_title(selected_row_count as i32),
                ))
                .into_widget(),
            );
        }
        if let Some(actions) = &widget.actions {
            header_widgets.extend(actions.iter().map(|action| {
                Padding::new(EdgeInsetsDirectional::only(24.0 - 8.0 * 2.0, 0.0, 0.0, 0.0).into())
                    // 8.0 is the default padding of an icon button
                    .child(action.clone())
                    .into_widget()
            }));
        }

        // FOOTER
        let footer_text_style = theme_data
            .text_theme
            .body_small
            .clone()
            .expect("the theme's text theme has bodySmall");
        let mut footer_widgets: Vec<WidgetRef> = Vec::new();
        if let Some(on_rows_per_page_changed) = widget.on_rows_per_page_changed.clone() {
            let available_rows_per_page: Vec<DropdownMenuItem<usize>> = widget
                .available_rows_per_page
                .iter()
                .filter(|value| **value <= row_count || **value == widget.rows_per_page)
                .map(|value| DropdownMenuItem::new(Text::new(value.to_string())).value(*value))
                .collect();
            footer_widgets.extend([
                // Match trailing padding, in case we overflow and end up scrolling.
                SizedBox::new().width(14.0).into_widget(),
                Text::new(localizations.rows_per_page_title()).into_widget(),
                ConstrainedBox::new(BoxConstraints {
                    min_width: 64.0, // 40.0 for the text, 24.0 for the icon
                    ..BoxConstraints::new()
                })
                .child(
                    Align::new().alignment(AlignmentGeometry::CENTER_END).child(
                        DropdownButtonHideUnderline::new(
                            DropdownButton::new(
                                Some(available_rows_per_page),
                                Some(on_rows_per_page_changed),
                            )
                            .value(Some(widget.rows_per_page))
                            .style(footer_text_style.clone()),
                        ),
                    ),
                )
                .into_widget(),
            ]);
        }
        let arrow_button = |icon: waft_widgets::IconData,
                            tooltip: String,
                            on_pressed: Option<Listener>|
         -> WidgetRef {
            let mut button = IconButton::new(on_pressed, waft_widgets::Icon::new(Some(icon)))
                .padding(EdgeInsets::ZERO.into())
                .tooltip(tooltip);
            if let Some(color) = widget.arrow_head_color {
                button = button.color(color);
            }
            button.into_widget()
        };
        footer_widgets.push(SizedBox::new().width(32.0).into_widget());
        footer_widgets.push(
            Text::new(localizations.page_rows_info_title(
                (first_row_index + 1) as i32,
                (first_row_index + widget.rows_per_page).min(row_count) as i32,
                row_count as i32,
                row_count_approximate,
            ))
            .into_widget(),
        );
        footer_widgets.push(SizedBox::new().width(32.0).into_widget());
        if widget.show_first_last_buttons {
            footer_widgets.push(arrow_button(
                Icons::skip_previous(),
                localizations.first_page_tooltip(),
                (first_row_index > 0).then(|| Listener::handle_method(self, Self::handle_first)),
            ));
        }
        footer_widgets.push(arrow_button(
            Icons::chevron_left(),
            localizations.previous_page_tooltip(),
            (first_row_index > 0).then(|| Listener::handle_method(self, Self::handle_previous)),
        ));
        footer_widgets.push(SizedBox::new().width(24.0).into_widget());
        let next_enabled = !self.is_next_page_unavailable(app);
        footer_widgets.push(arrow_button(
            Icons::chevron_right(),
            localizations.next_page_tooltip(),
            next_enabled.then(|| Listener::handle_method(self, Self::handle_next)),
        ));
        if widget.show_first_last_buttons {
            footer_widgets.push(arrow_button(
                Icons::skip_next(),
                localizations.last_page_tooltip(),
                next_enabled.then(|| Listener::handle_method(self, Self::handle_last)),
            ));
        }
        footer_widgets.push(SizedBox::new().width(14.0).into_widget());

        let rows = self.get_rows(app, first_row_index, widget.rows_per_page);
        let table_key = app.get(self).table_key.clone();
        let header_style = if selected_row_count > 0 {
            theme_data
                .text_theme
                .title_medium
                .clone()
                .expect("the theme's text theme has titleMedium")
                .copy_with()
                .color(theme_data.color_scheme.secondary)
        } else {
            theme_data
                .text_theme
                .title_large
                .clone()
                .expect("the theme's text theme has titleLarge")
                .copy_with()
                .font_weight(FontWeight::W400)
        };
        let secondary_header_color = theme_data.secondary_header_color;

        // CARD
        Card::new()
            .semantic_container(false)
            .child(
                LayoutBuilder::new(move |_app, _context, constraints| {
                    let mut children: Vec<WidgetRef> = Vec::new();
                    if !header_widgets.is_empty() {
                        let mut ink = Ink::new().height(64.0);
                        if selected_row_count > 0 {
                            ink = ink.color(secondary_header_color);
                        }
                        children.push(
                            Semantics::new()
                                .container(true)
                                .child(DefaultTextStyle::new(
                                    // These typographic styles aren't quite the regular ones. We pick
                                    // the closest ones from the regular list and then tweak them
                                    // appropriately.
                                    // See https://material.io/design/components/data-tables.html#tables-within-cards
                                    header_style.clone(),
                                    IconTheme::merge(
                                        None,
                                        IconThemeData::new().opacity(0.54),
                                        ink.child(
                                            Padding::new(
                                                EdgeInsetsDirectional::only(24.0, 0.0, 14.0, 0.0)
                                                    .into(),
                                            )
                                            .child(
                                                Row::new()
                                                    .main_axis_alignment(MainAxisAlignment::End)
                                                    .children(header_widgets.clone()),
                                            ),
                                        ),
                                    ),
                                ))
                                .into_widget(),
                        );
                    }
                    let mut table = DataTable::new(widget.columns.clone(), rows.clone())
                        .key(table_key.clone().into())
                        .sort_ascending(widget.sort_ascending)
                        // Make sure no decoration is set on the DataTable from the theme, as its
                        // already wrapped in a Card.
                        .decoration(BoxDecoration::new())
                        .heading_row_height(widget.heading_row_height)
                        .horizontal_margin(widget.horizontal_margin)
                        .column_spacing(widget.column_spacing)
                        .show_checkbox_column(widget.show_checkbox_column)
                        .show_bottom_border(true);
                    table.sort_column_index = widget.sort_column_index;
                    table.on_select_all = widget.on_select_all.clone();
                    table.divider_thickness = widget.divider_thickness;
                    table.data_row_min_height = widget.data_row_min_height;
                    table.data_row_max_height = widget.data_row_max_height;
                    table.checkbox_horizontal_margin = widget.checkbox_horizontal_margin;
                    table.heading_row_color = widget.heading_row_color.clone();
                    table.sort_icon_builder = widget.sort_icon_builder.clone();
                    let mut scroll = SingleChildScrollView::new()
                        .scroll_direction(Axis::Horizontal)
                        .drag_start_behavior(widget.drag_start_behavior);
                    if let Some(primary) = widget.primary {
                        scroll = scroll.primary(primary);
                    }
                    if let Some(controller) = widget.controller.clone() {
                        scroll = scroll.controller(controller);
                    }
                    children.push(
                        scroll
                            .child(
                                ConstrainedBox::new(BoxConstraints {
                                    min_width: constraints.min_width,
                                    ..BoxConstraints::new()
                                })
                                .child(table),
                            )
                            .into_widget(),
                    );
                    if !widget.show_empty_rows {
                        let missing = (widget.rows_per_page as i64 - row_count as i64
                            + first_row_index as i64)
                            .clamp(0, widget.rows_per_page as i64);
                        children.push(
                            SizedBox::new()
                                .height(
                                    widget
                                        .data_row_max_height
                                        .unwrap_or(K_MIN_INTERACTIVE_DIMENSION)
                                        * missing as f64,
                                )
                                .into_widget(),
                        );
                    }
                    children.push(
                        DefaultTextStyle::new(
                            footer_text_style.clone(),
                            IconTheme::merge(
                                None,
                                IconThemeData::new().opacity(0.54),
                                // TODO(bkonyi): this won't handle text zoom correctly,
                                //  https://github.com/flutter/flutter/issues/48522
                                SizedBox::new().height(56.0).child(
                                    SingleChildScrollView::new()
                                        .drag_start_behavior(widget.drag_start_behavior)
                                        .scroll_direction(Axis::Horizontal)
                                        .reverse(true)
                                        .child(Row::new().children(footer_widgets.clone())),
                                ),
                            ),
                        )
                        .into_widget(),
                    );
                    Column::new()
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .children(children)
                        .into_widget()
                })
                .into_widget(),
            )
            .into_widget()
    }
}

impl PaginatedDataTableState {
    /// The part of Dart's `initState` that runs `_handleDataSourceChanged()` once at start: the
    /// caches are filled; no rebuild is asked, the first build is about to run.
    fn handle_data_source_changed_in_init(self: Handle<Self>, app: &mut App) {
        self.update_caches(app);
    }
}

#[cfg(test)]
mod tests {
    //! The cases of `test/paginated_data_table_test.dart`.

    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::rc::Rc;
    use std::time::Duration;

    use crate::bottom_navigation_bar::tests::paragraph_style;
    use waft_embedder::{Color, Offset, Size};
    use waft_foundation::{ChangeNotifier, ChangeNotifierData, Ref};
    use waft_painting::BoxDecoration;
    use waft_widgets::testing::{
        Finder, WidgetTester, assert_f32_equals, assert_more_or_less_equals, expect, find,
        finds_n_widgets, finds_nothing, finds_one_widget, test_widgets,
    };
    use waft_widgets::{
        Align, Center, Container, GlobalKey, Padding, SizedBox, Table, Text, WidgetStatePropertyAll,
    };

    use super::*;
    use crate::checkbox::Checkbox;
    use crate::data_table::tests::{K_DESSERTS, app_of};
    use crate::data_table::{DataColumn, DataRow};
    use crate::data_table_source::{AnyDataTableSource, DataTableSource};
    use crate::data_table_theme::DataTableThemeData;
    use crate::icon_button::IconButton;
    use crate::material::Material;
    use crate::scaffold::Scaffold;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::m3;
    use waft_painting::DecorationRef;
    use waft_widgets::Icon;

    /// The `TestDataSource` of the test file.
    struct TestDataSource {
        change_notifier: ChangeNotifierData,
        allow_selection: bool,
        generation: i32,
        selected_rows: HashSet<usize>,
    }

    impl ChangeNotifier for TestDataSource {
        fn change_notifier_data(&self) -> &ChangeNotifierData {
            &self.change_notifier
        }

        fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
            &mut self.change_notifier
        }
    }

    impl TestDataSource {
        fn new(app: &mut App, allow_selection: bool, generation: i32) -> Ref<TestDataSource> {
            app.create(TestDataSource {
                change_notifier: ChangeNotifierData::new(),
                allow_selection,
                generation,
                selected_rows: HashSet::new(),
            })
        }

        fn set_generation(self: Handle<Self>, app: &mut App, value: i32) {
            if app.get(self).generation == value {
                return;
            }
            app.get_mut(self).generation = value;
            self.notify_listeners(app);
        }

        fn handle_selected(
            self: Handle<Self>,
            app: &mut App,
            index: usize,
            selected: Option<bool>,
        ) {
            if selected.unwrap_or(false) {
                app.get_mut(self).selected_rows.insert(index);
            } else {
                app.get_mut(self).selected_rows.remove(&index);
            }
            self.notify_listeners(app);
        }
    }

    impl DataTableSource for TestDataSource {
        fn get_row(self: Handle<Self>, app: &mut App, index: usize) -> Option<DataRow> {
            let dessert = &K_DESSERTS[index % K_DESSERTS.len()];
            let page = index / K_DESSERTS.len();
            let state = app.get(self);
            let selected = state.selected_rows.contains(&index);
            let generation = state.generation;
            let mut row = DataRow::by_index(
                Some(index as i32),
                vec![
                    DataCell::new(Text::new(format!("{} ({page})", dessert.name))),
                    DataCell::new(Text::new(dessert.calories.to_string())),
                    DataCell::new(Text::new(generation.to_string())),
                ],
            )
            .selected(selected);
            if state.allow_selection {
                row = row.on_select_changed(move |app, selected| {
                    self.handle_selected(app, index, selected)
                });
            }
            Some(row)
        }

        fn row_count(self: Handle<Self>, _app: &App) -> usize {
            50 * K_DESSERTS.len()
        }

        fn is_row_count_approximate(self: Handle<Self>, _app: &App) -> bool {
            false
        }

        fn selected_row_count(self: Handle<Self>, app: &App) -> usize {
            app.get(self).selected_rows.len()
        }
    }

    fn new_source(
        tester: &WidgetTester,
        allow_selection: bool,
        generation: i32,
    ) -> Ref<TestDataSource> {
        TestDataSource::new(&mut tester.cell.borrow_mut(), allow_selection, generation)
    }

    fn erased(source: &Ref<TestDataSource>) -> DataTableSourceRef {
        source.erase::<AnyDataTableSource>()
    }

    fn standard_columns() -> Vec<DataColumn> {
        vec![
            DataColumn::new(Text::new("Name")),
            DataColumn::new(Text::new("Calories")).numeric(true),
            DataColumn::new(Text::new("Generation")),
        ]
    }

    fn page_button(finder_tooltip: &str) -> Finder {
        find::ancestor(
            find::by_tooltip(finder_tooltip),
            find::by_type::<IconButton>(),
        )
    }

    fn page_button_enabled(tester: &WidgetTester, tooltip: &str) -> bool {
        tester.widget(page_button(tooltip), |button: &IconButton| {
            button.on_pressed.is_some()
        })
    }

    type Log = Rc<RefCell<Vec<String>>>;

    fn take_log(log: &Log) -> Vec<String> {
        std::mem::take(&mut *log.borrow_mut())
    }

    #[test]
    fn paginated_data_table_paging() {
        test_widgets(|tester| {
            let source = new_source(tester, false, 0);
            let log: Log = Rc::default();
            let rows_log = Rc::clone(&log);
            let page_log = Rc::clone(&log);
            let table = PaginatedDataTable::new(standard_columns(), erased(&source))
                .header(Text::new("Test table"))
                .rows_per_page(2)
                .show_first_last_buttons(true)
                .available_rows_per_page(vec![2, 4, 8, 16])
                .on_rows_per_page_changed(move |_app, rows_per_page| {
                    rows_log.borrow_mut().push(format!(
                        "rows-per-page-changed: {}",
                        rows_per_page.map_or("null".to_owned(), |value| value.to_string())
                    ))
                })
                .on_page_changed(move |_app, row_index| {
                    page_log
                        .borrow_mut()
                        .push(format!("page-changed: {row_index}"))
                });
            let root = app_of(tester, table);
            tester.pump_widget(root);

            tester.tap(find::by_tooltip("Next page"));
            assert_eq!(take_log(&log), ["page-changed: 2"]);

            tester.pump();
            expect(tester, &find::text("Frozen yogurt (0)"), finds_nothing());
            expect(tester, &find::text("Eclair (0)"), finds_one_widget());
            expect(tester, &find::text("Gingerbread (0)"), finds_nothing());

            tester.tap(find::by_icon(&Icons::chevron_left()));
            assert_eq!(take_log(&log), ["page-changed: 0"]);

            tester.pump();
            expect(tester, &find::text("Frozen yogurt (0)"), finds_one_widget());
            expect(tester, &find::text("Eclair (0)"), finds_nothing());
            expect(tester, &find::text("Gingerbread (0)"), finds_nothing());

            assert!(page_button_enabled(tester, "Last page"));

            tester.tap(page_button("Last page"));
            assert_eq!(take_log(&log), ["page-changed: 498"]);

            tester.pump();
            assert!(!page_button_enabled(tester, "Last page"));

            expect(tester, &find::text("Frozen yogurt (0)"), finds_nothing());
            expect(tester, &find::text("Donut (49)"), finds_one_widget());
            expect(tester, &find::text("KitKat (49)"), finds_one_widget());

            assert!(page_button_enabled(tester, "First page"));

            tester.tap(page_button("First page"));
            assert_eq!(take_log(&log), ["page-changed: 0"]);

            tester.pump();
            assert!(!page_button_enabled(tester, "First page"));

            expect(tester, &find::text("Frozen yogurt (0)"), finds_one_widget());
            expect(tester, &find::text("Eclair (0)"), finds_nothing());
            expect(tester, &find::text("Gingerbread (0)"), finds_nothing());

            tester.tap(find::by_icon(&Icons::chevron_left()));
            assert!(log.borrow().is_empty());

            tester.tap(find::text("2"));
            tester.pump_and_settle_by(Duration::from_millis(200));

            tester.tap(find::text("8").last());
            tester.pump_and_settle_by(Duration::from_millis(200));

            assert_eq!(take_log(&log), ["rows-per-page-changed: 8"]);
        });
    }

    #[test]
    fn paginated_data_table_footer_page_number() {
        test_widgets(|tester| {
            let source = new_source(tester, false, 0);
            let build_table = |tester: &WidgetTester, rows_per_page: usize| -> WidgetRef {
                app_of(
                    tester,
                    PaginatedDataTable::new(standard_columns(), erased(&source))
                        .header(Text::new("Test table"))
                        .rows_per_page(rows_per_page)
                        .show_first_last_buttons(true)
                        .available_rows_per_page(vec![2, 3, 4, 5, 7, 8])
                        .on_rows_per_page_changed(|_app, _rows_per_page| {})
                        .on_page_changed(|_app, _row_index| {}),
                )
            };

            let root = build_table(tester, 2);
            tester.pump_widget(root);
            expect(tester, &find::text("1\u{2013}2 of 500"), finds_one_widget());
            tester.tap(find::by_tooltip("Next page"));
            tester.pump();
            expect(tester, &find::text("3\u{2013}4 of 500"), finds_one_widget());

            assert!(page_button_enabled(tester, "Last page"));
            tester.tap(page_button("Last page"));
            tester.pump();
            expect(
                tester,
                &find::text("499\u{2013}500 of 500"),
                finds_one_widget(),
            );

            let element = tester.element(find::by_type::<PaginatedDataTable>());
            let state = element
                .state_handle::<PaginatedDataTableState>(&tester.cell.borrow())
                .expect("a PaginatedDataTableState");
            state.page_to(&mut tester.cell.borrow_mut(), 1);

            for (rows_per_page, first, next, last) in [
                (
                    3,
                    "1\u{2013}3 of 500",
                    "4\u{2013}6 of 500",
                    "499\u{2013}500 of 500",
                ),
                (
                    4,
                    "1\u{2013}4 of 500",
                    "5\u{2013}8 of 500",
                    "497\u{2013}500 of 500",
                ),
                (
                    5,
                    "1\u{2013}5 of 500",
                    "6\u{2013}10 of 500",
                    "496\u{2013}500 of 500",
                ),
                (
                    8,
                    "1\u{2013}8 of 500",
                    "9\u{2013}16 of 500",
                    "497\u{2013}500 of 500",
                ),
            ] {
                let root = build_table(tester, rows_per_page);
                tester.pump_widget(root);
                expect(tester, &find::text_containing(first), finds_one_widget());
                tester.tap(find::by_tooltip("Next page"));
                tester.pump();
                expect(tester, &find::text(next), finds_one_widget());
                assert!(page_button_enabled(tester, "Last page"));
                tester.tap(page_button("Last page"));
                tester.pump();
                expect(tester, &find::text(last), finds_one_widget());
                state.page_to(&mut tester.cell.borrow_mut(), 1);
            }
        });
    }

    #[test]
    fn paginated_data_table_control_test() {
        test_widgets(|tester| {
            let log: Log = Rc::default();
            let build_table = |tester: &WidgetTester, source: &Ref<TestDataSource>| -> WidgetRef {
                let sort_log = Rc::clone(&log);
                let page_log = Rc::clone(&log);
                let action_log = Rc::clone(&log);
                app_of(
                    tester,
                    PaginatedDataTable::new(
                        vec![
                            DataColumn::new(Text::new("Name")).tooltip("Name"),
                            DataColumn::new(Text::new("Calories"))
                                .tooltip("Calories")
                                .numeric(true)
                                .on_sort(move |_app, column_index, ascending| {
                                    sort_log
                                        .borrow_mut()
                                        .push(format!("column-sort: {column_index} {ascending}"))
                                }),
                            DataColumn::new(Text::new("Generation")).tooltip("Generation"),
                        ],
                        erased(source),
                    )
                    .header(Text::new("Test table"))
                    .on_page_changed(move |_app, row_index| {
                        page_log
                            .borrow_mut()
                            .push(format!("page-changed: {row_index}"))
                    })
                    .actions(vec![
                        IconButton::new(
                            Some(Listener::new(move |_| {
                                action_log.borrow_mut().push("action: adjust".to_owned())
                            })),
                            Icon::new(Some(Icons::adjust())),
                        )
                        .into_widget(),
                    ]),
                )
            };

            let source = new_source(tester, false, 42);
            let root = build_table(tester, &source);
            tester.pump_widget(root);

            // (Dart also expects a RenderFlex overflow exception here, because the test forces
            // the column to 600 pixels high; Waft has no `takeException`.)
            expect(tester, &find::text("Gingerbread (0)"), finds_one_widget());
            expect(tester, &find::text("Gingerbread (1)"), finds_nothing());
            expect(tester, &find::text("42"), finds_n_widgets(10));

            source
                .handle()
                .set_generation(&mut tester.cell.borrow_mut(), 43);
            tester.pump();

            expect(tester, &find::text("42"), finds_nothing());
            expect(tester, &find::text("43"), finds_n_widgets(10));

            let source = new_source(tester, false, 15);
            let root = build_table(tester, &source);
            tester.pump_widget(root);

            expect(tester, &find::text("42"), finds_nothing());
            expect(tester, &find::text("43"), finds_nothing());
            expect(tester, &find::text("15"), finds_n_widgets(10));

            let element = tester.element(find::by_type::<PaginatedDataTable>());
            let state = element
                .state_handle::<PaginatedDataTableState>(&tester.cell.borrow())
                .expect("a PaginatedDataTableState");

            assert!(log.borrow().is_empty());
            state.page_to(&mut tester.cell.borrow_mut(), 23);
            assert_eq!(take_log(&log), ["page-changed: 20"]);

            tester.pump();
            expect(tester, &find::text("Gingerbread (0)"), finds_nothing());
            expect(tester, &find::text("Gingerbread (1)"), finds_nothing());
            expect(tester, &find::text("Gingerbread (2)"), finds_one_widget());

            tester.tap(find::by_icon(&Icons::adjust()));
            assert_eq!(take_log(&log), ["action: adjust"]);
        });
    }

    #[test]
    fn paginated_data_table_text_alignment() {
        test_widgets(|tester| {
            let source = new_source(tester, false, 0);
            let root = app_of(
                tester,
                PaginatedDataTable::new(
                    vec![
                        DataColumn::new(Text::new("COL1")),
                        DataColumn::new(Text::new("COL2")),
                        DataColumn::new(Text::new("COL3")),
                    ],
                    erased(&source),
                )
                .header(Text::new("HEADER"))
                .rows_per_page(8)
                .available_rows_per_page(vec![8, 9])
                .on_rows_per_page_changed(|_app, _rows_per_page| {}),
            );
            tester.pump_widget(root);

            expect(tester, &find::text("Rows per page:"), finds_one_widget());
            expect(tester, &find::text("8"), finds_one_widget());
            // Within f32: global positions go through valo's f32 matrices (waft-gestures'
            // Matrix4 entry), where Dart's are f64 and compare equal.
            assert_f32_equals(
                tester.get_top_right(find::text("8")).dx(),
                tester.get_top_right(find::text("Rows per page:")).dx() + 40.0,
            ); // per spec
        });
    }

    #[test]
    fn paginated_data_table_with_and_without_header_and_actions() {
        test_widgets(|tester| {
            tester.set_surface_size(Some(Size::new(800.0, 800.0)));
            let header_text = "HEADER";
            let source = new_source(tester, true, 0);
            let build_table = |tester: &WidgetTester, header: Option<&str>, with_actions: bool| {
                let mut table = PaginatedDataTable::new(standard_columns(), erased(&source));
                if let Some(header) = header {
                    table = table.header(Text::new(header));
                }
                if with_actions {
                    table = table.actions(vec![
                        IconButton::new(Some(Listener::new(|_| {})), Icon::new(Some(Icons::add())))
                            .into_widget(),
                    ]);
                }
                app_of(tester, table)
            };

            let root = build_table(tester, Some(header_text), false);
            tester.pump_widget(root);
            expect(tester, &find::text(header_text), finds_one_widget());
            expect(tester, &find::by_icon(&Icons::add()), finds_nothing());

            let root = build_table(tester, Some(header_text), true);
            tester.pump_widget(root);
            expect(tester, &find::text(header_text), finds_one_widget());
            expect(tester, &find::by_icon(&Icons::add()), finds_one_widget());

            let root = build_table(tester, None, false);
            tester.pump_widget(root);
            expect(tester, &find::text(header_text), finds_nothing());
            expect(tester, &find::by_icon(&Icons::add()), finds_nothing());
            tester.set_surface_size(None);
        });
    }

    #[test]
    #[should_panic]
    fn paginated_data_table_actions_without_a_header_asserts() {
        test_widgets(|tester| {
            let source = new_source(tester, true, 0);
            let table = PaginatedDataTable::new(standard_columns(), erased(&source)).actions(vec![
                IconButton::new(Some(Listener::new(|_| {})), Icon::new(Some(Icons::add())))
                    .into_widget(),
            ]);
            let _ = table.into_widget();
        });
    }

    #[test]
    fn paginated_data_table_table_fills_card_width() {
        test_widgets(|tester| {
            // 800 is wide enough to ensure that all of the columns fit in the Card. The test makes
            // sure that the DataTable is exactly as wide as the Card, minus the Card's margin.
            let original_width = 800.0;
            let expanded_width = 1600.0;
            let height = 400.0;
            // By default, the margin of a Card is 4 in all directions, so the size of the
            // DataTable (inside the Card) is horizontally reduced by 4 * 2; the left and right
            // margins.
            let card_margin = 8.0;
            let source = new_source(tester, false, 0);
            let build_widget = |tester: &WidgetTester| {
                app_of(
                    tester,
                    PaginatedDataTable::new(standard_columns(), erased(&source))
                        .header(Text::new("Test table"))
                        .rows_per_page(2)
                        .available_rows_per_page(vec![2, 4, 8, 16])
                        .on_rows_per_page_changed(|_app, _rows_per_page| {})
                        .on_page_changed(|_app, _row_index| {}),
                )
            };
            tester.set_surface_size(Some(Size::new(original_width, height)));
            let root = build_widget(tester);
            tester.pump_widget(root);
            let mut card_width = tester.get_size(find::by_type::<Card>().first()).width();
            // Widths should be equal before we resize...
            assert_more_or_less_equals(
                tester
                    .get_size(find::by_type::<DataTable>().first())
                    .width(),
                card_width - card_margin,
            );

            tester.set_surface_size(Some(Size::new(expanded_width, height)));
            let root = build_widget(tester);
            tester.pump_widget(root);
            card_width = tester.get_size(find::by_type::<Card>().first()).width();
            // ... and should still be equal after the resize.
            assert_more_or_less_equals(
                tester
                    .get_size(find::by_type::<DataTable>().first())
                    .width(),
                card_width - card_margin,
            );
            // Double check to ensure we actually resized the surface properly.
            assert_more_or_less_equals(card_width, expanded_width);
            tester.set_surface_size(None);
        });
    }

    #[test]
    fn paginated_data_table_with_optional_column_checkbox() {
        test_widgets(|tester| {
            tester.set_surface_size(Some(Size::new(800.0, 800.0)));
            let source = new_source(tester, true, 0);
            let build_table = |tester: &WidgetTester, checkbox: bool| {
                app_of(
                    tester,
                    PaginatedDataTable::new(standard_columns(), erased(&source))
                        .header(Text::new("Test table"))
                        .show_checkbox_column(checkbox),
                )
            };
            let root = build_table(tester, true);
            tester.pump_widget(root);
            expect(tester, &find::by_type::<Checkbox>(), finds_n_widgets(11));

            let root = build_table(tester, false);
            tester.pump_widget(root);
            expect(tester, &find::by_type::<Checkbox>(), finds_nothing());
            tester.set_surface_size(None);
        });
    }

    #[test]
    fn table_should_not_use_decoration_from_data_table_theme() {
        test_widgets(|tester| {
            tester.set_surface_size(Some(Size::new(800.0, 800.0)));
            let source = new_source(tester, true, 0);
            let theme = ThemeData::builder()
                .data_table_theme(
                    DataTableThemeData::new()
                        .decoration(BoxDecoration::new().color(Color::new(0xffffffff))),
                )
                .build(&tester.cell.borrow());
            let root = material_app(
                theme,
                Material::new()
                    .child(
                        PaginatedDataTable::new(standard_columns(), erased(&source))
                            .header(Text::new("Test table")),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            let table_container =
                find::ancestor(find::by_type::<Table>(), find::by_type::<Container>()).first();
            let decoration = tester.widget(table_container, |container: &Container| {
                container.decoration.clone()
            });
            assert_eq!(decoration, Some(DecorationRef::new(BoxDecoration::new())));
            tester.set_surface_size(None);
        });
    }

    #[test]
    fn paginated_data_table_arrow_head_color_set_properly() {
        test_widgets(|tester| {
            tester.set_surface_size(Some(Size::new(800.0, 800.0)));
            let arrow_head_color = Color::new(0xffe53935);
            let source = new_source(tester, false, 0);
            let root = app_of(
                tester,
                PaginatedDataTable::new(standard_columns(), erased(&source))
                    .arrow_head_color(arrow_head_color)
                    .show_first_last_buttons(true)
                    .header(Text::new("Test table")),
            );
            tester.pump_widget(root);

            let icons = find::by_type::<IconButton>();
            for index in 0..4 {
                assert_eq!(
                    tester.widget(icons.clone().at(index), |button: &IconButton| button.color),
                    Some(arrow_head_color)
                );
            }
            tester.set_surface_size(None);
        });
    }

    #[test]
    fn paginated_data_table_custom_heading_row_color() {
        test_widgets(|tester| {
            let heading_row_color = Color::new(0xffff0000);
            let source = new_source(tester, false, 0);
            let root = material_app(
                m3(&tester.cell.borrow()),
                Scaffold::new()
                    .body(
                        PaginatedDataTable::new(standard_columns(), erased(&source))
                            .primary(true)
                            .header(Text::new("Test table"))
                            .rows_per_page(2)
                            .heading_row_color(Rc::new(WidgetStatePropertyAll::new(Some(
                                heading_row_color,
                            )))),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            let color = tester.widget(find::by_type::<Table>(), |table: &Table| {
                table.children[0]
                    .decoration
                    .as_ref()
                    .and_then(|decoration| decoration.as_any().downcast_ref::<BoxDecoration>())
                    .expect("a BoxDecoration")
                    .color
                    .clone()
                    .map(|color| color.color())
            });
            assert_eq!(color, Some(heading_row_color));
        });
    }

    fn heading_row_bottom_border_width(tester: &WidgetTester) -> f64 {
        expect(tester, &find::by_type::<Table>(), finds_one_widget());
        tester.widget(find::by_type::<Table>(), |table: &Table| {
            table.children[0]
                .decoration
                .as_ref()
                .and_then(|decoration| decoration.as_any().downcast_ref::<BoxDecoration>())
                .expect("a BoxDecoration")
                .clone()
                .border
                .expect("a border")
                .bottom()
                .width
        })
    }

    #[test]
    fn paginated_data_table_respects_custom_divider_thickness() {
        test_widgets(|tester| {
            let divider_thickness = 2.0;
            let source = new_source(tester, false, 0);
            let root = material_app(
                m3(&tester.cell.borrow()),
                Scaffold::new()
                    .body(
                        PaginatedDataTable::new(standard_columns(), erased(&source))
                            .primary(true)
                            .header(Text::new("Test table"))
                            .rows_per_page(2)
                            .divider_thickness(divider_thickness),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);
            assert_eq!(heading_row_bottom_border_width(tester), divider_thickness);
        });
    }

    #[test]
    fn paginated_data_table_respects_default_divider_thickness() {
        test_widgets(|tester| {
            let source = new_source(tester, false, 0);
            let root = material_app(
                m3(&tester.cell.borrow()),
                Scaffold::new()
                    .body(
                        PaginatedDataTable::new(standard_columns(), erased(&source))
                            .primary(true)
                            .header(Text::new("Test table"))
                            .rows_per_page(2),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);
            assert_eq!(heading_row_bottom_border_width(tester), 1.0);
        });
    }

    #[test]
    fn paginated_data_table_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let source = new_source(tester, false, 0);
            let root = app_of(
                tester,
                Center::new().child(
                    SizedBox::shrink().child(
                        PaginatedDataTable::new(
                            vec![
                                DataColumn::new(Text::new("X")),
                                DataColumn::new(Text::new("Y")),
                                DataColumn::new(Text::new("Z")),
                            ],
                            erased(&source),
                        )
                        .into_widget(),
                    ),
                ),
            );
            tester.pump_widget(root);
            assert_eq!(
                tester.get_size(find::by_type::<PaginatedDataTable>()),
                Size::ZERO
            );
        });
    }

    #[test]
    fn paginated_data_table_custom_sort_icon_builder_test() {
        test_widgets(|tester| {
            let source = new_source(tester, false, 0);
            let root = app_of(
                tester,
                PaginatedDataTable::new(
                    vec![
                        DataColumn::new(Text::new("Name")).on_sort(|_app, _index, _ascending| {}),
                        DataColumn::new(Text::new("Calories")).numeric(true),
                        DataColumn::new(Text::new("Generation")),
                    ],
                    erased(&source),
                )
                .sort_column_index(0)
                .sort_icon_builder(|_app, _context, visible, ascending| {
                    Text::new(if visible {
                        if ascending {
                            "PAGINATED_ASC"
                        } else {
                            "PAGINATED_DESC"
                        }
                    } else {
                        "PAGINATED_INACTIVE"
                    })
                    .into_widget()
                })
                .header(Text::new("Test Table"))
                .rows_per_page(2),
            );
            tester.pump_widget(root);
            expect(tester, &find::text("PAGINATED_ASC"), finds_one_widget());
        });
    }

    #[test]
    fn paginated_data_table_state_is_reachable_by_a_global_key() {
        test_widgets(|tester| {
            let source = new_source(tester, false, 0);
            let key = GlobalKey::new();
            let root = app_of(
                tester,
                PaginatedDataTable::new(standard_columns(), erased(&source))
                    .key(key.clone().into()),
            );
            tester.pump_widget(root);
            let state = key
                .current_state::<PaginatedDataTableState>(&tester.cell.borrow())
                .expect("the state of the table");
            state.page_to(&mut tester.cell.borrow_mut(), 10);
            tester.pump();
            expect(tester, &find::text("Frozen yogurt (1)"), finds_one_widget());
        });
    }
    /// Dart's `find.byWidgetPredicate((Widget widget) => widget is SizedBox && widget.height == h)`.
    fn sized_box_of_height(height: f64) -> Finder {
        find::by_widget_predicate("a SizedBox of that height", move |_app, widget| {
            waft_widgets::downcast_widget::<SizedBox>(widget)
                .is_some_and(|sized_box| sized_box.height == Some(height))
        })
    }

    #[test]
    #[allow(deprecated)]
    fn paginated_data_table_last_page_empty_space() {
        test_widgets(|tester| {
            let source = new_source(tester, false, 0);
            let row_count = 50 * K_DESSERTS.len();
            let build_table = |rows_per_page: usize| -> PaginatedDataTable {
                PaginatedDataTable::new(standard_columns(), erased(&source))
                    .header(Text::new("Test table"))
                    .rows_per_page(rows_per_page)
                    .show_first_last_buttons(true)
                    .data_row_height(46.0)
                    .available_rows_per_page(vec![3, 6, 7, 8, 9])
                    .on_rows_per_page_changed(|_app, _rows_per_page| {})
                    .on_page_changed(|_app, _row_index| {})
                    .show_empty_rows(false)
            };
            let theme = m3(&tester.cell.borrow());

            let root = material_app(
                theme.clone(),
                Scaffold::new()
                    .body(Center::new().child(build_table(3)))
                    .into_widget(),
            );
            tester.pump_widget(root);
            expect(tester, &sized_box_of_height(0.0), finds_one_widget());
            tester.tap(find::by_icon(&Icons::skip_next()));
            tester.pump();
            let expected = (3 - (row_count % 3)) as f64 * 46.0;
            expect(tester, &sized_box_of_height(expected), finds_one_widget());

            for rows_per_page in [6, 7, 8] {
                let root = material_app(
                    theme.clone(),
                    Material::new()
                        .child(build_table(rows_per_page))
                        .into_widget(),
                );
                tester.pump_widget(root);
                tester.tap(find::by_icon(&Icons::skip_previous()));
                tester.pump();
                expect(tester, &sized_box_of_height(0.0), finds_one_widget());
                tester.tap(find::by_icon(&Icons::skip_next()));
                tester.pump();
                let expected = (rows_per_page - (row_count % rows_per_page)) as f64 * 46.0;
                expect(tester, &sized_box_of_height(expected), finds_one_widget());
            }
        });
    }

    #[test]
    fn paginated_data_table_footer_scrolls() {
        test_widgets(|tester| {
            let source = new_source(tester, false, 0);
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Align::new()
                    .alignment(waft_painting::AlignmentGeometry::TOP_LEFT)
                    .child(
                        SizedBox::new().width(100.0).child(
                            PaginatedDataTable::new(
                                vec![
                                    DataColumn::new(Text::new("COL1")),
                                    DataColumn::new(Text::new("COL2")),
                                    DataColumn::new(Text::new("COL3")),
                                ],
                                erased(&source),
                            )
                            .header(Text::new("HEADER"))
                            .rows_per_page(5)
                            .drag_start_behavior(waft_gestures::DragStartBehavior::Down)
                            .available_rows_per_page(vec![5])
                            .on_rows_per_page_changed(|_app, _rows_per_page| {}),
                        ),
                    )
                    .into_widget(),
            ));
            expect(tester, &find::text("Rows per page:"), finds_one_widget());
            // off screen
            assert!(tester.get_top_left(find::text("Rows per page:")).dx() < 0.0);
            let y = tester.get_top_left(find::text("Rows per page:")).dy();
            let pointer = tester.next_pointer();
            tester.drag_from(Offset::new(50.0, y), Offset::new(1000.0, 0.0), pointer);
            tester.pump_and_settle();
            expect(tester, &find::text("Rows per page:"), finds_one_widget());
            // 14 padding in the footer row, 4 padding from the card
            assert_eq!(tester.get_top_left(find::text("Rows per page:")).dx(), 18.0);
        });
    }

    #[test]
    fn data_row_min_height_and_data_row_max_height_if_not_set_will_use_data_table_theme() {
        test_widgets(|tester| {
            tester.set_surface_size(Some(Size::new(800.0, 800.0)));
            let min_max_data_row_height = 30.0;
            let source = new_source(tester, true, 0);
            let theme = ThemeData::builder()
                .data_table_theme(
                    DataTableThemeData::new()
                        .data_row_min_height(min_max_data_row_height)
                        .data_row_max_height(min_max_data_row_height),
                )
                .build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                PaginatedDataTable::new(standard_columns(), erased(&source))
                    .header(Text::new("Test table"))
                    .into_widget(),
            ));

            let row_container =
                find::descendant(find::by_type::<Table>(), find::by_type::<Container>()).last();
            let constraints =
                tester.widget(row_container, |container: &Container| container.constraints);
            let constraints = constraints.expect("the row container's constraints");
            assert_eq!(constraints.min_height, min_max_data_row_height);
            assert_eq!(constraints.max_height, min_max_data_row_height);
            tester.set_surface_size(None);
        });
    }

    #[test]
    fn items_selected_text_uses_secondary_color() {
        test_widgets(|tester| {
            let selected_text_color = Color::new(0xff00ddff);
            let colors = crate::ColorScheme::light().with_secondary(selected_text_color);
            let theme = ThemeData::from(colors).build(&tester.cell.borrow());
            let source = new_source(tester, true, 0);

            tester.set_surface_size(Some(Size::new(800.0, 800.0)));
            tester.pump_widget(material_app(
                theme,
                PaginatedDataTable::new(standard_columns(), erased(&source))
                    .header(Text::new("Test table"))
                    .into_widget(),
            ));
            expect(tester, &find::text("Test table"), finds_one_widget());

            // Select a row with yogurt
            tester.tap(find::text("Frozen yogurt (0)"));
            tester.pump_and_settle();

            // The header should be replace with a selected text item
            expect(tester, &find::text("Test table"), finds_nothing());
            expect(tester, &find::text("1 item selected"), finds_one_widget());

            // The color of the selected text item should be the colorScheme.secondary
            let selected_text_style = paragraph_style(tester, "1 item selected");
            assert_eq!(
                selected_text_style.color.map(|color| color.color()),
                Some(selected_text_color)
            );
            tester.set_surface_size(None);
        });
    }

    #[test]
    fn overflow_bar_header_left_alignment() {
        // Test an old special case that tried to align the first child of a ButtonBar
        // and the left edge of a Text header widget. Still possible with OverflowBar
        // albeit without any special case in the implementation's build method.
        test_widgets(|tester| {
            let source = new_source(tester, false, 0);
            let theme = m3(&tester.cell.borrow());
            let build_frame = |header: WidgetRef| {
                material_app(
                    theme.clone(),
                    PaginatedDataTable::new(standard_columns(), erased(&source))
                        .header(header)
                        .rows_per_page(2)
                        .into_widget(),
                )
            };

            tester.pump_widget(build_frame(Text::new("HEADER").into_widget()));
            let header_x = tester.get_top_left(find::text("HEADER")).dx();
            let overflow_bar = waft_widgets::OverflowBar::new()
                .children(vec![
                    crate::ElevatedButton::new(
                        Some(Listener::new(|_app| {})),
                        Some(Text::new("BUTTON")),
                    )
                    .into_widget(),
                ])
                .into_widget();
            tester.pump_widget(build_frame(overflow_bar));
            assert_eq!(
                header_x,
                tester
                    .get_top_left(find::by_type::<crate::ElevatedButton>())
                    .dx()
            );
        });
    }

    #[test]
    fn paginated_data_table_can_be_scrolled_using_scroll_controller() {
        test_widgets(|tester| {
            let scroll_controller: waft_widgets::ScrollControllerRef =
                waft_widgets::ScrollController::default(&mut tester.cell.borrow_mut())
                    .into_erased();
            let source = new_source(tester, false, 0);
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Align::new()
                    .alignment(waft_painting::AlignmentGeometry::TOP_LEFT)
                    .child(
                        SizedBox::new().width(100.0).child(
                            PaginatedDataTable::new(
                                vec![
                                    DataColumn::new(Text::new("Name")).tooltip("Name"),
                                    DataColumn::new(Text::new("Calories"))
                                        .tooltip("Calories")
                                        .numeric(true),
                                    DataColumn::new(Text::new("Generation")).tooltip("Generation"),
                                ],
                                erased(&source),
                            )
                            .controller(scroll_controller.clone())
                            .header(Text::new("Test table"))
                            .rows_per_page(2),
                        ),
                    )
                    .into_widget(),
            ));

            // DataTable uses provided ScrollController
            let body_controller = tester.widget(
                find::by_type::<waft_widgets::Scrollable>().first(),
                |scrollable: &waft_widgets::Scrollable| scrollable.controller.clone(),
            );
            assert!(body_controller.as_ref() == Some(&scroll_controller));

            assert_eq!(scroll_controller.offset(&tester.cell.borrow()), 0.0);
            scroll_controller.jump_to(&mut tester.cell.borrow_mut(), 50.0);
            tester.pump_and_settle();

            assert_eq!(scroll_controller.offset(&tester.cell.borrow()), 50.0);
        });
    }

    #[test]
    fn paginated_data_table_uses_primary_scroll_controller_when_primary() {
        test_widgets(|tester| {
            let primary_scroll_controller: waft_widgets::ScrollControllerRef =
                waft_widgets::ScrollController::default(&mut tester.cell.borrow_mut())
                    .into_erased();
            let source = new_source(tester, false, 0);
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                waft_widgets::PrimaryScrollController::new(
                    primary_scroll_controller.clone(),
                    PaginatedDataTable::new(standard_columns(), erased(&source))
                        .primary(true)
                        .header(Text::new("Test table"))
                        .rows_per_page(2),
                )
                .into_widget(),
            ));

            // DataTable uses primaryScrollController
            let body_controller = tester.widget(
                find::by_type::<waft_widgets::Scrollable>().first(),
                |scrollable: &waft_widgets::Scrollable| scrollable.controller.clone(),
            );
            assert!(body_controller.as_ref() == Some(&primary_scroll_controller));

            // Footer does not use primaryScrollController
            let footer_controller = tester.widget(
                find::by_type::<waft_widgets::Scrollable>().last(),
                |scrollable: &waft_widgets::Scrollable| scrollable.controller.clone(),
            );
            assert!(footer_controller.is_none());
        });
    }
    #[test]
    #[allow(deprecated)]
    fn paginated_data_table_custom_row_height() {
        test_widgets(|tester| {
            let source = new_source(tester, false, 0);
            let theme = m3(&tester.cell.borrow());
            let height_of = |tester: &WidgetTester, text: &str| {
                tester
                    .get_size(find::widget_with_text::<Container>(text).first())
                    .height()
            };
            let build_table = |heading_row_height: f64,
                               data_row_height: Option<f64>,
                               min_max: Option<(f64, f64)>| {
                let mut table = PaginatedDataTable::new(standard_columns(), erased(&source))
                    .header(Text::new("Test table"))
                    .rows_per_page(2)
                    .available_rows_per_page(vec![2, 4, 8, 16])
                    .on_rows_per_page_changed(|_app, _rows_per_page| {})
                    .on_page_changed(|_app, _row_index| {})
                    .heading_row_height(heading_row_height);
                if let Some(data_row_height) = data_row_height {
                    table = table.data_row_height(data_row_height);
                }
                if let Some((min, max)) = min_max {
                    table = table.data_row_min_height(min).data_row_max_height(max);
                }
                material_app(theme.clone(), Material::new().child(table).into_widget())
            };

            // DEFAULT VALUES
            let root = material_app(
                theme.clone(),
                PaginatedDataTable::new(standard_columns(), erased(&source))
                    .header(Text::new("Test table"))
                    .rows_per_page(2)
                    .available_rows_per_page(vec![2, 4, 8, 16])
                    .on_rows_per_page_changed(|_app, _rows_per_page| {})
                    .on_page_changed(|_app, _row_index| {})
                    .into_widget(),
            );
            tester.pump_widget(root);
            // This is the header row height
            assert_eq!(height_of(tester, "Name"), 56.0);
            // This is the data row height
            assert_eq!(height_of(tester, "Frozen yogurt (0)"), 48.0);

            // CUSTOM VALUES
            tester.pump_widget(build_table(48.0, None, None));
            assert_eq!(height_of(tester, "Name"), 48.0);

            tester.pump_widget(build_table(64.0, None, None));
            assert_eq!(height_of(tester, "Name"), 64.0);

            tester.pump_widget(build_table(56.0, Some(30.0), None));
            assert_eq!(height_of(tester, "Frozen yogurt (0)"), 30.0);

            tester.pump_widget(build_table(56.0, Some(56.0), None));
            assert_eq!(height_of(tester, "Frozen yogurt (0)"), 56.0);

            tester.pump_widget(build_table(56.0, None, Some((51.0, 51.0))));
            assert_eq!(height_of(tester, "Frozen yogurt (0)"), 51.0);
        });
    }

    #[test]
    fn paginated_data_table_custom_checkbox_horizontal_margin_properly_applied() {
        test_widgets(|tester| {
            let custom_checkbox_horizontal_margin = 15.0;
            let custom_horizontal_margin = 10.0;
            // Ensure the containing Card is small enough that we don't expand too much,
            // resulting in our custom margin being ignored.
            tester.set_surface_size(Some(Size::new(400.0, 400.0)));
            let source = new_source(tester, true, 0);
            let theme = m3(&tester.cell.borrow());

            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(
                        PaginatedDataTable::new(standard_columns(), erased(&source))
                            .header(Text::new("Test table"))
                            .rows_per_page(2)
                            .available_rows_per_page(vec![2, 4])
                            .on_rows_per_page_changed(|_app, _rows_per_page| {})
                            .on_page_changed(|_app, _row_index| {})
                            .on_select_all(|_app, _value| {})
                            .horizontal_margin(custom_horizontal_margin)
                            .checkbox_horizontal_margin(custom_checkbox_horizontal_margin),
                    )
                    .into_widget(),
            ));

            // Custom checkbox padding.
            let checkbox = find::by_type::<Checkbox>().first();
            let padding = find::ancestor(checkbox.clone(), find::by_type::<Padding>()).first();
            assert_eq!(
                tester.get_rect(checkbox.clone()).left - tester.get_rect(padding.clone()).left,
                custom_checkbox_horizontal_margin
            );
            assert_eq!(
                tester.get_rect(padding).right - tester.get_rect(checkbox).right,
                custom_checkbox_horizontal_margin
            );

            // Custom first column padding.
            let padding = find::widget_with_text::<Padding>("Frozen yogurt (0)").first();
            // DataTable wraps its DataCells in an Align widget.
            let cell_content = find::widget_with_text::<Align>("Frozen yogurt (0)");
            assert_eq!(
                tester.get_rect(cell_content).left - tester.get_rect(padding).left,
                custom_horizontal_margin
            );
            tester.set_surface_size(None);
        });
    }
}
