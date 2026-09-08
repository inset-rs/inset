//! Flutter counterpart: `services/text_editing.dart`.
//!
//! Lives here because [`crate::View`] names [`crate::TextEditingValue`], which holds this type.

use std::fmt;
use std::hash::{Hash, Hasher};

use crate::{TextAffinity, TextPosition, TextRange};

/// A range of text that represents a selection.
///
/// Dart's `TextSelection extends TextRange`: the range is [`range`](Self::range), and
/// [`start`](Self::start) / [`end`](Self::end) are the normalized bounds.
#[derive(Clone, Copy, Debug)]
pub struct TextSelection {
    /// The offset at which the selection originates.
    ///
    /// Might be larger than, smaller than, or equal to extent.
    pub base_offset: i32,
    /// The offset at which the selection terminates.
    ///
    /// When the user uses the arrow keys to adjust the selection, this is the value that
    /// changes. Similarly, if the current theme paints a caret on one side of the selection,
    /// this is the location at which to paint the caret.
    ///
    /// Might be larger than, smaller than, or equal to base.
    pub extent_offset: i32,
    /// If the text range is collapsed and has more than one visual location (e.g., occurs at
    /// a line break), which of the two locations to use when painting the caret.
    pub affinity: TextAffinity,
    /// Whether this selection has disambiguated its base and extent.
    ///
    /// On some platforms, the base and extent are not disambiguated until the first time the
    /// user adjusts the selection. At that point, either the start or the end of the
    /// selection becomes the base and the other one becomes the extent and is adjusted.
    pub is_directional: bool,
}

impl TextSelection {
    /// Creates a text selection.
    pub const fn new(base_offset: i32, extent_offset: i32) -> TextSelection {
        TextSelection {
            base_offset,
            extent_offset,
            affinity: TextAffinity::Downstream,
            is_directional: false,
        }
    }

    /// Creates a collapsed selection at the given offset.
    ///
    /// A collapsed selection starts and ends at the same offset, which means it contains zero
    /// characters but instead serves as an insertion point in the text.
    pub const fn collapsed(offset: i32, affinity: TextAffinity) -> TextSelection {
        TextSelection {
            base_offset: offset,
            extent_offset: offset,
            affinity,
            is_directional: false,
        }
    }

    /// Creates a collapsed selection at the given text position.
    ///
    /// A collapsed selection starts and ends at the same offset, which means it contains zero
    /// characters but instead serves as an insertion point in the text.
    pub const fn from_position(position: TextPosition) -> TextSelection {
        TextSelection {
            base_offset: position.offset,
            extent_offset: position.offset,
            affinity: position.affinity,
            is_directional: false,
        }
    }

    /// The normalized range this selection covers (Dart's `TextRange` superclass).
    pub fn range(&self) -> TextRange {
        if self.base_offset < self.extent_offset {
            TextRange::new(self.base_offset, self.extent_offset)
        } else {
            TextRange::new(self.extent_offset, self.base_offset)
        }
    }

    /// The index of the first character in the range.
    pub fn start(&self) -> i32 {
        self.range().start
    }

    /// The next index after the characters in this range.
    pub fn end(&self) -> i32 {
        self.range().end
    }

    /// Whether this range represents a valid position in the text.
    pub fn is_valid(&self) -> bool {
        self.range().is_valid()
    }

    /// Whether this range is empty (but still potentially placed inside the text).
    pub fn is_collapsed(&self) -> bool {
        self.range().is_collapsed()
    }

    /// Whether the start of this range precedes the end.
    pub fn is_normalized(&self) -> bool {
        self.range().is_normalized()
    }

