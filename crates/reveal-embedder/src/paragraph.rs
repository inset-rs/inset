//! Flutter counterpart: `engine/src/flutter/lib/ui/text.dart`, the paragraph half:
//! `TextStyle`, `ParagraphStyle`, `ParagraphBuilder`, `Paragraph`, and the position, range,
//! box, and line-metric value types.
//!
//! Flutter's engine implements these on top of Skia's paragraph library; here they wrap valo's.
//! Offsets are UTF-16 code units, as in Dart; valo works in UTF-8 bytes and the wrapper
//! translates.

use std::fmt;
use std::ops::Range;

use crate::fonts::FontCollection;
use crate::text::{
    FontFeature, FontStyle, FontVariation, FontWeight, TextAlign, TextBaseline, TextDecoration,
    TextDecorationStyle, TextDirection, TextHeightBehavior, TextLeadingDistribution,
};
use crate::{Canvas, Color, Offset, Paint, Rect, Shadow};

/// A position in a string of text.
///
/// A `TextPosition` can be used to describe a caret position in between characters. The
/// [`offset`](Self::offset) points to the position between `offset - 1` and `offset`
/// characters of the string, and the [`affinity`](Self::affinity) is used to describe which
/// character this position affiliates with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextPosition {
    /// The index of the character that immediately follows the position in the string
    /// representation of the text.
    pub offset: i32,
    /// Disambiguates cases where the offset could be at the end of one line or the
    /// start of the next.
    pub affinity: TextAffinity,
}

impl TextPosition {
    /// Creates an object representing a particular position in a string, with downstream
    /// affinity.
    pub const fn new(offset: i32) -> TextPosition {
        TextPosition {
            offset,
            affinity: TextAffinity::Downstream,
        }
    }

    /// Creates an object representing a particular position in a string.
    pub const fn with_affinity(offset: i32, affinity: TextAffinity) -> TextPosition {
        TextPosition { offset, affinity }
    }
}

/// A way to disambiguate a [`TextPosition`] when its offset could match two
/// different locations in the rendered string.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextAffinity {
    /// The position has affinity for the upstream side of the text position, i.e. in the
    /// direction of the beginning of the string.
    Upstream,
    /// The position has affinity for the downstream side of the text position, i.e. in the
    /// direction of the end of the string.
    #[default]
    Downstream,
}

/// A range of characters in a string of text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextRange {
    /// The index of the first character in the range.
    ///
    /// If [`start`](Self::start) and [`end`](Self::end) are both -1, the text range is empty.
    pub start: i32,
    /// The next index after the characters in this range.
    pub end: i32,
}

impl TextRange {
    /// A text range that starts and ends at offset -1: the empty range.
    pub const EMPTY: TextRange = TextRange { start: -1, end: -1 };

    /// Creates a new [`TextRange`]. The `start` and `end` must be at least -1.
    pub const fn new(start: i32, end: i32) -> TextRange {
        debug_assert!(start >= -1);
        debug_assert!(end >= -1);
        TextRange { start, end }
    }

    /// A text range that starts and ends at `offset`.
    pub const fn collapsed(offset: i32) -> TextRange {
        debug_assert!(offset >= -1);
        TextRange {
            start: offset,
            end: offset,
        }
    }

    /// Whether this range represents a valid position in the text.
    pub fn is_valid(&self) -> bool {
        self.start >= 0 && self.end >= 0
    }

    /// Whether this range is empty (but still potentially placed inside the text).
    pub fn is_collapsed(&self) -> bool {
        self.start == self.end
    }

    /// Whether the start of this range precedes the end.
    pub fn is_normalized(&self) -> bool {
        self.end >= self.start
    }

    /// The text before this range.
    pub fn text_before<'a>(&self, text: &'a str) -> &'a str {
        debug_assert!(self.is_normalized());
        let start = byte_of_code_unit(text, self.start as usize);
        &text[..start]
    }

    /// The text after this range.
    pub fn text_after<'a>(&self, text: &'a str) -> &'a str {
        debug_assert!(self.is_normalized());
        let end = byte_of_code_unit(text, self.end as usize);
        &text[end..]
    }

    /// The text inside this range.
    pub fn text_inside<'a>(&self, text: &'a str) -> &'a str {
        debug_assert!(self.is_normalized());
        let start = byte_of_code_unit(text, self.start as usize);
        let end = byte_of_code_unit(text, self.end as usize);
        &text[start..end]
    }
}

/// Layout constraints for [`Paragraph`] objects.
///
/// Instances of this class are typically used with [`Paragraph::layout`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParagraphConstraints {
    /// The width the paragraph should use whey computing the positions of glyphs.
    ///
    /// If possible, the paragraph will select a soft line break prior to reaching this width.
    /// If no soft line break is available, the paragraph will select a hard line break prior
    /// to reaching this width. If that would force a line break without any glyphs, the
    /// paragraph will place the first glyph on the line anyway.
    pub width: f64,
}

impl ParagraphConstraints {
    /// Creates constraints for laying out a paragraph.
    pub const fn new(width: f64) -> ParagraphConstraints {
        ParagraphConstraints { width }
    }
}

/// A rectangle enclosing a run of text.
///
/// This is similar to [`Rect`] but includes an inherent [`TextDirection`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextBox {
    /// The left edge of the text box, relative to the parent widget.
    pub left: f64,
    /// The top edge of the text box, relative to the parent widget.
    pub top: f64,
    /// The right edge of the text box, relative to the parent widget.
    pub right: f64,
    /// The bottom edge of the text box, relative to the parent widget.
    pub bottom: f64,
    /// The direction in which text inside this box flows.
    pub direction: TextDirection,
}

