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
// Ported from `lib/utils/color_utils.dart` of material_color_utilities 0.13.0.

use crate::dart_math;
use crate::utils::math_utils::MathUtils;

/// Color science utilities.
///
/// Utility methods for color science constants and color space
/// conversions that aren't HCT or CAM16.
///
/// A color in ARGB format is a `u32`; an RGB component, Dart's `int`, an `i64`.
pub struct ColorUtils;

impl ColorUtils {
    const SRGB_TO_XYZ: [[f64; 3]; 3] = [
        [0.41233895, 0.35762064, 0.18051042],
        [0.2126, 0.7152, 0.0722],
        [0.01932141, 0.11916382, 0.95034478],
    ];

    const XYZ_TO_SRGB: [[f64; 3]; 3] = [
        [
            3.2413774792388685,
            -1.5376652402851851,
            -0.49885366846268053,
        ],
        [-0.9691452513005321, 1.8758853451067872, 0.04156585616912061],
        [
            0.05562093689691305,
            -0.20395524564742123,
            1.0571799111220335,
        ],
    ];

    const WHITE_POINT_D65: [f64; 3] = [95.047, 100.0, 108.883];

    /// Converts a color from RGB components to ARGB format.
    pub fn argb_from_rgb(red: i64, green: i64, blue: i64) -> u32 {
        (255 << 24 | (red & 255) << 16 | (green & 255) << 8 | blue & 255) as u32
    }

    /// Converts a color from linear RGB components to ARGB format.
    pub fn argb_from_linrgb(linrgb: &[f64; 3]) -> u32 {
        let r = Self::delinearized(linrgb[0]);
        let g = Self::delinearized(linrgb[1]);
        let b = Self::delinearized(linrgb[2]);
        Self::argb_from_rgb(r, g, b)
    }

    /// Returns the alpha component of a color in ARGB format.
    pub fn alpha_from_argb(argb: u32) -> i64 {
        (argb >> 24 & 255) as i64
    }

    /// Returns the red component of a color in ARGB format.
    pub fn red_from_argb(argb: u32) -> i64 {
        (argb >> 16 & 255) as i64
    }

    /// Returns the green component of a color in ARGB format.
    pub fn green_from_argb(argb: u32) -> i64 {
        (argb >> 8 & 255) as i64
    }

    /// Returns the blue component of a color in ARGB format.
    pub fn blue_from_argb(argb: u32) -> i64 {
        (argb & 255) as i64
    }

    /// Returns whether a color in ARGB format is opaque.
    pub fn is_opaque(argb: u32) -> bool {
        Self::alpha_from_argb(argb) >= 255
    }

    /// Converts a color from ARGB to XYZ.
    pub fn argb_from_xyz(x: f64, y: f64, z: f64) -> u32 {
        let matrix = Self::XYZ_TO_SRGB;
        let linear_r = matrix[0][0] * x + matrix[0][1] * y + matrix[0][2] * z;
        let linear_g = matrix[1][0] * x + matrix[1][1] * y + matrix[1][2] * z;
        let linear_b = matrix[2][0] * x + matrix[2][1] * y + matrix[2][2] * z;
        let r = Self::delinearized(linear_r);
        let g = Self::delinearized(linear_g);
        let b = Self::delinearized(linear_b);
        Self::argb_from_rgb(r, g, b)
    }

    /// Converts a color from XYZ to ARGB.
    pub fn xyz_from_argb(argb: u32) -> [f64; 3] {
        let r = Self::linearized(Self::red_from_argb(argb));
        let g = Self::linearized(Self::green_from_argb(argb));
        let b = Self::linearized(Self::blue_from_argb(argb));
        MathUtils::matrix_multiply(&[r, g, b], &Self::SRGB_TO_XYZ)
    }

    /// Converts a color represented in Lab color space into an ARGB
    /// integer.
    pub fn argb_from_lab(l: f64, a: f64, b: f64) -> u32 {
        let white_point = Self::WHITE_POINT_D65;
        let fy = (l + 16.0) / 116.0;
        let fx = a / 500.0 + fy;
        let fz = fy - b / 200.0;
        let x_normalized = Self::lab_invf(fx);
        let y_normalized = Self::lab_invf(fy);
        let z_normalized = Self::lab_invf(fz);
        let x = x_normalized * white_point[0];
        let y = y_normalized * white_point[1];
        let z = z_normalized * white_point[2];
        Self::argb_from_xyz(x, y, z)
    }

