//! Flutter counterpart: `painting/strut_style.dart`.

use std::fmt::{self, Debug};

use waft_embedder::StrutStyle as UiStrutStyle;
use waft_embedder::{FontStyle, FontWeight, TextLeadingDistribution};

use crate::basic_types::RenderComparison;
use crate::text_scaler::TextScaler;
use crate::text_style::TextStyle;

/// Defines the strut, which sets the minimum height a line can be relative to the baseline.
///
/// Strut applies to all lines in the paragraph. Strut is a feature that allows minimum line
/// heights to be set. The effect is as if a zero width space was included at the beginning of
/// each line in the paragraph. This imaginary space is "shaped" according the properties
/// defined in this class.
///
/// No lines may be shorter than the strut. The ascent and descent of the strut are
/// calculated, and any laid out text that has a shorter ascent or descent than the strut's
/// ascent or descent will take the ascent and descent of the strut. Text with ascents or
/// descents larger than the strut's ascent or descent will layout as normal and extend past
/// the strut.
///
/// Strut is defined independently from any text content or [`TextStyle`]s.
#[derive(Clone)]
pub struct StrutStyle {
    /// The name of the font to use when calculating the strut (e.g., Roboto). If the font is
    /// defined in a package, this is prefixed with `packages/package_name/`.
    ///
    /// The value provided in [`font_family`](Self::font_family) acts as the preferred family,
    /// followed in order by [`font_family_fallback_list`](Self::font_family_fallback_list).
    pub font_family: Option<String>,
    font_family_fallback: Option<Vec<String>>,
    package: Option<String>,
    /// The size of text (in logical pixels) to use when obtaining metrics from the font.
    ///
    /// The default is 14 logical pixels.
    pub font_size: Option<f64>,
    /// The minimum height of the strut, as a multiple of [`font_size`](Self::font_size).
    ///
    /// When `None`, the strut's height is the font-defined ascent plus descent.
    pub height: Option<f64>,
    /// How the vertical space added by the [`height`](Self::height) multiplier should be
    /// distributed over and under the strut.
    ///
    /// Defaults to `None`, which defers to the paragraph's `textHeightBehavior`'s leading
    /// distribution; [`TextLeadingDistribution::Even`] distributes it evenly.
    pub leading_distribution: Option<TextLeadingDistribution>,
    /// The typeface thickness to use when calculating the strut (e.g., bold).
    ///
    /// The default is w400.
    pub font_weight: Option<FontWeight>,
    /// The typeface variant to use when calculating the strut (e.g., italics).
    ///
    /// The default is upright.
    pub font_style: Option<FontStyle>,
    /// The additional leading to apply to the strut as a multiple of
    /// [`font_size`](Self::font_size), independent of [`height`](Self::height) and
    /// [`leading_distribution`](Self::leading_distribution).
    ///
    /// Half of the leading is added above the text and half below. Defaults to `None`, which
    /// adds none.
    pub leading: Option<f64>,
    /// Whether the strut height should be forced.
    ///
    /// When true, all lines are laid out with the height of the strut, and line- and
    /// run-specific metrics are ignored.
    ///
    /// The default is false.
    pub force_strut_height: Option<bool>,
    /// A human-readable description of this strut style.
    ///
    /// Maintained only in debug builds. Not considered by `==` or
    /// [`compare_to`](Self::compare_to).
    pub debug_label: Option<String>,
}

impl StrutStyle {
    /// Creates a strut style.
    ///
    /// Chain setters for the fields Dart's constructor takes as named arguments:
    /// `StrutStyle::new().font_size(14.0)`.
    pub fn new() -> StrutStyle {
        StrutStyle {
            font_family: None,
            font_family_fallback: None,
            package: None,
            font_size: None,
            height: None,
            leading_distribution: None,
            font_weight: None,
            font_style: None,
            leading: None,
            force_strut_height: None,
            debug_label: None,
        }
    }

