//! The [`TextBoundary`] subclasses defined by text layout rather than by the string alone:
//! Flutter's `LineBoundary` from `services/text_boundary.dart` and its `WordBoundary` from
//! `painting/text_painter.dart`.
//!
//! Both ask a laid-out paragraph where the breaks are, and reveal keeps a laid-out paragraph
//! in the render object that owns its painter, so both live here with [`RenderEditable`].

use std::rc::Rc;

use reveal_embedder::{TextPosition, TextRange};
use reveal_foundation::App;
use reveal_painting::InlineSpanRef;
use reveal_services::{TextBoundary, UntilPredicate};
use unicode_general_category::{GeneralCategory, get_general_category};

use crate::editable::RenderEditable;
use crate::object::RenderHandle;

/// A [`TextBoundary`] subclass for locating closest line breaks to a given
/// `position`.
///
/// When the given `position` points to a hard line break, the returned range
/// is the line's content range before the hard line break, and does not contain
/// the given `position`. For instance, the line breaks at `position = 1` for
/// "a\nb" is `[0, 1)`, which does not contain the position `1`.
#[derive(Clone, Copy, Debug)]
pub struct LineBoundary {
    text_layout: RenderHandle<RenderEditable>,
}

impl LineBoundary {
    /// Creates a [`LineBoundary`] with the text and layout information.
    pub fn new(text_layout: RenderHandle<RenderEditable>) -> LineBoundary {
        LineBoundary { text_layout }
    }
}

impl TextBoundary for LineBoundary {
    fn get_text_boundary_at(&self, app: &mut App, position: i32) -> TextRange {
        self.text_layout
            .get_line_at_offset(app, TextPosition::new(position.max(0)))
            .range()
    }
}

/// A [`TextBoundary`] subclass for locating word breaks.
///
/// The underlying implementation uses [UAX #29](https://unicode.org/reports/tr29/)
/// defined default word boundaries.
///
/// The default word break rules can be tailored to meet the requirements of
/// different use cases. For instance, the default rule set keeps horizontal
/// whitespaces together as a single word, which may not make sense in a
/// word-counting context -- "hello    world" counts as 3 words instead of 2.
/// An example is the [`move_by_word_boundary`](Self::move_by_word_boundary) variant, which
/// is a tailored word-break locator that more closely matches the default behavior of most
/// platforms and editors when it comes to handling text editing keyboard
/// shortcuts that move or delete word by word.
#[derive(Clone)]
pub struct WordBoundary {
    text: InlineSpanRef,
    /// Dart holds the `ui.Paragraph`; a laid-out paragraph lives in the arena here, so the
    /// render object that owns it answers the query instead.
    paragraph: RenderHandle<RenderEditable>,
}

impl WordBoundary {
    /// Creates a [`WordBoundary`] with the text and layout information.
    pub(crate) fn new(text: InlineSpanRef, paragraph: RenderHandle<RenderEditable>) -> WordBoundary {
        WordBoundary { text, paragraph }
    }

    /// Returns a [`TextBoundary`] suitable for handling keyboard navigation
    /// commands that change the current selection word by word.
    ///
    /// This [`TextBoundary`] is used by text widgets in the flutter framework to
    /// provide default implementation for text editing shortcuts, for example,
    /// "delete to the previous word".
    ///
    /// The implementation applies the same set of rules [`WordBoundary`] uses,
    /// except that word breaks end on a space separator or a punctuation will be
    /// skipped, to match the behavior of most platforms. Additional rules may be
    /// added in the future to better match platform behaviors.
    pub fn move_by_word_boundary(&self) -> Box<dyn TextBoundary> {
        let text = Rc::clone(&self.text);
        Box::new(UntilTextBoundary {
            text_boundary: Box::new(self.clone()),
            predicate: Rc::new(move |offset, forward| {
                skip_spaces_and_punctuations(&text, offset, forward)
            }),
        })
    }
}

impl TextBoundary for WordBoundary {
    fn get_text_boundary_at(&self, app: &mut App, position: i32) -> TextRange {
        self.paragraph
            .get_word_boundary(app, TextPosition::new(position.max(0)))
    }
}

