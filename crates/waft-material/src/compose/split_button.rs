//! Compose's material3 `SplitButton.kt`.
//!
//! A split button is two buttons side by side: a leading button for the primary action and a
//! trailing one for a related secondary action, with the corners next to the gap rounded tighter
//! than the outer ones. Compose builds both parts from `Surface` and its `ButtonDefaults`; here
//! they are the port's own `FilledButton`, `ElevatedButton` and `OutlinedButton`, whose styles
//! carry the same colours and elevations, with the split's shape, padding and minimum size
//! layered over them.
//!
//! Compose animates a part's shape through its effects spring. Flutter's [`Material`] animates a
//! shape change over the button's `animation_duration`, and that is what a part uses (PORTING.md).
//!
//! [`Material`]: crate::material::Material

use std::fmt;

use waft_embedder::{Radius, Size};
use waft_foundation::{App, Listener};
use waft_painting::{
    BorderRadius, BorderRadiusGeometry, BorderSide, EdgeInsetsDirectional, EdgeInsetsGeometry,
    OutlinedBorderRef, RoundedRectangleBorder,
};
use waft_widgets::{
    BuildContext, IntoWidget, IntrinsicHeight, KeyRef, MainAxisSize, Row, SizedBox,
    StatelessWidget, WidgetRef, WidgetState, WidgetStateProperty, WidgetStatePropertyRef,
};

use crate::button_style::ButtonStyle;
use crate::elevated_button::ElevatedButton;
use crate::filled_button::FilledButton;
use crate::outlined_button::OutlinedButton;

/// The default spacing between a split button's two parts.
pub const K_SPLIT_BUTTON_SPACING: f64 = 2.0;

/// The smallest horizontal extent of either part.
const K_SPLIT_BUTTON_MIN_WIDTH: f64 = 48.0;

/// Compose's `SplitButtonShapes`: the shapes a part morphs between as it is pressed, checked or
/// rested.
#[derive(Clone, Debug, PartialEq)]
pub struct SplitButtonShapes {
    /// The resting shape.
    pub shape: OutlinedBorderRef,
    /// The shape while pressed, when the part has one.
    pub pressed_shape: Option<OutlinedBorderRef>,
    /// The shape while checked, when the part has one.
    pub checked_shape: Option<OutlinedBorderRef>,
}

impl SplitButtonShapes {
    /// Creates a set of shapes; Compose's `SplitButtonShapes(shape, pressedShape, checkedShape)`.
    pub fn new(
        shape: OutlinedBorderRef,
        pressed_shape: Option<OutlinedBorderRef>,
        checked_shape: Option<OutlinedBorderRef>,
    ) -> SplitButtonShapes {
        SplitButtonShapes {
            shape,
            pressed_shape,
            checked_shape,
        }
    }
}

/// The four button styles a split button's part can take, one per `*SplitButton` in Compose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitButtonVariant {
    /// A high-emphasis filled button.
    Filled,
    /// A medium-emphasis tonal button.
    Tonal,
    /// A tonal button with a shadow.
    Elevated,
    /// An outlined button.
    Outlined,
}

/// Compose's `SplitButtonDefaults`: the token values, and the shapes, paddings and icon sizes
/// chosen for a part's height.
pub struct SplitButtonDefaults;

impl SplitButtonDefaults {
    /// The height of an extra small container.
    pub const EXTRA_SMALL_CONTAINER_HEIGHT: f64 = 32.0;
    /// The height of a small container, and the parts' default minimum height.
    pub const SMALL_CONTAINER_HEIGHT: f64 = 40.0;
    /// The height of a medium container.
    pub const MEDIUM_CONTAINER_HEIGHT: f64 = 56.0;
    /// The height of a large container.
    pub const LARGE_CONTAINER_HEIGHT: f64 = 96.0;
    /// The height of an extra large container.
    pub const EXTRA_LARGE_CONTAINER_HEIGHT: f64 = 136.0;

