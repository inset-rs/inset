//! Flutter counterpart: `widgets/text_editing_intents.dart`.
//!
//! The [`Intent`]s a text field's key bindings emit. The actions that consume them live in
//! `editable_text.dart`, which waits; `DefaultTextEditingShortcuts` binds them.

use std::any::Any;

use reveal_gestures::{PointerDownEvent, PointerUpEvent};
use reveal_services::SelectionChangedCause;

use crate::widgets::actions::Intent;
use crate::widgets::focus_manager::AnyFocusNode;

/// An [`Intent`] to send the event straight to the engine.
///
/// See also:
///
///   * `DefaultTextEditingShortcuts`, which triggers this [`Intent`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DoNothingAndStopPropagationTextIntent;

impl DoNothingAndStopPropagationTextIntent {
    /// Creates an instance of [`DoNothingAndStopPropagationTextIntent`].
    pub const fn new() -> DoNothingAndStopPropagationTextIntent {
        DoNothingAndStopPropagationTextIntent
    }
}

impl Intent for DoNothingAndStopPropagationTextIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A text editing related [`Intent`] that performs an operation towards a given
/// direction of the current caret location.
pub trait DirectionalTextEditingIntent: Intent {
    /// Whether the input field, if applicable, should perform the text editing
    /// operation from the current caret location towards the end of the document.
    ///
    /// Unless otherwise specified by the recipient of this intent, this parameter
    /// uses the logical order of characters in the string to determine the
    /// direction, and is not affected by the writing direction of the text.
    fn forward(&self) -> bool;
}

/// A [`DirectionalTextEditingIntent`] that moves the caret or the selection to a
/// new location.
pub trait DirectionalCaretMovementIntent: DirectionalTextEditingIntent {
    /// Whether this [`Intent`] should make the selection collapsed (so it becomes a
    /// caret), after the movement.
    ///
    /// When [`collapse_selection`](Self::collapse_selection) is false, the input field
    /// typically only moves the current `TextSelection.extent` to the new location, while
    /// maintains the current `TextSelection.base` location.
    ///
    /// When [`collapse_selection`](Self::collapse_selection) is true, the input field
    /// typically should move both the `TextSelection.base` and the `TextSelection.extent`
    /// to the new location.
    fn collapse_selection(&self) -> bool;

    /// Whether to collapse the selection when it would otherwise reverse order.
    ///
    /// For example, consider when forward is true and the extent is before the
    /// base. If `collapse_at_reversal` is true, then this will cause the selection to
    /// collapse at the base. If it's false, then the extent will be placed at the
    /// linebreak, reversing the order of base and offset.
    ///
    /// Cannot be true when [`collapse_selection`](Self::collapse_selection) is true.
    fn collapse_at_reversal(&self) -> bool;

    /// Whether or not to continue to the next line at a wordwrap.
    ///
    /// If true, when an [`Intent`] to go to the beginning/end of a wordwrapped line
    /// is received and the selection is already at the beginning/end of the line,
    /// then the selection will be moved to the next/previous line. If false, the
    /// selection will remain at the wordwrap.
    fn continues_at_wrap(&self) -> bool;
}

