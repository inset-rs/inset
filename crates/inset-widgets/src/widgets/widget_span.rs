//! Flutter counterpart: `widgets/widget_span.dart`.
//!
//! The `Semantics` wrapper that tags each inline widget with its placeholder index waits with
//! the rest of semantics.

use std::any::Any;
use std::fmt;
use std::rc::Rc;

use inset_embedder::{
    Matrix4, Offset, ParagraphBuilder, PlaceholderAlignment, Size, TextBaseline, TextPosition,
};
use inset_foundation::App;
use inset_painting::{
    Accumulator, InlineSpan, InlineSpanRef, K_DEFAULT_FONT_SIZE, PLACEHOLDER_CODE_UNIT,
    PlaceholderDimensions, PlaceholderSpan, RenderComparison, TextScaler, TextStyle, same_span,
};
use inset_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, ContainerLayer,
    PaintingContext, RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, TextParentData, TransformLayer,
};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, ParentDataWidget, RenderObjectWidget,
    SingleChildRenderObjectWidget, WidgetRef,
};

/// An immutable widget that is embedded inline within text.
///
/// The [`child`](Self::child) property is the widget that will be embedded. Children are
/// constrained by the width of the paragraph.
///
/// The [`child`](Self::child) property may contain its own `Widget` children (if applicable),
/// including `Text` and `RichText` widgets which may include additional [`WidgetSpan`]s.
/// Child `Text` and `RichText` widgets will be laid out independently and occupy a rectangular
/// space in the parent text layout.
///
/// [`WidgetSpan`]s will be ignored when passed into a [`TextPainter`](inset_painting::TextPainter)
/// directly. To properly layout and paint the [`WidgetSpan`], [`WidgetSpan`] should be passed
/// into a `Text::rich` widget.
///
/// ```text
/// Text::rich(
///     TextSpan::new()
///         .children(vec![
///             TextSpan::new().text("Flutter is").into_span(),
///             WidgetSpan::new(SizedBox::new().width(120.0).height(50.0).child(Card::new(...)))
///                 .into_span(),
///             TextSpan::new().text("the best!").into_span(),
///         ])
///         .into_span(),
/// )
/// ```
///
/// [`WidgetSpan`] contributes the semantics of the [`WidgetSpan::child`] to the semantics
/// tree.
///
/// See also:
///
///  * `TextSpan`, a node that represents text in an [`InlineSpan`] tree.
///  * `Text`, a widget for showing uniformly-styled text.
///  * `RichText`, a widget for finer control of text rendering.
///  * [`TextPainter`](inset_painting::TextPainter), a class for painting [`InlineSpan`]
///    objects on a `Canvas`.
///
/// Dart's named constructor arguments are the fluent setters; the constructor's assert runs
/// in [`into_span`](Self::into_span), once every setter has had its say.
#[derive(Clone)]
pub struct WidgetSpan {
    /// The widget to embed inline within text.
    pub child: WidgetRef,
    /// How the placeholder aligns vertically with the text.
    pub alignment: PlaceholderAlignment,
    /// The [`TextBaseline`] to align against when using [`PlaceholderAlignment::Baseline`],
    /// [`PlaceholderAlignment::AboveBaseline`], and [`PlaceholderAlignment::BelowBaseline`].
    pub baseline: Option<TextBaseline>,
    /// The [`TextStyle`] to apply to this span; only the decoration, foreground, background,
    /// and spacing options are used.
    pub style: Option<TextStyle>,
}

impl WidgetSpan {
    /// Creates a [`WidgetSpan`] with the given values.
    ///
    /// The [`child`](Self::child) property is the widget that will be embedded. Children are
    /// constrained by the width of the paragraph.
    ///
    /// The [`child`](Self::child) property may contain its own `Widget` children (if
    /// applicable), including `Text` and `RichText` widgets which may include additional
    /// [`WidgetSpan`]s. Child `Text` and `RichText` widgets will be laid out independently and
    /// occupy a rectangular space in the parent text layout.
    ///
    /// A [`TextStyle`] may be provided with the [`style`](Self::style) property, but only the
    /// decoration, foreground, background, and spacing options will be used.
    pub fn new<K>(child: impl IntoWidget<K>) -> WidgetSpan {
        WidgetSpan {
            child: child.into_widget(),
            alignment: PlaceholderAlignment::Bottom,
            baseline: None,
            style: None,
        }
    }

