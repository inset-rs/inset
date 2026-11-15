//! Flutter counterpart: material_ui's `generated/typography_defaults_m3.g.dart`, a Dart `part`
//! of `typography.dart`: `Typography::english_like2021` and its two siblings read it.

// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

use waft_embedder::{FontWeight, TextBaseline, TextLeadingDistribution};
use waft_painting::TextStyle;

use crate::text_theme::TextTheme;

/// Dart's `_M3Typography`.
pub(crate) struct M3Typography;

impl M3Typography {
    pub(crate) fn english_like() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("englishLike displayLarge 2021")
                    .inherit(false)
                    .font_size(57.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(-0.25)
                    .height(1.12)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("englishLike displayMedium 2021")
                    .inherit(false)
                    .font_size(45.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.16)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("englishLike displaySmall 2021")
                    .inherit(false)
                    .font_size(36.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.22)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("englishLike headlineLarge 2021")
                    .inherit(false)
                    .font_size(32.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.25)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("englishLike headlineMedium 2021")
                    .inherit(false)
                    .font_size(28.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.29)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("englishLike headlineSmall 2021")
                    .inherit(false)
                    .font_size(24.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.33)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("englishLike titleLarge 2021")
                    .inherit(false)
                    .font_size(22.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.27)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("englishLike titleMedium 2021")
                    .inherit(false)
                    .font_size(16.0)
                    .font_weight(FontWeight::W500)
                    .letter_spacing(0.15)
                    .height(1.50)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("englishLike titleSmall 2021")
                    .inherit(false)
                    .font_size(14.0)
                    .font_weight(FontWeight::W500)
                    .letter_spacing(0.1)
                    .height(1.43)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("englishLike labelLarge 2021")
                    .inherit(false)
                    .font_size(14.0)
                    .font_weight(FontWeight::W500)
                    .letter_spacing(0.1)
                    .height(1.43)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("englishLike labelMedium 2021")
                    .inherit(false)
                    .font_size(12.0)
                    .font_weight(FontWeight::W500)
                    .letter_spacing(0.5)
                    .height(1.33)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("englishLike labelSmall 2021")
                    .inherit(false)
                    .font_size(11.0)
                    .font_weight(FontWeight::W500)
                    .letter_spacing(0.5)
                    .height(1.45)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("englishLike bodyLarge 2021")
                    .inherit(false)
                    .font_size(16.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.5)
                    .height(1.50)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("englishLike bodyMedium 2021")
                    .inherit(false)
                    .font_size(14.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.25)
                    .height(1.43)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("englishLike bodySmall 2021")
                    .inherit(false)
                    .font_size(12.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.4)
                    .height(1.33)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
        }
    }

    pub(crate) fn dense() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("dense displayLarge 2021")
                    .inherit(false)
                    .font_size(57.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(-0.25)
                    .height(1.12)
                    .text_baseline(TextBaseline::Ideographic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("dense displayMedium 2021")
                    .inherit(false)
                    .font_size(45.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.16)
                    .text_baseline(TextBaseline::Ideographic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("dense displaySmall 2021")
                    .inherit(false)
                    .font_size(36.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.22)
                    .text_baseline(TextBaseline::Ideographic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("dense headlineLarge 2021")
                    .inherit(false)
                    .font_size(32.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.25)
                    .text_baseline(TextBaseline::Ideographic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("dense headlineMedium 2021")
                    .inherit(false)
                    .font_size(28.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.29)
                    .text_baseline(TextBaseline::Ideographic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("dense headlineSmall 2021")
                    .inherit(false)
                    .font_size(24.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.33)
                    .text_baseline(TextBaseline::Ideographic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("dense titleLarge 2021")
                    .inherit(false)
                    .font_size(22.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.27)
                    .text_baseline(TextBaseline::Ideographic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("dense titleMedium 2021")
                    .inherit(false)
                    .font_size(16.0)
                    .font_weight(FontWeight::W500)
                    .letter_spacing(0.15)
                    .height(1.50)
                    .text_baseline(TextBaseline::Ideographic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("dense titleSmall 2021")
                    .inherit(false)
                    .font_size(14.0)
                    .font_weight(FontWeight::W500)
                    .letter_spacing(0.1)
                    .height(1.43)
                    .text_baseline(TextBaseline::Ideographic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("dense labelLarge 2021")
                    .inherit(false)
                    .font_size(14.0)
                    .font_weight(FontWeight::W500)
                    .letter_spacing(0.1)
                    .height(1.43)
                    .text_baseline(TextBaseline::Ideographic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("dense labelMedium 2021")
                    .inherit(false)
                    .font_size(12.0)
                    .font_weight(FontWeight::W500)
                    .letter_spacing(0.5)
                    .height(1.33)
                    .text_baseline(TextBaseline::Ideographic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("dense labelSmall 2021")
                    .inherit(false)
                    .font_size(11.0)
                    .font_weight(FontWeight::W500)
                    .letter_spacing(0.5)
                    .height(1.45)
                    .text_baseline(TextBaseline::Ideographic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("dense bodyLarge 2021")
                    .inherit(false)
                    .font_size(16.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.5)
                    .height(1.50)
                    .text_baseline(TextBaseline::Ideographic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("dense bodyMedium 2021")
                    .inherit(false)
                    .font_size(14.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.25)
                    .height(1.43)
                    .text_baseline(TextBaseline::Ideographic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("dense bodySmall 2021")
                    .inherit(false)
                    .font_size(12.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.4)
                    .height(1.33)
                    .text_baseline(TextBaseline::Ideographic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
        }
    }

    pub(crate) fn tall() -> TextTheme {
        TextTheme {
            display_large: Some(
                TextStyle::new()
                    .debug_label("tall displayLarge 2021")
                    .inherit(false)
                    .font_size(57.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(-0.25)
                    .height(1.12)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            display_medium: Some(
                TextStyle::new()
                    .debug_label("tall displayMedium 2021")
                    .inherit(false)
                    .font_size(45.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.16)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            display_small: Some(
                TextStyle::new()
                    .debug_label("tall displaySmall 2021")
                    .inherit(false)
                    .font_size(36.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.22)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            headline_large: Some(
                TextStyle::new()
                    .debug_label("tall headlineLarge 2021")
                    .inherit(false)
                    .font_size(32.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.25)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            headline_medium: Some(
                TextStyle::new()
                    .debug_label("tall headlineMedium 2021")
                    .inherit(false)
                    .font_size(28.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.29)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            headline_small: Some(
                TextStyle::new()
                    .debug_label("tall headlineSmall 2021")
                    .inherit(false)
                    .font_size(24.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.33)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            title_large: Some(
                TextStyle::new()
                    .debug_label("tall titleLarge 2021")
                    .inherit(false)
                    .font_size(22.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.0)
                    .height(1.27)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            title_medium: Some(
                TextStyle::new()
                    .debug_label("tall titleMedium 2021")
                    .inherit(false)
                    .font_size(16.0)
                    .font_weight(FontWeight::W500)
                    .letter_spacing(0.15)
                    .height(1.50)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            title_small: Some(
                TextStyle::new()
                    .debug_label("tall titleSmall 2021")
                    .inherit(false)
                    .font_size(14.0)
                    .font_weight(FontWeight::W500)
                    .letter_spacing(0.1)
                    .height(1.43)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            label_large: Some(
                TextStyle::new()
                    .debug_label("tall labelLarge 2021")
                    .inherit(false)
                    .font_size(14.0)
                    .font_weight(FontWeight::W500)
                    .letter_spacing(0.1)
                    .height(1.43)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            label_medium: Some(
                TextStyle::new()
                    .debug_label("tall labelMedium 2021")
                    .inherit(false)
                    .font_size(12.0)
                    .font_weight(FontWeight::W500)
                    .letter_spacing(0.5)
                    .height(1.33)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            label_small: Some(
                TextStyle::new()
                    .debug_label("tall labelSmall 2021")
                    .inherit(false)
                    .font_size(11.0)
                    .font_weight(FontWeight::W500)
                    .letter_spacing(0.5)
                    .height(1.45)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            body_large: Some(
                TextStyle::new()
                    .debug_label("tall bodyLarge 2021")
                    .inherit(false)
                    .font_size(16.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.5)
                    .height(1.50)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            body_medium: Some(
                TextStyle::new()
                    .debug_label("tall bodyMedium 2021")
                    .inherit(false)
                    .font_size(14.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.25)
                    .height(1.43)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
            body_small: Some(
                TextStyle::new()
                    .debug_label("tall bodySmall 2021")
                    .inherit(false)
                    .font_size(12.0)
                    .font_weight(FontWeight::W400)
                    .letter_spacing(0.4)
                    .height(1.33)
                    .text_baseline(TextBaseline::Alphabetic)
                    .leading_distribution(TextLeadingDistribution::Even),
            ),
        }
    }
}
