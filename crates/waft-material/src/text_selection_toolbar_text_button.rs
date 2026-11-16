//! Flutter counterpart: material_ui's `text_selection_toolbar_text_button.dart`.

use std::fmt::{self, Debug};

use waft_embedder::{Brightness, Color, FontWeight, Size};
use waft_foundation::{App, Listener};
use waft_painting::{
    AlignmentGeometry, EdgeInsetsDirectional, EdgeInsetsGeometry, RoundedRectangleBorder, TextStyle,
};
use waft_widgets::{BuildContext, IntoWidget, KeyRef, StatelessWidget, WidgetRef};

use crate::color_scheme::ColorScheme;
use crate::constants::K_MIN_INTERACTIVE_DIMENSION;
use crate::text_button::TextButton;
use crate::theme::Theme;
use crate::theme_data::ThemeData;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextSelectionToolbarItemPosition {
    /// The first item among multiple in the menu.
    First,

    /// One of several items, not the first or last.
    Middle,

    /// The last item among multiple in the menu.
    Last,

    /// The only item in the menu.
    Only,
}

/// A button styled like a Material native Android text selection menu button.
#[derive(Clone)]
pub struct TextSelectionToolbarTextButton {
    pub key: Option<KeyRef>,
    /// The child of this button.
    ///
    /// Usually a `Text`.
    pub child: WidgetRef,
    /// Called when this button is pressed.
    pub on_pressed: Option<Listener>,
    /// The padding between the button's edge and its child.
    ///
    /// In a standard Material `TextSelectionToolbar`, the padding depends on the
    /// button's position within the toolbar.
    ///
    /// See also:
    ///
    ///  * [`get_padding`](Self::get_padding), which calculates the standard padding based on
    ///    the button's position.
    ///  * `ButtonStyle::padding`, which is where this padding is applied.
    pub padding: EdgeInsetsGeometry,
    /// The alignment of the button's child.
    ///
    /// By default, this will be `Alignment::CENTER`.
    ///
    /// See also:
    ///
    ///  * `ButtonStyle::alignment`, which is where this alignment is applied.
    pub alignment: Option<AlignmentGeometry>,
}

impl Debug for TextSelectionToolbarTextButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextSelectionToolbarTextButton")
            .field("padding", &self.padding)
            .field("alignment", &self.alignment)
            .finish_non_exhaustive()
    }
}

impl TextSelectionToolbarTextButton {
    // These values were eyeballed to match the native text selection menu on a
    // Pixel 2 running Android 10.
    const K_MIDDLE_PADDING: f64 = 9.5;
    const K_END_PADDING: f64 = 14.5;

    // These colors were taken from a screenshot of a Pixel 6 emulator running
    // Android API level 34.
    const DEFAULT_FOREGROUND_COLOR_LIGHT: Color = Color::new(0xff000000);
    const DEFAULT_FOREGROUND_COLOR_DARK: Color = Color::new(0xffffffff);

    // The background color is hardcoded to transparent by default so the buttons
    // are the color of the container behind them. For example TextSelectionToolbar
    // hardcodes the color value, and TextSelectionToolbarTextButtons that are its
    // children become that color.
    const DEFAULT_BACKGROUND_COLOR_TRANSPARENT: Color = Color::new(0x00000000);

    /// Creates an instance of TextSelectionToolbarTextButton.
    pub fn new<K>(
        child: impl IntoWidget<K>,
        padding: impl Into<EdgeInsetsGeometry>,
    ) -> TextSelectionToolbarTextButton {
        TextSelectionToolbarTextButton {
            key: None,
            child: child.into_widget(),
            on_pressed: None,
            padding: padding.into(),
            alignment: None,
        }
    }

    /// Dart `TextSelectionToolbarTextButton(key:)`.
    pub fn key(mut self, key: KeyRef) -> TextSelectionToolbarTextButton {
        self.key = Some(key);
        self
    }

