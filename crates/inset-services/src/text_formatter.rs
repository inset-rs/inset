//! Flutter counterpart: `services/text_formatter.dart`.

use std::rc::Rc;

use inset_embedder::{TargetPlatform, TextAffinity, TextRange, TextSelection};

use crate::TextEditingValue;

/// Mechanisms for enforcing maximum length limits.
///
/// This is used by `TextField` to specify how the `TextField.maxLength` should
/// be applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MaxLengthEnforcement {
    /// No enforcement applied to the editing value. It's possible to exceed the
    /// max length.
    None,

    /// Keep the length of the text input from exceeding the max length even when
    /// the text has an unfinished composing region.
    Enforced,

    /// Users can still input text if the current value is composing even after
    /// reaching the max length limit. After composing ends, the value will be
    /// truncated.
    TruncateAfterCompositionEnds,
}

/// A [`TextInputFormatter`] can be optionally injected into an `EditableText`
/// to provide as-you-type validation and formatting of the text being edited.
pub trait TextInputFormatter {
    /// Called when text is being typed or cut/copy/pasted in the `EditableText`.
    fn format_edit_update(
        &self,
        old_value: &TextEditingValue,
        new_value: &TextEditingValue,
    ) -> TextEditingValue;
}

/// Function signature expected for creating custom [`TextInputFormatter`]
/// shorthands via [`TextInputFormatterRef::with_function`].
pub type TextInputFormatFunction =
    Rc<dyn Fn(&TextEditingValue, &TextEditingValue) -> TextEditingValue>;

/// A shared [`TextInputFormatter`].
#[derive(Clone)]
pub struct TextInputFormatterRef(pub Rc<dyn TextInputFormatter>);

impl TextInputFormatterRef {
    /// A shorthand to creating a custom [`TextInputFormatter`] which formats
    /// incoming text input changes with the given function.
    pub fn with_function(format_function: TextInputFormatFunction) -> TextInputFormatterRef {
        TextInputFormatterRef(Rc::new(SimpleTextInputFormatter { format_function }))
    }
}

impl TextInputFormatter for TextInputFormatterRef {
    fn format_edit_update(
        &self,
        old_value: &TextEditingValue,
        new_value: &TextEditingValue,
    ) -> TextEditingValue {
        self.0.format_edit_update(old_value, new_value)
    }
}

struct SimpleTextInputFormatter {
    format_function: TextInputFormatFunction,
}

impl TextInputFormatter for SimpleTextInputFormatter {
    fn format_edit_update(
        &self,
        old_value: &TextEditingValue,
        new_value: &TextEditingValue,
    ) -> TextEditingValue {
        (self.format_function)(old_value, new_value)
    }
}

/// A mutable, half-open range [`base`, `extent`) within a string.
struct MutableTextRange {
    base: i32,
    extent: i32,
}

impl MutableTextRange {
    fn from_composing_range(range: TextRange) -> Option<MutableTextRange> {
        if range.is_valid() && !range.is_collapsed() {
            Some(MutableTextRange {
                base: range.start,
                extent: range.end,
            })
        } else {
            None
        }
    }

    fn from_text_selection(selection: TextSelection) -> Option<MutableTextRange> {
        if selection.is_valid() {
            Some(MutableTextRange {
                base: selection.base_offset,
                extent: selection.extent_offset,
            })
        } else {
            None
        }
    }
}

/// The intermediate state of a [`FilteringTextInputFormatter`] when it's
/// formatting a new user input.
struct TextEditingValueAccumulator {
    input_value: TextEditingValue,
    string_buffer: String,
    selection: Option<MutableTextRange>,
    composing_region: Option<MutableTextRange>,
}

impl TextEditingValueAccumulator {
    fn new(input_value: TextEditingValue) -> TextEditingValueAccumulator {
        let selection = MutableTextRange::from_text_selection(input_value.selection);
        let composing_region = MutableTextRange::from_composing_range(input_value.composing);
        TextEditingValueAccumulator {
            input_value,
            string_buffer: String::new(),
            selection,
            composing_region,
        }
    }

