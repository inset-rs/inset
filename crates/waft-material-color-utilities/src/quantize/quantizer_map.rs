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
// Ported from `lib/quantize/quantizer_map.dart` of material_color_utilities 0.13.0.

use std::future;

use indexmap::IndexMap;

use crate::quantize::quantizer::{Quantizer, QuantizerFuture, QuantizerResult};
use crate::utils::color_utils::ColorUtils;

pub struct QuantizerMap;

impl Quantizer for QuantizerMap {
    fn quantize(&mut self, pixels: &[u32], _max_colors: i64) -> QuantizerFuture {
        let mut count_by_color: IndexMap<u32, i64> = IndexMap::new();
        for pixel in pixels {
            let alpha = ColorUtils::alpha_from_argb(*pixel);
            if alpha < 255 {
                continue;
            }
            *count_by_color.entry(*pixel).or_insert(0) += 1;
        }
        future::ready(QuantizerResult::new(count_by_color))
    }
}
