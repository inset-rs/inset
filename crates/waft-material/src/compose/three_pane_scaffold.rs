//! The pane scaffold: up to three panes laid out and animated from a directive and a value.

use std::time::Duration;

use waft_embedder::{Rect, Size, TextDirection};
use waft_foundation::App;
use waft_widgets::SizedBox;
use waft_widgets::{
    AnimatedOpacity, AnimatedPositioned, BuildContext, Curves, Directionality, IgnorePointer,
    IntoWidget, KeyRef, MediaQuery, Stack, StackFit, StatelessWidget, WidgetRef,
};

use crate::compose::{
    Pane, PaneAdaptedValue, PaneScaffoldDirective, ThreePaneScaffoldRole, ThreePaneScaffoldValue,
};

/// The length of a pane's bounds animation.
const PANE_MOTION_DURATION: Duration = Duration::from_millis(300);

/// Left to right, in left-to-right text; mirrored for right-to-left.
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

    /// The index of `role` from the start, or `None` when it is not in the order.
    pub fn index_of(self, role: ThreePaneScaffoldRole) -> Option<usize> {
        self.roles().iter().position(|candidate| *candidate == role)
    }

    /// This order in left-to-right text: mirrored for a right-to-left direction.
    pub fn to_ltr(self, text_direction: TextDirection) -> ThreePaneScaffoldHorizontalOrder {
        if text_direction == TextDirection::Rtl {
            ThreePaneScaffoldHorizontalOrder::new(self.third, self.second, self.first)
        } else {
            self
        }
    }
}

/// One widget lays out and animates the three panes.
///
/// `list_detail` and `supporting` differ only in pane order and slot names. A hidden pane stays
/// mounted off-screen, so a list keeps its scroll position while the detail covers it.
#[derive(Debug)]
pub struct ThreePaneScaffold {
    /// The scaffold's key.
    pub key: Option<KeyRef>,
    /// The pane sizes and partitions.
    pub directive: PaneScaffoldDirective,
    /// The panes shown.
    pub value: ThreePaneScaffoldValue,
    /// The panes' order from start to end.
    pub pane_order: ThreePaneScaffoldHorizontalOrder,
    /// The primary pane's content.
    pub primary_pane: Pane,
    /// The secondary pane's content.
    pub secondary_pane: Pane,
    /// The tertiary pane's content, when the scaffold has one.
    pub tertiary_pane: Option<Pane>,
}

impl ThreePaneScaffold {
    /// Creates a pane scaffold; `new` takes the three panes' content and order explicitly.
    pub fn new(
        directive: PaneScaffoldDirective,
        value: ThreePaneScaffoldValue,
        pane_order: ThreePaneScaffoldHorizontalOrder,
        primary_pane: impl Into<Pane>,
        secondary_pane: impl Into<Pane>,
    ) -> ThreePaneScaffold {
        ThreePaneScaffold {
            key: None,
            directive,
            value,
            pane_order,
            primary_pane: primary_pane.into(),
            secondary_pane: secondary_pane.into(),
            tertiary_pane: None,
        }
    }

    /// A list-detail scaffold: list, detail, extra.
    pub fn list_detail(
        directive: PaneScaffoldDirective,
        value: ThreePaneScaffoldValue,
        list_pane: impl Into<Pane>,
        detail_pane: impl Into<Pane>,
    ) -> ThreePaneScaffold {
        ThreePaneScaffold::new(
            directive,
            value,
            ThreePaneScaffoldHorizontalOrder::LIST_DETAIL,
            detail_pane,
            list_pane,
        )
    }

    /// A supporting-pane scaffold: main, supporting, extra.
    pub fn supporting(
        directive: PaneScaffoldDirective,
        value: ThreePaneScaffoldValue,
        main_pane: impl Into<Pane>,
        supporting_pane: impl Into<Pane>,
    ) -> ThreePaneScaffold {
        ThreePaneScaffold::new(
            directive,
            value,
            ThreePaneScaffoldHorizontalOrder::SUPPORTING,
            main_pane,
            supporting_pane,
        )
    }

    /// Adds the tertiary pane.
    pub fn tertiary_pane(mut self, tertiary_pane: impl Into<Pane>) -> ThreePaneScaffold {
        self.tertiary_pane = Some(tertiary_pane.into());
        self
    }

    /// Sets the scaffold's key.
    pub fn key(mut self, key: KeyRef) -> ThreePaneScaffold {
        self.key = Some(key);
        self
    }

    fn pane(&self, role: ThreePaneScaffoldRole) -> Option<&Pane> {
        match role {
            ThreePaneScaffoldRole::Primary => Some(&self.primary_pane),
            ThreePaneScaffoldRole::Secondary => Some(&self.secondary_pane),
            ThreePaneScaffoldRole::Tertiary => self.tertiary_pane.as_ref(),
        }
    }
}