    /// The position at which the selection originates.
    ///
    /// The [`TextAffinity`] of the resulting [`TextPosition`] is based on the relative logical
    /// position in the text to the other selection endpoint:
    ///  * if [`base_offset`](Self::base_offset) < [`extent_offset`](Self::extent_offset),
    ///    [`base`](Self::base) will have a [`TextAffinity::Downstream`] affinity;
    ///  * if [`base_offset`](Self::base_offset) > [`extent_offset`](Self::extent_offset),
    ///    [`base`](Self::base) will have a [`TextAffinity::Upstream`] affinity;
    ///  * if the selection is collapsed, the affinity is [`affinity`](Self::affinity).
    pub fn base(&self) -> TextPosition {
        let affinity = if !self.is_valid() || self.base_offset == self.extent_offset {
            self.affinity
        } else if self.base_offset < self.extent_offset {
            TextAffinity::Downstream
        } else {
            TextAffinity::Upstream
        };
        TextPosition::with_affinity(self.base_offset, affinity)
    }

    /// The position at which the selection terminates.
    ///
    /// When the user uses the arrow keys to adjust the selection, this is the value that
    /// changes. Similarly, if the current theme paints a caret on one side of the selection,
    /// this is the location at which to paint the caret.
    pub fn extent(&self) -> TextPosition {
        let affinity = if !self.is_valid() || self.base_offset == self.extent_offset {
            self.affinity
        } else if self.base_offset < self.extent_offset {
            TextAffinity::Upstream
        } else {
            TextAffinity::Downstream
        };
        TextPosition::with_affinity(self.extent_offset, affinity)
    }

    /// Creates a new [`TextSelection`] based on the current selection, with the provided
    /// parameters overridden. Dart's `copyWith` named arguments are the fluent setters that
    /// follow.
    pub fn copy_with(&self) -> TextSelection {
        *self
    }

    /// Dart `copyWith(baseOffset:)`.
    pub fn base_offset(mut self, base_offset: i32) -> TextSelection {
        self.base_offset = base_offset;
        self
    }

    /// Dart `copyWith(extentOffset:)`.
    pub fn extent_offset(mut self, extent_offset: i32) -> TextSelection {
        self.extent_offset = extent_offset;
        self
    }

    /// Dart `copyWith(affinity:)`.
    pub fn affinity(mut self, affinity: TextAffinity) -> TextSelection {
        self.affinity = affinity;
        self
    }

    /// Dart `copyWith(isDirectional:)`.
    pub fn is_directional(mut self, is_directional: bool) -> TextSelection {
        self.is_directional = is_directional;
        self
    }

    /// Returns the smallest [`TextSelection`] that this could expand to in order to include
    /// the given [`TextPosition`].
    ///
    /// If the given [`TextPosition`] is already inside of the selection, then returns `self`
    /// without change.
    ///
    /// The returned selection will always be a strict superset of the current selection. In
    /// other words, the selection grows to include the given [`TextPosition`].
    ///
    /// If `extent_at_index` is set to true, then the [`extent`](Self::extent) will be
    /// placed at the given index regardless of the original order of it and the
    /// [`base`](Self::base). Otherwise, the base and extent are ordered so as to keep the
    /// original selection's direction.
    pub fn expand_to(&self, position: TextPosition, extent_at_index: bool) -> TextSelection {
        // If position is already within in the selection, there's nothing to do.
        if position.offset >= self.start() && position.offset <= self.end() {
            return *self;
        }
        let normalized = self.base_offset <= self.extent_offset;
        if position.offset <= self.start() {
            // Here the position is somewhere before the selection: ..|..[...].....
            if extent_at_index {
                return self
                    .copy_with()
                    .base_offset(self.end())
                    .extent_offset(position.offset)
                    .affinity(position.affinity);
            }
            return self
                .copy_with()
                .base_offset(if normalized {
                    position.offset
                } else {
                    self.base_offset
                })
                .extent_offset(if normalized {
                    self.extent_offset
                } else {
                    position.offset
                });
        }
        // Here the position is somewhere after the selection: .....[...]..|..
        if extent_at_index {
            return self
                .copy_with()
                .base_offset(self.start())
                .extent_offset(position.offset)
                .affinity(position.affinity);
        }
        self.copy_with()
            .base_offset(if normalized {
                self.base_offset
            } else {
                position.offset
            })
            .extent_offset(if normalized {
                position.offset
            } else {
                self.extent_offset
            })
    }