    /// Dart `WidgetSpan(alignment:)`.
    pub fn alignment(mut self, alignment: PlaceholderAlignment) -> WidgetSpan {
        self.alignment = alignment;
        self
    }

    /// Dart `WidgetSpan(baseline:)`.
    pub fn baseline(mut self, baseline: TextBaseline) -> WidgetSpan {
        self.baseline = Some(baseline);
        self
    }

    /// Dart `WidgetSpan(style:)`.
    pub fn style(mut self, style: TextStyle) -> WidgetSpan {
        self.style = Some(style);
        self
    }

    /// The span as a shared [`InlineSpanRef`].
    ///
    /// A baseline alignment needs a [`baseline`](Self::baseline).
    pub fn into_span(self) -> InlineSpanRef {
        debug_assert!(
            self.baseline.is_some()
                || !matches!(
                    self.alignment,
                    PlaceholderAlignment::AboveBaseline
                        | PlaceholderAlignment::BelowBaseline
                        | PlaceholderAlignment::Baseline
                )
        );
        std::rc::Rc::new(self)
    }

    /// Helper function for extracting [`WidgetSpan`]s in preorder, from the given
    /// [`InlineSpan`] as a list of widgets.
    ///
    /// The `text_scaler` is the scaling strategy for scaling the content.
    ///
    /// This function is used by `EditableText` and `RichText` so calling it directly is rarely
    /// necessary.
    pub fn extract_from_inline_span(
        span: &dyn InlineSpan,
        text_scaler: &TextScaler,
    ) -> Vec<WidgetRef> {
        let mut widgets = Vec::new();
        // _kEngineDefaultFontSize is the default font size to use when none of the
        // ancestor spans specifies one.
        let mut font_size_stack = vec![K_DEFAULT_FONT_SIZE];
        visit_subtree(span, text_scaler, &mut font_size_stack, &mut widgets);
        widgets
    }
}

/// `extract_from_inline_span`'s `visitSubtree`. This assumes an [`InlineSpan`] tree's logical
/// order is equivalent to preorder.
fn visit_subtree(
    span: &dyn InlineSpan,
    text_scaler: &TextScaler,
    font_size_stack: &mut Vec<f64>,
    widgets: &mut Vec<WidgetRef>,
) -> bool {
    let font_size_to_push = span
        .style()
        .and_then(|style| style.font_size)
        .filter(|size| Some(size) != font_size_stack.last());
    if let Some(size) = font_size_to_push {
        font_size_stack.push(size);
    }
    if let Some(widget_span) = span.as_any().downcast_ref::<WidgetSpan>() {
        let font_size = *font_size_stack.last().expect("never empty");
        let text_scale_factor = if font_size == 0.0 {
            0.0
        } else {
            text_scaler.scale(font_size) / font_size
        };
        widgets.push(
            WidgetSpanParentData {
                span: widget_span.clone(),
                child: AutoScaleInlineWidget {
                    span: widget_span.clone(),
                    text_scale_factor,
                    child: widget_span.child.clone(),
                }
                .into_widget(),
            }
            .into_widget(),
        );
    }
    debug_assert!(
        span.as_any().is::<WidgetSpan>() || span.as_placeholder_span().is_none(),
        "{span:?} is a PlaceholderSpan but not a WidgetSpan subclass. This is currently not \
         supported."
    );
    span.visit_direct_children(&mut |child| {
        visit_subtree(&**child, text_scaler, font_size_stack, widgets)
    });
    if let Some(pushed) = font_size_to_push {
        let popped_font_size = font_size_stack.pop();
        debug_assert!(!font_size_stack.is_empty());
        debug_assert_eq!(popped_font_size, Some(pushed));
    }
    true
}

