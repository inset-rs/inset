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
// Ported from `lib/scheme/scheme_monochrome.dart` of material_color_utilities 0.13.0.

use crate::dynamiccolor::dynamic_scheme::DynamicScheme;
use crate::dynamiccolor::variant::Variant;
use crate::hct::hct::Hct;
use crate::palettes::tonal_palette::TonalPalette;

/// A Dynamic Color theme that is grayscale.
///
/// Dart's subclass of [`DynamicScheme`] that only fills its constructor: [`new`](Self::new)
/// makes the scheme, a [`DynamicScheme`] of this variant.
pub struct SchemeMonochrome;

impl SchemeMonochrome {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(source_color_hct: Hct, is_dark: bool, contrast_level: f64) -> DynamicScheme {
        DynamicScheme::new(
            source_color_hct,
            Variant::Monochrome,
            contrast_level,
            is_dark,
            TonalPalette::of(source_color_hct.hue(), 0.0),
            TonalPalette::of(source_color_hct.hue(), 0.0),
            TonalPalette::of(source_color_hct.hue(), 0.0),
            TonalPalette::of(source_color_hct.hue(), 0.0),
            TonalPalette::of(source_color_hct.hue(), 0.0),
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    // Ported from `test/scheme_monochrome_test.dart`.
    use std::rc::Rc;

    use super::*;
    use crate::dynamiccolor::dynamic_color::DynamicColor;
    use crate::dynamiccolor::material_dynamic_colors::MaterialDynamicColors;

    fn assert_tones(scheme: &DynamicScheme, expected: &[(Rc<DynamicColor>, f64)]) {
        for (color, tone) in expected {
            let actual = color.get_hct(scheme).tone();
            assert!(
                (actual - tone).abs() <= 1.0,
                "{}: expected {actual} to be within 1 of {tone}",
                color.name
            );
        }
    }

    #[test]
    fn dark_theme_monochrome_spec() {
        let scheme = SchemeMonochrome::new(Hct::from_int(0xff0000ff), true, 0.0);
        assert_tones(
            &scheme,
            &[
                (MaterialDynamicColors::primary(), 100.0),
                (MaterialDynamicColors::on_primary(), 10.0),
                (MaterialDynamicColors::primary_container(), 85.0),
                (MaterialDynamicColors::on_primary_container(), 0.0),
                (MaterialDynamicColors::secondary(), 80.0),
                (MaterialDynamicColors::on_secondary(), 10.0),
                (MaterialDynamicColors::secondary_container(), 30.0),
                (MaterialDynamicColors::on_secondary_container(), 90.0),
                (MaterialDynamicColors::tertiary(), 90.0),
                (MaterialDynamicColors::on_tertiary(), 10.0),
                (MaterialDynamicColors::tertiary_container(), 60.0),
                (MaterialDynamicColors::on_tertiary_container(), 0.0),
            ],
        );
    }

    #[test]
    fn light_theme_monochrome_spec() {
        let scheme = SchemeMonochrome::new(Hct::from_int(0xff0000ff), false, 0.0);
        assert_tones(
            &scheme,
            &[
                (MaterialDynamicColors::primary(), 0.0),
                (MaterialDynamicColors::on_primary(), 90.0),
                (MaterialDynamicColors::primary_container(), 25.0),
                (MaterialDynamicColors::on_primary_container(), 100.0),
                (MaterialDynamicColors::secondary(), 40.0),
                (MaterialDynamicColors::on_secondary(), 100.0),
                (MaterialDynamicColors::secondary_container(), 85.0),
                (MaterialDynamicColors::on_secondary_container(), 10.0),
                (MaterialDynamicColors::tertiary(), 25.0),
                (MaterialDynamicColors::on_tertiary(), 90.0),
                (MaterialDynamicColors::tertiary_container(), 49.0),
                (MaterialDynamicColors::on_tertiary_container(), 100.0),
            ],
        );
    }
}
