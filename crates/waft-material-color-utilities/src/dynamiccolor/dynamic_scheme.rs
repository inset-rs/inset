// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// Ported from `lib/dynamiccolor/dynamic_scheme.dart` of material_color_utilities 0.13.0.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::dynamiccolor::dynamic_color::DynamicColor;
use crate::dynamiccolor::material_dynamic_colors::MaterialDynamicColors;
use crate::dynamiccolor::variant::Variant;
use crate::hct::hct::Hct;
use crate::palettes::tonal_palette::TonalPalette;
use crate::utils::math_utils::MathUtils;

/// The identity of a [`DynamicScheme`], which Dart has from the object itself.
///
/// No two schemes made in one process share one, so a [`DynamicColor`]'s cache keyed by it
/// never answers for a scheme made later at the same address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct DynamicSchemeId(u64);

impl DynamicSchemeId {
    fn next() -> DynamicSchemeId {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        DynamicSchemeId(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// Constructed by a set of values representing the current UI state (such as
/// whether or not its dark theme, what the theme style is, etc.), and
/// provides a set of [`TonalPalette`]s that can create colors that fit in
/// with the theme style. Used by [`DynamicColor`] to resolve into a color.
///
/// The fields are Dart's final fields, read through methods: the colors computed for a
/// scheme are cached per scheme, so it does not change once made.
pub struct DynamicScheme {
    id: DynamicSchemeId,

    /// The source color of the theme as an ARGB integer.
    source_color_argb: u32,

    /// The source color of the theme in HCT.
    source_color_hct: Hct,

    /// The variant, or style, of the theme.
    variant: Variant,

    /// Whether or not the scheme is in 'dark mode' or 'light mode'.
    is_dark: bool,

    /// Value from -1 to 1. -1 represents minimum contrast, 0 represents
    /// standard (i.e. the design as spec'd), and 1 represents maximum contrast.
    contrast_level: f64,

    /// Given a tone, produces a color. Hue and chroma of the color are specified
    /// in the design specification of the variant. Usually colorful.
    primary_palette: TonalPalette,

    /// Given a tone, produces a color. Hue and chroma of the color are specified
    /// in the design specification of the variant. Usually less colorful.
    secondary_palette: TonalPalette,

    /// Given a tone, produces a color. Hue and chroma of the color are specified
    /// in the design specification of the variant. Usually a different hue from
    /// primary and colorful.
    tertiary_palette: TonalPalette,

    /// Given a tone, produces a color. Hue and chroma of the color are specified
    /// in the design specification of the variant. Usually not colorful at all,
    /// intended for background & surface colors.
    neutral_palette: TonalPalette,

    /// Given a tone, produces a color. Hue and chroma of the color are specified
    /// in the design specification of the variant. Usually not colorful, but
    /// slightly more colorful than Neutral. Intended for backgrounds & surfaces.
    neutral_variant_palette: TonalPalette,

    /// Given a tone, produces a reddish, colorful, color.
    error_palette: TonalPalette,
}

impl DynamicScheme {
    /// Dart's `DynamicScheme(...)`, its named parameters in Dart's order. Dart's default for
    /// `contrastLevel` is 0.0, and an `error_palette` of `None` is `TonalPalette.of(25.0, 84.0)`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_color_hct: Hct,
        variant: Variant,
        contrast_level: f64,
        is_dark: bool,
        primary_palette: TonalPalette,
        secondary_palette: TonalPalette,
        tertiary_palette: TonalPalette,
        neutral_palette: TonalPalette,
        neutral_variant_palette: TonalPalette,
        error_palette: Option<TonalPalette>,
    ) -> DynamicScheme {
        DynamicScheme {
            id: DynamicSchemeId::next(),
            source_color_argb: source_color_hct.to_int(),
            source_color_hct,
            variant,
            is_dark,
            contrast_level,
            primary_palette,
            secondary_palette,
            tertiary_palette,
            neutral_palette,
            neutral_variant_palette,
            error_palette: error_palette.unwrap_or_else(|| TonalPalette::of(25.0, 84.0)),
        }
    }

    pub(crate) fn id(&self) -> DynamicSchemeId {
        self.id
    }

    /// The source color of the theme as an ARGB integer.
    pub fn source_color_argb(&self) -> u32 {
        self.source_color_argb
    }

    /// The source color of the theme in HCT.
    pub fn source_color_hct(&self) -> Hct {
        self.source_color_hct
    }

    /// The variant, or style, of the theme.
    pub fn variant(&self) -> Variant {
        self.variant
    }

    /// Whether or not the scheme is in 'dark mode' or 'light mode'.
    pub fn is_dark(&self) -> bool {
        self.is_dark
    }

    /// Value from -1 to 1. -1 represents minimum contrast, 0 represents
    /// standard (i.e. the design as spec'd), and 1 represents maximum contrast.
    pub fn contrast_level(&self) -> f64 {
        self.contrast_level
    }

    /// Given a tone, produces a color. Hue and chroma of the color are specified
    /// in the design specification of the variant. Usually colorful.
    pub fn primary_palette(&self) -> &TonalPalette {
        &self.primary_palette
    }

    /// Given a tone, produces a color. Hue and chroma of the color are specified
    /// in the design specification of the variant. Usually less colorful.
    pub fn secondary_palette(&self) -> &TonalPalette {
        &self.secondary_palette
    }

    /// Given a tone, produces a color. Hue and chroma of the color are specified
    /// in the design specification of the variant. Usually a different hue from
    /// primary and colorful.
    pub fn tertiary_palette(&self) -> &TonalPalette {
        &self.tertiary_palette
    }

    /// Given a tone, produces a color. Hue and chroma of the color are specified
    /// in the design specification of the variant. Usually not colorful at all,
    /// intended for background & surface colors.
    pub fn neutral_palette(&self) -> &TonalPalette {
        &self.neutral_palette
    }

    /// Given a tone, produces a color. Hue and chroma of the color are specified
    /// in the design specification of the variant. Usually not colorful, but
    /// slightly more colorful than Neutral. Intended for backgrounds & surfaces.
    pub fn neutral_variant_palette(&self) -> &TonalPalette {
        &self.neutral_variant_palette
    }

    /// Given a tone, produces a reddish, colorful, color.
    pub fn error_palette(&self) -> &TonalPalette {
        &self.error_palette
    }

    pub fn get_rotated_hue(source_color: &Hct, hues: &[f64], rotations: &[f64]) -> f64 {
        let source_hue = source_color.hue();
        debug_assert!(hues.len() == rotations.len());
        if rotations.len() == 1 {
            return MathUtils::sanitize_degrees_double(source_color.hue() + rotations[0]);
        }
        let size = hues.len() as i64;
        for i in 0..=(size - 2) {
            let this_hue = hues[i as usize];
            let next_hue = hues[i as usize + 1];
            if this_hue < source_hue && source_hue < next_hue {
                return MathUtils::sanitize_degrees_double(source_hue + rotations[i as usize]);
            }
        }
        // If this statement executes, something is wrong, there should have been a rotation
        // found using the arrays.
        source_hue
    }

    pub fn get_hct(&self, dynamic_color: &DynamicColor) -> Hct {
        dynamic_color.get_hct(self)
    }

    pub fn get_argb(&self, dynamic_color: &DynamicColor) -> u32 {
        dynamic_color.get_argb(self)
    }

    // Getters.
    pub fn primary_palette_key_color(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::primary_palette_key_color())
    }

    pub fn secondary_palette_key_color(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::secondary_palette_key_color())
    }

