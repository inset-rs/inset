//! Flutter counterpart: `painting/text_painter.dart`.
//!
//! `wordBoundaries` sits with `RenderEditable` in rendering, which owns the laid-out
//! paragraph it reads; `strutStyle` and `locale` wait. See `PORTING.md`.

use std::cell::OnceCell;

use reveal_embedder::{
    BoxHeightStyle, BoxWidthStyle, Canvas, CanvasText, Color, FontCollection, GlyphInfo,
    LineMetrics, Offset, Paint, PaintStyle, Paragraph, ParagraphBuilder, ParagraphConstraints,
    ParagraphStyle, Rect, Size, Stroke, TextAffinity, TextAlign, TextBaseline, TextBox,
    TextDirection, TextHeightBehavior, TextPosition, TextRange, TextSelection, clamp_double,
};
use reveal_foundation::PRECISION_ERROR_TOLERANCE;

use crate::basic_types::RenderComparison;
use crate::inline_span::{InlineSpanRef, utf16_len};
use crate::text_scaler::TextScaler;
use crate::text_style::TextStyle;

/// The default font size if none is specified.
///
/// This should be consistent with the default font size in `dart:ui`.
pub const K_DEFAULT_FONT_SIZE: f64 = 14.0;

/// How overflowing text should be handled.
///
/// A [`TextOverflow`] can be passed to `Text` and `RichText` via their `overflow` properties,
/// but they really work when the text will overflow its container. For example, they require
/// specifying the `maxLines` property, or wrapping the text in a `SizedBox` with a fixed
/// height.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextOverflow {
    /// Clip the overflowing text to fix its container.
    Clip,

    /// Fade the overflowing text to transparent.
    Fade,

    /// Use an ellipsis to indicate that the text has overflowed.
    Ellipsis,

    /// Render overflowing text outside of its box.
    Visible,
}

/// Holds the [`Size`] and baseline required to represent the dimensions of a placeholder
/// in text.
///
/// Placeholders specify an empty space in the text layout, which is used to later render
/// arbitrary inline widgets into defined by a `WidgetSpan`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaceholderDimensions {
    /// Width and height dimensions of the placeholder.
    pub size: Size,
    /// How the placeholder vertically aligns with the text.
    pub alignment: reveal_embedder::PlaceholderAlignment,
    /// Distance of the [`alignment`](Self::alignment) from the top of the placeholder.
    pub baseline_offset: Option<f64>,
    /// The [`TextBaseline`] to align against when using
    /// `PlaceholderAlignment::Baseline`, `AboveBaseline`, and `BelowBaseline`.
    pub baseline: Option<TextBaseline>,
}

impl PlaceholderDimensions {
    /// A constant representing an empty placeholder.
    pub const EMPTY: PlaceholderDimensions = PlaceholderDimensions {
        size: Size::ZERO,
        alignment: reveal_embedder::PlaceholderAlignment::Bottom,
        baseline_offset: None,
        baseline: None,
    };

    /// Constructs a [`PlaceholderDimensions`] object.
    pub const fn new(
        size: Size,
        alignment: reveal_embedder::PlaceholderAlignment,
    ) -> PlaceholderDimensions {
        PlaceholderDimensions {
            size,
            alignment,
            baseline_offset: None,
            baseline: None,
        }
    }
}

/// The different ways of measuring the width of one or more lines of text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextWidthBasis {
    /// Multiline text will take up the full width given by the parent. For single line text,
    /// only the minimum amount of width needed to contain the text will be used. A common use
    /// case for this is a standard series of paragraphs.
    #[default]
    Parent,

    /// The width will be exactly enough to contain the longest line and no longer. A common
    /// use case for this is chat bubbles.
    LongestLine,
}

