//! Flutter counterpart: `widgets/spell_check.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_embedder::{Color, TargetPlatform, TextDecoration, TextRange};
use reveal_foundation::App;
use reveal_painting::{InlineSpanRef, TextSpan, TextStyle};
use reveal_services::{SpellCheckResults, SpellCheckService, SuggestionSpan, TextEditingValue};

use crate::widgets::editable_text::EditableTextContextMenuBuilder;

/// Controls how spell check is performed for text input.
///
/// This configuration determines the [`SpellCheckService`] used to fetch the
/// [`SuggestionSpan`] spell check results and the [`TextStyle`] used to
/// mark misspelled words within text input.
#[derive(Clone)]
pub struct SpellCheckConfiguration {
    /// The service used to fetch spell check results for text input.
    pub spell_check_service: Option<Rc<dyn SpellCheckService>>,
    /// The color the paint the selection highlight when spell check is showing
    /// suggestions for a misspelled word.
    ///
    /// For example, on iOS, the selection appears red while the spell check menu
    /// is showing.
    pub misspelled_selection_color: Option<Color>,
    /// Style used to indicate misspelled words.
    ///
    /// This is nullable to allow style-specific wrappers of `EditableText`
    /// to infer this, but this must be specified if this configuration is
    /// provided directly to `EditableText` or its construction will fail with an
    /// assertion error.
    pub misspelled_text_style: Option<TextStyle>,
    /// Builds the toolbar used to display spell check suggestions for misspelled
    /// words.
    pub spell_check_suggestions_toolbar_builder: Option<EditableTextContextMenuBuilder>,
    spell_check_enabled: bool,
}

impl SpellCheckConfiguration {
    /// Creates a configuration that specifies the service and suggestions handler
    /// for spell check.
    pub const fn new() -> SpellCheckConfiguration {
        SpellCheckConfiguration {
            spell_check_service: None,
            misspelled_selection_color: None,
            misspelled_text_style: None,
            spell_check_suggestions_toolbar_builder: None,
            spell_check_enabled: true,
        }
    }

    /// Creates a configuration that disables spell check.
    pub const fn disabled() -> SpellCheckConfiguration {
        SpellCheckConfiguration {
            spell_check_enabled: false,
            spell_check_service: None,
            spell_check_suggestions_toolbar_builder: None,
            misspelled_text_style: None,
            misspelled_selection_color: None,
        }
    }

    /// Dart `SpellCheckConfiguration(spellCheckService:)`.
    pub fn spell_check_service(
        mut self,
        spell_check_service: Rc<dyn SpellCheckService>,
    ) -> SpellCheckConfiguration {
        self.spell_check_service = Some(spell_check_service);
        self
    }

    /// Dart `SpellCheckConfiguration(misspelledSelectionColor:)`.
    pub fn misspelled_selection_color(
        mut self,
        misspelled_selection_color: Color,
    ) -> SpellCheckConfiguration {
        self.misspelled_selection_color = Some(misspelled_selection_color);
        self
    }

    /// Dart `SpellCheckConfiguration(misspelledTextStyle:)`.
    pub fn misspelled_text_style(
        mut self,
        misspelled_text_style: TextStyle,
    ) -> SpellCheckConfiguration {
        self.misspelled_text_style = Some(misspelled_text_style);
        self
    }

    /// Dart `SpellCheckConfiguration(spellCheckSuggestionsToolbarBuilder:)`.
    pub fn spell_check_suggestions_toolbar_builder(
        mut self,
        spell_check_suggestions_toolbar_builder: EditableTextContextMenuBuilder,
    ) -> SpellCheckConfiguration {
        self.spell_check_suggestions_toolbar_builder =
            Some(spell_check_suggestions_toolbar_builder);
        self
    }

    /// Whether or not the configuration should enable or disable spell check.
    pub fn spell_check_enabled(&self) -> bool {
        self.spell_check_enabled
    }

    /// Returns a copy of the current [`SpellCheckConfiguration`] instance with
    /// specified overrides.
    ///
    /// If this configuration is disabled, returns [`SpellCheckConfiguration::disabled`].
    /// Chain setters: `config.copy_with().misspelled_text_style(style)`.
    pub fn copy_with(&self) -> SpellCheckConfiguration {
        if !self.spell_check_enabled {
            return SpellCheckConfiguration::disabled();
        }
        self.clone()
    }
}

