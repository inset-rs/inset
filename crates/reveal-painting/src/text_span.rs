//! Flutter counterpart: `painting/text_span.dart`.
//!
//! `recognizer`, `mouseCursor`, `onEnter` / `onExit` (the span as a hit-test target and mouse
//! tracker annotation), `semanticsLabel` / `semanticsIdentifier`, `locale`, and `spellOut`
//! wait; see `PORTING.md`.

use std::any::Any;
use std::fmt;

use reveal_embedder::{ParagraphBuilder, TextAffinity, TextPosition};

use crate::basic_types::RenderComparison;
use crate::inline_span::{Accumulator, InlineSpan, InlineSpanRef, same_span, utf16_len};
use crate::text_painter::PlaceholderDimensions;
use crate::text_scaler::TextScaler;
use crate::text_style::TextStyle;

/// An immutable span of text.
///
/// A [`TextSpan`] object can be styled using its [`style`](Self::style) property. The style
/// will be applied to the [`text`](Self::text) and the [`children`](Self::children).
///
/// A [`TextSpan`] object can just have plain text, or it can have children [`TextSpan`]
/// objects with their own styles that (in turn) can have their own children, and so on. The
/// [`text`](Self::text) of a span is rendered before its children.
///
/// Dart's named constructor arguments are the fluent setters (`TextSpan::new().text("x")`).
#[derive(Default)]
pub struct TextSpan {
    /// The text contained in this span.
    ///
    /// If both [`text`](Self::text) and [`children`](Self::children) are set, the text will
    /// precede the children.
    pub text: Option<String>,
    /// Additional spans to include as children.
    ///
    /// If both [`text`](Self::text) and [`children`](Self::children) are set, the text will
    /// precede the children.
    pub children: Option<Vec<InlineSpanRef>>,
    /// The [`TextStyle`] to apply to this span and its children.
    pub style: Option<TextStyle>,
}

impl TextSpan {
    /// Creates an empty [`TextSpan`]; set the fields with the fluent setters.
    pub fn new() -> TextSpan {
        TextSpan::default()
    }

    /// Dart `TextSpan(text:)`.
    pub fn text(mut self, text: impl Into<String>) -> TextSpan {
        self.text = Some(text.into());
        self
    }

    /// Dart `TextSpan(children:)`.
    pub fn children(mut self, children: Vec<InlineSpanRef>) -> TextSpan {
        self.children = Some(children);
        self
    }

    /// Dart `TextSpan(style:)`.
    pub fn style(mut self, style: TextStyle) -> TextSpan {
        self.style = Some(style);
        self
    }

    /// The span as a shared [`InlineSpanRef`].
    pub fn into_span(self) -> InlineSpanRef {
        std::rc::Rc::new(self)
    }
}

impl InlineSpan for TextSpan {
    fn style(&self) -> Option<&TextStyle> {
        self.style.as_ref()
    }

    /// Apply the [`style`](Self::style), [`text`](Self::text), and
    /// [`children`](Self::children) of this object to the given [`ParagraphBuilder`], from
    /// which a `Paragraph` can be obtained.
    ///
    /// Rather than using this directly, it's simpler to use the `TextPainter` class to paint
    /// [`TextSpan`] objects onto `Canvas` objects.
    fn build(
        &self,
        builder: &mut ParagraphBuilder,
        text_scaler: &TextScaler,
        dimensions: Option<&[PlaceholderDimensions]>,
    ) {
        debug_assert!(self.debug_assert_is_valid());
        let has_style = self.style.is_some();
        if let Some(style) = &self.style {
            builder.push_style(style.get_text_style_with(text_scaler));
        }
        if let Some(text) = &self.text {
            builder.add_text(text);
        }
        if let Some(children) = &self.children {
            for child in children {
                child.build(builder, text_scaler, dimensions);
            }
        }
        if has_style {
            builder.pop();
        }
    }