impl TextBox {
    /// Creates an object that describes a box containing text.
    pub const fn from_ltrbd(
        left: f64,
        top: f64,
        right: f64,
        bottom: f64,
        direction: TextDirection,
    ) -> TextBox {
        TextBox {
            left,
            top,
            right,
            bottom,
            direction,
        }
    }

    /// Returns a rect of the same size as this box.
    pub fn to_rect(&self) -> Rect {
        Rect::from_ltrb(self.left, self.top, self.right, self.bottom)
    }

    /// The [`left`](Self::left) edge of the box for left-to-right text; the
    /// [`right`](Self::right) edge of the box for right-to-left text.
    pub fn start(&self) -> f64 {
        match self.direction {
            TextDirection::Ltr => self.left,
            TextDirection::Rtl => self.right,
        }
    }

    /// The [`right`](Self::right) edge of the box for left-to-right text; the
    /// [`left`](Self::left) edge of the box for right-to-left text.
    pub fn end(&self) -> f64 {
        match self.direction {
            TextDirection::Ltr => self.right,
            TextDirection::Rtl => self.left,
        }
    }
}

/// Where to vertically align the placeholder relative to the surrounding text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlaceholderAlignment {
    Baseline,
    AboveBaseline,
    BelowBaseline,
    Top,
    Bottom,
    Middle,
}

/// Defines various ways to vertically bound the boxes returned by
/// [`Paragraph::get_boxes_for_range`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BoxHeightStyle {
    #[default]
    Tight,
    Max,
    IncludeLineSpacingMiddle,
    IncludeLineSpacingTop,
    IncludeLineSpacingBottom,
    Strut,
}

/// Defines various ways to horizontally bound the boxes returned by
/// [`Paragraph::get_boxes_for_range`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BoxWidthStyle {
    #[default]
    Tight,
    Max,
}

/// The measurements of a character (or a sequence of visually connected characters) within
/// a paragraph.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphInfo {
    /// The layout bounding rect of the associated character, in the paragraph's coordinates.
    pub grapheme_cluster_layout_bounds: Rect,
    /// The UTF-16 range of the associated character in the text.
    pub grapheme_cluster_code_unit_range: TextRange,
    /// The writing direction within the glyph.
    pub writing_direction: TextDirection,
}

/// [`LineMetrics`] stores the measurements and statistics of a single line in the paragraph.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineMetrics {
    /// True if this line ends with an explicit line break (e.g. `\n`) or is the end of the
    /// paragraph. False otherwise.
    pub hard_break: bool,
    /// The rise from the [`baseline`](Self::baseline) as calculated from the font and style
    /// for this line.
    pub ascent: f64,
    /// The drop from the [`baseline`](Self::baseline) as calculated from the font and style
    /// for this line.
    pub descent: f64,
    /// The rise from the [`baseline`](Self::baseline) as calculated from the font and style
    /// for this line ignoring the `TextStyle.height`.
    pub unscaled_ascent: f64,
    /// Total height of the line from the top edge to the bottom edge.
    pub height: f64,
    /// Width of the line from the left edge of the leftmost glyph to the right edge of the
    /// rightmost glyph.
    pub width: f64,
    /// The x coordinate of left edge of the line.
    pub left: f64,
    /// The y coordinate of the baseline for this line from the top of the paragraph.
    pub baseline: f64,
    /// The number of this line in the overall paragraph, with the first line being index zero.
    pub line_number: i32,
}

/// An opaque object that determines the size, position, and rendering of text.
///
/// Every field is optional, as in Dart: a field left `None` inherits from the style below it
/// on the [`ParagraphBuilder`]'s stack, or from the paragraph style.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextStyle {
    pub color: Option<Color>,
    pub decoration: Option<TextDecoration>,
    pub decoration_color: Option<Color>,
    pub decoration_style: Option<TextDecorationStyle>,
    pub decoration_thickness: Option<f64>,
    pub font_weight: Option<FontWeight>,
    pub font_style: Option<FontStyle>,
    pub text_baseline: Option<TextBaseline>,
    pub font_family: Option<String>,
    pub font_family_fallback: Option<Vec<String>>,
    pub font_size: Option<f64>,
    pub letter_spacing: Option<f64>,
    pub word_spacing: Option<f64>,
    pub height: Option<f64>,
    pub leading_distribution: Option<TextLeadingDistribution>,
    pub background: Option<Paint>,
    pub foreground: Option<Paint>,
    pub shadows: Option<Vec<Shadow>>,
    pub font_features: Option<Vec<FontFeature>>,
    pub font_variations: Option<Vec<FontVariation>>,
}

impl TextStyle {
    /// Creates a new text style with every field unset.
    pub fn new() -> TextStyle {
        TextStyle::default()
    }

