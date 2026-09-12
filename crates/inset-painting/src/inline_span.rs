//! Flutter counterpart: `painting/inline_span.dart`.
//!
//! `InlineSpanSemanticsInformation` and `computeSemanticsInformation` wait with the rest of
//! accessibility.

use std::any::Any;
use std::fmt::Debug;
use std::rc::Rc;

use inset_embedder::{ParagraphBuilder, TextPosition};

use crate::basic_types::RenderComparison;
use crate::text_painter::PlaceholderDimensions;
use crate::text_scaler::TextScaler;
use crate::text_style::TextStyle;

/// Mutable wrapper of an integer that can be passed by reference to track a value across a
/// recursive stack.
#[derive(Debug, Default)]
pub struct Accumulator {
    value: i32,
}

impl Accumulator {
    /// Creates an [`Accumulator`] starting at `value`.
    pub fn new(value: i32) -> Accumulator {
        Accumulator { value }
    }

    /// The integer stored in this [`Accumulator`].
    pub fn value(&self) -> i32 {
        self.value
    }

    /// Increases the [`value`](Self::value) by the `addend`.
    pub fn increment(&mut self, addend: i32) {
        debug_assert!(addend >= 0);
        self.value += addend;
    }
}

/// A shared, immutable span (Dart's `InlineSpan` reference).
pub type InlineSpanRef = Rc<dyn InlineSpan>;

/// An immutable span of inline content which forms part of a paragraph.
///
///  * The subclass `TextSpan` specifies text and may contain child [`InlineSpan`]s.
///  * `PlaceholderSpan` (deferred) represents a placeholder that may be filled with
///    non-text content.
///
/// Spans are shared as [`InlineSpanRef`]. A visitor receives `&dyn InlineSpan` with the tree's
/// lifetime, so [`get_span_for_position`](Self::get_span_for_position) can hand a span back.
pub trait InlineSpan: Debug {
    /// The [`TextStyle`] to apply to this span.
    ///
    /// The [`style`](Self::style) is also applied to any child spans when this is an instance
    /// of `TextSpan`.
    fn style(&self) -> Option<&TextStyle>;

    /// Apply the properties of this object to the given [`ParagraphBuilder`], from which a
    /// `Paragraph` can be obtained.
    ///
    /// The `text_scaler` parameter specifies a [`TextScaler`] that the text and placeholders
    /// will be scaled by. The scaling is performed before layout, so the text will be laid
    /// out with the scaled glyphs and placeholders.
    ///
    /// The `dimensions` parameter specifies the sizes of the placeholders. Each
    /// `PlaceholderSpan` must be paired with a [`PlaceholderDimensions`] in the same order as
    /// defined in the [`InlineSpan`] tree.
    ///
    /// `Paragraph` objects can be drawn on `Canvas` objects.
    fn build(
        &self,
        builder: &mut ParagraphBuilder,
        text_scaler: &TextScaler,
        dimensions: Option<&[PlaceholderDimensions]>,
    );

