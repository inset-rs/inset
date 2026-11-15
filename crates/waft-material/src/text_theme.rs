//! Flutter counterpart: material_ui's `text_theme.dart`.

use waft_embedder::{Color, FontFeature, FontVariation, TextDecoration, TextDecorationStyle};
use waft_foundation::App;
use waft_painting::TextStyle;
use waft_widgets::BuildContext;

use crate::theme::Theme;

/// Material design text theme.
///
/// Definitions for the various typographical styles found in Material Design
/// (e.g., label_large, body_small). Rather than creating a [`TextTheme`] directly,
/// you can obtain an instance as `Typography.black` or `Typography.white`.
///
/// To obtain the current text theme, call [`TextTheme::of`] with the current
/// `BuildContext`. This is equivalent to calling `Theme::of` and reading
/// the `ThemeData.text_theme` property.
///
/// The names of the TextTheme properties match this table from the
/// [Material Design spec](https://m3.material.io/styles/typography/tokens).
///
/// ![](https://lh3.googleusercontent.com/Yvngs5mQSjXa_9T4X3JDucO62c5hdZHPDa7qeRH6DsJQvGr_q7EBrTkhkPiQd9OeR1v_Uk38Cjd9nUpP3nevDyHpKWuXSfQ1Gq78bOnBN7sr=s0)
///
/// The Material Design typography scheme was significantly changed in the
/// current (2021) version of the specification
/// ([https://m3.material.io/styles/typography/tokens](https://m3.material.io/styles/typography/tokens)).
///
/// The **2021** spec has fifteen text styles:
///
/// | NAME           | SIZE |  HEIGHT |  WEIGHT |  SPACING |             |
/// |----------------|------|---------|---------|----------|-------------|
/// | displayLarge   | 57.0 |   64.0  | regular | -0.25    |             |
/// | displayMedium  | 45.0 |   52.0  | regular |  0.0     |             |
/// | displaySmall   | 36.0 |   44.0  | regular |  0.0     |             |
/// | headlineLarge  | 32.0 |   40.0  | regular |  0.0     |             |
/// | headlineMedium | 28.0 |   36.0  | regular |  0.0     |             |
/// | headlineSmall  | 24.0 |   32.0  | regular |  0.0     |             |
/// | titleLarge     | 22.0 |   28.0  | regular |  0.0     |             |
/// | titleMedium    | 16.0 |   24.0  | medium  |  0.15    |             |
/// | titleSmall     | 14.0 |   20.0  | medium  |  0.1     |             |
/// | bodyLarge      | 16.0 |   24.0  | regular |  0.5     |             |
/// | bodyMedium     | 14.0 |   20.0  | regular |  0.25    |             |
/// | bodySmall      | 12.0 |   16.0  | regular |  0.4     |             |
/// | labelLarge     | 14.0 |   20.0  | medium  |  0.1     |             |
/// | labelMedium    | 12.0 |   16.0  | medium  |  0.5     |             |
/// | labelSmall     | 11.0 |   16.0  | medium  |  0.5     |             |
///
/// ...where "regular" is `FontWeight.w400` and "medium" is `FontWeight.w500`.
///
/// The names of the 2018 TextTheme properties match this table from the
/// [Material Design spec](https://material.io/design/typography/the-type-system.html#type-scale)
/// with a few exceptions: the styles called H1-H6 in the spec are
/// displayLarge-titleLarge in the API chart, body1,body2 are called
/// bodyLarge and bodyMedium, caption is now bodySmall, button is labelLarge,
/// and overline is now labelSmall.
///
/// The **2018** spec has thirteen text styles:
///
/// | NAME           | SIZE |  WEIGHT |  SPACING |             |
/// |----------------|------|---------|----------|-------------|
/// | displayLarge   | 96.0 | light   | -1.5     |             |
/// | displayMedium  | 60.0 | light   | -0.5     |             |
/// | displaySmall   | 48.0 | regular |  0.0     |             |
/// | headlineMedium | 34.0 | regular |  0.25    |             |
/// | headlineSmall  | 24.0 | regular |  0.0     |             |
/// | titleLarge     | 20.0 | medium  |  0.15    |             |
/// | titleMedium    | 16.0 | regular |  0.15    |             |
/// | titleSmall     | 14.0 | medium  |  0.1     |             |
/// | bodyLarge      | 16.0 | regular |  0.5     |             |
/// | bodyMedium     | 14.0 | regular |  0.25    |             |
/// | bodySmall      | 12.0 | regular |  0.4     |             |
/// | labelLarge     | 14.0 | medium  |  1.25    |             |
/// | labelSmall     | 10.0 | regular |  1.5     |             |
///
/// ...where "light" is `FontWeight.w300`, "regular" is `FontWeight.w400` and
/// "medium" is `FontWeight.w500`.
///
/// By default, text styles are initialized to match the 2018 Material Design
/// specification as listed above. To provide backwards compatibility, the 2014
/// specification is also available.
///
/// To explicitly configure a `Theme` for the 2018 sizes, weights, and letter
/// spacings, you can initialize its `ThemeData.typography` value using
/// `Typography.material2018`. The `Typography` constructor defaults to this
/// configuration. To configure a `Theme` for the 2014 sizes, weights, and letter
/// spacings, initialize its `ThemeData.typography` value using
/// `Typography.material2014`.
///
/// See also:
///
///  * `Typography`, the class that generates [`TextTheme`]s appropriate for a platform.
///  * `Theme`, for other aspects of a Material Design application that can be
///    globally adjusted, such as the color scheme.
///  * <https://material.io/design/typography/>
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextTheme {
    /// Largest of the display styles.
    ///
    /// As the largest text on the screen, display styles are reserved for short,
    /// important text or numerals. They work best on large screens.
    pub display_large: Option<TextStyle>,
    /// Middle size of the display styles.
    ///
    /// As the largest text on the screen, display styles are reserved for short,
    /// important text or numerals. They work best on large screens.
    pub display_medium: Option<TextStyle>,
    /// Smallest of the display styles.
    ///
    /// As the largest text on the screen, display styles are reserved for short,
    /// important text or numerals. They work best on large screens.
    pub display_small: Option<TextStyle>,
    /// Largest of the headline styles.
    ///
    /// Headline styles are smaller than display styles. They're best-suited for
    /// short, high-emphasis text on smaller screens.
    pub headline_large: Option<TextStyle>,
    /// Middle size of the headline styles.
    ///
    /// Headline styles are smaller than display styles. They're best-suited for
    /// short, high-emphasis text on smaller screens.
    pub headline_medium: Option<TextStyle>,
    /// Smallest of the headline styles.
    ///
    /// Headline styles are smaller than display styles. They're best-suited for
    /// short, high-emphasis text on smaller screens.
    pub headline_small: Option<TextStyle>,
    /// Largest of the title styles.
    ///
    /// Titles are smaller than headline styles and should be used for shorter,
    /// medium-emphasis text.
    pub title_large: Option<TextStyle>,
    /// Middle size of the title styles.
    ///
    /// Titles are smaller than headline styles and should be used for shorter,
    /// medium-emphasis text.
    pub title_medium: Option<TextStyle>,
    /// Smallest of the title styles.
    ///
    /// Titles are smaller than headline styles and should be used for shorter,
    /// medium-emphasis text.
    pub title_small: Option<TextStyle>,
    /// Largest of the body styles.
    ///
    /// Body styles are used for longer passages of text.
    pub body_large: Option<TextStyle>,
    /// Middle size of the body styles.
    ///
    /// Body styles are used for longer passages of text.
    ///
    /// The default text style for `Material`.
    pub body_medium: Option<TextStyle>,
    /// Smallest of the body styles.
    ///
    /// Body styles are used for longer passages of text.
    pub body_small: Option<TextStyle>,
    /// Largest of the label styles.
    ///
    /// Label styles are smaller, utilitarian styles, used for areas of the UI
    /// such as text inside of components or very small supporting text in the
    /// content body, like captions.
    ///
    /// Used for text on `ElevatedButton`, `TextButton` and `OutlinedButton`.
    pub label_large: Option<TextStyle>,
    /// Middle size of the label styles.
    ///
    /// Label styles are smaller, utilitarian styles, used for areas of the UI
    /// such as text inside of components or very small supporting text in the
    /// content body, like captions.
    pub label_medium: Option<TextStyle>,
    /// Smallest of the label styles.
    ///
    /// Label styles are smaller, utilitarian styles, used for areas of the UI
    /// such as text inside of components or very small supporting text in the
    /// content body, like captions.
    pub label_small: Option<TextStyle>,
}

