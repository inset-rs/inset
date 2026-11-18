//! Flutter counterpart: material_ui's `color_scheme.dart`.
//!
//! Every colour of a `ColorScheme` is read through a method (`scheme.primary()`), and its fields
//! are private: Dart reads properties, and the nullable colours keep Flutter's fallbacks inside
//! the getter, as Dart's `_primaryContainer ?? primary`; they are set with the `with_*` setters.
//! `fromSeed` and `fromSwatch` take builders, since their arguments decide every colour at once.
//! `fromImageProvider` waits for reading an image's pixels (see this folder's `PORTING.md`).

use std::rc::Rc;

use waft_embedder::{Brightness, Color};
use waft_foundation::{App, PRECISION_ERROR_TOLERANCE};
use waft_material_color_utilities::{
    DynamicColor, DynamicScheme, Hct, MaterialDynamicColors, SchemeContent, SchemeExpressive,
    SchemeFidelity, SchemeFruitSalad, SchemeMonochrome, SchemeNeutral, SchemeRainbow,
    SchemeTonalSpot, SchemeVibrant,
};
use waft_widgets::BuildContext;

use crate::colors::{Colors, MaterialColor};
use crate::theme::Theme;
use crate::theme_data::ThemeData;

/// The algorithm used to construct a `ColorScheme` in `ColorScheme::from_seed`.
///
/// The `tonalSpot` variant builds default Material scheme colors. These colors are
/// mapped to light or dark tones to achieve visually accessible color
/// pairings with sufficient contrast between foreground and background elements.
///
/// In some cases, the tones can prevent colors from appearing as intended,
/// such as when a color is too light to offer enough contrast for accessibility.
/// Color fidelity (`DynamicSchemeVariant.fidelity`) is a feature that adjusts
/// tones in these cases to produce the intended visual results without harming
/// visual contrast.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DynamicSchemeVariant {
    /// Default for Material theme colors. Builds pastel palettes with a low chroma.
    TonalSpot,

    /// The resulting color palettes match seed color, even if the seed color
    /// is very bright (high chroma).
    Fidelity,

    /// All colors are grayscale, no chroma.
    Monochrome,

    /// Close to grayscale, a hint of chroma.
    Neutral,

    /// Pastel colors, high chroma palettes. The primary palette's chroma is at
    /// maximum. Use `fidelity` instead if tokens should alter their tone to match
    /// the palette vibrancy.
    Vibrant,

    /// Pastel colors, medium chroma palettes. The primary palette's hue is
    /// different from the seed color, for variety.
    Expressive,

    /// Almost identical to `fidelity`. Tokens and palettes match the seed color.
    /// `ColorScheme::primary_container` is the seed color, adjusted to ensure
    /// contrast with surfaces. The tertiary palette is analogue of the seed color.
    Content,

    /// A playful theme - the seed color's hue does not appear in the theme.
    Rainbow,

    /// A playful theme - the seed color's hue does not appear in the theme.
    FruitSalad,
}

impl DynamicSchemeVariant {
    /// Dart's `DynamicSchemeVariant.values`, in declaration order.
    pub const VALUES: [DynamicSchemeVariant; 9] = [
        DynamicSchemeVariant::TonalSpot,
        DynamicSchemeVariant::Fidelity,
        DynamicSchemeVariant::Monochrome,
        DynamicSchemeVariant::Neutral,
        DynamicSchemeVariant::Vibrant,
        DynamicSchemeVariant::Expressive,
        DynamicSchemeVariant::Content,
        DynamicSchemeVariant::Rainbow,
        DynamicSchemeVariant::FruitSalad,
    ];
}

/// {@template material_ui.color_scheme.ColorScheme}
/// A set of 45 colors based on the
/// [Material spec](https://m3.material.io/styles/color/the-color-system/color-roles)
/// that can be used to configure the color properties of most components.
/// {@endtemplate}
///
/// ### Colors in Material 3
///
/// {@macro material_ui.colors.colorRoles}
///
/// The main accent color groups in the scheme are `primary`, `secondary`,
/// and `tertiary`.
///
/// * Primary colors are used for key components across the UI, such as the FAB,
///   prominent buttons, and active states.
///
/// * Secondary colors are used for less prominent components in the UI, such as
///   filter chips, while expanding the opportunity for color expression.
///
/// * Tertiary colors are used for contrasting accents that can be used to
///   balance primary and secondary colors or bring heightened attention to
///   an element, such as an input field. The tertiary colors are left
///   for makers to use at their discretion and are intended to support
///   broader color expression in products.
///
/// Each accent color group (primary, secondary and tertiary) includes '-Fixed'
/// '-Dim' color roles, such as `primary_fixed` and `primary_fixed_dim`. Fixed roles
/// are appropriate to use in places where Container roles are normally used,
/// but they stay the same color between light and dark themes. The '-Dim' roles
/// provide a stronger, more emphasized color with the same fixed behavior.
///
/// The remaining colors of the scheme are composed of neutral colors used for
/// backgrounds and surfaces, as well as specific colors for errors, dividers
/// and shadows. Surface colors are used for backgrounds and large, low-emphasis
/// areas of the screen.
///
/// Material 3 also introduces tone-based surfaces and surface containers.
/// They replace the old opacity-based model which applied a tinted overlay on
/// top of surfaces based on their elevation. These colors include: `surface_bright`,
/// `surface_dim`, `surface_container_lowest`, `surface_container_low`, `surface_container`,
/// `surface_container_high`, and `surface_container_highest`.
///
/// Many of the colors have matching 'on' colors, which are used for drawing
/// content on top of the matching color. For example, if something is using
/// `primary` for a background color, `on_primary` would be used to paint text
/// and icons on top of it. For this reason, the 'on' colors should have a
/// contrast ratio with their matching colors of at least 4.5:1 in order to
/// be readable. On '-FixedVariant' roles, such as `on_primary_fixed_variant`,
/// also have the same color between light and dark themes, but compared
/// with on '-Fixed' roles, such as `on_primary_fixed`, they provide a
/// lower-emphasis option for text and icons.
///
/// <callout-box>
///
/// This example shows all Material `ColorScheme` roles in light and dark
/// brightnesses.
///
// TODO(framework): Replace the following block with a @dartpad directive
// when it's supported. https://github.com/dart-lang/dartdoc/issues/4123
/// {@macro material_ui.dartpad_guide}
///
/// {@example /example/lib/color_scheme/color_scheme.0.dart#body}
///
/// </callout-box>
///
/// ### Setting Colors in Flutter
///
///{@macro material_ui.colors.settingColors}
#[derive(Clone, Debug)]
pub struct ColorScheme {
    brightness: Brightness,
    /// The color displayed most frequently across your app’s screens and components.
    primary: Color,
    /// A color that's clearly legible when drawn on `primary`.
    ///
    /// To ensure that an app is accessible, a contrast ratio between
    /// `primary` and `on_primary` of at least 4.5:1 is recommended. See
    /// <https://www.w3.org/TR/UNDERSTANDING-WCAG20/visual-audio-contrast-contrast.html>.
    on_primary: Color,
    primary_container: Option<Color>,
    on_primary_container: Option<Color>,
    primary_fixed: Option<Color>,
    primary_fixed_dim: Option<Color>,
    on_primary_fixed: Option<Color>,
    on_primary_fixed_variant: Option<Color>,
    /// An accent color used for less prominent components in the UI, such as
    /// filter chips, while expanding the opportunity for color expression.
    secondary: Color,
    /// A color that's clearly legible when drawn on `secondary`.
    ///
    /// To ensure that an app is accessible, a contrast ratio between
    /// `secondary` and `on_secondary` of at least 4.5:1 is recommended. See
    /// <https://www.w3.org/TR/UNDERSTANDING-WCAG20/visual-audio-contrast-contrast.html>.
    on_secondary: Color,
    secondary_container: Option<Color>,
    on_secondary_container: Option<Color>,
    secondary_fixed: Option<Color>,
    secondary_fixed_dim: Option<Color>,
    on_secondary_fixed: Option<Color>,
    on_secondary_fixed_variant: Option<Color>,
    tertiary: Option<Color>,
    on_tertiary: Option<Color>,
    tertiary_container: Option<Color>,
    on_tertiary_container: Option<Color>,
    tertiary_fixed: Option<Color>,
    tertiary_fixed_dim: Option<Color>,
    on_tertiary_fixed: Option<Color>,
    on_tertiary_fixed_variant: Option<Color>,
    /// The color to use for input validation errors, e.g. for
    /// `InputDecoration.errorText`.
    error: Color,
    /// A color that's clearly legible when drawn on `error`.
    ///
    /// To ensure that an app is accessible, a contrast ratio between
    /// `error` and `on_error` of at least 4.5:1 is recommended. See
    /// <https://www.w3.org/TR/UNDERSTANDING-WCAG20/visual-audio-contrast-contrast.html>.
    on_error: Color,
    error_container: Option<Color>,
    on_error_container: Option<Color>,
    /// The background color for widgets like `Scaffold`.
    surface: Color,
    /// A color that's clearly legible when drawn on `surface`.
    ///
    /// To ensure that an app is accessible, a contrast ratio between
    /// `surface` and `on_surface` of at least 4.5:1 is recommended. See
    /// <https://www.w3.org/TR/UNDERSTANDING-WCAG20/visual-audio-contrast-contrast.html>.
    on_surface: Color,
    surface_variant: Option<Color>,
    surface_dim: Option<Color>,
    surface_bright: Option<Color>,
    surface_container_lowest: Option<Color>,
    surface_container_low: Option<Color>,
    surface_container: Option<Color>,
    surface_container_high: Option<Color>,
    surface_container_highest: Option<Color>,
    on_surface_variant: Option<Color>,
    outline: Option<Color>,
    outline_variant: Option<Color>,
    shadow: Option<Color>,
    scrim: Option<Color>,
    inverse_surface: Option<Color>,
    on_inverse_surface: Option<Color>,
    inverse_primary: Option<Color>,
    surface_tint: Option<Color>,
    background: Option<Color>,
    on_background: Option<Color>,
}