    /// Converts a color from ARGB representation to L*a*b*
    /// representation.
    ///
    /// `argb` the ARGB representation of a color
    /// Returns a Lab object representing the color
    pub fn lab_from_argb(argb: u32) -> [f64; 3] {
        let linear_r = Self::linearized(Self::red_from_argb(argb));
        let linear_g = Self::linearized(Self::green_from_argb(argb));
        let linear_b = Self::linearized(Self::blue_from_argb(argb));
        let matrix = Self::SRGB_TO_XYZ;
        let x = matrix[0][0] * linear_r + matrix[0][1] * linear_g + matrix[0][2] * linear_b;
        let y = matrix[1][0] * linear_r + matrix[1][1] * linear_g + matrix[1][2] * linear_b;
        let z = matrix[2][0] * linear_r + matrix[2][1] * linear_g + matrix[2][2] * linear_b;
        let white_point = Self::WHITE_POINT_D65;
        let x_normalized = x / white_point[0];
        let y_normalized = y / white_point[1];
        let z_normalized = z / white_point[2];
        let fx = Self::lab_f(x_normalized);
        let fy = Self::lab_f(y_normalized);
        let fz = Self::lab_f(z_normalized);
        let l = 116.0 * fy - 16.0;
        let a = 500.0 * (fx - fy);
        let b = 200.0 * (fy - fz);
        [l, a, b]
    }

    /// Converts an L* value to an ARGB representation.
    ///
    /// `lstar` L* in L*a*b*
    /// Returns ARGB representation of grayscale color with lightness
    /// matching L*
    pub fn argb_from_lstar(lstar: f64) -> u32 {
        let y = Self::y_from_lstar(lstar);
        let component = Self::delinearized(y);
        Self::argb_from_rgb(component, component, component)
    }

    /// Computes the L* value of a color in ARGB representation.
    ///
    /// `argb` ARGB representation of a color
    /// Returns L*, from L*a*b*, coordinate of the color
    pub fn lstar_from_argb(argb: u32) -> f64 {
        let y = Self::xyz_from_argb(argb)[1];
        116.0 * Self::lab_f(y / 100.0) - 16.0
    }

    /// Converts an L* value to a Y value.
    ///
    /// L* in L*a*b* and Y in XYZ measure the same quantity, luminance.
    ///
    /// L* measures perceptual luminance, a linear scale. Y in XYZ
    /// measures relative luminance, a logarithmic scale.
    ///
    /// `lstar` L* in L*a*b*
    /// Returns Y in XYZ
    pub fn y_from_lstar(lstar: f64) -> f64 {
        100.0 * Self::lab_invf((lstar + 16.0) / 116.0)
    }

    /// Converts a Y value to an L* value.
    ///
    /// L* in L*a*b* and Y in XYZ measure the same quantity, luminance.
    ///
    /// L* measures perceptual luminance, a linear scale. Y in XYZ
    /// measures relative luminance, a logarithmic scale.
    ///
    /// `y` Y in XYZ
    /// Returns L* in L*a*b*
    pub fn lstar_from_y(y: f64) -> f64 {
        Self::lab_f(y / 100.0) * 116.0 - 16.0
    }

    /// Linearizes an RGB component.
    ///
    /// `rgb_component` 0 <= rgb_component <= 255, represents R/G/B
    /// channel
    /// Returns 0.0 <= output <= 100.0, color channel converted to
    /// linear RGB space
    pub fn linearized(rgb_component: i64) -> f64 {
        let normalized = rgb_component as f64 / 255.0;
        if normalized <= 0.040449936 {
            normalized / 12.92 * 100.0
        } else {
            ((normalized + 0.055) / 1.055).powf(2.4) * 100.0
        }
    }

    /// Delinearizes an RGB component.
    ///
    /// `rgb_component` 0.0 <= rgb_component <= 100.0, represents linear
    /// R/G/B channel
    /// Returns 0 <= output <= 255, color channel converted to regular
    /// RGB space
    pub fn delinearized(rgb_component: f64) -> i64 {
        let normalized = rgb_component / 100.0;
        let delinearized = if normalized <= 0.0031308 {
            normalized * 12.92
        } else {
            1.055 * normalized.powf(1.0 / 2.4) - 0.055
        };
        MathUtils::clamp_int(0, 255, dart_math::round(delinearized * 255.0))
    }

    /// Returns the standard white point; white on a sunny day.
    ///
    /// Returns The white point
    pub fn white_point_d65() -> [f64; 3] {
        Self::WHITE_POINT_D65
    }