    /// The default size of the leading part's icon at the small height.
    pub const LEADING_ICON_SIZE: f64 = 20.0;
    /// The default size of the trailing part's icon at the small height.
    pub const TRAILING_ICON_SIZE: f64 = 22.0;

    /// The shapes of a leading part at `button_height`: the outer corners are full, and the inner
    /// ones follow the size's resting and pressed inner corner.
    pub fn leading_button_shapes_for(button_height: f64) -> SplitButtonShapes {
        let (inner, pressed) = inner_corners_for(button_height);
        SplitButtonShapes::new(
            rounded(full_corner(button_height), inner),
            Some(rounded(full_corner(button_height), pressed)),
            None,
        )
    }

    /// The shapes of a trailing part at `button_height`.
    pub fn trailing_button_shapes_for(button_height: f64) -> SplitButtonShapes {
        let (inner, pressed) = inner_corners_for(button_height);
        SplitButtonShapes::new(
            rounded(inner, full_corner(button_height)),
            Some(rounded(pressed, full_corner(button_height))),
            Some(rounded(
                full_corner(button_height),
                full_corner(button_height),
            )),
        )
    }

    /// The content padding of a leading part at `button_height`.
    pub fn leading_button_content_padding_for(button_height: f64) -> EdgeInsetsGeometry {
        let (start, end) = if button_height < Self::SMALL_CONTAINER_HEIGHT {
            (12.0, 10.0)
        } else if button_height < Self::MEDIUM_CONTAINER_HEIGHT {
            (16.0, 12.0)
        } else if button_height < Self::LARGE_CONTAINER_HEIGHT {
            (24.0, 24.0)
        } else if button_height < Self::EXTRA_LARGE_CONTAINER_HEIGHT {
            (48.0, 48.0)
        } else {
            (64.0, 64.0)
        };
        EdgeInsetsDirectional::only(start, 0.0, end, 0.0).into()
    }

    /// The content padding of a trailing part at `button_height`.
    pub fn trailing_button_content_padding_for(button_height: f64) -> EdgeInsetsGeometry {
        let horizontal = if button_height < Self::MEDIUM_CONTAINER_HEIGHT {
            13.0
        } else if button_height < Self::LARGE_CONTAINER_HEIGHT {
            15.0
        } else if button_height < Self::EXTRA_LARGE_CONTAINER_HEIGHT {
            29.0
        } else {
            43.0
        };
        EdgeInsetsDirectional::only(horizontal, 0.0, horizontal, 0.0).into()
    }

    /// The size of a leading part's icon at `button_height`.
    pub fn leading_button_icon_size_for(button_height: f64) -> f64 {
        if button_height < Self::MEDIUM_CONTAINER_HEIGHT {
            20.0
        } else if button_height < Self::LARGE_CONTAINER_HEIGHT {
            24.0
        } else if button_height < Self::EXTRA_LARGE_CONTAINER_HEIGHT {
            32.0
        } else {
            40.0
        }
    }

    /// The size of a trailing part's icon at `button_height`.
    pub fn trailing_button_icon_size_for(button_height: f64) -> f64 {
        if button_height < Self::MEDIUM_CONTAINER_HEIGHT {
            22.0
        } else if button_height < Self::LARGE_CONTAINER_HEIGHT {
            26.0
        } else if button_height < Self::EXTRA_LARGE_CONTAINER_HEIGHT {
            38.0
        } else {
            50.0
        }
    }
}

/// A shape whose start corners are `start` and end corners `end`.
fn rounded(start: Radius, end: Radius) -> OutlinedBorderRef {
    OutlinedBorderRef::new(RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadiusGeometry::BorderRadius(BorderRadius::only(start, end, start, end)),
    ))
}

/// Compose's `CornerFull`, as a radius: half of the part's height.
fn full_corner(button_height: f64) -> Radius {
    Radius::circular(button_height / 2.0)
}

