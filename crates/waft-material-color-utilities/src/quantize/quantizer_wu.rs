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
// Ported from `lib/quantize/quantizer_wu.dart` of material_color_utilities 0.13.0.

// rationale: This library relies heavily on numeric computation, and a key
// requirement is that it is 'the same' as implementations in different
// languages. Including variable types, though sometimes unnecessary, is a
// powerful help to verification and avoiding hard-to-debug issues.

use std::fmt;

use indexmap::IndexMap;

use crate::dart_math;
use crate::quantize::quantizer::{Quantizer, QuantizerFuture, QuantizerResult};
use crate::quantize::quantizer_map::QuantizerMap;
use crate::utils::color_utils::ColorUtils;

/// The histogram's sums and products are Dart's `int` arithmetic: `i64` that wraps on
/// overflow in every build, as Dart's does. An image of more than about 6.9 million pixels
/// overflows the squares of `variance` and `maximize`; keep each such operation wrapping.
#[derive(Default)]
pub struct QuantizerWu {
    pub weights: Vec<i64>,
    pub moments_r: Vec<i64>,
    pub moments_g: Vec<i64>,
    pub moments_b: Vec<i64>,
    pub moments: Vec<f64>,
    pub cubes: Vec<Box>,
}

impl QuantizerWu {
    // A histogram of all the input colors is constructed. It has the shape of a
    // cube. The cube would be too large if it contained all 16 million colors:
    // historical best practice is to use 5 bits  of the 8 in each channel,
    // reducing the histogram to a volume of ~32,000.
    pub const INDEX_BITS: i64 = 5;
    pub const MAX_INDEX: i64 = 32;
    pub const SIDE_LENGTH: i64 = 33;
    pub const TOTAL_SIZE: usize = 35937;

    pub fn new() -> QuantizerWu {
        QuantizerWu::default()
    }

    pub fn get_index(r: i64, g: i64, b: i64) -> usize {
        ((r << (Self::INDEX_BITS * 2))
            + (r << (Self::INDEX_BITS + 1))
            + (g << Self::INDEX_BITS)
            + r
            + g
            + b) as usize
    }

    pub fn construct_histogram(&mut self, pixels: &IndexMap<u32, i64>) {
        self.weights = vec![0; Self::TOTAL_SIZE];
        self.moments_r = vec![0; Self::TOTAL_SIZE];
        self.moments_g = vec![0; Self::TOTAL_SIZE];
        self.moments_b = vec![0; Self::TOTAL_SIZE];
        self.moments = vec![0.0; Self::TOTAL_SIZE];
        for (pixel, count) in pixels {
            let pixel = *pixel;
            let count = *count;
            let red = ColorUtils::red_from_argb(pixel);
            let green = ColorUtils::green_from_argb(pixel);
            let blue = ColorUtils::blue_from_argb(pixel);
            let bits_to_remove = 8 - Self::INDEX_BITS;
            let i_r = (red >> bits_to_remove) + 1;
            let i_g = (green >> bits_to_remove) + 1;
            let i_b = (blue >> bits_to_remove) + 1;
            let index = Self::get_index(i_r, i_g, i_b);
            self.weights[index] = self.weights[index].wrapping_add(count);
            self.moments_r[index] = self.moments_r[index].wrapping_add(red.wrapping_mul(count));
            self.moments_g[index] = self.moments_g[index].wrapping_add(green.wrapping_mul(count));
            self.moments_b[index] = self.moments_b[index].wrapping_add(blue.wrapping_mul(count));
            self.moments[index] +=
                count.wrapping_mul((red * red) + (green * green) + (blue * blue)) as f64;
        }
    }

