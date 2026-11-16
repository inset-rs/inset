//! `CommandBar`'s dynamic overflow (`CommandBar_Partial.cpp`, `MeasureOverrideForDynamicOverflow` and the functions it calls): which primary commands the bar shows and which it moves into the overflow menu as its width shrinks, and when it moves them back.
//!
//! The model holds WinUI's collections by index: `dynamic_primary` the primary commands still in the bar, `dynamic_secondary` what the overflow menu lists, the moved primary commands first, then the separator the bar inserts, then the secondary commands. A later editor must keep `secondary_command_start_index` the count of entries before the first secondary command: every insertion and removal below moves it, as in WinUI.

use super::CommandBarDynamicOverflowAction;

/// An entry of the overflow menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum OverflowEntry {
    /// A primary command, by its index in `PrimaryCommands`, moved into the overflow.
    Primary(usize),
    /// `m_tpAppBarSeparatorInOverflow`: the separator between moved and secondary commands.
    Separator,
    /// A secondary command, by its index in `SecondaryCommands`.
    Secondary(usize),
}

/// What the model reads of one primary command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PrimaryCommand {
    pub(crate) is_separator: bool,
    /// `DynamicOverflowOrder`.
    pub(crate) order: i32,
}

/// The widths of one layout pass that `MeasureOverrideForDynamicOverflow` compares.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OverflowMeasure {
    /// The bar's available width.
    pub(crate) available: f64,
    /// `availablePrimarySize.Width`: the primary commands' desired width after the bar's own measure, which is at most their column's width.
    pub(crate) available_primary: f64,
    /// `contentRootDesiredSize.Width`: the bar's content measured without a width limit.
    pub(crate) content_root_desired: f64,
    /// `primaryDesiredSize.Width`: the primary commands' natural width.
    pub(crate) primary_desired: f64,
}

/// `CommandBar`'s dynamic overflow fields: `m_tpDynamicPrimaryCommands`, `m_tpDynamicSecondaryCommands`, the two transition collections and the counters beside them.
#[derive(Debug)]
pub(crate) struct DynamicOverflow {
    primary: Vec<PrimaryCommand>,
    secondary_count: usize,
    pub(crate) dynamic_primary: Vec<usize>,
    pub(crate) dynamic_secondary: Vec<OverflowEntry>,
    primary_commands_in_transition: Vec<usize>,
    primary_commands_in_previous_transition: Vec<OverflowEntry>,
    secondary_command_start_index: usize,
    has_app_bar_separator_in_overflow: bool,
    restorable_primary_command_min_width: f64,
    has_already_fired_overflow_changing_event: bool,
    last_available_width: f64,
    /// `DynamicOverflowItemsChanging` events raised during layout, for the bar to deliver after it.
    pub(crate) raised: Vec<CommandBarDynamicOverflowAction>,
}

impl DynamicOverflow {
    pub(crate) fn new(primary: Vec<PrimaryCommand>, secondary_count: usize) -> Self {
        let mut overflow = Self {
            primary: Vec::new(),
            secondary_count: 0,
            dynamic_primary: Vec::new(),
            dynamic_secondary: Vec::new(),
            primary_commands_in_transition: Vec::new(),
            primary_commands_in_previous_transition: Vec::new(),
            secondary_command_start_index: 0,
            has_app_bar_separator_in_overflow: false,
            restorable_primary_command_min_width: 0.0,
            has_already_fired_overflow_changing_event: false,
            last_available_width: 0.0,
            raised: Vec::new(),
        };
        overflow.set_commands(primary, secondary_count);
        overflow
    }

    /// `OnPrimaryCommandsChanged` and `OnSecondaryCommandsChanged` with a `Reset`: new commands start over in the bar.
    pub(crate) fn set_commands(&mut self, primary: Vec<PrimaryCommand>, secondary_count: usize) {
        self.primary = primary;
        self.secondary_count = secondary_count;
        self.reset_dynamic_commands();
    }

    /// Whether the primary command `index` is in the overflow.
    pub(crate) fn is_moved(&self, index: usize) -> bool {
        self.dynamic_secondary
            .contains(&OverflowEntry::Primary(index))
    }

