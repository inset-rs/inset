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
// Ported from `lib/hct/hct.dart` of material_color_utilities 0.13.0.

use std::fmt;
use std::hash::{Hash, Hasher};

use crate::dart_math;
use crate::hct::cam16::Cam16;
use crate::hct::src::hct_solver::HctSolver;
use crate::hct::viewing_conditions::ViewingConditions;
use crate::utils::color_utils::ColorUtils;

/// HCT, hue, chroma, and tone. A color system that provides a perceptually
/// accurate color measurement system that can also accurately render what
/// colors will appear as in different lighting environments.
///
/// A value: a copy is a separate color, and setting its hue, chroma or tone leaves the
/// original as it was. Two `Hct`s are equal when their ARGB is.
#[derive(Clone, Copy)]
pub struct Hct {
    hue: f64,
    chroma: f64,
    tone: f64,
    argb: u32,
}

impl Hct {
    /// 0 <= `hue` < 360; invalid values are corrected.
    /// 0 <= `chroma` <= ?; Informally, colorfulness. The color returned may be
    ///    lower than the requested chroma. Chroma has a different maximum for any
    ///    given hue and tone.
    /// 0 <= `tone` <= 100; informally, lightness. Invalid values are corrected.
    pub fn from(hue: f64, chroma: f64, tone: f64) -> Hct {
        let argb = HctSolver::solve_to_int(hue, chroma, tone);
        Hct::new(argb)
    }

    /// HCT representation of `argb`.
    pub fn from_int(argb: u32) -> Hct {
        Hct::new(argb)
    }

    pub fn to_int(&self) -> u32 {
        self.argb
    }

    /// A number, in degrees, representing ex. red, orange, yellow, etc.
    /// Ranges from 0 <= [`hue`](Self::hue) < 360
    pub fn hue(&self) -> f64 {
        self.hue
    }

    /// 0 <= `new_hue` < 360; invalid values are corrected.
    /// After setting hue, the color is mapped from HCT to the more
    /// limited sRGB gamut for display. This will change its ARGB/integer
    /// representation. If the HCT color is outside of the sRGB gamut, chroma
    /// will decrease until it is inside the gamut.
    pub fn set_hue(&mut self, new_hue: f64) {
        self.argb = HctSolver::solve_to_int(new_hue, self.chroma(), self.tone());
        let cam16 = Cam16::from_int(self.argb);
        self.hue = cam16.hue;
        self.chroma = cam16.chroma;
        self.tone = ColorUtils::lstar_from_argb(self.argb);
    }

    pub fn chroma(&self) -> f64 {
        self.chroma
    }

    /// 0 <= `new_chroma` <= ?
    /// After setting chroma, the color is mapped from HCT to the more
    /// limited sRGB gamut for display. This will change its ARGB/integer
    /// representation. If the HCT color is outside of the sRGB gamut, chroma
    /// will decrease until it is inside the gamut.
    pub fn set_chroma(&mut self, new_chroma: f64) {
        self.argb = HctSolver::solve_to_int(self.hue(), new_chroma, self.tone());
        let cam16 = Cam16::from_int(self.argb);
        self.hue = cam16.hue;
        self.chroma = cam16.chroma;
        self.tone = ColorUtils::lstar_from_argb(self.argb);
    }

    /// Lightness. Ranges from 0 to 100.
    pub fn tone(&self) -> f64 {
        self.tone
    }

    /// 0 <= `new_tone` <= 100; invalid values are corrected.
    /// After setting tone, the color is mapped from HCT to the more
    /// limited sRGB gamut for display. This will change its ARGB/integer
    /// representation. If the HCT color is outside of the sRGB gamut, chroma
    /// will decrease until it is inside the gamut.
    pub fn set_tone(&mut self, new_tone: f64) {
        self.argb = HctSolver::solve_to_int(self.hue(), self.chroma(), new_tone);
        let cam16 = Cam16::from_int(self.argb);
        self.hue = cam16.hue;
        self.chroma = cam16.chroma;
        self.tone = ColorUtils::lstar_from_argb(self.argb);
    }

    fn new(argb: u32) -> Hct {
        let cam16 = Cam16::from_int(argb);
        Hct {
            argb,
            hue: cam16.hue,
            chroma: cam16.chroma,
            tone: ColorUtils::lstar_from_argb(argb),
        }
    }

