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
// Ported from `lib/dynamiccolor/material_dynamic_colors.dart` of material_color_utilities 0.13.0.

use std::rc::Rc;

use crate::dart_math;
use crate::dislike::dislike_analyzer::DislikeAnalyzer;
use crate::dynamiccolor::dynamic_color::DynamicColor;
use crate::dynamiccolor::dynamic_scheme::DynamicScheme;
use crate::dynamiccolor::src::contrast_curve::ContrastCurve;
use crate::dynamiccolor::src::tone_delta_pair::{ToneDeltaPair, TonePolarity};
use crate::dynamiccolor::variant::Variant;
use crate::hct::hct::Hct;

fn is_fidelity(scheme: &DynamicScheme) -> bool {
    scheme.variant() == Variant::Fidelity || scheme.variant() == Variant::Content
}

fn is_monochrome(scheme: &DynamicScheme) -> bool {
    scheme.variant() == Variant::Monochrome
}

/// Dart's `static final DynamicColor`: made on its first read, then the same color, with
/// its cache, on every read. One per thread, as Dart's statics are one per isolate.
macro_rules! static_dynamic_color {
    ($color:expr) => {{
        thread_local! {
            static COLOR: Rc<DynamicColor> = Rc::new($color);
        }
        COLOR.with(Rc::clone)
    }};
}

/// Tokens, or named colors, in the Material Design system.
pub struct MaterialDynamicColors;

impl MaterialDynamicColors {
    pub const CONTENT_ACCENT_TONE_DELTA: f64 = 15.0;

    pub fn highest_surface(s: &DynamicScheme) -> Rc<DynamicColor> {
        if s.is_dark() {
            Self::surface_bright()
        } else {
            Self::surface_dim()
        }
    }