impl ColorScheme {
    /// Create a ColorScheme instance from the given colors.
    ///
    /// `ColorScheme::from_seed` can be used as a simpler way to create a full
    /// color scheme derived from a single seed color.
    ///
    /// For the color parameters that are nullable, it is still recommended
    /// that applications provide values for them. They are only nullable due
    /// to backwards compatibility concerns.
    ///
    /// If a color is not provided, the closest fallback color from the given
    /// colors will be used for it (e.g. `primary_container` will default
    /// to `primary`). Material Design 3 makes use of these colors for many
    /// component defaults, so for the best results the application should
    /// supply colors for all the parameters. An easy way to ensure this is to
    /// use `ColorScheme::from_seed` to generate a full set of colors.
    ///
    /// During the migration to Material Design 3, if an app's
    /// `ThemeData.useMaterial3` is false, then components will only
    /// use the following colors for defaults:
    ///
    /// * `primary`
    /// * `on_primary`
    /// * `secondary`
    /// * `on_secondary`
    /// * `error`
    /// * `on_error`
    /// * `surface`
    /// * `on_surface`
    ///
    /// DEPRECATED:
    /// * `background`
    /// * `on_background`
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        brightness: Brightness,
        primary: Color,
        on_primary: Color,
        secondary: Color,
        on_secondary: Color,
        error: Color,
        on_error: Color,
        surface: Color,
        on_surface: Color,
    ) -> ColorScheme {
        ColorScheme {
            brightness,
            primary,
            on_primary,
            primary_container: None,
            on_primary_container: None,
            primary_fixed: None,
            primary_fixed_dim: None,
            on_primary_fixed: None,
            on_primary_fixed_variant: None,
            secondary,
            on_secondary,
            secondary_container: None,
            on_secondary_container: None,
            secondary_fixed: None,
            secondary_fixed_dim: None,
            on_secondary_fixed: None,
            on_secondary_fixed_variant: None,
            tertiary: None,
            on_tertiary: None,
            tertiary_container: None,
            on_tertiary_container: None,
            tertiary_fixed: None,
            tertiary_fixed_dim: None,
            on_tertiary_fixed: None,
            on_tertiary_fixed_variant: None,
            error,
            on_error,
            error_container: None,
            on_error_container: None,
            surface,
            on_surface,
            surface_variant: None,
            surface_dim: None,
            surface_bright: None,
            surface_container_lowest: None,
            surface_container_low: None,
            surface_container: None,
            surface_container_high: None,
            surface_container_highest: None,
            on_surface_variant: None,
            outline: None,
            outline_variant: None,
            shadow: None,
            scrim: None,
            inverse_surface: None,
            on_inverse_surface: None,
            inverse_primary: None,
            surface_tint: None,
            background: None,
            on_background: None,
        }
    }

    /// Dart `ColorScheme(brightness:)`.
    pub fn with_brightness(mut self, brightness: Brightness) -> ColorScheme {
        self.brightness = brightness;
        self
    }

    /// Dart `ColorScheme(primary:)`.
    pub fn with_primary(mut self, primary: Color) -> ColorScheme {
        self.primary = primary;
        self
    }

    /// Dart `ColorScheme(onPrimary:)`.
    pub fn with_on_primary(mut self, on_primary: Color) -> ColorScheme {
        self.on_primary = on_primary;
        self
    }

    /// Dart `ColorScheme(primaryContainer:)`.
    pub fn with_primary_container(mut self, primary_container: Color) -> ColorScheme {
        self.primary_container = Some(primary_container);
        self
    }

    /// Dart `ColorScheme(onPrimaryContainer:)`.
    pub fn with_on_primary_container(mut self, on_primary_container: Color) -> ColorScheme {
        self.on_primary_container = Some(on_primary_container);
        self
    }

    /// Dart `ColorScheme(primaryFixed:)`.
    pub fn with_primary_fixed(mut self, primary_fixed: Color) -> ColorScheme {
        self.primary_fixed = Some(primary_fixed);
        self
    }

    /// Dart `ColorScheme(primaryFixedDim:)`.
    pub fn with_primary_fixed_dim(mut self, primary_fixed_dim: Color) -> ColorScheme {
        self.primary_fixed_dim = Some(primary_fixed_dim);
        self
    }

    /// Dart `ColorScheme(onPrimaryFixed:)`.
    pub fn with_on_primary_fixed(mut self, on_primary_fixed: Color) -> ColorScheme {
        self.on_primary_fixed = Some(on_primary_fixed);
        self
    }

    /// Dart `ColorScheme(onPrimaryFixedVariant:)`.
    pub fn with_on_primary_fixed_variant(mut self, on_primary_fixed_variant: Color) -> ColorScheme {
        self.on_primary_fixed_variant = Some(on_primary_fixed_variant);
        self
    }

    /// Dart `ColorScheme(secondary:)`.
    pub fn with_secondary(mut self, secondary: Color) -> ColorScheme {
        self.secondary = secondary;
        self
    }

    /// Dart `ColorScheme(onSecondary:)`.
    pub fn with_on_secondary(mut self, on_secondary: Color) -> ColorScheme {
        self.on_secondary = on_secondary;
        self
    }

    /// Dart `ColorScheme(secondaryContainer:)`.
    pub fn with_secondary_container(mut self, secondary_container: Color) -> ColorScheme {
        self.secondary_container = Some(secondary_container);
        self
    }

    /// Dart `ColorScheme(onSecondaryContainer:)`.
    pub fn with_on_secondary_container(mut self, on_secondary_container: Color) -> ColorScheme {
        self.on_secondary_container = Some(on_secondary_container);
        self
    }

    /// Dart `ColorScheme(secondaryFixed:)`.
    pub fn with_secondary_fixed(mut self, secondary_fixed: Color) -> ColorScheme {
        self.secondary_fixed = Some(secondary_fixed);
        self
    }

    /// Dart `ColorScheme(secondaryFixedDim:)`.
    pub fn with_secondary_fixed_dim(mut self, secondary_fixed_dim: Color) -> ColorScheme {
        self.secondary_fixed_dim = Some(secondary_fixed_dim);
        self
    }

    /// Dart `ColorScheme(onSecondaryFixed:)`.
    pub fn with_on_secondary_fixed(mut self, on_secondary_fixed: Color) -> ColorScheme {
        self.on_secondary_fixed = Some(on_secondary_fixed);
        self
    }

    /// Dart `ColorScheme(onSecondaryFixedVariant:)`.
    pub fn with_on_secondary_fixed_variant(
        mut self,
        on_secondary_fixed_variant: Color,
    ) -> ColorScheme {
        self.on_secondary_fixed_variant = Some(on_secondary_fixed_variant);
        self
    }

    /// Dart `ColorScheme(tertiary:)`.
    pub fn with_tertiary(mut self, tertiary: Color) -> ColorScheme {
        self.tertiary = Some(tertiary);
        self
    }

    /// Dart `ColorScheme(onTertiary:)`.
    pub fn with_on_tertiary(mut self, on_tertiary: Color) -> ColorScheme {
        self.on_tertiary = Some(on_tertiary);
        self
    }

    /// Dart `ColorScheme(tertiaryContainer:)`.
    pub fn with_tertiary_container(mut self, tertiary_container: Color) -> ColorScheme {
        self.tertiary_container = Some(tertiary_container);
        self
    }

    /// Dart `ColorScheme(onTertiaryContainer:)`.
    pub fn with_on_tertiary_container(mut self, on_tertiary_container: Color) -> ColorScheme {
        self.on_tertiary_container = Some(on_tertiary_container);
        self
    }

    /// Dart `ColorScheme(tertiaryFixed:)`.
    pub fn with_tertiary_fixed(mut self, tertiary_fixed: Color) -> ColorScheme {
        self.tertiary_fixed = Some(tertiary_fixed);
        self
    }

    /// Dart `ColorScheme(tertiaryFixedDim:)`.
    pub fn with_tertiary_fixed_dim(mut self, tertiary_fixed_dim: Color) -> ColorScheme {
        self.tertiary_fixed_dim = Some(tertiary_fixed_dim);
        self
    }

    /// Dart `ColorScheme(onTertiaryFixed:)`.
    pub fn with_on_tertiary_fixed(mut self, on_tertiary_fixed: Color) -> ColorScheme {
        self.on_tertiary_fixed = Some(on_tertiary_fixed);
        self
    }

    /// Dart `ColorScheme(onTertiaryFixedVariant:)`.
    pub fn with_on_tertiary_fixed_variant(
        mut self,
        on_tertiary_fixed_variant: Color,
    ) -> ColorScheme {
        self.on_tertiary_fixed_variant = Some(on_tertiary_fixed_variant);
        self
    }

    /// Dart `ColorScheme(error:)`.
    pub fn with_error(mut self, error: Color) -> ColorScheme {
        self.error = error;
        self
    }

    /// Dart `ColorScheme(onError:)`.
    pub fn with_on_error(mut self, on_error: Color) -> ColorScheme {
        self.on_error = on_error;
        self
    }

    /// Dart `ColorScheme(errorContainer:)`.
    pub fn with_error_container(mut self, error_container: Color) -> ColorScheme {
        self.error_container = Some(error_container);
        self
    }

    /// Dart `ColorScheme(onErrorContainer:)`.
    pub fn with_on_error_container(mut self, on_error_container: Color) -> ColorScheme {
        self.on_error_container = Some(on_error_container);
        self
    }

    /// Dart `ColorScheme(surface:)`.
    pub fn with_surface(mut self, surface: Color) -> ColorScheme {
        self.surface = surface;
        self
    }

    /// Dart `ColorScheme(onSurface:)`.
    pub fn with_on_surface(mut self, on_surface: Color) -> ColorScheme {
        self.on_surface = on_surface;
        self
    }

    /// Dart `ColorScheme(surfaceDim:)`.
    pub fn with_surface_dim(mut self, surface_dim: Color) -> ColorScheme {
        self.surface_dim = Some(surface_dim);
        self
    }

    /// Dart `ColorScheme(surfaceBright:)`.
    pub fn with_surface_bright(mut self, surface_bright: Color) -> ColorScheme {
        self.surface_bright = Some(surface_bright);
        self
    }

    /// Dart `ColorScheme(surfaceContainerLowest:)`.
    pub fn with_surface_container_lowest(mut self, surface_container_lowest: Color) -> ColorScheme {
        self.surface_container_lowest = Some(surface_container_lowest);
        self
    }

    /// Dart `ColorScheme(surfaceContainerLow:)`.
    pub fn with_surface_container_low(mut self, surface_container_low: Color) -> ColorScheme {
        self.surface_container_low = Some(surface_container_low);
        self
    }

    /// Dart `ColorScheme(surfaceContainer:)`.
    pub fn with_surface_container(mut self, surface_container: Color) -> ColorScheme {
        self.surface_container = Some(surface_container);
        self
    }

    /// Dart `ColorScheme(surfaceContainerHigh:)`.
    pub fn with_surface_container_high(mut self, surface_container_high: Color) -> ColorScheme {
        self.surface_container_high = Some(surface_container_high);
        self
    }

    /// Dart `ColorScheme(surfaceContainerHighest:)`.
    pub fn with_surface_container_highest(
        mut self,
        surface_container_highest: Color,
    ) -> ColorScheme {
        self.surface_container_highest = Some(surface_container_highest);
        self
    }

    /// Dart `ColorScheme(onSurfaceVariant:)`.
    pub fn with_on_surface_variant(mut self, on_surface_variant: Color) -> ColorScheme {
        self.on_surface_variant = Some(on_surface_variant);
        self
    }

    /// Dart `ColorScheme(outline:)`.
    pub fn with_outline(mut self, outline: Color) -> ColorScheme {
        self.outline = Some(outline);
        self
    }

    /// Dart `ColorScheme(outlineVariant:)`.
    pub fn with_outline_variant(mut self, outline_variant: Color) -> ColorScheme {
        self.outline_variant = Some(outline_variant);
        self
    }

    /// Dart `ColorScheme(shadow:)`.
    pub fn with_shadow(mut self, shadow: Color) -> ColorScheme {
        self.shadow = Some(shadow);
        self
    }

    /// Dart `ColorScheme(scrim:)`.
    pub fn with_scrim(mut self, scrim: Color) -> ColorScheme {
        self.scrim = Some(scrim);
        self
    }

    /// Dart `ColorScheme(inverseSurface:)`.
    pub fn with_inverse_surface(mut self, inverse_surface: Color) -> ColorScheme {
        self.inverse_surface = Some(inverse_surface);
        self
    }

    /// Dart `ColorScheme(onInverseSurface:)`.
    pub fn with_on_inverse_surface(mut self, on_inverse_surface: Color) -> ColorScheme {
        self.on_inverse_surface = Some(on_inverse_surface);
        self
    }

    /// Dart `ColorScheme(inversePrimary:)`.
    pub fn with_inverse_primary(mut self, inverse_primary: Color) -> ColorScheme {
        self.inverse_primary = Some(inverse_primary);
        self
    }

    /// Dart `ColorScheme(surfaceTint:)`.
    pub fn with_surface_tint(mut self, surface_tint: Color) -> ColorScheme {
        self.surface_tint = Some(surface_tint);
        self
    }

    #[deprecated(note = "Use surface instead. This feature was deprecated after v3.18.0-0.1.pre.")]
    /// Dart `ColorScheme(background:)`.
    pub fn with_background(mut self, background: Color) -> ColorScheme {
        self.background = Some(background);
        self
    }

    #[deprecated(
        note = "Use onSurface instead. This feature was deprecated after v3.18.0-0.1.pre."
    )]
    /// Dart `ColorScheme(onBackground:)`.
    pub fn with_on_background(mut self, on_background: Color) -> ColorScheme {
        self.on_background = Some(on_background);
        self
    }

    #[deprecated(
        note = "Use surfaceContainerHighest instead. This feature was deprecated after v3.18.0-0.1.pre."
    )]
    /// Dart `ColorScheme(surfaceVariant:)`.
    pub fn with_surface_variant(mut self, surface_variant: Color) -> ColorScheme {
        self.surface_variant = Some(surface_variant);
        self
    }

    /// Generate a `ColorScheme` derived from the given `seedColor`.
    ///
    /// Using the `seedColor` as a starting point, a set of tonal palettes are
    /// constructed. By default, the tonal palettes are based on the Material 3
    /// Color system and provide all of the `ColorScheme` colors. These colors are
    /// designed to work well together and meet contrast requirements for
    /// accessibility.
    ///
    /// If any of the optional color parameters are non-null they will be
    /// used in place of the generated colors for that field in the resulting
    /// color scheme. This allows apps to override specific colors for their
    /// needs.
    ///
    /// Given the nature of the algorithm, the `seedColor` may not wind up as
    /// one of the ColorScheme colors.
    ///
    /// The `dynamicSchemeVariant` parameter creates different types of
    /// `DynamicScheme`s, which are used to generate different styles of `ColorScheme`s.
    /// By default, `dynamicSchemeVariant` is set to `tonalSpot`. A `ColorScheme`
    /// constructed by `dynamicSchemeVariant.tonalSpot` has pastel palettes and
    /// won't be too "colorful" even if the `seedColor` has a high chroma value.
    /// If the resulting color scheme is too dark, consider setting `dynamicSchemeVariant`
    /// to `DynamicSchemeVariant.fidelity`, whose palettes match the seed color.
    ///
    /// The `contrastLevel` parameter indicates the contrast level between color
    /// pairs, such as `primary` and `on_primary`. 0.0 is the default (normal);
    /// -1.0 is the lowest; 1.0 is the highest. From Material Design guideline, the
    /// medium and high contrast correspond to 0.5 and 1.0 respectively.
    ///
    /// <callout-box>
    ///
    /// This sample shows how to use `ColorScheme::from_seed` to create dynamic
    /// color schemes with different `DynamicSchemeVariant`s and different
    /// contrast level.
    ///
    // TODO(framework): Replace the following block with a @dartpad directive
    // when it's supported. https://github.com/dart-lang/dartdoc/issues/4123
    /// {@macro material_ui.dartpad_guide}
    ///
    /// {@example /example/lib/color_scheme/color_scheme.0.dart#body}
    ///
    /// </callout-box>
    ///
    /// See also:
    ///
    ///  * <https://m3.material.io/styles/color/the-color-system/color-roles>, the
    ///    Material 3 Color system specification.
    ///  * <https://pub.dev/packages/material_color_utilities>, the package
    ///    used to generate the tonal palettes needed for the scheme.
    ///
    /// The builder takes `brightness`, `dynamic_scheme_variant` and `contrast_level`; Dart's
    /// colour overrides are the `with_*` setters on the built scheme, since each replaces
    /// only its own colour.
    pub fn from_seed(seed_color: Color) -> ColorSchemeFromSeedBuilder {
        ColorSchemeFromSeedBuilder {
            seed_color,
            brightness: Brightness::Light,
            dynamic_scheme_variant: DynamicSchemeVariant::TonalSpot,
            contrast_level: 0.0,
        }
    }

    /// Create a light ColorScheme based on a purple primary color that matches the
    /// [baseline Material 2 color scheme](https://material.io/design/color/the-color-system.html#color-theme-creation).
    ///
    /// This constructor shouldn't be used to update the Material 3 color scheme.
    ///
    /// For Material 3, use `ColorScheme::from_seed` to create a color scheme
    /// from a single seed color based on the Material 3 color system.
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// This example demonstrates how to create a color scheme similar to `ColorScheme::light`
    /// using the `ColorScheme::from_seed` constructor:
    ///
    /// ```dart
    /// colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xff6200ee)).copyWith(
    ///   primaryContainer: const Color(0xff6200ee),
    ///   onPrimaryContainer: Colors.white,
    ///   secondaryContainer: const Color(0xff03dac6),
    ///   onSecondaryContainer: Colors.black,
    ///   error: const Color(0xffb00020),
    ///   onError: Colors.white,
    /// ),
    /// ```
    ///
    /// </callout-box>
    pub fn light() -> ColorScheme {
        ColorScheme {
            brightness: Brightness::Light,
            primary: Color::new(0xFF6200EE),
            on_primary: Colors::WHITE,
            primary_container: None,
            on_primary_container: None,
            primary_fixed: None,
            primary_fixed_dim: None,
            on_primary_fixed: None,
            on_primary_fixed_variant: None,
            secondary: Color::new(0xFF03DAC6),
            on_secondary: Colors::BLACK,
            secondary_container: None,
            on_secondary_container: None,
            secondary_fixed: None,
            secondary_fixed_dim: None,
            on_secondary_fixed: None,
            on_secondary_fixed_variant: None,
            tertiary: None,
            on_tertiary: None,
            tertiary_container: None,
            on_tertiary_container: None,
            tertiary_fixed: None,
            tertiary_fixed_dim: None,
            on_tertiary_fixed: None,
            on_tertiary_fixed_variant: None,
            error: Color::new(0xFFB00020),
            on_error: Colors::WHITE,
            error_container: None,
            on_error_container: None,
            surface: Colors::WHITE,
            on_surface: Colors::BLACK,
            surface_variant: None,
            surface_dim: None,
            surface_bright: None,
            surface_container_lowest: None,
            surface_container_low: None,
            surface_container: None,
            surface_container_high: None,
            surface_container_highest: None,
            on_surface_variant: None,
            outline: None,
            outline_variant: None,
            shadow: None,
            scrim: None,
            inverse_surface: None,
            on_inverse_surface: None,
            inverse_primary: None,
            surface_tint: None,
            background: Some(Colors::WHITE),
            on_background: Some(Colors::BLACK),
        }
    }

    /// Create the dark color scheme that matches the
    /// [baseline Material 2 color scheme](https://material.io/design/color/dark-theme.html#ui-application).
    ///
    /// This constructor shouldn't be used to update the Material 3 color scheme.
    ///
    /// For Material 3, use `ColorScheme::from_seed` to create a color scheme
    /// from a single seed color based on the Material 3 color system.
    /// Override the `brightness` property of `ColorScheme::from_seed` to create a
    /// dark color scheme.
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// This example demonstrates how to create a color scheme similar to `ColorScheme::dark`
    /// using the `ColorScheme::from_seed` constructor:
    ///
    /// ```dart
    /// colorScheme: ColorScheme.fromSeed(
    ///   seedColor: const Color(0xffbb86fc),
    ///   brightness: Brightness.dark,
    /// ).copyWith(
    ///   primaryContainer: const Color(0xffbb86fc),
    ///   onPrimaryContainer: Colors.black,
    ///   secondaryContainer: const Color(0xff03dac6),
    ///   onSecondaryContainer: Colors.black,
    ///   error: const Color(0xffcf6679),
    ///   onError: Colors.black,
    /// ),
    /// ```
    ///
    /// </callout-box>
    pub fn dark() -> ColorScheme {
        ColorScheme {
            brightness: Brightness::Dark,
            primary: Color::new(0xFFBB86FC),
            on_primary: Colors::BLACK,
            primary_container: None,
            on_primary_container: None,
            primary_fixed: None,
            primary_fixed_dim: None,
            on_primary_fixed: None,
            on_primary_fixed_variant: None,
            secondary: Color::new(0xFF03DAC6),
            on_secondary: Colors::BLACK,
            secondary_container: None,
            on_secondary_container: None,
            secondary_fixed: None,
            secondary_fixed_dim: None,
            on_secondary_fixed: None,
            on_secondary_fixed_variant: None,
            tertiary: None,
            on_tertiary: None,
            tertiary_container: None,
            on_tertiary_container: None,
            tertiary_fixed: None,
            tertiary_fixed_dim: None,
            on_tertiary_fixed: None,
            on_tertiary_fixed_variant: None,
            error: Color::new(0xFFCF6679),
            on_error: Colors::BLACK,
            error_container: None,
            on_error_container: None,
            surface: Color::new(0xFF121212),
            on_surface: Colors::WHITE,
            surface_variant: None,
            surface_dim: None,
            surface_bright: None,
            surface_container_lowest: None,
            surface_container_low: None,
            surface_container: None,
            surface_container_high: None,
            surface_container_highest: None,
            on_surface_variant: None,
            outline: None,
            outline_variant: None,
            shadow: None,
            scrim: None,
            inverse_surface: None,
            on_inverse_surface: None,
            inverse_primary: None,
            surface_tint: None,
            background: Some(Color::new(0xFF121212)),
            on_background: Some(Colors::WHITE),
        }
    }

    /// Create a high contrast ColorScheme based on a purple primary color that
    /// matches the [baseline Material 2 color scheme](https://material.io/design/color/the-color-system.html#color-theme-creation).
    ///
    /// This constructor shouldn't be used to update the Material 3 color scheme.
    ///
    /// For Material 3, use `ColorScheme::from_seed` to create a color scheme
    /// from a single seed color based on the Material 3 color system. To create a
    /// high-contrast color scheme, set `contrastLevel` to 1.0.
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// This example demonstrates how to create a color scheme similar to `ColorScheme::high_contrast_light`
    /// using the `ColorScheme::from_seed` constructor:
    ///
    /// ```dart
    /// colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xff0000ba)).copyWith(
    ///   primaryContainer: const Color(0xff0000ba),
    ///   onPrimaryContainer: Colors.white,
    ///   secondaryContainer: const Color(0xff66fff9),
    ///   onSecondaryContainer: Colors.black,
    ///   error: const Color(0xff790000),
    ///   onError: Colors.white,
    /// ),
    /// ```
    ///
    /// </callout-box>
    pub fn high_contrast_light() -> ColorScheme {
        ColorScheme {
            brightness: Brightness::Light,
            primary: Color::new(0xFF0000BA),
            on_primary: Colors::WHITE,
            primary_container: None,
            on_primary_container: None,
            primary_fixed: None,
            primary_fixed_dim: None,
            on_primary_fixed: None,
            on_primary_fixed_variant: None,
            secondary: Color::new(0xFF66FFF9),
            on_secondary: Colors::BLACK,
            secondary_container: None,
            on_secondary_container: None,
            secondary_fixed: None,
            secondary_fixed_dim: None,
            on_secondary_fixed: None,
            on_secondary_fixed_variant: None,
            tertiary: None,
            on_tertiary: None,
            tertiary_container: None,
            on_tertiary_container: None,
            tertiary_fixed: None,
            tertiary_fixed_dim: None,
            on_tertiary_fixed: None,
            on_tertiary_fixed_variant: None,
            error: Color::new(0xFF790000),
            on_error: Colors::WHITE,
            error_container: None,
            on_error_container: None,
            surface: Colors::WHITE,
            on_surface: Colors::BLACK,
            surface_variant: None,
            surface_dim: None,
            surface_bright: None,
            surface_container_lowest: None,
            surface_container_low: None,
            surface_container: None,
            surface_container_high: None,
            surface_container_highest: None,
            on_surface_variant: None,
            outline: None,
            outline_variant: None,
            shadow: None,
            scrim: None,
            inverse_surface: None,
            on_inverse_surface: None,
            inverse_primary: None,
            surface_tint: None,
            background: Some(Colors::WHITE),
            on_background: Some(Colors::BLACK),
        }
    }

    /// Create a high contrast ColorScheme based on the dark
    /// [baseline Material 2 color scheme](https://material.io/design/color/dark-theme.html#ui-application).
    ///
    /// This constructor shouldn't be used to update the Material 3 color scheme.
    ///
    /// For Material 3, use `ColorScheme::from_seed` to create a color scheme
    /// from a single seed color based on the Material 3 color system.
    /// Override the `brightness` property of `ColorScheme::from_seed` to create a
    /// dark color scheme. To create a high-contrast color scheme, set
    /// `contrastLevel` to 1.0.
    ///
    /// <callout-box>
    ///
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    ///
    /// This example demonstrates how to create a color scheme similar to `ColorScheme::high_contrast_dark`
    /// using the `ColorScheme::from_seed` constructor:
    ///
    /// ```dart
    /// colorScheme: ColorScheme.fromSeed(
    ///   seedColor: const Color(0xffefb7ff),
    ///   brightness: Brightness.dark,
    /// ).copyWith(
    ///   primaryContainer: const Color(0xffefb7ff),
    ///   onPrimaryContainer: Colors.black,
    ///   secondaryContainer: const Color(0xff66fff9),
    ///   onSecondaryContainer: Colors.black,
    ///   error: const Color(0xff9b374d),
    ///   onError: Colors.white,
    /// ),
    /// ```
    ///
    /// </callout-box>
    pub fn high_contrast_dark() -> ColorScheme {
        ColorScheme {
            brightness: Brightness::Dark,
            primary: Color::new(0xFFEFB7FF),
            on_primary: Colors::BLACK,
            primary_container: None,
            on_primary_container: None,
            primary_fixed: None,
            primary_fixed_dim: None,
            on_primary_fixed: None,
            on_primary_fixed_variant: None,
            secondary: Color::new(0xFF66FFF9),
            on_secondary: Colors::BLACK,
            secondary_container: None,
            on_secondary_container: None,
            secondary_fixed: None,
            secondary_fixed_dim: None,
            on_secondary_fixed: None,
            on_secondary_fixed_variant: None,
            tertiary: None,
            on_tertiary: None,
            tertiary_container: None,
            on_tertiary_container: None,
            tertiary_fixed: None,
            tertiary_fixed_dim: None,
            on_tertiary_fixed: None,
            on_tertiary_fixed_variant: None,
            error: Color::new(0xFF9B374D),
            on_error: Colors::BLACK,
            error_container: None,
            on_error_container: None,
            surface: Color::new(0xFF121212),
            on_surface: Colors::WHITE,
            surface_variant: None,
            surface_dim: None,
            surface_bright: None,
            surface_container_lowest: None,
            surface_container_low: None,
            surface_container: None,
            surface_container_high: None,
            surface_container_highest: None,
            on_surface_variant: None,
            outline: None,
            outline_variant: None,
            shadow: None,
            scrim: None,
            inverse_surface: None,
            on_inverse_surface: None,
            inverse_primary: None,
            surface_tint: None,
            background: Some(Color::new(0xFF121212)),
            on_background: Some(Colors::WHITE),
        }
    }

    /// Creates a color scheme from a `MaterialColor` swatch.
    ///
    /// In Material 3, this constructor is ignored by `ThemeData` when creating
    /// its default color scheme. Instead, `ThemeData` uses `ColorScheme::from_seed`
    /// to create its default color scheme. This constructor shouldn't be used
    /// to update the Material 3 color scheme. It will be phased out gradually;
    /// see https://github.com/flutter/flutter/issues/120064 for more details.
    ///
    /// If `ThemeData.useMaterial3` is false, then this constructor is used by
    /// `ThemeData` to create its default color scheme.
    pub fn from_swatch() -> ColorSchemeFromSwatchBuilder {
        ColorSchemeFromSwatchBuilder {
            primary_swatch: Colors::BLUE,
            accent_color: None,
            card_color: None,
            background_color: None,
            error_color: None,
            brightness: Brightness::Light,
        }
    }

    fn brightness_for(color: Color) -> Brightness {
        ThemeData::estimate_brightness_for_color(color)
    }

    /// The [`Brightness`] of this scheme.
    pub fn brightness(&self) -> Brightness {
        self.brightness
    }

    /// The color displayed most frequently across your app’s screens and components.
    pub fn primary(&self) -> Color {
        self.primary
    }

    /// A color that's clearly legible when drawn on [`primary`](Self::primary).
    pub fn on_primary(&self) -> Color {
        self.on_primary
    }

    /// A color used for elements needing less emphasis than `primary`.
    pub fn primary_container(&self) -> Color {
        self.primary_container.unwrap_or(self.primary)
    }

    /// A color that's clearly legible when drawn on `primary_container`.
    ///
    /// To ensure that an app is accessible, a contrast ratio between
    /// `primary_container` and `on_primary_container` of at least 4.5:1
    /// is recommended. See
    /// <https://www.w3.org/TR/UNDERSTANDING-WCAG20/visual-audio-contrast-contrast.html>.
    pub fn on_primary_container(&self) -> Color {
        self.on_primary_container.unwrap_or(self.on_primary)
    }

    /// A substitute for `primary_container` that's the same color for the dark
    /// and light themes.
    pub fn primary_fixed(&self) -> Color {
        self.primary_fixed.unwrap_or(self.primary)
    }

    /// A color used for elements needing more emphasis than `primary_fixed`.
    pub fn primary_fixed_dim(&self) -> Color {
        self.primary_fixed_dim.unwrap_or(self.primary)
    }

    /// A color that is used for text and icons that exist on top of elements having
    /// `primary_fixed` color.
    pub fn on_primary_fixed(&self) -> Color {
        self.on_primary_fixed.unwrap_or(self.on_primary)
    }

    /// A color that provides a lower-emphasis option for text and icons than
    /// `on_primary_fixed`.
    pub fn on_primary_fixed_variant(&self) -> Color {
        self.on_primary_fixed_variant.unwrap_or(self.on_primary)
    }

    /// An accent color used for less prominent components in the UI, such as filter chips,
    /// while expanding the opportunity for color expression.
    pub fn secondary(&self) -> Color {
        self.secondary
    }

    /// A color that's clearly legible when drawn on [`secondary`](Self::secondary).
    pub fn on_secondary(&self) -> Color {
        self.on_secondary
    }

    /// A color used for elements needing less emphasis than `secondary`.
    pub fn secondary_container(&self) -> Color {
        self.secondary_container.unwrap_or(self.secondary)
    }

    /// A color that's clearly legible when drawn on `secondary_container`.
    ///
    /// To ensure that an app is accessible, a contrast ratio between
    /// `secondary_container` and `on_secondary_container` of at least 4.5:1 is
    /// recommended. See
    /// <https://www.w3.org/TR/UNDERSTANDING-WCAG20/visual-audio-contrast-contrast.html>.
    pub fn on_secondary_container(&self) -> Color {
        self.on_secondary_container.unwrap_or(self.on_secondary)
    }

    /// A substitute for `secondary_container` that's the same color for the dark
    /// and light themes.
    pub fn secondary_fixed(&self) -> Color {
        self.secondary_fixed.unwrap_or(self.secondary)
    }

    /// A color used for elements needing more emphasis than `secondary_fixed`.
    pub fn secondary_fixed_dim(&self) -> Color {
        self.secondary_fixed_dim.unwrap_or(self.secondary)
    }

    /// A color that is used for text and icons that exist on top of elements having
    /// `secondary_fixed` color.
    pub fn on_secondary_fixed(&self) -> Color {
        self.on_secondary_fixed.unwrap_or(self.on_secondary)
    }

    /// A color that provides a lower-emphasis option for text and icons than
    /// `on_secondary_fixed`.
    pub fn on_secondary_fixed_variant(&self) -> Color {
        self.on_secondary_fixed_variant.unwrap_or(self.on_secondary)
    }

    /// A color used as a contrasting accent that can balance `primary`
    /// and `secondary` colors or bring heightened attention to an element,
    /// such as an input field.
    pub fn tertiary(&self) -> Color {
        self.tertiary.unwrap_or(self.secondary)
    }

    /// A color that's clearly legible when drawn on `tertiary`.
    ///
    /// To ensure that an app is accessible, a contrast ratio between
    /// `tertiary` and `on_tertiary` of at least 4.5:1 is recommended. See
    /// <https://www.w3.org/TR/UNDERSTANDING-WCAG20/visual-audio-contrast-contrast.html>.
    pub fn on_tertiary(&self) -> Color {
        self.on_tertiary.unwrap_or(self.on_secondary)
    }

    /// A color used for elements needing less emphasis than `tertiary`.
    pub fn tertiary_container(&self) -> Color {
        self.tertiary_container.unwrap_or_else(|| self.tertiary())
    }

    /// A color that's clearly legible when drawn on `tertiary_container`.
    ///
    /// To ensure that an app is accessible, a contrast ratio between
    /// `tertiary_container` and `on_tertiary_container` of at least 4.5:1 is
    /// recommended. See
    /// <https://www.w3.org/TR/UNDERSTANDING-WCAG20/visual-audio-contrast-contrast.html>.
    pub fn on_tertiary_container(&self) -> Color {
        self.on_tertiary_container
            .unwrap_or_else(|| self.on_tertiary())
    }

    /// A substitute for `tertiary_container` that's the same color for dark
    /// and light themes.
    pub fn tertiary_fixed(&self) -> Color {
        self.tertiary_fixed.unwrap_or_else(|| self.tertiary())
    }

    /// A color used for elements needing more emphasis than `tertiary_fixed`.
    pub fn tertiary_fixed_dim(&self) -> Color {
        self.tertiary_fixed_dim.unwrap_or_else(|| self.tertiary())
    }

    /// A color that is used for text and icons that exist on top of elements having
    /// `tertiary_fixed` color.
    pub fn on_tertiary_fixed(&self) -> Color {
        self.on_tertiary_fixed.unwrap_or_else(|| self.on_tertiary())
    }

    /// A color that provides a lower-emphasis option for text and icons than
    /// `on_tertiary_fixed`.
    pub fn on_tertiary_fixed_variant(&self) -> Color {
        self.on_tertiary_fixed_variant
            .unwrap_or_else(|| self.on_tertiary())
    }

    /// The color to use for input validation errors, e.g. for an `InputDecoration`'s error text.
    pub fn error(&self) -> Color {
        self.error
    }

    /// A color that's clearly legible when drawn on [`error`](Self::error).
    pub fn on_error(&self) -> Color {
        self.on_error
    }

    /// A color used for error elements needing less emphasis than `error`.
    pub fn error_container(&self) -> Color {
        self.error_container.unwrap_or(self.error)
    }

    /// A color that's clearly legible when drawn on `error_container`.
    ///
    /// To ensure that an app is accessible, a contrast ratio between
    /// `error_container` and `on_error_container` of at least 4.5:1 is
    /// recommended. See
    /// <https://www.w3.org/TR/UNDERSTANDING-WCAG20/visual-audio-contrast-contrast.html>.
    pub fn on_error_container(&self) -> Color {
        self.on_error_container.unwrap_or(self.on_error)
    }

    /// The background color for widgets like `Scaffold`.
    pub fn surface(&self) -> Color {
        self.surface
    }

    /// A color that's clearly legible when drawn on [`surface`](Self::surface).
    pub fn on_surface(&self) -> Color {
        self.on_surface
    }

    /// A color variant of `surface` that can be used for differentiation against
    /// a component using `surface`.
    #[deprecated(
        note = "Use surfaceContainerHighest instead. This feature was deprecated after v3.18.0-0.1.pre."
    )]
    pub fn surface_variant(&self) -> Color {
        self.surface_variant.unwrap_or(self.surface)
    }

    /// A color that's always darkest in the dark or light theme.
    pub fn surface_dim(&self) -> Color {
        self.surface_dim.unwrap_or(self.surface)
    }

    /// A color that's always the lightest in the dark or light theme.
    pub fn surface_bright(&self) -> Color {
        self.surface_bright.unwrap_or(self.surface)
    }

    /// A surface container color with the lightest tone and the least emphasis
    /// relative to the surface.
    pub fn surface_container_lowest(&self) -> Color {
        self.surface_container_lowest.unwrap_or(self.surface)
    }

    /// A surface container color with a lighter tone that creates less emphasis
    /// than `surface_container` but more emphasis than `surface_container_lowest`.
    pub fn surface_container_low(&self) -> Color {
        self.surface_container_low.unwrap_or(self.surface)
    }

    /// A recommended color role for a distinct area within the surface.
    ///
    /// Surface container color roles are independent of elevation. They replace the old
    /// opacity-based model which applied a tinted overlay on top of
    /// surfaces based on their elevation.
    ///
    /// Surface container colors include `surface_container_lowest`, `surface_container_low`,
    /// `surface_container`, `surface_container_high` and `surface_container_highest`.
    pub fn surface_container(&self) -> Color {
        self.surface_container.unwrap_or(self.surface)
    }

    /// A surface container color with a darker tone. It is used to create more
    /// emphasis than `surface_container` but less emphasis than `surface_container_highest`.
    pub fn surface_container_high(&self) -> Color {
        self.surface_container_high.unwrap_or(self.surface)
    }

    /// A surface container color with the darkest tone. It is used to create the
    /// most emphasis against the surface.
    pub fn surface_container_highest(&self) -> Color {
        self.surface_container_highest.unwrap_or(self.surface)
    }

    /// A color that's clearly legible when drawn on `surface_variant`.
    ///
    /// To ensure that an app is accessible, a contrast ratio between
    /// `surface_variant` and `on_surface_variant` of at least 4.5:1 is
    /// recommended. See
    /// <https://www.w3.org/TR/UNDERSTANDING-WCAG20/visual-audio-contrast-contrast.html>.
    pub fn on_surface_variant(&self) -> Color {
        self.on_surface_variant.unwrap_or(self.on_surface)
    }

    /// A utility color that creates boundaries and emphasis to improve usability.
    #[allow(deprecated)]
    pub fn outline(&self) -> Color {
        self.outline.unwrap_or_else(|| self.on_background())
    }

    /// A utility color that creates boundaries for decorative elements when a
    /// 3:1 contrast isn’t required, such as for dividers or decorative elements.
    #[allow(deprecated)]
    pub fn outline_variant(&self) -> Color {
        self.outline_variant.unwrap_or_else(|| self.on_background())
    }

    /// A color use to paint the drop shadows of elevated components.
    pub fn shadow(&self) -> Color {
        self.shadow.unwrap_or(Color::new(0xFF000000))
    }

    /// A color use to paint the scrim around of modal components.
    pub fn scrim(&self) -> Color {
        self.scrim.unwrap_or(Color::new(0xFF000000))
    }

    /// A surface color used for displaying the reverse of what’s seen in the
    /// surrounding UI, for example in a SnackBar to bring attention to
    /// an alert.
    pub fn inverse_surface(&self) -> Color {
        self.inverse_surface.unwrap_or(self.on_surface)
    }

    /// A color that's clearly legible when drawn on `inverse_surface`.
    ///
    /// To ensure that an app is accessible, a contrast ratio between
    /// `inverse_surface` and `on_inverse_surface` of at least 4.5:1 is
    /// recommended. See
    /// <https://www.w3.org/TR/UNDERSTANDING-WCAG20/visual-audio-contrast-contrast.html>.
    pub fn on_inverse_surface(&self) -> Color {
        self.on_inverse_surface.unwrap_or(self.surface)
    }

    /// An accent color used for displaying a highlight color on `inverse_surface`
    /// backgrounds, like button text in a SnackBar.
    pub fn inverse_primary(&self) -> Color {
        self.inverse_primary.unwrap_or(self.on_primary)
    }

    /// A color used as an overlay on a surface color to indicate a component's
    /// elevation.
    pub fn surface_tint(&self) -> Color {
        self.surface_tint.unwrap_or(self.primary)
    }

    /// A color that typically appears behind scrollable content.
    #[deprecated(note = "Use surface instead. This feature was deprecated after v3.18.0-0.1.pre.")]
    pub fn background(&self) -> Color {
        self.background.unwrap_or(self.surface)
    }

    /// A color that's clearly legible when drawn on `background`.
    ///
    /// To ensure that an app is accessible, a contrast ratio between
    /// `background` and `on_background` of at least 4.5:1 is recommended. See
    /// <https://www.w3.org/TR/UNDERSTANDING-WCAG20/visual-audio-contrast-contrast.html>.
    #[deprecated(
        note = "Use onSurface instead. This feature was deprecated after v3.18.0-0.1.pre."
    )]
    pub fn on_background(&self) -> Color {
        self.on_background.unwrap_or(self.on_surface)
    }

    /// Creates a copy of this color scheme with the given fields
    /// replaced by the non-null parameter values.
    ///
    /// Every colour that falls back to another is filled with what it reads now, as Dart's
    /// `copyWith` passes `this.primaryContainer` on; the `with_*` setters then replace them.
    #[allow(deprecated)]
    pub fn copy_with(&self) -> ColorScheme {
        ColorScheme {
            brightness: self.brightness,
            primary: self.primary,
            on_primary: self.on_primary,
            primary_container: Some(self.primary_container()),
            on_primary_container: Some(self.on_primary_container()),
            primary_fixed: Some(self.primary_fixed()),
            primary_fixed_dim: Some(self.primary_fixed_dim()),
            on_primary_fixed: Some(self.on_primary_fixed()),
            on_primary_fixed_variant: Some(self.on_primary_fixed_variant()),
            secondary: self.secondary,
            on_secondary: self.on_secondary,
            secondary_container: Some(self.secondary_container()),
            on_secondary_container: Some(self.on_secondary_container()),
            secondary_fixed: Some(self.secondary_fixed()),
            secondary_fixed_dim: Some(self.secondary_fixed_dim()),
            on_secondary_fixed: Some(self.on_secondary_fixed()),
            on_secondary_fixed_variant: Some(self.on_secondary_fixed_variant()),
            tertiary: Some(self.tertiary()),
            on_tertiary: Some(self.on_tertiary()),
            tertiary_container: Some(self.tertiary_container()),
            on_tertiary_container: Some(self.on_tertiary_container()),
            tertiary_fixed: Some(self.tertiary_fixed()),
            tertiary_fixed_dim: Some(self.tertiary_fixed_dim()),
            on_tertiary_fixed: Some(self.on_tertiary_fixed()),
            on_tertiary_fixed_variant: Some(self.on_tertiary_fixed_variant()),
            error: self.error,
            on_error: self.on_error,
            error_container: Some(self.error_container()),
            on_error_container: Some(self.on_error_container()),
            surface: self.surface,
            on_surface: self.on_surface,
            surface_variant: Some(self.surface_variant()),
            surface_dim: Some(self.surface_dim()),
            surface_bright: Some(self.surface_bright()),
            surface_container_lowest: Some(self.surface_container_lowest()),
            surface_container_low: Some(self.surface_container_low()),
            surface_container: Some(self.surface_container()),
            surface_container_high: Some(self.surface_container_high()),
            surface_container_highest: Some(self.surface_container_highest()),
            on_surface_variant: Some(self.on_surface_variant()),
            outline: Some(self.outline()),
            outline_variant: Some(self.outline_variant()),
            shadow: Some(self.shadow()),
            scrim: Some(self.scrim()),
            inverse_surface: Some(self.inverse_surface()),
            on_inverse_surface: Some(self.on_inverse_surface()),
            inverse_primary: Some(self.inverse_primary()),
            surface_tint: Some(self.surface_tint()),
            background: Some(self.background()),
            on_background: Some(self.on_background()),
        }
    }

    /// Linearly interpolate between two `ColorScheme` objects.
    ///
    /// {@macro dart.ui.shadow.lerp}
    #[allow(deprecated)]
    pub fn lerp(a: &ColorScheme, b: &ColorScheme, t: f64) -> ColorScheme {
        if std::ptr::eq(a, b) {
            return a.clone();
        }
        ColorScheme {
            brightness: if t < 0.5 { a.brightness } else { b.brightness },
            primary: Color::lerp(Some(a.primary), Some(b.primary), t).expect("both sides"),
            on_primary: Color::lerp(Some(a.on_primary), Some(b.on_primary), t).expect("both sides"),
            primary_container: Color::lerp(
                Some(a.primary_container()),
                Some(b.primary_container()),
                t,
            ),
            on_primary_container: Color::lerp(
                Some(a.on_primary_container()),
                Some(b.on_primary_container()),
                t,
            ),
            primary_fixed: Color::lerp(Some(a.primary_fixed()), Some(b.primary_fixed()), t),
            primary_fixed_dim: Color::lerp(
                Some(a.primary_fixed_dim()),
                Some(b.primary_fixed_dim()),
                t,
            ),
            on_primary_fixed: Color::lerp(
                Some(a.on_primary_fixed()),
                Some(b.on_primary_fixed()),
                t,
            ),
            on_primary_fixed_variant: Color::lerp(
                Some(a.on_primary_fixed_variant()),
                Some(b.on_primary_fixed_variant()),
                t,
            ),
            secondary: Color::lerp(Some(a.secondary), Some(b.secondary), t).expect("both sides"),
            on_secondary: Color::lerp(Some(a.on_secondary), Some(b.on_secondary), t)
                .expect("both sides"),
            secondary_container: Color::lerp(
                Some(a.secondary_container()),
                Some(b.secondary_container()),
                t,
            ),
            on_secondary_container: Color::lerp(
                Some(a.on_secondary_container()),
                Some(b.on_secondary_container()),
                t,
            ),
            secondary_fixed: Color::lerp(Some(a.secondary_fixed()), Some(b.secondary_fixed()), t),
            secondary_fixed_dim: Color::lerp(
                Some(a.secondary_fixed_dim()),
                Some(b.secondary_fixed_dim()),
                t,
            ),
            on_secondary_fixed: Color::lerp(
                Some(a.on_secondary_fixed()),
                Some(b.on_secondary_fixed()),
                t,
            ),
            on_secondary_fixed_variant: Color::lerp(
                Some(a.on_secondary_fixed_variant()),
                Some(b.on_secondary_fixed_variant()),
                t,
            ),
            tertiary: Color::lerp(Some(a.tertiary()), Some(b.tertiary()), t),
            on_tertiary: Color::lerp(Some(a.on_tertiary()), Some(b.on_tertiary()), t),
            tertiary_container: Color::lerp(
                Some(a.tertiary_container()),
                Some(b.tertiary_container()),
                t,
            ),
            on_tertiary_container: Color::lerp(
                Some(a.on_tertiary_container()),
                Some(b.on_tertiary_container()),
                t,
            ),
            tertiary_fixed: Color::lerp(Some(a.tertiary_fixed()), Some(b.tertiary_fixed()), t),
            tertiary_fixed_dim: Color::lerp(
                Some(a.tertiary_fixed_dim()),
                Some(b.tertiary_fixed_dim()),
                t,
            ),
            on_tertiary_fixed: Color::lerp(
                Some(a.on_tertiary_fixed()),
                Some(b.on_tertiary_fixed()),
                t,
            ),
            on_tertiary_fixed_variant: Color::lerp(
                Some(a.on_tertiary_fixed_variant()),
                Some(b.on_tertiary_fixed_variant()),
                t,
            ),
            error: Color::lerp(Some(a.error), Some(b.error), t).expect("both sides"),
            on_error: Color::lerp(Some(a.on_error), Some(b.on_error), t).expect("both sides"),
            error_container: Color::lerp(Some(a.error_container()), Some(b.error_container()), t),
            on_error_container: Color::lerp(
                Some(a.on_error_container()),
                Some(b.on_error_container()),
                t,
            ),
            surface: Color::lerp(Some(a.surface), Some(b.surface), t).expect("both sides"),
            on_surface: Color::lerp(Some(a.on_surface), Some(b.on_surface), t).expect("both sides"),
            surface_variant: Color::lerp(Some(a.surface_variant()), Some(b.surface_variant()), t),
            surface_dim: Color::lerp(Some(a.surface_dim()), Some(b.surface_dim()), t),
            surface_bright: Color::lerp(Some(a.surface_bright()), Some(b.surface_bright()), t),
            surface_container_lowest: Color::lerp(
                Some(a.surface_container_lowest()),
                Some(b.surface_container_lowest()),
                t,
            ),
            surface_container_low: Color::lerp(
                Some(a.surface_container_low()),
                Some(b.surface_container_low()),
                t,
            ),
            surface_container: Color::lerp(
                Some(a.surface_container()),
                Some(b.surface_container()),
                t,
            ),
            surface_container_high: Color::lerp(
                Some(a.surface_container_high()),
                Some(b.surface_container_high()),
                t,
            ),
            surface_container_highest: Color::lerp(
                Some(a.surface_container_highest()),
                Some(b.surface_container_highest()),
                t,
            ),
            on_surface_variant: Color::lerp(
                Some(a.on_surface_variant()),
                Some(b.on_surface_variant()),
                t,
            ),
            outline: Color::lerp(Some(a.outline()), Some(b.outline()), t),
            outline_variant: Color::lerp(Some(a.outline_variant()), Some(b.outline_variant()), t),
            shadow: Color::lerp(Some(a.shadow()), Some(b.shadow()), t),
            scrim: Color::lerp(Some(a.scrim()), Some(b.scrim()), t),
            inverse_surface: Color::lerp(Some(a.inverse_surface()), Some(b.inverse_surface()), t),
            on_inverse_surface: Color::lerp(
                Some(a.on_inverse_surface()),
                Some(b.on_inverse_surface()),
                t,
            ),
            inverse_primary: Color::lerp(Some(a.inverse_primary()), Some(b.inverse_primary()), t),
            surface_tint: Color::lerp(Some(a.surface_tint()), Some(b.surface_tint()), t),
            background: Color::lerp(Some(a.background()), Some(b.background()), t),
            on_background: Color::lerp(Some(a.on_background()), Some(b.on_background()), t),
        }
    }

    fn build_dynamic_scheme(
        brightness: Brightness,
        seed_color: Color,
        scheme_variant: DynamicSchemeVariant,
        contrast_level: f64,
    ) -> DynamicScheme {
        debug_assert!(
            contrast_level + PRECISION_ERROR_TOLERANCE >= -1.0
                && contrast_level - PRECISION_ERROR_TOLERANCE <= 1.0,
            "contrastLevel must be between -1.0 and 1.0 inclusive."
        );
        let is_dark = brightness == Brightness::Dark;
        let source_color = Hct::from_int(seed_color.to_argb32());
        match scheme_variant {
            DynamicSchemeVariant::TonalSpot => {
                SchemeTonalSpot::new(source_color, is_dark, contrast_level)
            }
            DynamicSchemeVariant::Fidelity => {
                SchemeFidelity::new(source_color, is_dark, contrast_level)
            }
            DynamicSchemeVariant::Content => {
                SchemeContent::new(source_color, is_dark, contrast_level)
            }
            DynamicSchemeVariant::Monochrome => {
                SchemeMonochrome::new(source_color, is_dark, contrast_level)
            }
            DynamicSchemeVariant::Neutral => {
                SchemeNeutral::new(source_color, is_dark, contrast_level)
            }
            DynamicSchemeVariant::Vibrant => {
                SchemeVibrant::new(source_color, is_dark, contrast_level)
            }
            DynamicSchemeVariant::Expressive => {
                SchemeExpressive::new(source_color, is_dark, contrast_level)
            }
            DynamicSchemeVariant::Rainbow => {
                SchemeRainbow::new(source_color, is_dark, contrast_level)
            }
            DynamicSchemeVariant::FruitSalad => {
                SchemeFruitSalad::new(source_color, is_dark, contrast_level)
            }
        }
    }
    /// The [`ThemeData::color_scheme`] of the ambient `Theme`.
    ///
    /// Equivalent to `Theme::of(app, context).color_scheme`.
    pub fn of(app: &mut App, context: BuildContext) -> ColorScheme {
        Theme::of(app, context).color_scheme.clone()
    }
}

