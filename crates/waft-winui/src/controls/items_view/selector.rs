//! `SelectorBase`, `NullSelector`, `SingleSelector`, `MultipleSelector` and `ExtendedSelector` (`controls/dev/ItemsView`): what a click, Space or Enter on an item (`OnInteractedAction`) and the keyboard moving the focus to it (`OnFocusedAction`) do to the selection, with Control and Shift.
//!
//! Each rule is the C++'s line for line; the selection model answers whether anything changed, and every rule answers the same, so a later editor who drops an answer loses a notification.

use super::selection_model::SelectionModel;

/// `ItemsViewSelectionMode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ItemsViewSelectionMode {
    /// No item can be selected.
    None,
    /// One item at a time; the default (`ItemsView::s_defaultSelectionMode`).
    #[default]
    Single,
    /// Each click toggles an item; Shift extends from the anchor.
    Multiple,
    /// A click selects one item; Control toggles and Shift extends.
    Extended,
}

/// The selector `ItemsView::UpdateSelector` makes for a selection mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Selector {
    Null,
    /// `SingleSelector`, with `m_followFocus`.
    Single {
        follow_focus: bool,
    },
    Multiple,
    Extended,
}

impl Selector {
    /// `ItemsView::UpdateSelector`: the selector for `mode`, after setting the model's single-select mode and, for `None`, clearing it.
    pub(crate) fn for_mode(
        mode: ItemsViewSelectionMode,
        model: &mut SelectionModel,
    ) -> (Selector, bool) {
        let mut changed = model.set_single_select(false);
        let selector = match mode {
            ItemsViewSelectionMode::None => {
                changed |= model.clear_selection();
                Selector::Null
            }
            ItemsViewSelectionMode::Single => {
                changed |= model.set_single_select(true);
                Selector::Single { follow_focus: true }
            }
            ItemsViewSelectionMode::Multiple => Selector::Multiple,
            ItemsViewSelectionMode::Extended => Selector::Extended,
        };
        (selector, changed)
    }

