//! Original-order partitions and width policy from TopNavigationViewDataProvider and NavigationView.cpp.

/// Cached top-level identity, natural width and partition membership.
#[derive(Clone, Debug)]
struct Item {
    /// Stable owner identity, independent of either projected list index.
    id: String,
    /// Invalid until the top presenter has measured this item.
    width: Option<f64>,
    /// Whether the item belongs to the primary strip.
    primary: bool,
}

/// Keeps projection indices separate from the owner's collection indices.
#[derive(Clone, Debug, Default)]
pub(crate) struct TopNavigationViewDataProvider {
    /// Original-order source with attached width and partition data.
    items: Vec<Item>,
    /// Measured width of the overflow button, including its template margin.
    pub overflow_button_width: f64,
}

impl TopNavigationViewDataProvider {
    /// Reconciles a source snapshot; collection changes restart the source measure policy.
    pub fn set_data_source(&mut self, ids: impl IntoIterator<Item = String>) {
        let ids: Vec<_> = ids.into_iter().collect();
        if self.items.iter().map(|item| &item.id).eq(ids.iter()) {
            return;
        }
        self.items = ids
            .into_iter()
            .map(|id| {
                let width = self
                    .items
                    .iter()
                    .find(|item| item.id == id)
                    .and_then(|item| item.width);
                Item {
                    id,
                    width,
                    primary: true,
                }
            })
            .collect();
    }

