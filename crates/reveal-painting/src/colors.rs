//! Flutter counterpart: `painting/colors.dart`.

use std::fmt::{self, Debug};

use reveal_embedder::{Color, clamp_double, lerp_double};

fn unit_channel(component: f64) -> f64 {
    (component * 255.0).round().clamp(0.0, 255.0) / 255.0
}

fn get_hue(red: f64, green: f64, blue: f64, max: f64, delta: f64) -> f64 {
    let hue = if max == 0.0 {
        0.0
    } else if max == red {
        60.0 * (((green - blue) / delta) % 6.0)
    } else if max == green {
        60.0 * (((blue - red) / delta) + 2.0)
    } else {
        60.0 * (((red - green) / delta) + 4.0)
    };
    if hue.is_nan() { 0.0 } else { hue }
}

fn color_from_hue(alpha: f64, hue: f64, chroma: f64, secondary: f64, match_: f64) -> Color {
    let (red, green, blue) = if hue < 60.0 {
        (chroma, secondary, 0.0)
    } else if hue < 120.0 {
        (secondary, chroma, 0.0)
    } else if hue < 180.0 {
        (0.0, chroma, secondary)
    } else if hue < 240.0 {
        (0.0, secondary, chroma)
    } else if hue < 300.0 {
        (secondary, 0.0, chroma)
    } else {
        (chroma, 0.0, secondary)
    };
    Color::from_argb(
        (alpha * (0xFF as f64)).round() as i32,
        ((red + match_) * (0xFF as f64)).round() as i32,
        ((green + match_) * (0xFF as f64)).round() as i32,
        ((blue + match_) * (0xFF as f64)).round() as i32,
    )
}

/// A color represented using [`alpha`], [`hue`], [`saturation`], and [`value`].
#[derive(Clone, Copy, PartialEq)]
pub struct HSVColor {
    /// Alpha, from 0.0 to 1.0.
    pub alpha: f64,
    /// Hue, from 0.0 to 360.0.
    pub hue: f64,
    /// Saturation, from 0.0 to 1.0.
    pub saturation: f64,
    /// Value, from 0.0 to 1.0.
    pub value: f64,
}

impl HSVColor {
    /// Creates a color. All arguments must be in their respective ranges.
    pub fn from_ahsv(alpha: f64, hue: f64, saturation: f64, value: f64) -> HSVColor {
        debug_assert!((0.0..=1.0).contains(&alpha));
        debug_assert!((0.0..=360.0).contains(&hue));
        debug_assert!((0.0..=1.0).contains(&saturation));
        debug_assert!((0.0..=1.0).contains(&value));
        HSVColor {
            alpha,
            hue,
            saturation,
            value,
        }
    }

    /// Creates an [`HSVColor`] from an RGB [`Color`].
    pub fn from_color(color: Color) -> HSVColor {
        let red = unit_channel(color.r);
        let green = unit_channel(color.g);
        let blue = unit_channel(color.b);
        let max = red.max(green.max(blue));
        let min = red.min(green.min(blue));
        let delta = max - min;
        let alpha = unit_channel(color.a);
        let hue = get_hue(red, green, blue, max, delta);
        let saturation = if max == 0.0 { 0.0 } else { delta / max };
        HSVColor::from_ahsv(alpha, hue, saturation, max)
    }

    /// Returns a copy of this color with the [`alpha`] parameter replaced.
    pub fn with_alpha(self, alpha: f64) -> HSVColor {
        HSVColor::from_ahsv(alpha, self.hue, self.saturation, self.value)
    }

    /// Returns a copy of this color with the [`hue`] parameter replaced.
    pub fn with_hue(self, hue: f64) -> HSVColor {
        HSVColor::from_ahsv(self.alpha, hue, self.saturation, self.value)
    }

    /// Returns a copy of this color with the [`saturation`] parameter replaced.
    pub fn with_saturation(self, saturation: f64) -> HSVColor {
        HSVColor::from_ahsv(self.alpha, self.hue, saturation, self.value)
    }

    /// Returns a copy of this color with the [`value`] parameter replaced.
    pub fn with_value(self, value: f64) -> HSVColor {
        HSVColor::from_ahsv(self.alpha, self.hue, self.saturation, value)
    }

    /// Returns this color in RGB.
    pub fn to_color(self) -> Color {
        let chroma = self.saturation * self.value;
        let secondary = chroma * (1.0 - (((self.hue / 60.0) % 2.0) - 1.0).abs());
        let match_ = self.value - chroma;
        color_from_hue(self.alpha, self.hue, chroma, secondary, match_)
    }

    fn scale_alpha(self, factor: f64) -> HSVColor {
        self.with_alpha(self.alpha * factor)
    }

