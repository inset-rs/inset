//! Flutter counterpart: `services/text_boundary.dart`.
//!
//! `LineBoundary` lives in rendering, next to the `RenderEditable` whose line metrics it
//! reads; see that crate's `PORTING.md`.

use reveal_embedder::TextRange;
use reveal_foundation::App;
use unicode_segmentation::UnicodeSegmentation;

use crate::text_layout_metrics::TextLayoutMetrics;

/// Signature for a predicate that takes an offset into a UTF-16 string, and a
/// boolean that indicates the search direction.
pub type UntilPredicate = dyn Fn(i32, bool) -> bool;

/// An interface for retrieving the logical text boundary (as opposed to the
/// visual boundary) at a given code unit offset in a document.
///
/// Either the [`get_text_boundary_at`](Self::get_text_boundary_at) method, or both the
/// [`get_leading_text_boundary_at`](Self::get_leading_text_boundary_at) method and the
/// [`get_trailing_text_boundary_at`](Self::get_trailing_text_boundary_at) method
/// must be implemented.
pub trait TextBoundary {
    /// Returns the offset of the closest text boundary before or at the given
    /// `position`, or `None` if no boundaries can be found.
    ///
    /// The return value, if not `None`, is usually less than or equal to `position`.
    ///
    /// The range of the return value is given by the closed interval
    /// `[0, string.length]`.
    fn get_leading_text_boundary_at(&self, app: &mut App, position: i32) -> Option<i32> {
        if position < 0 {
            return None;
        }
        let start = self.get_text_boundary_at(app, position).start;
        (start >= 0).then_some(start)
    }

    /// Returns the offset of the closest text boundary after the given
    /// `position`, or `None` if there is no boundary can be found after `position`.
    ///
    /// The return value, if not `None`, is usually greater than `position`.
    ///
    /// The range of the return value is given by the closed interval
    /// `[0, string.length]`.
    fn get_trailing_text_boundary_at(&self, app: &mut App, position: i32) -> Option<i32> {
        let end = self.get_text_boundary_at(app, position.max(0)).end;
        (end >= 0).then_some(end)
    }

    /// Returns the text boundary range that encloses the input position.
    ///
    /// The returned [`TextRange`] may contain `-1`, which indicates no boundaries
    /// can be found in that direction.
    fn get_text_boundary_at(&self, app: &mut App, position: i32) -> TextRange {
        let start = self
            .get_leading_text_boundary_at(app, position)
            .unwrap_or(-1);
        let end = self
            .get_trailing_text_boundary_at(app, position)
            .unwrap_or(-1);
        TextRange::new(start, end)
    }
}

/// A [`TextBoundary`] subclass for retrieving the range of the grapheme the given
/// `position` is in.
///
/// The class is implemented using the `unicode-segmentation` crate, which supplies the
/// same UAX #29 grapheme clusters as the `characters` package Dart uses.
#[derive(Clone, Debug)]
pub struct CharacterBoundary {
    text: String,
}

impl CharacterBoundary {
    /// Creates a [`CharacterBoundary`] with the text.
    pub fn new(text: impl Into<String>) -> CharacterBoundary {
        CharacterBoundary { text: text.into() }
    }
}

impl TextBoundary for CharacterBoundary {
    fn get_leading_text_boundary_at(&self, _app: &mut App, position: i32) -> Option<i32> {
        if position < 0 {
            return None;
        }
        let grapheme_start = CharacterRange::at(&self.text, position.min(utf16_len(&self.text)))
            .string_before_length;
        debug_assert!(CharacterRange::at(&self.text, grapheme_start).is_empty());
        Some(grapheme_start)
    }

    fn get_trailing_text_boundary_at(&self, _app: &mut App, position: i32) -> Option<i32> {
        if position >= utf16_len(&self.text) {
            return None;
        }
        let range_at_position = CharacterRange::at(&self.text, (position + 1).max(0));
        let next_boundary =
            range_at_position.string_before_length + range_at_position.current_length;
        debug_assert!(
            next_boundary == utf16_len(&self.text)
                || CharacterRange::at(&self.text, next_boundary).is_empty()
        );
        Some(next_boundary)
    }