    pub fn compute_moments(&mut self) {
        for r in 1..Self::SIDE_LENGTH {
            let side_length = Self::SIDE_LENGTH as usize;
            let mut area: Vec<i64> = vec![0; side_length];
            let mut area_r: Vec<i64> = vec![0; side_length];
            let mut area_g: Vec<i64> = vec![0; side_length];
            let mut area_b: Vec<i64> = vec![0; side_length];
            let mut area2: Vec<f64> = vec![0.0; side_length];
            for g in 1..Self::SIDE_LENGTH {
                let mut line: i64 = 0;
                let mut line_r: i64 = 0;
                let mut line_g: i64 = 0;
                let mut line_b: i64 = 0;
                let mut line2: f64 = 0.0;
                for b in 1..Self::SIDE_LENGTH {
                    let index: usize = Self::get_index(r, g, b);
                    line = line.wrapping_add(self.weights[index]);
                    line_r = line_r.wrapping_add(self.moments_r[index]);
                    line_g = line_g.wrapping_add(self.moments_g[index]);
                    line_b = line_b.wrapping_add(self.moments_b[index]);
                    line2 += self.moments[index];

                    let b = b as usize;
                    area[b] = area[b].wrapping_add(line);
                    area_r[b] = area_r[b].wrapping_add(line_r);
                    area_g[b] = area_g[b].wrapping_add(line_g);
                    area_b[b] = area_b[b].wrapping_add(line_b);
                    area2[b] += line2;

                    let previous_index: usize = Self::get_index(r - 1, g, b as i64);
                    self.weights[index] = self.weights[previous_index].wrapping_add(area[b]);
                    self.moments_r[index] = self.moments_r[previous_index].wrapping_add(area_r[b]);
                    self.moments_g[index] = self.moments_g[previous_index].wrapping_add(area_g[b]);
                    self.moments_b[index] = self.moments_b[previous_index].wrapping_add(area_b[b]);
                    self.moments[index] = self.moments[previous_index] + area2[b];
                }
            }
        }
    }

    #[allow(clippy::needless_range_loop)]
    pub fn create_boxes(&mut self, max_color_count: i64) -> CreateBoxesResult {
        self.cubes = (0..max_color_count).map(|_| Box::default()).collect();
        self.cubes[0] = Box {
            r0: 0,
            r1: Self::MAX_INDEX,
            g0: 0,
            g1: Self::MAX_INDEX,
            b0: 0,
            b1: Self::MAX_INDEX,
            vol: 0,
        };

        let mut volume_variance: Vec<f64> = vec![0.0; max_color_count as usize];
        let mut next: usize = 0;
        let mut generated_color_count: i64 = max_color_count;
        let mut i: i64 = 1;
        while i < max_color_count {
            let (mut one, mut two) = (self.cubes[next], self.cubes[i as usize]);
            let was_cut = self.cut(&mut one, &mut two);
            self.cubes[next] = one;
            self.cubes[i as usize] = two;
            if was_cut {
                volume_variance[next] = if self.cubes[next].vol > 1 {
                    self.variance(&self.cubes[next])
                } else {
                    0.0
                };
                volume_variance[i as usize] = if self.cubes[i as usize].vol > 1 {
                    self.variance(&self.cubes[i as usize])
                } else {
                    0.0
                };
            } else {
                volume_variance[next] = 0.0;
                i -= 1;
            }

            next = 0;
            let mut temp: f64 = volume_variance[0];
            for j in 1..=i as usize {
                if volume_variance[j] > temp {
                    temp = volume_variance[j];
                    next = j;
                }
            }
            if temp <= 0.0 {
                generated_color_count = i + 1;
                break;
            }
            i += 1;
        }

        CreateBoxesResult::new(max_color_count, generated_color_count)
    }

    pub fn create_result(&self, color_count: i64) -> Vec<u32> {
        let mut colors: Vec<u32> = Vec::new();
        for i in 0..color_count as usize {
            let cube = &self.cubes[i];
            let weight: i64 = Self::volume(cube, &self.weights);
            if weight > 0 {
                let r: i64 =
                    dart_math::round(Self::volume(cube, &self.moments_r) as f64 / weight as f64);
                let g: i64 =
                    dart_math::round(Self::volume(cube, &self.moments_g) as f64 / weight as f64);
                let b: i64 =
                    dart_math::round(Self::volume(cube, &self.moments_b) as f64 / weight as f64);
                let color: u32 = ColorUtils::argb_from_rgb(r, g, b);
                colors.push(color);
            }
        }
        colors
    }

    pub fn variance(&self, cube: &Box) -> f64 {
        let dr = Self::volume(cube, &self.moments_r);
        let dg = Self::volume(cube, &self.moments_g);
        let db = Self::volume(cube, &self.moments_b);
        let moments = &self.moments;
        let xx = moments[Self::get_index(cube.r1, cube.g1, cube.b1)]
            - moments[Self::get_index(cube.r1, cube.g1, cube.b0)]
            - moments[Self::get_index(cube.r1, cube.g0, cube.b1)]
            + moments[Self::get_index(cube.r1, cube.g0, cube.b0)]
            - moments[Self::get_index(cube.r0, cube.g1, cube.b1)]
            + moments[Self::get_index(cube.r0, cube.g1, cube.b0)]
            + moments[Self::get_index(cube.r0, cube.g0, cube.b1)]
            - moments[Self::get_index(cube.r0, cube.g0, cube.b0)];

        let hypotenuse = dr
            .wrapping_mul(dr)
            .wrapping_add(dg.wrapping_mul(dg))
            .wrapping_add(db.wrapping_mul(db));
        let volume = Self::volume(cube, &self.weights);
        xx - hypotenuse as f64 / volume as f64
    }

