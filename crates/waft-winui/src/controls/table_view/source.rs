//! `TableViewSource` (`controls/dev/TableView/TableViewSource.cpp`) over the shaping engine it fronts (`controls/dev/ShapedItemsSource/ShapedItemsSource.cpp`, `controls/dev/TabularShaping/ShapingPipeline.cpp`): the application's items, one filter predicate, and sort axes in the order they were declared, projected as the view a `TableView` shows.
//!
//! The view is rebuilt from the items whenever a verb changes the shape: the filter runs first, over the items in source order, then a stable sort over the active axes, the first declared primary and each later one breaking ties within the earlier. An item inserted, removed, replaced or updated in place is spliced into the view at its place (`ShapedItemsSource::ApplyIncrementalChange`, `TryApplyIncrementalSortedChange`), always exactly, for each row knows its item's source index: source order settles a tie and places a row among a filter's. Every operation notifies once, with the [`ViewChange`]s it made in order, which a `TableView` reads to move its selection as `SelectionNode::OnSourceListChanged` does; a verb that changes nothing (clearing an absent filter) does not notify. A later editor who changes the view without recording the change leaves a `TableView`'s selection on the wrong row.

use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;
use waft_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, HandleId, Ref};

/// XAML `SortDirection` (`TabularShaping/SortDirection.idl`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SortDirection {
    #[default]
    None,
    Ascending,
    Descending,
}

/// Keeps an item in the view: `TableViewPredicate`.
type ItemPredicate<T> = Rc<dyn Fn(&T) -> bool>;

/// Orders two items: a key selector's comparison, or a column's `CustomSortComparer`.
pub(crate) type SortComparer<T> = Rc<dyn Fn(&T, &T) -> Ordering>;

/// One change an operation made, in order: the items' indices shifting (`ItemInserted`, `ItemRemoved`), and the view's rows changing as its `IObservableVector` raises them (`RowInserted`, `RowRemoved`, `RowReplaced`, `Reset`), and a row moving with its item (`RowMoved`), which an `IObservableVector` raises as a removal and an insertion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ViewChange {
    /// An item was inserted at this index of the items; those at and after it moved up one.
    ItemInserted(usize),
    /// The item at this index of the items was removed; those after it moved down one.
    ItemRemoved(usize),
    /// A row was inserted at this index of the view.
    RowInserted(usize),
    /// The row at this index of the view was removed.
    RowRemoved(usize),
    /// The row at this index of the view shows a new item.
    RowReplaced(usize),
    /// The row at the first index of the view moved to the second, its item the same.
    RowMoved(usize, usize),
    /// The view was rebuilt.
    Reset,
}

/// `TabularShapingHelpers::SortAxis`: one active sort axis. `token` names an axis a `TableView` installs for a column; the axes an application declares through [`TableViewSource::sort`] have none.
struct SortAxis<T> {
    token: Option<HandleId>,
    compare: SortComparer<T>,
    direction: SortDirection,
}

/// XAML `TableViewSource`: the items a `TableView` shows and the shape it shows them in.
///
/// The application makes one with [`TableViewSource::from`], hands it to one `TableView`, and disposes it after the view.
pub struct TableViewSource<T> {
    notifier: ChangeNotifierData,
    /// Each shared, so a cell can be built from an item while the source is reachable through the `App`.
    items: Vec<Rc<T>>,
    /// `ShapingPipeline::m_filters`'s single untokenized axis: `Filter` replaces it.
    filter: Option<ItemPredicate<T>>,
    /// `ShapingPipeline::m_sorts`, in declaration order, which is precedence order.
    sorts: Vec<SortAxis<T>>,
    /// The projection: each row's index in `items`.
    view: Vec<usize>,
    /// The `TableView` showing the source, for `SetOwningTableView`'s single-owner check.
    owner: Option<HandleId>,
    /// What the last operation changed, for the listeners it notifies.
    changes: Vec<ViewChange>,
}