/// The resting and pressed inner corners at `button_height`, from the split button's tokens.
fn inner_corners_for(button_height: f64) -> (Radius, Radius) {
    use SplitButtonDefaults as tokens;
    let x_small = tokens::EXTRA_SMALL_CONTAINER_HEIGHT;
    let small = tokens::SMALL_CONTAINER_HEIGHT;
    let medium = tokens::MEDIUM_CONTAINER_HEIGHT;
    let large = tokens::LARGE_CONTAINER_HEIGHT;
    let x_large = tokens::EXTRA_LARGE_CONTAINER_HEIGHT;
    if button_height <= (x_small + small) / 2.0 {
        (Radius::circular(4.0), Radius::circular(8.0))
    } else if button_height <= (medium + large) / 2.0 {
        (Radius::circular(4.0), Radius::circular(12.0))
    } else if button_height <= (large + x_large) / 2.0 {
        (Radius::circular(8.0), Radius::circular(20.0))
    } else {
        (Radius::circular(12.0), Radius::circular(20.0))
    }
}

/// A split button's style: the part's variant and the state-dependent shape it draws with.
fn part_style(
    variant: SplitButtonVariant,
    shapes: &SplitButtonShapes,
    content_padding: EdgeInsetsGeometry,
    checked: bool,
) -> ButtonStyle {
    let shape: WidgetStatePropertyRef<Option<waft_painting::OutlinedBorderRef>> = {
        let resting = shapes.shape.clone();
        let pressed = shapes.pressed_shape.clone();
        let checked_shape = shapes.checked_shape.clone();
        <dyn WidgetStateProperty<Option<waft_painting::OutlinedBorderRef>>>::resolve_with(
            move |states| {
                if states.contains(&WidgetState::Pressed) {
                    return Some(pressed_shape_or(&pressed, &resting));
                }
                if checked {
                    return Some(checked_shape_or(&checked_shape, &resting));
                }
                Some(resting.clone())
            },
        )
    };
    let _ = variant;
    let padding: WidgetStatePropertyRef<Option<EdgeInsetsGeometry>> =
        <dyn WidgetStateProperty<Option<EdgeInsetsGeometry>>>::all(Some(content_padding));
    let minimum_size: WidgetStatePropertyRef<Option<Size>> =
        <dyn WidgetStateProperty<Option<Size>>>::all(Some(Size::new(
            K_SPLIT_BUTTON_MIN_WIDTH,
            SplitButtonDefaults::SMALL_CONTAINER_HEIGHT,
        )));
    ButtonStyle::new()
        .shape(shape)
        .padding(padding)
        .minimum_size(minimum_size)
        .animation_duration(crate::constants::K_THEME_CHANGE_DURATION)
}

fn pressed_shape_or(
    pressed: &Option<OutlinedBorderRef>,
    resting: &OutlinedBorderRef,
) -> OutlinedBorderRef {
    pressed.clone().unwrap_or_else(|| resting.clone())
}

fn checked_shape_or(
    checked: &Option<OutlinedBorderRef>,
    resting: &OutlinedBorderRef,
) -> OutlinedBorderRef {
    checked.clone().unwrap_or_else(|| resting.clone())
}

/// The leading part of a split button: Compose's `SplitButtonDefaults.LeadingButton` and its
/// tonal, outlined and elevated companions.
pub struct SplitButtonLeadingButton {
    pub key: Option<KeyRef>,
    /// The action the part performs; `None` disables it, as an absent `onClick` does in Flutter.
    pub on_pressed: Option<Listener>,
    /// The button style the part takes.
    pub variant: SplitButtonVariant,
    /// The shapes the part morphs between.
    pub shapes: SplitButtonShapes,
    /// The padding between the part's edge and its content.
    pub content_padding: EdgeInsetsGeometry,
    /// The part's content.
    pub child: WidgetRef,
}

