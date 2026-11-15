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
// Ported from `lib/palettes/tonal_palette.dart` of material_color_utilities 0.13.0.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

use crate::hct::hct::Hct;

/// A convenience class for retrieving colors that are constant in hue and
/// chroma, but vary in tone.
///
/// This class can be instantiated in two ways:
/// 1. [`of`](Self::of) From hue and chroma. (preferred)
/// 2. [`from_list`](Self::from_list) From a fixed-size ([`TonalPalette::COMMON_SIZE`]) list of ints
///    representing ARBG colors. Correctness (constant hue and chroma) of the input
///    is not enforced. [`get`](Self::get) will only return the input colors, corresponding to
///    [`COMMON_TONES`](Self::COMMON_TONES). This also initializes the key color to black.
pub struct TonalPalette {
    hue: f64,
    chroma: f64,
    key_color: Hct,

    /// A cache containing keys-value pairs where:
    /// - keys are integers that represent tones, and
    /// - values are colors in ARGB format.
    cache: RefCell<HashMap<i64, u32>>,
    is_from_cache: bool,
}

impl TonalPalette {
    /// Commonly-used tone values.
    pub const COMMON_TONES: [i64; 13] = [0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 99, 100];

    pub const COMMON_SIZE: usize = Self::COMMON_TONES.len();

    pub fn hue(&self) -> f64 {
        self.hue
    }

    pub fn chroma(&self) -> f64 {
        self.chroma
    }

    pub fn key_color(&self) -> Hct {
        self.key_color
    }

    fn new_from_hct(hct: Hct) -> TonalPalette {
        TonalPalette {
            cache: RefCell::new(HashMap::new()),
            hue: hct.hue(),
            chroma: hct.chroma(),
            key_color: hct,
            is_from_cache: false,
        }
    }

    fn from_hue_and_chroma(hue: f64, chroma: f64) -> TonalPalette {
        TonalPalette {
            hue,
            chroma,
            cache: RefCell::new(HashMap::new()),
            key_color: KeyColor::new(hue, chroma).create(),
            is_from_cache: false,
        }
    }

    fn from_cache(cache: HashMap<i64, u32>, hue: f64, chroma: f64) -> TonalPalette {
        TonalPalette {
            hue,
            chroma,
            cache: RefCell::new(cache),
            key_color: KeyColor::new(hue, chroma).create(),
            is_from_cache: true,
        }
    }

    /// Create colors using `hue` and `chroma`.
    pub fn of(hue: f64, chroma: f64) -> TonalPalette {
        TonalPalette::from_hue_and_chroma(hue, chroma)
    }

    /// Create a Tonal Palette from hue and chroma of `hct`.
    pub fn from_hct(hct: Hct) -> TonalPalette {
        TonalPalette::new_from_hct(hct)
    }

    /// Create colors from a fixed-size list of ARGB color ints.
    ///
    /// Inverse of [`as_list`](Self::as_list).
    pub fn from_list(colors: &[u32]) -> TonalPalette {
        debug_assert!(colors.len() == Self::COMMON_SIZE);
        let mut cache = HashMap::new();
        for (index, tone_value) in Self::COMMON_TONES.iter().enumerate() {
            cache.insert(*tone_value, colors[index]);
        }

        // Approximately deduces the original hue and chroma that generated this
        // list of colors.
        // Uses the hue and chroma of the provided color with the highest chroma.

        let mut best_hue = 0.0;
        let mut best_chroma = 0.0;
        for argb in colors {
            let hct = Hct::from_int(*argb);

            // If the color is too close to white, its chroma may have been
            // affected by a known issue, so we ignore it.
            // https://github.com/material-foundation/material-color-utilities/issues/140

            if hct.tone() > 98.0 {
                continue;
            }

            if hct.chroma() > best_chroma {
                best_hue = hct.hue();
                best_chroma = hct.chroma();
            }
        }

        TonalPalette::from_cache(cache, best_hue, best_chroma)
    }

    /// Returns a fixed-size list of ARGB color ints for common tone values.
    ///
    /// Inverse of [`from_list`](Self::from_list).
    pub fn as_list(&self) -> Vec<u32> {
        Self::COMMON_TONES
            .iter()
            .map(|tone| self.get(*tone))
            .collect()
    }