    /// The style with `other`'s set fields laid over this one, the engine's `pushStyle`
    /// inheritance.
    fn merged_with(&self, other: &TextStyle) -> TextStyle {
        TextStyle {
            color: other.color.or(self.color),
            decoration: other.decoration.or(self.decoration),
            decoration_color: other.decoration_color.or(self.decoration_color),
            decoration_style: other.decoration_style.or(self.decoration_style),
            decoration_thickness: other.decoration_thickness.or(self.decoration_thickness),
            font_weight: other.font_weight.or(self.font_weight),
            font_style: other.font_style.or(self.font_style),
            text_baseline: other.text_baseline.or(self.text_baseline),
            font_family: other
                .font_family
                .clone()
                .or_else(|| self.font_family.clone()),
            font_family_fallback: other
                .font_family_fallback
                .clone()
                .or_else(|| self.font_family_fallback.clone()),
            font_size: other.font_size.or(self.font_size),
            letter_spacing: other.letter_spacing.or(self.letter_spacing),
            word_spacing: other.word_spacing.or(self.word_spacing),
            height: other.height.or(self.height),
            leading_distribution: other.leading_distribution.or(self.leading_distribution),
            background: other.background.clone().or_else(|| self.background.clone()),
            foreground: other.foreground.clone().or_else(|| self.foreground.clone()),
            shadows: other.shadows.clone().or_else(|| self.shadows.clone()),
            font_features: other
                .font_features
                .clone()
                .or_else(|| self.font_features.clone()),
            font_variations: other
                .font_variations
                .clone()
                .or_else(|| self.font_variations.clone()),
        }
    }

    /// The host span style: unset fields take the host's defaults (14px, black).
    fn to_native(&self) -> valo::TextStyle {
        let mut style = valo::TextStyle::default();
        let mut families = Vec::new();
        if let Some(family) = &self.font_family {
            families.push(family.clone());
        }
        if let Some(fallback) = &self.font_family_fallback {
            families.extend(fallback.iter().cloned());
        }
        style.families = families;
        if let Some(weight) = self.font_weight {
            style.weight = weight.value as u16;
        }
        style.italic = self.font_style == Some(FontStyle::Italic);
        if let Some(size) = self.font_size {
            style.size = size as f32;
        }
        if let Some(foreground) = &self.foreground {
            style.color = foreground.color;
        } else if let Some(color) = self.color {
            style.color = color.into();
        }
        if let Some(letter_spacing) = self.letter_spacing {
            style.letter_spacing = letter_spacing as f32;
        }
        if let Some(word_spacing) = self.word_spacing {
            style.word_spacing = word_spacing as f32;
        }
        style.height = match self.height {
            Some(height) if height != crate::text::K_TEXT_HEIGHT_NONE => Some(height as f32),
            _ => None,
        };
        style.decoration = native_decoration(
            self.decoration,
            self.decoration_color,
            self.decoration_thickness,
        );
        if let Some(shadows) = &self.shadows {
            style.shadows = shadows
                .iter()
                .map(|shadow| valo::Shadow {
                    color: shadow.color.into(),
                    offset: shadow.offset.into(),
                    blur: shadow.blur_sigma() as f32,
                })
                .collect();
        }
        style
    }
}

/// Underline before overline before line-through: valo draws one decoration per run.
fn native_decoration(
    decoration: Option<TextDecoration>,
    color: Option<Color>,
    thickness: Option<f64>,
) -> Option<valo::Decoration> {
    let decoration = decoration?;
    let kind = if decoration.contains(TextDecoration::UNDERLINE) {
        valo::DecorationKind::Underline
    } else if decoration.contains(TextDecoration::OVERLINE) {
        valo::DecorationKind::Overline
    } else if decoration.contains(TextDecoration::LINE_THROUGH) {
        valo::DecorationKind::LineThrough
    } else {
        return None;
    };
    Some(valo::Decoration {
        kind,
        color: color.map(Into::into),
        thickness: thickness.unwrap_or(1.0) as f32,
    })
}

/// An opaque object that determines the configuration used by [`ParagraphBuilder`] to
/// position lines within a [`Paragraph`] of text.
///
/// The font fields (`font_family`, `font_size`, `height`, `font_weight`, `font_style`) are
/// the style for text that no pushed [`TextStyle`] covers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParagraphStyle {
    pub text_align: Option<TextAlign>,
    pub text_direction: Option<TextDirection>,
    pub max_lines: Option<i32>,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub height: Option<f64>,
    pub text_height_behavior: Option<TextHeightBehavior>,
    pub font_weight: Option<FontWeight>,
    pub font_style: Option<FontStyle>,
    pub ellipsis: Option<String>,
}

impl ParagraphStyle {
    /// Creates a new paragraph style with every field unset.
    pub fn new() -> ParagraphStyle {
        ParagraphStyle::default()
    }

    /// The text style the paragraph style implies for text with no pushed style.
    fn base_text_style(&self) -> TextStyle {
        TextStyle {
            font_family: self.font_family.clone(),
            font_size: self.font_size,
            height: self.height,
            font_weight: self.font_weight,
            font_style: self.font_style,
            ..TextStyle::default()
        }
    }

    fn to_native(&self) -> valo::ParagraphStyle {
        valo::ParagraphStyle {
            align: native_align(self.text_align, self.text_direction),
            direction: self.text_direction.map(native_direction),
            preserve_trailing_whitespace: false,
            max_lines: self.max_lines.map(|max_lines| max_lines as u32),
            ellipsis: self.ellipsis.clone(),
        }
    }
}

/// valo stores a physical alignment: `Start` / `End` resolve against the direction, LTR when
/// none is given.
fn native_align(align: Option<TextAlign>, direction: Option<TextDirection>) -> valo::TextAlign {
    match align {
        None | Some(TextAlign::Left) => valo::TextAlign::Left,
        Some(TextAlign::Right) => valo::TextAlign::Right,
        Some(TextAlign::Center) => valo::TextAlign::Center,
        Some(TextAlign::Justify) => valo::TextAlign::Justify,
        Some(TextAlign::Start) => match direction {
            Some(TextDirection::Rtl) => valo::TextAlign::Right,
            _ => valo::TextAlign::Left,
        },
        Some(TextAlign::End) => match direction {
            Some(TextDirection::Rtl) => valo::TextAlign::Left,
            _ => valo::TextAlign::Right,
        },
    }
}

