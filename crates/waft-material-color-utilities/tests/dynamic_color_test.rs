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
// Ported from `test/dynamic_color_test.dart` of material_color_utilities 0.13.0.

use std::rc::Rc;

use waft_material_color_utilities::{
    Contrast, DynamicColor, DynamicScheme, Hct, MaterialDynamicColors, SchemeContent,
    SchemeFidelity, SchemeMonochrome, SchemeTonalSpot,
};

fn seed_colors() -> [Hct; 4] {
    [
        Hct::from_int(0xFFFF0000),
        Hct::from_int(0xFFFFFF00),
        Hct::from_int(0xFF00FF00),
        Hct::from_int(0xFF0000FF),
    ]
}

const CONTRAST_LEVELS: [f64; 5] = [-1.0, -0.5, 0.0, 0.5, 1.0];

struct Pair {
    fg_name: &'static str,
    bg_name: &'static str,
}

fn pair(fg_name: &'static str, bg_name: &'static str) -> Pair {
    Pair { fg_name, bg_name }
}

fn colors() -> Vec<(&'static str, Rc<DynamicColor>)> {
    vec![
        ("background", MaterialDynamicColors::background()),
        ("on_background", MaterialDynamicColors::on_background()),
        ("surface", MaterialDynamicColors::surface()),
        ("surface_dim", MaterialDynamicColors::surface_dim()),
        ("surface_bright", MaterialDynamicColors::surface_bright()),
        (
            "surface_container_lowest",
            MaterialDynamicColors::surface_container_lowest(),
        ),
        (
            "surface_container_low",
            MaterialDynamicColors::surface_container_low(),
        ),
        (
            "surface_container",
            MaterialDynamicColors::surface_container(),
        ),
        (
            "surface_container_high",
            MaterialDynamicColors::surface_container_high(),
        ),
        (
            "surface_container_highest",
            MaterialDynamicColors::surface_container_highest(),
        ),
        ("on_surface", MaterialDynamicColors::on_surface()),
        ("surface_variant", MaterialDynamicColors::surface_variant()),
        (
            "on_surface_variant",
            MaterialDynamicColors::on_surface_variant(),
        ),
        ("inverse_surface", MaterialDynamicColors::inverse_surface()),
        (
            "inverse_on_surface",
            MaterialDynamicColors::inverse_on_surface(),
        ),
        ("outline", MaterialDynamicColors::outline()),
        ("outline_variant", MaterialDynamicColors::outline_variant()),
        ("shadow", MaterialDynamicColors::shadow()),
        ("scrim", MaterialDynamicColors::scrim()),
        ("surface_tint", MaterialDynamicColors::surface_tint()),
        ("primary", MaterialDynamicColors::primary()),
        ("on_primary", MaterialDynamicColors::on_primary()),
        (
            "primary_container",
            MaterialDynamicColors::primary_container(),
        ),
        (
            "on_primary_container",
            MaterialDynamicColors::on_primary_container(),
        ),
        ("inverse_primary", MaterialDynamicColors::inverse_primary()),
        ("secondary", MaterialDynamicColors::secondary()),
        ("on_secondary", MaterialDynamicColors::on_secondary()),
        (
            "secondary_container",
            MaterialDynamicColors::secondary_container(),
        ),
        (
            "on_secondary_container",
            MaterialDynamicColors::on_secondary_container(),
        ),
        ("tertiary", MaterialDynamicColors::tertiary()),
        ("on_tertiary", MaterialDynamicColors::on_tertiary()),
        (
            "tertiary_container",
            MaterialDynamicColors::tertiary_container(),
        ),
        (
            "on_tertiary_container",
            MaterialDynamicColors::on_tertiary_container(),
        ),
        ("error", MaterialDynamicColors::error()),
        ("on_error", MaterialDynamicColors::on_error()),
        ("error_container", MaterialDynamicColors::error_container()),
        (
            "on_error_container",
            MaterialDynamicColors::on_error_container(),
        ),
    ]
}

fn text_surface_pairs() -> [Pair; 12] {
    [
        pair("on_primary", "primary"),
        pair("on_primary_container", "primary_container"),
        pair("on_secondary", "secondary"),
        pair("on_secondary_container", "secondary_container"),
        pair("on_tertiary", "tertiary"),
        pair("on_tertiary_container", "tertiary_container"),
        pair("on_error", "error"),
        pair("on_error_container", "error_container"),
        pair("on_background", "background"),
        pair("on_surface_variant", "surface_bright"),
        pair("on_surface_variant", "surface_dim"),
        pair("inverse_on_surface", "inverse_surface"),
    ]
}

fn assert_close_to(actual: f64, expected: f64, delta: f64) {
    assert!(
        (actual - expected).abs() <= delta,
        "expected {actual} to be within {delta} of {expected}"
    );
}