    /// Keeping the selection's [`base`](Self::base) fixed, pivot the
    /// [`extent`](Self::extent) to the given [`TextPosition`].
    ///
    /// In some cases, the [`base`](Self::base) and [`extent`](Self::extent) can be flipped
    /// during this operation, or the size of the selection can be reduced.
    pub fn extend_to(&self, position: TextPosition) -> TextSelection {
        // If the selection's extent is at the position already, then nothing
        // happens.
        if self.extent() == position {
            return *self;
        }
        self.copy_with()
            .extent_offset(position.offset)
            .affinity(position.affinity)
    }
}

impl From<TextSelection> for TextRange {
    fn from(selection: TextSelection) -> TextRange {
        selection.range()
    }
}

impl PartialEq for TextSelection {
    fn eq(&self, other: &TextSelection) -> bool {
        if !self.is_valid() {
            return !other.is_valid();
        }
        other.base_offset == self.base_offset
            && other.extent_offset == self.extent_offset
            && (!self.is_collapsed() || other.affinity == self.affinity)
            && other.is_directional == self.is_directional
    }
}

impl Eq for TextSelection {}

impl Hash for TextSelection {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if !self.is_valid() {
            (-1i32).hash(state);
            (-1i32).hash(state);
            TextAffinity::Downstream.hash(state);
            return;
        }
        self.base_offset.hash(state);
        self.extent_offset.hash(state);
        if self.is_collapsed() {
            self.affinity.hash(state);
        } else {
            TextAffinity::Downstream.hash(state);
        }
        self.is_directional.hash(state);
    }
}

impl fmt::Display for TextSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.is_valid() {
            return write!(f, "TextSelection.invalid");
        }
        if self.is_collapsed() {
            write!(
                f,
                "TextSelection.collapsed(offset: {}, affinity: {:?}, isDirectional: {})",
                self.base_offset, self.affinity, self.is_directional
            )
        } else {
            write!(
                f,
                "TextSelection(baseOffset: {}, extentOffset: {}, isDirectional: {})",
                self.base_offset, self.extent_offset, self.is_directional
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reversed_selection_normalizes_its_range() {
        let selection = TextSelection::new(5, 2);
        assert_eq!(selection.start(), 2);
        assert_eq!(selection.end(), 5);
        assert_eq!(selection.base().affinity, TextAffinity::Upstream);
        assert_eq!(selection.extent().affinity, TextAffinity::Downstream);
        assert!(!selection.is_collapsed());
    }

    #[test]
    fn expand_to_grows_toward_the_position() {
        let selection = TextSelection::new(2, 5);
        assert_eq!(selection.expand_to(TextPosition::new(3), false), selection);
        let before = selection.expand_to(TextPosition::new(0), false);
        assert_eq!((before.base_offset, before.extent_offset), (0, 5));
        let after = selection.expand_to(TextPosition::new(8), true);
        assert_eq!((after.base_offset, after.extent_offset), (2, 8));
    }

    #[test]
    fn extend_to_moves_the_extent_only() {
        let selection = TextSelection::new(2, 5);
        let extended = selection.extend_to(TextPosition::new(1));
        assert_eq!((extended.base_offset, extended.extent_offset), (2, 1));
        assert_eq!(selection.extend_to(selection.extent()), selection);
    }

    #[test]
    fn invalid_selections_compare_equal() {
        assert_eq!(
            TextSelection::new(-1, -1),
            TextSelection::collapsed(-1, TextAffinity::Upstream)
        );
        assert_eq!(
            TextSelection::collapsed(3, TextAffinity::Upstream).to_string(),
            "TextSelection.collapsed(offset: 3, affinity: Upstream, isDirectional: false)"
        );
    }
}
