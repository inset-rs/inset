//! Flutter counterpart: `painting/text_style.dart`.

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug};

use reveal_embedder::lerp_double;
use reveal_embedder::{
    Color, FontFeature, FontStyle, FontVariation, FontWeight, K_TEXT_HEIGHT_NONE, Paint, Shadow,
    TextAlign, TextBaseline, TextDecoration, TextDecorationStyle, TextDirection,
    TextLeadingDistribution,
};
use reveal_embedder::{ParagraphStyle, TextStyle as UiTextStyle};

use crate::basic_types::RenderComparison;
use crate::text_painter::{K_DEFAULT_FONT_SIZE, TextOverflow};
use crate::text_scaler::TextScaler;

const K_DEFAULT_DEBUG_LABEL: &str = "unknown";

const K_COLOR_FOREGROUND_WARNING: &str = "Cannot provide both a color and a foreground\n\
The color argument is just a shorthand for \"foreground: Paint()..color = color\".";

const K_COLOR_BACKGROUND_WARNING: &str = "Cannot provide both a backgroundColor and a background\n\
The backgroundColor argument is just a shorthand for \"background: Paint()..color = color\".";

const K_TEXT_STYLE_HEIGHT_NAN_WARNING: &str = "TextStyle.height must not be NaN.";

/// An immutable style describing how to format and paint text.
#[derive(Clone)]
pub struct TextStyle {
    /// Whether null values in this [`TextStyle`] can be replaced with their
    /// value in another [`TextStyle`] using [`merge`](Self::merge).
    pub inherit: bool,
    /// The color to use when painting the text.
    ///
    /// If [`foreground`](Self::foreground) is specified, this value must be
    /// None. The color property is shorthand for `Paint()..color = color`.
    pub color: Option<Color>,
    /// The color to use as the background for the text.
    pub background_color: Option<Color>,
    /// The name of the font to use when painting the text (e.g., Roboto).
    ///
    /// If the font is defined in a package, this is prefixed with
    /// `packages/package_name/`.
    pub font_family: Option<String>,
    font_family_fallback: Option<Vec<String>>,
    package: Option<String>,
    /// The size of fonts (in logical pixels) to use when painting the text.
    pub font_size: Option<f64>,
    /// The typeface thickness to use when painting the text (e.g., bold).
    pub font_weight: Option<FontWeight>,
    /// The typeface variant to use when drawing the letters (e.g., italics).
    pub font_style: Option<FontStyle>,
    /// The amount of space (in logical pixels) to add between each letter.
    pub letter_spacing: Option<f64>,
    /// The amount of space (in logical pixels) to add at each sequence of
    /// white-space (i.e. between each word).
    pub word_spacing: Option<f64>,
    /// The common baseline that should be aligned between this text span and
    /// its parent text span, or, for the root text spans, with the line box.
    pub text_baseline: Option<TextBaseline>,
    /// The height of this text span, as a multiple of the font size.
    pub height: Option<f64>,
    /// How the vertical space added by the [`height`](Self::height) multiplier
    /// should be distributed over and under the text.
    pub leading_distribution: Option<TextLeadingDistribution>,
    /// The paint drawn as a foreground for the text.
    pub foreground: Option<Paint>,
    /// The paint drawn as a background for the text.
    pub background: Option<Paint>,
    /// The decorations to paint near the text (e.g., an underline).
    pub decoration: Option<TextDecoration>,
    /// The color in which to paint the text decorations.
    pub decoration_color: Option<Color>,
    /// The style in which to paint the text decorations (e.g., dashed).
    pub decoration_style: Option<TextDecorationStyle>,
    /// The thickness of the decoration stroke as a multiplier of the thickness
    /// defined by the font.
    pub decoration_thickness: Option<f64>,
    /// A human-readable description of this text style.
    ///
    /// This property is maintained only in debug builds. It is not considered
    /// when comparing text styles using `==` or [`compare_to`](Self::compare_to).
    pub debug_label: Option<String>,
    /// A list of [`Shadow`]s that will be painted underneath the text.
    pub shadows: Option<Vec<Shadow>>,
    /// A list of [`FontFeature`]s that affect how the font selects glyphs.
    pub font_features: Option<Vec<FontFeature>>,
    /// A list of [`FontVariation`]s that affect how a variable font is rendered.
    pub font_variations: Option<Vec<FontVariation>>,
    /// How visual text overflow should be handled.
    pub overflow: Option<TextOverflow>,
}

impl TextStyle {
    /// Creates a text style.
    ///
    /// Chain setters for the fields Dart's constructor takes as named arguments:
    /// `TextStyle::new().color(c).font_size(14.0)`.
    pub fn new() -> TextStyle {
        TextStyle {
            inherit: true,
            color: None,
            background_color: None,
            font_family: None,
            font_family_fallback: None,
            package: None,
            font_size: None,
            font_weight: None,
            font_style: None,
            letter_spacing: None,
            word_spacing: None,
            text_baseline: None,
            height: None,
            leading_distribution: None,
            foreground: None,
            background: None,
            decoration: None,
            decoration_color: None,
            decoration_style: None,
            decoration_thickness: None,
            debug_label: None,
            shadows: None,
            font_features: None,
            font_variations: None,
            overflow: None,
        }
    }

    /// Whether null values in this [`TextStyle`] can be replaced with their
    /// value in another [`TextStyle`] using [`merge`](Self::merge).
    pub fn inherit(mut self, inherit: bool) -> TextStyle {
        self.inherit = inherit;
        self
    }

    /// The color to use when painting the text.
    pub fn color(mut self, color: Color) -> TextStyle {
        debug_assert!(self.foreground.is_none(), "{K_COLOR_FOREGROUND_WARNING}");
        self.color = Some(color);
        self
    }

    /// The color to use as the background for the text.
    pub fn background_color(mut self, background_color: Color) -> TextStyle {
        debug_assert!(self.background.is_none(), "{K_COLOR_BACKGROUND_WARNING}");
        self.background_color = Some(background_color);
        self
    }

    /// The name of the font to use when painting the text (e.g., Roboto).
    pub fn font_family(mut self, font_family: impl Into<String>) -> TextStyle {
        let family = font_family.into();
        self.font_family = Some(match &self.package {
            Some(package) => format!("packages/{package}/{family}"),
            None => family,
        });
        self
    }