    fn get_text_boundary_at(&self, app: &mut App, position: i32) -> TextRange {
        if position < 0 {
            return TextRange::new(
                -1,
                self.get_trailing_text_boundary_at(app, position)
                    .unwrap_or(-1),
            );
        } else if position >= utf16_len(&self.text) {
            return TextRange::new(
                self.get_leading_text_boundary_at(app, position)
                    .unwrap_or(-1),
                -1,
            );
        }
        let range_at_position = CharacterRange::at(&self.text, position);
        if range_at_position.is_empty() {
            // An empty range means `position` is a grapheme boundary.
            TextRange::new(
                range_at_position.string_before_length,
                self.get_trailing_text_boundary_at(app, position)
                    .unwrap_or(-1),
            )
        } else {
            TextRange::new(
                range_at_position.string_before_length,
                range_at_position.string_before_length + range_at_position.current_length,
            )
        }
    }
}

/// A text boundary that uses paragraphs as logical boundaries.
///
/// A paragraph is defined as the range between line terminators. If no
/// line terminators exist then the paragraph boundary is the entire document.
#[derive(Clone, Debug)]
pub struct ParagraphBoundary {
    /// Dart indexes its `String` by UTF-16 code unit, which is what the walks below need.
    text: Vec<u16>,
}

impl ParagraphBoundary {
    /// Creates a [`ParagraphBoundary`] with the text.
    pub fn new(text: &str) -> ParagraphBoundary {
        ParagraphBoundary {
            text: text.encode_utf16().collect(),
        }
    }

    fn code_unit_at(&self, index: i32) -> i32 {
        i32::from(self.text[index as usize])
    }

    fn len(&self) -> i32 {
        self.text.len() as i32
    }
}

impl TextBoundary for ParagraphBoundary {
    /// Returns the offset representing the start position of the paragraph that
    /// bounds the given `position`. The returned offset is the position of the code unit
    /// that follows the line terminator that encloses the desired paragraph.
    fn get_leading_text_boundary_at(&self, _app: &mut App, position: i32) -> Option<i32> {
        if position < 0 || self.text.is_empty() {
            return None;
        }

        if position >= self.len() {
            return Some(self.len());
        }

        if position == 0 {
            return Some(0);
        }

        let mut index = position;

        if index > 1 && self.code_unit_at(index) == 0x0A && self.code_unit_at(index - 1) == 0x0D {
            index -= 2;
        } else if TextLayoutMetrics::is_line_terminator(self.code_unit_at(index)) {
            index -= 1;
        }

        while index > 0 {
            if TextLayoutMetrics::is_line_terminator(self.code_unit_at(index)) {
                return Some(index + 1);
            }
            index -= 1;
        }

        Some(index.max(0))
    }

    /// Returns the offset representing the end position of the paragraph that
    /// bounds the given `position`. The returned offset is the position of the
    /// code unit representing the trailing line terminator that encloses the
    /// desired paragraph.
    fn get_trailing_text_boundary_at(&self, _app: &mut App, position: i32) -> Option<i32> {
        if position >= self.len() || self.text.is_empty() {
            return None;
        }

        if position < 0 {
            return Some(0);
        }

        let mut index = position;

        while !TextLayoutMetrics::is_line_terminator(self.code_unit_at(index)) {
            index += 1;
            if index == self.len() {
                return Some(index);
            }
        }

        Some(
            if index < self.len() - 1
                && self.code_unit_at(index) == 0x0D
                && self.code_unit_at(index + 1) == 0x0A
            {
                index + 2
            } else {
                index + 1
            },
        )
    }
}

/// A text boundary that uses the entire document as logical boundary.
#[derive(Clone, Debug)]
pub struct DocumentBoundary {
    text: String,
}