impl<T: 'static> TableViewSource<T> {
    /// `TableViewSource.From`: a source over `items`, unshaped, its view populated by the time it returns.
    pub fn from(app: &mut App, items: Vec<T>) -> Ref<Self> {
        let mut source = Self {
            notifier: ChangeNotifierData::new(),
            items: items.into_iter().map(Rc::new).collect(),
            filter: None,
            sorts: Vec::new(),
            view: Vec::new(),
            owner: None,
            changes: Vec::new(),
        };
        source.rebuild_view();
        app.create(source)
    }

    /// Releases the listeners; the source is not used after.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        ChangeNotifierData::dispose(app, self);
    }

    /// How many items there are.
    pub fn item_count(self: Handle<Self>, app: &App) -> usize {
        app.get(self).items.len()
    }

    /// The item at `index`, in source order.
    pub fn item(self: Handle<Self>, app: &App, index: usize) -> &T {
        &app.get(self).items[index]
    }

    /// Replaces the items, as the collection the source wraps raising a reset: the filter and the sorts stay, the view is rebuilt from the new items, and a `TableView` showing the source clears its selection (`SelectionNode::OnSourceListChanged`, `Reset`).
    pub fn set_items(self: Handle<Self>, app: &mut App, items: Vec<T>) {
        let source = app.get_mut(self);
        source.items = items.into_iter().map(Rc::new).collect();
        source.rebuild_view();
        source.changes = vec![ViewChange::Reset];
        self.notify_listeners(app);
    }

    /// Inserts `item` at `index` of the items, as the collection raising `Add`: its row goes to its place in the view, unless the filter leaves it out.
    pub fn insert(self: Handle<Self>, app: &mut App, index: usize, item: T) {
        app.get_mut(self).insert_item(index, Rc::new(item));
        self.notify_listeners(app);
    }

    /// Removes the item at `index`, as the collection raising `Remove`: its row, if shown, is removed.
    pub fn remove(self: Handle<Self>, app: &mut App, index: usize) {
        app.get_mut(self).remove_item(index);
        self.notify_listeners(app);
    }

    /// Replaces the item at `index` with `item`, as the collection raising `Replace`: without a sort its row shows the new item; with one the row is removed and the new item inserted at its place. A row replaced is no longer selected, as `SelectionNode` treats a replacement as a removal and an insertion; [`update`](Self::update) keeps it.
    pub fn replace(self: Handle<Self>, app: &mut App, index: usize, item: T) {
        app.get_mut(self).replace_item(index, Rc::new(item));
        self.notify_listeners(app);
    }

    /// Changes the item at `index` in place, as an item raising `PropertyChanged`: its row shows the new values and stays selected, and moves to its new place if a sort key changed, or leaves or joins the view if the filter's answer did.
    pub fn update(self: Handle<Self>, app: &mut App, index: usize, change: impl FnOnce(&mut T)) {
        let source = app.get_mut(self);
        let item = Rc::get_mut(&mut source.items[index])
            .expect("a cell holds an item only while it is built");
        change(item);
        source.update_item(index);
        self.notify_listeners(app);
    }

    /// The item at `index`, kept while a cell is built from it.
    pub(crate) fn shared_item(self: Handle<Self>, app: &App, index: usize) -> Rc<T> {
        app.get(self).items[index].clone()
    }

    /// What the operation now notifying changed, in order.
    pub(crate) fn changes(self: Handle<Self>, app: &App) -> &[ViewChange] {
        &app.get(self).changes
    }

    /// `View`: each row of the projection, as its index in the items.
    pub fn view(self: Handle<Self>, app: &App) -> &[usize] {
        &app.get(self).view
    }

    /// `Filter`: the only items shown are those `predicate` keeps, in source order before any sort. Each call replaces the previous predicate.
    pub fn filter(
        self: Handle<Self>,
        app: &mut App,
        predicate: impl Fn(&T) -> bool + 'static,
    ) -> Handle<Self> {
        app.get_mut(self).filter = Some(Rc::new(predicate));
        self.rebuild(app);
        self
    }

    /// `ClearFilter`: nothing at all when no filter is set, not even a rebuild.
    pub fn clear_filter(self: Handle<Self>, app: &mut App) -> Handle<Self> {
        if app.get_mut(self).filter.take().is_some() {
            self.rebuild(app);
        }
        self
    }

    /// `Sort`: adds a sort axis ordering the rows by `key` in `direction`, after the axes already declared, which it breaks ties within. `SortDirection::None` adds nothing.
    pub fn sort<K: Ord>(
        self: Handle<Self>,
        app: &mut App,
        key: impl Fn(&T) -> K + 'static,
        direction: SortDirection,
    ) -> Handle<Self> {
        let compare: SortComparer<T> = Rc::new(move |a, b| key(a).cmp(&key(b)));
        if app.get_mut(self).set_sort(None, None, compare, direction) {
            self.rebuild(app);
        }
        self
    }

    /// `ClearSort`: every axis, the ones a `TableView` installed included; nothing when there are none.
    pub fn clear_sort(self: Handle<Self>, app: &mut App) -> Handle<Self> {
        let source = app.get_mut(self);
        if !source.sorts.is_empty() {
            source.sorts.clear();
            self.rebuild(app);
        }
        self
    }

    /// `SortReplacing`: the axis a `TableView` keeps for its sorted column, named by `token`, replacing the one named by `previous` (a different column), or removed when `direction` is `None`.
    pub(crate) fn sort_replacing(
        self: Handle<Self>,
        app: &mut App,
        previous: Option<HandleId>,
        token: HandleId,
        compare: SortComparer<T>,
        direction: SortDirection,
    ) {
        if app
            .get_mut(self)
            .set_sort(previous, Some(token), compare, direction)
        {
            self.rebuild(app);
        }
    }

    /// `ClearSort(sortAxisToken)`: the axis named by `token`.
    pub(crate) fn clear_sort_axis(self: Handle<Self>, app: &mut App, token: HandleId) {
        let sorts = &mut app.get_mut(self).sorts;
        let before = sorts.len();
        sorts.retain(|axis| axis.token != Some(token));
        if sorts.len() != before {
            self.rebuild(app);
        }
    }

    /// `SetOwningTableView`: a source has one owner; a second `TableView` is refused while the first holds it. Answers whether `owner` now owns it.
    pub(crate) fn set_owner(self: Handle<Self>, app: &mut App, owner: Option<HandleId>) -> bool {
        let source = app.get_mut(self);
        if let (Some(existing), Some(new)) = (source.owner, owner)
            && existing != new
        {
            debug_assert!(
                false,
                "TableViewSource: attempted to bind an already-owned source to a second TableView"
            );
            return false;
        }
        source.owner = owner;
        true
    }

    fn rebuild(self: Handle<Self>, app: &mut App) {
        let source = app.get_mut(self);
        source.rebuild_view();
        source.changes = vec![ViewChange::Reset];
        self.notify_listeners(app);
    }
}