    /// Dart `TextSelectionToolbarTextButton.copyWith(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> TextSelectionToolbarTextButton {
        self.child = child.into_widget();
        self
    }

    /// Dart `TextSelectionToolbarTextButton(onPressed:)`.
    pub fn on_pressed(mut self, on_pressed: Listener) -> TextSelectionToolbarTextButton {
        self.on_pressed = Some(on_pressed);
        self
    }

    /// Dart `TextSelectionToolbarTextButton.copyWith(padding:)`.
    pub fn padding(
        mut self,
        padding: impl Into<EdgeInsetsGeometry>,
    ) -> TextSelectionToolbarTextButton {
        self.padding = padding.into();
        self
    }

    /// Dart `TextSelectionToolbarTextButton(alignment:)`.
    pub fn alignment(
        mut self,
        alignment: impl Into<AlignmentGeometry>,
    ) -> TextSelectionToolbarTextButton {
        self.alignment = Some(alignment.into());
        self
    }

    /// Returns the standard padding for a button at index out of a total number
    /// of buttons.
    ///
    /// Standard Material `TextSelectionToolbar`s have buttons with different
    /// padding depending on their position in the toolbar.
    pub fn get_padding(index: usize, total: usize) -> EdgeInsetsGeometry {
        debug_assert!(total > 0 && index < total);
        let position = Self::get_position(index, total);
        EdgeInsetsDirectional::only(
            Self::get_start_padding(position),
            0.0,
            Self::get_end_padding(position),
            0.0,
        )
        .into()
    }

    fn get_start_padding(position: TextSelectionToolbarItemPosition) -> f64 {
        if position == TextSelectionToolbarItemPosition::First
            || position == TextSelectionToolbarItemPosition::Only
        {
            return Self::K_END_PADDING;
        }
        Self::K_MIDDLE_PADDING
    }

    fn get_end_padding(position: TextSelectionToolbarItemPosition) -> f64 {
        if position == TextSelectionToolbarItemPosition::Last
            || position == TextSelectionToolbarItemPosition::Only
        {
            return Self::K_END_PADDING;
        }
        Self::K_MIDDLE_PADDING
    }

    fn get_position(index: usize, total: usize) -> TextSelectionToolbarItemPosition {
        if index == 0 {
            return if total == 1 {
                TextSelectionToolbarItemPosition::Only
            } else {
                TextSelectionToolbarItemPosition::First
            };
        }
        if index == total - 1 {
            return TextSelectionToolbarItemPosition::Last;
        }
        TextSelectionToolbarItemPosition::Middle
    }

    /// Returns a copy of the current [`TextSelectionToolbarTextButton`] instance
    /// with specific overrides.
    ///
    /// Like Dart's, the copy has no key.
    pub fn copy_with(&self) -> TextSelectionToolbarTextButton {
        TextSelectionToolbarTextButton {
            key: None,
            ..self.clone()
        }
    }

    fn get_foreground_color(app: &App, color_scheme: &ColorScheme) -> Color {
        let is_default_on_surface = match color_scheme.brightness {
            Brightness::Light => {
                ThemeData::new(app).color_scheme.on_surface == color_scheme.on_surface
            }
            Brightness::Dark => {
                ThemeData::dark(app, None).color_scheme.on_surface == color_scheme.on_surface
            }
        };
        if !is_default_on_surface {
            return color_scheme.on_surface;
        }
        match color_scheme.brightness {
            Brightness::Light => Self::DEFAULT_FOREGROUND_COLOR_LIGHT,
            Brightness::Dark => Self::DEFAULT_FOREGROUND_COLOR_DARK,
        }
    }
}

impl StatelessWidget for TextSelectionToolbarTextButton {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let color_scheme = Theme::of(app, context).color_scheme.clone();
        let mut style = TextButton::style_from()
            .background_color(Self::DEFAULT_BACKGROUND_COLOR_TRANSPARENT)
            .foreground_color(Self::get_foreground_color(app, &color_scheme))
            .shape(RoundedRectangleBorder::default())
            .minimum_size(Size::new(
                K_MIN_INTERACTIVE_DIMENSION,
                K_MIN_INTERACTIVE_DIMENSION,
            ))
            .padding(self.padding);
        if let Some(alignment) = self.alignment {
            style = style.alignment(alignment);
        }
        // This value was eyeballed from a screenshot of a Pixel 6 emulator
        // running Android API level 34.
        style = style.text_style(TextStyle::new().font_weight(FontWeight::W400));
        TextButton::new(self.on_pressed.clone(), Some(self.child.clone()))
            .style(style.build())
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use waft_painting::EdgeInsets;
    use waft_widgets::{Center, SizedBox, StateSetter, StatefulBuilder, Text, WidgetStates};

