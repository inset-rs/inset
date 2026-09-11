//! Flutter counterpart: `rendering/paragraph.dart` (`RenderParagraph`).
//!
//! The paragraph is a leaf here: inline children (`WidgetSpan` placeholders), selection
//! (`SelectionRegistrar` / `_SelectableFragment`), the fade overflow shader, and semantics
//! wait; see `PORTING.md`.

use reveal_embedder::{
    BoxHeightStyle, BoxWidthStyle, ClipOp, FontCollection, Offset, Rect, Size, TextAlign,
    TextBaseline, TextBox, TextDirection, TextHeightBehavior, TextPosition, TextRange,
    TextSelection,
};
use reveal_foundation::{App, Handle};
use reveal_painting::{
    ClipContext, InlineSpanRef, PaintingBinding, RenderComparison, TextOverflow, TextPainter,
    TextScaler, TextWidthBasis,
};

use crate::box_::{BoxConstraints, BoxHitTestResult, RenderBox, RenderBoxData};
use crate::object::{AnyRenderObject, Constraints, RenderHandle, RenderObject, RenderObjectData};
use crate::painting_context::PaintingContext;
use crate::{RelayoutWhenSystemFontsChangeData, RelayoutWhenSystemFontsChangeMixin};

const K_ELLIPSIS: &str = "\u{2026}";

/// A render object that displays a paragraph of text.
///
/// Dart's constructor arguments beyond `text` and `textDirection` are the setters. The
/// paragraph shapes against `fonts` when given, else the app-wide collection of
/// `PaintingBinding`.
pub struct RenderParagraph {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    text_painter: TextPainter,
    text_intrinsics: Option<TextPainter>,
    fonts_override: Option<Handle<FontCollection>>,
    soft_wrap: bool,
    overflow: TextOverflow,
    device_pixel_ratio: f64,
    needs_clipping: bool,
    system_fonts: RelayoutWhenSystemFontsChangeData,
}

impl RenderParagraph {
    /// Creates a paragraph render object.
    pub fn new(
        app: &mut App,
        text: InlineSpanRef,
        text_direction: TextDirection,
        fonts: Option<Handle<FontCollection>>,
    ) -> RenderHandle<Self> {
        debug_assert!(text.debug_assert_is_valid());
        let mut text_painter = TextPainter::new();
        text_painter.set_text(Some(text));
        text_painter.set_text_direction(Some(text_direction));
        RenderHandle::new_box(
            app,
            RenderParagraph {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                text_painter,
                text_intrinsics: None,
                fonts_override: fonts,
                soft_wrap: true,
                overflow: TextOverflow::Clip,
                device_pixel_ratio: 1.0,
                needs_clipping: false,
                system_fonts: RelayoutWhenSystemFontsChangeData::new(),
            },
        )
    }

    fn painter(self: RenderHandle<Self>, app: &App) -> &TextPainter {
        &self.get(app).text_painter
    }

    fn painter_mut(self: RenderHandle<Self>, app: &mut App) -> &mut TextPainter {
        &mut self.get_mut(app).text_painter
    }

    /// The collection this paragraph shapes against: the override, else the app-wide one.
    pub fn fonts(self: RenderHandle<Self>, app: &mut App) -> Handle<FontCollection> {
        match self.get(app).fonts_override {
            Some(fonts) => fonts,
            None => PaintingBinding::instance(app).fonts(app),
        }
    }

