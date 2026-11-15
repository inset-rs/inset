// Copyright 2022 Google LLC
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
// Ported from `lib/dynamiccolor/dynamic_color.dart` of material_color_utilities 0.13.0.

use std::cell::RefCell;
use std::rc::Rc;

use indexmap::IndexMap;

use crate::contrast::contrast::Contrast;
use crate::dart_math;
use crate::dynamiccolor::dynamic_scheme::{DynamicScheme, DynamicSchemeId};
use crate::dynamiccolor::src::contrast_curve::ContrastCurve;
use crate::dynamiccolor::src::tone_delta_pair::{ToneDeltaPair, TonePolarity};
use crate::hct::hct::Hct;
use crate::palettes::tonal_palette::TonalPalette;
use crate::utils::math_utils::MathUtils;

/// Dart's `TonalPalette Function(DynamicScheme)`.
pub type DynamicSchemePalette = Rc<dyn Fn(&DynamicScheme) -> &TonalPalette>;
/// Dart's `double Function(DynamicScheme)`.
pub type DynamicSchemeTone = Rc<dyn Fn(&DynamicScheme) -> f64>;
/// Dart's `DynamicColor Function(DynamicScheme)`.
pub type DynamicSchemeColor = Rc<dyn Fn(&DynamicScheme) -> Rc<DynamicColor>>;
/// Dart's `ToneDeltaPair Function(DynamicScheme)`.
pub type DynamicSchemeToneDeltaPair = Rc<dyn Fn(&DynamicScheme) -> ToneDeltaPair>;

/// A color that adjusts itself based on UI state provided by [`DynamicScheme`].
///
/// This color automatically adjusts to accommodate a desired contrast level, or
/// other adjustments such as differing in light mode versus dark mode, or what
/// the theme is, or what the color that produced the theme is, etc.
///
/// Colors without backgrounds do not change tone when contrast changes. Colors
/// with backgrounds become closer to their background as contrast lowers, and
/// further when contrast increases.
///
/// Prefer the static constructors. They provide a much more simple interface,
/// such as requiring just a hexcode, or just a hexcode and a background.
///
/// Ultimately, each component necessary for calculating a color, adjusting it
/// for a desired contrast level, and ensuring it has a certain lightness/tone
/// difference from another color, is provided by a function that takes a
/// [`DynamicScheme`] and returns a value. This ensures ultimate flexibility, any
/// desired behavior of a color for any design system, but it usually
/// unnecessary. See the default constructor for more information.
///
/// The fields are Dart's final fields: a color is shared as an `Rc<DynamicColor>` once
/// made, and its cache of answers per scheme assumes they no longer change.
pub struct DynamicColor {
    pub name: String,
    pub palette: DynamicSchemePalette,
    pub tone: DynamicSchemeTone,
    pub is_background: bool,
    pub background: Option<DynamicSchemeColor>,
    pub second_background: Option<DynamicSchemeColor>,
    pub contrast_curve: Option<ContrastCurve>,
    pub tone_delta_pair: Option<DynamicSchemeToneDeltaPair>,

    /// Keyed by the scheme's identity, as Dart's map keyed by the scheme object is.
    hct_cache: RefCell<IndexMap<DynamicSchemeId, Hct>>,
}

