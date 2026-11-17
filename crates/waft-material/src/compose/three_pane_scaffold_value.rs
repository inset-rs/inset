//! Which panes the current window shows, and why.

use crate::compose::{
    AdaptStrategy, PaneAdaptedValue, ThreePaneScaffoldAdaptStrategies, ThreePaneScaffoldRole,
};

/// One back-stack entry as the pane scaffold sees it: the pane it fills, and what it shows there.
#[derive(Clone, Debug, PartialEq)]
pub struct ThreePaneScaffoldDestinationItem<T> {
    /// The pane the entry fills.
    pub pane: ThreePaneScaffoldRole,
    /// The entry's content key, if it has one.
    pub content_key: Option<T>,
}

impl<T> ThreePaneScaffoldDestinationItem<T> {
    /// Creates a destination item for `pane` showing `content_key`.
    pub fn new(
        pane: ThreePaneScaffoldRole,
        content_key: Option<T>,
    ) -> ThreePaneScaffoldDestinationItem<T> {
        ThreePaneScaffoldDestinationItem { pane, content_key }
    }
}

/// Which panes the current window shows.
///
/// It is computed, never stored: two values that differ are two layouts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThreePaneScaffoldValue {
    /// The primary pane's value.
    pub primary: PaneAdaptedValue,
    /// The secondary pane's value.
    pub secondary: PaneAdaptedValue,
    /// The tertiary pane's value.
    pub tertiary: PaneAdaptedValue,
}

impl ThreePaneScaffoldValue {
    /// The value for `role`.
    pub fn get(&self, role: ThreePaneScaffoldRole) -> PaneAdaptedValue {
        match role {
            ThreePaneScaffoldRole::Primary => self.primary,
            ThreePaneScaffoldRole::Secondary => self.secondary,
            ThreePaneScaffoldRole::Tertiary => self.tertiary,
        }
    }

    /// The number of panes shown, reflowed panes included.
    pub fn pane_count(&self) -> usize {
        [self.primary, self.secondary, self.tertiary]
            .into_iter()
            .filter(|value| *value != PaneAdaptedValue::Hidden)
            .count()
    }

    /// The latest destinations take the partitions first, then Primary, Secondary, Tertiary.
    ///
    /// Compose's `calculateThreePaneScaffoldValue` without levitated panes: a pane with a
    /// reflow strategy goes under its anchor only on a single column with a second vertical
    /// partition.
    pub fn calculate<T>(
        max_horizontal_partitions: usize,
        max_vertical_partitions: usize,
        adapt_strategies: &ThreePaneScaffoldAdaptStrategies,
        destination_history: &[ThreePaneScaffoldDestinationItem<T>],
    ) -> ThreePaneScaffoldValue {
        ThreePaneScaffoldValue::from_pane_history(
            max_horizontal_partitions,
            max_vertical_partitions,
            adapt_strategies,
            destination_history.iter().map(|item| item.pane),
        )
    }

