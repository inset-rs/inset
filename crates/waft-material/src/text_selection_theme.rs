//! Flutter counterpart: material_ui's `text_selection_theme.dart`.

use waft_embedder::Color;
use waft_foundation::App;
use waft_widgets::{
    BuildContext, DefaultSelectionStyle, InheritedTheme, InheritedWidget, IntoWidget, KeyRef,
    WidgetRef,
};

use crate::theme::Theme;

/// Defines the text selection visual properties for descendant `TextField` and
/// `SelectableText` widgets.
///
/// Descendant widgets obtain the current [`TextSelectionThemeData`] object using
/// [`TextSelectionTheme::of`]. Instances of [`TextSelectionThemeData`] can be customized
/// with [`TextSelectionThemeData::copy_with`].
///
/// Typically a [`TextSelectionThemeData`] is specified as part of the overall `Theme`
/// with `ThemeData::text_selection_theme`.
///
/// See also:
///
///  * [`TextSelectionTheme`], an `InheritedWidget` that propagates the theme down its
///    subtree.
///  * `InputDecorationThemeData`, which defines most other visual properties of
///    text fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextSelectionThemeData {
    /// The color of the cursor in the text field.
    ///
    /// The cursor indicates the current location of text insertion point in
    /// the field.
    pub cursor_color: Option<Color>,

    /// The background color of selected text.
    pub selection_color: Option<Color>,

    /// The color of the selection handles on the text field.
    ///
    /// Selection handles are used to indicate the bounds of the selected text,
    /// or as a handle to drag the cursor to a new location in the text.
    ///
    /// On iOS `TextField` and `SelectableText` cannot access
    /// [`selection_handle_color`](Self::selection_handle_color).
    /// To set the [`selection_handle_color`](Self::selection_handle_color) on iOS, you can
    /// change the `CupertinoThemeData::selection_handle_color` by wrapping the subtree
    /// containing your `TextField` or `SelectableText` with a `CupertinoTheme`.
    pub selection_handle_color: Option<Color>,
}

impl TextSelectionThemeData {
    /// Creates the set of properties used to configure `TextField`s; Dart's optional named
    /// arguments are the fluent setters.
    pub fn new() -> TextSelectionThemeData {
        TextSelectionThemeData::default()
    }

    /// Dart `TextSelectionThemeData(cursorColor:)`.
    pub fn cursor_color(mut self, cursor_color: Color) -> TextSelectionThemeData {
        self.cursor_color = Some(cursor_color);
        self
    }

    /// Dart `TextSelectionThemeData(selectionColor:)`.
    pub fn selection_color(mut self, selection_color: Color) -> TextSelectionThemeData {
        self.selection_color = Some(selection_color);
        self
    }

    /// Dart `TextSelectionThemeData(selectionHandleColor:)`.
    pub fn selection_handle_color(
        mut self,
        selection_handle_color: Color,
    ) -> TextSelectionThemeData {
        self.selection_handle_color = Some(selection_handle_color);
        self
    }

    /// Creates a copy of this object with the given fields replaced with the
    /// specified values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().cursor_color(color)`).
    pub fn copy_with(&self) -> TextSelectionThemeData {
        self.clone()
    }

    /// Linearly interpolate between two text field themes.
    ///
    /// If both arguments are null, then null is returned.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&TextSelectionThemeData>,
        b: Option<&TextSelectionThemeData>,
        t: f64,
    ) -> Option<TextSelectionThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(TextSelectionThemeData {
            cursor_color: Color::lerp(
                a.and_then(|a| a.cursor_color),
                b.and_then(|b| b.cursor_color),
                t,
            ),
            selection_color: Color::lerp(
                a.and_then(|a| a.selection_color),
                b.and_then(|b| b.selection_color),
                t,
            ),
            selection_handle_color: Color::lerp(
                a.and_then(|a| a.selection_handle_color),
                b.and_then(|b| b.selection_handle_color),
                t,
            ),
        })
    }
}