fn native_direction(direction: TextDirection) -> valo::TextDirection {
    match direction {
        TextDirection::Ltr => valo::TextDirection::Ltr,
        TextDirection::Rtl => valo::TextDirection::Rtl,
    }
}

/// Builds a [`Paragraph`] containing text with the given styling information.
///
/// To set the paragraph's alignment, truncation, and ellipsizing behavior, pass an
/// appropriately-configured [`ParagraphStyle`] object to [`ParagraphBuilder::new`].
///
/// Then, call combinations of [`push_style`](Self::push_style), [`add_text`](Self::add_text),
/// and [`pop`](Self::pop) to add styled text to the object.
///
/// Finally, call [`build`](Self::build) to obtain the constructed [`Paragraph`] object. After
/// this point, the builder is no longer usable.
///
/// After constructing a [`Paragraph`], call [`Paragraph::layout`] on it and then paint it
/// with [`CanvasText::draw_paragraph`].
pub struct ParagraphBuilder {
    paragraph_style: ParagraphStyle,
    style_stack: Vec<TextStyle>,
    runs: Vec<(String, valo::TextStyle)>,
}

impl ParagraphBuilder {
    /// Creates a new [`ParagraphBuilder`] object, which is used to create a [`Paragraph`].
    pub fn new(style: ParagraphStyle) -> ParagraphBuilder {
        let base = style.base_text_style();
        ParagraphBuilder {
            paragraph_style: style,
            style_stack: vec![base],
            runs: Vec::new(),
        }
    }

    /// The number of placeholders currently in the paragraph.
    ///
    /// Placeholders are not supported by the host; this is always zero.
    pub fn placeholder_count(&self) -> i32 {
        0
    }

    /// Applies the given style to the added text until [`pop`](Self::pop) is called.
    ///
    /// Fields the style leaves unset inherit from the style below it.
    pub fn push_style(&mut self, style: TextStyle) {
        let current = self
            .style_stack
            .last()
            .expect("the base style is never popped");
        let merged = current.merged_with(&style);
        self.style_stack.push(merged);
    }

    /// Ends the effect of the most recent call to [`push_style`](Self::push_style).
    ///
    /// Internally, the paragraph builder maintains a stack of text styles. Text added to the
    /// paragraph is affected by all the styles in the stack. Calling [`pop`](Self::pop)
    /// removes the topmost style in the stack, leaving the remaining styles in effect.
    pub fn pop(&mut self) {
        if self.style_stack.len() > 1 {
            self.style_stack.pop();
        }
    }

    /// Adds the given text to the paragraph.
    ///
    /// The text will be styled according to the current stack of text styles.
    pub fn add_text(&mut self, text: &str) {
        let style = self
            .style_stack
            .last()
            .expect("the base style is never popped")
            .to_native();
        self.runs.push((text.to_owned(), style));
    }

    /// Applies the given paragraph style and returns a [`Paragraph`] containing the added
    /// text and associated styling, shaped against `fonts`.
    ///
    /// After calling this function, the paragraph builder object is invalid and cannot be
    /// used further.
    pub fn build(self, fonts: &mut FontCollection) -> Paragraph {
        let direction = self
            .paragraph_style
            .text_direction
            .unwrap_or(TextDirection::Ltr);
        let mut builder = valo::ParagraphBuilder::new(fonts);
        builder.style(self.paragraph_style.to_native());
        for (text, style) in &self.runs {
            builder.add_text(text, style);
        }
        let inner = builder.build();
        Paragraph {
            code_units: CodeUnits::of(inner.text()),
            inner,
            direction,
            layout_width: 0.0,
            disposed: false,
        }
    }
}

/// The map between Dart's UTF-16 code unit offsets and valo's UTF-8 byte offsets.
#[derive(Clone, Debug)]
struct CodeUnits {
    /// `byte_of[code_unit]` for every code unit boundary, plus the end.
    byte_of: Vec<usize>,
    /// The number of UTF-16 code units.
    len: usize,
}

impl CodeUnits {
    fn of(text: &str) -> CodeUnits {
        let mut byte_of = Vec::with_capacity(text.len() + 1);
        for (byte, ch) in text.char_indices() {
            byte_of.push(byte);
            if ch.len_utf16() == 2 {
                // A surrogate pair: the second code unit sits inside the character.
                byte_of.push(byte);
            }
        }
        byte_of.push(text.len());
        let len = byte_of.len() - 1;
        CodeUnits { byte_of, len }
    }

    fn byte(&self, code_unit: i32) -> usize {
        let clamped = code_unit.clamp(0, self.len as i32) as usize;
        self.byte_of[clamped]
    }

    /// The first code unit at or before `byte` (a surrogate pair's two units share a byte).
    fn code_unit(&self, byte: usize) -> i32 {
        let first_at_or_after = self.byte_of.partition_point(|candidate| *candidate < byte);
        match self.byte_of.get(first_at_or_after) {
            Some(candidate) if *candidate == byte => first_at_or_after as i32,
            _ => first_at_or_after.saturating_sub(1) as i32,
        }
    }
}

/// The byte offset of the given UTF-16 code unit in `text`.
fn byte_of_code_unit(text: &str, code_unit: usize) -> usize {
    CodeUnits::of(text).byte(code_unit as i32)
}