impl Default for SpellCheckConfiguration {
    fn default() -> SpellCheckConfiguration {
        SpellCheckConfiguration::new()
    }
}

impl Debug for SpellCheckConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SpellCheckConfiguration({}, service: {}, text style: {:?}, toolbar builder: {})",
            if self.spell_check_enabled {
                "enabled"
            } else {
                "disabled"
            },
            if self.spell_check_service.is_some() {
                "Some"
            } else {
                "None"
            },
            self.misspelled_text_style,
            if self.spell_check_suggestions_toolbar_builder.is_some() {
                "Some"
            } else {
                "None"
            },
        )
    }
}

impl PartialEq for SpellCheckConfiguration {
    fn eq(&self, other: &SpellCheckConfiguration) -> bool {
        option_rc_ptr_eq(&self.spell_check_service, &other.spell_check_service)
            && self.misspelled_text_style == other.misspelled_text_style
            && option_rc_ptr_eq(
                &self.spell_check_suggestions_toolbar_builder,
                &other.spell_check_suggestions_toolbar_builder,
            )
            && self.spell_check_enabled == other.spell_check_enabled
    }
}

fn option_rc_ptr_eq<T: ?Sized>(left: &Option<Rc<T>>, right: &Option<Rc<T>>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => Rc::ptr_eq(left, right),
        _ => false,
    }
}

// Methods for displaying spell check results:

/// Adjusts spell check results to correspond to [`new_text`] if the only results
/// that the handler has access to are the [`results`] corresponding to
/// [`results_text`].
///
/// Used in the case where the request for the spell check results of the
/// [`new_text`] is lagging in order to avoid display of incorrect results.
fn correct_spell_check_results(
    new_text: &str,
    results_text: &str,
    results: &[SuggestionSpan],
) -> Vec<SuggestionSpan> {
    let mut corrected_spell_check_results = Vec::new();
    let mut span_pointer = 0;
    let mut offset = 0;

    // Assumes that the order of spans has not been jumbled for optimization
    // purposes, and will only search since the previously found span.
    let mut search_start = 0;

    while span_pointer < results.len() {
        let current_span = &results[span_pointer];
        let current_span_text = utf16_substring(
            results_text,
            current_span.range.start,
            current_span.range.end,
        );
        let span_length = current_span.range.end - current_span.range.start;

        // Try finding SuggestionSpan from resultsText in new text.
        let found_index =
            index_of_word_bounded(utf16_suffix(new_text, search_start), current_span_text);

        // Check whether word was found exactly where expected or elsewhere in the newText.
        let current_span_found_exactly = current_span.range.start == found_index + search_start;
        let current_span_found_exactly_with_offset =
            current_span.range.start + offset == found_index + search_start;
        let current_span_found_elsewhere = found_index >= 0;

        if current_span_found_exactly || current_span_found_exactly_with_offset {
            // currentSpan was found at the same index in newText and resultsText
            // or at the same index with the previously calculated adjustment by
            // the offset value, so apply it to new text by adding it to the list of
            // corrected results.
            let adjusted_span = SuggestionSpan::new(
                TextRange::new(
                    current_span.range.start + offset,
                    current_span.range.end + offset,
                ),
                current_span.suggestions.clone(),
            );

            // Start search for the next misspelled word at the end of currentSpan.
            search_start = (current_span.range.end + 1 + offset).min(utf16_len(new_text));
            corrected_spell_check_results.push(adjusted_span);
        } else if current_span_found_elsewhere {
            // Word was pushed forward but not modified.
            let adjusted_span_start = search_start + found_index;
            let adjusted_span_end = adjusted_span_start + span_length;
            let adjusted_span = SuggestionSpan::new(
                TextRange::new(adjusted_span_start, adjusted_span_end),
                current_span.suggestions.clone(),
            );

            // Start search for the next misspelled word at the end of the
            // adjusted currentSpan.
            search_start = (adjusted_span_end + 1).min(utf16_len(new_text));
            // Adjust offset to reflect the difference between where currentSpan
            // was positioned in resultsText versus in newText.
            offset = adjusted_span_start - current_span.range.start;
            corrected_spell_check_results.push(adjusted_span);
        }
        span_pointer += 1;
    }
    corrected_spell_check_results
}

