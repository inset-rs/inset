//! Flutter counterpart: `painting/text_span.dart`.
//!
//! `semanticsLabel` / `semanticsIdentifier`, `locale`, and `spellOut` wait; see `PORTING.md`.

use std::any::Any;
use std::fmt;
use std::rc::Rc;

use inset_embedder::{ParagraphBuilder, TextAffinity, TextPosition};
use inset_foundation::App;
use inset_gestures::{
    AnyGestureRecognizer, HitTestEntry, HitTestTarget, PointerEnterEventListener, PointerEvent,
    PointerExitEventListener,
};
use inset_services::{MouseCursor, MouseCursorRef, MouseTrackerAnnotation, SystemMouseCursors};

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
///
/// Dart's `TextSpan implements HitTestTarget, MouseTrackerAnnotation`: the span is a
/// [`HitTestTarget`], and [`mouse_tracker_annotation`](Self::mouse_tracker_annotation) is the
/// annotation it implements.
#[derive(Clone, Default)]
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
    /// A gesture recognizer that will receive events that hit this span.
    ///
    /// [`InlineSpan`] itself does not implement hit testing or event dispatch. The object that
    /// manages the [`InlineSpan`] painting is also responsible for dispatching events. In the
    /// rendering library, that is the `RenderParagraph` object, which corresponds to the
    /// `RichText` widget in the widgets layer; these objects do not bubble events in
    /// [`InlineSpan`]s, so a [`recognizer`](Self::recognizer) is only effective for events that
    /// directly hit the [`text`](Self::text) of that [`InlineSpan`], not any of its
    /// [`children`](Self::children).
    ///
    /// [`InlineSpan`] also does not manage the lifetime of the gesture recognizer. The code
    /// that owns the gesture recognizer must call its `dispose` when the [`InlineSpan`] object
    /// is no longer used.
    pub recognizer: Option<AnyGestureRecognizer>,
    /// Mouse cursor when the mouse hovers over this span, as the caller gave it; read it with
    /// [`effective_mouse_cursor`](Self::effective_mouse_cursor).
    ///
    /// The default value is [`SystemMouseCursors::CLICK`] if [`recognizer`](Self::recognizer)
    /// is not `None`, or `MouseCursor.defer` otherwise.
    ///
    /// [`TextSpan`] itself does not implement hit testing or cursor changing. The object that
    /// manages the [`TextSpan`] painting is responsible to return the [`TextSpan`] in its hit
    /// test, as well as providing the correct mouse cursor when the [`TextSpan`]'s mouse cursor
    /// is `MouseCursor.defer`.
    pub mouse_cursor: Option<MouseCursorRef>,
    /// Triggered when a mouse pointer has entered this span.
    pub on_enter: Option<PointerEnterEventListener>,
    /// Triggered when a mouse pointer has exited this span.
    pub on_exit: Option<PointerExitEventListener>,
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

    /// Dart `TextSpan(recognizer:)`.
    pub fn recognizer(mut self, recognizer: AnyGestureRecognizer) -> TextSpan {
        self.recognizer = Some(recognizer);
        self
    }

    /// Dart `TextSpan(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: impl Into<MouseCursorRef>) -> TextSpan {
        self.mouse_cursor = Some(mouse_cursor.into());
        self
    }

    /// Dart `TextSpan(onEnter:)`.
    pub fn on_enter(mut self, on_enter: PointerEnterEventListener) -> TextSpan {
        self.on_enter = Some(on_enter);
        self
    }

    /// Dart `TextSpan(onExit:)`.
    pub fn on_exit(mut self, on_exit: PointerExitEventListener) -> TextSpan {
        self.on_exit = Some(on_exit);
        self
    }

    /// Dart's `mouseCursor` field, as its constructor fills it: the cursor given, else
    /// [`SystemMouseCursors::CLICK`] when there is a [`recognizer`](Self::recognizer), else
    /// `MouseCursor.defer`.
    pub fn effective_mouse_cursor(&self) -> MouseCursorRef {
        match (&self.mouse_cursor, self.recognizer) {
            (Some(mouse_cursor), _) => Rc::clone(mouse_cursor),
            (None, None) => <dyn MouseCursor>::defer(),
            (None, Some(_)) => SystemMouseCursors::CLICK.into(),
        }
    }

    /// Dart's `TextSpan implements MouseTrackerAnnotation`: the annotation this span is to the
    /// mouse tracker, with its [`on_enter`](Self::on_enter), [`on_exit`](Self::on_exit) and
    /// [`effective_mouse_cursor`](Self::effective_mouse_cursor). A span is always valid for the
    /// mouse tracker.
    pub fn mouse_tracker_annotation(&self) -> MouseTrackerAnnotation {
        MouseTrackerAnnotation {
            on_enter: self.on_enter.clone(),
            on_exit: self.on_exit.clone(),
            cursor: self.effective_mouse_cursor(),
            valid_for_mouse_tracker: true,
        }
    }

    /// The span as a shared [`InlineSpanRef`].
    pub fn into_span(self) -> InlineSpanRef {
        std::rc::Rc::new(self)
    }
}