    /// Walks this [`TextSpan`] and its descendants in pre-order and calls `visitor` for
    /// each span that has text.
    ///
    /// When `visitor` returns true, the walk will continue. When `visitor` returns false,
    /// then the walk will end.
    fn visit_children<'a>(&'a self, visitor: &mut dyn FnMut(&'a dyn InlineSpan) -> bool) -> bool {
        if self.text.is_some() && !visitor(self) {
            return false;
        }
        if let Some(children) = &self.children {
            for child in children {
                if !child.visit_children(visitor) {
                    return false;
                }
            }
        }
        true
    }

    fn visit_direct_children<'a>(
        &'a self,
        visitor: &mut dyn FnMut(&'a dyn InlineSpan) -> bool,
    ) -> bool {
        if let Some(children) = &self.children {
            for child in children {
                if !visitor(&**child) {
                    return false;
                }
            }
        }
        true
    }

    /// Returns the text span that contains the given position in the text.
    fn get_span_for_position_visitor(
        &self,
        position: TextPosition,
        offset: &mut Accumulator,
    ) -> Option<&dyn InlineSpan> {
        let text = self.text.as_deref()?;
        if text.is_empty() {
            return None;
        }
        let affinity = position.affinity;
        let target_offset = position.offset;
        let end_offset = offset.value() + utf16_len(text);
        if (offset.value() == target_offset && affinity == TextAffinity::Downstream)
            || (offset.value() < target_offset && target_offset < end_offset)
            || (end_offset == target_offset && affinity == TextAffinity::Upstream)
        {
            return Some(self);
        }
        offset.increment(utf16_len(text));
        None
    }

    fn compute_to_plain_text(
        &self,
        buffer: &mut String,
        include_semantics_labels: bool,
        include_placeholders: bool,
    ) {
        debug_assert!(self.debug_assert_is_valid());
        if let Some(text) = &self.text {
            buffer.push_str(text);
        }
        if let Some(children) = &self.children {
            for child in children {
                child.compute_to_plain_text(buffer, include_semantics_labels, include_placeholders);
            }
        }
    }

    fn code_unit_at_visitor(&self, index: i32, offset: &mut Accumulator) -> Option<u16> {
        let text = self.text.as_deref()?;
        let local_offset = index - offset.value();
        debug_assert!(local_offset >= 0);
        let length = utf16_len(text);
        offset.increment(length);
        if local_offset < length {
            text.encode_utf16().nth(local_offset as usize)
        } else {
            None
        }
    }

    /// In debug mode, throws an exception if the object is not in a valid configuration.
    /// Otherwise, returns true.
    ///
    /// This is intended to be used as follows: `debug_assert!(span.debug_assert_is_valid())`.
    fn debug_assert_is_valid(&self) -> bool {
        if cfg!(debug_assertions)
            && let Some(children) = &self.children
        {
            for child in children {
                debug_assert!(child.debug_assert_is_valid());
            }
        }
        true
    }

    fn compare_to(&self, other: &dyn InlineSpan) -> RenderComparison {
        if same_span(self, other) {
            return RenderComparison::Identical;
        }
        let Some(text_span) = other.as_any().downcast_ref::<TextSpan>() else {
            return RenderComparison::Layout;
        };
        if text_span.text != self.text
            || self.children.as_ref().map(Vec::len) != text_span.children.as_ref().map(Vec::len)
            || self.style.is_none() != text_span.style.is_none()
        {
            return RenderComparison::Layout;
        }
        let mut result = RenderComparison::Identical;
        if let (Some(style), Some(other_style)) = (&self.style, &text_span.style) {
            let candidate = style.compare_to(other_style);
            if candidate > result {
                result = candidate;
            }
            if result == RenderComparison::Layout {
                return result;
            }
        }
        if let (Some(children), Some(other_children)) = (&self.children, &text_span.children) {
            for (child, other_child) in children.iter().zip(other_children) {
                let candidate = child.compare_to(&**other_child);
                if candidate > result {
                    result = candidate;
                }
                if result == RenderComparison::Layout {
                    return result;
                }
            }
        }
        result
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_span(&self, other: &dyn InlineSpan) -> bool {
        if same_span(self, other) {
            return true;
        }
        let Some(other) = other.as_any().downcast_ref::<TextSpan>() else {
            return false;
        };
        if other.style != self.style || other.text != self.text {
            return false;
        }
        match (&self.children, &other.children) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.eq_span(&**y))
            }
            _ => false,
        }
    }
}