impl TextTheme {
    /// Creates a text theme that uses the given values.
    ///
    /// Rather than creating a new text theme, consider using `Typography.black`
    /// or `Typography.white`, which implement the typography styles in the
    /// Material Design specification:
    ///
    /// <https://material.io/design/typography/#type-scale>
    ///
    /// If you do decide to create your own text theme, consider using one of
    /// those predefined themes as a starting point for [`copy_with`](Self::copy_with) or
    /// [`apply`](Self::apply).
    ///
    /// The 2018 styles cannot be mixed with the 2021 styles. Only one or the
    /// other is allowed in this constructor. The 2018 styles are deprecated and
    /// will eventually be removed.
    pub fn new() -> TextTheme {
        TextTheme::default()
    }

    /// Dart `TextTheme(displayLarge:)`.
    pub fn display_large(mut self, display_large: TextStyle) -> TextTheme {
        self.display_large = Some(display_large);
        self
    }

    /// Dart `TextTheme(displayMedium:)`.
    pub fn display_medium(mut self, display_medium: TextStyle) -> TextTheme {
        self.display_medium = Some(display_medium);
        self
    }

    /// Dart `TextTheme(displaySmall:)`.
    pub fn display_small(mut self, display_small: TextStyle) -> TextTheme {
        self.display_small = Some(display_small);
        self
    }

    /// Dart `TextTheme(headlineLarge:)`.
    pub fn headline_large(mut self, headline_large: TextStyle) -> TextTheme {
        self.headline_large = Some(headline_large);
        self
    }

    /// Dart `TextTheme(headlineMedium:)`.
    pub fn headline_medium(mut self, headline_medium: TextStyle) -> TextTheme {
        self.headline_medium = Some(headline_medium);
        self
    }

    /// Dart `TextTheme(headlineSmall:)`.
    pub fn headline_small(mut self, headline_small: TextStyle) -> TextTheme {
        self.headline_small = Some(headline_small);
        self
    }

    /// Dart `TextTheme(titleLarge:)`.
    pub fn title_large(mut self, title_large: TextStyle) -> TextTheme {
        self.title_large = Some(title_large);
        self
    }