    /// The ordered list of font families to fall back on when a glyph cannot be
    /// found in a higher priority font family.
    pub fn font_family_fallback(mut self, font_family_fallback: Vec<String>) -> TextStyle {
        self.font_family_fallback = Some(font_family_fallback);
        self
    }

    /// The package argument must be provided if the font family is defined in a
    /// package. It is combined with [`font_family`](Self::font_family) to set
    /// that property.
    pub fn package(mut self, package: impl Into<String>) -> TextStyle {
        let package = package.into();
        if let Some(family) = self.font_family_unprefixed() {
            self.font_family = Some(format!("packages/{package}/{family}"));
        }
        self.package = Some(package);
        self
    }

    /// The size of fonts (in logical pixels) to use when painting the text.
    pub fn font_size(mut self, font_size: f64) -> TextStyle {
        self.font_size = Some(font_size);
        self
    }

    /// The typeface thickness to use when painting the text (e.g., bold).
    pub fn font_weight(mut self, font_weight: FontWeight) -> TextStyle {
        self.font_weight = Some(font_weight);
        self
    }

    /// The typeface variant to use when drawing the letters (e.g., italics).
    pub fn font_style(mut self, font_style: FontStyle) -> TextStyle {
        self.font_style = Some(font_style);
        self
    }

    /// The amount of space (in logical pixels) to add between each letter.
    pub fn letter_spacing(mut self, letter_spacing: f64) -> TextStyle {
        self.letter_spacing = Some(letter_spacing);
        self
    }

    /// The amount of space (in logical pixels) to add at each sequence of
    /// white-space (i.e. between each word).
    pub fn word_spacing(mut self, word_spacing: f64) -> TextStyle {
        self.word_spacing = Some(word_spacing);
        self
    }

    /// The common baseline that should be aligned between this text span and
    /// its parent text span.
    pub fn text_baseline(mut self, text_baseline: TextBaseline) -> TextStyle {
        self.text_baseline = Some(text_baseline);
        self
    }

    /// The height of this text span, as a multiple of the font size.
    pub fn height(mut self, height: f64) -> TextStyle {
        debug_assert!(!height.is_nan(), "{K_TEXT_STYLE_HEIGHT_NAN_WARNING}");
        self.height = Some(height);
        self
    }

    /// How the vertical space added by the [`height`](Self::height) multiplier
    /// should be distributed over and under the text.
    pub fn leading_distribution(
        mut self,
        leading_distribution: TextLeadingDistribution,
    ) -> TextStyle {
        self.leading_distribution = Some(leading_distribution);
        self
    }

    /// The paint drawn as a foreground for the text.
    pub fn foreground(mut self, foreground: Paint) -> TextStyle {
        debug_assert!(self.color.is_none(), "{K_COLOR_FOREGROUND_WARNING}");
        self.foreground = Some(foreground);
        self
    }

    /// The paint drawn as a background for the text.
    pub fn background(mut self, background: Paint) -> TextStyle {
        debug_assert!(
            self.background_color.is_none(),
            "{K_COLOR_BACKGROUND_WARNING}"
        );
        self.background = Some(background);
        self
    }

    /// The decorations to paint near the text (e.g., an underline).
    pub fn decoration(mut self, decoration: TextDecoration) -> TextStyle {
        self.decoration = Some(decoration);
        self
    }

    /// The color in which to paint the text decorations.
    pub fn decoration_color(mut self, decoration_color: Color) -> TextStyle {
        self.decoration_color = Some(decoration_color);
        self
    }

    /// The style in which to paint the text decorations (e.g., dashed).
    pub fn decoration_style(mut self, decoration_style: TextDecorationStyle) -> TextStyle {
        self.decoration_style = Some(decoration_style);
        self
    }

    /// The thickness of the decoration stroke as a multiplier of the thickness
    /// defined by the font.
    pub fn decoration_thickness(mut self, decoration_thickness: f64) -> TextStyle {
        self.decoration_thickness = Some(decoration_thickness);
        self
    }

    /// A human-readable description of this text style.
    pub fn debug_label(mut self, debug_label: impl Into<String>) -> TextStyle {
        self.debug_label = Some(debug_label.into());
        self
    }

    /// A list of [`Shadow`]s that will be painted underneath the text.
    pub fn shadows(mut self, shadows: Vec<Shadow>) -> TextStyle {
        self.shadows = Some(shadows);
        self
    }

    /// A list of [`FontFeature`]s that affect how the font selects glyphs.
    pub fn font_features(mut self, font_features: Vec<FontFeature>) -> TextStyle {
        self.font_features = Some(font_features);
        self
    }

    /// A list of [`FontVariation`]s that affect how a variable font is rendered.
    pub fn font_variations(mut self, font_variations: Vec<FontVariation>) -> TextStyle {
        self.font_variations = Some(font_variations);
        self
    }

    /// How visual text overflow should be handled.
    pub fn overflow(mut self, overflow: TextOverflow) -> TextStyle {
        self.overflow = Some(overflow);
        self
    }

    /// The ordered list of font families to fall back on when a glyph cannot be
    /// found in a higher priority font family.
    ///
    /// If the font is defined in a package, each font family in the list is
    /// prefixed with `packages/package_name/`.
    pub fn font_family_fallback_list(&self) -> Option<Vec<String>> {
        match (&self.package, &self.font_family_fallback) {
            (_, None) => None,
            (None, Some(list)) => Some(list.clone()),
            (Some(package), Some(list)) => Some(
                list.iter()
                    .map(|family| format!("packages/{package}/{family}"))
                    .collect(),
            ),
        }
    }

    fn font_family_unprefixed(&self) -> Option<String> {
        match (&self.package, &self.font_family) {
            (Some(package), Some(family)) => {
                let prefix = format!("packages/{package}/");
                debug_assert!(
                    family.starts_with(&prefix),
                    "fontFamily should start with {prefix}"
                );
                Some(family[prefix.len()..].to_string())
            }
            (None, family) => family.clone(),
            (Some(_), None) => None,
        }
    }

