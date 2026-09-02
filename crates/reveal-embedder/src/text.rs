//! Flutter counterpart: `engine/src/flutter/lib/ui/text.dart` value types
//! used by painting (`FontWeight`, `TextDecoration`, `TextDirection`, …).
//! `ui.TextStyle` / `ParagraphStyle` are valo types; there is no engine encode.

use std::fmt::{self, Debug, Display};

use crate::{clamp_double, lerp_double, lerp_int};

/// A [`TextStyle.height`](https://api.flutter.dev/flutter/dart-ui/TextStyle/height.html)
/// value that indicates the text span should take the height defined by the
/// font, which may not be exactly the height of `fontSize`.
pub const K_TEXT_HEIGHT_NONE: f64 = 0.0;

/// Whether to use the italic type variation of glyphs in the font.
///
/// Some modern fonts allow this to be selected in a more fine-grained manner.
/// See `FontVariation.italic` for details.
///
/// Italic type is distinct from slanted glyphs. To control the slant of a
/// glyph, consider the `FontVariation.slant` font feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontStyle {
    /// Use the upright ("Roman") glyphs.
    Normal,

    /// Use glyphs that have a more pronounced angle and typically a cursive style
    /// ("italic type").
    Italic,
}

/// The thickness of the glyphs used to draw the text.
///
/// Values must be in the range 1..1000.
///
/// Fonts are typically weighted on a 9-point scale, which, for historical
/// reasons, uses the names 100 to 900. In Flutter, these are named `w100` to
/// `w900` and have the following conventional meanings:
///
///  * [`w100`](FontWeight::W100): Thin, the thinnest font weight.
///
///  * [`w200`](FontWeight::W200): Extra light.
///
///  * [`w300`](FontWeight::W300): Light.
///
///  * [`w400`](FontWeight::W400): Normal. The constant [`NORMAL`](FontWeight::NORMAL) is an alias for this value.
///
///  * [`w500`](FontWeight::W500): Medium.
///
///  * [`w600`](FontWeight::W600): Semi-bold.
///
///  * [`w700`](FontWeight::W700): Bold. The constant [`BOLD`](FontWeight::BOLD) is an alias for this value.
///
///  * [`w800`](FontWeight::W800): Extra-bold.
///
///  * [`w900`](FontWeight::W900): Black, the thickest font weight.
///
/// For example, the font named "Roboto Medium" is typically exposed as a font
/// with the name "Roboto" and the weight [`W500`](FontWeight::W500).
///
/// Some modern fonts allow the weight to be adjusted in arbitrary increments.
/// When using these fonts, applications can specify [`FontWeight`] instances
/// constructed using values other than the predefined values.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontWeight {
    /// The thickness value of this font weight.
    pub value: i32,
}

impl FontWeight {
    /// Creates a [`FontWeight`] object, which can be added to a text style to
    /// select the thickness of a font's glyphs.
    pub fn new(value: i32) -> FontWeight {
        debug_assert!(
            (1..=1000).contains(&value),
            "Font weight must be between 1 and 1000"
        );
        FontWeight { value }
    }

    /// The encoded integer value of this font weight.
    ///
    /// Deprecated in Dart in favor of [`value`](Self::value).
    pub fn index(self) -> i32 {
        ((self.value / 100) - 1).clamp(0, 8)
    }

    /// Thin, the least thick.
    pub const W100: FontWeight = FontWeight { value: 100 };

    /// Extra-light.
    pub const W200: FontWeight = FontWeight { value: 200 };

    /// Light.
    pub const W300: FontWeight = FontWeight { value: 300 };

    /// Normal / regular / plain.
    pub const W400: FontWeight = FontWeight { value: 400 };

    /// Medium.
    pub const W500: FontWeight = FontWeight { value: 500 };

    /// Semi-bold.
    pub const W600: FontWeight = FontWeight { value: 600 };

    /// Bold.
    pub const W700: FontWeight = FontWeight { value: 700 };

    /// Extra-bold.
    pub const W800: FontWeight = FontWeight { value: 800 };

    /// Black, the most thick.
    pub const W900: FontWeight = FontWeight { value: 900 };

    /// The default font weight.
    pub const NORMAL: FontWeight = FontWeight::W400;

    /// A commonly used font weight that is heavier than normal.
    pub const BOLD: FontWeight = FontWeight::W700;