impl PartialEq for ColorScheme {
    #[allow(deprecated)]
    fn eq(&self, other: &ColorScheme) -> bool {
        other.brightness == self.brightness
            && other.primary == self.primary
            && other.on_primary == self.on_primary
            && other.primary_container() == self.primary_container()
            && other.on_primary_container() == self.on_primary_container()
            && other.primary_fixed() == self.primary_fixed()
            && other.primary_fixed_dim() == self.primary_fixed_dim()
            && other.on_primary_fixed() == self.on_primary_fixed()
            && other.on_primary_fixed_variant() == self.on_primary_fixed_variant()
            && other.secondary == self.secondary
            && other.on_secondary == self.on_secondary
            && other.secondary_container() == self.secondary_container()
            && other.on_secondary_container() == self.on_secondary_container()
            && other.secondary_fixed() == self.secondary_fixed()
            && other.secondary_fixed_dim() == self.secondary_fixed_dim()
            && other.on_secondary_fixed() == self.on_secondary_fixed()
            && other.on_secondary_fixed_variant() == self.on_secondary_fixed_variant()
            && other.tertiary() == self.tertiary()
            && other.on_tertiary() == self.on_tertiary()
            && other.tertiary_container() == self.tertiary_container()
            && other.on_tertiary_container() == self.on_tertiary_container()
            && other.tertiary_fixed() == self.tertiary_fixed()
            && other.tertiary_fixed_dim() == self.tertiary_fixed_dim()
            && other.on_tertiary_fixed() == self.on_tertiary_fixed()
            && other.on_tertiary_fixed_variant() == self.on_tertiary_fixed_variant()
            && other.error == self.error
            && other.on_error == self.on_error
            && other.error_container() == self.error_container()
            && other.on_error_container() == self.on_error_container()
            && other.surface == self.surface
            && other.on_surface == self.on_surface
            && other.surface_dim() == self.surface_dim()
            && other.surface_bright() == self.surface_bright()
            && other.surface_container_lowest() == self.surface_container_lowest()
            && other.surface_container_low() == self.surface_container_low()
            && other.surface_container() == self.surface_container()
            && other.surface_container_high() == self.surface_container_high()
            && other.surface_container_highest() == self.surface_container_highest()
            && other.on_surface_variant() == self.on_surface_variant()
            && other.outline() == self.outline()
            && other.outline_variant() == self.outline_variant()
            && other.shadow() == self.shadow()
            && other.scrim() == self.scrim()
            && other.inverse_surface() == self.inverse_surface()
            && other.on_inverse_surface() == self.on_inverse_surface()
            && other.inverse_primary() == self.inverse_primary()
            && other.surface_tint() == self.surface_tint()
            && other.background() == self.background()
            && other.on_background() == self.on_background()
            && other.surface_variant() == self.surface_variant()
    }
}

