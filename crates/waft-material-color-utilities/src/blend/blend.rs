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
// Ported from `lib/blend/blend.dart` of material_color_utilities 0.13.0.

use crate::dart_math;
use crate::hct::cam16::Cam16;
use crate::hct::hct::Hct;
use crate::utils::color_utils::ColorUtils;
use crate::utils::math_utils::MathUtils;

/// Functions for blending in HCT and CAM16.
pub struct Blend;

impl Blend {
    /// Blend the design color's HCT hue towards the key color's HCT
    /// hue, in a way that leaves the original color recognizable and
    /// recognizably shifted towards the key color.
    ///
    /// `design_color` ARGB representation of an arbitrary color.
    /// `source_color` ARGB representation of the main theme color.
    /// Returns The design color with a hue shifted towards the
    /// system's color, a slightly warmer/cooler variant of the design
    /// color's hue.
    pub fn harmonize(design_color: u32, source_color: u32) -> u32 {
        let from_hct = Hct::from_int(design_color);
        let to_hct = Hct::from_int(source_color);
        let difference_degrees = MathUtils::difference_degrees(from_hct.hue(), to_hct.hue());
        let rotation_degrees = dart_math::min(difference_degrees * 0.5, 15.0);
        let output_hue = MathUtils::sanitize_degrees_double(
            from_hct.hue()
                + rotation_degrees * MathUtils::rotation_direction(from_hct.hue(), to_hct.hue()),
        );
        Hct::from(output_hue, from_hct.chroma(), from_hct.tone()).to_int()
    }

    /// Blends hue from one color into another. The chroma and tone of
    /// the original color are maintained.
    ///
    /// `from` ARGB representation of color
    /// `to` ARGB representation of color
    /// `amount` how much blending to perform; 0.0 >= and <= 1.0
    /// Returns from, with a hue blended towards to. Chroma and tone
    /// are constant.
    pub fn hct_hue(from: u32, to: u32, amount: f64) -> u32 {
        let ucs = Self::cam16_ucs(from, to, amount);
        let ucs_cam = Cam16::from_int(ucs);
        let from_cam = Cam16::from_int(from);
        let blended = Hct::from(
            ucs_cam.hue,
            from_cam.chroma,
            ColorUtils::lstar_from_argb(from),
        );
        blended.to_int()
    }

    /// Blend in CAM16-UCS space.
    ///
    /// `from` ARGB representation of color
    /// `to` ARGB representation of color
    /// `amount` how much blending to perform; 0.0 >= and <= 1.0
    /// Returns from, blended towards to. Hue, chroma, and tone will
    /// change.
    pub fn cam16_ucs(from: u32, to: u32, amount: f64) -> u32 {
        let from_cam = Cam16::from_int(from);
        let to_cam = Cam16::from_int(to);
        let from_j = from_cam.jstar;
        let from_a = from_cam.astar;
        let from_b = from_cam.bstar;
        let to_j = to_cam.jstar;
        let to_a = to_cam.astar;
        let to_b = to_cam.bstar;
        let jstar = from_j + (to_j - from_j) * amount;
        let astar = from_a + (to_a - from_a) * amount;
        let bstar = from_b + (to_b - from_b) * amount;
        Cam16::from_ucs(jstar, astar, bstar).to_int()
    }
}

#[cfg(test)]
mod tests {
    // Ported from `test/blend_test.dart`.
    use super::*;
    use crate::test_support::is_color;

    const RED: u32 = 0xffff0000;
    const BLUE: u32 = 0xff0000ff;
    const GREEN: u32 = 0xff00ff00;
    const YELLOW: u32 = 0xffffff00;

    #[test]
    fn harmonize_red_to_blue() {
        is_color(Blend::harmonize(RED, BLUE), 0xFFFB0057);
    }

    #[test]
    fn harmonize_red_to_green() {
        is_color(Blend::harmonize(RED, GREEN), 0xFFD85600);
    }

    #[test]
    fn harmonize_red_to_yellow() {
        is_color(Blend::harmonize(RED, YELLOW), 0xFFD85600);
    }

    #[test]
    fn harmonize_blue_to_green() {
        is_color(Blend::harmonize(BLUE, GREEN), 0xFF0047A3);
    }

    #[test]
    fn harmonize_blue_to_red() {
        is_color(Blend::harmonize(BLUE, RED), 0xFF5700DC);
    }

    #[test]
    fn harmonize_blue_to_yellow() {
        is_color(Blend::harmonize(BLUE, YELLOW), 0xFF0047A3);
    }

    #[test]
    fn harmonize_green_to_blue() {
        is_color(Blend::harmonize(GREEN, BLUE), 0xFF00FC94);
    }

    #[test]
    fn harmonize_green_to_red() {
        is_color(Blend::harmonize(GREEN, RED), 0xFFB1F000);
    }

    #[test]
    fn harmonize_green_to_yellow() {
        is_color(Blend::harmonize(GREEN, YELLOW), 0xFFB1F000);
    }

    #[test]
    fn harmonize_yellow_to_blue() {
        is_color(Blend::harmonize(YELLOW, BLUE), 0xFFEBFFBA);
    }

    #[test]
    fn harmonize_yellow_to_green() {
        is_color(Blend::harmonize(YELLOW, GREEN), 0xFFEBFFBA);
    }

    #[test]
    fn harmonize_yellow_to_red() {
        is_color(Blend::harmonize(YELLOW, RED), 0xFFFFF6E3);
    }
}
