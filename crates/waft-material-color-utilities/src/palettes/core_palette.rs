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
// Ported from `lib/palettes/core_palette.dart` of material_color_utilities 0.13.0.

// The deprecated type's own body and tests name it.
#![allow(deprecated)]

use std::fmt;
use std::hash::{Hash, Hasher};

use crate::dart_math;
use crate::hct::cam16::Cam16;
use crate::palettes::tonal_palette::TonalPalette;

/// An intermediate concept between the key color for a UI theme, and a full
/// color scheme. 5 tonal palettes are generated, all except one use the same
/// hue as the key color, and all vary in chroma.
#[deprecated(
    note = "Use `DynamicScheme` for color scheme generation.Use `CorePalettes` for core palettes container class."
)]
pub struct CorePalette {
    pub primary: TonalPalette,
    pub secondary: TonalPalette,
    pub tertiary: TonalPalette,
    pub neutral: TonalPalette,
    pub neutral_variant: TonalPalette,
    pub error: TonalPalette,
}

impl CorePalette {
    /// The number of generated tonal palettes.
    pub const SIZE: usize = 5;

    /// Create a [`CorePalette`] from a source ARGB color.
    #[deprecated(
        note = "Use `DynamicScheme` for color scheme generation.Use `CorePalettes` for core palettes container class."
    )]
    pub fn of(argb: u32) -> CorePalette {
        let cam = Cam16::from_int(argb);
        CorePalette::new(cam.hue, cam.chroma)
    }

    fn new(hue: f64, chroma: f64) -> CorePalette {
        CorePalette {
            primary: TonalPalette::of(hue, dart_math::max(48.0, chroma)),
            secondary: TonalPalette::of(hue, 16.0),
            tertiary: TonalPalette::of(hue + 60.0, 24.0),
            neutral: TonalPalette::of(hue, 4.0),
            neutral_variant: TonalPalette::of(hue, 8.0),
            error: TonalPalette::of(25.0, 84.0),
        }
    }

    /// Create a content [`CorePalette`] from a source ARGB color.
    pub fn content_of(argb: u32) -> CorePalette {
        let cam = Cam16::from_int(argb);
        CorePalette::new_content_of(cam.hue, cam.chroma)
    }

    fn new_content_of(hue: f64, chroma: f64) -> CorePalette {
        CorePalette {
            primary: TonalPalette::of(hue, chroma),
            secondary: TonalPalette::of(hue, chroma / 3.0),
            tertiary: TonalPalette::of(hue + 60.0, chroma / 2.0),
            neutral: TonalPalette::of(hue, dart_math::min(chroma / 12.0, 4.0)),
            neutral_variant: TonalPalette::of(hue, dart_math::min(chroma / 6.0, 8.0)),
            error: TonalPalette::of(25.0, 84.0),
        }
    }

    /// Create a [`CorePalette`] from a fixed-size list of ARGB color ints
    /// representing concatenated tonal palettes.
    ///
    /// Inverse of [`as_list`](Self::as_list).
    #[deprecated(
        note = "Use `DynamicScheme` for color scheme generation.Use `CorePalettes` for core palettes container class."
    )]
    pub fn from_list(colors: &[u32]) -> CorePalette {
        debug_assert!(colors.len() == Self::SIZE * TonalPalette::COMMON_SIZE);
        CorePalette {
            primary: TonalPalette::from_list(get_partition(colors, 0, TonalPalette::COMMON_SIZE)),
            secondary: TonalPalette::from_list(get_partition(colors, 1, TonalPalette::COMMON_SIZE)),
            tertiary: TonalPalette::from_list(get_partition(colors, 2, TonalPalette::COMMON_SIZE)),
            neutral: TonalPalette::from_list(get_partition(colors, 3, TonalPalette::COMMON_SIZE)),
            neutral_variant: TonalPalette::from_list(get_partition(
                colors,
                4,
                TonalPalette::COMMON_SIZE,
            )),
            error: TonalPalette::of(25.0, 84.0),
        }
    }

    /// Returns a list of ARGB color [`u32`]s from concatenated tonal palettes.
    ///
    /// Inverse of [`CorePalette::from_list`].
    #[deprecated(
        note = "Use `DynamicScheme` for color scheme generation.Use `CorePalettes` for core palettes container class."
    )]
    pub fn as_list(&self) -> Vec<u32> {
        [
            self.primary.as_list(),
            self.secondary.as_list(),
            self.tertiary.as_list(),
            self.neutral.as_list(),
            self.neutral_variant.as_list(),
        ]
        .concat()
    }
}

impl PartialEq for CorePalette {
    fn eq(&self, other: &Self) -> bool {
        self.primary == other.primary
            && self.secondary == other.secondary
            && self.tertiary == other.tertiary
            && self.neutral == other.neutral
            && self.neutral_variant == other.neutral_variant
            && self.error == other.error
    }
}

impl Hash for CorePalette {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.primary.hash(state);
        self.secondary.hash(state);
        self.tertiary.hash(state);
        self.neutral.hash(state);
        self.neutral_variant.hash(state);
        self.error.hash(state);
    }
}

impl fmt::Debug for CorePalette {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "primary: {:?}", self.primary)?;
        writeln!(f, "secondary: {:?}", self.secondary)?;
        writeln!(f, "tertiary: {:?}", self.tertiary)?;
        writeln!(f, "neutral: {:?}", self.neutral)?;
        writeln!(f, "neutralVariant: {:?}", self.neutral_variant)?;
        writeln!(f, "error: {:?}", self.error)
    }
}