/// Dart's `ColorScheme.fromSeed`, collecting what decides the generated colours.
#[derive(Clone, Debug)]
pub struct ColorSchemeFromSeedBuilder {
    seed_color: Color,
    brightness: Brightness,
    dynamic_scheme_variant: DynamicSchemeVariant,
    contrast_level: f64,
}

impl ColorSchemeFromSeedBuilder {
    /// Dart `ColorScheme.fromSeed(brightness:)`.
    pub fn brightness(mut self, brightness: Brightness) -> ColorSchemeFromSeedBuilder {
        self.brightness = brightness;
        self
    }

    /// Dart `ColorScheme.fromSeed(dynamicSchemeVariant:)`.
    pub fn dynamic_scheme_variant(
        mut self,
        dynamic_scheme_variant: DynamicSchemeVariant,
    ) -> ColorSchemeFromSeedBuilder {
        self.dynamic_scheme_variant = dynamic_scheme_variant;
        self
    }

    /// Dart `ColorScheme.fromSeed(contrastLevel:)`.
    pub fn contrast_level(mut self, contrast_level: f64) -> ColorSchemeFromSeedBuilder {
        self.contrast_level = contrast_level;
        self
    }

    /// The scheme generated from the seed color.
    pub fn build(self) -> ColorScheme {
        let scheme = ColorScheme::build_dynamic_scheme(
            self.brightness,
            self.seed_color,
            self.dynamic_scheme_variant,
            self.contrast_level,
        );
        let color = |dynamic_color: Rc<DynamicColor>| Color::new(dynamic_color.get_argb(&scheme));

        ColorScheme {
            brightness: self.brightness,
            primary: color(MaterialDynamicColors::primary()),
            on_primary: color(MaterialDynamicColors::on_primary()),
            primary_container: Some(color(MaterialDynamicColors::primary_container())),
            on_primary_container: Some(color(MaterialDynamicColors::on_primary_container())),
            primary_fixed: Some(color(MaterialDynamicColors::primary_fixed())),
            primary_fixed_dim: Some(color(MaterialDynamicColors::primary_fixed_dim())),
            on_primary_fixed: Some(color(MaterialDynamicColors::on_primary_fixed())),
            on_primary_fixed_variant: Some(
                color(MaterialDynamicColors::on_primary_fixed_variant()),
            ),
            secondary: color(MaterialDynamicColors::secondary()),
            on_secondary: color(MaterialDynamicColors::on_secondary()),
            secondary_container: Some(color(MaterialDynamicColors::secondary_container())),
            on_secondary_container: Some(color(MaterialDynamicColors::on_secondary_container())),
            secondary_fixed: Some(color(MaterialDynamicColors::secondary_fixed())),
            secondary_fixed_dim: Some(color(MaterialDynamicColors::secondary_fixed_dim())),
            on_secondary_fixed: Some(color(MaterialDynamicColors::on_secondary_fixed())),
            on_secondary_fixed_variant: Some(color(
                MaterialDynamicColors::on_secondary_fixed_variant(),
            )),
            tertiary: Some(color(MaterialDynamicColors::tertiary())),
            on_tertiary: Some(color(MaterialDynamicColors::on_tertiary())),
            tertiary_container: Some(color(MaterialDynamicColors::tertiary_container())),
            on_tertiary_container: Some(color(MaterialDynamicColors::on_tertiary_container())),
            tertiary_fixed: Some(color(MaterialDynamicColors::tertiary_fixed())),
            tertiary_fixed_dim: Some(color(MaterialDynamicColors::tertiary_fixed_dim())),
            on_tertiary_fixed: Some(color(MaterialDynamicColors::on_tertiary_fixed())),
            on_tertiary_fixed_variant: Some(color(
                MaterialDynamicColors::on_tertiary_fixed_variant(),
            )),
            error: color(MaterialDynamicColors::error()),
            on_error: color(MaterialDynamicColors::on_error()),
            error_container: Some(color(MaterialDynamicColors::error_container())),
            on_error_container: Some(color(MaterialDynamicColors::on_error_container())),
            surface: color(MaterialDynamicColors::surface()),
            on_surface: color(MaterialDynamicColors::on_surface()),
            surface_variant: Some(color(MaterialDynamicColors::surface_variant())),
            surface_dim: Some(color(MaterialDynamicColors::surface_dim())),
            surface_bright: Some(color(MaterialDynamicColors::surface_bright())),
            surface_container_lowest: Some(
                color(MaterialDynamicColors::surface_container_lowest()),
            ),
            surface_container_low: Some(color(MaterialDynamicColors::surface_container_low())),
            surface_container: Some(color(MaterialDynamicColors::surface_container())),
            surface_container_high: Some(color(MaterialDynamicColors::surface_container_high())),
            surface_container_highest: Some(color(
                MaterialDynamicColors::surface_container_highest(),
            )),
            on_surface_variant: Some(color(MaterialDynamicColors::on_surface_variant())),
            outline: Some(color(MaterialDynamicColors::outline())),
            outline_variant: Some(color(MaterialDynamicColors::outline_variant())),
            shadow: Some(color(MaterialDynamicColors::shadow())),
            scrim: Some(color(MaterialDynamicColors::scrim())),
            inverse_surface: Some(color(MaterialDynamicColors::inverse_surface())),
            on_inverse_surface: Some(color(MaterialDynamicColors::inverse_on_surface())),
            inverse_primary: Some(color(MaterialDynamicColors::inverse_primary())),
            surface_tint: Some(color(MaterialDynamicColors::primary())),
            background: Some(color(MaterialDynamicColors::background())),
            on_background: Some(color(MaterialDynamicColors::on_background())),
        }
    }
}

