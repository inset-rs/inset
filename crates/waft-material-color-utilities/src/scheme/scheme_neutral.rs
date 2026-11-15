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
// Ported from `lib/scheme/scheme_neutral.dart` of material_color_utilities 0.13.0.

use crate::dynamiccolor::dynamic_scheme::DynamicScheme;
use crate::dynamiccolor::variant::Variant;
use crate::hct::hct::Hct;
use crate::palettes::tonal_palette::TonalPalette;

/// A Dynamic Color theme that is near grayscale.
///
/// Dart's subclass of [`DynamicScheme`] that only fills its constructor: [`new`](Self::new)
/// makes the scheme, a [`DynamicScheme`] of this variant.
pub struct SchemeNeutral;

impl SchemeNeutral {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(source_color_hct: Hct, is_dark: bool, contrast_level: f64) -> DynamicScheme {
        DynamicScheme::new(
            source_color_hct,
            Variant::Neutral,
            contrast_level,
            is_dark,
            TonalPalette::of(source_color_hct.hue(), 12.0),
            TonalPalette::of(source_color_hct.hue(), 8.0),
            TonalPalette::of(source_color_hct.hue(), 16.0),
            TonalPalette::of(source_color_hct.hue(), 2.0),
            TonalPalette::of(source_color_hct.hue(), 2.0),
            None,
        )
    }
}
