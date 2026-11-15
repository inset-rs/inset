//! Flutter counterpart: material_ui's `typography.dart`.
//!
//! Dart's `static const TextTheme`s are functions: a [`TextStyle`] owns its font family string.

use std::rc::Rc;

use waft_embedder::{Brightness, FontWeight, TargetPlatform, TextBaseline, TextDecoration};
use waft_painting::TextStyle;

use crate::color_scheme::ColorScheme;
use crate::colors::Colors;
use crate::generated::typography_defaults_m3::M3Typography;
use crate::text_theme::TextTheme;

/// One of [`Typography`]'s text theme constants, as a function.
type TextThemeConstant = fn() -> TextTheme;

/// A characterization of the of a [`TextTheme`]'s glyphs that is used to define
/// its localized [`TextStyle`] geometry for `ThemeData.text_theme`.
///
/// The script category defines the overall geometry of a [`TextTheme`] for
/// the [`Typography::geometry_theme_for`] method in terms of the
/// three language categories defined in <https://material.io/go/design-typography>.
///
/// Generally speaking, font sizes for `ScriptCategory::Tall` and
/// `ScriptCategory::Dense` scripts - for text styles that are smaller than the
/// title style - are one unit larger than they are for
/// `ScriptCategory::EnglishLike` scripts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScriptCategory {
    /// The languages of Western, Central, and Eastern Europe and much of
    /// Africa are typically written in the Latin alphabet. Vietnamese is a
    /// notable exception in that, while it uses a localized form of the Latin
    /// writing system, its accented glyphs can be much taller than those
    /// found in Western European languages. The Greek and Cyrillic writing
    /// systems are very similar to Latin.
    EnglishLike,

    /// Language scripts that require extra line height to accommodate larger
    /// glyphs, including Chinese, Japanese, and Korean.
    Dense,

    /// Language scripts that require extra line height to accommodate
    /// larger glyphs, including South and Southeast Asian and
    /// Middle-Eastern languages, like Arabic, Hindi, Telugu, Thai, and
    /// Vietnamese.
    Tall,
}

/// The color and geometry [`TextTheme`]s for Material apps.
///
/// The text theme provided by the overall `Theme`,
/// `ThemeData.text_theme`, is based on the current locale's
/// `MaterialLocalizations.script_category` and is created
/// by merging a color text theme - [`black`](Self::black) for
/// [`Brightness::Light`] themes and [`white`](Self::white) for [`Brightness::Dark`]
/// themes - and a geometry text theme, one of [`english_like`](Self::english_like),
/// [`dense`](Self::dense), or [`tall`](Self::tall), depending on the locale.
///
/// To lookup the localized text theme use
/// `Theme::of(app, context).text_theme`.
///
/// The color text themes are [`black_mountain_view`](Self::black_mountain_view),
/// [`white_mountain_view`](Self::white_mountain_view),
/// [`black_cupertino`](Self::black_cupertino), and [`white_cupertino`](Self::white_cupertino).
/// The Mountain View theme [`TextStyle`]s
/// are based on the Roboto fonts as used on Android. The Cupertino themes are
/// based on the [San Francisco
/// font](https://developer.apple.com/design/human-interface-guidelines/typography/)
/// fonts as used by Apple on iOS.
///
/// Three sets of geometry themes are provided: 2014, 2018, and 2021. The 2014
/// themes correspond to the original version of the Material Design spec. The
/// 2018 themes correspond to the second iteration of the specification and
/// feature different font sizes, font weights, and letter spacing values. The
/// 2021 themes correspond to the Material Design 3 specification.
///
/// By default, `ThemeData.typography` is `Typography.material2021(platform:
/// platform, colorScheme: colorScheme)` (or `Typography.material2014(platform:
/// platform)` when `ThemeData.use_material3` is false). To use the 2018 text
/// theme geometries, specify a value using the [`Typography::material2018`]
/// constructor:
///
/// ```text
/// typography: Typography::material2018().platform(Some(platform)).build()
/// ```
///
/// See also:
///
///  * <https://material.io/design/typography/>
///  * <https://m3.material.io/styles/typography>
///
/// Each text theme is shared, as Dart shares the one `TextTheme` object: a clone holds the
/// same five, and `ThemeData::localize` keys its cache on the geometry theme's identity.
#[derive(Clone, Debug, PartialEq)]
pub struct Typography {
    /// A Material Design text theme with dark glyphs.
    ///
    /// This [`TextTheme`] should provide color but not geometry (font size,
    /// weight, etc). A text theme's geometry depends on the locale. To look
    /// up a localized [`TextTheme`], use the overall `Theme`, for example:
    /// `Theme::of(app, context).text_theme`.
    ///
    /// The [`english_like`](Self::english_like), [`dense`](Self::dense), and
    /// [`tall`](Self::tall) text theme's provide locale-specific geometry.
    pub black: Rc<TextTheme>,
    /// A Material Design text theme with light glyphs.
    ///
    /// This [`TextTheme`] provides color but not geometry (font size, weight, etc).
    /// A text theme's geometry depends on the locale. To look up a localized
    /// [`TextTheme`], use the overall `Theme`, for example:
    /// `Theme::of(app, context).text_theme`.
    ///
    /// The [`english_like`](Self::english_like), [`dense`](Self::dense), and
    /// [`tall`](Self::tall) text theme's provide locale-specific geometry.
    pub white: Rc<TextTheme>,
    /// Defines text geometry for `ScriptCategory::EnglishLike` scripts, such as
    /// English, French, Russian, etc.
    ///
    /// This text theme is merged with either [`black`](Self::black) or
    /// [`white`](Self::white), depending on the overall `ThemeData.brightness`, when the
    /// current locale's `MaterialLocalizations.script_category` is
    /// `ScriptCategory::EnglishLike`.
    ///
    /// To look up a localized [`TextTheme`], use the overall `Theme`, for
    /// example: `Theme::of(app, context).text_theme`.
    pub english_like: Rc<TextTheme>,
    /// Defines text geometry for dense scripts, such as Chinese, Japanese
    /// and Korean.
    ///
    /// This text theme is merged with either [`black`](Self::black) or
    /// [`white`](Self::white), depending on the overall `ThemeData.brightness`, when the
    /// current locale's `MaterialLocalizations.script_category` is `ScriptCategory::Dense`.
    ///
    /// To look up a localized [`TextTheme`], use the overall `Theme`, for
    /// example: `Theme::of(app, context).text_theme`.
    pub dense: Rc<TextTheme>,
    /// Defines text geometry for tall scripts, such as Farsi, Hindi, and Thai.
    ///
    /// This text theme is merged with either [`black`](Self::black) or
    /// [`white`](Self::white), depending on the overall `ThemeData.brightness`, when the
    /// current locale's `MaterialLocalizations.script_category` is `ScriptCategory::Tall`.
    ///
    /// To look up a localized [`TextTheme`], use the overall `Theme`, for
    /// example: `Theme::of(app, context).text_theme`.
    pub tall: Rc<TextTheme>,
}

impl Typography {
    /// Creates a typography instance.
    ///
    /// This constructor is identical to [`Typography::material2018`].
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> TypographyBuilder {
        Typography::material2018()
    }