/// Dart's `ColorScheme.fromSwatch`, collecting its arguments.
#[derive(Clone, Debug)]
pub struct ColorSchemeFromSwatchBuilder {
    primary_swatch: MaterialColor,
    accent_color: Option<Color>,
    card_color: Option<Color>,
    background_color: Option<Color>,
    error_color: Option<Color>,
    brightness: Brightness,
}

impl ColorSchemeFromSwatchBuilder {
    /// Dart `ColorScheme.fromSwatch(primarySwatch:)`.
    pub fn primary_swatch(mut self, primary_swatch: MaterialColor) -> ColorSchemeFromSwatchBuilder {
        self.primary_swatch = primary_swatch;
        self
    }

    /// Dart `ColorScheme.fromSwatch(accentColor:)`.
    pub fn accent_color(mut self, accent_color: Color) -> ColorSchemeFromSwatchBuilder {
        self.accent_color = Some(accent_color);
        self
    }

    /// Dart `ColorScheme.fromSwatch(cardColor:)`.
    pub fn card_color(mut self, card_color: Color) -> ColorSchemeFromSwatchBuilder {
        self.card_color = Some(card_color);
        self
    }

    /// Dart `ColorScheme.fromSwatch(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> ColorSchemeFromSwatchBuilder {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `ColorScheme.fromSwatch(errorColor:)`.
    pub fn error_color(mut self, error_color: Color) -> ColorSchemeFromSwatchBuilder {
        self.error_color = Some(error_color);
        self
    }

    /// Dart `ColorScheme.fromSwatch(brightness:)`.
    pub fn brightness(mut self, brightness: Brightness) -> ColorSchemeFromSwatchBuilder {
        self.brightness = brightness;
        self
    }