    /// A list of all the named font weights.
    pub const VALUES: [FontWeight; 9] = [
        FontWeight::W100,
        FontWeight::W200,
        FontWeight::W300,
        FontWeight::W400,
        FontWeight::W500,
        FontWeight::W600,
        FontWeight::W700,
        FontWeight::W800,
        FontWeight::W900,
    ];

    /// Linearly interpolates between two font weights.
    ///
    /// If both `a` and `b` are null, then this method will return null. Otherwise,
    /// any null values for `a` or `b` are interpreted as equivalent to [`NORMAL`](FontWeight::NORMAL)
    /// (also known as [`W400`](FontWeight::W400)).
    ///
    /// The `t` argument represents position on the timeline, with 0.0 meaning
    /// that the interpolation has not started, returning `a` (or something
    /// equivalent to `a`), 1.0 meaning that the interpolation has finished,
    /// returning `b` (or something equivalent to `b`), and values in between
    /// meaning that the interpolation is at the relevant point on the timeline
    /// between `a` and `b`. The interpolation can be extrapolated beyond 0.0 and
    /// 1.0, so negative values and values greater than 1.0 are valid. The result
    /// is clamped to the range [`W100`](FontWeight::W100)–[`W900`](FontWeight::W900).
    pub fn lerp(a: Option<FontWeight>, b: Option<FontWeight>, t: f64) -> Option<FontWeight> {
        if a.is_none() && b.is_none() {
            return None;
        }
        let a = a.unwrap_or(FontWeight::NORMAL).value;
        let b = b.unwrap_or(FontWeight::NORMAL).value;
        Some(FontWeight::new(
            lerp_int(a, b, t).round().clamp(100.0, 900.0) as i32,
        ))
    }
}

impl Debug for FontWeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for FontWeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.value % 100 != 0 {
            return write!(f, "FontWeight({})", self.value);
        }
        let name = match self.index() {
            0 => "w100",
            1 => "w200",
            2 => "w300",
            3 => "w400",
            4 => "w500",
            5 => "w600",
            6 => "w700",
            7 => "w800",
            8 => "w900",
            _ => unreachable!(),
        };
        write!(f, "FontWeight.{name}")
    }
}

/// Whether and how to align text horizontally.
///
/// The order of this enum must match the order of the values in
/// RenderStyleConstants.h's ETextAlign.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextAlign {
    /// Align the text on the left edge of the container.
    Left,

    /// Align the text on the right edge of the container.
    Right,

    /// Align the text in the center of the container.
    Center,

    /// Stretch lines of text that end with a soft line break to fill the width of
    /// the container.
    ///
    /// Lines that end with hard line breaks are aligned towards the [`Start`](TextAlign::Start) edge.
    Justify,

    /// Align the text on the leading edge of the container.
    ///
    /// For left-to-right text ([`TextDirection::Ltr`]), this is the left edge.
    ///
    /// For right-to-left text ([`TextDirection::Rtl`]), this is the right edge.
    Start,

    /// Align the text on the trailing edge of the container.
    ///
    /// For left-to-right text ([`TextDirection::Ltr`]), this is the right edge.
    ///
    /// For right-to-left text ([`TextDirection::Rtl`]), this is the left edge.
    End,
}

/// A horizontal line used for aligning text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextBaseline {
    /// The horizontal line used to align the bottom of glyphs for alphabetic characters.
    ///
    /// This baseline is often used for alphabetical scripts like Latin, Greek,
    /// Cyrillic, etc.
    ///
    /// Characters with descenders (like 'p', 'g', or 'y') extend below this line.
    Alphabetic,

    /// The horizontal line used to align ideographic characters.
    ///
    /// This baseline is often used for scripts with uniform square heights,
    /// like Chinese, Japanese, Korean, etc.
    Ideographic,
}

/// A linear decoration to draw near the text.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextDecoration {
    mask: i32,
}

impl TextDecoration {
    /// Do not draw a decoration
    pub const NONE: TextDecoration = TextDecoration { mask: 0x0 };

    /// Draw a line underneath each line of text
    pub const UNDERLINE: TextDecoration = TextDecoration { mask: 0x1 };

    /// Draw a line above each line of text
    pub const OVERLINE: TextDecoration = TextDecoration { mask: 0x2 };

    /// Draw a line through each line of text
    pub const LINE_THROUGH: TextDecoration = TextDecoration { mask: 0x4 };

    /// Creates a decoration that paints the union of all the given decorations.
    pub fn combine(decorations: &[TextDecoration]) -> TextDecoration {
        let mut mask = 0;
        for decoration in decorations {
            mask |= decoration.mask;
        }
        TextDecoration { mask }
    }

