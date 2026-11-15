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
// Ported from `lib/temperature/temperature_cache.dart` of material_color_utilities 0.13.0.

use std::cell::OnceCell;
use std::f64::consts::PI;

use indexmap::IndexMap;

use crate::dart_math;
use crate::hct::hct::Hct;
use crate::utils::color_utils::ColorUtils;
use crate::utils::math_utils::MathUtils;

/// Design utilities using color temperature theory.
///
/// Analogous colors, complementary color, and cache to efficiently, lazily,
/// generate data for calculations when needed.
pub struct TemperatureCache {
    input: Hct,

    hcts_by_temp: OnceCell<Vec<Hct>>,
    hcts_by_hue: OnceCell<Vec<Hct>>,
    temps_by_hct: OnceCell<IndexMap<Hct, f64>>,
    input_relative_temperature: OnceCell<f64>,
    complement: OnceCell<Hct>,
}

impl TemperatureCache {
    pub fn new(input: Hct) -> TemperatureCache {
        TemperatureCache {
            input,
            hcts_by_temp: OnceCell::new(),
            hcts_by_hue: OnceCell::new(),
            temps_by_hct: OnceCell::new(),
            input_relative_temperature: OnceCell::new(),
            complement: OnceCell::new(),
        }
    }

    pub fn input(&self) -> Hct {
        self.input
    }

    pub fn warmest(&self) -> Hct {
        *self.hcts_by_temp().last().unwrap()
    }

    pub fn coldest(&self) -> Hct {
        *self.hcts_by_temp().first().unwrap()
    }

    /// A set of colors with differing hues, equidistant in temperature.
    ///
    /// In art, this is usually described as a set of 5 colors on a color wheel
    /// divided into 12 sections. This method allows provision of either of those
    /// values.
    ///
    /// Behavior is undefined when `count` or `divisions` is 0.
    /// When divisions < count, colors repeat.
    ///
    /// `count` The number of colors to return, includes the input color; 5 in Dart by default.
    /// `divisions` The number of divisions on the color wheel; 12 in Dart by default.
    pub fn analogous(&self, count: i64, divisions: i64) -> Vec<Hct> {
        let start_hue = dart_math::round(self.input.hue());
        let start_hct = self.hcts_by_hue()[start_hue as usize];
        let mut last_temp = self.relative_temperature(&start_hct);
        let mut all_colors: Vec<Hct> = vec![start_hct];

        let mut absolute_total_temp_delta = 0.0;
        for i in 0..360 {
            let hue = MathUtils::sanitize_degrees_int(start_hue + i);
            let hct = self.hcts_by_hue()[hue as usize];
            let temp = self.relative_temperature(&hct);
            let temp_delta = (temp - last_temp).abs();
            last_temp = temp;
            absolute_total_temp_delta += temp_delta;
        }
        let mut hue_addend = 1;
        let temp_step = absolute_total_temp_delta / divisions as f64;
        let mut total_temp_delta = 0.0;
        last_temp = self.relative_temperature(&start_hct);
        while (all_colors.len() as i64) < divisions {
            let hue = MathUtils::sanitize_degrees_int(start_hue + hue_addend);
            let hct = self.hcts_by_hue()[hue as usize];
            let temp = self.relative_temperature(&hct);
            let temp_delta = (temp - last_temp).abs();
            total_temp_delta += temp_delta;

            let desired_total_temp_delta_for_index = all_colors.len() as f64 * temp_step;
            let mut index_satisfied = total_temp_delta >= desired_total_temp_delta_for_index;
            let mut index_addend = 1;
            // Keep adding this hue to the answers until its temperature is
            // insufficient. This ensures consistent behavior when there aren't
            // [divisions] discrete steps between 0 and 360 in hue with [tempStep]
            // delta in temperature between them.
            //
            // For example, white and black have no analogues: there are no other
            // colors at T100/T0. Therefore, they should just be added to the array
            // as answers.
            while index_satisfied && (all_colors.len() as i64) < divisions {
                all_colors.push(hct);
                let desired_total_temp_delta_for_index =
                    (all_colors.len() as i64 + index_addend) as f64 * temp_step;
                index_satisfied = total_temp_delta >= desired_total_temp_delta_for_index;
                index_addend += 1;
            }
            last_temp = temp;
            hue_addend += 1;
            if hue_addend > 360 {
                while (all_colors.len() as i64) < divisions {
                    all_colors.push(hct);
                }
                break;
            }
        }

        let mut answers: Vec<Hct> = vec![self.input];

        // First, generate analogues from rotating counter-clockwise.
        let increase_hue_count = dart_math::floor((count - 1) as f64 / 2.0);
        for i in 1..(increase_hue_count + 1) {
            let mut index = 0 - i;
            while index < 0 {
                index += all_colors.len() as i64;
            }
            if index >= all_colors.len() as i64 {
                index %= all_colors.len() as i64;
            }
            answers.insert(0, all_colors[index as usize]);
        }

        // Second, generate analogues from rotating clockwise.
        let decrease_hue_count = count - increase_hue_count - 1;
        for i in 1..(decrease_hue_count + 1) {
            let mut index = i;
            while index < 0 {
                index += all_colors.len() as i64;
            }
            if index >= all_colors.len() as i64 {
                index %= all_colors.len() as i64;
            }
            answers.push(all_colors[index as usize]);
        }

        answers
    }