/// An inherited widget that defines the appearance of text selection in
/// this widget's subtree.
///
/// Values specified here are used for `TextField` and `SelectableText`
/// properties that are not given an explicit non-null value.
///
/// Here is an example of a text selection theme that applies a blue cursor
/// color with light blue selection handles to the child text field.
///
/// ```text
/// TextSelectionTheme::new(
///     TextSelectionThemeData::new()
///         .cursor_color(Colors::BLUE.primary())
///         .selection_handle_color(Colors::LIGHT_BLUE.primary()),
///     TextField::new(),
/// )
/// ```
///
/// This widget also creates a [`DefaultSelectionStyle`] for its subtree with
/// [`data`](Self::data).
#[derive(Debug)]
pub struct TextSelectionTheme {
    pub key: Option<KeyRef>,
    /// The properties for descendant `TextField` and `SelectableText` widgets.
    pub data: TextSelectionThemeData,
    // Dart's `child` getter inserts the `DefaultSelectionStyle` into the subtree; here the
    // inserted widget is built by the constructor and held, as `InheritedWidget::child`
    // answers a reference.
    child: WidgetRef,
}

impl TextSelectionTheme {
    /// Creates a text selection theme widget that specifies the text
    /// selection properties for all widgets below it in the widget tree.
    pub fn new<K>(data: TextSelectionThemeData, child: impl IntoWidget<K>) -> TextSelectionTheme {
        let mut default_selection_style = DefaultSelectionStyle::new(child);
        if let Some(selection_color) = data.selection_color {
            default_selection_style = default_selection_style.selection_color(selection_color);
        }
        if let Some(cursor_color) = data.cursor_color {
            default_selection_style = default_selection_style.cursor_color(cursor_color);
        }
        TextSelectionTheme {
            key: None,
            data,
            child: default_selection_style.into_widget(),
        }
    }

    /// Dart `TextSelectionTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> TextSelectionTheme {
        self.key = Some(key);
        self
    }

    /// Returns the [`data`](Self::data) from the closest [`TextSelectionTheme`] ancestor. If
    /// there is no ancestor, it returns `ThemeData::text_selection_theme`.
    /// Applications can assume that the returned value will not be null.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = TextSelectionTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> TextSelectionThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<TextSelectionTheme>(app) {
            Some(selection_theme) => selection_theme.data.clone(),
            None => Theme::of(app, context).text_selection_theme.clone(),
        }
    }
}

