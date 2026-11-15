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
// Ported from `lib/quantize/src/point_provider_lab.dart` of material_color_utilities 0.13.0.

use crate::quantize::src::point_provider::PointProvider;
use crate::utils::color_utils::ColorUtils;

pub struct PointProviderLab;

impl PointProvider for PointProviderLab {
    fn from_int(&self, argb: u32) -> [f64; 3] {
        ColorUtils::lab_from_argb(argb)
    }

    fn to_int(&self, lab: &[f64; 3]) -> u32 {
        ColorUtils::argb_from_lab(lab[0], lab[1], lab[2])
    }

    fn distance(&self, one: &[f64; 3], two: &[f64; 3]) -> f64 {
        let d_l = one[0] - two[0];
        let d_a = one[1] - two[1];
        let d_b = one[2] - two[2];
        // Standard CIE 1976 delta E formula also takes the square root, unneeded
        // here. This method is used by quantization algorithms to compare distance,
        // and the relative ordering is the same, with or without a square root.

        // This relatively minor optimization is helpful because this method is
        // called at least once for each pixel in an image.
        d_l * d_l + d_a * d_a + d_b * d_b
    }
}
