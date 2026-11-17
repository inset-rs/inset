//! Which trailing back-stack entries one page shows together, and what back does to them.

use std::ops::Range;

use crate::compose::{
    BackNavigationBehavior, PaneScaffoldDirective, ThreePaneScaffoldAdaptStrategies,
    ThreePaneScaffoldDestinationItem, ThreePaneScaffoldRole, ThreePaneScaffoldValue,
};

/// How the trailing pane entries of a back stack share one page.
///
/// A back stack is the app's own list of entries; `back_stack[i]` is `None` for an entry that is
/// not a pane, such as a settings page. The scene strategy never holds the stack: the app calls
/// [`calculate_scene`](Self::calculate_scene) on every build.
#[derive(Clone, Debug, PartialEq)]
pub struct ThreePaneSceneStrategy {
    /// The pane sizes and partitions.
    pub directive: PaneScaffoldDirective,
    /// How each pane adapts when it has no partition.
    pub adapt_strategies: ThreePaneScaffoldAdaptStrategies,
    /// How back moves through the scene.
    pub back_navigation_behavior: BackNavigationBehavior,
    /// Also make a scene when one pane is shown, instead of leaving each entry to its own page.
    pub handles_single_pane_layout: bool,
}

impl ThreePaneSceneStrategy {
    /// The list-detail strategies, hiding extra panes and popping until the shown panes change.
    pub fn list_detail(directive: PaneScaffoldDirective) -> ThreePaneSceneStrategy {
        ThreePaneSceneStrategy {
            directive,
            adapt_strategies: ThreePaneScaffoldAdaptStrategies::list_detail(),
            back_navigation_behavior: BackNavigationBehavior::PopUntilScaffoldValueChange,
            handles_single_pane_layout: false,
        }
    }

    /// The supporting-pane strategies, otherwise the same.
    pub fn supporting(directive: PaneScaffoldDirective) -> ThreePaneSceneStrategy {
        ThreePaneSceneStrategy {
            directive,
            adapt_strategies: ThreePaneScaffoldAdaptStrategies::supporting(),
            back_navigation_behavior: BackNavigationBehavior::PopUntilScaffoldValueChange,
            handles_single_pane_layout: false,
        }
    }

    /// The trailing pane entries one page shows together.
    ///
    /// `None` means every pane entry is its own page: either the last entry is not a pane, or
    /// the trailing entries show one pane and
    /// [`handles_single_pane_layout`](Self::handles_single_pane_layout) is false.
    pub fn calculate_scene<T: PartialEq>(
        &self,
        back_stack: &[Option<ThreePaneScaffoldDestinationItem<T>>],
    ) -> Option<ThreePaneScaffoldScene> {
        let end = back_stack.len();
        if end == 0 || back_stack[end - 1].is_none() {
            return None;
        }
        let mut start = end - 1;
        while start > 0 && back_stack[start - 1].is_some() {
            start -= 1;
        }

        let items: Vec<&ThreePaneScaffoldDestinationItem<T>> = back_stack[start..end]
            .iter()
            .map(|item| {
                item.as_ref()
                    .expect("the trailing entries are pane entries")
            })
            .collect();
        let value = self.value_of(&items);
        if value.pane_count() == 0 || (value.pane_count() == 1 && !self.handles_single_pane_layout)
        {
            return None;
        }

        let previous_index = self.previous_destination_index(&items, value);
        let (previous_value, back_stack_len_after_back) = if previous_index < 0 {
            (None, start)
        } else {
            let previous_index = previous_index as usize;
            (
                Some(self.value_of(&items[..=previous_index])),
                start + previous_index + 1,
            )
        };

        let mut pane_entries = [None; 3];
        for (index, item) in items.iter().enumerate().rev() {
            let slot = &mut pane_entries[item.pane.index()];
            if slot.is_none() {
                *slot = Some(start + index);
            }
        }

        Some(ThreePaneScaffoldScene {
            directive: self.directive.clone(),
            entries: start..end,
            value,
            previous_value,
            back_stack_len_after_back,
            pane_entries,
        })
    }

    fn value_of<T>(
        &self,
        items: &[&ThreePaneScaffoldDestinationItem<T>],
    ) -> ThreePaneScaffoldValue {
        ThreePaneScaffoldValue::from_pane_history(
            self.directive.max_horizontal_partitions,
            self.directive.max_vertical_partitions,
            &self.adapt_strategies,
            items.iter().map(|item| item.pane),
        )
    }

