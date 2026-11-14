//! `SelectionModel` (`controls/dev/Repeater/SelectionModel.cpp` and `SelectionNode.cpp`) for one flat list: the selected indices as ranges, the anchor a range selection grows from, and single-select mode.
//!
//! Every method answers whether the selection changed, which is when WinUI's model raises `SelectionChanged`; the owner notifies from that answer, so a later editor who changes the selection without returning `true` leaves containers showing the old selection.

use std::ops::RangeInclusive;

/// The selection of a list of `item_count` items.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectionModel {
    /// Selected indices as disjoint, ordered ranges, which `SelectionNode` keeps as `m_selected`.
    ranges: Vec<RangeInclusive<usize>>,
    /// `AnchorIndex`: where Shift extends a range from.
    anchor: Option<usize>,
    /// `SingleSelect`: selecting an item first clears the others.
    single_select: bool,
    /// The data count `SelectionNode::IsValidIndex` checks indices against.
    item_count: usize,
}

impl SelectionModel {
    /// The number of items selection applies to.
    pub fn item_count(&self) -> usize {
        self.item_count
    }

    /// Sets the number of items; selected indices and an anchor past the end are dropped, as for items the source removed.
    pub fn set_item_count(&mut self, count: usize) -> bool {
        self.item_count = count;
        if self.anchor.is_some_and(|anchor| anchor >= count) {
            self.anchor = None;
        }
        let before = self.ranges.clone();
        self.ranges = before
            .iter()
            .filter(|range| *range.start() < count)
            .map(|range| *range.start()..=(*range.end()).min(count.saturating_sub(1)))
            .collect();
        before != self.ranges
    }

    /// `SingleSelect`: switching it on with several items selected keeps only the first.
    pub fn set_single_select(&mut self, value: bool) -> bool {
        if self.single_select == value {
            return false;
        }
        self.single_select = value;
        if value && self.selected_count() > 1 {
            let first = self.selected_index();
            self.clear();
            if let Some(first) = first {
                self.select_valid(first, true);
                self.anchor = Some(first);
            }
            return true;
        }
        false
    }

    /// `AnchorIndex`.
    pub fn anchor_index(&self) -> Option<usize> {
        self.anchor
    }

    /// `SetAnchorIndex`.
    pub fn set_anchor_index(&mut self, anchor: Option<usize>) {
        self.anchor = anchor;
    }

    /// `IsSelected`.
    pub fn is_selected(&self, index: usize) -> bool {
        self.ranges.iter().any(|range| range.contains(&index))
    }

