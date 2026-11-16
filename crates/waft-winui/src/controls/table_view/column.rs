//! `TableViewColumn`, `TableViewTextColumn` and `TableViewTemplateColumn` (`controls/dev/TableView/TableViewColumn.cpp`, `TableViewTextColumn.cpp`, `TableViewTemplateColumn.cpp`): what a column shows in each cell and its header, how wide it is, whether it is frozen, and how it sorts.
//!
//! A column is an object the application keeps and the `TableView` writes to, as WinUI's `DependencyObject` is: the table writes `actual_width` when it resolves the widths, `sort_direction` when it sorts, and `width` when the user drags the column's gripper. Every property setter notifies with the [`ColumnChange`] it made, which the owning table reads to decide what to rebuild; a later editor who adds a property without a change kind leaves the table showing the old value.

use super::source::{SortComparer, SortDirection};
use crate::*;
use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;
use waft_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, HandleId, Ref};
use waft_painting::{AlignmentGeometry, EdgeInsetsGeometry, TextOverflow};
use waft_widgets::*;

/// `c_widthDefault`: the width of a column whose `Width` is not set, and the provisional width of an `Auto` column before its cells are measured.
pub(crate) const DEFAULT_WIDTH: f64 = 120.0;

/// `MinWidth`'s default.
const DEFAULT_MIN_WIDTH: f64 = 20.0;

/// XAML `TableViewFrozenEdge`: `Leading` pins a column in the contiguous prefix from the first column; `Trailing` is reserved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TableViewFrozenEdge {
    #[default]
    None,
    Leading,
    Trailing,
}

/// XAML `TableViewSortCycle`: the directions repeated header clicks walk. The first is the direction an unsorted column opens in; the `None` variants add a step back to source order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TableViewSortCycle {
    #[default]
    AscendingDescending,
    AscendingDescendingNone,
    DescendingAscending,
    DescendingAscendingNone,
}

/// XAML `Visibility`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Visibility {
    #[default]
    Visible,
    Collapsed,
}

/// Which property of a column last changed: the branch of `TableViewColumn::OnPropertyChanged` the owning table takes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) enum ColumnChange {
    #[default]
    None,
    /// `Width`, `MinWidth` or `MaxWidth`: `OnColumnWidthChanged`.
    Width,
    /// `Header`: `OnColumnHeaderChanged`.
    Header,
    /// `HeaderToolTip`: `OnColumnHeaderToolTipChanged`.
    HeaderToolTip,
    /// `FrozenEdge`: `OnColumnFrozenEdgeChanged`.
    FrozenEdge,
    /// `CanResize`: `QueueRebuildHeaders`.
    CanResize,
    /// `Visibility`: `OnColumnVisibilityChanged`.
    Visibility,
    /// `CanSort`: `OnColumnCanSortChanged`.
    CanSort,
    /// `CustomSortComparer`: the active direction is applied again with the new comparer.
    CustomSortComparer,
    /// `SortDirection`, written by the owning table: `RefreshSortIndicators`.
    SortDirection,
    /// A property the table reads when it needs it (`SortCycle`, `SortMemberPath`).
    Other,
}

/// What a text column's cell takes from its table: `GetDensityCellPadding` and `GetCellFontSize`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TableViewCellStyle {
    pub(crate) padding: [f64; 4],
    pub(crate) font_size: f64,
}

/// Builds a template column's cell for an item: XAML's `CellTemplate`.
pub type TableViewCellTemplate<T> = Rc<dyn Fn(&mut App, BuildContext, &T) -> WidgetRef>;

/// The derived column's own part: a text column's `Binding`, or a template column's `CellTemplate`.
enum ColumnCell<T> {
    /// `TableViewTextColumn.Binding`: the text of each cell.
    Text(Rc<dyn Fn(&T) -> String>),
    /// `TableViewTemplateColumn.CellTemplate`.
    Template(TableViewCellTemplate<T>),
}