/// Writes the two traits for an intent whose caret movement fields are the four Dart
/// defines, with `collapse_at_reversal` and `continues_at_wrap` fixed to the values the
/// Dart constructor passes to `super`.
macro_rules! caret_movement_intent {
    ($intent:ty, $collapse_at_reversal:expr, $continues_at_wrap:expr) => {
        impl Intent for $intent {
            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        impl DirectionalTextEditingIntent for $intent {
            fn forward(&self) -> bool {
                self.forward
            }
        }

        impl DirectionalCaretMovementIntent for $intent {
            fn collapse_selection(&self) -> bool {
                self.collapse_selection
            }

            fn collapse_at_reversal(&self) -> bool {
                $collapse_at_reversal
            }

            fn continues_at_wrap(&self) -> bool {
                $continues_at_wrap
            }
        }
    };
}

/// Writes [`Intent`] and [`DirectionalTextEditingIntent`] for an intent whose only field is
/// `forward`.
macro_rules! directional_text_editing_intent {
    ($intent:ty) => {
        impl Intent for $intent {
            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        impl DirectionalTextEditingIntent for $intent {
            fn forward(&self) -> bool {
                self.forward
            }
        }
    };
}

/// Deletes the character before or after the caret location, based on whether
/// `forward` is true.
///
/// Typically a text field will not respond to this intent if it has no active
/// caret (`TextSelection::is_valid` is false for the current selection).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeleteCharacterIntent {
    /// See [`DirectionalTextEditingIntent::forward`].
    pub forward: bool,
}

impl DeleteCharacterIntent {
    /// Creates a [`DeleteCharacterIntent`].
    pub const fn new(forward: bool) -> DeleteCharacterIntent {
        DeleteCharacterIntent { forward }
    }
}

directional_text_editing_intent!(DeleteCharacterIntent);

/// Deletes from the current caret location to the previous or next word
/// boundary, based on whether `forward` is true.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeleteToNextWordBoundaryIntent {
    /// See [`DirectionalTextEditingIntent::forward`].
    pub forward: bool,
}

impl DeleteToNextWordBoundaryIntent {
    /// Creates a [`DeleteToNextWordBoundaryIntent`].
    pub const fn new(forward: bool) -> DeleteToNextWordBoundaryIntent {
        DeleteToNextWordBoundaryIntent { forward }
    }
}

directional_text_editing_intent!(DeleteToNextWordBoundaryIntent);

/// Deletes from the current caret location to the previous or next soft or hard
/// line break, based on whether `forward` is true.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeleteToLineBreakIntent {
    /// See [`DirectionalTextEditingIntent::forward`].
    pub forward: bool,
}

impl DeleteToLineBreakIntent {
    /// Creates a [`DeleteToLineBreakIntent`].
    pub const fn new(forward: bool) -> DeleteToLineBreakIntent {
        DeleteToLineBreakIntent { forward }
    }
}

directional_text_editing_intent!(DeleteToLineBreakIntent);

/// Extends, or moves the current selection from the current
/// `TextSelection.extent` position to the previous or the next character
/// boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtendSelectionByCharacterIntent {
    /// See [`DirectionalTextEditingIntent::forward`].
    pub forward: bool,
    /// See [`DirectionalCaretMovementIntent::collapse_selection`].
    pub collapse_selection: bool,
}

impl ExtendSelectionByCharacterIntent {
    /// Creates an [`ExtendSelectionByCharacterIntent`].
    pub const fn new(forward: bool, collapse_selection: bool) -> ExtendSelectionByCharacterIntent {
        ExtendSelectionByCharacterIntent {
            forward,
            collapse_selection,
        }
    }
}

caret_movement_intent!(ExtendSelectionByCharacterIntent, false, false);

/// Extends, or moves the current selection from the current
/// `TextSelection.extent` position to the previous or the next word
/// boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtendSelectionToNextWordBoundaryIntent {
    /// See [`DirectionalTextEditingIntent::forward`].
    pub forward: bool,
    /// See [`DirectionalCaretMovementIntent::collapse_selection`].
    pub collapse_selection: bool,
}

impl ExtendSelectionToNextWordBoundaryIntent {
    /// Creates an [`ExtendSelectionToNextWordBoundaryIntent`].
    pub const fn new(
        forward: bool,
        collapse_selection: bool,
    ) -> ExtendSelectionToNextWordBoundaryIntent {
        ExtendSelectionToNextWordBoundaryIntent {
            forward,
            collapse_selection,
        }
    }
}

caret_movement_intent!(ExtendSelectionToNextWordBoundaryIntent, false, false);