#[test]
fn values_are_correct() {
    assert_eq!(
        MaterialDynamicColors::on_primary_container().get_argb(&SchemeFidelity::new(
            Hct::from_int(0xFFFF0000),
            false,
            0.5
        )),
        0xFFFFFFFF
    );
    assert_eq!(
        MaterialDynamicColors::on_secondary_container().get_argb(&SchemeContent::new(
            Hct::from_int(0xFF0000FF),
            false,
            0.5
        )),
        0xFFFFFFFF
    );
    assert_eq!(
        MaterialDynamicColors::on_tertiary_container().get_argb(&SchemeContent::new(
            Hct::from_int(0xFFFFFF00),
            true,
            -0.5
        )),
        0xff959b1a
    );
    assert_eq!(
        MaterialDynamicColors::inverse_surface().get_argb(&SchemeContent::new(
            Hct::from_int(0xFF0000FF),
            false,
            0.0
        )),
        0xFF2F2F3B
    );
    assert_eq!(
        MaterialDynamicColors::inverse_primary().get_argb(&SchemeContent::new(
            Hct::from_int(0xFFFF0000),
            false,
            -0.5
        )),
        0xffff422f
    );
    assert_eq!(
        MaterialDynamicColors::outline_variant().get_argb(&SchemeContent::new(
            Hct::from_int(0xFFFFFF00),
            true,
            0.0
        )),
        0xFF484831
    );
}

// Parametric test, ensuring that dynamic schemes respect contrast
// between text-surface pairs.
#[test]
fn schemes_respect_contrast_between_text_surface_pairs() {
    let colors = colors();
    let color_named = |name: &str| colors.iter().find(|(n, _)| *n == name).unwrap().1.clone();
    for color in seed_colors() {
        for contrast_level in CONTRAST_LEVELS {
            for is_dark in [false, true] {
                let schemes: [(&str, DynamicScheme); 4] = [
                    (
                        "SchemeContent",
                        SchemeContent::new(color, is_dark, contrast_level),
                    ),
                    (
                        "SchemeMonochrome",
                        SchemeMonochrome::new(color, is_dark, contrast_level),
                    ),
                    (
                        "SchemeTonalSpot",
                        SchemeTonalSpot::new(color, is_dark, contrast_level),
                    ),
                    (
                        "SchemeFidelity",
                        SchemeFidelity::new(color, is_dark, contrast_level),
                    ),
                ];
                for (scheme_name, scheme) in &schemes {
                    for pair in text_surface_pairs() {
                        // Expect that each text-surface pair has a
                        // minimum contrast of 4.5 (unreduced contrast), or 3.0
                        // (reduced contrast).
                        let fg_name = pair.fg_name;
                        let bg_name = pair.bg_name;
                        let foreground_tone = color_named(fg_name).get_hct(scheme).tone();
                        let background_tone = color_named(bg_name).get_hct(scheme).tone();
                        let contrast = Contrast::ratio_of_tones(foreground_tone, background_tone);

                        let minimum_requirement = if contrast_level >= 0.0 { 4.5 } else { 3.0 };

                        assert!(
                            contrast >= minimum_requirement,
                            "Scheme: {scheme_name}; Seed color: {color:?}; Contrast level: {contrast_level}; Dark: {is_dark}: Contrast {contrast} is too low between foreground ({fg_name}; {foreground_tone}) and background ({bg_name}; {background_tone})"
                        );
                    }
                }
            }
        }
    }
}

fn assert_fixed_tones(scheme: &DynamicScheme, expected: [f64; 12]) {
    let colors = [
        MaterialDynamicColors::primary_fixed(),
        MaterialDynamicColors::primary_fixed_dim(),
        MaterialDynamicColors::on_primary_fixed(),
        MaterialDynamicColors::on_primary_fixed_variant(),
        MaterialDynamicColors::secondary_fixed(),
        MaterialDynamicColors::secondary_fixed_dim(),
        MaterialDynamicColors::on_secondary_fixed(),
        MaterialDynamicColors::on_secondary_fixed_variant(),
        MaterialDynamicColors::tertiary_fixed(),
        MaterialDynamicColors::tertiary_fixed_dim(),
        MaterialDynamicColors::on_tertiary_fixed(),
        MaterialDynamicColors::on_tertiary_fixed_variant(),
    ];
    for (color, tone) in colors.iter().zip(expected) {
        assert_close_to(color.get_hct(scheme).tone(), tone, 1.0);
    }
}

// Tests for fixed colors.
#[test]
fn fixed_colors_in_non_monochrome_schemes() {
    let scheme = SchemeTonalSpot::new(Hct::from_int(0xFFFF0000), true, 0.0);
    assert_fixed_tones(
        &scheme,
        [
            90.0, 80.0, 10.0, 30.0, 90.0, 80.0, 10.0, 30.0, 90.0, 80.0, 10.0, 30.0,
        ],
    );
}

#[test]
fn fixed_colors_in_light_monochrome_schemes() {
    let scheme = SchemeMonochrome::new(Hct::from_int(0xFFFF0000), false, 0.0);
    assert_fixed_tones(
        &scheme,
        [
            40.0, 30.0, 100.0, 90.0, 80.0, 70.0, 10.0, 25.0, 40.0, 30.0, 100.0, 90.0,
        ],
    );
}

#[test]
fn fixed_colors_in_dark_monochrome_schemes() {
    let scheme = SchemeMonochrome::new(Hct::from_int(0xFFFF0000), true, 0.0);
    assert_fixed_tones(
        &scheme,
        [
            40.0, 30.0, 100.0, 90.0, 80.0, 70.0, 10.0, 25.0, 40.0, 30.0, 100.0, 90.0,
        ],
    );
}
