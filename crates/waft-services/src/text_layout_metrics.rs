//! Flutter counterpart: `services/text_layout_metrics.dart`.
//!
//! The interface is implemented by the objects that own a laid-out paragraph. Waft's methods
//! take the [`App`] because reading a paragraph's layout does.

use waft_embedder::{TextPosition, TextRange, TextSelection};
use waft_foundation::App;

/// A read-only interface for accessing visual information about the implementing text.
///
/// The receiver is the object's copyable handle, as every arena receiver is: `RenderHandle` for
/// a render object, `Handle` for an arena object.
pub trait TextLayoutMetrics: Copy + 'static {
    /// Return a [`TextSelection`] containing the line of the given [`TextPosition`].
    fn get_line_at_offset(self, app: &mut App, position: TextPosition) -> TextSelection;

    /// Returns the word boundary around `position`.
    fn get_word_boundary(self, app: &mut App, position: TextPosition) -> TextRange;

    /// Returns the [`TextPosition`] above the given position.
    ///
    /// If the position is already on the first line, the given position is returned.
    fn get_text_position_above(self, app: &mut App, position: TextPosition) -> TextPosition;

    /// Returns the [`TextPosition`] below the given position.
    ///
    /// If the position is already on the last line, the given position is returned.
    fn get_text_position_below(self, app: &mut App, position: TextPosition) -> TextPosition;
}

// TODO(gspencergoog): replace when we expose this ICU information.
/// Check if the given code unit is a white space or separator character.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_and_line_terminators_match_dart() {
        assert!(is_whitespace(0x20));
        assert!(is_whitespace(0xA));
        assert!(!is_whitespace(b'a' as i32));
        assert!(is_line_terminator(0x0A));
        assert!(is_line_terminator(0x2028));
        assert!(!is_line_terminator(0x20));
    }
}