impl DynamicColor {
    /// The base (explicit) constructor for [`DynamicColor`].
    ///
    /// `name` The name of the dynamic color.
    /// `palette` Function that provides a TonalPalette given
    /// DynamicScheme. A TonalPalette is defined by a hue and chroma, so this
    /// replaces the need to specify hue/chroma. By providing a tonal palette,
    /// when contrast adjustments are made, intended chroma can be preserved.
    /// `tone` Function that provides a tone, given a DynamicScheme.
    /// `is_background` Whether this dynamic color is a background, with
    /// some other color as the foreground.
    /// `background` The background of the dynamic color (as a function of a
    /// `DynamicScheme`), if it exists.
    /// `second_background` A second background of the dynamic color (as a function
    /// of a `DynamicScheme`), if it
    /// exists.
    /// `contrast_curve` A [`ContrastCurve`] object specifying how its contrast
    /// against its background should behave in various contrast levels options.
    /// `tone_delta_pair` A [`ToneDeltaPair`] object specifying a tone delta
    /// constraint between two colors. One of them must be the color being
    /// constructed.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        palette: DynamicSchemePalette,
        tone: DynamicSchemeTone,
        is_background: bool,
        background: Option<DynamicSchemeColor>,
        second_background: Option<DynamicSchemeColor>,
        contrast_curve: Option<ContrastCurve>,
        tone_delta_pair: Option<DynamicSchemeToneDeltaPair>,
    ) -> DynamicColor {
        DynamicColor {
            name: name.into(),
            palette,
            tone,
            is_background,
            background,
            second_background,
            contrast_curve,
            tone_delta_pair,
            hct_cache: RefCell::new(IndexMap::new()),
        }
    }

    /// The convenience constructor for [`DynamicColor`].
    ///
    /// Similar to the base constructor, but all parameters other than `palette`
    /// and `tone` have defaults: the setters below take them, by their Dart names.
    ///
    /// `name` The name of the dynamic color. Defaults to empty.
    /// `palette` Function that provides a TonalPalette given
    /// DynamicScheme. A TonalPalette is defined by a hue and chroma, so this
    /// replaces the need to specify hue/chroma. By providing a tonal palette,
    /// when contrast adjustments are made, intended chroma can be preserved.
    /// `tone` Function that provides a tone, given a DynamicScheme.
    /// `is_background` Whether this dynamic color is a background, with
    /// some other color as the foreground. Defaults to false.
    /// `background` The background of the dynamic color (as a function of a
    /// `DynamicScheme`), if it exists.
    /// `second_background` A second background of the dynamic color (as a function
    /// of a `DynamicScheme`), if it exists.
    /// `contrast_curve` A [`ContrastCurve`] object specifying how its contrast
    /// against its background should behave in various contrast levels options.
    /// `tone_delta_pair` A [`ToneDeltaPair`] object specifying a tone delta
    /// constraint between two colors. One of them must be the color being
    /// constructed.
    pub fn from_palette(
        palette: impl Fn(&DynamicScheme) -> &TonalPalette + 'static,
        tone: impl Fn(&DynamicScheme) -> f64 + 'static,
    ) -> DynamicColor {
        DynamicColor::new(
            "",
            Rc::new(palette),
            Rc::new(tone),
            false,
            None,
            None,
            None,
            None,
        )
    }

    /// Dart `DynamicColor.fromPalette(name:)`.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Dart `DynamicColor.fromPalette(isBackground:)`.
    pub fn is_background(mut self, is_background: bool) -> Self {
        self.is_background = is_background;
        self
    }

    /// Dart `DynamicColor.fromPalette(background:)`.
    pub fn background(
        mut self,
        background: impl Fn(&DynamicScheme) -> Rc<DynamicColor> + 'static,
    ) -> Self {
        self.background = Some(Rc::new(background));
        self
    }

    /// Dart `DynamicColor.fromPalette(secondBackground:)`.
    pub fn second_background(
        mut self,
        second_background: impl Fn(&DynamicScheme) -> Rc<DynamicColor> + 'static,
    ) -> Self {
        self.second_background = Some(Rc::new(second_background));
        self
    }

    /// Dart `DynamicColor.fromPalette(contrastCurve:)`.
    pub fn contrast_curve(mut self, contrast_curve: ContrastCurve) -> Self {
        self.contrast_curve = Some(contrast_curve);
        self
    }

    /// Dart `DynamicColor.fromPalette(toneDeltaPair:)`.
    pub fn tone_delta_pair(
        mut self,
        tone_delta_pair: impl Fn(&DynamicScheme) -> ToneDeltaPair + 'static,
    ) -> Self {
        self.tone_delta_pair = Some(Rc::new(tone_delta_pair));
        self
    }

    /// Return a ARGB integer (i.e. a hex code).
    ///
    /// `scheme` Defines the conditions of the user interface, for example,
    /// whether or not it is dark mode or light mode, and what the desired
    /// contrast level is.
    pub fn get_argb(&self, scheme: &DynamicScheme) -> u32 {
        self.get_hct(scheme).to_int()
    }

    /// Return a color, expressed in the HCT color space, that this
    /// [`DynamicColor`] is under the conditions in `scheme`.
    ///
    /// `scheme` Defines the conditions of the user interface, for example,
    /// whether or not it is dark mode or light mode, and what the desired
    /// contrast level is.
    pub fn get_hct(&self, scheme: &DynamicScheme) -> Hct {
        let cached_answer = self.hct_cache.borrow().get(&scheme.id()).copied();
        if let Some(cached_answer) = cached_answer {
            return cached_answer;
        }
        let tone = self.get_tone(scheme);
        let answer = (self.palette)(scheme).get_hct(tone);
        let mut hct_cache = self.hct_cache.borrow_mut();
        if hct_cache.len() > 4 {
            hct_cache.clear();
        }
        hct_cache.insert(scheme.id(), answer);
        answer
    }

    /// Return a tone, T in the HCT color space, that this [`DynamicColor`] is under
    /// the conditions in `scheme`.
    ///
    /// `scheme` Defines the conditions of the user interface, for example,
    /// whether or not it is dark mode or light mode, and what the desired
    /// contrast level is.
    pub fn get_tone(&self, scheme: &DynamicScheme) -> f64 {
        let decreasing_contrast = scheme.contrast_level() < 0.0;

        // Case 1: dual foreground, pair of colors with delta constraint.
        if let Some(tone_delta_pair) = &self.tone_delta_pair {
            let pair = tone_delta_pair(scheme);
            let role_a = pair.role_a;
            let role_b = pair.role_b;
            let delta = pair.delta;
            let polarity = pair.polarity;
            let stay_together = pair.stay_together;

            let bg = (self.background.as_ref().unwrap())(scheme);
            let bg_tone = bg.get_tone(scheme);

            let a_is_nearer = polarity == TonePolarity::Nearer
                || (polarity == TonePolarity::Lighter && !scheme.is_dark())
                || (polarity == TonePolarity::Darker && scheme.is_dark());
            let nearer = if a_is_nearer { &role_a } else { &role_b };
            let farther = if a_is_nearer { &role_b } else { &role_a };
            let am_nearer = self.name == nearer.name;
            let expansion_dir = if scheme.is_dark() { 1.0 } else { -1.0 };

            // 1st round: solve to min, each
            let n_contrast = nearer.contrast_curve.unwrap().get(scheme.contrast_level());
            let f_contrast = farther.contrast_curve.unwrap().get(scheme.contrast_level());

            // If a color is good enough, it is not adjusted.
            // Initial and adjusted tones for `nearer`
            let n_initial_tone = (nearer.tone)(scheme);
            let mut n_tone = if Contrast::ratio_of_tones(bg_tone, n_initial_tone) >= n_contrast {
                n_initial_tone
            } else {
                DynamicColor::foreground_tone(bg_tone, n_contrast)
            };
            // Initial and adjusted tones for `farther`
            let f_initial_tone = (farther.tone)(scheme);
            let mut f_tone = if Contrast::ratio_of_tones(bg_tone, f_initial_tone) >= f_contrast {
                f_initial_tone
            } else {
                DynamicColor::foreground_tone(bg_tone, f_contrast)
            };

            if decreasing_contrast {
                // If decreasing contrast, adjust color to the "bare minimum"
                // that satisfies contrast.
                n_tone = DynamicColor::foreground_tone(bg_tone, n_contrast);
                f_tone = DynamicColor::foreground_tone(bg_tone, f_contrast);
            }

            if (f_tone - n_tone) * expansion_dir >= delta {
                // Good! Tones satisfy the constraint; no change needed.
            } else {
                // 2nd round: expand farther to match delta.
                f_tone = MathUtils::clamp_double(0.0, 100.0, n_tone + delta * expansion_dir);
                if (f_tone - n_tone) * expansion_dir >= delta {
                    // Good! Tones now satisfy the constraint; no change needed.
                } else {
                    // 3rd round: contract nearer to match delta.
                    n_tone = MathUtils::clamp_double(0.0, 100.0, f_tone - delta * expansion_dir);
                }
            }

            // Avoids the 50-59 awkward zone.
            if (50.0..60.0).contains(&n_tone) {
                // If `nearer` is in the awkward zone, move it away, together with
                // `farther`.
                if expansion_dir > 0.0 {
                    n_tone = 60.0;
                    f_tone = dart_math::max(f_tone, n_tone + delta * expansion_dir);
                } else {
                    n_tone = 49.0;
                    f_tone = dart_math::min(f_tone, n_tone + delta * expansion_dir);
                }
            } else if (50.0..60.0).contains(&f_tone) {
                if stay_together {
                    // Fixes both, to avoid two colors on opposite sides of the "awkward
                    // zone".
                    if expansion_dir > 0.0 {
                        n_tone = 60.0;
                        f_tone = dart_math::max(f_tone, n_tone + delta * expansion_dir);
                    } else {
                        n_tone = 49.0;
                        f_tone = dart_math::min(f_tone, n_tone + delta * expansion_dir);
                    }
                } else {
                    // Not required to stay together; fixes just one.
                    if expansion_dir > 0.0 {
                        f_tone = 60.0;
                    } else {
                        f_tone = 49.0;
                    }
                }
            }

            // Returns `nTone` if this color is `nearer`, otherwise `fTone`.
            if am_nearer { n_tone } else { f_tone }
        } else {
            // Case 2: No contrast pair; just solve for itself.
            let mut answer = (self.tone)(scheme);

            let Some(background) = &self.background else {
                return answer; // No adjustment for colors with no background.
            };

            let bg_tone = background(scheme).get_tone(scheme);

            let desired_ratio = self.contrast_curve.unwrap().get(scheme.contrast_level());

            if Contrast::ratio_of_tones(bg_tone, answer) >= desired_ratio {
                // Don't "improve" what's good enough.
            } else {
                // Rough improvement.
                answer = DynamicColor::foreground_tone(bg_tone, desired_ratio);
            }

            if decreasing_contrast {
                answer = DynamicColor::foreground_tone(bg_tone, desired_ratio);
            }

            if self.is_background && (50.0..60.0).contains(&answer) {
                // Must adjust
                if Contrast::ratio_of_tones(49.0, bg_tone) >= desired_ratio {
                    answer = 49.0;
                } else {
                    answer = 60.0;
                }
            }

            if let Some(second_background) = &self.second_background {
                // Case 3: Adjust for dual backgrounds.

                let bg_tone1 = background(scheme).get_tone(scheme);
                let bg_tone2 = second_background(scheme).get_tone(scheme);

                let upper = dart_math::max(bg_tone1, bg_tone2);
                let lower = dart_math::min(bg_tone1, bg_tone2);

                if Contrast::ratio_of_tones(upper, answer) >= desired_ratio
                    && Contrast::ratio_of_tones(lower, answer) >= desired_ratio
                {
                    return answer;
                }

                // The darkest light tone that satisfies the desired ratio,
                // or -1 if such ratio cannot be reached.
                let light_option = Contrast::lighter(upper, desired_ratio);

                // The lightest dark tone that satisfies the desired ratio,
                // or -1 if such ratio cannot be reached.
                let dark_option = Contrast::darker(lower, desired_ratio);

                // Tones suitable for the foreground.
                let mut availables: Vec<f64> = Vec::new();
                if light_option != -1.0 {
                    availables.push(light_option);
                }
                if dark_option != -1.0 {
                    availables.push(dark_option);
                }

                let prefers_light = DynamicColor::tone_prefers_light_foreground(bg_tone1)
                    || DynamicColor::tone_prefers_light_foreground(bg_tone2);
                if prefers_light {
                    return if light_option < 0.0 {
                        100.0
                    } else {
                        light_option
                    };
                }
                if availables.len() == 1 {
                    return availables[0];
                }
                return if dark_option < 0.0 { 0.0 } else { dark_option };
            }

            answer
        }
    }

    /// Given a background tone, find a foreground tone, while ensuring they reach
    /// a contrast ratio that is as close to `ratio` as possible.
    ///
    /// `bg_tone` Tone in HCT. Range is 0 to 100, undefined behavior when it falls
    /// outside that range.
    /// `ratio` The contrast ratio desired between `bg_tone` and the return value.
    pub fn foreground_tone(bg_tone: f64, ratio: f64) -> f64 {
        let lighter_tone = Contrast::lighter_unsafe(bg_tone, ratio);
        let darker_tone = Contrast::darker_unsafe(bg_tone, ratio);
        let lighter_ratio = Contrast::ratio_of_tones(lighter_tone, bg_tone);
        let darker_ratio = Contrast::ratio_of_tones(darker_tone, bg_tone);
        let prefer_lighter = Self::tone_prefers_light_foreground(bg_tone);

        if prefer_lighter {
            // This handles an edge case where the initial contrast ratio is high
            // (ex. 13.0), and the ratio passed to the function is that high ratio,
            // and both the lighter and darker ratio fails to pass that ratio.
            //
            // This was observed with Tonal Spot's On Primary Container turning black
            // momentarily between high and max contrast in light mode.
            // PC's standard tone was T90, OPC's was T10, it was light mode, and the
            // contrast value was 0.6568521221032331.
            let negligible_difference = (lighter_ratio - darker_ratio).abs() < 0.1
                && lighter_ratio < ratio
                && darker_ratio < ratio;
            if lighter_ratio >= ratio || lighter_ratio >= darker_ratio || negligible_difference {
                lighter_tone
            } else {
                darker_tone
            }
        } else if darker_ratio >= ratio || darker_ratio >= lighter_ratio {
            darker_tone
        } else {
            lighter_tone
        }
    }

    /// Adjust a tone such that white has 4.5 contrast, if the tone is
    /// reasonably close to supporting it.
    pub fn enable_light_foreground(tone: f64) -> f64 {
        if Self::tone_prefers_light_foreground(tone) && !Self::tone_allows_light_foreground(tone) {
            return 49.0;
        }
        tone
    }

    /// Returns whether `tone` prefers a light foreground.
    ///
    /// People prefer white foregrounds on ~T60-70. Observed over time, and also
    /// by Andrew Somers during research for APCA.
    ///
    /// T60 used as to create the smallest discontinuity possible when skipping
    /// down to T49 in order to ensure light foregrounds.
    ///
    /// Since `tertiaryContainer` in dark monochrome scheme requires a tone of
    /// 60, it should not be adjusted. Therefore, 60 is excluded here.
    pub fn tone_prefers_light_foreground(tone: f64) -> bool {
        dart_math::round(tone) < 60
    }

    /// Returns whether `tone` can reach a contrast ratio of 4.5 with a lighter
    /// color.
    pub fn tone_allows_light_foreground(tone: f64) -> bool {
        dart_math::round(tone) <= 49
    }
}
