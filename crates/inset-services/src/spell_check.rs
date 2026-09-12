//! Flutter counterpart: `services/spell_check.dart`.

use inset_embedder::{Locale, TextRange};
use inset_foundation::App;

/// A data structure representing a range of misspelled text and the suggested
/// replacements for this range.
///
/// For example, one [`SuggestionSpan`] of the
/// [`Vec<SuggestionSpan>`] suggestions of the [`SpellCheckResults`] corresponding
/// to "Hello, wrold!" may be:
///
/// ```
/// # use inset_embedder::TextRange;
/// # use inset_services::SuggestionSpan;
/// let suggestion_span = SuggestionSpan::new(
///     TextRange::new(7, 12),
///     vec!["word".into(), "world".into(), "old".into()],
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SuggestionSpan {
    /// The misspelled range of text.
    pub range: TextRange,
    /// The alternate suggestions for the misspelled range of text.
    pub suggestions: Vec<String>,
}

impl SuggestionSpan {
    /// Creates a span representing a misspelled range of text and the replacements
    /// suggested by a spell checker.
    pub fn new(range: TextRange, suggestions: Vec<String>) -> SuggestionSpan {
        SuggestionSpan { range, suggestions }
    }
}

impl std::fmt::Display for SuggestionSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SuggestionSpan(range: {:?}, suggestions: {:?})",
            self.range, self.suggestions
        )
    }
}

/// A data structure grouping together the [`SuggestionSpan`]s and related text of
/// results returned by a spell checker.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpellCheckResults {
    /// The text that the [`suggestion_spans`](Self::suggestion_spans) correspond to.
    pub spell_checked_text: String,
    /// The spell check results of the [`spell_checked_text`](Self::spell_checked_text).
    ///
    /// See also:
    ///
    ///  * [`SuggestionSpan`], the ranges of misspelled text and corresponding
    ///    replacement suggestions.
    pub suggestion_spans: Vec<SuggestionSpan>,
}

impl SpellCheckResults {
    /// Creates results based off those received by spell checking some text input.
    pub fn new(
        spell_checked_text: impl Into<String>,
        suggestion_spans: Vec<SuggestionSpan>,
    ) -> SpellCheckResults {
        SpellCheckResults {
            spell_checked_text: spell_checked_text.into(),
            suggestion_spans,
        }
    }
}

impl std::fmt::Display for SpellCheckResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SpellCheckResults(spellCheckText: {}, suggestionSpans: {:?})",
            self.spell_checked_text, self.suggestion_spans
        )
    }
}

/// Determines how spell check results are received for text input.
pub trait SpellCheckService {
    /// Facilitates a spell check request.
    ///
    /// Returns a [`Vec`] of [`SuggestionSpan`]s for all misspelled words in the
    /// given [`str`] for the given [`Locale`].
    ///
    /// A return value of [`None`] indicates that fetching the spell check
    /// suggestions was unsuccessful. If fetching the suggestions succeeded but
    /// none were found, the result is an empty list.
    fn fetch_spell_check_suggestions(
        &self,
        app: &App,
        locale: &Locale,
        text: &str,
    ) -> Option<Vec<SuggestionSpan>>;
}

/// The service used by default to fetch spell check results for text input.
///
/// Any widget may use this service to spell check text by calling
/// [`fetch_spell_check_suggestions`](Self::fetch_spell_check_suggestions) with an
/// instance of this class. This is currently only supported by Android and iOS.
///
/// See also:
///
///  * [`SpellCheckService`], the service that this implements that may be
///    overridden for use by `EditableText`.
///  * `EditableText`, which may use this service to fetch results.
#[derive(Clone, Debug, Default)]
pub struct DefaultSpellCheckService {
    /// The last received results from the shell side.
    pub last_saved_results: Option<SpellCheckResults>,
}

impl DefaultSpellCheckService {
    /// Creates service to spell check text input by default.
    pub fn new() -> DefaultSpellCheckService {
        DefaultSpellCheckService::default()
    }

    /// Merges two lists of spell check [`SuggestionSpan`]s.
    ///
    /// Used in cases where the text has not changed, but the spell check results
    /// received from the shell side have. This case is caused by IMEs (GBoard,
    /// for instance) that ignore the composing region when spell checking text.
    ///
    /// Assumes that the lists provided as parameters are sorted by range start
    /// and that both list of [`SuggestionSpan`]s apply to the same text.
    pub fn merge_results(
        old_results: &[SuggestionSpan],
        new_results: &[SuggestionSpan],
    ) -> Vec<SuggestionSpan> {
        let mut merged_results = Vec::new();

        let mut old_span_pointer = 0;
        let mut new_span_pointer = 0;

        while old_span_pointer < old_results.len() && new_span_pointer < new_results.len() {
            let old_span = &old_results[old_span_pointer];
            let new_span = &new_results[new_span_pointer];

            if old_span.range.start == new_span.range.start {
                merged_results.push(old_span.clone());
                old_span_pointer += 1;
                new_span_pointer += 1;
            } else if old_span.range.start < new_span.range.start {
                merged_results.push(old_span.clone());
                old_span_pointer += 1;
            } else {
                merged_results.push(new_span.clone());
                new_span_pointer += 1;
            }
        }

        merged_results.extend_from_slice(&old_results[old_span_pointer..]);
        merged_results.extend_from_slice(&new_results[new_span_pointer..]);

        merged_results
    }
}

impl SpellCheckService for DefaultSpellCheckService {
    fn fetch_spell_check_suggestions(
        &self,
        app: &App,
        locale: &Locale,
        text: &str,
    ) -> Option<Vec<SuggestionSpan>> {
        let _ = (app, locale, text);
        None
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;

    use super::*;

    #[test]
    fn suggestion_span_equality() {
        let span = SuggestionSpan::new(
            TextRange::new(7, 12),
            vec!["word".into(), "world".into(), "old".into()],
        );
        assert_eq!(
            span,
            SuggestionSpan::new(
                TextRange::new(7, 12),
                vec!["word".into(), "world".into(), "old".into()],
            )
        );
        assert_ne!(
            span,
            SuggestionSpan::new(TextRange::new(7, 12), vec!["world".into()])
        );
    }

    #[test]
    fn spell_check_results_equality() {
        let span = SuggestionSpan::new(TextRange::new(7, 12), vec!["world".into()]);
        let results = SpellCheckResults::new("Hello, wrold!", vec![span.clone()]);
        assert_eq!(results, SpellCheckResults::new("Hello, wrold!", vec![span]));
        assert_ne!(results, SpellCheckResults::new("Hello, world!", Vec::new()));
    }

    #[test]
    fn merge_results_keeps_old_span_on_same_start() {
        let old = vec![SuggestionSpan::new(
            TextRange::new(0, 4),
            vec!["old".into()],
        )];
        let new = vec![SuggestionSpan::new(
            TextRange::new(0, 4),
            vec!["new".into()],
        )];
        let merged = DefaultSpellCheckService::merge_results(&old, &new);
        assert_eq!(merged, old);
    }

    #[test]
    fn default_service_is_unsupported() {
        let cell = AppCell::new();
        let app = cell.borrow_mut();
        let service = DefaultSpellCheckService::new();
        assert!(
            service
                .fetch_spell_check_suggestions(&app, &Locale::new("en"), "wrold")
                .is_none()
        );
    }
}
