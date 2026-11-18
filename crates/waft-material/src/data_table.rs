//! Flutter counterpart: material_ui's `data_table.dart`.
//!
//! Dart's `_NullTableColumnWidth` and `_NullWidget` are placeholders that fill the lists
//! `build` then writes cell by cell; here the lists are built with their values, so neither
//! exists.

use std::collections::HashMap;
use std::f64::consts::PI;
use std::fmt;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Duration;

use waft_animation::{
    Animatable, Animation, AnimationBehavior, AnimationController, AnimationRef, AnimationStatus,
    AnimationStatusListener, CurveTween, CurvedAnimation, Curves, Drive, Tween,
};
use waft_embedder::{Clip, Color, Matrix4, Rect, TextDirection};
use waft_foundation::{App, Handle, Listener, LocalKey, Ref, UniqueKey, ValueChanged, ValueKey};
use waft_gestures::{
    GestureLongPressCallback, GestureTapCallback, GestureTapCancelCallback, GestureTapDownCallback,
};
use waft_painting::{
    AlignmentGeometry, Border, BoxDecoration, DecorationRef, EdgeInsetsDirectional,
    EdgeInsetsGeometry, TextStyle,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, FixedColumnWidth, IntrinsicColumnWidth,
    MainAxisAlignment, RenderBox, RenderTable, TableCellParentData, TableCellVerticalAlignment,
    TableColumnWidthRef,
};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_services::MouseCursorRef;
use waft_widgets::{
    AnimatedDefaultTextStyle, BuildContext, Center, Container, DefaultTextStyle, FadeTransition,
    Icon, IntoWidget, KeyRef, Padding, Row, Semantics, SizedBox, State, StateData, StatefulWidget,
    StatelessWidget, Table, TableBorder, TableCell, TableRow, TickerProviderStateMixin,
    TickerProviderStateMixinData, Transform, WidgetRef, WidgetState, WidgetStateProperty,
    WidgetStatePropertyRef, WidgetStates, debug_check_has_table,
};

use crate::checkbox::Checkbox;
use crate::constants::K_MIN_INTERACTIVE_DIMENSION;
use crate::data_table_theme::{DataTableSortIconBuilder, DataTableTheme};
use crate::debug::debug_check_has_material;
use crate::divider::Divider;
use crate::dropdown::DropdownButtonHideUnderline;
use crate::icons::Icons;
use crate::ink_well::{InkResponse, InkWell};
use crate::material::{Material, MaterialType, RectCallback};
use crate::theme::Theme;
use crate::tooltip::Tooltip;

/// Signature for [`DataColumn::on_sort`] and `DataTable`'s sort callbacks.
pub type DataColumnSortCallback = Rc<dyn Fn(&mut App, i32, bool)>;

/// Column configuration for a [`DataTable`].
///
/// One column configuration must be provided for each column to display in the table. The list
/// of [`DataColumn`] objects is passed as the `columns` argument to the [`DataTable::new`]
/// constructor.
#[derive(Clone)]
pub struct DataColumn {
    /// The column heading.
    ///
    /// Typically, this will be a `Text` widget. It could also be an `Icon` (typically using size
    /// 18), or a `Row` with an icon and some text.
    ///
    /// The [label] is placed within a [`Row`] along with an arrow icon (if [`on_sort`] is
    /// non-null). By default, [label] is only as wide as needed to fit its contents.
    pub label: WidgetRef,

    /// How the horizontal extents of this column of the table should be determined.
    ///
    /// The [`FixedColumnWidth`] class can be used to specify a specific width in pixels. This is
    /// the cheapest way to size a table's columns.
    ///
    /// The layout performance of the table depends critically on which column sizing algorithms
    /// are used here. In particular, [`IntrinsicColumnWidth`] is quite expensive because it needs
    /// to measure each cell in the column to determine the intrinsic size of the column.
    ///
    /// If this is `None`, the column width is determined by the table, as described by
    /// [`DataTable`].
    pub column_width: Option<TableColumnWidthRef>,

    /// The column heading's tooltip.
    ///
    /// This is a longer description of the column heading, for cases where the heading might have
    /// been abbreviated to keep the column width to a reasonable size.
    pub tooltip: Option<String>,

    /// Whether this column represents numeric data or not.
    ///
    /// The contents of cells of columns containing numeric data are right-aligned.
    pub numeric: bool,

    /// Called when the user asks to sort the table using this column.
    ///
    /// If non-null, the user can sort the table using this column. The callback is passed the
    /// column index and whether the new sort is ascending.
    pub on_sort: Option<DataColumnSortCallback>,

    /// The cursor for a mouse pointer when it enters or is hovering over the heading row.
    ///
    /// [`WidgetStateProperty::resolve`] is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Disabled`].
    ///
    /// If this is `None`, then the value of [`DataTableThemeData::heading_cell_cursor`] is used.
    /// If that's `None` too, then `SystemMouseCursors::click` is used.
    pub mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,

    /// Defines the horizontal layout of the [`label`](Self::label) and sort indicator in the
    /// heading row.
    ///
    /// If [`DataTable::heading_row_alignment`] is set, it takes precedence over this value.
    pub heading_row_alignment: Option<MainAxisAlignment>,
}

impl DataColumn {
    /// Creates the configuration for a column of a [`DataTable`]; Dart's optional named
    /// arguments are the setters.
    pub fn new<K>(label: impl IntoWidget<K>) -> DataColumn {
        DataColumn {
            label: label.into_widget(),
            column_width: None,
            tooltip: None,
            numeric: false,
            on_sort: None,
            mouse_cursor: None,
            heading_row_alignment: None,
        }
    }

    /// Dart `DataColumn(columnWidth:)`.
    pub fn column_width(mut self, column_width: TableColumnWidthRef) -> DataColumn {
        self.column_width = Some(column_width);
        self
    }

    /// Dart `DataColumn(tooltip:)`.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> DataColumn {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Dart `DataColumn(numeric:)`.
    pub fn numeric(mut self, numeric: bool) -> DataColumn {
        self.numeric = numeric;
        self
    }

    /// Dart `DataColumn(onSort:)`.
    pub fn on_sort(mut self, on_sort: impl Fn(&mut App, i32, bool) + 'static) -> DataColumn {
        self.on_sort = Some(Rc::new(on_sort));
        self
    }

    /// Dart `DataColumn(mouseCursor:)`.
    pub fn mouse_cursor(
        mut self,
        mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>>,
    ) -> DataColumn {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `DataColumn(headingRowAlignment:)`.
    pub fn heading_row_alignment(mut self, heading_row_alignment: MainAxisAlignment) -> DataColumn {
        self.heading_row_alignment = Some(heading_row_alignment);
        self
    }

    /// Dart's `_debugInteractive`.
    fn debug_interactive(&self) -> bool {
        self.on_sort.is_some()
    }
}

impl fmt::Debug for DataColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataColumn")
            .field("label", &self.label)
            .field("tooltip", &self.tooltip)
            .field("numeric", &self.numeric)
            .field("sortable", &self.on_sort.is_some())
            .finish_non_exhaustive()
    }
}

/// Row configuration and cell data for a [`DataTable`].
///
/// One row configuration must be provided for each row to display in the table. The list of
/// [`DataRow`] objects is passed as the `rows` argument to the [`DataTable::new`] constructor.
///
/// The data for this row of the table is provided in the [`cells`](Self::cells) property of the
/// [`DataRow`] object.
#[derive(Clone)]
pub struct DataRow {
    /// A [`LocalKey`] that identifies the row.
    ///
    /// Rows must have a key if the table can be reordered.
    pub key: Option<Rc<dyn LocalKey>>,

    /// Called when the user selects or unselects a selectable row.
    ///
    /// If this is not null, then the row is selectable. The current selection state of the row is
    /// given by [`selected`](Self::selected).
    ///
    /// If any row is selectable, then the table's heading row will have a checkbox that can be
    /// checked to select all selectable rows (and which is checked if all the rows are selected),
    /// and each subsequent row will have a checkbox to toggle just that row.
    ///
    /// A row whose [`on_select_changed`](Self::on_select_changed) callback is null is ignored for
    /// the purposes of determining the state of the "all" checkbox, and its checkbox is disabled.
    ///
    /// If a [`DataCell`] in the row has its [`DataCell::on_tap`] callback defined, that callback
    /// behavior overrides the gesture behavior of the row for that particular cell.
    pub on_select_changed: Option<ValueChanged<Option<bool>>>,

    /// Called if the row is long-pressed.
    ///
    /// If a [`DataCell`] in the row has its [`DataCell::on_tap`], [`DataCell::on_double_tap`],
    /// [`DataCell::on_long_press`], [`DataCell::on_tap_cancel`] or [`DataCell::on_tap_down`]
    /// callback defined, that callback behavior overrides the gesture behavior of the row for
    /// that particular cell.
    pub on_long_press: Option<GestureLongPressCallback>,

    /// Called when a pointer enters or exits the row.
    ///
    /// The boolean value passed to the callback is true if a pointer has entered the row and
    /// false if it has exited.
    pub on_hover: Option<ValueChanged<bool>>,

    /// Whether the row is selected.
    ///
    /// If [`on_select_changed`](Self::on_select_changed) is non-null for any row in the table,
    /// then a checkbox is shown at the start of each row. If the row is selected
    /// (`true`), the checkbox will be checked and the row will be highlighted.
    ///
    /// Otherwise, the checkbox, if present, will not be checked.
    pub selected: bool,

    /// The data for this row.
    ///
    /// There must be exactly as many cells as there are columns in the table.
    pub cells: Vec<DataCell>,

    /// The color for the row.
    ///
    /// By default, the color is transparent unless selected. Selected rows has a grey translucent
    /// color.
    ///
    /// The effective color can depend on the [`WidgetState`] state, if the row is selected,
    /// pressed, hovered, focused, disabled or enabled. The color is painted as an overlay to the
    /// row. To make sure that the row's [`InkWell`] is visible (when pressed, hovered and
    /// focused), it is recommended to use a translucent color.
    pub color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The cursor for a mouse pointer when it enters or is hovering over the data row.
    ///
    /// [`WidgetStateProperty::resolve`] is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Selected`].
    ///
    /// If this is `None`, then the value of [`DataTableThemeData::data_row_cursor`] is used. If
    /// that's `None` too, then `SystemMouseCursors::click` is used.
    pub mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,
}

impl DataRow {
    /// Creates the configuration for a row of a [`DataTable`]; Dart's optional named arguments
    /// are the setters.
    pub fn new(cells: Vec<DataCell>) -> DataRow {
        DataRow {
            key: None,
            on_select_changed: None,
            on_long_press: None,
            on_hover: None,
            selected: false,
            cells,
            color: None,
            mouse_cursor: None,
        }
    }

    /// Dart's `DataRow.byIndex`: creates the configuration for a row of a [`DataTable`], deriving
    /// the key from a row index.
    ///
    /// The row is keyed with a `ValueKey<Option<i32>>` of `index`.
    pub fn by_index(index: Option<i32>, cells: Vec<DataCell>) -> DataRow {
        DataRow {
            key: Some(Rc::new(ValueKey::new(index))),
            ..DataRow::new(cells)
        }
    }

    /// Dart `DataRow(key:)`.
    pub fn key(mut self, key: impl LocalKey + 'static) -> DataRow {
        self.key = Some(Rc::new(key));
        self
    }

    /// Dart `DataRow(selected:)`.
    pub fn selected(mut self, selected: bool) -> DataRow {
        self.selected = selected;
        self
    }

    /// Dart `DataRow(onSelectChanged:)`.
    pub fn on_select_changed(
        mut self,
        on_select_changed: impl Fn(&mut App, Option<bool>) + 'static,
    ) -> DataRow {
        self.on_select_changed = Some(Rc::new(on_select_changed));
        self
    }

    /// Dart `DataRow(onLongPress:)`.
    pub fn on_long_press(mut self, on_long_press: GestureLongPressCallback) -> DataRow {
        self.on_long_press = Some(on_long_press);
        self
    }

    /// Dart `DataRow(onHover:)`.
    pub fn on_hover(mut self, on_hover: impl Fn(&mut App, bool) + 'static) -> DataRow {
        self.on_hover = Some(Rc::new(on_hover));
        self
    }

    /// Dart `DataRow(color:)`.
    pub fn color(mut self, color: WidgetStatePropertyRef<Option<Color>>) -> DataRow {
        self.color = Some(color);
        self
    }

    /// Dart `DataRow(mouseCursor:)`.
    pub fn mouse_cursor(
        mut self,
        mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>>,
    ) -> DataRow {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart's `_debugInteractive`.
    fn debug_interactive(&self) -> bool {
        self.on_select_changed.is_some() || self.cells.iter().any(DataCell::debug_interactive)
    }
}

impl fmt::Debug for DataRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataRow")
            .field("key", &self.key)
            .field("selected", &self.selected)
            .field("selectable", &self.on_select_changed.is_some())
            .field("cells", &self.cells)
            .finish_non_exhaustive()
    }
}

/// The data for a cell of a [`DataTable`].
///
/// One list of [`DataCell`] objects must be provided for each [`DataRow`] in the [`DataTable`],
/// in the new [`DataRow`] constructor's `cells` argument. This list must have the same length as
/// the [`DataTable::columns`] argument to the [`DataTable`].
#[derive(Clone)]
pub struct DataCell {
    /// The data for the row.
    ///
    /// Typically a `Text` widget or a `DropdownButton` widget.
    ///
    /// If the cell has no data, then a `Text` widget with placeholder text should be provided
    /// instead, and then the [`placeholder`](Self::placeholder) argument should be set to true.
    pub child: WidgetRef,

    /// Whether the [`child`](Self::child) is actually a placeholder.
    ///
    /// If this is true, the default text style for the cell is changed to be appropriate for
    /// placeholder text.
    pub placeholder: bool,

    /// Whether to show an edit icon at the end of the cell.
    ///
    /// This does not make the cell actually editable; the caller must implement editing behavior
    /// if desired (initiated from the [`on_tap`](Self::on_tap) callback).
    ///
    /// If this is set, [`on_tap`](Self::on_tap) should also be set, otherwise tapping the icon
    /// will have no effect.
    pub show_edit_icon: bool,

    /// Called if the cell is tapped.
    ///
    /// If non-null, tapping the cell will call this callback. If null (including
    /// [`on_double_tap`](Self::on_double_tap), [`on_long_press`](Self::on_long_press),
    /// [`on_tap_cancel`](Self::on_tap_cancel) and [`on_tap_down`](Self::on_tap_down)), tapping the
    /// cell will attempt to select the row (if [`DataRow::on_select_changed`] is provided).
    pub on_tap: Option<GestureTapCallback>,

    /// Called when the cell is double tapped.
    ///
    /// If non-null, tapping the cell will call this callback. If null, tapping the cell will
    /// attempt to select the row (if [`DataRow::on_select_changed`] is provided).
    pub on_double_tap: Option<GestureTapCallback>,

    /// Called if the cell is long-pressed.
    ///
    /// If non-null, long-pressing the cell will call this callback. If null, long-pressing the
    /// cell will attempt to select the row (if [`DataRow::on_select_changed`] is provided).
    pub on_long_press: Option<GestureLongPressCallback>,

    /// Called if the cell is tapped down.
    ///
    /// If non-null, tapping down the cell will call this callback. If null, tapping down the
    /// cell will attempt to select the row (if [`DataRow::on_select_changed`] is provided).
    pub on_tap_down: Option<GestureTapDownCallback>,

    /// Called if the user cancels a tap was started on cell.
    ///
    /// If non-null, canceling the tap gesture will call this callback. If null, canceling the
    /// tap gesture will attempt to select the row (if [`DataRow::on_select_changed`] is
    /// provided).
    pub on_tap_cancel: Option<GestureTapCancelCallback>,
}

impl DataCell {
    /// Creates a data cell for a [`DataTable`]; Dart's optional named arguments are the setters.
    pub fn new<K>(child: impl IntoWidget<K>) -> DataCell {
        DataCell {
            child: child.into_widget(),
            placeholder: false,
            show_edit_icon: false,
            on_tap: None,
            on_double_tap: None,
            on_long_press: None,
            on_tap_down: None,
            on_tap_cancel: None,
        }
    }

    /// A data cell that has no content and that has zero width and height.
    ///
    /// This cell is used by default for cells in a [`DataRow`] where no cell is specified.
    ///
    /// Dart's `static const DataCell empty`.
    pub fn empty() -> DataCell {
        DataCell::new(SizedBox::shrink())
    }

    /// Dart `DataCell(placeholder:)`.
    pub fn placeholder(mut self, placeholder: bool) -> DataCell {
        self.placeholder = placeholder;
        self
    }

    /// Dart `DataCell(showEditIcon:)`.
    pub fn show_edit_icon(mut self, show_edit_icon: bool) -> DataCell {
        self.show_edit_icon = show_edit_icon;
        self
    }