    /// `SelectedIndices`, in ascending order.
    pub fn selected_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.ranges.iter().flat_map(|range| range.clone())
    }

    /// `SelectedIndex`: the first selected index.
    pub fn selected_index(&self) -> Option<usize> {
        self.ranges.first().map(|range| *range.start())
    }

    /// How many items are selected.
    pub fn selected_count(&self) -> usize {
        self.ranges
            .iter()
            .map(|range| range.end() - range.start() + 1)
            .sum()
    }

    /// `SelectAt` (`SelectWithPathImpl` with `select`): in single-select mode the others are cleared first; selecting or deselecting a valid index makes it the anchor.
    pub fn select_at(&mut self, index: usize) -> bool {
        self.select_with_path(index, true)
    }

    /// `DeselectAt`.
    pub fn deselect_at(&mut self, index: usize) -> bool {
        self.select_with_path(index, false)
    }

    /// `Select(int)` (`SelectImpl`): nothing when the item already is as asked.
    pub fn select(&mut self, index: usize) -> bool {
        self.select_impl(index, true)
    }

    /// `Deselect(int)`.
    pub fn deselect(&mut self, index: usize) -> bool {
        self.select_impl(index, false)
    }

    /// `SelectRangeFromAnchorTo`: the items from the anchor to `index` are selected, the anchor staying where it is.
    pub fn select_range_from_anchor_to(&mut self, index: usize) -> bool {
        self.range_from_anchor(index, true)
    }

    /// `DeselectRangeFromAnchorTo`.
    pub fn deselect_range_from_anchor_to(&mut self, index: usize) -> bool {
        self.range_from_anchor(index, false)
    }

    /// `SelectRange`.
    pub fn select_range(&mut self, start: usize, end: usize) -> bool {
        self.select_range_impl(start, end, true)
    }

    /// `SelectAllFlat`: every item, the anchor unchanged.
    pub fn select_all(&mut self) -> bool {
        if self.item_count == 0 {
            return true;
        }
        self.add_range(0..=self.item_count - 1);
        true
    }

    /// `ClearSelection`: no item selected and, as `SelectionNode::ClearSelection` does, no anchor.
    pub fn clear_selection(&mut self) -> bool {
        self.clear();
        true
    }

    fn clear(&mut self) {
        self.ranges.clear();
        self.anchor = None;
    }

    /// `SelectWithPathImpl`.
    fn select_with_path(&mut self, index: usize, select: bool) -> bool {
        let new_selection = if self.single_select {
            match self.selected_index() {
                Some(selected) => !(select && selected == index),
                None => select,
            }
        } else {
            true
        };
        if !new_selection {
            return false;
        }
        if self.single_select && select {
            self.clear();
        }
        let changed = index < self.item_count && self.is_selected(index) != select;
        if self.select_valid(index, select) {
            self.anchor = Some(index);
        }
        changed
    }

    /// `SelectImpl`.
    fn select_impl(&mut self, index: usize, select: bool) -> bool {
        if self.is_selected(index) == select {
            return false;
        }
        if self.single_select {
            self.clear();
        }
        if self.select_valid(index, select) {
            self.anchor = Some(index);
        }
        true
    }

    /// `SelectRangeImpl` from the anchor; an index with no anchor before it selects nothing.
    fn range_from_anchor(&mut self, index: usize, select: bool) -> bool {
        match self.anchor {
            Some(anchor) => self.select_range_impl(anchor, index, select),
            None => false,
        }
    }

    /// `SelectRangeImpl`: one index is `SelectAt` or `DeselectAt`, a range touches only the valid indices of it.
    fn select_range_impl(&mut self, start: usize, end: usize, select: bool) -> bool {
        if start == end {
            if select {
                self.select_at(start);
            } else {
                self.deselect_at(start);
            }
            return true;
        }
        let (low, high) = (start.min(end), start.max(end));
        if low < self.item_count {
            let range = low..=high.min(self.item_count - 1);
            if select {
                self.add_range(range);
            } else {
                self.remove_range(range);
            }
        }
        true
    }

    /// `SelectionNode::Select`: answers whether `index` is valid.
    fn select_valid(&mut self, index: usize, select: bool) -> bool {
        if index >= self.item_count {
            return false;
        }
        if select {
            self.add_range(index..=index);
        } else {
            self.remove_range(index..=index);
        }
        true
    }

    /// `SelectionNode::AddRange`, keeping the ranges disjoint and ordered.
    fn add_range(&mut self, range: RangeInclusive<usize>) {
        let (mut start, mut end) = (*range.start(), *range.end());
        let mut kept = Vec::with_capacity(self.ranges.len() + 1);
        for existing in self.ranges.drain(..) {
            if *existing.end() + 1 < start || end + 1 < *existing.start() {
                kept.push(existing);
            } else {
                start = start.min(*existing.start());
                end = end.max(*existing.end());
            }
        }
        kept.push(start..=end);
        kept.sort_by_key(|range| *range.start());
        self.ranges = kept;
    }

    /// `SelectionNode::RemoveRange`.
    fn remove_range(&mut self, range: RangeInclusive<usize>) {
        let (start, end) = (*range.start(), *range.end());
        let mut kept = Vec::with_capacity(self.ranges.len() + 1);
        for existing in self.ranges.drain(..) {
            if *existing.end() < start || end < *existing.start() {
                kept.push(existing);
                continue;
            }
            if *existing.start() < start {
                kept.push(*existing.start()..=start - 1);
            }
            if end < *existing.end() {
                kept.push(end + 1..=*existing.end());
            }
        }
        self.ranges = kept;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(count: usize) -> SelectionModel {
        let mut model = SelectionModel::default();
        model.set_item_count(count);
        model
    }

    fn selected(model: &SelectionModel) -> Vec<usize> {
        model.selected_indices().collect()
    }

    #[test]
    fn single_select_replaces_the_selection_and_moves_the_anchor() {
        let mut model = model(5);
        model.set_single_select(true);
        model.select_at(1);
        model.select_at(3);
        assert_eq!(selected(&model), [3]);
        assert_eq!(model.anchor_index(), Some(3));
        assert!(
            !model.select_at(3),
            "selecting the selected item is no change"
        );
    }

    #[test]
    fn deselecting_also_moves_the_anchor() {
        let mut model = model(5);
        model.select_at(1);
        model.deselect_at(4);
        assert_eq!(model.anchor_index(), Some(4));
    }

    #[test]
    fn ranges_from_the_anchor_merge_and_split() {
        let mut model = model(10);
        model.select_at(2);
        model.select_range_from_anchor_to(5);
        assert_eq!(selected(&model), [2, 3, 4, 5]);
        assert_eq!(model.anchor_index(), Some(2));
        model.set_anchor_index(Some(4));
        model.deselect_range_from_anchor_to(3);
        assert_eq!(selected(&model), [2, 5]);
        model.select_range(5, 7);
        assert_eq!(selected(&model), [2, 5, 6, 7]);
    }

    #[test]
    fn select_all_keeps_the_anchor_and_clear_drops_it() {
        let mut model = model(4);
        model.select_at(1);
        model.select_all();
        assert_eq!(selected(&model), [0, 1, 2, 3]);
        assert_eq!(model.anchor_index(), Some(1));
        model.clear_selection();
        assert_eq!(selected(&model), Vec::<usize>::new());
        assert_eq!(model.anchor_index(), None);
    }

    #[test]
    fn switching_to_single_select_keeps_the_first() {
        let mut model = model(6);
        model.select_range(1, 4);
        assert!(model.set_single_select(true));
        assert_eq!(selected(&model), [1]);
    }

    #[test]
    fn a_shorter_list_drops_what_is_past_its_end() {
        let mut model = model(10);
        model.select_range(3, 8);
        model.set_anchor_index(Some(8));
        assert!(model.set_item_count(5));
        assert_eq!(selected(&model), [3, 4]);
        assert_eq!(model.anchor_index(), None);
        assert!(!model.select_at(7), "past the end is no item");
    }
}