    /// Returns original indices of the primary projection.
    pub fn primary_items(&self) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| item.primary.then_some(i))
            .collect()
    }

    /// Returns original indices of the overflow projection.
    pub fn overflow_items(&self) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| (!item.primary).then_some(i))
            .collect()
    }

    /// Invalidates measurements after content or typography changes.
    pub fn invalidate_width_cache(&mut self) {
        for item in &mut self.items {
            item.width = None;
        }
        self.move_all_items_to_primary_list();
    }

    /// Accepts finite nonnegative natural presenter widths.
    pub fn set_width_for_item(&mut self, index: usize, width: f64) -> bool {
        if width.is_finite() && width >= 0.0 && self.items[index].width != Some(width) {
            self.items[index].width = Some(width);
            true
        } else {
            false
        }
    }

    /// Gets a valid cached width, using the source's zero for invalid data.
    fn get_width_for_item(&self, index: usize) -> f64 {
        self.items[index].width.unwrap_or(0.0)
    }

    /// Measures only primary items and the overflow affordance.
    fn desired_width(&self) -> f64 {
        self.primary_items()
            .iter()
            .map(|i| self.get_width_for_item(*i))
            .sum::<f64>()
            + if self.overflow_items().is_empty() {
                0.0
            } else {
                self.overflow_button_width
            }
    }

    /// Returns every item to primary for a fresh measurement.
    fn move_all_items_to_primary_list(&mut self) {
        for item in &mut self.items {
            item.primary = true;
        }
    }

    /// Moves original indices while preserving ordering in both projections.
    fn move_items(&mut self, indices: &[usize], primary: bool) {
        for index in indices {
            self.items[*index].primary = primary;
        }
    }

    /// Removes from the end while respecting the selected item exclusion.
    fn find_movable_items_to_be_removed(&self, mut width: f64, exclude: &[usize]) -> Vec<usize> {
        let mut result = Vec::new();
        for index in (0..self.items.len()).rev() {
            if width <= 0.0 {
                break;
            }
            if self.items[index].primary && !exclude.contains(&index) {
                result.push(index);
                width -= self.get_width_for_item(index);
            }
        }
        result
    }

    /// Recovers in source order and leaves one overflow item until full recovery qualifies.
    fn find_movable_items_recover(&self, mut available: f64, include: &[usize]) -> Vec<usize> {
        let mut result = include.to_vec();
        for index in include {
            available -= self.get_width_for_item(*index);
        }
        let mut index = 0;
        while index < self.items.len() && available > 0.0 {
            if !self.items[index].primary && !include.contains(&index) {
                let width = self.get_width_for_item(index);
                if available < width {
                    break;
                }
                result.push(index);
                available -= width;
            }
            index += 1;
        }
        if index == self.items.len() && !result.is_empty() {
            result.pop();
        }
        result
    }

    /// Keeps at least one primary item even when the host is narrower than that item.
    fn keep_at_least_one(&self, removed: &mut Vec<usize>, keep_first: bool) {
        if !removed.is_empty() && removed.len() == self.primary_items().len() {
            if keep_first {
                removed.remove(0);
            } else {
                removed.pop();
            }
        }
    }

    /// ShrinkTopNavigationSize's visible-prefix pass followed by reverse removal.
    fn shrink(&mut self, available: f64, selected: Option<usize>) {
        let possible = available - self.overflow_button_width;
        if possible >= 0.0 {
            let mut required = 0.0;
            let mut removed = Vec::new();
            for index in self.primary_items() {
                if Some(index) != selected {
                    required += self.get_width_for_item(index);
                    if required > possible {
                        removed.push(index);
                    }
                }
            }
            self.keep_at_least_one(&mut removed, true);
            self.move_items(&removed, false);
        }
        // The overflow button is visible during this measurement, even before the first move.
        let desired = self
            .primary_items()
            .iter()
            .map(|i| self.get_width_for_item(*i))
            .sum::<f64>()
            + self.overflow_button_width;
        let mut removed = self.find_movable_items_to_be_removed(
            desired - available,
            &selected.into_iter().collect::<Vec<_>>(),
        );
        self.keep_at_least_one(&mut removed, false);
        self.move_items(&removed, false);
    }

    /// Applies normal/overflow measurement, including the source's five-pixel recovery grace.
    pub fn arrange(&mut self, available: f64, selected: Option<usize>) -> bool {
        let before = self.primary_items();
        if available.is_infinite() {
            self.move_all_items_to_primary_list();
        } else if self.items.iter().all(|item| item.width.is_some()) {
            let desired = self.desired_width();
            if desired > available {
                self.shrink(available, selected);
            } else if !self.overflow_items().is_empty() && desired < available {
                let recovery = (self
                    .overflow_items()
                    .iter()
                    .map(|i| self.get_width_for_item(*i))
                    .sum::<f64>()
                    - self.overflow_button_width)
                    .max(0.0);
                if available >= desired + recovery + 5.0 {
                    self.move_all_items_to_primary_list();
                    if self.desired_width() >= available {
                        self.shrink(available, selected);
                    }
                } else {
                    let recovered = self.find_movable_items_recover(available - desired, &[]);
                    self.move_items(&recovered, true);
                }
            }
        }
        before != self.primary_items()
    }

    /// SelectOverflowItem promotes a nested selection's top-level ancestor.
    pub fn select_overflow_item(&mut self, selected: usize, available: f64) {
        if self.items[selected].primary {
            return;
        }
        if self.items.iter().any(|item| item.width.is_none()) {
            self.move_all_items_to_primary_list();
            return;
        }
        let minimum = self.desired_width() + self.get_width_for_item(selected) - available;
        let removed = self.find_movable_items_to_be_removed(minimum, &[]);
        let recovered_width = removed
            .iter()
            .map(|i| self.get_width_for_item(*i))
            .sum::<f64>()
            - minimum;
        let mut added = self.find_movable_items_recover(recovered_width, &[selected]);
        if !added.contains(&selected) {
            added.push(selected);
        }
        self.move_items(&added, true);
        self.move_items(&removed, false);
        let primary = self.primary_items();
        if let Some(position) = primary.iter().position(|index| *index == selected) {
            // Retain the source's previous-index check, which refers to the selected slot.
            let rearrange = position + 1 < primary.len()
                && (position > 0
                    || primary[position + 1..]
                        .iter()
                        .enumerate()
                        .any(|(offset, index)| *index != selected + offset + 1));
            if rearrange {
                self.move_all_items_to_primary_list();
                self.arrange(available, Some(selected));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> TopNavigationViewDataProvider {
        let mut p = TopNavigationViewDataProvider::default();
        p.set_data_source((0..5).map(|i| i.to_string()));
        for i in 0..5 {
            p.set_width_for_item(i, 100.0);
        }
        p.overflow_button_width = 40.0;
        p
    }

    #[test]
    fn shrinking_protects_selection_and_at_least_one_primary() {
        let mut p = provider();
        p.arrange(350.0, Some(4));
        assert_eq!(p.primary_items(), vec![0, 1, 4]);
        assert_eq!(p.overflow_items(), vec![2, 3]);
        p.arrange(20.0, Some(4));
        assert_eq!(p.primary_items(), vec![4]);
    }

    #[test]
    fn recovery_keeps_overflow_until_five_pixel_grace() {
        let mut p = provider();
        p.arrange(350.0, Some(0));
        p.arrange(500.0, Some(0));
        assert_eq!(p.primary_items(), vec![0, 1, 2, 3]);
        p.arrange(504.0, Some(0));
        assert_eq!(p.overflow_items(), vec![4]);
        p.arrange(505.0, Some(0));
        assert!(p.overflow_items().is_empty());
    }

    #[test]
    fn overflow_selection_promotes_identity_and_preserves_source_order() {
        let mut p = provider();
        p.arrange(350.0, Some(0));
        p.select_overflow_item(4, 350.0);
        assert_eq!(p.primary_items(), vec![0, 1, 4]);
        assert_eq!(p.overflow_items(), vec![2, 3]);
        p.arrange(f64::INFINITY, Some(4));
        assert!(p.overflow_items().is_empty());
    }
}