    /// The scheme made from the swatch.
    #[allow(deprecated)]
    pub fn build(self) -> ColorScheme {
        let is_dark = self.brightness == Brightness::Dark;
        let primary_is_dark =
            ColorScheme::brightness_for(self.primary_swatch.primary()) == Brightness::Dark;
        let secondary = self.accent_color.unwrap_or_else(|| {
            if is_dark {
                Colors::TEAL_ACCENT.get(&200).unwrap()
            } else {
                self.primary_swatch.primary()
            }
        });
        let secondary_is_dark = ColorScheme::brightness_for(secondary) == Brightness::Dark;

        ColorScheme::new(
            self.brightness,
            self.primary_swatch.primary(),
            if primary_is_dark {
                Colors::WHITE
            } else {
                Colors::BLACK
            },
            secondary,
            if secondary_is_dark {
                Colors::WHITE
            } else {
                Colors::BLACK
            },
            self.error_color
                .unwrap_or_else(|| Colors::RED.get(&700).unwrap()),
            if is_dark {
                Colors::BLACK
            } else {
                Colors::WHITE
            },
            self.card_color.unwrap_or_else(|| {
                if is_dark {
                    Colors::GREY.get(&800).unwrap()
                } else {
                    Colors::WHITE
                }
            }),
            if is_dark {
                Colors::WHITE
            } else {
                Colors::BLACK
            },
        )
        // DEPRECATED (newest deprecations at the bottom)
        .with_background(self.background_color.unwrap_or_else(|| {
            if is_dark {
                Colors::GREY.get(&700).unwrap()
            } else {
                self.primary_swatch.get(&200).unwrap()
            }
        }))
        .with_on_background(if primary_is_dark {
            Colors::WHITE
        } else {
            Colors::BLACK
        })
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use std::cell::RefCell;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use waft_foundation::Listener;
    use waft_widgets::{Builder, Container, IntoWidget, SizedBox};

    use waft_widgets::testing::{WidgetTester, find, test_widgets};

    use super::*;
    use crate::filled_button::FilledButton;
    use crate::material::Material;
    use crate::test_support::material_app;

    #[test]
    fn color_scheme_lerp_special_cases() {
        let scheme = ColorScheme::light();
        assert!(ColorScheme::lerp(&scheme, &scheme, 0.5) == scheme);
    }

    #[test]
    fn light_scheme_matches_the_spec() {
        let scheme = ColorScheme::light();
        // Colors should match the Material Design baseline default theme:
        // https://material.io/design/color/dark-theme.html#ui-application
        // with the new Material 3 colors defaulting to values from the M2
        // baseline.
        assert_eq!(scheme.brightness, Brightness::Light);
        assert_eq!(scheme.primary, Color::new(0xFF6200EE));
        assert_eq!(scheme.on_primary, Color::new(0xFFFFFFFF));
        assert_eq!(scheme.primary_container(), scheme.primary);
        assert_eq!(scheme.on_primary_container(), scheme.on_primary);
        assert_eq!(scheme.primary_fixed(), scheme.primary);
        assert_eq!(scheme.primary_fixed_dim(), scheme.primary);
        assert_eq!(scheme.on_primary_fixed(), scheme.on_primary);
        assert_eq!(scheme.on_primary_fixed_variant(), scheme.on_primary);
        assert_eq!(scheme.secondary, Color::new(0xFF03DAC6));
        assert_eq!(scheme.on_secondary, Color::new(0xFF000000));
        assert_eq!(scheme.secondary_container(), scheme.secondary);
        assert_eq!(scheme.on_secondary_container(), scheme.on_secondary);
        assert_eq!(scheme.secondary_fixed(), scheme.secondary);
        assert_eq!(scheme.secondary_fixed_dim(), scheme.secondary);
        assert_eq!(scheme.on_secondary_fixed(), scheme.on_secondary);
        assert_eq!(scheme.on_secondary_fixed_variant(), scheme.on_secondary);
        assert_eq!(scheme.tertiary(), scheme.secondary);
        assert_eq!(scheme.on_tertiary(), scheme.on_secondary);
        assert_eq!(scheme.tertiary_container(), scheme.tertiary());
        assert_eq!(scheme.on_tertiary_container(), scheme.on_tertiary());
        assert_eq!(scheme.tertiary_fixed(), scheme.tertiary());
        assert_eq!(scheme.tertiary_fixed_dim(), scheme.tertiary());
        assert_eq!(scheme.on_tertiary_fixed(), scheme.on_tertiary());
        assert_eq!(scheme.on_tertiary_fixed_variant(), scheme.on_tertiary());
        assert_eq!(scheme.error, Color::new(0xFFB00020));
        assert_eq!(scheme.on_error, Color::new(0xFFFFFFFF));
        assert_eq!(scheme.error_container(), scheme.error);
        assert_eq!(scheme.on_error_container(), scheme.on_error);
        assert_eq!(scheme.background(), Color::new(0xFFFFFFFF));
        assert_eq!(scheme.on_background(), Color::new(0xFF000000));
        assert_eq!(scheme.surface, Color::new(0xFFFFFFFF));
        assert_eq!(scheme.surface_bright(), scheme.surface);
        assert_eq!(scheme.surface_dim(), scheme.surface);
        assert_eq!(scheme.surface_container_lowest(), scheme.surface);
        assert_eq!(scheme.surface_container_low(), scheme.surface);
        assert_eq!(scheme.surface_container(), scheme.surface);
        assert_eq!(scheme.surface_container_high(), scheme.surface);
        assert_eq!(scheme.surface_container_highest(), scheme.surface);
        assert_eq!(scheme.on_surface, Color::new(0xFF000000));
        assert_eq!(scheme.surface_variant(), scheme.surface);
        assert_eq!(scheme.on_surface_variant(), scheme.on_surface);
        assert_eq!(scheme.outline(), scheme.on_background());
        assert_eq!(scheme.outline_variant(), scheme.on_background());
        assert_eq!(scheme.shadow(), Color::new(0xFF000000));
        assert_eq!(scheme.scrim(), Color::new(0xFF000000));
        assert_eq!(scheme.inverse_surface(), scheme.on_surface);
        assert_eq!(scheme.on_inverse_surface(), scheme.surface);
        assert_eq!(scheme.inverse_primary(), scheme.on_primary);
        assert_eq!(scheme.surface_tint(), scheme.primary);
    }

    #[test]
    fn dark_scheme_matches_the_spec() {
        let scheme = ColorScheme::dark();
        // Colors should match the Material Design baseline dark theme:
        // https://material.io/design/color/dark-theme.html#ui-application
        // with the new Material 3 colors defaulting to values from the M2
        // baseline.
        assert_eq!(scheme.brightness, Brightness::Dark);
        assert_eq!(scheme.primary, Color::new(0xFFBB86FC));
        assert_eq!(scheme.on_primary, Color::new(0xFF000000));
        assert_eq!(scheme.primary_container(), scheme.primary);
        assert_eq!(scheme.on_primary_container(), scheme.on_primary);
        assert_eq!(scheme.primary_fixed(), scheme.primary);
        assert_eq!(scheme.primary_fixed_dim(), scheme.primary);
        assert_eq!(scheme.on_primary_fixed(), scheme.on_primary);
        assert_eq!(scheme.on_primary_fixed_variant(), scheme.on_primary);
        assert_eq!(scheme.secondary, Color::new(0xFF03DAC6));
        assert_eq!(scheme.on_secondary, Color::new(0xFF000000));
        assert_eq!(scheme.secondary_container(), scheme.secondary);
        assert_eq!(scheme.on_secondary_container(), scheme.on_secondary);
        assert_eq!(scheme.secondary_fixed(), scheme.secondary);
        assert_eq!(scheme.secondary_fixed_dim(), scheme.secondary);
        assert_eq!(scheme.on_secondary_fixed(), scheme.on_secondary);
        assert_eq!(scheme.on_secondary_fixed_variant(), scheme.on_secondary);
        assert_eq!(scheme.tertiary(), scheme.secondary);
        assert_eq!(scheme.on_tertiary(), scheme.on_secondary);
        assert_eq!(scheme.tertiary_container(), scheme.tertiary());
        assert_eq!(scheme.on_tertiary_container(), scheme.on_tertiary());
        assert_eq!(scheme.tertiary_fixed(), scheme.tertiary());
        assert_eq!(scheme.tertiary_fixed_dim(), scheme.tertiary());
        assert_eq!(scheme.on_tertiary_fixed(), scheme.on_tertiary());
        assert_eq!(scheme.on_tertiary_fixed_variant(), scheme.on_tertiary());
        assert_eq!(scheme.error, Color::new(0xFFCF6679));
        assert_eq!(scheme.on_error, Color::new(0xFF000000));
        assert_eq!(scheme.error_container(), scheme.error);
        assert_eq!(scheme.on_error_container(), scheme.on_error);
        assert_eq!(scheme.background(), Color::new(0xFF121212));
        assert_eq!(scheme.on_background(), Color::new(0xFFFFFFFF));
        assert_eq!(scheme.surface, Color::new(0xFF121212));
        assert_eq!(scheme.surface_bright(), scheme.surface);
        assert_eq!(scheme.surface_dim(), scheme.surface);
        assert_eq!(scheme.surface_container_lowest(), scheme.surface);
        assert_eq!(scheme.surface_container_low(), scheme.surface);
        assert_eq!(scheme.surface_container(), scheme.surface);
        assert_eq!(scheme.surface_container_high(), scheme.surface);
        assert_eq!(scheme.surface_container_highest(), scheme.surface);
        assert_eq!(scheme.on_surface, Color::new(0xFFFFFFFF));
        assert_eq!(scheme.surface_variant(), scheme.surface);
        assert_eq!(scheme.on_surface_variant(), scheme.on_surface);
        assert_eq!(scheme.outline(), scheme.on_background());
        assert_eq!(scheme.outline_variant(), scheme.on_background());
        assert_eq!(scheme.shadow(), Color::new(0xFF000000));
        assert_eq!(scheme.scrim(), Color::new(0xFF000000));
        assert_eq!(scheme.inverse_surface(), scheme.on_surface);
        assert_eq!(scheme.on_inverse_surface(), scheme.surface);
        assert_eq!(scheme.inverse_primary(), scheme.on_primary);
        assert_eq!(scheme.surface_tint(), scheme.primary);
    }

    #[test]
    fn high_contrast_light_scheme_matches_the_spec() {
        let scheme = ColorScheme::high_contrast_light();
        // Colors are based off of the Material Design baseline default theme:
        // https://material.io/design/color/dark-theme.html#ui-application
        // with the new Material 3 colors defaulting to values from the M2
        // baseline.
        assert_eq!(scheme.brightness, Brightness::Light);
        assert_eq!(scheme.primary, Color::new(0xFF0000BA));
        assert_eq!(scheme.on_primary, Color::new(0xFFFFFFFF));
        assert_eq!(scheme.primary_container(), scheme.primary);
        assert_eq!(scheme.on_primary_container(), scheme.on_primary);
        assert_eq!(scheme.primary_fixed(), scheme.primary);
        assert_eq!(scheme.primary_fixed_dim(), scheme.primary);
        assert_eq!(scheme.on_primary_fixed(), scheme.on_primary);
        assert_eq!(scheme.on_primary_fixed_variant(), scheme.on_primary);
        assert_eq!(scheme.secondary, Color::new(0xFF66FFF9));
        assert_eq!(scheme.on_secondary, Color::new(0xFF000000));
        assert_eq!(scheme.secondary_container(), scheme.secondary);
        assert_eq!(scheme.on_secondary_container(), scheme.on_secondary);
        assert_eq!(scheme.secondary_fixed(), scheme.secondary);
        assert_eq!(scheme.secondary_fixed_dim(), scheme.secondary);
        assert_eq!(scheme.on_secondary_fixed(), scheme.on_secondary);
        assert_eq!(scheme.on_secondary_fixed_variant(), scheme.on_secondary);
        assert_eq!(scheme.tertiary(), scheme.secondary);
        assert_eq!(scheme.on_tertiary(), scheme.on_secondary);
        assert_eq!(scheme.tertiary_container(), scheme.tertiary());
        assert_eq!(scheme.on_tertiary_container(), scheme.on_tertiary());
        assert_eq!(scheme.tertiary_fixed(), scheme.tertiary());
        assert_eq!(scheme.tertiary_fixed_dim(), scheme.tertiary());
        assert_eq!(scheme.on_tertiary_fixed(), scheme.on_tertiary());
        assert_eq!(scheme.on_tertiary_fixed_variant(), scheme.on_tertiary());
        assert_eq!(scheme.error, Color::new(0xFF790000));
        assert_eq!(scheme.on_error, Color::new(0xFFFFFFFF));
        assert_eq!(scheme.error_container(), scheme.error);
        assert_eq!(scheme.on_error_container(), scheme.on_error);
        assert_eq!(scheme.background(), Color::new(0xFFFFFFFF));
        assert_eq!(scheme.on_background(), Color::new(0xFF000000));
        assert_eq!(scheme.surface, Color::new(0xFFFFFFFF));
        assert_eq!(scheme.surface_bright(), scheme.surface);
        assert_eq!(scheme.surface_dim(), scheme.surface);
        assert_eq!(scheme.surface_container_lowest(), scheme.surface);
        assert_eq!(scheme.surface_container_low(), scheme.surface);
        assert_eq!(scheme.surface_container(), scheme.surface);
        assert_eq!(scheme.surface_container_high(), scheme.surface);
        assert_eq!(scheme.surface_container_highest(), scheme.surface);
        assert_eq!(scheme.on_surface, Color::new(0xFF000000));
        assert_eq!(scheme.surface_variant(), scheme.surface);
        assert_eq!(scheme.on_surface_variant(), scheme.on_surface);
        assert_eq!(scheme.outline(), scheme.on_background());
        assert_eq!(scheme.outline_variant(), scheme.on_background());
        assert_eq!(scheme.shadow(), Color::new(0xFF000000));
        assert_eq!(scheme.scrim(), Color::new(0xFF000000));
        assert_eq!(scheme.inverse_surface(), scheme.on_surface);
        assert_eq!(scheme.on_inverse_surface(), scheme.surface);
        assert_eq!(scheme.inverse_primary(), scheme.on_primary);
        assert_eq!(scheme.surface_tint(), scheme.primary);
    }

    #[test]
    fn high_contrast_dark_scheme_matches_the_spec() {
        let scheme = ColorScheme::high_contrast_dark();
        // Colors are based off of the Material Design baseline dark theme:
        // https://material.io/design/color/dark-theme.html#ui-application
        // with the new Material 3 colors defaulting to values from the M2
        // baseline.
        assert_eq!(scheme.brightness, Brightness::Dark);
        assert_eq!(scheme.primary, Color::new(0xFFEFB7FF));
        assert_eq!(scheme.on_primary, Color::new(0xFF000000));
        assert_eq!(scheme.primary_container(), scheme.primary);
        assert_eq!(scheme.on_primary_container(), scheme.on_primary);
        assert_eq!(scheme.primary_fixed(), scheme.primary);
        assert_eq!(scheme.primary_fixed_dim(), scheme.primary);
        assert_eq!(scheme.on_primary_fixed(), scheme.on_primary);
        assert_eq!(scheme.on_primary_fixed_variant(), scheme.on_primary);
        assert_eq!(scheme.secondary, Color::new(0xFF66FFF9));
        assert_eq!(scheme.on_secondary, Color::new(0xFF000000));
        assert_eq!(scheme.secondary_container(), scheme.secondary);
        assert_eq!(scheme.on_secondary_container(), scheme.on_secondary);
        assert_eq!(scheme.secondary_fixed(), scheme.secondary);
        assert_eq!(scheme.secondary_fixed_dim(), scheme.secondary);
        assert_eq!(scheme.on_secondary_fixed(), scheme.on_secondary);
        assert_eq!(scheme.on_secondary_fixed_variant(), scheme.on_secondary);
        assert_eq!(scheme.tertiary(), scheme.secondary);
        assert_eq!(scheme.on_tertiary(), scheme.on_secondary);
        assert_eq!(scheme.tertiary_container(), scheme.tertiary());
        assert_eq!(scheme.on_tertiary_container(), scheme.on_tertiary());
        assert_eq!(scheme.tertiary_fixed(), scheme.tertiary());
        assert_eq!(scheme.tertiary_fixed_dim(), scheme.tertiary());
        assert_eq!(scheme.on_tertiary_fixed(), scheme.on_tertiary());
        assert_eq!(scheme.on_tertiary_fixed_variant(), scheme.on_tertiary());
        assert_eq!(scheme.error, Color::new(0xFF9B374D));
        assert_eq!(scheme.on_error, Color::new(0xFF000000));
        assert_eq!(scheme.error_container(), scheme.error);
        assert_eq!(scheme.on_error_container(), scheme.on_error);
        assert_eq!(scheme.background(), Color::new(0xFF121212));
        assert_eq!(scheme.on_background(), Color::new(0xFFFFFFFF));
        assert_eq!(scheme.surface, Color::new(0xFF121212));
        assert_eq!(scheme.surface_bright(), scheme.surface);
        assert_eq!(scheme.surface_dim(), scheme.surface);
        assert_eq!(scheme.surface_container_lowest(), scheme.surface);
        assert_eq!(scheme.surface_container_low(), scheme.surface);
        assert_eq!(scheme.surface_container(), scheme.surface);
        assert_eq!(scheme.surface_container_high(), scheme.surface);
        assert_eq!(scheme.surface_container_highest(), scheme.surface);
        assert_eq!(scheme.on_surface, Color::new(0xFFFFFFFF));
        assert_eq!(scheme.surface_variant(), scheme.surface);
        assert_eq!(scheme.on_surface_variant(), scheme.on_surface);
        assert_eq!(scheme.outline(), scheme.on_background());
        assert_eq!(scheme.outline_variant(), scheme.on_background());
        assert_eq!(scheme.shadow(), Color::new(0xFF000000));
        assert_eq!(scheme.scrim(), Color::new(0xFF000000));
        assert_eq!(scheme.inverse_surface(), scheme.on_surface);
        assert_eq!(scheme.on_inverse_surface(), scheme.surface);
        assert_eq!(scheme.inverse_primary(), scheme.on_primary);
        assert_eq!(scheme.surface_tint(), scheme.primary);
    }

    #[test]
    fn can_generate_a_light_scheme_from_a_seed_color() {
        let scheme = ColorScheme::from_seed(Colors::BLUE.primary()).build();
        assert_eq!(scheme.primary, Color::new(0xFF36618E));
        assert_eq!(scheme.on_primary, Color::new(0xFFFFFFFF));
        assert_eq!(scheme.primary_container(), Color::new(0xFFD1E4FF));
        assert_eq!(scheme.on_primary_container(), Color::new(0xFF194975));
        assert_eq!(scheme.primary_fixed(), Color::new(0xFFD1E4FF));
        assert_eq!(scheme.primary_fixed_dim(), Color::new(0xFFA0CAFD));
        assert_eq!(scheme.on_primary_fixed(), Color::new(0xFF001D36));
        assert_eq!(scheme.on_primary_fixed_variant(), Color::new(0xFF194975));
        assert_eq!(scheme.secondary, Color::new(0xFF535F70));
        assert_eq!(scheme.on_secondary, Color::new(0xFFFFFFFF));
        assert_eq!(scheme.secondary_container(), Color::new(0xFFD7E3F7));
        assert_eq!(scheme.on_secondary_container(), Color::new(0xFF3B4858));
        assert_eq!(scheme.secondary_fixed(), Color::new(0xFFD7E3F7));
        assert_eq!(scheme.secondary_fixed_dim(), Color::new(0xFFBBC7DB));
        assert_eq!(scheme.on_secondary_fixed(), Color::new(0xFF101C2B));
        assert_eq!(scheme.on_secondary_fixed_variant(), Color::new(0xFF3B4858));
        assert_eq!(scheme.tertiary(), Color::new(0xFF6B5778));
        assert_eq!(scheme.on_tertiary(), Color::new(0xFFFFFFFF));
        assert_eq!(scheme.tertiary_container(), Color::new(0xFFF2DAFF));
        assert_eq!(scheme.on_tertiary_container(), Color::new(0xFF523F5F));
        assert_eq!(scheme.tertiary_fixed(), Color::new(0xFFF2DAFF));
        assert_eq!(scheme.tertiary_fixed_dim(), Color::new(0xFFD6BEE4));
        assert_eq!(scheme.on_tertiary_fixed(), Color::new(0xFF251431));
        assert_eq!(scheme.on_tertiary_fixed_variant(), Color::new(0xFF523F5F));
        assert_eq!(scheme.error, Color::new(0xFFBA1A1A));
        assert_eq!(scheme.on_error, Color::new(0xFFFFFFFF));
        assert_eq!(scheme.error_container(), Color::new(0xFFFFDAD6));
        assert_eq!(scheme.on_error_container(), Color::new(0xFF93000A));
        assert_eq!(scheme.outline(), Color::new(0xFF73777F));
        assert_eq!(scheme.outline_variant(), Color::new(0xFFC3C7CF));
        assert_eq!(scheme.background(), Color::new(0xFFF8F9FF));
        assert_eq!(scheme.on_background(), Color::new(0xFF191C20));
        assert_eq!(scheme.surface, Color::new(0xFFF8F9FF));
        assert_eq!(scheme.surface_bright(), Color::new(0xFFF8F9FF));
        assert_eq!(scheme.surface_dim(), Color::new(0xFFD8DAE0));
        assert_eq!(scheme.surface_container_lowest(), Color::new(0xFFFFFFFF));
        assert_eq!(scheme.surface_container_low(), Color::new(0xFFF2F3FA));
        assert_eq!(scheme.surface_container(), Color::new(0xFFECEEF4));
        assert_eq!(scheme.surface_container_high(), Color::new(0xFFE6E8EE));
        assert_eq!(scheme.surface_container_highest(), Color::new(0xFFE1E2E8));
        assert_eq!(scheme.on_surface, Color::new(0xFF191C20));
        assert_eq!(scheme.surface_variant(), Color::new(0xFFDFE2EB));
        assert_eq!(scheme.on_surface_variant(), Color::new(0xFF43474E));
        assert_eq!(scheme.inverse_surface(), Color::new(0xFF2E3135));
        assert_eq!(scheme.on_inverse_surface(), Color::new(0xFFEFF0F7));
        assert_eq!(scheme.inverse_primary(), Color::new(0xFFA0CAFD));
        assert_eq!(scheme.shadow(), Color::new(0xFF000000));
        assert_eq!(scheme.scrim(), Color::new(0xFF000000));
        assert_eq!(scheme.surface_tint(), Color::new(0xFF36618E));
        assert_eq!(scheme.brightness, Brightness::Light);
    }

    #[test]
    fn copy_with_overrides_given_colors() {
        let scheme = ColorScheme::light()
            .copy_with()
            .with_brightness(Brightness::Dark)
            .with_primary(Color::new(0x00000001))
            .with_on_primary(Color::new(0x00000002))
            .with_primary_container(Color::new(0x00000003))
            .with_on_primary_container(Color::new(0x00000004))
            .with_primary_fixed(Color::new(0x0000001D))
            .with_primary_fixed_dim(Color::new(0x0000001E))
            .with_on_primary_fixed(Color::new(0x0000001F))
            .with_on_primary_fixed_variant(Color::new(0x00000020))
            .with_secondary(Color::new(0x00000005))
            .with_on_secondary(Color::new(0x00000006))
            .with_secondary_container(Color::new(0x00000007))
            .with_on_secondary_container(Color::new(0x00000008))
            .with_secondary_fixed(Color::new(0x00000021))
            .with_secondary_fixed_dim(Color::new(0x00000022))
            .with_on_secondary_fixed(Color::new(0x00000023))
            .with_on_secondary_fixed_variant(Color::new(0x00000024))
            .with_tertiary(Color::new(0x00000009))
            .with_on_tertiary(Color::new(0x0000000A))
            .with_tertiary_container(Color::new(0x0000000B))
            .with_on_tertiary_container(Color::new(0x0000000C))
            .with_tertiary_fixed(Color::new(0x00000025))
            .with_tertiary_fixed_dim(Color::new(0x00000026))
            .with_on_tertiary_fixed(Color::new(0x00000027))
            .with_on_tertiary_fixed_variant(Color::new(0x00000028))
            .with_error(Color::new(0x0000000D))
            .with_on_error(Color::new(0x0000000E))
            .with_error_container(Color::new(0x0000000F))
            .with_on_error_container(Color::new(0x00000010))
            .with_background(Color::new(0x00000011))
            .with_on_background(Color::new(0x00000012))
            .with_surface(Color::new(0x00000013))
            .with_surface_dim(Color::new(0x00000029))
            .with_surface_bright(Color::new(0x0000002A))
            .with_surface_container_lowest(Color::new(0x0000002B))
            .with_surface_container_low(Color::new(0x0000002C))
            .with_surface_container(Color::new(0x0000002D))
            .with_surface_container_high(Color::new(0x0000002E))
            .with_surface_container_highest(Color::new(0x0000002F))
            .with_on_surface(Color::new(0x00000014))
            .with_surface_variant(Color::new(0x00000015))
            .with_on_surface_variant(Color::new(0x00000016))
            .with_outline(Color::new(0x00000017))
            .with_outline_variant(Color::new(0x00000117))
            .with_shadow(Color::new(0x00000018))
            .with_scrim(Color::new(0x00000118))
            .with_inverse_surface(Color::new(0x00000019))
            .with_on_inverse_surface(Color::new(0x0000001A))
            .with_inverse_primary(Color::new(0x0000001B))
            .with_surface_tint(Color::new(0x0000001C));

        assert_eq!(scheme.brightness, Brightness::Dark);
        assert_eq!(scheme.primary, Color::new(0x00000001));
        assert_eq!(scheme.on_primary, Color::new(0x00000002));
        assert_eq!(scheme.primary_container(), Color::new(0x00000003));
        assert_eq!(scheme.on_primary_container(), Color::new(0x00000004));
        assert_eq!(scheme.primary_fixed(), Color::new(0x0000001D));
        assert_eq!(scheme.primary_fixed_dim(), Color::new(0x0000001E));
        assert_eq!(scheme.on_primary_fixed(), Color::new(0x0000001F));
        assert_eq!(scheme.on_primary_fixed_variant(), Color::new(0x00000020));
        assert_eq!(scheme.secondary, Color::new(0x00000005));
        assert_eq!(scheme.on_secondary, Color::new(0x00000006));
        assert_eq!(scheme.secondary_container(), Color::new(0x00000007));
        assert_eq!(scheme.on_secondary_container(), Color::new(0x00000008));
        assert_eq!(scheme.secondary_fixed(), Color::new(0x00000021));
        assert_eq!(scheme.secondary_fixed_dim(), Color::new(0x00000022));
        assert_eq!(scheme.on_secondary_fixed(), Color::new(0x00000023));
        assert_eq!(scheme.on_secondary_fixed_variant(), Color::new(0x00000024));
        assert_eq!(scheme.tertiary(), Color::new(0x00000009));
        assert_eq!(scheme.on_tertiary(), Color::new(0x0000000A));
        assert_eq!(scheme.tertiary_container(), Color::new(0x0000000B));
        assert_eq!(scheme.on_tertiary_container(), Color::new(0x0000000C));
        assert_eq!(scheme.tertiary_fixed(), Color::new(0x00000025));
        assert_eq!(scheme.tertiary_fixed_dim(), Color::new(0x00000026));
        assert_eq!(scheme.on_tertiary_fixed(), Color::new(0x00000027));
        assert_eq!(scheme.on_tertiary_fixed_variant(), Color::new(0x00000028));
        assert_eq!(scheme.error, Color::new(0x0000000D));
        assert_eq!(scheme.on_error, Color::new(0x0000000E));
        assert_eq!(scheme.error_container(), Color::new(0x0000000F));
        assert_eq!(scheme.on_error_container(), Color::new(0x00000010));
        assert_eq!(scheme.background(), Color::new(0x00000011));
        assert_eq!(scheme.on_background(), Color::new(0x00000012));
        assert_eq!(scheme.surface, Color::new(0x00000013));
        assert_eq!(scheme.surface_dim(), Color::new(0x00000029));
        assert_eq!(scheme.surface_bright(), Color::new(0x0000002A));
        assert_eq!(scheme.surface_container_lowest(), Color::new(0x0000002B));
        assert_eq!(scheme.surface_container_low(), Color::new(0x0000002C));
        assert_eq!(scheme.surface_container(), Color::new(0x0000002D));
        assert_eq!(scheme.surface_container_high(), Color::new(0x0000002E));
        assert_eq!(scheme.surface_container_highest(), Color::new(0x0000002F));
        assert_eq!(scheme.on_surface, Color::new(0x00000014));
        assert_eq!(scheme.surface_variant(), Color::new(0x00000015));
        assert_eq!(scheme.on_surface_variant(), Color::new(0x00000016));
        assert_eq!(scheme.outline(), Color::new(0x00000017));
        assert_eq!(scheme.outline_variant(), Color::new(0x00000117));
        assert_eq!(scheme.shadow(), Color::new(0x00000018));
        assert_eq!(scheme.scrim(), Color::new(0x00000118));
        assert_eq!(scheme.inverse_surface(), Color::new(0x00000019));
        assert_eq!(scheme.on_inverse_surface(), Color::new(0x0000001A));
        assert_eq!(scheme.inverse_primary(), Color::new(0x0000001B));
        assert_eq!(scheme.surface_tint(), Color::new(0x0000001C));
    }

    #[test]
    fn can_generate_a_dark_scheme_from_a_seed_color() {
        let scheme = ColorScheme::from_seed(Colors::BLUE.primary())
            .brightness(Brightness::Dark)
            .build();
        assert_eq!(scheme.primary, Color::new(0xFFA0CAFD));
        assert_eq!(scheme.on_primary, Color::new(0xFF003258));
        assert_eq!(scheme.primary_container(), Color::new(0xFF194975));
        assert_eq!(scheme.on_primary_container(), Color::new(0xFFD1E4FF));
        assert_eq!(scheme.primary_fixed(), Color::new(0xFFD1E4FF));
        assert_eq!(scheme.primary_fixed_dim(), Color::new(0xFFA0CAFD));
        assert_eq!(scheme.on_primary_fixed(), Color::new(0xFF001D36));
        assert_eq!(scheme.on_primary_fixed_variant(), Color::new(0xFF194975));
        assert_eq!(scheme.secondary, Color::new(0xFFBBC7DB));
        assert_eq!(scheme.on_secondary, Color::new(0xFF253140));
        assert_eq!(scheme.secondary_container(), Color::new(0xFF3B4858));
        assert_eq!(scheme.on_secondary_container(), Color::new(0xFFD7E3F7));
        assert_eq!(scheme.secondary_fixed(), Color::new(0xFFD7E3F7));
        assert_eq!(scheme.secondary_fixed_dim(), Color::new(0xFFBBC7DB));
        assert_eq!(scheme.on_secondary_fixed(), Color::new(0xFF101C2B));
        assert_eq!(scheme.on_secondary_fixed_variant(), Color::new(0xFF3B4858));
        assert_eq!(scheme.tertiary(), Color::new(0xFFD6BEE4));
        assert_eq!(scheme.on_tertiary(), Color::new(0xFF3B2948));
        assert_eq!(scheme.tertiary_container(), Color::new(0xFF523F5F));
        assert_eq!(scheme.on_tertiary_container(), Color::new(0xFFF2DAFF));
        assert_eq!(scheme.tertiary_fixed(), Color::new(0xFFF2DAFF));
        assert_eq!(scheme.tertiary_fixed_dim(), Color::new(0xFFD6BEE4));
        assert_eq!(scheme.on_tertiary_fixed(), Color::new(0xFF251431));
        assert_eq!(scheme.on_tertiary_fixed_variant(), Color::new(0xFF523F5F));
        assert_eq!(scheme.error, Color::new(0xFFFFB4AB));
        assert_eq!(scheme.on_error, Color::new(0xFF690005));
        assert_eq!(scheme.error_container(), Color::new(0xFF93000A));
        assert_eq!(scheme.on_error_container(), Color::new(0xFFFFDAD6));
        assert_eq!(scheme.outline(), Color::new(0xFF8D9199));
        assert_eq!(scheme.outline_variant(), Color::new(0xFF43474E));
        assert_eq!(scheme.background(), Color::new(0xFF111418));
        assert_eq!(scheme.on_background(), Color::new(0xFFE1E2E8));
        assert_eq!(scheme.surface, Color::new(0xFF111418));
        assert_eq!(scheme.surface_dim(), Color::new(0xFF111418));
        assert_eq!(scheme.surface_bright(), Color::new(0xFF36393E));
        assert_eq!(scheme.surface_container_lowest(), Color::new(0xFF0B0E13));
        assert_eq!(scheme.surface_container_low(), Color::new(0xFF191C20));
        assert_eq!(scheme.surface_container(), Color::new(0xFF1D2024));
        assert_eq!(scheme.surface_container_high(), Color::new(0xFF272A2F));
        assert_eq!(scheme.surface_container_highest(), Color::new(0xFF32353A));
        assert_eq!(scheme.on_surface, Color::new(0xFFE1E2E8));
        assert_eq!(scheme.surface_variant(), Color::new(0xFF43474E));
        assert_eq!(scheme.on_surface_variant(), Color::new(0xFFC3C7CF));
        assert_eq!(scheme.inverse_surface(), Color::new(0xFFE1E2E8));
        assert_eq!(scheme.on_inverse_surface(), Color::new(0xFF2E3135));
        assert_eq!(scheme.inverse_primary(), Color::new(0xFF36618E));
        assert_eq!(scheme.shadow(), Color::new(0xFF000000));
        assert_eq!(scheme.scrim(), Color::new(0xFF000000));
        assert_eq!(scheme.surface_tint(), Color::new(0xFFA0CAFD));
        assert_eq!(scheme.brightness, Brightness::Dark);
    }

    #[test]
    fn can_override_specific_colors_in_a_generated_scheme() {
        let base_scheme = ColorScheme::from_seed(Colors::BLUE.primary()).build();
        let primary_override = Color::new(0xFFABCDEF);
        let scheme = ColorScheme::from_seed(Colors::BLUE.primary())
            .build()
            .with_primary(primary_override);
        assert_eq!(scheme.primary, primary_override);
        // The rest should be the same.
        // The rest should be the same.
        assert_eq!(scheme.on_primary, base_scheme.on_primary);
        assert_eq!(scheme.primary_container(), base_scheme.primary_container());
        assert_eq!(
            scheme.on_primary_container(),
            base_scheme.on_primary_container()
        );
        assert_eq!(scheme.primary_fixed(), base_scheme.primary_fixed());
        assert_eq!(scheme.primary_fixed_dim(), base_scheme.primary_fixed_dim());
        assert_eq!(scheme.on_primary_fixed(), base_scheme.on_primary_fixed());
        assert_eq!(
            scheme.on_primary_fixed_variant(),
            base_scheme.on_primary_fixed_variant()
        );
        assert_eq!(scheme.secondary, base_scheme.secondary);
        assert_eq!(scheme.on_secondary, base_scheme.on_secondary);
        assert_eq!(
            scheme.secondary_container(),
            base_scheme.secondary_container()
        );
        assert_eq!(
            scheme.on_secondary_container(),
            base_scheme.on_secondary_container()
        );
        assert_eq!(scheme.secondary_fixed(), base_scheme.secondary_fixed());
        assert_eq!(
            scheme.secondary_fixed_dim(),
            base_scheme.secondary_fixed_dim()
        );
        assert_eq!(
            scheme.on_secondary_fixed(),
            base_scheme.on_secondary_fixed()
        );
        assert_eq!(
            scheme.on_secondary_fixed_variant(),
            base_scheme.on_secondary_fixed_variant()
        );
        assert_eq!(scheme.tertiary(), base_scheme.tertiary());
        assert_eq!(scheme.on_tertiary(), base_scheme.on_tertiary());
        assert_eq!(
            scheme.tertiary_container(),
            base_scheme.tertiary_container()
        );
        assert_eq!(
            scheme.on_tertiary_container(),
            base_scheme.on_tertiary_container()
        );
        assert_eq!(scheme.tertiary_fixed(), base_scheme.tertiary_fixed());
        assert_eq!(
            scheme.tertiary_fixed_dim(),
            base_scheme.tertiary_fixed_dim()
        );
        assert_eq!(scheme.on_tertiary_fixed(), base_scheme.on_tertiary_fixed());
        assert_eq!(
            scheme.on_tertiary_fixed_variant(),
            base_scheme.on_tertiary_fixed_variant()
        );
        assert_eq!(scheme.error, base_scheme.error);
        assert_eq!(scheme.on_error, base_scheme.on_error);
        assert_eq!(scheme.error_container(), base_scheme.error_container());
        assert_eq!(
            scheme.on_error_container(),
            base_scheme.on_error_container()
        );
        assert_eq!(scheme.outline(), base_scheme.outline());
        assert_eq!(scheme.outline_variant(), base_scheme.outline_variant());
        assert_eq!(scheme.background(), base_scheme.background());
        assert_eq!(scheme.on_background(), base_scheme.on_background());
        assert_eq!(scheme.surface, base_scheme.surface);
        assert_eq!(scheme.surface_bright(), base_scheme.surface_bright());
        assert_eq!(scheme.surface_dim(), base_scheme.surface_dim());
        assert_eq!(
            scheme.surface_container_lowest(),
            base_scheme.surface_container_lowest()
        );
        assert_eq!(
            scheme.surface_container_low(),
            base_scheme.surface_container_low()
        );
        assert_eq!(scheme.surface_container(), base_scheme.surface_container());
        assert_eq!(
            scheme.surface_container_high(),
            base_scheme.surface_container_high()
        );
        assert_eq!(
            scheme.surface_container_highest(),
            base_scheme.surface_container_highest()
        );
        assert_eq!(scheme.on_surface, base_scheme.on_surface);
        assert_eq!(scheme.surface_variant(), base_scheme.surface_variant());
        assert_eq!(
            scheme.on_surface_variant(),
            base_scheme.on_surface_variant()
        );
        assert_eq!(scheme.inverse_surface(), base_scheme.inverse_surface());
        assert_eq!(
            scheme.on_inverse_surface(),
            base_scheme.on_inverse_surface()
        );
        assert_eq!(scheme.inverse_primary(), base_scheme.inverse_primary());
        assert_eq!(scheme.shadow(), base_scheme.shadow());
        assert_eq!(scheme.scrim(), base_scheme.shadow());
        assert_eq!(scheme.surface_tint(), base_scheme.surface_tint());
        assert_eq!(scheme.brightness, base_scheme.brightness);
    }

    fn expect_colors_match_dynamic_scheme(
        color_scheme: &ColorScheme,
        dynamic_scheme: &DynamicScheme,
    ) {
        assert_eq!(
            color_scheme.primary.to_argb32(),
            MaterialDynamicColors::primary().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_primary.to_argb32(),
            MaterialDynamicColors::on_primary().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.primary_container().to_argb32(),
            MaterialDynamicColors::primary_container().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_primary_container().to_argb32(),
            MaterialDynamicColors::on_primary_container().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.primary_fixed().to_argb32(),
            MaterialDynamicColors::primary_fixed().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.primary_fixed_dim().to_argb32(),
            MaterialDynamicColors::primary_fixed_dim().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_primary_fixed().to_argb32(),
            MaterialDynamicColors::on_primary_fixed().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_primary_fixed_variant().to_argb32(),
            MaterialDynamicColors::on_primary_fixed_variant().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.secondary.to_argb32(),
            MaterialDynamicColors::secondary().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_secondary.to_argb32(),
            MaterialDynamicColors::on_secondary().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.secondary_container().to_argb32(),
            MaterialDynamicColors::secondary_container().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_secondary_container().to_argb32(),
            MaterialDynamicColors::on_secondary_container().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.secondary_fixed().to_argb32(),
            MaterialDynamicColors::secondary_fixed().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.secondary_fixed_dim().to_argb32(),
            MaterialDynamicColors::secondary_fixed_dim().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_secondary_fixed().to_argb32(),
            MaterialDynamicColors::on_secondary_fixed().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_secondary_fixed_variant().to_argb32(),
            MaterialDynamicColors::on_secondary_fixed_variant().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.tertiary().to_argb32(),
            MaterialDynamicColors::tertiary().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_tertiary().to_argb32(),
            MaterialDynamicColors::on_tertiary().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.tertiary_container().to_argb32(),
            MaterialDynamicColors::tertiary_container().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_tertiary_container().to_argb32(),
            MaterialDynamicColors::on_tertiary_container().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.tertiary_fixed().to_argb32(),
            MaterialDynamicColors::tertiary_fixed().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.tertiary_fixed_dim().to_argb32(),
            MaterialDynamicColors::tertiary_fixed_dim().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_tertiary_fixed().to_argb32(),
            MaterialDynamicColors::on_tertiary_fixed().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_tertiary_fixed_variant().to_argb32(),
            MaterialDynamicColors::on_tertiary_fixed_variant().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.error.to_argb32(),
            MaterialDynamicColors::error().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_error.to_argb32(),
            MaterialDynamicColors::on_error().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.error_container().to_argb32(),
            MaterialDynamicColors::error_container().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_error_container().to_argb32(),
            MaterialDynamicColors::on_error_container().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.background().to_argb32(),
            MaterialDynamicColors::background().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_background().to_argb32(),
            MaterialDynamicColors::on_background().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.surface.to_argb32(),
            MaterialDynamicColors::surface().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.surface_dim().to_argb32(),
            MaterialDynamicColors::surface_dim().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.surface_bright().to_argb32(),
            MaterialDynamicColors::surface_bright().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.surface_container_lowest().to_argb32(),
            MaterialDynamicColors::surface_container_lowest().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.surface_container_low().to_argb32(),
            MaterialDynamicColors::surface_container_low().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.surface_container().to_argb32(),
            MaterialDynamicColors::surface_container().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.surface_container_high().to_argb32(),
            MaterialDynamicColors::surface_container_high().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.surface_container_highest().to_argb32(),
            MaterialDynamicColors::surface_container_highest().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_surface.to_argb32(),
            MaterialDynamicColors::on_surface().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.surface_variant().to_argb32(),
            MaterialDynamicColors::surface_variant().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_surface_variant().to_argb32(),
            MaterialDynamicColors::on_surface_variant().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.outline().to_argb32(),
            MaterialDynamicColors::outline().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.outline_variant().to_argb32(),
            MaterialDynamicColors::outline_variant().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.shadow().to_argb32(),
            MaterialDynamicColors::shadow().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.scrim().to_argb32(),
            MaterialDynamicColors::scrim().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.inverse_surface().to_argb32(),
            MaterialDynamicColors::inverse_surface().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.on_inverse_surface().to_argb32(),
            MaterialDynamicColors::inverse_on_surface().get_argb(dynamic_scheme)
        );
        assert_eq!(
            color_scheme.inverse_primary().to_argb32(),
            MaterialDynamicColors::inverse_primary().get_argb(dynamic_scheme)
        );
    }

    #[test]
    fn from_seed_asserts_on_invalid_contrast_levels() {
        let below = catch_unwind(AssertUnwindSafe(|| {
            ColorScheme::from_seed(Colors::BLUE.primary())
                .contrast_level(-1.5)
                .build()
        }));
        assert!(below.is_err());

        let above = catch_unwind(AssertUnwindSafe(|| {
            ColorScheme::from_seed(Colors::BLUE.primary())
                .contrast_level(1.5)
                .build()
        }));
        assert!(above.is_err());
    }

    fn dynamic_scheme_for(
        variant: DynamicSchemeVariant,
        source_color: Hct,
        is_dark: bool,
        contrast_level: f64,
    ) -> DynamicScheme {
        match variant {
            DynamicSchemeVariant::TonalSpot => {
                SchemeTonalSpot::new(source_color, is_dark, contrast_level)
            }
            DynamicSchemeVariant::Fidelity => {
                SchemeFidelity::new(source_color, is_dark, contrast_level)
            }
            DynamicSchemeVariant::Content => {
                SchemeContent::new(source_color, is_dark, contrast_level)
            }
            DynamicSchemeVariant::Monochrome => {
                SchemeMonochrome::new(source_color, is_dark, contrast_level)
            }
            DynamicSchemeVariant::Neutral => {
                SchemeNeutral::new(source_color, is_dark, contrast_level)
            }
            DynamicSchemeVariant::Vibrant => {
                SchemeVibrant::new(source_color, is_dark, contrast_level)
            }
            DynamicSchemeVariant::Expressive => {
                SchemeExpressive::new(source_color, is_dark, contrast_level)
            }
            DynamicSchemeVariant::Rainbow => {
                SchemeRainbow::new(source_color, is_dark, contrast_level)
            }
            DynamicSchemeVariant::FruitSalad => {
                SchemeFruitSalad::new(source_color, is_dark, contrast_level)
            }
        }
    }

    #[test]
    fn color_values_in_from_seed_with_different_variants_match_values_in_dynamic_scheme() {
        let seed_color = Colors::ORANGE.primary();
        let source_color = Hct::from_int(seed_color.to_argb32());
        for scheme_variant in DynamicSchemeVariant::VALUES {
            let dynamic_scheme = dynamic_scheme_for(scheme_variant, source_color, false, 0.0);
            let color_scheme = ColorScheme::from_seed(seed_color)
                .dynamic_scheme_variant(scheme_variant)
                .build();

            expect_colors_match_dynamic_scheme(&color_scheme, &dynamic_scheme);
        }
    }

    #[test]
    fn colors_in_high_contrast_color_scheme_match_colors_in_dynamic_scheme() {
        let seed_color = Colors::BLUE.primary();
        let source_color = Hct::from_int(seed_color.to_argb32());

        let colors_match_dynamic_scheme_colors =
            |scheme_variant: DynamicSchemeVariant, brightness: Brightness, contrast_level: f64| {
                let is_dark = brightness == Brightness::Dark;
                let dynamic_scheme =
                    dynamic_scheme_for(scheme_variant, source_color, is_dark, contrast_level);

                let color_scheme = ColorScheme::from_seed(seed_color)
                    .brightness(brightness)
                    .dynamic_scheme_variant(scheme_variant)
                    .contrast_level(contrast_level)
                    .build();

                expect_colors_match_dynamic_scheme(&color_scheme, &dynamic_scheme);
            };

        for scheme_variant in DynamicSchemeVariant::VALUES {
            colors_match_dynamic_scheme_colors(scheme_variant, Brightness::Light, 1.0); // High contrast
            colors_match_dynamic_scheme_colors(scheme_variant, Brightness::Dark, 1.0);

            colors_match_dynamic_scheme_colors(scheme_variant, Brightness::Light, 0.5); // Medium contrast
            colors_match_dynamic_scheme_colors(scheme_variant, Brightness::Dark, 0.5);
        }
    }

    /// Dart's `_testFilledButtonColor`: the colour of the `Material` a `FilledButton` builds
    /// under `ThemeData(colorScheme: scheme)`.
    fn test_filled_button_color(
        tester: &mut WidgetTester,
        scheme: ColorScheme,
        expectation: Color,
    ) {
        tester.pump_widget(Container::new().into_widget()); // reset
        let theme = ThemeData::builder()
            .color_scheme(scheme)
            .build(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            FilledButton::new(
                Some(Listener::new(|_app| {})),
                Some(SizedBox::square(Some(200.0))),
            )
            .into_widget(),
        ));
        let button = find::by_type::<FilledButton>().evaluate(tester);
        assert_eq!(button.len(), 1);
        let materials: Vec<_> = find::by_type::<Material>()
            .evaluate(tester)
            .into_iter()
            .filter(|material| tester.ancestors(*material).contains(&button[0]))
            .collect();
        assert_eq!(materials.len(), 1);
        let color = tester.widget::<Material, _>(materials[0], |material| material.color);
        assert_eq!(color, Some(expectation));
    }