/// XAML `TableViewColumn`: one column of a `TableView` over items of type `T`.
pub struct TableViewColumn<T> {
    notifier: ChangeNotifierData,
    cell: ColumnCell<T>,
    header: String,
    header_tool_tip: Option<String>,
    width: GridLength,
    min_width: f64,
    /// Whether `MinWidth` was set, which a `0*` column honours (`MinWidthForStarFactor`).
    min_width_set: bool,
    max_width: f64,
    can_resize: bool,
    actual_width: f64,
    frozen_edge: TableViewFrozenEdge,
    visibility: Visibility,
    can_sort: bool,
    sort_cycle: TableViewSortCycle,
    /// `SortMemberPath` as the key it names.
    sort_key: Option<SortComparer<T>>,
    custom_sort_comparer: Option<SortComparer<T>>,
    sort_direction: SortDirection,
    /// `m_owningTableView`: the table the column belongs to, at most one.
    owner: Option<HandleId>,
    change: ColumnChange,
}

impl<T: 'static> TableViewColumn<T> {
    fn new(app: &mut App, header: String, cell: ColumnCell<T>) -> Ref<Self> {
        app.create(Self {
            notifier: ChangeNotifierData::new(),
            cell,
            header,
            header_tool_tip: None,
            width: GridLength::pixel(DEFAULT_WIDTH),
            min_width: DEFAULT_MIN_WIDTH,
            min_width_set: false,
            max_width: f64::INFINITY,
            can_resize: true,
            actual_width: DEFAULT_WIDTH,
            frozen_edge: TableViewFrozenEdge::None,
            visibility: Visibility::Visible,
            can_sort: true,
            sort_cycle: TableViewSortCycle::AscendingDescending,
            sort_key: None,
            custom_sort_comparer: None,
            sort_direction: SortDirection::None,
            owner: None,
            change: ColumnChange::None,
        })
    }

    /// XAML `TableViewTextColumn` with its `Header` and `Binding`: each cell shows the text `binding` gives for the row's item, and the column sorts by that text unless given a key of its own.
    pub fn text(
        app: &mut App,
        header: impl Into<String>,
        binding: impl Fn(&T) -> String + 'static,
    ) -> Ref<Self> {
        Self::new(app, header.into(), ColumnCell::Text(Rc::new(binding)))
    }

    /// XAML `TableViewTemplateColumn` with its `Header` and `CellTemplate`: each cell is what `cell_template` builds for the row's item. It sorts only by a key or comparer it is given.
    pub fn template(
        app: &mut App,
        header: impl Into<String>,
        cell_template: impl Fn(&mut App, BuildContext, &T) -> WidgetRef + 'static,
    ) -> Ref<Self> {
        Self::new(
            app,
            header.into(),
            ColumnCell::Template(Rc::new(cell_template)),
        )
    }

    /// Releases the listeners; the column is not used after.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        ChangeNotifierData::dispose(app, self);
    }

    /// `Header`.
    pub fn header(self: Handle<Self>, app: &App) -> &str {
        &app.get(self).header
    }

    /// Sets `Header`.
    pub fn set_header(self: Handle<Self>, app: &mut App, value: impl Into<String>) {
        let value = value.into();
        if app.get(self).header != value {
            app.get_mut(self).header = value;
            self.changed(app, ColumnChange::Header);
        }
    }

    /// `HeaderToolTip`: the header's tooltip; `None` or empty shows none.
    pub fn header_tool_tip(self: Handle<Self>, app: &App) -> Option<&str> {
        app.get(self).header_tool_tip.as_deref()
    }

    /// Sets `HeaderToolTip`.
    pub fn set_header_tool_tip(self: Handle<Self>, app: &mut App, value: Option<String>) {
        if app.get(self).header_tool_tip != value {
            app.get_mut(self).header_tool_tip = value;
            self.changed(app, ColumnChange::HeaderToolTip);
        }
    }

    /// `Width`: pixels, `Auto` (the widest realized cell) or a star share of the space the fixed columns leave.
    pub fn width(self: Handle<Self>, app: &App) -> GridLength {
        app.get(self).width
    }

    /// Sets `Width`.
    pub fn set_width(self: Handle<Self>, app: &mut App, value: GridLength) {
        if app.get(self).width != value {
            app.get_mut(self).width = value;
            self.width_changed(app);
        }
    }

    /// `MinWidth`.
    pub fn min_width(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).min_width
    }

    /// Sets `MinWidth`.
    pub fn set_min_width(self: Handle<Self>, app: &mut App, value: f64) {
        let column = app.get_mut(self);
        column.min_width_set = true;
        if column.min_width != value {
            column.min_width = value;
            self.width_changed(app);
        }
    }

    /// `MaxWidth`.
    pub fn max_width(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).max_width
    }

    /// Sets `MaxWidth`.
    pub fn set_max_width(self: Handle<Self>, app: &mut App, value: f64) {
        if app.get(self).max_width != value {
            app.get_mut(self).max_width = value;
            self.width_changed(app);
        }
    }

    /// `CanResize`: whether this column shows a gripper; the table's `can_user_resize_columns` gates them all.
    pub fn can_resize(self: Handle<Self>, app: &App) -> bool {
        app.get(self).can_resize
    }

    /// Sets `CanResize`.
    pub fn set_can_resize(self: Handle<Self>, app: &mut App, value: bool) {
        if app.get(self).can_resize != value {
            app.get_mut(self).can_resize = value;
            self.changed(app, ColumnChange::CanResize);
        }
    }

    /// `ActualWidth`: the width the table last resolved, clamped to `MinWidth` and `MaxWidth`. Read-only.
    pub fn actual_width(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).actual_width
    }

    /// `FrozenEdge`.
    pub fn frozen_edge(self: Handle<Self>, app: &App) -> TableViewFrozenEdge {
        app.get(self).frozen_edge
    }

    /// Sets `FrozenEdge`.
    pub fn set_frozen_edge(self: Handle<Self>, app: &mut App, value: TableViewFrozenEdge) {
        if app.get(self).frozen_edge != value {
            app.get_mut(self).frozen_edge = value;
            self.changed(app, ColumnChange::FrozenEdge);
        }
    }

    /// `Visibility`.
    pub fn visibility(self: Handle<Self>, app: &App) -> Visibility {
        app.get(self).visibility
    }

    /// Sets `Visibility`.
    pub fn set_visibility(self: Handle<Self>, app: &mut App, value: Visibility) {
        if app.get(self).visibility != value {
            app.get_mut(self).visibility = value;
            self.changed(app, ColumnChange::Visibility);
        }
    }

    /// `CanSort`: whether a header click sorts this column; sorting from code still works.
    pub fn can_sort(self: Handle<Self>, app: &App) -> bool {
        app.get(self).can_sort
    }

    /// Sets `CanSort`.
    pub fn set_can_sort(self: Handle<Self>, app: &mut App, value: bool) {
        if app.get(self).can_sort != value {
            app.get_mut(self).can_sort = value;
            self.changed(app, ColumnChange::CanSort);
        }
    }

    /// `SortCycle`, read when the header is clicked.
    pub fn sort_cycle(self: Handle<Self>, app: &App) -> TableViewSortCycle {
        app.get(self).sort_cycle
    }

    /// Sets `SortCycle`.
    pub fn set_sort_cycle(self: Handle<Self>, app: &mut App, value: TableViewSortCycle) {
        app.get_mut(self).sort_cycle = value;
    }

    /// Sets `SortMemberPath`, as the key it names: the column sorts its rows by `key`. A text column without one sorts by its cells' text.
    pub fn set_sort_key<K: Ord>(
        self: Handle<Self>,
        app: &mut App,
        key: impl Fn(&T) -> K + 'static,
    ) {
        app.get_mut(self).sort_key = Some(Rc::new(move |a, b| key(a).cmp(&key(b))));
        self.changed(app, ColumnChange::Other);
    }

    /// Sets `CustomSortComparer`, which takes precedence over the sort key. A column sorted at the time is sorted again with it.
    pub fn set_custom_sort_comparer(
        self: Handle<Self>,
        app: &mut App,
        comparer: impl Fn(&T, &T) -> Ordering + 'static,
    ) {
        app.get_mut(self).custom_sort_comparer = Some(Rc::new(comparer));
        self.changed(app, ColumnChange::CustomSortComparer);
    }

    /// `SortDirection`: `None` unless the column is part of the active sort. Written by the owning table; read-only.
    pub fn sort_direction(self: Handle<Self>, app: &App) -> SortDirection {
        app.get(self).sort_direction
    }
}

