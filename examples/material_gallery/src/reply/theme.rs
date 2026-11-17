//! Reply's theme: `ui/theme`'s colour schemes, type scale and shapes.
//!
//! The schemes are the sample's standard-contrast pair, made with Material Theme Builder; its
//! medium- and high-contrast schemes, picked from Android's contrast setting, wait for a host
//! that reports one. The type scale is the sample's sizes and weights in the platform's own face,
//! as the sample sets no font family. Compose's shape scale has no counterpart in Flutter, so the
//! components Reply shows take the scale's corners through their themes.

use waft_embedder::{Brightness, Color, FontWeight};
use waft_foundation::App;
use waft_material::{
    CardThemeData, ColorScheme, FloatingActionButtonThemeData, TextTheme, ThemeData,
};
use waft_painting::{BorderRadius, BorderSide, RoundedRectangleBorder, TextStyle};

/// The theme for `brightness`.
pub fn reply_theme(app: &App, brightness: Brightness) -> ThemeData {
    let color_scheme = match brightness {
        Brightness::Light => light_scheme(),
        Brightness::Dark => dark_scheme(),
    };
    ThemeData::builder()
        .color_scheme(color_scheme)
        .text_theme(reply_typography())
        .card_theme(CardThemeData::new().shape(rounded(shapes::MEDIUM)))
        .floating_action_button_theme(
            FloatingActionButtonThemeData::new().shape(rounded(shapes::LARGE)),
        )
        .build(app)
}

/// `Shapes.kt`: the corner radii of Reply's shape scale.
pub mod shapes {
    pub const EXTRA_SMALL: f64 = 4.0;
    pub const SMALL: f64 = 8.0;
    /// Compose's cards take this one.
    pub const MEDIUM: f64 = 16.0;
    /// Compose's floating action buttons take this one.
    pub const LARGE: f64 = 24.0;
    pub const EXTRA_LARGE: f64 = 32.0;
}

/// A rounded rectangle of `radius`.
pub fn rounded(radius: f64) -> RoundedRectangleBorder {
    RoundedRectangleBorder::new(BorderSide::NONE, BorderRadius::circular(radius).into())
}

/// `Theme.kt`'s `lightScheme`. Reply still fills its mail cards with `surfaceVariant`, a tone
/// apart from `surfaceContainerHighest` in this scheme, so it is kept though Flutter deprecates it.
#[allow(deprecated)]
fn light_scheme() -> ColorScheme {
    ColorScheme::new(
        Brightness::Light,
        Color::new(0xFF805610),
        Color::new(0xFFFFFFFF),
        Color::new(0xFF6F5B40),
        Color::new(0xFFFFFFFF),
        Color::new(0xFFBA1A1A),
        Color::new(0xFFFFFFFF),
        Color::new(0xFFFFF8F4),
        Color::new(0xFF201B13),
    )
    .copy_with()
    .with_primary_container(Color::new(0xFFFFDDB3))
    .with_on_primary_container(Color::new(0xFF291800))
    .with_secondary_container(Color::new(0xFFFBDEBC))
    .with_on_secondary_container(Color::new(0xFF271904))
    .with_tertiary(Color::new(0xFF51643F))
    .with_on_tertiary(Color::new(0xFFFFFFFF))
    .with_tertiary_container(Color::new(0xFFD4EABB))
    .with_on_tertiary_container(Color::new(0xFF102004))
    .with_error_container(Color::new(0xFFFFDAD6))
    .with_on_error_container(Color::new(0xFF410002))
    .with_surface_variant(Color::new(0xFFF0E0CF))
    .with_on_surface_variant(Color::new(0xFF4F4539))
    .with_outline(Color::new(0xFF817567))
    .with_outline_variant(Color::new(0xFFD3C4B4))
    .with_scrim(Color::new(0xFF000000))
    .with_inverse_surface(Color::new(0xFF362F27))
    .with_on_inverse_surface(Color::new(0xFFFCEFE2))
    .with_inverse_primary(Color::new(0xFFF4BD6F))
    .with_surface_dim(Color::new(0xFFE4D8CC))
    .with_surface_bright(Color::new(0xFFFFF8F4))
    .with_surface_container_lowest(Color::new(0xFFFFFFFF))
    .with_surface_container_low(Color::new(0xFFFFF1E5))
    .with_surface_container(Color::new(0xFFF9ECDF))
    .with_surface_container_high(Color::new(0xFFF3E6DA))
    .with_surface_container_highest(Color::new(0xFFEDE0D4))
    .with_surface_tint(Color::new(0xFF805610))
}