    /// `OnInteractedAction`: a click, Space or Enter on `index`.
    pub(crate) fn on_interacted_action(
        self,
        model: &mut SelectionModel,
        index: usize,
        ctrl: bool,
        shift: bool,
    ) -> bool {
        match self {
            Selector::Null => false,
            Selector::Single { .. } => {
                let mut changed = model.set_single_select(true);
                changed |= if !ctrl || !model.is_selected(index) {
                    model.select_at(index)
                } else {
                    model.deselect_at(index)
                };
                changed
            }
            Selector::Multiple => {
                if shift {
                    match model.anchor_index() {
                        Some(anchor) => {
                            let anchor_selected = model.is_selected(anchor);
                            if anchor_selected == model.is_selected(index) {
                                false
                            } else if anchor_selected {
                                model.select_range_from_anchor_to(index)
                            } else {
                                model.deselect_range_from_anchor_to(index)
                            }
                        }
                        None => false,
                    }
                } else if model.is_selected(index) {
                    model.deselect_at(index)
                } else {
                    model.select_at(index)
                }
            }
            Selector::Extended => {
                if shift {
                    extend_from_anchor(model, index)
                } else if ctrl {
                    if model.is_selected(index) {
                        model.deselect_at(index)
                    } else {
                        model.select_at(index)
                    }
                } else if !model.is_selected(index) {
                    // Only clear selection if interacting with a different item.
                    model.clear_selection();
                    model.select_at(index);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// `OnFocusedAction`: the keyboard moved the focus to `index`.
    pub(crate) fn on_focused_action(
        self,
        model: &mut SelectionModel,
        index: usize,
        ctrl: bool,
        shift: bool,
    ) -> bool {
        match self {
            Selector::Null => false,
            Selector::Single { follow_focus } => {
                let mut changed = model.set_single_select(true);
                if !ctrl && follow_focus {
                    changed |= model.select_at(index);
                }
                changed
            }
            Selector::Multiple => {
                if !shift {
                    return false;
                }
                match model.anchor_index() {
                    Some(anchor) if model.is_selected(anchor) => {
                        model.select_range_from_anchor_to(index)
                    }
                    Some(_) => model.deselect_range_from_anchor_to(index),
                    None => false,
                }
            }
            Selector::Extended => {
                if shift && ctrl {
                    model.anchor_index().is_some() && model.select_range_from_anchor_to(index)
                } else if shift {
                    extend_from_anchor(model, index)
                } else if !ctrl {
                    model.clear_selection();
                    model.select_at(index);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// `SelectorBase::DeselectWithAnchorPreservation`.
    pub(crate) fn deselect_with_anchor_preservation(
        model: &mut SelectionModel,
        index: usize,
    ) -> bool {
        let anchor = model.anchor_index();
        let changed = model.deselect(index);
        if anchor.is_some() {
            model.set_anchor_index(anchor);
        }
        changed
    }
}

/// Extended mode's Shift: the selection becomes the range from the anchor to `index`.
fn extend_from_anchor(model: &mut SelectionModel, index: usize) -> bool {
    match model.anchor_index() {
        Some(anchor) => {
            model.clear_selection();
            model.set_anchor_index(Some(anchor));
            model.select_range_from_anchor_to(index);
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(mode: ItemsViewSelectionMode, count: usize) -> (Selector, SelectionModel) {
        let mut model = SelectionModel::default();
        model.set_item_count(count);
        let (selector, _) = Selector::for_mode(mode, &mut model);
        (selector, model)
    }

    fn selected(model: &SelectionModel) -> Vec<usize> {
        model.selected_indices().collect()
    }

    #[test]
    fn single_click_selects_and_control_click_deselects() {
        let (selector, mut model) = with(ItemsViewSelectionMode::Single, 5);
        selector.on_interacted_action(&mut model, 2, false, false);
        selector.on_interacted_action(&mut model, 4, false, false);
        assert_eq!(selected(&model), [4]);
        selector.on_interacted_action(&mut model, 4, true, false);
        assert_eq!(selected(&model), Vec::<usize>::new());
        selector.on_interacted_action(&mut model, 1, true, false);
        assert_eq!(
            selected(&model),
            [1],
            "control on an unselected item selects it"
        );
    }

    #[test]
    fn single_selection_follows_the_focus_unless_control_is_held() {
        let (selector, mut model) = with(ItemsViewSelectionMode::Single, 5);
        selector.on_focused_action(&mut model, 3, false, false);
        assert_eq!(selected(&model), [3]);
        selector.on_focused_action(&mut model, 4, true, false);
        assert_eq!(selected(&model), [3]);
    }

    #[test]
    fn multiple_click_toggles_and_shift_follows_the_anchors_state() {
        let (selector, mut model) = with(ItemsViewSelectionMode::Multiple, 8);
        selector.on_interacted_action(&mut model, 1, false, false);
        selector.on_interacted_action(&mut model, 3, false, false);
        assert_eq!(selected(&model), [1, 3]);
        // The anchor, 3, is selected: Shift selects from it to 6.
        selector.on_interacted_action(&mut model, 6, false, true);
        assert_eq!(selected(&model), [1, 3, 4, 5, 6]);
        // Toggling 5 off makes it the anchor; Shift to 3 deselects 3 to 5.
        selector.on_interacted_action(&mut model, 5, false, false);
        selector.on_interacted_action(&mut model, 3, false, true);
        assert_eq!(selected(&model), [1, 6]);
        // Shift onto an item in the anchor's state changes nothing.
        selector.on_interacted_action(&mut model, 2, false, true);
        assert_eq!(selected(&model), [1, 6]);
    }

    #[test]
    fn multiple_focus_moves_select_only_with_shift() {
        let (selector, mut model) = with(ItemsViewSelectionMode::Multiple, 8);
        selector.on_interacted_action(&mut model, 2, false, false);
        selector.on_focused_action(&mut model, 3, false, false);
        assert_eq!(selected(&model), [2]);
        selector.on_focused_action(&mut model, 4, false, true);
        assert_eq!(selected(&model), [2, 3, 4]);
    }

    #[test]
    fn extended_click_control_and_shift() {
        let (selector, mut model) = with(ItemsViewSelectionMode::Extended, 10);
        selector.on_interacted_action(&mut model, 2, false, false);
        selector.on_interacted_action(&mut model, 5, true, false);
        assert_eq!(selected(&model), [2, 5]);
        // Shift: the range from the anchor, 5, replaces the selection.
        selector.on_interacted_action(&mut model, 7, false, true);
        assert_eq!(selected(&model), [5, 6, 7]);
        // A plain click on a selected item keeps the selection.
        selector.on_interacted_action(&mut model, 6, false, false);
        assert_eq!(selected(&model), [5, 6, 7]);
        // On another item it selects only that one.
        selector.on_interacted_action(&mut model, 0, false, false);
        assert_eq!(selected(&model), [0]);
    }

    #[test]
    fn extended_focus_moves() {
        let (selector, mut model) = with(ItemsViewSelectionMode::Extended, 10);
        selector.on_focused_action(&mut model, 2, false, false);
        assert_eq!(selected(&model), [2]);
        selector.on_focused_action(&mut model, 4, false, true);
        assert_eq!(selected(&model), [2, 3, 4]);
        // Control alone moves the focus and leaves the selection.
        selector.on_focused_action(&mut model, 6, true, false);
        assert_eq!(selected(&model), [2, 3, 4]);
        // Control and Shift add the range from the anchor without clearing.
        model.set_anchor_index(Some(8));
        selector.on_focused_action(&mut model, 9, true, true);
        assert_eq!(selected(&model), [2, 3, 4, 8, 9]);
    }

    #[test]
    fn none_clears_and_ignores() {
        let (_, mut model) = with(ItemsViewSelectionMode::Extended, 4);
        model.select_all();
        let (selector, _) = Selector::for_mode(ItemsViewSelectionMode::None, &mut model);
        assert_eq!(selected(&model), Vec::<usize>::new());
        assert!(!selector.on_interacted_action(&mut model, 1, false, false));
    }

    #[test]
    fn deselecting_with_anchor_preservation_keeps_the_anchor() {
        let (_, mut model) = with(ItemsViewSelectionMode::Extended, 6);
        model.select_at(1);
        model.select_range_from_anchor_to(4);
        Selector::deselect_with_anchor_preservation(&mut model, 3);
        assert_eq!(selected(&model), [1, 2, 4]);
        assert_eq!(model.anchor_index(), Some(1));
    }
}
