//! Flutter counterpart: `painting/text_painter.dart` (constants and
//! [`TextOverflow`] only; the painter itself is deferred).

/// The default font size if none is specified.
///
/// This should be kept in sync with the defaults set in the engine (e.g.,
/// LibTxt's text_style.h, paragraph_style.h).
pub const K_DEFAULT_FONT_SIZE: f64 = 14.0;

/// How overflowing text should be handled.
///
/// A [`TextOverflow`] can be passed to `Text` and `RichText` via their
/// overflow properties respectively.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextOverflow {
    /// Clip the overflowing text to fix its container.
    Clip,

    /// Fade the overflowing text to transparent.
    Fade,

    /// Use an ellipsis to indicate that the text has overflowed.
    Ellipsis,

    /// Render overflowing text outside of its container.
    Visible,
}