impl fmt::Debug for WidgetSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("WidgetSpan");
        if let Some(style) = &self.style {
            s.field("style", style);
        }
        s.field("alignment", &self.alignment)
            .field("baseline", &self.baseline)
            .field("widget", &self.child)
            .finish()
    }
}

impl PlaceholderSpan for WidgetSpan {
    fn alignment(&self) -> PlaceholderAlignment {
        self.alignment
    }

    fn baseline(&self) -> Option<TextBaseline> {
        self.baseline
    }
}

impl InlineSpan for WidgetSpan {
    fn style(&self) -> Option<&TextStyle> {
        self.style.as_ref()
    }

    /// Adds a placeholder box to the paragraph builder if a size has been calculated for the
    /// widget.
    ///
    /// Sizes are provided through `dimensions`, which should contain a 1:1 in-order mapping
    /// of widget to laid-out dimensions. If no such dimension is provided, the widget will be
    /// skipped.
    ///
    /// The `text_scaler` will be applied to the laid-out size of the widget.
    fn build(
        &self,
        builder: &mut ParagraphBuilder,
        text_scaler: &TextScaler,
        dimensions: Option<&[PlaceholderDimensions]>,
    ) {
        debug_assert!(InlineSpan::debug_assert_is_valid(self));
        let dimensions = dimensions.expect("a WidgetSpan is built with its dimensions");
        let has_style = self.style.is_some();
        if let Some(style) = &self.style {
            builder.push_style(style.get_text_style_with(text_scaler));
        }
        debug_assert!((builder.placeholder_count() as usize) < dimensions.len());
        let current_dimensions = dimensions[builder.placeholder_count() as usize];
        builder.add_placeholder(
            current_dimensions.size.width(),
            current_dimensions.size.height(),
            self.alignment,
            1.0,
            current_dimensions.baseline_offset,
            current_dimensions.baseline,
        );
        if has_style {
            builder.pop();
        }
    }

    /// Calls `visitor` on this [`WidgetSpan`]. There are no children spans to walk.
    fn visit_children(self: Rc<Self>, visitor: &mut dyn FnMut(&InlineSpanRef) -> bool) -> bool {
        visitor(&(self as InlineSpanRef))
    }

    fn visit_direct_children(&self, _visitor: &mut dyn FnMut(&InlineSpanRef) -> bool) -> bool {
        true
    }

    /// [`WidgetSpan`]s are not hit-test targets: this always returns `None`.
    fn get_span_for_position(self: Rc<Self>, _position: TextPosition) -> Option<InlineSpanRef> {
        debug_assert!(InlineSpan::debug_assert_is_valid(&*self));
        None
    }

    fn get_span_for_position_visitor(
        self: Rc<Self>,
        position: TextPosition,
        offset: &mut Accumulator,
    ) -> Option<InlineSpanRef> {
        if position.offset == offset.value() {
            return Some(self);
        }
        offset.increment(1);
        None
    }

    fn compute_to_plain_text(
        &self,
        buffer: &mut String,
        include_semantics_labels: bool,
        include_placeholders: bool,
    ) {
        PlaceholderSpan::compute_to_plain_text(
            self,
            buffer,
            include_semantics_labels,
            include_placeholders,
        );
    }

    fn code_unit_at_visitor(&self, index: i32, offset: &mut Accumulator) -> Option<u16> {
        let local_offset = index - offset.value();
        debug_assert!(local_offset >= 0);
        offset.increment(1);
        (local_offset == 0).then_some(PLACEHOLDER_CODE_UNIT)
    }

    /// [`WidgetSpan`]s are always valid as asserts prevent invalid [`WidgetSpan`]s from being
    /// constructed.
    fn debug_assert_is_valid(&self) -> bool {
        true
    }