    /// The bit mask value that represents this decoration.
    pub fn mask_value(self) -> i32 {
        self.mask
    }

    /// Whether this decoration will paint at least as much decoration as the given decoration.
    pub fn contains(self, other: TextDecoration) -> bool {
        (self.mask | other.mask) == self.mask
    }
}

impl Debug for TextDecoration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for TextDecoration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mask == 0 {
            return write!(f, "TextDecoration.none");
        }
        let mut values = Vec::new();
        if self.mask & TextDecoration::UNDERLINE.mask != 0 {
            values.push("underline");
        }
        if self.mask & TextDecoration::OVERLINE.mask != 0 {
            values.push("overline");
        }
        if self.mask & TextDecoration::LINE_THROUGH.mask != 0 {
            values.push("lineThrough");
        }
        if values.len() == 1 {
            write!(f, "TextDecoration.{}", values[0])
        } else {
            write!(f, "TextDecoration.combine([{}])", values.join(", "))
        }
    }
}

/// The style in which to draw a text decoration
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextDecorationStyle {
    /// Draw a solid line
    Solid,

    /// Draw two lines
    Double,

    /// Draw a dotted line
    Dotted,

    /// Draw a dashed line
    Dashed,

    /// Draw a sinusoidal line
    Wavy,
}

/// How the leading of the text is distributed over and under the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextLeadingDistribution {
    /// Distributes the leading of the text proportionally above and below the
    /// text, to the font's ascent/descent ratio.
    Proportional,

    /// Distributes the leading of the text evenly above and below the text
    /// (i.e. evenly above the font's ascender and below the descender).
    ///
    /// The leading can become negative when `TextStyle.height` is smaller than
    /// 1.0.
    ///
    /// This is the default strategy used by CSS, known as "half-leading".
    Even,
}

/// Defines how to apply `TextStyle.height` over and under text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextHeightBehavior {
    /// Whether to apply the `TextStyle.height` modifier to the ascent of the first
    /// line in the paragraph.
    pub apply_height_to_first_ascent: bool,

    /// Whether to apply the `TextStyle.height` modifier to the descent of the last
    /// line in the paragraph.
    pub apply_height_to_last_descent: bool,

    /// How the leading is distributed over and under the text.
    pub leading_distribution: TextLeadingDistribution,
}

impl Default for TextHeightBehavior {
    fn default() -> TextHeightBehavior {
        TextHeightBehavior {
            apply_height_to_first_ascent: true,
            apply_height_to_last_descent: true,
            leading_distribution: TextLeadingDistribution::Proportional,
        }
    }
}

