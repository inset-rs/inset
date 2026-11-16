//! `TableView`'s row selection (`controls/dev/TableView/TableView_Selection.cpp`): the table's gestures and `SelectionChanged`, and the selection held where the application reaches it: `Select`, `Deselect`, `IsSelected`, `DeselectAll`, `SelectedIndex` and `SelectedItem` of `TableView.idl`, over the single-select `SelectionModel` the control keeps.
//!
//! The selection is resolved only while a `TableView` holds it: the table gives it the number of rows, whether its mode allows selection, and how a row maps to an item. Every change of the selected row notifies; the table raises `SelectionChanged` from the item reported last, and each row listens for its own index. A later editor who changes the model without notifying leaves rows showing the old selection.

use super::super::items_view::selection_model::SelectionModel;
use super::source::ViewChange;
use super::*;
use std::fmt;
use std::rc::Rc;
use waft_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, HandleId, Ref};

/// The index in the source's items of a row of the view.
pub(crate) type ItemOfRow = Rc<dyn Fn(&App, usize) -> Option<usize>>;

/// What the table holding a selection tells it.
struct SelectionBinding {
    /// The table.
    owner: HandleId,
    /// `CanSelectRows`: the selection mode is not `None`.
    can_select: bool,
    /// The index in the source's items of a row of the view.
    item_of_row: ItemOfRow,
}

/// The selected row of a `TableView`: at most one, as WinUI's v1 has it.
///
/// The application makes one with [`TableViewSelection::new`], gives it to the table with `TableView::selection`, and disposes it after the table. A table given none selects nothing.
pub struct TableViewSelection {
    notifier: ChangeNotifierData,
    model: SelectionModel,
    binding: Option<SelectionBinding>,
    /// `m_lastRaisedSelectedItem`: the item `SelectionChanged` last reported.
    last_raised_item: Option<usize>,
}

impl TableViewSelection {
    /// Creates a selection with nothing selected.
    pub fn new(app: &mut App) -> Ref<Self> {
        let mut model = SelectionModel::default();
        model.set_single_select(true);
        app.create(Self {
            notifier: ChangeNotifierData::new(),
            model,
            binding: None,
            last_raised_item: None,
        })
    }

    /// Releases the listeners; the selection is not used after.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        ChangeNotifierData::dispose(app, self);
    }

    /// `Select`: the row `index` of the view becomes the selection. Nothing while no table holds the selection, while its mode is `None`, or for an index past the rows: an existing selection is never cleared to signal a refusal.
    pub fn select(self: Handle<Self>, app: &mut App, index: usize) {
        if !self.can_select(app) || index >= self.row_count(app) {
            return;
        }
        self.apply(app, Some(index));
    }

    /// `Deselect`: clears the selection only when `index` is the selected row, so a stale call cannot clear a newer one.
    pub fn deselect(self: Handle<Self>, app: &mut App, index: usize) {
        if self.is_selected(app, index) {
            self.apply(app, None);
        }
    }

    /// `IsSelected`.
    pub fn is_selected(self: Handle<Self>, app: &App, index: usize) -> bool {
        index < self.row_count(app) && app.get(self).model.is_selected(index)
    }

    /// `DeselectAll`, whatever the selection mode.
    pub fn deselect_all(self: Handle<Self>, app: &mut App) {
        self.apply(app, None);
    }

    /// `SelectedIndex`: the selected row of the view.
    pub fn selected_index(self: Handle<Self>, app: &App) -> Option<usize> {
        app.get(self)
            .model
            .selected_index()
            .filter(|index| *index < self.row_count(app))
    }

    /// `SelectedItem`: the selected row's index in the source's items.
    pub fn selected_item(self: Handle<Self>, app: &App) -> Option<usize> {
        let index = self.selected_index(app)?;
        let binding = app.get(self).binding.as_ref()?;
        (binding.item_of_row)(app, index)
    }
}

// MARK: The holding table's view of the selection

impl TableViewSelection {
    /// The table `owner` holds the selection; a selection new to it starts over on its rows, as `SelectionModel::Source` clears.
    pub(crate) fn bind(
        self: Handle<Self>,
        app: &mut App,
        owner: HandleId,
        can_select: bool,
        row_count: usize,
        item_of_row: ItemOfRow,
    ) {
        let new_owner = app
            .get(self)
            .binding
            .as_ref()
            .is_none_or(|binding| binding.owner != owner);
        app.get_mut(self).binding = Some(SelectionBinding {
            owner,
            can_select,
            item_of_row,
        });
        if new_owner {
            self.set_source(app, row_count);
        }
        if !can_select {
            // `OnSelectionModePropertyChanged`: turning selection off clears it.
            self.apply(app, None);
        }
    }

