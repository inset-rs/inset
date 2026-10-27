//! `controls/dev/Common/ColorConversion.h` and `ColorConversion.cpp`: the RGB, HSV and hex
//! conversions ColorPicker and its parts share, and the integer parsing their text boxes use.

use inset_embedder::Color;

/// `Rgb`: red, green and blue, each from 0 to 1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Rgb {
    pub(crate) r: f64,
    pub(crate) g: f64,
    pub(crate) b: f64,
}

impl Rgb {
    pub(crate) const fn new(r: f64, g: f64, b: f64) -> Rgb {
        Rgb { r, g, b }
    }
}

/// `Hsv`: hue in degrees from 0 to 360, saturation and value from 0 to 1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Hsv {
    pub(crate) h: f64,
    pub(crate) s: f64,
    pub(crate) v: f64,
}

impl Hsv {
    pub(crate) const fn new(h: f64, s: f64, v: f64) -> Hsv {
        Hsv { h, s, v }
    }
}

/// `TryParseInt` with base 10.
pub(crate) fn try_parse_int(text: &str) -> Option<u32> {
    try_parse_int_radix(text, 10)
}

/// `TryParseInt`: `text` as `wcstoul` reads it in `radix` when it reads all of it, as an
/// unsigned 32-bit number. Like `wcstoul`, it skips leading white space, takes a sign (a minus
/// negates the number modulo 2^32), takes `0x` before hexadecimal digits, and gives the largest
/// number on overflow.
pub(crate) fn try_parse_int_radix(text: &str, radix: u32) -> Option<u32> {
    if text.is_empty() {
        return None;
    }
    let (value, rest) = wcstoul(text, radix);
    rest.is_empty().then_some(value)
}

/// `wcstoul` with Windows' 32-bit `unsigned long`: the number at the start of `text` and the
/// text after it, which is all of `text` when there is no number.
fn wcstoul(text: &str, radix: u32) -> (u32, &str) {
    let unsigned = text.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let (negative, unsigned) = match unsigned.as_bytes().first() {
        Some(b'-') => (true, &unsigned[1..]),
        Some(b'+') => (false, &unsigned[1..]),
        _ => (false, unsigned),
    };
    let digits = if radix == 16
        && (unsigned.starts_with("0x") || unsigned.starts_with("0X"))
        && unsigned[2..].starts_with(|c: char| c.is_ascii_hexdigit())
    {
        &unsigned[2..]
    } else {
        unsigned
    };
    let length = digits
        .find(|c: char| !c.is_digit(radix))
        .unwrap_or(digits.len());
    if length == 0 {
        return (0, text);
    }
    let magnitude = digits[..length].chars().try_fold(0u32, |value, c| {
        value
            .checked_mul(radix)?
            .checked_add(c.to_digit(radix).expect("a digit of the radix"))
    });
    let value = match magnitude {
        None => u32::MAX,
        Some(magnitude) if negative => magnitude.wrapping_neg(),
        Some(magnitude) => magnitude,
    };
    (value, &digits[length..])
}

