//! WinUI's type ramp: the `*TextBlockStyle` styles of `TextBlock_themeresources.xaml` (Fluent 2 sizes) as reveal text styles in the bundled Selawik, Microsoft's open metric-compatible stand-in for Segoe UI Variable.

use super::generated::*;
use inset_embedder::{Color, FontWeight};
use inset_painting::TextStyle;

/// The family name an app registers Selawik under, the kit's stand-in for `XamlAutoFontFamily` (Segoe UI Variable on Windows) and the default `content_control_theme_font_family`.
pub const FONT_FAMILY: &str = "Selawik";

/// One of WinUI's text block styles; XAML `{StaticResource BodyTextBlockStyle}` and its siblings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextBlockStyle {
    Caption,
    Body,
    BodyStrong,
    BodyLarge,
    BodyLargeStrong,
    Subtitle,
    Title,
    TitleLarge,
    Display,
}

impl TextBlockStyle {
    /// The style's size and weight from `TextBlock_themeresources.xaml`: every style is based on `BaseTextBlockStyle` (SemiBold) and the body and caption styles set `Normal`.
    pub fn font(self, r: &ThemeResources) -> (f64, FontWeight) {
        match self {
            TextBlockStyle::Caption => (r.caption_text_block_font_size, FontWeight::W400),
            TextBlockStyle::Body => (r.body_text_block_font_size, FontWeight::W400),
            TextBlockStyle::BodyStrong => (r.body_text_block_font_size, FontWeight::W600),
            TextBlockStyle::BodyLarge => (r.body_large_text_block_font_size, FontWeight::W400),
            TextBlockStyle::BodyLargeStrong => {
                (r.body_large_text_block_font_size, FontWeight::W600)
            }
            TextBlockStyle::Subtitle => (r.subtitle_text_block_font_size, FontWeight::W600),
            TextBlockStyle::Title => (r.title_text_block_font_size, FontWeight::W600),
            TextBlockStyle::TitleLarge => (r.title_large_text_block_font_size, FontWeight::W600),
            TextBlockStyle::Display => (r.display_text_block_font_size, FontWeight::W600),
        }
    }

    /// The reveal text style in `color` (a `TextBlock`'s `Foreground`).
    pub fn text_style(self, r: &ThemeResources, color: Color) -> TextStyle {
        let (size, weight) = self.font(r);
        control_text_style(r, size, weight, color)
    }
}

/// A text style in the control font, `ContentControlThemeFontFamily`, at `size` and `weight`.
pub fn control_text_style(
    r: &ThemeResources,
    size: f64,
    weight: FontWeight,
    color: Color,
) -> TextStyle {
    TextStyle::new()
        .font_family(r.content_control_theme_font_family)
        .font_size(size)
        .font_weight(weight)
        .color(color)
}