    /// The painter together with the collection it shapes against.
    fn painter_with_fonts(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> (&mut TextPainter, &mut FontCollection) {
        let fonts = self.fonts(app);
        let (this, fonts) = app.get_disjoint_mut(self.handle(), fonts);
        (&mut this.text_painter, fonts)
    }

    /// The text to display.
    pub fn text(self: RenderHandle<Self>, app: &App) -> InlineSpanRef {
        self.painter(app)
            .text()
            .cloned()
            .expect("a paragraph always has text")
    }

    /// Sets [`text`](Self::text).
    pub fn set_text(self: RenderHandle<Self>, app: &mut App, value: InlineSpanRef) {
        match self.text(app).compare_to(&*value) {
            RenderComparison::Identical => {}
            RenderComparison::Metadata => {
                self.painter_mut(app).set_text(Some(value));
            }
            RenderComparison::Paint => {
                self.painter_mut(app).set_text(Some(value));
                self.mark_needs_paint(app);
            }
            RenderComparison::Layout => {
                self.painter_mut(app).set_text(Some(value));
                self.mark_needs_layout(app);
            }
        }
    }

    /// How the text should be aligned horizontally.
    pub fn text_align(self: RenderHandle<Self>, app: &App) -> TextAlign {
        self.painter(app).text_align()
    }

    /// Sets [`text_align`](Self::text_align).
    pub fn set_text_align(self: RenderHandle<Self>, app: &mut App, value: TextAlign) {
        if self.painter(app).text_align() == value {
            return;
        }
        self.painter_mut(app).set_text_align(value);
        self.mark_needs_paint(app);
    }

    /// The directionality of the text.
    ///
    /// This decides how the [`TextAlign::Start`], [`TextAlign::End`], and
    /// [`TextAlign::Justify`] values of [`text_align`](Self::text_align) are interpreted.
    ///
    /// This is also used to disambiguate how to render bidirectional text. For example, if
    /// the [`text`](Self::text) is an English phrase followed by a Hebrew phrase, in a
    /// [`TextDirection::Ltr`] context the English phrase will be on the left and the Hebrew
    /// phrase to its right, while in a [`TextDirection::Rtl`] context, the English phrase
    /// will be on the right and the Hebrew phrase on its left.
    pub fn text_direction(self: RenderHandle<Self>, app: &App) -> TextDirection {
        self.painter(app)
            .text_direction()
            .expect("a paragraph always has a direction")
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(self: RenderHandle<Self>, app: &mut App, value: TextDirection) {
        if self.painter(app).text_direction() == Some(value) {
            return;
        }
        self.painter_mut(app).set_text_direction(Some(value));
        self.mark_needs_layout(app);
    }

    /// Whether the text should break at soft line breaks.
    ///
    /// If false, the glyphs in the text will be positioned as if there was unlimited
    /// horizontal space.
    ///
    /// If the text exceeds the given number of lines, it will be truncated according to
    /// [`overflow`](Self::overflow) and [`soft_wrap`](Self::soft_wrap).
    pub fn soft_wrap(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).soft_wrap
    }

    /// Sets [`soft_wrap`](Self::soft_wrap).
    pub fn set_soft_wrap(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).soft_wrap == value {
            return;
        }
        self.get_mut(app).soft_wrap = value;
        self.mark_needs_layout(app);
    }

    /// How visual overflow should be handled.
    pub fn overflow(self: RenderHandle<Self>, app: &App) -> TextOverflow {
        self.get(app).overflow
    }

    /// Sets [`overflow`](Self::overflow).
    pub fn set_overflow(self: RenderHandle<Self>, app: &mut App, value: TextOverflow) {
        if self.get(app).overflow == value {
            return;
        }
        self.get_mut(app).overflow = value;
        let ellipsis = (value == TextOverflow::Ellipsis).then(|| K_ELLIPSIS.to_owned());
        self.painter_mut(app).set_ellipsis(ellipsis);
        self.mark_needs_layout(app);
    }

    /// The font scaling strategy to use when laying out and rendering the text.
    pub fn text_scaler(self: RenderHandle<Self>, app: &App) -> TextScaler {
        self.painter(app).text_scaler().clone()
    }

    /// Sets [`text_scaler`](Self::text_scaler).
    pub fn set_text_scaler(self: RenderHandle<Self>, app: &mut App, value: TextScaler) {
        if *self.painter(app).text_scaler() == value {
            return;
        }
        self.painter_mut(app).set_text_scaler(value);
        self.mark_needs_layout(app);
    }

