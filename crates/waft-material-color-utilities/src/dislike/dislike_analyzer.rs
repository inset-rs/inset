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
// Ported from `lib/dislike/dislike_analyzer.dart` of material_color_utilities 0.13.0.

use crate::dart_math;
use crate::hct::hct::Hct;

/// Check and/or fix universally disliked colors.
///
/// Color science studies of color preference indicate universal distaste for
/// dark yellow-greens, and also show this is correlated to distate for
/// biological waste and rotting food.
///
/// See Palmer and Schloss, 2010 or Schloss and Palmer's Chapter 21 in Handbook
/// of Color Psychology (2015).
pub struct DislikeAnalyzer;

impl DislikeAnalyzer {
    /// Returns true if `hct` is disliked.
    ///
    /// Disliked is defined as a dark yellow-green that is not neutral.
    pub fn is_disliked(hct: &Hct) -> bool {
        let hue_passes = dart_math::round(hct.hue()) >= 90 && dart_math::round(hct.hue()) <= 111;
        let chroma_passes = dart_math::round(hct.chroma()) > 16;
        let tone_passes = dart_math::round(hct.tone()) < 65;

        hue_passes && chroma_passes && tone_passes
    }

    /// If `hct` is disliked, lighten it to make it likable.
    pub fn fix_if_disliked(hct: Hct) -> Hct {
        if Self::is_disliked(&hct) {
            return Hct::from(hct.hue(), hct.chroma(), 70.0);
        }

        hct
    }
}

#[cfg(test)]
mod tests {
    // Ported from `test/dislike_analyzer_test.dart`.
    use super::*;

    #[test]
    fn monk_skin_tone_scale_colors_liked() {
        // From https://skintone.google#/get-started
        let monk_skin_tone_scale_colors: [u32; 10] = [
            0xfff6ede4, 0xfff3e7db, 0xfff7ead0, 0xffeadaba, 0xffd7bd96, 0xffa07e56, 0xff825c43,
            0xff604134, 0xff3a312a, 0xff292420,
        ];
        for color in monk_skin_tone_scale_colors {
            assert!(!DislikeAnalyzer::is_disliked(&Hct::from_int(color)));
        }
    }

    #[test]
    fn bile_colors_disliked() {
        let unlikable: [u32; 5] = [0xFF95884B, 0xFF716B40, 0xFFB08E00, 0xFF4C4308, 0xff464521];
        for color in unlikable {
            assert!(
                DislikeAnalyzer::is_disliked(&Hct::from_int(color)),
                "{color} was likable"
            );
        }
    }

    #[test]
    fn bile_colors_became_likable() {
        let unlikable: [u32; 5] = [0xFF95884B, 0xFF716B40, 0xFFB08E00, 0xFF4C4308, 0xff464521];
        for color in unlikable {
            let hct = Hct::from_int(color);
            assert!(DislikeAnalyzer::is_disliked(&hct));
            let likable = DislikeAnalyzer::fix_if_disliked(hct);
            assert!(!DislikeAnalyzer::is_disliked(&likable));
        }
    }

    #[test]
    fn tone_67_not_disliked() {
        let color = Hct::from(100.0, 50.0, 67.0);
        assert!(!DislikeAnalyzer::is_disliked(&color));
        assert_eq!(
            DislikeAnalyzer::fix_if_disliked(color).to_int(),
            color.to_int()
        );
    }
}