    pub fn cut(&self, one: &mut Box, two: &mut Box) -> bool {
        let whole_r = Self::volume(one, &self.moments_r);
        let whole_g = Self::volume(one, &self.moments_g);
        let whole_b = Self::volume(one, &self.moments_b);
        let whole_w = Self::volume(one, &self.weights);

        let max_r_result = self.maximize(
            one,
            Direction::Red,
            one.r0 + 1,
            one.r1,
            whole_r,
            whole_g,
            whole_b,
            whole_w,
        );
        let max_g_result = self.maximize(
            one,
            Direction::Green,
            one.g0 + 1,
            one.g1,
            whole_r,
            whole_g,
            whole_b,
            whole_w,
        );
        let max_b_result = self.maximize(
            one,
            Direction::Blue,
            one.b0 + 1,
            one.b1,
            whole_r,
            whole_g,
            whole_b,
            whole_w,
        );

        let cut_direction: Direction;
        let max_r = max_r_result.maximum;
        let max_g = max_g_result.maximum;
        let max_b = max_b_result.maximum;
        if max_r >= max_g && max_r >= max_b {
            cut_direction = Direction::Red;
            if max_r_result.cut_location < 0 {
                return false;
            }
        } else if max_g >= max_r && max_g >= max_b {
            cut_direction = Direction::Green;
        } else {
            cut_direction = Direction::Blue;
        }

        two.r1 = one.r1;
        two.g1 = one.g1;
        two.b1 = one.b1;

        match cut_direction {
            Direction::Red => {
                one.r1 = max_r_result.cut_location;
                two.r0 = one.r1;
                two.g0 = one.g0;
                two.b0 = one.b0;
            }
            Direction::Green => {
                one.g1 = max_g_result.cut_location;
                two.r0 = one.r0;
                two.g0 = one.g1;
                two.b0 = one.b0;
            }
            Direction::Blue => {
                one.b1 = max_b_result.cut_location;
                two.r0 = one.r0;
                two.g0 = one.g0;
                two.b0 = one.b1;
            }
        }

        one.vol = (one.r1 - one.r0) * (one.g1 - one.g0) * (one.b1 - one.b0);
        two.vol = (two.r1 - two.r0) * (two.g1 - two.g0) * (two.b1 - two.b0);
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub fn maximize(
        &self,
        cube: &Box,
        direction: Direction,
        first: i64,
        last: i64,
        whole_r: i64,
        whole_g: i64,
        whole_b: i64,
        whole_w: i64,
    ) -> MaximizeResult {
        let bottom_r: i64 = Self::bottom(cube, direction, &self.moments_r);
        let bottom_g: i64 = Self::bottom(cube, direction, &self.moments_g);
        let bottom_b: i64 = Self::bottom(cube, direction, &self.moments_b);
        let bottom_w: i64 = Self::bottom(cube, direction, &self.weights);

        let mut max: f64 = 0.0;
        let mut cut: i64 = -1;

        for i in first..last {
            let mut half_r: i64 =
                bottom_r.wrapping_add(Self::top(cube, direction, i, &self.moments_r));
            let mut half_g: i64 =
                bottom_g.wrapping_add(Self::top(cube, direction, i, &self.moments_g));
            let mut half_b: i64 =
                bottom_b.wrapping_add(Self::top(cube, direction, i, &self.moments_b));
            let mut half_w: i64 =
                bottom_w.wrapping_add(Self::top(cube, direction, i, &self.weights));

            if half_w == 0 {
                continue;
            }

            let mut temp_numerator: f64 = half_r
                .wrapping_mul(half_r)
                .wrapping_add(half_g.wrapping_mul(half_g))
                .wrapping_add(half_b.wrapping_mul(half_b))
                as f64;
            let mut temp_denominator: f64 = half_w as f64;
            let mut temp: f64 = temp_numerator / temp_denominator;

            half_r = whole_r.wrapping_sub(half_r);
            half_g = whole_g.wrapping_sub(half_g);
            half_b = whole_b.wrapping_sub(half_b);
            half_w = whole_w.wrapping_sub(half_w);
            if half_w == 0 {
                continue;
            }
            temp_numerator = half_r
                .wrapping_mul(half_r)
                .wrapping_add(half_g.wrapping_mul(half_g))
                .wrapping_add(half_b.wrapping_mul(half_b)) as f64;
            temp_denominator = half_w as f64;
            temp += temp_numerator / temp_denominator;

            if temp > max {
                max = temp;
                cut = i;
            }
        }
        MaximizeResult::new(cut, max)
    }

    pub fn volume(cube: &Box, moment: &[i64]) -> i64 {
        moment[Self::get_index(cube.r1, cube.g1, cube.b1)]
            .wrapping_sub(moment[Self::get_index(cube.r1, cube.g1, cube.b0)])
            .wrapping_sub(moment[Self::get_index(cube.r1, cube.g0, cube.b1)])
            .wrapping_add(moment[Self::get_index(cube.r1, cube.g0, cube.b0)])
            .wrapping_sub(moment[Self::get_index(cube.r0, cube.g1, cube.b1)])
            .wrapping_add(moment[Self::get_index(cube.r0, cube.g1, cube.b0)])
            .wrapping_add(moment[Self::get_index(cube.r0, cube.g0, cube.b1)])
            .wrapping_sub(moment[Self::get_index(cube.r0, cube.g0, cube.b0)])
    }

    pub fn bottom(cube: &Box, direction: Direction, moment: &[i64]) -> i64 {
        match direction {
            Direction::Red => moment[Self::get_index(cube.r0, cube.g1, cube.b1)]
                .wrapping_neg()
                .wrapping_add(moment[Self::get_index(cube.r0, cube.g1, cube.b0)])
                .wrapping_add(moment[Self::get_index(cube.r0, cube.g0, cube.b1)])
                .wrapping_sub(moment[Self::get_index(cube.r0, cube.g0, cube.b0)]),
            Direction::Green => moment[Self::get_index(cube.r1, cube.g0, cube.b1)]
                .wrapping_neg()
                .wrapping_add(moment[Self::get_index(cube.r1, cube.g0, cube.b0)])
                .wrapping_add(moment[Self::get_index(cube.r0, cube.g0, cube.b1)])
                .wrapping_sub(moment[Self::get_index(cube.r0, cube.g0, cube.b0)]),
            Direction::Blue => moment[Self::get_index(cube.r1, cube.g1, cube.b0)]
                .wrapping_neg()
                .wrapping_add(moment[Self::get_index(cube.r1, cube.g0, cube.b0)])
                .wrapping_add(moment[Self::get_index(cube.r0, cube.g1, cube.b0)])
                .wrapping_sub(moment[Self::get_index(cube.r0, cube.g0, cube.b0)]),
        }
    }

    pub fn top(cube: &Box, direction: Direction, position: i64, moment: &[i64]) -> i64 {
        match direction {
            Direction::Red => moment[Self::get_index(position, cube.g1, cube.b1)]
                .wrapping_sub(moment[Self::get_index(position, cube.g1, cube.b0)])
                .wrapping_sub(moment[Self::get_index(position, cube.g0, cube.b1)])
                .wrapping_add(moment[Self::get_index(position, cube.g0, cube.b0)]),
            Direction::Green => moment[Self::get_index(cube.r1, position, cube.b1)]
                .wrapping_sub(moment[Self::get_index(cube.r1, position, cube.b0)])
                .wrapping_sub(moment[Self::get_index(cube.r0, position, cube.b1)])
                .wrapping_add(moment[Self::get_index(cube.r0, position, cube.b0)]),
            Direction::Blue => moment[Self::get_index(cube.r1, cube.g1, position)]
                .wrapping_sub(moment[Self::get_index(cube.r1, cube.g0, position)])
                .wrapping_sub(moment[Self::get_index(cube.r0, cube.g1, position)])
                .wrapping_add(moment[Self::get_index(cube.r0, cube.g0, position)]),
        }
    }
}

impl Quantizer for QuantizerWu {
    fn quantize(&mut self, pixels: &[u32], color_count: i64) -> QuantizerFuture {
        let result = QuantizerMap.quantize(pixels, color_count).into_inner();
        self.construct_histogram(&result.color_to_count);
        self.compute_moments();
        let create_boxes_result = self.create_boxes(color_count);
        let results = self.create_result(create_boxes_result.result_count);
        std::future::ready(QuantizerResult::new(
            results.into_iter().map(|e| (e, 0)).collect(),
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Red,
    Green,
    Blue,
}

pub struct MaximizeResult {
    // < 0 if cut impossible
    pub cut_location: i64,
    pub maximum: f64,
}

impl MaximizeResult {
    pub fn new(cut_location: i64, maximum: f64) -> MaximizeResult {
        MaximizeResult {
            cut_location,
            maximum,
        }
    }
}

pub struct CreateBoxesResult {
    pub requested_count: i64,
    pub result_count: i64,
}

impl CreateBoxesResult {
    pub fn new(requested_count: i64, result_count: i64) -> CreateBoxesResult {
        CreateBoxesResult {
            requested_count,
            result_count,
        }
    }
}

/// A value: [`QuantizerWu::cut`] writes the two boxes it is handed, and the caller stores them
/// back in [`QuantizerWu::cubes`].
#[derive(Clone, Copy, Default)]
pub struct Box {
    pub r0: i64,
    pub r1: i64,
    pub g0: i64,
    pub g1: i64,
    pub b0: i64,
    pub b1: i64,
    pub vol: i64,
}

impl fmt::Debug for Box {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Box: R {} -> {} G  {} -> {} B {} -> {} VOL = {}",
            self.r0, self.r1, self.g0, self.g1, self.b0, self.b1, self.vol
        )
    }
}

#[cfg(test)]
mod tests {
    // Ported from `test/quantizer_wu_test.dart`.
    use std::collections::HashSet;