    /// Dart `TextTheme(titleMedium:)`.
    pub fn title_medium(mut self, title_medium: TextStyle) -> TextTheme {
        self.title_medium = Some(title_medium);
        self
    }

    /// Dart `TextTheme(titleSmall:)`.
    pub fn title_small(mut self, title_small: TextStyle) -> TextTheme {
        self.title_small = Some(title_small);
        self
    }

    /// Dart `TextTheme(bodyLarge:)`.
    pub fn body_large(mut self, body_large: TextStyle) -> TextTheme {
        self.body_large = Some(body_large);
        self
    }

    /// Dart `TextTheme(bodyMedium:)`.
    pub fn body_medium(mut self, body_medium: TextStyle) -> TextTheme {
        self.body_medium = Some(body_medium);
        self
    }

    /// Dart `TextTheme(bodySmall:)`.
    pub fn body_small(mut self, body_small: TextStyle) -> TextTheme {
        self.body_small = Some(body_small);
        self
    }

    /// Dart `TextTheme(labelLarge:)`.
    pub fn label_large(mut self, label_large: TextStyle) -> TextTheme {
        self.label_large = Some(label_large);
        self
    }

    /// Dart `TextTheme(labelMedium:)`.
    pub fn label_medium(mut self, label_medium: TextStyle) -> TextTheme {
        self.label_medium = Some(label_medium);
        self
    }

    /// Dart `TextTheme(labelSmall:)`.
    pub fn label_small(mut self, label_small: TextStyle) -> TextTheme {
        self.label_small = Some(label_small);
        self
    }

    /// Creates a copy of this text theme but with the given fields replaced with
    /// the new values.
    ///
    /// Consider using `Typography.black` or `Typography.white`, which implement
    /// the typography styles in the Material Design specification, as a starting
    /// point.
    ///
    /// Dart's named arguments are the setters on the copy:
    /// `theme.copy_with().title_large(style)`.
    ///
    /// See also:
    ///
    ///  * [`merge`](Self::merge) is used instead of [`copy_with`](Self::copy_with) when you
    ///    want to merge all of the fields of a TextTheme instead of individual fields.
    pub fn copy_with(&self) -> TextTheme {
        self.clone()
    }

    /// Creates a new [`TextTheme`] where each text style from this object has been
    /// merged with the matching text style from the `other` object.
    ///
    /// The merging is done by calling [`TextStyle::merge`] on each respective pair
    /// of text styles from this and the `other` text themes and is subject to
    /// the value of [`TextStyle::inherit`] flag. For more details, see the
    /// documentation on [`TextStyle::merge`] and [`TextStyle::inherit`].
    ///
    /// If this theme, or the `other` theme has members that are null, then the
    /// non-null one (if any) is used. If the `other` theme is itself null, then
    /// this [`TextTheme`] is returned unchanged. If values in both are set, then
    /// the values are merged using [`TextStyle::merge`].
    ///
    /// This is particularly useful if one [`TextTheme`] defines one set of
    /// properties and another defines a different set, e.g. having colors
    /// defined in one text theme and font sizes in another, or when one
    /// [`TextTheme`] has only some fields defined, and you want to define the rest
    /// by merging it with a default theme.
    ///
    /// See also:
    ///
    ///  * [`copy_with`](Self::copy_with) is used instead of [`merge`](Self::merge) when you
    ///    wish to override individual fields in the [`TextTheme`] instead of merging all of
    ///    the fields of two [`TextTheme`]s.
    pub fn merge(&self, other: Option<&TextTheme>) -> TextTheme {
        let Some(other) = other else {
            return self.clone();
        };

        fn merge_styles(
            style: &Option<TextStyle>,
            other_style: &Option<TextStyle>,
        ) -> Option<TextStyle> {
            match style {
                Some(style) => Some(style.merge(other_style.as_ref())),
                None => other_style.clone(),
            }
        }

        TextTheme {
            display_large: merge_styles(&self.display_large, &other.display_large),
            display_medium: merge_styles(&self.display_medium, &other.display_medium),
            display_small: merge_styles(&self.display_small, &other.display_small),
            headline_large: merge_styles(&self.headline_large, &other.headline_large),
            headline_medium: merge_styles(&self.headline_medium, &other.headline_medium),
            headline_small: merge_styles(&self.headline_small, &other.headline_small),
            title_large: merge_styles(&self.title_large, &other.title_large),
            title_medium: merge_styles(&self.title_medium, &other.title_medium),
            title_small: merge_styles(&self.title_small, &other.title_small),
            body_large: merge_styles(&self.body_large, &other.body_large),
            body_medium: merge_styles(&self.body_medium, &other.body_medium),
            body_small: merge_styles(&self.body_small, &other.body_small),
            label_large: merge_styles(&self.label_large, &other.label_large),
            label_medium: merge_styles(&self.label_medium, &other.label_medium),
            label_small: merge_styles(&self.label_small, &other.label_small),
        }
    }

