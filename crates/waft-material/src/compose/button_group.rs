//! Compose's material3 `ButtonGroup.kt`: the parts the port can carry.
//!
//! Compose's button group gathers a row of related buttons. A standard group spaces them 12
//! apart; a connected group spaces them 2 apart and rounds the corners that meet the gap
//! tighter than the outer ones. Compose also expands the pressed button and hides the ones that
//! do not fit into an overflow menu; see PORTING.md for why those are not here.
//!
//! The group lays out buttons the app builds, so an app styles a connected group's buttons with
//! the shapes in [`ButtonGroupDefaults`].

use std::fmt;

use waft_embedder::Radius;
use waft_foundation::App;
use waft_painting::{
    BorderRadius, BorderRadiusGeometry, BorderSide, OutlinedBorderRef, RoundedRectangleBorder,
};
use waft_widgets::{
    BuildContext, IntoWidget, KeyRef, MainAxisAlignment, Row, StatelessWidget, VerticalDirection,
    WidgetRef,
};

use crate::compose::split_button::SplitButtonDefaults;

/// Compose's `ButtonGroupDefaults`: the spacing and shapes a group uses.
pub struct ButtonGroupDefaults;

impl ButtonGroupDefaults {
    /// The fraction of the pressed button's width a press is allowed to add.
    ///
    /// The group does not expand a pressed button (PORTING.md); this is the value Compose would
    /// use.
    pub const EXPANDED_RATIO: f64 = 0.15;

    /// The default spacing between the buttons of a standard group.
    pub const BETWEEN_SPACE: f64 = 12.0;

    /// The default spacing between the buttons of a connected group.
    pub const CONNECTED_SPACE_BETWEEN: f64 = 2.0;

    /// The resting inner corner of a connected button, `ShapeTokens.CornerValueSmall`.
    const INNER: f64 = 8.0;

    /// The pressed inner corner of a connected button, `ShapeTokens.CornerValueExtraSmall`.
    const PRESSED_INNER: f64 = 4.0;

    /// The shapes of the first button of a connected group at `button_height`.
    pub fn connected_leading_button_shapes_for(
        button_height: f64,
    ) -> (OutlinedBorderRef, OutlinedBorderRef, OutlinedBorderRef) {
        let full = Radius::circular(button_height / 2.0);
        (
            rounded(full, Radius::circular(Self::INNER)),
            rounded(full, Radius::circular(Self::PRESSED_INNER)),
            rounded(full, full),
        )
    }

    /// The shapes of a middle button of a connected group at `button_height`.
    pub fn connected_middle_button_shapes_for(
        button_height: f64,
    ) -> (OutlinedBorderRef, OutlinedBorderRef, OutlinedBorderRef) {
        let full = Radius::circular(button_height / 2.0);
        (
            rounded(Radius::circular(Self::INNER), Radius::circular(Self::INNER)),
            rounded(
                Radius::circular(Self::PRESSED_INNER),
                Radius::circular(Self::PRESSED_INNER),
            ),
            rounded(full, full),
        )
    }

    /// The shapes of the last button of a connected group at `button_height`.
    pub fn connected_trailing_button_shapes_for(
        button_height: f64,
    ) -> (OutlinedBorderRef, OutlinedBorderRef, OutlinedBorderRef) {
        let full = Radius::circular(button_height / 2.0);
        (
            rounded(Radius::circular(Self::INNER), full),
            rounded(Radius::circular(Self::PRESSED_INNER), full),
            rounded(full, full),
        )
    }

    /// The three shapes of the first button of a connected group of small buttons.
    pub fn connected_leading_button_shapes()
    -> (OutlinedBorderRef, OutlinedBorderRef, OutlinedBorderRef) {
        Self::connected_leading_button_shapes_for(SplitButtonDefaults::SMALL_CONTAINER_HEIGHT)
    }

    /// The three shapes of a middle button of a connected group of small buttons.
    pub fn connected_middle_button_shapes()
    -> (OutlinedBorderRef, OutlinedBorderRef, OutlinedBorderRef) {
        Self::connected_middle_button_shapes_for(SplitButtonDefaults::SMALL_CONTAINER_HEIGHT)
    }