/// A direction in which text flows.
///
/// Some languages are written from the left to the right (for example, English,
/// Tamil, or Chinese), while others are written from the right to the left (for
/// example Aramaic, Hebrew, or Urdu). Some are also written in a mixture, for
/// example Arabic is mostly written right-to-left, with numerals written
/// left-to-right.
///
/// The text direction must be provided to APIs that render text or lay out
/// boxes horizontally, so that they can determine which direction to start in:
/// either right-to-left, [`TextDirection::Rtl`]; or left-to-right,
/// [`TextDirection::Ltr`].
///
/// ## Design discussion
///
/// Flutter is designed to address the needs of applications written in any of
/// the world's currently-used languages, whether they use a right-to-left or
/// left-to-right writing direction. Flutter does not support other writing
/// modes, such as vertical text or boustrophedon text, as these are rarely used
/// in computer programs.
///
/// It is common when developing user interface frameworks to pick a default
/// text direction — typically left-to-right, the direction most familiar to the
/// engineers working on the framework — because this simplifies the development
/// of applications on the platform. Unfortunately, this frequently results in
/// the platform having unexpected left-to-right biases or assumptions, as
/// engineers will typically miss places where they need to support
/// right-to-left text. This then results in bugs that only manifest in
/// right-to-left environments.
///
/// In an effort to minimize the extent to which Flutter experiences this
/// category of issues, the lowest levels of the Flutter framework do not have a
/// default text reading direction. Any time a reading direction is necessary,
/// for example when text is to be displayed, or when a
/// writing-direction-dependent value is to be interpreted, the reading
/// direction must be explicitly specified. Where possible, such as in `match`
/// statements, the right-to-left case is listed first, to avoid the impression
/// that it is an afterthought.
///
/// At the higher levels (specifically starting at the widgets library), an
/// ambient `Directionality` is introduced, which provides a default. Thus, for
/// instance, a `Text` widget in the scope of a `MaterialApp` widget does not
/// need to be given an explicit writing direction. The `Directionality.of`
/// static method can be used to obtain the ambient text direction for a
/// particular `BuildContext`.
///
/// ### Known left-to-right biases in Flutter
///
/// Despite the design intent described above, certain left-to-right biases have
/// nonetheless crept into Flutter's design. These include:
///
///  * The `Canvas` origin is at the top left, and the x-axis increases in a
///    left-to-right direction.
///
///  * The default localization in the widgets and material libraries is
///    American English, which is left-to-right.
///
/// ### Visual properties vs directional properties
///
/// Many classes in the Flutter framework are offered in two versions, a
/// visually-oriented variant, and a text-direction-dependent variant. For
/// example, `EdgeInsets` is described in terms of top, left, right, and
/// bottom, while `EdgeInsetsDirectional` is described in terms of top, start,
/// end, and bottom, where start and end correspond to right and left in
/// right-to-left text and left and right in left-to-right text.
///
/// There are distinct use cases for each of these variants.
///
/// Text-direction-dependent variants are useful when developing user interfaces
/// that should "flip" with the text direction. For example, a paragraph of text
/// in English will typically be left-aligned and a quote will be indented from
/// the left, while in Arabic it will be right-aligned and indented from the
/// right. Both of these cases are described by the direction-dependent
/// [`TextAlign::Start`] and `EdgeInsetsDirectional` start.
///
/// In contrast, the visual variants are useful when the text direction is known
/// and not affected by the reading direction. For example, an application
/// giving driving directions might show a "turn left" arrow on the left and a
/// "turn right" arrow on the right — and would do so whether the application
/// was localized to French (left-to-right) or Hebrew (right-to-left).
///
/// In practice, it is also expected that many developers will only be targeting
/// one language, and in that case it may be simpler to think in visual terms.
///
/// The order of this enum must match the order of the values in
/// `TextDirection.h`'s `TextDirection`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextDirection {
    /// The text flows from right to left (e.g. Arabic, Hebrew).
    Rtl,

    /// The text flows from left to right (e.g., English, French).
    Ltr,
}

/// A feature tag and value that affect the selection of glyphs in a font.
///
/// Named constructors for individual OpenType tags (`alternative`, `fractions`,
/// …) are deferred until a caller needs them. [`new`](FontFeature::new),
/// [`enable`](FontFeature::enable), and [`disable`](FontFeature::disable) are
/// the constructors Flutter's painting `TextStyle` uses.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontFeature {
    /// The tag that identifies the effect of this feature. Must consist of 4
    /// ASCII characters.
    pub feature: String,
    /// The value assigned to this feature.
    ///
    /// Must be a positive integer. Many features are Boolean values using 1
    /// (on) and 0 (off).
    pub value: i32,
}

impl FontFeature {
    /// Creates a [`FontFeature`] object, which can be added to a text style to
    /// change how the engine selects glyphs when rendering text.
    pub fn new(feature: impl Into<String>, value: i32) -> FontFeature {
        let feature = feature.into();
        debug_assert!(
            feature.len() == 4,
            "Feature tag must be exactly four characters long."
        );
        debug_assert!(
            value >= 0,
            "Feature value must be zero or a positive integer."
        );
        FontFeature { feature, value }
    }

    /// Create a [`FontFeature`] object that enables the feature with the given
    /// tag.
    pub fn enable(feature: impl Into<String>) -> FontFeature {
        FontFeature::new(feature, 1)
    }

    /// Create a [`FontFeature`] object that disables the feature with the given
    /// tag.
    pub fn disable(feature: impl Into<String>) -> FontFeature {
        FontFeature::new(feature, 0)
    }
}

impl Display for FontFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FontFeature('{}', {})", self.feature, self.value)
    }
}

/// An axis tag and value that can be used to customize variable fonts.
///
/// Named constructors besides [`italic`](FontVariation::italic),
/// [`optical_size`](FontVariation::optical_size), [`slant`](FontVariation::slant),
/// [`width`](FontVariation::width), and [`weight`](FontVariation::weight) are
/// not added until a caller needs them.
#[derive(Clone, Debug, PartialEq)]
pub struct FontVariation {
    /// The tag that identifies the design axis.
    ///
    /// An axis tag must consist of 4 ASCII characters.
    pub axis: String,
    /// The value assigned to this design axis.
    pub value: f64,
}