    /// Translate a color into different [`ViewingConditions`].
    ///
    /// Colors change appearance. They look different with lights on versus off,
    /// the same color, as in hex code, on white looks different when on black.
    /// This is called color relativity, most famously explicated by Josef Albers
    /// in Interaction of Color.
    ///
    /// In color science, color appearance models can account for this and
    /// calculate the appearance of a color in different settings. HCT is based on
    /// CAM16, a color appearance model, and uses it to make these calculations.
    ///
    /// See [`ViewingConditions::make`] for parameters affecting color appearance.
    pub fn in_viewing_conditions(&self, vc: &ViewingConditions) -> Hct {
        // 1. Use CAM16 to find XYZ coordinates of color in specified VC.
        let cam16 = Cam16::from_int(self.to_int());
        let viewed_in_vc = cam16.xyz_in_viewing_conditions(vc, None);

        // 2. Create CAM16 of those XYZ coordinates in default VC.
        let recast_in_vc = Cam16::from_xyz_in_viewing_conditions(
            viewed_in_vc[0],
            viewed_in_vc[1],
            viewed_in_vc[2],
            &ViewingConditions::make().build(),
        );

        // 3. Create HCT from:
        // - CAM16 using default VC with XYZ coordinates in specified VC.
        // - L* converted from Y in XYZ coordinates in specified VC.
        Hct::from(
            recast_in_vc.hue,
            recast_in_vc.chroma,
            ColorUtils::lstar_from_y(viewed_in_vc[1]),
        )
    }
}

impl PartialEq for Hct {
    fn eq(&self, other: &Self) -> bool {
        other.argb == self.argb
    }
}

impl Eq for Hct {}

impl Hash for Hct {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.argb.hash(state);
    }
}

impl fmt::Debug for Hct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "H{} C{} T{}",
            dart_math::round(self.hue()),
            dart_math::round(self.chroma()),
            dart_math::round(self.tone())
        )
    }
}

#[cfg(test)]
mod tests {
    // Ported from `test/hct_test.dart`.
    use std::collections::hash_map::DefaultHasher;

    use super::*;
    use crate::test_support::is_color;
    use crate::utils::string_utils::StringUtils;

    const BLACK: u32 = 0xff000000;
    const WHITE: u32 = 0xffffffff;
    const RED: u32 = 0xffff0000;
    const GREEN: u32 = 0xff00ff00;
    const BLUE: u32 = 0xff0000ff;
    const MIDGRAY: u32 = 0xff777777;

    fn color_is_on_boundary(argb: u32) -> bool {
        ColorUtils::red_from_argb(argb) == 0
            || ColorUtils::red_from_argb(argb) == 255
            || ColorUtils::green_from_argb(argb) == 0
            || ColorUtils::green_from_argb(argb) == 255
            || ColorUtils::blue_from_argb(argb) == 0
            || ColorUtils::blue_from_argb(argb) == 255
    }

    fn assert_close_to(actual: f64, expected: f64, delta: f64) {
        assert!(
            (actual - expected).abs() <= delta,
            "expected {actual} to be within {delta} of {expected}"
        );
    }

