//! Which family carries a character the bundled Roboto slice cannot draw.
//!
//! Flutter counterpart: `web_ui/lib/src/engine/font_fallbacks.dart` and the
//! generated `font_fallback_data.dart`.
//!
//! Flutter ships every Noto font's coverage as generated data and covers the
//! missing characters with a set of fonts chosen from it. These tables are
//! that same data reduced to one family per Unicode script, which is all the
//! CSS2 `text=` endpoint needs: it subsets a family to the demanded characters
//! itself, so the per-character coverage half of Flutter's table has no work
//! to do here.

/// Roboto itself, for the scripts it carries.
pub const ROBOTO: &str = "Roboto";

/// The family that should be asked for `ch`, given the reader's language.
///
/// `language` is a BCP 47 tag (`navigator.language`), which decides the
/// regional face for the unified Han characters exactly as Flutter's
/// `FontFallbackManager` does.
pub fn fallback_family(ch: char, language: &str) -> &'static str {
    let code = ch as u32;
    if is_emoji(code) {
        return "Noto Color Emoji";
    }
    if let Some(family) = regional_family(code, language) {
        return family;
    }
    if covers_roboto(code) {
        return ROBOTO;
    }
    script_family(code).unwrap_or("Noto Sans")
}

/// Han, kana and hangul: the character does not say which regional face draws
/// it, so the reader's language does (Flutter's `_kLanguageFontPreferences`,
/// falling through to its `Noto Sans SC` tie-break when nothing matches).
fn regional_family(code: u32, language: &str) -> Option<&'static str> {
    if is_hangul(code) {
        return Some("Noto Sans KR");
    }
    if is_kana(code) {
        return Some("Noto Sans JP");
    }
    if !is_han(code) {
        return None;
    }
    Some(han_family(language))
}

fn han_family(language: &str) -> &'static str {
    let exact = match language {
        "zh-Hant" | "zh-TW" | "zh-MO" => Some("Noto Sans TC"),
        "zh-HK" => Some("Noto Sans HK"),
        "ja" => Some("Noto Sans JP"),
        "ko" => Some("Noto Sans KR"),
        "zh" | "zh-Hans" | "zh-CN" => Some("Noto Sans SC"),
        _ => None,
    };
    if let Some(family) = exact {
        return family;
    }
    match language.split('-').next() {
        Some("ja") => "Noto Sans JP",
        Some("ko") => "Noto Sans KR",
        _ => "Noto Sans SC",
    }
}

fn covers_roboto(code: u32) -> bool {
    ROBOTO_RANGES
        .binary_search_by(|(start, end)| range_order(*start, *end, code))
        .is_ok()
}

fn script_family(code: u32) -> Option<&'static str> {
    SCRIPT_FAMILIES
        .binary_search_by(|(start, end, _)| range_order(*start, *end, code))
        .ok()
        .map(|at| SCRIPT_FAMILIES[at].2)
}

fn range_order(start: u32, end: u32, code: u32) -> std::cmp::Ordering {
    if end < code {
        std::cmp::Ordering::Less
    } else if start > code {
        std::cmp::Ordering::Greater
    } else {
        std::cmp::Ordering::Equal
    }
}

fn is_hangul(code: u32) -> bool {
    matches!(
        code,
        0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xAC00..=0xD7FF
    )
}

fn is_kana(code: u32) -> bool {
    matches!(code, 0x3040..=0x30FF | 0x31F0..=0x31FF | 0xFF65..=0xFF9F)
}

fn is_han(code: u32) -> bool {
    matches!(
        code,
        0x2E80..=0x2FFF
            | 0x3000..=0x303F
            | 0x31C0..=0x31EF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x20000..=0x2FA1F
    )
}

fn is_emoji(code: u32) -> bool {
    matches!(
        code,
        0x200D | 0xFE0F | 0x2600..=0x27BF | 0x1F1E6..=0x1F1FF | 0x1F300..=0x1FAFF
    )
}

