// Copyright 2023 Google LLC
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
// Ported from `test/utils/scheme_from_variant.dart` of material_color_utilities 0.13.0.

use waft_material_color_utilities::{
    DynamicScheme, Hct, SchemeContent, SchemeExpressive, SchemeFidelity, SchemeFruitSalad,
    SchemeMonochrome, SchemeNeutral, SchemeRainbow, SchemeTonalSpot, SchemeVibrant, Variant,
};

pub fn scheme_from_variant(
    variant: Variant,
    source_color_hct: Hct,
    is_dark: bool,
    contrast_level: f64,
) -> DynamicScheme {
    match variant {
        Variant::Content => SchemeContent::new(source_color_hct, is_dark, contrast_level),
        Variant::Expressive => SchemeExpressive::new(source_color_hct, is_dark, contrast_level),
        Variant::Fidelity => SchemeFidelity::new(source_color_hct, is_dark, contrast_level),
        Variant::FruitSalad => SchemeFruitSalad::new(source_color_hct, is_dark, contrast_level),
        Variant::Monochrome => SchemeMonochrome::new(source_color_hct, is_dark, contrast_level),
        Variant::Neutral => SchemeNeutral::new(source_color_hct, is_dark, contrast_level),
        Variant::Rainbow => SchemeRainbow::new(source_color_hct, is_dark, contrast_level),
        Variant::TonalSpot => SchemeTonalSpot::new(source_color_hct, is_dark, contrast_level),
        Variant::Vibrant => SchemeVibrant::new(source_color_hct, is_dark, contrast_level),
    }
}
