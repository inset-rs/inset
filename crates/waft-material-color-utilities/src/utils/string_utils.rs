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
// Ported from `lib/utils/string_utils.dart` of material_color_utilities 0.13.0.

use crate::utils::color_utils::ColorUtils;

pub struct StringUtils;

impl StringUtils {
    /// `leading_hash_sign` is `true` by default in Dart.
    pub fn hex_from_argb(argb: u32, leading_hash_sign: bool) -> String {
        let red = ColorUtils::red_from_argb(argb);
        let green = ColorUtils::green_from_argb(argb);
        let blue = ColorUtils::blue_from_argb(argb);
        format!(
            "{}{:02X}{:02X}{:02X}",
            if leading_hash_sign { "#" } else { "" },
            red,
            green,
            blue
        )
    }

    /// Dart's `int.tryParse(radix: 16)` of `hex` without its `#`s: surrounding whitespace
    /// (Dart's, which counts U+FEFF) and a sign are accepted, and a value outside 64 bits is
    /// `None`.
    pub fn argb_from_hex(hex: &str) -> Option<i64> {
        let digits = hex.replace('#', "");
        let digits = digits.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}');
        i64::from_str_radix(digits, 16).ok()
    }
}

#[cfg(test)]
mod tests {
    // Not in the package's tests. The expected values are the Dart VM's: `hexFromArgb` of the
    // colors, and `int.tryParse(radix: 16)` of the strings, `#`s removed.
    use super::*;

    #[test]
    fn hex_from_argb_writes_dart_s_upper_case_digits() {
        assert_eq!(StringUtils::hex_from_argb(0xff0a1b2c, true), "#0A1B2C");
        assert_eq!(StringUtils::hex_from_argb(0xff0a1b2c, false), "0A1B2C");
    }

    #[test]
    fn argb_from_hex_parses_as_dart_s_int_try_parse() {
        assert_eq!(StringUtils::argb_from_hex(" ff "), Some(255));
        assert_eq!(StringUtils::argb_from_hex("+ff"), Some(255));
        assert_eq!(StringUtils::argb_from_hex("-ff"), Some(-255));
        assert_eq!(StringUtils::argb_from_hex("\u{FEFF}ff"), Some(255));
        assert_eq!(StringUtils::argb_from_hex("#FF00FF00"), Some(4278255360));
        assert_eq!(
            StringUtils::argb_from_hex("7fffffffffffffff"),
            Some(i64::MAX)
        );
        assert_eq!(StringUtils::argb_from_hex("ffffffffffffffff"), None);
        assert_eq!(StringUtils::argb_from_hex(""), None);
        assert_eq!(StringUtils::argb_from_hex("zz"), None);
        assert_eq!(StringUtils::argb_from_hex("0x10"), None);
    }
}