    pub fn tertiary_palette_key_color(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::tertiary_palette_key_color())
    }

    pub fn neutral_palette_key_color(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::neutral_palette_key_color())
    }

    pub fn neutral_variant_palette_key_color(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::neutral_variant_palette_key_color())
    }

    pub fn background(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::background())
    }

    pub fn on_background(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_background())
    }

    pub fn surface(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::surface())
    }

    pub fn surface_dim(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::surface_dim())
    }

    pub fn surface_bright(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::surface_bright())
    }

    pub fn surface_container_lowest(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::surface_container_lowest())
    }

    pub fn surface_container_low(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::surface_container_low())
    }

    pub fn surface_container(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::surface_container())
    }

    pub fn surface_container_high(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::surface_container_high())
    }

    pub fn surface_container_highest(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::surface_container_highest())
    }

    pub fn on_surface(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_surface())
    }

    pub fn surface_variant(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::surface_variant())
    }

    pub fn on_surface_variant(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_surface_variant())
    }

    pub fn inverse_surface(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::inverse_surface())
    }

    pub fn inverse_on_surface(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::inverse_on_surface())
    }

    pub fn outline(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::outline())
    }

    pub fn outline_variant(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::outline_variant())
    }

    pub fn shadow(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::shadow())
    }

    pub fn scrim(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::scrim())
    }

    pub fn surface_tint(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::surface_tint())
    }

    pub fn primary(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::primary())
    }

    pub fn on_primary(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_primary())
    }

    pub fn primary_container(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::primary_container())
    }

    pub fn on_primary_container(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_primary_container())
    }

    pub fn inverse_primary(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::inverse_primary())
    }

    pub fn secondary(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::secondary())
    }

    pub fn on_secondary(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_secondary())
    }

    pub fn secondary_container(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::secondary_container())
    }

    pub fn on_secondary_container(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_secondary_container())
    }

    pub fn tertiary(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::tertiary())
    }

    pub fn on_tertiary(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_tertiary())
    }

    pub fn tertiary_container(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::tertiary_container())
    }

    pub fn on_tertiary_container(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_tertiary_container())
    }

    pub fn error(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::error())
    }

    pub fn on_error(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_error())
    }

    pub fn error_container(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::error_container())
    }

    pub fn on_error_container(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_error_container())
    }

    pub fn primary_fixed(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::primary_fixed())
    }

    pub fn primary_fixed_dim(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::primary_fixed_dim())
    }

    pub fn on_primary_fixed(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_primary_fixed())
    }

    pub fn on_primary_fixed_variant(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_primary_fixed_variant())
    }

    pub fn secondary_fixed(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::secondary_fixed())
    }

    pub fn secondary_fixed_dim(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::secondary_fixed_dim())
    }

    pub fn on_secondary_fixed(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_secondary_fixed())
    }

    pub fn on_secondary_fixed_variant(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_secondary_fixed_variant())
    }

    pub fn tertiary_fixed(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::tertiary_fixed())
    }

    pub fn tertiary_fixed_dim(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::tertiary_fixed_dim())
    }

    pub fn on_tertiary_fixed(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_tertiary_fixed())
    }

    pub fn on_tertiary_fixed_variant(&self) -> u32 {
        self.get_argb(&MaterialDynamicColors::on_tertiary_fixed_variant())
    }
}