impl fmt::Debug for TextSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("TextSpan");
        if let Some(text) = &self.text {
            s.field("text", text);
        }
        if let Some(style) = &self.style {
            s.field("style", style);
        }
        if let Some(children) = &self.children {
            s.field("children", children);
        }
        if self.style.is_none() && self.text.is_none() && self.children.is_none() {
            s.field("(empty)", &true);
        }
        s.finish()
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::Color;

    use super::*;

    fn tree() -> TextSpan {
        TextSpan::new().text("ab").children(vec![
            TextSpan::new().text("cd").into_span(),
            TextSpan::new()
                .children(vec![TextSpan::new().text("e😀").into_span()])
                .into_span(),
        ])
    }

    #[test]
    fn plain_text_concatenates_text_before_children() {
        assert_eq!(tree().to_plain_text(true, true), "abcde😀");
    }

    #[test]
    fn code_units_are_utf16_across_the_tree() {
        let span = tree();
        assert_eq!(span.code_unit_at(0), Some(u16::from(b'a')));
        assert_eq!(span.code_unit_at(2), Some(u16::from(b'c')));
        assert_eq!(span.code_unit_at(4), Some(u16::from(b'e')));
        assert_eq!(
            span.code_unit_at(5),
            Some(0xD83D),
            "the emoji's high surrogate"
        );
        assert_eq!(span.code_unit_at(7), None);
        assert_eq!(span.code_unit_at(-1), None);
    }

    #[test]
    fn span_for_position_follows_affinity_at_boundaries() {
        let span = tree();
        let hit = |offset: i32, affinity: TextAffinity| {
            span.get_span_for_position(TextPosition::with_affinity(offset, affinity))
                .and_then(|found| found.as_any().downcast_ref::<TextSpan>())
                .and_then(|found| found.text.clone())
        };
        assert_eq!(hit(1, TextAffinity::Downstream).as_deref(), Some("ab"));
        assert_eq!(hit(2, TextAffinity::Downstream).as_deref(), Some("cd"));
        assert_eq!(hit(2, TextAffinity::Upstream).as_deref(), Some("ab"));
        assert_eq!(hit(7, TextAffinity::Upstream).as_deref(), Some("e😀"));
        assert_eq!(hit(7, TextAffinity::Downstream), None);
    }

    #[test]
    fn compare_to_ranks_the_change() {
        let a = tree();
        assert_eq!(a.compare_to(&a), RenderComparison::Identical);
        assert_eq!(a.compare_to(&tree()), RenderComparison::Identical);
        let recolored = TextSpan::new()
            .text("ab")
            .style(TextStyle::new().color(Color::from_argb(255, 255, 0, 0)));
        let resized = TextSpan::new()
            .text("ab")
            .style(TextStyle::new().color(Color::from_argb(255, 0, 0, 255)));
        assert_eq!(recolored.compare_to(&resized), RenderComparison::Paint);
        let relaid = TextSpan::new()
            .text("ab")
            .style(TextStyle::new().font_size(30.0));
        assert_eq!(recolored.compare_to(&relaid), RenderComparison::Layout);
        assert_eq!(
            a.compare_to(&TextSpan::new().text("ab")),
            RenderComparison::Layout
        );
    }

    #[test]
    fn equality_is_deep() {
        let a: InlineSpanRef = tree().into_span();
        let b: InlineSpanRef = tree().into_span();
        assert!(*a == *b);
        let c: InlineSpanRef = TextSpan::new().text("ab").into_span();
        assert!(*a != *c);
    }
}