    /// The three shapes of the last button of a connected group of small buttons.
    pub fn connected_trailing_button_shapes()
    -> (OutlinedBorderRef, OutlinedBorderRef, OutlinedBorderRef) {
        Self::connected_trailing_button_shapes_for(SplitButtonDefaults::SMALL_CONTAINER_HEIGHT)
    }
}

/// A shape whose start corners are `start` and end corners `end`.
fn rounded(start: Radius, end: Radius) -> OutlinedBorderRef {
    OutlinedBorderRef::new(RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadiusGeometry::BorderRadius(BorderRadius::only(start, end, start, end)),
    ))
}

/// A row of related buttons.
///
/// Compose's `ButtonGroup`, without its press expansion or its overflow menu (PORTING.md).
pub struct ButtonGroup {
    pub key: Option<KeyRef>,
    /// The group's buttons, which the app builds and styles.
    pub children: Vec<WidgetRef>,
    /// The space between neighbouring buttons.
    pub spacing: f64,
    /// How the buttons are placed along the row.
    pub horizontal_alignment: MainAxisAlignment,
    /// How a button shorter than the row is aligned vertically.
    pub vertical_alignment: VerticalDirection,
}

impl ButtonGroup {
    /// Creates a standard button group; Compose's `ButtonGroup(overflowIndicator, content)` with
    /// its default arrangement, and no overflow indicator.
    pub fn new(children: impl IntoIterator<Item = WidgetRef>) -> ButtonGroup {
        ButtonGroup {
            key: None,
            children: children.into_iter().collect(),
            spacing: ButtonGroupDefaults::BETWEEN_SPACE,
            horizontal_alignment: MainAxisAlignment::Start,
            vertical_alignment: VerticalDirection::Down,
        }
    }

    /// Creates a connected button group, whose buttons sit 2 apart and are styled with
    /// [`ButtonGroupDefaults`]'s connected shapes.
    pub fn connected(children: impl IntoIterator<Item = WidgetRef>) -> ButtonGroup {
        ButtonGroup {
            spacing: ButtonGroupDefaults::CONNECTED_SPACE_BETWEEN,
            ..ButtonGroup::new(children)
        }
    }

    /// Dart `key:`.
    pub fn key(mut self, key: KeyRef) -> ButtonGroup {
        self.key = Some(key);
        self
    }

    /// Compose's `horizontalArrangement:` argument, as the space between buttons and the row's
    /// alignment.
    pub fn spacing(mut self, spacing: f64) -> ButtonGroup {
        self.spacing = spacing;
        self
    }

    /// Compose's `horizontalArrangement: Arrangement.Start/Center/End`.
    pub fn horizontal_alignment(mut self, horizontal_alignment: MainAxisAlignment) -> ButtonGroup {
        self.horizontal_alignment = horizontal_alignment;
        self
    }

    /// Compose's `verticalAlignment:` argument.
    pub fn vertical_alignment(mut self, vertical_alignment: VerticalDirection) -> ButtonGroup {
        self.vertical_alignment = vertical_alignment;
        self
    }
}