    /// Dart `DataCell(onTap:)`.
    pub fn on_tap(mut self, on_tap: GestureTapCallback) -> DataCell {
        self.on_tap = Some(on_tap);
        self
    }

    /// Dart `DataCell(onDoubleTap:)`.
    pub fn on_double_tap(mut self, on_double_tap: GestureTapCallback) -> DataCell {
        self.on_double_tap = Some(on_double_tap);
        self
    }

    /// Dart `DataCell(onLongPress:)`.
    pub fn on_long_press(mut self, on_long_press: GestureLongPressCallback) -> DataCell {
        self.on_long_press = Some(on_long_press);
        self
    }

    /// Dart `DataCell(onTapDown:)`.
    pub fn on_tap_down(mut self, on_tap_down: GestureTapDownCallback) -> DataCell {
        self.on_tap_down = Some(on_tap_down);
        self
    }

    /// Dart `DataCell(onTapCancel:)`.
    pub fn on_tap_cancel(mut self, on_tap_cancel: GestureTapCancelCallback) -> DataCell {
        self.on_tap_cancel = Some(on_tap_cancel);
        self
    }

    /// Dart's `_debugInteractive`.
    fn debug_interactive(&self) -> bool {
        self.on_tap.is_some()
            || self.on_double_tap.is_some()
            || self.on_long_press.is_some()
            || self.on_tap_down.is_some()
            || self.on_tap_cancel.is_some()
    }
}

impl fmt::Debug for DataCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataCell")
            .field("child", &self.child)
            .field("placeholder", &self.placeholder)
            .field("show_edit_icon", &self.show_edit_icon)
            .finish_non_exhaustive()
    }
}

/// The key of the heading row, Dart's `static final LocalKey _headingRowKey = UniqueKey()`.
fn heading_row_key() -> Rc<dyn LocalKey> {
    static KEY: OnceLock<UniqueKey> = OnceLock::new();
    Rc::new(*KEY.get_or_init(UniqueKey::new))
}

const HEADING_ROW_HEIGHT: f64 = 56.0;
const HORIZONTAL_MARGIN: f64 = 24.0;
const COLUMN_SPACING: f64 = 56.0;
const SORT_ARROW_PADDING: f64 = 2.0;
const DIVIDER_THICKNESS: f64 = 1.0;
const SORT_ARROW_ANIMATION_DURATION: Duration = Duration::from_millis(150);

/// A Material Design data table.
///
/// Displaying data in a table is expected to be a common operation. For example, a list of
/// users, with each user having a name, an age, and so on.
///
/// If the [`on_select_changed`](DataRow::on_select_changed) of any row is set, a checkbox
/// column is shown first, and a [`DataRow`] with a null `on_select_changed` is disabled.
///
/// The [`DataTable`] widget does not scroll on its own: if the table might be too large for its
/// parent, wrap it in a scrollable. A [`DataTable`] must have a `Material` ancestor if any of
/// its columns or rows are interactive.
///
/// The width of each column is determined by the algorithm in `Table`: by default, a column
/// that is the only non-numeric column gets an [`IntrinsicColumnWidth`] with a flex of 1, and
/// every other column an [`IntrinsicColumnWidth`] without one, unless the [`DataColumn`] gives a
/// [`column_width`](DataColumn::column_width).
///
/// See also:
///
///  * `PaginatedDataTable`, which shows part of the data in a data table and provides controls
///    for paging through the remainder of the data.
///  * `TableRowInkWell`, a widget used to provide ink to a table row.
#[derive(Clone)]
pub struct DataTable {
    pub key: Option<KeyRef>,

    /// The configuration and labels for the columns in the table.
    pub columns: Vec<DataColumn>,

    /// The current primary sort key's column.
    ///
    /// If non-null, indicates that the indicated column is the column by which the data is
    /// sorted. The number must correspond to the index of the relevant column in [`columns`].
    ///
    /// Setting this will cause the relevant column to have a sort indicator displayed.
    ///
    /// When this is null, it implies that the table's sort order does not correspond to any of
    /// the columns.
    ///
    /// The direction of the sort is specified using [`sort_ascending`](Self::sort_ascending).
    pub sort_column_index: Option<usize>,

    /// Whether the column mentioned in [`sort_column_index`](Self::sort_column_index), if any,
    /// is sorted in ascending order.
    ///
    /// If true, the order is ascending (meaning the rows with the smallest values for the
    /// current sort column are first in the table).
    ///
    /// If false, the order is descending (meaning the rows with the smallest values for the
    /// current sort column are last in the table).
    ///
    /// Ascending order is represented by an upwards-facing arrow.
    pub sort_ascending: bool,

    /// Overrides the default sort indicator (an arrow) shown in a sortable column's heading.
    ///
    /// If null, the default arrow is used. Otherwise it is called with whether the column is
    /// sorted and whether it is sorted ascending.
    pub sort_icon_builder: Option<DataTableSortIconBuilder>,

    /// Invoked when the user selects or unselects every row, using the checkbox in the heading
    /// row.
    ///
    /// If this is null, then the [`DataRow::on_select_changed`] callback of every row in the
    /// table is invoked appropriately instead.
    ///
    /// To control whether a particular row is selectable or not, see
    /// [`DataRow::on_select_changed`]. This callback is only relevant if any row is selectable.
    pub on_select_all: Option<ValueChanged<Option<bool>>>,

    /// The background and border decoration for the table.
    pub decoration: Option<DecorationRef>,

    /// The background color for the data rows.
    ///
    /// The effective background color can be made to depend on the [`WidgetState`] state, i.e. if
    /// the row is selected, pressed, hovered, focused, disabled or enabled. The color is painted
    /// as an overlay to the row. To make sure that the row's [`InkWell`] is visible (when
    /// pressed, hovered and focused), it is recommended to use a translucent color.
    ///
    /// If [`DataRow::color`] is non-null for a row, that color is used for that row instead.
    pub data_row_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The minimum height of each row (excluding the row that contains column headings).
    ///
    /// This value is optional and defaults to `K_MIN_INTERACTIVE_DIMENSION` if not specified.
    pub data_row_min_height: Option<f64>,

    /// The maximum height of each row (excluding the row that contains column headings).
    ///
    /// This value is optional and defaults to `K_MIN_INTERACTIVE_DIMENSION` if not specified.
    pub data_row_max_height: Option<f64>,

    /// The text style for data rows.
    ///
    /// By default, the text style is `TextTheme::body_medium`.
    pub data_text_style: Option<TextStyle>,

    /// The background color for the heading row.
    ///
    /// The effective background color can be made to depend on the [`WidgetState`] state, i.e. if
    /// the row is pressed, hovered, focused when sorted. The color is painted as an overlay to
    /// the row.
    pub heading_row_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The height of the heading row.
    ///
    /// This value is optional and defaults to 56.0 if not specified.
    pub heading_row_height: Option<f64>,

    /// The text style for the heading row.
    ///
    /// By default, the text style is `TextTheme::title_small`.
    pub heading_text_style: Option<TextStyle>,

    /// The horizontal margin between the edges of the table and the content in the first and
    /// last cells of each row.
    ///
    /// When a checkbox is displayed, it is also the margin between the checkbox the content in
    /// the first data column.
    ///
    /// This value defaults to 24.0 to adhere to the Material Design specifications.
    ///
    /// If [`checkbox_horizontal_margin`](Self::checkbox_horizontal_margin) is null, then
    /// [`horizontal_margin`](Self::horizontal_margin) is also the margin between the edge of the
    /// table and the checkbox, as well as the margin between the checkbox and the content in the
    /// first data column.
    pub horizontal_margin: Option<f64>,

    /// The horizontal margin between the contents of each data column.
    ///
    /// This value defaults to 56.0 to adhere to the Material Design specifications.
    pub column_spacing: Option<f64>,

    /// Whether the widget should display checkboxes for selectable rows.
    ///
    /// If true, a [`Checkbox`] will be placed at the beginning of each row that is selectable.
    /// However, if [`DataRow::on_select_changed`] is not set for any row, checkboxes will not be
    /// placed, even if this value is true.
    ///
    /// If false, all rows will not display a [`Checkbox`].
    pub show_checkbox_column: bool,

    /// The data to show in each row (excluding the row that contains the column headings).
    ///
    /// The list may be empty.
    pub rows: Vec<DataRow>,

    /// The thickness of the divider between rows.
    ///
    /// This value is optional and defaults to 1.0 if not specified.
    pub divider_thickness: Option<f64>,

    /// Whether a border at the bottom of the table is displayed.
    ///
    /// By default, a border is not shown at the bottom to allow for a border around the table
    /// defined by [`decoration`](Self::decoration).
    pub show_bottom_border: bool,

    /// Horizontal margin around the checkbox, if it is displayed.
    ///
    /// If null, then [`horizontal_margin`](Self::horizontal_margin) is used as the margin
    /// between the edge of the table and the checkbox, as well as the margin between the
    /// checkbox and the content in the first data column. This value defaults to 24.0.
    pub checkbox_horizontal_margin: Option<f64>,

    /// The style to use when painting the boundary and interior divisions of the table.
    pub border: Option<TableBorder>,

    /// {@macro flutter.material.Material.clipBehavior}
    ///
    /// This can be used to clip the content within the border of the [`DataTable`].
    ///
    /// Defaults to [`Clip::None`].
    pub clip_behavior: Clip,

    /// Dart's `final int? _onlyTextColumn`: the index of the only non-numeric column, if there is
    /// exactly one.
    only_text_column: Option<usize>,
}

impl DataTable {
    /// Creates a widget describing a data table; Dart's optional named arguments are the
    /// setters.
    ///
    /// The [`columns`](Self::columns) argument must be a list of as many [`DataColumn`] objects
    /// as the table is to have columns, ignoring the leading checkbox column which may be
    /// omitted. The [`rows`](Self::rows) argument must be a list of [`DataRow`] objects to be
    /// shown, each with a [`DataRow::cells`] list of the same length as `columns`.
    pub fn new(columns: Vec<DataColumn>, rows: Vec<DataRow>) -> DataTable {
        let only_text_column = Self::init_only_text_column(&columns);
        DataTable {
            key: None,
            columns,
            sort_column_index: None,
            sort_ascending: true,
            sort_icon_builder: None,
            on_select_all: None,
            decoration: None,
            data_row_color: None,
            data_row_min_height: None,
            data_row_max_height: None,
            data_text_style: None,
            heading_row_color: None,
            heading_row_height: None,
            heading_text_style: None,
            horizontal_margin: None,
            column_spacing: None,
            show_checkbox_column: true,
            rows,
            divider_thickness: None,
            show_bottom_border: false,
            checkbox_horizontal_margin: None,
            border: None,
            clip_behavior: Clip::None,
            only_text_column,
        }
    }

    /// Dart `DataTable(key:)`.
    pub fn key(mut self, key: KeyRef) -> DataTable {
        self.key = Some(key);
        self
    }

    /// Dart `DataTable(sortColumnIndex:)`.
    pub fn sort_column_index(mut self, sort_column_index: usize) -> DataTable {
        self.sort_column_index = Some(sort_column_index);
        self
    }

    /// Dart `DataTable(sortAscending:)`.
    pub fn sort_ascending(mut self, sort_ascending: bool) -> DataTable {
        self.sort_ascending = sort_ascending;
        self
    }

    /// Dart `DataTable(sortIconBuilder:)`.
    pub fn sort_icon_builder(
        mut self,
        sort_icon_builder: impl Fn(&mut App, BuildContext, bool, bool) -> WidgetRef + 'static,
    ) -> DataTable {
        self.sort_icon_builder = Some(Rc::new(sort_icon_builder));
        self
    }

    /// Dart `DataTable(onSelectAll:)`.
    pub fn on_select_all(
        mut self,
        on_select_all: impl Fn(&mut App, Option<bool>) + 'static,
    ) -> DataTable {
        self.on_select_all = Some(Rc::new(on_select_all));
        self
    }

    /// Dart `DataTable(decoration:)`.
    pub fn decoration(mut self, decoration: impl Into<DecorationRef>) -> DataTable {
        self.decoration = Some(decoration.into());
        self
    }

