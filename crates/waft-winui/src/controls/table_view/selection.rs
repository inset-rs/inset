//! `TableView`'s row selection (`controls/dev/TableView/TableView_Selection.cpp`): the table's gestures and `SelectionChanged`, and the selection held where the application reaches it: `Select`, `Deselect`, `IsSelected`, `DeselectAll`, `SelectedIndex` and `SelectedItem` of `TableView.idl`, over the single-select `SelectionModel` the control keeps.
//!
//! The selection is resolved only while a `TableView` holds it: the table gives it the number of rows, whether its mode allows selection, and how a row maps to an item. Every change of the selected row notifies, and nothing else does, for the table binds the selection while it builds and a listener above the table may rebuild; the table raises `SelectionChanged` from the item reported last, and each row listens for its own index. A later editor who changes the model without notifying leaves rows showing the old selection.

use super::super::items_view::selection_model::SelectionModel;
use super::source::{TableViewSource, ViewChange};
use super::*;
use std::fmt;
use std::rc::Rc;
use waft_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, HandleId, Ref};

/// The index in the source's items of a row of the view.
pub(crate) type ItemOfRow = Rc<dyn Fn(&App, usize) -> Option<usize>>;

/// Adds a listener to an items source.
type AddSourceListener = Rc<dyn Fn(&mut App, Listener)>;
/// Removes a listener from an items source.
type RemoveSourceListener = Rc<dyn Fn(&mut App, &Listener)>;
/// Reads the changes an items source last made.
type ReadSourceChanges = Rc<dyn Fn(&App) -> Vec<ViewChange>>;
/// Reads the number of rows in an items source's view.
type ReadSourceRowCount = Rc<dyn Fn(&App) -> usize>;

/// The items source a selection follows, erased from the source's item type: how to listen to it,
/// and how to read the changes it last made and the number of rows in its view.
#[derive(Clone)]
pub(crate) struct SelectionSource {
    id: HandleId,
    add_listener: AddSourceListener,
    remove_listener: RemoveSourceListener,
    changes: ReadSourceChanges,
    row_count: ReadSourceRowCount,
}