    /// Which primary commands are separators, by index.
    pub(crate) fn separators(&self) -> Vec<bool> {
        self.primary
            .iter()
            .map(|command| command.is_separator)
            .collect()
    }

    fn is_separator(&self, index: usize) -> bool {
        self.primary[index].is_separator
    }

    fn order(&self, entry: OverflowEntry) -> i32 {
        match entry {
            OverflowEntry::Primary(index) => self.primary[index].order,
            // The bar's own separator has the default order; a secondary command never moves back.
            OverflowEntry::Separator | OverflowEntry::Secondary(_) => 0,
        }
    }

    /// `MeasureOverrideForDynamicOverflow` after the bar's own measure, with `width` the desired width of a primary command. Answers whether the collections changed, after which WinUI measures again.
    pub(crate) fn measure(
        &mut self,
        measure: OverflowMeasure,
        width: &dyn Fn(usize) -> f64,
    ) -> bool {
        let before = (self.dynamic_primary.clone(), self.dynamic_secondary.clone());
        if measure.content_root_desired > measure.available {
            if !self.dynamic_primary.is_empty() {
                let mut count = self.find_movable_primary_commands(
                    measure.available_primary,
                    measure.primary_desired,
                    width,
                );
                if count > 0 {
                    self.trim_primary_command_separator_in_overflow(&mut count);
                    if count > 0 {
                        if !self.has_already_fired_overflow_changing_event {
                            self.fire_dynamic_overflow_items_changing_event(false);
                        }
                        self.update_primary_command_element_min_width_in_overflow(width);
                        self.move_transition_primary_commands_into_overflow(count);
                        self.save_moved_primary_commands_into_previous_transition_collection();
                    }
                }
            }
        } else if self.last_available_width < measure.available
            && self.secondary_command_start_index > 0
            && self.restorable_primary_command_min_width >= 0.0
        {
            let mut restorable_primary_command_min_width =
                self.restorable_primary_command_min_width;
            let available_width_to_restore = measure.available - measure.content_root_desired;
            let restorable_min_count = self.get_restorable_primary_commands_minimum_count();
            if restorable_min_count > 0 {
                restorable_primary_command_min_width *= restorable_min_count as f64;
            }
            if available_width_to_restore > restorable_primary_command_min_width {
                self.fire_dynamic_overflow_items_changing_event(true);
                self.has_already_fired_overflow_changing_event = true;
                self.reset_dynamic_commands();
                self.save_moved_primary_commands_into_previous_transition_collection();
            }
        }
        self.has_already_fired_overflow_changing_event = false;
        self.last_available_width = measure.available;
        before != (self.dynamic_primary.clone(), self.dynamic_secondary.clone())
    }

    /// `FindMovablePrimaryCommands`: the commands to move, by `DynamicOverflowOrder` when any has one, else the rightmost that leave the rest fitting. Answers how many.
    fn find_movable_primary_commands(
        &mut self,
        available_primary_commands_width: f64,
        mut primary_items_control_desired_width: f64,
        width: &dyn Fn(usize) -> f64,
    ) -> usize {
        let mut count = 0;
        if self.find_movable_primary_commands_from_order_set(
            available_primary_commands_width,
            primary_items_control_desired_width,
            &mut count,
            width,
        ) {
            return count;
        }
        let dynamic_primary_count = self.dynamic_primary.len();
        let mut move_start_index = dynamic_primary_count - 1;
        count = 0;
        let mut i = move_start_index;
        while i > 0 {
            primary_items_control_desired_width -= width(self.dynamic_primary[i]);
            if primary_items_control_desired_width < available_primary_commands_width {
                break;
            }
            move_start_index -= 1;
            i -= 1;
        }
        self.primary_commands_in_transition.clear();
        for i in move_start_index..dynamic_primary_count {
            self.primary_commands_in_transition
                .push(self.dynamic_primary[i]);
            count += 1;
        }
        count
    }