/// Extends, or moves the current selection from the current
/// `TextSelection.extent` position to the previous or the next word
/// boundary, or the `TextSelection.base` position if it's closer in the move
/// direction.
///
/// This [`Intent`] typically has the same effect as an
/// [`ExtendSelectionToNextWordBoundaryIntent`], except it collapses the selection
/// when the order of `TextSelection.base` and `TextSelection.extent` would
/// reverse.
///
/// This is typically only used on MacOS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtendSelectionToNextWordBoundaryOrCaretLocationIntent {
    /// See [`DirectionalTextEditingIntent::forward`].
    pub forward: bool,
}

impl ExtendSelectionToNextWordBoundaryOrCaretLocationIntent {
    /// Creates an [`ExtendSelectionToNextWordBoundaryOrCaretLocationIntent`].
    pub const fn new(forward: bool) -> ExtendSelectionToNextWordBoundaryOrCaretLocationIntent {
        ExtendSelectionToNextWordBoundaryOrCaretLocationIntent { forward }
    }
}

impl Intent for ExtendSelectionToNextWordBoundaryOrCaretLocationIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl DirectionalTextEditingIntent for ExtendSelectionToNextWordBoundaryOrCaretLocationIntent {
    fn forward(&self) -> bool {
        self.forward
    }
}

impl DirectionalCaretMovementIntent for ExtendSelectionToNextWordBoundaryOrCaretLocationIntent {
    fn collapse_selection(&self) -> bool {
        false
    }

    fn collapse_at_reversal(&self) -> bool {
        true
    }

    fn continues_at_wrap(&self) -> bool {
        false
    }
}

/// Expands the current selection to the document boundary in the direction
/// given by [`forward`](DirectionalTextEditingIntent::forward).
///
/// Unlike [`ExpandSelectionToLineBreakIntent`], the extent will be moved, which
/// matches the behavior on MacOS.
///
/// See also:
///
///   [`ExtendSelectionToDocumentBoundaryIntent`], which is similar but always
///   moves the extent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpandSelectionToDocumentBoundaryIntent {
    /// See [`DirectionalTextEditingIntent::forward`].
    pub forward: bool,
}

impl ExpandSelectionToDocumentBoundaryIntent {
    /// Creates an [`ExpandSelectionToDocumentBoundaryIntent`].
    pub const fn new(forward: bool) -> ExpandSelectionToDocumentBoundaryIntent {
        ExpandSelectionToDocumentBoundaryIntent { forward }
    }
}

impl Intent for ExpandSelectionToDocumentBoundaryIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl DirectionalTextEditingIntent for ExpandSelectionToDocumentBoundaryIntent {
    fn forward(&self) -> bool {
        self.forward
    }
}

impl DirectionalCaretMovementIntent for ExpandSelectionToDocumentBoundaryIntent {
    fn collapse_selection(&self) -> bool {
        false
    }

    fn collapse_at_reversal(&self) -> bool {
        false
    }

    fn continues_at_wrap(&self) -> bool {
        false
    }
}

/// Expands the current selection to the closest line break in the direction
/// given by [`forward`](DirectionalTextEditingIntent::forward).
///
/// Either the base or extent can move, whichever is closer to the line break.
/// The selection will never shrink.
///
/// This behavior is common on MacOS.
///
/// See also:
///
///   [`ExtendSelectionToLineBreakIntent`], which is similar but always moves the
///   extent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpandSelectionToLineBreakIntent {
    /// See [`DirectionalTextEditingIntent::forward`].
    pub forward: bool,
}

impl ExpandSelectionToLineBreakIntent {
    /// Creates an [`ExpandSelectionToLineBreakIntent`].
    pub const fn new(forward: bool) -> ExpandSelectionToLineBreakIntent {
        ExpandSelectionToLineBreakIntent { forward }
    }
}

impl Intent for ExpandSelectionToLineBreakIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl DirectionalTextEditingIntent for ExpandSelectionToLineBreakIntent {
    fn forward(&self) -> bool {
        self.forward
    }
}

