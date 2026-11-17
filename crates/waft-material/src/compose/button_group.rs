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
    BorderRadiusDirectional, BorderRadiusGeometry, BorderSide, OutlinedBorderRef,
    RoundedRectangleBorder,
};
use waft_widgets::{
    BuildContext, CrossAxisAlignment, IntoWidget, KeyRef, MainAxisAlignment, MainAxisSize, Row,
    StatelessWidget, WidgetRef,
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

/// A shape whose start corners are `start` and end corners `end`, flipped for right-to-left text
/// as Compose's `RoundedCornerShape(topStart, topEnd, bottomEnd, bottomStart)` is.
fn rounded(start: Radius, end: Radius) -> OutlinedBorderRef {
    OutlinedBorderRef::new(RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadiusGeometry::Directional(BorderRadiusDirectional::only(start, end, start, end)),
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
    /// How a button shorter than the group is aligned vertically; Compose's default is the top.
    pub vertical_alignment: CrossAxisAlignment,
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
            vertical_alignment: CrossAxisAlignment::Start,
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
    pub fn vertical_alignment(mut self, vertical_alignment: CrossAxisAlignment) -> ButtonGroup {
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
            .main_axis_size(MainAxisSize::Min)
            .main_axis_alignment(self.horizontal_alignment)
            .cross_axis_alignment(self.vertical_alignment)
            .spacing(self.spacing)
            .children(self.children.clone())
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{Radius, Rect};
    use waft_widgets::testing::{WidgetTester, find, test_widgets};
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

    fn sized_button(label: &'static str, height: f64) -> WidgetRef {
        SizedBox::new()
            .height(height)
            .child(button(label))
            .into_widget()
    }

    fn pump<K>(tester: &mut WidgetTester, group: impl IntoWidget<K>) {
        let theme = ThemeData::builder().build(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            Center::new().child(group).into_widget(),
        ));
    }

    fn button_rects(tester: &WidgetTester) -> Vec<Rect> {
        find::by_type::<FilledButton>()
            .evaluate(tester)
            .into_iter()
            .map(|element| tester.get_rect(element))
            .collect()
    }

    /// Compose's `default_positioning`, whose twin `differentHorizontalSpacing_positioning` asserts
    /// the same: the buttons sit 12 apart and the group wraps them.
    #[test]
    fn default_positioning() {
        test_widgets(|tester| {
            pump(
                tester,
                ButtonGroup::new(vec![button("A"), button("B"), button("C"), button("D")]),
            );
            let wrapper = tester.get_rect(find::by_type::<ButtonGroup>());
            let buttons = button_rects(tester);
            assert_eq!(buttons[0].left - wrapper.left, 0.0);
            assert_eq!(buttons[1].left - buttons[0].right, 12.0);
            assert_eq!(buttons[2].left - buttons[1].right, 12.0);
            assert_eq!(buttons[3].left - buttons[2].right, 12.0);
            assert_eq!(wrapper.right - buttons[3].right, 0.0);
        });
    }

    /// Not a Compose test: a spacing of zero puts the buttons edge to edge.
    #[test]
    fn a_custom_spacing_puts_the_buttons_edge_to_edge() {
        test_widgets(|tester| {
            pump(
                tester,
                ButtonGroup::new(vec![button("One"), button("Two")]).spacing(0.0),
            );
            let buttons = button_rects(tester);
            assert!((buttons[1].left - buttons[0].right).abs() < 1e-3);
        });
    }

    /// Compose's `horizontalArrangement_*Arrangement_buttonGroupPositioning`: a group as wide as
    /// a phone's screen, 360 here, places three buttons so that they cover the point `fraction` of
    /// its width.
    fn check_arrangement(alignment: MainAxisAlignment, fraction: f64) {
        test_widgets(|tester| {
            pump(
                tester,
                SizedBox::new().width(360.0).child(
                    ButtonGroup::new(vec![button("0"), button("1"), button("2")])
                        .horizontal_alignment(alignment),
                ),
            );
            let screen = tester.get_rect(find::by_type::<ButtonGroup>());
            let buttons = button_rects(tester);
            let expected_positioning = screen.left + screen.width() * fraction;
            assert!((buttons[0].left..=buttons[2].right).contains(&expected_positioning));
        });
    }

    #[test]
    fn horizontal_arrangement_start_arrangement_button_group_positioning() {
        check_arrangement(MainAxisAlignment::Start, 1.0 / 3.0);
    }

    #[test]
    fn horizontal_arrangement_center_arrangement_button_group_positioning() {
        check_arrangement(MainAxisAlignment::Center, 1.0 / 2.0);
    }

    #[test]
    fn horizontal_arrangement_end_arrangement_button_group_positioning() {
        check_arrangement(MainAxisAlignment::End, 2.0 / 3.0);
    }

    /// Compose's `buttonGroup_verticalAlignment_is*`: buttons 48 and 64 high, aligned by the
    /// group's vertical alignment.
    fn aligned_button_rects(tester: &mut WidgetTester, alignment: CrossAxisAlignment) -> Vec<Rect> {
        pump(
            tester,
            ButtonGroup::new(vec![sized_button("A", 48.0), sized_button("B", 64.0)])
                .vertical_alignment(alignment),
        );
        button_rects(tester)
    }

    #[test]
    fn button_group_vertical_alignment_is_top() {
        test_widgets(|tester| {
            let buttons = aligned_button_rects(tester, CrossAxisAlignment::Start);
            assert_eq!(buttons[0].top, buttons[1].top);
        });
    }

    #[test]
    fn button_group_vertical_alignment_is_center() {
        test_widgets(|tester| {
            let buttons = aligned_button_rects(tester, CrossAxisAlignment::Center);
            assert_eq!(
                buttons[0].center().dy().round(),
                buttons[1].center().dy().round()
            );
        });
    }

    #[test]
    fn button_group_vertical_alignment_is_bottom() {
        test_widgets(|tester| {
            let buttons = aligned_button_rects(tester, CrossAxisAlignment::End);
            assert_eq!(buttons[0].bottom, buttons[1].bottom);
        });
    }

    /// The default vertical alignment is Compose's `Alignment.Top`.
    #[test]
    fn button_group_vertical_alignment_defaults_to_top() {
        test_widgets(|tester| {
            pump(
                tester,
                ButtonGroup::new(vec![sized_button("A", 48.0), sized_button("B", 64.0)]),
            );
            let buttons = button_rects(tester);
            assert_eq!(buttons[0].top, buttons[1].top);
        });
    }

    fn corners(shape: &OutlinedBorderRef) -> [Radius; 4] {
        let shape = shape
            .as_any()
            .downcast_ref::<RoundedRectangleBorder>()
            .expect("a rounded rectangle");
        let BorderRadiusGeometry::Directional(radius) = shape.border_radius else {
            panic!("a directional border radius");
        };
        [
            radius.top_start,
            radius.top_end,
            radius.bottom_start,
            radius.bottom_end,
        ]
    }

    /// Not a Compose test: `connectedLeadingButtonShapes`' outer corners take the full height's
    /// radius, the inner ones the connected group's resting and pressed inner corner.
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

    /// Not a Compose test: `connectedMiddleButtonShapes` and `connectedTrailingButtonShapes`' corners.
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
