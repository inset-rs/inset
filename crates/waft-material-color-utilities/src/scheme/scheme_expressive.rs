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
// Ported from `lib/scheme/scheme_expressive.dart` of material_color_utilities 0.13.0.

use crate::dynamiccolor::dynamic_scheme::DynamicScheme;
use crate::dynamiccolor::variant::Variant;
use crate::hct::hct::Hct;
use crate::palettes::tonal_palette::TonalPalette;
use crate::utils::math_utils::MathUtils;

/// A Dynamic Color theme that is intentionally detached from the input color.
///
/// Dart's subclass of [`DynamicScheme`] that only fills its constructor: [`new`](Self::new)
/// makes the scheme, a [`DynamicScheme`] of this variant.
pub struct SchemeExpressive;

impl SchemeExpressive {
    /// Hues used at breakpoints such that designers can specify a hue rotation
    /// that occurs at a given break point.
    const HUES: [f64; 9] = [0.0, 21.0, 51.0, 121.0, 151.0, 191.0, 271.0, 321.0, 360.0];

    /// Hue rotations of the Secondary [`TonalPalette`], corresponding to the
    /// breakpoints in `HUES`.
    const SECONDARY_ROTATIONS: [f64; 9] = [45.0, 95.0, 45.0, 20.0, 45.0, 90.0, 45.0, 45.0, 45.0];

    /// Hue rotations of the Tertiary [`TonalPalette`], corresponding to the
    /// breakpoints in `HUES`.
    const TERTIARY_ROTATIONS: [f64; 9] = [120.0, 120.0, 20.0, 45.0, 20.0, 15.0, 20.0, 120.0, 120.0];

    #[allow(clippy::new_ret_no_self)]
    pub fn new(source_color_hct: Hct, is_dark: bool, contrast_level: f64) -> DynamicScheme {
        DynamicScheme::new(
            source_color_hct,
            Variant::Expressive,
            contrast_level,
            is_dark,
            TonalPalette::of(
                MathUtils::sanitize_degrees_double(source_color_hct.hue() + 240.0),
                40.0,
            ),
            TonalPalette::of(
                DynamicScheme::get_rotated_hue(
                    &source_color_hct,
                    &Self::HUES,
                    &Self::SECONDARY_ROTATIONS,
                ),
                24.0,
            ),
            TonalPalette::of(
                DynamicScheme::get_rotated_hue(
                    &source_color_hct,
                    &Self::HUES,
                    &Self::TERTIARY_ROTATIONS,
                ),
                32.0,
            ),
            TonalPalette::of(source_color_hct.hue() + 15.0, 8.0),
            TonalPalette::of(source_color_hct.hue() + 15.0, 12.0),
            None,
        )
    }
}
