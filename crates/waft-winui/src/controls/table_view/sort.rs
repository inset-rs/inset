//! `TableView`'s single-column sort (`controls/dev/TableView/TableView_Sort.cpp`): `SortByColumn`, `ToggleSortDirection` and `ClearSort`, the cancelable `Sorting` before and `Sorted` after, the source axis the table installs for its sorted column, and the selection kept on its item across the reshape; and `TableViewSort`, which carries those requests from code to the table.
//!
//! The sort state lives on the columns, as `TableViewColumn::sort_direction`, and the table performs every sort, so sorting from a header click needs no `TableViewSort`. A later editor who keeps sort state in `TableViewSort` makes two answers to what is sorted.

use super::column::TableViewColumn;
use super::source::SortDirection;
use super::*;
use std::fmt;
use std::rc::Rc;
use waft_foundation::{App, Handle, HandleId, Ref};

/// A sort the application asks of a table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SortRequest {
    /// `SortByColumn`.
    ByColumn(HandleId, SortDirection),
    /// `ToggleSortDirection`.
    Toggle(HandleId),
    /// `ClearSort`.
    Clear,
}

/// The table a sort is given to: it performs a request and answers whether the sort state changed.
pub(crate) type SortPerformer = Rc<dyn Fn(&mut App, SortRequest) -> bool>;

/// Sorting a `TableView` from code.
///
/// The application makes one with [`TableViewSort::new`], and gives it to the table with `TableView::sort`. Until a mounted table holds it, every request answers false.
pub struct TableViewSort {
    performer: Option<(HandleId, SortPerformer)>,
}

impl TableViewSort {
    /// Creates a sort no table holds yet.
    pub fn new(app: &mut App) -> Ref<Self> {
        app.create(Self { performer: None })
    }

    /// `SortByColumn`: clears any other column's sort, then sorts `column` in `direction`; `None` clears it. Raises `Sorting`, which can cancel, then `Sorted`. Answers whether the sort state changed.
    pub fn sort_by_column<T: 'static>(
        self: Handle<Self>,
        app: &mut App,
        column: &Ref<TableViewColumn<T>>,
        direction: SortDirection,
    ) -> bool {
        self.perform(app, SortRequest::ByColumn(column.handle().id(), direction))
    }

    /// `ToggleSortDirection`: the column's next direction in its `SortCycle`, any other column's sort cleared first.
    pub fn toggle_sort_direction<T: 'static>(
        self: Handle<Self>,
        app: &mut App,
        column: &Ref<TableViewColumn<T>>,
    ) -> bool {
        self.perform(app, SortRequest::Toggle(column.handle().id()))
    }

    /// `ClearSort`: every column's sort, in one change. Answers whether anything was cleared.
    pub fn clear_sort(self: Handle<Self>, app: &mut App) -> bool {
        self.perform(app, SortRequest::Clear)
    }

    /// The table `owner` holds the sort and performs its requests.
    pub(crate) fn bind(
        self: Handle<Self>,
        app: &mut App,
        owner: HandleId,
        performer: SortPerformer,
    ) {
        app.get_mut(self).performer = Some((owner, performer));
    }

    /// The table `owner` lets the sort go.
    pub(crate) fn unbind(self: Handle<Self>, app: &mut App, owner: HandleId) {
        let sort = app.get_mut(self);
        if sort
            .performer
            .as_ref()
            .is_some_and(|(held, _)| *held == owner)
        {
            sort.performer = None;
        }
    }

    fn perform(self: Handle<Self>, app: &mut App, request: SortRequest) -> bool {
        match app.get(self).performer.clone() {
            Some((_, performer)) => performer(app, request),
            None => false,
        }
    }
}

impl fmt::Debug for TableViewSort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TableViewSort")
            .field("bound", &self.performer.is_some())
            .finish()
    }
}

// MARK: The table's sorting (`TableView_Sort.cpp`)