    /// Creates a copy of this text theme but with the given field replaced in
    /// each of the individual text styles.
    ///
    /// The `display_color` is applied to [`display_large`](Self::display_large),
    /// [`display_medium`](Self::display_medium), [`display_small`](Self::display_small),
    /// [`headline_large`](Self::headline_large), [`headline_medium`](Self::headline_medium),
    /// and [`body_small`](Self::body_small). The `body_color` is applied to the remaining
    /// text styles.
    ///
    /// Consider using `Typography.black` or `Typography.white`, which implement
    /// the typography styles in the Material Design specification, as a starting
    /// point.
    ///
    /// Chain the named arguments, then [`TextThemeApply::into_text_theme`];
    /// `theme.apply()` with no further setters is Dart `apply()` with defaults.
    pub fn apply(&self) -> TextThemeApply<'_> {
        TextThemeApply::new(self)
    }

    /// Linearly interpolate between two text themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(a: Option<&TextTheme>, b: Option<&TextTheme>, t: f64) -> TextTheme {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        let lerp = |style: fn(&TextTheme) -> Option<&TextStyle>| {
            TextStyle::lerp(a.and_then(style), b.and_then(style), t)
        };
        TextTheme {
            display_large: lerp(|theme| theme.display_large.as_ref()),
            display_medium: lerp(|theme| theme.display_medium.as_ref()),
            display_small: lerp(|theme| theme.display_small.as_ref()),
            headline_large: lerp(|theme| theme.headline_large.as_ref()),
            headline_medium: lerp(|theme| theme.headline_medium.as_ref()),
            headline_small: lerp(|theme| theme.headline_small.as_ref()),
            title_large: lerp(|theme| theme.title_large.as_ref()),
            title_medium: lerp(|theme| theme.title_medium.as_ref()),
            title_small: lerp(|theme| theme.title_small.as_ref()),
            body_large: lerp(|theme| theme.body_large.as_ref()),
            body_medium: lerp(|theme| theme.body_medium.as_ref()),
            body_small: lerp(|theme| theme.body_small.as_ref()),
            label_large: lerp(|theme| theme.label_large.as_ref()),
            label_medium: lerp(|theme| theme.label_medium.as_ref()),
            label_small: lerp(|theme| theme.label_small.as_ref()),
        }
    }

    /// The `ThemeData.text_theme` property of the ambient `Theme`.
    ///
    /// Equivalent to `Theme::of(app, context).text_theme`.
    ///
    /// See also:
    /// * [`TextTheme::primary_of`], which returns the `ThemeData.primary_text_theme` property
    ///   of the ambient `Theme` instead.
    pub fn of(app: &mut App, context: BuildContext) -> TextTheme {
        Theme::of(app, context).text_theme
    }

    /// The `ThemeData.primary_text_theme` property of the ambient `Theme`.
    ///
    ///
    /// Equivalent to `Theme::of(app, context).primary_text_theme`.
    ///
    /// See also:
    /// * [`TextTheme::of`], which returns the `ThemeData.text_theme` property of the ambient
    ///   `Theme` instead.
    pub fn primary_of(app: &mut App, context: BuildContext) -> TextTheme {
        Theme::of(app, context).primary_text_theme
    }
}

/// Dart `TextTheme.apply` named optionals.
pub struct TextThemeApply<'a> {
    theme: &'a TextTheme,
    font_family: Option<String>,
    font_family_fallback: Option<Vec<String>>,
    package: Option<String>,
    font_size_factor: f64,
    font_size_delta: f64,
    letter_spacing_factor: f64,
    letter_spacing_delta: f64,
    word_spacing_factor: f64,
    word_spacing_delta: f64,
    height_factor: f64,
    height_delta: f64,
    display_color: Option<Color>,
    body_color: Option<Color>,
    decoration: Option<TextDecoration>,
    decoration_color: Option<Color>,
    decoration_style: Option<TextDecorationStyle>,
    font_features: Option<Vec<FontFeature>>,
    font_variations: Option<Vec<FontVariation>>,
}

impl<'a> TextThemeApply<'a> {
    fn new(theme: &'a TextTheme) -> TextThemeApply<'a> {
        TextThemeApply {
            theme,
            font_family: None,
            font_family_fallback: None,
            package: None,
            font_size_factor: 1.0,
            font_size_delta: 0.0,
            letter_spacing_factor: 1.0,
            letter_spacing_delta: 0.0,
            word_spacing_factor: 1.0,
            word_spacing_delta: 0.0,
            height_factor: 1.0,
            height_delta: 0.0,
            display_color: None,
            body_color: None,
            decoration: None,
            decoration_color: None,
            decoration_style: None,
            font_features: None,
            font_variations: None,
        }
    }

    /// Dart `apply(fontFamily:)`.
    pub fn font_family(mut self, font_family: impl Into<String>) -> TextThemeApply<'a> {
        self.font_family = Some(font_family.into());
        self
    }