    /// `FindMovablePrimaryCommandsFromOrderSet`: order set by order set, lowest first, each command with the separators it would strand, then the unordered ones from the right. Answers `canProcessDynamicOverflowOrder`.
    fn find_movable_primary_commands_from_order_set(
        &mut self,
        available_primary_commands_width: f64,
        mut primary_items_control_desired_width: f64,
        count: &mut usize,
        width: &dyn Fn(usize) -> f64,
    ) -> bool {
        let mut can_process_dynamic_overflow_order = false;
        let mut first_movable_order = 0;
        let mut previous_first_movable_order = 0;
        let mut dynamic_primary_count;
        loop {
            let mut is_set_movable_candidate_order = false;
            let mut should_find_movable_order_set = false;
            dynamic_primary_count = self.dynamic_primary.len();
            for i in 0..dynamic_primary_count {
                let order = self.primary[self.dynamic_primary[i]].order;
                if order > 0 && order > previous_first_movable_order {
                    if !is_set_movable_candidate_order {
                        first_movable_order = order;
                        is_set_movable_candidate_order = true;
                    } else {
                        first_movable_order = order.min(first_movable_order);
                    }
                    should_find_movable_order_set = true;
                }
            }
            if should_find_movable_order_set && first_movable_order > previous_first_movable_order {
                if !can_process_dynamic_overflow_order {
                    can_process_dynamic_overflow_order = true;
                    self.primary_commands_in_transition.clear();
                }
                for i in 0..dynamic_primary_count {
                    let order = self.primary[self.dynamic_primary[i]].order;
                    if order > 0 && order == first_movable_order {
                        self.insert_primary_command_to_primary_commands_in_transition(
                            i,
                            count,
                            &mut primary_items_control_desired_width,
                            width,
                        );
                        if i > 0 {
                            self.find_movable_separators_in_backward_direction(
                                i,
                                count,
                                &mut primary_items_control_desired_width,
                                width,
                            );
                        }
                        if i < dynamic_primary_count - 1 {
                            self.find_movable_separators_in_forward_direction(
                                i,
                                count,
                                &mut primary_items_control_desired_width,
                                width,
                            );
                        }
                        if primary_items_control_desired_width < available_primary_commands_width {
                            should_find_movable_order_set = false;
                        }
                    }
                }
            }
            previous_first_movable_order = first_movable_order;
            if !should_find_movable_order_set {
                break;
            }
        }
        if can_process_dynamic_overflow_order
            && primary_items_control_desired_width > available_primary_commands_width
        {
            for i in (1..=dynamic_primary_count).rev() {
                if self.primary[self.dynamic_primary[i - 1]].order == 0 {
                    self.insert_primary_command_to_primary_commands_in_transition(
                        i - 1,
                        count,
                        &mut primary_items_control_desired_width,
                        width,
                    );
                    if primary_items_control_desired_width < available_primary_commands_width {
                        break;
                    }
                }
            }
        }
        can_process_dynamic_overflow_order
    }

    /// `InsertPrimaryCommandToPrimaryCommandsInTransition`.
    fn insert_primary_command_to_primary_commands_in_transition(
        &mut self,
        index_moving_primary_command: usize,
        count: &mut usize,
        primary_items_control_desired_width: &mut f64,
        width: &dyn Fn(usize) -> f64,
    ) {
        let element = self.dynamic_primary[index_moving_primary_command];
        self.primary_commands_in_transition.push(element);
        *count += 1;
        *primary_items_control_desired_width -= width(element);
    }

    /// `InsertSeparatorToPrimaryCommandsInTransition`: a separator not yet moving.
    fn insert_separator_to_primary_commands_in_transition(
        &mut self,
        index_moving_separator: usize,
        count: &mut usize,
        primary_items_control_desired_width: &mut f64,
        width: &dyn Fn(usize) -> f64,
    ) {
        let element = self.dynamic_primary[index_moving_separator];
        if self.is_separator(element) && !self.primary_commands_in_transition.contains(&element) {
            self.primary_commands_in_transition.push(element);
            *count += 1;
            *primary_items_control_desired_width -= width(element);
        }
    }

    /// `IsAppBarSeparatorInDynamicPrimaryCommands`.
    fn is_app_bar_separator_in_dynamic_primary_commands(&self, index: usize) -> bool {
        self.dynamic_primary
            .get(index)
            .is_some_and(|element| self.is_separator(*element))
    }

