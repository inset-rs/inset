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
// Ported from `lib/utils/math_utils.dart` of material_color_utilities 0.13.0.

use crate::dart_math;

/// Utility methods for mathematical operations.
pub struct MathUtils;

impl MathUtils {
    /// The signum function.
    ///
    /// Returns 1 if num > 0, -1 if num < 0, and 0 if num = 0
    pub fn signum(num: f64) -> i64 {
        if num < 0.0 {
            -1
        } else if num == 0.0 {
            0
        } else {
            1
        }
    }

    /// The linear interpolation function.
    ///
    /// Returns start if amount = 0 and stop if amount = 1
    pub fn lerp(start: f64, stop: f64, amount: f64) -> f64 {
        (1.0 - amount) * start + amount * stop
    }

    /// Clamps an integer between two integers.
    ///
    /// Returns input when min <= input <= max, and either min or max
    /// otherwise.
    pub fn clamp_int(min: i64, max: i64, input: i64) -> i64 {
        if input < min {
            return min;
        } else if input > max {
            return max;
        }

        input
    }

    /// Clamps an integer between two floating-point numbers.
    ///
    /// Returns input when min <= input <= max, and either min or max
    /// otherwise.
    pub fn clamp_double(min: f64, max: f64, input: f64) -> f64 {
        if input < min {
            return min;
        } else if input > max {
            return max;
        }

        input
    }

    /// Sanitizes a degree measure as an integer.
    ///
    /// Returns a degree measure between 0 (inclusive) and 360
    /// (exclusive).
    pub fn sanitize_degrees_int(mut degrees: i64) -> i64 {
        degrees %= 360;
        if degrees < 0 {
            degrees += 360;
        }
        degrees
    }

    /// Sanitizes a degree measure as a floating-point number.
    ///
    /// Returns a degree measure between 0.0 (inclusive) and 360.0
    /// (exclusive).
    pub fn sanitize_degrees_double(mut degrees: f64) -> f64 {
        degrees = dart_math::modulo(degrees, 360.0);
        if degrees < 0.0 {
            degrees += 360.0;
        }
        degrees
    }

    /// Sign of direction change needed to travel from one angle to
    /// another.
    ///
    /// For angles that are 180 degrees apart from each other, both
    /// directions have the same travel distance, so either direction is
    /// shortest. The value 1.0 is returned in this case.
    ///
    /// `from` The angle travel starts from, in degrees.
    /// `to` The angle travel ends at, in degrees.
    /// Returns -1 if decreasing from leads to the shortest travel
    /// distance, 1 if increasing from leads to the shortest travel
    /// distance.
    pub fn rotation_direction(from: f64, to: f64) -> f64 {
        let increasing_difference = Self::sanitize_degrees_double(to - from);
        if increasing_difference <= 180.0 {
            1.0
        } else {
            -1.0
        }
    }

    /// Distance of two points on a circle, represented using degrees.
    pub fn difference_degrees(a: f64, b: f64) -> f64 {
        180.0 - ((a - b).abs() - 180.0).abs()
    }

    /// Multiplies a 1x3 row vector with a 3x3 matrix.
    pub fn matrix_multiply(row: &[f64; 3], matrix: &[[f64; 3]; 3]) -> [f64; 3] {
        let a = row[0] * matrix[0][0] + row[1] * matrix[0][1] + row[2] * matrix[0][2];
        let b = row[0] * matrix[1][0] + row[1] * matrix[1][1] + row[2] * matrix[1][2];
        let c = row[0] * matrix[2][0] + row[1] * matrix[2][1] + row[2] * matrix[2][2];
        [a, b, c]
    }
}

#[cfg(test)]
mod tests {
    // Ported from `test/math_utils_test.dart`.
    use super::*;

    // Original implementation for MathUtils.rotationDirection.
    // Included here to test equivalence with new implementation.
    fn rotation_direction(from: f64, to: f64) -> f64 {
        let a = to - from;
        let b = to - from + 360.0;
        let c = to - from - 360.0;
        let a_abs = a.abs();
        let b_abs = b.abs();
        let c_abs = c.abs();
        if a_abs <= b_abs && a_abs <= c_abs {
            if a >= 0.0 { 1.0 } else { -1.0 }
        } else if b_abs <= a_abs && b_abs <= c_abs {
            if b >= 0.0 { 1.0 } else { -1.0 }
        } else if c >= 0.0 {
            1.0
        } else {
            -1.0
        }
    }

    #[test]
    fn rotation_direction_behaves_correctly() {
        let mut from = 0.0;
        while from < 360.0 {
            let mut to = 7.5;
            while to < 360.0 {
                let expected_answer = rotation_direction(from, to);
                let actual_answer = MathUtils::rotation_direction(from, to);
                assert_eq!(
                    actual_answer, expected_answer,
                    "should be {expected_answer} from {from} to {to}"
                );
                assert_eq!(
                    actual_answer.abs(),
                    1.0,
                    "should be either +1.0 or -1.0from {from} to {to} (got {actual_answer})"
                );
                to += 15.0;
            }
            from += 15.0;
        }
    }

    // Not in the package's tests: Dart's `%` gives `0.0`, not `-0.0`, for a zero remainder.
    // The expected values are the Dart package's, printed by
    // `MathUtils.sanitizeDegreesDouble(-360.0).isNegative` and the same with `-0.0` on Dart 3.13.1.
    #[test]
    fn sanitize_degrees_double_gives_a_positive_zero_as_dart_does() {
        assert!(MathUtils::sanitize_degrees_double(-360.0).is_sign_positive());
        assert!(MathUtils::sanitize_degrees_double(-0.0).is_sign_positive());
        assert_eq!(MathUtils::sanitize_degrees_double(-360.0), 0.0);
        assert_eq!(MathUtils::sanitize_degrees_double(-10.0), 350.0);
    }
}