    /// A [`StrutStyle`] that will have no impact on the text layout.
    ///
    /// Equivalent to having no strut at all. All lines will be laid out according to the
    /// properties defined in [`TextStyle`].
    pub const DISABLED: StrutStyle = StrutStyle {
        font_family: None,
        font_family_fallback: None,
        package: None,
        font_size: None,
        height: Some(0.0),
        leading_distribution: None,
        font_weight: None,
        font_style: None,
        leading: Some(0.0),
        force_strut_height: None,
        debug_label: None,
    };

    /// Builds a [`StrutStyle`] that contains values of the equivalent properties in
    /// `text_style`.
    ///
    /// Chain optional Dart named arguments, then [`FromTextStyle::build`]. They override the
    /// text style's values.
    pub fn from_text_style(text_style: &TextStyle) -> FromTextStyle<'_> {
        FromTextStyle {
            text_style,
            font_family: None,
            font_family_fallback: None,
            font_size: None,
            height: None,
            leading_distribution: None,
            leading: None,
            font_weight: None,
            font_style: None,
            force_strut_height: None,
            debug_label: None,
            package: None,
        }
    }

    /// The name of the font to use when calculating the strut.
    pub fn font_family(mut self, font_family: impl Into<String>) -> StrutStyle {
        let family = font_family.into();
        self.font_family = Some(match &self.package {
            Some(package) => format!("packages/{package}/{family}"),
            None => family,
        });
        self
    }

    /// The ordered list of font families to fall back on when a higher priority font family
    /// cannot be found.
    pub fn font_family_fallback(mut self, font_family_fallback: Vec<String>) -> StrutStyle {
        self.font_family_fallback = Some(font_family_fallback);
        self
    }

    /// The package argument must be provided if the font family is defined in a package. It is
    /// combined with [`font_family`](Self::font_family) to set that property.
    pub fn package(mut self, package: impl Into<String>) -> StrutStyle {
        let package = package.into();
        if let Some(family) = self.font_family_unprefixed() {
            self.font_family = Some(format!("packages/{package}/{family}"));
        }
        self.package = Some(package);
        self
    }

    /// The size of text (in logical pixels) to use when obtaining metrics from the font.
    pub fn font_size(mut self, font_size: f64) -> StrutStyle {
        debug_assert!(font_size > 0.0, "fontSize must be positive and non-zero");
        self.font_size = Some(font_size);
        self
    }

    /// The minimum height of the strut, as a multiple of [`font_size`](Self::font_size).
    pub fn height(mut self, height: f64) -> StrutStyle {
        self.height = Some(height);
        self
    }

    /// How the vertical space added by the [`height`](Self::height) multiplier should be
    /// distributed over and under the strut.
    pub fn leading_distribution(
        mut self,
        leading_distribution: TextLeadingDistribution,
    ) -> StrutStyle {
        self.leading_distribution = Some(leading_distribution);
        self
    }

    /// The typeface thickness to use when calculating the strut.
    pub fn font_weight(mut self, font_weight: FontWeight) -> StrutStyle {
        self.font_weight = Some(font_weight);
        self
    }

    /// The typeface variant to use when calculating the strut.
    pub fn font_style(mut self, font_style: FontStyle) -> StrutStyle {
        self.font_style = Some(font_style);
        self
    }

    /// The additional leading to apply to the strut as a multiple of
    /// [`font_size`](Self::font_size).
    pub fn leading(mut self, leading: f64) -> StrutStyle {
        debug_assert!(leading >= 0.0, "leading must be zero or positive");
        self.leading = Some(leading);
        self
    }

    /// Whether the strut height should be forced.
    pub fn force_strut_height(mut self, force_strut_height: bool) -> StrutStyle {
        self.force_strut_height = Some(force_strut_height);
        self
    }

    /// A human-readable description of this strut style.
    pub fn debug_label(mut self, debug_label: impl Into<String>) -> StrutStyle {
        self.debug_label = Some(debug_label.into());
        self
    }

    /// The ordered list of font families to fall back on, with the package prefix applied.
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

    /// Describe the difference between this style and another, in terms of how much damage it
    /// will make to the rendering.
    pub fn compare_to(&self, other: &StrutStyle) -> RenderComparison {
        if std::ptr::eq(self, other) {
            return RenderComparison::Identical;
        }
        if self.font_family != other.font_family
            || self.font_size != other.font_size
            || self.font_weight != other.font_weight
            || self.font_style != other.font_style
            || self.height != other.height
            || self.leading != other.leading
            || self.force_strut_height != other.force_strut_height
            || self.font_family_fallback_list() != other.font_family_fallback_list()
            || (self.height.is_some() && self.leading_distribution != other.leading_distribution)
        {
            return RenderComparison::Layout;
        }
        RenderComparison::Identical
    }

    /// Returns a new strut style that inherits its `None` values from corresponding properties
    /// in `other`.
    ///
    /// If `other` is `None`, returns this strut style unchanged.
    pub fn inherit_from_text_style(&self, other: Option<&TextStyle>) -> StrutStyle {
        let Some(other) = other else {
            return self.clone();
        };
        let effective_height = self.height.or(other.height);
        StrutStyle {
            font_family: self
                .font_family
                .clone()
                .or_else(|| other.font_family.clone()),
            // The text style's getter is already package-prefixed, and the result keeps no
            // package of its own.
            font_family_fallback: self
                .font_family_fallback_list()
                .or_else(|| other.font_family_fallback_list()),
            // Package is embedded within the getters for fontFamilyFallback.
            package: None,
            font_size: self.font_size.or(other.font_size),
            height: effective_height,
            leading_distribution: if effective_height.is_some() {
                self.leading_distribution.or(other.leading_distribution)
            } else {
                None
            },
            leading: self.leading,
            font_weight: self.font_weight.or(other.font_weight),
            font_style: self.font_style.or(other.font_style),
            force_strut_height: self.force_strut_height,
            debug_label: self
                .debug_label
                .clone()
                .or_else(|| other.debug_label.clone()),
        }
    }

    /// Returns a new strut style that is a combination of this style and `other`.
    ///
    /// All `None` values of `other` are replaced with members from this style.
    pub fn merge(&self, other: Option<&StrutStyle>) -> StrutStyle {
        let Some(other) = other else {
            return self.clone();
        };
        StrutStyle {
            font_family: other
                .font_family
                .clone()
                .or_else(|| self.font_family.clone()),
            // Dart merges the two package-prefixed getters, then applies the merged package.
            font_family_fallback: other
                .font_family_fallback_list()
                .or_else(|| self.font_family_fallback_list()),
            package: other.package.clone().or_else(|| self.package.clone()),
            font_size: other.font_size.or(self.font_size),
            height: other.height.or(self.height),
            leading_distribution: other.leading_distribution.or(self.leading_distribution),
            leading: other.leading.or(self.leading),
            font_weight: other.font_weight.or(self.font_weight),
            font_style: other.font_style.or(self.font_style),
            force_strut_height: other.force_strut_height.or(self.force_strut_height),
            debug_label: other
                .debug_label
                .clone()
                .or_else(|| self.debug_label.clone()),
        }
    }

    /// This style as `dart:ui`'s, with the font size scaled by `text_scaler`: the conversion
    /// Dart's `TextStyle.getParagraphStyle` makes for a paragraph's strut.
    pub fn get_paragraph_strut_style(&self, text_scaler: &TextScaler) -> UiStrutStyle {
        UiStrutStyle {
            font_family: self.font_family.clone(),
            font_family_fallback: self.font_family_fallback_list(),
            font_size: self.font_size.map(|font_size| text_scaler.scale(font_size)),
            height: self.height,
            leading: self.leading,
            leading_distribution: self.leading_distribution,
            font_weight: self.font_weight,
            font_style: self.font_style,
            force_strut_height: self.force_strut_height,
        }
    }
}