/// `_wtoi`: the decimal integer at the start of `text`, after white space and a sign; 0 when
/// there is none.
pub(crate) fn wtoi(text: &str) -> i32 {
    let unsigned = text.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let (negative, unsigned) = match unsigned.as_bytes().first() {
        Some(b'-') => (true, &unsigned[1..]),
        Some(b'+') => (false, &unsigned[1..]),
        _ => (false, unsigned),
    };
    let magnitude = unsigned
        .chars()
        .map_while(|c| c.to_digit(10))
        .fold(0i64, |value, digit| {
            (value * 10 + i64::from(digit)).min(i64::from(i32::MAX) + 1)
        });
    let value = if negative { -magnitude } else { magnitude };
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// `RgbToHsv`.
pub(crate) fn rgb_to_hsv(rgb: Rgb) -> Hsv {
    let max = if rgb.r >= rgb.g {
        if rgb.r >= rgb.b { rgb.r } else { rgb.b }
    } else if rgb.g >= rgb.b {
        rgb.g
    } else {
        rgb.b
    };
    let min = if rgb.r <= rgb.g {
        if rgb.r <= rgb.b { rgb.r } else { rgb.b }
    } else if rgb.g <= rgb.b {
        rgb.g
    } else {
        rgb.b
    };
    let value = max;
    let chroma = max - min;
    // A grey has no hue; hue and saturation are zero.
    if chroma == 0.0 {
        return Hsv::new(0.0, 0.0, value);
    }
    let mut hue = if rgb.r == max {
        60.0 * (rgb.g - rgb.b) / chroma
    } else if rgb.g == max {
        120.0 + 60.0 * (rgb.b - rgb.r) / chroma
    } else {
        240.0 + 60.0 * (rgb.r - rgb.g) / chroma
    };
    if hue < 0.0 {
        hue += 360.0;
    }
    Hsv::new(hue, chroma / value, value)
}

/// `HsvToRgb`: the hue is brought into `[0, 360)` and saturation and value into `[0, 1]` first.
pub(crate) fn hsv_to_rgb(hsv: Hsv) -> Rgb {
    let mut hue = hsv.h;
    while hue >= 360.0 {
        hue -= 360.0;
    }
    while hue < 0.0 {
        hue += 360.0;
    }
    let saturation = hsv.s.clamp(0.0, 1.0);
    let value = hsv.v.clamp(0.0, 1.0);
    let chroma = saturation * value;
    let min = value - chroma;
    if chroma == 0.0 {
        return Rgb::new(min, min, min);
    }
    // The sextant of the colour wheel: red to yellow, yellow to green, and so on.
    let sextant = (hue / 60.0) as i32;
    let intermediate = hue / 60.0 - f64::from(sextant);
    let max = chroma + min;
    match sextant {
        0 => Rgb::new(max, min + chroma * intermediate, min),
        1 => Rgb::new(min + chroma * (1.0 - intermediate), max, min),
        2 => Rgb::new(min, max, min + chroma * intermediate),
        3 => Rgb::new(min, min + chroma * (1.0 - intermediate), max),
        4 => Rgb::new(min + chroma * intermediate, min, max),
        5 => Rgb::new(max, min, min + chroma * (1.0 - intermediate)),
        _ => Rgb::new(0.0, 0.0, 0.0),
    }
}

/// `HexToRgba`: `#AARRGGBB`, with fewer digits the leading channels zero; `None` for text that is
/// not a number after its first character, where the source returns -1 for every channel.
pub(crate) fn hex_to_rgba(input: &str) -> Option<(Rgb, f64)> {
    // The input always begins with a `#`, which is skipped.
    let digits = input.char_indices().nth(1).map_or("", |(i, _)| &input[i..]);
    let hex = try_parse_int_radix(digits, 16)?;
    let [a, r, g, b] = hex.to_be_bytes();
    let channel = |byte: u8| f64::from(byte) / 255.0;
    Some((Rgb::new(channel(r), channel(g), channel(b)), channel(a)))
}

/// `HexToRgb`: `hex_to_rgba` without its alpha.
pub(crate) fn hex_to_rgb(input: &str) -> Option<Rgb> {
    hex_to_rgba(input).map(|(rgb, _)| rgb)
}

/// The byte `static_cast<byte>(round(channel * 255))` gives.
fn channel_byte(channel: f64) -> u8 {
    (channel * 255.0).round() as u8
}

/// `RgbToHex`: `#RRGGBB`.
pub(crate) fn rgb_to_hex(rgb: Rgb) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        channel_byte(rgb.r),
        channel_byte(rgb.g),
        channel_byte(rgb.b)
    )
}