    /// The table `owner` lets the selection go.
    pub(crate) fn unbind(self: Handle<Self>, app: &mut App, owner: HandleId) {
        let selection = app.get_mut(self);
        if selection
            .binding
            .as_ref()
            .is_some_and(|binding| binding.owner == owner)
        {
            selection.binding = None;
        }
    }

    /// `UpdateSelectionModelSource`: the rows were reshaped, which clears the selection (`SelectionNode::OnSourceListChanged`, `Reset`).
    pub(crate) fn set_source(self: Handle<Self>, app: &mut App, row_count: usize) {
        if app.get_mut(self).model.set_source(row_count) {
            self.notify_listeners(app);
        }
    }

    /// `SelectionNode::OnSourceListChanged` for each change the source made, in order, notifying once: the item `SelectionChanged` last reported keeps naming the same item as the items move, a row inserted or removed moves the selection with its row and a removed selected row is deselected, a row moved keeps its selection, and a rebuilt view starts over. The view has `row_count` rows after the changes.
    pub(crate) fn source_changed(
        self: Handle<Self>,
        app: &mut App,
        changes: &[ViewChange],
        row_count: usize,
    ) {
        let mut invalidated = false;
        for &change in changes {
            let selection = app.get_mut(self);
            invalidated |= match change {
                ViewChange::ItemInserted(index) => {
                    if let Some(item) = &mut selection.last_raised_item
                        && *item >= index
                    {
                        *item += 1;
                    }
                    false
                }
                ViewChange::ItemRemoved(index) => {
                    // The removed item keeps its old index, to be reported as removed.
                    if let Some(item) = &mut selection.last_raised_item
                        && *item > index
                    {
                        *item -= 1;
                    }
                    false
                }
                ViewChange::RowInserted(row) => selection.model.on_items_added(row, 1),
                ViewChange::RowRemoved(row) => selection.model.on_items_removed(row, 1),
                ViewChange::RowMoved(from, to) => {
                    // The row's item is the same, so the selection moves with it.
                    let was_selected = selection.model.is_selected(from);
                    selection.model.on_items_removed(from, 1);
                    selection.model.on_items_added(to, 1);
                    if was_selected {
                        selection.model.select(to);
                    }
                    true
                }
                ViewChange::RowReplaced(row) => {
                    let removed = selection.model.on_items_removed(row, 1);
                    selection.model.on_items_added(row, 1) | removed
                }
                ViewChange::Reset => selection.model.set_source(row_count),
            };
        }
        debug_assert_eq!(app.get(self).model.item_count(), row_count);
        // Once, after the last change: a listener reads rows of the view the changes ended with.
        if invalidated {
            self.notify_listeners(app);
        }
    }

    /// `SelectRowIndexFromInteraction`: a pointer or keyboard gesture selects the row; with `toggle` (Control) a selected row is deselected.
    pub(crate) fn select_from_interaction(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        toggle: bool,
    ) {
        if !self.can_select(app) {
            return;
        }
        if toggle && self.is_selected(app, index) {
            self.apply(app, None);
        } else {
            self.apply(app, Some(index));
        }
    }

    /// The item `SelectionChanged` should now report as removed and added, if it changed since the last report (`RaiseSelectionChanged`).
    pub(crate) fn take_change(
        self: Handle<Self>,
        app: &mut App,
    ) -> Option<(Option<usize>, Option<usize>)> {
        let added = self.selected_item(app);
        let removed = app.get(self).last_raised_item;
        if removed == added {
            return None;
        }
        app.get_mut(self).last_raised_item = added;
        Some((removed, added))
    }

    fn can_select(self: Handle<Self>, app: &App) -> bool {
        app.get(self)
            .binding
            .as_ref()
            .is_some_and(|binding| binding.can_select)
    }

    fn row_count(self: Handle<Self>, app: &App) -> usize {
        app.get(self).model.item_count()
    }

    /// `ApplySelection`: the single writer.
    fn apply(self: Handle<Self>, app: &mut App, index: Option<usize>) {
        let index = index.filter(|index| self.can_select(app) && *index < self.row_count(app));
        let model = &mut app.get_mut(self).model;
        let changed = match index {
            Some(index) => model.select(index),
            None => model.selected_index().is_some() && model.clear_selection(),
        };
        if changed {
            self.notify_listeners(app);
        }
    }
}

impl ChangeNotifier for TableViewSelection {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.notifier
    }
}

impl fmt::Debug for TableViewSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TableViewSelection")
            .field("selected_index", &self.model.selected_index())
            .field("bound", &self.binding.is_some())
            .finish_non_exhaustive()
    }
}

// MARK: The table's selection

