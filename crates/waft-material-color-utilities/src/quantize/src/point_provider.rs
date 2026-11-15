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
// Ported from `lib/quantize/src/point_provider.dart` of material_color_utilities 0.13.0.

// Dart's names: `from_int` reads a point from a color.
#[allow(clippy::wrong_self_convention)]
pub trait PointProvider {
    fn from_int(&self, argb: u32) -> [f64; 3];
    fn to_int(&self, point: &[f64; 3]) -> u32;
    fn distance(&self, a: &[f64; 3], b: &[f64; 3]) -> f64;
}