    /// Creates a typography instance using Material Design's 2014 defaults.
    ///
    /// If `platform` is [`TargetPlatform::IOS`] or [`TargetPlatform::MacOS`], the
    /// default values for [`black`](Self::black) and [`white`](Self::white) are
    /// [`black_cupertino`](Self::black_cupertino) and [`white_cupertino`](Self::white_cupertino)
    /// respectively. Otherwise they are [`black_mountain_view`](Self::black_mountain_view) and
    /// [`white_mountain_view`](Self::white_mountain_view). If `platform` is null then both
    /// [`black`](Self::black) and [`white`](Self::white) must be specified.
    ///
    /// The default values for [`english_like`](Self::english_like), [`dense`](Self::dense),
    /// and [`tall`](Self::tall) are [`english_like2014`](Self::english_like2014),
    /// [`dense2014`](Self::dense2014), and [`tall2014`](Self::tall2014).
    pub fn material2014() -> TypographyBuilder {
        TypographyBuilder::new(
            Typography::english_like2014,
            Typography::dense2014,
            Typography::tall2014,
        )
    }

    /// Creates a typography instance using Material Design's 2018 defaults.
    ///
    /// If `platform` is [`TargetPlatform::IOS`] or [`TargetPlatform::MacOS`], the
    /// default values for [`black`](Self::black) and [`white`](Self::white) are
    /// [`black_cupertino`](Self::black_cupertino) and [`white_cupertino`](Self::white_cupertino)
    /// respectively. Otherwise they are [`black_mountain_view`](Self::black_mountain_view) and
    /// [`white_mountain_view`](Self::white_mountain_view). If `platform` is null then both
    /// [`black`](Self::black) and [`white`](Self::white) must be specified.
    ///
    /// The default values for [`english_like`](Self::english_like), [`dense`](Self::dense),
    /// and [`tall`](Self::tall) are [`english_like2018`](Self::english_like2018),
    /// [`dense2018`](Self::dense2018), and [`tall2018`](Self::tall2018).
    pub fn material2018() -> TypographyBuilder {
        TypographyBuilder::new(
            Typography::english_like2018,
            Typography::dense2018,
            Typography::tall2018,
        )
    }

    /// Creates a typography instance using Material Design 3 2021 defaults.
    ///
    /// Text colors are derived from `color_scheme`. For a light `color_scheme`,
    /// every style in the resulting [`black`](Self::black) uses
    /// `ColorScheme.on_surface` and every style in [`white`](Self::white) uses
    /// `ColorScheme.surface`. For a dark `color_scheme`, these are reversed. The
    /// same colors are used for [`TextStyle::decoration_color`].
    ///
    /// The default values for [`black`](Self::black) and [`white`](Self::white) depend on
    /// `platform`: [`black_cupertino`](Self::black_cupertino) and
    /// [`white_cupertino`](Self::white_cupertino) on iOS,
    /// [`black_redwood_city`](Self::black_redwood_city) and
    /// [`white_redwood_city`](Self::white_redwood_city) on macOS,
    /// [`black_redmond`](Self::black_redmond) and [`white_redmond`](Self::white_redmond) on
    /// Windows, [`black_helsinki`](Self::black_helsinki) and
    /// [`white_helsinki`](Self::white_helsinki) on Linux, and
    /// [`black_mountain_view`](Self::black_mountain_view) and
    /// [`white_mountain_view`](Self::white_mountain_view) otherwise. If `platform` is
    /// null then both [`black`](Self::black) and [`white`](Self::white) must be specified.
    ///
    /// The default values for [`english_like`](Self::english_like), [`dense`](Self::dense),
    /// and [`tall`](Self::tall) are [`english_like2021`](Self::english_like2021),
    /// [`dense2021`](Self::dense2021), and [`tall2021`](Self::tall2021).
    ///
    /// The `black` and `white` parameters are deprecated. Only their non-color
    /// properties (such as font family) are used. Any [`TextStyle::color`] or
    /// [`TextStyle::decoration_color`] they specify is replaced with a color from
    /// `color_scheme` as described above. To customize the text theme's colors,
    /// call [`copy_with`](Self::copy_with) on the result:
    ///
    /// ```text
    /// let base = Typography::material2021().color_scheme(color_scheme).build();
    /// let typography = base
    ///     .copy_with()
    ///     .black(base.black.merge(Some(&my_black)))
    ///     .white(base.white.merge(Some(&my_white)));
    /// ```
    ///
    /// See also:
    ///  * <https://m3.material.io/styles/typography>
    pub fn material2021() -> TypographyMaterial2021Builder {
        TypographyMaterial2021Builder {
            platform: Some(TargetPlatform::Android),
            color_scheme: ColorScheme::light(),
            black: None,
            white: None,
            english_like: None,
            dense: None,
            tall: None,
        }
    }

    /// Dart's private `Typography._withPlatform`.
    fn with_platform(
        platform: Option<TargetPlatform>,
        black: Option<Rc<TextTheme>>,
        white: Option<Rc<TextTheme>>,
        english_like: Rc<TextTheme>,
        dense: Rc<TextTheme>,
        tall: Rc<TextTheme>,
    ) -> Typography {
        debug_assert!(platform.is_some() || (black.is_some() && white.is_some()));
        let (black_default, white_default): (TextThemeConstant, TextThemeConstant) = match platform
        {
            Some(TargetPlatform::IOS) => (Typography::black_cupertino, Typography::white_cupertino),
            Some(TargetPlatform::Android | TargetPlatform::Fuchsia) => (
                Typography::black_mountain_view,
                Typography::white_mountain_view,
            ),
            Some(TargetPlatform::Windows) => (Typography::black_redmond, Typography::white_redmond),
            Some(TargetPlatform::MacOS) => (
                Typography::black_redwood_city,
                Typography::white_redwood_city,
            ),
            Some(TargetPlatform::Linux) => (Typography::black_helsinki, Typography::white_helsinki),
            None => (
                || panic!("black is given when platform is null"),
                || panic!("white is given when platform is null"),
            ),
        };
        // The defaults are picked as functions so that only the chosen text themes are built.
        Typography {
            black: black.unwrap_or_else(|| Rc::new(black_default())),
            white: white.unwrap_or_else(|| Rc::new(white_default())),
            english_like,
            dense,
            tall,
        }
    }

    /// Returns one of [`english_like`](Self::english_like), [`dense`](Self::dense), or
    /// [`tall`](Self::tall).
    pub fn geometry_theme_for(&self, category: ScriptCategory) -> &Rc<TextTheme> {
        match category {
            ScriptCategory::EnglishLike => &self.english_like,
            ScriptCategory::Dense => &self.dense,
            ScriptCategory::Tall => &self.tall,
        }
    }

    /// Creates a copy of this [`Typography`] with the given fields
    /// replaced by the non-null parameter values.
    ///
    /// Dart's named arguments are the setters on the copy:
    /// `typography.copy_with().black(text_theme)`.
    pub fn copy_with(&self) -> Typography {
        self.clone()
    }

    /// Dart `copyWith(black:)`.
    pub fn black(mut self, black: impl Into<Rc<TextTheme>>) -> Typography {
        self.black = black.into();
        self
    }

    /// Dart `copyWith(white:)`.
    pub fn white(mut self, white: impl Into<Rc<TextTheme>>) -> Typography {
        self.white = white.into();
        self
    }

    /// Dart `copyWith(englishLike:)`.
    pub fn english_like(mut self, english_like: impl Into<Rc<TextTheme>>) -> Typography {
        self.english_like = english_like.into();
        self
    }

    /// Dart `copyWith(dense:)`.
    pub fn dense(mut self, dense: impl Into<Rc<TextTheme>>) -> Typography {
        self.dense = dense.into();
        self
    }

    /// Dart `copyWith(tall:)`.
    pub fn tall(mut self, tall: impl Into<Rc<TextTheme>>) -> Typography {
        self.tall = tall.into();
        self
    }