/// Builds the [`TextSpan`] tree given the current state of the text input and
/// spell check results.
///
/// The [`value`] is the current [`TextEditingValue`] requested to be rendered
/// by a text input widget. The [`composing_within_current_text_range`] value
/// represents whether or not there is a valid composing region in the
/// [`value`]. The [`style`] is the [`TextStyle`] to render the [`value`]'s text with,
/// and the [`misspelled_text_style`] is the [`TextStyle`] to render misspelled
/// words within the [`value`]'s text with. The [`spell_check_results`] are the
/// results of spell checking the [`value`]'s text.
pub fn build_text_span_with_spell_check_suggestions(
    app: &App,
    value: &TextEditingValue,
    composing_within_current_text_range: bool,
    style: Option<&TextStyle>,
    misspelled_text_style: &TextStyle,
    spell_check_results: &SpellCheckResults,
) -> TextSpan {
    let mut spell_check_results_spans = spell_check_results.suggestion_spans.clone();
    let spell_check_results_text = &spell_check_results.spell_checked_text;

    if spell_check_results_text != &value.text {
        spell_check_results_spans = correct_spell_check_results(
            &value.text,
            spell_check_results_text,
            &spell_check_results_spans,
        );
    }

    // We will draw the TextSpan tree based on the composing region, if it is
    // available.
    // TODO(camsim99): The two separate strategies for building TextSpan trees
    // based on the availability of a composing region should be merged:
    // https://github.com/flutter/flutter/issues/124142.
    let should_consider_composing_region =
        app.platform().target_platform() == TargetPlatform::Android;
    if should_consider_composing_region {
        return text_span_children(
            style,
            build_subtrees_with_composing_region(
                &spell_check_results_spans,
                value,
                style,
                misspelled_text_style,
                composing_within_current_text_range,
            ),
        );
    }

    text_span_children(
        style,
        build_subtrees_without_composing_region(
            &spell_check_results_spans,
            value,
            style,
            misspelled_text_style,
            value.selection.base_offset,
        ),
    )
}

/// Builds the [`TextSpan`] tree for spell check without considering the composing
/// region. Instead, uses the cursor to identify the word that's actively being
/// edited and shouldn't be spell checked. This is useful for platforms and IMEs
/// that don't use the composing region for the active word.
fn build_subtrees_without_composing_region(
    spell_check_suggestions: &[SuggestionSpan],
    value: &TextEditingValue,
    style: Option<&TextStyle>,
    misspelled_style: &TextStyle,
    cursor_index: i32,
) -> Vec<TextSpan> {
    let mut text_span_tree_children = Vec::new();

    let mut text_pointer = 0;
    let mut current_span_pointer = 0;
    let text = &value.text;
    let text_len = utf16_len(text);
    let misspelled_joint_style = style
        .map(|style| style.merge(Some(misspelled_style)))
        .unwrap_or_else(|| misspelled_style.clone());

    // Add text interwoven with any misspelled words to the tree.
    while text_pointer < text_len && current_span_pointer < spell_check_suggestions.len() {
        let current_span = &spell_check_suggestions[current_span_pointer];

        if current_span.range.start > text_pointer {
            let end_index = if current_span.range.start < text_len {
                current_span.range.start
            } else {
                text_len
            };
            text_span_tree_children.push(span_text(
                style,
                utf16_substring(text, text_pointer, end_index),
            ));
            text_pointer = end_index;
        } else {
            let end_index = if current_span.range.end < text_len {
                current_span.range.end
            } else {
                text_len
            };
            let cursor_in_current_span =
                current_span.range.start <= cursor_index && current_span.range.end >= cursor_index;
            text_span_tree_children.push(span_text(
                if cursor_in_current_span {
                    style
                } else {
                    Some(&misspelled_joint_style)
                },
                utf16_substring(text, current_span.range.start, end_index),
            ));

            text_pointer = end_index;
            current_span_pointer += 1;
        }
    }

    // Add any remaining text to the tree if applicable.
    if text_pointer < text_len {
        text_span_tree_children.push(span_text(
            style,
            utf16_substring(text, text_pointer, text_len),
        ));
    }

    text_span_tree_children
}

