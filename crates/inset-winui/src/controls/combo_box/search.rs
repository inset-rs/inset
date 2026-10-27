//! ComboBox_Partial.cpp's text search: the type-ahead string a non-editable box collects
//! (`AppendCharToSearchString`, `HasSearchStringTimedOut`, `IsInSearchingMode`), the walk over the
//! items' text (`SearchItemSourceIndex`) and the comparisons it makes
//! (`StartsWithIgnoreLinguisticSemantics`, `AreStringsEqual`, `IsSearchStringValid`).

use std::time::Duration;

/// `HasSearchStringTimedOut`'s `timeOutInMilliseconds`: typing pauses longer than this start a
/// new search string.
const SEARCH_TIMEOUT: Duration = Duration::from_millis(1000);

/// `AppendCharToSearchString`'s `maxNumCharacters`: a character is appended only while the
/// string is no longer than this, in UTF-16 code units.
const MAX_SEARCH_LENGTH: usize = 256;

/// `m_searchString`, `m_timeSinceLastCharacterReceived` and `m_isInSearchingMode`: what a
/// non-editable box has been typed, and when.
#[derive(Clone, Debug, Default)]
pub(super) struct TypeAhead {
    /// `m_searchString`.
    pub(super) search: String,

    /// When the last character arrived; none before the first.
    last_character: Option<Duration>,

    /// `m_isInSearchingMode`.
    searching: bool,
}

impl TypeAhead {
    /// `HasSearchStringTimedOut`: more than a second since the last character, or no character
    /// yet.
    fn has_timed_out(&self, now: Duration) -> bool {
        self.last_character
            .is_none_or(|last| now.saturating_sub(last) > SEARCH_TIMEOUT)
    }

    /// `IsInSearchingMode`, without its `IsTextSearchEnabled` test: whether a character arrived
    /// within the last second, in which case Space is typed rather than opening the box.
    pub(super) fn is_searching(&mut self, now: Duration) -> bool {
        if self.has_timed_out(now) {
            self.searching = false;
        }
        self.searching
    }

    /// `AppendCharToSearchString`: appends `character`, after starting a new string when the
    /// last one timed out; returns whether it did.
    pub(super) fn append(&mut self, character: &str, now: Duration) -> bool {
        let created = self.has_timed_out(now);
        if created {
            self.search.clear();
        }
        self.last_character = Some(now);
        if self.search.encode_utf16().count() <= MAX_SEARCH_LENGTH {
            self.search.push_str(character);
        }
        self.searching = true;
        created
    }

    /// `ResetSearchString`.
    pub(super) fn reset(&mut self) {
        self.search.clear();
    }
}

/// How `SearchItemSourceIndex` compares an item's text with the search string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Match {
    /// `StartsWithIgnoreLinguisticSemantics`.
    Prefix,
    /// `AreStringsEqual`: `searchExactMatch`, and Backspace in an editable box.
    Exact,
}

/// `SearchItemSourceIndex`'s walk: from `start`, wrapping at the end, the first item whose text,
/// without leading spaces, matches `search`. An item without text ends the walk, as the source
/// stops where it cannot get a string from an item.
pub(super) fn find_item(
    texts: &[Option<&str>],
    search: &str,
    start: usize,
    rule: Match,
) -> Option<usize> {
    let count = texts.len();
    let mut index = start;
    for _ in 0..count {
        // An empty string is a null HSTRING, which is what the source tests for.
        let text = texts[index].filter(|text| !text.is_empty())?;
        let text = text.trim_start_matches(' ');
        let found = match rule {
            Match::Exact => are_strings_equal(text, search),
            Match::Prefix => starts_with_ignoring_linguistics(text, search),
        };
        if found {
            return Some(index);
        }
        index += 1;
        if index == count {
            index = 0;
        }
    }
    None
}

/// `SearchItemSourceIndex`'s first index for a non-editable box: the item after the selected
/// one, wrapping, when typing started a new string, else the selected item itself, or the
/// first.
pub(super) fn search_start(selected: Option<usize>, count: usize, created: bool) -> usize {
    match selected {
        Some(index) if created && index + 1 < count => index + 1,
        Some(_) if created => 0,
        None if created => 0,
        Some(index) => index,
        None => 0,
    }
}