impl<T: 'static> TableViewState<T> {
    /// Gives the selection the table's rows and mode, and listens to it.
    pub(super) fn sync_selection(self: Handle<Self>, app: &mut App) {
        let id = self.id_of();
        let selection = self.widget(app).selection.clone();
        let held = app
            .get(self)
            .selection
            .as_ref()
            .map(|(held, _)| held.handle());
        if held != selection.as_ref().map(|selection| selection.handle()) {
            if let Some((old, listener)) = app.get_mut(self).selection.take() {
                old.remove_listener(app, &listener);
                old.unbind(app, id);
            }
            if let Some(selection) = &selection {
                let listener = Listener::new(move |app| self.selection_heard(app));
                selection.add_listener(app, listener.clone());
                app.get_mut(self).selection = Some((selection.clone(), listener));
            }
        }
        if let Some(selection) = selection {
            let can_select = self.widget(app).selection_mode != TableViewSelectionMode::None;
            let count = self.row_count(app);
            let item_of_row: selection::ItemOfRow = Rc::new(move |app, row| {
                if !app.contains(self) {
                    return None;
                }
                self.source_handle(app).view(app).get(row).copied()
            });
            selection.bind(app, id, can_select, count, item_of_row);
        }
    }

    /// `OnSelectionModelSelectionChanged`'s `RaiseSelectionChanged`: one event for the change of the selected item.
    pub(super) fn selection_heard(self: Handle<Self>, app: &mut App) {
        if !self.alive(app) {
            return;
        }
        let Some(selection) = self.selection_handle(app) else {
            return;
        };
        let Some((removed, added)) = selection.take_change(app) else {
            return;
        };
        if let Some(changed) = self.widget(app).selection_changed.clone() {
            let args = TableViewSelectionChangedEventArgs {
                removed_items: removed.into_iter().collect(),
                added_items: added.into_iter().collect(),
            };
            changed(app, &args);
        }
    }

    pub(super) fn can_select_rows(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).selection.is_some()
            && self.widget(app).selection_mode != TableViewSelectionMode::None
    }

    /// `SelectRowIndexFromInteraction`: Control toggles.
    pub(super) fn select_row_from_interaction(self: Handle<Self>, app: &mut App, index: usize) {
        let control = HardwareKeyboard::instance(app).is_control_pressed(app);
        if let Some(selection) = self.selection_handle(app) {
            selection.select_from_interaction(app, index, control);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use waft_foundation::AppCell;

    fn bound(app: &mut App, rows: usize, can_select: bool) -> Ref<TableViewSelection> {
        let selection = TableViewSelection::new(app);
        let owner = app.create(0u8);
        selection.bind(
            app,
            owner.handle().id(),
            can_select,
            rows,
            Rc::new(|_, row| Some(row + 10)),
        );
        selection
    }

    // `TableView::Select` is a no-op before a source exists, while the mode is None, and past the
    // rows; it never clears an existing selection to signal a refusal.
    #[test]
    fn a_select_that_cannot_be_resolved_does_nothing() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let unbound = TableViewSelection::new(&mut app);
        unbound.select(&mut app, 0);
        assert_eq!(unbound.selected_index(&app), None);

        let selection = bound(&mut app, 3, true);
        selection.select(&mut app, 1);
        selection.select(&mut app, 9);
        assert_eq!(selection.selected_index(&app), Some(1));
        assert_eq!(selection.selected_item(&app), Some(11));

        let off = bound(&mut app, 3, false);
        off.select(&mut app, 1);
        assert_eq!(off.selected_index(&app), None);
    }

    #[test]
    fn deselect_clears_only_the_selected_row_and_control_toggles() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let selection = bound(&mut app, 3, true);
        selection.select(&mut app, 2);
        selection.deselect(&mut app, 1);
        assert_eq!(selection.selected_index(&app), Some(2));
        selection.select_from_interaction(&mut app, 2, true);
        assert_eq!(selection.selected_index(&app), None);
        selection.select_from_interaction(&mut app, 0, false);
        selection.select_from_interaction(&mut app, 1, false);
        assert_eq!(selection.selected_index(&app), Some(1));
    }

    #[test]
    fn a_change_is_reported_once_from_the_item_reported_last() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let selection = bound(&mut app, 3, true);
        selection.select(&mut app, 0);
        assert_eq!(selection.take_change(&mut app), Some((None, Some(10))));
        assert_eq!(selection.take_change(&mut app), None);
        selection.set_source(&mut app, 3);
        assert_eq!(selection.take_change(&mut app), Some((Some(10), None)));
    }

    // A row the source moved keeps its selection; one moved across it shifts it.
    #[test]
    fn a_moved_row_keeps_its_selection() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let selection = bound(&mut app, 3, true);
        selection.select(&mut app, 2);
        selection.source_changed(&mut app, &[ViewChange::RowMoved(2, 0)], 3);
        assert_eq!(selection.selected_index(&app), Some(0));
        selection.source_changed(&mut app, &[ViewChange::RowMoved(2, 0)], 3);
        assert_eq!(selection.selected_index(&app), Some(1));
    }
}