impl SplitButtonLeadingButton {
    /// Creates a filled leading part; Compose's `LeadingButton(onClick, content)`.
    pub fn new<K>(
        on_pressed: Option<Listener>,
        child: impl IntoWidget<K>,
    ) -> SplitButtonLeadingButton {
        let shapes = SplitButtonDefaults::leading_button_shapes_for(
            SplitButtonDefaults::SMALL_CONTAINER_HEIGHT,
        );
        SplitButtonLeadingButton {
            key: None,
            on_pressed,
            variant: SplitButtonVariant::Filled,
            shapes,
            content_padding: SplitButtonDefaults::leading_button_content_padding_for(
                SplitButtonDefaults::SMALL_CONTAINER_HEIGHT,
            ),
            child: child.into_widget(),
        }
    }

    /// Dart `key:`.
    pub fn key(mut self, key: KeyRef) -> SplitButtonLeadingButton {
        self.key = Some(key);
        self
    }

    /// Compose's `TonalLeadingButton`, `OutlinedLeadingButton` and `ElevatedLeadingButton`.
    pub fn variant(mut self, variant: SplitButtonVariant) -> SplitButtonLeadingButton {
        self.variant = variant;
        self
    }

    /// Compose's `shapes:` argument.
    pub fn shapes(mut self, shapes: SplitButtonShapes) -> SplitButtonLeadingButton {
        self.shapes = shapes;
        self
    }

    /// Compose's `contentPadding:` argument.
    pub fn content_padding(
        mut self,
        content_padding: impl Into<EdgeInsetsGeometry>,
    ) -> SplitButtonLeadingButton {
        self.content_padding = content_padding.into();
        self
    }
}

impl fmt::Debug for SplitButtonLeadingButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SplitButtonLeadingButton")
            .field("variant", &self.variant)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for SplitButtonLeadingButton {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let style = part_style(self.variant, &self.shapes, self.content_padding, false);
        let child = self.child.clone();
        match self.variant {
            SplitButtonVariant::Filled => FilledButton::new(self.on_pressed.clone(), Some(child))
                .style(style)
                .into_widget(),
            SplitButtonVariant::Tonal => FilledButton::tonal(self.on_pressed.clone(), Some(child))
                .style(style)
                .into_widget(),
            SplitButtonVariant::Elevated => {
                ElevatedButton::new(self.on_pressed.clone(), Some(child))
                    .style(style)
                    .into_widget()
            }
            SplitButtonVariant::Outlined => {
                OutlinedButton::new(self.on_pressed.clone(), Some(child))
                    .style(style)
                    .into_widget()
            }
        }
    }
}

/// The trailing part of a split button: Compose's `SplitButtonDefaults.TrailingButton` and its
/// tonal, outlined and elevated companions, both as a plain button and as a toggle.
pub struct SplitButtonTrailingButton {
    pub key: Option<KeyRef>,
    /// The action the part performs; `None` disables it.
    pub on_pressed: Option<Listener>,
    /// Whether the toggle is checked; `None` for the plain button.
    pub checked: Option<bool>,
    /// The part's variant.
    pub variant: SplitButtonVariant,
    /// The shapes the part morphs between.
    pub shapes: SplitButtonShapes,
    /// The padding between the part's edge and its content.
    pub content_padding: EdgeInsetsGeometry,
    /// The part's content.
    pub child: WidgetRef,
}

impl SplitButtonTrailingButton {
    /// Creates a filled trailing part that acts as a button; Compose's
    /// `TrailingButton(onClick, content)`.
    pub fn new<K>(
        on_pressed: Option<Listener>,
        child: impl IntoWidget<K>,
    ) -> SplitButtonTrailingButton {
        let shapes = SplitButtonDefaults::trailing_button_shapes_for(
            SplitButtonDefaults::SMALL_CONTAINER_HEIGHT,
        );
        SplitButtonTrailingButton {
            key: None,
            on_pressed,
            checked: None,
            variant: SplitButtonVariant::Filled,
            shapes,
            content_padding: SplitButtonDefaults::trailing_button_content_padding_for(
                SplitButtonDefaults::SMALL_CONTAINER_HEIGHT,
            ),
            child: child.into_widget(),
        }
    }