    /// Dart `apply(fontFamilyFallback:)`.
    pub fn font_family_fallback(mut self, font_family_fallback: Vec<String>) -> TextThemeApply<'a> {
        self.font_family_fallback = Some(font_family_fallback);
        self
    }

    /// Dart `apply(package:)`.
    pub fn package(mut self, package: impl Into<String>) -> TextThemeApply<'a> {
        self.package = Some(package.into());
        self
    }

    /// Dart `apply(fontSizeFactor:)`.
    pub fn font_size_factor(mut self, font_size_factor: f64) -> TextThemeApply<'a> {
        self.font_size_factor = font_size_factor;
        self
    }

    /// Dart `apply(fontSizeDelta:)`.
    pub fn font_size_delta(mut self, font_size_delta: f64) -> TextThemeApply<'a> {
        self.font_size_delta = font_size_delta;
        self
    }

    /// Dart `apply(letterSpacingFactor:)`.
    pub fn letter_spacing_factor(mut self, letter_spacing_factor: f64) -> TextThemeApply<'a> {
        self.letter_spacing_factor = letter_spacing_factor;
        self
    }

    /// Dart `apply(letterSpacingDelta:)`.
    pub fn letter_spacing_delta(mut self, letter_spacing_delta: f64) -> TextThemeApply<'a> {
        self.letter_spacing_delta = letter_spacing_delta;
        self
    }

    /// Dart `apply(wordSpacingFactor:)`.
    pub fn word_spacing_factor(mut self, word_spacing_factor: f64) -> TextThemeApply<'a> {
        self.word_spacing_factor = word_spacing_factor;
        self
    }

    /// Dart `apply(wordSpacingDelta:)`.
    pub fn word_spacing_delta(mut self, word_spacing_delta: f64) -> TextThemeApply<'a> {
        self.word_spacing_delta = word_spacing_delta;
        self
    }

    /// Dart `apply(heightFactor:)`.
    pub fn height_factor(mut self, height_factor: f64) -> TextThemeApply<'a> {
        self.height_factor = height_factor;
        self
    }

    /// Dart `apply(heightDelta:)`.
    pub fn height_delta(mut self, height_delta: f64) -> TextThemeApply<'a> {
        self.height_delta = height_delta;
        self
    }

    /// Dart `apply(displayColor:)`.
    pub fn display_color(mut self, display_color: Color) -> TextThemeApply<'a> {
        self.display_color = Some(display_color);
        self
    }

    /// Dart `apply(bodyColor:)`.
    pub fn body_color(mut self, body_color: Color) -> TextThemeApply<'a> {
        self.body_color = Some(body_color);
        self
    }

    /// Dart `apply(decoration:)`.
    pub fn decoration(mut self, decoration: TextDecoration) -> TextThemeApply<'a> {
        self.decoration = Some(decoration);
        self
    }

    /// Dart `apply(decorationColor:)`.
    pub fn decoration_color(mut self, decoration_color: Color) -> TextThemeApply<'a> {
        self.decoration_color = Some(decoration_color);
        self
    }

    /// Dart `apply(decorationStyle:)`.
    pub fn decoration_style(mut self, decoration_style: TextDecorationStyle) -> TextThemeApply<'a> {
        self.decoration_style = Some(decoration_style);
        self
    }

    /// Dart `apply(fontFeatures:)`.
    pub fn font_features(mut self, font_features: Vec<FontFeature>) -> TextThemeApply<'a> {
        self.font_features = Some(font_features);
        self
    }

    /// Dart `apply(fontVariations:)`.
    pub fn font_variations(mut self, font_variations: Vec<FontVariation>) -> TextThemeApply<'a> {
        self.font_variations = Some(font_variations);
        self
    }

    /// The text theme with every style applied.
    pub fn into_text_theme(self) -> TextTheme {
        let theme = self.theme;
        let display_color = self.display_color;
        let body_color = self.body_color;
        TextTheme {
            display_large: self.apply_to(theme.display_large.as_ref(), display_color),
            display_medium: self.apply_to(theme.display_medium.as_ref(), display_color),
            display_small: self.apply_to(theme.display_small.as_ref(), display_color),
            headline_large: self.apply_to(theme.headline_large.as_ref(), display_color),
            headline_medium: self.apply_to(theme.headline_medium.as_ref(), display_color),
            headline_small: self.apply_to(theme.headline_small.as_ref(), body_color),
            title_large: self.apply_to(theme.title_large.as_ref(), body_color),
            title_medium: self.apply_to(theme.title_medium.as_ref(), body_color),
            title_small: self.apply_to(theme.title_small.as_ref(), body_color),
            body_large: self.apply_to(theme.body_large.as_ref(), body_color),
            body_medium: self.apply_to(theme.body_medium.as_ref(), body_color),
            body_small: self.apply_to(theme.body_small.as_ref(), display_color),
            label_large: self.apply_to(theme.label_large.as_ref(), body_color),
            label_medium: self.apply_to(theme.label_medium.as_ref(), body_color),
            label_small: self.apply_to(theme.label_small.as_ref(), body_color),
        }
    }

    /// Dart's local `applyTo`.
    fn apply_to(&self, style: Option<&TextStyle>, color: Option<Color>) -> Option<TextStyle> {
        let mut apply = style?
            .apply()
            .font_size_factor(self.font_size_factor)
            .font_size_delta(self.font_size_delta)
            .letter_spacing_delta(self.letter_spacing_delta)
            .letter_spacing_factor(self.letter_spacing_factor)
            .word_spacing_delta(self.word_spacing_delta)
            .word_spacing_factor(self.word_spacing_factor)
            .height_factor(self.height_factor)
            .height_delta(self.height_delta);
        if let Some(color) = color {
            apply = apply.color(color);
        }
        if let Some(decoration) = self.decoration {
            apply = apply.decoration(decoration);
        }
        if let Some(decoration_color) = self.decoration_color {
            apply = apply.decoration_color(decoration_color);
        }
        if let Some(decoration_style) = self.decoration_style {
            apply = apply.decoration_style(decoration_style);
        }
        if let Some(font_family) = &self.font_family {
            apply = apply.font_family(font_family.clone());
        }
        if let Some(font_family_fallback) = &self.font_family_fallback {
            apply = apply.font_family_fallback(font_family_fallback.clone());
        }
        if let Some(package) = &self.package {
            apply = apply.package(package.clone());
        }
        if let Some(font_features) = &self.font_features {
            apply = apply.font_features(font_features.clone());
        }
        if let Some(font_variations) = &self.font_variations {
            apply = apply.font_variations(font_variations.clone());
        }
        Some(apply.into_style())
    }
}