/// `Theme.kt`'s `darkScheme`.
#[allow(deprecated)]
fn dark_scheme() -> ColorScheme {
    ColorScheme::new(
        Brightness::Dark,
        Color::new(0xFFF4BD6F),
        Color::new(0xFF452B00),
        Color::new(0xFFDDC2A1),
        Color::new(0xFF3E2D16),
        Color::new(0xFFFFB4AB),
        Color::new(0xFF690005),
        Color::new(0xFF18120B),
        Color::new(0xFFEDE0D4),
    )
    .copy_with()
    .with_primary_container(Color::new(0xFF633F00))
    .with_on_primary_container(Color::new(0xFFFFDDB3))
    .with_secondary_container(Color::new(0xFF56442A))
    .with_on_secondary_container(Color::new(0xFFFBDEBC))
    .with_tertiary(Color::new(0xFFB8CEA1))
    .with_on_tertiary(Color::new(0xFF243515))
    .with_tertiary_container(Color::new(0xFF3A4C2A))
    .with_on_tertiary_container(Color::new(0xFFD4EABB))
    .with_error_container(Color::new(0xFF93000A))
    .with_on_error_container(Color::new(0xFFFFDAD6))
    .with_surface_variant(Color::new(0xFF4F4539))
    .with_on_surface_variant(Color::new(0xFFD3C4B4))
    .with_outline(Color::new(0xFF9C8F80))
    .with_outline_variant(Color::new(0xFF4F4539))
    .with_scrim(Color::new(0xFF000000))
    .with_inverse_surface(Color::new(0xFFEDE0D4))
    .with_on_inverse_surface(Color::new(0xFF362F27))
    .with_inverse_primary(Color::new(0xFF805610))
    .with_surface_dim(Color::new(0xFF18120B))
    .with_surface_bright(Color::new(0xFF3F3830))
    .with_surface_container_lowest(Color::new(0xFF120D07))
    .with_surface_container_low(Color::new(0xFF201B13))
    .with_surface_container(Color::new(0xFF251F17))
    .with_surface_container_high(Color::new(0xFF2F2921))
    .with_surface_container_highest(Color::new(0xFF3B342B))
    .with_surface_tint(Color::new(0xFFF4BD6F))
}

/// One style of `Type.kt`: Compose's line height becomes Flutter's height, a multiple of the
/// size.
fn style(weight: FontWeight, size: f64, line_height: f64, letter_spacing: f64) -> TextStyle {
    TextStyle::new()
        .font_weight(weight)
        .font_size(size)
        .height(line_height / size)
        .letter_spacing(letter_spacing)
}

/// `Type.kt`'s `replyTypography`. The display styles are Material's.
fn reply_typography() -> TextTheme {
    const NORMAL: FontWeight = FontWeight::W400;
    const MEDIUM: FontWeight = FontWeight::W500;
    const SEMI_BOLD: FontWeight = FontWeight::W600;
    const BOLD: FontWeight = FontWeight::W700;
    let mut text_theme = TextTheme::new();
    text_theme.headline_large = Some(style(SEMI_BOLD, 32.0, 40.0, 0.0));
    text_theme.headline_medium = Some(style(SEMI_BOLD, 28.0, 36.0, 0.0));
    text_theme.headline_small = Some(style(SEMI_BOLD, 24.0, 32.0, 0.0));
    text_theme.title_large = Some(style(SEMI_BOLD, 22.0, 28.0, 0.0));
    text_theme.title_medium = Some(style(SEMI_BOLD, 16.0, 24.0, 0.15));
    text_theme.title_small = Some(style(BOLD, 14.0, 20.0, 0.1));
    text_theme.body_large = Some(style(NORMAL, 16.0, 24.0, 0.15));
    text_theme.body_medium = Some(style(MEDIUM, 14.0, 20.0, 0.25));
    text_theme.body_small = Some(style(BOLD, 12.0, 16.0, 0.4));
    text_theme.label_large = Some(style(SEMI_BOLD, 14.0, 20.0, 0.1));
    text_theme.label_medium = Some(style(SEMI_BOLD, 12.0, 16.0, 0.5));
    text_theme.label_small = Some(style(SEMI_BOLD, 11.0, 16.0, 0.5));
    text_theme
}