impl DocumentBoundary {
    /// Creates a [`DocumentBoundary`] with the text.
    pub fn new(text: impl Into<String>) -> DocumentBoundary {
        DocumentBoundary { text: text.into() }
    }
}

impl TextBoundary for DocumentBoundary {
    fn get_leading_text_boundary_at(&self, _app: &mut App, position: i32) -> Option<i32> {
        (position >= 0).then_some(0)
    }

    fn get_trailing_text_boundary_at(&self, _app: &mut App, position: i32) -> Option<i32> {
        let length = utf16_len(&self.text);
        (position < length).then_some(length)
    }
}

/// Dart `String.length`: UTF-16 code units.
fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

/// The grapheme cluster at a code unit offset, as the `characters` package's
/// `CharacterRange.at` reports it: empty when the offset is already on a cluster
/// boundary, and the whole cluster when the offset falls inside one.
struct CharacterRange {
    /// The number of code units before the range.
    string_before_length: i32,
    /// The code unit length of `current`, the cluster the range covers.
    current_length: i32,
}

impl CharacterRange {
    fn at(text: &str, position: i32) -> CharacterRange {
        let mut start = 0;
        for cluster in text.graphemes(true) {
            if position <= start {
                break;
            }
            let length = utf16_len(cluster);
            if position < start + length {
                return CharacterRange {
                    string_before_length: start,
                    current_length: length,
                };
            }
            start += length;
        }
        CharacterRange {
            string_before_length: start,
            current_length: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.current_length == 0
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::{App, AppCell};

    use super::{CharacterBoundary, DocumentBoundary, ParagraphBoundary, TextBoundary};

    /// The boundaries below never read the arena; `LineBoundary` in rendering does.
    fn leading(app: &mut App, boundary: &dyn TextBoundary, position: i32) -> Option<i32> {
        boundary.get_leading_text_boundary_at(app, position)
    }

    fn trailing(app: &mut App, boundary: &dyn TextBoundary, position: i32) -> Option<i32> {
        boundary.get_trailing_text_boundary_at(app, position)
    }

    /// Dart's `_hasConsistentTextRangeImplementationWithinRange`: the range at a position
    /// must agree with the two one-sided lookups.
    fn assert_consistent_within_range(app: &mut App, boundary: &dyn TextBoundary, length: i32) {
        for position in -1..=length {
            let range = boundary.get_text_boundary_at(app, position);
            assert_eq!(
                range.start,
                leading(app, boundary, position).unwrap_or(-1),
                "leading disagrees at {position}"
            );
            assert_eq!(
                range.end,
                trailing(app, boundary, position).unwrap_or(-1),
                "trailing disagrees at {position}"
            );
        }
    }

    #[test]
    fn character_boundary_works() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let boundary = CharacterBoundary::new("abc");
        assert_consistent_within_range(&mut app, &boundary, 3);

        assert_eq!(leading(&mut app, &boundary, -1), None);
        assert_eq!(trailing(&mut app, &boundary, -1), Some(0));

        assert_eq!(leading(&mut app, &boundary, 0), Some(0));
        assert_eq!(trailing(&mut app, &boundary, 0), Some(1));

        assert_eq!(leading(&mut app, &boundary, 1), Some(1));
        assert_eq!(trailing(&mut app, &boundary, 1), Some(2));

        assert_eq!(leading(&mut app, &boundary, 3), Some(3));
        assert_eq!(trailing(&mut app, &boundary, 3), None);

        assert_eq!(leading(&mut app, &boundary, 4), Some(3));
        assert_eq!(trailing(&mut app, &boundary, 4), None);
    }

    #[test]
    fn character_boundary_works_with_grapheme() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        // "a❄︎c": the snowflake carries a variation selector, so one grapheme spans two
        // code units.
        let text = "a\u{2744}\u{FE0E}c";
        let boundary = CharacterBoundary::new(text);
        let length = text.encode_utf16().count() as i32;
        assert_eq!(length, 4);
        assert_consistent_within_range(&mut app, &boundary, length);

        assert_eq!(leading(&mut app, &boundary, -1), None);
        assert_eq!(trailing(&mut app, &boundary, -1), Some(0));

        assert_eq!(leading(&mut app, &boundary, 0), Some(0));
        assert_eq!(trailing(&mut app, &boundary, 0), Some(1));

        assert_eq!(leading(&mut app, &boundary, 1), Some(1));
        assert_eq!(trailing(&mut app, &boundary, 1), Some(3));

        assert_eq!(leading(&mut app, &boundary, 2), Some(1));
        assert_eq!(trailing(&mut app, &boundary, 2), Some(3));

        assert_eq!(leading(&mut app, &boundary, 3), Some(3));
        assert_eq!(trailing(&mut app, &boundary, 3), Some(4));

        assert_eq!(leading(&mut app, &boundary, length), Some(length));
        assert_eq!(trailing(&mut app, &boundary, length), None);
    }

