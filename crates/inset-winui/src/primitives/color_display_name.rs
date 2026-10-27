//! `ColorHelper::ToDisplayName` (`ColorDisplayNameHelper_Partial.cpp`): the friendly name of a
//! colour. Windows classifies the colour (`ColorHelper_ToDisplayNameId`, not in the repository)
//! and the name is one of the 42 strings `TEXT_COLOR_*` of `Microsoft.UI.Xaml.Common.rc`.
//!
//! The classification here is the one `ColorIntegrationTests::CanRetrieveColorNames` lays out,
//! in HSL with every channel from 0 to 255: black and white by luminosity, greys by low
//! saturation, and every other colour by one of five saturation bands, a hue range within the
//! band, and a dark, mid or light luminosity whose limits depend on the hue range.

use crate::argb;
use inset_embedder::Color;

/// The friendly name Windows gives `color`, as ColorPicker's spectrum and sliders show it;
/// its alpha is not read.
pub fn color_display_name(color: Color) -> &'static str {
    let (hue, saturation, luminosity) = hsl_255(color);
    if luminosity < BLACK_LUMINOSITY {
        return "Black";
    }
    if luminosity > WHITE_LUMINOSITY {
        return "White";
    }
    if saturation <= GRAY_SATURATION {
        return gray_name(luminosity);
    }
    let family = &FAMILIES[hue_family(saturation, hue)];
    if luminosity < family.low_luminosity {
        family.dark
    } else if luminosity <= family.high_luminosity {
        family.mid
    } else {
        family.light
    }
}

/// Below this luminosity every colour is black.
const BLACK_LUMINOSITY: f64 = 20.0;

/// Above this luminosity every colour is white.
const WHITE_LUMINOSITY: f64 = 240.0;

/// At or below this saturation a colour is a grey.
const GRAY_SATURATION: f64 = 20.0;

/// `Light gray` above 170, `Gray` above 100, `Dark gray` from 20 to 100.
fn gray_name(luminosity: f64) -> &'static str {
    if luminosity > 170.0 {
        "Light gray"
    } else if luminosity > 100.0 {
        "Gray"
    } else {
        "Dark gray"
    }
}

/// One of the test's 23 hue families: its dark, mid and light names and the luminosity limits
/// between them; a colour is dark below `low_luminosity` and light above `high_luminosity`.
struct Family {
    dark: &'static str,
    mid: &'static str,
    light: &'static str,
    low_luminosity: f64,
    high_luminosity: f64,
}

/// A family of `colorNamesDark`, `colorNamesMid` and `colorNamesLight`, with the limits of
/// `lowLuminosityLimits` and `highLuminosityLimits`.
const fn family(
    dark: &'static str,
    mid: &'static str,
    light: &'static str,
    low_luminosity: f64,
    high_luminosity: f64,
) -> Family {
    Family {
        dark,
        mid,
        light,
        low_luminosity,
        high_luminosity,
    }
}

/// The families in the test's order, numbered from 1 there.
const FAMILIES: [Family; 23] = [
    family("Brown", "Coral", "Coral", 130.0, 170.0),
    family("Dark red", "Red", "Rose", 100.0, 170.0),
    family("Brown", "Orange", "Light orange", 115.0, 170.0),
    family("Brown", "Brown", "Tan", 100.0, 155.0),
    family("Brown", "Tan", "Tan", 100.0, 170.0),
    family("Dark yellow", "Gold", "Light yellow", 100.0, 170.0),
    family("Dark yellow", "Yellow", "Light yellow", 110.0, 170.0),
    family("Brown", "Olive green", "Tan", 75.0, 170.0),
    family("Dark green", "Olive green", "Light green", 100.0, 170.0),
    family("Dark green", "Green", "Lime", 90.0, 115.0),
    family("Dark green", "Green", "Light green", 100.0, 170.0),
    family("Dark green", "Bright green", "Light green", 100.0, 170.0),
    family("Dark teal", "Teal", "Aqua", 100.0, 170.0),
    family("Dark teal", "Aqua", "Sky blue", 100.0, 170.0),
    family("Dark teal", "Turquoise", "Light turquoise", 80.0, 170.0),
    family("Dark blue", "Pale blue", "Pale blue", 100.0, 170.0),
    family("Dark blue", "Blue", "Light blue", 100.0, 170.0),
    family("Blue-gray", "Blue-gray", "Ice blue", 100.0, 170.0),
    family("Indigo", "Indigo", "Periwinkle", 100.0, 150.0),
    family("Dark purple", "Purple", "Lavender", 100.0, 150.0),
    family("Plum", "Pink", "Pink", 100.0, 170.0),
    family("Brown", "Brown", "Tan", 100.0, 140.0),
    family("Dark red", "Red", "Rose", 100.0, 165.0),
];