impl<T> TableViewSource<T> {
    /// `ShapingPipeline::SetSort`: answers whether the axes changed.
    fn set_sort(
        &mut self,
        previous: Option<HandleId>,
        token: Option<HandleId>,
        compare: SortComparer<T>,
        direction: SortDirection,
    ) -> bool {
        let mut changed = false;
        if previous.is_some() && previous != token {
            let before = self.sorts.len();
            self.sorts.retain(|axis| axis.token != previous);
            changed = self.sorts.len() != before;
        }
        if let Some(token) = token
            && let Some(position) = self.sorts.iter().position(|axis| axis.token == Some(token))
        {
            if direction == SortDirection::None {
                self.sorts.remove(position);
            } else {
                // Re-sorting an axis replaces it in place and keeps its precedence.
                let axis = &mut self.sorts[position];
                axis.compare = compare;
                axis.direction = direction;
            }
            return true;
        }
        if direction != SortDirection::None {
            self.sorts.push(SortAxis {
                token,
                compare,
                direction,
            });
            return true;
        }
        changed
    }

    /// `ShapedItemsSource::RebuildFlat`: filter in source order, then a stable sort over the axes.
    fn rebuild_view(&mut self) {
        let mut view: Vec<usize> = (0..self.items.len())
            .filter(|&index| self.passes_filter(index))
            .collect();
        if !self.sorts.is_empty() {
            view.sort_by(|&a, &b| self.compare(a, b));
        }
        self.view = view;
    }

    /// `ShapingPipeline::PassesFilter`.
    fn passes_filter(&self, index: usize) -> bool {
        self.filter
            .as_ref()
            .is_none_or(|predicate| predicate(&self.items[index]))
    }

    /// `CompareKeysToRow`: two items on the active axes, each in its direction.
    fn compare(&self, a: usize, b: usize) -> Ordering {
        for axis in &self.sorts {
            let order = (axis.compare)(&self.items[a], &self.items[b]);
            if order != Ordering::Equal {
                return match axis.direction {
                    SortDirection::Descending => order.reverse(),
                    _ => order,
                };
            }
        }
        Ordering::Equal
    }