    /// Creates a copy of this text style but with the given fields replaced
    /// with the new values.
    ///
    /// Chain setters: `style.copy_with().color(c)`.
    pub fn copy_with(&self) -> TextStyle {
        let mut style = self.clone();
        if cfg!(debug_assertions) {
            if let Some(label) = &self.debug_label {
                style.debug_label = Some(format!("({label}).copyWith"));
            }
        } else {
            style.debug_label = None;
        }
        style
    }

    /// Creates a copy of this text style replacing or altering the specified
    /// properties.
    ///
    /// Chain replacements and factors, then [`TextStyleApply::into_style`].
    /// `style.apply()` with no further setters is Dart `apply()` with defaults.
    pub fn apply(&self) -> TextStyleApply {
        TextStyleApply::new(self)
    }

    /// Returns a new text style that is a combination of this style and the
    /// given `other` style.
    ///
    /// If `other` has [`inherit`](Self::inherit) set to true, its null
    /// properties are replaced with the non-null properties of this text style.
    /// If `other` has inherit set to false, returns `other` unchanged.
    pub fn merge(&self, other: Option<&TextStyle>) -> TextStyle {
        let Some(other) = other else {
            return self.clone();
        };
        if !other.inherit {
            return other.clone();
        }

        let mut merged = self.copy_with();
        merged.color = if self.foreground.is_none() && other.foreground.is_none() {
            other.color.or(self.color)
        } else {
            None
        };
        merged.background_color = if self.background.is_none() && other.background.is_none() {
            other.background_color.or(self.background_color)
        } else {
            None
        };
        merged.font_size = other.font_size.or(self.font_size);
        merged.font_weight = other.font_weight.or(self.font_weight);
        merged.font_style = other.font_style.or(self.font_style);
        merged.letter_spacing = other.letter_spacing.or(self.letter_spacing);
        merged.word_spacing = other.word_spacing.or(self.word_spacing);
        merged.text_baseline = other.text_baseline.or(self.text_baseline);
        merged.height = other.height.or(self.height);
        merged.leading_distribution = other.leading_distribution.or(self.leading_distribution);
        merged.foreground = other.foreground.clone().or_else(|| self.foreground.clone());
        merged.background = other.background.clone().or_else(|| self.background.clone());
        merged.shadows = other.shadows.clone().or_else(|| self.shadows.clone());
        merged.font_features = other
            .font_features
            .clone()
            .or_else(|| self.font_features.clone());
        merged.font_variations = other
            .font_variations
            .clone()
            .or_else(|| self.font_variations.clone());
        merged.decoration = other.decoration.or(self.decoration);
        merged.decoration_color = other.decoration_color.or(self.decoration_color);
        merged.decoration_style = other.decoration_style.or(self.decoration_style);
        merged.decoration_thickness = other.decoration_thickness.or(self.decoration_thickness);
        merged.overflow = other.overflow.or(self.overflow);
        merged.package = other.package.clone().or_else(|| self.package.clone());
        merged.font_family_fallback = other
            .font_family_fallback
            .clone()
            .or_else(|| self.font_family_fallback.clone());
        merged.font_family = other
            .font_family_unprefixed()
            .or_else(|| self.font_family_unprefixed());
        if let (Some(package), Some(family)) = (&merged.package, merged.font_family.clone()) {
            merged.font_family = Some(format!("packages/{package}/{family}"));
        }
        if cfg!(debug_assertions) && (other.debug_label.is_some() || self.debug_label.is_some()) {
            merged.debug_label = Some(format!(
                "({}).merge({})",
                self.debug_label.as_deref().unwrap_or(K_DEFAULT_DEBUG_LABEL),
                other
                    .debug_label
                    .as_deref()
                    .unwrap_or(K_DEFAULT_DEBUG_LABEL)
            ));
        }
        merged
    }

    /// Interpolate between two text styles for animated transitions.
    pub fn lerp(a: Option<&TextStyle>, b: Option<&TextStyle>, t: f64) -> Option<TextStyle> {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return Some(a.clone());
        }

        let mut lerp_debug_label = None;
        if cfg!(debug_assertions) {
            lerp_debug_label = Some(format!(
                "lerp({} ⎯{t:.1}→ {})",
                a.and_then(|s| s.debug_label.as_deref())
                    .unwrap_or(K_DEFAULT_DEBUG_LABEL),
                b.and_then(|s| s.debug_label.as_deref())
                    .unwrap_or(K_DEFAULT_DEBUG_LABEL)
            ));
        }

        if a.is_none() {
            let b = b.unwrap();
            let mut style = TextStyle::new().inherit(b.inherit);
            style.color = Color::lerp(None, b.color, t);
            style.background_color = Color::lerp(None, b.background_color, t);
            style.font_size = if t < 0.5 { None } else { b.font_size };
            style.font_weight = FontWeight::lerp(None, b.font_weight, t);
            style.font_style = if t < 0.5 { None } else { b.font_style };
            style.letter_spacing = if t < 0.5 { None } else { b.letter_spacing };
            style.word_spacing = if t < 0.5 { None } else { b.word_spacing };
            style.text_baseline = if t < 0.5 { None } else { b.text_baseline };
            style.height = if t < 0.5 { None } else { b.height };
            style.leading_distribution = if t < 0.5 {
                None
            } else {
                b.leading_distribution
            };
            style.foreground = if t < 0.5 { None } else { b.foreground.clone() };
            style.background = if t < 0.5 { None } else { b.background.clone() };
            style.shadows = if t < 0.5 { None } else { b.shadows.clone() };
            style.font_features = if t < 0.5 {
                None
            } else {
                b.font_features.clone()
            };
            style.font_variations = lerp_font_variations(None, b.font_variations.as_deref(), t);
            style.decoration = if t < 0.5 { None } else { b.decoration };
            style.decoration_color = Color::lerp(None, b.decoration_color, t);
            style.decoration_style = if t < 0.5 { None } else { b.decoration_style };
            style.decoration_thickness = if t < 0.5 {
                None
            } else {
                b.decoration_thickness
            };
            style.debug_label = lerp_debug_label;
            if t >= 0.5 {
                style.font_family = b.font_family_unprefixed();
                style.font_family_fallback = b.font_family_fallback.clone();
                style.package = b.package.clone();
                if let (Some(package), Some(family)) = (&style.package, style.font_family.clone()) {
                    style.font_family = Some(format!("packages/{package}/{family}"));
                }
                style.overflow = b.overflow;
            }
            return Some(style);
        }