impl Default for StrutStyle {
    fn default() -> StrutStyle {
        StrutStyle::new()
    }
}

impl PartialEq for StrutStyle {
    fn eq(&self, other: &StrutStyle) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        self.font_family == other.font_family
            && self.font_size == other.font_size
            && self.font_weight == other.font_weight
            && self.font_style == other.font_style
            && self.height == other.height
            && self.leading == other.leading
            && self.force_strut_height == other.force_strut_height
            && (self.height.is_none() || self.leading_distribution == other.leading_distribution)
            && self.font_family_fallback_list() == other.font_family_fallback_list()
    }
}

impl Debug for StrutStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("StrutStyle");
        if cfg!(debug_assertions) {
            s.field("debug_label", &self.debug_label);
        }
        s.field("family", &self.font_family)
            .field("family_fallback", &self.font_family_fallback_list())
            .field("size", &self.font_size)
            .field("weight", &self.font_weight)
            .field("style", &self.font_style)
            .field("height", &self.height)
            .field("force_strut_height", &self.force_strut_height);
        if self.height.is_some() {
            s.field("leading_distribution", &self.leading_distribution);
        }
        s.finish()
    }
}

/// Builds a [`StrutStyle`] from a [`TextStyle`]'s equivalent fields, with Dart's named
/// overrides.
pub struct FromTextStyle<'a> {
    text_style: &'a TextStyle,
    font_family: Option<String>,
    font_family_fallback: Option<Vec<String>>,
    font_size: Option<f64>,
    height: Option<f64>,
    leading_distribution: Option<TextLeadingDistribution>,
    leading: Option<f64>,
    font_weight: Option<FontWeight>,
    font_style: Option<FontStyle>,
    force_strut_height: Option<bool>,
    debug_label: Option<String>,
    package: Option<String>,
}