impl StatelessWidget for ThreePaneScaffold {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let size = MediaQuery::size_of(app, context);
        let order = self.pane_order.to_ltr(Directionality::of(app, context));
        let rects = layout_panes(self, size, order);

        let mut children = Vec::with_capacity(3);
        for (index, role) in order.roles().into_iter().enumerate() {
            let Some(pane) = self.pane(role) else {
                continue;
            };
            let shown = self.value.get(role) != PaneAdaptedValue::Hidden;
            let rect = rects[index];
            let opacity = if shown { 1.0 } else { 0.0 };
            children.push(
                AnimatedPositioned::from_rect(
                    IgnorePointer::new()
                        .ignoring(!shown)
                        .child(
                            AnimatedOpacity::new(opacity, PANE_MOTION_DURATION)
                                .child(pane.child.clone()),
                        )
                        .into_widget(),
                    rect,
                    PANE_MOTION_DURATION,
                )
                .curve(Curves::fast_out_slow_in())
                .into_widget(),
            );
        }

        SizedBox::expand()
            .child(Stack::new().fit(StackFit::Expand).children(children))
            .into_widget()
    }
}

/// The target rectangle of each pane in `order`, hidden panes off-screen.
fn layout_panes(
    scaffold: &ThreePaneScaffold,
    size: Size,
    order: ThreePaneScaffoldHorizontalOrder,
) -> [Rect; 3] {
    let mut rects = [Rect::ZERO; 3];
    let roles = order.roles();
    let roles_with_panes: Vec<(usize, ThreePaneScaffoldRole)> = roles
        .into_iter()
        .enumerate()
        .filter(|(_, role)| scaffold.pane(*role).is_some())
        .collect();

    let expanded: Vec<(usize, ThreePaneScaffoldRole)> = roles_with_panes
        .iter()
        .copied()
        .filter(|(_, role)| scaffold.value.get(*role) == PaneAdaptedValue::Expanded)
        .collect();
    let reflowed: Option<(ThreePaneScaffoldRole, ThreePaneScaffoldRole)> = roles_with_panes
        .iter()
        .find_map(|(_, role)| match scaffold.value.get(*role) {
            PaneAdaptedValue::Reflowed { reflow_under } => Some((*role, reflow_under)),
            _ => None,
        });

    let spacer = scaffold.directive.horizontal_partition_spacer_size;
    let allocatable_width = size.width() - (expanded.len().saturating_sub(1)) as f64 * spacer;
    let mut widths: Vec<f64> = expanded
        .iter()
        .map(|(_, role)| {
            scaffold
                .pane(*role)
                .expect("the role has a pane")
                .width_for(size, scaffold.directive.default_pane_preferred_width)
        })
        .collect();
    let total: f64 = widths.iter().sum();
    if allocatable_width > total && !widths.is_empty() {
        let winner = expanded
            .iter()
            .enumerate()
            .max_by_key(|(_, (_, role))| priority(*role))
            .map(|(index, _)| index)
            .expect("an expanded pane");
        widths[winner] += allocatable_width - total;
    } else if allocatable_width < total && total > 0.0 {
        let scale = allocatable_width / total;
        for width in &mut widths {
            *width *= scale;
        }
    }

    let vertical_spacer = scaffold.directive.vertical_partition_spacer_size;
    let mut x = 0.0;
    let mut expanded_index = 0;
    for (order_index, role) in roles_with_panes {
        match scaffold.value.get(role) {
            PaneAdaptedValue::Expanded => {
                let width = widths[expanded_index];
                expanded_index += 1;
                let mut height = size.height();
                if let Some((reflow_role, anchor)) = reflowed
                    && anchor == role
                {
                    let available = (size.height() - vertical_spacer).max(0.0);
                    let reflow_height = scaffold
                        .pane(reflow_role)
                        .expect("the role has a pane")
                        .height_for(size, scaffold.directive.default_pane_preferred_height)
                        .min(available);
                    height = (available - reflow_height).max(available / 2.0);
                    rects[order.index_of(reflow_role).expect("in the order")] =
                        Rect::from_ltwh(x, height + vertical_spacer, width, available - height);
                }
                rects[order_index] = Rect::from_ltwh(x, 0.0, width, height);
                x += width + spacer;
            }
            PaneAdaptedValue::Hidden => {
                let width = scaffold
                    .pane(role)
                    .expect("the role has a pane")
                    .width_for(size, scaffold.directive.default_pane_preferred_width);
                let left = if order_index == 0 {
                    -width
                } else if order_index + 1 == roles.len() {
                    size.width()
                } else {
                    -width
                };
                rects[order_index] = Rect::from_ltwh(left, 0.0, width, size.height());
            }
            PaneAdaptedValue::Reflowed { .. } => {}
        }
    }
    rects
}