    /// Dart `DataTable(dataRowColor:)`.
    pub fn data_row_color(
        mut self,
        data_row_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> DataTable {
        self.data_row_color = Some(data_row_color);
        self
    }

    /// Dart's deprecated `DataTable(dataRowHeight:)`: it sets
    /// [`data_row_min_height`](Self::data_row_min_height) and
    /// [`data_row_max_height`](Self::data_row_max_height) to one value.
    ///
    /// Dart asserts that the argument is not given with either of them; a fluent chain has no
    /// one call to compare, so a later setter replaces what an earlier one wrote.
    #[deprecated = "Migrate to use data_row_min_height and data_row_max_height instead. This feature was deprecated after v3.7.0-5.0.pre."]
    pub fn data_row_height(mut self, data_row_height: f64) -> DataTable {
        self.data_row_min_height = Some(data_row_height);
        self.data_row_max_height = Some(data_row_height);
        self
    }

    /// Dart `DataTable(dataRowMinHeight:)`.
    pub fn data_row_min_height(mut self, data_row_min_height: f64) -> DataTable {
        self.data_row_min_height = Some(data_row_min_height);
        self
    }

    /// Dart `DataTable(dataRowMaxHeight:)`.
    pub fn data_row_max_height(mut self, data_row_max_height: f64) -> DataTable {
        self.data_row_max_height = Some(data_row_max_height);
        self
    }

    /// Dart `DataTable(dataTextStyle:)`.
    pub fn data_text_style(mut self, data_text_style: TextStyle) -> DataTable {
        self.data_text_style = Some(data_text_style);
        self
    }

    /// Dart `DataTable(headingRowColor:)`.
    pub fn heading_row_color(
        mut self,
        heading_row_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> DataTable {
        self.heading_row_color = Some(heading_row_color);
        self
    }

    /// Dart `DataTable(headingRowHeight:)`.
    pub fn heading_row_height(mut self, heading_row_height: f64) -> DataTable {
        self.heading_row_height = Some(heading_row_height);
        self
    }

    /// Dart `DataTable(headingTextStyle:)`.
    pub fn heading_text_style(mut self, heading_text_style: TextStyle) -> DataTable {
        self.heading_text_style = Some(heading_text_style);
        self
    }

    /// Dart `DataTable(horizontalMargin:)`.
    pub fn horizontal_margin(mut self, horizontal_margin: f64) -> DataTable {
        self.horizontal_margin = Some(horizontal_margin);
        self
    }

    /// Dart `DataTable(columnSpacing:)`.
    pub fn column_spacing(mut self, column_spacing: f64) -> DataTable {
        self.column_spacing = Some(column_spacing);
        self
    }

    /// Dart `DataTable(showCheckboxColumn:)`.
    pub fn show_checkbox_column(mut self, show_checkbox_column: bool) -> DataTable {
        self.show_checkbox_column = show_checkbox_column;
        self
    }

    /// Dart `DataTable(showBottomBorder:)`.
    pub fn show_bottom_border(mut self, show_bottom_border: bool) -> DataTable {
        self.show_bottom_border = show_bottom_border;
        self
    }

    /// Dart `DataTable(dividerThickness:)`.
    pub fn divider_thickness(mut self, divider_thickness: f64) -> DataTable {
        debug_assert!(divider_thickness >= 0.0);
        self.divider_thickness = Some(divider_thickness);
        self
    }

    /// Dart `DataTable(checkboxHorizontalMargin:)`.
    pub fn checkbox_horizontal_margin(mut self, checkbox_horizontal_margin: f64) -> DataTable {
        self.checkbox_horizontal_margin = Some(checkbox_horizontal_margin);
        self
    }

    /// Dart `DataTable(border:)`.
    pub fn border(mut self, border: TableBorder) -> DataTable {
        self.border = Some(border);
        self
    }

    /// Dart `DataTable(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> DataTable {
        self.clip_behavior = clip_behavior;
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

    /// Dart's `_initOnlyTextColumn`: the index of the only column that is non-numeric, if there
    /// is exactly one, otherwise `None`.
    fn init_only_text_column(columns: &[DataColumn]) -> Option<usize> {
        let mut result = None;
        for (index, column) in columns.iter().enumerate() {
            if !column.numeric {
                if result.is_some() {
                    return None;
                }
                result = Some(index);
            }
        }
        result
    }

    /// Dart's `_debugInteractive`.
    fn debug_interactive(&self) -> bool {
        self.columns.iter().any(DataColumn::debug_interactive)
            || self.rows.iter().any(DataRow::debug_interactive)
    }

    fn handle_select_all(&self, app: &mut App, checked: Option<bool>, some_checked: bool) {
        // If some checkboxes are checked, all checkboxes are selected. Otherwise, use the new
        // checked value but default to false if it's null.
        let effective_checked = some_checked || checked.unwrap_or(false);
        if let Some(on_select_all) = &self.on_select_all {
            on_select_all(app, Some(effective_checked));
        } else {
            for row in &self.rows {
                if let Some(on_select_changed) = &row.on_select_changed
                    && row.selected != effective_checked
                {
                    on_select_changed(app, Some(effective_checked));
                }
            }
        }
    }

    #[expect(clippy::too_many_arguments, reason = "Dart's named arguments")]
    fn build_checkbox(
        &self,
        app: &mut App,
        context: BuildContext,
        checked: Option<bool>,
        on_row_tap: Option<Listener>,
        on_checkbox_changed: Option<ValueChanged<Option<bool>>>,
        overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        tristate: bool,
        row_mouse_cursor: Option<MouseCursorRef>,
    ) -> WidgetRef {
        let theme_data = Theme::of(app, context);
        let effective_horizontal_margin = self
            .horizontal_margin
            .or(theme_data.data_table_theme.horizontal_margin)
            .unwrap_or(HORIZONTAL_MARGIN);
        let effective_checkbox_horizontal_margin_start = self
            .checkbox_horizontal_margin
            .or(theme_data.data_table_theme.checkbox_horizontal_margin)
            .unwrap_or(effective_horizontal_margin);
        let effective_checkbox_horizontal_margin_end = self
            .checkbox_horizontal_margin
            .or(theme_data.data_table_theme.checkbox_horizontal_margin)
            .unwrap_or(effective_horizontal_margin / 2.0);
        let mut contents = Semantics::new()
            .container(true)
            .child(
                Padding::new(EdgeInsetsDirectional::only(
                    effective_checkbox_horizontal_margin_start,
                    0.0,
                    effective_checkbox_horizontal_margin_end,
                    0.0,
                ))
                .child(
                    Center::new()
                        .child(Checkbox::new(checked, on_checkbox_changed).tristate(tristate)),
                ),
            )
            .into_widget();
        if let Some(on_row_tap) = on_row_tap {
            let mut ink_well = TableRowInkWell::new().on_tap(on_row_tap);
            if let Some(overlay_color) = overlay_color {
                ink_well = ink_well.overlay_color(overlay_color);
            }
            if let Some(row_mouse_cursor) = row_mouse_cursor {
                ink_well = ink_well.mouse_cursor(row_mouse_cursor);
            }
            contents = ink_well.child(contents).into_widget();
        }
        TableCell::new(contents)
            .vertical_alignment(TableCellVerticalAlignment::Fill)
            .into_widget()
    }

    #[expect(clippy::too_many_arguments, reason = "Dart's named arguments")]
    fn build_heading_cell(
        &self,
        app: &mut App,
        context: BuildContext,
        padding: EdgeInsetsGeometry,
        label: WidgetRef,
        tooltip: Option<String>,
        numeric: bool,
        on_sort: Option<Listener>,
        sorted: bool,
        ascending: bool,
        overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        mouse_cursor: Option<MouseCursorRef>,
        heading_row_alignment: MainAxisAlignment,
    ) -> WidgetRef {
        let theme_data = Theme::of(app, context);
        let data_table_theme = DataTableTheme::of(app, context);
        let effective_sort_icon_builder = self
            .sort_icon_builder
            .clone()
            .or_else(|| data_table_theme.sort_icon_builder.clone());

        let mut row = Row::new().main_axis_alignment(heading_row_alignment);
        if numeric {
            row = row.text_direction(TextDirection::Rtl);
        }
        let mut children = Vec::new();
        if heading_row_alignment == MainAxisAlignment::Center && on_sort.is_some() {
            children.push(
                SizedBox::new()
                    .width(SortArrowState::ARROW_ICON_SIZE + SORT_ARROW_PADDING)
                    .into_widget(),
            );
        }
        children.push(label);
        if on_sort.is_some() {
            children.push(match effective_sort_icon_builder {
                Some(builder) => builder(app, context, sorted, ascending),
                None => SortArrow {
                    visible: sorted,
                    up: if sorted { Some(ascending) } else { None },
                    duration: SORT_ARROW_ANIMATION_DURATION,
                }
                .into_widget(),
            });
            children.push(SizedBox::new().width(SORT_ARROW_PADDING).into_widget());
        }
        let label = Semantics::new()
            .role(waft_embedder::SemanticsRole::ColumnHeader)
            .child(row.children(children))
            .into_widget();

        let effective_heading_text_style = self
            .heading_text_style
            .clone()
            .or_else(|| data_table_theme.heading_text_style.clone())
            .or_else(|| theme_data.data_table_theme.heading_text_style.clone())
            .unwrap_or_else(|| {
                theme_data
                    .text_theme
                    .title_small
                    .clone()
                    .expect("the theme's text theme has titleSmall")
            });
        let effective_heading_row_height = self
            .heading_row_height
            .or(data_table_theme.heading_row_height)
            .or(theme_data.data_table_theme.heading_row_height)
            .unwrap_or(HEADING_ROW_HEIGHT);
        let default_text_style = DefaultTextStyle::of(app, context);
        let mut label = Container::new()
            .padding(padding)
            .height(effective_heading_row_height)
            .alignment(if numeric {
                AlignmentGeometry::CENTER_RIGHT
            } else {
                AlignmentGeometry::CENTER_START
            })
            .child(
                AnimatedDefaultTextStyle::new(
                    label,
                    default_text_style
                        .style
                        .merge(Some(&effective_heading_text_style)),
                    SORT_ARROW_ANIMATION_DURATION,
                )
                .soft_wrap(false),
            )
            .into_widget();
        if let Some(tooltip) = tooltip {
            label = Tooltip::new().message(tooltip).child(label).into_widget();
        }

        let mut ink_well = InkWell::new();
        if let Some(on_sort) = on_sort {
            ink_well = ink_well.on_tap(on_sort);
        }
        if let Some(overlay_color) = overlay_color {
            ink_well = ink_well.overlay_color(overlay_color);
        }
        if let Some(mouse_cursor) = mouse_cursor {
            ink_well = ink_well.mouse_cursor(mouse_cursor);
        }
        ink_well.child(label).into_widget()
    }

    #[expect(clippy::too_many_arguments, reason = "Dart's named arguments")]
    #[allow(deprecated)] // Dart's `withOpacity`.
    fn build_data_cell(
        &self,
        app: &mut App,
        context: BuildContext,
        padding: EdgeInsetsGeometry,
        label: WidgetRef,
        numeric: bool,
        placeholder: bool,
        show_edit_icon: bool,
        on_tap: Option<GestureTapCallback>,
        on_select_changed: Option<Listener>,
        on_double_tap: Option<GestureTapCallback>,
        on_long_press: Option<GestureLongPressCallback>,
        on_tap_down: Option<GestureTapDownCallback>,
        on_tap_cancel: Option<GestureTapCancelCallback>,
        overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        on_row_long_press: Option<GestureLongPressCallback>,
        on_row_hover: Option<ValueChanged<bool>>,
        mouse_cursor: Option<MouseCursorRef>,
    ) -> WidgetRef {
        let theme_data = Theme::of(app, context);
        let data_table_theme = DataTableTheme::of(app, context);
        let mut label = label;
        if show_edit_icon {
            let icon = Icon::new(Some(Icons::edit())).size(18.0);
            label = waft_widgets::Expanded::new(label).into_widget();
            let mut row = Row::new();
            if numeric {
                row = row.text_direction(TextDirection::Rtl);
            }
            label = row.children(vec![label, icon.into_widget()]).into_widget();
        }

        let effective_data_text_style = self
            .data_text_style
            .clone()
            .or_else(|| data_table_theme.data_text_style.clone())
            .or_else(|| theme_data.data_table_theme.data_text_style.clone())
            .unwrap_or_else(|| {
                theme_data
                    .text_theme
                    .body_medium
                    .clone()
                    .expect("the theme's text theme has bodyMedium")
            });
        let effective_data_row_min_height = self
            .data_row_min_height
            .or(data_table_theme.data_row_min_height)
            .or(theme_data.data_table_theme.data_row_min_height)
            .unwrap_or(K_MIN_INTERACTIVE_DIMENSION);
        let effective_data_row_max_height = self
            .data_row_max_height
            .or(data_table_theme.data_row_max_height)
            .or(theme_data.data_table_theme.data_row_max_height)
            .unwrap_or(K_MIN_INTERACTIVE_DIMENSION);
        let default_text_style = DefaultTextStyle::of(app, context);
        let merged = default_text_style
            .style
            .merge(Some(&effective_data_text_style));
        let style = if placeholder {
            merged.copy_with().color(
                effective_data_text_style
                    .color
                    .as_ref()
                    .expect("a data text style has a color")
                    .color()
                    .with_opacity(0.6),
            )
        } else {
            merged
        };
        let mut label = Container::new()
            .padding(padding)
            .constraints(BoxConstraints {
                min_height: effective_data_row_min_height,
                max_height: effective_data_row_max_height,
                ..BoxConstraints::new()
            })
            .alignment(if numeric {
                AlignmentGeometry::CENTER_RIGHT
            } else {
                AlignmentGeometry::CENTER_START
            })
            .child(DefaultTextStyle::new(
                style,
                DropdownButtonHideUnderline::new(label),
            ))
            .into_widget();
        if on_tap.is_some()
            || on_double_tap.is_some()
            || on_long_press.is_some()
            || on_tap_down.is_some()
            || on_tap_cancel.is_some()
        {
            let mut ink_well = InkWell::new();
            if let Some(on_tap) = on_tap {
                ink_well = ink_well.on_tap(on_tap);
            }
            if let Some(on_double_tap) = on_double_tap {
                ink_well = ink_well.on_double_tap(on_double_tap);
            }
            if let Some(on_long_press) = on_long_press {
                ink_well = ink_well.on_long_press(on_long_press);
            }
            if let Some(on_tap_cancel) = on_tap_cancel {
                ink_well = ink_well.on_tap_cancel(on_tap_cancel);
            }
            if let Some(on_tap_down) = on_tap_down {
                ink_well = ink_well.on_tap_down(on_tap_down);
            }
            if let Some(overlay_color) = overlay_color {
                ink_well = ink_well.overlay_color(overlay_color);
            }
            label = ink_well.child(label).into_widget();
        } else if on_select_changed.is_some()
            || on_row_long_press.is_some()
            || on_row_hover.is_some()
        {
            let mut ink_well = TableRowInkWell::new();
            if let Some(on_select_changed) = on_select_changed {
                ink_well = ink_well.on_tap(on_select_changed);
            }
            if let Some(on_row_long_press) = on_row_long_press {
                ink_well = ink_well.on_long_press(on_row_long_press);
            }
            if let Some(on_row_hover) = on_row_hover {
                ink_well = ink_well.on_hover(on_row_hover);
            }
            if let Some(overlay_color) = overlay_color {
                ink_well = ink_well.overlay_color(overlay_color);
            }
            if let Some(mouse_cursor) = mouse_cursor {
                ink_well = ink_well.mouse_cursor(mouse_cursor);
            }
            label = ink_well.child(label).into_widget();
        }
        TableCell::new(label).into_widget()
    }
}

impl fmt::Debug for DataTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataTable")
            .field("columns", &self.columns)
            .field("rows", &self.rows)
            .field("sort_column_index", &self.sort_column_index)
            .field("sort_ascending", &self.sort_ascending)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for DataTable {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(!self.columns.is_empty());
        debug_assert!(
            self.sort_column_index
                .is_none_or(|index| index < self.columns.len())
        );
        debug_assert!(
            self.rows
                .iter()
                .all(|row| row.cells.len() == self.columns.len()),
            "All rows must have the same number of cells as there are header cells ({})",
            self.columns.len()
        );
        debug_assert!(
            self.divider_thickness
                .is_none_or(|thickness| thickness >= 0.0)
        );
        debug_assert!(match (self.data_row_min_height, self.data_row_max_height) {
            (Some(min), Some(max)) => max >= min,
            _ => true,
        });
    }

    #[allow(deprecated)] // Dart's `withOpacity`.
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(!self.debug_interactive() || debug_check_has_material(app, context));

        let theme = Theme::of(app, context);
        let data_table_theme = DataTableTheme::of(app, context);
        let effective_heading_row_color = self
            .heading_row_color
            .clone()
            .or_else(|| data_table_theme.heading_row_color.clone())
            .or_else(|| theme.data_table_theme.heading_row_color.clone());
        let effective_data_row_color = self
            .data_row_color
            .clone()
            .or_else(|| data_table_theme.data_row_color.clone())
            .or_else(|| theme.data_table_theme.data_row_color.clone());
        let primary = theme.color_scheme.primary;
        let default_row_color =
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Selected) {
                    return Some(primary.with_opacity(0.08));
                }
                None
            });
        let any_row_selectable = self.rows.iter().any(|row| row.on_select_changed.is_some());
        let display_checkbox_column = self.show_checkbox_column && any_row_selectable;
        let rows_with_checkbox: Vec<&DataRow> = if display_checkbox_column {
            self.rows
                .iter()
                .filter(|row| row.on_select_changed.is_some())
                .collect()
        } else {
            Vec::new()
        };
        let rows_checked = rows_with_checkbox.iter().filter(|row| row.selected).count();
        let all_checked = display_checkbox_column && rows_checked == rows_with_checkbox.len();
        let any_checked = display_checkbox_column && rows_checked > 0;
        let some_checked = any_checked && !all_checked;
        let effective_horizontal_margin = self
            .horizontal_margin
            .or(data_table_theme.horizontal_margin)
            .or(theme.data_table_theme.horizontal_margin)
            .unwrap_or(HORIZONTAL_MARGIN);
        let effective_checkbox_horizontal_margin_start = self
            .checkbox_horizontal_margin
            .or(data_table_theme.checkbox_horizontal_margin)
            .or(theme.data_table_theme.checkbox_horizontal_margin)
            .unwrap_or(effective_horizontal_margin);
        let effective_checkbox_horizontal_margin_end = self
            .checkbox_horizontal_margin
            .or(data_table_theme.checkbox_horizontal_margin)
            .or(theme.data_table_theme.checkbox_horizontal_margin)
            .unwrap_or(effective_horizontal_margin / 2.0);
        let effective_column_spacing = self
            .column_spacing
            .or(data_table_theme.column_spacing)
            .or(theme.data_table_theme.column_spacing)
            .unwrap_or(COLUMN_SPACING);

        let table_column_count = self.columns.len() + usize::from(display_checkbox_column);
        let mut table_columns: Vec<Option<TableColumnWidthRef>> = vec![None; table_column_count];
        // The first row is the heading; each cell is filled in below.
        let mut cells: Vec<Vec<Option<WidgetRef>>> =
            vec![vec![None; table_column_count]; self.rows.len() + 1];
        let mut row_decorations = Vec::with_capacity(self.rows.len() + 1);
        for index in 0..=self.rows.len() {
            let is_selected = index > 0 && self.rows[index - 1].selected;
            let is_disabled =
                index > 0 && any_row_selectable && self.rows[index - 1].on_select_changed.is_none();
            let mut states = WidgetStates::new();
            if is_selected {
                states.insert(WidgetState::Selected);
            }
            if is_disabled {
                states.insert(WidgetState::Disabled);
            }
            let resolved_data_row_color = if index > 0 {
                self.rows[index - 1]
                    .color
                    .clone()
                    .or_else(|| effective_data_row_color.clone())
                    .and_then(|color| color.resolve(&states))
            } else {
                None
            };
            let resolved_heading_row_color = effective_heading_row_color
                .as_ref()
                .and_then(|color| color.resolve(&WidgetStates::new()));
            let row_color = if index > 0 {
                resolved_data_row_color
            } else {
                resolved_heading_row_color
            };
            let border_side = Divider::create_border_side(
                app,
                Some(context),
                None,
                Some(
                    self.divider_thickness
                        .or(data_table_theme.divider_thickness)
                        .or(theme.data_table_theme.divider_thickness)
                        .unwrap_or(DIVIDER_THICKNESS),
                ),
            );
            let border = if self.show_bottom_border {
                Some(Border {
                    bottom: border_side,
                    ..Border::default()
                })
            } else if index == 0 {
                None
            } else {
                Some(Border {
                    top: border_side,
                    ..Border::default()
                })
            };
            let mut decoration = BoxDecoration::new();
            if let Some(border) = border {
                decoration = decoration.border(border);
            }
            if let Some(color) = row_color.or_else(|| default_row_color.resolve(&states)) {
                decoration = decoration.color(color);
            }
            row_decorations.push(decoration);
        }

        let mut display_column_index = 0;
        if display_checkbox_column {
            table_columns[0] = Some(Rc::new(FixedColumnWidth::new(
                effective_checkbox_horizontal_margin_start
                    + Checkbox::WIDTH
                    + effective_checkbox_horizontal_margin_end,
            )));
            let this = self.clone();
            cells[0][0] = Some(self.build_checkbox(
                app,
                context,
                if some_checked {
                    None
                } else {
                    Some(all_checked)
                },
                None,
                Some(Rc::new(move |app: &mut App, checked: Option<bool>| {
                    this.handle_select_all(app, checked, some_checked);
                })),
                None,
                true,
                None,
            ));
            for (row_index, row) in self.rows.iter().enumerate() {
                let mut states = WidgetStates::new();
                if row.selected {
                    states.insert(WidgetState::Selected);
                }
                let on_row_tap = row.on_select_changed.clone().map(|on_select_changed| {
                    let selected = row.selected;
                    Listener::new(move |app: &mut App| on_select_changed(app, Some(!selected)))
                });
                cells[row_index + 1][0] = Some(
                    self.build_checkbox(
                        app,
                        context,
                        Some(row.selected),
                        on_row_tap,
                        row.on_select_changed.clone(),
                        row.color
                            .clone()
                            .or_else(|| effective_data_row_color.clone()),
                        false,
                        row.mouse_cursor
                            .as_ref()
                            .and_then(|cursor| cursor.resolve(&states))
                            .or_else(|| {
                                data_table_theme
                                    .data_row_cursor
                                    .as_ref()
                                    .and_then(|cursor| cursor.resolve(&states))
                            }),
                    ),
                );
            }
            display_column_index += 1;
        }

        for (data_column_index, column) in self.columns.iter().enumerate() {
            let padding_start = if data_column_index == 0
                && display_checkbox_column
                && self.checkbox_horizontal_margin.is_none()
            {
                effective_horizontal_margin / 2.0
            } else if data_column_index == 0 {
                effective_horizontal_margin
            } else {
                effective_column_spacing / 2.0
            };
            let padding_end = if data_column_index == self.columns.len() - 1 {
                effective_horizontal_margin
            } else {
                effective_column_spacing / 2.0
            };
            let padding: EdgeInsetsGeometry =
                EdgeInsetsDirectional::only(padding_start, 0.0, padding_end, 0.0).into();
            table_columns[display_column_index] = Some(match &column.column_width {
                Some(width) => Rc::clone(width),
                None if Some(data_column_index) == self.only_text_column => {
                    Rc::new(IntrinsicColumnWidth::new(Some(1.0)))
                }
                None => Rc::new(IntrinsicColumnWidth::new(None)),
            });

            let mut header_states = WidgetStates::new();
            if column.on_sort.is_none() {
                header_states.insert(WidgetState::Disabled);
            }
            let on_sort = column.on_sort.clone().map(|on_sort| {
                let sort_column_index = self.sort_column_index;
                let sort_ascending = self.sort_ascending;
                Listener::new(move |app: &mut App| {
                    on_sort(
                        app,
                        data_column_index as i32,
                        sort_column_index != Some(data_column_index) || !sort_ascending,
                    )
                })
            });
            cells[0][display_column_index] = Some(
                self.build_heading_cell(
                    app,
                    context,
                    padding,
                    column.label.clone(),
                    column.tooltip.clone(),
                    column.numeric,
                    on_sort,
                    Some(data_column_index) == self.sort_column_index,
                    self.sort_ascending,
                    effective_heading_row_color.clone(),
                    column
                        .mouse_cursor
                        .as_ref()
                        .and_then(|cursor| cursor.resolve(&header_states))
                        .or_else(|| {
                            data_table_theme
                                .heading_cell_cursor
                                .as_ref()
                                .and_then(|cursor| cursor.resolve(&header_states))
                        }),
                    column
                        .heading_row_alignment
                        .or(data_table_theme.heading_row_alignment)
                        .unwrap_or(MainAxisAlignment::Start),
                ),
            );
            for (row_index, row) in self.rows.iter().enumerate() {
                let mut states = WidgetStates::new();
                if row.selected {
                    states.insert(WidgetState::Selected);
                }
                let cell = &row.cells[data_column_index];
                let on_select_changed = row.on_select_changed.clone().map(|on_select_changed| {
                    let selected = row.selected;
                    Listener::new(move |app: &mut App| on_select_changed(app, Some(!selected)))
                });
                cells[row_index + 1][display_column_index] = Some(
                    self.build_data_cell(
                        app,
                        context,
                        padding,
                        cell.child.clone(),
                        column.numeric,
                        cell.placeholder,
                        cell.show_edit_icon,
                        cell.on_tap.clone(),
                        on_select_changed,
                        cell.on_double_tap.clone(),
                        cell.on_long_press.clone(),
                        cell.on_tap_down.clone(),
                        cell.on_tap_cancel.clone(),
                        row.color
                            .clone()
                            .or_else(|| effective_data_row_color.clone()),
                        row.on_long_press.clone(),
                        row.on_hover.clone(),
                        row.mouse_cursor
                            .as_ref()
                            .and_then(|cursor| cursor.resolve(&states))
                            .or_else(|| {
                                data_table_theme
                                    .data_row_cursor
                                    .as_ref()
                                    .and_then(|cursor| cursor.resolve(&states))
                            }),
                    ),
                );
            }
            display_column_index += 1;
        }

        let table_rows: Vec<TableRow> = cells
            .into_iter()
            .zip(row_decorations)
            .enumerate()
            .map(|(index, (children, decoration))| {
                {
                    let mut row = TableRow::new().decoration(decoration);
                    if index == 0 {
                        row = row.key(heading_row_key());
                    } else if let Some(key) = self.rows[index - 1].key.clone() {
                        row = row.key(key);
                    }
                    row
                }
                .children(
                    children
                        .into_iter()
                        .map(|child| child.expect("every cell is built")),
                )
            })
            .collect();
        let column_widths: HashMap<usize, TableColumnWidthRef> = table_columns
            .into_iter()
            .enumerate()
            .map(|(index, width)| (index, width.expect("every column has a width")))
            .collect();

        let mut table = Table::new()
            .column_widths(column_widths)
            .default_vertical_alignment(TableCellVerticalAlignment::Middle)
            .children(table_rows);
        if let Some(border) = &self.border {
            table = table.border(*border);
        }
        let mut material = Material::new()
            .r#type(MaterialType::Transparency)
            .clip_behavior(self.clip_behavior);
        if let Some(border) = &self.border {
            material = material.border_radius(border.border_radius);
        }
        let mut container = Container::new();
        if let Some(decoration) = self
            .decoration
            .clone()
            .or_else(|| data_table_theme.decoration.clone())
            .or_else(|| theme.data_table_theme.decoration.clone())
        {
            container = container.decoration(decoration);
        }
        container
            .child(material.child(table.into_widget()))
            .into_widget()
    }
}