impl<T: 'static> TableViewState<T> {
    pub(super) fn sync_sort(self: Handle<Self>, app: &mut App) {
        let id = self.id_of();
        let sort = self.widget(app).sort.clone();
        if app.get(self).sort.handle() == sort.handle() {
            return;
        }
        if let Some(old) = app.get_mut(self).sort.take() {
            old.unbind(app, id);
        }
        if let Some(sort) = &sort {
            sort.bind(
                app,
                id,
                Rc::new(move |app, request| self.alive(app) && self.perform_sort(app, request)),
            );
        }
        app.get_mut(self).sort = sort;
    }

    pub(super) fn column_of(
        self: Handle<Self>,
        app: &App,
        id: HandleId,
    ) -> Option<Handle<TableViewColumn<T>>> {
        self.owned_columns(app)
            .into_iter()
            .find(|column| column.id() == id)
    }

    pub(super) fn perform_sort(self: Handle<Self>, app: &mut App, request: SortRequest) -> bool {
        match request {
            SortRequest::ByColumn(id, direction) => self
                .column_of(app, id)
                .is_some_and(|column| self.sort_by_column(app, column, direction)),
            SortRequest::Toggle(id) => self
                .column_of(app, id)
                .is_some_and(|column| self.toggle_sort_direction(app, column)),
            SortRequest::Clear => self.clear_sort(app),
        }
    }

    /// `IsSortRequestStillValid`: the column is still one of ours and can still sort; a `Sorting` handler may have changed either.
    pub(super) fn is_sort_request_still_valid(
        self: Handle<Self>,
        app: &App,
        column: Handle<TableViewColumn<T>>,
    ) -> bool {
        app.contains(column) && column.is_sortable(app) && self.owned_columns(app).contains(&column)
    }

    /// `SortByColumn`.
    pub(super) fn sort_by_column(
        self: Handle<Self>,
        app: &mut App,
        column: Handle<TableViewColumn<T>>,
        direction: SortDirection,
    ) -> bool {
        if !column.is_sortable(app) {
            return false;
        }
        let sorted = app.get(self).sorted_column;
        if direction == SortDirection::None && sorted.is_none() {
            return false;
        }
        if sorted == Some(column) && column.sort_direction(app) == direction {
            return false;
        }
        self.apply_sort(app, column, direction)
    }

    /// `ToggleSortDirection`: the column's `SortCycle` names the direction an unsorted column opens in, and whether a third click returns to source order.
    pub(super) fn toggle_sort_direction(
        self: Handle<Self>,
        app: &mut App,
        column: Handle<TableViewColumn<T>>,
    ) -> bool {
        if !column.is_sortable(app) {
            return false;
        }
        let cycle = column.sort_cycle(app);
        let opens_descending = matches!(
            cycle,
            TableViewSortCycle::DescendingAscending | TableViewSortCycle::DescendingAscendingNone
        );
        let cycles_to_unsorted = matches!(
            cycle,
            TableViewSortCycle::AscendingDescendingNone
                | TableViewSortCycle::DescendingAscendingNone
        );
        let (opening, second) = if opens_descending {
            (SortDirection::Descending, SortDirection::Ascending)
        } else {
            (SortDirection::Ascending, SortDirection::Descending)
        };
        let current = column.sort_direction(app);
        let next = if current == opening {
            second
        } else if current == second {
            if cycles_to_unsorted {
                SortDirection::None
            } else {
                opening
            }
        } else {
            opening
        };
        self.apply_sort(app, column, next)
    }

    /// The shared tail of `SortByColumn` and `ToggleSortDirection`: `Sorting`, which can cancel, then the state, the reshape and `Sorted`.
    pub(super) fn apply_sort(
        self: Handle<Self>,
        app: &mut App,
        column: Handle<TableViewColumn<T>>,
        direction: SortDirection,
    ) -> bool {
        if !self.is_sort_request_still_valid(app, column) {
            return false;
        }
        if self.raise_sorting_and_check_canceled(app, Some(column), direction) {
            return false;
        }
        if !self.is_sort_request_still_valid(app, column) {
            return false;
        }
        self.apply_single_column_sort_state(app, column, direction);
        self.recompute_sort_and_raise(app, Some(column));
        true
    }

    /// `ClearSort`.
    pub(super) fn clear_sort(self: Handle<Self>, app: &mut App) -> bool {
        if app.get(self).sorted_column.is_none() {
            return false;
        }
        if self.raise_sorting_and_check_canceled(app, None, SortDirection::None) {
            return false;
        }
        let Some(sorted) = app.get_mut(self).sorted_column.take() else {
            return false;
        };
        if app.contains(sorted) {
            sorted.set_sort_state(app, SortDirection::None);
        }
        self.recompute_sort_and_raise(app, None);
        true
    }

    /// `ResetSortStateForNewItemsSource`: silent, for the rows the sort ordered are gone.
    pub(super) fn reset_sort_state_for_new_items_source(self: Handle<Self>, app: &mut App) {
        if let Some(sorted) = app.get_mut(self).sorted_column.take()
            && app.contains(sorted)
        {
            sorted.set_sort_state(app, SortDirection::None);
        }
        app.get_mut(self).source_sort_token = None;
    }

    /// `ApplySingleColumnSortState`: every other column's sort is cleared first.
    pub(super) fn apply_single_column_sort_state(
        self: Handle<Self>,
        app: &mut App,
        column: Handle<TableViewColumn<T>>,
        direction: SortDirection,
    ) {
        if let Some(sorted) = app.get_mut(self).sorted_column.take()
            && sorted != column
            && app.contains(sorted)
        {
            sorted.set_sort_state(app, SortDirection::None);
        }
        column.set_sort_state(app, direction);
        if direction != SortDirection::None {
            app.get_mut(self).sorted_column = Some(column);
        }
    }

    /// `RaiseSortingAndCheckCanceled`.
    pub(super) fn raise_sorting_and_check_canceled(
        self: Handle<Self>,
        app: &mut App,
        column: Option<Handle<TableViewColumn<T>>>,
        direction: SortDirection,
    ) -> bool {
        let Some(sorting) = self.widget(app).sorting.clone() else {
            return false;
        };
        let args = TableViewSortingEventArgs {
            column,
            direction,
            cancel: Cell::new(false),
        };
        sorting(app, &args);
        args.cancel()
    }

    /// `RecomputeSortDPsAndRaiseInternal`: reshapes the rows, keeps the selection on its item, and raises `Sorted`.
    pub(super) fn recompute_sort_and_raise(
        self: Handle<Self>,
        app: &mut App,
        trigger: Option<Handle<TableViewColumn<T>>>,
    ) {
        if !self.alive(app) {
            return;
        }
        // A clear queued earlier and overtaken by a newer sort must not tear that sort down.
        if trigger.is_none() && app.get(self).sorted_column.is_some() {
            return;
        }
        let direction = trigger
            .filter(|column| app.contains(*column))
            .map_or(SortDirection::None, |column| column.sort_direction(app));
        let selection = self.selection_handle(app);
        let preserved = selection.and_then(|selection| selection.selected_item(app));
        let reshaped = self.sync_table_view_source_sort(app, trigger, direction);
        if reshaped && let (Some(selection), Some(item)) = (selection, preserved) {
            // Selection is by row, and a re-sort moves every row: it follows the item instead.
            let row = self
                .source_handle(app)
                .view(app)
                .iter()
                .position(|&i| i == item);
            match row {
                Some(row) => selection.select(app, row),
                None => selection.deselect_all(app),
            }
        }
        if let Some(sorted) = self.widget(app).sorted.clone() {
            let args = TableViewSortedEventArgs {
                column: trigger,
                direction,
            };
            sorted(app, &args);
        }
    }

    /// `SyncTableViewSourceSort`: installs, replaces or removes the source axis for the sorted column. Answers whether it reshaped.
    pub(super) fn sync_table_view_source_sort(
        self: Handle<Self>,
        app: &mut App,
        trigger: Option<Handle<TableViewColumn<T>>>,
        direction: SortDirection,
    ) -> bool {
        let source = self.source_handle(app);
        let previous = app.get(self).source_sort_token;
        match trigger {
            Some(column) => {
                let Some(compare) = column.sort_comparer(app) else {
                    return false;
                };
                let token = column.id();
                if direction == SortDirection::None {
                    source.clear_sort_axis(app, token);
                    app.get_mut(self).source_sort_token = None;
                } else {
                    source.sort_replacing(app, previous, token, compare, direction);
                    app.get_mut(self).source_sort_token = Some(token);
                }
            }
            None => {
                let Some(token) = previous else {
                    return false;
                };
                source.clear_sort_axis(app, token);
                app.get_mut(self).source_sort_token = None;
            }
        }
        true
    }
}
