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
// Ported from `lib/quantize/quantizer_celebi.dart` of material_color_utilities 0.13.0.

use crate::quantize::quantizer::{Quantizer, QuantizerFuture};
use crate::quantize::quantizer_wsmeans::QuantizerWsmeans;
use crate::quantize::quantizer_wu::QuantizerWu;
use crate::quantize::src::point_provider_lab::PointProviderLab;

pub struct QuantizerCelebi;

impl QuantizerCelebi {
    /// Dart's `quantize` with its extra named parameter, `false` by default in Dart.
    pub fn quantize(
        &mut self,
        pixels: &[u32],
        max_colors: i64,
        return_input_pixel_to_cluster_pixel: bool,
    ) -> QuantizerFuture {
        let mut wu = QuantizerWu::new();
        let wu_result = wu.quantize(pixels, max_colors).into_inner();
        let starting_clusters: Vec<u32> = wu_result.color_to_count.keys().copied().collect();
        let wsmeans_result = QuantizerWsmeans::quantize(
            pixels,
            max_colors,
            &starting_clusters,
            &PointProviderLab,
            // Dart's default `maxIterations`.
            5,
            return_input_pixel_to_cluster_pixel,
        );
        std::future::ready(wsmeans_result)
    }
}

impl Quantizer for QuantizerCelebi {
    fn quantize(&mut self, pixels: &[u32], max_colors: i64) -> QuantizerFuture {
        QuantizerCelebi::quantize(self, pixels, max_colors, false)
    }
}

#[cfg(test)]
mod tests {
    // Ported from `test/quantizer_celebi_test.dart`.
    use std::collections::HashSet;

    use super::*;

    const RED: u32 = 0xffff0000;
    const GREEN: u32 = 0xff00ff00;
    const BLUE: u32 = 0xff0000ff;
    const MAX_COLORS: i64 = 256;

    fn colors_of(pixels: &[u32]) -> Vec<u32> {
        let mut celebi = QuantizerCelebi;
        let result = Quantizer::quantize(&mut celebi, pixels, MAX_COLORS).into_inner();
        result.color_to_count.keys().copied().collect()
    }

    #[test]
    fn one_red() {
        let colors = colors_of(&[RED]);
        assert_eq!(colors.len(), 1);
        assert_eq!(colors[0], RED);
    }

    #[test]
    fn one_green() {
        let colors = colors_of(&[GREEN]);
        assert_eq!(colors.len(), 1);
        assert_eq!(colors[0], GREEN);
    }

    #[test]
    fn one_blue() {
        let colors = colors_of(&[BLUE]);
        assert_eq!(colors.len(), 1);
        assert_eq!(colors[0], BLUE);
    }

    #[test]
    fn five_blue() {
        let colors = colors_of(&[BLUE, BLUE, BLUE, BLUE, BLUE]);
        assert_eq!(colors.len(), 1);
        assert_eq!(colors[0], BLUE);
    }

    #[test]
    fn one_red_one_green_one_blue() {
        let colors = colors_of(&[RED, GREEN, BLUE]);

        assert_eq!(colors.iter().collect::<HashSet<_>>().len(), 3);
        assert_eq!(colors[0], BLUE);
        assert_eq!(colors[1], RED);
        assert_eq!(colors[2], GREEN);
    }

    #[test]
    fn two_red_three_green() {
        let colors = colors_of(&[RED, RED, GREEN, GREEN, GREEN]);
        assert_eq!(colors.iter().collect::<HashSet<_>>().len(), 2);

        assert_eq!(colors[0], GREEN);
        assert_eq!(colors[1], RED);
    }

    // Not in the package's tests: a synthetic 4096-pixel image through Wu, Wsmeans and Score.
    // The expected values are the Dart package's, printed on Dart 3.13.1 by
    // `QuantizerCelebi().quantize(image, 128)` and `Score.score(colorToCount, desired: 4)`.
    #[test]
    fn a_synthetic_image_gives_dart_s_clusters_and_scores() {
        let mut s: i64 = 12345;
        let mut image: Vec<u32> = Vec::new();
        for _ in 0..4096 {
            s = (s * 1103515245 + 12345) & 0x7fffffff;
            image.push(0xff000000 | (s & 0xffffff) as u32);
        }
        let mut celebi = QuantizerCelebi;
        let result = Quantizer::quantize(&mut celebi, &image, 128).into_inner();
        assert_eq!(result.color_to_count.len(), 128);
        let first: Vec<(u32, i64)> = result
            .color_to_count
            .iter()
            .take(8)
            .map(|(k, v)| (*k, *v))
            .collect();
        assert_eq!(
            first,
            [
                (0xff302030, 45),
                (0xff25a62a, 55),
                (0xff271c8e, 34),
                (0xff39a685, 52),
                (0xffad202f, 55),
                (0xff8da098, 55),
                (0xffbc25ad, 60),
                (0xff969d28, 60),
            ]
        );
        let ranked = crate::score::score::Score::score(&result.color_to_count, 4, 0xFF4285F4, true);
        assert_eq!(ranked, [0xff31f128, 0xffe223eb, 0xff3aa4df, 0xffd38e64]);
    }
}