/// Builds [`TextSpan`] subtree for text with misspelled words with logic based on
/// a valid composing region.
fn build_subtrees_with_composing_region(
    spell_check_suggestions: &[SuggestionSpan],
    value: &TextEditingValue,
    style: Option<&TextStyle>,
    misspelled_style: &TextStyle,
    composing_within_current_text_range: bool,
) -> Vec<TextSpan> {
    let mut text_span_tree_children = Vec::new();

    let mut text_pointer = 0;
    let mut current_span_pointer = 0;
    let text = &value.text;
    let text_len = utf16_len(text);
    let composing_region = value.composing;
    let underline = TextStyle::new().decoration(TextDecoration::UNDERLINE);
    let composing_text_style = style
        .map(|style| style.merge(Some(&underline)))
        .unwrap_or(underline);
    let misspelled_joint_style = style
        .map(|style| style.merge(Some(misspelled_style)))
        .unwrap_or_else(|| misspelled_style.clone());

    // Add text interwoven with any misspelled words to the tree.
    while text_pointer < text_len && current_span_pointer < spell_check_suggestions.len() {
        let current_span = &spell_check_suggestions[current_span_pointer];

        if current_span.range.start > text_pointer {
            let end_index = if current_span.range.start < text_len {
                current_span.range.start
            } else {
                text_len
            };
            let text_pointer_within_composing_region = composing_region.start >= text_pointer
                && composing_region.end <= end_index
                && !composing_within_current_text_range;

            if text_pointer_within_composing_region {
                add_composing_region_text_spans(
                    &mut text_span_tree_children,
                    text,
                    text_pointer,
                    composing_region,
                    style,
                    &composing_text_style,
                );
                text_span_tree_children.push(span_text(
                    style,
                    utf16_substring(text, composing_region.end, end_index),
                ));
            } else {
                text_span_tree_children.push(span_text(
                    style,
                    utf16_substring(text, text_pointer, end_index),
                ));
            }

            text_pointer = end_index;
        } else {
            let end_index = if current_span.range.end < text_len {
                current_span.range.end
            } else {
                text_len
            };
            let current_span_is_composing_region = text_pointer >= composing_region.start
                && end_index <= composing_region.end
                && !composing_within_current_text_range;
            text_span_tree_children.push(span_text(
                if current_span_is_composing_region {
                    Some(&composing_text_style)
                } else {
                    Some(&misspelled_joint_style)
                },
                utf16_substring(text, current_span.range.start, end_index),
            ));

            text_pointer = end_index;
            current_span_pointer += 1;
        }
    }

    // Add any remaining text to the tree if applicable.
    if text_pointer < text_len {
        if text_pointer < composing_region.start && !composing_within_current_text_range {
            add_composing_region_text_spans(
                &mut text_span_tree_children,
                text,
                text_pointer,
                composing_region,
                style,
                &composing_text_style,
            );

            if composing_region.end != text_len {
                text_span_tree_children.push(span_text(
                    style,
                    utf16_substring(text, composing_region.end, text_len),
                ));
            }
        } else {
            text_span_tree_children.push(span_text(
                style,
                utf16_substring(text, text_pointer, text_len),
            ));
        }
    }

    text_span_tree_children
}

/// Helper method to create [`TextSpan`] tree children for specified range of
/// text up to and including the composing region.
fn add_composing_region_text_spans(
    tree_children: &mut Vec<TextSpan>,
    text: &str,
    start: i32,
    composing_region: TextRange,
    style: Option<&TextStyle>,
    composing_text_style: &TextStyle,
) {
    tree_children.push(span_text(
        style,
        utf16_substring(text, start, composing_region.start),
    ));
    tree_children.push(span_text(
        Some(composing_text_style),
        utf16_substring(text, composing_region.start, composing_region.end),
    ));
}

fn span_text(style: Option<&TextStyle>, text: impl Into<String>) -> TextSpan {
    let span = TextSpan::new().text(text);
    match style {
        Some(style) => span.style(style.clone()),
        None => span,
    }
}

