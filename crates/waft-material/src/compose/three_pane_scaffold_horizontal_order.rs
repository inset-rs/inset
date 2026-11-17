//! The order of a pane scaffold's three panes, from start to end.

use waft_embedder::TextDirection;

use crate::compose::ThreePaneScaffoldRole;

/// The order of a pane scaffold's three panes, from start to end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThreePaneScaffoldHorizontalOrder {
    first: ThreePaneScaffoldRole,
    second: ThreePaneScaffoldRole,
    third: ThreePaneScaffoldRole,
}

impl ThreePaneScaffoldHorizontalOrder {
    /// The list-detail order: list, detail, extra.
    pub const LIST_DETAIL: ThreePaneScaffoldHorizontalOrder = ThreePaneScaffoldHorizontalOrder {
        first: ThreePaneScaffoldRole::Secondary,
        second: ThreePaneScaffoldRole::Primary,
        third: ThreePaneScaffoldRole::Tertiary,
    };

    /// The supporting-pane order: main, supporting, extra.
    pub const SUPPORTING: ThreePaneScaffoldHorizontalOrder = ThreePaneScaffoldHorizontalOrder {
        first: ThreePaneScaffoldRole::Primary,
        second: ThreePaneScaffoldRole::Secondary,
        third: ThreePaneScaffoldRole::Tertiary,
    };

    /// Creates an order; panics unless the three roles differ.
    pub fn new(
        first: ThreePaneScaffoldRole,
        second: ThreePaneScaffoldRole,
        third: ThreePaneScaffoldRole,
    ) -> ThreePaneScaffoldHorizontalOrder {
        assert!(
            first != second && second != third && first != third,
            "invalid ThreePaneScaffoldHorizontalOrder({first:?}, {second:?}, {third:?}) - panes \
             must be unique"
        );
        ThreePaneScaffoldHorizontalOrder {
            first,
            second,
            third,
        }
    }

    /// The roles from start to end.
    pub fn roles(self) -> [ThreePaneScaffoldRole; 3] {
        [self.first, self.second, self.third]
    }

    /// The role at `index`, from the start.
    pub fn get(self, index: usize) -> ThreePaneScaffoldRole {
        self.roles()[index]
    }

    /// The index of `role` from the start.
    pub fn index_of(self, role: ThreePaneScaffoldRole) -> usize {
        self.roles()
            .iter()
            .position(|candidate| *candidate == role)
            .expect("an order holds all three roles")
    }

    /// This order from left to right: mirrored for a right-to-left direction.
    pub fn to_ltr(self, text_direction: TextDirection) -> ThreePaneScaffoldHorizontalOrder {
        if text_direction == TextDirection::Rtl {
            ThreePaneScaffoldHorizontalOrder::new(self.third, self.second, self.first)
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::ListDetailPaneScaffoldRole;

    #[test]
    fn the_list_detail_order_is_list_then_detail_and_mirrors_for_rtl() {
        assert_eq!(
            ThreePaneScaffoldHorizontalOrder::LIST_DETAIL.roles(),
            [
                ListDetailPaneScaffoldRole::LIST,
                ListDetailPaneScaffoldRole::DETAIL,
                ListDetailPaneScaffoldRole::EXTRA,
            ]
        );
        assert_eq!(
            ThreePaneScaffoldHorizontalOrder::LIST_DETAIL
                .index_of(ListDetailPaneScaffoldRole::DETAIL),
            1
        );
        assert_eq!(
            ThreePaneScaffoldHorizontalOrder::LIST_DETAIL.to_ltr(TextDirection::Rtl),
            ThreePaneScaffoldHorizontalOrder::new(
                ListDetailPaneScaffoldRole::EXTRA,
                ListDetailPaneScaffoldRole::DETAIL,
                ListDetailPaneScaffoldRole::LIST,
            )
        );
    }

    #[test]
    #[should_panic(expected = "panes must be unique")]
    fn an_order_with_a_repeated_role_panics() {
        let _ = ThreePaneScaffoldHorizontalOrder::new(
            ListDetailPaneScaffoldRole::LIST,
            ListDetailPaneScaffoldRole::DETAIL,
            ListDetailPaneScaffoldRole::DETAIL,
        );
    }
}