    /// The scene entry back stays on, or -1 when back leaves the scene.
    fn previous_destination_index<T: PartialEq>(
        &self,
        items: &[&ThreePaneScaffoldDestinationItem<T>],
        current_value: ThreePaneScaffoldValue,
    ) -> isize {
        if items.len() <= 1 {
            return -1;
        }
        let last_index = items.len() - 1;
        let current = items[last_index];
        for index in (0..last_index).rev() {
            match self.back_navigation_behavior {
                BackNavigationBehavior::PopLatest => return last_index as isize - 1,
                BackNavigationBehavior::PopUntilScaffoldValueChange => {
                    if self.value_of(&items[..=index]) != current_value {
                        return index as isize;
                    }
                }
                BackNavigationBehavior::PopUntilCurrentDestinationChange => {
                    if items[index].pane != current.pane {
                        return index as isize;
                    }
                }
                BackNavigationBehavior::PopUntilContentChange => {
                    if items[index].content_key != current.content_key
                        || self.value_of(&items[..=index]) != current_value
                    {
                        return index as isize;
                    }
                }
            }
        }
        -1
    }
}

/// The trailing back-stack entries one page shows together, and what back does to them.
#[derive(Clone, Debug, PartialEq)]
pub struct ThreePaneScaffoldScene {
    /// The directive the scene was computed with.
    pub directive: PaneScaffoldDirective,
    /// The back-stack indices the scene covers.
    pub entries: Range<usize>,
    /// The panes the scene shows.
    pub value: ThreePaneScaffoldValue,
    /// Set when back stays in the scene and only changes which panes show.
    pub previous_value: Option<ThreePaneScaffoldValue>,
    /// How long the back stack is after back.
    pub back_stack_len_after_back: usize,
    pane_entries: [Option<usize>; 3],
}