    fn hash_of(hct: &Hct) -> u64 {
        let mut hasher = DefaultHasher::new();
        hct.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn equality_and_hash_code_basics() {
        assert_eq!(Hct::from_int(123), Hct::from_int(123));
        assert_eq!(hash_of(&Hct::from_int(123)), hash_of(&Hct::from_int(123)));
    }

    #[test]
    fn conversions_are_reflexive() {
        let cam = Cam16::from_int(RED);
        let color = cam.viewed(ViewingConditions::standard());
        assert_eq!(color, RED);
    }

    #[test]
    fn y_midgray() {
        assert_close_to(18.418, ColorUtils::y_from_lstar(50.0), 0.001);
    }

    #[test]
    fn y_black() {
        assert_close_to(0.0, ColorUtils::y_from_lstar(0.0), 0.001);
    }

    #[test]
    fn y_white() {
        assert_close_to(100.0, ColorUtils::y_from_lstar(100.0), 0.001);
    }

    fn assert_cam(argb: u32, j: f64, chroma: f64, hue: f64, m: f64, s: f64, q: f64) {
        let cam = Cam16::from_int(argb);
        assert_close_to(j, cam.j, 0.001);
        assert_close_to(chroma, cam.chroma, 0.001);
        assert_close_to(hue, cam.hue, 0.001);
        assert_close_to(m, cam.m, 0.001);
        assert_close_to(s, cam.s, 0.001);
        assert_close_to(q, cam.q, 0.001);
    }

    #[test]
    fn cam_red() {
        assert_cam(RED, 46.445, 113.357, 27.408, 89.494, 91.889, 105.988);
    }

    #[test]
    fn cam_green() {
        assert_cam(GREEN, 79.331, 108.410, 142.139, 85.587, 78.604, 138.520);
    }

    #[test]
    fn cam_blue() {
        assert_cam(BLUE, 25.465, 87.230, 282.788, 68.867, 93.674, 78.481);
    }

    #[test]
    fn cam_black() {
        assert_cam(BLACK, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    }

    #[test]
    fn cam_white() {
        assert_cam(WHITE, 100.0, 2.869, 209.492, 2.265, 12.068, 155.521);
    }

    fn gamut_map(color_to_test: u32) -> u32 {
        let cam = Cam16::from_int(color_to_test);
        Hct::from(
            cam.hue,
            cam.chroma,
            ColorUtils::lstar_from_argb(color_to_test),
        )
        .to_int()
    }

    #[test]
    fn gamut_map_red() {
        assert_eq!(RED, gamut_map(RED));
    }

    #[test]
    fn gamut_map_green() {
        assert_eq!(GREEN, gamut_map(GREEN));
    }

    #[test]
    fn gamut_map_blue() {
        assert_eq!(BLUE, gamut_map(BLUE));
    }

    #[test]
    fn gamut_map_white() {
        assert_eq!(WHITE, gamut_map(WHITE));
    }

    #[test]
    fn gamut_map_midgray() {
        assert_eq!(GREEN, gamut_map(GREEN));
    }

    #[test]
    fn hct_returns_a_sufficiently_close_color() {
        for hue in (15..360).step_by(30) {
            for chroma in (0..=100).step_by(10) {
                for tone in (20..=80).step_by(10) {
                    let hct_request_description = format!("H{hue} C{chroma} T{tone}");
                    let hct_color = Hct::from(hue as f64, chroma as f64, tone as f64);

                    if chroma > 0 {
                        assert!(
                            (hct_color.hue() - hue as f64).abs() <= 4.0,
                            "Hue should be close for {hct_request_description}"
                        );
                    }

                    assert!(
                        (0.0..=chroma as f64 + 2.5).contains(&hct_color.chroma()),
                        "Chroma should be close or less for {hct_request_description}"
                    );

                    if hct_color.chroma() < chroma as f64 - 2.5 {
                        assert!(
                            color_is_on_boundary(hct_color.to_int()),
                            "HCT request for non-sRGB color should return a color on the boundary of the sRGB cube for {hct_request_description}, but got {} instead",
                            StringUtils::hex_from_argb(hct_color.to_int(), true)
                        );
                    }

                    assert!(
                        (hct_color.tone() - tone as f64).abs() <= 0.5,
                        "Tone should be close for {hct_request_description}"
                    );
                }
            }
        }
    }

    #[test]
    fn cam16_to_xyz_without_array() {
        let color_to_test = RED;
        let cam = Cam16::from_int(color_to_test);
        let xyz = cam.xyz_in_viewing_conditions(ViewingConditions::s_rgb(), None);
        assert_close_to(xyz[0], 41.23, 0.01);
        assert_close_to(xyz[1], 21.26, 0.01);
        assert_close_to(xyz[2], 1.93, 0.01);
    }

    #[test]
    fn cam16_to_xyz_with_array() {
        let color_to_test = RED;
        let cam = Cam16::from_int(color_to_test);
        let mut array = [0.0, 0.0, 0.0];
        let xyz = cam.xyz_in_viewing_conditions(ViewingConditions::s_rgb(), Some(&mut array));
        assert_close_to(xyz[0], 41.23, 0.01);
        assert_close_to(xyz[1], 21.26, 0.01);
        assert_close_to(xyz[2], 1.93, 0.01);
        assert_eq!(array, xyz);
    }

    fn in_background(color_to_test: u32, background_lstar: f64) -> u32 {
        let hct = Hct::from_int(color_to_test);
        hct.in_viewing_conditions(
            &ViewingConditions::make()
                .background_lstar(background_lstar)
                .build(),
        )
        .to_int()
    }

    #[test]
    fn color_relativity_red_in_black() {
        is_color(in_background(RED, 0.0), 0xFF9F5C51);
    }

    #[test]
    fn color_relativity_red_in_white() {
        is_color(in_background(RED, 100.0), 0xFFFF5D48);
    }

    #[test]
    fn color_relativity_green_in_black() {
        is_color(in_background(GREEN, 0.0), 0xFFACD69D);
    }

    #[test]
    fn color_relativity_green_in_white() {
        is_color(in_background(GREEN, 100.0), 0xFF8EFF77);
    }

    #[test]
    fn color_relativity_blue_in_black() {
        is_color(in_background(BLUE, 0.0), 0xff343654);
    }

    #[test]
    fn color_relativity_blue_in_white() {
        is_color(in_background(BLUE, 100.0), 0xFF3F49FF);
    }

    #[test]
    fn color_relativity_white_in_black() {
        is_color(in_background(WHITE, 0.0), 0xFFFFFFFF);
    }

    #[test]
    fn color_relativity_white_in_white() {
        is_color(in_background(WHITE, 100.0), 0xFFFFFFFF);
    }

    #[test]
    fn color_relativity_midgray_in_black() {
        is_color(in_background(MIDGRAY, 0.0), 0xFF605F5F);
    }

    #[test]
    fn color_relativity_midgray_in_white() {
        is_color(in_background(MIDGRAY, 100.0), 0xFF8E8E8E);
    }

    #[test]
    fn color_relativity_black_in_black() {
        is_color(in_background(BLACK, 0.0), 0xff000000);
    }

    #[test]
    fn color_relativity_black_in_white() {
        is_color(in_background(BLACK, 100.0), 0xff000000);
    }

    // Ported from `test/hct_round_trip_test.dart`.
    #[test]
    fn hct_preserves_original_color() {
        for argb in 0xFF000000..=0xFFFFFFFFu32 {
            let hct = Hct::from_int(argb);
            let reconstructed_argb = Hct::from(hct.hue(), hct.chroma(), hct.tone()).to_int();

            is_color(reconstructed_argb, argb);
        }
    }
}
