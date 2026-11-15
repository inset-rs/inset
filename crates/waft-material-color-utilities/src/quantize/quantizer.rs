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
// Ported from `lib/quantize/quantizer.dart` of material_color_utilities 0.13.0.

use std::future::Ready;

use indexmap::IndexMap;

/// Dart's `Future<QuantizerResult>` from `quantize`: every quantizer here finishes in the call,
/// so the future is ready, and `into_inner` takes its result without awaiting.
pub type QuantizerFuture = Ready<QuantizerResult>;

pub trait Quantizer {
    fn quantize(&mut self, pixels: &[u32], max_colors: i64) -> QuantizerFuture;
}

pub struct QuantizerResult {
    pub color_to_count: IndexMap<u32, i64>,

    pub input_pixel_to_cluster_pixel: IndexMap<u32, u32>,
}

impl QuantizerResult {
    pub fn new(color_to_count: IndexMap<u32, i64>) -> QuantizerResult {
        QuantizerResult {
            color_to_count,
            input_pixel_to_cluster_pixel: IndexMap::new(),
        }
    }

    /// Dart `QuantizerResult(inputPixelToClusterPixel:)`.
    pub fn input_pixel_to_cluster_pixel(
        mut self,
        input_pixel_to_cluster_pixel: IndexMap<u32, u32>,
    ) -> Self {
        self.input_pixel_to_cluster_pixel = input_pixel_to_cluster_pixel;
        self
    }
}