fn text_span_children(style: Option<&TextStyle>, children: Vec<TextSpan>) -> TextSpan {
    let children: Vec<InlineSpanRef> = children.into_iter().map(TextSpan::into_span).collect();
    let span = TextSpan::new().children(children);
    match style {
        Some(style) => span.style(style.clone()),
        None => span,
    }
}

fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

fn utf16_substring(text: &str, start: i32, end: i32) -> &str {
    TextRange::new(start, end).text_inside(text)
}

fn utf16_suffix(text: &str, start: i32) -> &str {
    TextRange::collapsed(start).text_after(text)
}

/// First UTF-16 index of `needle` in `haystack` with ASCII word boundaries on
/// both sides. Dart: `haystack.indexOf(RegExp('\\b${RegExp.escape(needle)}\\b'))`.
///
/// `\b` is a transition between `[A-Za-z0-9_]` and a non-word char or a string
/// edge, matching Dart's default (non-`unicode`) JS-style `\b`.
fn index_of_word_bounded(haystack: &str, needle: &str) -> i32 {
    let mut utf16 = 0i32;
    let mut byte = 0usize;
    loop {
        let end_byte = byte + needle.len();
        let ends_on_boundary = needle.is_empty()
            || (end_byte <= haystack.len() && haystack.is_char_boundary(end_byte));
        if haystack.is_char_boundary(byte)
            && ends_on_boundary
            && haystack[byte..].starts_with(needle)
            && is_word_boundary(haystack, byte)
            && is_word_boundary(haystack, end_byte)
        {
            return utf16;
        }
        if byte >= haystack.len() {
            return -1;
        }
        let Some(ch) = haystack[byte..].chars().next() else {
            return -1;
        };
        byte += ch.len_utf8();
        utf16 += ch.len_utf16() as i32;
    }
}

fn is_word_boundary(text: &str, byte_pos: usize) -> bool {
    let left_word = byte_pos > 0
        && text[..byte_pos]
            .chars()
            .next_back()
            .is_some_and(is_ascii_word_char);
    let right_word = byte_pos < text.len()
        && text[byte_pos..]
            .chars()
            .next()
            .is_some_and(is_ascii_word_char);
    left_word != right_word
}

fn is_ascii_word_char(c: char) -> bool {
    matches!(c, 'A'..='Z' | 'a'..='z' | '0'..='9' | '_')
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use reveal_painting::InlineSpan;

    use super::*;

    fn misspelled_style() -> TextStyle {
        TextStyle::new().decoration(TextDecoration::UNDERLINE)
    }

    #[test]
    fn empty_spans_yield_a_single_child_with_the_value_text() {
        let cell = AppCell::new();
        let app = cell.borrow_mut();
        let text = "Hello, wrold! Hey";
        let value = TextEditingValue::new().text(text);
        let spell_check_results = SpellCheckResults::new(text, Vec::new());
        let misspelled = misspelled_style();

        let actual = build_text_span_with_spell_check_suggestions(
            &app,
            &value,
            true,
            None,
            &misspelled,
            &spell_check_results,
        );
        let expected = TextSpan::new().children(vec![TextSpan::new().text(text).into_span()]);
        assert!(actual.eq_span(&expected));
    }

    #[test]
    fn a_span_in_the_middle_gets_the_misspelled_style() {
        let cell = AppCell::new();
        let app = cell.borrow_mut();
        let text = "Hello, wrold! Hey";
        let value = TextEditingValue::new().text(text);
        let misspelled = misspelled_style();
        let spell_check_results = SpellCheckResults::new(
            text,
            vec![SuggestionSpan::new(
                TextRange::new(7, 12),
                vec!["world".into(), "word".into(), "old".into()],
            )],
        );

        let actual = build_text_span_with_spell_check_suggestions(
            &app,
            &value,
            true,
            None,
            &misspelled,
            &spell_check_results,
        );
        let expected = TextSpan::new().children(vec![
            TextSpan::new().text("Hello, ").into_span(),
            TextSpan::new()
                .style(misspelled.clone())
                .text("wrold")
                .into_span(),
            TextSpan::new().text("! Hey").into_span(),
        ]);
        assert!(actual.eq_span(&expected));
    }
}
