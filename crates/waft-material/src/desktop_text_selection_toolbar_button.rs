//! Flutter counterpart: material_ui's `desktop_text_selection_toolbar_button.dart`.

use std::fmt::{self, Debug};

use waft_embedder::{Brightness, FontWeight, Size};
use waft_foundation::{App, Listener};
use waft_painting::{
    AlignmentGeometry, EdgeInsets, RoundedRectangleBorder, TextOverflow, TextStyle,
};
use waft_services::SystemMouseCursors;
use waft_widgets::{BuildContext, IntoWidget, KeyRef, SizedBox, StatelessWidget, Text, WidgetRef};

use crate::colors::Colors;
use crate::constants::K_MIN_INTERACTIVE_DIMENSION;
use crate::text_button::TextButton;
use crate::theme::Theme;

fn k_toolbar_button_font_style() -> TextStyle {
    TextStyle::new()
        .inherit(false)
        .font_size(14.0)
        .letter_spacing(-0.15)
        .font_weight(FontWeight::W400)
}

const K_TOOLBAR_BUTTON_PADDING: EdgeInsets = EdgeInsets::from_ltrb(20.0, 0.0, 20.0, 3.0);

/// A [`TextButton`] for the Material desktop text selection toolbar.
pub struct DesktopTextSelectionToolbarButton {
    pub key: Option<KeyRef>,
    /// Called when this button is pressed.
    pub on_pressed: Option<Listener>,
    /// The child of this button.
    ///
    /// Usually a [`Text`].
    pub child: WidgetRef,
}

impl Debug for DesktopTextSelectionToolbarButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DesktopTextSelectionToolbarButton")
            .finish_non_exhaustive()
    }
}

impl DesktopTextSelectionToolbarButton {
    /// Creates an instance of DesktopTextSelectionToolbarButton.
    pub fn new<K>(
        on_pressed: Option<Listener>,
        child: impl IntoWidget<K>,
    ) -> DesktopTextSelectionToolbarButton {
        DesktopTextSelectionToolbarButton {
            key: None,
            on_pressed,
            child: child.into_widget(),
        }
    }

    /// Create an instance of [`DesktopTextSelectionToolbarButton`] whose child is
    /// a [`Text`] widget in the style of the Material text selection toolbar.
    pub fn text(
        app: &mut App,
        context: BuildContext,
        on_pressed: Option<Listener>,
        text: impl Into<String>,
    ) -> DesktopTextSelectionToolbarButton {
        let color = if Theme::of(app, context).color_scheme.brightness() == Brightness::Dark {
            Colors::WHITE
        } else {
            Colors::BLACK87
        };
        DesktopTextSelectionToolbarButton {
            key: None,
            on_pressed,
            child: Text::new(text)
                .overflow(TextOverflow::Ellipsis)
                .style(k_toolbar_button_font_style().copy_with().color(color))
                .into_widget(),
        }
    }

    /// Dart `DesktopTextSelectionToolbarButton(key:)`.
    pub fn key(mut self, key: KeyRef) -> DesktopTextSelectionToolbarButton {
        self.key = Some(key);
        self
    }
}

impl StatelessWidget for DesktopTextSelectionToolbarButton {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        // TODO(hansmuller): Should be colorScheme.onSurface
        let theme = Theme::of(app, context);
        let is_dark = theme.color_scheme.brightness() == Brightness::Dark;
        let foreground_color = if is_dark {
            Colors::WHITE
        } else {
            Colors::BLACK87
        };

        SizedBox::new()
            .width(f64::INFINITY)
            .child(
                TextButton::new(self.on_pressed.clone(), Some(self.child.clone())).style(
                    TextButton::style_from()
                        .alignment(AlignmentGeometry::CENTER_LEFT)
                        .enabled_mouse_cursor(SystemMouseCursors::BASIC.into())
                        .disabled_mouse_cursor(SystemMouseCursors::BASIC.into())
                        .foreground_color(foreground_color)
                        .shape(RoundedRectangleBorder::default())
                        .minimum_size(Size::new(K_MIN_INTERACTIVE_DIMENSION, 36.0))
                        .padding(K_TOOLBAR_BUTTON_PADDING.into())
                        .build(),
                ),
            )
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use waft_widgets::Center;
    use waft_widgets::testing::{WidgetTester, find, test_widgets};

    use super::*;
    use crate::button_style_button::ButtonStyleButton;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    /// `MaterialApp(home: Center(child: child))`.
    fn pump_centered<K>(tester: &mut WidgetTester, child: impl IntoWidget<K>) {
        let root = {
            let app = tester.cell.borrow();
            material_app(
                ThemeData::new(&app),
                Center::new().child(child).into_widget(),
            )
        };
        tester.pump_widget(root);
    }

    #[test]
    fn can_press() {
        test_widgets(|tester| {
            let pressed = Rc::new(Cell::new(false));
            let on_pressed = {
                let pressed = Rc::clone(&pressed);
                Listener::new(move |_app| pressed.set(true))
            };
            pump_centered(
                tester,
                DesktopTextSelectionToolbarButton::new(Some(on_pressed), Text::new("Tap me")),
            );

            assert!(!pressed.get());

            let buttons = find::by_type::<DesktopTextSelectionToolbarButton>().evaluate(tester);
            let center = tester.get_rect(buttons[0]).center();
            tester.tap_at(center, 1);
            assert!(pressed.get());
        });
    }

    #[test]
    fn passing_null_to_on_pressed_disables_the_button() {
        test_widgets(|tester| {
            pump_centered(
                tester,
                DesktopTextSelectionToolbarButton::new(None, Text::new("Cannot tap me")),
            );

            let buttons = find::by_type::<TextButton>().evaluate(tester);
            assert_eq!(buttons.len(), 1);
            assert!(!tester.widget(buttons[0], |button: &TextButton| button.enabled()));
        });
    }

    #[test]
    fn desktop_text_selection_toolbar_button_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            pump_centered(
                tester,
                SizedBox::shrink()
                    .child(DesktopTextSelectionToolbarButton::new(None, Text::new("X"))),
            );
            let buttons = find::by_type::<DesktopTextSelectionToolbarButton>().evaluate(tester);
            assert_eq!(tester.get_size(buttons[0]), Size::ZERO);
        });
    }
}