/// `RgbaToHex`: `#AARRGGBB`.
pub(crate) fn rgba_to_hex(rgb: Rgb, alpha: f64) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        channel_byte(alpha),
        channel_byte(rgb.r),
        channel_byte(rgb.g),
        channel_byte(rgb.b)
    )
}

/// `ColorFromRgba`: each channel rounded to a byte.
pub(crate) fn color_from_rgba(rgb: Rgb, alpha: f64) -> Color {
    Color::from_argb(
        i32::from(channel_byte(alpha)),
        i32::from(channel_byte(rgb.r)),
        i32::from(channel_byte(rgb.g)),
        i32::from(channel_byte(rgb.b)),
    )
}

/// The bytes of a `Windows.UI.Color`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Argb {
    pub(crate) a: u8,
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
}

/// The bytes of `color`.
pub(crate) fn argb(color: Color) -> Argb {
    let [a, r, g, b] = color.to_argb32().to_be_bytes();
    Argb { a, r, g, b }
}

/// `RgbFromColor`.
pub(crate) fn rgb_from_color(color: Color) -> Rgb {
    let bytes = argb(color);
    Rgb::new(
        f64::from(bytes.r) / 255.0,
        f64::from(bytes.g) / 255.0,
        f64::from(bytes.b) / 255.0,
    )
}

/// Whether two colours have the same bytes, as `Windows.UI.Color` compares them.
pub(crate) fn same_color(a: Color, b: Color) -> bool {
    a.to_argb32() == b.to_argb32()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_parse_as_wcstoul_reads_all_of_them() {
        assert_eq!(try_parse_int("255"), Some(255));
        assert_eq!(try_parse_int(" +12"), Some(12));
        assert_eq!(try_parse_int("-0"), Some(0));
        assert_eq!(try_parse_int("-1"), Some(u32::MAX));
        assert_eq!(try_parse_int(""), None);
        assert_eq!(try_parse_int("12a"), None);
        assert_eq!(try_parse_int("1 "), None);
        assert_eq!(try_parse_int("+"), None);
        assert_eq!(try_parse_int_radix("0xFF", 16), Some(255));
        assert_eq!(try_parse_int_radix("FFFFFFFFF", 16), Some(u32::MAX));
        assert_eq!(try_parse_int_radix("foo", 16), None);
    }

    #[test]
    fn wtoi_reads_the_leading_integer() {
        assert_eq!(wtoi("128"), 128);
        assert_eq!(wtoi(" -5x"), -5);
        assert_eq!(wtoi("abc"), 0);
    }

    #[test]
    fn hex_text_converts_both_ways() {
        let (rgb, alpha) = hex_to_rgba("#FF804040").unwrap();
        assert_eq!(
            color_from_rgba(rgb, alpha),
            Color::from_argb(255, 128, 64, 64)
        );
        assert_eq!(rgb_to_hex(rgb), "#804040");
        assert_eq!(rgba_to_hex(rgb, 0.5), "#80804040");
        // Six digits with alpha enabled read as a transparent colour.
        assert_eq!(hex_to_rgba("#FF0000").unwrap().1, 0.0);
        assert_eq!(hex_to_rgba("#foo"), None);
        assert_eq!(hex_to_rgba("#"), None);
    }

    #[test]
    fn hsv_and_rgb_round_trip() {
        let hsv = rgb_to_hsv(Rgb::new(1.0, 38.0 / 255.0, 49.0 / 255.0));
        assert_eq!(
            color_from_rgba(hsv_to_rgb(hsv), 1.0),
            Color::from_argb(255, 255, 38, 49)
        );
        assert_eq!(rgb_to_hsv(Rgb::new(0.5, 0.5, 0.5)), Hsv::new(0.0, 0.0, 0.5));
        assert_eq!(
            color_from_rgba(hsv_to_rgb(Hsv::new(357.0, 1.0, 1.0)), 1.0),
            Color::from_argb(255, 255, 0, 13)
        );
    }
}