/// A saturation band: its highest saturation and its hue ranges, each the hue it ends before
/// and the number of its family.
struct Band {
    max_saturation: f64,
    hues: &'static [(f64, usize)],
}

/// The five bands of `BEGIN_SATURATION_RANGE`, each with its `TEST_NEXT_HUE_RANGE` calls.
const BANDS: [Band; 5] = [
    Band {
        max_saturation: 75.0,
        hues: &[
            (8.0, 1),
            (44.0, 4),
            (63.0, 8),
            (122.0, 11),
            (134.0, 13),
            (166.0, 18),
            (176.0, 19),
            (241.0, 20),
            (256.0, 22),
        ],
    },
    Band {
        max_saturation: 115.0,
        hues: &[
            (10.0, 2),
            (32.0, 4),
            (46.0, 5),
            (60.0, 9),
            (120.0, 11),
            (131.0, 13),
            (144.0, 14),
            (158.0, 18),
            (166.0, 19),
            (241.0, 20),
            (256.0, 23),
        ],
    },
    Band {
        max_saturation: 150.0,
        hues: &[
            (8.0, 2),
            (39.0, 5),
            (46.0, 6),
            (71.0, 10),
            (120.0, 11),
            (131.0, 13),
            (144.0, 14),
            (163.0, 17),
            (177.0, 19),
            (211.0, 20),
            (249.0, 21),
            (256.0, 23),
        ],
    },
    Band {
        max_saturation: 240.0,
        hues: &[
            (11.0, 2),
            (26.0, 3),
            (38.0, 6),
            (45.0, 7),
            (56.0, 10),
            (100.0, 11),
            (121.0, 12),
            (129.0, 13),
            (140.0, 15),
            (180.0, 17),
            (224.0, 20),
            (241.0, 21),
            (256.0, 23),
        ],
    },
    Band {
        max_saturation: 255.0,
        hues: &[
            (13.0, 2),
            (27.0, 3),
            (36.0, 6),
            (45.0, 7),
            (59.0, 10),
            (118.0, 11),
            (127.0, 13),
            (136.0, 14),
            (142.0, 15),
            (185.0, 17),
            (216.0, 20),
            (239.0, 21),
            (256.0, 23),
        ],
    },
];

/// The index in `FAMILIES` of a colour with `saturation` above the greys and `hue`.
fn hue_family(saturation: f64, hue: f64) -> usize {
    let band = BANDS
        .iter()
        .find(|band| saturation <= band.max_saturation)
        .unwrap_or(&BANDS[BANDS.len() - 1]);
    let (_, number) = band
        .hues
        .iter()
        .find(|(end, _)| hue < *end)
        .unwrap_or(&band.hues[band.hues.len() - 1]);
    number - 1
}