/// `StartsWithIgnoreLinguisticSemantics`: whether `source` starts with `prefix` regardless of
/// case, of the full-width forms of Latin letters, digits and punctuation, and of hiragana
/// against katakana.
pub(super) fn starts_with_ignoring_linguistics(source: &str, prefix: &str) -> bool {
    if prefix.encode_utf16().count() > source.encode_utf16().count() {
        return false;
    }
    fold(source).starts_with(&fold(prefix))
}

/// `AreStringsEqual`: the same length, and equal but for the case of ASCII letters, as
/// `_wcsicmp` compares in the C locale.
pub(super) fn are_strings_equal(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// `IsSearchStringValid`: neither empty nor only spaces.
pub(super) fn is_search_string_valid(text: &str) -> bool {
    !text.trim_start_matches(' ').is_empty()
}

/// The text `starts_with_ignoring_linguistics` compares: lower case, full-width forms narrowed
/// and hiragana written as katakana.
fn fold(text: &str) -> String {
    text.chars()
        .map(|character| match character {
            // NORM_IGNOREWIDTH: the full-width forms of U+0021..U+007E, and the ideographic
            // space.
            '\u{FF01}'..='\u{FF5E}' => {
                char::from_u32(character as u32 - 0xFF01 + 0x21).unwrap_or(character)
            }
            '\u{3000}' => ' ',
            // NORM_IGNOREKANATYPE: hiragana as the katakana 0x60 code points later.
            '\u{3041}'..='\u{3096}' => char::from_u32(character as u32 + 0x60).unwrap_or(character),
            other => other,
        })
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_prefix_matches_regardless_of_case_width_and_kana() {
        assert!(starts_with_ignoring_linguistics("wAsHINGton", "Wa"));
        assert!(starts_with_ignoring_linguistics("Ｂlue", "bl"));
        assert!(starts_with_ignoring_linguistics("カタカナ", "かた"));
        assert!(!starts_with_ignoring_linguistics("Red", "Reds"));
        assert!(!starts_with_ignoring_linguistics("Green", "r"));
    }

    #[test]
    fn exact_matches_ignore_only_ascii_case() {
        assert!(are_strings_equal("Arial", "ARIAL"));
        assert!(!are_strings_equal("Arial", "Arial Black"));
        assert!(!are_strings_equal("É", "é"));
    }

    #[test]
    fn a_search_string_of_spaces_is_not_valid() {
        assert!(!is_search_string_valid(""));
        assert!(!is_search_string_valid("   "));
        assert!(is_search_string_valid(" a"));
    }

    #[test]
    fn the_walk_wraps_and_trims_leading_spaces() {
        let texts = [Some("Apple"), Some(" Banana"), Some("Avocado")];
        assert_eq!(find_item(&texts, "a", 1, Match::Prefix), Some(2));
        assert_eq!(find_item(&texts, "b", 2, Match::Prefix), Some(1));
        assert_eq!(find_item(&texts, "apple", 0, Match::Exact), Some(0));
        assert_eq!(find_item(&texts, "c", 0, Match::Prefix), None);
    }

    #[test]
    fn an_item_without_text_ends_the_walk() {
        let texts = [Some("Apple"), None, Some("Cherry")];
        assert_eq!(find_item(&texts, "c", 0, Match::Prefix), None);
        assert_eq!(find_item(&texts, "c", 2, Match::Prefix), Some(2));
        assert_eq!(find_item(&[Some("")], "a", 0, Match::Prefix), None);
    }

    #[test]
    fn a_new_string_starts_after_the_selection() {
        assert_eq!(search_start(Some(1), 3, true), 2);
        assert_eq!(search_start(Some(2), 3, true), 0);
        assert_eq!(search_start(Some(1), 3, false), 1);
        assert_eq!(search_start(None, 3, false), 0);
        assert_eq!(search_start(None, 3, true), 0);
    }

    #[test]
    fn characters_within_a_second_extend_the_string() {
        let mut type_ahead = TypeAhead::default();
        assert!(type_ahead.append("a", Duration::from_millis(5000)));
        assert!(!type_ahead.append("b", Duration::from_millis(5900)));
        assert_eq!(type_ahead.search, "ab");
        assert!(type_ahead.is_searching(Duration::from_millis(6800)));
        assert!(!type_ahead.is_searching(Duration::from_millis(7000)));
        assert!(type_ahead.append("c", Duration::from_millis(7000)));
        assert_eq!(type_ahead.search, "c");
    }
}