    fn finalize(self) -> TextEditingValue {
        let selection = self.selection;
        let composing_region = self.composing_region;
        TextEditingValue::new()
            .text(self.string_buffer)
            .composing(match &composing_region {
                None => TextRange::EMPTY,
                Some(range) if range.base == range.extent => TextRange::EMPTY,
                Some(range) => TextRange::new(range.base, range.extent),
            })
            .selection(match selection {
                None => TextSelection::collapsed(-1, TextAffinity::Downstream),
                Some(range) => TextSelection {
                    base_offset: range.base,
                    extent_offset: range.extent,
                    affinity: self.input_value.selection.affinity,
                    is_directional: self.input_value.selection.is_directional,
                },
            })
    }
}

/// A pattern [`FilteringTextInputFormatter`] matches against incoming text.
///
/// Dart's `Pattern` is a string or a `RegExp`. There is no regex crate here;
/// [`Self::Digits`] is `RegExp(r'[0-9]')`.
#[derive(Clone, Debug)]
pub enum FilterPattern {
    /// Dart `String` pattern: every occurrence of this substring.
    Literal(String),
    /// Dart `RegExp(r'[0-9]')`: every ASCII digit.
    Digits,
}

impl FilterPattern {
    fn all_matches(&self, text: &str) -> Vec<(i32, i32)> {
        match self {
            FilterPattern::Literal(needle) => {
                if needle.is_empty() {
                    return Vec::new();
                }
                let mut matches = Vec::new();
                let mut search_from = 0usize;
                while let Some(byte) = text[search_from..].find(needle) {
                    let start_byte = search_from + byte;
                    let end_byte = start_byte + needle.len();
                    matches.push((utf16_len(&text[..start_byte]), utf16_len(&text[..end_byte])));
                    search_from = end_byte;
                }
                matches
            }
            FilterPattern::Digits => {
                let mut matches = Vec::new();
                let mut utf16 = 0i32;
                for ch in text.chars() {
                    let width = ch.len_utf16() as i32;
                    if ch.is_ascii_digit() {
                        matches.push((utf16, utf16 + width));
                    }
                    utf16 += width;
                }
                matches
            }
        }
    }
}

/// A [`TextInputFormatter`] that prevents the insertion of characters matching
/// (or not matching) a particular pattern, by replacing the characters with the
/// given [`replacement_string`](Self::replacement_string).
#[derive(Clone, Debug)]
pub struct FilteringTextInputFormatter {
    /// A pattern to match or replace in incoming [`TextEditingValue`]s.
    pub filter_pattern: FilterPattern,
    /// Whether the pattern is an allow list or not.
    pub allow: bool,
    /// String used to replace banned patterns.
    pub replacement_string: String,
}

impl FilteringTextInputFormatter {
    /// Creates a formatter that replaces banned patterns with the given
    /// `replacement_string`.
    pub fn new(filter_pattern: FilterPattern, allow: bool) -> FilteringTextInputFormatter {
        FilteringTextInputFormatter {
            filter_pattern,
            allow,
            replacement_string: String::new(),
        }
    }

    /// Dart `FilteringTextInputFormatter(replacementString:)`.
    pub fn replacement_string(mut self, replacement_string: impl Into<String>) -> Self {
        self.replacement_string = replacement_string.into();
        self
    }

    /// Creates a formatter that only allows characters matching a pattern.
    pub fn allow(filter_pattern: FilterPattern) -> FilteringTextInputFormatter {
        FilteringTextInputFormatter::new(filter_pattern, true)
    }

    /// Creates a formatter that blocks characters matching a pattern.
    pub fn deny(filter_pattern: FilterPattern) -> FilteringTextInputFormatter {
        FilteringTextInputFormatter::new(filter_pattern, false)
    }