    /// `FindNonSeparatorInDynamicPrimaryCommands`: the first command that is not a separator from `index_moving` in the direction, if any.
    fn find_non_separator_in_dynamic_primary_commands(
        &self,
        is_forward: bool,
        mut index_moving: isize,
    ) -> Option<isize> {
        let count = self.dynamic_primary.len() as isize;
        while if is_forward {
            index_moving < count
        } else {
            index_moving >= 0
        } {
            if !self.is_separator(self.dynamic_primary[index_moving as usize]) {
                return Some(index_moving);
            }
            index_moving += if is_forward { 1 } else { -1 };
        }
        None
    }

    /// `FindMovableSeparatorsInBackwardDirection`.
    fn find_movable_separators_in_backward_direction(
        &mut self,
        moving_primary_command_index: usize,
        count: &mut usize,
        primary_items_control_desired_width: &mut f64,
        width: &dyn Fn(usize) -> f64,
    ) {
        if moving_primary_command_index == 0
            || !self
                .is_app_bar_separator_in_dynamic_primary_commands(moving_primary_command_index - 1)
        {
            return;
        }
        let mut index_moving_backward = moving_primary_command_index as isize - 1;
        match self.find_non_separator_in_dynamic_primary_commands(false, index_moving_backward) {
            None => {
                while index_moving_backward >= 0 {
                    self.insert_separator_to_primary_commands_in_transition(
                        index_moving_backward as usize,
                        count,
                        primary_items_control_desired_width,
                        width,
                    );
                    index_moving_backward -= 1;
                }
            }
            Some(index_non_separator) => {
                while index_moving_backward > index_non_separator
                    && index_moving_backward - index_non_separator > 1
                {
                    self.insert_separator_to_primary_commands_in_transition(
                        index_moving_backward as usize,
                        count,
                        primary_items_control_desired_width,
                        width,
                    );
                    index_moving_backward -= 1;
                }
            }
        }
    }

    /// `FindMovableSeparatorsInForwardDirection`.
    fn find_movable_separators_in_forward_direction(
        &mut self,
        moving_primary_command_index: usize,
        count: &mut usize,
        primary_items_control_desired_width: &mut f64,
        width: &dyn Fn(usize) -> f64,
    ) {
        if !self.is_app_bar_separator_in_dynamic_primary_commands(moving_primary_command_index + 1)
        {
            return;
        }
        let mut index_moving_forward = moving_primary_command_index as isize + 1;
        let non_separator =
            self.find_non_separator_in_dynamic_primary_commands(true, index_moving_forward);
        let dynamic_primary_count = self.dynamic_primary.len() as isize;
        match non_separator {
            None => {
                while index_moving_forward < dynamic_primary_count {
                    self.insert_separator_to_primary_commands_in_transition(
                        index_moving_forward as usize,
                        count,
                        primary_items_control_desired_width,
                        width,
                    );
                    index_moving_forward += 1;
                }
            }
            Some(index_non_separator) => {
                while index_moving_forward < index_non_separator
                    && index_non_separator - index_moving_forward > 1
                {
                    self.insert_separator_to_primary_commands_in_transition(
                        index_moving_forward as usize,
                        count,
                        primary_items_control_desired_width,
                        width,
                    );
                    index_moving_forward += 1;
                }
            }
        }
        // The separator after a command moving from the start goes with it.
        if moving_primary_command_index == 0 {
            self.insert_separator_to_primary_commands_in_transition(
                moving_primary_command_index + 1,
                count,
                primary_items_control_desired_width,
                width,
            );
        }
        if moving_primary_command_index > 0
            && self
                .is_app_bar_separator_in_dynamic_primary_commands(moving_primary_command_index - 1)
        {
            self.insert_separator_to_primary_commands_in_transition(
                moving_primary_command_index + 1,
                count,
                primary_items_control_desired_width,
                width,
            );
            if non_separator.is_none() {
                self.insert_separator_to_primary_commands_in_transition(
                    moving_primary_command_index - 1,
                    count,
                    primary_items_control_desired_width,
                    width,
                );
            }
        }
    }

