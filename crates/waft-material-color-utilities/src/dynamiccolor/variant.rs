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
// Ported from `lib/dynamiccolor/variant.dart` of material_color_utilities 0.13.0.

/// Set of themes supported by Dynamic Color.
/// Instantiate the corresponding subclass, ex. SchemeTonalSpot, to create
/// colors corresponding to the theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Variant {
    Monochrome,
    Neutral,
    TonalSpot,
    Vibrant,
    Expressive,
    Content,
    Fidelity,
    Rainbow,
    FruitSalad,
}

impl Variant {
    /// Dart's `Variant.values`: every variant, in declaration order.
    pub const VALUES: [Variant; 9] = [
        Variant::Monochrome,
        Variant::Neutral,
        Variant::TonalSpot,
        Variant::Vibrant,
        Variant::Expressive,
        Variant::Content,
        Variant::Fidelity,
        Variant::Rainbow,
        Variant::FruitSalad,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Variant::Monochrome => "monochrome",
            Variant::Neutral => "neutral",
            Variant::TonalSpot => "tonal spot",
            Variant::Vibrant => "vibrant",
            Variant::Expressive => "expressive",
            Variant::Content => "content",
            Variant::Fidelity => "fidelity",
            Variant::Rainbow => "rainbow",
            Variant::FruitSalad => "fruit salad",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Variant::Monochrome => "All colors are grayscale, no chroma.",
            Variant::Neutral => "Close to grayscale, a hint of chroma.",
            Variant::TonalSpot => {
                "Pastel tokens, low chroma palettes (32).\nDefault Material You theme at 2021 launch."
            }
            Variant::Vibrant => {
                "Pastel colors, high chroma palettes. (max).\nThe primary palette's chroma is at maximum.\nUse Fidelity instead if tokens should alter their tone to match the palette vibrancy."
            }
            Variant::Expressive => {
                "Pastel colors, medium chroma palettes.\nThe primary palette's hue is different from source color, for variety."
            }
            Variant::Content => {
                "Almost identical to Fidelity.\nTokens and palettes match source color.\nPrimary Container is source color, adjusted to ensure contrast with surfaces.\n\nTertiary palette is analogue of source color.\nFound by dividing color wheel by 6, then finding the 2 colors adjacent to source.\nThe one that increases hue is used."
            }
            Variant::Fidelity => {
                "Tokens and palettes match source color.\nPrimary Container is source color, adjusted to ensure contrast with surfaces.\nFor example, if source color is black, it is lightened so it doesn't match surfaces in dark mode.\n\nTertiary palette is complement of source color."
            }
            Variant::Rainbow => {
                "A playful theme - the source color's hue does not appear in the theme."
            }
            Variant::FruitSalad => {
                "A playful theme - the source color's hue does not appear in the theme."
            }
        }
    }
}