    use waft_widgets::testing::{WidgetTester, test_widgets};

    use super::*;
    use crate::button_style::ButtonStyle;
    use crate::button_style_button::ButtonStyleButton;
    use crate::colors::Colors;
    #[allow(deprecated)]
    use crate::material_state::MaterialStatePropertyAll;
    use crate::scaffold::Scaffold;
    use crate::test_support::material_app;
    use crate::text_button_theme::TextButtonThemeData;

    /// Dart's `<ColorScheme>[ThemeData().colorScheme, ThemeData.dark().colorScheme]`.
    fn color_schemes(tester: &WidgetTester) -> [ColorScheme; 2] {
        let app = tester.cell.borrow();
        [
            ThemeData::new(&app).color_scheme,
            ThemeData::dark(&app, None).color_scheme,
        ]
    }

    /// Dart's `ThemeData(colorScheme: color_scheme)`.
    fn themed(tester: &WidgetTester, color_scheme: ColorScheme) -> ThemeData {
        ThemeData::builder()
            .color_scheme(color_scheme)
            .build(&tester.cell.borrow())
    }

    /// `MaterialApp(theme: theme, home: Scaffold(body: Center(child: button)))`.
    fn pump_button(tester: &mut WidgetTester, theme: ThemeData) {
        let button = TextSelectionToolbarTextButton::new(
            Text::new("button"),
            TextSelectionToolbarTextButton::get_padding(0, 1),
        );
        tester.pump_widget(material_app(
            theme,
            Scaffold::new()
                .body(Center::new().child(button))
                .into_widget(),
        ));
    }

    /// Dart's `tester.widget<TextButton>(find.byType(TextButton)).style!`.
    fn text_button_style(tester: &WidgetTester) -> ButtonStyle {
        let buttons = tester.find_by_type::<TextButton>();
        assert_eq!(buttons.len(), 1, "one TextButton");
        tester.widget(buttons[0], |button: &TextButton| {
            button
                .button_style_button_data()
                .style
                .clone()
                .expect("the button has a style")
        })
    }

    #[test]
    fn position_in_the_toolbar_changes_width() {
        test_widgets(|tester| {
            let position = Rc::new(RefCell::new((1, 3)));
            let set_state: Rc<RefCell<Option<StateSetter>>> = Rc::new(RefCell::new(None));
            let root = {
                let app = tester.cell.borrow();
                let position = Rc::clone(&position);
                let set_state = Rc::clone(&set_state);
                material_app(
                    ThemeData::new(&app),
                    Scaffold::new()
                        .body(Center::new().child(StatefulBuilder::new(
                            move |_app, _context, setter| {
                                *set_state.borrow_mut() = Some(setter);
                                let (index, total) = *position.borrow();
                                TextSelectionToolbarTextButton::new(
                                    Text::new("button"),
                                    TextSelectionToolbarTextButton::get_padding(index, total),
                                )
                                .into_widget()
                            },
                        )))
                        .into_widget(),
                )
            };
            tester.pump_widget(root);
            let size = |tester: &WidgetTester| {
                let buttons = tester.find_by_type::<TextSelectionToolbarTextButton>();
                let render = tester.render_box_of(buttons[0]);
                render.size(&tester.cell.borrow())
            };
            let set = |tester: &mut WidgetTester, index: usize, total: usize| {
                let setter = set_state.borrow().clone().expect("built");
                let position = Rc::clone(&position);
                setter(&mut tester.cell.borrow_mut(), &mut |_app| {
                    *position.borrow_mut() = (index, total);
                });
                tester.pump_by(std::time::Duration::ZERO);
            };

            let middle_size = size(tester);

            set(tester, 0, 3);
            let first_size = size(tester);
            assert!(first_size.width() > middle_size.width());

            set(tester, 2, 3);
            let last_size = size(tester);
            assert!(last_size.width() > middle_size.width());
            assert_eq!(last_size.width(), first_size.width());

            set(tester, 0, 1);
            let only_size = size(tester);
            assert!(only_size.width() > middle_size.width());
            assert!(only_size.width() > first_size.width());
            assert!(only_size.width() > last_size.width());
        });
    }

