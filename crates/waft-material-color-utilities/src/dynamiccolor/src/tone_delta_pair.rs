// Copyright 2023 Google LLC
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
// Ported from `lib/dynamiccolor/src/tone_delta_pair.dart` of material_color_utilities 0.13.0.

use std::rc::Rc;

use crate::dynamiccolor::dynamic_color::DynamicColor;

/// Describes the different in tone between colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TonePolarity {
    Darker,
    Lighter,
    Nearer,
    Farther,
}

/// Documents a constraint between two DynamicColors, in which their tones must
/// have a certain distance from each other.
///
/// Prefer a DynamicColor with a background, this is for special cases when
/// designers want tonal distance, literally contrast, between two colors that
/// don't have a background / foreground relationship or a contrast guarantee.
pub struct ToneDeltaPair {
    pub role_a: Rc<DynamicColor>,
    pub role_b: Rc<DynamicColor>,
    pub delta: f64,
    pub polarity: TonePolarity,
    pub stay_together: bool,
}

impl ToneDeltaPair {
    /// Documents a constraint in tone distance between two DynamicColors.
    ///
    /// The polarity is an adjective that describes "A", compared to "B".
    ///
    /// For instance, ToneDeltaPair(A, B, 15, 'darker', stayTogether) states that
    /// A's tone should be at least 15 darker than B's.
    ///
    /// 'nearer' and 'farther' describes closeness to the surface roles. For
    /// instance, ToneDeltaPair(A, B, 10, 'nearer', stayTogether) states that A
    /// should be 10 lighter than B in light mode, and 10 darker than B in dark
    /// mode.
    ///
    /// `role_a` The first role in a pair.
    /// `role_b` The second role in a pair.
    /// `delta` Required difference between tones. Absolute value, negative
    /// values have undefined behavior.
    /// `polarity` The relative relation between tones of roleA and roleB,
    /// as described above.
    /// `stay_together` Whether these two roles should stay on the same side of
    /// the "awkward zone" (T50-59). This is necessary for certain cases where
    /// one role has two backgrounds.
    pub fn new(
        role_a: Rc<DynamicColor>,
        role_b: Rc<DynamicColor>,
        delta: f64,
        polarity: TonePolarity,
        stay_together: bool,
    ) -> ToneDeltaPair {
        ToneDeltaPair {
            role_a,
            role_b,
            delta,
            polarity,
            stay_together,
        }
    }
}