    /// A [`TextInputFormatter`] that forces input to be a single line.
    pub fn single_line_formatter() -> TextInputFormatterRef {
        TextInputFormatterRef(Rc::new(FilteringTextInputFormatter::deny(
            FilterPattern::Literal("\n".into()),
        )))
    }

    /// A [`TextInputFormatter`] that takes in digits `[0-9]` only.
    pub fn digits_only() -> TextInputFormatterRef {
        TextInputFormatterRef(Rc::new(FilteringTextInputFormatter::allow(
            FilterPattern::Digits,
        )))
    }

    fn process_region(
        &self,
        is_banned_region: bool,
        region_start: i32,
        region_end: i32,
        state: &mut TextEditingValueAccumulator,
    ) {
        let replacement_string = if is_banned_region {
            if region_start == region_end {
                String::new()
            } else {
                self.replacement_string.clone()
            }
        } else {
            utf16_slice(&state.input_value.text, region_start, region_end).to_owned()
        };

        let region_len = region_end - region_start;
        let replacement_len = utf16_len(&replacement_string);
        state.string_buffer.push_str(&replacement_string);

        if replacement_len == region_len {
            return;
        }

        let adjust_index = |original_index: i32| -> i32 {
            let replaced_length = if original_index <= region_start && original_index < region_end {
                0
            } else {
                replacement_len
            };
            let removed_length = original_index.clamp(region_start, region_end) - region_start;
            replaced_length - removed_length
        };

        if let Some(selection) = state.selection.as_mut() {
            selection.base += adjust_index(state.input_value.selection.base_offset);
            selection.extent += adjust_index(state.input_value.selection.extent_offset);
        }
        if let Some(composing) = state.composing_region.as_mut() {
            composing.base += adjust_index(state.input_value.composing.start);
            composing.extent += adjust_index(state.input_value.composing.end);
        }
    }
}

impl TextInputFormatter for FilteringTextInputFormatter {
    fn format_edit_update(
        &self,
        _old_value: &TextEditingValue,
        new_value: &TextEditingValue,
    ) -> TextEditingValue {
        let mut format_state = TextEditingValueAccumulator::new(new_value.clone());
        let matches = self.filter_pattern.all_matches(&new_value.text);
        let mut previous_end = 0i32;
        for (start, end) in matches {
            self.process_region(self.allow, previous_end, start, &mut format_state);
            self.process_region(!self.allow, start, end, &mut format_state);
            previous_end = end;
        }
        self.process_region(
            self.allow,
            previous_end,
            utf16_len(&new_value.text),
            &mut format_state,
        );
        format_state.finalize()
    }
}

/// A [`TextInputFormatter`] that prevents the insertion of more characters
/// than allowed.
pub struct LengthLimitingTextInputFormatter {
    /// The limit on the number of user-perceived characters that this formatter
    /// will allow.
    pub max_length: Option<i32>,
    /// Determines how the [`max_length`](Self::max_length) limit should be enforced.
    pub max_length_enforcement: Option<MaxLengthEnforcement>,
}

impl LengthLimitingTextInputFormatter {
    /// Creates a formatter that prevents the insertion of more characters than a
    /// limit.
    pub fn new(max_length: Option<i32>) -> LengthLimitingTextInputFormatter {
        debug_assert!(
            max_length.is_none_or(|n| n == -1 || n > 0),
            "maxLength must be null, -1 or greater than zero"
        );
        LengthLimitingTextInputFormatter {
            max_length,
            max_length_enforcement: None,
        }
    }

    /// Dart `LengthLimitingTextInputFormatter(maxLengthEnforcement:)`.
    pub fn max_length_enforcement(mut self, value: MaxLengthEnforcement) -> Self {
        self.max_length_enforcement = Some(value);
        self
    }

