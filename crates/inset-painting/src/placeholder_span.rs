//! Flutter counterpart: `painting/placeholder_span.dart`.

use inset_embedder::{PlaceholderAlignment, TextBaseline};

use crate::inline_span::InlineSpan;

/// The unicode character to represent a placeholder (Dart's
/// `PlaceholderSpan.placeholderCodeUnit`).
pub const PLACEHOLDER_CODE_UNIT: u16 = 0xFFFC;

/// An immutable placeholder that is embedded inline within text.
///
/// [`PlaceholderSpan`] represents a placeholder that acts as a stand-in for other content. A
/// [`PlaceholderSpan`] by itself does not contain useful information to change a `TextSpan`.
/// `WidgetSpan` from the widgets library extends [`PlaceholderSpan`] and may be used instead
/// to specify a widget as the contents of the placeholder.
///
/// Flutter widgets such as `TextField`, `Text` and `RichText` do not recognize
/// [`PlaceholderSpan`] subclasses other than `WidgetSpan`. **Consider implementing the
/// `WidgetSpan` interface instead of the `Placeholder` interface.**
///
/// A [`TextStyle`](crate::TextStyle) may be provided with the
/// [`style`](InlineSpan::style) property, but only the decoration, foreground, background,
/// and spacing options will be used.
///
/// See also:
///
///  * `WidgetSpan`, a leaf node that represents an embedded inline widget.
///  * `TextSpan`, a node that represents text in a `TextSpan` tree.
///  * `Text`, a widget for showing uniformly-styled text.
///  * `RichText`, a widget for finer control of text rendering.
///  * [`TextPainter`](crate::TextPainter), a class for painting `TextSpan` objects on a
///    `Canvas`.
pub trait PlaceholderSpan: InlineSpan {
    /// How the placeholder aligns vertically with the text.
    ///
    /// See [`PlaceholderAlignment`] for details on each mode.
    fn alignment(&self) -> PlaceholderAlignment;

    /// The [`TextBaseline`] to align against when using [`PlaceholderAlignment::Baseline`],
    /// [`PlaceholderAlignment::AboveBaseline`], and [`PlaceholderAlignment::BelowBaseline`].
    ///
    /// This is ignored when using other alignment modes.
    fn baseline(&self) -> Option<TextBaseline>;

    /// [`PlaceholderSpan`]s are flattened to a `0xFFFC` object replacement character in the
    /// plain text representation when `include_placeholders` is true.
    fn compute_to_plain_text(
        &self,
        buffer: &mut String,
        include_semantics_labels: bool,
        include_placeholders: bool,
    ) {
        let _ = include_semantics_labels;
        if include_placeholders {
            buffer.push('\u{FFFC}');
        }
    }

    /// In debug mode, throws an exception: a placeholder that is not a `WidgetSpan` is not
    /// supported.
    fn debug_assert_is_valid(&self) -> bool {
        debug_assert!(
            false,
            "Consider implementing the WidgetSpan interface instead."
        );
        true
    }
}