        if b.is_none() {
            let a = a.unwrap();
            let mut style = TextStyle::new().inherit(a.inherit);
            style.color = Color::lerp(a.color, None, t);
            style.background_color = Color::lerp(None, a.background_color, t);
            style.font_size = if t < 0.5 { a.font_size } else { None };
            style.font_weight = FontWeight::lerp(a.font_weight, None, t);
            style.font_style = if t < 0.5 { a.font_style } else { None };
            style.letter_spacing = if t < 0.5 { a.letter_spacing } else { None };
            style.word_spacing = if t < 0.5 { a.word_spacing } else { None };
            style.text_baseline = if t < 0.5 { a.text_baseline } else { None };
            style.height = if t < 0.5 { a.height } else { None };
            style.leading_distribution = if t < 0.5 {
                a.leading_distribution
            } else {
                None
            };
            style.foreground = if t < 0.5 { a.foreground.clone() } else { None };
            style.background = if t < 0.5 { a.background.clone() } else { None };
            style.shadows = if t < 0.5 { a.shadows.clone() } else { None };
            style.font_features = if t < 0.5 {
                a.font_features.clone()
            } else {
                None
            };
            style.font_variations = lerp_font_variations(a.font_variations.as_deref(), None, t);
            style.decoration = if t < 0.5 { a.decoration } else { None };
            style.decoration_color = Color::lerp(a.decoration_color, None, t);
            style.decoration_style = if t < 0.5 { a.decoration_style } else { None };
            style.decoration_thickness = if t < 0.5 {
                a.decoration_thickness
            } else {
                None
            };
            style.debug_label = lerp_debug_label;
            if t < 0.5 {
                style.font_family = a.font_family_unprefixed();
                style.font_family_fallback = a.font_family_fallback.clone();
                style.package = a.package.clone();
                if let (Some(package), Some(family)) = (&style.package, style.font_family.clone()) {
                    style.font_family = Some(format!("packages/{package}/{family}"));
                }
                style.overflow = a.overflow;
            }
            return Some(style);
        }

        let a = a.unwrap();
        let b = b.unwrap();

