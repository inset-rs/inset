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
// Ported from `test/scheme_correctness_test.dart` of material_color_utilities 0.13.0.

mod utils;

use std::rc::Rc;

use waft_material_color_utilities::dynamiccolor::src::contrast_curve::ContrastCurve;
use waft_material_color_utilities::dynamiccolor::src::tone_delta_pair::TonePolarity;
use waft_material_color_utilities::{
    Contrast, DynamicColor, DynamicScheme, Hct, MaterialDynamicColors, Variant,
};

use utils::scheme_from_variant::scheme_from_variant;

enum Constraint {
    Contrast {
        foreground: Rc<DynamicColor>,
        background: Rc<DynamicColor>,
        contrast_curve: ContrastCurve,
    },
    Delta {
        role_a: Rc<DynamicColor>,
        role_b: Rc<DynamicColor>,
        delta: f64,
        polarity: TonePolarity,
    },
    Background(Rc<DynamicColor>),
}

const CONTRAST_TOLERANCE: f64 = 0.05;
const DELTA_TOLERANCE: f64 = 0.5;

fn scheme_parameters(scheme: &DynamicScheme) -> String {
    format!(
        "Scheme parameters:\n  Variant: {:?}\n  Source color: {:?}\n  Brightness: {}\n  Contrast level: {}\n",
        scheme.variant(),
        scheme.source_color_hct(),
        if scheme.is_dark() { "dark" } else { "light" },
        scheme.contrast_level()
    )
}

impl Constraint {
    /// Tests this constraint against `scheme`, failing if constraint not met.
    fn test_against(&self, scheme: &DynamicScheme) {
        match self {
            Constraint::Contrast {
                foreground,
                background,
                contrast_curve,
            } => {
                let foreground_color = foreground.get_hct(scheme);
                let background_color = background.get_hct(scheme);
                let actual_contrast =
                    Contrast::ratio_of_tones(foreground_color.tone(), background_color.tone());
                let desired_contrast = contrast_curve.get(scheme.contrast_level());

                if desired_contrast <= 4.5 {
                    // A requirement of <= 4.5 must be met (with tolerance)
                    if actual_contrast < desired_contrast - CONTRAST_TOLERANCE {
                        panic!(
                            "Dynamic scheme fails contrast constraint:\n{} should have contrast at least {desired_contrast} against {}, but has {actual_contrast}\n\n{}",
                            foreground.name,
                            background.name,
                            scheme_parameters(scheme)
                        );
                    }
                } else if actual_contrast < 4.5 - CONTRAST_TOLERANCE {
                    panic!(
                        "Dynamic scheme fails contrast constraint:\n{} should have contrast at least 4.5 against {}, but has {actual_contrast}\n\n{}",
                        foreground.name,
                        background.name,
                        scheme_parameters(scheme)
                    );
                } else if actual_contrast < desired_contrast - CONTRAST_TOLERANCE
                    && foreground_color.tone() != 100.0
                    && foreground_color.tone() != 0.0
                {
                    panic!(
                        "Dynamic scheme fails contrast constraint:\n{} should have contrast at least {desired_contrast} against {}, but has {actual_contrast}, and no color has a tone of 0 or 100\n\n{}",
                        foreground.name,
                        background.name,
                        scheme_parameters(scheme)
                    );
                }
            }
            Constraint::Delta {
                role_a,
                role_b,
                delta,
                polarity,
            } => {
                let role_a_color = role_a.get_hct(scheme);
                let role_b_color = role_b.get_hct(scheme);
                let role_a_should_be_lighter = (*polarity == TonePolarity::Lighter)
                    || (*polarity == TonePolarity::Nearer && !scheme.is_dark())
                    || (*polarity == TonePolarity::Farther && scheme.is_dark());
                let lighter_or_darker = if role_a_should_be_lighter {
                    "lighter"
                } else {
                    "darker"
                };

                let actual_delta = if role_a_should_be_lighter {
                    role_a_color.tone() - role_b_color.tone()
                } else {
                    role_b_color.tone() - role_a_color.tone()
                };
                if actual_delta < delta - DELTA_TOLERANCE {
                    panic!(
                        "Dynamic scheme fails delta constraint:\n{} should be {delta} {lighter_or_darker} than {}, but they have tones {} and {}, respectively\n\n{}",
                        role_a.name,
                        role_b.name,
                        role_a_color.tone(),
                        role_b_color.tone(),
                        scheme_parameters(scheme)
                    );
                }
            }
            Constraint::Background(background) => {
                let color = background.get_hct(scheme);
                if color.tone() >= 50.5 && color.tone() < 59.5 {
                    panic!(
                        "Dynamic scheme fails background constraint:\n{} has tone {} which is in the forbidden zone 50.5 <= tone < 59.5\n\n{}",
                        background.name,
                        color.tone(),
                        scheme_parameters(scheme)
                    );
                }
            }
        }
    }
}