    fn lab_f(t: f64) -> f64 {
        let e = 216.0 / 24389.0;
        let kappa = 24389.0 / 27.0;
        if t > e {
            t.powf(1.0 / 3.0)
        } else {
            (kappa * t + 16.0) / 116.0
        }
    }

    fn lab_invf(ft: f64) -> f64 {
        let e = 216.0 / 24389.0;
        let kappa = 24389.0 / 27.0;
        let ft3 = ft * ft * ft;
        if ft3 > e {
            ft3
        } else {
            (116.0 * ft - 16.0) / kappa
        }
    }
}

#[cfg(test)]
mod tests {
    // Ported from `test/color_utils_test.dart`.
    use super::*;

    fn range(start: f64, stop: f64, case_count: i64) -> Vec<f64> {
        let step_size = (stop - start) / (case_count - 1) as f64;
        (0..case_count)
            .map(|index| start + step_size * index as f64)
            .collect()
    }

    fn rgb_range() -> Vec<i64> {
        range(0.0, 255.0, 8)
            .into_iter()
            .map(|element| element.round() as i64)
            .collect()
    }

    fn full_rgb_range() -> Vec<i64> {
        (0..256).collect()
    }

    fn assert_close_to(actual: f64, expected: f64, delta: f64) {
        assert!(
            (actual - expected).abs() <= delta,
            "expected {actual} to be within {delta} of {expected}"
        );
    }

    #[test]
    fn range_integrity() {
        let range = range(3.0, 9999.0, 1234);
        for (i, value) in range.iter().enumerate() {
            assert_close_to(*value, 3.0 + 8.1070559611 * i as f64, 1e-5);
        }
    }

    #[test]
    fn argb_from_rgb_returns_correct_value_for_black() {
        assert_eq!(ColorUtils::argb_from_rgb(0, 0, 0), 0xff000000);
        assert_eq!(ColorUtils::argb_from_rgb(0, 0, 0), 4278190080);
    }

    #[test]
    fn argb_from_rgb_returns_correct_value_for_white() {
        assert_eq!(ColorUtils::argb_from_rgb(255, 255, 255), 0xffffffff);
        assert_eq!(ColorUtils::argb_from_rgb(255, 255, 255), 4294967295);
    }

    #[test]
    fn argb_from_rgb_returns_correct_value_for_random_color() {
        assert_eq!(ColorUtils::argb_from_rgb(50, 150, 250), 0xff3296fa);
        assert_eq!(ColorUtils::argb_from_rgb(50, 150, 250), 4281505530);
    }

    #[test]
    fn y_to_lstar_to_y() {
        for y in range(0.0, 100.0, 1001) {
            assert_close_to(
                ColorUtils::y_from_lstar(ColorUtils::lstar_from_y(y)),
                y,
                1e-5,
            );
        }
    }

    #[test]
    fn lstar_to_y_to_lstar() {
        for lstar in range(0.0, 100.0, 1001) {
            assert_close_to(
                ColorUtils::lstar_from_y(ColorUtils::y_from_lstar(lstar)),
                lstar,
                1e-5,
            );
        }
    }

    #[test]
    fn y_from_lstar() {
        let cases = [
            (0.0, 0.0),
            (0.1, 0.0110705),
            (0.2, 0.0221411),
            (0.3, 0.0332116),
            (0.4, 0.0442822),
            (0.5, 0.0553528),
            (1.0, 0.1107056),
            (2.0, 0.2214112),
            (3.0, 0.3321169),
            (4.0, 0.4428225),
            (5.0, 0.5535282),
            (8.0, 0.8856451),
            (10.0, 1.1260199),
            (15.0, 1.9085832),
            (20.0, 2.9890524),
            (25.0, 4.4154767),
            (30.0, 6.2359055),
            (40.0, 11.2509737),
            (50.0, 18.4186518),
            (60.0, 28.1233342),
            (70.0, 40.7494157),
            (80.0, 56.6812907),
            (90.0, 76.3033539),
            (95.0, 87.6183294),
            (99.0, 97.4360239),
            (100.0, 100.0),
        ];
        for (lstar, y) in cases {
            assert_close_to(ColorUtils::y_from_lstar(lstar), y, 1e-5);
        }
    }