        if cfg!(debug_assertions) && a.inherit != b.inherit {
            let mut null_fields = Vec::new();
            if a.foreground.is_none()
                && b.foreground.is_none()
                && a.color.is_none()
                && b.color.is_none()
            {
                null_fields.push("color");
            }
            if a.background.is_none()
                && b.background.is_none()
                && a.background_color.is_none()
                && b.background_color.is_none()
            {
                null_fields.push("backgroundColor");
            }
            if a.font_size.is_none() && b.font_size.is_none() {
                null_fields.push("fontSize");
            }
            if a.letter_spacing.is_none() && b.letter_spacing.is_none() {
                null_fields.push("letterSpacing");
            }
            if a.word_spacing.is_none() && b.word_spacing.is_none() {
                null_fields.push("wordSpacing");
            }
            if a.height.is_none() && b.height.is_none() {
                null_fields.push("height");
            }
            if a.decoration_color.is_none() && b.decoration_color.is_none() {
                null_fields.push("decorationColor");
            }
            if a.decoration_thickness.is_none() && b.decoration_thickness.is_none() {
                null_fields.push("decorationThickness");
            }
            if !null_fields.is_empty() {
                panic!(
                    "Failed to interpolate TextStyles with different inherit values. \
                     The following fields are unspecified in both TextStyles: {}.",
                    null_fields
                        .iter()
                        .map(|name| format!("\"{name}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }

        let mut style = TextStyle::new().inherit(if t < 0.5 { a.inherit } else { b.inherit });
        style.color = if a.foreground.is_none() && b.foreground.is_none() {
            Color::lerp(a.color, b.color, t)
        } else {
            None
        };
        style.background_color = if a.background.is_none() && b.background.is_none() {
            Color::lerp(a.background_color, b.background_color, t)
        } else {
            None
        };
        style.font_size = lerp_double(a.font_size.or(b.font_size), b.font_size.or(a.font_size), t);
        style.font_weight = FontWeight::lerp(a.font_weight, b.font_weight, t);
        style.font_style = if t < 0.5 { a.font_style } else { b.font_style };
        style.letter_spacing = lerp_double(
            a.letter_spacing.or(b.letter_spacing),
            b.letter_spacing.or(a.letter_spacing),
            t,
        );
        style.word_spacing = lerp_double(
            a.word_spacing.or(b.word_spacing),
            b.word_spacing.or(a.word_spacing),
            t,
        );
        style.text_baseline = if t < 0.5 {
            a.text_baseline
        } else {
            b.text_baseline
        };
        style.height = lerp_double(a.height.or(b.height), b.height.or(a.height), t);
        style.leading_distribution = if t < 0.5 {
            a.leading_distribution
        } else {
            b.leading_distribution
        };
        style.foreground = if a.foreground.is_some() || b.foreground.is_some() {
            if t < 0.5 {
                Some(a.foreground.clone().unwrap_or_else(|| Paint {
                    color: a.color.unwrap().into(),
                    ..Paint::default()
                }))
            } else {
                Some(b.foreground.clone().unwrap_or_else(|| Paint {
                    color: b.color.unwrap().into(),
                    ..Paint::default()
                }))
            }
        } else {
            None
        };
        style.background = if a.background.is_some() || b.background.is_some() {
            if t < 0.5 {
                Some(a.background.clone().unwrap_or_else(|| Paint {
                    color: a.background_color.unwrap().into(),
                    ..Paint::default()
                }))
            } else {
                Some(b.background.clone().unwrap_or_else(|| Paint {
                    color: b.background_color.unwrap().into(),
                    ..Paint::default()
                }))
            }
        } else {
            None
        };
        style.shadows = Shadow::lerp_list(a.shadows.as_deref(), b.shadows.as_deref(), t);
        style.font_features = if t < 0.5 {
            a.font_features.clone()
        } else {
            b.font_features.clone()
        };
        style.font_variations = lerp_font_variations(
            a.font_variations.as_deref(),
            b.font_variations.as_deref(),
            t,
        );
        style.decoration = if t < 0.5 { a.decoration } else { b.decoration };
        style.decoration_color = Color::lerp(a.decoration_color, b.decoration_color, t);
        style.decoration_style = if t < 0.5 {
            a.decoration_style
        } else {
            b.decoration_style
        };
        style.decoration_thickness = lerp_double(
            a.decoration_thickness.or(b.decoration_thickness),
            b.decoration_thickness.or(a.decoration_thickness),
            t,
        );
        style.debug_label = lerp_debug_label;
        let src = if t < 0.5 { a } else { b };
        style.font_family = src.font_family_unprefixed();
        style.font_family_fallback = src.font_family_fallback.clone();
        style.package = src.package.clone();
        if let (Some(package), Some(family)) = (&style.package, style.font_family.clone()) {
            style.font_family = Some(format!("packages/{package}/{family}"));
        }
        style.overflow = src.overflow;
        Some(style)
    }

    /// The style information for text runs, as a valo [`TextStyle`](UiTextStyle).
    pub fn get_text_style(&self) -> UiTextStyle {
        self.get_text_style_with(&TextScaler::NO_SCALING)
    }

    /// The style information for text runs, scaled by `text_scaler`.
    pub fn get_text_style_with(&self, text_scaler: &TextScaler) -> UiTextStyle {
        let font_size = self.font_size.map(|size| text_scaler.scale(size));
        let mut style = UiTextStyle::default();
        let mut families = Vec::new();
        if let Some(family) = &self.font_family {
            families.push(family.clone());
        }
        if let Some(fallback) = self.font_family_fallback_list() {
            families.extend(fallback);
        }
        style.families = families;
        if let Some(weight) = self.font_weight {
            style.weight = weight.value as u16;
        }
        style.italic = self.font_style == Some(FontStyle::Italic);
        if let Some(size) = font_size {
            style.size = size as f32;
        }
        if let Some(foreground) = &self.foreground {
            style.color = foreground.color;
        } else if let Some(color) = self.color {
            style.color = color.into();
        }
        if let Some(letter_spacing) = self.letter_spacing {
            style.letter_spacing = letter_spacing as f32;
        }
        if let Some(word_spacing) = self.word_spacing {
            style.word_spacing = word_spacing as f32;
        }
        style.height = match self.height {
            None => None,
            Some(height) if height == K_TEXT_HEIGHT_NONE => None,
            Some(height) => Some(height as f32),
        };
        style.decoration = valo_decoration(
            self.decoration,
            self.decoration_color,
            self.decoration_thickness,
        );
        if let Some(shadows) = &self.shadows {
            style.shadows = shadows
                .iter()
                .map(|shadow| reveal_embedder::valo::Shadow {
                    color: shadow.color.into(),
                    offset: shadow.offset.into(),
                    blur: shadow.blur_sigma() as f32,
                })
                .collect();
        }
        style
    }

    /// The style information for paragraphs, as a valo [`ParagraphStyle`].
    ///
    /// Chain optional Dart named arguments, then [`GetParagraphStyle::build`].
    pub fn get_paragraph_style(&self) -> GetParagraphStyle<'_> {
        GetParagraphStyle {
            style: self,
            text_align: None,
            text_direction: None,
            text_scaler: TextScaler::NO_SCALING,
            ellipsis: None,
            max_lines: None,
        }
    }

    /// Describe the difference between this style and another, in terms of how
    /// much damage it will make to the rendering.
    pub fn compare_to(&self, other: &TextStyle) -> RenderComparison {
        if std::ptr::eq(self, other) {
            return RenderComparison::Identical;
        }
        if self.inherit != other.inherit
            || self.font_family != other.font_family
            || self.font_size != other.font_size
            || self.font_weight != other.font_weight
            || self.font_style != other.font_style
            || self.letter_spacing != other.letter_spacing
            || self.word_spacing != other.word_spacing
            || self.text_baseline != other.text_baseline
            || self.height != other.height
            || self.leading_distribution != other.leading_distribution
            || self.foreground != other.foreground
            || self.background != other.background
            || self.shadows != other.shadows
            || self.font_features != other.font_features
            || self.font_variations != other.font_variations
            || self.font_family_fallback_list() != other.font_family_fallback_list()
            || self.overflow != other.overflow
        {
            return RenderComparison::Layout;
        }
        if self.color != other.color
            || self.background_color != other.background_color
            || self.decoration != other.decoration
            || self.decoration_color != other.decoration_color
            || self.decoration_style != other.decoration_style
            || self.decoration_thickness != other.decoration_thickness
        {
            return RenderComparison::Paint;
        }
        RenderComparison::Identical
    }
}

impl Default for TextStyle {
    fn default() -> TextStyle {
        TextStyle::new()
    }
}

impl PartialEq for TextStyle {
    fn eq(&self, other: &TextStyle) -> bool {
        self.inherit == other.inherit
            && self.color == other.color
            && self.background_color == other.background_color
            && self.font_size == other.font_size
            && self.font_weight == other.font_weight
            && self.font_style == other.font_style
            && self.letter_spacing == other.letter_spacing
            && self.word_spacing == other.word_spacing
            && self.text_baseline == other.text_baseline
            && self.height == other.height
            && self.leading_distribution == other.leading_distribution
            && self.foreground == other.foreground
            && self.background == other.background
            && self.shadows == other.shadows
            && self.font_features == other.font_features
            && self.font_variations == other.font_variations
            && self.decoration == other.decoration
            && self.decoration_color == other.decoration_color
            && self.decoration_style == other.decoration_style
            && self.decoration_thickness == other.decoration_thickness
            && self.font_family == other.font_family
            && self.font_family_fallback_list() == other.font_family_fallback_list()
            && self.package == other.package
            && self.overflow == other.overflow
    }
}

impl Debug for TextStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("TextStyle");
        s.field("inherit", &self.inherit)
            .field("color", &self.color)
            .field("background_color", &self.background_color)
            .field("family", &self.font_family)
            .field("size", &self.font_size)
            .field("weight", &self.font_weight)
            .field("style", &self.font_style)
            .field("letter_spacing", &self.letter_spacing)
            .field("word_spacing", &self.word_spacing)
            .field("baseline", &self.text_baseline)
            .field("height", &self.height)
            .field("leading_distribution", &self.leading_distribution)
            .field("foreground", &self.foreground)
            .field("background", &self.background)
            .field("decoration", &self.decoration)
            .field("decoration_color", &self.decoration_color)
            .field("decoration_style", &self.decoration_style)
            .field("decoration_thickness", &self.decoration_thickness)
            .field("overflow", &self.overflow);
        if cfg!(debug_assertions) {
            s.field("debug_label", &self.debug_label);
        }
        s.finish()
    }
}

/// Dart `TextStyle.apply` named optionals.
pub struct TextStyleApply {
    style: TextStyle,
    color: Option<Color>,
    background_color: Option<Color>,
    decoration: Option<TextDecoration>,
    decoration_color: Option<Color>,
    decoration_style: Option<TextDecorationStyle>,
    decoration_thickness_factor: f64,
    decoration_thickness_delta: f64,
    font_family: Option<String>,
    font_family_fallback: Option<Vec<String>>,
    font_size_factor: f64,
    font_size_delta: f64,
    font_weight_delta: i32,
    font_style: Option<FontStyle>,
    letter_spacing_factor: f64,
    letter_spacing_delta: f64,
    word_spacing_factor: f64,
    word_spacing_delta: f64,
    height_factor: f64,
    height_delta: f64,
    text_baseline: Option<TextBaseline>,
    leading_distribution: Option<TextLeadingDistribution>,
    shadows: Option<Vec<Shadow>>,
    font_features: Option<Vec<FontFeature>>,
    font_variations: Option<Vec<FontVariation>>,
    package: Option<String>,
    overflow: Option<TextOverflow>,
}

impl TextStyleApply {
    fn new(style: &TextStyle) -> TextStyleApply {
        TextStyleApply {
            style: style.clone(),
            color: None,
            background_color: None,
            decoration: None,
            decoration_color: None,
            decoration_style: None,
            decoration_thickness_factor: 1.0,
            decoration_thickness_delta: 0.0,
            font_family: None,
            font_family_fallback: None,
            font_size_factor: 1.0,
            font_size_delta: 0.0,
            font_weight_delta: 0,
            font_style: None,
            letter_spacing_factor: 1.0,
            letter_spacing_delta: 0.0,
            word_spacing_factor: 1.0,
            word_spacing_delta: 0.0,
            height_factor: 1.0,
            height_delta: 0.0,
            text_baseline: None,
            leading_distribution: None,
            shadows: None,
            font_features: None,
            font_variations: None,
            package: None,
            overflow: None,
        }
    }