fn constraints() -> Vec<Constraint> {
    vec![
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_surface(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(4.5, 7.0, 11.0, 21.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_surface(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(4.5, 7.0, 11.0, 21.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::primary(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 7.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::primary(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 7.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::secondary(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 7.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::secondary(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 7.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::tertiary(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 7.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::tertiary(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 7.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::error(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 7.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::error(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 7.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_surface_variant(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 11.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_surface_variant(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 11.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::outline(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(1.5, 3.0, 4.5, 7.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::outline(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(1.5, 3.0, 4.5, 7.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::primary_container(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::primary_container(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::primary_fixed(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::primary_fixed(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::primary_fixed_dim(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::primary_fixed_dim(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::secondary_container(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::secondary_container(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::secondary_fixed(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::secondary_fixed(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::secondary_fixed_dim(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::secondary_fixed_dim(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::tertiary_container(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::tertiary_container(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::tertiary_fixed(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::tertiary_fixed(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::tertiary_fixed_dim(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::tertiary_fixed_dim(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::error_container(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::error_container(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::outline_variant(),
            background: MaterialDynamicColors::surface_dim(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::outline_variant(),
            background: MaterialDynamicColors::surface_bright(),
            contrast_curve: ContrastCurve::new(0.0, 0.0, 3.0, 4.5),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::inverse_on_surface(),
            background: MaterialDynamicColors::inverse_surface(),
            contrast_curve: ContrastCurve::new(4.5, 7.0, 11.0, 21.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::inverse_primary(),
            background: MaterialDynamicColors::inverse_surface(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 7.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_primary(),
            background: MaterialDynamicColors::primary(),
            contrast_curve: ContrastCurve::new(4.5, 7.0, 11.0, 21.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_secondary(),
            background: MaterialDynamicColors::secondary(),
            contrast_curve: ContrastCurve::new(4.5, 7.0, 11.0, 21.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_tertiary(),
            background: MaterialDynamicColors::tertiary(),
            contrast_curve: ContrastCurve::new(4.5, 7.0, 11.0, 21.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_error(),
            background: MaterialDynamicColors::error(),
            contrast_curve: ContrastCurve::new(4.5, 7.0, 11.0, 21.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_primary_container(),
            background: MaterialDynamicColors::primary_container(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 11.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_secondary_container(),
            background: MaterialDynamicColors::secondary_container(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 11.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_tertiary_container(),
            background: MaterialDynamicColors::tertiary_container(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 11.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_error_container(),
            background: MaterialDynamicColors::error_container(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 11.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_primary_fixed(),
            background: MaterialDynamicColors::primary_fixed(),
            contrast_curve: ContrastCurve::new(4.5, 7.0, 11.0, 21.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_primary_fixed(),
            background: MaterialDynamicColors::primary_fixed_dim(),
            contrast_curve: ContrastCurve::new(4.5, 7.0, 11.0, 21.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_secondary_fixed(),
            background: MaterialDynamicColors::secondary_fixed(),
            contrast_curve: ContrastCurve::new(4.5, 7.0, 11.0, 21.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_secondary_fixed(),
            background: MaterialDynamicColors::secondary_fixed_dim(),
            contrast_curve: ContrastCurve::new(4.5, 7.0, 11.0, 21.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_tertiary_fixed(),
            background: MaterialDynamicColors::tertiary_fixed(),
            contrast_curve: ContrastCurve::new(4.5, 7.0, 11.0, 21.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_tertiary_fixed(),
            background: MaterialDynamicColors::tertiary_fixed_dim(),
            contrast_curve: ContrastCurve::new(4.5, 7.0, 11.0, 21.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_primary_fixed_variant(),
            background: MaterialDynamicColors::primary_fixed(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 11.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_primary_fixed_variant(),
            background: MaterialDynamicColors::primary_fixed_dim(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 11.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_secondary_fixed_variant(),
            background: MaterialDynamicColors::secondary_fixed(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 11.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_secondary_fixed_variant(),
            background: MaterialDynamicColors::secondary_fixed_dim(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 11.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_tertiary_fixed_variant(),
            background: MaterialDynamicColors::tertiary_fixed(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 11.0),
        },
        Constraint::Contrast {
            foreground: MaterialDynamicColors::on_tertiary_fixed_variant(),
            background: MaterialDynamicColors::tertiary_fixed_dim(),
            contrast_curve: ContrastCurve::new(3.0, 4.5, 7.0, 11.0),
        },
        Constraint::Delta {
            role_a: MaterialDynamicColors::primary(),
            role_b: MaterialDynamicColors::primary_container(),
            delta: 10.0,
            polarity: TonePolarity::Farther,
        },
        Constraint::Delta {
            role_a: MaterialDynamicColors::secondary(),
            role_b: MaterialDynamicColors::secondary_container(),
            delta: 10.0,
            polarity: TonePolarity::Farther,
        },
        Constraint::Delta {
            role_a: MaterialDynamicColors::tertiary(),
            role_b: MaterialDynamicColors::tertiary_container(),
            delta: 10.0,
            polarity: TonePolarity::Farther,
        },
        Constraint::Delta {
            role_a: MaterialDynamicColors::error(),
            role_b: MaterialDynamicColors::error_container(),
            delta: 10.0,
            polarity: TonePolarity::Farther,
        },
        Constraint::Delta {
            role_a: MaterialDynamicColors::primary_fixed_dim(),
            role_b: MaterialDynamicColors::primary_fixed(),
            delta: 10.0,
            polarity: TonePolarity::Darker,
        },
        Constraint::Delta {
            role_a: MaterialDynamicColors::secondary_fixed_dim(),
            role_b: MaterialDynamicColors::secondary_fixed(),
            delta: 10.0,
            polarity: TonePolarity::Darker,
        },
        Constraint::Delta {
            role_a: MaterialDynamicColors::tertiary_fixed_dim(),
            role_b: MaterialDynamicColors::tertiary_fixed(),
            delta: 10.0,
            polarity: TonePolarity::Darker,
        },
        Constraint::Background(MaterialDynamicColors::background()),
        Constraint::Background(MaterialDynamicColors::error()),
        Constraint::Background(MaterialDynamicColors::error_container()),
        Constraint::Background(MaterialDynamicColors::primary()),
        Constraint::Background(MaterialDynamicColors::primary_container()),
        Constraint::Background(MaterialDynamicColors::primary_fixed()),
        Constraint::Background(MaterialDynamicColors::primary_fixed_dim()),
        Constraint::Background(MaterialDynamicColors::secondary()),
        Constraint::Background(MaterialDynamicColors::secondary_container()),
        Constraint::Background(MaterialDynamicColors::secondary_fixed()),
        Constraint::Background(MaterialDynamicColors::secondary_fixed_dim()),
        Constraint::Background(MaterialDynamicColors::surface()),
        Constraint::Background(MaterialDynamicColors::surface_bright()),
        Constraint::Background(MaterialDynamicColors::surface_container()),
        Constraint::Background(MaterialDynamicColors::surface_container_high()),
        Constraint::Background(MaterialDynamicColors::surface_container_highest()),
        Constraint::Background(MaterialDynamicColors::surface_container_low()),
        Constraint::Background(MaterialDynamicColors::surface_container_lowest()),
        Constraint::Background(MaterialDynamicColors::surface_dim()),
        Constraint::Background(MaterialDynamicColors::surface_tint()),
        Constraint::Background(MaterialDynamicColors::surface_variant()),
        Constraint::Background(MaterialDynamicColors::tertiary()),
        Constraint::Background(MaterialDynamicColors::tertiary_container()),
        Constraint::Background(MaterialDynamicColors::tertiary_fixed()),
        Constraint::Background(MaterialDynamicColors::tertiary_fixed_dim()),
    ]
}

/// For each variant-contrast combination, tests across four seed colors as well as two
/// brightnesses.
fn test_variant(variant: Variant, contrast_level: f64) {
    let constraints = constraints();
    for source_color_argb in [0xFF0000FF, 0xFF00FF00, 0xFFFFFF00, 0xFFFF0000] {
        for is_dark in [false, true] {
            let s = scheme_from_variant(
                variant,
                Hct::from_int(source_color_argb),
                is_dark,
                contrast_level,
            );
            // Ensures all constraints are satisfied.
            for constraint in &constraints {
                constraint.test_against(&s);
            }
        }
    }
}

#[test]
fn scheme_correctness_test() {
    for variant in Variant::VALUES {
        for contrast_level in [-1.0, 0.0, 0.5, 1.0] {
            test_variant(variant, contrast_level);
        }
    }
}
