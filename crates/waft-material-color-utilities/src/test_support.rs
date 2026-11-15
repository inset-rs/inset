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
// Ported from `test/utils/color_matcher.dart` of material_color_utilities 0.13.0.

//! The package's colour matchers, as assertions.

use crate::hct::cam16::Cam16;
use crate::utils::string_utils::StringUtils;

/// Dart `expect(actual, isColor(color))`.
#[track_caller]
pub(crate) fn is_color(actual: u32, color: u32) {
    assert!(
        actual == color,
        "expected hex code\n  {}\nbut got\n  {}",
        StringUtils::hex_from_argb(color, true),
        StringUtils::hex_from_argb(actual, true)
    );
}

/// Dart `expect(actual, isCloseToColor(color))`.
#[track_caller]
pub(crate) fn is_close_to_color(actual: u32, color: u32) {
    // Succeeds if the two colors are fairly indistinguishable to the human eye,
    // using the value 5 as a threshold.
    assert!(
        Cam16::from_int(actual).distance(&Cam16::from_int(color)) <= 5.0,
        "expected close to hex code\n  {}\nbut got\n  {}",
        StringUtils::hex_from_argb(color, true),
        StringUtils::hex_from_argb(actual, true)
    );
}