    /// The pixel ratio of the device this paragraph is being rendered on.
    ///
    /// Flutter repaints on change only on the web; here it is bookkeeping.
    pub fn device_pixel_ratio(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).device_pixel_ratio
    }

    /// Sets [`device_pixel_ratio`](Self::device_pixel_ratio).
    pub fn set_device_pixel_ratio(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).device_pixel_ratio == value {
            return;
        }
        self.get_mut(app).device_pixel_ratio = value;
    }

    /// An optional maximum number of lines for the text to span, wrapping if necessary.
    ///
    /// If the text exceeds the given number of lines, it will be truncated according to
    /// [`overflow`](Self::overflow) and [`soft_wrap`](Self::soft_wrap).
    pub fn max_lines(self: RenderHandle<Self>, app: &App) -> Option<i32> {
        self.painter(app).max_lines()
    }

    /// Sets [`max_lines`](Self::max_lines). The value may be `None`. If it is not `None`, then
    /// it must be greater than zero.
    pub fn set_max_lines(self: RenderHandle<Self>, app: &mut App, value: Option<i32>) {
        debug_assert!(value.is_none_or(|value| value > 0));
        if self.painter(app).max_lines() == value {
            return;
        }
        self.painter_mut(app).set_max_lines(value);
        self.mark_needs_layout(app);
    }

    /// Defines how to measure the width of the rendered text.
    pub fn text_width_basis(self: RenderHandle<Self>, app: &App) -> TextWidthBasis {
        self.painter(app).text_width_basis()
    }

    /// Sets [`text_width_basis`](Self::text_width_basis).
    pub fn set_text_width_basis(self: RenderHandle<Self>, app: &mut App, value: TextWidthBasis) {
        if self.painter(app).text_width_basis() == value {
            return;
        }
        self.painter_mut(app).set_text_width_basis(value);
        self.mark_needs_layout(app);
    }

    /// Defines how to apply `TextStyle.height` over and under text.
    pub fn text_height_behavior(self: RenderHandle<Self>, app: &App) -> Option<TextHeightBehavior> {
        self.painter(app).text_height_behavior()
    }

    /// Sets [`text_height_behavior`](Self::text_height_behavior).
    pub fn set_text_height_behavior(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TextHeightBehavior>,
    ) {
        if self.painter(app).text_height_behavior() == value {
            return;
        }
        self.painter_mut(app).set_text_height_behavior(value);
        self.mark_needs_layout(app);
    }

    /// The height of a space in [`text`](Self::text) in logical pixels.
    pub fn preferred_line_height(self: RenderHandle<Self>, app: &mut App) -> f64 {
        let (painter, fonts) = self.painter_with_fonts(app);
        painter.preferred_line_height(fonts)
    }

    /// Flutter's `_textIntrinsics`: a second painter kept in sync with this paragraph's own,
    /// together with the collection it shapes against.
    ///
    /// Laying the paragraph's painter out for intrinsics would destroy the state its own layout
    /// left behind.
    fn text_intrinsics_with_fonts(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> (&mut TextPainter, &mut FontCollection) {
        let fonts = self.fonts(app);
        let (this, fonts) = app.get_disjoint_mut(self.handle(), fonts);
        let painter = &this.text_painter;
        let intrinsics = this.text_intrinsics.get_or_insert_with(TextPainter::new);
        intrinsics.set_text(painter.text().cloned());
        intrinsics.set_text_align(painter.text_align());
        intrinsics.set_text_direction(painter.text_direction());
        intrinsics.set_text_scaler(painter.text_scaler().clone());
        intrinsics.set_max_lines(painter.max_lines());
        intrinsics.set_ellipsis(painter.ellipsis().map(str::to_owned));
        intrinsics.set_text_width_basis(painter.text_width_basis());
        intrinsics.set_text_height_behavior(painter.text_height_behavior());
        (intrinsics, fonts)
    }

    /// Flutter's `_computeIntrinsicHeight`: the height the text takes at `width`.
    fn compute_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let max_width = self.adjust_max_width(app, width);
        let (intrinsics, fonts) = self.text_intrinsics_with_fonts(app);
        intrinsics.layout(fonts, width, max_width);
        intrinsics.height()
    }

    fn adjust_max_width(self: RenderHandle<Self>, app: &App, max_width: f64) -> f64 {
        if self.get(app).soft_wrap || self.get(app).overflow == TextOverflow::Ellipsis {
            max_width
        } else {
            f64::INFINITY
        }
    }

    fn layout_text_with_constraints(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) {
        let max_width = self.adjust_max_width(app, constraints.max_width);
        let (painter, fonts) = self.painter_with_fonts(app);
        painter.set_placeholder_dimensions(None);
        painter.layout(fonts, constraints.min_width, max_width);
    }

    /// Returns the offset at which to paint the caret.
    ///
    /// Valid only after layout.
    pub fn get_offset_for_caret(
        self: RenderHandle<Self>,
        app: &mut App,
        position: TextPosition,
        caret_prototype: Rect,
    ) -> Offset {
        debug_assert!(!self.debug_needs_layout(app));
        let constraints = self.constraints(app);
        self.layout_text_with_constraints(app, constraints);
        let (painter, fonts) = self.painter_with_fonts(app);
        painter.get_offset_for_caret(fonts, position, caret_prototype)
    }

    /// Returns the full height of the caret at the given position.
    ///
    /// Valid only after layout.
    pub fn get_full_height_for_caret(
        self: RenderHandle<Self>,
        app: &mut App,
        position: TextPosition,
    ) -> f64 {
        debug_assert!(!self.debug_needs_layout(app));
        let constraints = self.constraints(app);
        self.layout_text_with_constraints(app, constraints);
        let (painter, fonts) = self.painter_with_fonts(app);
        painter.get_full_height_for_caret(fonts, position, Rect::ZERO)
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
    /// Valid only after layout.
    pub fn get_boxes_for_selection(
        self: RenderHandle<Self>,
        app: &mut App,
        selection: TextSelection,
        box_height_style: BoxHeightStyle,
        box_width_style: BoxWidthStyle,
    ) -> Vec<TextBox> {
        debug_assert!(!self.debug_needs_layout(app));
        let constraints = self.constraints(app);
        self.layout_text_with_constraints(app, constraints);
        self.painter(app)
            .get_boxes_for_selection(selection, box_height_style, box_width_style)
    }

    /// Returns the position within the text for the given pixel offset.
    ///
    /// Valid only after layout.
    pub fn get_position_for_offset(
        self: RenderHandle<Self>,
        app: &mut App,
        offset: Offset,
    ) -> TextPosition {
        debug_assert!(!self.debug_needs_layout(app));
        let constraints = self.constraints(app);
        self.layout_text_with_constraints(app, constraints);
        self.painter(app).get_position_for_offset(offset)
    }

    /// Returns the text range of the word at the given offset. Characters not part of a word,
    /// such as spaces, symbols, and punctuation, have word breaks on both sides. In such
    /// cases, this method will return a text range that contains the given text position.
    ///
    /// Word boundaries are defined more precisely in Unicode Standard Annex #29
    /// <http://www.unicode.org/reports/tr29/#Word_Boundaries>.
    ///
    /// Valid only after layout.
    pub fn get_word_boundary(
        self: RenderHandle<Self>,
        app: &mut App,
        position: TextPosition,
    ) -> TextRange {
        debug_assert!(!self.debug_needs_layout(app));
        let constraints = self.constraints(app);
        self.layout_text_with_constraints(app, constraints);
        self.painter(app).get_word_boundary(position)
    }

    /// Returns the size of the text as laid out.
    ///
    /// This can differ from [`size`](RenderBox::size) if the text overflowed or if the
    /// [`constraints`](RenderBox::constraints) provided by the parent
    /// [`RenderObject`](crate::RenderObject) forced the layout to be bigger than necessary
    /// for the given [`text`](Self::text).
    ///
    /// This returns the [`TextPainter::size`] of the underlying [`TextPainter`].
    ///
    /// Valid only after layout.
    pub fn text_size(self: RenderHandle<Self>, app: &App) -> Size {
        debug_assert!(!self.debug_needs_layout(app));
        self.painter(app).size()
    }

    /// Whether the text was truncated or ellipsized as laid out.
    ///
    /// This returns the [`TextPainter::did_exceed_max_lines`] of the underlying
    /// [`TextPainter`].
    ///
    /// Valid only after layout.
    pub fn did_exceed_max_lines(self: RenderHandle<Self>, app: &App) -> bool {
        debug_assert!(!self.debug_needs_layout(app));
        self.painter(app).did_exceed_max_lines()
    }
}