    /// The view's order: the active axes, ties in source order as the stable sort leaves them; source order alone without a sort.
    fn view_order(&self, a: usize, b: usize) -> Ordering {
        self.compare(a, b).then(a.cmp(&b))
    }

    /// Where the item at `index` belongs among the view's rows: after every row the view's order puts before it. Exact because each row knows its source index, which settles a tie where WinUI's `SortedInsertPlacementFor` cannot.
    fn placement(&self, index: usize) -> usize {
        self.view
            .partition_point(|&row| self.view_order(row, index) == Ordering::Less)
    }

    /// Puts the item at `index` into the view if the filter keeps it.
    fn insert_row(&mut self, index: usize) {
        if self.passes_filter(index) {
            let row = self.placement(index);
            self.view.insert(row, index);
            self.changes.push(ViewChange::RowInserted(row));
        }
    }

    /// Takes the item at `index` out of the view, if it is shown.
    fn remove_row(&mut self, index: usize) -> Option<usize> {
        let row = self.view.iter().position(|&row| row == index)?;
        self.view.remove(row);
        self.changes.push(ViewChange::RowRemoved(row));
        Some(row)
    }

    /// `Add`: the items after `index` move up one, and the new item's row goes to its place.
    fn insert_item(&mut self, index: usize, item: Rc<T>) {
        self.items.insert(index, item);
        for row in &mut self.view {
            if *row >= index {
                *row += 1;
            }
        }
        self.changes = vec![ViewChange::ItemInserted(index)];
        self.insert_row(index);
    }

    /// `Remove`: the item's row goes, if shown, and the items after it move down one.
    fn remove_item(&mut self, index: usize) {
        self.changes = vec![ViewChange::ItemRemoved(index)];
        self.remove_row(index);
        self.items.remove(index);
        for row in &mut self.view {
            if *row > index {
                *row -= 1;
            }
        }
    }

    /// `Replace`: a new item in the old one's place. Without a sort its row keeps its place (`SetAt`); with one the old row is removed and the new item's inserted at its place, as `TryApplyIncrementalSortedChange` does.
    fn replace_item(&mut self, index: usize, item: Rc<T>) {
        self.items[index] = item;
        self.changes = Vec::new();
        let shown = self.view.iter().position(|&row| row == index);
        match shown {
            Some(row) if self.sorts.is_empty() && self.passes_filter(index) => {
                self.changes.push(ViewChange::RowReplaced(row));
            }
            _ => {
                self.remove_row(index);
                self.insert_row(index);
            }
        }
    }

    /// An item's values changed in place, `PropertyChanged` without a collection change: its row stays and keeps its selection, moving to its new place if a sort key changed and leaving or joining the view if the filter's answer did.
    fn update_item(&mut self, index: usize) {
        self.changes = Vec::new();
        let shown = self.view.iter().position(|&row| row == index);
        let kept = self.passes_filter(index);
        match (shown, kept) {
            (Some(from), true) => {
                self.view.remove(from);
                let to = self.placement(index);
                self.view.insert(to, index);
                if to != from {
                    self.changes.push(ViewChange::RowMoved(from, to));
                }
            }
            (Some(_), false) => {
                self.remove_row(index);
            }
            (None, true) => self.insert_row(index),
            (None, false) => {}
        }
    }
}

impl<T: 'static> ChangeNotifier for TableViewSource<T> {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.notifier
    }
}