impl HitTestTarget for TextSpan {
    fn handle_event(&self, app: &mut App, event: &PointerEvent, _entry: &HitTestEntry) {
        if let PointerEvent::Down(event) = event
            && let Some(recognizer) = self.recognizer
        {
            recognizer.add_pointer(app, event);
        }
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
    fn visit_children(self: Rc<Self>, visitor: &mut dyn FnMut(&InlineSpanRef) -> bool) -> bool {
        if self.text.is_some() && !visitor(&(self.clone() as InlineSpanRef)) {
            return false;
        }
        if let Some(children) = &self.children {
            for child in children {
                if !child.clone().visit_children(visitor) {
                    return false;
                }
            }
        }
        true
    }

    fn visit_direct_children(&self, visitor: &mut dyn FnMut(&InlineSpanRef) -> bool) -> bool {
        if let Some(children) = &self.children {
            for child in children {
                if !visitor(child) {
                    return false;
                }
            }
        }
        true
    }

    /// Returns the text span that contains the given position in the text.
    fn get_span_for_position_visitor(
        self: Rc<Self>,
        position: TextPosition,
        offset: &mut Accumulator,
    ) -> Option<InlineSpanRef> {
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
        let mut result = if self.recognizer == text_span.recognizer {
            RenderComparison::Identical
        } else {
            RenderComparison::Metadata
        };
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

    fn to_hit_test_target(self: Rc<Self>) -> Option<Rc<dyn HitTestTarget>> {
        Some(self)
    }

    fn eq_span(&self, other: &dyn InlineSpan) -> bool {
        if same_span(self, other) {
            return true;
        }
        let Some(other) = other.as_any().downcast_ref::<TextSpan>() else {
            return false;
        };
        if other.style != self.style
            || other.text != self.text
            || other.recognizer != self.recognizer
            || !same_listener(&self.on_enter, &other.on_enter)
            || !same_listener(&self.on_exit, &other.on_exit)
            || *self.effective_mouse_cursor() != *other.effective_mouse_cursor()
        {
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

/// Dart's `==` on two optional function values: the same closure, or both absent.
fn same_listener<T: ?Sized>(a: &Option<Rc<T>>, b: &Option<Rc<T>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
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
        if self.style.is_none() && self.text.is_none() && self.children.is_none() {
            s.field("(empty)", &true);
        }
        if let Some(recognizer) = &self.recognizer {
            s.field("recognizer", recognizer);
        }
        let mut callbacks = Vec::new();
        if self.on_enter.is_some() {
            callbacks.push("enter");
        }
        if self.on_exit.is_some() {
            callbacks.push("exit");
        }
        if !callbacks.is_empty() {
            s.field("callbacks", &callbacks);
        }
        let cursor = self.effective_mouse_cursor();
        if !cursor.is_defer() {
            s.field("mouse_cursor", &cursor.debug_description());
        }
        if let Some(children) = &self.children {
            s.field("children", children);
        }
        s.finish()
    }
}

#[cfg(test)]
mod tests {
    use inset_embedder::Color;

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
        let span = tree().into_span();
        let code_unit_at = |index| span.clone().code_unit_at(index);
        assert_eq!(code_unit_at(0), Some(u16::from(b'a')));
        assert_eq!(code_unit_at(2), Some(u16::from(b'c')));
        assert_eq!(code_unit_at(4), Some(u16::from(b'e')));
        assert_eq!(
            code_unit_at(5),
            Some(0xD83D),
            "the emoji's high surrogate"
        );
        assert_eq!(code_unit_at(7), None);
        assert_eq!(code_unit_at(-1), None);
    }

    #[test]
    fn span_for_position_follows_affinity_at_boundaries() {
        let span = tree().into_span();
        let hit = |offset: i32, affinity: TextAffinity| {
            span.clone()
                .get_span_for_position(TextPosition::with_affinity(offset, affinity))
                .and_then(|found| found.as_any().downcast_ref::<TextSpan>()?.text.clone())
        };
        assert_eq!(hit(1, TextAffinity::Downstream).as_deref(), Some("ab"));
        assert_eq!(hit(2, TextAffinity::Downstream).as_deref(), Some("cd"));
        assert_eq!(hit(2, TextAffinity::Upstream).as_deref(), Some("ab"));
        assert_eq!(hit(7, TextAffinity::Upstream).as_deref(), Some("e😀"));
        assert_eq!(hit(7, TextAffinity::Downstream), None);
    }

    #[test]
    fn span_for_position_returns_the_tree_s_own_span() {
        let child = TextSpan::new().text("cd").into_span();
        let root = TextSpan::new()
            .text("ab")
            .children(vec![child.clone()])
            .into_span();
        let found = root.clone().get_span_for_position(TextPosition::new(0));
        assert!(Rc::ptr_eq(&found.unwrap(), &root));
        let found = root.clone().get_span_for_position(TextPosition::new(3));
        assert!(Rc::ptr_eq(&found.unwrap(), &child));
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

    /// `text_span_test.dart`, "TextSpan equals": the callbacks compare by identity and the
    /// cursors by value.
    #[test]
    fn equality_compares_callbacks_by_identity_and_cursors_by_value() {
        let callback1: PointerEnterEventListener = Rc::new(|_app, _event| {});
        let callback2: PointerEnterEventListener = Rc::new(|_app, _event| {});
        let a1 = TextSpan::new().text("a");
        let d1 = TextSpan::new().text("a").on_enter(Rc::clone(&callback1));
        let d2 = TextSpan::new().text("a").on_enter(Rc::clone(&callback1));
        let d3 = TextSpan::new().text("a").on_enter(Rc::clone(&callback2));
        let e1 = TextSpan::new()
            .text("a")
            .on_enter(Rc::clone(&callback2))
            .mouse_cursor(SystemMouseCursors::FORBIDDEN);
        let e2 = TextSpan::new()
            .text("a")
            .on_enter(Rc::clone(&callback2))
            .mouse_cursor(SystemMouseCursors::FORBIDDEN);

        assert!(!a1.eq_span(&d1));
        assert!(d1.eq_span(&d2));
        assert!(!d2.eq_span(&d3));
        assert!(!d3.eq_span(&e1));
        assert!(e1.eq_span(&e2));
    }

    /// A span with a recognizer shows the click cursor unless it names one, and a span without
    /// defers; a change of recognizer alone is metadata.
    #[test]
    fn the_recognizer_sets_the_default_cursor() {
        use inset_foundation::AppCell;
        use inset_gestures::{GestureRecognizerLeaf, TapGestureRecognizer};

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let first = TapGestureRecognizer::new(&mut app).as_recognizer();
        let second = TapGestureRecognizer::new(&mut app).as_recognizer();
        let click: MouseCursorRef = SystemMouseCursors::CLICK.into();
        let forbidden: MouseCursorRef = SystemMouseCursors::FORBIDDEN.into();

        let link = TextSpan::new().text("a").recognizer(first);
        assert!(*link.effective_mouse_cursor() == *click);
        assert!(
            TextSpan::new()
                .text("a")
                .effective_mouse_cursor()
                .is_defer()
        );
        let named = TextSpan::new()
            .text("a")
            .recognizer(first)
            .mouse_cursor(SystemMouseCursors::FORBIDDEN);
        assert!(*named.effective_mouse_cursor() == *forbidden);
        assert!(*link.mouse_tracker_annotation().cursor == *click);

        let relinked = TextSpan::new().text("a").recognizer(second);
        assert_eq!(link.compare_to(&relinked), RenderComparison::Metadata);
        assert!(!link.eq_span(&relinked));
        assert!(
            Rc::new(link).to_hit_test_target().is_some(),
            "a text span is a target"
        );
        first.dispose(&mut app);
        second.dispose(&mut app);
    }
}