impl fmt::Debug for ButtonGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ButtonGroup")
            .field("buttons", &self.children.len())
            .field("spacing", &self.spacing)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for ButtonGroup {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        Row::new()
            .main_axis_alignment(self.horizontal_alignment)
            .vertical_direction(self.vertical_alignment)
            .spacing(self.spacing)
            .children(self.children.clone())
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::Radius;
    use waft_widgets::testing::{find, test_widgets};
    use waft_widgets::{Center, IntoWidget, SizedBox, Text};

    use super::*;
    use crate::filled_button::FilledButton;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    fn button(label: &'static str) -> WidgetRef {
        FilledButton::new(
            Some(waft_foundation::Listener::new(|_app| {})),
            Some(Text::new(label)),
        )
        .into_widget()
    }

    fn pump(tester: &mut waft_widgets::testing::WidgetTester, group: ButtonGroup) {
        let theme = ThemeData::builder().build(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            Center::new().child(group).into_widget(),
        ));
    }

    /// Compose's `default_positioning`: the buttons sit 12 apart and the first starts at the
    /// group's start.
    #[test]
    fn buttons_are_spaced_apart() {
        test_widgets(|tester| {
            pump(
                tester,
                ButtonGroup::new(vec![button("One"), button("Two"), button("Three")]),
            );
            let buttons: Vec<_> = find::by_type::<FilledButton>()
                .evaluate(tester)
                .into_iter()
                .map(|element| tester.get_rect(element))
                .collect();
            assert_eq!(buttons[1].left - buttons[0].right, 12.0);
            assert_eq!(buttons[2].left - buttons[1].right, 12.0);
        });
    }

    /// Compose's `differentHorizontalSpacing_positioning`: a spacing of zero puts the buttons
    /// edge to edge.
    #[test]
    fn a_custom_spacing_puts_the_buttons_edge_to_edge() {
        test_widgets(|tester| {
            pump(
                tester,
                ButtonGroup::new(vec![button("One"), button("Two")]).spacing(0.0),
            );
            let buttons: Vec<_> = find::by_type::<FilledButton>()
                .evaluate(tester)
                .into_iter()
                .map(|element| tester.get_rect(element))
                .collect();
            assert_eq!(buttons[1].left, buttons[0].right);
        });
    }

    /// Compose's `horizontalArrangement_centerArrangement_buttonGroupPositioning`: a centred
    /// group keeps its buttons centred in the space it is given.
    #[test]
    fn a_centred_group_centres_its_buttons() {
        test_widgets(|tester| {
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                SizedBox::new()
                    .width(400.0)
                    .child(
                        ButtonGroup::new(vec![button("One"), button("Two")])
                            .horizontal_alignment(MainAxisAlignment::Center),
                    )
                    .into_widget(),
            ));
            let buttons: Vec<_> = find::by_type::<FilledButton>()
                .evaluate(tester)
                .into_iter()
                .map(|element| tester.get_rect(element))
                .collect();
            let group = tester.get_rect(find::by_type::<ButtonGroup>());
            let middle = (buttons[0].left + buttons[1].right) / 2.0;
            assert!((middle - group.center().dx()).abs() < 1.0);
        });
    }

    fn corners(shape: &OutlinedBorderRef) -> [Radius; 4] {
        let shape = shape
            .as_any()
            .downcast_ref::<RoundedRectangleBorder>()
            .expect("a rounded rectangle");
        let BorderRadiusGeometry::BorderRadius(radius) = shape.border_radius else {
            panic!("a resolved border radius");
        };
        [
            radius.top_left,
            radius.top_right,
            radius.bottom_left,
            radius.bottom_right,
        ]
    }

    /// Compose's `ButtonGroupDefaults.connectedLeadingButtonShapes`: the outer corners take the
    /// full height's radius, the inner ones the small and pressed inner corner.
    #[test]
    fn the_connected_leading_shapes_follow_their_corners() {
        let (shape, pressed, checked) =
            ButtonGroupDefaults::connected_leading_button_shapes_for(40.0);
        assert_eq!(
            corners(&shape),
            [
                Radius::circular(20.0),
                Radius::circular(8.0),
                Radius::circular(20.0),
                Radius::circular(8.0)
            ]
        );
        assert_eq!(
            corners(&pressed),
            [
                Radius::circular(20.0),
                Radius::circular(4.0),
                Radius::circular(20.0),
                Radius::circular(4.0)
            ]
        );
        assert_eq!([20.0; 4].map(Radius::circular), corners(&checked));
    }

    /// Compose's `connectedMiddleButtonShapes` and `connectedTrailingButtonShapes`.
    #[test]
    fn the_connected_middle_and_trailing_shapes_follow_their_corners() {
        let (middle, pressed, _) = ButtonGroupDefaults::connected_middle_button_shapes_for(40.0);
        assert_eq!(
            corners(&middle),
            [
                Radius::circular(8.0),
                Radius::circular(8.0),
                Radius::circular(8.0),
                Radius::circular(8.0)
            ]
        );
        assert_eq!([4.0; 4].map(Radius::circular), corners(&pressed));

        let (trailing, pressed, checked) =
            ButtonGroupDefaults::connected_trailing_button_shapes_for(40.0);
        assert_eq!(
            corners(&trailing),
            [
                Radius::circular(8.0),
                Radius::circular(20.0),
                Radius::circular(8.0),
                Radius::circular(20.0)
            ]
        );
        assert_eq!(
            corners(&pressed),
            [
                Radius::circular(4.0),
                Radius::circular(20.0),
                Radius::circular(4.0),
                Radius::circular(20.0)
            ]
        );
        assert_eq!([20.0; 4].map(Radius::circular), corners(&checked));
    }
}