// MARK: The owning table's view of the column

impl<T: 'static> TableViewColumn<T> {
    /// `SetOwningTableViewInternal`: a column belongs to one table; a second is refused while the first holds it.
    pub(crate) fn set_owner(self: Handle<Self>, app: &mut App, owner: Option<HandleId>) -> bool {
        let column = app.get_mut(self);
        if let (Some(existing), Some(new)) = (column.owner, owner)
            && existing != new
        {
            debug_assert!(
                false,
                "TableViewColumn re-owned without removing it from the previous TableView first"
            );
            return false;
        }
        column.owner = owner;
        true
    }

    pub(crate) fn owner(self: Handle<Self>, app: &App) -> Option<HandleId> {
        app.get(self).owner
    }

    pub(crate) fn change(self: Handle<Self>, app: &App) -> ColumnChange {
        app.get(self).change
    }

    pub(crate) fn min_width_set(self: Handle<Self>, app: &App) -> bool {
        app.get(self).min_width_set
    }

    /// `SetResolvedActualWidthInternal`: the width the table resolved. It does not notify: the table writes it while laying itself out.
    pub(crate) fn set_actual_width(self: Handle<Self>, app: &mut App, width: f64) {
        app.get_mut(self).actual_width = width;
    }

    /// `SetSortStateInternal`.
    pub(crate) fn set_sort_state(self: Handle<Self>, app: &mut App, direction: SortDirection) {
        if app.get(self).sort_direction != direction {
            app.get_mut(self).sort_direction = direction;
            self.changed(app, ColumnChange::SortDirection);
        }
    }

    /// `GetSortMemberPathCore` with `CustomSortComparer` ahead of it: what the column sorts by, if anything. A text column falls back to its cells' text.
    pub(crate) fn sort_comparer(self: Handle<Self>, app: &App) -> Option<SortComparer<T>> {
        let column = app.get(self);
        if let Some(comparer) = &column.custom_sort_comparer {
            return Some(comparer.clone());
        }
        if let Some(key) = &column.sort_key {
            return Some(key.clone());
        }
        match &column.cell {
            ColumnCell::Text(binding) => {
                let binding = binding.clone();
                Some(Rc::new(move |a, b| binding(a).cmp(&binding(b))))
            }
            ColumnCell::Template(_) => None,
        }
    }

    /// `CanSortColumn`: the column opts in and has something to sort by.
    pub(crate) fn is_sortable(self: Handle<Self>, app: &App) -> bool {
        self.can_sort(app) && self.sort_comparer(app).is_some()
    }

    /// `GenerateElement`: a text column's `TextBlock`, vertically centred, ellipsized and padded by the density; a template column's content.
    pub(crate) fn generate_element(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        item: &T,
        style: TableViewCellStyle,
    ) -> WidgetRef {
        match &app.get(self).cell {
            ColumnCell::Text(binding) => {
                let text = binding(item);
                let r = ThemeResources::of(app, context);
                let [left, top, right, bottom] = style.padding;
                Align::new()
                    .alignment(AlignmentGeometry::CENTER_LEFT)
                    .child(
                        Padding::new(EdgeInsetsGeometry::from_ltrb(left, top, right, bottom))
                            .child(
                                Text::new(text)
                                    .style(
                                        waft_painting::TextStyle::new()
                                            .font_family(r.content_control_theme_font_family)
                                            .font_size(style.font_size)
                                            .font_weight(waft_embedder::FontWeight::NORMAL),
                                    )
                                    .max_lines(1)
                                    .soft_wrap(false)
                                    .overflow(TextOverflow::Ellipsis),
                            ),
                    )
                    .into_widget()
            }
            ColumnCell::Template(template) => {
                let template = template.clone();
                template(app, context, item)
            }
        }
    }

    /// `UpdateActualWidth`: a pixel width clamped to the bounds, else the default, until the table resolves it.
    fn width_changed(self: Handle<Self>, app: &mut App) {
        let column = app.get_mut(self);
        let pixels = if column.width.unit == GridUnitType::Pixel {
            column.width.value
        } else {
            DEFAULT_WIDTH
        };
        let lo = column.min_width;
        let hi = lo.max(column.max_width);
        column.actual_width = pixels.clamp(lo, hi);
        self.changed(app, ColumnChange::Width);
    }

    fn changed(self: Handle<Self>, app: &mut App, change: ColumnChange) {
        app.get_mut(self).change = change;
        self.notify_listeners(app);
    }
}

impl<T: 'static> ChangeNotifier for TableViewColumn<T> {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.notifier
    }
}

impl<T> fmt::Debug for TableViewColumn<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TableViewColumn")
            .field("header", &self.header)
            .field("width", &self.width)
            .field("actual_width", &self.actual_width)
            .field("sort_direction", &self.sort_direction)
            .finish_non_exhaustive()
    }
}