    /// Creates a filled trailing part that acts as a toggle; Compose's
    /// `TrailingButton(checked, onCheckedChange, content)`. `on_pressed` receives the value the
    /// toggle should take next.
    pub fn toggle<K>(
        checked: bool,
        on_pressed: Option<Listener>,
        child: impl IntoWidget<K>,
    ) -> SplitButtonTrailingButton {
        let mut button = SplitButtonTrailingButton::new(on_pressed, child);
        button.checked = Some(checked);
        button
    }

    /// Dart `key:`.
    pub fn key(mut self, key: KeyRef) -> SplitButtonTrailingButton {
        self.key = Some(key);
        self
    }

    /// Compose's `TonalTrailingButton`, `OutlinedTrailingButton` and `ElevatedTrailingButton`.
    pub fn variant(mut self, variant: SplitButtonVariant) -> SplitButtonTrailingButton {
        self.variant = variant;
        self
    }

    /// Compose's `shapes:` argument.
    pub fn shapes(mut self, shapes: SplitButtonShapes) -> SplitButtonTrailingButton {
        self.shapes = shapes;
        self
    }

    /// Compose's `contentPadding:` argument.
    pub fn content_padding(
        mut self,
        content_padding: impl Into<EdgeInsetsGeometry>,
    ) -> SplitButtonTrailingButton {
        self.content_padding = content_padding.into();
        self
    }
}

impl fmt::Debug for SplitButtonTrailingButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SplitButtonTrailingButton")
            .field("variant", &self.variant)
            .field("checked", &self.checked)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for SplitButtonTrailingButton {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let style = part_style(
            self.variant,
            &self.shapes,
            self.content_padding,
            self.checked.unwrap_or(false),
        );
        let child = self.child.clone();
        match self.variant {
            SplitButtonVariant::Filled => FilledButton::new(self.on_pressed.clone(), Some(child))
                .style(style)
                .into_widget(),
            SplitButtonVariant::Tonal => FilledButton::tonal(self.on_pressed.clone(), Some(child))
                .style(style)
                .into_widget(),
            SplitButtonVariant::Elevated => {
                ElevatedButton::new(self.on_pressed.clone(), Some(child))
                    .style(style)
                    .into_widget()
            }
            SplitButtonVariant::Outlined => {
                OutlinedButton::new(self.on_pressed.clone(), Some(child))
                    .style(style)
                    .into_widget()
            }
        }
    }
}

/// Compose's `SplitButtonLayout`: the leading part, the trailing part, and the spacing between
/// them, at one height.
///
/// Compose measures the trailing part first, so it keeps its width when the pair must shrink, and
/// forces both parts to the taller intrinsic height. Here an [`IntrinsicHeight`] and a
/// [`Row`] with stretched children give both parts one height, and the row lays the leading part
/// out first (PORTING.md).
pub struct SplitButtonLayout {
    pub key: Option<KeyRef>,
    /// The part that performs the primary action.
    pub leading_button: WidgetRef,
    /// The part that performs the related secondary action.
    pub trailing_button: WidgetRef,
    /// The space between the two parts.
    pub spacing: f64,
}

impl SplitButtonLayout {
    /// Creates a split button; Compose's `SplitButtonLayout(leadingButton, trailingButton)`.
    pub fn new<L, T>(
        leading_button: impl IntoWidget<L>,
        trailing_button: impl IntoWidget<T>,
    ) -> SplitButtonLayout {
        SplitButtonLayout {
            key: None,
            leading_button: leading_button.into_widget(),
            trailing_button: trailing_button.into_widget(),
            spacing: K_SPLIT_BUTTON_SPACING,
        }
    }

    /// Dart `key:`.
    pub fn key(mut self, key: KeyRef) -> SplitButtonLayout {
        self.key = Some(key);
        self
    }

