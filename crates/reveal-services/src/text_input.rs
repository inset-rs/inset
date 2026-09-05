//! Flutter counterpart: `services/text_input.dart` ([`SelectionChangedCause`] only).

/// Indicates what triggered the change in selected text (including changes to
/// the cursor location).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectionChangedCause {
    /// The user tapped on the text and that caused the selection (or the location
    /// of the cursor) to change.
    Tap,

    /// The user tapped twice in quick succession on the text and that caused
    /// the selection (or the location of the cursor) to change.
    DoubleTap,

    /// The user long-pressed the text and that caused the selection (or the
    /// location of the cursor) to change.
    LongPress,

    /// The user force-pressed the text and that caused the selection (or the
    /// location of the cursor) to change.
    ForcePress,

    /// The user used the keyboard to change the selection or the location of the
    /// cursor.
    ///
    /// Keyboard-triggered selection changes may be caused by the IME as well as
    /// by accessibility tools (e.g. TalkBack on Android).
    Keyboard,

    /// The user used the selection toolbar to change the selection or the
    /// location of the cursor.
    ///
    /// An example is when the user taps on select all in the tool bar.
    Toolbar,

    /// The user used the mouse to change the selection by dragging over a piece
    /// of text.
    Drag,

    /// The user used stylus handwriting to change the selection.
    ///
    /// Currently, this is only supported on iPadOS 14+ via the Scribble feature,
    /// or on Android API 34+ via the Scribe feature.
    StylusHandwriting,
}