    fn compare_to(&self, other: &dyn InlineSpan) -> RenderComparison {
        if same_span(self, other) {
            return RenderComparison::Identical;
        }
        let Some(typed_other) = other.as_any().downcast_ref::<WidgetSpan>() else {
            return RenderComparison::Layout;
        };
        if self.style.is_none() != typed_other.style.is_none() {
            return RenderComparison::Layout;
        }
        if !std::rc::Rc::ptr_eq(&self.child, &typed_other.child)
            || self.alignment != typed_other.alignment
        {
            return RenderComparison::Layout;
        }
        let mut result = RenderComparison::Identical;
        if let (Some(style), Some(other_style)) = (&self.style, &typed_other.style) {
            let candidate = style.compare_to(other_style);
            if candidate > result {
                result = candidate;
            }
            if result == RenderComparison::Layout {
                return result;
            }
        }
        result
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_placeholder_span(&self) -> Option<&dyn PlaceholderSpan> {
        Some(self)
    }

    fn eq_span(&self, other: &dyn InlineSpan) -> bool {
        if same_span(self, other) {
            return true;
        }
        let Some(other) = other.as_any().downcast_ref::<WidgetSpan>() else {
            return false;
        };
        other.style == self.style
            && std::rc::Rc::ptr_eq(&other.child, &self.child)
            && other.alignment == self.alignment
            && other.baseline == self.baseline
    }
}

/// A `ParentDataWidget` that sets [`TextParentData::span`].
#[derive(Debug)]
struct WidgetSpanParentData {
    span: WidgetSpan,
    child: WidgetRef,
}

impl ParentDataWidget for WidgetSpanParentData {
    type ParentData = TextParentData;

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn apply_parent_data(&self, app: &mut App, render_object: AnyRenderObject) {
        let parent_data = render_object.parent_data_of_mut::<TextParentData>(app);
        parent_data.span = Some(std::rc::Rc::new(self.span.clone()));
    }
}

// A RenderObjectWidget that automatically applies text scaling on inline
// widgets.
//
// TODO(LongCatIsLooong): this shouldn't happen automatically, at least there
// should be a way to opt out: https://github.com/flutter/flutter/issues/126962
#[derive(Debug)]
struct AutoScaleInlineWidget {
    span: WidgetSpan,
    text_scale_factor: f64,
    child: WidgetRef,
}

impl RenderObjectWidget for AutoScaleInlineWidget {
    type RenderObject = RenderScaledInlineWidget;

    fn key(&self) -> Option<&KeyRef> {
        None
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderScaledInlineWidget::new(
            app,
            self.span.alignment,
            self.span.baseline,
            self.text_scale_factor,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderScaledInlineWidget>,
    ) {
        render_object.set_alignment(app, self.span.alignment);
        render_object.set_baseline(app, self.span.baseline);
        render_object.set_scale(app, self.text_scale_factor);
    }
}

impl SingleChildRenderObjectWidget for AutoScaleInlineWidget {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

/// The render box of an inline widget: its child laid out at the paragraph's width and
/// scaled by the text scale factor.
struct RenderScaledInlineWidget {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    scale: f64,
    alignment: PlaceholderAlignment,
    baseline: Option<TextBaseline>,
}

impl RenderScaledInlineWidget {
    fn new(
        app: &mut App,
        alignment: PlaceholderAlignment,
        baseline: Option<TextBaseline>,
        scale: f64,
    ) -> RenderHandle<Self> {
        RenderHandle::new_box(
            app,
            RenderScaledInlineWidget {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                scale,
                alignment,
                baseline,
            },
        )
    }

    fn scale(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).scale
    }

    fn set_scale(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if value == self.get(app).scale {
            return;
        }
        debug_assert!(value > 0.0);
        debug_assert!(value.is_finite());
        self.get_mut(app).scale = value;
        self.as_object().mark_needs_layout(app);
    }

    fn set_alignment(self: RenderHandle<Self>, app: &mut App, value: PlaceholderAlignment) {
        if self.get(app).alignment == value {
            return;
        }
        self.get_mut(app).alignment = value;
        self.as_object().mark_needs_layout(app);
    }