/// Whether `code_point` is one of the newline code points the paragraph breaks on.
fn is_newline(code_point: u32) -> bool {
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

/// A laid-out paragraph and the direction it was laid out in (Dart's `_TextLayout`).
struct TextLayout {
    paragraph: Paragraph,
    writing_direction: TextDirection,
    end_of_text_caret_metrics: OnceCell<LineCaretMetrics>,
}

impl TextLayout {
    fn new(paragraph: Paragraph, writing_direction: TextDirection) -> TextLayout {
        TextLayout {
            paragraph,
            writing_direction,
            end_of_text_caret_metrics: OnceCell::new(),
        }
    }

    fn height(&self) -> f64 {
        self.paragraph.height()
    }

    fn min_intrinsic_line_extent(&self) -> f64 {
        self.paragraph.min_intrinsic_width()
    }

    fn max_intrinsic_line_extent(&self) -> f64 {
        self.paragraph.max_intrinsic_width()
    }

    fn longest_line(&self) -> f64 {
        self.paragraph.longest_line()
    }

    fn get_distance_to_baseline(&self, baseline: TextBaseline) -> f64 {
        match baseline {
            TextBaseline::Alphabetic => self.paragraph.alphabetic_baseline(),
            TextBaseline::Ideographic => self.paragraph.ideographic_baseline(),
        }
    }

    /// The caret metrics for the end of the text, anchored at the last line's baseline.
    fn end_of_text_caret_metrics(&self, plain_text: &str) -> LineCaretMetrics {
        *self
            .end_of_text_caret_metrics
            .get_or_init(|| self.compute_end_of_text_caret_anchor_offset(plain_text))
    }

    fn compute_end_of_text_caret_anchor_offset(&self, raw_string: &str) -> LineCaretMetrics {
        let last_line_index = self.paragraph.number_of_lines() - 1;
        debug_assert!(last_line_index >= 0);
        let line_metrics = self
            .paragraph
            .get_line_metrics_at(last_line_index)
            .expect("the last line exists");
        let last_code_unit = raw_string.encode_utf16().last().unwrap_or(0);
        let has_trailing_spaces = match last_code_unit {
            0x0009 => true,                    // horizontal tab
            0x00A0 | 0x2007 | 0x202F => false, // no-break, figure, narrow no-break space
            unit => char::from_u32(u32::from(unit)).is_some_and(is_space_separator),
        };
        let baseline = line_metrics.baseline;
        let last_glyph = self.paragraph.get_glyph_info_at(utf16_len(raw_string) - 1);
        let (dx, height) = match last_glyph {
            Some(last_glyph) if has_trailing_spaces => {
                let glyph_bounds = last_glyph.grapheme_cluster_layout_bounds;
                debug_assert!(!glyph_bounds.is_empty());
                let dx = match self.writing_direction {
                    TextDirection::Ltr => glyph_bounds.right,
                    TextDirection::Rtl => glyph_bounds.left,
                };
                (dx, glyph_bounds.height())
            }
            _ => {
                let dx = match self.writing_direction {
                    TextDirection::Ltr => line_metrics.left + line_metrics.width,
                    TextDirection::Rtl => line_metrics.left,
                };
                (dx, line_metrics.height)
            }
        };
        LineCaretMetrics {
            offset: Offset::new(dx, baseline),
            writing_direction: self.writing_direction,
            height,
        }
    }

    fn content_width_for(
        &self,
        min_width: f64,
        max_width: f64,
        width_basis: TextWidthBasis,
    ) -> f64 {
        match width_basis {
            TextWidthBasis::LongestLine => clamp_double(self.longest_line(), min_width, max_width),
            TextWidthBasis::Parent => {
                clamp_double(self.max_intrinsic_line_extent(), min_width, max_width)
            }
        }
    }
}

/// Unicode `Space_Separator` (Zs).
fn is_space_separator(ch: char) -> bool {
    matches!(
        ch,
        '\u{0020}' | '\u{00A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
    )
}

/// This struct stores the current text layout and the corresponding `text_align` and
/// `content_width`.
///
/// This class also does some cache invalidation.
struct TextPainterLayoutCacheWithOffset {
    layout: TextLayout,
    /// The input width used to lay out the paragraph.
    layout_max_width: f64,
    /// The content width the text painter should report in [`TextPainter::width`].
    /// This is also used to compute the `paint_offset`.
    content_width: f64,
    /// The effective text alignment in the [`TextPainter`]'s canvas. The value is within the
    /// `[0, 1]` interval: 0 for left aligned and 1 for right aligned.
    text_alignment: f64,
    cached_inline_placeholder_boxes: Option<Vec<TextBox>>,
    cached_line_metrics: Option<Vec<LineMetrics>>,
    /// Used to determine whether the caret metrics cache should be invalidated.
    previous_caret_position_key: Option<i32>,
}

impl TextPainterLayoutCacheWithOffset {
    fn new(
        layout: TextLayout,
        text_alignment: f64,
        layout_max_width: f64,
        content_width: f64,
    ) -> TextPainterLayoutCacheWithOffset {
        debug_assert!((0.0..=1.0).contains(&text_alignment));
        debug_assert!(!layout_max_width.is_nan());
        debug_assert!(!content_width.is_nan());
        TextPainterLayoutCacheWithOffset {
            layout,
            layout_max_width,
            content_width,
            text_alignment,
            cached_inline_placeholder_boxes: None,
            cached_line_metrics: None,
            previous_caret_position_key: None,
        }
    }

    /// The paint offset that should be applied to the paragraph before painting.
    fn paint_offset(&self) -> Offset {
        if self.text_alignment == 0.0 {
            return Offset::ZERO;
        }
        if !self.paragraph().width().is_finite() {
            return Offset::new(f64::INFINITY, 0.0);
        }
        let dx = self.text_alignment * (self.content_width - self.paragraph().width());
        debug_assert!(!dx.is_nan());
        Offset::new(dx, 0.0)
    }

    fn paragraph(&self) -> &Paragraph {
        &self.layout.paragraph
    }

    // Try to resize the current paragraph to fit the new input width range, without
    // re-laying out the paragraph.
    //
    // Returns true if the resizing was successful.
    fn resize_to_fit(
        &mut self,
        min_width: f64,
        max_width: f64,
        width_basis: TextWidthBasis,
    ) -> bool {
        debug_assert!(self.layout.max_intrinsic_line_extent().is_finite());
        debug_assert!(min_width <= max_width);
        // The assumption here is that if the length of the paragraph is
        // already the same as the maxWidth, the paragraph's width can't be
        // adjusted.
        if max_width == self.content_width && min_width == self.content_width {
            self.content_width = self
                .layout
                .content_width_for(min_width, max_width, width_basis);
            return true;
        }

        // Special case:
        // When the paint offset and the paragraph width are both +∞, it's likely
        // that the text layout engine skipped layout because there weren't anything
        // to paint. Override the paragraph width with paragraph max width to
        // avoid infinite paint offsets.
        if !self.paint_offset().dx().is_finite()
            && !self.paragraph().width().is_finite()
            && min_width.is_finite()
        {
            debug_assert!(self.paint_offset().dx() == f64::INFINITY);
            debug_assert!(self.paragraph().width() == f64::INFINITY);
            return false;
        }

        let max_intrinsic_width = self.paragraph().max_intrinsic_width();
        // Skip line breaking if the input width remains the same, of there will be
        // no soft breaks.
        let skip_line_breaking = max_width == self.layout_max_width // Same input max width so relayout is unnecessary.
            || ((self.paragraph().width() - max_intrinsic_width) > -PRECISION_ERROR_TOLERANCE
                && (max_width - max_intrinsic_width) > -PRECISION_ERROR_TOLERANCE);
        if skip_line_breaking {
            // Adjust width to the same input width.
            self.content_width = self
                .layout
                .content_width_for(min_width, max_width, width_basis);
            return true;
        }
        false
    }

    // ---- Cached values whose life cycle ends when the paragraph is relaid out ----

    fn inline_placeholder_boxes(&mut self) -> &[TextBox] {
        self.cached_inline_placeholder_boxes
            .get_or_insert_with(|| self.layout.paragraph.get_boxes_for_placeholders())
    }

    fn line_metrics(&mut self) -> &[LineMetrics] {
        self.cached_line_metrics
            .get_or_insert_with(|| self.layout.paragraph.compute_line_metrics())
    }
}

/// The _CaretMetrics for carets located in a non-empty paragraph. Such carets are anchored
/// to the trailing edge or the leading edge of a glyph, or a ligature component.
#[derive(Clone, Copy, Debug, PartialEq)]
struct LineCaretMetrics {
    /// The offset from the top left corner of the paragraph to the caret's top start
    /// location.
    offset: Offset,
    /// The writing direction of the glyph the caret is anchored to. This is used to compute
    /// the caret's rect.
    writing_direction: TextDirection,
    /// The full height of the glyph at the caret position.
    height: f64,
}

impl LineCaretMetrics {
    fn shift(&self, offset: Offset) -> LineCaretMetrics {
        if offset == Offset::ZERO {
            *self
        } else {
            LineCaretMetrics {
                offset: offset + self.offset,
                writing_direction: self.writing_direction,
                height: self.height,
            }
        }
    }
}

/// An object that paints a `TextSpan` tree into a `Canvas`.
///
/// To use a [`TextPainter`], follow these steps:
///
/// 1. Create a `TextSpan` tree and pass it to the [`TextPainter`] with
///    [`set_text`](Self::set_text); set [`set_text_direction`](Self::set_text_direction).
/// 2. Call [`layout`](Self::layout) to prepare the paragraph.
/// 3. Call [`paint`](Self::paint) as often as desired to paint the paragraph.
/// 4. Call [`dispose`](Self::dispose) when the object will no longer be accessed to release
///    native resources. For [`TextPainter`] objects that are used repeatedly and stored on a
///    `State` or `RenderObject`, call [`dispose`](Self::dispose) from `State::dispose` or
///    `RenderObject::dispose` or similar. For those used only ephemerally, it is sufficient
///    to immediately dispose them after use.
///
/// If the width of the area into which the text is being painted changes, return to step 2.
/// If the text to be painted changes, return to step 1.
///
/// Dart's constructor arguments are the setters: `TextPainter::new()` is a painter with
/// every default. The methods that shape take the [`FontCollection`] to shape against;
/// `PaintingBinding::fonts` is the app-wide one.
pub struct TextPainter {
    text: Option<InlineSpanRef>,
    text_align: TextAlign,
    text_direction: Option<TextDirection>,
    text_scaler: TextScaler,
    ellipsis: Option<String>,
    max_lines: Option<i32>,
    text_width_basis: TextWidthBasis,
    text_height_behavior: Option<TextHeightBehavior>,
    placeholder_dimensions: Option<Vec<PlaceholderDimensions>>,
    layout_cache: Option<TextPainterLayoutCacheWithOffset>,
    layout_template: Option<Paragraph>,
    // Whether _layoutCache contains outdated paint information and needs to be
    // updated before painting.
    //
    // ui.Paragraph is entirely immutable, thus text style changes that can affect
    // layout and those who can't both require the ui.Paragraph object being
    // recreated. The caller may not call `layout` again after text color change.
    // See: https://github.com/flutter/flutter/issues/85108
    rebuild_paragraph_for_paint: bool,
    cached_plain_text: Option<String>,
    caret_metrics: Option<LineCaretMetrics>,
    debug_needs_relayout: bool,
    /// Paints the layout boxes of each character when true. For debugging.
    pub debug_paint_text_layout_boxes: bool,
    disposed: bool,
}

impl Default for TextPainter {
    fn default() -> TextPainter {
        TextPainter::new()
    }
}

impl TextPainter {
    /// Creates a text painter that paints the given text.
    ///
    /// The `text` and `text_direction` must be set before [`layout`](Self::layout) is
    /// called.
    pub fn new() -> TextPainter {
        TextPainter {
            text: None,
            text_align: TextAlign::Start,
            text_direction: None,
            text_scaler: TextScaler::NO_SCALING,
            ellipsis: None,
            max_lines: None,
            text_width_basis: TextWidthBasis::Parent,
            text_height_behavior: None,
            placeholder_dimensions: None,
            layout_cache: None,
            layout_template: None,
            rebuild_paragraph_for_paint: true,
            cached_plain_text: None,
            caret_metrics: None,
            debug_needs_relayout: true,
            debug_paint_text_layout_boxes: false,
            disposed: false,
        }
    }

    /// Computes the width of a configured [`TextPainter`].
    ///
    /// This is a convenience method that creates a text painter with the supplied
    /// parameters, lays it out with the supplied `min_width` and `max_width`, and returns its
    /// [`width`](Self::width), disposing the painter. Other settings go through a painter's
    /// setters. `fonts` is the collection to shape against.
    pub fn compute_width(
        fonts: &mut FontCollection,
        text: InlineSpanRef,
        text_direction: TextDirection,
        min_width: f64,
        max_width: f64,
    ) -> f64 {
        let mut painter = TextPainter::new();
        painter.set_text(Some(text));
        painter.set_text_direction(Some(text_direction));
        painter.layout(fonts, min_width, max_width);
        let width = painter.width();
        painter.dispose();
        width
    }

    /// Computes the max intrinsic width of a configured [`TextPainter`].
    ///
    /// This is a convenience method that creates a text painter with the supplied
    /// parameters, lays it out with the supplied `min_width` and `max_width`, and returns its
    /// [`max_intrinsic_width`](Self::max_intrinsic_width), disposing the painter.
    pub fn compute_max_intrinsic_width(
        fonts: &mut FontCollection,
        text: InlineSpanRef,
        text_direction: TextDirection,
        min_width: f64,
        max_width: f64,
    ) -> f64 {
        let mut painter = TextPainter::new();
        painter.set_text(Some(text));
        painter.set_text_direction(Some(text_direction));
        painter.layout(fonts, min_width, max_width);
        let width = painter.max_intrinsic_width();
        painter.dispose();
        width
    }

    fn debug_assert_text_layout_is_valid(&self) -> bool {
        debug_assert!(!self.debug_disposed());
        assert!(
            self.layout_cache.is_some(),
            "Text layout not available: the TextPainter has never been laid out."
        );
        true
    }

    /// Marks this text painter's layout information as dirty and removes cached information.
    ///
    /// Uses this method to notify text painter to relayout in the scenario where the layout
    /// information may change without the text being changed.
    pub fn mark_needs_layout(&mut self) {
        if let Some(mut cache) = self.layout_cache.take() {
            cache.layout.paragraph.dispose();
        }
    }

    /// The (potentially styled) text to paint.
    ///
    /// After this is set, you must call [`layout`](Self::layout) before the next call to
    /// [`paint`](Self::paint).
    pub fn text(&self) -> Option<&InlineSpanRef> {
        self.text.as_ref()
    }

    /// Sets [`text`](Self::text).
    pub fn set_text(&mut self, value: Option<InlineSpanRef>) {
        debug_assert!(
            value
                .as_ref()
                .is_none_or(|value| value.debug_assert_is_valid())
        );
        let same = match (&self.text, &value) {
            (None, None) => true,
            (Some(current), Some(value)) => current.eq_span(&**value),
            _ => false,
        };
        if same {
            return;
        }
        let style_changed = self.text.as_ref().and_then(|text| text.style())
            != value.as_ref().and_then(|value| value.style());
        if style_changed && let Some(mut template) = self.layout_template.take() {
            template.dispose();
        }

        let comparison = match (&self.text, &value) {
            (_, None) => RenderComparison::Layout,
            (Some(current), Some(value)) => current.compare_to(&**value),
            (None, Some(_)) => RenderComparison::Layout,
        };

        self.text = value;
        self.cached_plain_text = None;

        if comparison >= RenderComparison::Layout {
            self.mark_needs_layout();
        } else if comparison >= RenderComparison::Paint {
            // Don't invalid the _layoutCache just yet. It still contains valid layout
            // information.
            self.rebuild_paragraph_for_paint = true;
        }
        // Neither relayout or repaint is needed.
    }

    /// Returns a plain text version of the text to paint.
    ///
    /// This uses `InlineSpan::to_plain_text` to get the full contents of all nodes in the
    /// tree.
    pub fn plain_text(&mut self) -> &str {
        if self.cached_plain_text.is_none() {
            self.cached_plain_text = Some(
                self.text
                    .as_ref()
                    .map(|text| text.to_plain_text(false, true))
                    .unwrap_or_default(),
            );
        }
        self.cached_plain_text.as_deref().unwrap_or("")
    }

    /// How the text should be aligned horizontally.
    ///
    /// After this is set, you must call [`layout`](Self::layout) before the next call to
    /// [`paint`](Self::paint).
    ///
    /// The [`text_align`](Self::text_align) property defaults to [`TextAlign::Start`].
    pub fn text_align(&self) -> TextAlign {
        self.text_align
    }

    /// Sets [`text_align`](Self::text_align).
    pub fn set_text_align(&mut self, value: TextAlign) {
        if self.text_align == value {
            return;
        }
        self.text_align = value;
        self.mark_needs_layout();
    }

    /// The default directionality of the text.
    ///
    /// This controls how the [`TextAlign::Start`], [`TextAlign::End`], and
    /// [`TextAlign::Justify`] values of [`text_align`](Self::text_align) are resolved.
    ///
    /// This is also used to disambiguate how to render bidirectional text. For example, if
    /// the [`text`](Self::text) is an English phrase followed by a Hebrew phrase, in a
    /// [`TextDirection::Ltr`] context the English phrase will be on the left and the Hebrew
    /// phrase to its right, while in a [`TextDirection::Rtl`] context, the English phrase
    /// will be on the right and the Hebrew phrase on its left.
    ///
    /// After this is set, you must call [`layout`](Self::layout) before the next call to
    /// [`paint`](Self::paint).
    ///
    /// This and [`text`](Self::text) must be set before [`layout`](Self::layout) is called.
    pub fn text_direction(&self) -> Option<TextDirection> {
        self.text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(&mut self, value: Option<TextDirection>) {
        if self.text_direction == value {
            return;
        }
        self.text_direction = value;
        self.mark_needs_layout();
        // Shouldn't really matter, but for strict correctness...
        if let Some(mut template) = self.layout_template.take() {
            template.dispose();
        }
    }

    /// The font scaling strategy to use when laying out and rendering the text.
    ///
    /// The value usually comes from `MediaQuery.textScalerOf`, which typically reflects the
    /// user-specified text scaling value in the platform's accessibility settings.
    ///
    /// The [`TextStyle::font_size`] of the text will be adjusted by the [`TextScaler`] before
    /// layout and painting.
    ///
    /// After this is set, you must call [`layout`](Self::layout) before the next call to
    /// [`paint`](Self::paint).
    pub fn text_scaler(&self) -> &TextScaler {
        &self.text_scaler
    }

    /// Sets [`text_scaler`](Self::text_scaler).
    pub fn set_text_scaler(&mut self, value: TextScaler) {
        if value == self.text_scaler {
            return;
        }
        self.text_scaler = value;
        self.mark_needs_layout();
        if let Some(mut template) = self.layout_template.take() {
            template.dispose();
        }
    }

    /// The string used to ellipsize overflowing text. Setting this to a non-empty string
    /// will cause this string to be substituted for the remaining text if the text can not
    /// fit within the specified maximum width.
    ///
    /// Specifically, the ellipsis is applied to the last line before the line truncated by
    /// [`max_lines`](Self::max_lines), if [`max_lines`](Self::max_lines) is non-`None` and
    /// that line overflows the width constraint, or to the first line that is wider than the
    /// width constraint, if [`max_lines`](Self::max_lines) is `None`. The width constraint
    /// is the `max_width` passed to [`layout`](Self::layout).
    ///
    /// After this is set, you must call [`layout`](Self::layout) before the next call to
    /// [`paint`](Self::paint).
    ///
    /// The string will be applied at the end of the text, even if the text is longer than
    /// one line.
    pub fn ellipsis(&self) -> Option<&str> {
        self.ellipsis.as_deref()
    }

    /// Sets [`ellipsis`](Self::ellipsis).
    pub fn set_ellipsis(&mut self, value: Option<String>) {
        debug_assert!(value.as_ref().is_none_or(|value| !value.is_empty()));
        if self.ellipsis == value {
            return;
        }
        self.ellipsis = value;
        self.mark_needs_layout();
    }

    /// An optional maximum number of lines for the text to span, wrapping if necessary.
    ///
    /// If the text exceeds the given number of lines, it is truncated such that subsequent
    /// lines are dropped.
    ///
    /// After this is set, you must call [`layout`](Self::layout) before the next call to
    /// [`paint`](Self::paint).
    pub fn max_lines(&self) -> Option<i32> {
        self.max_lines
    }

    /// Sets [`max_lines`](Self::max_lines). The value may be `None`. If it is not `None`,
    /// then it must be greater than zero.
    pub fn set_max_lines(&mut self, value: Option<i32>) {
        debug_assert!(value.is_none_or(|value| value > 0));
        if self.max_lines == value {
            return;
        }
        self.max_lines = value;
        self.mark_needs_layout();
    }

    /// Defines how to measure the width of the rendered text.
    pub fn text_width_basis(&self) -> TextWidthBasis {
        self.text_width_basis
    }

    /// Sets [`text_width_basis`](Self::text_width_basis).
    pub fn set_text_width_basis(&mut self, value: TextWidthBasis) {
        if self.text_width_basis == value {
            return;
        }
        if cfg!(debug_assertions) {
            self.debug_needs_relayout = true;
        }
        self.text_width_basis = value;
    }

    /// Defines how to apply `TextStyle.height` over and under text.
    pub fn text_height_behavior(&self) -> Option<TextHeightBehavior> {
        self.text_height_behavior
    }

    /// Sets [`text_height_behavior`](Self::text_height_behavior).
    pub fn set_text_height_behavior(&mut self, value: Option<TextHeightBehavior>) {
        if self.text_height_behavior == value {
            return;
        }
        self.text_height_behavior = value;
        self.mark_needs_layout();
    }

    /// An ordered list of [`TextBox`]es that bound the positions of the placeholders in the
    /// paragraph.
    ///
    /// Each box corresponds to a `PlaceholderSpan` in the order they were defined in the
    /// `InlineSpan` tree.
    pub fn inline_placeholder_boxes(&mut self) -> Option<Vec<TextBox>> {
        let layout = self.layout_cache.as_mut()?;
        let offset = layout.paint_offset();
        if !offset.dx().is_finite() || !offset.dy().is_finite() {
            return Some(Vec::new());
        }
        let raw_boxes = layout.inline_placeholder_boxes();
        if offset == Offset::ZERO {
            return Some(raw_boxes.to_vec());
        }
        Some(
            raw_boxes
                .iter()
                .map(|text_box| shift_text_box(*text_box, offset))
                .collect(),
        )
    }

    /// Sets the dimensions of each placeholder in [`text`](Self::text).
    ///
    /// The number of [`PlaceholderDimensions`] provided should be the same as the number of
    /// `PlaceholderSpan`s in text. Passing in an empty or `None` `value` will do nothing.
    ///
    /// If [`layout`](Self::layout) is attempted without setting the placeholder dimensions,
    /// the placeholders will be ignored in the text layout and no valid
    /// [`inline_placeholder_boxes`](Self::inline_placeholder_boxes) will be returned.
    pub fn set_placeholder_dimensions(&mut self, value: Option<Vec<PlaceholderDimensions>>) {
        let Some(value) = value else {
            return;
        };
        if value.is_empty() || Some(&value) == self.placeholder_dimensions.as_ref() {
            return;
        }
        self.placeholder_dimensions = Some(value);
        self.mark_needs_layout();
    }

    fn create_paragraph_style(&self, text_align_override: Option<TextAlign>) -> ParagraphStyle {
        debug_assert!(
            self.text_direction.is_some(),
            "TextPainter.textDirection must be set to a non-null value before using the TextPainter."
        );
        let default_style = TextStyle::new();
        let base_style = self
            .text
            .as_ref()
            .and_then(|text| text.style())
            .unwrap_or(&default_style);
        let mut paragraph_style = base_style
            .get_paragraph_style()
            .text_align(text_align_override.unwrap_or(self.text_align))
            .text_scaler(self.text_scaler.clone());
        if let Some(text_direction) = self.text_direction {
            paragraph_style = paragraph_style.text_direction(text_direction);
        }
        if let Some(max_lines) = self.max_lines {
            paragraph_style = paragraph_style.max_lines(max_lines);
        }
        if let Some(text_height_behavior) = self.text_height_behavior {
            paragraph_style = paragraph_style.text_height_behavior(text_height_behavior);
        }
        if let Some(ellipsis) = &self.ellipsis {
            paragraph_style = paragraph_style.ellipsis(ellipsis.clone());
        }
        paragraph_style.build()
    }

    fn create_layout_template(&self, fonts: &mut FontCollection) -> Paragraph {
        // Direction doesn't matter, text is just a space.
        let mut builder = ParagraphBuilder::new(self.create_paragraph_style(Some(TextAlign::Left)));
        let text_style = self
            .text
            .as_ref()
            .and_then(|text| text.style())
            .map(|style| style.get_text_style_with(&self.text_scaler));
        if let Some(text_style) = text_style {
            builder.push_style(text_style);
        }
        builder.add_text(" ");
        let mut template = builder.build(fonts);
        template.layout(ParagraphConstraints::new(f64::INFINITY));
        template
    }

    fn get_or_create_layout_template(&mut self, fonts: &mut FontCollection) -> &Paragraph {
        if self.layout_template.is_none() {
            self.layout_template = Some(self.create_layout_template(fonts));
        }
        self.layout_template.as_ref().expect("just created")
    }

    /// The height of a space in [`text`](Self::text) in logical pixels.
    ///
    /// Not every line of text in [`text`](Self::text) will have this height, but this height
    /// is "typical" for text in [`text`](Self::text) and useful for sizing other objects
    /// relative a typical line of text.
    ///
    /// Obtaining this value does not require calling [`layout`](Self::layout).
    ///
    /// The style of the [`text`](Self::text) property is used to determine the font settings
    /// that contribute to the [`preferred_line_height`](Self::preferred_line_height). If
    /// [`text`](Self::text) is `None` or if it specifies no styles, the default `TextStyle`
    /// values are used (a 10 pixel sans-serif font).
    pub fn preferred_line_height(&mut self, fonts: &mut FontCollection) -> f64 {
        self.get_or_create_layout_template(fonts).height()
    }

    /// The width at which decreasing the width of the text would prevent it from painting
    /// itself completely within its bounds.
    ///
    /// Valid only after [`layout`](Self::layout) has been called.
    pub fn min_intrinsic_width(&self) -> f64 {
        debug_assert!(self.debug_assert_text_layout_is_valid());
        self.layout_cache().layout.min_intrinsic_line_extent()
    }

    /// The width at which increasing the width of the text no longer decreases the height.
    ///
    /// Valid only after [`layout`](Self::layout) has been called.
    pub fn max_intrinsic_width(&self) -> f64 {
        debug_assert!(self.debug_assert_text_layout_is_valid());
        self.layout_cache().layout.max_intrinsic_line_extent()
    }

    /// The horizontal space required to paint this text.
    ///
    /// Valid only after [`layout`](Self::layout) has been called.
    pub fn width(&self) -> f64 {
        debug_assert!(self.debug_assert_text_layout_is_valid());
        debug_assert!(!self.debug_needs_relayout);
        self.layout_cache().content_width
    }

    /// The vertical space required to paint this text.
    ///
    /// Valid only after [`layout`](Self::layout) has been called.
    pub fn height(&self) -> f64 {
        debug_assert!(self.debug_assert_text_layout_is_valid());
        self.layout_cache().layout.height()
    }

    /// The amount of space required to paint this text.
    ///
    /// Valid only after [`layout`](Self::layout) has been called.
    pub fn size(&self) -> Size {
        debug_assert!(self.debug_assert_text_layout_is_valid());
        debug_assert!(!self.debug_needs_relayout);
        Size::new(self.width(), self.height())
    }

    /// Returns the distance from the top of the text to the first baseline of the given
    /// type.
    ///
    /// Valid only after [`layout`](Self::layout) has been called.
    pub fn compute_distance_to_actual_baseline(&self, baseline: TextBaseline) -> f64 {
        debug_assert!(self.debug_assert_text_layout_is_valid());
        self.layout_cache()
            .layout
            .get_distance_to_baseline(baseline)
    }

    /// Whether any text was truncated or ellipsized.
    ///
    /// If [`max_lines`](Self::max_lines) is not `None`, this is true if there were more lines
    /// to draw than the given [`max_lines`](Self::max_lines), and thus at least one line was
    /// omitted. If [`max_lines`](Self::max_lines) is `None`, this is true if
    /// [`ellipsis`](Self::ellipsis) is not the empty string and there was a line that
    /// overflowed the `max_width` argument passed to [`layout`](Self::layout).
    ///
    /// Valid only after [`layout`](Self::layout) has been called.
    pub fn did_exceed_max_lines(&self) -> bool {
        debug_assert!(self.debug_assert_text_layout_is_valid());
        self.layout_cache().paragraph().did_exceed_max_lines()
    }

    fn layout_cache(&self) -> &TextPainterLayoutCacheWithOffset {
        self.layout_cache
            .as_ref()
            .expect("Text layout not available: the TextPainter has never been laid out.")
    }

    // Creates a ui.Paragraph using the current configurations in this class and
    // assign it to _paragraph.
    fn create_paragraph(&mut self, fonts: &mut FontCollection, text: &InlineSpanRef) -> Paragraph {
        let mut builder = ParagraphBuilder::new(self.create_paragraph_style(None));
        text.build(
            &mut builder,
            &self.text_scaler,
            self.placeholder_dimensions.as_deref(),
        );
        self.rebuild_paragraph_for_paint = false;
        builder.build(fonts)
    }

    /// Computes the visual position of the glyphs for painting the text.
    ///
    /// The text will layout with a width that's as close to its max intrinsic width (or its
    /// longest line, if [`text_width_basis`](Self::text_width_basis) is set to
    /// [`TextWidthBasis::Parent`]) as possible while still being greater than or equal to
    /// `min_width` and less than or equal to `max_width`.
    ///
    /// The [`text`](Self::text) and [`text_direction`](Self::text_direction) properties
    /// must be non-`None` before this is called. `fonts` is the collection to shape against.
    pub fn layout(&mut self, fonts: &mut FontCollection, min_width: f64, max_width: f64) {
        debug_assert!(!max_width.is_nan());
        debug_assert!(!min_width.is_nan());
        if cfg!(debug_assertions) {
            self.debug_needs_relayout = false;
        }

        if let Some(cached_layout) = self.layout_cache.as_mut()
            && cached_layout.resize_to_fit(min_width, max_width, self.text_width_basis)
        {
            return;
        }

        let text = self.text.clone().expect(
            "TextPainter.text must be set to a non-null value before using the TextPainter.",
        );
        let text_direction = self.text_direction.expect(
            "TextPainter.textDirection must be set to a non-null value before using the TextPainter.",
        );

        // Determines the actual max width, and whether the max width needs to be adjusted.
        let paint_offset_alignment = compute_paint_offset_fraction(self.text_align, text_direction);
        let adjust_max_width = !max_width.is_finite() && paint_offset_alignment != 0.0;
        let adjusted_max_width = if !adjust_max_width {
            Some(max_width)
        } else {
            self.layout_cache
                .as_ref()
                .map(|cached| cached.layout.max_intrinsic_line_extent())
        };
        let layout_max_width = adjusted_max_width.unwrap_or(max_width);

        // Only rebuild the paragraph when there're layout changes, even when
        // `_rebuildParagraphForPaint` is true. It's best to not eagerly rebuild
        // the paragraph to avoid the extra work, because:
        // 1. the text color could change again before `paint` is called (so one of
        //    the paragraph rebuilds is unnecessary)
        // 2. the user could be measuring the text layout so `paint` will never be
        //    called.
        let mut paragraph = match self.layout_cache.take() {
            Some(cached) => cached.layout.paragraph,
            None => self.create_paragraph(fonts, &text),
        };
        paragraph.layout(ParagraphConstraints::new(layout_max_width));
        let layout = TextLayout::new(paragraph, text_direction);
        let content_width = layout.content_width_for(min_width, max_width, self.text_width_basis);

        let new_layout_cache = if adjusted_max_width.is_none() && min_width.is_finite() {
            debug_assert!(max_width.is_infinite());
            let new_input_width = layout.max_intrinsic_line_extent();
            let mut layout = layout;
            layout
                .paragraph
                .layout(ParagraphConstraints::new(new_input_width));
            TextPainterLayoutCacheWithOffset::new(
                layout,
                paint_offset_alignment,
                new_input_width,
                content_width,
            )
        } else {
            TextPainterLayoutCacheWithOffset::new(
                layout,
                paint_offset_alignment,
                layout_max_width,
                content_width,
            )
        };
        self.layout_cache = Some(new_layout_cache);
    }

    /// Paints the text onto the given canvas at the given offset.
    ///
    /// Valid only after [`layout`](Self::layout) has been called.
    ///
    /// If you cannot see the text being painted, check that your text color does not conflict
    /// with the background on which you are drawing. The default text color is white (to
    /// contrast with the default black background color), so if you are writing an
    /// application with a white background, the text will not be visible by default.
    ///
    /// To set the text style, specify a `TextStyle` when creating the `TextSpan` that you
    /// pass to the [`TextPainter`].
    pub fn paint(&mut self, fonts: &mut FontCollection, canvas: &mut Canvas, offset: Offset) {
        assert!(
            self.layout_cache.is_some(),
            "TextPainter.paint called when text geometry was not yet calculated.\n\
             Please call layout() before paint() to position the text before painting it."
        );
        let paint_offset = self.layout_cache().paint_offset();
        if !paint_offset.dx().is_finite() || !paint_offset.dy().is_finite() {
            return;
        }

        if self.rebuild_paragraph_for_paint {
            let debug_size = cfg!(debug_assertions).then(|| self.size());
            let text = self.text.clone().expect("a laid-out painter has text");
            let layout_max_width = self.layout_cache().layout_max_width;
            debug_assert!(!layout_max_width.is_nan());
            let mut rebuilt = self.create_paragraph(fonts, &text);
            rebuilt.layout(ParagraphConstraints::new(layout_max_width));
            let layout_cache = self.layout_cache.as_mut().expect("checked above");
            debug_assert!(layout_cache.paragraph().width() == rebuilt.width());
            let mut previous = std::mem::replace(&mut layout_cache.layout.paragraph, rebuilt);
            previous.dispose();
            debug_assert!(debug_size.is_none_or(|size| size == self.size()));
        }
        debug_assert!(!self.rebuild_paragraph_for_paint);
        if self.debug_paint_text_layout_boxes {
            self.debug_paint_character_layout_boxes(canvas, offset);
        }
        canvas.draw_paragraph(self.layout_cache().paragraph(), offset + paint_offset);
    }

    fn debug_paint_character_layout_boxes(&mut self, canvas: &mut Canvas, offset: Offset) {
        let paint = Paint {
            style: PaintStyle::Stroke(Stroke::new(1.0)),
            color: Color::from_argb(255, 0, 255, 255).into(),
            ..Paint::default()
        };
        let length = utf16_len(self.plain_text());
        let text_boxes = self.get_boxes_for_selection(
            TextSelection::new(0, length),
            BoxHeightStyle::Tight,
            BoxWidthStyle::Tight,
        );
        for text_box in text_boxes {
            let host_rect: reveal_embedder::valo::Rect = text_box.to_rect().shift(offset).into();
            canvas.draw_rect(host_rect, &paint);
        }
    }

    /// Whether `value` is a high surrogate UTF-16 code unit.
    pub fn is_high_surrogate(value: u16) -> bool {
        value & 0xFC00 == 0xD800
    }

    /// Whether `value` is a low surrogate UTF-16 code unit.
    pub fn is_low_surrogate(value: u16) -> bool {
        value & 0xFC00 == 0xDC00
    }

    /// Returns the closest offset after `offset` at which the input cursor can be positioned.
    pub fn get_offset_after(&self, offset: i32) -> Option<i32> {
        let next_code_unit = self.text.as_ref()?.code_unit_at(offset)?;
        // TODO(goderbauer): doesn't handle extended grapheme clusters with more than one
        // Unicode scalar value (https://github.com/flutter/flutter/issues/13404).
        Some(if TextPainter::is_high_surrogate(next_code_unit) {
            offset + 2
        } else {
            offset + 1
        })
    }

    /// Returns the closest offset before `offset` at which the input cursor can be
    /// positioned.
    pub fn get_offset_before(&self, offset: i32) -> Option<i32> {
        let prev_code_unit = self.text.as_ref()?.code_unit_at(offset - 1)?;
        // TODO(goderbauer): doesn't handle extended grapheme clusters with more than one
        // Unicode scalar value (https://github.com/flutter/flutter/issues/13404).
        Some(if TextPainter::is_low_surrogate(prev_code_unit) {
            offset - 2
        } else {
            offset - 1
        })
    }

    /// Returns the offset at which to paint the caret.
    ///
    /// Valid only after [`layout`](Self::layout) has been called.
    pub fn get_offset_for_caret(
        &mut self,
        fonts: &mut FontCollection,
        position: TextPosition,
        caret_prototype: Rect,
    ) -> Offset {
        let Some(caret_metrics) = self.compute_caret_metrics(fonts, position) else {
            let text_direction = self.text_direction.expect("laid out");
            let paint_offset_alignment =
                compute_paint_offset_fraction(self.text_align, text_direction);
            let dx = if paint_offset_alignment == 0.0 {
                0.0
            } else {
                paint_offset_alignment * self.layout_cache().content_width
            };
            return Offset::new(dx, 0.0);
        };
        let layout_cache = self.layout_cache();
        let raw_offset = match caret_metrics.writing_direction {
            TextDirection::Ltr => caret_metrics.offset,
            TextDirection::Rtl => Offset::new(
                caret_metrics.offset.dx() - caret_prototype.width(),
                caret_metrics.offset.dy(),
            ),
        };
        // If the caret is at the end of the paragraph and beyond the layout width
        // (thus disregarding the paragraph's width), clamp it to the content width.
        let adjusted_dx = clamp_double(
            raw_offset.dx() + layout_cache.paint_offset().dx(),
            0.0,
            layout_cache.content_width,
        );
        Offset::new(
            adjusted_dx,
            raw_offset.dy() + layout_cache.paint_offset().dy(),
        )
    }

    /// Returns the strut bounded height of the glyph at the given `position`.
    ///
    /// Valid only after [`layout`](Self::layout) has been called. Without strut support the
    /// caret metrics decide; the layout template is the fallback.
    pub fn get_full_height_for_caret(
        &mut self,
        fonts: &mut FontCollection,
        position: TextPosition,
        caret_prototype: Rect,
    ) -> f64 {
        let _ = caret_prototype;
        if let Some(metrics) = self.compute_caret_metrics(fonts, position) {
            return metrics.height;
        }
        let preferred_line_height = self.preferred_line_height(fonts);
        let boxes = self
            .get_or_create_layout_template(fonts)
            .get_boxes_for_range(0, 1, BoxHeightStyle::Strut, BoxWidthStyle::Tight);
        match boxes.as_slice() {
            [single] => single.to_rect().height(),
            _ => preferred_line_height,
        }
    }

    fn is_newline_at_offset(&mut self, offset: i32) -> bool {
        if offset < 0 {
            return false;
        }
        self.plain_text()
            .encode_utf16()
            .nth(offset as usize)
            .is_some_and(|unit| is_newline(u32::from(unit)))
    }

    // Cached caret metrics. This allows multiple invokes of [getOffsetForCaret] and
    // [getFullHeightForCaret] in a row without performing redundant and expensive
    // get rect calls to the paragraph.
    //
    // The cache implementation assumes (and asserts) the caller does not directly
    // call methods on the underlying paragraph after the text layout changes.
    fn compute_caret_metrics(
        &mut self,
        fonts: &mut FontCollection,
        position: TextPosition,
    ) -> Option<LineCaretMetrics> {
        debug_assert!(self.debug_assert_text_layout_is_valid());
        debug_assert!(!self.debug_needs_relayout);
        if self.layout_cache().paragraph().number_of_lines() < 1 {
            return None;
        }
        let upstream_after_newline = position.affinity == TextAffinity::Upstream
            && self.is_newline_at_offset(position.offset - 1);
        let (offset, anchor_to_leading_edge) = match position {
            // As a special case, always anchor to the leading edge of the first
            // grapheme regardless of the affinity.
            TextPosition { offset: 0, .. } => (0, true),
            TextPosition {
                offset,
                affinity: TextAffinity::Downstream,
            } => (offset, true),
            TextPosition {
                offset,
                affinity: TextAffinity::Upstream,
            } if upstream_after_newline => (offset, true),
            TextPosition {
                offset,
                affinity: TextAffinity::Upstream,
            } => (offset - 1, false),
        };
        let caret_position_cache_key = if anchor_to_leading_edge {
            offset
        } else {
            -offset - 1
        };
        if Some(caret_position_cache_key) == self.layout_cache().previous_caret_position_key {
            return self.caret_metrics;
        }

        let Some(glyph_info) = self.layout_cache().paragraph().get_glyph_info_at(offset) else {
            // If the glyph isn't laid out, then the caret can't be placed after it. Use the
            // end-of-text caret instead.
            let plain_text = self.plain_text().to_owned();
            let baseline_offset = self
                .get_or_create_layout_template(fonts)
                .get_line_metrics_at(0)
                .expect("the template has one line")
                .baseline;
            let layout = &self.layout_cache().layout;
            return Some(
                layout
                    .end_of_text_caret_metrics(&plain_text)
                    .shift(Offset::new(0.0, -baseline_offset)),
            );
        };
        let grapheme_range = glyph_info.grapheme_cluster_code_unit_range;

        // Works around a SkParagraph bug (https://github.com/flutter/flutter/issues/120836#issuecomment-1937343854):
        // placeholders with a size of (0, 0) always have a rect of Rect.zero and a
        // range of (0, 0).
        if grapheme_range.is_collapsed() {
            debug_assert!(grapheme_range.start == 0);
            return self.compute_caret_metrics(fonts, TextPosition::new(offset + 1));
        }
        if anchor_to_leading_edge && grapheme_range.start != offset {
            debug_assert!(grapheme_range.end > grapheme_range.start + 1);
            // This is a pretty rare case, the glyph info at the offset is a grapheme cluster
            // (probably a ligature) that starts before the given offset. Anchor the caret to
            // the trailing edge of the cluster.
            return self.compute_caret_metrics(fonts, TextPosition::new(grapheme_range.end));
        }

        let boxes = self.layout_cache().paragraph().get_boxes_for_range(
            grapheme_range.start,
            grapheme_range.end,
            BoxHeightStyle::Strut,
            BoxWidthStyle::Tight,
        );
        let anchor_to_left = match glyph_info.writing_direction {
            TextDirection::Ltr => anchor_to_leading_edge,
            TextDirection::Rtl => !anchor_to_leading_edge,
        };
        let text_box = if anchor_to_left {
            boxes.first()
        } else {
            boxes.last()
        }?;
        let metrics = LineCaretMetrics {
            offset: Offset::new(
                if anchor_to_left {
                    text_box.left
                } else {
                    text_box.right
                },
                text_box.top,
            ),
            writing_direction: text_box.direction,
            height: text_box.bottom - text_box.top,
        };
        self.layout_cache
            .as_mut()
            .expect("checked above")
            .previous_caret_position_key = Some(caret_position_cache_key);
        self.caret_metrics = Some(metrics);
        Some(metrics)
    }

    /// Returns a list of rects that bound the given selection.
    ///
    /// The `box_height_style` and `box_width_style` arguments may be used to select the
    /// shape of the [`TextBox`]es. These properties default to [`BoxHeightStyle::Tight`] and
    /// [`BoxWidthStyle::Tight`] respectively.
    ///
    /// A given selection might have more than one rect if this text painter contains
    /// bidirectional text because logically contiguous text might not be visually contiguous.
    ///
    /// Leading or trailing newline characters will be represented by zero-width [`TextBox`]es.
    ///
    /// The method only returns [`TextBox`]es of glyphs that are entirely enclosed by the given
    /// `selection`: a multi-code-unit glyph will be excluded if only part of its code units
    /// are in `selection`.
    pub fn get_boxes_for_selection(
        &self,
        selection: TextSelection,
        box_height_style: BoxHeightStyle,
        box_width_style: BoxWidthStyle,
    ) -> Vec<TextBox> {
        debug_assert!(self.debug_assert_text_layout_is_valid());
        debug_assert!(selection.is_valid());
        debug_assert!(!self.debug_needs_relayout);
        let cached_layout = self.layout_cache();
        let offset = cached_layout.paint_offset();
        if !offset.dx().is_finite() || !offset.dy().is_finite() {
            return Vec::new();
        }
        let boxes = cached_layout.paragraph().get_boxes_for_range(
            selection.start(),
            selection.end(),
            box_height_style,
            box_width_style,
        );
        if offset == Offset::ZERO {
            boxes
        } else {
            boxes
                .into_iter()
                .map(|text_box| shift_text_box(text_box, offset))
                .collect()
        }
    }

    /// Returns the [`GlyphInfo`] of the glyph closest to the given `offset` in the paragraph
    /// coordinate system, or `None` if the text is empty, or is entirely clipped or
    /// ellipsized away.
    ///
    /// This method first finds the line closest to `offset.dy`, and then returns the
    /// [`GlyphInfo`] of the closest glyph(s) within that line.
    pub fn get_closest_glyph_for_offset(&self, offset: Offset) -> Option<GlyphInfo> {
        debug_assert!(self.debug_assert_text_layout_is_valid());
        debug_assert!(!self.debug_needs_relayout);
        let cached_layout = self.layout_cache();
        let raw_glyph_info = cached_layout
            .paragraph()
            .get_closest_glyph_info_for_offset(offset - cached_layout.paint_offset())?;
        if cached_layout.paint_offset() == Offset::ZERO {
            return Some(raw_glyph_info);
        }
        Some(GlyphInfo {
            grapheme_cluster_layout_bounds: raw_glyph_info
                .grapheme_cluster_layout_bounds
                .shift(cached_layout.paint_offset()),
            grapheme_cluster_code_unit_range: raw_glyph_info.grapheme_cluster_code_unit_range,
            writing_direction: raw_glyph_info.writing_direction,
        })
    }

    /// Returns the closest position within the text for the given pixel offset.
    pub fn get_position_for_offset(&self, offset: Offset) -> TextPosition {
        debug_assert!(self.debug_assert_text_layout_is_valid());
        debug_assert!(!self.debug_needs_relayout);
        let cached_layout = self.layout_cache();
        cached_layout
            .paragraph()
            .get_position_for_offset(offset - cached_layout.paint_offset())
    }

    /// Returns the text range of the word at the given offset. Characters not part of a word,
    /// such as spaces, symbols, and punctuation, have word breaks on both sides. In such
    /// cases, this method will return a text range that contains the given text position.
    ///
    /// Word boundaries are defined more precisely in Unicode Standard Annex #29
    /// <http://www.unicode.org/reports/tr29/#Word_Boundaries>.
    pub fn get_word_boundary(&self, position: TextPosition) -> TextRange {
        debug_assert!(self.debug_assert_text_layout_is_valid());
        self.layout_cache().paragraph().get_word_boundary(position)
    }

    /// Returns the text range of the line at the given offset.
    ///
    /// The newline (if any) is not returned as part of the range.
    pub fn get_line_boundary(&self, position: TextPosition) -> TextRange {
        debug_assert!(self.debug_assert_text_layout_is_valid());
        self.layout_cache().paragraph().get_line_boundary(position)
    }

    /// Returns the full list of [`LineMetrics`] that describe in detail the various metrics
    /// of each laid out line.
    ///
    /// The [`LineMetrics`] list is presented in the order of the lines they represent.
    ///
    /// Valid only after [`layout`](Self::layout) has been called.
    pub fn compute_line_metrics(&mut self) -> Vec<LineMetrics> {
        debug_assert!(self.debug_assert_text_layout_is_valid());
        debug_assert!(!self.debug_needs_relayout);
        let layout = self.layout_cache.as_mut().expect("laid out");
        let offset = layout.paint_offset();
        if !offset.dx().is_finite() || !offset.dy().is_finite() {
            return Vec::new();
        }
        let raw_metrics = layout.line_metrics();
        if offset == Offset::ZERO {
            raw_metrics.to_vec()
        } else {
            raw_metrics
                .iter()
                .map(|metrics| shift_line_metrics(*metrics, offset))
                .collect()
        }
    }

    /// Whether this object has been disposed or not.
    pub fn debug_disposed(&self) -> bool {
        self.disposed
    }

    /// Releases the resources associated with this painter.
    ///
    /// After disposal this painter is unusable.
    pub fn dispose(&mut self) {
        debug_assert!(!self.debug_disposed());
        self.disposed = true;
        if let Some(mut template) = self.layout_template.take() {
            template.dispose();
        }
        if let Some(mut cache) = self.layout_cache.take() {
            cache.layout.paragraph.dispose();
        }
        self.text = None;
    }
}

fn compute_paint_offset_fraction(text_align: TextAlign, text_direction: TextDirection) -> f64 {
    match (text_align, text_direction) {
        (TextAlign::Left, _) => 0.0,
        (TextAlign::Right, _) => 1.0,
        (TextAlign::Center, _) => 0.5,
        (TextAlign::Start | TextAlign::Justify, TextDirection::Ltr) => 0.0,
        (TextAlign::Start | TextAlign::Justify, TextDirection::Rtl) => 1.0,
        (TextAlign::End, TextDirection::Ltr) => 1.0,
        (TextAlign::End, TextDirection::Rtl) => 0.0,
    }
}

fn shift_line_metrics(metrics: LineMetrics, offset: Offset) -> LineMetrics {
    debug_assert!(offset.dx().is_finite());
    debug_assert!(offset.dy().is_finite());
    LineMetrics {
        left: metrics.left + offset.dx(),
        baseline: metrics.baseline + offset.dy(),
        ..metrics
    }
}

fn shift_text_box(text_box: TextBox, offset: Offset) -> TextBox {
    debug_assert!(offset.dx().is_finite());
    debug_assert!(offset.dy().is_finite());
    TextBox::from_ltrbd(
        text_box.left + offset.dx(),
        text_box.top + offset.dy(),
        text_box.right + offset.dx(),
        text_box.bottom + offset.dy(),
        text_box.direction,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_span::TextSpan;

    fn fonts() -> FontCollection {
        let mut fonts = FontCollection::new();
        fonts.add_source(valo_system_fonts::SystemFonts::load());
        fonts
    }

    fn painter(text: &str) -> TextPainter {
        let mut painter = TextPainter::new();
        painter.set_text(Some(
            TextSpan::new()
                .text(text)
                .style(TextStyle::new().font_size(20.0))
                .into_span(),
        ));
        painter.set_text_direction(Some(TextDirection::Ltr));
        painter
    }

    #[test]
    fn layout_sizes_the_text_and_paint_records_it() {
        let mut fonts = fonts();
        let mut painter = painter("Hello world");
        painter.layout(&mut fonts, 0.0, f64::INFINITY);
        assert!(painter.width() > 0.0);
        assert!(painter.height() > 0.0);
        assert_eq!(painter.size(), Size::new(painter.width(), painter.height()));
        assert!(painter.compute_distance_to_actual_baseline(TextBaseline::Alphabetic) > 0.0);
        assert_eq!(painter.compute_line_metrics().len(), 1);
        let mut canvas = Canvas::new();
        painter.paint(&mut fonts, &mut canvas, Offset::new(5.0, 5.0));
        assert!(!canvas.build().ops().is_empty());
        painter.dispose();
    }

    #[test]
    fn a_min_width_widens_the_reported_width_without_relayout() {
        let mut fonts = fonts();
        let mut painter = painter("Hi");
        painter.layout(&mut fonts, 0.0, f64::INFINITY);
        let natural = painter.width();
        painter.layout(&mut fonts, natural + 100.0, f64::INFINITY);
        assert_eq!(painter.width(), natural + 100.0);
        assert_eq!(painter.max_intrinsic_width(), natural);
    }

    #[test]
    fn right_alignment_offsets_the_paint_and_the_caret() {
        let mut fonts = fonts();
        let mut painter = painter("Hi");
        painter.set_text_align(TextAlign::Right);
        painter.layout(&mut fonts, 200.0, 200.0);
        let natural = painter.max_intrinsic_width();
        let caret = painter.get_offset_for_caret(&mut fonts, TextPosition::new(0), Rect::ZERO);
        assert!(
            (caret.dx() - (200.0 - natural)).abs() < 1.0,
            "caret at {caret:?}"
        );
        let end = painter.get_offset_for_caret(&mut fonts, TextPosition::new(2), Rect::ZERO);
        assert!((end.dx() - 200.0).abs() < 1.0, "end caret at {end:?}");
    }

    #[test]
    fn changing_only_the_color_does_not_drop_the_layout() {
        let mut fonts = fonts();
        let mut painter = painter("Hi");
        painter.layout(&mut fonts, 0.0, f64::INFINITY);
        painter.set_text(Some(
            TextSpan::new()
                .text("Hi")
                .style(
                    TextStyle::new()
                        .font_size(20.0)
                        .color(Color::from_argb(255, 255, 0, 0)),
                )
                .into_span(),
        ));
        assert!(
            painter.layout_cache.is_some(),
            "a paint-only change keeps the layout"
        );
        assert!(painter.rebuild_paragraph_for_paint);
        let mut canvas = Canvas::new();
        painter.paint(&mut fonts, &mut canvas, Offset::ZERO);
        assert!(!painter.rebuild_paragraph_for_paint);
    }

    #[test]
    fn max_lines_and_ellipsis_report_overflow() {
        let mut fonts = fonts();
        let mut painter = painter("one two three four five six");
        painter.layout(&mut fonts, 0.0, f64::INFINITY);
        let full = painter.width();
        painter.set_max_lines(Some(1));
        painter.set_ellipsis(Some("\u{2026}".to_owned()));
        painter.layout(&mut fonts, 0.0, full / 2.0);
        assert!(painter.did_exceed_max_lines());
        assert_eq!(painter.compute_line_metrics().len(), 1);
    }

    #[test]
    fn positions_words_and_lines_round_trip() {
        let mut fonts = fonts();
        let mut painter = painter("ab cd");
        painter.layout(&mut fonts, 0.0, f64::INFINITY);
        let end = painter.get_position_for_offset(Offset::new(painter.width() + 10.0, 1.0));
        assert_eq!(end.offset, 5);
        assert_eq!(
            painter.get_word_boundary(TextPosition::new(4)),
            TextRange::new(3, 5)
        );
        assert_eq!(
            painter.get_line_boundary(TextPosition::new(1)),
            TextRange::new(0, 5)
        );
        let boxes = painter.get_boxes_for_selection(
            TextSelection::new(0, 5),
            BoxHeightStyle::Tight,
            BoxWidthStyle::Tight,
        );
        assert!(!boxes.is_empty());
        assert_eq!(painter.get_offset_after(0), Some(1));
        assert_eq!(painter.get_offset_before(1), Some(0));
        assert_eq!(painter.get_offset_after(5), None);
    }

    #[test]
    fn preferred_line_height_needs_no_layout() {
        let mut fonts = fonts();
        let mut painter = painter("x");
        assert!(painter.preferred_line_height(&mut fonts) > 0.0);
    }

    #[test]
    fn compute_width_is_the_laid_out_width() {
        let mut fonts = fonts();
        let span = TextSpan::new().text("Hello").into_span();
        let width = TextPainter::compute_width(
            &mut fonts,
            span.clone(),
            TextDirection::Ltr,
            0.0,
            f64::INFINITY,
        );
        let mut painter = TextPainter::new();
        painter.set_text(Some(span));
        painter.set_text_direction(Some(TextDirection::Ltr));
        painter.layout(&mut fonts, 0.0, f64::INFINITY);
        assert_eq!(width, painter.width());
    }
}