impl RelayoutWhenSystemFontsChangeMixin for RenderParagraph {
    fn relayout_when_system_fonts_change_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RelayoutWhenSystemFontsChangeData {
        &self.get(app).system_fonts
    }

    fn relayout_when_system_fonts_change_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RelayoutWhenSystemFontsChangeData {
        &mut self.get_mut(app).system_fonts
    }

    fn system_fonts_did_change(self: RenderHandle<Self>, app: &mut App) {
        self.as_render_object(app).mark_needs_layout(app);
        self.painter_mut(app).mark_needs_layout();
    }
}

impl RenderObject for RenderParagraph {
    crate::render_object_accessors!();

    fn visit_children(
        self: RenderHandle<Self>,
        _app: &App,
        _visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        // A leaf until inline children (`WidgetSpan`) are ported.
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, _owner: Handle<crate::PipelineOwner>) {
        RelayoutWhenSystemFontsChangeMixin::attach_system_fonts(self, app);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        RelayoutWhenSystemFontsChangeMixin::detach_system_fonts(self, app);
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        self.layout_text_with_constraints(app, constraints);
        let text_size = self.painter(app).size();
        let size = constraints.constrain(text_size);
        self.set_size(app, size);

        let did_overflow_height =
            size.height() < text_size.height() || self.painter(app).did_exceed_max_lines();
        let did_overflow_width = size.width() < text_size.width();
        // TODO(abarth): We're only measuring the sizes of the line boxes here. If
        // the glyphs draw outside the line boxes, we might think that there isn't
        // visual overflow when there actually is visual overflow. This can become
        // a problem if we start having horizontal overflow and introduce a clip
        // that affects the actual (but undetected) vertical overflow.
        let has_visual_overflow = did_overflow_width || did_overflow_height;
        let needs_clipping = has_visual_overflow
            && match self.get(app).overflow {
                TextOverflow::Visible => false,
                // The fade shader waits on gradients; a fade clips meanwhile.
                TextOverflow::Clip | TextOverflow::Ellipsis | TextOverflow::Fade => true,
            };
        self.get_mut(app).needs_clipping = needs_clipping;
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        // Ideally we could compute the min/max intrinsic width/height with a
        // non-destructive operation. However, currently, computing these values
        // will destroy state inside the painter. If that happens, we need to get back
        // the correct state by calling _layout again.
        //
        // TODO(abarth): Make computing the min/max intrinsic width/height a
        //  non-destructive operation.
        //
        // If you remove this call, make sure that changing the textAlign still
        // works properly.
        let constraints = self.constraints(app);
        self.layout_text_with_constraints(app, constraints);

        let needs_clipping = self.get(app).needs_clipping;
        if needs_clipping {
            let bounds = offset & self.size(app);
            context.canvas().save();
            context.canvas().clip_rect(bounds, ClipOp::Intersect);
        }
        let (painter, fonts) = self.painter_with_fonts(app);
        painter.paint(fonts, context.canvas(), offset);
        if needs_clipping {
            context.canvas().restore();
        }
    }
}