    fn set_baseline(self: RenderHandle<Self>, app: &mut App, value: Option<TextBaseline>) {
        if value == self.get(app).baseline {
            return;
        }
        self.get_mut(app).baseline = value;
        self.as_object().mark_needs_layout(app);
    }

    /// `Matrix4.diagonal3Values(scale, scale, 1.0)`.
    fn scale_matrix(self: RenderHandle<Self>, app: &App) -> Matrix4 {
        let scale = self.scale(app) as f32;
        Matrix4::scale(scale, scale)
    }
}

impl RenderObjectWithChildMixin for RenderScaledInlineWidget {
    type ChildType = AnyRenderBox;

    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderObject for RenderScaledInlineWidget {
    inset_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let Some(child) = self.child(app) else {
            return;
        };
        let constraints = self.constraints(app);
        debug_assert!(!constraints.has_bounded_height());
        let scale = self.scale(app);
        // Only constrain the width to the maximum width of the paragraph.
        // Leave height unconstrained, which will overflow if expanded past.
        child.layout(
            app,
            BoxConstraints::new().max_width(constraints.max_width / scale),
            true,
        );
        let size = constraints.constrain(child.size(app) * scale);
        self.set_size(app, size);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let Some(child) = self.child(app) else {
            self.as_object().set_layer(app, None);
            return;
        };
        if self.scale(app) == 1.0 {
            context.paint_child(app, child.as_object(), offset);
            self.as_object().set_layer(app, None);
            return;
        }
        let old_layer = self.as_object().layer_as::<TransformLayer>(app);
        let layer = context.push_transform(
            app,
            self.as_object().needs_compositing(app),
            offset,
            self.scale_matrix(app),
            |app, context, offset| context.paint_child(app, child.as_object(), offset),
            old_layer,
        );
        self.as_object()
            .set_layer(app, layer.map(|layer| layer.as_container_layer()));
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }
}

impl RenderBox for RenderScaledInlineWidget {
    inset_rendering::render_box_accessors!();

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let scale = self.scale(app);
        self.child(app).map_or(0.0, |child| {
            child.get_max_intrinsic_height(app, width / scale)
        }) * scale
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let scale = self.scale(app);
        self.child(app).map_or(0.0, |child| {
            child.get_max_intrinsic_width(app, height / scale)
        }) * scale
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let scale = self.scale(app);
        self.child(app).map_or(0.0, |child| {
            child.get_min_intrinsic_height(app, width / scale)
        }) * scale
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let scale = self.scale(app);
        self.child(app).map_or(0.0, |child| {
            child.get_min_intrinsic_width(app, height / scale)
        }) * scale
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let scale = self.scale(app);
        self.child(app)
            .and_then(|child| child.get_distance_to_actual_baseline(app, baseline))
            .map(|child_baseline| scale * child_baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let scale = self.scale(app);
        let distance = self.child(app)?.get_dry_baseline(
            app,
            BoxConstraints::new().max_width(constraints.max_width / scale),
            baseline,
        );
        distance.map(|distance| scale * distance)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        debug_assert!(!constraints.has_bounded_height());
        let scale = self.scale(app);
        let unscaled_size = self.child(app).map_or(Size::ZERO, |child| {
            child.get_dry_layout(
                app,
                BoxConstraints::new().max_width(constraints.max_width / scale),
            )
        });
        constraints.constrain(unscaled_size * scale)
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        _child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        *transform = transform.then(&self.scale_matrix(app));
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let Some(child) = self.child(app) else {
            return false;
        };
        let transform = self.scale_matrix(app);
        result.add_with_paint_transform(Some(transform), position, |result, transformed| {
            child.hit_test(app, result, transformed)
        })
    }
}

#[cfg(test)]
mod tests {
    use inset_embedder::Color;
    use inset_embedder::{TextDirection, TextLeadingDistribution};
    use inset_foundation::AppCell;
    use inset_gestures::HitTestResult;
    use inset_painting::{PaintingBinding, TextOverflow, TextSpan};
    use inset_rendering::{RenderColoredBox, RenderIntrinsicHeight, RenderParagraph};

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{
        Center, ColoredBox, Directionality, IntrinsicHeight, Row, SizedBox,
    };
    use crate::widgets::text::{RichText, Text};

