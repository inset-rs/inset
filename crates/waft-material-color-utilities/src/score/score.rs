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
// Ported from `lib/score/score.dart` of material_color_utilities 0.13.0.

use std::cmp::Ordering;

use indexmap::IndexMap;

use crate::dart_math;
use crate::hct::hct::Hct;
use crate::utils::math_utils::MathUtils;

#[derive(Clone)]
struct ScoredHct {
    hct: Hct,
    score: f64,
}

impl ScoredHct {
    fn new(hct: Hct, score: f64) -> ScoredHct {
        ScoredHct { hct, score }
    }

    fn compare_to(&self, other: &ScoredHct) -> Ordering {
        if self.score > other.score {
            Ordering::Less
        } else if self.score == other.score {
            Ordering::Equal
        } else {
            Ordering::Greater
        }
    }
}

/// Given a large set of colors, remove colors that are unsuitable for a UI
/// theme, and rank the rest based on suitability.
///
/// Enables use of a high cluster count for image quantization, thus ensuring
/// colors aren't muddied, while curating the high cluster count to a much
///  smaller number of appropriate choices.
pub struct Score;

impl Score {
    const TARGET_CHROMA: f64 = 48.0; // A1 Chroma
    const WEIGHT_PROPORTION: f64 = 0.7;
    const WEIGHT_CHROMA_ABOVE: f64 = 0.3;
    const WEIGHT_CHROMA_BELOW: f64 = 0.1;
    const CUTOFF_CHROMA: f64 = 5.0;
    const CUTOFF_EXCITED_PROPORTION: f64 = 0.01;

    /// Given a map with keys of colors and values of how often the color appears,
    /// rank the colors based on suitability for being used for a UI theme.
    ///
    /// `colors_to_population` is a map with keys of colors and values of often the
    /// color appears, usually from a source image.
    /// `desired` max count of colors to be returned in the list; 4 in Dart by default.
    /// `fallback_color_argb` color to be returned if no other options available;
    /// `0xFF4285F4` in Dart by default.
    /// `filter` whether to filter out undesireable combinations; `true` in Dart by default.
    ///
    /// The list returned is of length <= `desired`. The recommended color is the
    /// first item, the least suitable is the last. There will always be at least
    /// one color returned. If all the input colors were not suitable for a theme,
    /// a default fallback color will be provided, Google Blue. The default
    /// number of colors returned is 4, simply because thats the # of colors
    /// display in Android 12's wallpaper picker.
    pub fn score(
        colors_to_population: &IndexMap<u32, i64>,
        desired: i64,
        fallback_color_argb: u32,
        filter: bool,
    ) -> Vec<u32> {
        // Get the HCT color for each Argb value, while finding the per hue count and
        // total count.
        let mut colors_hct: Vec<Hct> = Vec::new();
        let mut hue_population: Vec<i64> = vec![0; 360];
        let mut population_sum: i64 = 0;
        for (argb, population) in colors_to_population {
            let hct = Hct::from_int(*argb);
            colors_hct.push(hct);
            let hue = dart_math::floor(hct.hue()) as usize;
            hue_population[hue] = hue_population[hue].wrapping_add(*population);
            population_sum = population_sum.wrapping_add(*population);
        }

        // Hues with more usage in neighboring 30 degree slice get a larger number.
        let mut hue_excited_proportions: Vec<f64> = vec![0.0; 360];
        for hue in 0..360 {
            let proportion = hue_population[hue as usize] as f64 / population_sum as f64;
            for i in (hue - 14)..(hue + 16) {
                let neighbor_hue = MathUtils::sanitize_degrees_int(i);
                hue_excited_proportions[neighbor_hue as usize] += proportion;
            }
        }

        // Scores each HCT color based on usage and chroma, while optionally
        // filtering out values that do not have enough chroma or usage.
        let mut scored_hcts: Vec<ScoredHct> = Vec::new();
        for hct in colors_hct {
            let hue = MathUtils::sanitize_degrees_int(dart_math::round(hct.hue()));
            let proportion = hue_excited_proportions[hue as usize];
            if filter
                && (hct.chroma() < Self::CUTOFF_CHROMA
                    || proportion <= Self::CUTOFF_EXCITED_PROPORTION)
            {
                continue;
            }

            let proportion_score = proportion * 100.0 * Self::WEIGHT_PROPORTION;
            let chroma_weight = if hct.chroma() < Self::TARGET_CHROMA {
                Self::WEIGHT_CHROMA_BELOW
            } else {
                Self::WEIGHT_CHROMA_ABOVE
            };
            let chroma_score = (hct.chroma() - Self::TARGET_CHROMA) * chroma_weight;
            let score = proportion_score + chroma_score;
            scored_hcts.push(ScoredHct::new(hct, score));
        }
        // Sorted so that colors with higher scores come first.
        dart_math::sort(&mut scored_hcts, ScoredHct::compare_to);

        // Iterates through potential hue differences in degrees in order to select
        // the colors with the largest distribution of hues possible. Starting at
        // 90 degrees(maximum difference for 4 colors) then decreasing down to a
        // 15 degree minimum.
        let mut chosen_colors: Vec<Hct> = Vec::new();
        for difference_degrees in (15..=90).rev() {
            chosen_colors.clear();
            for entry in &scored_hcts {
                let hct = entry.hct;
                let duplicate_hue = chosen_colors.iter().any(|chosen_hct| {
                    MathUtils::difference_degrees(hct.hue(), chosen_hct.hue())
                        < difference_degrees as f64
                });
                if !duplicate_hue {
                    chosen_colors.push(hct);
                }
                if chosen_colors.len() as i64 >= desired {
                    break;
                }
            }
            if chosen_colors.len() as i64 >= desired {
                break;
            }
        }
        let mut colors: Vec<u32> = Vec::new();
        if chosen_colors.is_empty() {
            colors.push(fallback_color_argb);
        }
        for chosen_hct in chosen_colors {
            colors.push(chosen_hct.to_int());
        }
        colors
    }
}