    /// `TrimPrimaryCommandSeparatorInOverflow`: separators never move into the overflow; the moving ones leave the bar.
    fn trim_primary_command_separator_in_overflow(&mut self, count: &mut usize) {
        for i in (1..=*count).rev() {
            let element = self.primary_commands_in_transition[i - 1];
            if self.is_separator(element) {
                if let Some(index) = self.dynamic_primary.iter().position(|e| *e == element) {
                    self.dynamic_primary.remove(index);
                }
                self.primary_commands_in_transition.remove(i - 1);
                *count -= 1;
            }
        }
    }

    /// `UpdatePrimaryCommandElementMinWidthInOverflow`: the narrowest moving command, the width that must free up before one comes back.
    fn update_primary_command_element_min_width_in_overflow(
        &mut self,
        width: &dyn Fn(usize) -> f64,
    ) {
        self.restorable_primary_command_min_width = -1.0;
        for &element in &self.primary_commands_in_transition {
            if self.primary[element].is_separator {
                continue;
            }
            let desired = width(element);
            self.restorable_primary_command_min_width =
                if self.restorable_primary_command_min_width == -1.0 {
                    desired
                } else {
                    self.restorable_primary_command_min_width.min(desired)
                };
        }
    }

    /// `MoveTransitionPrimaryCommandsIntoOverflow`.
    fn move_transition_primary_commands_into_overflow(&mut self, count: usize) {
        if self.secondary_command_start_index == 0 && !self.dynamic_secondary.is_empty() {
            self.dynamic_secondary
                .insert(self.secondary_command_start_index, OverflowEntry::Separator);
            self.secondary_command_start_index += 1;
            self.has_app_bar_separator_in_overflow = true;
        }
        for _ in 0..count {
            let element = self.primary_commands_in_transition.remove(0);
            if let Some(index) = self.dynamic_primary.iter().position(|e| *e == element) {
                self.dynamic_primary.remove(index);
            }
            self.insert_transition_primary_command_into_overflow(element);
        }
    }

    /// `InsertTransitionPrimaryCommandIntoOverflow`: moved commands keep their order in `PrimaryCommands`.
    fn insert_transition_primary_command_into_overflow(&mut self, element: usize) {
        let index_for_moved_primary_command = if self.has_app_bar_separator_in_overflow {
            self.secondary_command_start_index - 1
        } else {
            self.secondary_command_start_index
        };
        for i in 0..index_for_moved_primary_command {
            if let OverflowEntry::Primary(existing) = self.dynamic_secondary[i]
                && element < existing
            {
                self.dynamic_secondary
                    .insert(i, OverflowEntry::Primary(element));
                self.secondary_command_start_index += 1;
                return;
            }
        }
        self.dynamic_secondary.insert(
            index_for_moved_primary_command,
            OverflowEntry::Primary(element),
        );
        self.secondary_command_start_index += 1;
    }

    /// `ResetDynamicCommands`: every primary command back in the bar, the overflow only the secondary commands.
    pub(crate) fn reset_dynamic_commands(&mut self) {
        self.secondary_command_start_index = 0;
        self.has_app_bar_separator_in_overflow = false;
        self.dynamic_primary = (0..self.primary.len()).collect();
        self.dynamic_secondary = (0..self.secondary_count)
            .map(OverflowEntry::Secondary)
            .collect();
    }

    /// `SaveMovedPrimaryCommandsIntoPreviousTransitionCollection`: what is in the overflow before the secondary commands, the separator included.
    fn save_moved_primary_commands_into_previous_transition_collection(&mut self) {
        let primary_count_in_overflow = self.dynamic_secondary.len() - self.secondary_count;
        self.primary_commands_in_previous_transition =
            self.dynamic_secondary[..primary_count_in_overflow].to_vec();
    }

    /// `FireDynamicOverflowItemsChangingEvent`: adding when a command moving now was not in the overflow before, removing otherwise and on a restore.
    fn fire_dynamic_overflow_items_changing_event(&mut self, is_force_to_restore: bool) {
        let mut is_adding = false;
        if !is_force_to_restore {
            let same_primary_count = self
                .primary_commands_in_previous_transition
                .iter()
                .map(|previous| {
                    self.primary_commands_in_transition
                        .iter()
                        .filter(|current| OverflowEntry::Primary(**current) == *previous)
                        .count()
                })
                .sum::<usize>();
            is_adding = self.primary_commands_in_transition.len() > same_primary_count;
        }
        self.raised.push(if is_adding {
            CommandBarDynamicOverflowAction::AddingToOverflow
        } else {
            CommandBarDynamicOverflowAction::RemovingFromOverflow
        });
    }