    /// Linearly interpolate between two HSVColors.
    pub fn lerp(a: Option<HSVColor>, b: Option<HSVColor>, t: f64) -> Option<HSVColor> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b.scale_alpha(t)),
            (Some(a), None) => Some(a.scale_alpha(1.0 - t)),
            (Some(a), Some(b)) => Some(HSVColor::from_ahsv(
                clamp_double(
                    lerp_double(Some(a.alpha), Some(b.alpha), t).unwrap(),
                    0.0,
                    1.0,
                ),
                lerp_double(Some(a.hue), Some(b.hue), t).unwrap() % 360.0,
                clamp_double(
                    lerp_double(Some(a.saturation), Some(b.saturation), t).unwrap(),
                    0.0,
                    1.0,
                ),
                clamp_double(
                    lerp_double(Some(a.value), Some(b.value), t).unwrap(),
                    0.0,
                    1.0,
                ),
            )),
        }
    }
}

impl Debug for HSVColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HSVColor({}, {}, {}, {})",
            self.alpha, self.hue, self.saturation, self.value
        )
    }
}

/// A color represented using [`alpha`], [`hue`], [`saturation`], and [`lightness`].
#[derive(Clone, Copy, PartialEq)]
pub struct HSLColor {
    /// Alpha, from 0.0 to 1.0.
    pub alpha: f64,
    /// Hue, from 0.0 to 360.0.
    pub hue: f64,
    /// Saturation, from 0.0 to 1.0.
    pub saturation: f64,
    /// Lightness, from 0.0 to 1.0.
    pub lightness: f64,
}

impl HSLColor {
    /// Creates a color. All arguments must be in their respective ranges.
    pub fn from_ahsl(alpha: f64, hue: f64, saturation: f64, lightness: f64) -> HSLColor {
        debug_assert!((0.0..=1.0).contains(&alpha));
        debug_assert!((0.0..=360.0).contains(&hue));
        debug_assert!((0.0..=1.0).contains(&saturation));
        debug_assert!((0.0..=1.0).contains(&lightness));
        HSLColor {
            alpha,
            hue,
            saturation,
            lightness,
        }
    }

    /// Creates an [`HSLColor`] from an RGB [`Color`].
    pub fn from_color(color: Color) -> HSLColor {
        let red = unit_channel(color.r);
        let green = unit_channel(color.g);
        let blue = unit_channel(color.b);
        let max = red.max(green.max(blue));
        let min = red.min(green.min(blue));
        let delta = max - min;
        let alpha = unit_channel(color.a);
        let hue = get_hue(red, green, blue, max, delta);
        let lightness = (max + min) / 2.0;
        let saturation = if min == max {
            0.0
        } else {
            clamp_double(delta / (1.0 - (2.0 * lightness - 1.0).abs()), 0.0, 1.0)
        };
        HSLColor::from_ahsl(alpha, hue, saturation, lightness)
    }

    /// Returns a copy of this color with the alpha parameter replaced.
    pub fn with_alpha(self, alpha: f64) -> HSLColor {
        HSLColor::from_ahsl(alpha, self.hue, self.saturation, self.lightness)
    }

    /// Returns a copy of this color with the [`hue`] parameter replaced.
    pub fn with_hue(self, hue: f64) -> HSLColor {
        HSLColor::from_ahsl(self.alpha, hue, self.saturation, self.lightness)
    }

    /// Returns a copy of this color with the [`saturation`] parameter replaced.
    pub fn with_saturation(self, saturation: f64) -> HSLColor {
        HSLColor::from_ahsl(self.alpha, self.hue, saturation, self.lightness)
    }

    /// Returns a copy of this color with the [`lightness`] parameter replaced.
    pub fn with_lightness(self, lightness: f64) -> HSLColor {
        HSLColor::from_ahsl(self.alpha, self.hue, self.saturation, lightness)
    }

    /// Returns this HSL color in RGB.
    pub fn to_color(self) -> Color {
        let chroma = (1.0 - (2.0 * self.lightness - 1.0).abs()) * self.saturation;
        let secondary = chroma * (1.0 - (((self.hue / 60.0) % 2.0) - 1.0).abs());
        let match_ = self.lightness - chroma / 2.0;
        color_from_hue(self.alpha, self.hue, chroma, secondary, match_)
    }

    fn scale_alpha(self, factor: f64) -> HSLColor {
        self.with_alpha(self.alpha * factor)
    }

    /// Linearly interpolate between two HSLColors.
    pub fn lerp(a: Option<HSLColor>, b: Option<HSLColor>, t: f64) -> Option<HSLColor> {
        if a == b {
            return a;
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b.scale_alpha(t)),
            (Some(a), None) => Some(a.scale_alpha(1.0 - t)),
            (Some(a), Some(b)) => Some(HSLColor::from_ahsl(
                clamp_double(
                    lerp_double(Some(a.alpha), Some(b.alpha), t).unwrap(),
                    0.0,
                    1.0,
                ),
                lerp_double(Some(a.hue), Some(b.hue), t).unwrap() % 360.0,
                clamp_double(
                    lerp_double(Some(a.saturation), Some(b.saturation), t).unwrap(),
                    0.0,
                    1.0,
                ),
                clamp_double(
                    lerp_double(Some(a.lightness), Some(b.lightness), t).unwrap(),
                    0.0,
                    1.0,
                ),
            )),
        }
    }
}