// Returns a partition from a list.
//
// For example, given a list with 2 partitions of size 3.
// range = [1, 2, 3, 4, 5, 6];
//
// range.getPartition(0, 3) // [1, 2, 3]
// range.getPartition(1, 3) // [4, 5, 6]
fn get_partition(list: &[u32], partition_number: usize, partition_size: usize) -> &[u32] {
    &list[partition_number * partition_size..(partition_number + 1) * partition_size]
}

#[cfg(test)]
mod tests {
    // Ported from the `CorePalette` group of `test/palettes_test.dart`.
    use std::collections::hash_map::DefaultHasher;

    use super::*;

    fn hash_of(core_palette: &CorePalette) -> u64 {
        let mut hasher = DefaultHasher::new();
        core_palette.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn as_list() {
        let ints: Vec<u32> = (0..(CorePalette::SIZE * TonalPalette::COMMON_SIZE) as u32).collect();
        let core_palette = CorePalette::from_list(&ints);
        assert_eq!(core_palette.as_list(), ints);
    }

    #[test]
    fn operator_equals_and_hash_code() {
        let core_palette_a = CorePalette::of(0xff0000ff);
        let core_palette_b = CorePalette::of(0xff0000ff);
        let core_palette_c = CorePalette::of(0xff123456);

        assert!(core_palette_a == core_palette_b);
        assert!(core_palette_b != core_palette_c);

        assert_eq!(hash_of(&core_palette_a), hash_of(&core_palette_b));
        assert_ne!(hash_of(&core_palette_b), hash_of(&core_palette_c));
    }

    #[test]
    fn of_blue() {
        let core = CorePalette::of(0xff0000ff);

        assert_eq!(core.primary.get(100), 0xffffffff);
        assert_eq!(core.primary.get(95), 0xfff1efff);
        assert_eq!(core.primary.get(90), 0xffe0e0ff);
        assert_eq!(core.primary.get(80), 0xffbec2ff);
        assert_eq!(core.primary.get(70), 0xff9da3ff);
        assert_eq!(core.primary.get(60), 0xff7c84ff);
        assert_eq!(core.primary.get(50), 0xff5a64ff);
        assert_eq!(core.primary.get(40), 0xff343dff);
        assert_eq!(core.primary.get(30), 0xff0000ef);
        assert_eq!(core.primary.get(20), 0xff0001ac);
        assert_eq!(core.primary.get(10), 0xff00006e);
        assert_eq!(core.primary.get(0), 0xff000000);
        assert_eq!(core.secondary.get(100), 0xffffffff);
        assert_eq!(core.secondary.get(95), 0xfff1efff);
        assert_eq!(core.secondary.get(90), 0xffe1e0f9);
        assert_eq!(core.secondary.get(80), 0xffc5c4dd);
        assert_eq!(core.secondary.get(70), 0xffa9a9c1);
        assert_eq!(core.secondary.get(60), 0xff8f8fa6);
        assert_eq!(core.secondary.get(50), 0xff75758b);
        assert_eq!(core.secondary.get(40), 0xff5c5d72);
        assert_eq!(core.secondary.get(30), 0xff444559);
        assert_eq!(core.secondary.get(20), 0xff2e2f42);
        assert_eq!(core.secondary.get(10), 0xff191a2c);
        assert_eq!(core.secondary.get(0), 0xff000000);
    }

    #[test]
    fn content_of_blue() {
        let core = CorePalette::content_of(0xff0000ff);

        assert_eq!(core.primary.get(100), 0xffffffff);
        assert_eq!(core.primary.get(95), 0xfff1efff);
        assert_eq!(core.primary.get(90), 0xffe0e0ff);
        assert_eq!(core.primary.get(80), 0xffbec2ff);
        assert_eq!(core.primary.get(70), 0xff9da3ff);
        assert_eq!(core.primary.get(60), 0xff7c84ff);
        assert_eq!(core.primary.get(50), 0xff5a64ff);
        assert_eq!(core.primary.get(40), 0xff343dff);
        assert_eq!(core.primary.get(30), 0xff0000ef);
        assert_eq!(core.primary.get(20), 0xff0001ac);
        assert_eq!(core.primary.get(10), 0xff00006e);
        assert_eq!(core.primary.get(0), 0xff000000);
        assert_eq!(core.secondary.get(100), 0xffffffff);
        assert_eq!(core.secondary.get(95), 0xfff1efff);
        assert_eq!(core.secondary.get(90), 0xffe0e0ff);
        assert_eq!(core.secondary.get(80), 0xffc1c3f4);
        assert_eq!(core.secondary.get(70), 0xffa5a7d7);
        assert_eq!(core.secondary.get(60), 0xff8b8dbb);
        assert_eq!(core.secondary.get(50), 0xff7173a0);
        assert_eq!(core.secondary.get(40), 0xff585b86);
        assert_eq!(core.secondary.get(30), 0xff40436d);
        assert_eq!(core.secondary.get(20), 0xff2a2d55);
        assert_eq!(core.secondary.get(10), 0xff14173f);
        assert_eq!(core.secondary.get(0), 0xff000000);
    }
}