/// Combines two UTF-16 code units (high surrogate + low surrogate) into a
/// single code point that represents a supplementary character.
fn code_point_from_surrogates(high_surrogate: i32, low_surrogate: i32) -> i32 {
    const BASE: i32 = 0x010000 - (0xD800 << 10) - 0xDC00;
    (high_surrogate << 10) + low_surrogate + BASE
}

/// Dart's `WordBoundary._codePointAt`: `Runes` gives no random access by code unit offset.
fn code_point_at(text: &InlineSpanRef, index: i32) -> Option<i32> {
    let code_unit_at_index = i32::from(text.code_unit_at(index)?);
    Some(match code_unit_at_index & 0xFC00 {
        0xD800 => code_point_from_surrogates(
            code_unit_at_index,
            i32::from(text.code_unit_at(index + 1)?),
        ),
        0xDC00 => code_point_from_surrogates(
            i32::from(text.code_unit_at(index - 1)?),
            code_unit_at_index,
        ),
        _ => code_unit_at_index,
    })
}

fn is_newline(code_point: i32) -> bool {
    // Carriage Return is not treated as a hard line break.
    matches!(
        code_point,
        0x000A | // Line Feed
        0x0085 | // New Line
        0x000B | // Form Feed
        0x000C | // Vertical Feed
        0x2028 | // Line Separator
        0x2029 // Paragraph Separator
    )
}

/// Dart matches `[\p{Space_Separator}\p{Punctuation}]`.
fn is_space_separator_or_punctuation(code_point: i32) -> bool {
    let Some(character) = u32::try_from(code_point).ok().and_then(char::from_u32) else {
        return false;
    };
    matches!(
        get_general_category(character),
        GeneralCategory::SpaceSeparator
            | GeneralCategory::ConnectorPunctuation
            | GeneralCategory::DashPunctuation
            | GeneralCategory::OpenPunctuation
            | GeneralCategory::ClosePunctuation
            | GeneralCategory::InitialPunctuation
            | GeneralCategory::FinalPunctuation
            | GeneralCategory::OtherPunctuation
    )
}

fn skip_spaces_and_punctuations(text: &InlineSpanRef, offset: i32, forward: bool) -> bool {
    // Use code point since some punctuations are supplementary characters.
    // "inner" here refers to the code unit that's before the break in the
    // search direction (`forward`).
    let inner_code_point = code_point_at(text, if forward { offset - 1 } else { offset });
    let outer_code_unit = text.code_unit_at(if forward { offset } else { offset - 1 });

    // Make sure the hard break rules in UAX#29 take precedence over the ones we
    // add below. Luckily there're only 4 hard break rules for word breaks, and
    // dictionary based breaking does not introduce new hard breaks:
    // https://unicode-org.github.io/icu/userguide/boundaryanalysis/break-rules.html#word-dictionaries
    //
    // WB1 & WB2: always break at the start or the end of the text.
    let Some(inner_code_point) = inner_code_point else {
        return true;
    };
    let Some(outer_code_unit) = outer_code_unit else {
        return true;
    };
    // WB3a & WB3b: always break before and after newlines.
    let hard_break_rules_apply =
        is_newline(inner_code_point) || is_newline(i32::from(outer_code_unit));
    hard_break_rules_apply || !is_space_separator_or_punctuation(inner_code_point)
}

/// Dart's `_UntilTextBoundary`: a boundary that keeps searching past the breaks its
/// predicate rejects.
struct UntilTextBoundary {
    text_boundary: Box<dyn TextBoundary>,
    predicate: Rc<UntilPredicate>,
}

impl TextBoundary for UntilTextBoundary {
    fn get_leading_text_boundary_at(&self, app: &mut App, position: i32) -> Option<i32> {
        if position < 0 {
            return None;
        }
        let offset = self.text_boundary.get_leading_text_boundary_at(app, position)?;
        if (self.predicate)(offset, false) {
            Some(offset)
        } else {
            self.get_leading_text_boundary_at(app, offset - 1)
        }
    }

    fn get_trailing_text_boundary_at(&self, app: &mut App, position: i32) -> Option<i32> {
        let offset = self
            .text_boundary
            .get_trailing_text_boundary_at(app, position.max(0))?;
        if (self.predicate)(offset, true) {
            Some(offset)
        } else {
            self.get_trailing_text_boundary_at(app, offset)
        }
    }
}