    pub fn color(mut self, color: Color) -> TextStyleApply {
        self.color = Some(color);
        self
    }

    pub fn background_color(mut self, background_color: Color) -> TextStyleApply {
        self.background_color = Some(background_color);
        self
    }

    pub fn decoration(mut self, decoration: TextDecoration) -> TextStyleApply {
        self.decoration = Some(decoration);
        self
    }

    pub fn decoration_color(mut self, decoration_color: Color) -> TextStyleApply {
        self.decoration_color = Some(decoration_color);
        self
    }

    pub fn decoration_style(mut self, decoration_style: TextDecorationStyle) -> TextStyleApply {
        self.decoration_style = Some(decoration_style);
        self
    }

    pub fn decoration_thickness_factor(
        mut self,
        decoration_thickness_factor: f64,
    ) -> TextStyleApply {
        self.decoration_thickness_factor = decoration_thickness_factor;
        self
    }

    pub fn decoration_thickness_delta(mut self, decoration_thickness_delta: f64) -> TextStyleApply {
        self.decoration_thickness_delta = decoration_thickness_delta;
        self
    }

    pub fn font_family(mut self, font_family: impl Into<String>) -> TextStyleApply {
        self.font_family = Some(font_family.into());
        self
    }

    pub fn font_family_fallback(mut self, font_family_fallback: Vec<String>) -> TextStyleApply {
        self.font_family_fallback = Some(font_family_fallback);
        self
    }

    pub fn font_size_factor(mut self, font_size_factor: f64) -> TextStyleApply {
        self.font_size_factor = font_size_factor;
        self
    }

    pub fn font_size_delta(mut self, font_size_delta: f64) -> TextStyleApply {
        self.font_size_delta = font_size_delta;
        self
    }

    pub fn font_weight_delta(mut self, font_weight_delta: i32) -> TextStyleApply {
        self.font_weight_delta = font_weight_delta;
        self
    }

    pub fn font_style(mut self, font_style: FontStyle) -> TextStyleApply {
        self.font_style = Some(font_style);
        self
    }

    pub fn letter_spacing_factor(mut self, letter_spacing_factor: f64) -> TextStyleApply {
        self.letter_spacing_factor = letter_spacing_factor;
        self
    }

    pub fn letter_spacing_delta(mut self, letter_spacing_delta: f64) -> TextStyleApply {
        self.letter_spacing_delta = letter_spacing_delta;
        self
    }

    pub fn word_spacing_factor(mut self, word_spacing_factor: f64) -> TextStyleApply {
        self.word_spacing_factor = word_spacing_factor;
        self
    }

    pub fn word_spacing_delta(mut self, word_spacing_delta: f64) -> TextStyleApply {
        self.word_spacing_delta = word_spacing_delta;
        self
    }

    pub fn height_factor(mut self, height_factor: f64) -> TextStyleApply {
        self.height_factor = height_factor;
        self
    }

    pub fn height_delta(mut self, height_delta: f64) -> TextStyleApply {
        self.height_delta = height_delta;
        self
    }

    pub fn text_baseline(mut self, text_baseline: TextBaseline) -> TextStyleApply {
        self.text_baseline = Some(text_baseline);
        self
    }

    pub fn leading_distribution(
        mut self,
        leading_distribution: TextLeadingDistribution,
    ) -> TextStyleApply {
        self.leading_distribution = Some(leading_distribution);
        self
    }

    pub fn shadows(mut self, shadows: Vec<Shadow>) -> TextStyleApply {
        self.shadows = Some(shadows);
        self
    }

    pub fn font_features(mut self, font_features: Vec<FontFeature>) -> TextStyleApply {
        self.font_features = Some(font_features);
        self
    }

