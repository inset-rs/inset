//! `TableViewSource` (`controls/dev/TableView/TableViewSource.cpp`) over the shaping engine it fronts (`controls/dev/ShapedItemsSource/ShapedItemsSource.cpp`, `controls/dev/TabularShaping/ShapingPipeline.cpp`): the application's items, one filter predicate, and sort axes in the order they were declared, projected as the view a `TableView` shows.
//!
//! The view is rebuilt from the items whenever a verb changes the shape: the filter runs first, over the items in source order, then a stable sort over the active axes, the first declared primary and each later one breaking ties within the earlier. A rebuild notifies once; a verb that changes nothing (clearing an absent filter) does not. A later editor who notifies without rebuilding, or rebuilds without notifying, leaves a `TableView` showing rows the view no longer holds.

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
    /// Shared, so a cell can be built from an item while the source is reachable through the `App`.
    items: Rc<Vec<T>>,
    /// `ShapingPipeline::m_filters`'s single untokenized axis: `Filter` replaces it.
    filter: Option<ItemPredicate<T>>,
    /// `ShapingPipeline::m_sorts`, in declaration order, which is precedence order.
    sorts: Vec<SortAxis<T>>,
    /// The projection: each row's index in `items`.
    view: Vec<usize>,
    /// The `TableView` showing the source, for `SetOwningTableView`'s single-owner check.
    owner: Option<HandleId>,
}

impl<T: 'static> TableViewSource<T> {
    /// `TableViewSource.From`: a source over `items`, unshaped, its view populated by the time it returns.
    pub fn from(app: &mut App, items: Vec<T>) -> Ref<Self> {
        let mut source = Self {
            notifier: ChangeNotifierData::new(),
            items: Rc::new(items),
            filter: None,
            sorts: Vec::new(),
            view: Vec::new(),
            owner: None,
        };
        source.rebuild_view();
        app.create(source)
    }

    /// Releases the listeners; the source is not used after.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        ChangeNotifierData::dispose(app, self);
    }

    /// The items, in source order.
    pub fn items(self: Handle<Self>, app: &App) -> &[T] {
        &app.get(self).items
    }

    /// Replaces the items, as the collection the source wraps raising a reset: the filter and the sorts stay, the view is rebuilt from the new items, and a `TableView` showing the source clears its selection (`SelectionNode::OnSourceListChanged`, `Reset`).
    pub fn set_items(self: Handle<Self>, app: &mut App, items: Vec<T>) {
        app.get_mut(self).items = Rc::new(items);
        self.rebuild(app);
    }

    /// The items, kept while a cell is built from one.
    pub(crate) fn shared_items(self: Handle<Self>, app: &App) -> Rc<Vec<T>> {
        app.get(self).items.clone()
    }

    /// `View`: each row of the projection, as its index in [`items`](Self::items).
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
        app.get_mut(self).rebuild_view();
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
        let items = &self.items;
        let mut view: Vec<usize> = match &self.filter {
            Some(predicate) => (0..items.len()).filter(|&i| predicate(&items[i])).collect(),
            None => (0..items.len()).collect(),
        };
        if !self.sorts.is_empty() {
            let sorts = &self.sorts;
            view.sort_by(|&a, &b| {
                for axis in sorts {
                    let order = (axis.compare)(&items[a], &items[b]);
                    if order != Ordering::Equal {
                        return match axis.direction {
                            SortDirection::Descending => order.reverse(),
                            _ => order,
                        };
                    }
                }
                Ordering::Equal
            });
        }
        self.view = view;
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
            .map(|&i| source.items(app)[i].0)
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
}