impl InheritedTheme for TextSelectionTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        TextSelectionTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for TextSelectionTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &TextSelectionTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use waft_painting::AnyColor;
    use waft_widgets::{Builder, SizedBox};

    use waft_widgets::testing::test_widgets;

    use super::*;
    use crate::colors::Colors;
    use crate::test_support::probe;
    use crate::theme_data::ThemeData;

    fn any(color: Color) -> AnyColor {
        AnyColor::new(color)
    }

    #[test]
    fn text_selection_theme_data_copy_with_and_equality_basics() {
        assert_eq!(
            TextSelectionThemeData::new(),
            TextSelectionThemeData::new().copy_with()
        );
    }

    #[test]
    fn text_selection_theme_data_lerp_special_cases() {
        assert_eq!(TextSelectionThemeData::lerp(None, None, 0.0), None);
        let data = TextSelectionThemeData::new();
        assert_eq!(
            TextSelectionThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    #[test]
    fn text_selection_theme_data_lerps_each_color() {
        let a = TextSelectionThemeData::new().cursor_color(Colors::BLACK);
        let b = TextSelectionThemeData::new()
            .cursor_color(Colors::WHITE)
            .selection_color(Color::new(0x88888888));

        let lerped = TextSelectionThemeData::lerp(Some(&a), Some(&b), 0.5).expect("both given");

        assert_eq!(
            lerped.cursor_color,
            Color::lerp(Some(Colors::BLACK), Some(Colors::WHITE), 0.5)
        );
        assert_eq!(
            lerped.selection_color,
            Color::lerp(None, Some(Color::new(0x88888888)), 0.5)
        );
        assert_eq!(lerped.selection_handle_color, None);
    }

    #[test]
    fn text_selection_theme_data_null_fields_by_default() {
        let theme = TextSelectionThemeData::new();
        assert_eq!(theme.cursor_color, None);
        assert_eq!(theme.selection_color, None);
        assert_eq!(theme.selection_handle_color, None);
    }

    #[test]
    fn text_selection_theme_data_copy_with_replaces_given_fields() {
        let theme = TextSelectionThemeData::new().cursor_color(Colors::RED.primary());

        let copy = theme.copy_with().selection_color(Colors::BLUE.primary());

        assert_eq!(copy.cursor_color, Some(Colors::RED.primary()));
        assert_eq!(copy.selection_color, Some(Colors::BLUE.primary()));
        assert_ne!(copy, theme);
    }

    #[test]
    fn cursor_color() {
        assert_eq!(
            TextSelectionThemeData::new()
                .cursor_color(Colors::RED.primary())
                .cursor_color,
            Some(Colors::RED.primary())
        );
    }

    #[test]
    fn text_selection_theme_overrides_default_selection_style() {
        test_widgets(|tester| {
            const THEME_SELECTION_COLOR: Color = Color::new(0xffaabbcc);
            const THEME_CURSOR_COLOR: Color = Color::new(0x00ccbbaa);
            const DEFAULT_SELECTION_COLOR: Color = Color::new(0xffaa1111);
            const DEFAULT_CURSOR_COLOR: Color = Color::new(0x00cc2222);
            type Seen = Rc<RefCell<Option<DefaultSelectionStyle>>>;
            let outer: Seen = Rc::default();
            let inner: Seen = Rc::default();
            let recording = |seen: &Seen, child: Option<WidgetRef>| {
                let seen = Rc::clone(seen);
                Builder::new(move |app, context| {
                    *seen.borrow_mut() = Some(DefaultSelectionStyle::of(app, context));
                    child
                        .clone()
                        .unwrap_or_else(|| SizedBox::shrink().into_widget())
                })
                .into_widget()
            };
            let theme = TextSelectionTheme::new(
                TextSelectionThemeData::new()
                    .selection_color(THEME_SELECTION_COLOR)
                    .cursor_color(THEME_CURSOR_COLOR),
                recording(&inner, None),
            );
            let root = DefaultSelectionStyle::new(recording(&outer, Some(theme.into_widget())))
                .selection_color(DEFAULT_SELECTION_COLOR)
                .cursor_color(DEFAULT_CURSOR_COLOR)
                .into_widget();

            tester.pump_widget(root);

            let style = outer.borrow_mut().take().expect("the outer probe built");
            assert_eq!(style.selection_color, Some(any(DEFAULT_SELECTION_COLOR)));
            assert_eq!(style.cursor_color, Some(any(DEFAULT_CURSOR_COLOR)));
            let style = inner.borrow_mut().take().expect("the inner probe built");
            assert_eq!(style.selection_color, Some(any(THEME_SELECTION_COLOR)));
            assert_eq!(style.cursor_color, Some(any(THEME_CURSOR_COLOR)));
        });
    }

    #[test]
    fn of_returns_the_theme_data_text_selection_theme_without_an_ancestor() {
        test_widgets(|tester| {
            let theme_data_selection = TextSelectionThemeData::new()
                .cursor_color(Color::new(0xffaabbcc))
                .selection_handle_color(Color::new(0x00ccbbaa));
            let expected = theme_data_selection.clone();

            let seen = probe(
                tester,
                move |app, child| {
                    let data = ThemeData::builder()
                        .text_selection_theme(theme_data_selection.clone())
                        .build(app);
                    Theme::new(data, child).into_widget()
                },
                TextSelectionTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn of_returns_the_closest_text_selection_theme_over_theme_data() {
        test_widgets(|tester| {
            let widget_selection = TextSelectionThemeData::new()
                .cursor_color(Color::new(0xffddeeff))
                .selection_color(Color::new(0x44444444))
                .selection_handle_color(Color::new(0x00ffeedd));
            let expected = widget_selection.clone();

            let seen = probe(
                tester,
                move |app, child| {
                    let data = ThemeData::builder()
                        .text_selection_theme(
                            TextSelectionThemeData::new().cursor_color(Color::new(0xffaabbcc)),
                        )
                        .build(app);
                    Theme::new(
                        data,
                        TextSelectionTheme::new(widget_selection.clone(), child),
                    )
                    .into_widget()
                },
                TextSelectionTheme::of,
            );

            assert_eq!(seen, expected);
        });
    }
}