impl DirectionalCaretMovementIntent for ExpandSelectionToLineBreakIntent {
    fn collapse_selection(&self) -> bool {
        false
    }

    fn collapse_at_reversal(&self) -> bool {
        false
    }

    fn continues_at_wrap(&self) -> bool {
        false
    }
}

/// Extends, or moves the current selection from the current
/// `TextSelection.extent` position to the closest line break in the direction
/// given by [`forward`](DirectionalTextEditingIntent::forward).
///
/// See also:
///
///   [`ExpandSelectionToLineBreakIntent`], which is similar but always increases
///   the size of the selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtendSelectionToLineBreakIntent {
    /// See [`DirectionalTextEditingIntent::forward`].
    pub forward: bool,
    /// See [`DirectionalCaretMovementIntent::collapse_selection`].
    pub collapse_selection: bool,
    /// See [`DirectionalCaretMovementIntent::collapse_at_reversal`].
    pub collapse_at_reversal: bool,
    /// See [`DirectionalCaretMovementIntent::continues_at_wrap`].
    pub continues_at_wrap: bool,
}

impl ExtendSelectionToLineBreakIntent {
    /// Creates an [`ExtendSelectionToLineBreakIntent`].
    pub const fn new(forward: bool, collapse_selection: bool) -> ExtendSelectionToLineBreakIntent {
        ExtendSelectionToLineBreakIntent {
            forward,
            collapse_selection,
            collapse_at_reversal: false,
            continues_at_wrap: false,
        }
    }

    /// Dart `ExtendSelectionToLineBreakIntent(collapseAtReversal:)`.
    pub const fn collapse_at_reversal(
        mut self,
        collapse_at_reversal: bool,
    ) -> ExtendSelectionToLineBreakIntent {
        debug_assert!(!(self.collapse_selection && collapse_at_reversal));
        self.collapse_at_reversal = collapse_at_reversal;
        self
    }

    /// Dart `ExtendSelectionToLineBreakIntent(continuesAtWrap:)`.
    pub const fn continues_at_wrap(
        mut self,
        continues_at_wrap: bool,
    ) -> ExtendSelectionToLineBreakIntent {
        self.continues_at_wrap = continues_at_wrap;
        self
    }
}

impl Intent for ExtendSelectionToLineBreakIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl DirectionalTextEditingIntent for ExtendSelectionToLineBreakIntent {
    fn forward(&self) -> bool {
        self.forward
    }
}

impl DirectionalCaretMovementIntent for ExtendSelectionToLineBreakIntent {
    fn collapse_selection(&self) -> bool {
        self.collapse_selection
    }

    fn collapse_at_reversal(&self) -> bool {
        self.collapse_at_reversal
    }

    fn continues_at_wrap(&self) -> bool {
        self.continues_at_wrap
    }
}

/// Extends, or moves the current selection from the current
/// `TextSelection.extent` position to the closest position on the adjacent
/// line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtendSelectionVerticallyToAdjacentLineIntent {
    /// See [`DirectionalTextEditingIntent::forward`].
    pub forward: bool,
    /// See [`DirectionalCaretMovementIntent::collapse_selection`].
    pub collapse_selection: bool,
}

impl ExtendSelectionVerticallyToAdjacentLineIntent {
    /// Creates an [`ExtendSelectionVerticallyToAdjacentLineIntent`].
    pub const fn new(
        forward: bool,
        collapse_selection: bool,
    ) -> ExtendSelectionVerticallyToAdjacentLineIntent {
        ExtendSelectionVerticallyToAdjacentLineIntent {
            forward,
            collapse_selection,
        }
    }
}

caret_movement_intent!(ExtendSelectionVerticallyToAdjacentLineIntent, false, false);

/// Expands, or moves the current selection from the current
/// `TextSelection.extent` position to the closest position on the adjacent
/// page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtendSelectionVerticallyToAdjacentPageIntent {
    /// See [`DirectionalTextEditingIntent::forward`].
    pub forward: bool,
    /// See [`DirectionalCaretMovementIntent::collapse_selection`].
    pub collapse_selection: bool,
}