impl ThreePaneScaffoldScene {
    /// The back-stack index of the latest entry for `role`; `None` when no entry fills it.
    pub fn entry_for(&self, role: ThreePaneScaffoldRole) -> Option<usize> {
        self.pane_entries[role.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::{
        ListDetailPaneScaffoldRole, PaneAdaptedValue, SupportingPaneScaffoldRole,
        WindowHeightSizeClass, WindowSizeClass, WindowWidthSizeClass,
    };

    fn wide_directive() -> PaneScaffoldDirective {
        PaneScaffoldDirective::from_window_size_class(WindowSizeClass {
            width: WindowWidthSizeClass::Expanded,
            height: WindowHeightSizeClass::Expanded,
        })
    }

    fn narrow_directive() -> PaneScaffoldDirective {
        PaneScaffoldDirective::from_window_size_class(WindowSizeClass {
            width: WindowWidthSizeClass::Compact,
            height: WindowHeightSizeClass::Expanded,
        })
    }

    fn item<T>(
        pane: ThreePaneScaffoldRole,
        key: Option<T>,
    ) -> Option<ThreePaneScaffoldDestinationItem<T>> {
        Some(ThreePaneScaffoldDestinationItem::new(pane, key))
    }

    #[test]
    fn a_narrow_stack_leaves_each_entry_to_its_own_page() {
        let strategy = ThreePaneSceneStrategy::list_detail(narrow_directive());
        let stack = vec![
            item(ListDetailPaneScaffoldRole::LIST, None),
            item(ListDetailPaneScaffoldRole::DETAIL, Some(1)),
        ];
        assert_eq!(strategy.calculate_scene(&stack), None);
    }

    #[test]
    fn handling_single_panes_turns_a_narrow_stack_into_a_scene() {
        let mut strategy = ThreePaneSceneStrategy::list_detail(narrow_directive());
        strategy.handles_single_pane_layout = true;
        let stack = vec![item(ListDetailPaneScaffoldRole::DETAIL, Some(1))];
        let scene = strategy.calculate_scene(&stack).expect("a scene");
        assert_eq!(scene.entries, 0..1);
        assert_eq!(scene.value.primary, PaneAdaptedValue::Expanded);
        assert_eq!(scene.previous_value, None);
        assert_eq!(scene.back_stack_len_after_back, 0);
    }

    #[test]
    fn a_wide_list_and_detail_share_one_scene() {
        let strategy = ThreePaneSceneStrategy::list_detail(wide_directive());
        let stack = vec![
            item(ListDetailPaneScaffoldRole::LIST, None),
            item(ListDetailPaneScaffoldRole::DETAIL, Some(7)),
        ];
        let scene = strategy.calculate_scene(&stack).expect("a scene");
        assert_eq!(scene.entries, 0..2);
        assert_eq!(scene.entry_for(ListDetailPaneScaffoldRole::LIST), Some(0));
        assert_eq!(scene.entry_for(ListDetailPaneScaffoldRole::DETAIL), Some(1));
        assert_eq!(scene.previous_value, None);
        assert_eq!(scene.back_stack_len_after_back, 0);
    }

    #[test]
    fn a_non_pane_entry_before_the_scene_stays_its_own_page() {
        let strategy = ThreePaneSceneStrategy::list_detail(wide_directive());
        let stack = vec![
            None,
            item(ListDetailPaneScaffoldRole::LIST, None),
            item(ListDetailPaneScaffoldRole::DETAIL, Some(7)),
        ];
        let scene = strategy.calculate_scene(&stack).expect("a scene");
        assert_eq!(scene.entries, 1..3);
        assert_eq!(scene.entry_for(ListDetailPaneScaffoldRole::LIST), Some(1));
        assert_eq!(scene.back_stack_len_after_back, 1);
    }

    #[test]
    fn a_non_pane_entry_at_the_end_has_no_scene() {
        let strategy = ThreePaneSceneStrategy::list_detail(wide_directive());
        let stack: Vec<Option<ThreePaneScaffoldDestinationItem<()>>> =
            vec![item(ListDetailPaneScaffoldRole::LIST, None), None];
        assert_eq!(strategy.calculate_scene(&stack), None);
    }

    #[test]
    fn the_latest_entry_for_each_pane_wins() {
        let strategy = ThreePaneSceneStrategy::list_detail(wide_directive());
        let stack = vec![
            item(ListDetailPaneScaffoldRole::LIST, None),
            item(ListDetailPaneScaffoldRole::DETAIL, Some(1)),
            item(ListDetailPaneScaffoldRole::DETAIL, Some(2)),
        ];
        let scene = strategy.calculate_scene(&stack).expect("a scene");
        assert_eq!(scene.entry_for(ListDetailPaneScaffoldRole::DETAIL), Some(2));
    }

    #[test]
    fn a_missing_pane_has_no_entry() {
        let strategy = ThreePaneSceneStrategy::list_detail(wide_directive());
        let stack = vec![
            item(ListDetailPaneScaffoldRole::LIST, None),
            item(ListDetailPaneScaffoldRole::DETAIL, Some(7)),
        ];
        let scene = strategy.calculate_scene(&stack).expect("a scene");
        assert_eq!(scene.entry_for(ListDetailPaneScaffoldRole::EXTRA), None);
    }

    #[test]
    fn popping_latest_leaves_the_entry_before_the_latest() {
        let mut strategy = ThreePaneSceneStrategy::list_detail(wide_directive());
        strategy.back_navigation_behavior = BackNavigationBehavior::PopLatest;
        let stack = vec![
            item(ListDetailPaneScaffoldRole::LIST, None),
            item(ListDetailPaneScaffoldRole::DETAIL, Some(1)),
            item(ListDetailPaneScaffoldRole::EXTRA, Some(2)),
        ];
        let scene = strategy.calculate_scene(&stack).expect("a scene");
        assert_eq!(scene.back_stack_len_after_back, 2);
    }

    #[test]
    fn popping_until_the_current_destination_changes_skips_same_pane_entries() {
        let mut strategy = ThreePaneSceneStrategy::list_detail(wide_directive());
        strategy.back_navigation_behavior =
            BackNavigationBehavior::PopUntilCurrentDestinationChange;
        let stack = vec![
            item(ListDetailPaneScaffoldRole::LIST, None),
            item(ListDetailPaneScaffoldRole::DETAIL, Some(1)),
            item(ListDetailPaneScaffoldRole::DETAIL, Some(2)),
        ];
        let scene = strategy.calculate_scene(&stack).expect("a scene");
        assert_eq!(scene.back_stack_len_after_back, 1);
    }

    #[test]
    fn popping_until_the_content_changes_skips_the_same_content() {
        let mut strategy = ThreePaneSceneStrategy::list_detail(wide_directive());
        strategy.back_navigation_behavior = BackNavigationBehavior::PopUntilContentChange;
        let stack = vec![
            item(ListDetailPaneScaffoldRole::LIST, None),
            item(ListDetailPaneScaffoldRole::DETAIL, Some(1)),
            item(ListDetailPaneScaffoldRole::DETAIL, Some(1)),
        ];
        let scene = strategy.calculate_scene(&stack).expect("a scene");
        assert_eq!(scene.back_stack_len_after_back, 1);
    }

    #[test]
    fn an_extra_pane_takes_a_partition_on_a_wide_window() {
        let strategy = ThreePaneSceneStrategy::list_detail(wide_directive());
        let stack = vec![
            item(ListDetailPaneScaffoldRole::LIST, None),
            item(ListDetailPaneScaffoldRole::DETAIL, Some(1)),
            item(ListDetailPaneScaffoldRole::EXTRA, Some(2)),
        ];
        let scene = strategy.calculate_scene(&stack).expect("a scene");
        assert_eq!(scene.value.pane_count(), 2);
        assert_eq!(scene.entry_for(ListDetailPaneScaffoldRole::LIST), Some(0));
    }

    #[test]
    fn a_supporting_scene_reflows_on_a_tall_single_column() {
        let strategy = ThreePaneSceneStrategy::supporting(narrow_directive());
        let stack = vec![
            item(SupportingPaneScaffoldRole::MAIN, None),
            item(SupportingPaneScaffoldRole::SUPPORTING, Some(1)),
        ];
        // A reflowed pane counts as shown, so the supporting pane makes a scene even on one
        // column.
        let scene = strategy.calculate_scene(&stack).expect("a scene");
        assert_eq!(
            scene.value.get(SupportingPaneScaffoldRole::SUPPORTING),
            PaneAdaptedValue::Reflowed {
                reflow_under: SupportingPaneScaffoldRole::MAIN
            }
        );
    }
}