/// A paragraph of text.
///
/// A paragraph retains the size and position of each glyph in the text and can be
/// efficiently resized and painted.
///
/// To create a [`Paragraph`] object, use a [`ParagraphBuilder`].
///
/// Paragraphs can be displayed on a [`Canvas`] using [`CanvasText::draw_paragraph`].
pub struct Paragraph {
    inner: valo::Paragraph,
    code_units: CodeUnits,
    direction: TextDirection,
    /// The width given to the last [`layout`](Self::layout); Skia's `getMaxWidth`.
    layout_width: f64,
    disposed: bool,
}

impl Paragraph {
    /// The amount of horizontal space this paragraph occupies.
    ///
    /// Valid only after [`layout`](Self::layout) has been called. As in Skia, this is the
    /// width the paragraph was laid out at (possibly infinite), not the widest line: the
    /// lines are aligned within it.
    pub fn width(&self) -> f64 {
        self.layout_width
    }

    /// The amount of vertical space this paragraph occupies.
    ///
    /// Valid only after [`layout`](Self::layout) has been called.
    pub fn height(&self) -> f64 {
        f64::from(self.inner.height())
    }

    /// The distance from the left edge of the leftmost glyph to the right edge of the
    /// rightmost glyph in the paragraph.
    pub fn longest_line(&self) -> f64 {
        f64::from(self.inner.longest_line())
    }

    /// The minimum width that this paragraph could be without failing to paint its
    /// contents within itself.
    pub fn min_intrinsic_width(&self) -> f64 {
        f64::from(self.inner.min_intrinsic_width())
    }

    /// Returns the smallest width beyond which increasing the width never decreases the
    /// height.
    pub fn max_intrinsic_width(&self) -> f64 {
        f64::from(self.inner.max_intrinsic_width())
    }

    /// The distance from the top of the paragraph to the alphabetic baseline of the first
    /// line, in logical pixels.
    pub fn alphabetic_baseline(&self) -> f64 {
        self.inner
            .lines()
            .first()
            .map_or(0.0, |line| f64::from(line.baseline))
    }

    /// The distance from the top of the paragraph to the ideographic baseline of the first
    /// line, in logical pixels.
    ///
    /// The host has no ideographic metrics; this is the bottom of the first line.
    pub fn ideographic_baseline(&self) -> f64 {
        self.inner
            .lines()
            .first()
            .map_or(0.0, |line| f64::from(line.baseline + line.descent))
    }

    /// True if there is more vertical content, but the text was truncated, either because we
    /// reached `max_lines` lines of text or because the `max_lines` was `None`, `ellipsis`
    /// was not `None`, and one of the lines exceeded the width constraint.
    pub fn did_exceed_max_lines(&self) -> bool {
        self.inner.truncated()
    }

    /// Computes the size and position of each glyph in the paragraph.
    ///
    /// The [`ParagraphConstraints`] control how wide the text is allowed to be.
    pub fn layout(&mut self, constraints: ParagraphConstraints) {
        debug_assert!(!self.disposed);
        self.layout_width = constraints.width;
        self.inner.layout(constraints.width as f32);
    }

    /// Returns a list of text boxes that enclose the given text range.
    ///
    /// The host returns tight boxes only: `box_height_style` and `box_width_style` do not
    /// change the result.
    pub fn get_boxes_for_range(
        &self,
        start: i32,
        end: i32,
        box_height_style: BoxHeightStyle,
        box_width_style: BoxWidthStyle,
    ) -> Vec<TextBox> {
        let _ = (box_height_style, box_width_style);
        let range = self.code_units.byte(start)..self.code_units.byte(end);
        self.inner
            .rects_for_range(range)
            .into_iter()
            .map(|rect| self.text_box(rect))
            .collect()
    }

    fn text_box(&self, rect: valo::Rect) -> TextBox {
        TextBox::from_ltrbd(
            f64::from(rect.x),
            f64::from(rect.y),
            f64::from(rect.x + rect.width),
            f64::from(rect.y + rect.height),
            self.direction,
        )
    }

    /// Returns a list of text boxes that enclose all placeholders in the paragraph.
    ///
    /// The host has no placeholders; this is always empty.
    pub fn get_boxes_for_placeholders(&self) -> Vec<TextBox> {
        Vec::new()
    }

    /// Returns the text position closest to the given offset.
    pub fn get_position_for_offset(&self, offset: Offset) -> TextPosition {
        let position = self.inner.glyph_position_at(offset.into());
        TextPosition::with_affinity(
            self.code_units.code_unit(position.offset),
            if position.downstream {
                TextAffinity::Downstream
            } else {
                TextAffinity::Upstream
            },
        )
    }

    /// Returns the [`GlyphInfo`] of the glyph closest to the given `offset` in the paragraph
    /// coordinate system, or `None` if if the text is empty, or is entirely clipped or
    /// ellipsized away.
    pub fn get_closest_glyph_info_for_offset(&self, offset: Offset) -> Option<GlyphInfo> {
        let position = self.inner.glyph_position_at(offset.into());
        let anchor = if position.downstream {
            position.offset
        } else {
            position.offset.saturating_sub(1)
        };
        self.glyph_info_at_byte(anchor)
    }

    /// Returns the [`GlyphInfo`] located at the given UTF-16 `code_unit_offset` in the
    /// paragraph, or `None` if the given `code_unit_offset` is out of the visible lines' range.
    pub fn get_glyph_info_at(&self, code_unit_offset: i32) -> Option<GlyphInfo> {
        if code_unit_offset < 0 || code_unit_offset as usize >= self.code_units.len {
            return None;
        }
        self.glyph_info_at_byte(self.code_units.byte(code_unit_offset))
    }

