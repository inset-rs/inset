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
// Ported from `lib/hct/cam16.dart` of material_color_utilities 0.13.0.

use std::f64::consts::PI;

use crate::dart_math;
use crate::hct::viewing_conditions::ViewingConditions;
use crate::utils::color_utils::ColorUtils;
use crate::utils::math_utils::MathUtils;

/// CAM16, a color appearance model. Colors are not just defined by their hex
/// code, but rather, a hex code and viewing conditions.
///
/// CAM16 instances also have coordinates in the CAM16-UCS space, called J*, a*,
/// b*, or jstar, astar, bstar in code. CAM16-UCS is included in the CAM16
/// specification, and should be used when measuring distances between colors.
///
/// In traditional color spaces, a color can be identified solely by the
/// observer's measurement of the color. Color appearance models such as CAM16
/// also use information about the environment where the color was
/// observed, known as the viewing conditions.
///
/// For example, white under the traditional assumption of a midday sun white
/// point is accurately measured as a slightly chromatic blue by CAM16.
/// (roughly, hue 203, chroma 3, lightness 100)
/// CAM16, a color appearance model. Colors are not just defined by their hex
/// code, but rather, a hex code and viewing conditions.
///
/// CAM16 instances also have coordinates in the CAM16-UCS space, called J*, a*,
/// b*, or jstar, astar, bstar in code. CAM16-UCS is included in the CAM16
/// specification, and should be used when measuring distances between colors.
///
/// In traditional color spaces, a color can be identified solely by the
/// observer's measurement of the color. Color appearance models such as CAM16
/// also use information about the environment where the color was
/// observed, known as the viewing conditions.
///
/// For example, white under the traditional assumption of a midday sun white
/// point is accurately measured as a slightly chromatic blue by CAM16.
/// (roughly, hue 203, chroma 3, lightness 100)
#[derive(Clone, Copy, Debug)]
pub struct Cam16 {
    /// Like red, orange, yellow, green, etc.
    pub hue: f64,

    /// Informally, colorfulness / color intensity. Like saturation in HSL,
    /// except perceptually accurate.
    pub chroma: f64,

    /// Lightness
    pub j: f64,

    /// Brightness; ratio of lightness to white point's lightness
    pub q: f64,

    /// Colorfulness
    pub m: f64,

    /// Saturation; ratio of chroma to white point's chroma
    pub s: f64,

    /// CAM16-UCS J coordinate
    pub jstar: f64,

    /// CAM16-UCS a coordinate
    pub astar: f64,

    /// CAM16-UCS b coordinate
    pub bstar: f64,
}