    #[test]
    fn paragraph_boundary_works_for_simple_cases() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        // Position enclosed inside of paragraph, "abcd efg h|i\n".
        let position = 10;

        let boundary = ParagraphBoundary::new("abcd efg hi\njklmno\npqrstuv");
        // The range includes the line terminator.
        assert_eq!(leading(&mut app, &boundary, position), Some(0));
        assert_eq!(trailing(&mut app, &boundary, position), Some(12));

        // This text includes a carriage return followed by a line feed.
        let boundary = ParagraphBoundary::new("abcd efg hi\r\njklmno\npqrstuv");
        assert_eq!(leading(&mut app, &boundary, position), Some(0));
        assert_eq!(trailing(&mut app, &boundary, position), Some(13));
    }

    #[test]
    fn paragraph_boundary_works_for_consecutive_line_terminators_involving_crlf() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let boundary = ParagraphBoundary::new(concat!(
            "Now is the time for\n", // 20
            "all good people\n\r\n", // 20 + 18 => 38
            "to come to the aid\n",  // 38 + 19 => 57
            "of their country.",     // 57 + 17 => 74
        ));
        assert_eq!(leading(&mut app, &boundary, 56), Some(38));
        assert_eq!(trailing(&mut app, &boundary, 56), Some(57));
        assert_eq!(leading(&mut app, &boundary, 38), Some(38));
        assert_eq!(trailing(&mut app, &boundary, 38), Some(57));
        assert_eq!(leading(&mut app, &boundary, 37), Some(36));
        assert_eq!(trailing(&mut app, &boundary, 37), Some(38));
    }

    #[test]
    fn paragraph_boundary_works_when_position_is_between_two_crlf() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let boundary = ParagraphBoundary::new("abcd efg hi\r\nhello\r\n\n");
        assert_eq!(leading(&mut app, &boundary, 16), Some(13));
        assert_eq!(trailing(&mut app, &boundary, 16), Some(20));
    }

    #[test]
    fn document_boundary_works() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let text = "abcd efg hi\njklmno\npqrstuv";
        let length = text.len() as i32;
        let boundary = DocumentBoundary::new(text);
        assert_consistent_within_range(&mut app, &boundary, length);

        assert_eq!(leading(&mut app, &boundary, -1), None);
        assert_eq!(trailing(&mut app, &boundary, -1), Some(length));

        assert_eq!(leading(&mut app, &boundary, 0), Some(0));
        assert_eq!(trailing(&mut app, &boundary, 0), Some(length));

        assert_eq!(leading(&mut app, &boundary, 10), Some(0));
        assert_eq!(trailing(&mut app, &boundary, 10), Some(length));

        assert_eq!(leading(&mut app, &boundary, length), Some(0));
        assert_eq!(trailing(&mut app, &boundary, length), None);

        assert_eq!(leading(&mut app, &boundary, length + 1), Some(0));
        assert_eq!(trailing(&mut app, &boundary, length + 1), None);
    }
}