impl RenderBox for RenderParagraph {
    crate::render_box_accessors!();

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, _height: f64) -> f64 {
        let (intrinsics, fonts) = self.text_intrinsics_with_fonts(app);
        intrinsics.layout(fonts, 0.0, f64::INFINITY);
        intrinsics.min_intrinsic_width()
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, _height: f64) -> f64 {
        let (intrinsics, fonts) = self.text_intrinsics_with_fonts(app);
        intrinsics.layout(fonts, 0.0, f64::INFINITY);
        intrinsics.max_intrinsic_width()
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.compute_intrinsic_height(app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.compute_intrinsic_height(app, width)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let max_width = self.adjust_max_width(app, constraints.max_width);
        let (intrinsics, fonts) = self.text_intrinsics_with_fonts(app);
        intrinsics.layout(fonts, constraints.min_width, max_width);
        let size = intrinsics.size();
        constraints.constrain(size)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        _baseline: TextBaseline,
    ) -> Option<f64> {
        debug_assert!(!self.as_object().debug_needs_layout(app));
        let constraints = self.constraints(app);
        debug_assert!(constraints.debug_assert_is_valid(false));
        self.layout_text_with_constraints(app, constraints);
        // Since the metric for the ideographic baseline is inaccurate and the non-alphabetic
        // baselines are based off of the alphabetic baseline, the alphabetic one is used for
        // now to produce correct layouts.
        Some(
            self.painter(app)
                .compute_distance_to_actual_baseline(TextBaseline::Alphabetic),
        )
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        _baseline: TextBaseline,
    ) -> Option<f64> {
        debug_assert!(constraints.debug_assert_is_valid(false));
        let max_width = self.adjust_max_width(app, constraints.max_width);
        let (intrinsics, fonts) = self.text_intrinsics_with_fonts(app);
        intrinsics.layout(fonts, constraints.min_width, max_width);
        Some(intrinsics.compute_distance_to_actual_baseline(TextBaseline::Alphabetic))
    }

    fn hit_test_self(self: RenderHandle<Self>, _app: &App, _position: Offset) -> bool {
        true
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        _app: &mut App,
        _result: &mut BoxHitTestResult<'_>,
        _position: Offset,
    ) -> bool {
        // Spans are not hit-test targets, and there are no inline children.
        false
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::Color;
    use reveal_foundation::AppCell;
    use reveal_gestures::HitTestResult;
    use reveal_painting::{TextSpan, TextStyle};

    use super::*;
    use crate::layer::{ContainerLayer, ErasedLayer, OffsetLayer};
    use crate::pipeline_owner::PipelineOwner;
    use crate::proxy_box::RenderRepaintBoundary;

    /// What the shell does at start-up: the app-wide fonts, here the OS fonts.
    fn install_fonts(app: &mut App) {
        let binding = PaintingBinding::instance(app);
        if !binding.has_fonts(app) {
            binding.install_fonts(app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
    }

    fn span(text: &str) -> InlineSpanRef {
        TextSpan::new()
            .text(text)
            .style(TextStyle::new().font_size(20.0))
            .into_span()
    }

    fn laid_out(
        app: &mut App,
        text: &str,
        constraints: BoxConstraints,
    ) -> (
        RenderHandle<RenderParagraph>,
        RenderHandle<RenderRepaintBoundary>,
    ) {
        install_fonts(app);
        let paragraph = RenderParagraph::new(app, span(text), TextDirection::Ltr, None);
        let root = RenderRepaintBoundary::new(app, Some(paragraph.as_box()));
        let owner = PipelineOwner::new(app, None);
        owner.set_root_node(app, Some(root.as_object()));
        root.schedule_initial_layout(app);
        root.layout(app, constraints, false);
        let paint_root = OffsetLayer::new(app, Offset::ZERO);
        paint_root.as_layer().attach(app, root.as_object().id());
        root.as_object()
            .schedule_initial_paint(app, paint_root.as_container_layer());
        (paragraph, root)
    }

    /// `paragraph_test.dart`: the minimum intrinsic width is the widest word, the maximum is the
    /// whole text on one line, and the intrinsic height at a width is what the text takes there.
    #[test]
    fn paragraph_intrinsics_come_from_the_text() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        let paragraph = RenderParagraph::new(
            &mut app,
            span("one two three four"),
            TextDirection::Ltr,
            None,
        );
        let box_ = paragraph.as_box();
        let min_width = box_.get_min_intrinsic_width(&mut app, f64::INFINITY);
        let max_width = box_.get_max_intrinsic_width(&mut app, f64::INFINITY);
        assert!(min_width > 0.0);
        assert!(
            min_width < max_width,
            "the widest word is narrower than the whole text"
        );

        let one_line = box_.get_min_intrinsic_height(&mut app, max_width);
        let wrapped = box_.get_min_intrinsic_height(&mut app, max_width / 2.0);
        assert!(wrapped > one_line, "half the width takes more lines");
        assert_eq!(box_.get_max_intrinsic_height(&mut app, max_width), one_line);
    }

    /// The dry layout of a paragraph is the size it lays out to, and its dry baseline is the
    /// baseline that layout reports.
    #[test]
    fn paragraph_dry_layout_and_baseline_match_its_layout() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let constraints = BoxConstraints::new().max_width(200.0).max_height(1000.0);
        install_fonts(&mut app);
        let paragraph =
            RenderParagraph::new(&mut app, span("one two three"), TextDirection::Ltr, None);
        let dry = paragraph.as_box().get_dry_layout(&mut app, constraints);
        let dry_baseline =
            paragraph
                .as_box()
                .get_dry_baseline(&mut app, constraints, TextBaseline::Alphabetic);

        let (paragraph, _root) = laid_out(&mut app, "one two three", constraints);
        assert_eq!(dry, paragraph.size(&app));
        let baseline =
            paragraph
                .as_box()
                .get_distance_to_baseline(&mut app, TextBaseline::Alphabetic, true);
        assert_eq!(dry_baseline, baseline);
        assert!(baseline.is_some_and(|baseline| baseline > 0.0));
    }

    #[test]
    fn a_paragraph_sizes_itself_to_its_text_within_loose_constraints() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let loose = BoxConstraints::new().max_width(1000.0).max_height(1000.0);
        let (paragraph, _root) = laid_out(&mut app, "Hello", loose);
        let size = paragraph.size(&app);
        assert!(size.width() > 0.0 && size.width() < 1000.0);
        assert_eq!(size, paragraph.text_size(&app));
        assert!(!paragraph.did_exceed_max_lines(&app));
    }

    #[test]
    fn narrow_constraints_wrap_the_text() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let loose = BoxConstraints::new().max_width(1000.0).max_height(1000.0);
        let (wide, _root) = laid_out(&mut app, "one two three four", loose);
        let natural = wide.size(&app);
        let narrow = BoxConstraints::new()
            .max_width(natural.width() / 2.0)
            .max_height(1000.0);
        let (wrapped, _root) = laid_out(&mut app, "one two three four", narrow);
        assert!(wrapped.size(&app).height() > natural.height());
        assert!(wrapped.size(&app).width() <= natural.width() / 2.0);
    }

    #[test]
    fn soft_wrap_off_lets_the_text_overflow_and_clips_it() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let loose = BoxConstraints::new().max_width(1000.0).max_height(1000.0);
        let (wide, _root) = laid_out(&mut app, "one two three four", loose);
        let natural = wide.size(&app);
        install_fonts(&mut app);
        let paragraph = RenderParagraph::new(
            &mut app,
            span("one two three four"),
            TextDirection::Ltr,
            None,
        );
        paragraph.set_soft_wrap(&mut app, false);
        let root = RenderRepaintBoundary::new(&mut app, Some(paragraph.as_box()));
        let owner = PipelineOwner::new(&mut app, None);
        owner.set_root_node(&mut app, Some(root.as_object()));
        root.schedule_initial_layout(&mut app);
        let narrow = BoxConstraints::new()
            .max_width(natural.width() / 2.0)
            .max_height(1000.0);
        root.layout(&mut app, narrow, false);
        assert_eq!(paragraph.size(&app).width(), natural.width() / 2.0);
        assert_eq!(
            paragraph.text_size(&app).width(),
            natural.width(),
            "laid out unwrapped"
        );
        assert!(paragraph.get(&app).needs_clipping);

        paragraph.set_overflow(&mut app, TextOverflow::Visible);
        owner.flush_layout(&mut app);
        assert!(!paragraph.get(&app).needs_clipping);
    }

    #[test]
    fn ellipsis_overflow_reports_exceeded_lines() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let loose = BoxConstraints::new().max_width(1000.0).max_height(1000.0);
        let (wide, _root) = laid_out(&mut app, "one two three four", loose);
        let natural = wide.size(&app);
        install_fonts(&mut app);
        let paragraph = RenderParagraph::new(
            &mut app,
            span("one two three four"),
            TextDirection::Ltr,
            None,
        );
        paragraph.set_max_lines(&mut app, Some(1));
        paragraph.set_overflow(&mut app, TextOverflow::Ellipsis);
        let root = RenderRepaintBoundary::new(&mut app, Some(paragraph.as_box()));
        let owner = PipelineOwner::new(&mut app, None);
        owner.set_root_node(&mut app, Some(root.as_object()));
        root.schedule_initial_layout(&mut app);
        root.layout(
            &mut app,
            BoxConstraints::new()
                .max_width(natural.width() / 2.0)
                .max_height(1000.0),
            false,
        );
        assert!(paragraph.did_exceed_max_lines(&app));
        assert_eq!(paragraph.size(&app).height(), natural.height());
    }

    #[test]
    fn setting_text_marks_layout_or_paint_by_the_comparison() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let loose = BoxConstraints::new().max_width(1000.0).max_height(1000.0);
        let (paragraph, root) = laid_out(&mut app, "Hello", loose);
        let owner = root.owner(&app).unwrap();
        owner.flush_compositing_bits(&mut app);
        owner.flush_paint(&mut app);
        assert!(!paragraph.debug_needs_layout(&app));

        let recolored = TextSpan::new()
            .text("Hello")
            .style(
                TextStyle::new()
                    .font_size(20.0)
                    .color(Color::from_argb(255, 255, 0, 0)),
            )
            .into_span();
        paragraph.set_text(&mut app, recolored);
        assert!(
            !paragraph.debug_needs_layout(&app),
            "a color change is paint only"
        );
        assert!(paragraph.as_object().debug_needs_paint(&app));

        paragraph.set_text(&mut app, span("Hello there"));
        assert!(paragraph.debug_needs_layout(&app));
    }

    #[test]
    fn painting_records_glyphs_and_hit_testing_lands_on_the_paragraph() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let loose = BoxConstraints::new().max_width(1000.0).max_height(1000.0);
        let (paragraph, root) = laid_out(&mut app, "Hello", loose);
        let owner = root.owner(&app).unwrap();
        owner.flush_compositing_bits(&mut app);
        owner.flush_paint(&mut app);
        assert!(!paragraph.as_object().debug_needs_paint(&app));

        let mut result = HitTestResult::new();
        let hit = paragraph.hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            Offset::new(2.0, 2.0),
        );
        assert!(hit);
        assert_eq!(result.path().len(), 1);
        let position = paragraph.get_position_for_offset(&mut app, Offset::new(0.0, 1.0));
        assert_eq!(position.offset, 0);
        assert_eq!(
            paragraph.get_word_boundary(&mut app, TextPosition::new(1)),
            TextRange::new(0, 5)
        );
    }
}