fn priority(role: ThreePaneScaffoldRole) -> u8 {
    match role {
        ThreePaneScaffoldRole::Primary => 10,
        ThreePaneScaffoldRole::Secondary => 5,
        ThreePaneScaffoldRole::Tertiary => 1,
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{Size, TextDirection};
    use waft_foundation::ValueKey;
    use waft_widgets::testing::{WidgetTester, find, test_widgets};
    use waft_widgets::{Directionality, IntoWidget, MediaQuery, MediaQueryData, SizedBox};

    use super::*;
    use crate::compose::{
        ListDetailPaneScaffoldRole, PaneAdaptedValue, ThreePaneScaffoldValue,
        WindowHeightSizeClass, WindowSizeClass, WindowWidthSizeClass,
    };

    fn size_class(width: WindowWidthSizeClass, height: WindowHeightSizeClass) -> WindowSizeClass {
        WindowSizeClass { width, height }
    }

    fn pane(key: i32) -> Pane {
        Pane::new(SizedBox::new().key(ValueKey::new(key).into()))
    }

    fn pump(tester: &mut WidgetTester, width: f64, height: f64, scaffold: ThreePaneScaffold) {
        tester.pump_widget(
            MediaQuery::new(
                MediaQueryData::new().size(Size::new(width, height)),
                Directionality::new(TextDirection::Ltr, scaffold),
            )
            .into_widget(),
        );
    }

    fn two_pane_value() -> ThreePaneScaffoldValue {
        ThreePaneScaffoldValue {
            primary: PaneAdaptedValue::Expanded,
            secondary: PaneAdaptedValue::Expanded,
            tertiary: PaneAdaptedValue::Hidden,
        }
    }

    fn one_primary_value() -> ThreePaneScaffoldValue {
        ThreePaneScaffoldValue {
            primary: PaneAdaptedValue::Expanded,
            secondary: PaneAdaptedValue::Hidden,
            tertiary: PaneAdaptedValue::Hidden,
        }
    }

    #[test]
    fn a_wide_list_detail_places_the_list_beside_the_detail() {
        test_widgets(|tester| {
            let directive = PaneScaffoldDirective::from_window_size_class(size_class(
                WindowWidthSizeClass::Expanded,
                WindowHeightSizeClass::Expanded,
            ));
            pump(
                tester,
                1000.0,
                600.0,
                ThreePaneScaffold::list_detail(directive, two_pane_value(), pane(1), pane(2)),
            );
            assert_eq!(
                tester.get_rect(find::by_key(ValueKey::new(1))),
                Rect::from_ltwh(0.0, 0.0, 360.0, 600.0)
            );
            assert_eq!(
                tester.get_rect(find::by_key(ValueKey::new(2))),
                Rect::from_ltwh(384.0, 0.0, 616.0, 600.0)
            );
        });
    }

    #[test]
    fn a_phone_shows_only_the_detail_and_keeps_the_list_off_screen() {
        test_widgets(|tester| {
            let directive = PaneScaffoldDirective::from_window_size_class(size_class(
                WindowWidthSizeClass::Compact,
                WindowHeightSizeClass::Medium,
            ));
            pump(
                tester,
                400.0,
                600.0,
                ThreePaneScaffold::list_detail(directive, one_primary_value(), pane(1), pane(2)),
            );
            assert_eq!(
                tester.get_rect(find::by_key(ValueKey::new(2))),
                Rect::from_ltwh(0.0, 0.0, 400.0, 600.0)
            );
            assert!(tester.get_rect(find::by_key(ValueKey::new(1))).right <= 0.0);
        });
    }

    #[test]
    fn a_reflowed_pane_sits_under_its_anchor() {
        test_widgets(|tester| {
            let directive = PaneScaffoldDirective::from_window_size_class(size_class(
                WindowWidthSizeClass::Compact,
                WindowHeightSizeClass::Expanded,
            ));
            let value = ThreePaneScaffoldValue {
                primary: PaneAdaptedValue::Expanded,
                secondary: PaneAdaptedValue::Reflowed {
                    reflow_under: crate::compose::SupportingPaneScaffoldRole::MAIN,
                },
                tertiary: PaneAdaptedValue::Hidden,
            };
            pump(
                tester,
                400.0,
                800.0,
                ThreePaneScaffold::supporting(directive, value, pane(1), pane(2)),
            );
            let main = tester.get_rect(find::by_key(ValueKey::new(1)));
            let supporting = tester.get_rect(find::by_key(ValueKey::new(2)));
            assert_eq!(main, Rect::from_ltwh(0.0, 0.0, 400.0, 388.0));
            assert_eq!(supporting, Rect::from_ltwh(0.0, 412.0, 400.0, 388.0));
        });
    }

    #[test]
    fn a_pane_animates_from_its_old_bounds_to_its_new_ones() {
        test_widgets(|tester| {
            let narrow = PaneScaffoldDirective::from_window_size_class(size_class(
                WindowWidthSizeClass::Compact,
                WindowHeightSizeClass::Medium,
            ));
            pump(
                tester,
                400.0,
                600.0,
                ThreePaneScaffold::list_detail(
                    narrow.clone(),
                    one_primary_value(),
                    pane(1),
                    pane(2),
                ),
            );
            let wide = PaneScaffoldDirective::from_window_size_class(size_class(
                WindowWidthSizeClass::Expanded,
                WindowHeightSizeClass::Expanded,
            ));
            pump(
                tester,
                1000.0,
                600.0,
                ThreePaneScaffold::list_detail(wide, two_pane_value(), pane(1), pane(2)),
            );
            tester.pump_by(Duration::from_millis(150));
            let halfway = tester.get_rect(find::by_key(ValueKey::new(2)));
            assert!(
                halfway.left > 0.0 && halfway.left < 384.0,
                "the detail is on its way, at {}",
                halfway.left
            );
            tester.pump_and_settle();
            assert_eq!(
                tester.get_rect(find::by_key(ValueKey::new(2))),
                Rect::from_ltwh(384.0, 0.0, 616.0, 600.0)
            );
        });
    }

    #[test]
    fn right_to_left_mirrors_the_panes() {
        test_widgets(|tester| {
            let directive = PaneScaffoldDirective::from_window_size_class(size_class(
                WindowWidthSizeClass::Expanded,
                WindowHeightSizeClass::Expanded,
            ));
            tester.pump_widget(
                MediaQuery::new(
                    MediaQueryData::new().size(Size::new(1000.0, 600.0)),
                    Directionality::new(
                        TextDirection::Rtl,
                        ThreePaneScaffold::list_detail(
                            directive,
                            two_pane_value(),
                            pane(1),
                            pane(2),
                        ),
                    ),
                )
                .into_widget(),
            );
            assert_eq!(
                tester.get_rect(find::by_key(ValueKey::new(1))),
                Rect::from_ltwh(640.0, 0.0, 360.0, 600.0)
            );
            assert_eq!(
                tester.get_rect(find::by_key(ValueKey::new(2))),
                Rect::from_ltwh(0.0, 0.0, 616.0, 600.0)
            );
        });
    }

    #[test]
    fn a_proportional_pane_takes_its_fraction_of_the_scaffold() {
        test_widgets(|tester| {
            let directive = PaneScaffoldDirective::from_window_size_class(size_class(
                WindowWidthSizeClass::Expanded,
                WindowHeightSizeClass::Expanded,
            ));
            let list = pane(1).preferred_width(crate::compose::PanePreferredSize::Proportion(0.25));
            pump(
                tester,
                1000.0,
                600.0,
                ThreePaneScaffold::list_detail(directive, two_pane_value(), list, pane(2)),
            );
            let list = tester.get_rect(find::by_key(ValueKey::new(1)));
            assert_eq!(list.width(), 250.0);
            assert_eq!(list.left, 0.0);
        });
    }

    #[test]
    fn a_tertiary_pane_takes_the_third_partition() {
        test_widgets(|tester| {
            let directive = PaneScaffoldDirective::from_window_size_class(size_class(
                WindowWidthSizeClass::Large,
                WindowHeightSizeClass::Expanded,
            ));
            let value = ThreePaneScaffoldValue {
                primary: PaneAdaptedValue::Expanded,
                secondary: PaneAdaptedValue::Expanded,
                tertiary: PaneAdaptedValue::Expanded,
            };
            pump(
                tester,
                1400.0,
                600.0,
                ThreePaneScaffold::list_detail(directive, value, pane(1), pane(2))
                    .tertiary_pane(pane(3)),
            );
            let extra = tester.get_rect(find::by_key(ValueKey::new(3)));
            assert_eq!(extra.left, 988.0);
            assert_eq!(extra.width(), 412.0);
        });
    }

    #[test]
    fn a_pane_of_its_own_is_a_pane() {
        let pane = Pane::from(SizedBox::new().into_widget());
        assert_eq!(pane.preferred_width, None);
    }

    #[test]
    fn the_list_detail_order_is_list_then_detail() {
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
            Some(1)
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
