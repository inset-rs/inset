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
// Ported from `lib/scheme/scheme_vibrant.dart` of material_color_utilities 0.13.0.

use crate::dynamiccolor::dynamic_scheme::DynamicScheme;
use crate::dynamiccolor::variant::Variant;
use crate::hct::hct::Hct;
use crate::palettes::tonal_palette::TonalPalette;

/// A Dynamic Color theme that maxes out colorfulness at each position in the
/// Primary [`TonalPalette`].
///
/// Dart's subclass of [`DynamicScheme`] that only fills its constructor: [`new`](Self::new)
/// makes the scheme, a [`DynamicScheme`] of this variant.
pub struct SchemeVibrant;

impl SchemeVibrant {
    /// Hues used at breakpoints such that designers can specify a hue rotation
    /// that occurs at a given break point.
    const HUES: [f64; 9] = [0.0, 41.0, 61.0, 101.0, 131.0, 181.0, 251.0, 301.0, 360.0];

    /// Hue rotations of the Secondary [`TonalPalette`], corresponding to the
    /// breakpoints in `HUES`.
    const SECONDARY_ROTATIONS: [f64; 9] = [18.0, 15.0, 10.0, 12.0, 15.0, 18.0, 15.0, 12.0, 12.0];

    /// Hue rotations of the Tertiary [`TonalPalette`], corresponding to the
    /// breakpoints in `HUES`.
    const TERTIARY_ROTATIONS: [f64; 9] = [35.0, 30.0, 20.0, 25.0, 30.0, 35.0, 30.0, 25.0, 25.0];

    #[allow(clippy::new_ret_no_self)]
    pub fn new(source_color_hct: Hct, is_dark: bool, contrast_level: f64) -> DynamicScheme {
        DynamicScheme::new(
            source_color_hct,
            Variant::Vibrant,
            contrast_level,
            is_dark,
            TonalPalette::of(source_color_hct.hue(), 200.0),
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
            TonalPalette::of(source_color_hct.hue(), 10.0),
            TonalPalette::of(source_color_hct.hue(), 12.0),
            None,
        )
    }
}