impl<T> fmt::Debug for TableViewSource<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TableViewSource")
            .field("items", &self.items.len())
            .field("view", &self.view)
            .field("filter", &self.filter.is_some())
            .field("sorts", &self.sorts.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use waft_foundation::{AppCell, ListenableObject, Listener};

    fn people(app: &mut App) -> Ref<TableViewSource<(&'static str, u32)>> {
        TableViewSource::from(
            app,
            vec![("Mina", 30), ("Asha", 25), ("Diego", 30), ("Lea", 25)],
        )
    }

    fn names(app: &App, source: &Ref<TableViewSource<(&'static str, u32)>>) -> Vec<&'static str> {
        source
            .view(app)
            .iter()
            .map(|&i| source.item(app, i).0)
            .collect()
    }

    #[test]
    fn the_view_is_populated_when_from_returns() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let source = people(&mut app);
        assert_eq!(source.view(&app), [0, 1, 2, 3]);
    }

    #[test]
    fn the_filter_runs_before_the_sort_and_each_filter_replaces_the_last() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let source = people(&mut app);
        source
            .filter(&mut app, |p| p.1 == 30)
            .sort(&mut app, |p| p.0, SortDirection::Ascending);
        assert_eq!(names(&app, &source), ["Diego", "Mina"]);
        source.filter(&mut app, |p| p.1 == 25);
        assert_eq!(names(&app, &source), ["Asha", "Lea"]);
        source.clear_filter(&mut app);
        assert_eq!(names(&app, &source), ["Asha", "Diego", "Lea", "Mina"]);
    }

    // `ShapingPipeline::ActiveSortAxes`: the first declared axis is primary, a later one breaks
    // its ties, and the stable sort keeps source order for what is still tied.
    #[test]
    fn the_first_declared_axis_is_primary_and_ties_keep_source_order() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let source = people(&mut app);
        source.sort(&mut app, |p| p.1, SortDirection::Descending);
        assert_eq!(names(&app, &source), ["Mina", "Diego", "Asha", "Lea"]);
        source.sort(&mut app, |p| p.0, SortDirection::Ascending);
        assert_eq!(names(&app, &source), ["Diego", "Mina", "Asha", "Lea"]);
    }

    #[test]
    fn a_column_axis_is_replaced_in_place_and_removed_by_none() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let source = people(&mut app);
        let token = app.create(0u8);
        let column = token.handle().id();
        let by_age: SortComparer<(&str, u32)> = Rc::new(|a, b| a.1.cmp(&b.1));
        source.sort_replacing(
            &mut app,
            None,
            column,
            by_age.clone(),
            SortDirection::Ascending,
        );
        assert_eq!(names(&app, &source), ["Asha", "Lea", "Mina", "Diego"]);
        source.sort_replacing(
            &mut app,
            Some(column),
            column,
            by_age.clone(),
            SortDirection::Descending,
        );
        assert_eq!(names(&app, &source), ["Mina", "Diego", "Asha", "Lea"]);
        source.sort_replacing(&mut app, Some(column), column, by_age, SortDirection::None);
        assert_eq!(source.view(&app), [0, 1, 2, 3]);
    }

    #[test]
    fn new_items_keep_the_shape_and_clearing_nothing_does_not_notify() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let source = people(&mut app);
        let heard = Rc::new(Cell::new(0));
        let counter = heard.clone();
        source.add_listener(
            &mut app,
            Listener::new(move |_| counter.set(counter.get() + 1)),
        );
        source.clear_filter(&mut app);
        source.clear_sort(&mut app);
        assert_eq!(
            heard.get(),
            0,
            "ClearFilter and ClearSort with no stage do nothing"
        );
        source.sort(&mut app, |p| p.0, SortDirection::Descending);
        source.set_items(&mut app, vec![("Bo", 1), ("Ann", 2)]);
        assert_eq!(names(&app, &source), ["Bo", "Ann"]);
        assert_eq!(heard.get(), 2);
    }

    fn changes(app: &App, source: &Ref<TableViewSource<(&'static str, u32)>>) -> Vec<ViewChange> {
        source.changes(app).to_vec()
    }

    // `ApplyIncrementalChange` on an unshaped projection: the row is spliced at the item's index.
    #[test]
    fn unshaped_changes_splice_the_row_at_the_items_index() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let source = people(&mut app);
        source.insert(&mut app, 1, ("Bo", 50));
        assert_eq!(names(&app, &source), ["Mina", "Bo", "Asha", "Diego", "Lea"]);
        assert_eq!(
            changes(&app, &source),
            [ViewChange::ItemInserted(1), ViewChange::RowInserted(1)]
        );
        source.replace(&mut app, 0, ("Ana", 20));
        assert_eq!(changes(&app, &source), [ViewChange::RowReplaced(0)]);
        source.remove(&mut app, 2);
        assert_eq!(names(&app, &source), ["Ana", "Bo", "Diego", "Lea"]);
        assert_eq!(
            changes(&app, &source),
            [ViewChange::ItemRemoved(2), ViewChange::RowRemoved(2)]
        );
        assert_eq!(source.view(&app), [0, 1, 2, 3]);
    }

    // `TryApplyIncrementalSortedChange`: an item goes in at its sorted place and one the filter
    // leaves out changes no row; where WinUI rebuilds for an item tying with a row, source order
    // places it.
    #[test]
    fn sorted_changes_place_the_item_and_source_order_settles_a_tie() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let source = people(&mut app);
        source
            .filter(&mut app, |p| p.1 < 40)
            .sort(&mut app, |p| p.1, SortDirection::Ascending);
        assert_eq!(names(&app, &source), ["Asha", "Lea", "Mina", "Diego"]);
        source.insert(&mut app, 0, ("Bo", 27));
        assert_eq!(names(&app, &source), ["Asha", "Lea", "Bo", "Mina", "Diego"]);
        assert_eq!(
            changes(&app, &source),
            [ViewChange::ItemInserted(0), ViewChange::RowInserted(2)]
        );
        source.insert(&mut app, 0, ("Old", 90));
        assert_eq!(
            changes(&app, &source),
            [ViewChange::ItemInserted(0)],
            "filtered out"
        );
        let count = source.item_count(&app);
        source.insert(&mut app, count, ("Zed", 25));
        assert_eq!(
            names(&app, &source),
            ["Asha", "Lea", "Zed", "Bo", "Mina", "Diego"]
        );
        assert_eq!(
            changes(&app, &source)[1],
            ViewChange::RowInserted(2),
            "a tie appended at the end"
        );
        source.insert(&mut app, 0, ("Ann", 25));
        assert_eq!(
            changes(&app, &source),
            [ViewChange::ItemInserted(0), ViewChange::RowInserted(0)]
        );
        assert_eq!(
            names(&app, &source),
            ["Ann", "Asha", "Lea", "Zed", "Bo", "Mina", "Diego"]
        );
        // Removing a filtered-out item moves the items after it and no row.
        source.remove(&mut app, 1);
        assert_eq!(changes(&app, &source), [ViewChange::ItemRemoved(1)]);
        assert_eq!(
            names(&app, &source),
            ["Ann", "Asha", "Lea", "Zed", "Bo", "Mina", "Diego"]
        );
    }

    #[test]
    fn a_sorted_replace_moves_the_row_and_a_filter_alone_places_it_in_source_order() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let source = people(&mut app);
        source.sort(&mut app, |p| p.0, SortDirection::Ascending);
        source.replace(&mut app, 1, ("Zoe", 25));
        assert_eq!(names(&app, &source), ["Diego", "Lea", "Mina", "Zoe"]);
        assert_eq!(
            changes(&app, &source),
            [ViewChange::RowRemoved(0), ViewChange::RowInserted(3)]
        );
        source.clear_sort(&mut app).filter(&mut app, |p| p.1 == 25);
        source.remove(&mut app, 3);
        assert_eq!(
            changes(&app, &source),
            [ViewChange::ItemRemoved(3), ViewChange::RowRemoved(1)]
        );
        assert_eq!(names(&app, &source), ["Zoe"]);
        source.insert(&mut app, 0, ("Ann", 25));
        assert_eq!(
            changes(&app, &source),
            [ViewChange::ItemInserted(0), ViewChange::RowInserted(0)]
        );
        source.insert(&mut app, 3, ("Kai", 25));
        assert_eq!(
            changes(&app, &source),
            [ViewChange::ItemInserted(3), ViewChange::RowInserted(2)]
        );
        assert_eq!(names(&app, &source), ["Ann", "Zoe", "Kai"]);
    }

    // `update` is `PropertyChanged`: the row stays where it is unless a sort key moved it, and
    // leaves or joins the view as the filter's answer changes.
    #[test]
    fn an_update_moves_a_row_only_when_its_place_changes() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let source = people(&mut app);
        source.sort(&mut app, |p| p.1, SortDirection::Ascending);
        assert_eq!(names(&app, &source), ["Asha", "Lea", "Mina", "Diego"]);
        source.update(&mut app, 0, |p| p.0 = "Mia");
        assert_eq!(changes(&app, &source), []);
        source.update(&mut app, 0, |p| p.1 = 20);
        assert_eq!(names(&app, &source), ["Mia", "Asha", "Lea", "Diego"]);
        assert_eq!(changes(&app, &source), [ViewChange::RowMoved(2, 0)]);
        source.filter(&mut app, |p| p.1 < 30);
        source.update(&mut app, 2, |p| p.1 = 10);
        assert_eq!(changes(&app, &source), [ViewChange::RowInserted(0)]);
        source.update(&mut app, 2, |p| p.1 = 99);
        assert_eq!(changes(&app, &source), [ViewChange::RowRemoved(0)]);
        assert_eq!(names(&app, &source), ["Mia", "Asha", "Lea"]);
    }
}
