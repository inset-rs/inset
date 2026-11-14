//! `ItemsView`'s selection and current item, held where the application reaches them: `Select`, `Deselect`, `IsSelected`, `SelectAll`, `DeselectAll`, `InvertSelection`, `SelectedItems` and `CurrentItemIndex` of `ItemsView.idl`, over its private `SelectionModel` and `m_currentElementSelectionModel`.
//!
//! Listeners hear every change of the selection or of the current item; each `ItemContainer` listens for its own index. A later editor who changes the model here without notifying leaves containers showing the old selection.

use super::selection_model::SelectionModel;
use super::selector::{ItemsViewSelectionMode, Selector};
use waft_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle};

/// The selection and the current item of an `ItemsView`.
///
/// An application makes one with [`ItemsViewSelection::new`], hands it to the `ItemsView`, and disposes it after the view; an `ItemsView` given none makes its own.
pub struct ItemsViewSelection {
    notifier: ChangeNotifierData,
    model: SelectionModel,
    selector: Selector,
    /// `CurrentItemIndex`: the item that has, or last had, the focus.
    current_item_index: Option<usize>,
    /// Counts the selection's changes, which the view raises `SelectionChanged` for, apart from the current item's.
    revision: u64,
}

impl ItemsViewSelection {
    /// Creates an empty selection for `ItemsView`'s default `Single` mode.
    pub fn new(app: &mut App) -> Handle<Self> {
        let mut model = SelectionModel::default();
        let (selector, _) = Selector::for_mode(ItemsViewSelectionMode::Single, &mut model);
        app.create(Self {
            notifier: ChangeNotifierData::new(),
            model,
            selector,
            current_item_index: None,
            revision: 0,
        })
    }

    /// Releases the listeners; the selection is not used after.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).notifier.dispose();
    }

    /// `IsSelected`.
    pub fn is_selected(self: Handle<Self>, app: &App, index: usize) -> bool {
        app.get(self).model.is_selected(index)
    }

    /// The selected indices in ascending order: `SelectedItems` as positions.
    pub fn selected_indices(self: Handle<Self>, app: &App) -> Vec<usize> {
        app.get(self).model.selected_indices().collect()
    }

    /// `SelectedItem` as a position: the first selected index.
    pub fn selected_index(self: Handle<Self>, app: &App) -> Option<usize> {
        app.get(self).model.selected_index()
    }

    /// `CurrentItemIndex`.
    pub fn current_item_index(self: Handle<Self>, app: &App) -> Option<usize> {
        app.get(self).current_item_index
    }

    /// `Select`.
    pub fn select(self: Handle<Self>, app: &mut App, index: usize) {
        self.change(app, |model| model.select(index));
    }

    /// `Deselect`.
    pub fn deselect(self: Handle<Self>, app: &mut App, index: usize) {
        self.change(app, |model| model.deselect(index));
    }

    /// `SelectAll` (`SelectAllFlat`): every item, whatever the mode, as WinUI's model does.
    pub fn select_all(self: Handle<Self>, app: &mut App) {
        self.change(app, SelectionModel::select_all);
    }

    /// `DeselectAll`.
    pub fn deselect_all(self: Handle<Self>, app: &mut App) {
        self.change(app, SelectionModel::clear_selection);
    }

    /// `InvertSelection`: the selected items become unselected and the others selected.
    pub fn invert_selection(self: Handle<Self>, app: &mut App) {
        self.change(app, |model| {
            let selected: Vec<usize> = model.selected_indices().collect();
            let mut index_end = model.item_count() as isize - 1;
            for index in selected.into_iter().rev() {
                if (index as isize) < index_end {
                    model.select_range(index + 1, index_end as usize);
                }
                model.deselect_at(index);
                index_end = index as isize - 1;
            }
            if index_end >= 0 {
                model.select_range(0, index_end as usize);
            }
            true
        });
    }

    pub(crate) fn selector(self: Handle<Self>, app: &App) -> Selector {
        app.get(self).selector
    }

    /// `ItemsView::OnSelectionModeChanged` and `UpdateSelector`.
    pub(crate) fn set_selection_mode(
        self: Handle<Self>,
        app: &mut App,
        mode: ItemsViewSelectionMode,
    ) {
        let mut selector = Selector::Null;
        self.change(app, |model| {
            let (made, changed) = Selector::for_mode(mode, model);
            selector = made;
            changed
        });
        app.get_mut(self).selector = selector;
    }

    /// The number of items the view shows; selection past the end is dropped.
    pub(crate) fn set_item_count(self: Handle<Self>, app: &mut App, count: usize) {
        self.change(app, |model| model.set_item_count(count));
    }

    /// `ItemsView::SetCurrentElementIndex`.
    pub(crate) fn set_current_item_index(self: Handle<Self>, app: &mut App, index: Option<usize>) {
        if app.get(self).current_item_index != index {
            app.get_mut(self).current_item_index = index;
            self.notify_listeners(app);
        }
    }

    /// `OnInteractedAction` of the selector for the view's mode.
    pub(crate) fn interacted(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        ctrl: bool,
        shift: bool,
    ) {
        let selector = self.selector(app);
        self.change(app, |model| {
            selector.on_interacted_action(model, index, ctrl, shift)
        });
    }

    /// `OnFocusedAction` of the selector for the view's mode.
    pub(crate) fn focused(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        ctrl: bool,
        shift: bool,
    ) {
        let selector = self.selector(app);
        self.change(app, |model| {
            selector.on_focused_action(model, index, ctrl, shift)
        });
    }

    /// `SelectorBase::DeselectWithAnchorPreservation`.
    pub(crate) fn deselect_with_anchor_preservation(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
    ) {
        self.change(app, |model| {
            Selector::deselect_with_anchor_preservation(model, index)
        });
    }

    /// How many times the selection has changed.
    pub(crate) fn revision(self: Handle<Self>, app: &App) -> u64 {
        app.get(self).revision
    }

    /// Tells the containers that something they show besides the selection changed: which item Tab stops at.
    pub(crate) fn notify_items(self: Handle<Self>, app: &mut App) {
        self.notify_listeners(app);
    }

    /// Runs `change` on the model and notifies when it says the selection changed.
    fn change(self: Handle<Self>, app: &mut App, change: impl FnOnce(&mut SelectionModel) -> bool) {
        if change(&mut app.get_mut(self).model) {
            app.get_mut(self).revision += 1;
            self.notify_listeners(app);
        }
    }
}

impl ChangeNotifier for ItemsViewSelection {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.notifier
    }
}