#[cfg(test)]
mod tests {
    // Ported from `test/dynamic_scheme_test.dart`.
    use super::*;

    fn assert_close_to(actual: f64, expected: f64, delta: f64) {
        assert!(
            (actual - expected).abs() <= delta,
            "expected {actual} to be within {delta} of {expected}"
        );
    }

    #[test]
    fn zero_length_input() {
        let hue = DynamicScheme::get_rotated_hue(&Hct::from(43.0, 16.0, 16.0), &[], &[]);
        assert_close_to(hue, 43.0, 1.0);
    }

    #[test]
    fn one_length_input_no_rotation() {
        let hue = DynamicScheme::get_rotated_hue(&Hct::from(43.0, 16.0, 16.0), &[0.0], &[0.0]);
        assert_close_to(hue, 43.0, 1.0);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn input_length_mismatch_asserts() {
        DynamicScheme::get_rotated_hue(&Hct::from(43.0, 16.0, 16.0), &[0.0, 1.0], &[0.0]);
    }

    #[test]
    fn on_boundary_rotation_correct() {
        let hue = DynamicScheme::get_rotated_hue(
            &Hct::from(43.0, 16.0, 16.0),
            &[0.0, 42.0, 360.0],
            &[0.0, 15.0, 0.0],
        );
        assert_close_to(hue, 43.0 + 15.0, 1.0);
    }

    #[test]
    fn rotation_result_larger_than_360_degrees_wraps() {
        let hue = DynamicScheme::get_rotated_hue(
            &Hct::from(43.0, 16.0, 16.0),
            &[0.0, 42.0, 360.0],
            &[0.0, 480.0, 0.0],
        );
        assert_close_to(hue, 163.0, 1.0);
    }
}