    /// Compose's `spacing:` argument.
    pub fn spacing(mut self, spacing: f64) -> SplitButtonLayout {
        self.spacing = spacing;
        self
    }
}

impl fmt::Debug for SplitButtonLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SplitButtonLayout")
            .field("spacing", &self.spacing)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for SplitButtonLayout {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        IntrinsicHeight::new()
            .child(
                Row::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(waft_rendering::CrossAxisAlignment::Stretch)
                    .children(vec![
                        self.leading_button.clone(),
                        SizedBox::new().width(self.spacing).into_widget(),
                        self.trailing_button.clone(),
                    ]),
            )
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use waft_widgets::testing::{WidgetTester, find, test_widgets};
    use waft_widgets::{Center, IntoWidget, Row, SizedBox, Text};

    use super::*;
    use crate::icons::Icons;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;
    use waft_widgets::Icon;

    fn pump(tester: &mut WidgetTester, layout: SplitButtonLayout) {
        let theme = ThemeData::builder().build(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            Center::new().child(layout).into_widget(),
        ));
    }

    fn recorded(flag: Rc<Cell<bool>>) -> Listener {
        Listener::new(move |_app| flag.set(true))
    }

    fn leading_with_icon_and_text(
        on_pressed: Option<Listener>,
        icon_size: f64,
        icon_spacing: f64,
    ) -> SplitButtonLeadingButton {
        SplitButtonLeadingButton::new(
            on_pressed,
            Row::new()
                .children(vec![
                    Icon::new(Some(Icons::edit())).size(icon_size).into_widget(),
                    SizedBox::new().width(icon_spacing).into_widget(),
                    Text::new("My Button").into_widget(),
                ])
                .into_widget(),
        )
    }

    fn trailing_with_icon(on_pressed: Option<Listener>) -> SplitButtonTrailingButton {
        SplitButtonTrailingButton::toggle(
            false,
            on_pressed,
            Icon::new(Some(Icons::arrow_drop_down())),
        )
    }

    #[test]
    fn a_filled_split_button_shows_both_parts_and_reports_their_presses() {
        test_widgets(|tester| {
            let leading_pressed = Rc::new(Cell::new(false));
            let trailing_pressed = Rc::new(Cell::new(false));
            pump(
                tester,
                SplitButtonLayout::new(
                    leading_with_icon_and_text(
                        Some(recorded(Rc::clone(&leading_pressed))),
                        20.0,
                        4.0,
                    ),
                    trailing_with_icon(Some(recorded(Rc::clone(&trailing_pressed)))),
                ),
            );

            assert_eq!(find::text("My Button").evaluate(tester).len(), 1);
            assert_eq!(find::by_icon(&Icons::edit()).evaluate(tester).len(), 1);
            assert_eq!(
                find::by_icon(&Icons::arrow_drop_down())
                    .evaluate(tester)
                    .len(),
                1
            );

            tester.tap(find::text("My Button"));
            assert!(leading_pressed.get());

            tester.tap(find::by_icon(&Icons::arrow_drop_down()));
            assert!(trailing_pressed.get());
        });
    }

    #[test]
    fn a_tonal_elevated_and_outlined_split_button_build() {
        test_widgets(|tester| {
            for variant in [
                SplitButtonVariant::Tonal,
                SplitButtonVariant::Elevated,
                SplitButtonVariant::Outlined,
            ] {
                pump(
                    tester,
                    SplitButtonLayout::new(
                        leading_with_icon_and_text(Some(Listener::new(|_app| {})), 20.0, 4.0)
                            .variant(variant),
                        trailing_with_icon(Some(Listener::new(|_app| {}))).variant(variant),
                    ),
                );
                assert_eq!(find::text("My Button").evaluate(tester).len(), 1);
            }
        });
    }

    /// Compose's positioning cases: the content sits between the size's start and end paddings,
    /// with the icon separated from the text by `icon_spacing`.
    fn check_leading_padding(tester: &mut WidgetTester, height: f64) {
        let shapes = SplitButtonDefaults::leading_button_shapes_for(height);
        let content_padding = SplitButtonDefaults::leading_button_content_padding_for(height);
        let icon_size = SplitButtonDefaults::leading_button_icon_size_for(height);
        let icon_spacing = if height < SplitButtonDefaults::SMALL_CONTAINER_HEIGHT {
            4.0
        } else {
            8.0
        };
        pump(
            tester,
            SplitButtonLayout::new(
                SizedBox::new().height(height).child(
                    leading_with_icon_and_text(
                        Some(Listener::new(|_app| {})),
                        icon_size,
                        icon_spacing,
                    )
                    .shapes(shapes)
                    .content_padding(content_padding),
                ),
                trailing_with_icon(None),
            ),
        );

        let button = tester.get_rect(find::by_type::<SplitButtonLeadingButton>());
        let icon = tester.get_rect(find::by_icon(&Icons::edit()));
        let text = tester.get_rect(find::text("My Button"));
        let padding = content_padding;
        let resolved = padding.resolve(Some(waft_embedder::TextDirection::Ltr));
        assert_eq!(icon.left - button.left, resolved.left);
        assert_eq!(text.left - icon.right, icon_spacing);
        assert_eq!(button.right - text.right, resolved.right);
    }

    #[test]
    fn an_extra_small_leading_part_uses_its_start_and_end_paddings() {
        test_widgets(|tester| {
            check_leading_padding(tester, SplitButtonDefaults::EXTRA_SMALL_CONTAINER_HEIGHT);
        });
    }

    #[test]
    fn a_small_leading_part_uses_its_start_and_end_paddings() {
        test_widgets(|tester| {
            check_leading_padding(tester, SplitButtonDefaults::SMALL_CONTAINER_HEIGHT);
        });
    }

    fn part_heights(tester: &WidgetTester) -> (f64, f64) {
        (
            tester
                .get_size(find::by_type::<SplitButtonLeadingButton>())
                .height(),
            tester
                .get_size(find::by_type::<SplitButtonTrailingButton>())
                .height(),
        )
    }

    #[test]
    fn a_split_button_stretches_both_parts_to_the_taller_one() {
        test_widgets(|tester| {
            pump(
                tester,
                SplitButtonLayout::new(
                    SizedBox::new()
                        .height(40.0)
                        .child(leading_with_icon_and_text(
                            Some(Listener::new(|_app| {})),
                            20.0,
                            4.0,
                        )),
                    SizedBox::new().height(80.0).child(trailing_with_icon(None)),
                ),
            );
            assert_eq!(part_heights(tester), (80.0, 80.0));
        });
    }

    #[test]
    fn a_split_button_matches_the_leading_part_to_a_taller_trailing_one() {
        test_widgets(|tester| {
            pump(
                tester,
                SplitButtonLayout::new(
                    SizedBox::new()
                        .height(80.0)
                        .child(leading_with_icon_and_text(
                            Some(Listener::new(|_app| {})),
                            20.0,
                            4.0,
                        )),
                    SizedBox::new().height(34.0).child(trailing_with_icon(None)),
                ),
            );
            assert_eq!(part_heights(tester), (80.0, 80.0));
        });
    }

    #[test]
    fn a_split_button_spaces_its_parts_by_the_given_spacing() {
        test_widgets(|tester| {
            pump(
                tester,
                SplitButtonLayout::new(
                    leading_with_icon_and_text(Some(Listener::new(|_app| {})), 20.0, 4.0),
                    trailing_with_icon(None),
                )
                .spacing(12.0),
            );
            let leading = tester.get_rect(find::by_type::<SplitButtonLeadingButton>());
            let trailing = tester.get_rect(find::by_type::<SplitButtonTrailingButton>());
            assert_eq!(trailing.left - leading.right, 12.0);
        });
    }
}