    #[test]
    fn color_scheme_from_seed_with_different_variants_spot_checks() {
        test_widgets(|tester| {
            // Default (Variant.tonalSpot).
            for (seed, expectation) in [
                (0xFF000000, 0xFF8C4A60),
                (0xFF00FF00, 0xFF406836),
                (0xFF6559F5, 0xFF5B5891),
                (0xFFFFFFFF, 0xFF006874),
            ] {
                test_filled_button_color(
                    tester,
                    ColorScheme::from_seed(Color::new(seed)).build(),
                    Color::new(expectation),
                );
            }

            // Variant.fidelity.
            for (seed, expectation) in [
                (0xFF000000, 0xFF000000),
                (0xFF00FF00, 0xFF026E00),
                (0xFF6559F5, 0xFF4C3CDB),
                (0xFFFFFFFF, 0xFF5D5F5F),
            ] {
                test_filled_button_color(
                    tester,
                    ColorScheme::from_seed(Color::new(seed))
                        .dynamic_scheme_variant(DynamicSchemeVariant::Fidelity)
                        .build(),
                    Color::new(expectation),
                );
            }
        });
    }

    #[test]
    fn color_scheme_of_context_is_equivalent_to_theme_of_context_color_scheme() {
        test_widgets(|tester| {
            let color_scheme = ColorScheme::from_seed(Colors::RED.primary()).build();
            let seen: Rc<RefCell<Option<(ColorScheme, ColorScheme)>>> = Rc::default();
            let theme = ThemeData::from(color_scheme.clone()).build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Builder::new({
                    let seen = Rc::clone(&seen);
                    move |app, context| {
                        let color_scheme_of_theme = Theme::of(app, context).color_scheme.clone();
                        let color_scheme_from_context = ColorScheme::of(app, context);
                        *seen.borrow_mut() =
                            Some((color_scheme_of_theme, color_scheme_from_context));
                        SizedBox::shrink().into_widget()
                    }
                })
                .into_widget(),
            ));
            let (color_scheme_of_theme, color_scheme_from_context) =
                seen.borrow_mut().take().expect("the probe built");
            assert_eq!(color_scheme_of_theme, color_scheme);
            assert_eq!(color_scheme_from_context, color_scheme);
        });
    }
}