impl Cam16 {
    /// All of the CAM16 dimensions can be calculated from 3 of the dimensions, in
    /// the following combinations:
    ///     -  {j or q} and {c, m, or s} and hue
    ///     - jstar, astar, bstar
    /// Prefer using a static method that constructs from 3 of those dimensions.
    /// This constructor is intended for those methods to use to return all
    /// possible dimensions.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        hue: f64,
        chroma: f64,
        j: f64,
        q: f64,
        m: f64,
        s: f64,
        jstar: f64,
        astar: f64,
        bstar: f64,
    ) -> Cam16 {
        Cam16 {
            hue,
            chroma,
            j,
            q,
            m,
            s,
            jstar,
            astar,
            bstar,
        }
    }

    /// CAM16 instances also have coordinates in the CAM16-UCS space, called J*,
    /// a*, b*, or jstar, astar, bstar in code. CAM16-UCS is included in the CAM16
    /// specification, and should be used when measuring distances between colors.
    pub fn distance(&self, other: &Cam16) -> f64 {
        let d_j = self.jstar - other.jstar;
        let d_a = self.astar - other.astar;
        let d_b = self.bstar - other.bstar;
        let d_e_prime = (d_j * d_j + d_a * d_a + d_b * d_b).sqrt();
        1.41 * d_e_prime.powf(0.63)
    }

    /// Convert `argb` to CAM16, assuming the color was viewed in default viewing
    /// conditions.
    pub fn from_int(argb: u32) -> Cam16 {
        Self::from_int_in_viewing_conditions(argb, ViewingConditions::s_rgb())
    }

    /// Given `viewing_conditions`, convert `argb` to CAM16.
    pub fn from_int_in_viewing_conditions(
        argb: u32,
        viewing_conditions: &ViewingConditions,
    ) -> Cam16 {
        // Transform ARGB int to XYZ
        let xyz = ColorUtils::xyz_from_argb(argb);
        let x = xyz[0];
        let y = xyz[1];
        let z = xyz[2];
        Self::from_xyz_in_viewing_conditions(x, y, z, viewing_conditions)
    }

    /// Given color expressed in XYZ and viewed in `viewing_conditions`, convert to
    /// CAM16.
    pub fn from_xyz_in_viewing_conditions(
        x: f64,
        y: f64,
        z: f64,
        viewing_conditions: &ViewingConditions,
    ) -> Cam16 {
        // Transform XYZ to 'cone'/'rgb' responses

        let r_c = 0.401288 * x + 0.650173 * y - 0.051461 * z;
        let g_c = -0.250268 * x + 1.204414 * y + 0.045854 * z;
        let b_c = -0.002079 * x + 0.048952 * y + 0.953127 * z;

        // Discount illuminant
        let r_d = viewing_conditions.rgb_d[0] * r_c;
        let g_d = viewing_conditions.rgb_d[1] * g_c;
        let b_d = viewing_conditions.rgb_d[2] * b_c;

        // chromatic adaptation
        let r_af = (viewing_conditions.fl * r_d.abs() / 100.0).powf(0.42);
        let g_af = (viewing_conditions.fl * g_d.abs() / 100.0).powf(0.42);
        let b_af = (viewing_conditions.fl * b_d.abs() / 100.0).powf(0.42);
        let r_a = MathUtils::signum(r_d) as f64 * 400.0 * r_af / (r_af + 27.13);
        let g_a = MathUtils::signum(g_d) as f64 * 400.0 * g_af / (g_af + 27.13);
        let b_a = MathUtils::signum(b_d) as f64 * 400.0 * b_af / (b_af + 27.13);

        // redness-greenness
        let a = (11.0 * r_a + -12.0 * g_a + b_a) / 11.0;
        // yellowness-blueness
        let b = (r_a + g_a - 2.0 * b_a) / 9.0;

        // auxiliary components
        let u = (20.0 * r_a + 20.0 * g_a + 21.0 * b_a) / 20.0;
        let p2 = (40.0 * r_a + 20.0 * g_a + b_a) / 20.0;

        // hue
        let atan2 = b.atan2(a);
        let atan_degrees = atan2 * 180.0 / PI;
        let hue = if atan_degrees < 0.0 {
            atan_degrees + 360.0
        } else if atan_degrees >= 360.0 {
            atan_degrees - 360.0
        } else {
            atan_degrees
        };
        let hue_radians = hue * PI / 180.0;
        debug_assert!((0.0..360.0).contains(&hue), "hue was really {hue}");

        // achromatic response to color
        let ac = p2 * viewing_conditions.nbb;

        // CAM16 lightness and brightness
        let j =
            100.0 * (ac / viewing_conditions.aw).powf(viewing_conditions.c * viewing_conditions.z);
        let q = (4.0 / viewing_conditions.c)
            * (j / 100.0).sqrt()
            * (viewing_conditions.aw + 4.0)
            * (viewing_conditions.f_l_root);

        let hue_prime = if hue < 20.14 { hue + 360.0 } else { hue };
        let e_hue = (1.0 / 4.0) * ((hue_prime * PI / 180.0 + 2.0).cos() + 3.8);
        let p1 = 50000.0 / 13.0 * e_hue * viewing_conditions.n_c * viewing_conditions.ncb;
        let t = p1 * (a * a + b * b).sqrt() / (u + 0.305);
        let alpha = t.powf(0.9)
            * (1.64 - 0.29f64.powf(viewing_conditions.background_y_towhite_point_y)).powf(0.73);
        // CAM16 chroma, colorfulness, chroma
        let c = alpha * (j / 100.0).sqrt();
        let m = c * viewing_conditions.f_l_root;
        let s = 50.0 * ((alpha * viewing_conditions.c) / (viewing_conditions.aw + 4.0)).sqrt();

        // CAM16-UCS components
        let jstar = (1.0 + 100.0 * 0.007) * j / (1.0 + 0.007 * j);
        let mstar = (1.0 + 0.0228 * m).ln() / 0.0228;
        let astar = mstar * hue_radians.cos();
        let bstar = mstar * hue_radians.sin();
        Cam16::new(hue, c, j, q, m, s, jstar, astar, bstar)
    }

    /// Create a CAM16 color from lightness `j`, chroma `c`, and hue `h`,
    /// assuming the color was viewed in default viewing conditions.
    pub fn from_jch(j: f64, c: f64, h: f64) -> Cam16 {
        Self::from_jch_in_viewing_conditions(j, c, h, ViewingConditions::s_rgb())
    }

    /// Create a CAM16 color from lightness `j`, chroma `c`, and hue `h`,
    /// in `viewing_conditions`.
    pub fn from_jch_in_viewing_conditions(
        j: f64,
        c: f64,
        h: f64,
        viewing_conditions: &ViewingConditions,
    ) -> Cam16 {
        let q = (4.0 / viewing_conditions.c)
            * (j / 100.0).sqrt()
            * (viewing_conditions.aw + 4.0)
            * (viewing_conditions.f_l_root);
        let m = c * viewing_conditions.f_l_root;
        let alpha = c / (j / 100.0).sqrt();
        let s = 50.0 * ((alpha * viewing_conditions.c) / (viewing_conditions.aw + 4.0)).sqrt();

        let hue_radians = h * PI / 180.0;
        let jstar = (1.0 + 100.0 * 0.007) * j / (1.0 + 0.007 * j);
        let mstar = 1.0 / 0.0228 * (1.0 + 0.0228 * m).ln();
        let astar = mstar * hue_radians.cos();
        let bstar = mstar * hue_radians.sin();
        Cam16::new(h, c, j, q, m, s, jstar, astar, bstar)
    }

    /// Create a CAM16 color from CAM16-UCS coordinates `jstar`, `astar`, `bstar`.
    /// assuming the color was viewed in default viewing conditions.
    pub fn from_ucs(jstar: f64, astar: f64, bstar: f64) -> Cam16 {
        Self::from_ucs_in_viewing_conditions(jstar, astar, bstar, ViewingConditions::standard())
    }

    /// Create a CAM16 color from CAM16-UCS coordinates `jstar`, `astar`, `bstar`.
    /// in `viewing_conditions`.
    pub fn from_ucs_in_viewing_conditions(
        jstar: f64,
        astar: f64,
        bstar: f64,
        viewing_conditions: &ViewingConditions,
    ) -> Cam16 {
        let a = astar;
        let b = bstar;
        let m = (a * a + b * b).sqrt();
        let big_m = ((m * 0.0228).exp() - 1.0) / 0.0228;
        let c = big_m / viewing_conditions.f_l_root;
        let mut h = b.atan2(a) * (180.0 / PI);
        if h < 0.0 {
            h += 360.0;
        }
        let j = jstar / (1.0 - (jstar - 100.0) * 0.007);

        Cam16::from_jch_in_viewing_conditions(j, c, h, viewing_conditions)
    }

    /// ARGB representation of color, assuming the color was viewed in default
    /// viewing conditions.
    pub fn to_int(&self) -> u32 {
        self.viewed(ViewingConditions::s_rgb())
    }

    /// ARGB representation of a color, given the color was viewed in
    /// `viewing_conditions`
    pub fn viewed(&self, viewing_conditions: &ViewingConditions) -> u32 {
        let xyz = self.xyz_in_viewing_conditions(viewing_conditions, None);
        ColorUtils::argb_from_xyz(xyz[0], xyz[1], xyz[2])
    }

    /// XYZ representation of CAM16 seen in `viewing_conditions`.
    ///
    /// When `array` is given, it is filled with the answer too.
    pub fn xyz_in_viewing_conditions(
        &self,
        viewing_conditions: &ViewingConditions,
        array: Option<&mut [f64; 3]>,
    ) -> [f64; 3] {
        let alpha = if self.chroma == 0.0 || self.j == 0.0 {
            0.0
        } else {
            self.chroma / (self.j / 100.0).sqrt()
        };

        let t = (alpha
            / (1.64 - 0.29f64.powf(viewing_conditions.background_y_towhite_point_y)).powf(0.73))
        .powf(1.0 / 0.9);
        let h_rad = self.hue * PI / 180.0;

        let e_hue = 0.25 * ((h_rad + 2.0).cos() + 3.8);
        let ac = viewing_conditions.aw
            * (self.j / 100.0).powf(1.0 / viewing_conditions.c / viewing_conditions.z);
        let p1 = e_hue * (50000.0 / 13.0) * viewing_conditions.n_c * viewing_conditions.ncb;

        let p2 = ac / viewing_conditions.nbb;

        let h_sin = h_rad.sin();
        let h_cos = h_rad.cos();

        let gamma = 23.0 * (p2 + 0.305) * t / (23.0 * p1 + 11.0 * t * h_cos + 108.0 * t * h_sin);
        let a = gamma * h_cos;
        let b = gamma * h_sin;
        let r_a = (460.0 * p2 + 451.0 * a + 288.0 * b) / 1403.0;
        let g_a = (460.0 * p2 - 891.0 * a - 261.0 * b) / 1403.0;
        let b_a = (460.0 * p2 - 220.0 * a - 6300.0 * b) / 1403.0;

        let r_c_base = dart_math::max(0.0, (27.13 * r_a.abs()) / (400.0 - r_a.abs()));
        let r_c = MathUtils::signum(r_a) as f64
            * (100.0 / viewing_conditions.fl)
            * r_c_base.powf(1.0 / 0.42);
        let g_c_base = dart_math::max(0.0, (27.13 * g_a.abs()) / (400.0 - g_a.abs()));
        let g_c = MathUtils::signum(g_a) as f64
            * (100.0 / viewing_conditions.fl)
            * g_c_base.powf(1.0 / 0.42);
        let b_c_base = dart_math::max(0.0, (27.13 * b_a.abs()) / (400.0 - b_a.abs()));
        let b_c = MathUtils::signum(b_a) as f64
            * (100.0 / viewing_conditions.fl)
            * b_c_base.powf(1.0 / 0.42);
        let r_f = r_c / viewing_conditions.rgb_d[0];
        let g_f = g_c / viewing_conditions.rgb_d[1];
        let b_f = b_c / viewing_conditions.rgb_d[2];

        let x = 1.86206786 * r_f - 1.01125463 * g_f + 0.14918677 * b_f;
        let y = 0.38752654 * r_f + 0.62144744 * g_f - 0.00897398 * b_f;
        let z = -0.01584150 * r_f - 0.03412294 * g_f + 1.04996444 * b_f;

        if let Some(array) = array {
            array[0] = x;
            array[1] = y;
            array[2] = z;
        }
        [x, y, z]
    }
}