impl ExtendSelectionVerticallyToAdjacentPageIntent {
    /// Creates an [`ExtendSelectionVerticallyToAdjacentPageIntent`].
    pub const fn new(
        forward: bool,
        collapse_selection: bool,
    ) -> ExtendSelectionVerticallyToAdjacentPageIntent {
        ExtendSelectionVerticallyToAdjacentPageIntent {
            forward,
            collapse_selection,
        }
    }
}

caret_movement_intent!(ExtendSelectionVerticallyToAdjacentPageIntent, false, false);

/// Extends, or moves the current selection from the current
/// `TextSelection.extent` position to the previous or the next paragraph
/// boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtendSelectionToNextParagraphBoundaryIntent {
    /// See [`DirectionalTextEditingIntent::forward`].
    pub forward: bool,
    /// See [`DirectionalCaretMovementIntent::collapse_selection`].
    pub collapse_selection: bool,
}

impl ExtendSelectionToNextParagraphBoundaryIntent {
    /// Creates an [`ExtendSelectionToNextParagraphBoundaryIntent`].
    pub const fn new(
        forward: bool,
        collapse_selection: bool,
    ) -> ExtendSelectionToNextParagraphBoundaryIntent {
        ExtendSelectionToNextParagraphBoundaryIntent {
            forward,
            collapse_selection,
        }
    }
}

caret_movement_intent!(ExtendSelectionToNextParagraphBoundaryIntent, false, false);

/// Extends, or moves the current selection from the current
/// `TextSelection.extent` position to the previous or the next paragraph
/// boundary depending on the [`forward`](DirectionalTextEditingIntent::forward) parameter.
///
/// This [`Intent`] typically has the same effect as an
/// [`ExtendSelectionToNextParagraphBoundaryIntent`], except it collapses the selection
/// when the order of `TextSelection.base` and `TextSelection.extent` would
/// reverse.
///
/// This is typically only used on MacOS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent {
    /// See [`DirectionalTextEditingIntent::forward`].
    pub forward: bool,
}

impl ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent {
    /// Creates an [`ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent`].
    pub const fn new(forward: bool) -> ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent {
        ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent { forward }
    }
}

impl Intent for ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl DirectionalTextEditingIntent for ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent {
    fn forward(&self) -> bool {
        self.forward
    }
}

impl DirectionalCaretMovementIntent
    for ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent
{
    fn collapse_selection(&self) -> bool {
        false
    }

    fn collapse_at_reversal(&self) -> bool {
        true
    }

    fn continues_at_wrap(&self) -> bool {
        false
    }
}

/// Extends, or moves the current selection from the current
/// `TextSelection.extent` position to the start or the end of the document.
///
/// See also:
///
///   [`ExpandSelectionToDocumentBoundaryIntent`], which is similar but always
///   increases the size of the selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtendSelectionToDocumentBoundaryIntent {
    /// See [`DirectionalTextEditingIntent::forward`].
    pub forward: bool,
    /// See [`DirectionalCaretMovementIntent::collapse_selection`].
    pub collapse_selection: bool,
}

impl ExtendSelectionToDocumentBoundaryIntent {
    /// Creates an [`ExtendSelectionToDocumentBoundaryIntent`].
    pub const fn new(
        forward: bool,
        collapse_selection: bool,
    ) -> ExtendSelectionToDocumentBoundaryIntent {
        ExtendSelectionToDocumentBoundaryIntent {
            forward,
            collapse_selection,
        }
    }
}

caret_movement_intent!(ExtendSelectionToDocumentBoundaryIntent, false, false);

/// Scrolls to the beginning or end of the document depending on the
/// [`forward`](DirectionalTextEditingIntent::forward) parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollToDocumentBoundaryIntent {
    /// See [`DirectionalTextEditingIntent::forward`].
    pub forward: bool,
}