#[cfg(test)]
mod tests {
    // Ported from `test/score_test.dart`.
    use super::*;
    use crate::test_support::is_color;

    const DEFAULT_FALLBACK: u32 = 0xFF4285F4;

    fn ranked(entries: &[(u32, i64)], desired: i64, fallback: u32, filter: bool) -> Vec<u32> {
        let colors_to_population: IndexMap<u32, i64> = entries.iter().copied().collect();
        Score::score(&colors_to_population, desired, fallback, filter)
    }

    fn assert_ranked(ranked: &[u32], expected: &[u32]) {
        assert_eq!(ranked.len(), expected.len());
        for (actual, expected) in ranked.iter().zip(expected) {
            is_color(*actual, *expected);
        }
    }

    #[test]
    fn prioritizes_chroma() {
        let ranked = ranked(
            &[(0xff000000, 1), (0xffffffff, 1), (0xff0000ff, 1)],
            4,
            DEFAULT_FALLBACK,
            true,
        );
        assert_ranked(&ranked, &[0xff0000ff]);
    }

    #[test]
    fn prioritizes_chroma_when_proportions_equal() {
        let ranked = ranked(
            &[(0xffff0000, 1), (0xff00ff00, 1), (0xff0000ff, 1)],
            4,
            DEFAULT_FALLBACK,
            true,
        );
        assert_ranked(&ranked, &[0xffff0000, 0xff00ff00, 0xff0000ff]);
    }

    #[test]
    fn generates_g_blue_when_no_colors_available() {
        let ranked = ranked(&[(0xff000000, 1)], 4, DEFAULT_FALLBACK, true);
        assert_ranked(&ranked, &[0xff4285f4]);
    }

    #[test]
    fn dedupes_nearby_hues() {
        let ranked = ranked(
            &[
                (0xff008772, 1), // H 180 C 42 T 50
                (0xff318477, 1), // H 184 C 35 T 50
            ],
            4,
            DEFAULT_FALLBACK,
            true,
        );
        assert_ranked(&ranked, &[0xff008772]);
    }

    #[test]
    fn maximizes_hue_distance() {
        let ranked = ranked(
            &[
                (0xff008772, 1), // H 180 C 42 T 50
                (0xff008587, 1), // H 198 C 50 T 50
                (0xff007ebc, 1), // H 245 C 50 T 50
            ],
            2,
            DEFAULT_FALLBACK,
            true,
        );
        assert_ranked(&ranked, &[0xff007ebc, 0xff008772]);
    }

    // Not in the package's tests: with no population every proportion is 0/0, every score is
    // NaN and `compareTo` answers 1 for every pair. The expected values are the Dart package's,
    // printed by `Score.score({0xffff0000: 0, 0xff00ff00: 0, 0xff0000ff: 0})` on Dart 3.13.1.
    #[test]
    fn ranks_colors_with_no_population_as_dart_does() {
        let ranked = ranked(
            &[(0xffff0000, 0), (0xff00ff00, 0), (0xff0000ff, 0)],
            4,
            DEFAULT_FALLBACK,
            true,
        );
        assert_ranked(&ranked, &[0xff0000ff, 0xff00ff00, 0xffff0000]);
    }