    use super::*;

    const RED: u32 = 0xffff0000;
    const GREEN: u32 = 0xff00ff00;
    const BLUE: u32 = 0xff0000ff;
    const MAX_COLORS: i64 = 256;

    fn colors_of(pixels: &[u32]) -> Vec<u32> {
        let mut wu = QuantizerWu::new();
        let result = wu.quantize(pixels, MAX_COLORS).into_inner();
        result.color_to_count.keys().copied().collect()
    }

    #[test]
    fn one_red() {
        let colors = colors_of(&[RED]);
        assert_eq!(colors.len(), 1);
    }

    #[test]
    fn one_rando() {
        let colors = colors_of(&[0xff141216]);
        assert_eq!(colors.len(), 1);
        assert_eq!(colors[0], 0xff141216);
    }

    #[test]
    fn one_red_is_red() {
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
    fn two_red_three_green() {
        let colors = colors_of(&[RED, RED, GREEN, GREEN, GREEN]);
        assert_eq!(colors.iter().collect::<HashSet<_>>().len(), 2);
        assert_eq!(colors[0], GREEN);
        assert_eq!(colors[1], RED);
    }

    #[test]
    fn one_red_one_green_one_blue() {
        let colors = colors_of(&[RED, GREEN, BLUE]);
        assert_eq!(colors.iter().collect::<HashSet<_>>().len(), 3);
        assert_eq!(colors[0], BLUE);
        assert_eq!(colors[1], RED);
        assert_eq!(colors[2], GREEN);
    }

    // Not in the package's tests: counts whose squared moments overflow 64 bits wrap as
    // Dart's `int` does. The expected values are the Dart package's, printed by
    // `constructHistogram`, `computeMoments`, `createBoxes(3)`, `createResult` and
    // `variance(cubes[0])` on the same histogram on Dart 3.13.1.
    #[test]
    fn overflowing_moments_wrap_as_dart_s_int_does() {
        let mut wu = QuantizerWu::new();
        let pixels: IndexMap<u32, i64> = [
            (RED, 4_000_000_000_000),
            (GREEN, 3_000_000_000_000),
            (BLUE, 5_000_000_000_000),
        ]
        .into_iter()
        .collect();
        wu.construct_histogram(&pixels);
        wu.compute_moments();
        let boxes = wu.create_boxes(3);
        assert_eq!(boxes.requested_count, 3);
        assert_eq!(boxes.result_count, 2);
        assert_eq!(wu.create_result(boxes.result_count), [0xff926d00, BLUE]);
        assert_eq!(wu.variance(&wu.cubes[0]), 455174999999110850.0);
    }
}