    pub fn primary_palette_key_color() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.primary_palette(),
                |s| s.primary_palette().key_color().tone(),
            )
            .name("primary_palette_key_color")
        )
    }

    pub fn secondary_palette_key_color() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.secondary_palette(),
                |s| s.secondary_palette().key_color().tone(),
            )
            .name("secondary_palette_key_color")
        )
    }

    pub fn tertiary_palette_key_color() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.tertiary_palette(),
                |s| s.tertiary_palette().key_color().tone(),
            )
            .name("tertiary_palette_key_color")
        )
    }

    pub fn neutral_palette_key_color() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_palette(),
                |s| s.neutral_palette().key_color().tone(),
            )
            .name("neutral_palette_key_color")
        )
    }

    pub fn neutral_variant_palette_key_color() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_variant_palette(),
                |s| s.neutral_variant_palette().key_color().tone(),
            )
            .name("neutral_variant_palette_key_color")
        )
    }

    pub fn background() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_palette(),
                |s| if s.is_dark() { 6.0 } else { 98.0 },
            )
            .name("background")
            .is_background(true)
        )
    }

    pub fn on_background() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_palette(),
                |s| if s.is_dark() { 90.0 } else { 10.0 },
            )
            .name("on_background")
            .background(|_| MaterialDynamicColors::background())
            .contrast_curve(ContrastCurve::new(3.0, 3.0, 4.5, 7.0))
        )
    }

    pub fn surface() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_palette(),
                |s| if s.is_dark() { 6.0 } else { 98.0 },
            )
            .name("surface")
            .is_background(true)
        )
    }

    pub fn surface_dim() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_palette(),
                |s| {
                    if s.is_dark() {
                        6.0
                    } else {
                        ContrastCurve::new(87.0, 87.0, 80.0, 75.0).get(s.contrast_level())
                    }
                },
            )
            .name("surface_dim")
            .is_background(true)
        )
    }

    pub fn surface_bright() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_palette(),
                |s| {
                    if s.is_dark() {
                        ContrastCurve::new(24.0, 24.0, 29.0, 34.0).get(s.contrast_level())
                    } else {
                        98.0
                    }
                },
            )
            .name("surface_bright")
            .is_background(true)
        )
    }

    pub fn surface_container_lowest() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_palette(),
                |s| {
                    if s.is_dark() {
                        ContrastCurve::new(4.0, 4.0, 2.0, 0.0).get(s.contrast_level())
                    } else {
                        100.0
                    }
                },
            )
            .name("surface_container_lowest")
            .is_background(true)
        )
    }

    pub fn surface_container_low() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_palette(),
                |s| {
                    if s.is_dark() {
                        ContrastCurve::new(10.0, 10.0, 11.0, 12.0).get(s.contrast_level())
                    } else {
                        ContrastCurve::new(96.0, 96.0, 96.0, 95.0).get(s.contrast_level())
                    }
                },
            )
            .name("surface_container_low")
            .is_background(true)
        )
    }

    pub fn surface_container() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_palette(),
                |s| {
                    if s.is_dark() {
                        ContrastCurve::new(12.0, 12.0, 16.0, 20.0).get(s.contrast_level())
                    } else {
                        ContrastCurve::new(94.0, 94.0, 92.0, 90.0).get(s.contrast_level())
                    }
                },
            )
            .name("surface_container")
            .is_background(true)
        )
    }

    pub fn surface_container_high() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_palette(),
                |s| {
                    if s.is_dark() {
                        ContrastCurve::new(17.0, 17.0, 21.0, 25.0).get(s.contrast_level())
                    } else {
                        ContrastCurve::new(92.0, 92.0, 88.0, 85.0).get(s.contrast_level())
                    }
                },
            )
            .name("surface_container_high")
            .is_background(true)
        )
    }

    pub fn surface_container_highest() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_palette(),
                |s| {
                    if s.is_dark() {
                        ContrastCurve::new(22.0, 22.0, 26.0, 30.0).get(s.contrast_level())
                    } else {
                        ContrastCurve::new(90.0, 90.0, 84.0, 80.0).get(s.contrast_level())
                    }
                },
            )
            .name("surface_container_highest")
            .is_background(true)
        )
    }

    pub fn on_surface() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_palette(),
                |s| if s.is_dark() { 90.0 } else { 10.0 },
            )
            .name("on_surface")
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(4.5, 7.0, 11.0, 21.0))
        )
    }

    pub fn surface_variant() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_variant_palette(),
                |s| if s.is_dark() { 30.0 } else { 90.0 },
            )
            .name("surface_variant")
            .is_background(true)
        )
    }

    pub fn on_surface_variant() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_variant_palette(),
                |s| if s.is_dark() { 80.0 } else { 30.0 },
            )
            .name("on_surface_variant")
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(3.0, 4.5, 7.0, 11.0))
        )
    }

    pub fn inverse_surface() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_palette(),
                |s| if s.is_dark() { 90.0 } else { 20.0 },
            )
            .name("inverse_surface")
        )
    }

    pub fn inverse_on_surface() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_palette(),
                |s| if s.is_dark() { 20.0 } else { 95.0 },
            )
            .name("inverse_on_surface")
            .background(|_| MaterialDynamicColors::inverse_surface())
            .contrast_curve(ContrastCurve::new(4.5, 7.0, 11.0, 21.0))
        )
    }

    pub fn outline() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_variant_palette(),
                |s| if s.is_dark() { 60.0 } else { 50.0 },
            )
            .name("outline")
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(1.5, 3.0, 4.5, 7.0))
        )
    }

    pub fn outline_variant() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.neutral_variant_palette(),
                |s| if s.is_dark() { 30.0 } else { 80.0 },
            )
            .name("outline_variant")
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(1.0, 1.0, 3.0, 4.5))
        )
    }

    pub fn shadow() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(|s| s.neutral_palette(), |_| 0.0).name("shadow")
        )
    }

    pub fn scrim() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(|s| s.neutral_palette(), |_| 0.0).name("scrim")
        )
    }

    pub fn surface_tint() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.primary_palette(),
                |s| if s.is_dark() { 80.0 } else { 40.0 },
            )
            .name("surface_tint")
            .is_background(true)
        )
    }

    pub fn primary() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.primary_palette(),
                |s| {
                    if is_monochrome(s) {
                        return if s.is_dark() { 100.0 } else { 0.0 };
                    }
                    if s.is_dark() { 80.0 } else { 40.0 }
                },
            )
            .name("primary")
            .is_background(true)
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(3.0, 4.5, 7.0, 7.0))
            .tone_delta_pair(|_| {
                ToneDeltaPair::new(
                    MaterialDynamicColors::primary_container(),
                    MaterialDynamicColors::primary(),
                    10.0,
                    TonePolarity::Nearer,
                    false,
                )
            })
        )
    }

    pub fn on_primary() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.primary_palette(),
                |s| {
                    if is_monochrome(s) {
                        return if s.is_dark() { 10.0 } else { 90.0 };
                    }
                    if s.is_dark() { 20.0 } else { 100.0 }
                },
            )
            .name("on_primary")
            .background(|_| MaterialDynamicColors::primary())
            .contrast_curve(ContrastCurve::new(4.5, 7.0, 11.0, 21.0))
        )
    }

    pub fn primary_container() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.primary_palette(),
                |s| {
                    if is_fidelity(s) {
                        return s.source_color_hct().tone();
                    }
                    if is_monochrome(s) {
                        return if s.is_dark() { 85.0 } else { 25.0 };
                    }
                    if s.is_dark() { 30.0 } else { 90.0 }
                },
            )
            .name("primary_container")
            .is_background(true)
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(1.0, 1.0, 3.0, 4.5))
            .tone_delta_pair(|_| {
                ToneDeltaPair::new(
                    MaterialDynamicColors::primary_container(),
                    MaterialDynamicColors::primary(),
                    10.0,
                    TonePolarity::Nearer,
                    false,
                )
            })
        )
    }

    pub fn on_primary_container() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.primary_palette(),
                |s| {
                    if is_fidelity(s) {
                        return DynamicColor::foreground_tone(
                            (MaterialDynamicColors::primary_container().tone)(s),
                            4.5,
                        );
                    }
                    if is_monochrome(s) {
                        return if s.is_dark() { 0.0 } else { 100.0 };
                    }
                    if s.is_dark() { 90.0 } else { 30.0 }
                },
            )
            .name("on_primary_container")
            .background(|_| MaterialDynamicColors::primary_container())
            .contrast_curve(ContrastCurve::new(3.0, 4.5, 7.0, 11.0))
        )
    }

    pub fn inverse_primary() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.primary_palette(),
                |s| if s.is_dark() { 40.0 } else { 80.0 },
            )
            .name("inverse_primary")
            .background(|_| MaterialDynamicColors::inverse_surface())
            .contrast_curve(ContrastCurve::new(3.0, 4.5, 7.0, 7.0))
        )
    }

    pub fn secondary() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.secondary_palette(),
                |s| if s.is_dark() { 80.0 } else { 40.0 },
            )
            .name("secondary")
            .is_background(true)
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(3.0, 4.5, 7.0, 7.0))
            .tone_delta_pair(|_| {
                ToneDeltaPair::new(
                    MaterialDynamicColors::secondary_container(),
                    MaterialDynamicColors::secondary(),
                    10.0,
                    TonePolarity::Nearer,
                    false,
                )
            })
        )
    }

    pub fn on_secondary() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.secondary_palette(),
                |s| {
                    if is_monochrome(s) {
                        if s.is_dark() { 10.0 } else { 100.0 }
                    } else if s.is_dark() {
                        20.0
                    } else {
                        100.0
                    }
                },
            )
            .name("on_secondary")
            .background(|_| MaterialDynamicColors::secondary())
            .contrast_curve(ContrastCurve::new(4.5, 7.0, 11.0, 21.0))
        )
    }

    pub fn secondary_container() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.secondary_palette(),
                |s| {
                    let initial_tone = if s.is_dark() { 30.0 } else { 90.0 };
                    if is_monochrome(s) {
                        return if s.is_dark() { 30.0 } else { 85.0 };
                    }
                    if !is_fidelity(s) {
                        return initial_tone;
                    }
                    MaterialDynamicColors::find_desired_chroma_by_tone(
                        s.secondary_palette().hue(),
                        s.secondary_palette().chroma(),
                        initial_tone,
                        !s.is_dark(),
                    )
                },
            )
            .name("secondary_container")
            .is_background(true)
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(1.0, 1.0, 3.0, 4.5))
            .tone_delta_pair(|_| {
                ToneDeltaPair::new(
                    MaterialDynamicColors::secondary_container(),
                    MaterialDynamicColors::secondary(),
                    10.0,
                    TonePolarity::Nearer,
                    false,
                )
            })
        )
    }

    pub fn on_secondary_container() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.secondary_palette(),
                |s| {
                    if is_monochrome(s) {
                        return if s.is_dark() { 90.0 } else { 10.0 };
                    }
                    if !is_fidelity(s) {
                        return if s.is_dark() { 90.0 } else { 30.0 };
                    }
                    DynamicColor::foreground_tone(
                        (MaterialDynamicColors::secondary_container().tone)(s),
                        4.5,
                    )
                },
            )
            .name("on_secondary_container")
            .background(|_| MaterialDynamicColors::secondary_container())
            .contrast_curve(ContrastCurve::new(3.0, 4.5, 7.0, 11.0))
        )
    }

    pub fn tertiary() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.tertiary_palette(),
                |s| {
                    if is_monochrome(s) {
                        return if s.is_dark() { 90.0 } else { 25.0 };
                    }
                    if s.is_dark() { 80.0 } else { 40.0 }
                },
            )
            .name("tertiary")
            .is_background(true)
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(3.0, 4.5, 7.0, 7.0))
            .tone_delta_pair(|_| {
                ToneDeltaPair::new(
                    MaterialDynamicColors::tertiary_container(),
                    MaterialDynamicColors::tertiary(),
                    10.0,
                    TonePolarity::Nearer,
                    false,
                )
            })
        )
    }

    pub fn on_tertiary() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.tertiary_palette(),
                |s| {
                    if is_monochrome(s) {
                        return if s.is_dark() { 10.0 } else { 90.0 };
                    }
                    if s.is_dark() { 20.0 } else { 100.0 }
                },
            )
            .name("on_tertiary")
            .background(|_| MaterialDynamicColors::tertiary())
            .contrast_curve(ContrastCurve::new(4.5, 7.0, 11.0, 21.0))
        )
    }

    pub fn tertiary_container() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.tertiary_palette(),
                |s| {
                    if is_monochrome(s) {
                        return if s.is_dark() { 60.0 } else { 49.0 };
                    }
                    if !is_fidelity(s) {
                        return if s.is_dark() { 30.0 } else { 90.0 };
                    }
                    let proposed_hct = s.tertiary_palette().get_hct(s.source_color_hct().tone());
                    DislikeAnalyzer::fix_if_disliked(proposed_hct).tone()
                },
            )
            .name("tertiary_container")
            .is_background(true)
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(1.0, 1.0, 3.0, 4.5))
            .tone_delta_pair(|_| {
                ToneDeltaPair::new(
                    MaterialDynamicColors::tertiary_container(),
                    MaterialDynamicColors::tertiary(),
                    10.0,
                    TonePolarity::Nearer,
                    false,
                )
            })
        )
    }

    pub fn on_tertiary_container() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.tertiary_palette(),
                |s| {
                    if is_monochrome(s) {
                        return if s.is_dark() { 0.0 } else { 100.0 };
                    }
                    if !is_fidelity(s) {
                        return if s.is_dark() { 90.0 } else { 30.0 };
                    }
                    DynamicColor::foreground_tone(
                        (MaterialDynamicColors::tertiary_container().tone)(s),
                        4.5,
                    )
                },
            )
            .name("on_tertiary_container")
            .background(|_| MaterialDynamicColors::tertiary_container())
            .contrast_curve(ContrastCurve::new(3.0, 4.5, 7.0, 11.0))
        )
    }

    pub fn error() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.error_palette(),
                |s| if s.is_dark() { 80.0 } else { 40.0 },
            )
            .name("error")
            .is_background(true)
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(3.0, 4.5, 7.0, 7.0))
            .tone_delta_pair(|_| {
                ToneDeltaPair::new(
                    MaterialDynamicColors::error_container(),
                    MaterialDynamicColors::error(),
                    10.0,
                    TonePolarity::Nearer,
                    false,
                )
            })
        )
    }

    pub fn on_error() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.error_palette(),
                |s| if s.is_dark() { 20.0 } else { 100.0 },
            )
            .name("on_error")
            .background(|_| MaterialDynamicColors::error())
            .contrast_curve(ContrastCurve::new(4.5, 7.0, 11.0, 21.0))
        )
    }

    pub fn error_container() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.error_palette(),
                |s| if s.is_dark() { 30.0 } else { 90.0 },
            )
            .name("error_container")
            .is_background(true)
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(1.0, 1.0, 3.0, 4.5))
            .tone_delta_pair(|_| {
                ToneDeltaPair::new(
                    MaterialDynamicColors::error_container(),
                    MaterialDynamicColors::error(),
                    10.0,
                    TonePolarity::Nearer,
                    false,
                )
            })
        )
    }

    pub fn on_error_container() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.error_palette(),
                |s| {
                    if is_monochrome(s) {
                        return if s.is_dark() { 90.0 } else { 10.0 };
                    }
                    if s.is_dark() { 90.0 } else { 30.0 }
                },
            )
            .name("on_error_container")
            .background(|_| MaterialDynamicColors::error_container())
            .contrast_curve(ContrastCurve::new(3.0, 4.5, 7.0, 11.0))
        )
    }

    pub fn primary_fixed() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.primary_palette(),
                |s| if is_monochrome(s) { 40.0 } else { 90.0 },
            )
            .name("primary_fixed")
            .is_background(true)
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(1.0, 1.0, 3.0, 4.5))
            .tone_delta_pair(|_| {
                ToneDeltaPair::new(
                    MaterialDynamicColors::primary_fixed(),
                    MaterialDynamicColors::primary_fixed_dim(),
                    10.0,
                    TonePolarity::Lighter,
                    true,
                )
            })
        )
    }

    pub fn primary_fixed_dim() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.primary_palette(),
                |s| if is_monochrome(s) { 30.0 } else { 80.0 },
            )
            .name("primary_fixed_dim")
            .is_background(true)
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(1.0, 1.0, 3.0, 4.5))
            .tone_delta_pair(|_| {
                ToneDeltaPair::new(
                    MaterialDynamicColors::primary_fixed(),
                    MaterialDynamicColors::primary_fixed_dim(),
                    10.0,
                    TonePolarity::Lighter,
                    true,
                )
            })
        )
    }

    pub fn on_primary_fixed() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.primary_palette(),
                |s| if is_monochrome(s) { 100.0 } else { 10.0 },
            )
            .name("on_primary_fixed")
            .background(|_| MaterialDynamicColors::primary_fixed_dim())
            .second_background(|_| MaterialDynamicColors::primary_fixed())
            .contrast_curve(ContrastCurve::new(4.5, 7.0, 11.0, 21.0))
        )
    }

    pub fn on_primary_fixed_variant() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.primary_palette(),
                |s| if is_monochrome(s) { 90.0 } else { 30.0 },
            )
            .name("on_primary_fixed_variant")
            .background(|_| MaterialDynamicColors::primary_fixed_dim())
            .second_background(|_| MaterialDynamicColors::primary_fixed())
            .contrast_curve(ContrastCurve::new(3.0, 4.5, 7.0, 11.0))
        )
    }

    pub fn secondary_fixed() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.secondary_palette(),
                |s| if is_monochrome(s) { 80.0 } else { 90.0 },
            )
            .name("secondary_fixed")
            .is_background(true)
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(1.0, 1.0, 3.0, 4.5))
            .tone_delta_pair(|_| {
                ToneDeltaPair::new(
                    MaterialDynamicColors::secondary_fixed(),
                    MaterialDynamicColors::secondary_fixed_dim(),
                    10.0,
                    TonePolarity::Lighter,
                    true,
                )
            })
        )
    }

    pub fn secondary_fixed_dim() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.secondary_palette(),
                |s| if is_monochrome(s) { 70.0 } else { 80.0 },
            )
            .name("secondary_fixed_dim")
            .is_background(true)
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(1.0, 1.0, 3.0, 4.5))
            .tone_delta_pair(|_| {
                ToneDeltaPair::new(
                    MaterialDynamicColors::secondary_fixed(),
                    MaterialDynamicColors::secondary_fixed_dim(),
                    10.0,
                    TonePolarity::Lighter,
                    true,
                )
            })
        )
    }

    pub fn on_secondary_fixed() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(|s| s.secondary_palette(), |_| 10.0)
                .name("on_secondary_fixed")
                .background(|_| MaterialDynamicColors::secondary_fixed_dim())
                .second_background(|_| MaterialDynamicColors::secondary_fixed())
                .contrast_curve(ContrastCurve::new(4.5, 7.0, 11.0, 21.0))
        )
    }

    pub fn on_secondary_fixed_variant() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.secondary_palette(),
                |s| if is_monochrome(s) { 25.0 } else { 30.0 },
            )
            .name("on_secondary_fixed_variant")
            .background(|_| MaterialDynamicColors::secondary_fixed_dim())
            .second_background(|_| MaterialDynamicColors::secondary_fixed())
            .contrast_curve(ContrastCurve::new(3.0, 4.5, 7.0, 11.0))
        )
    }

    pub fn tertiary_fixed() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.tertiary_palette(),
                |s| if is_monochrome(s) { 40.0 } else { 90.0 },
            )
            .name("tertiary_fixed")
            .is_background(true)
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(1.0, 1.0, 3.0, 4.5))
            .tone_delta_pair(|_| {
                ToneDeltaPair::new(
                    MaterialDynamicColors::tertiary_fixed(),
                    MaterialDynamicColors::tertiary_fixed_dim(),
                    10.0,
                    TonePolarity::Lighter,
                    true,
                )
            })
        )
    }

    pub fn tertiary_fixed_dim() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.tertiary_palette(),
                |s| if is_monochrome(s) { 30.0 } else { 80.0 },
            )
            .name("tertiary_fixed_dim")
            .is_background(true)
            .background(MaterialDynamicColors::highest_surface)
            .contrast_curve(ContrastCurve::new(1.0, 1.0, 3.0, 4.5))
            .tone_delta_pair(|_| {
                ToneDeltaPair::new(
                    MaterialDynamicColors::tertiary_fixed(),
                    MaterialDynamicColors::tertiary_fixed_dim(),
                    10.0,
                    TonePolarity::Lighter,
                    true,
                )
            })
        )
    }

    pub fn on_tertiary_fixed() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.tertiary_palette(),
                |s| if is_monochrome(s) { 100.0 } else { 10.0 },
            )
            .name("on_tertiary_fixed")
            .background(|_| MaterialDynamicColors::tertiary_fixed_dim())
            .second_background(|_| MaterialDynamicColors::tertiary_fixed())
            .contrast_curve(ContrastCurve::new(4.5, 7.0, 11.0, 21.0))
        )
    }

    pub fn on_tertiary_fixed_variant() -> Rc<DynamicColor> {
        static_dynamic_color!(
            DynamicColor::from_palette(
                |s| s.tertiary_palette(),
                |s| if is_monochrome(s) { 90.0 } else { 30.0 },
            )
            .name("on_tertiary_fixed_variant")
            .background(|_| MaterialDynamicColors::tertiary_fixed_dim())
            .second_background(|_| MaterialDynamicColors::tertiary_fixed())
            .contrast_curve(ContrastCurve::new(3.0, 4.5, 7.0, 11.0))
        )
    }

    fn find_desired_chroma_by_tone(
        hue: f64,
        chroma: f64,
        tone: f64,
        by_decreasing_tone: bool,
    ) -> f64 {
        let mut answer = tone;

        let mut closest_to_chroma = Hct::from(hue, chroma, tone);
        if closest_to_chroma.chroma() < chroma {
            let mut chroma_peak = closest_to_chroma.chroma();
            while closest_to_chroma.chroma() < chroma {
                answer += if by_decreasing_tone { -1.0 } else { 1.0 };
                let potential_solution = Hct::from(hue, chroma, answer);
                if chroma_peak > potential_solution.chroma() {
                    break;
                }
                if (potential_solution.chroma() - chroma).abs() < 0.4 {
                    break;
                }

                let potential_delta = (potential_solution.chroma() - chroma).abs();
                let current_delta = (closest_to_chroma.chroma() - chroma).abs();
                if potential_delta < current_delta {
                    closest_to_chroma = potential_solution;
                }
                chroma_peak = dart_math::max(chroma_peak, potential_solution.chroma());
            }
        }

        answer
    }
}