    #[test]
    fn passes_generated_scenario_one() {
        let ranked = ranked(
            &[(0xff7ea16d, 67), (0xffd8ccae, 67), (0xff835c0d, 49)],
            3,
            0xff8d3819,
            false,
        );
        assert_ranked(&ranked, &[0xff7ea16d, 0xffd8ccae, 0xff835c0d]);
    }

    #[test]
    fn passes_generated_scenario_two() {
        let ranked = ranked(
            &[
                (0xffd33881, 14),
                (0xff3205cc, 77),
                (0xff0b48cf, 36),
                (0xffa08f5d, 81),
            ],
            4,
            0xff7d772b,
            true,
        );
        assert_ranked(&ranked, &[0xff3205cc, 0xffa08f5d, 0xffd33881]);
    }

    #[test]
    fn passes_generated_scenario_three() {
        let ranked = ranked(
            &[
                (0xffbe94a6, 23),
                (0xffc33fd7, 42),
                (0xff899f36, 90),
                (0xff94c574, 82),
            ],
            3,
            0xffaa79a4,
            true,
        );
        assert_ranked(&ranked, &[0xff94c574, 0xffc33fd7, 0xffbe94a6]);
    }

    #[test]
    fn passes_generated_scenario_four() {
        let ranked = ranked(
            &[
                (0xffdf241c, 85),
                (0xff685859, 44),
                (0xffd06d5f, 34),
                (0xff561c54, 27),
                (0xff713090, 88),
            ],
            5,
            0xff58c19c,
            false,
        );
        assert_ranked(&ranked, &[0xffdf241c, 0xff561c54]);
    }

    #[test]
    fn passes_generated_scenario_five() {
        let ranked = ranked(
            &[
                (0xffbe66f8, 41),
                (0xff4bbda9, 88),
                (0xff80f6f9, 44),
                (0xffab8017, 43),
                (0xffe89307, 65),
            ],
            3,
            0xff916691,
            false,
        );
        assert_ranked(&ranked, &[0xffab8017, 0xff4bbda9, 0xffbe66f8]);
    }

    #[test]
    fn passes_generated_scenario_six() {
        let ranked = ranked(
            &[
                (0xff18ea8f, 93),
                (0xff327593, 18),
                (0xff066a18, 53),
                (0xfffa8a23, 74),
                (0xff04ca1f, 62),
            ],
            2,
            0xff4c377a,
            false,
        );
        assert_ranked(&ranked, &[0xff18ea8f, 0xfffa8a23]);
    }

    #[test]
    fn passes_generated_scenario_seven() {
        let ranked = ranked(
            &[
                (0xff2e05ed, 23),
                (0xff153e55, 90),
                (0xff9ab220, 23),
                (0xff153379, 66),
                (0xff68bcc3, 81),
            ],
            2,
            0xfff588dc,
            true,
        );
        assert_ranked(&ranked, &[0xff2e05ed, 0xff9ab220]);
    }

    #[test]
    fn passes_generated_scenario_eight() {
        let ranked = ranked(
            &[
                (0xff816ec5, 24),
                (0xff6dcb94, 19),
                (0xff3cae91, 98),
                (0xff5b542f, 25),
            ],
            1,
            0xff84b0fd,
            false,
        );
        assert_ranked(&ranked, &[0xff3cae91]);
    }

    #[test]
    fn passes_generated_scenario_nine() {
        let ranked = ranked(
            &[
                (0xff206f86, 52),
                (0xff4a620d, 96),
                (0xfff51401, 85),
                (0xff2b8ebf, 3),
                (0xff277766, 59),
            ],
            3,
            0xff02b415,
            true,
        );
        assert_ranked(&ranked, &[0xfff51401, 0xff4a620d, 0xff2b8ebf]);
    }

    #[test]
    fn passes_generated_scenario_ten() {
        let ranked = ranked(
            &[
                (0xff8b1d99, 54),
                (0xff27effe, 43),
                (0xff6f558d, 2),
                (0xff77fdf2, 78),
            ],
            4,
            0xff5e7a10,
            true,
        );
        assert_ranked(&ranked, &[0xff27effe, 0xff8b1d99, 0xff6f558d]);
    }
}