impl FontVariation {
    /// Creates a [`FontVariation`] object, which can be added to a text style to
    /// change the variable attributes of a font.
    pub fn new(axis: impl Into<String>, value: f64) -> FontVariation {
        let axis = axis.into();
        debug_assert!(
            axis.len() == 4,
            "Axis tag must be exactly four characters long."
        );
        debug_assert!(
            (-32768.0..32768.0).contains(&value),
            "Value must be representable as a signed 16.16 fixed-point number, i.e. it must be in this range: -32768.0 ≤ value < 32768.0"
        );
        FontVariation { axis, value }
    }

    /// Variable font style. (`ital`)
    pub fn italic(value: f64) -> FontVariation {
        debug_assert!((0.0..=1.0).contains(&value));
        FontVariation::new("ital", value)
    }

    /// Optical size optimization. (`opsz`)
    pub fn optical_size(value: f64) -> FontVariation {
        debug_assert!(value > 0.0);
        FontVariation::new("opsz", value)
    }

    /// Variable font slant. (`slnt`)
    pub fn slant(value: f64) -> FontVariation {
        debug_assert!(value > -90.0 && value < 90.0);
        FontVariation::new("slnt", value)
    }

    /// Variable font width. (`wdth`)
    pub fn width(value: f64) -> FontVariation {
        debug_assert!(value >= 0.0);
        FontVariation::new("wdth", value)
    }

    /// Variable font weight. (`wght`)
    pub fn weight(value: f64) -> FontVariation {
        debug_assert!((1.0..=1000.0).contains(&value));
        FontVariation::new("wght", value)
    }

    /// Linearly interpolates between two font variations.
    ///
    /// If the two variations have different axis tags, the interpolation
    /// switches abruptly from one to the other at t=0.5. Otherwise, the value
    /// is interpolated.
    pub fn lerp(
        a: Option<&FontVariation>,
        b: Option<&FontVariation>,
        t: f64,
    ) -> Option<FontVariation> {
        if a.map(|v| v.axis.as_str()) != b.map(|v| v.axis.as_str()) || (a.is_none() && b.is_none())
        {
            return if t < 0.5 { a.cloned() } else { b.cloned() };
        }
        Some(FontVariation::new(
            a.unwrap().axis.clone(),
            clamp_double(
                lerp_double(Some(a.unwrap().value), Some(b.unwrap().value), t).unwrap(),
                -32768.0,
                32768.0 - 1.0 / 65536.0,
            ),
        ))
    }
}

impl Display for FontVariation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FontVariation('{}', {})", self.axis, self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_weight_lerp_treats_null_as_normal() {
        assert!(FontWeight::lerp(None, None, 0.5).is_none());
        assert_eq!(
            FontWeight::lerp(None, Some(FontWeight::W700), 0.0),
            Some(FontWeight::NORMAL)
        );
        assert_eq!(
            FontWeight::lerp(Some(FontWeight::W100), Some(FontWeight::W900), 0.5),
            Some(FontWeight::W500)
        );
    }

    #[test]
    fn text_decoration_combine_unions_masks() {
        let combined =
            TextDecoration::combine(&[TextDecoration::UNDERLINE, TextDecoration::LINE_THROUGH]);
        assert!(combined.contains(TextDecoration::UNDERLINE));
        assert!(combined.contains(TextDecoration::LINE_THROUGH));
        assert!(!combined.contains(TextDecoration::OVERLINE));
        assert_eq!(
            format!("{combined}"),
            "TextDecoration.combine([underline, lineThrough])"
        );
    }

    #[test]
    fn text_direction_lists_rtl_first() {
        assert_eq!(TextDirection::Rtl as u8, 0);
        assert_eq!(TextDirection::Ltr as u8, 1);
    }

    #[test]
    fn font_variation_lerp_switches_axis_at_half() {
        let a = FontVariation::weight(400.0);
        let b = FontVariation::width(100.0);
        assert_eq!(
            FontVariation::lerp(Some(&a), Some(&b), 0.4).unwrap().axis,
            "wght"
        );
        assert_eq!(
            FontVariation::lerp(Some(&a), Some(&b), 0.6).unwrap().axis,
            "wdth"
        );
    }

    #[test]
    fn font_variation_lerp_interpolates_same_axis() {
        let a = FontVariation::weight(100.0);
        let b = FontVariation::weight(900.0);
        let mid = FontVariation::lerp(Some(&a), Some(&b), 0.5).unwrap();
        assert_eq!(mid.axis, "wght");
        assert!((mid.value - 500.0).abs() < 1e-9);
    }
}