impl ScrollToDocumentBoundaryIntent {
    /// Creates a [`ScrollToDocumentBoundaryIntent`].
    pub const fn new(forward: bool) -> ScrollToDocumentBoundaryIntent {
        ScrollToDocumentBoundaryIntent { forward }
    }
}

directional_text_editing_intent!(ScrollToDocumentBoundaryIntent);

/// An [`Intent`] to select everything in the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectAllTextIntent {
    /// The [`SelectionChangedCause`] that triggered the intent.
    pub cause: SelectionChangedCause,
}

impl SelectAllTextIntent {
    /// Creates an instance of [`SelectAllTextIntent`].
    pub const fn new(cause: SelectionChangedCause) -> SelectAllTextIntent {
        SelectAllTextIntent { cause }
    }
}

impl Intent for SelectAllTextIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Intent`] that represents a user interaction that attempts to copy or cut
/// the current selection in the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CopySelectionTextIntent {
    cause: SelectionChangedCause,
    collapse_selection: bool,
}

impl CopySelectionTextIntent {
    /// An [`Intent`] that represents a user interaction that attempts to copy the
    /// current selection in the field.
    pub const COPY: CopySelectionTextIntent = CopySelectionTextIntent {
        cause: SelectionChangedCause::Keyboard,
        collapse_selection: false,
    };

    /// Creates an [`Intent`] that represents a user interaction that attempts to
    /// cut the current selection in the field.
    pub const fn cut(cause: SelectionChangedCause) -> CopySelectionTextIntent {
        CopySelectionTextIntent {
            cause,
            collapse_selection: true,
        }
    }

    /// The [`SelectionChangedCause`] that triggered the intent.
    pub const fn cause(&self) -> SelectionChangedCause {
        self.cause
    }

    /// Whether the original text needs to be removed from the input field if the
    /// copy action was successful.
    pub const fn collapse_selection(&self) -> bool {
        self.collapse_selection
    }
}

impl Intent for CopySelectionTextIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Intent`] to paste text from the clipboard to the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PasteTextIntent {
    /// The [`SelectionChangedCause`] that triggered the intent.
    pub cause: SelectionChangedCause,
}

impl PasteTextIntent {
    /// Creates an instance of [`PasteTextIntent`].
    pub const fn new(cause: SelectionChangedCause) -> PasteTextIntent {
        PasteTextIntent { cause }
    }
}

impl Intent for PasteTextIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Intent`] that represents a user interaction that attempts to go back to
/// the previous editing state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RedoTextIntent {
    /// The [`SelectionChangedCause`] that triggered the intent.
    pub cause: SelectionChangedCause,
}

impl RedoTextIntent {
    /// Creates a [`RedoTextIntent`].
    pub const fn new(cause: SelectionChangedCause) -> RedoTextIntent {
        RedoTextIntent { cause }
    }
}

impl Intent for RedoTextIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Intent`] that represents a user interaction that attempts to go back to
/// the previous editing state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UndoTextIntent {
    /// The [`SelectionChangedCause`] that triggered the intent.
    pub cause: SelectionChangedCause,
}

impl UndoTextIntent {
    /// Creates an [`UndoTextIntent`].
    pub const fn new(cause: SelectionChangedCause) -> UndoTextIntent {
        UndoTextIntent { cause }
    }
}

impl Intent for UndoTextIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Intent`] that represents a user interaction that attempts to swap the
/// characters immediately around the cursor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TransposeCharactersIntent;

impl TransposeCharactersIntent {
    /// Creates a [`TransposeCharactersIntent`].
    pub const fn new() -> TransposeCharactersIntent {
        TransposeCharactersIntent
    }
}

impl Intent for TransposeCharactersIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Intent`] that represents a tap outside the field.
///
/// Invoked when the user taps outside the focused `EditableText` if
/// `EditableText.onTapOutside` is `None`.
///
/// Override this [`Intent`] to modify the default behavior, which is to unfocus
/// on a touch event on web and do nothing on other platforms.
///
/// See also:
///
///  * [`AnyAction::overridable`](crate::AnyAction::overridable) for an example on how to make
///    an `Action` overridable.
#[derive(Clone, Debug)]
pub struct EditableTextTapOutsideIntent {
    /// The `FocusNode` that this [`Intent`]'s action should be performed on.
    pub focus_node: AnyFocusNode,
    /// The `PointerDownEvent` that initiated this [`Intent`].
    pub pointer_down_event: PointerDownEvent,
}

