//! Maps winit [`Ime`] onto Flutter [`TextEditingValue`].
//!
//! winit's preedit offsets are UTF-8 bytes; [`TextEditingValue`] offsets are
//! UTF-16 code units.

use inset_embedder::{TextAffinity, TextEditingValue, TextRange, TextSelection};
use winit::event::Ime;

/// What the host should do with one winit IME event.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ImeOutcome {
    None,
    Closed,
    Value(TextEditingValue),
}

/// Applies one winit IME event to the last editing state the framework sent.
pub(crate) fn apply_ime(value: &TextEditingValue, ime: &Ime) -> ImeOutcome {
    match ime {
        Ime::Enabled => ImeOutcome::None,
        Ime::Disabled => ImeOutcome::Closed,
        Ime::Preedit(text, cursor) => ImeOutcome::Value(apply_preedit(value, text, *cursor)),
        Ime::Commit(text) => ImeOutcome::Value(apply_commit(value, text)),
    }
}

/// Replaces the composing region (or the selection) with `text` and marks that
/// span as composing. `cursor` is a UTF-8 byte range inside `text`; `None` hides
/// the caret.
fn apply_preedit(
    value: &TextEditingValue,
    text: &str,
    cursor: Option<(usize, usize)>,
) -> TextEditingValue {
    let range = insertion_range(value);
    let start = range.start;
    let next = value.replaced(range, text);
    if text.is_empty() {
        return next
            .composing(TextRange::EMPTY)
            .selection(TextSelection::collapsed(start, TextAffinity::Downstream));
    }
    let inserted = utf16_len(text);
    let selection = match cursor {
        Some((a, b)) => TextSelection::new(start + utf16_at(text, a), start + utf16_at(text, b)),
        None => TextSelection::collapsed(start + inserted, TextAffinity::Downstream),
    };
    next.composing(TextRange::new(start, start + inserted))
        .selection(selection)
}

/// Inserts `text` in place of the composing region (or the selection) and
/// clears composing. The caret sits after the inserted text.
fn apply_commit(value: &TextEditingValue, text: &str) -> TextEditingValue {
    let range = insertion_range(value);
    let start = range.start;
    value
        .replaced(range, text)
        .composing(TextRange::EMPTY)
        .selection(TextSelection::collapsed(
            start + utf16_len(text),
            TextAffinity::Downstream,
        ))
}

/// The range a preedit or commit overwrites: composing if it is valid, else the
/// selection, else offset 0.
fn insertion_range(value: &TextEditingValue) -> TextRange {
    if value.composing.is_valid() {
        return value.composing;
    }
    let range = value.selection.range();
    if range.is_valid() {
        range
    } else {
        TextRange::collapsed(0)
    }
}

/// Dart `String.length`: UTF-16 code units.
fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

/// Converts a UTF-8 byte offset in `text` to a UTF-16 offset, snapping back to
/// the nearest character boundary.
fn utf16_at(text: &str, utf8: usize) -> i32 {
    let mut end = utf8.min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].encode_utf16().count() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_field() -> TextEditingValue {
        TextEditingValue::new().selection(TextSelection::collapsed(0, TextAffinity::Downstream))
    }

    #[test]
    fn commit_inserts_at_the_caret() {
        let value = apply_ime(&empty_field(), &Ime::Commit("hi".into()));
        assert_eq!(
            value,
            ImeOutcome::Value(
                TextEditingValue::new()
                    .text("hi")
                    .selection(TextSelection::collapsed(2, TextAffinity::Downstream))
                    .composing(TextRange::EMPTY)
            )
        );
    }

    #[test]
    fn preedit_marks_the_composing_region() {
        let value = apply_ime(&empty_field(), &Ime::Preedit("ni".into(), Some((0, 2))));
        assert_eq!(
            value,
            ImeOutcome::Value(
                TextEditingValue::new()
                    .text("ni")
                    .selection(TextSelection::new(0, 2))
                    .composing(TextRange::new(0, 2))
            )
        );
    }

    #[test]
    fn commit_replaces_the_composing_region() {
        let composing = TextEditingValue::new()
            .text("ni")
            .selection(TextSelection::new(0, 2))
            .composing(TextRange::new(0, 2));
        let value = apply_ime(&composing, &Ime::Commit("你".into()));
        assert_eq!(
            value,
            ImeOutcome::Value(
                TextEditingValue::new()
                    .text("你")
                    .selection(TextSelection::collapsed(1, TextAffinity::Downstream))
                    .composing(TextRange::EMPTY)
            )
        );
    }

    #[test]
    fn empty_preedit_clears_composing() {
        let composing = TextEditingValue::new()
            .text("ni")
            .selection(TextSelection::collapsed(2, TextAffinity::Downstream))
            .composing(TextRange::new(0, 2));
        let value = apply_ime(&composing, &Ime::Preedit(String::new(), None));
        assert_eq!(
            value,
            ImeOutcome::Value(
                TextEditingValue::new()
                    .text("")
                    .selection(TextSelection::collapsed(0, TextAffinity::Downstream))
                    .composing(TextRange::EMPTY)
            )
        );
    }

    #[test]
    fn disabled_closes_the_connection() {
        assert_eq!(
            apply_ime(&empty_field(), &Ime::Disabled),
            ImeOutcome::Closed
        );
    }
}