    pub fn font_variations(mut self, font_variations: Vec<FontVariation>) -> TextStyleApply {
        self.font_variations = Some(font_variations);
        self
    }

    pub fn package(mut self, package: impl Into<String>) -> TextStyleApply {
        self.package = Some(package.into());
        self
    }

    pub fn overflow(mut self, overflow: TextOverflow) -> TextStyleApply {
        self.overflow = Some(overflow);
        self
    }

    /// Dart `apply` returns a [`TextStyle`].
    pub fn into_style(self) -> TextStyle {
        let TextStyleApply {
            style,
            color,
            background_color,
            decoration,
            decoration_color,
            decoration_style,
            decoration_thickness_factor,
            decoration_thickness_delta,
            font_family,
            font_family_fallback,
            font_size_factor,
            font_size_delta,
            font_weight_delta,
            font_style,
            letter_spacing_factor,
            letter_spacing_delta,
            word_spacing_factor,
            word_spacing_delta,
            height_factor,
            height_delta,
            text_baseline,
            leading_distribution,
            shadows,
            font_features,
            font_variations,
            package,
            overflow,
        } = self;

        debug_assert!(
            style.font_size.is_some() || (font_size_factor == 1.0 && font_size_delta == 0.0)
        );
        debug_assert!(style.font_weight.is_some() || font_weight_delta == 0);
        debug_assert!(
            style.letter_spacing.is_some()
                || (letter_spacing_factor == 1.0 && letter_spacing_delta == 0.0)
        );
        debug_assert!(
            style.word_spacing.is_some()
                || (word_spacing_factor == 1.0 && word_spacing_delta == 0.0)
        );
        debug_assert!(
            style.decoration_thickness.is_some()
                || (decoration_thickness_factor == 1.0 && decoration_thickness_delta == 0.0)
        );

        let mut result = TextStyle::new().inherit(style.inherit);
        result.color = if style.foreground.is_none() {
            color.or(style.color)
        } else {
            None
        };
        result.background_color = if style.background.is_none() {
            background_color.or(style.background_color)
        } else {
            None
        };
        result.font_family = font_family.or_else(|| style.font_family_unprefixed());
        result.font_family_fallback = font_family_fallback.or(style.font_family_fallback);
        result.package = package.or(style.package);
        if let (Some(package), Some(family)) = (&result.package, result.font_family.clone()) {
            result.font_family = Some(format!("packages/{package}/{family}"));
        }
        result.font_size = style
            .font_size
            .map(|font_size| font_size * font_size_factor + font_size_delta);
        result.font_weight = style.font_weight.map(|font_weight| {
            let index = (font_weight.index() + font_weight_delta).clamp(0, 8) as usize;
            FontWeight::VALUES[index]
        });
        result.font_style = font_style.or(style.font_style);
        result.letter_spacing = style
            .letter_spacing
            .map(|letter_spacing| letter_spacing * letter_spacing_factor + letter_spacing_delta);
        result.word_spacing = style
            .word_spacing
            .map(|word_spacing| word_spacing * word_spacing_factor + word_spacing_delta);
        result.text_baseline = text_baseline.or(style.text_baseline);
        result.height = match style.height {
            None => None,
            Some(height) if height == K_TEXT_HEIGHT_NONE => Some(height),
            Some(height) => Some(height * height_factor + height_delta),
        };
        result.leading_distribution = leading_distribution.or(style.leading_distribution);
        result.foreground = style.foreground;
        result.background = style.background;
        result.shadows = shadows.or(style.shadows);
        result.font_features = font_features.or(style.font_features);
        result.font_variations = font_variations.or(style.font_variations);
        result.decoration = decoration.or(style.decoration);
        result.decoration_color = decoration_color.or(style.decoration_color);
        result.decoration_style = decoration_style.or(style.decoration_style);
        result.decoration_thickness = style.decoration_thickness.map(|decoration_thickness| {
            decoration_thickness * decoration_thickness_factor + decoration_thickness_delta
        });
        result.overflow = overflow.or(style.overflow);
        if cfg!(debug_assertions)
            && let Some(label) = &style.debug_label
        {
            result.debug_label = Some(format!("({label}).apply"));
        }
        result
    }
}

impl From<TextStyleApply> for TextStyle {
    fn from(apply: TextStyleApply) -> TextStyle {
        apply.into_style()
    }
}

/// Dart `TextStyle.getParagraphStyle` named optionals.
pub struct GetParagraphStyle<'a> {
    style: &'a TextStyle,
    text_align: Option<TextAlign>,
    text_direction: Option<TextDirection>,
    text_scaler: TextScaler,
    ellipsis: Option<String>,
    max_lines: Option<i32>,
}

impl GetParagraphStyle<'_> {
    pub fn text_align(mut self, text_align: TextAlign) -> Self {
        self.text_align = Some(text_align);
        self
    }

    pub fn text_direction(mut self, text_direction: TextDirection) -> Self {
        self.text_direction = Some(text_direction);
        self
    }

    pub fn text_scaler(mut self, text_scaler: TextScaler) -> Self {
        self.text_scaler = text_scaler;
        self
    }

    pub fn ellipsis(mut self, ellipsis: impl Into<String>) -> Self {
        self.ellipsis = Some(ellipsis.into());
        self
    }

    pub fn max_lines(mut self, max_lines: i32) -> Self {
        debug_assert!(max_lines > 0);
        self.max_lines = Some(max_lines);
        self
    }

    /// Dart `getParagraphStyle` returns a `ui.ParagraphStyle`.
    pub fn build(self) -> ParagraphStyle {
        debug_assert!(
            self.style.height.is_none_or(|height| !height.is_nan()),
            "{K_TEXT_STYLE_HEIGHT_NAN_WARNING}"
        );
        let _ = self
            .text_scaler
            .scale(self.style.font_size.unwrap_or(K_DEFAULT_FONT_SIZE));
        ParagraphStyle {
            align: valo_align(self.text_align, self.text_direction),
            direction: self.text_direction.map(valo_direction),
            preserve_trailing_whitespace: false,
            max_lines: self.max_lines.map(|max_lines| max_lines as u32),
            ellipsis: self.ellipsis,
        }
    }
}