/// Hue, saturation and luminosity of `color`, each from 0 to 255, as `ColorFromHsl` of the test
/// inverts them.
fn hsl_255(color: Color) -> (f64, f64, f64) {
    let bytes = argb(color);
    let [r, g, b] = [bytes.r, bytes.g, bytes.b].map(|c| f64::from(c) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let luminosity = (max + min) / 2.0;
    let chroma = max - min;
    if chroma == 0.0 {
        return (0.0, 0.0, luminosity * 255.0);
    }
    let saturation = chroma / (1.0 - (2.0 * luminosity - 1.0).abs());
    let mut hue = if max == r {
        60.0 * (g - b) / chroma
    } else if max == g {
        120.0 + 60.0 * (b - r) / chroma
    } else {
        240.0 + 60.0 * (r - g) / chroma
    };
    if hue < 0.0 {
        hue += 360.0;
    }
    (hue * 255.0 / 360.0, saturation * 255.0, luminosity * 255.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ColorIntegrationTests::ColorFromHsl`: the colour of hue, saturation and luminosity from 0
    /// to 255, each channel truncated to a byte.
    fn color_from_hsl(h: f64, s: f64, l: f64) -> Color {
        let (h, s, l) = (h / 255.0, s / 255.0, l / 255.0);
        let chroma = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let min = l - chroma / 2.0;
        let sextant = (h * 6.0) as i32;
        let intermediate = h * 6.0 - f64::from(sextant);
        let max = chroma + min;
        let (r, g, b) = match sextant {
            0 => (max, min + chroma * intermediate, min),
            1 => (min + chroma * (1.0 - intermediate), max, min),
            2 => (min, max, min + chroma * intermediate),
            3 => (min, min + chroma * (1.0 - intermediate), max),
            4 => (min + chroma * intermediate, min, max),
            5 => (max, min, min + chroma * (1.0 - intermediate)),
            _ => (0.0, 0.0, 0.0),
        };
        let byte = |c: f64| (c * 255.0) as i32;
        Color::from_argb(255, byte(r), byte(g), byte(b))
    }

    /// `ValidateColorNameFromHslRange`: the points a quarter, half and three quarters into a
    /// range, kept five away from its ends, have the name.
    #[allow(clippy::too_many_arguments)]
    fn validate_range(h: (f64, f64), s: (f64, f64), l: (f64, f64), expected: &str) {
        let quarter = |(min, max): (f64, f64)| ((min * 3.0 + max) / 4.0).max(min + 5.0);
        let half = |(min, max): (f64, f64)| (min + max) / 2.0;
        let three_quarters = |(min, max): (f64, f64)| {
            let point = (min + max * 3.0) / 4.0;
            if point > max + 5.0 { max - 5.0 } else { point }
        };
        let (h1, s1, l1) = (quarter(h), quarter(s), quarter(l));
        let (h2, s2, l2) = (half(h), half(s), half(l));
        let (h3, s3, l3) = (three_quarters(h), three_quarters(s), three_quarters(l));
        if h1 < h2 && s1 < s2 && l1 < l2 {
            assert_eq!(color_display_name(color_from_hsl(h1, s1, l1)), expected);
        }
        assert_eq!(color_display_name(color_from_hsl(h2, s2, l2)), expected);
        if h3 > h2 && s3 > s2 && l3 > l2 {
            assert_eq!(color_display_name(color_from_hsl(h3, s3, l3)), expected);
        }
    }

    #[test]
    fn the_integration_test_classification_holds() {
        validate_range((0.0, 0.0), (0.0, 255.0), (0.0, 19.0), "Black");
        validate_range((0.0, 0.0), (0.0, 255.0), (241.0, 255.0), "White");
        validate_range((0.0, 0.0), (0.0, 20.0), (171.0, 240.0), "Light gray");
        validate_range((0.0, 0.0), (0.0, 20.0), (101.0, 170.0), "Gray");
        validate_range((0.0, 0.0), (0.0, 20.0), (20.0, 100.0), "Dark gray");
        let mut min_saturation = GRAY_SATURATION;
        for band in &BANDS {
            let saturation = (min_saturation, band.max_saturation);
            let mut min_hue = 0.0;
            for &(end, number) in band.hues {
                let hue = (min_hue, end - 1.0);
                let f = &FAMILIES[number - 1];
                validate_range(hue, saturation, (f.high_luminosity + 1.0, 240.0), f.light);
                validate_range(
                    hue,
                    saturation,
                    (f.low_luminosity, f.high_luminosity),
                    f.mid,
                );
                validate_range(hue, saturation, (20.0, f.low_luminosity), f.dark);
                min_hue = end;
            }
            min_saturation = band.max_saturation + 1.0;
        }
    }

    #[test]
    fn the_spectrum_test_colours_have_their_names() {
        // `CanUseKeyboardToInteractWithColorSpectrum` and `CanUseKeyboardToInteractWithSliders`.
        for (r, g, b, name) in [
            (255, 0, 0, "Red"),
            (255, 119, 0, "Orange"),
            (255, 0, 170, "Pink"),
            (255, 153, 158, "Rose"),
            (255, 242, 243, "White"),
            (102, 15, 20, "Dark red"),
            (15, 2, 3, "Black"),
        ] {
            assert_eq!(color_display_name(Color::from_argb(255, r, g, b)), name);
        }
    }
}