/// The characters Roboto draws itself, so they are asked of Roboto rather than
/// of a Noto family. Taken from the family's own character map.
#[rustfmt::skip]
const ROBOTO_RANGES: &[(u32, u32)] = &[
    (0x0000, 0x000D),
    (0x0020, 0x007E),
    (0x00A0, 0x01B0),
    (0x01F0, 0x01FF),
    (0x0218, 0x021B),
    (0x02BC, 0x02DD),
    (0x02F3, 0x030F),
    (0x0384, 0x03D6),
    (0x0400, 0x0513),
    (0x1E80, 0x1E85),
    (0x1E9E, 0x1EF9),
    (0x2000, 0x2044),
    (0x2070, 0x208E),
    (0x20A3, 0x20C1),
    (0x2105, 0x212E),
    (0x215B, 0x215E),
    (0x2202, 0x222B),
    (0x2260, 0x2265),
    (0x25CA, 0x25CF),
    (0xFB01, 0xFB04),
];

/// One family per Unicode script, reduced from Flutter's generated coverage
/// data. Han, kana, hangul and emoji are absent: those are chosen above.
#[rustfmt::skip]
const SCRIPT_FAMILIES: &[(u32, u32, &str)] = &[
    (0x0180, 0x02E4, "Noto Sans"),
    (0x0370, 0x037F, "Noto Sans"),
    (0x03CF, 0x03E1, "Noto Sans"),
    (0x03E2, 0x03EF, "Noto Sans Coptic"),
    (0x03F0, 0x03FF, "Noto Sans"),
    (0x0514, 0x052F, "Noto Sans"),
    (0x0531, 0x058F, "Noto Sans Armenian"),
    (0x0591, 0x05F4, "Noto Sans Hebrew"),
    (0x0600, 0x06FF, "Noto Sans Arabic"),
    (0x0700, 0x074F, "Noto Sans Syriac"),
    (0x0750, 0x077F, "Noto Sans Arabic"),
    (0x0780, 0x07B1, "Noto Sans Thaana"),
    (0x07C0, 0x07FF, "Noto Sans NKo"),
    (0x0840, 0x085E, "Noto Sans Mandaic"),
    (0x0870, 0x08FF, "Noto Sans Arabic"),
    (0x0900, 0x097F, "Noto Sans Devanagari"),
    (0x0980, 0x09FE, "Noto Sans Bengali"),
    (0x0A01, 0x0A76, "Noto Sans Gurmukhi"),
    (0x0A81, 0x0AFF, "Noto Sans Gujarati"),
    (0x0B01, 0x0B77, "Noto Sans Oriya"),
    (0x0B82, 0x0BFA, "Noto Sans Tamil"),
    (0x0C00, 0x0C7F, "Noto Sans Telugu"),
    (0x0C80, 0x0CF3, "Noto Sans Kannada"),
    (0x0D00, 0x0D7F, "Noto Sans Malayalam"),
    (0x0D81, 0x0DF4, "Noto Sans Sinhala"),
    (0x0E01, 0x0E5B, "Noto Sans Thai"),
    (0x0E81, 0x0EDF, "Noto Sans Lao"),
    (0x0F00, 0x0FDA, "Noto Serif Tibetan"),
    (0x1000, 0x109F, "Noto Sans Myanmar"),
    (0x10A0, 0x10FF, "Noto Sans Georgian"),
    (0x1200, 0x1399, "Noto Sans Ethiopic"),
    (0x13A0, 0x13FD, "Noto Sans Cherokee"),
    (0x1400, 0x167F, "Noto Sans Canadian Aboriginal"),
    (0x1680, 0x169C, "Noto Sans Ogham"),
    (0x16A0, 0x16F8, "Noto Sans Runic"),
    (0x1700, 0x171F, "Noto Sans Tagalog"),
    (0x1720, 0x1734, "Noto Sans Hanunoo"),
    (0x1740, 0x1753, "Noto Sans Buhid"),
    (0x1760, 0x1773, "Noto Sans Tagbanwa"),
    (0x1780, 0x17F9, "Noto Sans Khmer"),
    (0x1800, 0x18AA, "Noto Sans Mongolian"),
    (0x18B0, 0x18F5, "Noto Sans Canadian Aboriginal"),
    (0x1900, 0x194F, "Noto Sans Limbu"),
    (0x1950, 0x1974, "Noto Sans Tai Le"),
    (0x1980, 0x19DF, "Noto Sans New Tai Lue"),
    (0x19E0, 0x19FF, "Noto Sans Khmer"),
    (0x1A00, 0x1A1F, "Noto Sans Buginese"),
    (0x1A20, 0x1AAD, "Noto Sans Tai Tham"),
    (0x1B00, 0x1B7C, "Noto Sans Balinese"),
    (0x1B80, 0x1BBF, "Noto Sans Sundanese"),
    (0x1BC0, 0x1BFF, "Noto Sans Batak"),
    (0x1C00, 0x1C4F, "Noto Sans Lepcha"),
    (0x1C50, 0x1C7F, "Noto Sans Ol Chiki"),
    (0x1C80, 0x1C88, "Noto Sans"),
    (0x1C90, 0x1CBF, "Noto Sans Georgian"),
    (0x1CC0, 0x1CC7, "Noto Sans Sundanese"),
    (0x1D00, 0x1DBF, "Noto Sans"),
    (0x1E02, 0x1E9F, "Noto Sans"),
    (0x1EFA, 0x1FFE, "Noto Sans"),
    (0x2071, 0x209C, "Noto Sans"),
    (0x212A, 0x2188, "Noto Sans"),
    (0x2800, 0x28FF, "Noto Sans Symbols 2"),
    (0x2C00, 0x2C5E, "Noto Sans Glagolitic"),
    (0x2C60, 0x2C7F, "Noto Sans"),
    (0x2C80, 0x2CFF, "Noto Sans Coptic"),
    (0x2D00, 0x2D2D, "Noto Sans Georgian"),
    (0x2D30, 0x2D7F, "Noto Sans Tifinagh"),
    (0x2D80, 0x2DDE, "Noto Sans Ethiopic"),
    (0x2DE0, 0x2DFF, "Noto Sans"),
    (0xA000, 0xA4C6, "Noto Sans Yi"),
    (0xA4D0, 0xA4FF, "Noto Sans Lisu"),
    (0xA500, 0xA62B, "Noto Sans Vai"),
    (0xA640, 0xA69F, "Noto Sans"),
    (0xA6A0, 0xA6F7, "Noto Sans Bamum"),
    (0xA722, 0xA7FF, "Noto Sans"),
    (0xA800, 0xA82C, "Noto Sans Syloti Nagri"),
    (0xA840, 0xA877, "Noto Sans PhagsPa"),
    (0xA880, 0xA8D9, "Noto Sans Saurashtra"),
    (0xA8E0, 0xA8FF, "Noto Sans Devanagari"),
    (0xA900, 0xA92F, "Noto Sans Kayah Li"),
    (0xA930, 0xA95F, "Noto Sans Rejang"),
    (0xA980, 0xA9DF, "Noto Sans Javanese"),
    (0xA9E0, 0xA9FE, "Noto Sans Myanmar"),
    (0xAA00, 0xAA5F, "Noto Sans Cham"),
    (0xAA60, 0xAA7F, "Noto Sans Myanmar"),
    (0xAA80, 0xAADF, "Noto Sans Tai Viet"),
    (0xAAE0, 0xAAF6, "Noto Sans Meetei Mayek"),
    (0xAB01, 0xAB2E, "Noto Sans Ethiopic"),
    (0xAB30, 0xAB69, "Noto Sans"),
    (0xAB70, 0xABBF, "Noto Sans Cherokee"),
    (0xABC0, 0xABF9, "Noto Sans Meetei Mayek"),
    (0xFB1D, 0xFB4F, "Noto Sans Hebrew"),
    (0xFB50, 0xFDFF, "Noto Sans Arabic"),
    (0xFE70, 0xFEFC, "Noto Sans Arabic"),
    (0xFF21, 0xFF5A, "Noto Sans"),
    (0x10000, 0x100FA, "Noto Sans Linear B"),
    (0x10140, 0x101A0, "Noto Sans"),
    (0x10280, 0x1029C, "Noto Sans Lycian"),
    (0x102A0, 0x102D0, "Noto Sans Carian"),
    (0x10300, 0x1032F, "Noto Sans Old Italic"),
    (0x10330, 0x1034A, "Noto Sans Gothic"),
    (0x10350, 0x1037A, "Noto Sans Old Permic"),
    (0x10380, 0x1039F, "Noto Sans Ugaritic"),
    (0x103A0, 0x103D5, "Noto Sans Old Persian"),
    (0x10400, 0x1044F, "Noto Sans Deseret"),
    (0x10450, 0x1047F, "Noto Sans Shavian"),
    (0x10480, 0x104A9, "Noto Sans Osmanya"),
    (0x104B0, 0x104FB, "Noto Sans Osage"),
    (0x10500, 0x10527, "Noto Sans Elbasan"),
    (0x10530, 0x1056F, "Noto Sans Caucasian Albanian"),
    (0x10600, 0x10767, "Noto Sans Linear A"),
    (0x10780, 0x107BA, "Noto Sans"),
    (0x10800, 0x1083F, "Noto Sans Cypriot"),
    (0x10840, 0x1085F, "Noto Sans Imperial Aramaic"),
    (0x10860, 0x1087F, "Noto Sans Palmyrene"),
    (0x10880, 0x108AF, "Noto Sans Nabataean"),
    (0x108E0, 0x108FF, "Noto Sans Hatran"),
    (0x10900, 0x1091F, "Noto Sans Phoenician"),
    (0x10920, 0x1093F, "Noto Sans Lydian"),
    (0x10980, 0x109FF, "Noto Sans Meroitic"),
    (0x10A00, 0x10A58, "Noto Sans Kharoshthi"),
    (0x10A60, 0x10A7F, "Noto Sans Old South Arabian"),
    (0x10A80, 0x10A9F, "Noto Sans Old North Arabian"),
    (0x10AC0, 0x10AF6, "Noto Sans Manichaean"),
    (0x10B00, 0x10B3F, "Noto Sans Avestan"),
    (0x10B40, 0x10B5F, "Noto Sans Inscriptional Parthian"),
    (0x10B60, 0x10B7F, "Noto Sans Inscriptional Pahlavi"),
    (0x10B80, 0x10BAF, "Noto Sans Psalter Pahlavi"),
    (0x10C00, 0x10C48, "Noto Sans Old Turkic"),
    (0x10C80, 0x10CFF, "Noto Sans Old Hungarian"),
    (0x10E60, 0x10E7E, "Noto Sans Arabic"),
    (0x10F00, 0x10F27, "Noto Sans Old Sogdian"),
    (0x10F30, 0x10F59, "Noto Sans Sogdian"),
    (0x10FE0, 0x10FF6, "Noto Sans Elymaic"),
    (0x11000, 0x1107F, "Noto Sans Brahmi"),
    (0x11080, 0x110CD, "Noto Sans Kaithi"),
    (0x110D0, 0x110F9, "Noto Sans Sora Sompeng"),
    (0x11100, 0x11147, "Noto Sans Chakma"),
    (0x11150, 0x11176, "Noto Sans Mahajani"),
    (0x11180, 0x111DF, "Noto Sans Sharada"),
    (0x111E1, 0x111F4, "Noto Sans Sinhala"),
    (0x11200, 0x11240, "Noto Sans Khojki"),
    (0x11280, 0x112A9, "Noto Sans Multani"),
    (0x112B0, 0x112F9, "Noto Sans Khudawadi"),
    (0x11300, 0x11374, "Noto Sans Grantha"),
    (0x11400, 0x11461, "Noto Sans Newa"),
    (0x11480, 0x114D9, "Noto Sans Tirhuta"),
    (0x11580, 0x115DD, "Noto Sans Siddham"),
    (0x11600, 0x11659, "Noto Sans Modi"),
    (0x11660, 0x1166C, "Noto Sans Mongolian"),
    (0x11680, 0x116C9, "Noto Sans Takri"),
    (0x118A0, 0x118FF, "Noto Sans Warang Citi"),
    (0x11A00, 0x11A47, "Noto Sans Zanabazar Square"),
    (0x11A50, 0x11AA2, "Noto Sans Soyombo"),
    (0x11AB0, 0x11ABF, "Noto Sans Canadian Aboriginal"),
    (0x11AC0, 0x11AF8, "Noto Sans Pau Cin Hau"),
    (0x11B00, 0x11B09, "Noto Sans Devanagari"),
    (0x11C00, 0x11C6C, "Noto Sans Bhaiksuki"),
    (0x11C70, 0x11CB6, "Noto Sans Marchen"),
    (0x11D00, 0x11D59, "Noto Sans Masaram Gondi"),
    (0x11D60, 0x11DA9, "Noto Sans Gunjala Gondi"),
    (0x11FC0, 0x11FFF, "Noto Sans Tamil"),
    (0x12000, 0x12399, "Noto Sans Cuneiform"),
    (0x12400, 0x12543, "Noto Sans Cuneiform"),
    (0x13000, 0x13446, "Noto Sans Egyptian Hieroglyphs"),
    (0x14400, 0x14646, "Noto Sans Anatolian Hieroglyphs"),
    (0x16800, 0x16A38, "Noto Sans Bamum"),
    (0x16A40, 0x16A6F, "Noto Sans Mro"),
    (0x16AD0, 0x16AF5, "Noto Sans Bassa Vah"),
    (0x16B00, 0x16B8F, "Noto Sans Pahawh Hmong"),
    (0x16E40, 0x16E9A, "Noto Sans Medefaidrin"),
    (0x16F00, 0x16F9F, "Noto Sans Miao"),
    (0x1B170, 0x1B2FB, "Noto Sans Nushu"),
    (0x1BC00, 0x1BC9F, "Noto Sans Duployan"),
    (0x1D200, 0x1D245, "Noto Sans"),
    (0x1DF00, 0x1DF1E, "Noto Sans"),
    (0x1E000, 0x1E02A, "Noto Sans Glagolitic"),
    (0x1E2C0, 0x1E2FF, "Noto Sans Wancho"),
    (0x1E7E0, 0x1E7FE, "Noto Sans Ethiopic"),
    (0x1E900, 0x1E95F, "Noto Sans Adlam"),
    (0x1EE00, 0x1EEF1, "Noto Sans Arabic"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_script_asks_for_its_own_noto_family() {
        // The bug this table replaces: everything outside a handful of
        // hand-written buckets asked for "Noto Sans", whose subset came back
        // without the glyphs, and the text stayed blank for good.
        for (ch, family) in [
            ('ש', "Noto Sans Hebrew"),
            ('հ', "Noto Sans Armenian"),
            ('გ', "Noto Sans Georgian"),
            ('न', "Noto Sans Devanagari"),
            ('ก', "Noto Sans Thai"),
            ('ا', "Noto Sans Arabic"),
            ('ܐ', "Noto Sans Syriac"),
            ('አ', "Noto Sans Ethiopic"),
            ('ಕ', "Noto Sans Kannada"),
            ('ᏸ', "Noto Sans Cherokee"),
        ] {
            assert_eq!(fallback_family(ch, "en-US"), family, "for {ch:?}");
        }
    }

    #[test]
    fn roboto_answers_for_the_scripts_it_carries() {
        // Latin beyond the bundled slice, Greek and Cyrillic are Roboto's own:
        // sending them to Noto would change typeface mid-paragraph.
        for ch in ['Ā', 'Ž', 'ά', 'Д', 'я', 'ế'] {
            assert_eq!(fallback_family(ch, "en-US"), ROBOTO, "for {ch:?}");
        }
    }

    #[test]
    fn han_follows_the_readers_language() {
        assert_eq!(fallback_family('字', "ja"), "Noto Sans JP");
        assert_eq!(fallback_family('字', "ja-JP"), "Noto Sans JP");
        assert_eq!(fallback_family('字', "ko-KR"), "Noto Sans KR");
        assert_eq!(fallback_family('字', "zh-Hant"), "Noto Sans TC");
        assert_eq!(fallback_family('字', "zh-HK"), "Noto Sans HK");
        assert_eq!(fallback_family('字', "zh-CN"), "Noto Sans SC");
        // Flutter's tie-break when no preference matches.
        assert_eq!(fallback_family('字', "en-US"), "Noto Sans SC");
    }

    #[test]
    fn kana_and_hangul_do_not_wait_on_a_language() {
        assert_eq!(fallback_family('あ', "en-US"), "Noto Sans JP");
        assert_eq!(fallback_family('한', "en-US"), "Noto Sans KR");
    }

    #[test]
    fn emoji_come_before_any_text_family() {
        assert_eq!(fallback_family('🍶', "en-US"), "Noto Color Emoji");
        assert_eq!(fallback_family('☀', "en-US"), "Noto Color Emoji");
    }

    #[test]
    fn ranges_are_sorted_and_disjoint() {
        // Both tables are binary-searched, so unsorted or overlapping entries
        // would silently resolve to the wrong family.
        for pair in ROBOTO_RANGES.windows(2) {
            assert!(pair[0].1 < pair[1].0, "{pair:?}");
        }
        for pair in SCRIPT_FAMILIES.windows(2) {
            assert!(pair[0].1 < pair[1].0, "{:?}", (pair[0], pair[1]));
        }
    }
}