    /// Linearly interpolate between two [`Typography`] objects.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(a: &Typography, b: &Typography, t: f64) -> Typography {
        if std::ptr::eq(a, b) {
            return a.clone();
        }
        Typography {
            black: lerp_text_theme(&a.black, &b.black, t),
            white: lerp_text_theme(&a.white, &b.white, t),
            english_like: lerp_text_theme(&a.english_like, &b.english_like, t),
            dense: lerp_text_theme(&a.dense, &b.dense, t),
            tall: lerp_text_theme(&a.tall, &b.tall, t),
        }
    }

    /// A Material Design text theme with dark glyphs based on Roboto.
    ///
    /// This `TextTheme` provides color but not geometry (font size, weight, etc).
    pub fn black_mountain_view() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("blackMountainView displayLarge")
                    .font_family("Roboto")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("blackMountainView displayMedium")
                    .font_family("Roboto")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("blackMountainView displaySmall")
                    .font_family("Roboto")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("blackMountainView headlineLarge")
                    .font_family("Roboto")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("blackMountainView headlineMedium")
                    .font_family("Roboto")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("blackMountainView headlineSmall")
                    .font_family("Roboto")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("blackMountainView titleLarge")
                    .font_family("Roboto")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("blackMountainView titleMedium")
                    .font_family("Roboto")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("blackMountainView titleSmall")
                    .font_family("Roboto")
                    .color(Colors::BLACK)
                    .decoration(TextDecoration::NONE),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("blackMountainView bodyLarge")
                    .font_family("Roboto")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("blackMountainView bodyMedium")
                    .font_family("Roboto")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("blackMountainView bodySmall")
                    .font_family("Roboto")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("blackMountainView labelLarge")
                    .font_family("Roboto")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("blackMountainView labelMedium")
                    .font_family("Roboto")
                    .color(Colors::BLACK)
                    .decoration(TextDecoration::NONE),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("blackMountainView labelSmall")
                    .font_family("Roboto")
                    .color(Colors::BLACK)
                    .decoration(TextDecoration::NONE),
            ),
        }
    }

    /// A Material Design text theme with light glyphs based on Roboto.
    ///
    /// This `TextTheme` provides color but not geometry (font size, weight, etc).
    pub fn white_mountain_view() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("whiteMountainView displayLarge")
                    .font_family("Roboto")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("whiteMountainView displayMedium")
                    .font_family("Roboto")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("whiteMountainView displaySmall")
                    .font_family("Roboto")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("whiteMountainView headlineLarge")
                    .font_family("Roboto")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("whiteMountainView headlineMedium")
                    .font_family("Roboto")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("whiteMountainView headlineSmall")
                    .font_family("Roboto")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("whiteMountainView titleLarge")
                    .font_family("Roboto")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("whiteMountainView titleMedium")
                    .font_family("Roboto")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("whiteMountainView titleSmall")
                    .font_family("Roboto")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("whiteMountainView bodyLarge")
                    .font_family("Roboto")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("whiteMountainView bodyMedium")
                    .font_family("Roboto")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("whiteMountainView bodySmall")
                    .font_family("Roboto")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("whiteMountainView labelLarge")
                    .font_family("Roboto")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("whiteMountainView labelMedium")
                    .font_family("Roboto")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("whiteMountainView labelSmall")
                    .font_family("Roboto")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
        }
    }

    /// A Material Design text theme with dark glyphs based on Segoe UI.
    ///
    /// This `TextTheme` provides color but not geometry (font size, weight, etc).
    pub fn black_redmond() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("blackRedmond displayLarge")
                    .font_family("Segoe UI")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("blackRedmond displayMedium")
                    .font_family("Segoe UI")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("blackRedmond displaySmall")
                    .font_family("Segoe UI")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("blackRedmond headlineLarge")
                    .font_family("Segoe UI")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("blackRedmond headlineMedium")
                    .font_family("Segoe UI")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("blackRedmond headlineSmall")
                    .font_family("Segoe UI")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("blackRedmond titleLarge")
                    .font_family("Segoe UI")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("blackRedmond titleMedium")
                    .font_family("Segoe UI")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("blackRedmond titleSmall")
                    .font_family("Segoe UI")
                    .color(Colors::BLACK)
                    .decoration(TextDecoration::NONE),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("blackRedmond bodyLarge")
                    .font_family("Segoe UI")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("blackRedmond bodyMedium")
                    .font_family("Segoe UI")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("blackRedmond bodySmall")
                    .font_family("Segoe UI")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("blackRedmond labelLarge")
                    .font_family("Segoe UI")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("blackRedmond labelMedium")
                    .font_family("Segoe UI")
                    .color(Colors::BLACK)
                    .decoration(TextDecoration::NONE),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("blackRedmond labelSmall")
                    .font_family("Segoe UI")
                    .color(Colors::BLACK)
                    .decoration(TextDecoration::NONE),
            ),
        }
    }

    /// A Material Design text theme with light glyphs based on Segoe UI.
    ///
    /// This `TextTheme` provides color but not geometry (font size, weight, etc).
    pub fn white_redmond() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("whiteRedmond displayLarge")
                    .font_family("Segoe UI")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("whiteRedmond displayMedium")
                    .font_family("Segoe UI")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("whiteRedmond displaySmall")
                    .font_family("Segoe UI")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("whiteRedmond headlineLarge")
                    .font_family("Segoe UI")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("whiteRedmond headlineMedium")
                    .font_family("Segoe UI")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("whiteRedmond headlineSmall")
                    .font_family("Segoe UI")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("whiteRedmond titleLarge")
                    .font_family("Segoe UI")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("whiteRedmond titleMedium")
                    .font_family("Segoe UI")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("whiteRedmond titleSmall")
                    .font_family("Segoe UI")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("whiteRedmond bodyLarge")
                    .font_family("Segoe UI")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("whiteRedmond bodyMedium")
                    .font_family("Segoe UI")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("whiteRedmond bodySmall")
                    .font_family("Segoe UI")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("whiteRedmond labelLarge")
                    .font_family("Segoe UI")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("whiteRedmond labelMedium")
                    .font_family("Segoe UI")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("whiteRedmond labelSmall")
                    .font_family("Segoe UI")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
        }
    }

    fn helsinki_font_fallbacks() -> Vec<String> {
        [
            "Ubuntu",
            "Adwaita Sans",
            "Cantarell",
            "DejaVu Sans",
            "Liberation Sans",
            "Arial",
        ]
        .iter()
        .map(|&family| family.to_owned())
        .collect()
    }

    /// A Material Design text theme with dark glyphs based on Roboto, with
    /// fallback fonts that are likely (but not guaranteed) to be installed on
    /// Linux.
    ///
    /// This `TextTheme` provides color but not geometry (font size, weight, etc).
    pub fn black_helsinki() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("blackHelsinki displayLarge")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("blackHelsinki displayMedium")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("blackHelsinki displaySmall")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("blackHelsinki headlineLarge")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("blackHelsinki headlineMedium")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("blackHelsinki headlineSmall")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("blackHelsinki titleLarge")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("blackHelsinki titleMedium")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("blackHelsinki titleSmall")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::BLACK)
                    .decoration(TextDecoration::NONE),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("blackHelsinki bodyLarge")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("blackHelsinki bodyMedium")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("blackHelsinki bodySmall")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("blackHelsinki labelLarge")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("blackHelsinki labelMedium")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::BLACK)
                    .decoration(TextDecoration::NONE),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("blackHelsinki labelSmall")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::BLACK)
                    .decoration(TextDecoration::NONE),
            ),
        }
    }

    /// A Material Design text theme with light glyphs based on Roboto, with fallbacks of DejaVu Sans, Liberation Sans and Arial.
    ///
    /// This `TextTheme` provides color but not geometry (font size, weight, etc).
    pub fn white_helsinki() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("whiteHelsinki displayLarge")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("whiteHelsinki displayMedium")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("whiteHelsinki displaySmall")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("whiteHelsinki headlineLarge")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("whiteHelsinki headlineMedium")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("whiteHelsinki headlineSmall")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("whiteHelsinki titleLarge")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("whiteHelsinki titleMedium")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("whiteHelsinki titleSmall")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("whiteHelsinki bodyLarge")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("whiteHelsinki bodyMedium")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("whiteHelsinki bodySmall")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("whiteHelsinki labelLarge")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("whiteHelsinki labelMedium")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("whiteHelsinki labelSmall")
                    .font_family("Roboto")
                    .font_family_fallback(Typography::helsinki_font_fallbacks())
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
        }
    }

    /// A Material Design text theme with dark glyphs based on San Francisco.
    ///
    /// This `TextTheme` provides color but not geometry (font size, weight, etc).
    ///
    /// This theme uses the iOS version of the font names.
    pub fn black_cupertino() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("blackCupertino displayLarge")
                    .font_family("CupertinoSystemDisplay")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("blackCupertino displayMedium")
                    .font_family("CupertinoSystemDisplay")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("blackCupertino displaySmall")
                    .font_family("CupertinoSystemDisplay")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("blackCupertino headlineLarge")
                    .font_family("CupertinoSystemDisplay")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("blackCupertino headlineMedium")
                    .font_family("CupertinoSystemDisplay")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("blackCupertino headlineSmall")
                    .font_family("CupertinoSystemDisplay")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("blackCupertino titleLarge")
                    .font_family("CupertinoSystemDisplay")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("blackCupertino titleMedium")
                    .font_family("CupertinoSystemText")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("blackCupertino titleSmall")
                    .font_family("CupertinoSystemText")
                    .color(Colors::BLACK)
                    .decoration(TextDecoration::NONE),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("blackCupertino bodyLarge")
                    .font_family("CupertinoSystemText")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("blackCupertino bodyMedium")
                    .font_family("CupertinoSystemText")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("blackCupertino bodySmall")
                    .font_family("CupertinoSystemText")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("blackCupertino labelLarge")
                    .font_family("CupertinoSystemText")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("blackCupertino labelMedium")
                    .font_family("CupertinoSystemText")
                    .color(Colors::BLACK)
                    .decoration(TextDecoration::NONE),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("blackCupertino labelSmall")
                    .font_family("CupertinoSystemText")
                    .color(Colors::BLACK)
                    .decoration(TextDecoration::NONE),
            ),
        }
    }

    /// A Material Design text theme with light glyphs based on San Francisco.
    ///
    /// This `TextTheme` provides color but not geometry (font size, weight, etc).
    ///
    /// This theme uses the iOS version of the font names.
    pub fn white_cupertino() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("whiteCupertino displayLarge")
                    .font_family("CupertinoSystemDisplay")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("whiteCupertino displayMedium")
                    .font_family("CupertinoSystemDisplay")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("whiteCupertino displaySmall")
                    .font_family("CupertinoSystemDisplay")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("whiteCupertino headlineLarge")
                    .font_family("CupertinoSystemDisplay")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("whiteCupertino headlineMedium")
                    .font_family("CupertinoSystemDisplay")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("whiteCupertino headlineSmall")
                    .font_family("CupertinoSystemDisplay")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("whiteCupertino titleLarge")
                    .font_family("CupertinoSystemDisplay")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("whiteCupertino titleMedium")
                    .font_family("CupertinoSystemText")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("whiteCupertino titleSmall")
                    .font_family("CupertinoSystemText")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("whiteCupertino bodyLarge")
                    .font_family("CupertinoSystemText")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("whiteCupertino bodyMedium")
                    .font_family("CupertinoSystemText")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("whiteCupertino bodySmall")
                    .font_family("CupertinoSystemText")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("whiteCupertino labelLarge")
                    .font_family("CupertinoSystemText")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("whiteCupertino labelMedium")
                    .font_family("CupertinoSystemText")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("whiteCupertino labelSmall")
                    .font_family("CupertinoSystemText")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
        }
    }

    /// A Material Design text theme with dark glyphs based on San Francisco.
    ///
    /// This `TextTheme` provides color but not geometry (font size, weight, etc).
    ///
    /// This theme uses the macOS version of the font names.
    pub fn black_redwood_city() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("blackRedwoodCity displayLarge")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("blackRedwoodCity displayMedium")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("blackRedwoodCity displaySmall")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("blackRedwoodCity headlineLarge")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("blackRedwoodCity headlineMedium")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("blackRedwoodCity headlineSmall")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("blackRedwoodCity titleLarge")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("blackRedwoodCity titleMedium")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("blackRedwoodCity titleSmall")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::BLACK)
                    .decoration(TextDecoration::NONE),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("blackRedwoodCity bodyLarge")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("blackRedwoodCity bodyMedium")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("blackRedwoodCity bodySmall")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::BLACK54)
                    .decoration(TextDecoration::NONE),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("blackRedwoodCity labelLarge")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::BLACK87)
                    .decoration(TextDecoration::NONE),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("blackRedwoodCity labelMedium")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::BLACK)
                    .decoration(TextDecoration::NONE),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("blackRedwoodCity labelSmall")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::BLACK)
                    .decoration(TextDecoration::NONE),
            ),
        }
    }

    /// A Material Design text theme with light glyphs based on San Francisco.
    ///
    /// This `TextTheme` provides color but not geometry (font size, weight, etc).
    ///
    /// This theme uses the macOS version of the font names.
    pub fn white_redwood_city() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("whiteRedwoodCity displayLarge")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("whiteRedwoodCity displayMedium")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("whiteRedwoodCity displaySmall")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("whiteRedwoodCity headlineLarge")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("whiteRedwoodCity headlineMedium")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("whiteRedwoodCity headlineSmall")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("whiteRedwoodCity titleLarge")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("whiteRedwoodCity titleMedium")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("whiteRedwoodCity titleSmall")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("whiteRedwoodCity bodyLarge")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("whiteRedwoodCity bodyMedium")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("whiteRedwoodCity bodySmall")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::WHITE70)
                    .decoration(TextDecoration::NONE),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("whiteRedwoodCity labelLarge")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("whiteRedwoodCity labelMedium")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("whiteRedwoodCity labelSmall")
                    .font_family(".AppleSystemUIFont")
                    .color(Colors::WHITE)
                    .decoration(TextDecoration::NONE),
            ),
        }
    }

    /// Defines text geometry for `ScriptCategory::EnglishLike` scripts, such as
    /// English, French, Russian, etc.
    pub fn english_like2014() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("englishLike displayLarge 2014")
                    .inherit(false)
                    .font_size(112.0)
                    .font_weight(FontWeight::W100)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("englishLike displayMedium 2014")
                    .inherit(false)
                    .font_size(56.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("englishLike displaySmall 2014")
                    .inherit(false)
                    .font_size(45.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("englishLike headlineLarge 2014")
                    .inherit(false)
                    .font_size(40.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("englishLike headlineMedium 2014")
                    .inherit(false)
                    .font_size(34.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("englishLike headlineSmall 2014")
                    .inherit(false)
                    .font_size(24.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("englishLike titleLarge 2014")
                    .inherit(false)
                    .font_size(20.0)
                    .font_weight(FontWeight::W500)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("englishLike titleMedium 2014")
                    .inherit(false)
                    .font_size(16.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("englishLike titleSmall 2014")
                    .inherit(false)
                    .font_size(14.0)
                    .font_weight(FontWeight::W500)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(0.1),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("englishLike bodyLarge 2014")
                    .inherit(false)
                    .font_size(14.0)
                    .font_weight(FontWeight::W500)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("englishLike bodyMedium 2014")
                    .inherit(false)
                    .font_size(14.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("englishLike bodySmall 2014")
                    .inherit(false)
                    .font_size(12.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("englishLike labelLarge 2014")
                    .inherit(false)
                    .font_size(14.0)
                    .font_weight(FontWeight::W500)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("englishLike labelMedium 2014")
                    .inherit(false)
                    .font_size(12.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("englishLike labelSmall 2014")
                    .inherit(false)
                    .font_size(10.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(1.5),
            ),
        }
    }

    /// Defines text geometry for `ScriptCategory::EnglishLike` scripts, such as
    /// English, French, Russian, etc.
    ///
    /// The font sizes, weights, and letter spacings in this version match the
    /// [2018 Material Design specification](https://material.io/go/design-typography#typography-styles).
    pub fn english_like2018() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("englishLike displayLarge 2018")
                    .inherit(false)
                    .font_size(96.0)
                    .font_weight(FontWeight::W300)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(-1.5),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("englishLike displayMedium 2018")
                    .inherit(false)
                    .font_size(60.0)
                    .font_weight(FontWeight::W300)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(-0.5),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("englishLike displaySmall 2018")
                    .inherit(false)
                    .font_size(48.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(0.0),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("englishLike headlineLarge 2018")
                    .inherit(false)
                    .font_size(40.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(0.25),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("englishLike headlineMedium 2018")
                    .inherit(false)
                    .font_size(34.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(0.25),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("englishLike headlineSmall 2018")
                    .inherit(false)
                    .font_size(24.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(0.0),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("englishLike titleLarge 2018")
                    .inherit(false)
                    .font_size(20.0)
                    .font_weight(FontWeight::W500)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(0.15),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("englishLike titleMedium 2018")
                    .inherit(false)
                    .font_size(16.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(0.15),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("englishLike titleSmall 2018")
                    .inherit(false)
                    .font_size(14.0)
                    .font_weight(FontWeight::W500)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(0.1),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("englishLike bodyLarge 2018")
                    .inherit(false)
                    .font_size(16.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(0.5),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("englishLike bodyMedium 2018")
                    .inherit(false)
                    .font_size(14.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(0.25),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("englishLike bodySmall 2018")
                    .inherit(false)
                    .font_size(12.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(0.4),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("englishLike labelLarge 2018")
                    .inherit(false)
                    .font_size(14.0)
                    .font_weight(FontWeight::W500)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(1.25),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("englishLike labelMedium 2018")
                    .inherit(false)
                    .font_size(11.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(1.5),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("englishLike labelSmall 2018")
                    .inherit(false)
                    .font_size(10.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic)
                    .letter_spacing(1.5),
            ),
        }
    }

    /// Defines text geometry for dense scripts, such as Chinese, Japanese
    /// and Korean.
    pub fn dense2014() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("dense displayLarge 2014")
                    .inherit(false)
                    .font_size(112.0)
                    .font_weight(FontWeight::W100)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("dense displayMedium 2014")
                    .inherit(false)
                    .font_size(56.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("dense displaySmall 2014")
                    .inherit(false)
                    .font_size(45.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("dense headlineLarge 2014")
                    .inherit(false)
                    .font_size(40.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("dense headlineMedium 2014")
                    .inherit(false)
                    .font_size(34.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("dense headlineSmall 2014")
                    .inherit(false)
                    .font_size(24.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("dense titleLarge 2014")
                    .inherit(false)
                    .font_size(21.0)
                    .font_weight(FontWeight::W500)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("dense titleMedium 2014")
                    .inherit(false)
                    .font_size(17.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("dense titleSmall 2014")
                    .inherit(false)
                    .font_size(15.0)
                    .font_weight(FontWeight::W500)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("dense bodyLarge 2014")
                    .inherit(false)
                    .font_size(15.0)
                    .font_weight(FontWeight::W500)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("dense bodyMedium 2014")
                    .inherit(false)
                    .font_size(15.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("dense bodySmall 2014")
                    .inherit(false)
                    .font_size(13.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("dense labelLarge 2014")
                    .inherit(false)
                    .font_size(15.0)
                    .font_weight(FontWeight::W500)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("dense labelMedium 2014")
                    .inherit(false)
                    .font_size(12.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("dense labelSmall 2014")
                    .inherit(false)
                    .font_size(11.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
        }
    }

    /// Defines text geometry for dense scripts, such as Chinese, Japanese
    /// and Korean.
    ///
    /// The font sizes, weights, and letter spacings in this version match the
    /// 2018 [Material Design specification](https://material.io/go/design-typography#typography-styles).
    pub fn dense2018() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("dense displayLarge 2018")
                    .inherit(false)
                    .font_size(96.0)
                    .font_weight(FontWeight::W100)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("dense displayMedium 2018")
                    .inherit(false)
                    .font_size(60.0)
                    .font_weight(FontWeight::W100)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("dense displaySmall 2018")
                    .inherit(false)
                    .font_size(48.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("dense headlineLarge 2018")
                    .inherit(false)
                    .font_size(40.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("dense headlineMedium 2018")
                    .inherit(false)
                    .font_size(34.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("dense headlineSmall 2018")
                    .inherit(false)
                    .font_size(24.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("dense titleLarge 2018")
                    .inherit(false)
                    .font_size(21.0)
                    .font_weight(FontWeight::W500)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("dense titleMedium 2018")
                    .inherit(false)
                    .font_size(17.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("dense titleSmall 2018")
                    .inherit(false)
                    .font_size(15.0)
                    .font_weight(FontWeight::W500)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("dense bodyLarge 2018")
                    .inherit(false)
                    .font_size(17.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("dense bodyMedium 2018")
                    .inherit(false)
                    .font_size(15.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("dense bodySmall 2018")
                    .inherit(false)
                    .font_size(13.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("dense labelLarge 2018")
                    .inherit(false)
                    .font_size(15.0)
                    .font_weight(FontWeight::W500)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("dense labelMedium 2018")
                    .inherit(false)
                    .font_size(12.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("dense labelSmall 2018")
                    .inherit(false)
                    .font_size(11.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Ideographic),
            ),
        }
    }

    /// Defines text geometry for tall scripts, such as Farsi, Hindi, and Thai.
    pub fn tall2014() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("tall displayLarge 2014")
                    .inherit(false)
                    .font_size(112.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("tall displayMedium 2014")
                    .inherit(false)
                    .font_size(56.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("tall displaySmall 2014")
                    .inherit(false)
                    .font_size(45.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("tall headlineLarge 2014")
                    .inherit(false)
                    .font_size(40.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("tall headlineMedium 2014")
                    .inherit(false)
                    .font_size(34.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("tall headlineSmall 2014")
                    .inherit(false)
                    .font_size(24.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("tall titleLarge 2014")
                    .inherit(false)
                    .font_size(21.0)
                    .font_weight(FontWeight::W700)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("tall titleMedium 2014")
                    .inherit(false)
                    .font_size(17.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("tall titleSmall 2014")
                    .inherit(false)
                    .font_size(15.0)
                    .font_weight(FontWeight::W500)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("tall bodyLarge 2014")
                    .inherit(false)
                    .font_size(15.0)
                    .font_weight(FontWeight::W700)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("tall bodyMedium 2014")
                    .inherit(false)
                    .font_size(15.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("tall bodySmall 2014")
                    .inherit(false)
                    .font_size(13.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("tall labelLarge 2014")
                    .inherit(false)
                    .font_size(15.0)
                    .font_weight(FontWeight::W700)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("tall labelMedium 2014")
                    .inherit(false)
                    .font_size(12.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("tall labelSmall 2014")
                    .inherit(false)
                    .font_size(11.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
        }
    }

    /// Defines text geometry for tall scripts, such as Farsi, Hindi, and Thai.
    ///
    /// The font sizes, weights, and letter spacings in this version match the
    /// 2018 [Material Design specification](https://material.io/go/design-typography#typography-styles).
    pub fn tall2018() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("tall displayLarge 2018")
                    .inherit(false)
                    .font_size(96.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("tall displayMedium 2018")
                    .inherit(false)
                    .font_size(60.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("tall displaySmall 2018")
                    .inherit(false)
                    .font_size(48.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("tall headlineLarge 2018")
                    .inherit(false)
                    .font_size(40.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("tall headlineMedium 2018")
                    .inherit(false)
                    .font_size(34.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("tall headlineSmall 2018")
                    .inherit(false)
                    .font_size(24.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("tall titleLarge 2018")
                    .inherit(false)
                    .font_size(21.0)
                    .font_weight(FontWeight::W700)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("tall titleMedium 2018")
                    .inherit(false)
                    .font_size(17.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("tall titleSmall 2018")
                    .inherit(false)
                    .font_size(15.0)
                    .font_weight(FontWeight::W500)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("tall bodyLarge 2018")
                    .inherit(false)
                    .font_size(17.0)
                    .font_weight(FontWeight::W700)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("tall bodyMedium 2018")
                    .inherit(false)
                    .font_size(15.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("tall bodySmall 2018")
                    .inherit(false)
                    .font_size(13.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("tall labelLarge 2018")
                    .inherit(false)
                    .font_size(15.0)
                    .font_weight(FontWeight::W700)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("tall labelMedium 2018")
                    .inherit(false)
                    .font_size(12.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("tall labelSmall 2018")
                    .inherit(false)
                    .font_size(11.0)
                    .font_weight(FontWeight::W400)
                    .text_baseline(TextBaseline::Alphabetic),
            ),
        }
    }

    /// Defines text geometry for `ScriptCategory::EnglishLike` scripts, such as
    /// English, French, Russian, etc.
    ///
    /// The font sizes, weights, and letter spacings in this version match the
    /// [2021 Material Design 3 specification](https://m3.material.io/styles/typography/overview).
    pub fn english_like2021() -> TextTheme {
        M3Typography::english_like()
    }

    /// Defines text geometry for dense scripts, such as Chinese, Japanese
    /// and Korean.
    ///
    /// The Material Design 3 specification does not include 'dense' text themes,
    /// so this is just here to be consistent with the API.
    pub fn dense2021() -> TextTheme {
        M3Typography::dense()
    }

    /// Defines text geometry for tall scripts, such as Farsi, Hindi, and Thai.
    ///
    /// The Material Design 3 specification does not include 'tall' text themes,
    /// so this is just here to be consistent with the API.
    pub fn tall2021() -> TextTheme {
        M3Typography::tall()
    }
}

/// Dart's `TextTheme.lerp` on two shared text themes: `identical(a, b)` returns `a` itself.
fn lerp_text_theme(a: &Rc<TextTheme>, b: &Rc<TextTheme>, t: f64) -> Rc<TextTheme> {
    if Rc::ptr_eq(a, b) {
        return Rc::clone(a);
    }
    Rc::new(TextTheme::lerp(Some(a), Some(b), t))
}

/// Dart's `Typography.material2014` and `Typography.material2018` arguments; the two
/// factories differ only in their default geometry text themes.
#[derive(Clone, Debug)]
pub struct TypographyBuilder {
    platform: Option<TargetPlatform>,
    black: Option<Rc<TextTheme>>,
    white: Option<Rc<TextTheme>>,
    english_like: Option<Rc<TextTheme>>,
    dense: Option<Rc<TextTheme>>,
    tall: Option<Rc<TextTheme>>,
    english_like_default: TextThemeConstant,
    dense_default: TextThemeConstant,
    tall_default: TextThemeConstant,
}

impl TypographyBuilder {
    fn new(
        english_like_default: TextThemeConstant,
        dense_default: TextThemeConstant,
        tall_default: TextThemeConstant,
    ) -> TypographyBuilder {
        TypographyBuilder {
            platform: Some(TargetPlatform::Android),
            black: None,
            white: None,
            english_like: None,
            dense: None,
            tall: None,
            english_like_default,
            dense_default,
            tall_default,
        }
    }

    /// Dart `platform:`, which defaults to [`TargetPlatform::Android`]; `None` is Dart's
    /// explicit null, which needs both `black` and `white`.
    pub fn platform(mut self, platform: Option<TargetPlatform>) -> TypographyBuilder {
        self.platform = platform;
        self
    }

    /// Dart `black:`.
    pub fn black(mut self, black: impl Into<Rc<TextTheme>>) -> TypographyBuilder {
        self.black = Some(black.into());
        self
    }

    /// Dart `white:`.
    pub fn white(mut self, white: impl Into<Rc<TextTheme>>) -> TypographyBuilder {
        self.white = Some(white.into());
        self
    }

    /// Dart `englishLike:`.
    pub fn english_like(mut self, english_like: impl Into<Rc<TextTheme>>) -> TypographyBuilder {
        self.english_like = Some(english_like.into());
        self
    }

    /// Dart `dense:`.
    pub fn dense(mut self, dense: impl Into<Rc<TextTheme>>) -> TypographyBuilder {
        self.dense = Some(dense.into());
        self
    }

    /// Dart `tall:`.
    pub fn tall(mut self, tall: impl Into<Rc<TextTheme>>) -> TypographyBuilder {
        self.tall = Some(tall.into());
        self
    }

    /// The typography the factory returns.
    pub fn build(self) -> Typography {
        debug_assert!(self.platform.is_some() || (self.black.is_some() && self.white.is_some()));
        Typography::with_platform(
            self.platform,
            self.black,
            self.white,
            self.english_like
                .unwrap_or_else(|| Rc::new((self.english_like_default)())),
            self.dense
                .unwrap_or_else(|| Rc::new((self.dense_default)())),
            self.tall.unwrap_or_else(|| Rc::new((self.tall_default)())),
        )
    }
}

/// Dart's `Typography.material2021` arguments.
#[derive(Clone, Debug)]
pub struct TypographyMaterial2021Builder {
    platform: Option<TargetPlatform>,
    color_scheme: ColorScheme,
    black: Option<Rc<TextTheme>>,
    white: Option<Rc<TextTheme>>,
    english_like: Option<Rc<TextTheme>>,
    dense: Option<Rc<TextTheme>>,
    tall: Option<Rc<TextTheme>>,
}

impl TypographyMaterial2021Builder {
    /// Dart `platform:`, which defaults to [`TargetPlatform::Android`]; `None` is Dart's
    /// explicit null, which needs both `black` and `white`.
    pub fn platform(mut self, platform: Option<TargetPlatform>) -> TypographyMaterial2021Builder {
        self.platform = platform;
        self
    }

    /// Dart `colorScheme:`, which defaults to `ColorScheme::light()`.
    pub fn color_scheme(mut self, color_scheme: ColorScheme) -> TypographyMaterial2021Builder {
        self.color_scheme = color_scheme;
        self
    }

    /// Dart `black:`.
    #[deprecated(
        note = "Text colors are derived from colorScheme, so colors in this text theme are ignored. Use copyWith on the result to customize the black text theme. This feature was deprecated after material_ui v1.8.0."
    )]
    pub fn black(mut self, black: impl Into<Rc<TextTheme>>) -> TypographyMaterial2021Builder {
        self.black = Some(black.into());
        self
    }

    /// Dart `white:`.
    #[deprecated(
        note = "Text colors are derived from colorScheme, so colors in this text theme are ignored. Use copyWith on the result to customize the white text theme. This feature was deprecated after material_ui v1.8.0."
    )]
    pub fn white(mut self, white: impl Into<Rc<TextTheme>>) -> TypographyMaterial2021Builder {
        self.white = Some(white.into());
        self
    }

    /// Dart `englishLike:`.
    pub fn english_like(
        mut self,
        english_like: impl Into<Rc<TextTheme>>,
    ) -> TypographyMaterial2021Builder {
        self.english_like = Some(english_like.into());
        self
    }

    /// Dart `dense:`.
    pub fn dense(mut self, dense: impl Into<Rc<TextTheme>>) -> TypographyMaterial2021Builder {
        self.dense = Some(dense.into());
        self
    }

    /// Dart `tall:`.
    pub fn tall(mut self, tall: impl Into<Rc<TextTheme>>) -> TypographyMaterial2021Builder {
        self.tall = Some(tall.into());
        self
    }

    /// The typography the factory returns.
    pub fn build(self) -> Typography {
        debug_assert!(self.platform.is_some() || (self.black.is_some() && self.white.is_some()));
        let base = Typography::with_platform(
            self.platform,
            self.black,
            self.white,
            self.english_like
                .unwrap_or_else(|| Rc::new(Typography::english_like2021())),
            self.dense
                .unwrap_or_else(|| Rc::new(Typography::dense2021())),
            self.tall.unwrap_or_else(|| Rc::new(Typography::tall2021())),
        );

        // Ensure they are all uniformly dark or light, with
        // no color variation based on style as it was in previous
        // versions of Material Design.
        let color_scheme = &self.color_scheme;
        let dark = if color_scheme.brightness == Brightness::Light {
            color_scheme.on_surface
        } else {
            color_scheme.surface
        };
        let light = if color_scheme.brightness == Brightness::Light {
            color_scheme.surface
        } else {
            color_scheme.on_surface
        };
        let black = base
            .black
            .apply()
            .display_color(dark)
            .body_color(dark)
            .decoration_color(dark)
            .into_text_theme();
        let white = base
            .white
            .apply()
            .display_color(light)
            .body_color(light)
            .decoration_color(light)
            .into_text_theme();
        Typography {
            black: Rc::new(black),
            white: Rc::new(white),
            ..base
        }
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{Color, TextLeadingDistribution};
    use waft_painting::AnyColor;

    use super::*;
    use crate::theme_data::ThemeData;

    const ALL_PLATFORMS: [TargetPlatform; 6] = [
        TargetPlatform::Android,
        TargetPlatform::Fuchsia,
        TargetPlatform::IOS,
        TargetPlatform::Linux,
        TargetPlatform::MacOS,
        TargetPlatform::Windows,
    ];

    fn styles(theme: &TextTheme) -> [&Option<TextStyle>; 15] {
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

    fn style(style: &Option<TextStyle>) -> &TextStyle {
        style.as_ref().expect("the text theme has this style")
    }

    fn family(style: &Option<TextStyle>) -> Option<&str> {
        self::style(style).font_family.as_deref()
    }

    fn color(color: Color) -> Option<AnyColor> {
        Some(AnyColor::new(color))
    }

    #[test]
    fn typography_is_defined_for_all_target_platforms() {
        for platform in ALL_PLATFORMS {
            let typography = Typography::material2018().platform(Some(platform)).build();
            for style in styles(&typography.black)
                .into_iter()
                .chain(styles(&typography.white))
            {
                assert!(style.is_some(), "null typography for {platform:?}");
            }
        }
    }

    #[test]
    fn typography_lerp_of_a_typography_with_itself_is_that_typography() {
        let typography = Typography::new().build();
        assert_eq!(Typography::lerp(&typography, &typography, 0.5), typography);
    }

    #[test]
    fn typography_on_non_apple_platforms_defaults_to_the_correct_font() {
        let on = |platform| Typography::material2018().platform(Some(platform)).build();
        let helsinki = [
            "Ubuntu",
            "Adwaita Sans",
            "Cantarell",
            "DejaVu Sans",
            "Liberation Sans",
            "Arial",
        ]
        .map(String::from)
        .to_vec();
        let android = Typography::material2018().build();
        for theme in [&android.black, &android.white] {
            assert_eq!(family(&theme.title_large), Some("Roboto"));
        }
        for platform in [TargetPlatform::Fuchsia, TargetPlatform::Linux] {
            let typography = on(platform);
            assert_eq!(family(&typography.black.title_large), Some("Roboto"));
            assert_eq!(family(&typography.white.title_large), Some("Roboto"));
        }
        let linux = on(TargetPlatform::Linux);
        assert_eq!(
            style(&linux.black.title_large).font_family_fallback_list(),
            Some(helsinki.clone())
        );
        assert_eq!(
            style(&linux.white.title_large).font_family_fallback_list(),
            Some(helsinki)
        );
        let windows = on(TargetPlatform::Windows);
        assert_eq!(family(&windows.black.title_large), Some("Segoe UI"));
        assert_eq!(family(&windows.white.title_large), Some("Segoe UI"));
    }

    #[test]
    fn typography_on_ios_defaults_to_the_correct_sf_font_family_based_on_size() {
        let typography = Typography::material2018()
            .platform(Some(TargetPlatform::IOS))
            .build();
        for theme in [&typography.black, &typography.white] {
            let families: Vec<_> = styles(theme).into_iter().map(family).collect();
            let display = Some("CupertinoSystemDisplay");
            let text = Some("CupertinoSystemText");
            assert_eq!(
                families,
                [
                    display, display, display, display, display, display, display, text, text,
                    text, text, text, text, text, text,
                ]
            );
        }
    }

    #[test]
    fn typography_on_macos_defaults_to_the_system_ui_meta_font() {
        let typography = Typography::material2018()
            .platform(Some(TargetPlatform::MacOS))
            .build();
        for theme in [&typography.black, &typography.white] {
            for style in styles(theme) {
                assert_eq!(family(style), Some(".AppleSystemUIFont"));
            }
        }
    }

    #[test]
    fn can_lerp_between_different_typographies() {
        let mut all = Vec::new();
        for platform in ALL_PLATFORMS {
            all.push(Typography::material2014().platform(Some(platform)).build());
        }
        for platform in ALL_PLATFORMS {
            all.push(Typography::material2018().platform(Some(platform)).build());
        }
        for platform in ALL_PLATFORMS {
            all.push(Typography::material2021().platform(Some(platform)).build());
        }
        for from in &all {
            for to in &all {
                Typography::lerp(from, to, 0.5);
            }
        }
    }

    /// `(family, weight, size, letter spacing)` of each style, in [`styles`] order.
    fn assert_geometry(theme: &TextTheme, expected: [(FontWeight, f64, f64); 15]) {
        for (style, (weight, size, letter_spacing)) in styles(theme).into_iter().zip(expected) {
            let style = self::style(style);
            assert_eq!(style.font_family.as_deref(), Some("Roboto"));
            assert_eq!(style.font_weight, Some(weight));
            assert_eq!(style.font_size, Some(size));
            assert_eq!(style.letter_spacing, Some(letter_spacing));
        }
    }

    #[test]
    fn english_like2018_text_theme_matches_material_design_spec() {
        // Check the default material text theme against the style values
        // shown https://material.io/design/typography/#type-scale.
        let theme = Typography::english_like2018().merge(Some(&Typography::black_mountain_view()));
        let light = FontWeight::W300;
        let regular = FontWeight::W400;
        let medium = FontWeight::W500;
        assert_geometry(
            &theme,
            [
                // Display Large Roboto light 96 -1.5
                (light, 96.0, -1.5),
                // Display Medium Roboto light 60 -0.5
                (light, 60.0, -0.5),
                // Display Small Roboto regular 48 0
                (regular, 48.0, 0.0),
                // Headline Large (from Material 3 for backwards compatibility) Roboto regular 40 0.25
                (regular, 40.0, 0.25),
                // Headline Medium Roboto regular 34 0.25
                (regular, 34.0, 0.25),
                // Headline Small Roboto regular 24 0
                (regular, 24.0, 0.0),
                // Title Large Roboto medium 20 0.15
                (medium, 20.0, 0.15),
                // Title Medium Roboto regular 16 0.15
                (regular, 16.0, 0.15),
                // Title Small Roboto medium 14 0.1
                (medium, 14.0, 0.1),
                // Body Large Roboto regular 16 0.5
                (regular, 16.0, 0.5),
                // Body Medium Roboto regular 14 0.25
                (regular, 14.0, 0.25),
                // Body Small Roboto regular 12 0.4
                (regular, 12.0, 0.4),
                // Label Large Roboto medium 14 1.25
                (medium, 14.0, 1.25),
                // Label Medium (from Material 3 for backwards compatibility) Roboto regular 11 1.5
                (regular, 11.0, 1.5),
                // Label Small Roboto regular 10 1.5
                (regular, 10.0, 1.5),
            ],
        );
    }

    #[test]
    fn english_like2021_text_theme_matches_material_design_3_spec() {
        // Check the default material text theme against the style values
        // shown https://m3.material.io/styles/typography/tokens.
        //
        // This may need to be updated if the token values change.
        let theme = Typography::english_like2021().merge(Some(&Typography::black_mountain_view()));
        let regular = FontWeight::W400;
        let medium = FontWeight::W500;
        assert_geometry(
            &theme,
            [
                (regular, 57.0, -0.25),
                (regular, 45.0, 0.0),
                (regular, 36.0, 0.0),
                (regular, 32.0, 0.0),
                (regular, 28.0, 0.0),
                (regular, 24.0, 0.0),
                (regular, 22.0, 0.0),
                (medium, 16.0, 0.15),
                (medium, 14.0, 0.1),
                (regular, 16.0, 0.5),
                (regular, 14.0, 0.25),
                (regular, 12.0, 0.4),
                (medium, 14.0, 0.1),
                (medium, 12.0, 0.5),
                (medium, 11.0, 0.5),
            ],
        );
        let heights = [
            1.12, 1.16, 1.22, 1.25, 1.29, 1.33, 1.27, 1.50, 1.43, 1.50, 1.43, 1.33, 1.43, 1.33,
            1.45,
        ];
        for (style, height) in styles(&theme).into_iter().zip(heights) {
            let style = self::style(style);
            assert_eq!(style.height, Some(height));
            assert_eq!(style.text_baseline, Some(TextBaseline::Alphabetic));
            assert_eq!(
                style.leading_distribution,
                Some(TextLeadingDistribution::Even)
            );
        }
    }

    #[test]
    fn default_m3_text_theme_styles_all_use_on_surface() {
        let cell = crate::test_support::test_cell();
        let app = cell.borrow();
        for brightness in [Brightness::Light, Brightness::Dark] {
            let theme = ThemeData::builder().brightness(brightness).build(&app);
            for style in styles(&theme.text_theme) {
                assert_eq!(
                    self::style(style).color,
                    color(theme.color_scheme.on_surface)
                );
            }
        }
    }

    fn light_scheme() -> ColorScheme {
        ColorScheme::light()
            .with_on_surface(Color::new(0xFF000001))
            .with_surface(Color::new(0xFFFFFFF1))
    }

    fn custom() -> TextTheme {
        TextTheme::new().body_medium(
            TextStyle::new()
                .font_family("Custom")
                .color(Color::new(0xFFFF0000))
                .decoration_color(Color::new(0xFF00FF00)),
        )
    }

    #[test]
    #[allow(deprecated)]
    fn material2021_keeps_non_color_properties_of_black_and_white_but_replaces_colors_from_the_color_scheme()
     {
        let light_scheme = light_scheme();
        let typography = Typography::material2021()
            .color_scheme(light_scheme.clone())
            .black(custom())
            .white(custom())
            .build();
        let black = style(&typography.black.body_medium);
        assert_eq!(black.font_family.as_deref(), Some("Custom"));
        assert_eq!(black.color, color(light_scheme.on_surface));
        assert_eq!(black.decoration_color, color(light_scheme.on_surface));
        let white = style(&typography.white.body_medium);
        assert_eq!(white.font_family.as_deref(), Some("Custom"));
        assert_eq!(white.color, color(light_scheme.surface));
        assert_eq!(white.decoration_color, color(light_scheme.surface));
    }

    #[test]
    fn material2021_copy_with_can_be_used_to_customize_colors_instead() {
        let light_scheme = light_scheme();
        let base = Typography::material2021()
            .color_scheme(light_scheme.clone())
            .build();
        let typography = base
            .copy_with()
            .black(base.black.merge(Some(&custom())))
            .white(base.white.merge(Some(&custom())));
        let black = style(&typography.black.body_medium);
        assert_eq!(black.font_family.as_deref(), Some("Custom"));
        assert_eq!(black.color, color(Color::new(0xFFFF0000)));
        assert_eq!(
            style(&typography.white.body_medium).color,
            color(Color::new(0xFFFF0000))
        );
        // Styles that are not overridden keep the generated values.
        assert_eq!(family(&typography.black.body_large), Some("Roboto"));
        assert_eq!(
            style(&typography.black.body_large).color,
            color(light_scheme.on_surface)
        );
    }

    #[test]
    fn material2021_colors_black_and_white_text_themes_from_the_color_scheme() {
        let light_scheme = light_scheme();
        let dark_scheme = ColorScheme::dark()
            .with_on_surface(Color::new(0xFFFFFFF2))
            .with_surface(Color::new(0xFF000002));

        let expect_uniform = |theme: &TextTheme, expected: Color| {
            for style in styles(theme) {
                let style = self::style(style);
                assert_eq!(style.color, color(expected));
                assert_eq!(style.decoration_color, color(expected));
            }
        };

        let light = Typography::material2021()
            .color_scheme(light_scheme.clone())
            .build();
        expect_uniform(&light.black, light_scheme.on_surface);
        expect_uniform(&light.white, light_scheme.surface);

        let dark = Typography::material2021()
            .color_scheme(dark_scheme.clone())
            .build();
        expect_uniform(&dark.black, dark_scheme.surface);
        expect_uniform(&dark.white, dark_scheme.on_surface);
    }

    #[test]
    fn material2021_uses_the_provided_geometry_text_themes() {
        let english_like = TextTheme::new().body_medium(TextStyle::new().font_size(1.0));
        let dense = TextTheme::new().body_medium(TextStyle::new().font_size(2.0));
        let tall = TextTheme::new().body_medium(TextStyle::new().font_size(3.0));

        let typography = Typography::material2021()
            .english_like(english_like.clone())
            .dense(dense.clone())
            .tall(tall.clone())
            .build();
        assert_eq!(*typography.english_like, english_like);
        assert_eq!(*typography.dense, dense);
        assert_eq!(*typography.tall, tall);

        let defaults = Typography::material2021().build();
        assert_eq!(*defaults.english_like, Typography::english_like2021());
        assert_eq!(*defaults.dense, Typography::dense2021());
        assert_eq!(*defaults.tall, Typography::tall2021());
    }
}