    /// Walks this [`InlineSpan`] and any descendants in pre-order and calls `visitor` for
    /// each span that has content.
    ///
    /// When `visitor` returns true, the walk will continue. When `visitor` returns false,
    /// then the walk will end.
    fn visit_children<'a>(&'a self, visitor: &mut dyn FnMut(&'a dyn InlineSpan) -> bool) -> bool;

    /// Calls `visitor` for each immediate child of this [`InlineSpan`].
    ///
    /// The immediate children are visited in the same order they are added to a
    /// [`ParagraphBuilder`] in the [`build`](Self::build) method, which is also the logical
    /// order of the child [`InlineSpan`]s in the text.
    ///
    /// The traversal stops when all immediate children are visited, or when `visitor` returns
    /// false. When `visitor` returns false, the walk will end.
    fn visit_direct_children<'a>(
        &'a self,
        visitor: &mut dyn FnMut(&'a dyn InlineSpan) -> bool,
    ) -> bool;

    /// Returns the [`InlineSpan`] that contains the given position in the text.
    fn get_span_for_position(&self, position: TextPosition) -> Option<&dyn InlineSpan> {
        debug_assert!(self.debug_assert_is_valid());
        let mut offset = Accumulator::default();
        let mut result: Option<&dyn InlineSpan> = None;
        self.visit_children(&mut |span| {
            result = span.get_span_for_position_visitor(position, &mut offset);
            result.is_none()
        });
        result
    }

    /// Performs the check at each [`InlineSpan`] for if the `position` falls within the
    /// range.
    ///
    /// The `offset` parameter tracks the current index offset in the text buffer formed if
    /// the contents of the [`InlineSpan`] tree were concatenated together starting from the
    /// root [`InlineSpan`].
    ///
    /// This method should not be directly called. Use [`get_span_for_position`] instead.
    ///
    /// [`get_span_for_position`]: Self::get_span_for_position
    fn get_span_for_position_visitor(
        &self,
        position: TextPosition,
        offset: &mut Accumulator,
    ) -> Option<&dyn InlineSpan>;

    /// Flattens the [`InlineSpan`] tree into a single string.
    ///
    /// Styles are not honored in this process. If `include_semantics_labels` is true, then
    /// the text returned will include the semantics labels over the text. If
    /// `include_placeholders` is true, then the placeholder character will be included for
    /// placeholders.
    fn to_plain_text(&self, include_semantics_labels: bool, include_placeholders: bool) -> String {
        let mut buffer = String::new();
        self.compute_to_plain_text(&mut buffer, include_semantics_labels, include_placeholders);
        buffer
    }

    /// Walks the [`InlineSpan`] tree and writes the plain text representation to the
    /// `buffer`.
    ///
    /// This method should not be directly called. Use [`to_plain_text`] instead.
    ///
    /// [`to_plain_text`]: Self::to_plain_text
    fn compute_to_plain_text(
        &self,
        buffer: &mut String,
        include_semantics_labels: bool,
        include_placeholders: bool,
    );

    /// Returns the UTF-16 code unit at the given `index` in the flattened string.
    ///
    /// This only accounts for the `TextSpan.text` values and ignores `PlaceholderSpan`s.
    ///
    /// Returns `None` if the `index` is out of bounds.
    fn code_unit_at(&self, index: i32) -> Option<u16> {
        if index < 0 {
            return None;
        }
        let mut offset = Accumulator::default();
        let mut result = None;
        self.visit_children(&mut |span| {
            result = span.code_unit_at_visitor(index, &mut offset);
            result.is_none()
        });
        result
    }

    /// Performs the check at each [`InlineSpan`] for if the `index` falls within the range
    /// of the span and returns the corresponding code unit. Returns `None` otherwise.
    ///
    /// The `offset` parameter tracks the current index offset in the text buffer formed if
    /// the contents of the [`InlineSpan`] tree were concatenated together starting from the
    /// root [`InlineSpan`].
    ///
    /// This method should not be directly called. Use [`code_unit_at`] instead.
    ///
    /// [`code_unit_at`]: Self::code_unit_at
    fn code_unit_at_visitor(&self, index: i32, offset: &mut Accumulator) -> Option<u16>;

    /// In debug mode, throws an exception if the object is not in a valid configuration.
    /// Otherwise, returns true.
    ///
    /// This is intended to be used as follows: `debug_assert!(span.debug_assert_is_valid())`.
    fn debug_assert_is_valid(&self) -> bool {
        true
    }

    /// Describe the difference between this span and another, in terms of how much damage
    /// it will make to the rendering. The comparison is deep.
    ///
    /// Comparing [`InlineSpan`] objects of different types, for example, comparing a
    /// `TextSpan` to a `WidgetSpan`, always results in [`RenderComparison::Layout`].
    fn compare_to(&self, other: &dyn InlineSpan) -> RenderComparison;

    /// Downcast support for Dart's `runtimeType` and `is` checks.
    fn as_any(&self) -> &dyn Any;

    /// Dart's `==`: deep equality on the same span class.
    fn eq_span(&self, other: &dyn InlineSpan) -> bool;
}

impl PartialEq for dyn InlineSpan {
    fn eq(&self, other: &dyn InlineSpan) -> bool {
        self.eq_span(other)
    }
}

/// Dart's `identical(a, b)` on two span references.
pub fn same_span(a: &dyn InlineSpan, b: &dyn InlineSpan) -> bool {
    std::ptr::addr_eq(a as *const dyn InlineSpan, b as *const dyn InlineSpan)
}

/// The number of UTF-16 code units in `text` (Dart's `String.length`).
pub(crate) fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}