impl From<TextThemeApply<'_>> for TextTheme {
    fn from(apply: TextThemeApply<'_>) -> TextTheme {
        apply.into_text_theme()
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{Brightness, FontStyle};
    use waft_widgets::IntoWidget;

    use super::*;
    use crate::colors::Colors;
    use crate::test_support::{probe, test_cell, with_main_thread_stack};
    use crate::theme::Theme;
    use crate::theme_data::ThemeData;
    use crate::typography::Typography;

    fn styles(theme: &TextTheme) -> [&TextStyle; 15] {
        [
            &theme.display_large,
            &theme.display_medium,
            &theme.display_small,
            &theme.headline_large,
            &theme.headline_medium,
            &theme.headline_small,
            &theme.title_large,
            &theme.title_medium,
            &theme.title_small,
            &theme.body_large,
            &theme.body_medium,
            &theme.body_small,
            &theme.label_large,
            &theme.label_medium,
            &theme.label_small,
        ]
        .map(|style| style.as_ref().expect("the text theme has this style"))
    }

    fn optional_styles(theme: &TextTheme) -> [&Option<TextStyle>; 15] {
        [
            &theme.display_large,
            &theme.display_medium,
            &theme.display_small,
            &theme.headline_large,
            &theme.headline_medium,
            &theme.headline_small,
            &theme.title_large,
            &theme.title_medium,
            &theme.title_small,
            &theme.body_large,
            &theme.body_medium,
            &theme.body_small,
            &theme.label_large,
            &theme.label_medium,
            &theme.label_small,
        ]
    }

    fn color(color: Color) -> Option<waft_painting::AnyColor> {
        Some(color.into())
    }

    #[test]
    fn copy_with_apply_and_merge_of_an_empty_text_theme_are_empty() {
        assert_eq!(TextTheme::new(), TextTheme::new().copy_with());
        assert_eq!(TextTheme::new(), TextTheme::new().apply().into_text_theme());
        assert_eq!(TextTheme::new(), TextTheme::new().merge(None));
    }

    #[test]
    fn lerp_special_cases() {
        assert_eq!(TextTheme::lerp(None, None, 0.0), TextTheme::new());
        let theme = TextTheme::new();
        assert_eq!(TextTheme::lerp(Some(&theme), Some(&theme), 0.5), theme);
    }

    #[test]
    fn copy_with_apply_and_merge_of_typography_black() {
        let typography = Typography::material2018().build();
        assert_eq!(typography.black, typography.black.copy_with());
        assert_eq!(typography.black, typography.black.apply().into_text_theme());
        assert_eq!(typography.black, typography.black.merge(None));
        assert_eq!(
            typography.black,
            TextTheme::new().merge(Some(&typography.black))
        );
        assert_eq!(
            typography.black,
            typography.black.merge(Some(&typography.black))
        );
        assert_eq!(
            typography.white,
            typography.black.merge(Some(&typography.white))
        );
        assert_ne!(typography.black, typography.white);
    }

    #[test]
    fn copy_with_replaces_every_style() {
        let typography = Typography::material2018().build();
        let white = &typography.white;
        let style = |style: &Option<TextStyle>| style.clone().expect("a style");
        let white_copy = typography
            .black
            .copy_with()
            .display_large(style(&white.display_large))
            .display_medium(style(&white.display_medium))
            .display_small(style(&white.display_small))
            .headline_large(style(&white.headline_large))
            .headline_medium(style(&white.headline_medium))
            .headline_small(style(&white.headline_small))
            .title_large(style(&white.title_large))
            .title_medium(style(&white.title_medium))
            .title_small(style(&white.title_small))
            .body_large(style(&white.body_large))
            .body_medium(style(&white.body_medium))
            .body_small(style(&white.body_small))
            .label_large(style(&white.label_large))
            .label_medium(style(&white.label_medium))
            .label_small(style(&white.label_small));
        assert_eq!(typography.white, white_copy);
    }

    #[test]
    fn merges_properly_in_the_presence_of_null_fields() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let partial_theme =
                TextTheme::new().title_large(TextStyle::new().color(Color::new(0xcafefeed)));
            let full_theme = ThemeData::fallback(&cell.borrow(), None)
                .text_theme
                .merge(Some(&partial_theme));
            assert_eq!(
                full_theme.title_large.and_then(|style| style.color),
                color(Color::new(0xcafefeed))
            );
        });

        let only_headline_small_and_title_large = TextTheme::new()
            .headline_small(TextStyle::new().color(Color::new(0xcafefeed)))
            .title_large(TextStyle::new().color(Color::new(0xbeefcafe)));
        let only_body_medium_and_title_large = TextTheme::new()
            .body_medium(TextStyle::new().color(Color::new(0xfeedfeed)))
            .title_large(TextStyle::new().color(Color::new(0xdeadcafe)));
        let merged =
            only_headline_small_and_title_large.merge(Some(&only_body_medium_and_title_large));
        assert!(merged.body_large.is_none());
        assert_eq!(
            merged.body_medium.and_then(|style| style.color),
            color(Color::new(0xfeedfeed))
        );
        assert_eq!(
            merged.headline_small.and_then(|style| style.color),
            color(Color::new(0xcafefeed))
        );
        assert_eq!(
            merged.title_large.and_then(|style| style.color),
            color(Color::new(0xdeadcafe))
        );

        let merged = only_headline_small_and_title_large.merge(None);
        assert_eq!(merged, only_headline_small_and_title_large);
    }

    #[test]
    fn apply_applies_custom_properties() {
        // The `displayColor` is applied to [displayLarge], [displayMedium],
        // [displaySmall], [headlineLarge], [headlineMedium], and [bodySmall]. The
        // `bodyColor` is applied to the remaining text styles.
        let display_color = Color::new(0x00000001);
        let body_color = Color::new(0x00000002);
        let font_family = "fontFamily";
        let font_family_fallback: Vec<String> =
            ["font", "family", "fallback"].map(String::from).to_vec();
        let decoration_color = Color::new(0x00000003);
        let decoration_style = waft_embedder::TextDecorationStyle::Dashed;
        let decoration =
            TextDecoration::combine(&[TextDecoration::UNDERLINE, TextDecoration::LINE_THROUGH]);

        let typography = Typography::material2018().build();
        let theme = typography
            .black
            .apply()
            .font_family(font_family)
            .font_family_fallback(font_family_fallback.clone())
            .display_color(display_color)
            .body_color(body_color)
            .decoration(decoration)
            .decoration_color(decoration_color)
            .decoration_style(decoration_style)
            .into_text_theme();

        let display = color(display_color);
        let body = color(body_color);
        let colors = styles(&theme).map(|style| style.color.clone());
        assert_eq!(
            colors,
            [
                display.clone(),
                display.clone(),
                display.clone(),
                display.clone(),
                display.clone(),
                body.clone(),
                body.clone(),
                body.clone(),
                body.clone(),
                body.clone(),
                body.clone(),
                display,
                body.clone(),
                body.clone(),
                body,
            ]
        );

        for style in styles(&theme) {
            assert_eq!(style.font_family.as_deref(), Some(font_family));
            assert_eq!(
                style.font_family_fallback_list(),
                Some(font_family_fallback.clone())
            );
            assert_eq!(style.decoration_color, color(decoration_color));
            assert_eq!(style.decoration_style, Some(decoration_style));
            assert_eq!(style.decoration, Some(decoration));
        }
    }

    #[test]
    fn apply_applies_package_font_family_and_font_family_fallback() {
        let typography = Typography::material2018().build();
        let theme = typography
            .black
            .apply()
            .font_family("fontFamily")
            .font_family_fallback(vec!["fallback".to_string()])
            .package("package")
            .into_text_theme();

        for style in styles(&theme) {
            assert_eq!(
                style.font_family.as_deref(),
                Some("packages/package/fontFamily")
            );
            assert_eq!(
                style.font_family_fallback_list(),
                Some(vec!["packages/package/fallback".to_string()])
            );
        }
    }

    #[test]
    fn apply_leaves_null_styles_null() {
        let body_color = Color::new(0x00000002);
        let theme = TextTheme::new()
            .body_medium(TextStyle::new().font_size(10.0))
            .apply()
            .body_color(body_color)
            .display_color(body_color)
            .font_family("fontFamily")
            .into_text_theme();

        assert_eq!(
            theme,
            TextTheme::new().body_medium(
                TextStyle::new()
                    .font_size(10.0)
                    .color(body_color)
                    .font_family("fontFamily")
            )
        );
    }

    #[test]
    fn apply_applies_font_features_and_font_variations() {
        let font_features = vec![FontFeature::enable("smcp")];
        let font_variations = vec![FontVariation::new("wght", 400.0)];
        let typography = Typography::material2018().build();
        let theme = typography
            .black
            .apply()
            .font_features(font_features.clone())
            .font_variations(font_variations.clone())
            .into_text_theme();

        for style in styles(&theme) {
            assert_eq!(style.font_features, Some(font_features.clone()));
            assert_eq!(style.font_variations, Some(font_variations.clone()));
        }
    }

    #[test]
    fn apply_without_font_features_or_font_variations_preserves_the_originals() {
        let font_features = vec![FontFeature::enable("smcp")];
        let font_variations = vec![FontVariation::new("wght", 400.0)];
        let typography = Typography::material2018().build();
        let display_large = typography
            .black
            .display_large
            .as_ref()
            .expect("a display style")
            .copy_with()
            .font_features(font_features.clone())
            .font_variations(font_variations.clone());
        let base_theme = typography.black.copy_with().display_large(display_large);
        let theme = base_theme.apply().into_text_theme();

        let display_large = theme.display_large.as_ref().expect("a display style");
        assert_eq!(display_large.font_features, Some(font_features));
        assert_eq!(display_large.font_variations, Some(font_variations));
        let display_medium = theme.display_medium.as_ref().expect("a display style");
        assert!(display_medium.font_features.is_none());
        assert!(display_medium.font_variations.is_none());
    }

    #[test]
    fn apply_font_size_factor_and_font_size_delta() {
        let typography = Typography::material2018().build();
        let base_theme = Typography::english_like2018().merge(Some(&typography.black));
        let size_theme = base_theme
            .apply()
            .font_size_factor(2.0)
            .font_size_delta(5.0)
            .into_text_theme();

        for (sized, base) in styles(&size_theme).into_iter().zip(styles(&base_theme)) {
            assert_eq!(
                sized.font_size,
                Some(base.font_size.expect("a size") * 2.0 + 5.0)
            );
        }
    }

    #[test]
    fn apply_letter_spacing_factor_and_letter_spacing_delta() {
        let typography = Typography::material2018().build();
        let base_theme = Typography::english_like2018().merge(Some(&typography.black));
        let size_theme = base_theme
            .apply()
            .letter_spacing_factor(2.0)
            .letter_spacing_delta(5.0)
            .into_text_theme();

        for (sized, base) in styles(&size_theme).into_iter().zip(styles(&base_theme)) {
            assert_eq!(
                sized.letter_spacing,
                Some(base.letter_spacing.expect("a letter spacing") * 2.0 + 5.0)
            );
        }
    }

    #[test]
    fn apply_word_spacing_factor_and_word_spacing_delta() {
        let typography = Typography::material2018().build();
        let base_theme = Typography::english_like2018().merge(Some(&typography.black));
        let with_word_spacing = |style: &Option<TextStyle>| {
            style
                .as_ref()
                .expect("a style")
                .copy_with()
                .word_spacing(1.0)
        };
        let mut base_theme_with_word_spacing = base_theme.copy_with();
        for style in [
            &mut base_theme_with_word_spacing.display_large,
            &mut base_theme_with_word_spacing.display_medium,
            &mut base_theme_with_word_spacing.display_small,
            &mut base_theme_with_word_spacing.headline_large,
            &mut base_theme_with_word_spacing.headline_medium,
            &mut base_theme_with_word_spacing.headline_small,
            &mut base_theme_with_word_spacing.title_large,
            &mut base_theme_with_word_spacing.title_medium,
            &mut base_theme_with_word_spacing.title_small,
            &mut base_theme_with_word_spacing.body_large,
            &mut base_theme_with_word_spacing.body_medium,
            &mut base_theme_with_word_spacing.body_small,
            &mut base_theme_with_word_spacing.label_large,
            &mut base_theme_with_word_spacing.label_medium,
            &mut base_theme_with_word_spacing.label_small,
        ] {
            *style = Some(with_word_spacing(style));
        }
        let size_theme = base_theme_with_word_spacing
            .apply()
            .word_spacing_factor(2.0)
            .word_spacing_delta(5.0)
            .into_text_theme();

        for (sized, base) in styles(&size_theme)
            .into_iter()
            .zip(styles(&base_theme_with_word_spacing))
        {
            assert_eq!(
                sized.word_spacing,
                Some(base.word_spacing.expect("a word spacing") * 2.0 + 5.0)
            );
        }
    }

    #[test]
    fn apply_height_factor_and_height_delta() {
        let typography = Typography::material2021().build();
        let base_theme = Typography::english_like2021().merge(Some(&typography.black));
        let size_theme = base_theme
            .apply()
            .height_factor(2.0)
            .height_delta(5.0)
            .into_text_theme();

        for (sized, base) in styles(&size_theme).into_iter().zip(styles(&base_theme)) {
            assert_eq!(
                sized.height,
                Some(base.height.expect("a height") * 2.0 + 5.0)
            );
        }
    }

    #[test]
    fn lerp_with_one_parameter_null_lerps_each_style_with_null() {
        let theme = Typography::material2018().build().black;
        let lerped = TextTheme::lerp(Some(&theme), None, 0.25);
        for (lerped, style) in optional_styles(&lerped)
            .into_iter()
            .zip(optional_styles(&theme))
        {
            assert_eq!(*lerped, TextStyle::lerp(style.as_ref(), None, 0.25));
        }

        let lerped = TextTheme::lerp(None, Some(&theme), 0.25);
        for (lerped, style) in optional_styles(&lerped)
            .into_iter()
            .zip(optional_styles(&theme))
        {
            assert_eq!(*lerped, TextStyle::lerp(None, style.as_ref(), 0.25));
        }
    }

    #[test]
    fn lerp_with_null_parameters_is_empty() {
        let lerped = TextTheme::lerp(None, None, 0.25);
        for style in optional_styles(&lerped) {
            assert!(style.is_none());
        }
    }

    #[test]
    fn text_theme_of_and_primary_of_are_the_ambient_theme_text_themes() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let (theme, text_theme, primary_text_theme) = probe(
                &cell,
                |app, child| {
                    let data = ThemeData::builder()
                        .text_theme(
                            TextTheme::new().display_large(
                                TextStyle::new()
                                    .color(Colors::BLUE.primary())
                                    .font_size(30.0),
                            ),
                        )
                        .primary_text_theme(
                            TextTheme::new().display_large(
                                TextStyle::new()
                                    .background_color(Colors::GREEN.primary())
                                    .font_style(FontStyle::Italic),
                            ),
                        )
                        .build(app);
                    Theme::new(data, child).into_widget()
                },
                |app, context| {
                    (
                        Theme::of(app, context),
                        TextTheme::of(app, context),
                        TextTheme::primary_of(app, context),
                    )
                },
            );
            assert_eq!(text_theme, theme.text_theme);
            assert_eq!(primary_text_theme, theme.primary_text_theme);
            assert_eq!(theme.brightness(), Brightness::Light);
        });
    }
}
