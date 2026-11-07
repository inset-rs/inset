//! Flutter counterpart: `services/text_layout_metrics.dart`.
//!
//! The instance methods (`getLineAtOffset` and friends) live on `RenderEditable`;
//! they need the arena. The statics stay here.

/// Static classifiers from Flutter's `TextLayoutMetrics`.
pub struct TextLayoutMetrics;

impl TextLayoutMetrics {
    // TODO(gspencergoog): replace when we expose this ICU information.
    /// Check if the given code unit is a white space or separator
    /// character.
    ///
    /// Includes newline characters from ASCII and separators from the
    /// [unicode separator category](https://www.compart.com/en/unicode/category/Zs)
    pub fn is_whitespace(code_unit: i32) -> bool {
        matches!(
            code_unit,
            0x9 | // horizontal tab
            0xA | // line feed
            0xB | // vertical tab
            0xC | // form feed
            0xD | // carriage return
            0x1C | // file separator
            0x1D | // group separator
            0x1E | // record separator
            0x1F | // unit separator
            0x20 | // space
            0xA0 | // no-break space
            0x1680 | // ogham space mark
            0x2000 | // en quad
            0x2001 | // em quad
            0x2002 | // en space
            0x2003 | // em space
            0x2004 | // three-per-em space
            0x2005 | // four-er-em space
            0x2006 | // six-per-em space
            0x2007 | // figure space
            0x2008 | // punctuation space
            0x2009 | // thin space
            0x200A | // hair space
            0x202F | // narrow no-break space
            0x205F | // medium mathematical space
            0x3000 // ideographic space
        )
    }

    /// Check if the given code unit is a line terminator character.
    ///
    /// Includes newline characters from ASCII
    /// (https://www.unicode.org/standard/reports/tr13/tr13-5.html).
    pub fn is_line_terminator(code_unit: i32) -> bool {
        matches!(
            code_unit,
            0x0A | // line feed
            0x0B | // vertical feed
            0x0C | // form feed
            0x0D | // carriage return
            0x85 | // new line
            0x2028 | // line separator
            0x2029 // paragraph separator
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_and_line_terminators_match_dart() {
        assert!(TextLayoutMetrics::is_whitespace(0x20));
        assert!(TextLayoutMetrics::is_whitespace(0xA));
        assert!(!TextLayoutMetrics::is_whitespace(b'a' as i32));
        assert!(TextLayoutMetrics::is_line_terminator(0x0A));
        assert!(TextLayoutMetrics::is_line_terminator(0x2028));
        assert!(!TextLayoutMetrics::is_line_terminator(0x20));
    }
}