    /// What the shell does at start-up: the app-wide fonts, here the OS fonts.
    fn install_fonts(app: &mut App) {
        let binding = PaintingBinding::instance(app);
        if !binding.has_fonts(app) {
            binding.install_fonts(app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
    }

    /// Mounts `child` under a left-to-right `Directionality` with fonts installed, and pumps
    /// the first frame.
    fn mount(app: &mut App, child: WidgetRef) -> Harness {
        install_fonts(app);
        let harness = Harness::mount(
            app,
            Directionality::new(TextDirection::Ltr, child).into_widget(),
        );
        harness.pump(app);
        harness
    }

    /// The only child of `parent` in the render tree.
    fn only_child(app: &App, parent: AnyRenderObject) -> AnyRenderObject {
        let mut children = Vec::new();
        parent.visit_children(app, &mut |child| children.push(child));
        assert_eq!(children.len(), 1, "one child");
        children[0]
    }

    fn paragraph_under(app: &App, harness: &Harness) -> RenderHandle<RenderParagraph> {
        only_child(app, harness.render_root(app).as_object())
            .downcast::<RenderParagraph>(app)
            .expect("a RenderParagraph")
    }

    fn sized_box(width: f64, height: f64) -> WidgetRef {
        SizedBox::new().width(width).height(height).into_widget()
    }

    fn text(text: &str) -> InlineSpanRef {
        TextSpan::new().text(text).into_span()
    }

    #[test]
    fn a_widget_span_lays_its_child_out_between_the_text() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let span = TextSpan::new()
            .children(vec![
                text("ab"),
                WidgetSpan::new(sized_box(30.0, 20.0)).into_span(),
                text("cd"),
            ])
            .into_span();
        let harness = mount(
            &mut app,
            Center::new().child(Text::rich(span)).into_widget(),
        );
        let paragraph = only_child(&app, harness.render_root(&app).as_object())
            .downcast::<inset_rendering::RenderPositionedBox>(&app)
            .map(|center| only_child(&app, center.as_object()))
            .and_then(|child| child.downcast::<RenderParagraph>(&app))
            .expect("a RenderParagraph under the Center");
        assert_eq!(
            paragraph.text(&app).to_plain_text(true, true),
            "ab\u{FFFC}cd"
        );
        let inline = only_child(&app, paragraph.as_object());
        let parent_data = inline.parent_data_of::<TextParentData>(&app);
        assert!(
            parent_data.span.is_some(),
            "the parent data widget set the span"
        );
        let offset = parent_data.offset().expect("laid out");
        assert!(offset.dx() > 0.0, "after \"ab\"");
        let inline = inline.as_box().expect("a box");
        assert_eq!(inline.size(&app), Size::new(30.0, 20.0));
        let width = paragraph.size(&app).width();
        assert!(width > offset.dx() + 30.0, "\"cd\" follows the widget");
        assert!(paragraph.size(&app).height() >= 20.0);
    }

    // Regression test for https://github.com/flutter/flutter/issues/35869
    #[test]
    fn inline_widgets_works_with_ellipsis() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let truncated =
            RichText::new(text("widget should be truncated")).text_direction(TextDirection::Rtl);
        let span = TextSpan::new()
            .children(vec![
                text("a very very very very very very very very very very long line"),
                WidgetSpan::new(SizedBox::new().width(20.0).height(40.0).child(truncated))
                    .into_span(),
            ])
            .into_span();
        let harness = mount(
            &mut app,
            Text::rich(span)
                .max_lines(1)
                .overflow(TextOverflow::Ellipsis)
                .into_widget(),
        );
        let paragraph = paragraph_under(&app, &harness);
        let inline = only_child(&app, paragraph.as_object());
        assert_eq!(
            inline.parent_data_of::<TextParentData>(&app).offset(),
            None,
            "ellipsized away"
        );
    }