    /// A color that complements the input color aesthetically.
    ///
    /// In art, this is usually described as being across the color wheel.
    /// History of this shows intent as a color that is just as cool-warm as the
    /// input color is warm-cool.
    pub fn complement(&self) -> Hct {
        *self.complement.get_or_init(|| self.compute_complement())
    }

    fn compute_complement(&self) -> Hct {
        let coldest_hue = self.coldest().hue();
        let coldest_temp = self.temps_by_hct()[&self.coldest()];

        let warmest_hue = self.warmest().hue();
        let warmest_temp = self.temps_by_hct()[&self.warmest()];
        let range = warmest_temp - coldest_temp;
        let start_hue_is_coldest_to_warmest =
            Self::is_between(self.input.hue(), coldest_hue, warmest_hue);
        let start_hue = if start_hue_is_coldest_to_warmest {
            warmest_hue
        } else {
            coldest_hue
        };
        let end_hue = if start_hue_is_coldest_to_warmest {
            coldest_hue
        } else {
            warmest_hue
        };
        const DIRECTION_OF_ROTATION: f64 = 1.0;
        let mut smallest_error = 1000.0;
        let mut answer = self.hcts_by_hue()[dart_math::round(self.input.hue()) as usize];

        let complement_relative_temp = 1.0 - self.input_relative_temperature();
        // Find the color in the other section, closest to the inverse percentile
        // of the input color. This is the complement.
        let mut hue_addend = 0.0;
        while hue_addend <= 360.0 {
            let hue =
                MathUtils::sanitize_degrees_double(start_hue + DIRECTION_OF_ROTATION * hue_addend);
            if Self::is_between(hue, start_hue, end_hue) {
                let possible_answer = self.hcts_by_hue()[dart_math::round(hue) as usize];
                let relative_temp = (self.temps_by_hct()[&possible_answer] - coldest_temp) / range;
                let error = (complement_relative_temp - relative_temp).abs();
                if error < smallest_error {
                    smallest_error = error;
                    answer = possible_answer;
                }
            }
            hue_addend += 1.0;
        }
        answer
    }

    /// Temperature relative to all colors with the same chroma and tone.
    /// Value on a scale from 0 to 1.
    pub fn relative_temperature(&self, hct: &Hct) -> f64 {
        let range = self.temps_by_hct()[&self.warmest()] - self.temps_by_hct()[&self.coldest()];
        let difference_from_coldest =
            self.temps_by_hct()[hct] - self.temps_by_hct()[&self.coldest()];
        // Handle when there's no difference in temperature between warmest and
        // coldest: for example, at T100, only one color is available, white.
        if range == 0.0 {
            return 0.5;
        }
        difference_from_coldest / range
    }