impl Debug for HSLColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HSLColor({}, {}, {}, {})",
            self.alpha, self.hue, self.saturation, self.lightness
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // colors_test.dart 'HSVColor control test'.
    #[test]
    fn hsv_color_control_test() {
        let color = HSVColor::from_ahsv(0.7, 28.0, 0.3, 0.6);
        assert_eq!(
            color.with_alpha(0.8),
            HSVColor::from_ahsv(0.8, 28.0, 0.3, 0.6)
        );
        assert_eq!(
            color.with_hue(123.0),
            HSVColor::from_ahsv(0.7, 123.0, 0.3, 0.6)
        );
        assert_eq!(
            color.with_saturation(0.9),
            HSVColor::from_ahsv(0.7, 28.0, 0.9, 0.6)
        );
        assert_eq!(
            color.with_value(0.1),
            HSVColor::from_ahsv(0.7, 28.0, 0.3, 0.1)
        );
        assert_eq!(color.to_color(), Color::new(0xb399816b));

        let result = HSVColor::lerp(
            Some(color),
            Some(HSVColor::from_ahsv(0.3, 128.0, 0.7, 0.2)),
            0.25,
        )
        .unwrap();
        assert!((result.alpha - 0.6).abs() < 1e-10);
        assert!((result.hue - 53.0).abs() < 1e-10);
        assert!(result.saturation > 0.3999 && result.saturation < 0.4001);
        assert!((result.value - 0.5).abs() < 1e-10);
    }

    // colors_test.dart 'HSVColor hue sweep test'.
    #[test]
    fn hsv_color_hue_sweep() {
        let mut output = Vec::new();
        let mut hue = 0.0;
        while hue <= 360.0 {
            output.push(HSVColor::from_ahsv(1.0, hue, 1.0, 1.0).to_color());
            hue += 36.0;
        }
        let expected = [
            Color::new(0xffff0000),
            Color::new(0xffff9900),
            Color::new(0xffccff00),
            Color::new(0xff33ff00),
            Color::new(0xff00ff66),
            Color::new(0xff00ffff),
            Color::new(0xff0066ff),
            Color::new(0xff3300ff),
            Color::new(0xffcc00ff),
            Color::new(0xffff0099),
            Color::new(0xffff0000),
        ];
        assert_eq!(output, expected);
    }

    // colors_test.dart 'HSVColor saturation sweep test'.
    #[test]
    fn hsv_color_saturation_sweep() {
        let mut output = Vec::new();
        let mut saturation = 0.0;
        while saturation < 1.0 {
            output.push(HSVColor::from_ahsv(1.0, 0.0, saturation, 1.0).to_color());
            saturation += 0.1;
        }
        let expected = [
            Color::new(0xffffffff),
            Color::new(0xffffe6e6),
            Color::new(0xffffcccc),
            Color::new(0xffffb3b3),
            Color::new(0xffff9999),
            Color::new(0xffff8080),
            Color::new(0xffff6666),
            Color::new(0xffff4d4d),
            Color::new(0xffff3333),
            Color::new(0xffff1a1a),
            Color::new(0xffff0000),
        ];
        assert_eq!(output, expected);
    }

    // colors_test.dart 'HSVColor value sweep test'.
    #[test]
    fn hsv_color_value_sweep() {
        let mut output = Vec::new();
        let mut value = 0.0;
        while value < 1.0 {
            output.push(HSVColor::from_ahsv(1.0, 0.0, 1.0, value).to_color());
            value += 0.1;
        }
        let expected = [
            Color::new(0xff000000),
            Color::new(0xff1a0000),
            Color::new(0xff330000),
            Color::new(0xff4d0000),
            Color::new(0xff660000),
            Color::new(0xff800000),
            Color::new(0xff990000),
            Color::new(0xffb30000),
            Color::new(0xffcc0000),
            Color::new(0xffe50000),
            Color::new(0xffff0000),
        ];
        assert_eq!(output, expected);
    }

    // colors_test.dart 'HSLColor control test'.
    #[test]
    fn hsl_color_control_test() {
        let color = HSLColor::from_ahsl(0.7, 28.0, 0.3, 0.6);
        assert_eq!(
            color.with_alpha(0.8),
            HSLColor::from_ahsl(0.8, 28.0, 0.3, 0.6)
        );
        assert_eq!(
            color.with_hue(123.0),
            HSLColor::from_ahsl(0.7, 123.0, 0.3, 0.6)
        );
        assert_eq!(
            color.with_saturation(0.9),
            HSLColor::from_ahsl(0.7, 28.0, 0.9, 0.6)
        );
        assert_eq!(
            color.with_lightness(0.1),
            HSLColor::from_ahsl(0.7, 28.0, 0.3, 0.1)
        );
        assert_eq!(color.to_color(), Color::new(0xb3b8977a));

        let result = HSLColor::lerp(
            Some(color),
            Some(HSLColor::from_ahsl(0.3, 128.0, 0.7, 0.2)),
            0.25,
        )
        .unwrap();
        assert!((result.alpha - 0.6).abs() < 1e-10);
        assert!((result.hue - 53.0).abs() < 1e-10);
        assert!(result.saturation > 0.3999 && result.saturation < 0.4001);
        assert!((result.lightness - 0.5).abs() < 1e-10);
    }
}