    #[test]
    fn foreground_color_by_default() {
        for index in 0..2 {
            test_widgets(|tester| {
                let color_scheme = color_schemes(tester)[index].clone();
                let brightness = color_scheme.brightness;
                let theme = themed(tester, color_scheme);
                pump_button(tester, theme);

                // The foreground color is hardcoded to black or white by default, not the
                // default value from ColorScheme.onSurface.
                let style = text_button_style(tester);
                assert_eq!(
                    style
                        .foreground_color
                        .expect("a foreground color")
                        .resolve(&WidgetStates::new()),
                    Some(match brightness {
                        Brightness::Light => Color::new(0xff000000),
                        Brightness::Dark => Color::new(0xffffffff),
                    })
                );
            });
        }
    }

    #[test]
    fn custom_foreground_color() {
        for index in 0..2 {
            test_widgets(|tester| {
                let color_scheme = color_schemes(tester)[index].clone();
                let custom_foreground_color = Colors::RED.primary();
                let theme = themed(
                    tester,
                    color_scheme
                        .copy_with()
                        .with_on_surface(custom_foreground_color),
                );
                pump_button(tester, theme);

                let style = text_button_style(tester);
                assert_eq!(
                    style
                        .foreground_color
                        .expect("a foreground color")
                        .resolve(&WidgetStates::new()),
                    Some(custom_foreground_color)
                );
            });
        }
    }

    #[test]
    fn background_color_by_default() {
        for index in 0..2 {
            test_widgets(|tester| {
                let color_scheme = color_schemes(tester)[index].clone();
                let theme = themed(tester, color_scheme);
                pump_button(tester, theme);

                // The background color is hardcoded to transparent by default so the buttons
                // are the color of the container behind them. For example TextSelectionToolbar
                // hardcodes the color value, and TextSelectionToolbarTextButton that are its
                // children should be that color.
                let style = text_button_style(tester);
                assert_eq!(
                    style
                        .background_color
                        .expect("a background color")
                        .resolve(&WidgetStates::new()),
                    Some(Colors::TRANSPARENT)
                );
            });
        }
    }

    #[test]
    #[allow(deprecated)]
    fn text_button_theme_should_not_override_default_background_color() {
        for index in 0..2 {
            test_widgets(|tester| {
                let color_scheme = color_schemes(tester)[index].clone();
                let theme = ThemeData::builder()
                    .color_scheme(color_scheme)
                    .text_button_theme(TextButtonThemeData::new().style(
                        ButtonStyle::new().background_color(Rc::new(
                            MaterialStatePropertyAll::new(Some(Colors::BLUE.primary())),
                        )),
                    ))
                    .build(&tester.cell.borrow());
                pump_button(tester, theme);

                // The background color is hardcoded to transparent by default so the buttons
                // are the color of the container behind them. For example TextSelectionToolbar
                // hardcodes the color value, and TextSelectionToolbarTextButton that are its
                // children should be that color.
                let style = text_button_style(tester);
                assert_eq!(
                    style
                        .background_color
                        .expect("a background color")
                        .resolve(&WidgetStates::new()),
                    Some(Colors::TRANSPARENT)
                );
            });
        }
    }

    #[test]
    fn text_selection_toolbar_text_button_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let root = {
                let app = tester.cell.borrow();
                material_app(
                    ThemeData::new(&app),
                    Center::new()
                        .child(
                            SizedBox::shrink().child(TextSelectionToolbarTextButton::new(
                                Text::new("X"),
                                EdgeInsets::all(5.0),
                            )),
                        )
                        .into_widget(),
                )
            };
            tester.pump_widget(root);
            let buttons = tester.find_by_type::<TextSelectionToolbarTextButton>();
            let render = tester.render_box_of(buttons[0]);
            assert_eq!(render.size(&tester.cell.borrow()), Size::ZERO);
        });
    }

    #[test]
    fn get_padding_gives_the_end_padding_on_the_outer_edges() {
        let padding = |index, total| match TextSelectionToolbarTextButton::get_padding(index, total)
        {
            EdgeInsetsGeometry::Directional(directional) => (directional.start, directional.end),
            other => panic!("directional padding, not {other:?}"),
        };
        assert_eq!(padding(0, 1), (14.5, 14.5));
        assert_eq!(padding(0, 3), (14.5, 9.5));
        assert_eq!(padding(1, 3), (9.5, 9.5));
        assert_eq!(padding(2, 3), (9.5, 14.5));
    }
}