impl FromTextStyle<'_> {
    /// Overrides the text style's `fontFamily`.
    pub fn font_family(mut self, font_family: impl Into<String>) -> Self {
        self.font_family = Some(font_family.into());
        self
    }

    /// Overrides the text style's `fontFamilyFallback`.
    pub fn font_family_fallback(mut self, font_family_fallback: Vec<String>) -> Self {
        self.font_family_fallback = Some(font_family_fallback);
        self
    }

    /// Overrides the text style's `fontSize`.
    pub fn font_size(mut self, font_size: f64) -> Self {
        self.font_size = Some(font_size);
        self
    }

    /// Overrides the text style's `height`.
    pub fn height(mut self, height: f64) -> Self {
        self.height = Some(height);
        self
    }

    /// Overrides the text style's `leadingDistribution`.
    pub fn leading_distribution(mut self, leading_distribution: TextLeadingDistribution) -> Self {
        self.leading_distribution = Some(leading_distribution);
        self
    }

    /// [`TextStyle`] has no equivalent property.
    pub fn leading(mut self, leading: f64) -> Self {
        self.leading = Some(leading);
        self
    }

    /// Overrides the text style's `fontWeight`.
    pub fn font_weight(mut self, font_weight: FontWeight) -> Self {
        self.font_weight = Some(font_weight);
        self
    }

    /// Overrides the text style's `fontStyle`.
    pub fn font_style(mut self, font_style: FontStyle) -> Self {
        self.font_style = Some(font_style);
        self
    }

    /// [`StrutStyle`]-unique property.
    pub fn force_strut_height(mut self, force_strut_height: bool) -> Self {
        self.force_strut_height = Some(force_strut_height);
        self
    }

    /// Overrides the text style's `debugLabel`.
    pub fn debug_label(mut self, debug_label: impl Into<String>) -> Self {
        self.debug_label = Some(debug_label.into());
        self
    }

    /// Prefixes the family names with `packages/package_name/`.
    pub fn package(mut self, package: impl Into<String>) -> Self {
        self.package = Some(package.into());
        self
    }

    /// The built strut style.
    pub fn build(self) -> StrutStyle {
        let text_style = self.text_style;
        let family = match (self.font_family, &self.package) {
            (Some(family), Some(package)) => Some(format!("packages/{package}/{family}")),
            (Some(family), None) => Some(family),
            (None, _) => text_style.font_family.clone(),
        };
        StrutStyle {
            font_family: family,
            font_family_fallback: self
                .font_family_fallback
                .or_else(|| text_style.font_family_fallback_list()),
            package: self.package,
            font_size: self.font_size.or(text_style.font_size),
            height: self.height.or(text_style.height),
            leading_distribution: self
                .leading_distribution
                .or(text_style.leading_distribution),
            leading: self.leading,
            font_weight: self.font_weight.or(text_style.font_weight),
            font_style: self.font_style.or(text_style.font_style),
            force_strut_height: self.force_strut_height,
            debug_label: self.debug_label.or_else(|| text_style.debug_label.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dart's `strut_style_test.dart`. Its `toString` assertions are diagnostics-only: Rust
    /// formats with `Debug`, so they are not ported.
    #[test]
    fn strut_style_diagnostics_test() {
        let s0 = StrutStyle::new().font_family("Serif").font_size(14.0);
        assert_eq!(s0.font_family.as_deref(), Some("Serif"));
        assert_eq!(s0.font_size, Some(14.0));

        let s1 = StrutStyle::new()
            .font_family("Serif")
            .font_size(14.0)
            .force_strut_height(true);
        assert_eq!(s1.font_family.as_deref(), Some("Serif"));
        assert_eq!(s1.font_size, Some(14.0));
        assert_eq!(s1, s1.clone());

        let s2 = StrutStyle::new()
            .font_family("Serif")
            .font_size(14.0)
            .force_strut_height(false);
        assert_eq!(s2.force_strut_height, Some(false));

        let s6 = StrutStyle::new()
            .height(14.0)
            .leading_distribution(TextLeadingDistribution::Even);
        let s7 = StrutStyle::new()
            .height(14.0)
            .leading_distribution(TextLeadingDistribution::Proportional);
        assert_ne!(s6, s7);
    }

    #[test]
    fn from_text_style_takes_the_text_styles_fields() {
        let text_style = TextStyle::new()
            .font_family("Serif")
            .font_size(12.0)
            .height(1.5)
            .font_weight(FontWeight::BOLD);
        let strut_style = StrutStyle::from_text_style(&text_style).build();
        assert_eq!(strut_style.font_family.as_deref(), Some("Serif"));
        assert_eq!(strut_style.font_size, Some(12.0));
        assert_eq!(strut_style.height, Some(1.5));
        assert_eq!(strut_style.font_weight, Some(FontWeight::BOLD));
        assert_eq!(strut_style.force_strut_height, None);

        let forced = StrutStyle::from_text_style(&text_style)
            .force_strut_height(true)
            .build();
        assert_eq!(forced.force_strut_height, Some(true));
    }

    #[test]
    fn inherit_from_text_style_fills_the_unset_fields() {
        let text_style = TextStyle::new()
            .font_family("Serif")
            .font_size(12.0)
            .height(1.5)
            .leading_distribution(TextLeadingDistribution::Even);
        let strut_style = StrutStyle::new()
            .font_size(24.0)
            .inherit_from_text_style(Some(&text_style));
        assert_eq!(strut_style.font_family.as_deref(), Some("Serif"));
        assert_eq!(strut_style.font_size, Some(24.0));
        assert_eq!(strut_style.height, Some(1.5));
        assert_eq!(
            strut_style.leading_distribution,
            Some(TextLeadingDistribution::Even)
        );

        let without_height =
            StrutStyle::new().inherit_from_text_style(Some(&TextStyle::new().font_size(12.0)));
        assert_eq!(without_height.leading_distribution, None);
    }

    #[test]
    fn merge_lets_the_other_style_win_where_it_is_set() {
        let base = StrutStyle::new().font_size(10.0).height(2.0);
        let merged = base.merge(Some(&StrutStyle::new().font_size(20.0)));
        assert_eq!(merged.font_size, Some(20.0));
        assert_eq!(merged.height, Some(2.0));
    }

    #[test]
    fn equality_ignores_leading_distribution_without_a_height() {
        let even = StrutStyle::new().leading_distribution(TextLeadingDistribution::Even);
        let proportional =
            StrutStyle::new().leading_distribution(TextLeadingDistribution::Proportional);
        assert_eq!(even, proportional);
        assert_ne!(
            even.height(1.0),
            proportional.height(1.0),
            "a height makes the distribution matter"
        );
    }
}
