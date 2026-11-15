// Copyright 2024 Google LLC
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
// Ported from `lib/palettes/core_palettes.dart` of material_color_utilities 0.13.0.

use crate::palettes::tonal_palette::TonalPalette;

/// Comprises foundational palettes to build a color scheme. Generated from a
/// source color, these palettes will then be part of a [`DynamicScheme`](crate::DynamicScheme)
/// together with appearance preferences.
pub struct CorePalettes {
    pub primary: TonalPalette,
    pub secondary: TonalPalette,
    pub tertiary: TonalPalette,
    pub neutral: TonalPalette,
    pub neutral_variant: TonalPalette,
}

impl CorePalettes {
    pub fn new(
        primary: TonalPalette,
        secondary: TonalPalette,
        tertiary: TonalPalette,
        neutral: TonalPalette,
        neutral_variant: TonalPalette,
    ) -> CorePalettes {
        CorePalettes {
            primary,
            secondary,
            tertiary,
            neutral,
            neutral_variant,
        }
    }
}