/// Interpolate between two lists of [`FontVariation`] objects.
///
/// Variations are paired by axis, and interpolated using [`FontVariation::lerp`].
pub fn lerp_font_variations(
    a: Option<&[FontVariation]>,
    b: Option<&[FontVariation]>,
    t: f64,
) -> Option<Vec<FontVariation>> {
    if t == 0.0 {
        return a.map(<[FontVariation]>::to_vec);
    }
    if t == 1.0 {
        return b.map(<[FontVariation]>::to_vec);
    }
    if a.is_none_or(<[FontVariation]>::is_empty) || b.is_none_or(<[FontVariation]>::is_empty) {
        return if t < 0.5 {
            a.map(<[FontVariation]>::to_vec)
        } else {
            b.map(<[FontVariation]>::to_vec)
        };
    }
    let a = a.unwrap();
    let b = b.unwrap();
    let mut result = Vec::new();
    let mut index = 0;
    let min_length = a.len().min(b.len());
    while index < min_length {
        if a[index].axis != b[index].axis {
            break;
        }
        result.push(FontVariation::lerp(Some(&a[index]), Some(&b[index]), t).unwrap());
        index += 1;
    }
    let max_length = a.len().max(b.len());
    if index < max_length {
        let mut axes = HashSet::new();
        let mut a_variations = HashMap::new();
        for variation in &a[index..] {
            a_variations.insert(variation.axis.clone(), variation);
            axes.insert(variation.axis.clone());
        }
        let mut b_variations = HashMap::new();
        for variation in &b[index..] {
            b_variations.insert(variation.axis.clone(), variation);
            axes.insert(variation.axis.clone());
        }
        for axis in axes {
            if let Some(variation) = FontVariation::lerp(
                a_variations.get(&axis).copied(),
                b_variations.get(&axis).copied(),
                t,
            ) {
                result.push(variation);
            }
        }
    }
    Some(result)
}

fn valo_decoration(
    decoration: Option<TextDecoration>,
    color: Option<Color>,
    thickness: Option<f64>,
) -> Option<reveal_embedder::valo::Decoration> {
    let decoration = decoration?;
    if decoration == TextDecoration::NONE {
        return None;
    }
    let kind = if decoration.contains(TextDecoration::UNDERLINE) {
        reveal_embedder::valo::DecorationKind::Underline
    } else if decoration.contains(TextDecoration::OVERLINE) {
        reveal_embedder::valo::DecorationKind::Overline
    } else if decoration.contains(TextDecoration::LINE_THROUGH) {
        reveal_embedder::valo::DecorationKind::LineThrough
    } else {
        return None;
    };
    Some(reveal_embedder::valo::Decoration {
        kind,
        color: color.map(Into::into),
        thickness: thickness.unwrap_or(1.0) as f32,
    })
}

fn valo_align(
    align: Option<TextAlign>,
    direction: Option<TextDirection>,
) -> reveal_embedder::valo::TextAlign {
    match align {
        None | Some(TextAlign::Left) => reveal_embedder::valo::TextAlign::Left,
        Some(TextAlign::Right) => reveal_embedder::valo::TextAlign::Right,
        Some(TextAlign::Center) => reveal_embedder::valo::TextAlign::Center,
        Some(TextAlign::Justify) => reveal_embedder::valo::TextAlign::Justify,
        Some(TextAlign::Start) => match direction {
            Some(TextDirection::Rtl) => reveal_embedder::valo::TextAlign::Right,
            _ => reveal_embedder::valo::TextAlign::Left,
        },
        Some(TextAlign::End) => match direction {
            Some(TextDirection::Rtl) => reveal_embedder::valo::TextAlign::Left,
            _ => reveal_embedder::valo::TextAlign::Right,
        },
    }
}

fn valo_direction(direction: TextDirection) -> reveal_embedder::valo::TextDirection {
    match direction {
        TextDirection::Ltr => reveal_embedder::valo::TextDirection::Ltr,
        TextDirection::Rtl => reveal_embedder::valo::TextDirection::Rtl,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_with_replaces_color() {
        let style = TextStyle::new().color(Color::from_argb(255, 0, 0, 0));
        let copied = style.copy_with().color(Color::from_argb(255, 255, 0, 0));
        assert_eq!(copied.color, Some(Color::from_argb(255, 255, 0, 0)));
        assert_eq!(style.color, Some(Color::from_argb(255, 0, 0, 0)));
    }

    #[test]
    fn merge_fills_nulls_when_other_inherits() {
        let base = TextStyle::new()
            .color(Color::from_argb(255, 0, 0, 0))
            .font_size(14.0);
        let other = TextStyle::new().font_weight(FontWeight::BOLD);
        let merged = base.merge(Some(&other));
        assert_eq!(merged.color, Some(Color::from_argb(255, 0, 0, 0)));
        assert_eq!(merged.font_size, Some(14.0));
        assert_eq!(merged.font_weight, Some(FontWeight::BOLD));
    }

    #[test]
    fn merge_returns_other_when_it_does_not_inherit() {
        let base = TextStyle::new().color(Color::from_argb(255, 0, 0, 0));
        let other = TextStyle::new().inherit(false).font_size(20.0);
        let merged = base.merge(Some(&other));
        assert!(merged.color.is_none());
        assert_eq!(merged.font_size, Some(20.0));
        assert!(!merged.inherit);
    }

    #[test]
    fn get_text_style_maps_weight_and_size() {
        let style = TextStyle::new()
            .font_size(18.0)
            .font_weight(FontWeight::W700)
            .font_style(FontStyle::Italic)
            .color(Color::from_argb(255, 255, 0, 0));
        let ui = style.get_text_style();
        assert_eq!(ui.size, 18.0);
        assert_eq!(ui.weight, 700);
        assert!(ui.italic);
    }

    #[test]
    fn apply_font_size_factor() {
        let style = TextStyle::new().font_size(10.0);
        let applied = style
            .apply()
            .font_size_factor(2.0)
            .font_size_delta(1.0)
            .into_style();
        assert_eq!(applied.font_size, Some(21.0));
    }

    #[test]
    fn package_prefixes_font_family() {
        let style = TextStyle::new().font_family("Roboto").package("cool_fonts");
        assert_eq!(
            style.font_family.as_deref(),
            Some("packages/cool_fonts/Roboto")
        );
    }
}