    /// [`calculate`](Self::calculate) over the panes of the destinations, oldest first.
    pub(crate) fn from_pane_history(
        max_horizontal_partitions: usize,
        max_vertical_partitions: usize,
        adapt_strategies: &ThreePaneScaffoldAdaptStrategies,
        pane_history: impl DoubleEndedIterator<Item = ThreePaneScaffoldRole>,
    ) -> ThreePaneScaffoldValue {
        let mut values = [None; 3];
        let mut expanded_count = 0;
        let mut check_reflowed_pane = max_horizontal_partitions == 1
            && max_vertical_partitions > 1
            && ThreePaneScaffoldRole::ALL
                .into_iter()
                .any(|role| matches!(adapt_strategies.get(role), AdaptStrategy::Reflow { .. }));

        let panes_by_priority = pane_history.rev().chain(ThreePaneScaffoldRole::ALL);
        for pane in panes_by_priority {
            let has_available_partition = expanded_count < max_horizontal_partitions;
            if !has_available_partition && !check_reflowed_pane {
                break;
            }
            if values[pane.index()].is_some() {
                continue;
            }

            let mut reflowed_pane = None;
            let mut anchor_pane = pane;
            let mut anchor_value = None;
            if check_reflowed_pane
                && let AdaptStrategy::Reflow { reflow_under } = adapt_strategies.get(pane)
            {
                reflowed_pane = Some(pane);
                anchor_pane = reflow_under;
                anchor_value = values[anchor_pane.index()];
            }

            match anchor_value {
                None => {
                    if has_available_partition {
                        values[anchor_pane.index()] = Some(PaneAdaptedValue::Expanded);
                        expanded_count += 1;
                    } else {
                        continue;
                    }
                }
                Some(PaneAdaptedValue::Expanded) => {}
                Some(_) => continue,
            }

            if let Some(pane) = reflowed_pane {
                values[pane.index()] = Some(PaneAdaptedValue::Reflowed {
                    reflow_under: anchor_pane,
                });
                check_reflowed_pane = false;
            }
        }

        ThreePaneScaffoldValue {
            primary: values[0].unwrap_or(PaneAdaptedValue::Hidden),
            secondary: values[1].unwrap_or(PaneAdaptedValue::Hidden),
            tertiary: values[2].unwrap_or(PaneAdaptedValue::Hidden),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn destination(pane: ThreePaneScaffoldRole) -> ThreePaneScaffoldDestinationItem<()> {
        ThreePaneScaffoldDestinationItem::new(pane, None)
    }

    fn strategies(
        primary: AdaptStrategy,
        secondary: AdaptStrategy,
        tertiary: AdaptStrategy,
    ) -> ThreePaneScaffoldAdaptStrategies {
        ThreePaneScaffoldAdaptStrategies {
            primary,
            secondary,
            tertiary,
        }
    }

    #[test]
    fn one_partition_without_history_expands_primary() {
        let value = ThreePaneScaffoldValue::calculate(
            1,
            1,
            &strategies(
                AdaptStrategy::Hide,
                AdaptStrategy::Hide,
                AdaptStrategy::Hide,
            ),
            &[] as &[ThreePaneScaffoldDestinationItem<()>],
        );
        assert_eq!(value.primary, PaneAdaptedValue::Expanded);
        assert_eq!(value.secondary, PaneAdaptedValue::Hidden);
        assert_eq!(value.tertiary, PaneAdaptedValue::Hidden);
    }

    #[test]
    fn the_latest_destination_takes_the_partition() {
        let value = ThreePaneScaffoldValue::calculate(
            1,
            1,
            &strategies(
                AdaptStrategy::Hide,
                AdaptStrategy::Hide,
                AdaptStrategy::Hide,
            ),
            &[
                destination(ThreePaneScaffoldRole::Tertiary),
                destination(ThreePaneScaffoldRole::Secondary),
            ],
        );
        assert_eq!(value.primary, PaneAdaptedValue::Hidden);
        assert_eq!(value.secondary, PaneAdaptedValue::Expanded);
        assert_eq!(value.tertiary, PaneAdaptedValue::Hidden);
    }

    #[test]
    fn two_partitions_expand_primary_and_secondary() {
        let value = ThreePaneScaffoldValue::calculate(
            2,
            1,
            &strategies(
                AdaptStrategy::Hide,
                AdaptStrategy::Hide,
                AdaptStrategy::Hide,
            ),
            &[] as &[ThreePaneScaffoldDestinationItem<()>],
        );
        assert_eq!(value.primary, PaneAdaptedValue::Expanded);
        assert_eq!(value.secondary, PaneAdaptedValue::Expanded);
        assert_eq!(value.tertiary, PaneAdaptedValue::Hidden);
    }

    #[test]
    fn three_partitions_expand_every_pane() {
        let value = ThreePaneScaffoldValue::calculate(
            3,
            1,
            &strategies(
                AdaptStrategy::Hide,
                AdaptStrategy::Hide,
                AdaptStrategy::Hide,
            ),
            &[] as &[ThreePaneScaffoldDestinationItem<()>],
        );
        assert_eq!(value.pane_count(), 3);
    }

    #[test]
    fn a_reflowed_pane_stacks_under_its_anchor_on_a_tall_single_column() {
        let value = ThreePaneScaffoldValue::calculate(
            1,
            2,
            &strategies(
                AdaptStrategy::Hide,
                AdaptStrategy::Reflow {
                    reflow_under: ThreePaneScaffoldRole::Tertiary,
                },
                AdaptStrategy::Hide,
            ),
            &[destination(ThreePaneScaffoldRole::Secondary)],
        );
        assert_eq!(value.primary, PaneAdaptedValue::Hidden);
        assert_eq!(
            value.secondary,
            PaneAdaptedValue::Reflowed {
                reflow_under: ThreePaneScaffoldRole::Tertiary
            }
        );
        assert_eq!(value.tertiary, PaneAdaptedValue::Expanded);
    }

    #[test]
    fn an_expanded_anchor_takes_the_reflowed_pane() {
        let value = ThreePaneScaffoldValue::calculate(
            1,
            2,
            &strategies(
                AdaptStrategy::Hide,
                AdaptStrategy::Hide,
                AdaptStrategy::Reflow {
                    reflow_under: ThreePaneScaffoldRole::Primary,
                },
            ),
            &[
                destination(ThreePaneScaffoldRole::Tertiary),
                destination(ThreePaneScaffoldRole::Secondary),
                destination(ThreePaneScaffoldRole::Primary),
            ],
        );
        assert_eq!(value.primary, PaneAdaptedValue::Expanded);
        assert_eq!(value.secondary, PaneAdaptedValue::Hidden);
        assert_eq!(
            value.tertiary,
            PaneAdaptedValue::Reflowed {
                reflow_under: ThreePaneScaffoldRole::Primary
            }
        );
    }

    #[test]
    fn a_reflow_needs_a_second_vertical_partition() {
        let value = ThreePaneScaffoldValue::calculate(
            1,
            1,
            &strategies(
                AdaptStrategy::Reflow {
                    reflow_under: ThreePaneScaffoldRole::Tertiary,
                },
                AdaptStrategy::Hide,
                AdaptStrategy::Hide,
            ),
            &[] as &[ThreePaneScaffoldDestinationItem<()>],
        );
        assert_eq!(value.primary, PaneAdaptedValue::Expanded);
        assert_eq!(value.secondary, PaneAdaptedValue::Hidden);
        assert_eq!(value.tertiary, PaneAdaptedValue::Hidden);
    }

    #[test]
    fn two_columns_never_reflow() {
        let value = ThreePaneScaffoldValue::calculate(
            2,
            2,
            &strategies(
                AdaptStrategy::Hide,
                AdaptStrategy::Reflow {
                    reflow_under: ThreePaneScaffoldRole::Primary,
                },
                AdaptStrategy::Hide,
            ),
            &[
                destination(ThreePaneScaffoldRole::Primary),
                destination(ThreePaneScaffoldRole::Tertiary),
                destination(ThreePaneScaffoldRole::Secondary),
            ],
        );
        assert_eq!(value.primary, PaneAdaptedValue::Hidden);
        assert_eq!(value.secondary, PaneAdaptedValue::Expanded);
        assert_eq!(value.tertiary, PaneAdaptedValue::Expanded);
    }
}