/// An ink well for the cells of a [`DataTable`] row that paints its highlight across the whole
/// row.
///
/// A [`TableRowInkWell`] must be in the same coordinate space (modulo translations) as the
/// `Table` widget that it is a descendant of. A [`TableRowInkWell`] must be an ancestor of the
/// `Table`'s cells' render objects, and a descendant of a `Table`.
///
/// This widget is the cell of a row, which makes the whole row respond to input.
///
/// See also:
///
///  * [`DataTable`], which makes use of [`TableRowInkWell`] when [`DataRow::on_select_changed`]
///    is set, or one of its cells has gesture callbacks.
///  * [`InkResponse`], which is the base class of this widget.
#[derive(Clone, Debug)]
pub struct TableRowInkWell {
    /// Dart's `extends InkResponse`: the fields it inherits.
    pub ink_response: InkResponse,
}

impl TableRowInkWell {
    /// Creates an ink well for a table row; Dart's optional named arguments are the setters.
    pub fn new() -> TableRowInkWell {
        TableRowInkWell {
            ink_response: InkResponse {
                contained_ink_well: true,
                highlight_shape: waft_painting::BoxShape::Rectangle,
                get_rect_callback: Rc::new(TableRowInkWell::get_rect_callback),
                debug_check_context: Rc::new(|app, context| {
                    debug_check_has_table(app, context)
                        && InkResponse::debug_check_context(app, context)
                }),
                ..InkResponse::default()
            },
        }
    }

    /// Dart `TableRowInkWell(key:)`.
    pub fn key(mut self, key: KeyRef) -> TableRowInkWell {
        self.ink_response.key = Some(key);
        self
    }