    /// Returns the ARGB representation of an HCT color at the given `tone`.
    ///
    /// If the palette is constructed from a list of colors
    /// (i.e. using [`from_list`](Self::from_list)), the color provided at construction is returned
    /// if possible; otherwise the result is generated from the deduced
    /// [`hue`](Self::hue) and [`chroma`](Self::chroma).
    ///
    /// If the palette is constructed from a hue and chroma (i.e. using [`of`](Self::of) or
    /// [`from_hct`](Self::from_hct)), the result is generated from the given [`hue`](Self::hue) and
    /// [`chroma`](Self::chroma).
    pub fn get(&self, tone: i64) -> u32 {
        *self
            .cache
            .borrow_mut()
            .entry(tone)
            .or_insert_with(|| Hct::from(self.hue, self.chroma, tone as f64).to_int())
    }

    /// Returns the HCT color at the given `tone`.
    ///
    /// If the palette is constructed from a list of colors
    /// (i.e. using [`from_list`](Self::from_list)), the color provided at construction is returned
    /// if possible; otherwise the result is generated from the deduced
    /// [`hue`](Self::hue) and [`chroma`](Self::chroma).
    ///
    /// If the palette is constructed from a hue and chroma (i.e. using [`of`](Self::of) or
    /// [`from_hct`](Self::from_hct)), the result is generated from the given [`hue`](Self::hue) and
    /// [`chroma`](Self::chroma).
    pub fn get_hct(&self, tone: f64) -> Hct {
        let cached =
            Self::cache_key_of(tone).and_then(|key| self.cache.borrow().get(&key).copied());
        match cached {
            Some(argb) => Hct::from_int(argb),
            None => Hct::from(self.hue, self.chroma, tone),
        }
    }

    /// The int key a Dart map lookup with the double `tone` finds: Dart's `50.0` equals and
    /// hashes as `50`, and a fractional, infinite or NaN tone equals no int.
    fn cache_key_of(tone: f64) -> Option<i64> {
        const TWO_TO_63: f64 = 9223372036854775808.0;
        if tone.fract() == 0.0 && (-TWO_TO_63..TWO_TO_63).contains(&tone) {
            Some(tone as i64)
        } else {
            None
        }
    }
}

impl PartialEq for TonalPalette {
    fn eq(&self, other: &Self) -> bool {
        if !self.is_from_cache && !other.is_from_cache {
            // Both created with .of or .fromHct
            self.hue == other.hue && self.chroma == other.chroma
        } else {
            self.as_list() == other.as_list()
        }
    }
}

impl Hash for TonalPalette {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if !self.is_from_cache {
            hash_double(self.hue, state);
            hash_double(self.chroma, state);
        } else {
            self.as_list().hash(state);
        }
    }
}

/// Hashes `value` so that doubles Dart's `==` calls equal hash alike: `-0.0` as `0.0`.
fn hash_double<H: Hasher>(value: f64, state: &mut H) {
    let value = if value == 0.0 { 0.0 } else { value };
    value.to_bits().hash(state);
}

impl fmt::Debug for TonalPalette {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.is_from_cache {
            write!(f, "TonalPalette.of({:?}, {:?})", self.hue, self.chroma)
        } else {
            write!(f, "TonalPalette.fromList({:?})", self.as_list())
        }
    }
}

/// Key color is a color that represents the hue and chroma of a tonal palette.
pub struct KeyColor {
    hue: f64,
    requested_chroma: f64,

    /// Cache that maps (hue, tone) to max chroma to avoid duplicated HCT
    /// calculation.
    chroma_cache: RefCell<HashMap<i64, f64>>,
}

impl KeyColor {
    const MAX_CHROMA_VALUE: f64 = 200.0;