    /// Returns a [`MaxLengthEnforcement`] that follows the specified `platform`'s
    /// convention.
    pub fn get_default_max_length_enforcement(
        platform: Option<TargetPlatform>,
    ) -> MaxLengthEnforcement {
        match platform.unwrap_or(TargetPlatform::Android) {
            TargetPlatform::Android | TargetPlatform::Windows => MaxLengthEnforcement::Enforced,
            TargetPlatform::IOS
            | TargetPlatform::MacOS
            | TargetPlatform::Linux
            | TargetPlatform::Fuchsia => MaxLengthEnforcement::TruncateAfterCompositionEnds,
        }
    }

    /// Truncate the given [`TextEditingValue`] to `max_length` Unicode scalar
    /// values.
    pub fn truncate(value: &TextEditingValue, max_length: i32) -> TextEditingValue {
        let truncated: String = value.text.chars().take(max_length as usize).collect();
        let truncated_len = utf16_len(&truncated);
        TextEditingValue::new()
            .text(truncated)
            .selection(
                value
                    .selection
                    .copy_with()
                    .base_offset(value.selection.start().min(truncated_len))
                    .extent_offset(value.selection.end().min(truncated_len)),
            )
            .composing(
                if !value.composing.is_collapsed() && truncated_len > value.composing.start {
                    TextRange::new(
                        value.composing.start,
                        value.composing.end.min(truncated_len),
                    )
                } else {
                    TextRange::EMPTY
                },
            )
    }

    fn character_count(text: &str) -> i32 {
        text.chars().count() as i32
    }
}

impl TextInputFormatter for LengthLimitingTextInputFormatter {
    fn format_edit_update(
        &self,
        old_value: &TextEditingValue,
        new_value: &TextEditingValue,
    ) -> TextEditingValue {
        let Some(max_length) = self.max_length else {
            return new_value.clone();
        };
        if max_length == -1 || Self::character_count(&new_value.text) <= max_length {
            return new_value.clone();
        }
        debug_assert!(max_length > 0);
        match self
            .max_length_enforcement
            .unwrap_or(Self::get_default_max_length_enforcement(None))
        {
            MaxLengthEnforcement::None => new_value.clone(),
            MaxLengthEnforcement::Enforced => {
                if Self::character_count(&old_value.text) == max_length
                    && old_value.selection.is_collapsed()
                {
                    return old_value.clone();
                }
                Self::truncate(new_value, max_length)
            }
            MaxLengthEnforcement::TruncateAfterCompositionEnds => {
                if Self::character_count(&old_value.text) == max_length
                    && !old_value.composing.is_valid()
                {
                    return old_value.clone();
                }
                if new_value.composing.is_valid() {
                    return new_value.clone();
                }
                Self::truncate(new_value, max_length)
            }
        }
    }
}

fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

fn utf16_slice(text: &str, start: i32, end: i32) -> &str {
    let start = start.max(0) as usize;
    let end = end.max(0) as usize;
    let mut byte_start = None;
    let mut byte_end = None;
    let mut units = 0usize;
    for (i, ch) in text.char_indices() {
        if byte_start.is_none() && units == start {
            byte_start = Some(i);
        }
        if units == end {
            byte_end = Some(i);
            break;
        }
        units += ch.len_utf16();
    }
    let byte_start = byte_start.unwrap_or(text.len());
    let byte_end = byte_end.unwrap_or(text.len());
    &text[byte_start.min(byte_end)..byte_end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line_formatter_strips_newlines() {
        let formatter = FilteringTextInputFormatter::single_line_formatter();
        let old = TextEditingValue::EMPTY;
        let new = TextEditingValue::new()
            .text("a\nb")
            .selection(TextSelection::collapsed(3, TextAffinity::Downstream));
        let out = formatter.format_edit_update(&old, &new);
        assert_eq!(out.text, "ab");
        assert_eq!(out.selection.extent_offset, 2);
    }

    #[test]
    fn digits_only_keeps_digits() {
        let formatter = FilteringTextInputFormatter::digits_only();
        let old = TextEditingValue::EMPTY;
        let new = TextEditingValue::new().text("a1b2");
        let out = formatter.format_edit_update(&old, &new);
        assert_eq!(out.text, "12");
    }
}