    #[test]
    fn lstar_from_y() {
        let cases = [
            (0.0, 0.0),
            (0.1, 0.9032962),
            (0.2, 1.8065925),
            (0.3, 2.7098888),
            (0.4, 3.6131851),
            (0.5, 4.5164814),
            (0.8856451, 8.0),
            (1.0, 8.9914424),
            (2.0, 15.4872443),
            (3.0, 20.0438970),
            (4.0, 23.6714419),
            (5.0, 26.7347653),
            (10.0, 37.8424304),
            (15.0, 45.6341970),
            (20.0, 51.8372115),
            (25.0, 57.0754208),
            (30.0, 61.6542222),
            (40.0, 69.4695307),
            (50.0, 76.0692610),
            (60.0, 81.8381891),
            (70.0, 86.9968642),
            (80.0, 91.6848609),
            (90.0, 95.9967686),
            (95.0, 98.0335184),
            (99.0, 99.6120372),
            (100.0, 100.0),
        ];
        for (y, lstar) in cases {
            assert_close_to(ColorUtils::lstar_from_y(y), lstar, 1e-5);
        }
    }

    #[test]
    fn y_continuity() {
        let epsilon = 1e-6;
        let delta = 1e-8;
        let left = 8.0 - delta;
        let mid = 8.0;
        let right = 8.0 + delta;
        assert_close_to(
            ColorUtils::y_from_lstar(left),
            ColorUtils::y_from_lstar(mid),
            epsilon,
        );
        assert_close_to(
            ColorUtils::y_from_lstar(right),
            ColorUtils::y_from_lstar(mid),
            epsilon,
        );
    }

    #[test]
    fn rgb_to_xyz_to_rgb() {
        for r in rgb_range() {
            for g in rgb_range() {
                for b in rgb_range() {
                    let argb = ColorUtils::argb_from_rgb(r, g, b);
                    let xyz = ColorUtils::xyz_from_argb(argb);
                    let converted = ColorUtils::argb_from_xyz(xyz[0], xyz[1], xyz[2]);
                    assert_close_to(ColorUtils::red_from_argb(converted) as f64, r as f64, 1.5);
                    assert_close_to(ColorUtils::green_from_argb(converted) as f64, g as f64, 1.5);
                    assert_close_to(ColorUtils::blue_from_argb(converted) as f64, b as f64, 1.5);
                }
            }
        }
    }

    #[test]
    fn rgb_to_lab_to_rgb() {
        for r in rgb_range() {
            for g in rgb_range() {
                for b in rgb_range() {
                    let argb = ColorUtils::argb_from_rgb(r, g, b);
                    let lab = ColorUtils::lab_from_argb(argb);
                    let converted = ColorUtils::argb_from_lab(lab[0], lab[1], lab[2]);
                    assert_close_to(ColorUtils::red_from_argb(converted) as f64, r as f64, 1.5);
                    assert_close_to(ColorUtils::green_from_argb(converted) as f64, g as f64, 1.5);
                    assert_close_to(ColorUtils::blue_from_argb(converted) as f64, b as f64, 1.5);
                }
            }
        }
    }

    #[test]
    fn rgb_to_lstar_to_rgb() {
        for component in full_rgb_range() {
            let argb = ColorUtils::argb_from_rgb(component, component, component);
            let lstar = ColorUtils::lstar_from_argb(argb);
            let converted = ColorUtils::argb_from_lstar(lstar);
            assert_eq!(converted, argb);
        }
    }

    #[test]
    fn rgb_to_lstar_to_y_commutes() {
        for r in rgb_range() {
            for g in rgb_range() {
                for b in rgb_range() {
                    let argb = ColorUtils::argb_from_rgb(r, g, b);
                    let lstar = ColorUtils::lstar_from_argb(argb);
                    let y = ColorUtils::y_from_lstar(lstar);
                    let y2 = ColorUtils::xyz_from_argb(argb)[1];
                    assert_close_to(y, y2, 1e-5);
                }
            }
        }
    }

    #[test]
    fn lstar_to_rgb_to_y_commutes() {
        for lstar in range(0.0, 100.0, 1001) {
            let argb = ColorUtils::argb_from_lstar(lstar);
            let y = ColorUtils::xyz_from_argb(argb)[1];
            let y2 = ColorUtils::y_from_lstar(lstar);
            assert_close_to(y, y2, 1.0);
        }
    }

    #[test]
    fn linearize_delinearize() {
        for rgb_component in full_rgb_range() {
            let converted = ColorUtils::delinearized(ColorUtils::linearized(rgb_component));
            assert_eq!(converted, rgb_component);
        }
    }
}