    /// `GetRestorablePrimaryCommandsMinimumCount`: how many commands come back together, those of the highest order among the moved ones but the last entry before the secondary commands.
    fn get_restorable_primary_commands_minimum_count(&self) -> usize {
        if self.secondary_command_start_index <= 1 {
            return 0;
        }
        let moved = &self.dynamic_secondary[..self.secondary_command_start_index - 1];
        let first_restorable_order = moved
            .iter()
            .map(|entry| self.order(*entry))
            .filter(|order| *order > 0)
            .max()
            .unwrap_or(0);
        if first_restorable_order > 0 {
            moved
                .iter()
                .filter(|entry| self.order(**entry) == first_restorable_order)
                .count()
        } else {
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commands(orders: &[i32], separators: &[usize]) -> Vec<PrimaryCommand> {
        orders
            .iter()
            .enumerate()
            .map(|(index, order)| PrimaryCommand {
                is_separator: separators.contains(&index),
                order: *order,
            })
            .collect()
    }

    /// One layout pass of a bar whose commands are `widths` wide, with nothing beside them.
    fn pass(model: &mut DynamicOverflow, widths: &[f64], available: f64) -> bool {
        let shown: f64 = model.dynamic_primary.iter().map(|i| widths[*i]).sum();
        model.measure(
            OverflowMeasure {
                available,
                available_primary: shown.min(available),
                content_root_desired: shown,
                primary_desired: shown,
            },
            &|index| widths[index],
        )
    }

    #[test]
    fn the_rightmost_commands_move_first() {
        let mut model = DynamicOverflow::new(commands(&[0, 0, 0, 0], &[]), 1);
        assert!(pass(&mut model, &[68.0; 4], 210.0));
        assert_eq!(model.dynamic_primary, vec![0, 1, 2]);
        assert_eq!(
            model.dynamic_secondary,
            vec![
                OverflowEntry::Primary(3),
                OverflowEntry::Separator,
                OverflowEntry::Secondary(0)
            ]
        );
        assert_eq!(
            model.raised,
            vec![CommandBarDynamicOverflowAction::AddingToOverflow]
        );
    }

    #[test]
    fn the_lowest_order_moves_first() {
        let mut model = DynamicOverflow::new(commands(&[2, 1, 0, 0], &[]), 0);
        assert!(pass(&mut model, &[68.0; 4], 210.0));
        assert_eq!(model.dynamic_primary, vec![0, 2, 3]);
        assert!(pass(&mut model, &[68.0; 4], 140.0));
        assert_eq!(model.dynamic_primary, vec![2, 3]);
        assert_eq!(
            model.dynamic_secondary,
            vec![OverflowEntry::Primary(0), OverflowEntry::Primary(1)]
        );
    }

    #[test]
    fn a_stranded_separator_leaves_the_bar() {
        let mut model = DynamicOverflow::new(commands(&[0, 1, 0, 0], &[0]), 0);
        let widths = [5.0, 68.0, 68.0, 68.0];
        assert!(pass(&mut model, &widths, 150.0));
        assert_eq!(model.dynamic_primary, vec![2, 3]);
        assert_eq!(model.dynamic_secondary, vec![OverflowEntry::Primary(1)]);
    }

    #[test]
    fn commands_come_back_when_their_width_frees_up() {
        let mut model = DynamicOverflow::new(commands(&[0, 0, 0], &[]), 0);
        let widths = [68.0; 3];
        assert!(pass(&mut model, &widths, 150.0));
        assert_eq!(model.dynamic_primary, vec![0, 1]);
        // Not enough room for the narrowest moved command.
        assert!(!pass(&mut model, &widths, 190.0));
        model.raised.clear();
        assert!(pass(&mut model, &widths, 210.0));
        assert_eq!(model.dynamic_primary, vec![0, 1, 2]);
        assert_eq!(
            model.raised,
            vec![CommandBarDynamicOverflowAction::RemovingFromOverflow]
        );
    }
}