    /// Relative temperature of the input color. See [`relative_temperature`](Self::relative_temperature).
    pub fn input_relative_temperature(&self) -> f64 {
        *self.input_relative_temperature.get_or_init(|| {
            let coldest_temp = self.temps_by_hct()[&self.coldest()];

            let range = self.temps_by_hct()[&self.warmest()] - coldest_temp;
            let difference_from_coldest = self.temps_by_hct()[&self.input] - coldest_temp;
            if range == 0.0 {
                0.5
            } else {
                difference_from_coldest / range
            }
        })
    }

    /// HCTs for all hues, with the same chroma/tone as the input.
    /// Sorted from coldest first to warmest last.
    pub fn hcts_by_temp(&self) -> &[Hct] {
        self.hcts_by_temp.get_or_init(|| {
            let mut hcts = self.hcts_by_hue().to_vec();
            hcts.push(self.input);
            let temperatures_by_hct = self.temps_by_hct();
            dart_math::sort(&mut hcts, |a, b| {
                dart_math::compare_to(temperatures_by_hct[a], temperatures_by_hct[b])
            });
            hcts
        })
    }

    /// A Map with keys of HCTs in [`hcts_by_temp`](Self::hcts_by_temp), values of raw temperature.
    pub fn temps_by_hct(&self) -> &IndexMap<Hct, f64> {
        self.temps_by_hct.get_or_init(|| {
            let mut all_hcts = self.hcts_by_hue().to_vec();
            all_hcts.push(self.input);
            all_hcts
                .into_iter()
                .map(|e| (e, Self::raw_temperature(&e)))
                .collect()
        })
    }

    /// HCTs for all hues, with the same chroma/tone as the input.
    /// Sorted ascending, hue 0 to 360.
    pub fn hcts_by_hue(&self) -> &[Hct] {
        self.hcts_by_hue.get_or_init(|| {
            let mut hcts = Vec::new();
            let mut hue = 0.0;
            while hue <= 360.0 {
                let color_at_hue = Hct::from(hue, self.input.chroma(), self.input.tone());
                hcts.push(color_at_hue);
                hue += 1.0;
            }
            hcts
        })
    }

    /// Determines if an angle is between two other angles, rotating clockwise.
    pub fn is_between(angle: f64, a: f64, b: f64) -> bool {
        if a < b {
            return a <= angle && angle <= b;
        }
        a <= angle || angle <= b
    }

    /// Value representing cool-warm factor of a color.
    /// Values below 0 are considered cool, above, warm.
    ///
    /// Color science has researched emotion and harmony, which art uses to select
    /// colors. Warm-cool is the foundation of analogous and complementary colors.
    /// See:
    /// - Li-Chen Ou's Chapter 19 in Handbook of Color Psychology (2015).
    /// - Josef Albers' Interaction of Color chapters 19 and 21.
    ///
    /// Implementation of Ou, Woodcock and Wright's algorithm, which uses
    /// L*a*b*/LCH color space.
    /// Return value has these properties:
    /// - Values below 0 are cool, above 0 are warm.
    /// - Lower bound: -0.52 - (chroma ^ 1.07 / 20). L*a*b* chroma is infinite.
    ///   Assuming max of 130 chroma, -9.66.
    /// - Upper bound: -0.52 + (chroma ^ 1.07 / 20). L*a*b* chroma is infinite.
    ///   Assuming max of 130 chroma, 8.61.
    pub fn raw_temperature(color: &Hct) -> f64 {
        let lab = ColorUtils::lab_from_argb(color.to_int());
        let hue = MathUtils::sanitize_degrees_double(lab[2].atan2(lab[1]) * 180.0 / PI);
        let chroma = ((lab[1] * lab[1]) + (lab[2] * lab[2])).sqrt();
        -0.5 + 0.02
            * chroma.powf(1.07)
            * (MathUtils::sanitize_degrees_double(hue - 50.0) * PI / 180.0).cos()
    }
}

#[cfg(test)]
mod tests {
    // Ported from `test/temperature_cache_test.dart`.
    use super::*;
    use crate::test_support::is_color;