    pub fn new(hue: f64, requested_chroma: f64) -> KeyColor {
        KeyColor {
            hue,
            requested_chroma,
            chroma_cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn hue(&self) -> f64 {
        self.hue
    }

    pub fn requested_chroma(&self) -> f64 {
        self.requested_chroma
    }

    /// Creates a key color from a [`hue`](Self::hue) and a
    /// [`requested_chroma`](Self::requested_chroma).
    /// The key color is the first tone, starting from T50, matching the given hue
    /// and chroma.
    ///
    /// @return Key color [`Hct`]
    pub fn create(&self) -> Hct {
        // Pivot around T50 because T50 has the most chroma available, on
        // average. Thus it is most likely to have a direct answer.
        const PIVOT_TONE: i64 = 50;
        const TONE_STEP_SIZE: i64 = 1;
        // Epsilon to accept values slightly higher than the requested chroma.
        const EPSILON: f64 = 0.01;

        // Binary search to find the tone that can provide a chroma that is closest
        // to the requested chroma.
        let mut lower_tone: i64 = 0;
        let mut upper_tone: i64 = 100;
        while lower_tone < upper_tone {
            let mid_tone = (lower_tone + upper_tone) / 2;
            let is_ascending =
                self.max_chroma(mid_tone) < self.max_chroma(mid_tone + TONE_STEP_SIZE);
            let sufficient_chroma = self.max_chroma(mid_tone) >= self.requested_chroma - EPSILON;

            if sufficient_chroma {
                // Either range [lowerTone, midTone] or [midTone, upperTone] has
                // the answer, so search in the range that is closer the pivot tone.
                if (lower_tone - PIVOT_TONE).abs() < (upper_tone - PIVOT_TONE).abs() {
                    upper_tone = mid_tone;
                } else {
                    if lower_tone == mid_tone {
                        return Hct::from(self.hue, self.requested_chroma, lower_tone as f64);
                    }
                    lower_tone = mid_tone;
                }
            } else {
                // As there is no sufficient chroma in the midTone, follow the direction
                // to the chroma peak.
                if is_ascending {
                    lower_tone = mid_tone + TONE_STEP_SIZE;
                } else {
                    // Keep midTone for potential chroma peak.
                    upper_tone = mid_tone;
                }
            }
        }

        Hct::from(self.hue, self.requested_chroma, lower_tone as f64)
    }

    // Find the maximum chroma for a given tone
    fn max_chroma(&self, tone: i64) -> f64 {
        *self
            .chroma_cache
            .borrow_mut()
            .entry(tone)
            .or_insert_with(|| Hct::from(self.hue, Self::MAX_CHROMA_VALUE, tone as f64).chroma())
    }
}

#[cfg(test)]
mod tests {
    // Ported from the `TonalPalette` and `KeyColor` groups of `test/palettes_test.dart`.
    use std::collections::HashSet;
    use std::collections::hash_map::DefaultHasher;

    use super::*;
    use crate::test_support::{is_close_to_color, is_color};

    fn hash_of(palette: &TonalPalette) -> u64 {
        let mut hasher = DefaultHasher::new();
        palette.hash(&mut hasher);
        hasher.finish()
    }

    fn assert_close_to(actual: f64, expected: f64, delta: f64) {
        assert!(
            (actual - expected).abs() <= delta,
            "expected {actual} to be within {delta} of {expected}"
        );
    }

    // Regression test for https://github.com/material-foundation/material-color-utilities/issues/56
    #[test]
    fn of_and_from_list_constructors_operator_equals() {
        let a1 = TonalPalette::of(1.0, 1.0);
        let a2 = TonalPalette::of(1.0, 1.0);
        let b1 = TonalPalette::from_list(&TonalPalette::COMMON_TONES.map(|_| 0xDEADBEEF));
        let b2 = TonalPalette::from_list(&TonalPalette::COMMON_TONES.map(|_| 0xDEADBEEF));
        assert!(!(a1 == b1));
        assert!(!(b1 == a1));
        assert!(a1 != b1);
        assert!(b1 != a1);
        assert!(a1 == a2);
        assert!(b1 == b2);

        let c1 = TonalPalette::from_list(&TonalPalette::COMMON_TONES.map(|_| 123));

        let c2 = TonalPalette::from_list(
            &TonalPalette::COMMON_TONES.map(|e| if e < 15 { 456 } else { 123 }),
        );

        assert_eq!(c1.get(50), c2.get(50));
        assert!(!(c1 == c2));
    }

    #[test]
    fn of_constructor_tones_of_blue() {
        let hct = Hct::from_int(0xff0000ff);
        let tones = TonalPalette::of(hct.hue(), hct.chroma());

        assert_eq!(tones.get(0), 0xff000000);
        assert_eq!(tones.get(10), 0xff00006e);
        assert_eq!(tones.get(20), 0xff0001ac);
        assert_eq!(tones.get(30), 0xff0000ef);
        assert_eq!(tones.get(40), 0xff343dff);
        assert_eq!(tones.get(50), 0xff5a64ff);
        assert_eq!(tones.get(60), 0xff7c84ff);
        assert_eq!(tones.get(70), 0xff9da3ff);
        assert_eq!(tones.get(80), 0xffbec2ff);
        assert_eq!(tones.get(90), 0xffe0e0ff);
        assert_eq!(tones.get(95), 0xfff1efff);
        assert_eq!(tones.get(99), 0xfffffbff);
        assert_eq!(tones.get(100), 0xffffffff);

        // Tone not in [TonalPalette.commonTones]
        assert_eq!(tones.get(3), 0xff00003c);
    }

    #[test]
    fn of_constructor_as_list() {
        let hct = Hct::from_int(0xff0000ff);
        let tones = TonalPalette::of(hct.hue(), hct.chroma());

        assert_eq!(
            tones.as_list(),
            [
                0xff000000, 0xff00006e, 0xff0001ac, 0xff0000ef, 0xff343dff, 0xff5a64ff, 0xff7c84ff,
                0xff9da3ff, 0xffbec2ff, 0xffe0e0ff, 0xfff1efff, 0xfffffbff, 0xffffffff,
            ]
        );
    }

    #[test]
    fn of_constructor_operator_equals_and_hash_code() {
        let hct_a_b = Hct::from_int(0xff0000ff);
        let tones_a = TonalPalette::of(hct_a_b.hue(), hct_a_b.chroma());
        let tones_b = TonalPalette::of(hct_a_b.hue(), hct_a_b.chroma());
        let hct_c = Hct::from_int(0xff123456);
        let tones_c = TonalPalette::of(hct_c.hue(), hct_c.chroma());

        assert!(tones_a == tones_b);
        assert!(tones_b != tones_c);

        assert_eq!(hash_of(&tones_a), hash_of(&tones_b));
        assert_ne!(hash_of(&tones_b), hash_of(&tones_c));
    }

    struct FromListFixture {
        hue_chroma_palette: TonalPalette,
        cached_palette: Vec<u32>,
        broken_palette: Vec<u32>,
        rebuilt_palette: TonalPalette,
    }

    fn from_list_fixture() -> FromListFixture {
        let hue_chroma_palette = TonalPalette::of(270.0, 36.0);
        let cached_palette: Vec<u32> = TonalPalette::COMMON_TONES
            .iter()
            .map(|tone| hue_chroma_palette.get(*tone))
            .collect();
        let broken_palette = vec![
            cached_palette[0],
            cached_palette[1],
            Hct::from(180.0, 24.0, 20.0).to_int(),
            cached_palette[3],
            cached_palette[4],
            cached_palette[5],
            cached_palette[6],
            cached_palette[7],
            cached_palette[8],
            Hct::from(0.0, 12.0, 90.0).to_int(),
            cached_palette[10],
            cached_palette[11],
            cached_palette[12],
        ];

        let rebuilt_palette = TonalPalette::from_list(&broken_palette);
        FromListFixture {
            hue_chroma_palette,
            cached_palette,
            broken_palette,
            rebuilt_palette,
        }
    }

    #[test]
    fn from_list_constructor_correctly_deduces_original_hue_and_chroma() {
        let fixture = from_list_fixture();
        assert_close_to(fixture.rebuilt_palette.hue(), 270.0, 1.0);
        assert_close_to(fixture.rebuilt_palette.chroma(), 36.0, 1.0);
    }

    #[test]
    fn from_list_constructor_low_chroma_noise_does_not_affect_the_hue_and_chroma_deduced() {
        let fixture = from_list_fixture();
        let rebuilt_clean_palette = TonalPalette::from_list(&fixture.cached_palette);

        assert_eq!(fixture.rebuilt_palette.hue(), rebuilt_clean_palette.hue());
        assert_eq!(
            fixture.rebuilt_palette.chroma(),
            rebuilt_clean_palette.chroma()
        );
    }

    #[test]
    fn from_list_constructor_returns_cached_colors_when_possible() {
        let fixture = from_list_fixture();
        is_color(fixture.rebuilt_palette.get(20), fixture.broken_palette[2]);
        is_color(fixture.rebuilt_palette.get(50), fixture.broken_palette[5]);
        is_color(fixture.rebuilt_palette.get(90), fixture.broken_palette[9]);
        is_color(fixture.rebuilt_palette.get(99), fixture.broken_palette[11]);
    }

    #[test]
    fn from_list_constructor_correctly_deduces_colors_at_other_tones() {
        let fixture = from_list_fixture();
        is_close_to_color(
            fixture.rebuilt_palette.get(15),
            fixture.hue_chroma_palette.get(15),
        );
        is_close_to_color(
            fixture.rebuilt_palette.get(53),
            fixture.hue_chroma_palette.get(53),
        );
        is_close_to_color(
            fixture.rebuilt_palette.get(78),
            fixture.hue_chroma_palette.get(78),
        );
    }

    #[test]
    fn from_list_constructor_deduced_colors_have_correct_tone() {
        let fixture = from_list_fixture();
        assert_close_to(fixture.rebuilt_palette.get_hct(15.0).tone(), 15.0, 1.0);
        assert_close_to(fixture.rebuilt_palette.get_hct(53.0).tone(), 53.0, 1.0);
        assert_close_to(fixture.rebuilt_palette.get_hct(78.0).tone(), 78.0, 1.0);
    }

    #[test]
    fn from_list_constructor_as_list() {
        let ints: Vec<u32> = (0..TonalPalette::COMMON_SIZE as u32).collect();
        let tones = TonalPalette::from_list(&ints);
        assert_eq!(tones.as_list(), ints);
    }

    #[test]
    fn from_list_constructor_operator_equals_and_hash_code() {
        // This test confirms that `==` and `hashCode` behave the way they are
        // expected to behave. By defining five palettes, three from hue and
        // chroma, and two from lists, we expect their `hashCode` to be
        // distinct, and that their equality should satisfy the following grid:
        //
        // ==? 1   2   3   4   5
        // 1   YES -   -   YES -
        // 2   -   YES -   -   -
        // 3   -   -   YES -   -
        // 4   YES -   -   YES -
        // 5   -   -   -   -   YES

        let palette1 = TonalPalette::of(270.0, 36.0);
        let palette2 = TonalPalette::of(180.0, 36.0);
        let palette3 = TonalPalette::of(270.0, 12.0);

        let palette4 = TonalPalette::from_list(&palette1.as_list());
        let mut broken_list = palette1.as_list();
        broken_list[2] = Hct::from(180.0, 24.0, 20.0).to_int();
        broken_list[9] = Hct::from(0.0, 12.0, 90.0).to_int();
        let palette5 = TonalPalette::from_list(&broken_list);

        assert!(palette1 == palette1);
        assert!(palette1 != palette2);
        assert!(palette1 != palette3);
        assert!(palette1 == palette4);
        assert!(palette1 != palette5);

        assert!(palette2 != palette1);
        assert!(palette2 == palette2);
        assert!(palette2 != palette3);
        assert!(palette2 != palette4);
        assert!(palette2 != palette5);

        assert!(palette3 != palette1);
        assert!(palette3 != palette2);
        assert!(palette3 == palette3);
        assert!(palette3 != palette4);
        assert!(palette3 != palette5);

        assert!(palette4 == palette1);
        assert!(palette4 != palette2);
        assert!(palette4 != palette3);
        assert!(palette4 == palette4);
        assert!(palette4 != palette5);

        assert!(palette5 != palette1);
        assert!(palette5 != palette2);
        assert!(palette5 != palette3);
        assert!(palette5 != palette4);
        assert!(palette5 == palette5);

        // They should have five distinct hash codes (ignoring hash collision).
        let hash_codes: HashSet<u64> = [&palette1, &palette2, &palette3, &palette4, &palette5]
            .map(hash_of)
            .into_iter()
            .collect();
        assert_eq!(hash_codes.len(), 5);
    }

    #[test]
    fn key_color_exact_chroma_is_available() {
        // Requested chroma is exactly achievable at a certain tone.
        let palette = TonalPalette::of(50.0, 60.0);
        let result = palette.key_color();

        assert_close_to(result.hue(), 50.0, 10.0);
        assert_close_to(result.chroma(), 60.0, 0.5);
        // Tone might vary, but should be within the range from 0 to 100.
        assert!(result.tone() > 0.0);
        assert!(result.tone() < 100.0);
    }

    #[test]
    fn key_color_requesting_unusually_high_chroma() {
        // Requested chroma is above what is achievable. For Hue 149, chroma peak is 89.6 at Tone 87.9.
        // The result key color's chroma should be close to the chroma peak.
        let palette = TonalPalette::of(149.0, 200.0);
        let result = palette.key_color();

        assert_close_to(result.hue(), 149.0, 10.0);
        assert!(result.chroma() > 89.0);
        // Tone might vary, but should be within the range from 0 to 100.
        assert!(result.tone() > 0.0);
        assert!(result.tone() < 100.0);
    }

    #[test]
    fn key_color_requesting_unusually_low_chroma() {
        // By definition, the key color should be the first tone, starting from Tone 50, matching the
        // given hue and chroma. When requesting a very low chroma, the result should be close to Tone
        // 50, since most tones can produce a low chroma.
        let palette = TonalPalette::of(50.0, 3.0);
        let result = palette.key_color();

        // Higher error tolerance for hue when the requested chroma is unusually low.
        assert_close_to(result.hue(), 50.0, 10.0);
        assert_close_to(result.chroma(), 3.0, 0.5);
        assert_close_to(result.tone(), 50.0, 0.5);
    }
}