    /// The glyph whose cluster contains `byte`: its layout box spans the glyph's advance
    /// horizontally and the line vertically.
    fn glyph_info_at_byte(&self, byte: usize) -> Option<GlyphInfo> {
        for line in self.inner.lines() {
            if !line.range.contains(&byte) {
                continue;
            }
            let top = line.baseline - line.ascent;
            let bottom = line.baseline + line.descent;
            let cluster_starts = cluster_starts(line);
            for run in &line.runs {
                for glyph in &run.glyphs {
                    let cluster = cluster_range(&cluster_starts, glyph.cluster, line.range.end);
                    if !cluster.contains(&byte) {
                        continue;
                    }
                    let (left, right) = if glyph.advance >= 0.0 {
                        (glyph.x, glyph.x + glyph.advance)
                    } else {
                        (glyph.x + glyph.advance, glyph.x)
                    };
                    return Some(GlyphInfo {
                        grapheme_cluster_layout_bounds: Rect::from_ltrb(
                            f64::from(left),
                            f64::from(top),
                            f64::from(right),
                            f64::from(bottom),
                        ),
                        grapheme_cluster_code_unit_range: TextRange::new(
                            self.code_units.code_unit(cluster.start),
                            self.code_units.code_unit(cluster.end),
                        ),
                        writing_direction: if run.rtl {
                            TextDirection::Rtl
                        } else {
                            TextDirection::Ltr
                        },
                    });
                }
            }
        }
        None
    }

    /// Returns the [`TextRange`] of the word at the given [`TextPosition`].
    pub fn get_word_boundary(&self, position: TextPosition) -> TextRange {
        let range = self
            .inner
            .word_boundary(self.code_units.byte(position.offset));
        self.text_range(range)
    }

    /// Returns the [`TextRange`] of the line at the given [`TextPosition`].
    ///
    /// The newline (if any) is not returned as part of the range.
    pub fn get_line_boundary(&self, position: TextPosition) -> TextRange {
        let byte = self.code_units.byte(position.offset);
        let Some(line) = self.line_containing(byte, position.affinity) else {
            return TextRange::EMPTY;
        };
        let text = self.inner.text();
        let mut end = line.range.end;
        if end > line.range.start && text.as_bytes().get(end - 1) == Some(&b'\n') {
            end -= 1;
        }
        self.text_range(line.range.start..end)
    }

    fn line_containing(&self, byte: usize, affinity: TextAffinity) -> Option<&valo::Line> {
        let lines = self.inner.lines();
        let index = lines.iter().position(|line| {
            let starts_next_line = byte == line.range.start && affinity == TextAffinity::Upstream;
            (line.range.contains(&byte) && !starts_next_line) || byte == line.range.end
        });
        index.and_then(|index| lines.get(index)).or_else(|| {
            let last = lines.last()?;
            (byte >= last.range.end).then_some(last)
        })
    }

    fn text_range(&self, range: Range<usize>) -> TextRange {
        TextRange::new(
            self.code_units.code_unit(range.start),
            self.code_units.code_unit(range.end),
        )
    }

    /// Returns the full list of [`LineMetrics`] that describe in detail the various metrics
    /// of each laid out line.
    pub fn compute_line_metrics(&self) -> Vec<LineMetrics> {
        let lines = self.inner.lines();
        let text = self.inner.text();
        lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                let is_last = index + 1 == lines.len();
                let ends_with_newline = line.range.end > line.range.start
                    && text.as_bytes().get(line.range.end - 1) == Some(&b'\n');
                LineMetrics {
                    hard_break: is_last || ends_with_newline,
                    ascent: f64::from(line.ascent),
                    descent: f64::from(line.descent),
                    unscaled_ascent: f64::from(line.ascent),
                    height: f64::from(line.ascent + line.descent),
                    width: f64::from(line.width),
                    left: f64::from(line.left),
                    baseline: f64::from(line.baseline),
                    line_number: index as i32,
                }
            })
            .collect()
    }

    /// Returns the [`LineMetrics`] for the line at `line_number`, or `None` if the given
    /// `line_number` is greater than or equal to [`number_of_lines`](Self::number_of_lines).
    pub fn get_line_metrics_at(&self, line_number: i32) -> Option<LineMetrics> {
        if line_number < 0 {
            return None;
        }
        self.compute_line_metrics()
            .into_iter()
            .nth(line_number as usize)
    }

    /// The total number of visible lines in the paragraph.
    ///
    /// Returns a non-negative number. If `max_lines` is set, the value of this getter never
    /// exceeds `max_lines`.
    pub fn number_of_lines(&self) -> i32 {
        self.inner.lines().len() as i32
    }

    /// Returns the line number of the line that contains the code unit that
    /// `code_unit_offset` points to, or `None` if the `code_unit_offset` is out of range.
    pub fn get_line_number_at(&self, code_unit_offset: i32) -> Option<i32> {
        if code_unit_offset < 0 || code_unit_offset as usize >= self.code_units.len {
            return None;
        }
        let byte = self.code_units.byte(code_unit_offset);
        self.inner
            .lines()
            .iter()
            .position(|line| line.range.contains(&byte))
            .map(|index| index as i32)
    }

    /// Release the resources used by this object. The object is no longer usable after this
    /// method is called.
    pub fn dispose(&mut self) {
        debug_assert!(!self.disposed);
        self.disposed = true;
    }

    /// Whether this reference to the underlying paragraph is disposed.
    pub fn debug_disposed(&self) -> bool {
        self.disposed
    }

    /// The host paragraph, for drawing.
    pub(crate) fn native(&self) -> &valo::Paragraph {
        &self.inner
    }
}