    fn assert_close_to(actual: f64, expected: f64, delta: f64) {
        assert!(
            (actual - expected).abs() <= delta,
            "expected {actual} to be within {delta} of {expected}"
        );
    }

    #[test]
    fn raw_temperature() {
        let blue_temp = TemperatureCache::raw_temperature(&Hct::from_int(0xff0000ff));
        assert_close_to(blue_temp, -1.393, 0.001);

        let red_temp = TemperatureCache::raw_temperature(&Hct::from_int(0xffff0000));
        assert_close_to(red_temp, 2.351, 0.001);

        let green_temp = TemperatureCache::raw_temperature(&Hct::from_int(0xff00ff00));
        assert_close_to(green_temp, -0.267, 0.001);

        let white_temp = TemperatureCache::raw_temperature(&Hct::from_int(0xffffffff));
        assert_close_to(white_temp, -0.5, 0.001);

        let black_temp = TemperatureCache::raw_temperature(&Hct::from_int(0xff000000));
        assert_close_to(black_temp, -0.5, 0.001);
    }

    #[test]
    fn relative_temperature() {
        let cases = [
            (0xff0000ff, 0.0),
            (0xffff0000, 1.0),
            (0xff00ff00, 0.467),
            (0xffffffff, 0.5),
            (0xff000000, 0.5),
        ];
        for (argb, expected) in cases {
            let temp = TemperatureCache::new(Hct::from_int(argb)).input_relative_temperature();
            assert_close_to(temp, expected, 0.001);
        }
    }

    #[test]
    fn complement() {
        let cases = [
            (0xff0000ff, 0xff9d0002),
            (0xffff0000, 0xff007bfc),
            (0xff00ff00, 0xffffd2c9),
            (0xffffffff, 0xffffffff),
            (0xff000000, 0xff000000),
        ];
        for (argb, expected) in cases {
            is_color(
                TemperatureCache::new(Hct::from_int(argb))
                    .complement()
                    .to_int(),
                expected,
            );
        }
    }

    fn analogous_of(argb: u32) -> Vec<u32> {
        TemperatureCache::new(Hct::from_int(argb))
            .analogous(5, 12)
            .iter()
            .map(Hct::to_int)
            .collect()
    }

    #[test]
    fn analogous() {
        let blue_analogous = analogous_of(0xff0000ff);
        is_color(blue_analogous[0], 0xff00590c);
        is_color(blue_analogous[1], 0xff00564e);
        is_color(blue_analogous[2], 0xff0000ff);
        is_color(blue_analogous[3], 0xff6700cc);
        is_color(blue_analogous[4], 0xff81009f);

        let red_analogous = analogous_of(0xffff0000);
        is_color(red_analogous[0], 0xfff60082);
        is_color(red_analogous[1], 0xfffc004c);
        is_color(red_analogous[2], 0xffff0000);
        is_color(red_analogous[3], 0xffd95500);
        is_color(red_analogous[4], 0xffaf7200);

        let green_analogous = analogous_of(0xff00ff00);
        is_color(green_analogous[0], 0xffcee900);
        is_color(green_analogous[1], 0xff92f500);
        is_color(green_analogous[2], 0xff00ff00);
        is_color(green_analogous[3], 0xff00fd6f);
        is_color(green_analogous[4], 0xff00fab3);

        let black_analogous = analogous_of(0xff000000);
        is_color(black_analogous[0], 0xff000000);
        is_color(black_analogous[1], 0xff000000);
        is_color(black_analogous[2], 0xff000000);
        is_color(black_analogous[3], 0xff000000);
        is_color(black_analogous[4], 0xff000000);

        let white_analogous = analogous_of(0xffffffff);
        is_color(white_analogous[0], 0xffffffff);
        is_color(white_analogous[1], 0xffffffff);
        is_color(white_analogous[2], 0xffffffff);
        is_color(white_analogous[3], 0xffffffff);
        is_color(white_analogous[4], 0xffffffff);
    }
}
