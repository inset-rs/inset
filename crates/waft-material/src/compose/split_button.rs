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
use waft_foundation::{App, Listener, ValueChanged};
use waft_painting::{
    BorderRadiusDirectional, BorderRadiusGeometry, BorderSide, EdgeInsetsDirectional,
    EdgeInsetsGeometry, OutlinedBorderRef, RoundedRectangleBorder,
};
use waft_widgets::{
    BuildContext, CrossAxisAlignment, Flexible, IntoWidget, IntrinsicHeight, KeyRef, MainAxisSize,
    Row, SizedBox, StatelessWidget, WidgetRef, WidgetState, WidgetStateProperty,
    WidgetStatePropertyRef,
};

use crate::button_style::ButtonStyle;
use crate::elevated_button::ElevatedButton;
use crate::filled_button::FilledButton;
use crate::outlined_button::OutlinedButton;

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
    /// The default spacing between a split button's two parts.
    pub const SPACING: f64 = 2.0;

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

/// A shape whose start corners are `start` and end corners `end`, flipped for right-to-left text
/// as Compose's `RoundedCornerShape(topStart, topEnd, bottomEnd, bottomStart)` is.
fn rounded(start: Radius, end: Radius) -> OutlinedBorderRef {
    OutlinedBorderRef::new(RoundedRectangleBorder::new(
        BorderSide::NONE,
        BorderRadiusGeometry::Directional(BorderRadiusDirectional::only(start, end, start, end)),
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

/// A part's style: the state-dependent shape, padding and minimum size the split layers over the
/// variant's own style.
fn part_style(
    shapes: &SplitButtonShapes,
    content_padding: EdgeInsetsGeometry,
    checked: bool,
) -> ButtonStyle {
    let resting = shapes.shape.clone();
    let pressed = shapes.pressed_shape.clone();
    let checked_shape = shapes.checked_shape.clone();
    let shape: WidgetStatePropertyRef<Option<OutlinedBorderRef>> =
        <dyn WidgetStateProperty<Option<OutlinedBorderRef>>>::resolve_with(move |states| {
            let shape = if states.contains(&WidgetState::Pressed) {
                pressed.as_ref()
            } else if checked {
                checked_shape.as_ref()
            } else {
                None
            };
            Some(shape.unwrap_or(&resting).clone())
        });
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

/// The Flutter button that draws a part of `variant`, with `style` over its own.
fn part_button(
    variant: SplitButtonVariant,
    on_pressed: Option<Listener>,
    style: ButtonStyle,
    child: WidgetRef,
) -> WidgetRef {
    match variant {
        SplitButtonVariant::Filled => FilledButton::new(on_pressed, Some(child))
            .style(style)
            .into_widget(),
        SplitButtonVariant::Tonal => FilledButton::tonal(on_pressed, Some(child))
            .style(style)
            .into_widget(),
        SplitButtonVariant::Elevated => ElevatedButton::new(on_pressed, Some(child))
            .style(style)
            .into_widget(),
        SplitButtonVariant::Outlined => OutlinedButton::new(on_pressed, Some(child))
            .style(style)
            .into_widget(),
    }
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
        part_button(
            self.variant,
            self.on_pressed.clone(),
            part_style(&self.shapes, self.content_padding, false),
            self.child.clone(),
        )
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
    /// `TrailingButton(checked, onCheckedChange, content)`. A press calls `on_checked_change` with
    /// `!checked`; `None` disables the part.
    pub fn toggle<K>(
        checked: bool,
        on_checked_change: Option<ValueChanged<bool>>,
        child: impl IntoWidget<K>,
    ) -> SplitButtonTrailingButton {
        let on_pressed = on_checked_change.map(|on_checked_change| {
            Listener::new(move |app: &mut App| on_checked_change(app, !checked))
        });
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
        part_button(
            self.variant,
            self.on_pressed.clone(),
            part_style(
                &self.shapes,
                self.content_padding,
                self.checked.unwrap_or(false),
            ),
            self.child.clone(),
        )
    }
}

/// Compose's `SplitButtonLayout`: the leading part, the trailing part, and the spacing between
/// them, at one height.
///
/// Compose measures the trailing part first, so it keeps its width when the pair must shrink, and
/// forces both parts to the taller intrinsic height. Here an [`IntrinsicHeight`] and a [`Row`]
/// with stretched children give both parts one height, and the leading part is a loose
/// [`Flexible`], so the row lays the trailing part out first and gives the leading part what is
/// left.
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
            spacing: SplitButtonDefaults::SPACING,
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
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .children(vec![
                        Flexible::new(self.leading_button.clone()).into_widget(),
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
    use waft_widgets::{
        Axis, Center, Icon, IntoWidget, Row, SingleChildScrollView, SizedBox, StatefulBuilder, Text,
    };

    use super::*;
    use crate::icons::Icons;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    fn pump<K>(tester: &mut WidgetTester, layout: impl IntoWidget<K>) {
        let theme = ThemeData::builder().build(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            Center::new().child(layout).into_widget(),
        ));
    }

    fn recorded(flag: Rc<Cell<bool>>) -> Listener {
        Listener::new(move |_app| flag.set(true))
    }

    fn ignored() -> Option<Listener> {
        Some(Listener::new(|_app| {}))
    }

    /// Compose's `ButtonDefaults.iconSpacingFor`, which the positioning cases space the icon by;
    /// its small and medium spacings are both 8.
    fn icon_spacing_for(height: f64) -> f64 {
        if height < SplitButtonDefaults::SMALL_CONTAINER_HEIGHT {
            4.0
        } else if height < SplitButtonDefaults::LARGE_CONTAINER_HEIGHT {
            8.0
        } else if height < SplitButtonDefaults::EXTRA_LARGE_CONTAINER_HEIGHT {
            12.0
        } else {
            16.0
        }
    }

    fn leading_with_icon_and_text(
        on_pressed: Option<Listener>,
        icon_size: f64,
        icon_spacing: f64,
    ) -> SplitButtonLeadingButton {
        SplitButtonLeadingButton::new(
            on_pressed,
            Row::new()
                .main_axis_size(MainAxisSize::Min)
                .children(vec![
                    Icon::new(Some(Icons::edit())).size(icon_size).into_widget(),
                    SizedBox::new().width(icon_spacing).into_widget(),
                    Text::new("My Button").into_widget(),
                ])
                .into_widget(),
        )
    }

    fn trailing_toggle(
        checked: bool,
        on_checked_change: Option<ValueChanged<bool>>,
    ) -> SplitButtonTrailingButton {
        SplitButtonTrailingButton::toggle(
            checked,
            on_checked_change,
            Icon::new(Some(Icons::keyboard_arrow_down())),
        )
    }

    fn label_leading() -> SplitButtonLeadingButton {
        SplitButtonLeadingButton::new(ignored(), Text::new("Label"))
    }

    fn icon_trailing() -> SplitButtonTrailingButton {
        SplitButtonTrailingButton::new(ignored(), Icon::new(Some(Icons::keyboard_arrow_down())))
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

    fn assert_content_displayed(tester: &WidgetTester) {
        assert_eq!(find::text("My Button").evaluate(tester).len(), 1);
        assert_eq!(find::by_icon(&Icons::edit()).evaluate(tester).len(), 1);
        assert_eq!(
            find::by_icon(&Icons::keyboard_arrow_down())
                .evaluate(tester)
                .len(),
            1
        );
    }

    /// Compose's `filledSplitButton_contentDisplay`: both parts' content shows and both parts
    /// act on a press.
    #[test]
    fn filled_split_button_content_display() {
        test_widgets(|tester| {
            let leading_pressed = Rc::new(Cell::new(false));
            let trailing_pressed = Rc::new(Cell::new(false));
            let on_checked_change: ValueChanged<bool> = {
                let trailing_pressed = Rc::clone(&trailing_pressed);
                Rc::new(move |_app, _checked| trailing_pressed.set(true))
            };
            pump(
                tester,
                SplitButtonLayout::new(
                    leading_with_icon_and_text(
                        Some(recorded(Rc::clone(&leading_pressed))),
                        24.0,
                        8.0,
                    ),
                    SizedBox::new()
                        .width(34.0)
                        .height(34.0)
                        .child(trailing_toggle(false, Some(on_checked_change))),
                ),
            );

            assert_content_displayed(tester);

            tester.tap(find::text("My Button"));
            assert!(leading_pressed.get());
            tester.tap(find::by_icon(&Icons::keyboard_arrow_down()));
            assert!(trailing_pressed.get());
        });
    }

    /// Compose's `filledSplitButton_trailingButtonChecked`: the trailing toggle starts off and a
    /// press turns it on.
    #[test]
    fn filled_split_button_trailing_button_checked() {
        test_widgets(|tester| {
            let checked = Rc::new(Cell::new(false));
            let layout = {
                let checked = Rc::clone(&checked);
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let on_checked_change: ValueChanged<bool> = {
                        let checked = Rc::clone(&checked);
                        Rc::new(move |app, value| {
                            let checked = Rc::clone(&checked);
                            set_state(app, &mut |_app| checked.set(value));
                        })
                    };
                    SplitButtonLayout::new(
                        leading_with_icon_and_text(ignored(), 24.0, 8.0),
                        SizedBox::new()
                            .width(34.0)
                            .height(34.0)
                            .child(trailing_toggle(checked.get(), Some(on_checked_change))),
                    )
                    .into_widget()
                })
            };
            pump(tester, layout);

            assert_eq!(tester_checked(tester), Some(false));
            tester.tap(find::by_icon(&Icons::keyboard_arrow_down()));
            tester.pump();
            assert!(checked.get());
            assert_eq!(tester_checked(tester), Some(true));
        });
    }

    fn tester_checked(tester: &WidgetTester) -> Option<bool> {
        tester.widget(
            find::by_type::<SplitButtonTrailingButton>(),
            |button: &SplitButtonTrailingButton| button.checked,
        )
    }

    /// Compose's `TonalSplitButton_contentDisplay`, `ElevatedSplitButton_contentDisplay` and
    /// `OutlinedSplitButton_contentDisplay`.
    #[test]
    fn tonal_elevated_and_outlined_split_buttons_display_their_content() {
        test_widgets(|tester| {
            for variant in [
                SplitButtonVariant::Tonal,
                SplitButtonVariant::Elevated,
                SplitButtonVariant::Outlined,
            ] {
                pump(
                    tester,
                    SplitButtonLayout::new(
                        leading_with_icon_and_text(ignored(), 24.0, 8.0).variant(variant),
                        SizedBox::new().width(34.0).height(34.0).child(
                            trailing_toggle(false, Some(Rc::new(|_app, _checked| {})))
                                .variant(variant),
                        ),
                    ),
                );
                assert_content_displayed(tester);
            }
        });
    }

    /// Compose's `filledSplitButton_*_positioning`: the content sits between the size's start
    /// and end paddings, with the icon separated from the text by the size's icon spacing.
    fn check_leading_positioning(
        tester: &mut WidgetTester,
        height: f64,
        start: f64,
        between: f64,
        end: f64,
    ) {
        pump(
            tester,
            SplitButtonLayout::new(
                SizedBox::new().height(height).child(
                    leading_with_icon_and_text(ignored(), 24.0, icon_spacing_for(height))
                        .shapes(SplitButtonDefaults::leading_button_shapes_for(height))
                        .content_padding(SplitButtonDefaults::leading_button_content_padding_for(
                            height,
                        )),
                ),
                SizedBox::new().height(height).child(
                    trailing_toggle(false, Some(Rc::new(|_app, _checked| {})))
                        .shapes(SplitButtonDefaults::trailing_button_shapes_for(height))
                        .content_padding(SplitButtonDefaults::trailing_button_content_padding_for(
                            height,
                        )),
                ),
            ),
        );

        let button = tester.get_rect(find::by_type::<SplitButtonLeadingButton>());
        let icon = tester.get_rect(find::by_icon(&Icons::edit()));
        let text = tester.get_rect(find::text("My Button"));
        assert_is_equal_to(
            icon.left - button.left,
            start,
            "start padding for leading button",
        );
        assert_is_equal_to(
            text.left - icon.right,
            between,
            "icon and text between spacing for leading button",
        );
        assert_is_equal_to(
            button.right - text.right,
            end,
            "end padding for leading button",
        );
    }

    /// Compose's `Dp.assertIsEqualTo`, whose default tolerance is half a pixel.
    fn assert_is_equal_to(actual: f64, expected: f64, subject: &str) {
        assert!(
            (actual - expected).abs() <= 0.5,
            "{subject}: expected {expected}, got {actual}"
        );
    }

    #[test]
    fn filled_split_button_extra_small_positioning() {
        test_widgets(|tester| {
            check_leading_positioning(
                tester,
                SplitButtonDefaults::EXTRA_SMALL_CONTAINER_HEIGHT,
                12.0,
                4.0,
                10.0,
            );
        });
    }

    #[test]
    fn filled_split_button_small_positioning() {
        test_widgets(|tester| {
            check_leading_positioning(
                tester,
                SplitButtonDefaults::SMALL_CONTAINER_HEIGHT,
                16.0,
                8.0,
                12.0,
            );
        });
    }

    #[test]
    fn filled_split_button_medium_positioning() {
        test_widgets(|tester| {
            check_leading_positioning(
                tester,
                SplitButtonDefaults::MEDIUM_CONTAINER_HEIGHT,
                24.0,
                8.0,
                24.0,
            );
        });
    }

    #[test]
    fn filled_split_button_large_positioning() {
        test_widgets(|tester| {
            check_leading_positioning(
                tester,
                SplitButtonDefaults::LARGE_CONTAINER_HEIGHT,
                48.0,
                12.0,
                48.0,
            );
        });
    }

    #[test]
    fn filled_split_button_extra_large_positioning() {
        test_widgets(|tester| {
            check_leading_positioning(
                tester,
                SplitButtonDefaults::EXTRA_LARGE_CONTAINER_HEIGHT,
                64.0,
                16.0,
                64.0,
            );
        });
    }

    /// Compose's `splitButton_leadingFillMaxWidth_horizontalScroll_noCrash`, without the
    /// `fillMaxWidth`, which has no Flutter equivalent under unbounded width: a split button laid
    /// out at unbounded width still shows its label.
    #[test]
    fn split_button_in_a_horizontal_scroll_shows_its_label() {
        test_widgets(|tester| {
            pump(
                tester,
                SingleChildScrollView::new()
                    .scroll_direction(Axis::Horizontal)
                    .child(SplitButtonLayout::new(label_leading(), icon_trailing())),
            );
            assert_eq!(find::text("Label").evaluate(tester).len(), 1);
            assert!(tester.get_size(find::text("Label")).width() > 0.0);
        });
    }

    /// Compose's `splitButton_stretchesButtonsToMaxHeight`.
    #[test]
    fn split_button_stretches_buttons_to_max_height() {
        test_widgets(|tester| {
            pump(
                tester,
                SizedBox::new()
                    .height(100.0)
                    .child(SplitButtonLayout::new(label_leading(), icon_trailing())),
            );
            assert_eq!(part_heights(tester), (100.0, 100.0));
        });
    }

    /// Compose's `splitButton_trailingMatchLeadingHeight`.
    #[test]
    fn split_button_trailing_matches_leading_height() {
        test_widgets(|tester| {
            pump(
                tester,
                SplitButtonLayout::new(
                    SizedBox::new().height(100.0).child(label_leading()),
                    icon_trailing(),
                ),
            );
            assert_eq!(part_heights(tester), (100.0, 100.0));
        });
    }

    /// Compose's `splitButton_leadingMatchTrailingHeight`.
    #[test]
    fn split_button_leading_matches_trailing_height() {
        test_widgets(|tester| {
            pump(
                tester,
                SplitButtonLayout::new(
                    label_leading(),
                    SizedBox::new().height(100.0).child(icon_trailing()),
                ),
            );
            assert_eq!(part_heights(tester), (100.0, 100.0));
        });
    }

    /// Not a Compose test: the layout's spacing separates the parts.
    #[test]
    fn a_split_button_spaces_its_parts_by_the_given_spacing() {
        test_widgets(|tester| {
            pump(
                tester,
                SplitButtonLayout::new(label_leading(), icon_trailing()).spacing(12.0),
            );
            let leading = tester.get_rect(find::by_type::<SplitButtonLeadingButton>());
            let trailing = tester.get_rect(find::by_type::<SplitButtonTrailingButton>());
            assert_eq!(trailing.left - leading.right, 12.0);
        });
    }

    /// Not a Compose test: as Compose's measure policy does, a width too small for the pair
    /// shrinks the leading part and leaves the trailing part its own width.
    #[test]
    fn a_narrow_split_button_shrinks_the_leading_part() {
        test_widgets(|tester| {
            pump(
                tester,
                SplitButtonLayout::new(label_leading(), icon_trailing()),
            );
            let trailing_width = tester
                .get_size(find::by_type::<SplitButtonTrailingButton>())
                .width();

            pump(
                tester,
                SizedBox::new()
                    .width(100.0)
                    .child(SplitButtonLayout::new(label_leading(), icon_trailing())),
            );
            let leading = tester.get_size(find::by_type::<SplitButtonLeadingButton>());
            let trailing = tester.get_size(find::by_type::<SplitButtonTrailingButton>());
            assert_eq!(trailing.width(), trailing_width);
            assert_eq!(
                leading.width(),
                100.0 - SplitButtonDefaults::SPACING - trailing_width
            );
        });
    }
}