/// Every cluster start on the line, in text order. A cluster runs from its start to the next
/// start on the line, whichever run that glyph sits in.
fn cluster_starts(line: &valo::Line) -> Vec<usize> {
    let mut starts: Vec<usize> = line
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.cluster))
        .collect();
    starts.sort_unstable();
    starts.dedup();
    starts
}

fn cluster_range(cluster_starts: &[usize], start: usize, line_end: usize) -> Range<usize> {
    let next = cluster_starts
        .iter()
        .find(|candidate| **candidate > start)
        .copied()
        .unwrap_or(line_end);
    start..next.max(start + 1)
}

impl fmt::Debug for Paragraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Paragraph")
            .field("text", &self.inner.text())
            .field("width", &self.width())
            .field("height", &self.height())
            .finish()
    }
}

/// Paragraph drawing on a [`Canvas`] (Dart's `Canvas.drawParagraph`).
pub trait CanvasText {
    /// Draws the text in the given [`Paragraph`] into this canvas at the given [`Offset`].
    ///
    /// The [`Paragraph`] object must have had [`Paragraph::layout`] called on it first.
    fn draw_paragraph(&mut self, paragraph: &Paragraph, offset: Offset);
}

impl CanvasText for Canvas {
    fn draw_paragraph(&mut self, paragraph: &Paragraph, offset: Offset) {
        valo::DrawParagraphExt::draw_paragraph(self, paragraph.native(), offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_units_map_surrogate_pairs_to_one_character() {
        let units = CodeUnits::of("a😀b");
        assert_eq!(units.len, 4);
        assert_eq!(units.byte(0), 0);
        assert_eq!(units.byte(1), 1);
        assert_eq!(
            units.byte(2),
            1,
            "the second half of the pair maps to the same byte"
        );
        assert_eq!(units.byte(3), 5);
        assert_eq!(units.byte(4), 6);
        assert_eq!(units.code_unit(5), 3);
        assert_eq!(units.code_unit(6), 4);
    }

    #[test]
    fn text_range_slices_by_code_unit() {
        let range = TextRange::new(1, 3);
        assert_eq!(range.text_inside("a😀b"), "😀");
        assert_eq!(range.text_before("a😀b"), "a");
        assert_eq!(range.text_after("a😀b"), "b");
        assert!(TextRange::EMPTY.is_collapsed());
        assert!(!TextRange::EMPTY.is_valid());
    }

    #[test]
    fn text_box_start_and_end_follow_direction() {
        let ltr = TextBox::from_ltrbd(1.0, 2.0, 3.0, 4.0, TextDirection::Ltr);
        let rtl = TextBox::from_ltrbd(1.0, 2.0, 3.0, 4.0, TextDirection::Rtl);
        assert_eq!(ltr.start(), 1.0);
        assert_eq!(ltr.end(), 3.0);
        assert_eq!(rtl.start(), 3.0);
        assert_eq!(rtl.end(), 1.0);
        assert_eq!(ltr.to_rect(), Rect::from_ltrb(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn pushed_styles_inherit_unset_fields() {
        let mut builder = ParagraphBuilder::new(ParagraphStyle {
            font_size: Some(20.0),
            ..ParagraphStyle::default()
        });
        builder.push_style(TextStyle {
            color: Some(Color::from_argb(255, 255, 0, 0)),
            ..TextStyle::default()
        });
        builder.add_text("x");
        builder.pop();
        builder.add_text("y");
        let styles: Vec<&valo::TextStyle> = builder.runs.iter().map(|(_, s)| s).collect();
        assert_eq!(styles[0].size, 20.0, "the child keeps the paragraph's size");
        assert_eq!(styles[0].color, Color::from_argb(255, 255, 0, 0).into());
        assert_eq!(styles[1].size, 20.0);
        assert_eq!(
            styles[1].color,
            valo::Color::BLACK,
            "popping restores the base"
        );
    }

    #[test]
    fn start_and_end_alignment_resolve_against_direction() {
        assert_eq!(
            native_align(Some(TextAlign::Start), Some(TextDirection::Rtl)),
            valo::TextAlign::Right
        );
        assert_eq!(
            native_align(Some(TextAlign::End), None),
            valo::TextAlign::Right,
            "no direction reads as LTR"
        );
    }

    #[test]
    fn unspecified_font_family_stays_unset_on_the_native_style() {
        let mut builder = ParagraphBuilder::new(ParagraphStyle::new());
        builder.add_text("x");
        assert!(
            builder.runs[0].1.families.is_empty(),
            "dart:ui leaves fontFamily unset; the collection's default manager supplies the face"
        );
    }
}

#[cfg(test)]
mod layout_tests {
    //! Shaping needs real fonts: these shape against the OS fonts.

    use super::*;

    fn fonts() -> FontCollection {
        let mut fonts = FontCollection::new();
        fonts.add_source(valo_system_fonts::SystemFonts::load());
        fonts
    }

    fn paragraph(text: &str, style: ParagraphStyle, width: f64) -> Paragraph {
        let mut builder = ParagraphBuilder::new(style);
        builder.push_style(TextStyle {
            font_size: Some(20.0),
            ..TextStyle::default()
        });
        builder.add_text(text);
        let mut paragraph = builder.build(&mut fonts());
        paragraph.layout(ParagraphConstraints::new(width));
        paragraph
    }

    #[test]
    fn a_laid_out_paragraph_has_a_size_and_one_line() {
        let paragraph = paragraph("Hello", ParagraphStyle::new(), f64::INFINITY);
        assert_eq!(
            paragraph.width(),
            f64::INFINITY,
            "the layout width, as in Skia"
        );
        assert!(paragraph.longest_line() > 0.0);
        assert!(paragraph.height() > 0.0);
        assert_eq!(paragraph.number_of_lines(), 1);
        assert!(paragraph.alphabetic_baseline() > 0.0);
        assert!(paragraph.alphabetic_baseline() < paragraph.height());
        assert!(!paragraph.did_exceed_max_lines());
        assert_eq!(paragraph.max_intrinsic_width(), paragraph.longest_line());
    }

    #[test]
    fn a_narrow_width_wraps_onto_more_lines() {
        let wide = paragraph("one two three four", ParagraphStyle::new(), f64::INFINITY);
        let narrow = paragraph(
            "one two three four",
            ParagraphStyle::new(),
            wide.longest_line() / 2.0,
        );
        assert!(narrow.number_of_lines() > 1);
        assert!(narrow.height() > wide.height());
        let metrics = narrow.compute_line_metrics();
        assert_eq!(metrics.len() as i32, narrow.number_of_lines());
        assert!(
            metrics.last().unwrap().hard_break,
            "the last line is a hard break"
        );
        assert!(!metrics[0].hard_break, "a wrapped line is a soft break");
        assert_eq!(metrics[1].line_number, 1);
    }

    #[test]
    fn max_lines_with_an_ellipsis_truncates() {
        let wide = paragraph("one two three four", ParagraphStyle::new(), f64::INFINITY);
        let style = ParagraphStyle {
            max_lines: Some(1),
            ellipsis: Some("\u{2026}".to_owned()),
            ..ParagraphStyle::default()
        };
        let clipped = paragraph("one two three four", style, wide.longest_line() / 2.0);
        assert_eq!(clipped.number_of_lines(), 1);
        assert!(clipped.did_exceed_max_lines());
    }

    #[test]
    fn positions_boxes_and_boundaries_use_code_units() {
        let paragraph = paragraph("ab cd", ParagraphStyle::new(), f64::INFINITY);
        let start = paragraph.get_position_for_offset(Offset::new(0.0, 1.0));
        assert_eq!(start.offset, 0);
        let end =
            paragraph.get_position_for_offset(Offset::new(paragraph.longest_line() + 50.0, 1.0));
        assert_eq!(end.offset, 5);

        let boxes =
            paragraph.get_boxes_for_range(0, 2, BoxHeightStyle::Tight, BoxWidthStyle::Tight);
        assert_eq!(boxes.len(), 1);
        assert!(boxes[0].right > boxes[0].left);
        assert_eq!(boxes[0].direction, TextDirection::Ltr);

        assert_eq!(
            paragraph.get_word_boundary(TextPosition::new(4)),
            TextRange::new(3, 5)
        );
        assert_eq!(
            paragraph.get_line_boundary(TextPosition::new(1)),
            TextRange::new(0, 5)
        );
        assert_eq!(paragraph.get_line_number_at(4), Some(0));
        assert_eq!(paragraph.get_line_number_at(5), None);

        let glyph = paragraph.get_glyph_info_at(3).expect("the c glyph");
        assert_eq!(glyph.grapheme_cluster_code_unit_range, TextRange::new(3, 4));
        assert!(glyph.grapheme_cluster_layout_bounds.width() > 0.0);
        assert!(paragraph.get_glyph_info_at(5).is_none());
    }

    #[test]
    fn a_surrogate_pair_is_one_glyph_of_two_code_units() {
        let paragraph = paragraph("a😀b", ParagraphStyle::new(), f64::INFINITY);
        let glyph = paragraph.get_glyph_info_at(1).expect("the emoji");
        assert_eq!(glyph.grapheme_cluster_code_unit_range, TextRange::new(1, 3));
        assert_eq!(
            paragraph
                .get_glyph_info_at(3)
                .unwrap()
                .grapheme_cluster_code_unit_range,
            TextRange::new(3, 4)
        );
    }

    #[test]
    fn drawing_records_the_paragraph() {
        let paragraph = paragraph("Hi", ParagraphStyle::new(), f64::INFINITY);
        let mut canvas = Canvas::new();
        canvas.draw_paragraph(&paragraph, Offset::new(10.0, 10.0));
        let picture = canvas.build();
        assert!(!picture.ops().is_empty());
    }

    #[test]
    fn unspecified_family_at_black_weight_covers_latin() {
        let mut fonts = FontCollection::new();
        crate::set_default_font_manager(
            &mut fonts,
            Box::new(crate::SystemFontSource::platform()),
        );
        let mut builder = ParagraphBuilder::new(ParagraphStyle::new());
        builder.push_style(TextStyle {
            font_size: Some(10.2),
            font_weight: Some(FontWeight::W900),
            ..TextStyle::default()
        });
        builder.add_text("DEBUG");
        let mut paragraph = builder.build(&mut fonts);
        paragraph.layout(ParagraphConstraints::new(f64::INFINITY));
        assert!(paragraph.longest_line() > 0.0);
        assert!(
            fonts.take_unanswered().is_empty(),
            "the engine default family covers Latin at weight 900"
        );
        for line in paragraph.inner.lines() {
            for run in &line.runs {
                assert!(
                    run.glyphs.iter().all(|glyph| glyph.id != 0),
                    "no .notdef tofu"
                );
            }
        }
    }
}