    /// Dart `TableRowInkWell(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> TableRowInkWell {
        self.ink_response.child = Some(child.into_widget());
        self
    }

    /// Dart `TableRowInkWell(onTap:)`.
    pub fn on_tap(mut self, on_tap: GestureTapCallback) -> TableRowInkWell {
        self.ink_response.on_tap = Some(on_tap);
        self
    }

    /// Dart `TableRowInkWell(onDoubleTap:)`.
    pub fn on_double_tap(mut self, on_double_tap: GestureTapCallback) -> TableRowInkWell {
        self.ink_response.on_double_tap = Some(on_double_tap);
        self
    }

    /// Dart `TableRowInkWell(onLongPress:)`.
    pub fn on_long_press(mut self, on_long_press: GestureLongPressCallback) -> TableRowInkWell {
        self.ink_response.on_long_press = Some(on_long_press);
        self
    }

    /// Dart `TableRowInkWell(onHighlightChanged:)`.
    pub fn on_highlight_changed(
        mut self,
        on_highlight_changed: ValueChanged<bool>,
    ) -> TableRowInkWell {
        self.ink_response.on_highlight_changed = Some(on_highlight_changed);
        self
    }

    /// Dart `TableRowInkWell(onHover:)`.
    pub fn on_hover(mut self, on_hover: ValueChanged<bool>) -> TableRowInkWell {
        self.ink_response.on_hover = Some(on_hover);
        self
    }

    /// Dart `TableRowInkWell(onSecondaryTap:)`.
    pub fn on_secondary_tap(mut self, on_secondary_tap: GestureTapCallback) -> TableRowInkWell {
        self.ink_response.on_secondary_tap = Some(on_secondary_tap);
        self
    }

    /// Dart `TableRowInkWell(onSecondaryTapDown:)`.
    pub fn on_secondary_tap_down(
        mut self,
        on_secondary_tap_down: GestureTapDownCallback,
    ) -> TableRowInkWell {
        self.ink_response.on_secondary_tap_down = Some(on_secondary_tap_down);
        self
    }

    /// Dart `TableRowInkWell(overlayColor:)`.
    pub fn overlay_color(
        mut self,
        overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> TableRowInkWell {
        self.ink_response.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `TableRowInkWell(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> TableRowInkWell {
        self.ink_response.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart's override of `getRectCallback`: the rect of the whole table row the reference box
    /// is a cell of, in the reference box's coordinate space.
    fn get_rect_callback(_app: &App, reference_box: AnyRenderBox) -> Option<RectCallback> {
        Some(Rc::new(move |app: &App| {
            let mut cell: AnyRenderObject = reference_box.as_object();
            let mut table = cell.parent(app);
            let mut transform = Matrix4::IDENTITY;
            while let Some(parent) = table {
                if parent.downcast::<RenderTable>(app).is_some() {
                    break;
                }
                parent.apply_paint_transform(app, cell, &mut transform);
                debug_assert_eq!(Some(parent), cell.parent(app));
                cell = parent;
                table = parent.parent(app);
            }
            if let Some(table) = table.and_then(|table| table.downcast::<RenderTable>(app)) {
                let cell_parent_data = cell.parent_data_of::<TableCellParentData>(app);
                let row = cell_parent_data.y.expect("a laid-out cell has a row");
                let rect = table.get_row_box(app, row);
                // The rect is in the table's coordinate space. We need to change it to the
                // TableRowInkWell's coordinate space.
                table
                    .as_object()
                    .apply_paint_transform(app, cell, &mut transform);
                if let Some(offset) = waft_painting::get_as_translation(transform) {
                    return rect.shift(-offset);
                }
            }
            Rect::ZERO
        }))
    }
}

impl Default for TableRowInkWell {
    fn default() -> TableRowInkWell {
        TableRowInkWell::new()
    }
}

impl StatelessWidget for TableRowInkWell {
    fn key(&self) -> Option<&KeyRef> {
        self.ink_response.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        self.ink_response.build_state_widget(app, context)
    }
}

/// Dart's `_SortArrow`.
#[derive(Debug)]
struct SortArrow {
    visible: bool,
    up: Option<bool>,
    duration: Duration,
}

impl StatefulWidget for SortArrow {
    type State = SortArrowState;

    fn create_state(&self) -> SortArrowState {
        SortArrowState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            opacity_controller: None,
            opacity_animation: None,
            orientation_controller: None,
            orientation_animation: None,
            turn_tween: None,
            orientation_offset: 0.0,
            up: None,
        }
    }
}

/// Dart's `_SortArrowState`.
struct SortArrowState {
    state: StateData<SortArrow>,
    ticker_provider: TickerProviderStateMixinData,
    opacity_controller: Option<Ref<AnimationController>>,
    opacity_animation: Option<Ref<CurvedAnimation>>,
    orientation_controller: Option<Ref<AnimationController>>,
    orientation_animation: Option<AnimationRef<f64>>,
    /// Dart's `static final Animatable<double> _turnTween`.
    turn_tween: Option<Ref<Tween<f64>>>,
    orientation_offset: f64,
    up: Option<bool>,
}

impl SortArrowState {
    const ARROW_ICON_BASELINE_OFFSET: f64 = -1.5;
    const ARROW_ICON_SIZE: f64 = 16.0;

    fn opacity_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .opacity_controller
            .as_ref()
            .expect("created in init_state")
            .handle()
    }

    fn orientation_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .orientation_controller
            .as_ref()
            .expect("created in init_state")
            .handle()
    }

    fn rebuild(self: Handle<Self>, app: &mut App) {
        // The animations changed, so we need to rebuild.
        self.set_state(app, |_state| {});
    }

    fn reset_orientation_animation(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        if status.is_completed() {
            let orientation_animation = app
                .get(self)
                .orientation_animation
                .clone()
                .expect("created in init_state");
            debug_assert_eq!(orientation_animation.value(app), PI);
            app.get_mut(self).orientation_offset += PI;
            self.orientation_controller(app).set_value(app, 0.0); // TODO(ianh): This triggers a pointless rebuild.
        }
    }
}

impl TickerProviderStateMixin for SortArrowState {
    fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData {
        &app.get(self).ticker_provider
    }

    fn ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TickerProviderStateMixinData {
        &mut app.get_mut(self).ticker_provider
    }
}

/// Dart's `vsync: this`.
impl TickerProviderObject for SortArrowState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for SortArrowState {
    type Widget = SortArrow;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget_up = self.widget(app).up;
        let widget_visible = self.widget(app).visible;
        let duration = self.widget(app).duration;
        let opacity_controller = AnimationController::new(
            app,
            None,
            Some(duration),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let opacity_animation = CurvedAnimation::new(
            app,
            opacity_controller.erase(),
            Curves::fast_out_slow_in(),
            None,
        );
        opacity_animation.add_listener(app, Listener::handle_method(self, Self::rebuild));
        opacity_controller.set_value(app, if widget_visible { 1.0 } else { 0.0 });
        let orientation_controller = AnimationController::new(
            app,
            None,
            Some(duration),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let turn_tween = Tween::new(app, Some(0.0), Some(PI));
        let ease_in = CurveTween::new(app, Curves::ease_in());
        let orientation_animation =
            orientation_controller.drive(app, turn_tween.clone().chain(ease_in));
        orientation_animation.add_listener(app, Listener::handle_method(self, Self::rebuild));
        orientation_animation.add_status_listener(
            app,
            AnimationStatusListener::new(move |status, app| {
                self.reset_orientation_animation(app, status)
            }),
        );
        let state = app.get_mut(self);
        state.up = widget_up;
        state.opacity_controller = Some(opacity_controller);
        state.opacity_animation = Some(opacity_animation);
        state.orientation_controller = Some(orientation_controller);
        state.orientation_animation = Some(orientation_animation);
        state.turn_tween = Some(turn_tween);
        if widget_visible {
            state.orientation_offset = if widget_up.expect("a visible arrow has a direction") {
                0.0
            } else {
                PI
            };
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &SortArrow) {
        let mut skip_arrow = false;
        let widget_up = self.widget(app).up;
        let widget_visible = self.widget(app).visible;
        let new_up = widget_up.or(app.get(self).up);
        if old_widget.visible != widget_visible {
            let opacity_controller = self.opacity_controller(app);
            if widget_visible && opacity_controller.is_dismissed(app) {
                let orientation_controller = self.orientation_controller(app);
                orientation_controller.stop(app, false);
                orientation_controller.set_value(app, 0.0);
                app.get_mut(self).orientation_offset =
                    if new_up.expect("a visible arrow has a direction") {
                        0.0
                    } else {
                        PI
                    };
                skip_arrow = true;
            }
            if widget_visible {
                opacity_controller.forward(app, None);
            } else {
                opacity_controller.reverse(app, None);
            }
        }
        if app.get(self).up != new_up && !skip_arrow {
            let orientation_controller = self.orientation_controller(app);
            if orientation_controller.is_dismissed(app) {
                orientation_controller.forward(app, None);
            } else {
                orientation_controller.reverse(app, None);
            }
        }
        app.get_mut(self).up = new_up;
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.opacity_controller(app).dispose(app);
        self.orientation_controller(app).dispose(app);
        app.get(self)
            .opacity_animation
            .as_ref()
            .expect("created in init_state")
            .handle()
            .dispose(app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let state = app.get(self);
        let opacity = state
            .opacity_animation
            .as_ref()
            .expect("created in init_state")
            .erase();
        let orientation = state
            .orientation_animation
            .clone()
            .expect("created in init_state")
            .value(app);
        let angle = (state.orientation_offset + orientation) as f32;
        let [a, b, c, d, _, _] = Matrix4::rotation(angle).to_affine();
        let transform = Matrix4::from_affine(
            a,
            b,
            c,
            d,
            0.0,
            SortArrowState::ARROW_ICON_BASELINE_OFFSET as f32,
        );
        FadeTransition::new(opacity)
            .child(
                Transform::new(transform)
                    .alignment(AlignmentGeometry::CENTER)
                    .child(
                        Icon::new(Some(Icons::arrow_upward()))
                            .size(SortArrowState::ARROW_ICON_SIZE),
                    ),
            )
            .into_widget()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    //! The cases of `test/data_table_test.dart`.

    use std::cell::RefCell;
    use std::f64::consts::PI;
    use std::rc::Rc;
    use std::time::Duration;

    use waft_embedder::{Offset, Rect, Size};
    use waft_gestures::TapDownDetails;
    use waft_painting::{Border, BorderRadius, BorderSide, BorderStyle, BoxDecoration, TextStyle};
    use waft_rendering::FlexColumnWidth;
    use waft_rendering::testing::{PaintCall, paints};
    use waft_services::SystemMouseCursors;
    use waft_widgets::testing::{
        Finder, TestPointer, WidgetTester, assert_f32_equals, expect, find, finds_n_widgets,
        finds_nothing, finds_one_widget, test_widgets,
    };
    use waft_widgets::{Align, Builder, Column, Expanded, Text, WidgetStatePropertyAll};

    use super::*;
    use crate::bottom_navigation_bar::tests::paragraph_style;
    use crate::checkbox_theme::CheckboxThemeData;
    use crate::colors::Colors;
    use crate::data_table_theme::DataTableThemeData;
    use crate::scaffold::Scaffold;
    use crate::test_support::PaintInspection;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::m3;

    /// The `Dessert` of `data_table_test_utils.dart`, with the fields the cases read.
    pub(crate) struct Dessert {
        pub name: &'static str,
        pub calories: i32,
        pub fat: f64,
    }

    /// `kDesserts` of `data_table_test_utils.dart`.
    pub(crate) const K_DESSERTS: [Dessert; 10] = [
        Dessert {
            name: "Frozen yogurt",
            calories: 159,
            fat: 6.0,
        },
        Dessert {
            name: "Ice cream sandwich",
            calories: 237,
            fat: 9.0,
        },
        Dessert {
            name: "Eclair",
            calories: 262,
            fat: 16.0,
        },
        Dessert {
            name: "Cupcake",
            calories: 305,
            fat: 3.7,
        },
        Dessert {
            name: "Gingerbread",
            calories: 356,
            fat: 16.0,
        },
        Dessert {
            name: "Jelly bean",
            calories: 375,
            fat: 0.0,
        },
        Dessert {
            name: "Lollipop",
            calories: 392,
            fat: 0.2,
        },
        Dessert {
            name: "Honeycomb",
            calories: 408,
            fat: 3.2,
        },
        Dessert {
            name: "Donut",
            calories: 452,
            fat: 25.0,
        },
        Dessert {
            name: "KitKat",
            calories: 518,
            fat: 26.0,
        },
    ];

    type Log = Rc<RefCell<Vec<String>>>;

    fn new_log() -> Log {
        Rc::default()
    }

    fn log_of(log: &Log) -> Vec<String> {
        log.borrow().clone()
    }

    fn take_log(log: &Log) -> Vec<String> {
        std::mem::take(&mut *log.borrow_mut())
    }

    /// Dart's `MaterialApp(home: Material(child: child))`.
    pub(crate) fn app_of<K>(tester: &WidgetTester, child: impl IntoWidget<K>) -> WidgetRef {
        material_app(
            m3(&tester.cell.borrow()),
            Material::new().child(child).into_widget(),
        )
    }

    fn name_column() -> DataColumn {
        DataColumn::new(Text::new("Name")).tooltip("Name")
    }

    fn calories_column() -> DataColumn {
        DataColumn::new(Text::new("Calories")).tooltip("Calories")
    }

    fn name_rows() -> Vec<DataRow> {
        K_DESSERTS
            .iter()
            .map(|dessert| DataRow::new(vec![DataCell::new(Text::new(dessert.name))]))
            .collect()
    }

    fn rotation_of(tester: &WidgetTester, finder: Finder) -> [f32; 4] {
        tester.widget(finder, |transform: &Transform| {
            let [a, b, c, d, _, _] = transform.transform.to_affine();
            [a, b, c, d]
        })
    }

    fn assert_rotation(actual: [f32; 4], expected: [f32; 4]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-6, "{actual} vs {expected}");
        }
    }

    fn control_table(
        log: &Log,
        sort_column_index: Option<usize>,
        sort_ascending: bool,
    ) -> DataTable {
        let select_all_log = Rc::clone(log);
        let sort_log = Rc::clone(log);
        let rows = K_DESSERTS
            .iter()
            .map(|dessert| {
                let name = dessert.name;
                let calories = dessert.calories;
                let selected_log = Rc::clone(log);
                let long_press_log = Rc::clone(log);
                let hover_log = Rc::clone(log);
                let tap_log = Rc::clone(log);
                let double_tap_log = Rc::clone(log);
                let cell_long_press_log = Rc::clone(log);
                let cancel_log = Rc::clone(log);
                let down_log = Rc::clone(log);
                DataRow::new(vec![
                    DataCell::new(Text::new(name)),
                    DataCell::new(Text::new(calories.to_string()))
                        .show_edit_icon(true)
                        .on_tap(Listener::new(move |_| {
                            tap_log.borrow_mut().push(format!("cell-tap: {calories}"))
                        }))
                        .on_double_tap(Listener::new(move |_| {
                            double_tap_log
                                .borrow_mut()
                                .push(format!("cell-doubleTap: {calories}"))
                        }))
                        .on_long_press(Listener::new(move |_| {
                            cell_long_press_log
                                .borrow_mut()
                                .push(format!("cell-longPress: {calories}"))
                        }))
                        .on_tap_cancel(Listener::new(move |_| {
                            cancel_log
                                .borrow_mut()
                                .push(format!("cell-tapCancel: {calories}"))
                        }))
                        .on_tap_down(Rc::new(move |_app: &mut App, _details: TapDownDetails| {
                            down_log
                                .borrow_mut()
                                .push(format!("cell-tapDown: {calories}"))
                        })),
                ])
                .key(ValueKey::new(name.to_owned()))
                .on_select_changed(move |_app, _selected| {
                    selected_log
                        .borrow_mut()
                        .push(format!("row-selected: {name}"))
                })
                .on_long_press(Listener::new(move |_| {
                    long_press_log
                        .borrow_mut()
                        .push(format!("onLongPress: {name}"))
                }))
                .on_hover(move |_app, hovering| {
                    if hovering {
                        hover_log.borrow_mut().push(format!("onHover: {name}"));
                    }
                })
            })
            .collect();
        let mut table = DataTable::new(
            vec![
                name_column(),
                calories_column()
                    .numeric(true)
                    .on_sort(move |_app, column_index, ascending| {
                        sort_log
                            .borrow_mut()
                            .push(format!("column-sort: {column_index} {ascending}"))
                    }),
            ],
            rows,
        )
        .sort_ascending(sort_ascending)
        .on_select_all(move |_app, value| {
            select_all_log.borrow_mut().push(format!(
                "select-all: {}",
                value.map_or("null".to_owned(), |v| v.to_string())
            ))
        });
        table.sort_column_index = sort_column_index;
        table
    }

    #[test]
    fn data_table_control_test() {
        test_widgets(|tester| {
            let log = new_log();
            let root = app_of(tester, control_table(&log, None, true));
            tester.pump_widget(root);

            tester.tap(find::by_type::<Checkbox>().first());
            assert_eq!(take_log(&log), ["select-all: true"]);

            tester.tap(find::text("Cupcake"));
            assert_eq!(take_log(&log), ["row-selected: Cupcake"]);

            tester.long_press(find::text("Cupcake"));
            assert_eq!(take_log(&log), ["onLongPress: Cupcake"]);

            let mut gesture = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.pump();
            let at = tester.get_center(find::text("Cupcake"));
            tester.move_to(&mut gesture, at, false);
            assert_eq!(take_log(&log), ["onHover: Cupcake"]);

            tester.tap(find::text("Calories"));
            assert_eq!(take_log(&log), ["column-sort: 1 true"]);

            let root = app_of(tester, control_table(&log, Some(1), true));
            tester.pump_widget(root);
            tester.pump_and_settle_by(Duration::from_millis(200));
            tester.tap(find::text("Calories"));
            assert_eq!(take_log(&log), ["column-sort: 1 false"]);

            let root = app_of(tester, control_table(&log, Some(1), false));
            tester.pump_widget(root);
            tester.pump_and_settle_by(Duration::from_millis(200));

            tester.long_press(find::text("375"));
            // The tap down is triggered on gesture down. Then, the cancel is triggered when the
            // gesture arena recognizes that the long press overrides the tap event so it
            // triggers a tap cancel, followed by the long press.
            assert_eq!(
                take_log(&log),
                [
                    "cell-tapDown: 375",
                    "cell-tapCancel: 375",
                    "cell-longPress: 375"
                ]
            );

            let mut gesture = TestPointer::touch(tester.next_pointer());
            let center = tester.get_rect(find::text("375")).center();
            tester.down(&mut gesture, center);
            tester.pump_by(Duration::from_millis(100));
            // onTapDown callback is registered.
            assert_eq!(log_of(&log), ["cell-tapDown: 375"]);
            tester.up(&gesture);

            tester.pump_by(Duration::from_secs(1));
            // onTap callback is registered after the gesture is removed.
            assert_eq!(take_log(&log), ["cell-tapDown: 375", "cell-tap: 375"]);

            // dragging off the bounds of the cell calls the cancel callback
            let mut gesture = TestPointer::touch(tester.next_pointer());
            let center = tester.get_rect(find::text("375")).center();
            tester.down(&mut gesture, center);
            tester.pump_by(Duration::from_millis(100));
            tester.move_by(&mut gesture, Offset::new(0.0, 200.0), true);
            tester.cancel(&gesture);
            assert_eq!(take_log(&log), ["cell-tapDown: 375", "cell-tapCancel: 375"]);

            tester.tap(find::by_type::<Checkbox>().last());
            assert_eq!(take_log(&log), ["row-selected: KitKat"]);
        });
    }

    fn tristate_table(log: &Log, selected: [bool; 3], disabled_index: Option<usize>) -> DataTable {
        let select_all_log = Rc::clone(log);
        let rows = (0..3)
            .map(|index| {
                let row_log = Rc::clone(log);
                let mut row = DataRow::new(vec![DataCell::new(Text::new(format!("Row {index}")))])
                    .selected(selected[index]);
                if Some(index) != disabled_index {
                    row = row.on_select_changed(move |_app, _value| {
                        row_log.borrow_mut().push(format!("row-selected: {index}"))
                    });
                }
                row
            })
            .collect();
        DataTable::new(vec![name_column()], rows).on_select_all(move |_app, value| {
            select_all_log.borrow_mut().push(format!(
                "select-all: {}",
                value.map_or("null".to_owned(), |v| v.to_string())
            ))
        })
    }

    #[test]
    fn data_table_control_test_tristate() {
        test_widgets(|tester| {
            let log = new_log();
            // Tapping the parent checkbox when no rows are selected, selects all.
            let root = app_of(tester, tristate_table(&log, [false, false, false], None));
            tester.pump_widget(root);
            tester.tap(find::by_type::<Checkbox>().first());
            assert_eq!(take_log(&log), ["select-all: true"]);

            // Tapping the parent checkbox when some rows are selected, selects all.
            let root = app_of(tester, tristate_table(&log, [true, false, true], None));
            tester.pump_widget(root);
            tester.tap(find::by_type::<Checkbox>().first());
            assert_eq!(take_log(&log), ["select-all: true"]);

            // Tapping the parent checkbox when all rows are selected, deselects all.
            let root = app_of(tester, tristate_table(&log, [true, true, true], None));
            tester.pump_widget(root);
            tester.tap(find::by_type::<Checkbox>().first());
            assert_eq!(take_log(&log), ["select-all: false"]);

            // Tapping the parent checkbox when all rows are selected and one is disabled,
            // deselects all.
            let root = app_of(tester, tristate_table(&log, [true, true, false], Some(2)));
            tester.pump_widget(root);
            tester.tap(find::by_type::<Checkbox>().first());
            assert_eq!(take_log(&log), ["select-all: false"]);
        });
    }

    fn no_checkbox_table(log: &Log, checkboxes: bool) -> DataTable {
        let select_all_log = Rc::clone(log);
        let rows = K_DESSERTS
            .iter()
            .map(|dessert| {
                let name = dessert.name;
                let calories = dessert.calories;
                let selected_log = Rc::clone(log);
                let tap_log = Rc::clone(log);
                DataRow::new(vec![
                    DataCell::new(Text::new(name)),
                    DataCell::new(Text::new(calories.to_string()))
                        .show_edit_icon(true)
                        .on_tap(Listener::new(move |_| {
                            tap_log.borrow_mut().push(format!("cell-tap: {calories}"))
                        })),
                ])
                .key(ValueKey::new(name.to_owned()))
                .on_select_changed(move |_app, _selected| {
                    selected_log
                        .borrow_mut()
                        .push(format!("row-selected: {name}"))
                })
            })
            .collect();
        DataTable::new(vec![name_column(), calories_column().numeric(true)], rows)
            .show_checkbox_column(checkboxes)
            .on_select_all(move |_app, value| {
                select_all_log.borrow_mut().push(format!(
                    "select-all: {}",
                    value.map_or("null".to_owned(), |v| v.to_string())
                ))
            })
    }

    #[test]
    fn data_table_control_test_no_checkboxes() {
        test_widgets(|tester| {
            let log = new_log();
            let root = app_of(tester, no_checkbox_table(&log, false));
            tester.pump_widget(root);

            expect(tester, &find::by_type::<Checkbox>(), finds_nothing());
            tester.tap(find::text("Cupcake"));
            assert_eq!(take_log(&log), ["row-selected: Cupcake"]);

            let root = app_of(tester, no_checkbox_table(&log, true));
            tester.pump_widget(root);

            tester.pump_and_settle_by(Duration::from_millis(200));
            let checkboxes = find::by_type::<Checkbox>();
            expect(tester, &checkboxes, finds_n_widgets(11));
            tester.tap(checkboxes.first());
            assert_eq!(take_log(&log), ["select-all: true"]);
        });
    }

    fn lone_table(label: String, cell: String) -> DataTable {
        DataTable::new(
            vec![DataColumn::new(Text::new(label))],
            vec![DataRow::new(vec![DataCell::new(Text::new(cell))])],
        )
    }

    fn width_of(tester: &WidgetTester, finder: Finder) -> f64 {
        tester.get_size(finder.first()).width()
    }

    #[test]
    fn data_table_overflow_test_header() {
        test_widgets(|tester| {
            let table = DataTable::new(
                vec![DataColumn::new(Text::new("X".repeat(2000)))],
                vec![DataRow::new(vec![DataCell::new(Text::new("X"))])],
            )
            .heading_text_style(
                // Will overflow if letter spacing is larger than 0.0.
                TextStyle::new().font_size(14.0).letter_spacing(0.0),
            );
            let root = app_of(tester, table);
            tester.pump_widget(root);

            assert!(width_of(tester, find::by_type::<Text>()) > 800.0);
            assert!(width_of(tester, find::by_type::<Row>()) > 800.0);
            // column overflows table, but text doesn't overflow cell
        });
    }

    #[test]
    fn data_table_overflow_test_cell() {
        test_widgets(|tester| {
            let root = app_of(tester, lone_table("X".to_owned(), "X".repeat(2000)));
            tester.pump_widget(root);
            assert!(width_of(tester, find::by_type::<Text>()) < 800.0);
            assert!(width_of(tester, find::by_type::<Row>()) > 800.0);
            // cell overflows table, but text doesn't overflow cell
        });
    }

    #[test]
    fn data_table_overflow_test_cell_that_wraps() {
        test_widgets(|tester| {
            let root = app_of(tester, lone_table("X".to_owned(), "X ".repeat(2000)));
            tester.pump_widget(root);
            assert!(width_of(tester, find::by_type::<Text>()) < 800.0);
            assert!(width_of(tester, find::by_type::<Row>()) < 800.0);
        });
    }

    #[test]
    fn data_table_column_on_sort_test() {
        test_widgets(|tester| {
            let root = app_of(
                tester,
                lone_table("Dessert".to_owned(), "Lollipop".to_owned()),
            );
            tester.pump_widget(root);
            tester.tap(find::text("Dessert"));
            tester.pump();
        });
    }

    fn sort_table(title: &str, ascending: bool) -> DataTable {
        DataTable::new(
            vec![
                DataColumn::new(Text::new(title))
                    .tooltip("Name")
                    .on_sort(|_app, _index, _ascending| {}),
            ],
            name_rows(),
        )
        .sort_column_index(0)
        .sort_ascending(ascending)
    }

    #[test]
    fn data_table_sort_indicator_orientation() {
        test_widgets(|tester| {
            let icon_finder = find::descendant(
                find::by_type::<DataTable>(),
                find::widget_with_icon::<Transform>(&Icons::arrow_upward()),
            );
            // Check for ascending list
            let root = app_of(tester, sort_table("Name", true));
            tester.pump_widget(root);
            // The `tester.widget` ensures that there is exactly one upward arrow.
            assert_rotation(
                rotation_of(tester, icon_finder.clone()),
                [1.0, 0.0, 0.0, 1.0],
            );

            // Check for descending list.
            let root = app_of(tester, sort_table("Name", false));
            tester.pump_widget(root);
            tester.pump_and_settle();
            assert_rotation(
                rotation_of(tester, icon_finder),
                [
                    PI.cos() as f32,
                    PI.sin() as f32,
                    -PI.sin() as f32,
                    PI.cos() as f32,
                ],
            );
        });
    }

    #[test]
    fn data_table_sort_indicator_orientation_does_not_change_on_state_update() {
        test_widgets(|tester| {
            let icon_finder = find::descendant(
                find::by_type::<DataTable>(),
                find::widget_with_icon::<Transform>(&Icons::arrow_upward()),
            );
            let root = app_of(tester, sort_table("Name1", true));
            tester.pump_widget(root);
            assert_rotation(
                rotation_of(tester, icon_finder.clone()),
                [1.0, 0.0, 0.0, 1.0],
            );

            // Cause a rebuild by updating the widget
            let root = app_of(tester, sort_table("Name2", true));
            tester.pump_widget(root);
            tester.pump_and_settle();
            assert_rotation(rotation_of(tester, icon_finder), [1.0, 0.0, 0.0, 1.0]);
        });
    }

    #[test]
    fn data_table_sort_indicator_orientation_does_not_change_on_state_update_reverse() {
        test_widgets(|tester| {
            let icon_finder = find::descendant(
                find::by_type::<DataTable>(),
                find::widget_with_icon::<Transform>(&Icons::arrow_upward()),
            );
            let expected = [
                PI.cos() as f32,
                PI.sin() as f32,
                -PI.sin() as f32,
                PI.cos() as f32,
            ];
            let root = app_of(tester, sort_table("Name1", false));
            tester.pump_widget(root);
            assert_rotation(rotation_of(tester, icon_finder.clone()), expected);

            let root = app_of(tester, sort_table("Name2", false));
            tester.pump_widget(root);
            tester.pump_and_settle();
            assert_rotation(rotation_of(tester, icon_finder), expected);
        });
    }

    #[test]
    fn data_table_row_on_select_changed_test() {
        test_widgets(|tester| {
            let root = app_of(
                tester,
                lone_table("Dessert".to_owned(), "Lollipop".to_owned()),
            );
            tester.pump_widget(root);
            tester.tap(find::text("Lollipop"));
            tester.pump();
        });
    }

    fn first_container_for(text: &str) -> Finder {
        find::widget_with_text::<Container>(text).first()
    }

    fn row_height_table(
        data_row_min_height: Option<f64>,
        data_row_max_height: Option<f64>,
        heading_row_height: f64,
    ) -> DataTable {
        let rows = K_DESSERTS
            .iter()
            .map(|dessert| {
                DataRow::new(vec![
                    DataCell::new(Text::new(dessert.name)),
                    DataCell::new(Text::new(dessert.calories.to_string()))
                        .show_edit_icon(true)
                        .on_tap(Listener::new(|_| {})),
                ])
                .key(ValueKey::new(dessert.name.to_owned()))
                .on_select_changed(|_app, _selected| {})
            })
            .collect();
        let mut table = DataTable::new(
            vec![
                name_column(),
                calories_column()
                    .numeric(true)
                    .on_sort(|_app, _index, _ascending| {}),
            ],
            rows,
        )
        .on_select_all(|_app, _value| {})
        .heading_row_height(heading_row_height);
        table.data_row_min_height = data_row_min_height;
        table.data_row_max_height = data_row_max_height;
        table
    }

    #[test]
    fn data_table_custom_row_height() {
        test_widgets(|tester| {
            // DEFAULT VALUES
            let root = app_of(tester, row_height_table(None, None, 56.0));
            tester.pump_widget(root);

            // The finder matches with the Container of the cell content, as well as the
            // Container wrapping the whole table. The first one is used to test row heights.
            assert_eq!(tester.get_size(first_container_for("Name")).height(), 56.0);
            assert_eq!(
                tester
                    .get_size(first_container_for("Frozen yogurt"))
                    .height(),
                K_MIN_INTERACTIVE_DIMENSION
            );

            // CUSTOM VALUES
            let root = app_of(tester, row_height_table(None, None, 48.0));
            tester.pump_widget(root);
            assert_eq!(tester.get_size(first_container_for("Name")).height(), 48.0);

            let root = app_of(tester, row_height_table(None, None, 64.0));
            tester.pump_widget(root);
            assert_eq!(tester.get_size(first_container_for("Name")).height(), 64.0);

            let root = app_of(tester, row_height_table(Some(30.0), Some(30.0), 56.0));
            tester.pump_widget(root);
            assert_eq!(
                tester
                    .get_size(first_container_for("Frozen yogurt"))
                    .height(),
                30.0
            );

            let root = app_of(
                tester,
                row_height_table(Some(0.0), Some(f64::INFINITY), 56.0),
            );
            tester.pump_widget(root);
            assert!(
                tester
                    .get_size(first_container_for("Frozen yogurt"))
                    .height()
                    > 0.0
            );
        });
    }

    #[test]
    fn data_table_custom_row_height_one_row_taller_than_others() {
        test_widgets(|tester| {
            let multiline_text = "Line one.\nLine two.\nLine three.\nLine four.";
            let table = DataTable::new(
                vec![
                    DataColumn::new(Text::new("SingleRowColumn")),
                    DataColumn::new(Text::new("MultiRowColumn")),
                ],
                vec![DataRow::new(vec![
                    DataCell::new(Text::new("Data")),
                    DataCell::new(
                        Column::new().children(vec![Text::new(multiline_text).into_widget()]),
                    ),
                ])],
            )
            .data_row_min_height(0.0)
            .data_row_max_height(f64::INFINITY);
            let root = app_of(tester, table);
            tester.pump_widget(root);

            let single_line_row_height = tester.get_size(first_container_for("Data")).height();
            let multiline_row_height = tester
                .get_size(first_container_for(multiline_text))
                .height();

            assert!(multiline_row_height > single_line_row_height);
        });
    }

    #[test]
    #[expect(deprecated)]
    fn data_table_custom_row_height_separate_test_for_deprecated_data_row_height() {
        test_widgets(|tester| {
            let mut table = row_height_table(None, None, 56.0);
            table = table.data_row_height(30.0);
            let root = app_of(tester, table);
            tester.pump_widget(root);
            assert_eq!(
                tester
                    .get_size(first_container_for("Frozen yogurt"))
                    .height(),
                30.0
            );
        });
    }

    #[test]
    fn data_table_custom_sort_icon_builder_test() {
        test_widgets(|tester| {
            let build_table = |sort_icon_builder: Option<DataTableSortIconBuilder>,
                               theme_data: Option<DataTableThemeData>,
                               sort_column_index: usize,
                               sort_ascending: bool|
             -> DataTable {
                let mut table = DataTable::new(
                    vec![
                        DataColumn::new(Text::new("Name")).on_sort(|_app, _index, _ascending| {}),
                        DataColumn::new(Text::new("Calories"))
                            .on_sort(|_app, _index, _ascending| {}),
                    ],
                    K_DESSERTS
                        .iter()
                        .map(|dessert| {
                            DataRow::new(vec![
                                DataCell::new(Text::new(dessert.name)),
                                DataCell::new(Text::new(dessert.calories.to_string())),
                            ])
                        })
                        .collect(),
                )
                .sort_column_index(sort_column_index)
                .sort_ascending(sort_ascending);
                table.sort_icon_builder = sort_icon_builder;
                let _ = theme_data;
                table
            };
            let pump = |tester: &mut WidgetTester,
                        sort_icon_builder: Option<DataTableSortIconBuilder>,
                        theme_data: Option<DataTableThemeData>,
                        sort_column_index: usize,
                        sort_ascending: bool| {
                let table = build_table(sort_icon_builder, None, sort_column_index, sort_ascending);
                let home: WidgetRef = match theme_data {
                    Some(data) => DataTableTheme::new(data, table).into_widget(),
                    None => table.into_widget(),
                };
                let root = app_of(tester, home);
                tester.pump_widget(root);
            };
            let builder = |f: fn(bool, bool) -> &'static str| -> DataTableSortIconBuilder {
                Rc::new(move |_app, _context, visible, ascending| {
                    Text::new(f(visible, ascending)).into_widget()
                })
            };

            // Default (null sortIconBuilder): uses standard Icon(Icons.arrow_upward).
            pump(tester, None, None, 0, true);
            expect(
                tester,
                &find::descendant(
                    find::by_type::<DataTable>(),
                    find::by_icon(&Icons::arrow_upward()),
                ),
                finds_n_widgets(2),
            );

            // Custom sortIconBuilder on DataTable.
            let labels = builder(|visible, ascending| {
                if visible {
                    if ascending { "ASC" } else { "DESC" }
                } else {
                    "INACTIVE"
                }
            });
            pump(tester, Some(labels.clone()), None, 0, true);
            expect(tester, &find::text("ASC"), finds_one_widget());
            expect(tester, &find::text("INACTIVE"), finds_one_widget());

            // Verify sortAscending = false passes ascending = false.
            pump(tester, Some(labels), None, 0, false);
            expect(tester, &find::text("DESC"), finds_one_widget());
            expect(tester, &find::text("INACTIVE"), finds_one_widget());

            // Verify sorted column index change passes updated visible flags.
            pump(
                tester,
                Some(builder(|visible, _ascending| {
                    if visible {
                        "SORTED_COL1"
                    } else {
                        "UNSORTED_COL0"
                    }
                })),
                None,
                1,
                true,
            );
            expect(tester, &find::text("SORTED_COL1"), finds_one_widget());
            expect(tester, &find::text("UNSORTED_COL0"), finds_one_widget());

            // Verify DataTableThemeData.sortIconBuilder theme resolution.
            let theme_builder = builder(|_visible, _ascending| "THEME_BUILDER");
            let mut theme = DataTableThemeData::new();
            theme.sort_icon_builder = Some(theme_builder.clone());
            pump(tester, None, Some(theme.clone()), 0, true);
            expect(tester, &find::text("THEME_BUILDER"), finds_n_widgets(2));

            // Verify DataTable.sortIconBuilder overrides DataTableThemeData.sortIconBuilder.
            pump(
                tester,
                Some(builder(|_visible, _ascending| "WIDGET_BUILDER")),
                Some(theme),
                0,
                true,
            );
            expect(tester, &find::text("WIDGET_BUILDER"), finds_n_widgets(2));
            expect(tester, &find::text("THEME_BUILDER"), finds_nothing());
        });
    }

    fn padding_table(
        horizontal_margin: Option<f64>,
        column_spacing: Option<f64>,
        checkbox_horizontal_margin: Option<f64>,
        selectable: bool,
    ) -> DataTable {
        let rows = K_DESSERTS
            .iter()
            .map(|dessert| {
                let mut row = DataRow::new(vec![
                    DataCell::new(Text::new(dessert.name)),
                    DataCell::new(Text::new(dessert.calories.to_string()))
                        .show_edit_icon(true)
                        .on_tap(Listener::new(|_| {})),
                    DataCell::new(Text::new(format!("{:?}", dessert.fat)))
                        .show_edit_icon(true)
                        .on_tap(Listener::new(|_| {})),
                ])
                .key(ValueKey::new(dessert.name.to_owned()));
                if selectable {
                    row = row.on_select_changed(|_app, _selected| {});
                }
                row
            })
            .collect();
        let mut table = DataTable::new(
            vec![
                name_column(),
                calories_column()
                    .numeric(true)
                    .on_sort(|_app, _index, _ascending| {}),
                DataColumn::new(Text::new("Fat"))
                    .tooltip("Fat")
                    .numeric(true)
                    .on_sort(|_app, _index, _ascending| {}),
            ],
            rows,
        );
        if selectable {
            table = table.on_select_all(|_app, _value| {});
        }
        table.horizontal_margin = horizontal_margin;
        table.column_spacing = column_spacing;
        table.checkbox_horizontal_margin = checkbox_horizontal_margin;
        table
    }

    /// The distances Dart's padding tests read: the content's left from the padding's left, and
    /// the padding's right from the content's right.
    fn padding_around(tester: &WidgetTester, text: &str, first_padding: bool) -> (f64, f64) {
        let mut padding = find::widget_with_text::<Padding>(text);
        if first_padding {
            padding = padding.first();
        }
        let cell_content = find::widget_with_text::<Align>(text);
        (
            tester.get_rect(cell_content.clone()).left - tester.get_rect(padding.clone()).left,
            tester.get_rect(padding).right - tester.get_rect(cell_content).right,
        )
    }

    #[test]
    fn data_table_custom_horizontal_padding_checkbox() {
        test_widgets(|tester| {
            let default_horizontal_margin = 24.0;
            let default_column_spacing = 56.0;
            let custom_horizontal_margin = 10.0;
            let custom_column_spacing = 15.0;

            // DEFAULT VALUES
            let root = app_of(tester, padding_table(None, None, None, true));
            tester.pump_widget(root);

            // default checkbox padding
            let checkbox = find::by_type::<Checkbox>().first();
            let padding = find::ancestor(checkbox.clone(), find::by_type::<Padding>());
            assert_eq!(
                tester.get_rect(checkbox.clone()).left - tester.get_rect(padding.clone()).left,
                default_horizontal_margin
            );
            assert_eq!(
                tester.get_rect(padding).right - tester.get_rect(checkbox).right,
                default_horizontal_margin / 2.0
            );

            // default first column padding
            assert_eq!(
                padding_around(tester, "Frozen yogurt", false),
                (
                    default_horizontal_margin / 2.0,
                    default_column_spacing / 2.0
                )
            );
            // default middle column padding
            assert_eq!(
                padding_around(tester, "159", false),
                (default_column_spacing / 2.0, default_column_spacing / 2.0)
            );
            // default last column padding
            assert_eq!(
                padding_around(tester, "6.0", false),
                (default_column_spacing / 2.0, default_horizontal_margin)
            );

            // CUSTOM VALUES
            let root = app_of(
                tester,
                padding_table(
                    Some(custom_horizontal_margin),
                    Some(custom_column_spacing),
                    None,
                    true,
                ),
            );
            tester.pump_widget(root);

            // custom checkbox padding
            let checkbox = find::by_type::<Checkbox>().first();
            let padding = find::ancestor(checkbox.clone(), find::by_type::<Padding>());
            assert_eq!(
                tester.get_rect(checkbox.clone()).left - tester.get_rect(padding.clone()).left,
                custom_horizontal_margin
            );
            assert_eq!(
                tester.get_rect(padding).right - tester.get_rect(checkbox).right,
                custom_horizontal_margin / 2.0
            );

            // custom first column padding
            assert_eq!(
                padding_around(tester, "Frozen yogurt", true),
                (custom_horizontal_margin / 2.0, custom_column_spacing / 2.0)
            );
            // custom middle column padding
            assert_eq!(
                padding_around(tester, "159", false),
                (custom_column_spacing / 2.0, custom_column_spacing / 2.0)
            );
            // custom last column padding
            assert_eq!(
                padding_around(tester, "6.0", false),
                (custom_column_spacing / 2.0, custom_horizontal_margin)
            );
        });
    }

    #[test]
    fn data_table_custom_horizontal_padding_no_checkbox() {
        test_widgets(|tester| {
            let default_horizontal_margin = 24.0;
            let default_column_spacing = 56.0;
            let custom_horizontal_margin = 10.0;
            let custom_column_spacing = 15.0;

            // DEFAULT VALUES
            let root = app_of(tester, padding_table(None, None, None, false));
            tester.pump_widget(root);

            assert_eq!(
                padding_around(tester, "Frozen yogurt", false),
                (default_horizontal_margin, default_column_spacing / 2.0)
            );
            assert_eq!(
                padding_around(tester, "159", false),
                (default_column_spacing / 2.0, default_column_spacing / 2.0)
            );
            assert_eq!(
                padding_around(tester, "6.0", false),
                (default_column_spacing / 2.0, default_horizontal_margin)
            );

            // CUSTOM VALUES
            let root = app_of(
                tester,
                padding_table(
                    Some(custom_horizontal_margin),
                    Some(custom_column_spacing),
                    None,
                    false,
                ),
            );
            tester.pump_widget(root);

            assert_eq!(
                padding_around(tester, "Frozen yogurt", false),
                (custom_horizontal_margin, custom_column_spacing / 2.0)
            );
            assert_eq!(
                padding_around(tester, "159", false),
                (custom_column_spacing / 2.0, custom_column_spacing / 2.0)
            );
            assert_eq!(
                padding_around(tester, "6.0", false),
                (custom_column_spacing / 2.0, custom_horizontal_margin)
            );
        });
    }

    #[test]
    fn checkbox_horizontal_margin_properly_applied() {
        test_widgets(|tester| {
            let custom_checkbox_horizontal_margin = 15.0;
            let custom_horizontal_margin = 10.0;
            let root = app_of(
                tester,
                padding_table(
                    Some(custom_horizontal_margin),
                    None,
                    Some(custom_checkbox_horizontal_margin),
                    true,
                ),
            );
            tester.pump_widget(root);

            // Custom checkbox padding.
            let checkbox = find::by_type::<Checkbox>().first();
            let padding = find::ancestor(checkbox.clone(), find::by_type::<Padding>());
            assert_eq!(
                tester.get_rect(checkbox.clone()).left - tester.get_rect(padding.clone()).left,
                custom_checkbox_horizontal_margin
            );
            assert_eq!(
                tester.get_rect(padding).right - tester.get_rect(checkbox).right,
                custom_checkbox_horizontal_margin
            );

            // First column padding.
            assert_eq!(
                padding_around(tester, "Frozen yogurt", true).0,
                custom_horizontal_margin
            );
        });
    }

    fn two_row_table() -> (Vec<DataColumn>, Vec<DataRow>) {
        let cells = || {
            vec![
                DataCell::new(Text::new("cell1")),
                DataCell::new(Text::new("cell2")),
            ]
        };
        (
            vec![
                DataColumn::new(Text::new("column1")),
                DataColumn::new(Text::new("column2")),
            ],
            vec![DataRow::new(cells()), DataRow::new(cells())],
        )
    }

    fn last_row_decoration(tester: &WidgetTester) -> BoxDecoration {
        tester.widget(find::by_type::<Table>(), |table: &Table| {
            table
                .children
                .last()
                .expect("rows")
                .decoration
                .as_ref()
                .and_then(|decoration| decoration.as_any().downcast_ref::<BoxDecoration>())
                .expect("a BoxDecoration")
                .clone()
        })
    }

    #[test]
    fn data_table_set_border_width_test() {
        test_widgets(|tester| {
            // no thickness provided - border should be default: i.e "1.0" as it set in DataTable
            // constructor
            let (columns, rows) = two_row_table();
            let root = app_of(tester, DataTable::new(columns, rows));
            tester.pump_widget(root);
            assert_eq!(
                last_row_decoration(tester)
                    .border
                    .expect("a border")
                    .top()
                    .width,
                1.0
            );

            let thickness = 4.2;
            let (columns, rows) = two_row_table();
            let root = app_of(
                tester,
                DataTable::new(columns, rows).divider_thickness(thickness),
            );
            tester.pump_widget(root);
            assert_eq!(
                last_row_decoration(tester)
                    .border
                    .expect("a border")
                    .top()
                    .width,
                thickness
            );
        });
    }

    #[test]
    fn data_table_set_show_bottom_border() {
        test_widgets(|tester| {
            let (columns, rows) = two_row_table();
            let root = app_of(
                tester,
                DataTable::new(columns, rows).show_bottom_border(true),
            );
            tester.pump_widget(root);
            assert_eq!(
                last_row_decoration(tester)
                    .border
                    .expect("a border")
                    .bottom()
                    .width,
                1.0
            );

            let (columns, rows) = two_row_table();
            let root = app_of(tester, DataTable::new(columns, rows));
            tester.pump_widget(root);
            assert_eq!(
                last_row_decoration(tester)
                    .border
                    .expect("a border")
                    .bottom()
                    .width,
                0.0
            );
        });
    }

    #[test]
    fn data_table_column_heading_cell_with_and_without_sorting() {
        test_widgets(|tester| {
            let build_table = |sort_column_index: Option<usize>, sort_enabled: bool| {
                let mut column =
                    DataColumn::new(Expanded::new(Center::new().child(Text::new("Name"))))
                        .tooltip("Name");
                if sort_enabled {
                    column = column.on_sort(|_app, _index, _ascending| {});
                }
                let mut table = DataTable::new(
                    vec![column],
                    vec![DataRow::new(vec![DataCell::new(Text::new(
                        "A long desert name",
                    ))])],
                );
                table.sort_column_index = sort_column_index;
                table
            };
            let name_cell =
                || find::ancestor(find::text("Name"), find::by_type::<Container>()).first();

            // Start with without sorting
            let root = app_of(tester, build_table(None, false));
            tester.pump_widget(root);
            expect(tester, &find::text("Name"), finds_one_widget());
            // Within f32: global positions go through valo's f32 matrices (waft-gestures'
            // Matrix4 entry), where Dart's are f64 and compare equal.
            let (text_center, cell_center) = (
                tester.get_center(find::text("Name")),
                tester.get_center(name_cell()),
            );
            assert_f32_equals(text_center.dx(), cell_center.dx());
            assert_f32_equals(text_center.dy(), cell_center.dy());
            expect(
                tester,
                &find::descendant(name_cell(), find::by_type::<Icon>()),
                finds_nothing(),
            );

            // Turn on sorting
            let root = app_of(tester, build_table(None, true));
            tester.pump_widget(root);
            expect(tester, &find::text("Name"), finds_one_widget());
            expect(
                tester,
                &find::descendant(name_cell(), find::by_type::<Icon>()),
                finds_one_widget(),
            );

            // Turn off sorting again
            let root = app_of(tester, build_table(None, false));
            tester.pump_widget(root);
            expect(tester, &find::text("Name"), finds_one_widget());
            // Within f32: global positions go through valo's f32 matrices (waft-gestures'
            // Matrix4 entry), where Dart's are f64 and compare equal.
            let (text_center, cell_center) = (
                tester.get_center(find::text("Name")),
                tester.get_center(name_cell()),
            );
            assert_f32_equals(text_center.dx(), cell_center.dx());
            assert_f32_equals(text_center.dy(), cell_center.dy());
            expect(
                tester,
                &find::descendant(name_cell(), find::by_type::<Icon>()),
                finds_nothing(),
            );
        });
    }

    #[test]
    #[allow(deprecated)] // Dart's `withOpacity`.
    fn data_row_renders_default_selected_row_colors() {
        test_widgets(|tester| {
            let theme_data = m3(&tester.cell.borrow());
            let build_table = |selected: bool| {
                material_app(
                    theme_data.clone(),
                    Material::new()
                        .child(DataTable::new(
                            vec![DataColumn::new(Text::new("Column1"))],
                            vec![
                                DataRow::new(vec![DataCell::new(Text::new("Content1"))])
                                    .on_select_changed(|_app, _checked| {})
                                    .selected(selected),
                            ],
                        ))
                        .into_widget(),
                )
            };

            tester.pump_widget(build_table(false));
            assert_eq!(last_row_decoration(tester).color, None);

            tester.pump_widget(build_table(true));
            assert_eq!(
                last_row_decoration(tester).color.map(|color| color.color()),
                Some(theme_data.color_scheme.primary.with_opacity(0.08))
            );
        });
    }

    fn state_color_table(
        selected: bool,
        disabled: bool,
        selected_color: Color,
        disabled_color: Color,
        default_color: Color,
    ) -> WidgetRef {
        let color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Selected) {
                return Some(selected_color);
            }
            if states.contains(&WidgetState::Disabled) {
                return Some(disabled_color);
            }
            Some(default_color)
        });
        let mut rows = Vec::new();
        if disabled || !selected {
            rows.push(
                DataRow::new(vec![DataCell::new(Text::new("Content1"))])
                    .on_select_changed(|_app, _value| {}),
            );
        }
        let mut row = DataRow::new(vec![DataCell::new(Text::new("Content2"))])
            .selected(selected)
            .color(color);
        if !disabled {
            row = row.on_select_changed(|_app, _value| {});
        }
        rows.push(row);
        Material::new()
            .child(DataTable::new(
                vec![DataColumn::new(Text::new("Column1"))],
                rows,
            ))
            .into_widget()
    }

    #[test]
    fn data_row_renders_custom_colors_when_selected() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme.clone(),
                state_color_table(
                    false,
                    false,
                    Colors::GREEN.primary(),
                    Colors::GREY.primary(),
                    Colors::RED.primary(),
                ),
            ));
            assert_eq!(
                last_row_decoration(tester).color.map(|color| color.color()),
                Some(Colors::RED.primary())
            );

            tester.pump_widget(material_app(
                theme,
                state_color_table(
                    true,
                    false,
                    Colors::GREEN.primary(),
                    Colors::GREY.primary(),
                    Colors::RED.primary(),
                ),
            ));
            assert_eq!(
                last_row_decoration(tester).color.map(|color| color.color()),
                Some(Colors::GREEN.primary())
            );
        });
    }

    #[test]
    fn data_row_renders_custom_colors_when_disabled() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme.clone(),
                state_color_table(
                    false,
                    false,
                    Colors::GREEN.primary(),
                    Colors::GREY.primary(),
                    Colors::RED.primary(),
                ),
            ));
            assert_eq!(
                last_row_decoration(tester).color.map(|color| color.color()),
                Some(Colors::RED.primary())
            );

            tester.pump_widget(material_app(
                theme,
                state_color_table(
                    false,
                    true,
                    Colors::GREEN.primary(),
                    Colors::GREY.primary(),
                    Colors::RED.primary(),
                ),
            ));
            assert_eq!(
                last_row_decoration(tester).color.map(|color| color.color()),
                Some(Colors::GREY.primary())
            );
        });
    }

    #[test]
    fn material2_data_row_renders_custom_colors_when_pressed() {
        test_widgets(|tester| {
            let pressed_color = Color::new(0xff4caf50);
            let color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Pressed) {
                    return Some(pressed_color);
                }
                Some(Colors::TRANSPARENT)
            });
            let theme = ThemeData::builder()
                .use_material3(false)
                .build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(DataTable::new(
                        vec![DataColumn::new(Text::new("Column1"))],
                        vec![
                            DataRow::new(vec![DataCell::new(Text::new("Content1"))])
                                .color(color)
                                .on_select_changed(|_app, _value| {}),
                        ],
                    ))
                    .into_widget(),
            ));

            let mut gesture = TestPointer::touch(1);
            let center = tester.get_center(find::text("Content1"));
            tester.down(&mut gesture, center);
            tester.pump_by(Duration::from_millis(200)); // splash is well underway
            // valo records no circle of radius 0, which the splash has until its first frame
            // after this one; Flutter's `paints` sees the zero circle.
            tester.pump_by(Duration::from_millis(16));
            let ink_well = tester.element(find::by_type::<InkWell>());
            let ops = tester.nearest_material_ops(ink_well);
            assert!(
                paints(
                    &ops,
                    &[PaintCall::Circle {
                        x: Some(68.0),
                        y: Some(24.0),
                        radius: None,
                        color: Some(pressed_color),
                        style: None,
                    }],
                )
                .is_ok()
            );
            tester.up(&gesture);
        });
    }

    #[test]
    fn table_row_ink_well_can_handle_secondary_taps() {
        test_widgets(|tester| {
            let secondary_tapped = Rc::new(std::cell::Cell::new(false));
            let secondary_tapped_down = Rc::new(std::cell::Cell::new(false));
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(
                        Table::new()
                            .children(vec![TableRow::new().children(vec![
                            TableRowInkWell::new()
                                .on_secondary_tap({
                                    let secondary_tapped = Rc::clone(&secondary_tapped);
                                    Listener::new(move |_app| secondary_tapped.set(true))
                                })
                                .on_secondary_tap_down({
                                    let secondary_tapped_down = Rc::clone(&secondary_tapped_down);
                                    Rc::new(move |_app: &mut App, _details: TapDownDetails| {
                                        secondary_tapped_down.set(true)
                                    })
                                })
                                .child(SizedBox::new().width(100.0).height(100.0))
                                .into_widget(),
                    ])])
                            .into_widget(),
                    )
                    .into_widget(),
            ));
            assert!(!secondary_tapped.get());
            assert!(!secondary_tapped_down.get());
            expect(
                tester,
                &find::by_type::<TableRowInkWell>(),
                finds_one_widget(),
            );

            let center = tester.get_center(find::by_type::<TableRowInkWell>());
            let mut mouse = TestPointer::mouse(1, waft_gestures::K_SECONDARY_MOUSE_BUTTON);
            tester.down(&mut mouse, center);
            tester.up(&mouse);
            tester.pump_and_settle();

            assert!(secondary_tapped.get());
            assert!(secondary_tapped_down.get());
        });
    }

    #[test]
    fn data_table_correctly_renders_with_a_mouse() {
        test_widgets(|tester| {
            let build_table = |sort_column_index: Option<usize>| {
                let mut table = DataTable::new(
                    vec![
                        DataColumn::new(Expanded::new(Center::new().child(Text::new("column1"))))
                            .tooltip("Column1"),
                        DataColumn::new(Expanded::new(Center::new().child(Text::new("column2"))))
                            .tooltip("Column2")
                            .on_sort(|_app, _index, _ascending| {}),
                    ],
                    vec![DataRow::new(vec![
                        DataCell::new(Text::new("Content1")),
                        DataCell::new(Text::new("Content2")),
                    ])],
                );
                table.sort_column_index = sort_column_index;
                table
            };
            let root = app_of(tester, build_table(None));
            tester.pump_widget(root);

            let attached = |tester: &WidgetTester, text: &str| {
                let render = tester.render_object(find::text(text));
                render.attached(&tester.cell.borrow())
            };
            assert!(attached(tester, "column1"));
            assert!(attached(tester, "column2"));

            let mut gesture = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);

            tester.pump_and_settle();
            assert!(attached(tester, "column1"));
            assert!(attached(tester, "column2"));

            // Wait for the tooltip timer to expire to prevent it scheduling a new frame after
            // the view is destroyed, which causes exceptions.
            tester.pump_and_settle_by(Duration::from_secs(1));
        });
    }

    #[test]
    fn data_row_is_disabled_when_on_select_changed_is_not_set() {
        test_widgets(|tester| {
            let table = DataTable::new(
                vec![
                    DataColumn::new(Text::new("Col1")),
                    DataColumn::new(Text::new("Col2")),
                ],
                vec![
                    DataRow::new(vec![
                        DataCell::new(Text::new("Hello")),
                        DataCell::new(Text::new("world")),
                    ])
                    .on_select_changed(|_app, _value| {}),
                    DataRow::new(vec![
                        DataCell::new(Text::new("Bug")),
                        DataCell::new(Text::new("report")),
                    ]),
                    DataRow::new(vec![
                        DataCell::new(Text::new("GitHub")),
                        DataCell::new(Text::new("issue")),
                    ]),
                ],
            );
            let root = app_of(tester, table);
            tester.pump_widget(root);
            expect(
                tester,
                &find::widget_with_text::<TableRowInkWell>("Hello"),
                finds_one_widget(),
            );
            expect(
                tester,
                &find::widget_with_text::<TableRowInkWell>("Bug"),
                finds_nothing(),
            );
            expect(
                tester,
                &find::widget_with_text::<TableRowInkWell>("GitHub"),
                finds_nothing(),
            );
        });
    }

    #[test]
    fn data_table_set_interior_border_test() {
        test_widgets(|tester| {
            let (columns, rows) = two_row_table();
            let root = app_of(
                tester,
                DataTable::new(columns, rows).border(TableBorder::all(
                    Colors::RED.primary(),
                    2.0,
                    BorderStyle::Solid,
                    BorderRadius::ZERO,
                )),
            );
            tester.pump_widget(root);
            assert_eq!(
                tester.get_size(find::by_type::<DataTable>()),
                Size::new(800.0, 600.0)
            );

            let (columns, rows) = two_row_table();
            let root = app_of(
                tester,
                DataTable::new(columns, rows).border(TableBorder::all(
                    Colors::RED.primary(),
                    1.0,
                    BorderStyle::Solid,
                    BorderRadius::ZERO,
                )),
            );
            tester.pump_widget(root);
            let border = tester.widget(find::by_type::<Table>(), |table: &Table| table.border);
            let border = border.expect("a border");
            assert_eq!(border.top.color, Colors::RED.primary());
            assert_eq!(border.bottom.width, 1.0);

            let (columns, rows) = two_row_table();
            let root = app_of(tester, DataTable::new(columns, rows));
            tester.pump_widget(root);
            let border = tester.widget(find::by_type::<Table>(), |table: &Table| table.border);
            assert!(border.is_none());
        });
    }

    #[test]
    fn do_not_crashes_when_paint_borders_in_a_narrow_space() {
        test_widgets(|tester| {
            let (columns, rows) = two_row_table();
            let root = app_of(
                tester,
                Center::new().child(SizedBox::new().width(117.0).child(
                    DataTable::new(columns, rows).border(TableBorder::all(
                        Colors::RED.primary(),
                        2.0,
                        BorderStyle::Solid,
                        BorderRadius::ZERO,
                    )),
                )),
            );
            tester.pump_widget(root);
            // Go without crashes.
        });
    }

    #[test]
    fn data_table_clip_behavior() {
        test_widgets(|tester| {
            let border_radius = BorderRadius::circular(30.0);
            let theme = m3(&tester.cell.borrow());
            let build_table = |clip_behavior: Clip| {
                let border =
                    TableBorder::all(Colors::BLACK, 1.0, BorderStyle::Solid, border_radius);
                material_app(
                    theme.clone(),
                    Material::new()
                        .child(
                            DataTable::new(
                                vec![DataColumn::new(Text::new("Column1"))],
                                vec![DataRow::new(vec![DataCell::new(Text::new("Content1"))])],
                            )
                            .clip_behavior(clip_behavior)
                            .border(border),
                        )
                        .into_widget(),
                )
            };

            tester.pump_widget(build_table(Clip::None));
            let (clip_behavior, radius) = tester
                .widget(find::by_type::<Material>().last(), |material: &Material| {
                    (material.clip_behavior, material.border_radius)
                });
            assert_eq!(clip_behavior, Clip::None);
            assert_eq!(radius, Some(border_radius.into()));

            tester.pump_widget(build_table(Clip::HardEdge));
            let (clip_behavior, radius) = tester
                .widget(find::by_type::<Material>().last(), |material: &Material| {
                    (material.clip_behavior, material.border_radius)
                });
            assert_eq!(clip_behavior, Clip::HardEdge);
            assert_eq!(radius, Some(border_radius.into()));
        });
    }

    fn mouse_cursor_table(columns: Vec<DataColumn>, rows: Vec<DataRow>) -> DataTable {
        DataTable::new(columns, rows).sort_column_index(0)
    }

    fn cursor_by_state(
        selected_or_disabled: WidgetState,
        when: MouseCursorRef,
        otherwise: MouseCursorRef,
    ) -> WidgetStatePropertyRef<Option<MouseCursorRef>> {
        <dyn WidgetStateProperty<Option<MouseCursorRef>>>::resolve_with(move |states| {
            if states.contains(&selected_or_disabled) {
                return Some(when.clone());
            }
            Some(otherwise.clone())
        })
    }

    fn assert_active_cursor(tester: &WidgetTester, expected: MouseCursorRef) {
        assert!(
            *tester.active_cursor(1) == *expected,
            "the active cursor is {:?}, not {expected:?}",
            tester.active_cursor(1)
        );
    }

    #[test]
    fn heading_cell_cursor_resolves_widget_state_mouse_cursor_correctly() {
        test_widgets(|tester| {
            let cursor = || {
                cursor_by_state(
                    WidgetState::Disabled,
                    SystemMouseCursors::FORBIDDEN.into(),
                    SystemMouseCursors::COPY.into(),
                )
            };
            let table = mouse_cursor_table(
                vec![
                    // This column can be sorted.
                    DataColumn::new(Text::new("A"))
                        .mouse_cursor(cursor())
                        .on_sort(|_app, _index, _ascending| {}),
                    // This column cannot be sorted.
                    DataColumn::new(Text::new("B")).mouse_cursor(cursor()),
                ],
                vec![
                    DataRow::new(vec![
                        DataCell::new(Text::new("Data 1")),
                        DataCell::new(Text::new("Data 2")),
                    ]),
                    DataRow::new(vec![
                        DataCell::new(Text::new("Data 3")),
                        DataCell::new(Text::new("Data 4")),
                    ]),
                ],
            );
            let root = material_app(
                m3(&tester.cell.borrow()),
                Scaffold::new().body(table).into_widget(),
            );
            tester.pump_widget(root);

            let mut gesture = TestPointer::mouse(1, 0);
            let at = tester.get_center(find::text("A"));
            tester.add_pointer(&mut gesture, at);
            tester.pump();
            assert_active_cursor(tester, SystemMouseCursors::COPY.into());

            let at = tester.get_center(find::text("B"));
            tester.move_to(&mut gesture, at, false);
            tester.pump();
            assert_active_cursor(tester, SystemMouseCursors::FORBIDDEN.into());
        });
    }

    #[test]
    fn data_row_cursor_resolves_widget_state_mouse_cursor_correctly() {
        test_widgets(|tester| {
            let cursor = || {
                cursor_by_state(
                    WidgetState::Selected,
                    SystemMouseCursors::COPY.into(),
                    SystemMouseCursors::FORBIDDEN.into(),
                )
            };
            let table = mouse_cursor_table(
                vec![
                    DataColumn::new(Text::new("A")).on_sort(|_app, _index, _ascending| {}),
                    DataColumn::new(Text::new("B")),
                ],
                vec![
                    // This row can be selected.
                    DataRow::new(vec![
                        DataCell::new(Text::new("Data 1")),
                        DataCell::new(Text::new("Data 2")),
                    ])
                    .mouse_cursor(cursor())
                    .on_select_changed(|_app, _selected| {}),
                    // This row is selected.
                    DataRow::new(vec![
                        DataCell::new(Text::new("Data 3")),
                        DataCell::new(Text::new("Data 4")),
                    ])
                    .selected(true)
                    .on_select_changed(|_app, _selected| {})
                    .mouse_cursor(cursor()),
                ],
            );
            let root = material_app(
                m3(&tester.cell.borrow()),
                Scaffold::new().body(table).into_widget(),
            );
            tester.pump_widget(root);

            let mut gesture = TestPointer::mouse(1, 0);
            let at = tester.get_center(find::text("Data 1"));
            tester.add_pointer(&mut gesture, at);
            tester.pump();
            assert_active_cursor(tester, SystemMouseCursors::FORBIDDEN.into());

            let at = tester.get_center(find::text("Data 3"));
            tester.move_to(&mut gesture, at, false);
            tester.pump();
            assert_active_cursor(tester, SystemMouseCursors::COPY.into());
        });
    }

    #[test]
    fn data_row_cursor_doesnt_update_checkbox_cursor() {
        test_widgets(|tester| {
            let copy: WidgetStatePropertyRef<Option<MouseCursorRef>> = Rc::new(
                WidgetStatePropertyAll::new(Some(SystemMouseCursors::COPY.into())),
            );
            let table = mouse_cursor_table(
                vec![
                    DataColumn::new(Text::new("A")).on_sort(|_app, _index, _ascending| {}),
                    DataColumn::new(Text::new("B")),
                ],
                vec![
                    DataRow::new(vec![
                        DataCell::new(Text::new("Data")),
                        DataCell::new(Text::new("Data 2")),
                    ])
                    .on_select_changed(|_app, _selected| {})
                    .mouse_cursor(copy),
                ],
            );
            let root = material_app(
                m3(&tester.cell.borrow()),
                Scaffold::new().body(table).into_widget(),
            );
            tester.pump_widget(root);

            let mut gesture = TestPointer::mouse(1, 0);
            let at = tester.get_center(find::by_type::<Checkbox>().last());
            tester.add_pointer(&mut gesture, at);
            tester.pump();
            // Test that the checkbox cursor is not changed.
            assert_active_cursor(tester, SystemMouseCursors::BASIC.into());

            let at = tester.get_center(find::text("Data"));
            tester.move_to(&mut gesture, at, false);
            tester.pump();
            // Test that cursor is updated for the row.
            assert_active_cursor(tester, SystemMouseCursors::COPY.into());
        });
    }

    #[test]
    fn data_table_text_styles_are_merged_with_default_text_style() {
        test_widgets(|tester| {
            let default_text_style: Rc<RefCell<Option<DefaultTextStyle>>> = Rc::default();
            let slot = Rc::clone(&default_text_style);
            let body = Builder::new(move |app, context| {
                *slot.borrow_mut() = Some(DefaultTextStyle::of(app, context));
                DataTable::new(
                    vec![
                        DataColumn::new(Text::new("Header 1")),
                        DataColumn::new(Text::new("Header 2")),
                    ],
                    vec![DataRow::new(vec![
                        DataCell::new(Text::new("Data 1")),
                        DataCell::new(Text::new("Data 2")),
                    ])],
                )
                .heading_text_style(TextStyle::new())
                .data_text_style(TextStyle::new())
                .into_widget()
            });
            let root = material_app(
                m3(&tester.cell.borrow()),
                Scaffold::new().body(body).into_widget(),
            );
            tester.pump_widget(root);

            let default_style = default_text_style
                .borrow()
                .clone()
                .expect("the builder ran")
                .style;
            assert_eq!(paragraph_style(tester, "Header 1"), default_style);
            assert_eq!(paragraph_style(tester, "Data 1"), default_style);
        });
    }

    #[test]
    fn data_column_label_can_be_centered() {
        test_widgets(|tester| {
            let horizontal_margin = 24.0;
            let build_table = |heading_row_alignment: Option<MainAxisAlignment>,
                               sort_enabled: bool| {
                let mut column = DataColumn::new(Text::new("Header"));
                column.heading_row_alignment = heading_row_alignment;
                if sort_enabled {
                    column = column.on_sort(|_app, _index, _ascending| {});
                }
                DataTable::new(
                    vec![column],
                    vec![DataRow::new(vec![DataCell::new(Text::new("Data"))])],
                )
            };

            // Test mainAxisAlignment without sort arrow.
            let root = app_of(tester, build_table(None, false));
            tester.pump_widget(root);
            assert_eq!(
                tester.get_top_left(find::text("Header")).dx(),
                horizontal_margin
            );

            // Test mainAxisAlignment.center without sort arrow.
            let root = app_of(tester, build_table(Some(MainAxisAlignment::Center), false));
            tester.pump_widget(root);
            assert_f32_equals(tester.get_center(find::text("Header")).dx(), 400.0);

            // Test mainAxisAlignment with sort arrow.
            let root = app_of(tester, build_table(None, true));
            tester.pump_widget(root);
            assert_eq!(
                tester.get_top_left(find::text("Header")).dx(),
                horizontal_margin
            );

            // Test mainAxisAlignment.center with sort arrow.
            let root = app_of(tester, build_table(Some(MainAxisAlignment::Center), true));
            tester.pump_widget(root);
            assert_f32_equals(tester.get_center(find::text("Header")).dx(), 400.0);
        });
    }

    fn custom_widths_table(selectable: bool) -> WidgetRef {
        let mut row = DataRow::new(vec![
            DataCell::new(Text::new("1")),
            DataCell::new(Text::new("1")),
            DataCell::new(Text::new("D")),
        ]);
        if selectable {
            row = row.on_select_changed(|_app, _value| {});
        }
        SizedBox::new()
            .width(500.0)
            .child(DataTable::new(
                vec![
                    DataColumn::new(Text::new("Flex Numeric"))
                        .column_width(Rc::new(FlexColumnWidth::new(1.0)))
                        .numeric(true),
                    DataColumn::new(Text::new("Numeric")).numeric(true),
                    DataColumn::new(Text::new("Text")),
                ],
                vec![row],
            ))
            .into_widget()
    }

    fn column_width_debug(tester: &WidgetTester, column: usize) -> String {
        tester.widget(find::by_type::<Table>(), |table: &Table| {
            format!(
                "{:?}",
                table.column_widths.as_ref().expect("column widths")[&column]
            )
        })
    }

    #[test]
    fn data_table_with_custom_column_widths_checkbox() {
        test_widgets(|tester| {
            let root = app_of(tester, custom_widths_table(true));
            tester.pump_widget(root);
            assert!(column_width_debug(tester, 0).starts_with("FixedColumnWidth")); // Checkbox column
            assert_eq!(
                column_width_debug(tester, 1),
                format!("{:?}", FlexColumnWidth::new(1.0))
            );
            assert_eq!(
                column_width_debug(tester, 2),
                format!("{:?}", IntrinsicColumnWidth::new(None))
            );
            assert_eq!(
                column_width_debug(tester, 3),
                format!("{:?}", IntrinsicColumnWidth::new(Some(1.0)))
            );
        });
    }

    #[test]
    fn data_table_with_custom_column_widths_no_checkbox() {
        test_widgets(|tester| {
            let root = app_of(tester, custom_widths_table(false));
            tester.pump_widget(root);
            assert_eq!(
                column_width_debug(tester, 0),
                format!("{:?}", FlexColumnWidth::new(1.0))
            );
            assert_eq!(
                column_width_debug(tester, 1),
                format!("{:?}", IntrinsicColumnWidth::new(None))
            );
            assert_eq!(
                column_width_debug(tester, 2),
                format!("{:?}", IntrinsicColumnWidth::new(Some(1.0)))
            );
        });
    }

    #[test]
    fn data_table_data_column_data_row_and_data_cell_render_at_zero_area() {
        test_widgets(|tester| {
            let root = app_of(
                tester,
                SizedBox::shrink().child(DataTable::new(
                    vec![DataColumn::new(Text::new("X"))],
                    vec![DataRow::new(vec![DataCell::new(Text::new("X"))])],
                )),
            );
            tester.pump_widget(root);
        });
    }

    #[test]
    fn table_row_ink_well_renders_at_zero_area() {
        test_widgets(|tester| {
            let table = Table::new().children(vec![TableRow::new().children(vec![
                TableRowInkWell::new().child(Text::new("X")).into_widget(),
            ])]);
            let root = app_of(tester, SizedBox::shrink().child(table.into_widget()));
            tester.pump_widget(root);
        });
    }

    #[test]
    fn data_table_renders_with_border_and_background_decoration() {
        test_widgets(|tester| {
            let (width, height) = (800.0, 600.0);
            let (border_horizontal, border_vertical) = (5.0, 10.0);
            let border_color = Color::new(0xff2196f3);
            let background_color = Color::new(0xfff5f5f5);
            let vertical = BorderSide {
                width: border_vertical,
                color: border_color,
                ..BorderSide::default()
            };
            let horizontal = BorderSide {
                width: border_horizontal,
                color: border_color,
                ..BorderSide::default()
            };
            let table = DataTable::new(
                vec![DataColumn::new(Text::new("Col1"))],
                vec![DataRow::new(vec![DataCell::new(Text::new("1"))])],
            )
            .decoration(
                BoxDecoration::new()
                    .color(background_color)
                    .border(Border::symmetric(vertical, horizontal)),
            );
            let root = material_app(m3(&tester.cell.borrow()), table.into_widget());
            tester.pump_widget(root);

            let container = find::ancestor(find::by_type::<Table>(), find::by_type::<Container>());
            let ops = tester.painted_ops(tester.render_object(container));
            assert!(
                paints(
                    &ops,
                    &[PaintCall::rect_at(
                        Rect::from_ltrb(
                            border_vertical / 2.0,
                            border_horizontal / 2.0,
                            width - border_vertical / 2.0,
                            height - border_horizontal / 2.0,
                        ),
                        Some(background_color),
                    )]
                )
                .is_ok()
            );
            assert!(paints(&ops, &[PaintCall::path(Some(border_color))]).is_ok());
            assert_eq!(
                tester.get_top_left(find::by_type::<Table>()),
                Offset::new(border_vertical, border_horizontal)
            );
            assert_eq!(
                tester.get_bottom_right(find::by_type::<Table>()),
                Offset::new(width - border_vertical, height - border_horizontal)
            );
        });
    }

    #[test]
    fn data_row_renders_checkbox_with_colors_from_checkbox_theme() {
        test_widgets(|tester| {
            let fill_color = Color::new(0xFF00FF00);
            let check_color = Color::new(0xFF0000FF);
            let theme = ThemeData::builder()
                .checkbox_theme(
                    CheckboxThemeData::new()
                        .fill_color(Rc::new(WidgetStatePropertyAll::new(Some(fill_color))))
                        .check_color(Rc::new(WidgetStatePropertyAll::new(Some(check_color)))),
                )
                .build(&tester.cell.borrow());
            let table = DataTable::new(
                vec![DataColumn::new(Text::new("Column1"))],
                vec![
                    DataRow::new(vec![DataCell::new(Text::new("Content1"))])
                        .selected(true)
                        .on_select_changed(|_app, _checked| {}),
                ],
            );
            let root = material_app(theme, Material::new().child(table).into_widget());
            tester.pump_widget(root);

            let checkbox = tester.element(find::by_type::<Checkbox>().last());
            let ops = tester.nearest_material_ops(checkbox);
            assert!(
                paints(
                    &ops,
                    &[
                        PaintCall::rrect(Some(fill_color)),
                        PaintCall::path(Some(check_color)),
                    ]
                )
                .is_ok()
            );
        });
    }
}
