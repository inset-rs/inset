// Copyright 2021 Google LLC
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
// Ported from `lib/scheme/scheme.dart` of material_color_utilities 0.13.0.

// The deprecated type's own body names it.
#![allow(deprecated)]

use crate::palettes::core_palette::CorePalette;

/// Prefer [`DynamicScheme`](crate::DynamicScheme). This class is the same concept as Flutter's
/// ColorScheme class, inlined to ensure parity across languages.
#[deprecated(
    note = "The `Scheme` class is deprecated in favor of `DynamicScheme`.\nPlease see https://github.com/material-foundation/material-color-utilities/blob/main/make_schemes.mdfor migration guidance."
)]
#[derive(Clone, Copy, Debug)]
pub struct Scheme {
    pub primary: u32,
    pub on_primary: u32,
    pub primary_container: u32,
    pub on_primary_container: u32,
    pub secondary: u32,
    pub on_secondary: u32,
    pub secondary_container: u32,
    pub on_secondary_container: u32,
    pub tertiary: u32,
    pub on_tertiary: u32,
    pub tertiary_container: u32,
    pub on_tertiary_container: u32,
    pub error: u32,
    pub on_error: u32,
    pub error_container: u32,
    pub on_error_container: u32,
    pub background: u32,
    pub on_background: u32,
    pub surface: u32,
    pub on_surface: u32,
    pub surface_variant: u32,
    pub on_surface_variant: u32,
    pub outline: u32,
    pub outline_variant: u32,
    pub shadow: u32,
    pub scrim: u32,
    pub inverse_surface: u32,
    pub inverse_on_surface: u32,
    pub inverse_primary: u32,
}

impl Scheme {
    pub fn light(color: u32) -> Scheme {
        Self::light_from_core_palette(&CorePalette::of(color))
    }

    pub fn dark(color: u32) -> Scheme {
        Self::dark_from_core_palette(&CorePalette::of(color))
    }

    pub fn light_content(color: u32) -> Scheme {
        Self::light_from_core_palette(&CorePalette::content_of(color))
    }

    pub fn dark_content(color: u32) -> Scheme {
        Self::dark_from_core_palette(&CorePalette::content_of(color))
    }

    pub fn light_from_core_palette(palette: &CorePalette) -> Scheme {
        Scheme {
            primary: palette.primary.get(40),
            on_primary: palette.primary.get(100),
            primary_container: palette.primary.get(90),
            on_primary_container: palette.primary.get(10),
            secondary: palette.secondary.get(40),
            on_secondary: palette.secondary.get(100),
            secondary_container: palette.secondary.get(90),
            on_secondary_container: palette.secondary.get(10),
            tertiary: palette.tertiary.get(40),
            on_tertiary: palette.tertiary.get(100),
            tertiary_container: palette.tertiary.get(90),
            on_tertiary_container: palette.tertiary.get(10),
            error: palette.error.get(40),
            on_error: palette.error.get(100),
            error_container: palette.error.get(90),
            on_error_container: palette.error.get(10),
            background: palette.neutral.get(99),
            on_background: palette.neutral.get(10),
            surface: palette.neutral.get(99),
            on_surface: palette.neutral.get(10),
            surface_variant: palette.neutral_variant.get(90),
            on_surface_variant: palette.neutral_variant.get(30),
            outline: palette.neutral_variant.get(50),
            outline_variant: palette.neutral_variant.get(80),
            shadow: palette.neutral.get(0),
            scrim: palette.neutral.get(0),
            inverse_surface: palette.neutral.get(20),
            inverse_on_surface: palette.neutral.get(95),
            inverse_primary: palette.primary.get(80),
        }
    }

    pub fn dark_from_core_palette(palette: &CorePalette) -> Scheme {
        Scheme {
            primary: palette.primary.get(80),
            on_primary: palette.primary.get(20),
            primary_container: palette.primary.get(30),
            on_primary_container: palette.primary.get(90),
            secondary: palette.secondary.get(80),
            on_secondary: palette.secondary.get(20),
            secondary_container: palette.secondary.get(30),
            on_secondary_container: palette.secondary.get(90),
            tertiary: palette.tertiary.get(80),
            on_tertiary: palette.tertiary.get(20),
            tertiary_container: palette.tertiary.get(30),
            on_tertiary_container: palette.tertiary.get(90),
            error: palette.error.get(80),
            on_error: palette.error.get(20),
            error_container: palette.error.get(30),
            on_error_container: palette.error.get(80),
            background: palette.neutral.get(10),
            on_background: palette.neutral.get(90),
            surface: palette.neutral.get(10),
            on_surface: palette.neutral.get(90),
            surface_variant: palette.neutral_variant.get(30),
            on_surface_variant: palette.neutral_variant.get(80),
            outline: palette.neutral_variant.get(60),
            outline_variant: palette.neutral_variant.get(30),
            shadow: palette.neutral.get(0),
            scrim: palette.neutral.get(0),
            inverse_surface: palette.neutral.get(90),
            inverse_on_surface: palette.neutral.get(20),
            inverse_primary: palette.primary.get(40),
        }
    }
}