impl EditableTextTapOutsideIntent {
    /// Creates an [`EditableTextTapOutsideIntent`].
    pub fn new(
        focus_node: AnyFocusNode,
        pointer_down_event: PointerDownEvent,
    ) -> EditableTextTapOutsideIntent {
        EditableTextTapOutsideIntent {
            focus_node,
            pointer_down_event,
        }
    }
}

impl Intent for EditableTextTapOutsideIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Intent`] that represents a tap outside the field.
///
/// Invoked when the user taps up outside the focused `EditableText` if
/// `EditableText.onTapUpOutside` is `None`.
///
/// Override this [`Intent`] to modify the default behavior, which is to unfocus
/// on a touch event on web and do nothing on other platforms.
///
/// See also:
///
///  * [`AnyAction::overridable`](crate::AnyAction::overridable) for an example on how to make
///    an `Action` overridable.
#[derive(Clone, Debug)]
pub struct EditableTextTapUpOutsideIntent {
    /// The `FocusNode` that this [`Intent`]'s action should be performed on.
    pub focus_node: AnyFocusNode,
    /// The `PointerUpEvent` that initiated this [`Intent`].
    pub pointer_up_event: PointerUpEvent,
}

impl EditableTextTapUpOutsideIntent {
    /// Creates an [`EditableTextTapUpOutsideIntent`].
    pub fn new(
        focus_node: AnyFocusNode,
        pointer_up_event: PointerUpEvent,
    ) -> EditableTextTapUpOutsideIntent {
        EditableTextTapUpOutsideIntent {
            focus_node,
            pointer_up_event,
        }
    }
}

impl Intent for EditableTextTapUpOutsideIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_or_caret_location_intents_collapse_at_a_reversal() {
        let word = ExtendSelectionToNextWordBoundaryOrCaretLocationIntent::new(true);
        assert!(word.forward());
        assert!(!word.collapse_selection());
        assert!(word.collapse_at_reversal());
        assert!(!word.continues_at_wrap());

        let paragraph = ExtendSelectionToNextParagraphBoundaryOrCaretLocationIntent::new(false);
        assert!(!paragraph.forward());
        assert!(!paragraph.collapse_selection());
        assert!(paragraph.collapse_at_reversal());
    }

    #[test]
    fn a_line_break_intent_carries_the_two_optional_caret_arguments() {
        let plain = ExtendSelectionToLineBreakIntent::new(true, true);
        assert!(!plain.collapse_at_reversal);
        assert!(!plain.continues_at_wrap);

        let wrapping = ExtendSelectionToLineBreakIntent::new(true, false)
            .collapse_at_reversal(true)
            .continues_at_wrap(true);
        // The setters take the field's name, so the getters of the abstract type are reached
        // through the trait.
        assert!(DirectionalCaretMovementIntent::collapse_at_reversal(
            &wrapping
        ));
        assert!(DirectionalCaretMovementIntent::continues_at_wrap(&wrapping));
    }

    #[test]
    fn an_expanding_intent_never_collapses_the_selection() {
        assert!(!ExpandSelectionToLineBreakIntent::new(true).collapse_selection());
        assert!(!ExpandSelectionToLineBreakIntent::new(true).collapse_at_reversal());
        assert!(!ExpandSelectionToDocumentBoundaryIntent::new(false).collapse_selection());
        assert!(!ExpandSelectionToDocumentBoundaryIntent::new(false).collapse_at_reversal());
    }
}