    #[test]
    fn a_hit_on_an_inline_widget_reaches_it_and_a_hit_on_the_text_lands_on_its_span() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let opaque = ColoredBox::new(Color::from_argb(255, 0, 0, 255)).child(sized_box(30.0, 20.0));
        let span = TextSpan::new()
            .children(vec![text("ab"), WidgetSpan::new(opaque).into_span()])
            .into_span();
        let harness = mount(
            &mut app,
            Center::new().child(Text::rich(span)).into_widget(),
        );
        let center = only_child(&app, harness.render_root(&app).as_object());
        let paragraph = only_child(&app, center)
            .downcast::<RenderParagraph>(&app)
            .expect("a RenderParagraph");
        let inline = only_child(&app, paragraph.as_object());
        let offset = inline
            .parent_data_of::<TextParentData>(&app)
            .offset()
            .expect("laid out");

        let mut result = HitTestResult::new();
        let hit = paragraph.as_box().hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            offset + Offset::new(15.0, 10.0),
        );
        assert!(hit);
        assert!(
            only_child(&app, inline)
                .downcast::<RenderColoredBox>(&app)
                .is_some()
        );
        assert_eq!(
            result.path().len(),
            3,
            "the colored box, the scaled inline box and the paragraph"
        );

        let mut result = HitTestResult::new();
        paragraph.as_box().hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            Offset::new(1.0, offset.dy() + 10.0),
        );
        assert_eq!(result.path().len(), 2, "the span and the paragraph");
        let target: &dyn std::any::Any = result.path()[0].target();
        assert_eq!(
            target
                .downcast_ref::<TextSpan>()
                .and_then(|span| span.text.as_deref()),
            Some("ab")
        );
    }

    // Regression test for https://github.com/flutter/flutter/issues/59316
    #[test]
    fn inline_widgets_works_with_text_scaler() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let rich = |scale: f64| {
            let span = TextSpan::new()
                .children(vec![WidgetSpan::new(sized_box(30.0, 20.0)).into_span()])
                .into_span();
            Center::new()
                .child(Text::rich(span).text_scaler(TextScaler::linear(scale)))
                .into_widget()
        };
        let harness = mount(&mut app, rich(1.0));
        let inline_size = |app: &App| {
            let center = only_child(app, harness.render_root(app).as_object());
            let paragraph = only_child(app, center);
            only_child(app, paragraph)
                .as_box()
                .expect("a box")
                .size(app)
        };
        assert_eq!(inline_size(&app), Size::new(30.0, 20.0));

        harness.set_child(
            &mut app,
            Directionality::new(TextDirection::Ltr, rich(2.0)).into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(inline_size(&app), Size::new(60.0, 40.0));
    }

    #[test]
    fn widget_span_calculate_correct_intrinsic_heights() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let line = TextStyle::new()
            .height(1.0)
            .font_size(16.0)
            .leading_distribution(TextLeadingDistribution::Even);
        let span = TextSpan::new()
            .children(vec![
                TextSpan::new()
                    .text("Start\n")
                    .style(line.clone())
                    .into_span(),
                WidgetSpan::new(Row::new().children([sized_box(16.0, 16.0)])).into_span(),
                TextSpan::new().text("End").style(line).into_span(),
            ])
            .into_span();
        let harness = mount(
            &mut app,
            Center::new()
                .child(IntrinsicHeight::new().child(RichText::new(span)))
                .into_widget(),
        );
        let center = only_child(&app, harness.render_root(&app).as_object());
        let intrinsic = only_child(&app, center)
            .downcast::<RenderIntrinsicHeight>(&app)
            .expect("a RenderIntrinsicHeight");
        assert_eq!(intrinsic.size(&app).height(), 3.0 * 16.0);
    }
}