impl SelectionSource {
    /// The subscription a selection makes to `source`.
    pub(crate) fn new<T: 'static>(source: &Ref<TableViewSource<T>>) -> SelectionSource {
        let for_adding = source.clone();
        let for_removing = source.clone();
        let for_changes = source.clone();
        let for_rows = source.clone();
        SelectionSource {
            id: source.handle().id(),
            add_listener: Rc::new(move |app, listener| for_adding.add_listener(app, listener)),
            remove_listener: Rc::new(move |app, listener| {
                for_removing.remove_listener(app, listener)
            }),
            changes: Rc::new(move |app| for_changes.changes(app).to_vec()),
            row_count: Rc::new(move |app| for_rows.view(app).len()),
        }
    }
}

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
    /// The items source the selection follows, kept across tables and heard from whether or not a
    /// table holds the selection; a table adopting it again changes nothing.
    source: Option<SelectionSource>,
    /// The listener the selection has on its source.
    source_listener: Option<Listener>,
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
            source: None,
            source_listener: None,
            last_raised_item: None,
        })
    }

    /// Releases the listeners, including the one on the source it follows; the selection is not
    /// used after.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        self.stop_following_source(app);
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
    /// The table `owner` holds the selection over `source`; a selection set to a different source starts over on its rows, as `SelectionModel::Source` clears. Adopting the same source again keeps the selection. Answers whether the selected row was cleared, which the table defers to its listener until after the frame.
    pub(crate) fn bind(
        self: Handle<Self>,
        app: &mut App,
        owner: HandleId,
        can_select: bool,
        row_count: usize,
        source: SelectionSource,
        item_of_row: ItemOfRow,
    ) -> bool {
        let source_changed = app
            .get(self)
            .source
            .as_ref()
            .is_none_or(|bound| bound.id != source.id);
        if source_changed {
            // The selection follows its source itself, as WinUI's model does, so rows that change
            // while no table holds the selection still move it.
            self.stop_following_source(app);
            let listener = Listener::handle_method(self, Self::source_heard);
            (source.add_listener)(app, listener.clone());
            let selection = app.get_mut(self);
            selection.source = Some(source);
            selection.source_listener = Some(listener);
        }
        app.get_mut(self).binding = Some(SelectionBinding {
            owner,
            can_select,
            item_of_row,
        });
        let cleared = if source_changed {
            app.get_mut(self).start_over(row_count)
        } else {
            false
        };
        if !can_select {
            // `OnSelectionModePropertyChanged`: turning selection off clears it.
            self.apply(app, None);
        }
        cleared
    }

    /// Stops listening to the source the selection follows, if any.
    fn stop_following_source(self: Handle<Self>, app: &mut App) {
        let (source, listener) = {
            let selection = app.get_mut(self);
            (selection.source.take(), selection.source_listener.take())
        };
        if let (Some(source), Some(listener)) = (source, listener) {
            (source.remove_listener)(app, &listener);
        }
    }

    /// The source the selection follows changed its view: `SelectionNode::OnSourceListChanged`,
    /// which the model hears whether or not a table is attached.
    fn source_heard(self: Handle<Self>, app: &mut App) {
        let (changes, row_count) = {
            let selection = app.get(self);
            let Some(source) = &selection.source else {
                return;
            };
            ((source.changes)(app), (source.row_count)(app))
        };
        self.source_changed(app, &changes, row_count);
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
                ViewChange::Reset => selection.start_over(row_count),
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

    /// `SelectionModel::Source`: the selection starts over on `row_count` rows. Answers whether a row was selected, the only change a listener can read: WinUI's model raises `SelectionChanged` whatever it cleared, and `TableView::PushSelectionProperties` republishes `SelectedIndex` and `SelectedItem` only when they differ.
    fn start_over(&mut self, row_count: usize) -> bool {
        let cleared = self.model.selected_index().is_some();
        self.model.set_source(row_count);
        cleared
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
            let source = SelectionSource::new(&self.widget(app).items_source);
            // A replaced source clears the selected row while the table builds; its listener hears
            // of it once the frame is over, as a rebuild of the table during build must not dirty
            // a widget above it.
            if selection.bind(app, id, can_select, count, source, item_of_row) {
                let selection = selection.clone();
                self.after_frame(app, move |app| {
                    if app.contains(selection.handle()) {
                        selection.handle().notify_listeners(app);
                    }
                });
            }
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
        let source = TableViewSource::from(app, vec![0u8; rows]);
        bound_to(app, rows, can_select, &source)
    }

    fn bound_to(
        app: &mut App,
        rows: usize,
        can_select: bool,
        source: &Ref<TableViewSource<u8>>,
    ) -> Ref<TableViewSelection> {
        let selection = TableViewSelection::new(app);
        let owner = app.create(0u8);
        selection.bind(
            app,
            owner.handle().id(),
            can_select,
            rows,
            SelectionSource::new(source),
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
        // A replaced source clears the row and the clear is reported from the item last raised.
        let owner = app.create(0u8);
        let other_source = TableViewSource::from(&mut app, vec![0u8; 3]);
        assert!(selection.bind(
            &mut app,
            owner.handle().id(),
            true,
            3,
            SelectionSource::new(&other_source),
            Rc::new(|_, row| Some(row)),
        ));
        assert_eq!(selection.take_change(&mut app), Some((Some(10), None)));
    }

    /// A table adopting the same source again changes nothing: the selected row stays and no
    /// listener hears of it.
    #[test]
    fn rebinding_the_same_source_keeps_the_selection() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let source = TableViewSource::from(&mut app, vec![0u8; 3]);
        let selection = bound_to(&mut app, 3, true, &source);
        selection.select(&mut app, 1);
        assert_eq!(selection.selected_index(&app), Some(1));

        let heard = Rc::new(std::cell::Cell::new(0));
        let heard_here = Rc::clone(&heard);
        selection.add_listener(
            &mut app,
            Listener::new(move |_app| heard_here.set(heard_here.get() + 1)),
        );

        let owner = app.create(0u8);
        let cleared = selection.bind(
            &mut app,
            owner.handle().id(),
            true,
            3,
            SelectionSource::new(&source),
            Rc::new(|_, row| Some(row)),
        );
        assert!(!cleared);
        assert_eq!(selection.selected_index(&app), Some(1));
        assert_eq!(heard.get(), 0);
    }

    /// The selection follows its source with no table holding it, as WinUI's model does: a row
    /// inserted above moves the selected row down, and removing the selected row clears it.
    #[test]
    fn the_selection_follows_its_source_with_no_table_holding_it() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let source = TableViewSource::from(&mut app, vec![10u8, 20, 30, 40]);
        let owner = app.create(0u8);
        let selection = TableViewSelection::new(&mut app);
        let rows_of = source.clone();
        selection.bind(
            &mut app,
            owner.handle().id(),
            true,
            4,
            SelectionSource::new(&source),
            Rc::new(move |app, row| rows_of.view(app).get(row).copied()),
        );
        selection.select(&mut app, 2);
        assert_eq!(selection.selected_index(&app), Some(2));

        // The table goes away; the source inserts a row above the selected one.
        selection.unbind(&mut app, owner.handle().id());
        source.insert(&mut app, 0, 5);
        assert_eq!(selection.selected_index(&app), Some(3));

        // Rows leave from the end down to the selected row, which clears it.
        source.remove(&mut app, 4);
        source.remove(&mut app, 3);
        assert_eq!(selection.selected_index(&app), None);
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
