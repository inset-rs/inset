//! Flutter counterpart: `rendering/paragraph.dart` (`RenderParagraph`, `_SelectableFragment`).
//!
//! A paragraph with a registrar splits its text at its placeholders into selectable fragments
//! and registers each; a fragment is removed from the registrar before it is disposed, since the
//! registrar does not keep it. The fade overflow shader and semantics wait; see `PORTING.md`.

use std::any::{Any, TypeId};
use std::fmt;
use std::rc::Rc;

use waft_embedder::{
    BoxHeightStyle, BoxWidthStyle, ClipOp, Color, FontCollection, Matrix4, Offset, Paint,
    PlaceholderAlignment, Rect, Size, TextAffinity, TextAlign, TextBaseline, TextBox,
    TextDirection, TextHeightBehavior, TextPosition, TextRange, TextSelection,
};
use waft_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, Ref};
use waft_painting::{
    ClipContext, InlineSpanRef, PaintingBinding, PlaceholderDimensions, PlaceholderSpan,
    RenderComparison, TextOverflow, TextPainter, TextScaler, TextWidthBasis,
};

use crate::box_::{AnyRenderBox, BoxConstraints, BoxHitTestResult, RenderBox, RenderBoxData};
use crate::layer::{ContainerLayer, LayerLink, LeaderLayer};
use crate::layout_helper::{ChildBaselineGetter, ChildLayoutHelper, ChildLayouter};
use crate::object::{
    AnyRenderObject, Constraints, ContainerParentData, ContainerParentDataMixin,
    ContainerRenderObjectData, ContainerRenderObjectMixin, ParentData, RenderHandle, RenderObject,
    RenderObjectData,
};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;
use crate::selection::{
    AnySelectable, AnySelectionRegistrar, Selectable, SelectedContent, SelectedContentRange,
    SelectionEvent, SelectionExtendDirection, SelectionGeometry, SelectionHandler, SelectionPoint,
    SelectionRegistrarRef, SelectionResult, SelectionStatus, SelectionUtils, TextGranularity,
    TextSelectionHandleType,
};
use crate::text_boundary::{LineBoundary, WordBoundary};
use crate::{RelayoutWhenSystemFontsChangeData, RelayoutWhenSystemFontsChangeMixin};
use waft_services::{CharacterBoundary, DocumentBoundary, ParagraphBoundary, TextBoundary};

/// The start and end positions for a text boundary.
#[derive(Clone, Copy, Debug)]
struct TextBoundaryRecord {
    boundary_start: TextPosition,
    boundary_end: TextPosition,
}

impl TextBoundaryRecord {
    fn new(boundary_start: TextPosition, boundary_end: TextPosition) -> TextBoundaryRecord {
        TextBoundaryRecord {
            boundary_start,
            boundary_end,
        }
    }
}

/// Signature for a function that determines the [`TextBoundaryRecord`] at the given
/// [`TextPosition`].
type TextBoundaryAtPosition =
    fn(Handle<SelectableFragment>, &mut App, TextPosition) -> TextBoundaryRecord;

/// Signature for a function that determines the [`TextBoundaryRecord`] at the given
/// [`TextPosition`], for the given text.
type TextBoundaryAtPositionInText =
    fn(Handle<SelectableFragment>, &mut App, TextPosition, &str) -> TextBoundaryRecord;

const K_ELLIPSIS: &str = "\u{2026}";

/// Parent data used by [`RenderParagraph`] and `RenderEditable` to annotate inline contents
/// (such as `WidgetSpan`s) with.
#[derive(Debug, Default)]
pub struct TextParentData {
    container_parent_data: ContainerParentData<AnyRenderBox>,
    offset: Option<Offset>,

    /// The [`PlaceholderSpan`] associated with this render child.
    ///
    /// This field is usually set by a `ParentDataWidget`, and is typically not `None` when
    /// `perform_layout` is called.
    pub span: Option<Rc<dyn PlaceholderSpan>>,
}

impl TextParentData {
    /// The offset at which to paint the child in the parent's coordinate system.
    ///
    /// A `None` value indicates this inline widget is not laid out. For instance, when the
    /// inline widget has never been laid out, or the inline widget is ellipsized away.
    pub fn offset(&self) -> Option<Offset> {
        self.offset
    }
}

impl ParentData for TextParentData {
    fn detach(&mut self) {
        self.span = None;
        self.offset = None;
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        (id == TypeId::of::<TextParentData>()).then_some(self)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        (id == TypeId::of::<TextParentData>()).then_some(self)
    }
}

impl ContainerParentDataMixin for TextParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container_parent_data
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container_parent_data
    }
}

impl fmt::Display for TextParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "widget: {:?}, ", self.span)?;
        match self.offset {
            None => f.write_str("not laid out"),
            Some(offset) => write!(f, "offset: {offset:?}"),
        }
    }
}

/// A mixin that provides useful default behaviors for text [`RenderBox`]es
/// ([`RenderParagraph`] and `RenderEditable` for example) with inline content children
/// managed by the [`ContainerRenderObjectMixin`] mixin.
///
/// This mixin assumes every child managed by the [`ContainerRenderObjectMixin`] mixin
/// corresponds to a [`PlaceholderSpan`], and they are organized in logical order of the text
/// (the order each [`PlaceholderSpan`] is encountered when the user reads the text).
///
/// To use this mixin in a [`RenderBox`] class:
///
///  * Call [`layout_inline_children`](Self::layout_inline_children) in the `perform_layout`
///    and `compute_dry_layout` implementation, and during intrinsic size calculations, to get
///    the size information of the inline widgets as a `Vec` of [`PlaceholderDimensions`].
///    Determine the positioning of the inline widgets (which is usually done by a
///    [`TextPainter`] using its line break algorithm).
///
///  * Call [`position_inline_children`](Self::position_inline_children) with the positioning
///    information of the inline widgets.
///
///  * Implement [`RenderBox::apply_paint_transform`], optionally with
///    [`default_apply_paint_transform`](Self::default_apply_paint_transform).
///
///  * Call [`paint_inline_children`](Self::paint_inline_children) in [`RenderObject::paint`]
///    to paint the inline widgets.
///
///  * Call [`hit_test_inline_children`](Self::hit_test_inline_children) in
///    [`RenderBox::hit_test_children`] to hit test the inline widgets.
///
/// See also:
///
///  * `WidgetSpan::extract_from_inline_span`, a helper function for extracting `WidgetSpan`s
///    from an `InlineSpan` tree.
pub trait RenderInlineChildrenContainerDefaults:
    RenderBox + ContainerRenderObjectMixin<ChildType = AnyRenderBox, ParentDataType = TextParentData>
{
    /// Installs [`TextParentData`] on `child`, the mixin's `setupParentData`.
    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<TextParentData>(app) {
            child.set_parent_data(app, TextParentData::default());
        }
    }

    /// Computes the layout for every inline child using the `max_width` constraint.
    ///
    /// Returns a list of [`PlaceholderDimensions`], representing the layout results for each
    /// child managed by the [`ContainerRenderObjectMixin`] mixin.
    ///
    /// The `get_child_baseline` parameter and the `layout_child` parameter must be
    /// consistent: if `layout_child` computes the size of the child without modifying the
    /// actual layout of that child, then `get_child_baseline` must also be "dry", and vice
    /// versa.
    ///
    /// Since this method does not impose a maximum height constraint on the inline children,
    /// some children may become taller than this [`RenderBox`].
    ///
    /// See also:
    ///
    ///  * [`TextPainter::set_placeholder_dimensions`], the method that usually takes the
    ///    layout results from this method as the input.
    fn layout_inline_children(
        self: RenderHandle<Self>,
        app: &mut App,
        max_width: f64,
        layout_child: ChildLayouter,
        get_child_baseline: ChildBaselineGetter,
    ) -> Vec<PlaceholderDimensions> {
        let constraints = BoxConstraints::new().max_width(max_width);
        let mut dimensions = Vec::new();
        let mut child = self.first_child(app);
        while let Some(current) = child {
            dimensions.push(layout_child_inline(
                app,
                current,
                constraints,
                layout_child,
                get_child_baseline,
            ));
            child = self.child_after(app, current);
        }
        dimensions
    }

    /// Positions each inline child according to the coordinates provided in the `boxes`
    /// list.
    ///
    /// The `boxes` list must be in logical order, which is the order each child is
    /// encountered when the user reads the text. Usually the length of the list equals
    /// [`child_count`](ContainerRenderObjectMixin::child_count), but it can be less than
    /// that, when some children are omitted due to ellipsing. It never exceeds
    /// [`child_count`](ContainerRenderObjectMixin::child_count).
    ///
    /// See also:
    ///
    ///  * [`TextPainter::inline_placeholder_boxes`], the method that can be used to get the
    ///    input `boxes`.
    fn position_inline_children(self: RenderHandle<Self>, app: &mut App, boxes: &[TextBox]) {
        let mut child = self.first_child(app);
        for text_box in boxes {
            let Some(current) = child else {
                debug_assert!(
                    false,
                    "Invalid number of boxes provided to positionInlineChildren. The number of \
                     boxes ({}) exceeds the number of child render objects ({}).",
                    boxes.len(),
                    self.child_count(app)
                );
                return;
            };
            text_parent_data_mut(app, current).offset =
                Some(Offset::new(text_box.left, text_box.top));
            child = self.child_after(app, current);
        }
        while let Some(current) = child {
            text_parent_data_mut(app, current).offset = None;
            child = self.child_after(app, current);
        }
    }

    /// Applies the transform that would be applied when painting the given child to the given
    /// matrix.
    ///
    /// Render children whose [`TextParentData::offset`] is `None` zeros out the `transform` to
    /// indicate they're invisible thus should not be painted.
    fn default_apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        match child.parent_data_of::<TextParentData>(app).offset {
            None => *transform = crate::object::zero_matrix(),
            Some(offset) => crate::object::translate(transform, offset.dx(), offset.dy()),
        }
    }

    /// Paints each inline child.
    ///
    /// Render children whose [`TextParentData::offset`] is `None` will be skipped by this
    /// method.
    fn paint_inline_children(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let mut child = self.first_child(app);
        while let Some(current) = child {
            let Some(child_offset) = text_parent_data(app, current).offset else {
                return;
            };
            context.paint_child(app, current.as_object(), child_offset + offset);
            child = self.child_after(app, current);
        }
    }

    /// Performs a hit test on each inline child.
    ///
    /// Render children whose [`TextParentData::offset`] is `None` will be skipped by this
    /// method.
    fn hit_test_inline_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let mut child = self.first_child(app);
        while let Some(current) = child {
            let Some(child_offset) = text_parent_data(app, current).offset else {
                return false;
            };
            let is_hit = result.add_with_paint_offset(
                Some(child_offset),
                position,
                |result, transformed| current.hit_test(app, result, transformed),
            );
            if is_hit {
                return true;
            }
            child = self.child_after(app, current);
        }
        false
    }
}

/// The mixin's `_layoutChild`: lays `child` out with `layout_child` and describes it as a
/// placeholder for its span.
fn layout_child_inline(
    app: &mut App,
    child: AnyRenderBox,
    child_constraints: BoxConstraints,
    layout_child: ChildLayouter,
    get_baseline: ChildBaselineGetter,
) -> PlaceholderDimensions {
    let span = text_parent_data(app, child).span.clone();
    debug_assert!(span.is_some());
    let Some(span) = span else {
        return PlaceholderDimensions::EMPTY;
    };
    let size = layout_child(app, child, child_constraints);
    let baseline_offset = match span.alignment() {
        PlaceholderAlignment::AboveBaseline
        | PlaceholderAlignment::BelowBaseline
        | PlaceholderAlignment::Bottom
        | PlaceholderAlignment::Middle
        | PlaceholderAlignment::Top => None,
        PlaceholderAlignment::Baseline => get_baseline(
            app,
            child,
            child_constraints,
            span.baseline()
                .expect("a baseline-aligned placeholder names its baseline"),
        ),
    };
    PlaceholderDimensions {
        size,
        alignment: span.alignment(),
        baseline: span.baseline(),
        baseline_offset,
    }
}

fn text_parent_data(app: &App, child: AnyRenderBox) -> &TextParentData {
    child.as_object().parent_data_of::<TextParentData>(app)
}

fn text_parent_data_mut(app: &mut App, child: AnyRenderBox) -> &mut TextParentData {
    child.as_object().parent_data_of_mut::<TextParentData>(app)
}

/// A render object that displays a paragraph of text.
///
/// Dart's constructor arguments beyond `text` and `textDirection` are the setters; add the
/// inline children with [`add`](ContainerRenderObjectMixin::add) or
/// [`add_all`](ContainerRenderObjectMixin::add_all). The paragraph shapes against `fonts` when
/// given, else the app-wide collection of `PaintingBinding`.
pub struct RenderParagraph {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    text_painter: TextPainter,
    text_intrinsics: Option<TextPainter>,
    fonts_override: Option<Handle<FontCollection>>,
    soft_wrap: bool,
    overflow: TextOverflow,
    device_pixel_ratio: f64,
    needs_clipping: bool,
    // Placeholder dimensions representing the sizes of child inline widgets.
    //
    // These need to be cached because the text painter's placeholder dimensions
    // will be overwritten during intrinsic width/height calculations and must be
    // restored to the original values before final layout and painting.
    placeholder_dimensions: Option<Vec<PlaceholderDimensions>>,
    system_fonts: RelayoutWhenSystemFontsChangeData,
    selection_color: Option<Color>,
    // Should be `None` if selection is not enabled, i.e. `registrar` is `None`. The paragraph
    // splits on [`PLACEHOLDER_CODE_UNIT`] and keeps each fragment here.
    last_selectable_fragments: Option<Vec<Ref<SelectableFragment>>>,
    registrar: Option<SelectionRegistrarRef>,
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
                container: ContainerRenderObjectData::new(),
                text_painter,
                text_intrinsics: None,
                fonts_override: fonts,
                soft_wrap: true,
                overflow: TextOverflow::Clip,
                device_pixel_ratio: 1.0,
                needs_clipping: false,
                placeholder_dimensions: None,
                system_fonts: RelayoutWhenSystemFontsChangeData::new(),
                selection_color: None,
                last_selectable_fragments: None,
                registrar: None,
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
                self.remove_selection_registrar_subscription(app);
                self.dispose_selectable_fragments(app);
                self.update_selection_registrar_subscription(app);
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
        let placeholder_dimensions = self.layout_inline_children(
            app,
            width,
            ChildLayoutHelper::dry_layout_child,
            ChildLayoutHelper::get_dry_baseline,
        );
        let max_width = self.adjust_max_width(app, width);
        let (intrinsics, fonts) = self.text_intrinsics_with_fonts(app);
        intrinsics.set_placeholder_dimensions(Some(placeholder_dimensions));
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
        let placeholder_dimensions = self.get(app).placeholder_dimensions.clone();
        let (painter, fonts) = self.painter_with_fonts(app);
        painter.set_placeholder_dimensions(placeholder_dimensions);
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

    /// The ongoing selections in this paragraph.
    ///
    /// Selections in placeholder spans are not included; those are handled by their own
    /// subtrees.
    pub fn selections(self: RenderHandle<Self>, app: &App) -> Vec<TextSelection> {
        let Some(fragments) = &self.get(app).last_selectable_fragments else {
            return Vec::new();
        };
        fragments
            .iter()
            .filter_map(|fragment| {
                let this = app.get(fragment);
                Some(TextSelection::new(
                    this.text_selection_start?.offset,
                    this.text_selection_end?.offset,
                ))
            })
            .collect()
    }

    /// The [`SelectionRegistrar`](crate::SelectionRegistrar) this paragraph is, or will be,
    /// registered to.
    pub fn registrar(self: RenderHandle<Self>, app: &App) -> Option<AnySelectionRegistrar> {
        self.get(app)
            .registrar
            .as_ref()
            .map(SelectionRegistrarRef::handle)
    }

    /// Sets [`registrar`](Self::registrar).
    pub fn set_registrar(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<SelectionRegistrarRef>,
    ) {
        if value.as_ref().map(SelectionRegistrarRef::handle) == self.registrar(app) {
            return;
        }
        self.remove_selection_registrar_subscription(app);
        self.dispose_selectable_fragments(app);
        self.get_mut(app).registrar = value;
        self.update_selection_registrar_subscription(app);
    }

    fn update_selection_registrar_subscription(self: RenderHandle<Self>, app: &mut App) {
        let Some(registrar) = self.registrar(app) else {
            return;
        };
        if self.get(app).last_selectable_fragments.is_none() {
            let fragments = self.get_selectable_fragments(app);
            self.get_mut(app).last_selectable_fragments = Some(fragments);
        }
        let fragments = self
            .get(app)
            .last_selectable_fragments
            .clone()
            .expect("assigned just now");
        for fragment in &fragments {
            registrar.add(app, fragment.as_selectable());
        }
        if !fragments.is_empty() {
            self.as_object().mark_needs_compositing_bits_update(app);
        }
    }

    fn remove_selection_registrar_subscription(self: RenderHandle<Self>, app: &mut App) {
        let (Some(registrar), Some(fragments)) = (
            self.registrar(app),
            self.get(app).last_selectable_fragments.clone(),
        ) else {
            return;
        };
        for fragment in &fragments {
            registrar.remove(app, fragment.as_selectable());
        }
    }

    fn get_selectable_fragments(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> Vec<Ref<SelectableFragment>> {
        let plain_text = self.text(app).to_plain_text(false, true);
        let code_units: Vec<u16> = plain_text.encode_utf16().collect();
        let mut result = Vec::new();
        let mut start = 0usize;
        while start < code_units.len() {
            let mut end = code_units[start..]
                .iter()
                .position(|unit| *unit == waft_painting::PLACEHOLDER_CODE_UNIT)
                .map_or(-1, |offset| (start + offset) as i32);
            if start as i32 != end {
                if end == -1 {
                    end = code_units.len() as i32;
                }
                result.push(app.create(SelectableFragment::new(
                    self,
                    TextRange::new(start as i32, end),
                    plain_text.clone(),
                )));
                start = end as usize;
            }
            start += 1;
        }
        result
    }

    /// Whether `selectable` was created by this paragraph.
    pub fn selectable_belongs_to_paragraph(
        self: RenderHandle<Self>,
        app: &App,
        selectable: AnySelectable,
    ) -> bool {
        self.get(app)
            .last_selectable_fragments
            .as_ref()
            .is_some_and(|fragments| {
                fragments
                    .iter()
                    .any(|f| f.as_selectable().id() == selectable.id())
            })
    }

    fn dispose_selectable_fragments(self: RenderHandle<Self>, app: &mut App) {
        let Some(fragments) = self.get_mut(app).last_selectable_fragments.take() else {
            return;
        };
        for fragment in fragments {
            fragment.dispose_selectable(app);
        }
    }

    /// The color to use when painting the selection.
    ///
    /// Ignored if the text is not selectable (for example if [`registrar`](Self::registrar) is
    /// `None`).
    pub fn selection_color(self: RenderHandle<Self>, app: &App) -> Option<Color> {
        self.get(app).selection_color
    }

    /// Sets [`selection_color`](Self::selection_color).
    pub fn set_selection_color(self: RenderHandle<Self>, app: &mut App, value: Option<Color>) {
        if self.get(app).selection_color == value {
            return;
        }
        self.get_mut(app).selection_color = value;
        let has_selection = self
            .get(app)
            .last_selectable_fragments
            .as_ref()
            .is_some_and(|fragments| {
                fragments
                    .iter()
                    .any(|fragment| app.get(fragment).selection_geometry.has_selection())
            });
        if has_selection {
            self.mark_needs_paint(app);
        }
    }

    fn get_offset_for_position(
        self: RenderHandle<Self>,
        app: &mut App,
        position: TextPosition,
    ) -> Offset {
        self.get_offset_for_caret(app, position, Rect::ZERO)
            + Offset::new(0.0, self.get_full_height_for_caret(app, position))
    }

    /// Returns a [`TextSelection`] containing the line of the given [`TextPosition`].
    pub fn get_line_at_offset(
        self: RenderHandle<Self>,
        app: &mut App,
        position: TextPosition,
    ) -> TextSelection {
        let constraints = self.constraints(app);
        self.layout_text_with_constraints(app, constraints);
        let line = self.painter(app).get_line_boundary(position);
        TextSelection::new(line.start, line.end)
    }

    /// Returns the [`TextPosition`] above the given offset into the text.
    pub fn get_text_position_above(
        self: RenderHandle<Self>,
        app: &mut App,
        position: TextPosition,
    ) -> TextPosition {
        let line_height = self.preferred_line_height(app);
        self.get_text_position_vertical(app, position, -0.5 * line_height)
    }

    /// Returns the [`TextPosition`] below the given offset into the text.
    pub fn get_text_position_below(
        self: RenderHandle<Self>,
        app: &mut App,
        position: TextPosition,
    ) -> TextPosition {
        let line_height = self.preferred_line_height(app);
        self.get_text_position_vertical(app, position, 1.5 * line_height)
    }

    fn get_text_position_vertical(
        self: RenderHandle<Self>,
        app: &mut App,
        position: TextPosition,
        vertical_offset: f64,
    ) -> TextPosition {
        let (painter, fonts) = self.painter_with_fonts(app);
        let caret_offset = painter.get_offset_for_caret(fonts, position, Rect::ZERO);
        painter.get_position_for_offset(caret_offset.translate(0.0, vertical_offset))
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

impl ContainerRenderObjectMixin for RenderParagraph {
    type ChildType = AnyRenderBox;
    type ParentDataType = TextParentData;

    fn container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &ContainerRenderObjectData<AnyRenderBox> {
        &self.get(app).container
    }

    fn container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut ContainerRenderObjectData<AnyRenderBox> {
        &mut self.get_mut(app).container
    }
}

impl RenderInlineChildrenContainerDefaults for RenderParagraph {}

impl RenderObject for RenderParagraph {
    crate::render_object_accessors!();

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner);
        RelayoutWhenSystemFontsChangeMixin::attach_system_fonts(self, app);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        RelayoutWhenSystemFontsChangeMixin::detach_system_fonts(self, app);
        ContainerRenderObjectMixin::did_detach(self, app);
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app)
    }

    fn always_needs_compositing(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app)
            .last_selectable_fragments
            .as_ref()
            .is_some_and(|fragments| !fragments.is_empty())
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        self.remove_selection_registrar_subscription(app);
        self.dispose_selectable_fragments(app);
        self.painter_mut(app).dispose();
        if let Some(mut intrinsics) = self.get_mut(app).text_intrinsics.take() {
            intrinsics.dispose();
        }
        crate::object::RenderObjectBase::dispose(self, app);
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let placeholder_dimensions = self.layout_inline_children(
            app,
            constraints.max_width,
            ChildLayoutHelper::layout_child,
            ChildLayoutHelper::get_baseline,
        );
        self.get_mut(app).placeholder_dimensions = Some(placeholder_dimensions);
        self.layout_text_with_constraints(app, constraints);
        let boxes = self
            .painter_mut(app)
            .inline_placeholder_boxes()
            .expect("laid out just now");
        self.position_inline_children(app, &boxes);
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
        if let Some(fragments) = self.get(app).last_selectable_fragments.clone() {
            if needs_clipping {
                context.canvas().save();
                context
                    .canvas()
                    .clip_rect(offset & self.size(app), ClipOp::Intersect, true);
            }
            for fragment in fragments {
                fragment.paint_selection(app, context, offset);
            }
            if needs_clipping {
                context.canvas().restore();
            }
        }

        if needs_clipping {
            // The fade shader's `saveLayer` waits on gradients; a fade clips meanwhile.
            let bounds = offset & self.size(app);
            context.canvas().save();
            context.canvas().clip_rect(bounds, ClipOp::Intersect, true);
        }

        let (painter, fonts) = self.painter_with_fonts(app);
        painter.paint(fonts, context.canvas(), offset);

        self.paint_inline_children(app, context, offset);

        if needs_clipping {
            context.canvas().restore();
        }

        if let Some(fragments) = self.get(app).last_selectable_fragments.clone() {
            for fragment in fragments {
                fragment.paint_handles(app, context, offset);
            }
        }
    }
}

impl RenderBox for RenderParagraph {
    crate::render_box_accessors!();

    fn mark_needs_layout(self: RenderHandle<Self>, app: &mut App) {
        if let Some(fragments) = self.get(app).last_selectable_fragments.clone() {
            for fragment in fragments {
                fragment.did_change_paragraph_layout(app);
            }
        }
        crate::box_::RenderBoxBase::mark_needs_layout(self, app);
    }

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderInlineChildrenContainerDefaults::setup_parent_data(self, app, child);
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, _height: f64) -> f64 {
        let placeholder_dimensions = self.layout_inline_children(
            app,
            f64::INFINITY,
            |app, child, _| Size::new(child.get_min_intrinsic_width(app, f64::INFINITY), 0.0),
            ChildLayoutHelper::get_dry_baseline,
        );
        let (intrinsics, fonts) = self.text_intrinsics_with_fonts(app);
        intrinsics.set_placeholder_dimensions(Some(placeholder_dimensions));
        intrinsics.layout(fonts, 0.0, f64::INFINITY);
        intrinsics.min_intrinsic_width()
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, _height: f64) -> f64 {
        let placeholder_dimensions = self.layout_inline_children(
            app,
            f64::INFINITY,
            // Height and baseline is irrelevant as all text will be laid
            // out in a single line. Therefore, using 0.0 as a dummy for the height.
            |app, child, _| Size::new(child.get_max_intrinsic_width(app, f64::INFINITY), 0.0),
            ChildLayoutHelper::get_dry_baseline,
        );
        let (intrinsics, fonts) = self.text_intrinsics_with_fonts(app);
        intrinsics.set_placeholder_dimensions(Some(placeholder_dimensions));
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
        let placeholder_dimensions = self.layout_inline_children(
            app,
            constraints.max_width,
            ChildLayoutHelper::dry_layout_child,
            ChildLayoutHelper::get_dry_baseline,
        );
        let max_width = self.adjust_max_width(app, constraints.max_width);
        let (intrinsics, fonts) = self.text_intrinsics_with_fonts(app);
        intrinsics.set_placeholder_dimensions(Some(placeholder_dimensions));
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
        let placeholder_dimensions = self.layout_inline_children(
            app,
            constraints.max_width,
            ChildLayoutHelper::dry_layout_child,
            ChildLayoutHelper::get_dry_baseline,
        );
        let max_width = self.adjust_max_width(app, constraints.max_width);
        let (intrinsics, fonts) = self.text_intrinsics_with_fonts(app);
        intrinsics.set_placeholder_dimensions(Some(placeholder_dimensions));
        intrinsics.layout(fonts, constraints.min_width, max_width);
        Some(intrinsics.compute_distance_to_actual_baseline(TextBaseline::Alphabetic))
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        self.default_apply_paint_transform(app, child, transform);
    }

    fn hit_test_self(self: RenderHandle<Self>, _app: &App, _position: Offset) -> bool {
        true
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let painter = self.painter(app);
        let glyph = painter.get_closest_glyph_for_offset(position);
        // The hit-test can't fall through the horizontal gaps between visually
        // adjacent characters on the same line, even with a large letter-spacing or
        // text justification, as grapheme_cluster_layout_bounds.width is the advance
        // width to the next character, so there's no gap between their
        // grapheme_cluster_layout_bounds rects.
        let span_hit = glyph
            .filter(|glyph| glyph.grapheme_cluster_layout_bounds.contains(position))
            .and_then(|glyph| {
                painter
                    .text()
                    .expect("a paragraph always has text")
                    .clone()
                    .get_span_for_position(TextPosition::new(
                        glyph.grapheme_cluster_code_unit_range.start,
                    ))
                    .and_then(|span| span.to_hit_test_target())
            });
        match span_hit {
            Some(span) => {
                result.add(span.into());
                true
            }
            None => self.hit_test_inline_children(app, result, position),
        }
    }
}

/// A continuous, selectable piece of paragraph.
///
/// Placeholder spans are selected by their own subtrees, so a selection in a paragraph cannot
/// continue across one. The paragraph splits itself on placeholders and keeps one fragment per
/// run of text, each selectable separately.
struct SelectableFragment {
    change_notifier: ChangeNotifierData,
    range: TextRange,
    paragraph: RenderHandle<RenderParagraph>,
    full_text: String,
    text_selection_start: Option<TextPosition>,
    text_selection_end: Option<TextPosition>,
    selectable_contains_origin_text_boundary: bool,
    start_handle_layer_link: Option<Ref<LayerLink>>,
    end_handle_layer_link: Option<Ref<LayerLink>>,
    selection_geometry: SelectionGeometry,
    cached_bounding_boxes: Option<Vec<Rect>>,
    cached_rect: Option<Rect>,
}

#[allow(
    clippy::unnecessary_unwrap,
    clippy::collapsible_if,
    clippy::too_many_arguments
)]
impl SelectableFragment {
    fn new(
        paragraph: RenderHandle<RenderParagraph>,
        range: TextRange,
        full_text: String,
    ) -> SelectableFragment {
        debug_assert!(range.is_valid() && !range.is_collapsed() && range.is_normalized());
        SelectableFragment {
            change_notifier: ChangeNotifierData::new(),
            range,
            paragraph,
            full_text,
            text_selection_start: None,
            text_selection_end: None,
            selectable_contains_origin_text_boundary: false,
            start_handle_layer_link: None,
            end_handle_layer_link: None,
            selection_geometry: SelectionGeometry::new(SelectionStatus::None, true),
            cached_bounding_boxes: None,
            cached_rect: None,
        }
    }

    fn update_selection_geometry(self: Handle<Self>, app: &mut App) {
        let new_value = self.get_selection_geometry(app);
        if app.get(self).selection_geometry == new_value {
            return;
        }
        app.get_mut(self).selection_geometry = new_value;
        self.notify_listeners(app);
    }

    fn get_selection_geometry(self: Handle<Self>, app: &mut App) -> SelectionGeometry {
        let (Some(selection_start), Some(selection_end)) = (
            app.get(self).text_selection_start,
            app.get(self).text_selection_end,
        ) else {
            return SelectionGeometry::new(SelectionStatus::None, true);
        };
        let paragraph = app.get(self).paragraph;
        let is_reversed = selection_start.offset > selection_end.offset;
        let start_offset_in_paragraph_coordinates =
            paragraph.get_offset_for_position(app, selection_start);
        let end_offset_in_paragraph_coordinates = if selection_start.offset == selection_end.offset
        {
            start_offset_in_paragraph_coordinates
        } else {
            paragraph.get_offset_for_position(app, selection_end)
        };
        let flip_handles = is_reversed != (paragraph.text_direction(app) == TextDirection::Rtl);
        let selection = TextSelection::new(selection_start.offset, selection_end.offset);
        let selection_rects: Vec<Rect> = paragraph
            .get_boxes_for_selection(app, selection, BoxHeightStyle::Tight, BoxWidthStyle::Tight)
            .into_iter()
            .map(|text_box| text_box.to_rect())
            .collect();
        let selection_collapsed = selection_start.offset == selection_end.offset;
        let (start_selection_handle_type, end_selection_handle_type) = if selection_collapsed {
            (
                TextSelectionHandleType::Collapsed,
                TextSelectionHandleType::Collapsed,
            )
        } else if flip_handles {
            (
                TextSelectionHandleType::Right,
                TextSelectionHandleType::Left,
            )
        } else {
            (
                TextSelectionHandleType::Left,
                TextSelectionHandleType::Right,
            )
        };
        let line_height = paragraph.preferred_line_height(app);
        let status = if selection_collapsed {
            SelectionStatus::Collapsed
        } else {
            SelectionStatus::Uncollapsed
        };
        SelectionGeometry::new(status, true)
            .start_selection_point(Some(SelectionPoint::new(
                start_offset_in_paragraph_coordinates,
                line_height,
                start_selection_handle_type,
            )))
            .end_selection_point(Some(SelectionPoint::new(
                end_offset_in_paragraph_coordinates,
                line_height,
                end_selection_handle_type,
            )))
            .selection_rects(selection_rects)
    }

    fn did_change_selection(self: Handle<Self>, app: &mut App) {
        app.get(self).paragraph.mark_needs_paint(app);
        self.update_selection_geometry(app);
    }

    fn set_selection_position(
        self: Handle<Self>,
        app: &mut App,
        position: Option<TextPosition>,
        is_end: bool,
    ) {
        if is_end {
            app.get_mut(self).text_selection_end = position;
        } else {
            app.get_mut(self).text_selection_start = position;
        }
    }

    fn handle_clear_selection(self: Handle<Self>, app: &mut App) -> SelectionResult {
        let this = app.get_mut(self);
        this.text_selection_start = None;
        this.text_selection_end = None;
        this.selectable_contains_origin_text_boundary = false;
        SelectionResult::None
    }

    fn handle_select_all(self: Handle<Self>, app: &mut App) -> SelectionResult {
        let range = app.get(self).range;
        let this = app.get_mut(self);
        this.text_selection_start = Some(TextPosition::new(range.start));
        this.text_selection_end = Some(TextPosition::with_affinity(
            range.end,
            TextAffinity::Upstream,
        ));
        SelectionResult::None
    }

    fn handle_select_text_boundary(
        self: Handle<Self>,
        app: &mut App,
        text_boundary: TextBoundaryRecord,
    ) -> SelectionResult {
        let range = app.get(self).range;
        if text_boundary.boundary_start.offset < range.start
            && text_boundary.boundary_end.offset <= range.start
        {
            return SelectionResult::Previous;
        } else if text_boundary.boundary_start.offset >= range.end
            && text_boundary.boundary_end.offset > range.end
        {
            return SelectionResult::Next;
        }
        debug_assert!(
            text_boundary.boundary_start.offset >= range.start
                && text_boundary.boundary_end.offset <= range.end
        );
        let this = app.get_mut(self);
        this.text_selection_start = Some(text_boundary.boundary_start);
        this.text_selection_end = Some(text_boundary.boundary_end);
        this.selectable_contains_origin_text_boundary = true;
        SelectionResult::End
    }

    fn intersect(a: TextRange, b: TextRange) -> Option<TextRange> {
        debug_assert!(a.is_normalized());
        debug_assert!(b.is_normalized());
        let start_max = a.start.max(b.start);
        let end_min = a.end.min(b.end);
        (start_max <= end_min).then(|| TextRange::new(start_max, end_min))
    }

    fn handle_select_multi_fragment_text_boundary(
        self: Handle<Self>,
        app: &mut App,
        text_boundary: TextBoundaryRecord,
    ) -> SelectionResult {
        let range = app.get(self).range;
        if text_boundary.boundary_start.offset < range.start
            && text_boundary.boundary_end.offset <= range.start
        {
            return SelectionResult::Previous;
        } else if text_boundary.boundary_start.offset >= range.end
            && text_boundary.boundary_end.offset > range.end
        {
            return SelectionResult::Next;
        }
        let boundary_as_range = TextRange::new(
            text_boundary.boundary_start.offset,
            text_boundary.boundary_end.offset,
        );
        let Some(intersect_range) = Self::intersect(range, boundary_as_range) else {
            return SelectionResult::None;
        };
        let this = app.get_mut(self);
        this.text_selection_start = Some(TextPosition::new(intersect_range.start));
        this.text_selection_end = Some(TextPosition::new(intersect_range.end));
        this.selectable_contains_origin_text_boundary = true;
        if range.end < text_boundary.boundary_end.offset {
            return SelectionResult::Next;
        }
        SelectionResult::End
    }

    fn adjust_text_boundary_at_position(
        text_boundary: TextRange,
        position: TextPosition,
    ) -> TextBoundaryRecord {
        if position.offset > text_boundary.end {
            let at = TextPosition::new(position.offset);
            TextBoundaryRecord::new(at, at)
        } else {
            TextBoundaryRecord::new(
                TextPosition::new(text_boundary.start),
                TextPosition::with_affinity(text_boundary.end, TextAffinity::Upstream),
            )
        }
    }

    fn handle_select_word(
        self: Handle<Self>,
        app: &mut App,
        global_position: Offset,
    ) -> SelectionResult {
        let paragraph = app.get(self).paragraph;
        let local_position = paragraph
            .as_box()
            .global_to_local(app, global_position, None);
        let position = paragraph.get_position_for_offset(app, local_position);
        if self.position_is_within_current_selection(app, position)
            && app.get(self).text_selection_start != app.get(self).text_selection_end
        {
            return SelectionResult::End;
        }
        let word_boundary = self.get_word_boundary_at_position(app, position);
        self.handle_select_text_boundary(app, word_boundary)
    }

    fn get_word_boundary_at_position(
        self: Handle<Self>,
        app: &mut App,
        position: TextPosition,
    ) -> TextBoundaryRecord {
        let word = app.get(self).paragraph.get_word_boundary(app, position);
        debug_assert!(word.is_normalized());
        Self::adjust_text_boundary_at_position(word, position)
    }

    fn handle_select_paragraph(
        self: Handle<Self>,
        app: &mut App,
        global_position: Offset,
    ) -> SelectionResult {
        let paragraph = app.get(self).paragraph;
        let local_position = paragraph
            .as_box()
            .global_to_local(app, global_position, None);
        let position = paragraph.get_position_for_offset(app, local_position);
        let full_text = app.get(self).full_text.clone();
        let paragraph_boundary = self.get_paragraph_boundary_at_position(app, position, &full_text);
        self.handle_select_multi_fragment_text_boundary(app, paragraph_boundary)
    }

    fn get_position_in_paragraph(
        self: Handle<Self>,
        app: &mut App,
        target_paragraph: RenderHandle<RenderParagraph>,
    ) -> TextPosition {
        let paragraph = app.get(self).paragraph;
        let transform = paragraph
            .as_object()
            .get_transform_to(app, Some(target_paragraph.as_object()));
        let local_center = paragraph.as_object().paint_bounds(app).center_left();
        let local_pos = waft_painting::transform_point(&transform, local_center);
        target_paragraph.get_position_for_offset(app, local_pos)
    }

    fn get_paragraph_boundary_at_position(
        self: Handle<Self>,
        app: &mut App,
        position: TextPosition,
        text: &str,
    ) -> TextBoundaryRecord {
        let paragraph_boundary = ParagraphBoundary::new(text);
        let paragraph_start = paragraph_boundary
            .get_leading_text_boundary_at(
                app,
                if position.offset == text.encode_utf16().count() as i32
                    || position.affinity == TextAffinity::Upstream
                {
                    position.offset - 1
                } else {
                    position.offset
                },
            )
            .unwrap_or(0);
        let paragraph_end = paragraph_boundary
            .get_trailing_text_boundary_at(app, position.offset)
            .unwrap_or(text.encode_utf16().count() as i32);
        let paragraph_range = TextRange::new(paragraph_start, paragraph_end);
        debug_assert!(paragraph_range.is_normalized());
        Self::adjust_text_boundary_at_position(paragraph_range, position)
    }

    fn get_clamped_paragraph_boundary_at_position(
        self: Handle<Self>,
        app: &mut App,
        position: TextPosition,
    ) -> TextBoundaryRecord {
        let full_text = app.get(self).full_text.clone();
        let range = app.get(self).range;
        let paragraph_boundary = ParagraphBoundary::new(&full_text);
        let mut paragraph_start = paragraph_boundary
            .get_leading_text_boundary_at(
                app,
                if position.offset == full_text.encode_utf16().count() as i32
                    || position.affinity == TextAffinity::Upstream
                {
                    position.offset - 1
                } else {
                    position.offset
                },
            )
            .unwrap_or(0);
        let mut paragraph_end = paragraph_boundary
            .get_trailing_text_boundary_at(app, position.offset)
            .unwrap_or(full_text.encode_utf16().count() as i32);
        paragraph_start = if paragraph_start < range.start {
            range.start
        } else if paragraph_start > range.end {
            range.end
        } else {
            paragraph_start
        };
        paragraph_end = if paragraph_end > range.end {
            range.end
        } else if paragraph_end < range.start {
            range.start
        } else {
            paragraph_end
        };
        let paragraph_range = TextRange::new(paragraph_start, paragraph_end);
        debug_assert!(paragraph_range.is_normalized());
        Self::adjust_text_boundary_at_position(paragraph_range, position)
    }

    fn closest_text_boundary(
        text_boundary: TextBoundaryRecord,
        position: TextPosition,
    ) -> TextPosition {
        let difference_a = (position.offset - text_boundary.boundary_start.offset).abs();
        let difference_b = (position.offset - text_boundary.boundary_end.offset).abs();
        if difference_a < difference_b {
            text_boundary.boundary_start
        } else {
            text_boundary.boundary_end
        }
    }

    fn is_placeholder(self: Handle<Self>, app: &App) -> bool {
        let mut current = app.get(self).paragraph.as_object().parent(app);
        while let Some(object) = current {
            if object.downcast::<RenderParagraph>(app).is_some() {
                return true;
            }
            current = object.parent(app);
        }
        false
    }

    fn get_origin_paragraph(self: Handle<Self>, app: &App) -> RenderHandle<RenderParagraph> {
        debug_assert!(app.get(self).selectable_contains_origin_text_boundary);
        let paragraph = app.get(self).paragraph;
        let mut current = paragraph.as_object().parent(app);
        let mut origin_paragraph = None;
        while let Some(object) = current {
            if let Some(candidate) = object.downcast::<RenderParagraph>(app) {
                if let Some(fragments) = &app.get(candidate.handle()).last_selectable_fragments {
                    let mut paragraph_contains_origin = false;
                    for fragment in fragments {
                        if app.get(fragment).selectable_contains_origin_text_boundary {
                            paragraph_contains_origin = true;
                            origin_paragraph = Some(candidate);
                            break;
                        }
                    }
                    if !paragraph_contains_origin {
                        return origin_paragraph.unwrap_or(paragraph);
                    }
                }
            }
            current = object.parent(app);
        }
        origin_paragraph.unwrap_or(paragraph)
    }

    fn get_paragraph_containing_position(
        self: Handle<Self>,
        app: &App,
        global_position: Offset,
    ) -> Option<(RenderHandle<RenderParagraph>, Offset)> {
        let mut current = Some(app.get(self).paragraph.as_object());
        while let Some(object) = current {
            if let Some(paragraph) = object.downcast::<RenderParagraph>(app) {
                let local = global_to_paragraph(app, paragraph, global_position);
                if paragraph.as_object().paint_bounds(app).contains(local) {
                    return Some((paragraph, local));
                }
            }
            current = object.parent(app);
        }
        None
    }

    fn bounding_boxes_contain(self: Handle<Self>, app: &mut App, position: Offset) -> bool {
        self.bounding_boxes(app)
            .iter()
            .any(|rect| rect.contains(position))
    }

    fn clamp_text_position(self: Handle<Self>, app: &App, position: TextPosition) -> TextPosition {
        let range = app.get(self).range;
        if position.offset > range.end
            || (position.offset == range.end && position.affinity == TextAffinity::Downstream)
        {
            return TextPosition::with_affinity(range.end, TextAffinity::Upstream);
        }
        if position.offset < range.start {
            return TextPosition::new(range.start);
        }
        position
    }

    fn compare_text_positions(position: TextPosition, other_position: TextPosition) -> i32 {
        if position.offset < other_position.offset {
            1
        } else if position.offset > other_position.offset {
            -1
        } else if position.affinity == other_position.affinity {
            0
        } else if position.affinity == TextAffinity::Upstream {
            1
        } else {
            -1
        }
    }

    fn position_is_within_current_selection(
        self: Handle<Self>,
        app: &App,
        position: TextPosition,
    ) -> bool {
        let (Some(start), Some(end)) = (
            app.get(self).text_selection_start,
            app.get(self).text_selection_end,
        ) else {
            return false;
        };
        let (current_start, current_end) = if Self::compare_text_positions(start, end) > 0 {
            (start, end)
        } else {
            (end, start)
        };
        Self::compare_text_positions(current_start, position) >= 0
            && Self::compare_text_positions(current_end, position) <= 0
    }

    fn rect(self: Handle<Self>, app: &mut App) -> Rect {
        if let Some(cached) = app.get(self).cached_rect {
            return cached;
        }
        let range = app.get(self).range;
        let paragraph = app.get(self).paragraph;
        let boxes = paragraph.get_boxes_for_selection(
            app,
            TextSelection::new(range.start, range.end),
            BoxHeightStyle::Max,
            BoxWidthStyle::Tight,
        );
        let rect = if let Some(first) = boxes.first() {
            boxes
                .iter()
                .skip(1)
                .fold(first.to_rect(), |result, text_box| {
                    result.expand_to_include(text_box.to_rect())
                })
        } else {
            let offset = paragraph.get_offset_for_position(app, TextPosition::new(range.start));
            let line_height = paragraph.preferred_line_height(app);
            Rect::from_ltrb(
                offset.dx(),
                offset.dy() - line_height,
                offset.dx(),
                offset.dy(),
            )
        };
        app.get_mut(self).cached_rect = Some(rect);
        rect
    }

    /// Dart's `didChangeParagraphLayout`: forgets the rects measured from the old layout.
    fn did_change_paragraph_layout(self: Handle<Self>, app: &mut App) {
        let this = app.get_mut(self);
        this.cached_rect = None;
        this.cached_bounding_boxes = None;
    }

    /// `global_position` in this fragment's paragraph's coordinates.
    fn local_position(self: Handle<Self>, app: &App, global_position: Offset) -> Offset {
        let paragraph = app.get(self).paragraph;
        global_to_paragraph(app, paragraph, global_position)
    }

    fn update_selection_edge(
        self: Handle<Self>,
        app: &mut App,
        global_position: Offset,
        is_end: bool,
    ) -> SelectionResult {
        self.set_selection_position(app, None, is_end);
        let local_position = self.local_position(app, global_position);
        let rect = self.rect(app);
        let range = app.get(self).range;
        if rect.is_empty() {
            let result = SelectionUtils::get_result_based_on_rect(rect, local_position);
            let position = if result == SelectionResult::Next {
                TextPosition::new(range.end)
            } else {
                TextPosition::with_affinity(range.start, TextAffinity::Upstream)
            };
            self.set_selection_position(app, Some(position), is_end);
            return result;
        }
        let direction = app.get(self).paragraph.text_direction(app);
        let adjusted_offset = SelectionUtils::adjust_drag_offset(rect, local_position, direction);
        let paragraph = app.get(self).paragraph;
        let position = paragraph.get_position_for_offset(app, adjusted_offset);
        let position = self.clamp_text_position(app, position);
        self.set_selection_position(app, Some(position), is_end);
        if position.offset == range.end {
            return SelectionResult::Next;
        }
        if position.offset == range.start {
            return SelectionResult::Previous;
        }
        SelectionUtils::get_result_based_on_rect(rect, local_position)
    }

    fn update_selection_edge_by_text_boundary(
        self: Handle<Self>,
        app: &mut App,
        global_position: Offset,
        is_end: bool,
        get_text_boundary: TextBoundaryAtPosition,
    ) -> SelectionResult {
        let existing_selection_start = app.get(self).text_selection_start;
        let existing_selection_end = app.get(self).text_selection_end;
        self.set_selection_position(app, None, is_end);
        let local_position = self.local_position(app, global_position);
        let rect = self.rect(app);
        let range = app.get(self).range;
        if rect.is_empty() {
            let result = SelectionUtils::get_result_based_on_rect(rect, local_position);
            let position = if result == SelectionResult::Next {
                TextPosition::new(range.end)
            } else {
                TextPosition::with_affinity(range.start, TextAffinity::Upstream)
            };
            self.set_selection_position(app, Some(position), is_end);
            return result;
        }
        let direction = app.get(self).paragraph.text_direction(app);
        let adjusted_offset = SelectionUtils::adjust_drag_offset(rect, local_position, direction);
        let paragraph = app.get(self).paragraph;
        let position = paragraph.get_position_for_offset(app, adjusted_offset);
        let mut text_boundary = if rect.contains(local_position) {
            Some(get_text_boundary(self, app, position))
        } else {
            None
        };
        if let Some(boundary) = text_boundary {
            if (boundary.boundary_start.offset < range.start
                && boundary.boundary_end.offset <= range.start)
                || (boundary.boundary_start.offset >= range.end
                    && boundary.boundary_end.offset > range.end)
            {
                text_boundary = None;
            }
        }
        let edge_position = if is_end {
            self.update_selection_end_edge_by_text_boundary(
                app,
                text_boundary,
                get_text_boundary,
                position,
                existing_selection_start,
                existing_selection_end,
            )
        } else {
            self.update_selection_start_edge_by_text_boundary(
                app,
                text_boundary,
                get_text_boundary,
                position,
                existing_selection_start,
                existing_selection_end,
            )
        };
        let target_position = self.clamp_text_position(app, edge_position);
        self.set_selection_position(app, Some(target_position), is_end);
        if target_position.offset == range.end {
            return SelectionResult::Next;
        }
        if target_position.offset == range.start {
            return SelectionResult::Previous;
        }
        SelectionUtils::get_result_based_on_rect(rect, local_position)
    }

    fn update_selection_start_edge_by_text_boundary(
        self: Handle<Self>,
        app: &mut App,
        text_boundary: Option<TextBoundaryRecord>,
        get_text_boundary: TextBoundaryAtPosition,
        position: TextPosition,
        existing_selection_start: Option<TextPosition>,
        existing_selection_end: Option<TextPosition>,
    ) -> TextPosition {
        let range = app.get(self).range;
        let contains_origin = app.get(self).selectable_contains_origin_text_boundary;
        let mut target_position = None;

        if contains_origin
            && existing_selection_start.is_some()
            && existing_selection_end.is_some()
            && text_boundary.is_some()
        {
            let text_boundary = text_boundary.unwrap();
            let existing_start = existing_selection_start.unwrap();
            let existing_end = existing_selection_end.unwrap();
            debug_assert!(
                text_boundary.boundary_start.offset >= range.start
                    && text_boundary.boundary_end.offset <= range.end
            );
            let is_same_position = position.offset == existing_end.offset;
            let is_selection_inverted = existing_start.offset > existing_end.offset;
            let should_swap_edges = !is_same_position
                && (is_selection_inverted != (position.offset > existing_end.offset));
            if should_swap_edges {
                target_position = Some(if position.offset < existing_end.offset {
                    text_boundary.boundary_start
                } else {
                    text_boundary.boundary_end
                });
                let local_text_boundary = get_text_boundary(self, app, existing_end);
                debug_assert!(
                    local_text_boundary.boundary_start.offset >= range.start
                        && local_text_boundary.boundary_end.offset <= range.end
                );
                self.set_selection_position(
                    app,
                    Some(
                        if existing_end.offset == local_text_boundary.boundary_start.offset {
                            local_text_boundary.boundary_end
                        } else {
                            local_text_boundary.boundary_start
                        },
                    ),
                    true,
                );
            } else if position.offset < existing_end.offset {
                target_position = Some(text_boundary.boundary_start);
            } else if position.offset > existing_end.offset {
                target_position = Some(text_boundary.boundary_end);
            } else {
                target_position = Some(existing_start);
            }
        } else if let Some(text_boundary) = text_boundary {
            if let Some(existing_end) = existing_selection_end {
                if position.offset < existing_end.offset {
                    target_position = Some(text_boundary.boundary_start);
                } else {
                    target_position = Some(text_boundary.boundary_end);
                }
            } else {
                target_position = Some(Self::closest_text_boundary(text_boundary, position));
            }
        } else if contains_origin
            && existing_selection_start.is_some()
            && existing_selection_end.is_some()
        {
            let existing_start = existing_selection_start.unwrap();
            let existing_end = existing_selection_end.unwrap();
            let is_same_position = position.offset == existing_end.offset;
            let is_selection_inverted = existing_start.offset > existing_end.offset;
            let should_swap_edges = !is_same_position
                && (is_selection_inverted != (position.offset > existing_end.offset));
            if should_swap_edges {
                let local_text_boundary = get_text_boundary(self, app, existing_end);
                debug_assert!(
                    local_text_boundary.boundary_start.offset >= range.start
                        && local_text_boundary.boundary_end.offset <= range.end
                );
                self.set_selection_position(
                    app,
                    Some(if is_selection_inverted {
                        local_text_boundary.boundary_end
                    } else {
                        local_text_boundary.boundary_start
                    }),
                    true,
                );
            }
        }
        target_position.unwrap_or(position)
    }

    fn update_selection_end_edge_by_text_boundary(
        self: Handle<Self>,
        app: &mut App,
        text_boundary: Option<TextBoundaryRecord>,
        get_text_boundary: TextBoundaryAtPosition,
        position: TextPosition,
        existing_selection_start: Option<TextPosition>,
        existing_selection_end: Option<TextPosition>,
    ) -> TextPosition {
        let range = app.get(self).range;
        let contains_origin = app.get(self).selectable_contains_origin_text_boundary;
        let mut target_position = None;

        if contains_origin
            && existing_selection_start.is_some()
            && existing_selection_end.is_some()
            && text_boundary.is_some()
        {
            let text_boundary = text_boundary.unwrap();
            let existing_start = existing_selection_start.unwrap();
            let existing_end = existing_selection_end.unwrap();
            debug_assert!(
                text_boundary.boundary_start.offset >= range.start
                    && text_boundary.boundary_end.offset <= range.end
            );
            let is_same_position = position.offset == existing_start.offset;
            let is_selection_inverted = existing_start.offset > existing_end.offset;
            let should_swap_edges = !is_same_position
                && (is_selection_inverted != (position.offset < existing_start.offset));
            if should_swap_edges {
                target_position = Some(if position.offset < existing_start.offset {
                    text_boundary.boundary_start
                } else {
                    text_boundary.boundary_end
                });
                let local_text_boundary = get_text_boundary(self, app, existing_start);
                debug_assert!(
                    local_text_boundary.boundary_start.offset >= range.start
                        && local_text_boundary.boundary_end.offset <= range.end
                );
                self.set_selection_position(
                    app,
                    Some(
                        if existing_start.offset == local_text_boundary.boundary_start.offset {
                            local_text_boundary.boundary_end
                        } else {
                            local_text_boundary.boundary_start
                        },
                    ),
                    false,
                );
            } else if position.offset < existing_start.offset {
                target_position = Some(text_boundary.boundary_start);
            } else if position.offset > existing_start.offset {
                target_position = Some(text_boundary.boundary_end);
            } else {
                target_position = Some(existing_end);
            }
        } else if let Some(text_boundary) = text_boundary {
            if let Some(existing_start) = existing_selection_start {
                if position.offset < existing_start.offset {
                    target_position = Some(text_boundary.boundary_start);
                } else {
                    target_position = Some(text_boundary.boundary_end);
                }
            } else {
                target_position = Some(Self::closest_text_boundary(text_boundary, position));
            }
        } else if contains_origin
            && existing_selection_start.is_some()
            && existing_selection_end.is_some()
        {
            let existing_start = existing_selection_start.unwrap();
            let existing_end = existing_selection_end.unwrap();
            let is_same_position = position.offset == existing_start.offset;
            let is_selection_inverted = existing_start.offset > existing_end.offset;
            let should_swap_edges = is_selection_inverted
                != (position.offset < existing_start.offset)
                || is_same_position;
            if should_swap_edges {
                let local_text_boundary = get_text_boundary(self, app, existing_start);
                debug_assert!(
                    local_text_boundary.boundary_start.offset >= range.start
                        && local_text_boundary.boundary_end.offset <= range.end
                );
                self.set_selection_position(
                    app,
                    Some(if is_selection_inverted {
                        local_text_boundary.boundary_start
                    } else {
                        local_text_boundary.boundary_end
                    }),
                    false,
                );
            }
        }
        target_position.unwrap_or(position)
    }

    fn update_selection_start_edge_by_multi_selectable_text_boundary(
        self: Handle<Self>,
        app: &mut App,
        get_text_boundary: TextBoundaryAtPositionInText,
        paragraph_contains_position: bool,
        position: TextPosition,
        existing_selection_start: Option<TextPosition>,
        existing_selection_end: Option<TextPosition>,
    ) -> Option<SelectionResult> {
        let is_end = false;
        let range = app.get(self).range;
        let full_text = app.get(self).full_text.clone();
        if app.get(self).selectable_contains_origin_text_boundary
            && existing_selection_start.is_some()
            && existing_selection_end.is_some()
        {
            let existing_start = existing_selection_start.unwrap();
            let existing_end = existing_selection_end.unwrap();
            let forward_selection = existing_end.offset >= existing_start.offset;
            if paragraph_contains_position {
                let boundary_at_position = get_text_boundary(self, app, position, &full_text);
                let origin_text_boundary = get_text_boundary(
                    self,
                    app,
                    if forward_selection {
                        TextPosition::with_affinity(existing_end.offset - 1, existing_end.affinity)
                    } else {
                        existing_end
                    },
                    &full_text,
                );
                let pivot_offset = if forward_selection {
                    origin_text_boundary.boundary_end.offset
                } else {
                    origin_text_boundary.boundary_start.offset
                };
                let should_swap_edges = !forward_selection != (position.offset > pivot_offset);
                let target_position = if position.offset < pivot_offset {
                    boundary_at_position.boundary_start
                } else if position.offset > pivot_offset {
                    boundary_at_position.boundary_end
                } else {
                    if forward_selection {
                        existing_start
                    } else {
                        existing_end
                    }
                };
                if should_swap_edges {
                    let clamped = self.clamp_text_position(
                        app,
                        if forward_selection {
                            origin_text_boundary.boundary_start
                        } else {
                            origin_text_boundary.boundary_end
                        },
                    );
                    self.set_selection_position(app, Some(clamped), true);
                }
                let clamped = self.clamp_text_position(app, target_position);
                self.set_selection_position(app, Some(clamped), is_end);
                let final_selection_is_forward = app.get(self).text_selection_end.unwrap().offset
                    >= app.get(self).text_selection_start.unwrap().offset;
                if boundary_at_position.boundary_start.offset > range.end
                    && boundary_at_position.boundary_end.offset > range.end
                {
                    return Some(SelectionResult::Next);
                }
                if boundary_at_position.boundary_start.offset < range.start
                    && boundary_at_position.boundary_end.offset < range.start
                {
                    return Some(SelectionResult::Previous);
                }
                if final_selection_is_forward {
                    if boundary_at_position.boundary_start.offset
                        >= origin_text_boundary.boundary_start.offset
                    {
                        return Some(SelectionResult::End);
                    }
                    if boundary_at_position.boundary_start.offset
                        < origin_text_boundary.boundary_start.offset
                    {
                        return Some(SelectionResult::Previous);
                    }
                } else {
                    if boundary_at_position.boundary_end.offset
                        <= origin_text_boundary.boundary_end.offset
                    {
                        return Some(SelectionResult::End);
                    }
                    if boundary_at_position.boundary_end.offset
                        > origin_text_boundary.boundary_end.offset
                    {
                        return Some(SelectionResult::Next);
                    }
                }
            } else {
                let clamped_position = self.clamp_text_position(app, position);
                let origin_text_boundary = get_text_boundary(
                    self,
                    app,
                    if forward_selection {
                        TextPosition::with_affinity(existing_end.offset - 1, existing_end.affinity)
                    } else {
                        existing_end
                    },
                    &full_text,
                );
                if forward_selection && clamped_position.offset == range.start {
                    self.set_selection_position(app, Some(clamped_position), is_end);
                    return Some(SelectionResult::Previous);
                }
                if !forward_selection && clamped_position.offset == range.end {
                    self.set_selection_position(app, Some(clamped_position), is_end);
                    return Some(SelectionResult::Next);
                }
                if forward_selection && clamped_position.offset == range.end {
                    let clamped =
                        self.clamp_text_position(app, origin_text_boundary.boundary_start);
                    self.set_selection_position(app, Some(clamped), true);
                    self.set_selection_position(app, Some(clamped_position), is_end);
                    return Some(SelectionResult::Next);
                }
                if !forward_selection && clamped_position.offset == range.start {
                    let clamped = self.clamp_text_position(app, origin_text_boundary.boundary_end);
                    self.set_selection_position(app, Some(clamped), true);
                    self.set_selection_position(app, Some(clamped_position), is_end);
                    return Some(SelectionResult::Previous);
                }
            }
        } else {
            let position_on_placeholder = app
                .get(self)
                .paragraph
                .get_word_boundary(app, position)
                .text_inside(&full_text)
                == "\u{FFFC}";
            if !paragraph_contains_position || position_on_placeholder {
                return None;
            }
            if let Some(existing_end) = existing_selection_end {
                let boundary_at_position = get_text_boundary(self, app, position, &full_text);
                let backward_selection = (existing_selection_start.is_none()
                    && existing_end.offset == range.start)
                    || (existing_selection_start == existing_selection_end
                        && existing_end.offset == range.start)
                    || (existing_selection_start.is_some()
                        && existing_selection_start.unwrap().offset > existing_end.offset);
                if boundary_at_position.boundary_start.offset < range.start
                    && boundary_at_position.boundary_end.offset < range.start
                {
                    self.set_selection_position(app, Some(TextPosition::new(range.start)), is_end);
                    return Some(SelectionResult::Previous);
                }
                if boundary_at_position.boundary_start.offset > range.end
                    && boundary_at_position.boundary_end.offset > range.end
                {
                    self.set_selection_position(app, Some(TextPosition::new(range.end)), is_end);
                    return Some(SelectionResult::Next);
                }
                if backward_selection {
                    if boundary_at_position.boundary_end.offset <= range.end {
                        let clamped =
                            self.clamp_text_position(app, boundary_at_position.boundary_end);
                        self.set_selection_position(app, Some(clamped), is_end);
                        return Some(SelectionResult::End);
                    }
                    if boundary_at_position.boundary_end.offset > range.end {
                        self.set_selection_position(
                            app,
                            Some(TextPosition::new(range.end)),
                            is_end,
                        );
                        return Some(SelectionResult::Next);
                    }
                } else {
                    let clamped =
                        self.clamp_text_position(app, boundary_at_position.boundary_start);
                    self.set_selection_position(app, Some(clamped), is_end);
                    if boundary_at_position.boundary_start.offset < range.start {
                        return Some(SelectionResult::Previous);
                    }
                    if boundary_at_position.boundary_start.offset >= range.start {
                        return Some(SelectionResult::End);
                    }
                }
            }
        }
        None
    }

    fn update_selection_end_edge_by_multi_selectable_text_boundary(
        self: Handle<Self>,
        app: &mut App,
        get_text_boundary: TextBoundaryAtPositionInText,
        paragraph_contains_position: bool,
        position: TextPosition,
        existing_selection_start: Option<TextPosition>,
        existing_selection_end: Option<TextPosition>,
    ) -> Option<SelectionResult> {
        let is_end = true;
        let range = app.get(self).range;
        let full_text = app.get(self).full_text.clone();
        if app.get(self).selectable_contains_origin_text_boundary
            && existing_selection_start.is_some()
            && existing_selection_end.is_some()
        {
            let existing_start = existing_selection_start.unwrap();
            let existing_end = existing_selection_end.unwrap();
            let forward_selection = existing_end.offset >= existing_start.offset;
            if paragraph_contains_position {
                let boundary_at_position = get_text_boundary(self, app, position, &full_text);
                let origin_text_boundary = get_text_boundary(
                    self,
                    app,
                    if forward_selection {
                        existing_start
                    } else {
                        TextPosition::with_affinity(
                            existing_start.offset - 1,
                            existing_start.affinity,
                        )
                    },
                    &full_text,
                );
                let pivot_offset = if forward_selection {
                    origin_text_boundary.boundary_start.offset
                } else {
                    origin_text_boundary.boundary_end.offset
                };
                let should_swap_edges = !forward_selection != (position.offset < pivot_offset);
                let target_position = if position.offset < pivot_offset {
                    boundary_at_position.boundary_start
                } else if position.offset > pivot_offset {
                    boundary_at_position.boundary_end
                } else {
                    if forward_selection {
                        existing_end
                    } else {
                        existing_start
                    }
                };
                if should_swap_edges {
                    let clamped = self.clamp_text_position(
                        app,
                        if forward_selection {
                            origin_text_boundary.boundary_end
                        } else {
                            origin_text_boundary.boundary_start
                        },
                    );
                    self.set_selection_position(app, Some(clamped), false);
                }
                let clamped = self.clamp_text_position(app, target_position);
                self.set_selection_position(app, Some(clamped), is_end);
                let final_selection_is_forward = app.get(self).text_selection_end.unwrap().offset
                    >= app.get(self).text_selection_start.unwrap().offset;
                if boundary_at_position.boundary_start.offset > range.end
                    && boundary_at_position.boundary_end.offset > range.end
                {
                    return Some(SelectionResult::Next);
                }
                if boundary_at_position.boundary_start.offset < range.start
                    && boundary_at_position.boundary_end.offset < range.start
                {
                    return Some(SelectionResult::Previous);
                }
                if final_selection_is_forward {
                    if boundary_at_position.boundary_end.offset
                        <= origin_text_boundary.boundary_end.offset
                    {
                        return Some(SelectionResult::End);
                    }
                    if boundary_at_position.boundary_end.offset
                        > origin_text_boundary.boundary_end.offset
                    {
                        return Some(SelectionResult::Next);
                    }
                } else {
                    if boundary_at_position.boundary_start.offset
                        >= origin_text_boundary.boundary_start.offset
                    {
                        return Some(SelectionResult::End);
                    }
                    if boundary_at_position.boundary_start.offset
                        < origin_text_boundary.boundary_start.offset
                    {
                        return Some(SelectionResult::Previous);
                    }
                }
            } else {
                let clamped_position = self.clamp_text_position(app, position);
                let origin_text_boundary = get_text_boundary(
                    self,
                    app,
                    if forward_selection {
                        existing_start
                    } else {
                        TextPosition::with_affinity(
                            existing_start.offset - 1,
                            existing_start.affinity,
                        )
                    },
                    &full_text,
                );
                if forward_selection && clamped_position.offset == range.start {
                    let clamped = self.clamp_text_position(app, origin_text_boundary.boundary_end);
                    self.set_selection_position(app, Some(clamped), false);
                    self.set_selection_position(app, Some(clamped_position), is_end);
                    return Some(SelectionResult::Previous);
                }
                if !forward_selection && clamped_position.offset == range.end {
                    let clamped =
                        self.clamp_text_position(app, origin_text_boundary.boundary_start);
                    self.set_selection_position(app, Some(clamped), false);
                    self.set_selection_position(app, Some(clamped_position), is_end);
                    return Some(SelectionResult::Next);
                }
                if forward_selection && clamped_position.offset == range.end {
                    self.set_selection_position(app, Some(clamped_position), is_end);
                    return Some(SelectionResult::Next);
                }
                if !forward_selection && clamped_position.offset == range.start {
                    self.set_selection_position(app, Some(clamped_position), is_end);
                    return Some(SelectionResult::Previous);
                }
            }
        } else {
            let position_on_placeholder = app
                .get(self)
                .paragraph
                .get_word_boundary(app, position)
                .text_inside(&full_text)
                == "\u{FFFC}";
            if !paragraph_contains_position || position_on_placeholder {
                return None;
            }
            if let Some(existing_start) = existing_selection_start {
                let boundary_at_position = get_text_boundary(self, app, position, &full_text);
                let backward_selection = (existing_selection_end.is_none()
                    && existing_start.offset == range.end)
                    || (existing_selection_start == existing_selection_end
                        && existing_start.offset == range.end)
                    || (existing_selection_end.is_some()
                        && existing_start.offset > existing_selection_end.unwrap().offset);
                if boundary_at_position.boundary_start.offset < range.start
                    && boundary_at_position.boundary_end.offset < range.start
                {
                    self.set_selection_position(app, Some(TextPosition::new(range.start)), is_end);
                    return Some(SelectionResult::Previous);
                }
                if boundary_at_position.boundary_start.offset > range.end
                    && boundary_at_position.boundary_end.offset > range.end
                {
                    self.set_selection_position(app, Some(TextPosition::new(range.end)), is_end);
                    return Some(SelectionResult::Next);
                }
                if backward_selection {
                    let clamped =
                        self.clamp_text_position(app, boundary_at_position.boundary_start);
                    self.set_selection_position(app, Some(clamped), is_end);
                    if boundary_at_position.boundary_start.offset < range.start {
                        return Some(SelectionResult::Previous);
                    }
                    if boundary_at_position.boundary_start.offset >= range.start {
                        return Some(SelectionResult::End);
                    }
                } else {
                    if boundary_at_position.boundary_end.offset <= range.end {
                        let clamped =
                            self.clamp_text_position(app, boundary_at_position.boundary_end);
                        self.set_selection_position(app, Some(clamped), is_end);
                        return Some(SelectionResult::End);
                    }
                    if boundary_at_position.boundary_end.offset > range.end {
                        self.set_selection_position(
                            app,
                            Some(TextPosition::new(range.end)),
                            is_end,
                        );
                        return Some(SelectionResult::Next);
                    }
                }
            }
        }
        None
    }

    fn update_selection_edge_by_multi_selectable_text_boundary(
        self: Handle<Self>,
        app: &mut App,
        global_position: Offset,
        is_end: bool,
        get_text_boundary: TextBoundaryAtPositionInText,
    ) -> SelectionResult {
        let existing_selection_start = app.get(self).text_selection_start;
        let existing_selection_end = app.get(self).text_selection_end;
        self.set_selection_position(app, None, is_end);
        let local_position = self.local_position(app, global_position);
        let rect = self.rect(app);
        let range = app.get(self).range;
        if rect.is_empty() {
            let result = SelectionUtils::get_result_based_on_rect(rect, local_position);
            let position = if result == SelectionResult::Next {
                TextPosition::new(range.end)
            } else {
                TextPosition::with_affinity(range.start, TextAffinity::Upstream)
            };
            self.set_selection_position(app, Some(position), is_end);
            return result;
        }
        let paragraph = app.get(self).paragraph;
        let direction = paragraph.text_direction(app);
        let adjusted_offset = SelectionUtils::adjust_drag_offset(rect, local_position, direction);
        let paint_bounds = paragraph.as_object().paint_bounds(app);
        let adjusted_offset_relative_to_paragraph =
            SelectionUtils::adjust_drag_offset(paint_bounds, local_position, direction);
        let position = paragraph.get_position_for_offset(app, adjusted_offset);
        let position_in_full_text =
            paragraph.get_position_for_offset(app, adjusted_offset_relative_to_paragraph);
        let paragraph_contains_position = paint_bounds.contains(local_position);
        let result = if self.is_placeholder(app) {
            if is_end {
                self.update_selection_end_edge_at_placeholder(
                    app,
                    get_text_boundary,
                    global_position,
                    paragraph_contains_position,
                    position_in_full_text,
                    existing_selection_start,
                    existing_selection_end,
                )
            } else {
                self.update_selection_start_edge_at_placeholder(
                    app,
                    get_text_boundary,
                    global_position,
                    paragraph_contains_position,
                    position_in_full_text,
                    existing_selection_start,
                    existing_selection_end,
                )
            }
        } else if is_end {
            self.update_selection_end_edge_by_multi_selectable_text_boundary(
                app,
                get_text_boundary,
                paragraph_contains_position,
                position_in_full_text,
                existing_selection_start,
                existing_selection_end,
            )
        } else {
            self.update_selection_start_edge_by_multi_selectable_text_boundary(
                app,
                get_text_boundary,
                paragraph_contains_position,
                position_in_full_text,
                existing_selection_start,
                existing_selection_end,
            )
        };
        if let Some(result) = result {
            return result;
        }
        let mut text_boundary = if self.bounding_boxes_contain(app, local_position) {
            Some(self.get_clamped_paragraph_boundary_at_position(app, position))
        } else {
            None
        };
        if let Some(boundary) = text_boundary {
            if (boundary.boundary_start.offset < range.start
                && boundary.boundary_end.offset <= range.start)
                || (boundary.boundary_start.offset >= range.end
                    && boundary.boundary_end.offset > range.end)
            {
                text_boundary = None;
            }
        }
        let edge_position = if is_end {
            self.update_selection_end_edge_by_text_boundary(
                app,
                text_boundary,
                Self::get_clamped_paragraph_boundary_at_position,
                position,
                existing_selection_start,
                existing_selection_end,
            )
        } else {
            self.update_selection_start_edge_by_text_boundary(
                app,
                text_boundary,
                Self::get_clamped_paragraph_boundary_at_position,
                position,
                existing_selection_start,
                existing_selection_end,
            )
        };
        let target_position = self.clamp_text_position(app, edge_position);
        self.set_selection_position(app, Some(target_position), is_end);
        if target_position.offset == range.end {
            return SelectionResult::Next;
        }
        if target_position.offset == range.start {
            return SelectionResult::Previous;
        }
        SelectionUtils::get_result_based_on_rect(rect, local_position)
    }

    fn update_selection_start_edge_at_placeholder(
        self: Handle<Self>,
        app: &mut App,
        get_text_boundary: TextBoundaryAtPositionInText,
        global_position: Offset,
        paragraph_contains_position: bool,
        position: TextPosition,
        existing_selection_start: Option<TextPosition>,
        existing_selection_end: Option<TextPosition>,
    ) -> Option<SelectionResult> {
        let is_end = false;
        let range = app.get(self).range;
        if app.get(self).selectable_contains_origin_text_boundary
            && existing_selection_start.is_some()
            && existing_selection_end.is_some()
        {
            let existing_start = existing_selection_start.unwrap();
            let existing_end = existing_selection_end.unwrap();
            let forward_selection = existing_end.offset >= existing_start.offset;
            let origin_paragraph = self.get_origin_paragraph(app);
            if origin_paragraph == app.get(self).paragraph {
                return self.update_selection_start_edge_by_multi_selectable_text_boundary(
                    app,
                    get_text_boundary,
                    paragraph_contains_position,
                    position,
                    existing_selection_start,
                    existing_selection_end,
                );
            }
            let origin_local = global_to_paragraph(app, origin_paragraph, global_position);
            let within_origin = origin_paragraph
                .as_object()
                .paint_bounds(app)
                .contains(origin_local);
            let origin_position = origin_paragraph.get_position_for_offset(app, origin_local);
            if within_origin {
                let origin_text = origin_paragraph.text(app).to_plain_text(false, true);
                let boundary = get_text_boundary(self, app, origin_position, &origin_text);
                let origin_placeholder_position =
                    self.get_position_in_paragraph(app, origin_paragraph);
                let origin_boundary =
                    get_text_boundary(self, app, origin_placeholder_position, &origin_text);
                let pivot = if forward_selection {
                    origin_boundary.boundary_end.offset
                } else {
                    origin_boundary.boundary_start.offset
                };
                let swap = !forward_selection != (origin_position.offset > pivot);
                let target = if origin_position.offset < pivot {
                    boundary.boundary_start
                } else if origin_position.offset > pivot {
                    boundary.boundary_end
                } else {
                    existing_start
                };
                if swap {
                    self.set_selection_position(app, Some(existing_start), true);
                }
                let target = self.clamp_text_position(app, target);
                self.set_selection_position(app, Some(target), is_end);
                let forward = app.get(self).text_selection_end.unwrap().offset
                    >= app.get(self).text_selection_start.unwrap().offset;
                let placeholder = self.get_position_in_paragraph(app, origin_paragraph);
                let placeholder_range = TextRange::new(placeholder.offset, placeholder.offset + 1);
                if boundary.boundary_start.offset > placeholder_range.end
                    && boundary.boundary_end.offset > placeholder_range.end
                {
                    return Some(SelectionResult::Next);
                }
                if boundary.boundary_start.offset < placeholder_range.start
                    && boundary.boundary_end.offset < placeholder_range.start
                {
                    return Some(SelectionResult::Previous);
                }
                if forward {
                    if boundary.boundary_end.offset <= origin_boundary.boundary_end.offset {
                        return Some(SelectionResult::End);
                    }
                    if boundary.boundary_end.offset > origin_boundary.boundary_end.offset {
                        return Some(SelectionResult::Next);
                    }
                } else {
                    if boundary.boundary_start.offset >= origin_boundary.boundary_start.offset {
                        return Some(SelectionResult::End);
                    }
                    if boundary.boundary_start.offset < origin_boundary.boundary_start.offset {
                        return Some(SelectionResult::Previous);
                    }
                }
            } else {
                let direction = app.get(self).paragraph.text_direction(app);
                let adjusted = SelectionUtils::adjust_drag_offset(
                    origin_paragraph.as_object().paint_bounds(app),
                    origin_local,
                    direction,
                );
                let adjusted_position = origin_paragraph.get_position_for_offset(app, adjusted);
                let placeholder = self.get_position_in_paragraph(app, origin_paragraph);
                let placeholder_range = TextRange::new(placeholder.offset, placeholder.offset + 1);
                if forward_selection && adjusted_position.offset <= placeholder_range.start {
                    self.set_selection_position(app, Some(TextPosition::new(range.start)), is_end);
                    return Some(SelectionResult::Previous);
                }
                if !forward_selection && adjusted_position.offset >= placeholder_range.end {
                    self.set_selection_position(app, Some(TextPosition::new(range.end)), is_end);
                    return Some(SelectionResult::Next);
                }
                if forward_selection && adjusted_position.offset >= placeholder_range.end {
                    self.set_selection_position(app, Some(existing_start), true);
                    self.set_selection_position(app, Some(TextPosition::new(range.end)), is_end);
                    return Some(SelectionResult::Next);
                }
                if !forward_selection && adjusted_position.offset <= placeholder_range.start {
                    self.set_selection_position(app, Some(existing_start), true);
                    self.set_selection_position(app, Some(TextPosition::new(range.start)), is_end);
                    return Some(SelectionResult::Previous);
                }
            }
        } else {
            if paragraph_contains_position {
                return self.update_selection_start_edge_by_multi_selectable_text_boundary(
                    app,
                    get_text_boundary,
                    paragraph_contains_position,
                    position,
                    existing_selection_start,
                    existing_selection_end,
                );
            }
            if existing_selection_end.is_some() {
                let (target_paragraph, target_local) =
                    self.get_paragraph_containing_position(app, global_position)?;
                let target_position = target_paragraph.get_position_for_offset(app, target_local);
                let target_text = target_paragraph.text(app).to_plain_text(false, true);
                if target_paragraph
                    .get_word_boundary(app, target_position)
                    .text_inside(&target_text)
                    == "\u{FFFC}"
                {
                    return None;
                }
                let existing_end = existing_selection_end.unwrap();
                let backward = (existing_selection_start.is_none()
                    && existing_end.offset == range.start)
                    || (existing_selection_start == existing_selection_end
                        && existing_end.offset == range.start)
                    || (existing_selection_start.is_some()
                        && existing_selection_start.unwrap().offset > existing_end.offset);
                let boundary = get_text_boundary(self, app, target_position, &target_text);
                let placeholder = self.get_position_in_paragraph(app, target_paragraph);
                let placeholder_range = TextRange::new(placeholder.offset, placeholder.offset + 1);
                if boundary.boundary_start.offset < placeholder_range.start
                    && boundary.boundary_end.offset < placeholder_range.start
                {
                    self.set_selection_position(app, Some(TextPosition::new(range.start)), is_end);
                    return Some(SelectionResult::Previous);
                }
                if boundary.boundary_start.offset > placeholder_range.end
                    && boundary.boundary_end.offset > placeholder_range.end
                {
                    self.set_selection_position(app, Some(TextPosition::new(range.end)), is_end);
                    return Some(SelectionResult::Next);
                }
                if backward {
                    if boundary.boundary_end.offset <= placeholder_range.end {
                        self.set_selection_position(
                            app,
                            Some(TextPosition::new(range.end)),
                            is_end,
                        );
                        return Some(SelectionResult::End);
                    }
                    if boundary.boundary_end.offset > placeholder_range.end {
                        self.set_selection_position(
                            app,
                            Some(TextPosition::new(range.end)),
                            is_end,
                        );
                        return Some(SelectionResult::Next);
                    }
                } else {
                    if boundary.boundary_start.offset >= placeholder_range.start {
                        self.set_selection_position(
                            app,
                            Some(TextPosition::new(range.start)),
                            is_end,
                        );
                        return Some(SelectionResult::End);
                    }
                    if boundary.boundary_start.offset < placeholder_range.start {
                        self.set_selection_position(
                            app,
                            Some(TextPosition::new(range.start)),
                            is_end,
                        );
                        return Some(SelectionResult::Previous);
                    }
                }
            }
        }
        None
    }

    fn update_selection_end_edge_at_placeholder(
        self: Handle<Self>,
        app: &mut App,
        get_text_boundary: TextBoundaryAtPositionInText,
        global_position: Offset,
        paragraph_contains_position: bool,
        position: TextPosition,
        existing_selection_start: Option<TextPosition>,
        existing_selection_end: Option<TextPosition>,
    ) -> Option<SelectionResult> {
        let is_end = true;
        let range = app.get(self).range;
        if app.get(self).selectable_contains_origin_text_boundary
            && existing_selection_start.is_some()
            && existing_selection_end.is_some()
        {
            let existing_start = existing_selection_start.unwrap();
            let existing_end = existing_selection_end.unwrap();
            let forward_selection = existing_end.offset >= existing_start.offset;
            let origin_paragraph = self.get_origin_paragraph(app);
            if origin_paragraph == app.get(self).paragraph {
                return self.update_selection_end_edge_by_multi_selectable_text_boundary(
                    app,
                    get_text_boundary,
                    paragraph_contains_position,
                    position,
                    existing_selection_start,
                    existing_selection_end,
                );
            }
            let origin_local = global_to_paragraph(app, origin_paragraph, global_position);
            let within_origin = origin_paragraph
                .as_object()
                .paint_bounds(app)
                .contains(origin_local);
            let origin_position = origin_paragraph.get_position_for_offset(app, origin_local);
            if within_origin {
                let origin_text = origin_paragraph.text(app).to_plain_text(false, true);
                let boundary = get_text_boundary(self, app, origin_position, &origin_text);
                let origin_placeholder_position =
                    self.get_position_in_paragraph(app, origin_paragraph);
                let origin_boundary =
                    get_text_boundary(self, app, origin_placeholder_position, &origin_text);
                let pivot = if forward_selection {
                    origin_boundary.boundary_start.offset
                } else {
                    origin_boundary.boundary_end.offset
                };
                let swap = !forward_selection != (origin_position.offset < pivot);
                let target = if origin_position.offset < pivot {
                    boundary.boundary_start
                } else if origin_position.offset > pivot {
                    boundary.boundary_end
                } else {
                    existing_end
                };
                if swap {
                    self.set_selection_position(app, Some(existing_end), false);
                }
                let target = self.clamp_text_position(app, target);
                self.set_selection_position(app, Some(target), is_end);
                let forward = app.get(self).text_selection_end.unwrap().offset
                    >= app.get(self).text_selection_start.unwrap().offset;
                let placeholder = self.get_position_in_paragraph(app, origin_paragraph);
                let placeholder_range = TextRange::new(placeholder.offset, placeholder.offset + 1);
                if boundary.boundary_start.offset > placeholder_range.end
                    && boundary.boundary_end.offset > placeholder_range.end
                {
                    return Some(SelectionResult::Next);
                }
                if boundary.boundary_start.offset < placeholder_range.start
                    && boundary.boundary_end.offset < placeholder_range.start
                {
                    return Some(SelectionResult::Previous);
                }
                if forward {
                    if boundary.boundary_end.offset <= origin_boundary.boundary_end.offset {
                        return Some(SelectionResult::End);
                    }
                    if boundary.boundary_end.offset > origin_boundary.boundary_end.offset {
                        return Some(SelectionResult::Next);
                    }
                } else {
                    if boundary.boundary_start.offset >= origin_boundary.boundary_start.offset {
                        return Some(SelectionResult::End);
                    }
                    if boundary.boundary_start.offset < origin_boundary.boundary_start.offset {
                        return Some(SelectionResult::Previous);
                    }
                }
            } else {
                let direction = app.get(self).paragraph.text_direction(app);
                let adjusted = SelectionUtils::adjust_drag_offset(
                    origin_paragraph.as_object().paint_bounds(app),
                    origin_local,
                    direction,
                );
                let adjusted_position = origin_paragraph.get_position_for_offset(app, adjusted);
                let placeholder = self.get_position_in_paragraph(app, origin_paragraph);
                let placeholder_range = TextRange::new(placeholder.offset, placeholder.offset + 1);
                if forward_selection && adjusted_position.offset <= placeholder_range.start {
                    self.set_selection_position(app, Some(existing_end), false);
                    self.set_selection_position(app, Some(TextPosition::new(range.start)), is_end);
                    return Some(SelectionResult::Previous);
                }
                if !forward_selection && adjusted_position.offset >= placeholder_range.end {
                    self.set_selection_position(app, Some(existing_end), false);
                    self.set_selection_position(app, Some(TextPosition::new(range.end)), is_end);
                    return Some(SelectionResult::Next);
                }
                if forward_selection && adjusted_position.offset >= placeholder_range.end {
                    self.set_selection_position(app, Some(TextPosition::new(range.end)), is_end);
                    return Some(SelectionResult::Next);
                }
                if !forward_selection && adjusted_position.offset <= placeholder_range.start {
                    self.set_selection_position(app, Some(TextPosition::new(range.start)), is_end);
                    return Some(SelectionResult::Previous);
                }
            }
        } else {
            if paragraph_contains_position {
                return self.update_selection_end_edge_by_multi_selectable_text_boundary(
                    app,
                    get_text_boundary,
                    paragraph_contains_position,
                    position,
                    existing_selection_start,
                    existing_selection_end,
                );
            }
            if existing_selection_start.is_some() {
                let (target_paragraph, target_local) =
                    self.get_paragraph_containing_position(app, global_position)?;
                let target_position = target_paragraph.get_position_for_offset(app, target_local);
                let target_text = target_paragraph.text(app).to_plain_text(false, true);
                if target_paragraph
                    .get_word_boundary(app, target_position)
                    .text_inside(&target_text)
                    == "\u{FFFC}"
                {
                    return None;
                }
                let existing_start = existing_selection_start.unwrap();
                let backward = (existing_selection_end.is_none()
                    && existing_start.offset == range.end)
                    || (existing_selection_start == existing_selection_end
                        && existing_start.offset == range.end)
                    || (existing_selection_end.is_some()
                        && existing_start.offset > existing_selection_end.unwrap().offset);
                let boundary = get_text_boundary(self, app, target_position, &target_text);
                let placeholder = self.get_position_in_paragraph(app, target_paragraph);
                let placeholder_range = TextRange::new(placeholder.offset, placeholder.offset + 1);
                if boundary.boundary_start.offset < placeholder_range.start
                    && boundary.boundary_end.offset < placeholder_range.start
                {
                    self.set_selection_position(app, Some(TextPosition::new(range.start)), is_end);
                    return Some(SelectionResult::Previous);
                }
                if boundary.boundary_start.offset > placeholder_range.end
                    && boundary.boundary_end.offset > placeholder_range.end
                {
                    self.set_selection_position(app, Some(TextPosition::new(range.end)), is_end);
                    return Some(SelectionResult::Next);
                }
                if backward {
                    if boundary.boundary_start.offset >= placeholder_range.start {
                        self.set_selection_position(
                            app,
                            Some(TextPosition::new(range.start)),
                            is_end,
                        );
                        return Some(SelectionResult::End);
                    }
                    if boundary.boundary_start.offset < placeholder_range.start {
                        self.set_selection_position(
                            app,
                            Some(TextPosition::new(range.start)),
                            is_end,
                        );
                        return Some(SelectionResult::Previous);
                    }
                } else {
                    if boundary.boundary_end.offset <= placeholder_range.end {
                        self.set_selection_position(
                            app,
                            Some(TextPosition::new(range.end)),
                            is_end,
                        );
                        return Some(SelectionResult::End);
                    }
                    if boundary.boundary_end.offset > placeholder_range.end {
                        self.set_selection_position(
                            app,
                            Some(TextPosition::new(range.end)),
                            is_end,
                        );
                        return Some(SelectionResult::Next);
                    }
                }
            }
        }
        None
    }
    fn handle_granularly_extend_selection(
        self: Handle<Self>,
        app: &mut App,
        forward: bool,
        is_extent: bool,
        granularity: TextGranularity,
    ) -> SelectionResult {
        let range = app.get(self).range;
        if app.get(self).text_selection_end.is_none() {
            app.get_mut(self).text_selection_end = Some(if forward {
                TextPosition::new(range.start)
            } else {
                TextPosition::with_affinity(range.end, TextAffinity::Upstream)
            });
        }
        if app.get(self).text_selection_start.is_none() {
            let end = app.get(self).text_selection_end.unwrap();
            app.get_mut(self).text_selection_start = Some(end);
        }
        let targeted_edge = if is_extent {
            app.get(self).text_selection_end.unwrap()
        } else {
            app.get(self).text_selection_start.unwrap()
        };
        if forward && targeted_edge.offset == range.end {
            return SelectionResult::Next;
        }
        if !forward && targeted_edge.offset == range.start {
            return SelectionResult::Previous;
        }
        let (new_position, result) = match granularity {
            TextGranularity::Character => {
                let text = range.text_inside(&app.get(self).full_text).to_owned();
                (
                    self.move_beyond_text_boundary_at_direction(
                        app,
                        targeted_edge,
                        forward,
                        &CharacterBoundary::new(text),
                    ),
                    SelectionResult::End,
                )
            }
            TextGranularity::Word => {
                let paragraph = app.get(self).paragraph;
                let text = paragraph.text(app);
                let boundary = WordBoundary::new(text, FragmentTextLayout(self));
                (
                    self.move_beyond_text_boundary_at_direction(
                        app,
                        targeted_edge,
                        forward,
                        &*boundary.move_by_word_boundary(),
                    ),
                    SelectionResult::End,
                )
            }
            TextGranularity::Paragraph => {
                let text = range.text_inside(&app.get(self).full_text).to_owned();
                (
                    self.move_beyond_text_boundary_at_direction(
                        app,
                        targeted_edge,
                        forward,
                        &ParagraphBoundary::new(&text),
                    ),
                    SelectionResult::End,
                )
            }
            TextGranularity::Line => {
                let boundary = LineBoundary::new(FragmentTextLayout(self));
                (
                    self.move_to_text_boundary_at_direction(app, targeted_edge, forward, &boundary),
                    SelectionResult::End,
                )
            }
            TextGranularity::Document => {
                let text = range.text_inside(&app.get(self).full_text).to_owned();
                let position = self.move_beyond_text_boundary_at_direction(
                    app,
                    targeted_edge,
                    forward,
                    &DocumentBoundary::new(text),
                );
                let result = if forward && position.offset == range.end {
                    SelectionResult::Next
                } else if !forward && position.offset == range.start {
                    SelectionResult::Previous
                } else {
                    SelectionResult::End
                };
                (position, result)
            }
        };
        if is_extent {
            app.get_mut(self).text_selection_end = Some(new_position);
        } else {
            app.get_mut(self).text_selection_start = Some(new_position);
        }
        result
    }

    fn move_beyond_text_boundary_at_direction(
        self: Handle<Self>,
        app: &mut App,
        end: TextPosition,
        forward: bool,
        text_boundary: &dyn TextBoundary,
    ) -> TextPosition {
        let range = app.get(self).range;
        let new_offset = if forward {
            text_boundary
                .get_trailing_text_boundary_at(app, end.offset)
                .unwrap_or(range.end)
        } else {
            text_boundary
                .get_leading_text_boundary_at(app, end.offset - 1)
                .unwrap_or(range.start)
        };
        TextPosition::new(new_offset)
    }

    fn move_to_text_boundary_at_direction(
        self: Handle<Self>,
        app: &mut App,
        end: TextPosition,
        forward: bool,
        text_boundary: &dyn TextBoundary,
    ) -> TextPosition {
        let range = app.get(self).range;
        debug_assert!(end.offset >= 0);
        let caret_offset = match end.affinity {
            TextAffinity::Upstream => {
                if end.offset < 1 && !forward {
                    debug_assert_eq!(end.offset, 0);
                    return TextPosition::new(0);
                }
                let full_text = app.get(self).full_text.clone();
                let character_boundary = CharacterBoundary::new(full_text);
                character_boundary
                    .get_leading_text_boundary_at(app, range.start + end.offset)
                    .unwrap_or(range.start)
                    .max(0)
                    - 1
            }
            TextAffinity::Downstream => end.offset,
        };
        let offset = if forward {
            text_boundary
                .get_trailing_text_boundary_at(app, caret_offset)
                .unwrap_or(range.end)
        } else {
            text_boundary
                .get_leading_text_boundary_at(app, caret_offset)
                .unwrap_or(range.start)
        };
        TextPosition::new(offset)
    }

    fn handle_vertical_movement(
        self: Handle<Self>,
        app: &mut App,
        position: TextPosition,
        horizontal_baseline_in_paragraph_coordinates: f64,
        below: bool,
    ) -> (TextPosition, SelectionResult) {
        let range = app.get(self).range;
        let paragraph = app.get(self).paragraph;
        let constraints = paragraph.constraints(app);
        paragraph.layout_text_with_constraints(app, constraints);
        let lines = paragraph.painter_mut(app).compute_line_metrics();
        let offset = paragraph.get_offset_for_caret(app, position, Rect::ZERO);
        let mut current_line = lines.len() as i32 - 1;
        for line_metrics in &lines {
            if line_metrics.baseline > offset.dy() {
                current_line = line_metrics.line_number;
                break;
            }
        }
        let new_position = if below && current_line == lines.len() as i32 - 1 {
            TextPosition::with_affinity(range.end, TextAffinity::Upstream)
        } else if !below && current_line == 0 {
            TextPosition::new(range.start)
        } else {
            let new_line = if below {
                current_line + 1
            } else {
                current_line - 1
            };
            let position = paragraph.get_position_for_offset(
                app,
                Offset::new(
                    horizontal_baseline_in_paragraph_coordinates,
                    lines[new_line as usize].baseline,
                ),
            );
            self.clamp_text_position(app, position)
        };
        let result = if new_position.offset == range.start {
            SelectionResult::Previous
        } else if new_position.offset == range.end {
            SelectionResult::Next
        } else {
            SelectionResult::End
        };
        debug_assert!(result != SelectionResult::Next || below);
        debug_assert!(result != SelectionResult::Previous || !below);
        (new_position, result)
    }

    fn handle_directionally_extend_selection(
        self: Handle<Self>,
        app: &mut App,
        horizontal_baseline: f64,
        is_extent: bool,
        movement: SelectionExtendDirection,
    ) -> SelectionResult {
        let transform = app
            .get(self)
            .paragraph
            .as_object()
            .get_transform_to(app, None);
        let Some(inverse) = transform.invert() else {
            return match movement {
                SelectionExtendDirection::PreviousLine | SelectionExtendDirection::Backward => {
                    SelectionResult::Previous
                }
                SelectionExtendDirection::NextLine | SelectionExtendDirection::Forward => {
                    SelectionResult::Next
                }
            };
        };
        let baseline_in_paragraph_coordinates =
            waft_painting::transform_point(&inverse, Offset::new(horizontal_baseline, 0.0)).dx();
        debug_assert!(!baseline_in_paragraph_coordinates.is_nan());
        let (new_position, result) = match movement {
            SelectionExtendDirection::PreviousLine | SelectionExtendDirection::NextLine => {
                debug_assert!(app.get(self).text_selection_end.is_some());
                debug_assert!(app.get(self).text_selection_start.is_some());
                let targeted_edge = if is_extent {
                    app.get(self).text_selection_end.unwrap()
                } else {
                    app.get(self).text_selection_start.unwrap()
                };
                self.handle_vertical_movement(
                    app,
                    targeted_edge,
                    baseline_in_paragraph_coordinates,
                    movement == SelectionExtendDirection::NextLine,
                )
            }
            SelectionExtendDirection::Forward | SelectionExtendDirection::Backward => {
                if app.get(self).text_selection_end.is_none() {
                    app.get_mut(self).text_selection_end =
                        Some(if movement == SelectionExtendDirection::Forward {
                            TextPosition::new(app.get(self).range.start)
                        } else {
                            TextPosition::with_affinity(
                                app.get(self).range.end,
                                TextAffinity::Upstream,
                            )
                        });
                }
                if app.get(self).text_selection_start.is_none() {
                    let end = app.get(self).text_selection_end.unwrap();
                    app.get_mut(self).text_selection_start = Some(end);
                }
                let targeted_edge = if is_extent {
                    app.get(self).text_selection_end.unwrap()
                } else {
                    app.get(self).text_selection_start.unwrap()
                };
                let edge_offset = app
                    .get(self)
                    .paragraph
                    .get_offset_for_position(app, targeted_edge);
                let line_height = app.get(self).paragraph.preferred_line_height(app);
                let baseline_offset = Offset::new(
                    baseline_in_paragraph_coordinates,
                    edge_offset.dy() - line_height / 2.0,
                );
                let new_position = app
                    .get(self)
                    .paragraph
                    .get_position_for_offset(app, baseline_offset);
                (new_position, SelectionResult::End)
            }
        };
        if is_extent {
            app.get_mut(self).text_selection_end = Some(new_position);
        } else {
            app.get_mut(self).text_selection_start = Some(new_position);
        }
        result
    }

    fn paint_selection(
        self: Handle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let (Some(start), Some(end)) = (
            app.get(self).text_selection_start,
            app.get(self).text_selection_end,
        ) else {
            return;
        };
        let paragraph = app.get(self).paragraph;
        let Some(selection_color) = paragraph.selection_color(app) else {
            return;
        };
        let selection = TextSelection::new(start.offset, end.offset);
        let selection_paint = Paint::from_color(selection_color.into());
        let boxes = paragraph.get_boxes_for_selection(
            app,
            selection,
            BoxHeightStyle::Tight,
            BoxWidthStyle::Tight,
        );
        for text_box in boxes {
            context
                .canvas()
                .draw_rect(text_box.to_rect().shift(offset), &selection_paint);
        }
    }

    fn paint_handles(
        self: Handle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if app.get(self).text_selection_start.is_none()
            || app.get(self).text_selection_end.is_none()
        {
            return;
        }
        let geometry = app.get(self).selection_geometry.clone();
        let start_link = app.get(self).start_handle_layer_link.clone();
        let end_link = app.get(self).end_handle_layer_link.clone();
        if let (Some(link), Some(point)) = (start_link, geometry.start_selection_point) {
            let layer = LeaderLayer::new(app, link);
            layer.set_offset(app, offset + point.local_position);
            context.push_layer(
                app,
                ContainerLayer::as_container_layer(layer),
                |_, _, _| {},
                Offset::ZERO,
                None,
            );
        }
        if let (Some(link), Some(point)) = (end_link, geometry.end_selection_point) {
            let layer = LeaderLayer::new(app, link);
            layer.set_offset(app, offset + point.local_position);
            context.push_layer(
                app,
                ContainerLayer::as_container_layer(layer),
                |_, _, _| {},
                Offset::ZERO,
                None,
            );
        }
    }
}
/// `global_position` in `paragraph`'s coordinates, through the inverse of its transform to the
/// root; a transform that cannot be inverted is used as it is, as Dart's `Matrix4.invert` leaves
/// it.
fn global_to_paragraph(
    app: &App,
    paragraph: RenderHandle<RenderParagraph>,
    global_position: Offset,
) -> Offset {
    let transform = paragraph.as_object().get_transform_to(app, None);
    let inverse = transform.invert().unwrap_or(transform);
    waft_painting::transform_point(&inverse, global_position)
}

impl SelectableFragment {
    fn get_line_at_offset(
        self: Handle<Self>,
        app: &mut App,
        position: TextPosition,
    ) -> TextSelection {
        let range = app.get(self).range;
        let line = app.get(self).paragraph.get_line_at_offset(app, position);
        TextSelection::new(
            line.base_offset.clamp(range.start, range.end),
            line.extent_offset.clamp(range.start, range.end),
        )
    }

    fn bounding_boxes(self: Handle<Self>, app: &mut App) -> Vec<Rect> {
        if let Some(cached) = &app.get(self).cached_bounding_boxes {
            return cached.clone();
        }
        let range = app.get(self).range;
        let paragraph = app.get(self).paragraph;
        let boxes = paragraph.get_boxes_for_selection(
            app,
            TextSelection::new(range.start, range.end),
            BoxHeightStyle::Max,
            BoxWidthStyle::Tight,
        );
        let rects: Vec<Rect> = if boxes.is_empty() {
            let offset = paragraph.get_offset_for_position(app, TextPosition::new(range.start));
            let line_height = paragraph.preferred_line_height(app);
            vec![Rect::from_ltrb(
                offset.dx(),
                offset.dy() - line_height,
                offset.dx(),
                offset.dy(),
            )]
        } else {
            boxes
                .into_iter()
                .map(|text_box| text_box.to_rect())
                .collect()
        };
        app.get_mut(self).cached_bounding_boxes = Some(rects.clone());
        rects
    }
}

/// The fragment as [`TextLayoutMetrics`](waft_services::TextLayoutMetrics): the orphan rule
/// keeps the trait off `Handle<SelectableFragment>`, which names a foreign handle.
#[derive(Clone, Copy)]
struct FragmentTextLayout(Handle<SelectableFragment>);

impl waft_services::TextLayoutMetrics for FragmentTextLayout {
    fn get_line_at_offset(
        self,
        app: &mut App,
        position: TextPosition,
    ) -> TextSelection {
        let range = app.get(self.0).range;
        let line = app.get(self.0).paragraph.get_line_at_offset(app, position);
        TextSelection::new(
            line.base_offset.clamp(range.start, range.end),
            line.extent_offset.clamp(range.start, range.end),
        )
    }

    fn get_word_boundary(self, app: &mut App, position: TextPosition) -> TextRange {
        let paragraph = app.get(self.0).paragraph;
        paragraph.get_word_boundary(app, position)
    }

    fn get_text_position_above(self, app: &mut App, position: TextPosition) -> TextPosition {
        let position = app
            .get(self.0)
            .paragraph
            .get_text_position_above(app, position);
        self.0.clamp_text_position(app, position)
    }

    fn get_text_position_below(self, app: &mut App, position: TextPosition) -> TextPosition {
        let position = app
            .get(self.0)
            .paragraph
            .get_text_position_below(app, position);
        self.0.clamp_text_position(app, position)
    }
}

impl ChangeNotifier for SelectableFragment {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl SelectionHandler for SelectableFragment {
    fn value(self: Handle<Self>, app: &App) -> SelectionGeometry {
        app.get(self).selection_geometry.clone()
    }

    fn push_handle_layers(
        self: Handle<Self>,
        app: &mut App,
        start_handle: Option<Ref<LayerLink>>,
        end_handle: Option<Ref<LayerLink>>,
    ) {
        let paragraph = app.get(self).paragraph;
        if !paragraph.as_object().attached(app) {
            debug_assert!(
                start_handle.is_none() && end_handle.is_none(),
                "Only clean up can be called."
            );
            return;
        }
        let mut needs_paint = false;
        if app.get(self).start_handle_layer_link != start_handle {
            app.get_mut(self).start_handle_layer_link = start_handle;
            needs_paint = true;
        }
        if app.get(self).end_handle_layer_link != end_handle {
            app.get_mut(self).end_handle_layer_link = end_handle;
            needs_paint = true;
        }
        if needs_paint {
            paragraph.mark_needs_paint(app);
        }
    }

    fn selected_content(self: Handle<Self>, app: &App) -> Option<SelectedContent> {
        let this = app.get(self);
        let start = this.text_selection_start?;
        let end = this.text_selection_end?;
        let range = TextRange::new(start.offset.min(end.offset), start.offset.max(end.offset));
        Some(SelectedContent::new(range.text_inside(&this.full_text)))
    }

    fn selection(self: Handle<Self>, app: &App) -> Option<SelectedContentRange> {
        let this = app.get(self);
        Some(SelectedContentRange::new(
            this.text_selection_start?.offset,
            this.text_selection_end?.offset,
        ))
    }

    fn dispatch_selection_event(
        self: Handle<Self>,
        app: &mut App,
        event: &SelectionEvent,
    ) -> SelectionResult {
        let existing_start = app.get(self).text_selection_start;
        let existing_end = app.get(self).text_selection_end;
        let result = match event {
            SelectionEvent::StartEdgeUpdate {
                global_position,
                granularity,
            }
            | SelectionEvent::EndEdgeUpdate {
                global_position,
                granularity,
            } => {
                let is_end = matches!(event, SelectionEvent::EndEdgeUpdate { .. });
                match granularity {
                    TextGranularity::Character => {
                        self.update_selection_edge(app, *global_position, is_end)
                    }
                    TextGranularity::Word => self.update_selection_edge_by_text_boundary(
                        app,
                        *global_position,
                        is_end,
                        Self::get_word_boundary_at_position,
                    ),
                    TextGranularity::Paragraph => self
                        .update_selection_edge_by_multi_selectable_text_boundary(
                            app,
                            *global_position,
                            is_end,
                            Self::get_paragraph_boundary_at_position,
                        ),
                    TextGranularity::Document | TextGranularity::Line => {
                        debug_assert!(
                            false,
                            "Moving the selection edge by line or document is not supported."
                        );
                        unreachable!()
                    }
                }
            }
            SelectionEvent::Clear => self.handle_clear_selection(app),
            SelectionEvent::SelectAll => self.handle_select_all(app),
            SelectionEvent::SelectWord { global_position } => {
                self.handle_select_word(app, *global_position)
            }
            SelectionEvent::SelectParagraph {
                global_position,
                absorb,
            } => {
                if *absorb {
                    self.handle_select_all(app);
                    app.get_mut(self).selectable_contains_origin_text_boundary = true;
                    SelectionResult::Next
                } else {
                    self.handle_select_paragraph(app, *global_position)
                }
            }
            SelectionEvent::GranularlyExtendSelection {
                forward,
                is_end,
                granularity,
            } => self.handle_granularly_extend_selection(app, *forward, *is_end, *granularity),
            SelectionEvent::DirectionallyExtendSelection {
                dx,
                is_end,
                direction,
            } => self.handle_directionally_extend_selection(app, *dx, *is_end, *direction),
        };
        if existing_start != app.get(self).text_selection_start
            || existing_end != app.get(self).text_selection_end
        {
            self.did_change_selection(app);
        }
        result
    }

    fn content_length(self: Handle<Self>, app: &App) -> usize {
        let range = app.get(self).range;
        (range.end - range.start) as usize
    }
}

impl Selectable for SelectableFragment {
    fn get_transform_to(
        self: Handle<Self>,
        app: &App,
        ancestor: Option<AnyRenderObject>,
    ) -> Matrix4 {
        app.get(self)
            .paragraph
            .as_object()
            .get_transform_to(app, ancestor)
    }

    fn size(self: Handle<Self>, app: &mut App) -> Size {
        self.rect(app).size()
    }

    fn bounding_boxes(self: Handle<Self>, app: &mut App) -> Vec<Rect> {
        SelectableFragment::bounding_boxes(self, app)
    }

    fn dispose_selectable(self: Handle<Self>, app: &mut App) {
        ChangeNotifierData::dispose(app, self);
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::Color;
    use waft_foundation::AppCell;
    use waft_gestures::HitTestResult;
    use waft_painting::{AlignmentGeometry, TextSpan, TextStyle};

    use super::*;
    use crate::proxy_box::RenderRepaintBoundary;
    use crate::testing::{EnginePhase, layout_with, pump_frame, test_render_root};

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
    ) -> RenderHandle<RenderParagraph> {
        laid_out_span(app, span(text), constraints)
    }

    /// A paragraph of `text` laid out within `constraints` at the test view's top left, so that
    /// its positions are the view's.
    fn laid_out_span(
        app: &mut App,
        text: InlineSpanRef,
        constraints: BoxConstraints,
    ) -> RenderHandle<RenderParagraph> {
        let paragraph = RenderParagraph::new(app, text, TextDirection::Ltr, None);
        lay_out_at_the_top_left(app, paragraph, constraints);
        paragraph
    }

    fn lay_out_at_the_top_left(
        app: &mut App,
        paragraph: RenderHandle<RenderParagraph>,
        constraints: BoxConstraints,
    ) {
        layout_with(
            app,
            paragraph.as_box(),
            Some(constraints),
            AlignmentGeometry::TOP_LEFT,
            EnginePhase::Layout,
        );
    }

    /// `paragraph_test.dart`: the minimum intrinsic width is the widest word, the maximum is the
    /// whole text on one line, and the intrinsic height at a width is what the text takes there.
    #[test]
    fn paragraph_intrinsics_come_from_the_text() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        crate::testing::install_test_fonts(&mut app);
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
        crate::testing::install_test_fonts(&mut app);
        let constraints = BoxConstraints::new().max_width(200.0).max_height(1000.0);
        let paragraph =
            RenderParagraph::new(&mut app, span("one two three"), TextDirection::Ltr, None);
        let dry = paragraph.as_box().get_dry_layout(&mut app, constraints);
        let dry_baseline =
            paragraph
                .as_box()
                .get_dry_baseline(&mut app, constraints, TextBaseline::Alphabetic);

        let paragraph = laid_out(&mut app, "one two three", constraints);
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
        let paragraph = laid_out(&mut app, "Hello", loose);
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
        let wide = laid_out(&mut app, "one two three four", loose);
        let natural = wide.size(&app);
        let narrow = BoxConstraints::new()
            .max_width(natural.width() / 2.0)
            .max_height(1000.0);
        let wrapped = laid_out(&mut app, "one two three four", narrow);
        assert!(wrapped.size(&app).height() > natural.height());
        assert!(wrapped.size(&app).width() <= natural.width() / 2.0);
    }

    #[test]
    fn soft_wrap_off_lets_the_text_overflow_and_clips_it() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let loose = BoxConstraints::new().max_width(1000.0).max_height(1000.0);
        let wide = laid_out(&mut app, "one two three four", loose);
        let natural = wide.size(&app);
        let paragraph = RenderParagraph::new(
            &mut app,
            span("one two three four"),
            TextDirection::Ltr,
            None,
        );
        paragraph.set_soft_wrap(&mut app, false);
        let narrow = BoxConstraints::new()
            .max_width(natural.width() / 2.0)
            .max_height(1000.0);
        lay_out_at_the_top_left(&mut app, paragraph, narrow);
        assert_eq!(paragraph.size(&app).width(), natural.width() / 2.0);
        assert_eq!(
            paragraph.text_size(&app).width(),
            natural.width(),
            "laid out unwrapped"
        );
        assert!(paragraph.get(&app).needs_clipping);

        paragraph.set_overflow(&mut app, TextOverflow::Visible);
        pump_frame(&mut app, EnginePhase::Layout);
        assert!(!paragraph.get(&app).needs_clipping);
    }

    #[test]
    fn ellipsis_overflow_reports_exceeded_lines() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let loose = BoxConstraints::new().max_width(1000.0).max_height(1000.0);
        let wide = laid_out(&mut app, "one two three four", loose);
        let natural = wide.size(&app);
        let paragraph = RenderParagraph::new(
            &mut app,
            span("one two three four"),
            TextDirection::Ltr,
            None,
        );
        paragraph.set_max_lines(&mut app, Some(1));
        paragraph.set_overflow(&mut app, TextOverflow::Ellipsis);
        lay_out_at_the_top_left(
            &mut app,
            paragraph,
            BoxConstraints::new()
                .max_width(natural.width() / 2.0)
                .max_height(1000.0),
        );
        assert!(paragraph.did_exceed_max_lines(&app));
        assert_eq!(paragraph.size(&app).height(), natural.height());
    }

    #[test]
    fn setting_text_marks_layout_or_paint_by_the_comparison() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let loose = BoxConstraints::new().max_width(1000.0).max_height(1000.0);
        let paragraph = laid_out(&mut app, "Hello", loose);
        pump_frame(&mut app, EnginePhase::Paint);
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
    fn painting_records_glyphs_and_hit_testing_lands_on_the_span_and_the_paragraph() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let loose = BoxConstraints::new().max_width(1000.0).max_height(1000.0);
        let paragraph = laid_out(&mut app, "Hello", loose);
        pump_frame(&mut app, EnginePhase::Paint);
        assert!(!paragraph.as_object().debug_needs_paint(&app));

        let mut result = HitTestResult::new();
        let hit = paragraph.hit_test(
            &mut app,
            &mut BoxHitTestResult::wrap(&mut result),
            Offset::new(2.0, 2.0),
        );
        assert!(hit);
        assert_eq!(result.path().len(), 2, "the span, then the paragraph");
        let position = paragraph.get_position_for_offset(&mut app, Offset::new(0.0, 1.0));
        assert_eq!(position.offset, 0);
        assert_eq!(
            paragraph.get_word_boundary(&mut app, TextPosition::new(1)),
            TextRange::new(0, 5)
        );
    }

    // ---- spans as hit-test targets and mouse tracker annotations ----

    /// The boxes of the text between UTF-16 offsets `start` and `end`, one per line.
    fn boxes_of(
        app: &mut App,
        paragraph: RenderHandle<RenderParagraph>,
        start: i32,
        end: i32,
    ) -> Vec<TextBox> {
        paragraph.get_boxes_for_selection(
            app,
            TextSelection::new(start, end),
            BoxHeightStyle::Tight,
            BoxWidthStyle::Tight,
        )
    }

    fn center_of(text_box: &TextBox) -> Offset {
        Offset::new(
            (text_box.left + text_box.right) / 2.0,
            (text_box.top + text_box.bottom) / 2.0,
        )
    }

    /// The text of each span a hit test at `position` added, front to back.
    fn spans_hit(
        app: &mut App,
        paragraph: RenderHandle<RenderParagraph>,
        position: Offset,
    ) -> (bool, Vec<String>) {
        let mut result = HitTestResult::new();
        let hit = paragraph.hit_test(app, &mut BoxHitTestResult::wrap(&mut result), position);
        let spans = result
            .path()
            .iter()
            .filter_map(|entry| {
                let target: &dyn Any = entry.target();
                target.downcast_ref::<TextSpan>()?.text.clone()
            })
            .collect();
        (hit, spans)
    }

    /// `paragraph_test.dart`, "Basic TextSpan Hit testing": a hit on a glyph adds the span the
    /// glyph is in, and a hit off the glyphs adds no span.
    #[test]
    fn a_hit_on_a_glyph_adds_its_span_and_a_hit_off_the_glyphs_adds_none() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let text = TextSpan::new()
            .style(TextStyle::new().font_size(10.0))
            .children(vec![
                TextSpan::new().text("A".repeat(10)).into_span(),
                TextSpan::new()
                    .text("BC")
                    .style(TextStyle::new().letter_spacing(26.0))
                    .into_span(),
            ])
            .into_span();
        let paragraph = laid_out_span(&mut app, text, BoxConstraints::tight_for(Some(300.0), None));

        let first_a = center_of(&boxes_of(&mut app, paragraph, 0, 1)[0]);
        let last_a = center_of(&boxes_of(&mut app, paragraph, 9, 10)[0]);
        let b = boxes_of(&mut app, paragraph, 10, 11)[0];
        let c = center_of(&boxes_of(&mut app, paragraph, 11, 12)[0]);
        for position in [first_a, last_a] {
            assert_eq!(
                spans_hit(&mut app, paragraph, position),
                (true, vec!["A".repeat(10)])
            );
        }
        // Past B's glyph, in its letter spacing: still B, as the advance reaches C.
        let between_b_and_c = Offset::new(b.right - 1.0, center_of(&b).dy());
        for position in [center_of(&b), between_b_and_c, c] {
            assert_eq!(
                spans_hit(&mut app, paragraph, position),
                (true, vec!["BC".to_owned()])
            );
        }

        let after_c = Offset::new(299.0, c.dy());
        assert_eq!(
            spans_hit(&mut app, paragraph, after_c),
            (true, Vec::new()),
            "the paragraph hits itself, and no span is under the pointer"
        );
        assert_eq!(
            spans_hit(&mut app, paragraph, Offset::new(9999.0, 9999.0)),
            (false, Vec::new())
        );
    }

    /// Plays `RendererBinding`'s part: hit-tests the test view's root before the gesture binding
    /// adds itself to the path.
    struct RootHitTester {
        root: RenderHandle<RenderRepaintBoundary>,
    }

    impl waft_gestures::GestureBindingOverridesObject for RootHitTester {
        fn hit_test_in_view(
            self: Handle<Self>,
            app: &mut App,
            result: &mut HitTestResult,
            position: Offset,
            _view_id: waft_embedder::ViewId,
        ) {
            let root = app.get(self).root;
            root.hit_test(app, &mut BoxHitTestResult::wrap(result), position);
        }

        fn will_dispatch_event(
            self: Handle<Self>,
            _app: &mut App,
            _event: &waft_gestures::PointerEvent,
            _hit_test_result: Option<&HitTestResult>,
        ) {
        }
    }

    fn tap_at(app: &mut App, position: Offset) {
        use waft_gestures::{GestureBinding, PointerDownEvent, PointerEvent, PointerUpEvent};
        let binding = GestureBinding::instance(app);
        binding.handle_pointer_event(
            app,
            PointerEvent::Down(PointerDownEvent {
                position,
                ..PointerDownEvent::default()
            }),
        );
        app.drain_microtasks();
        binding.handle_pointer_event(
            app,
            PointerEvent::Up(PointerUpEvent {
                position,
                ..PointerUpEvent::default()
            }),
        );
        app.drain_microtasks();
    }

    /// A press on a span reaches its `TapGestureRecognizer`; a tap on the text beside it does
    /// not.
    #[test]
    fn a_tap_on_a_span_runs_its_recognizer_and_a_tap_beside_it_does_not() {
        use waft_gestures::{GestureBinding, GestureRecognizerLeaf, TapGestureRecognizer};

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let taps = Rc::new(std::cell::Cell::new(0));
        let recognizer = TapGestureRecognizer::new(&mut app);
        let counter = Rc::clone(&taps);
        recognizer.set_on_tap(
            &mut app,
            Some(waft_foundation::Listener::new(move |_app| {
                counter.set(counter.get() + 1)
            })),
        );
        let text = TextSpan::new()
            .style(TextStyle::new().font_size(20.0))
            .children(vec![
                TextSpan::new().text("Tap ").into_span(),
                TextSpan::new()
                    .text("here")
                    .recognizer(recognizer.erase())
                    .into_span(),
                TextSpan::new().text(" please").into_span(),
            ])
            .into_span();
        let paragraph = laid_out_span(
            &mut app,
            text,
            BoxConstraints::new().max_width(1000.0).max_height(1000.0),
        );
        let root = test_render_root(&mut app);
        let hit_tester = app.create(RootHitTester { root });
        GestureBinding::instance(&mut app).set_overrides(&mut app, Rc::new(hit_tester));

        let here = center_of(&boxes_of(&mut app, paragraph, 4, 8)[0]);
        tap_at(&mut app, here);
        assert_eq!(taps.get(), 1);

        let beside = center_of(&boxes_of(&mut app, paragraph, 0, 3)[0]);
        tap_at(&mut app, beside);
        assert_eq!(taps.get(), 1, "the plain span has no recognizer");
        recognizer.as_recognizer().dispose(&mut app);
    }

    /// A span's cursor shows while the mouse is over it: the click cursor by default for a span
    /// with a recognizer, the one it names otherwise, and the one behind when it defers.
    #[test]
    fn the_mouse_shows_the_cursor_of_the_span_under_it() {
        use waft_gestures::{
            GestureRecognizerLeaf, PointerAddedEvent, PointerEvent, PointerHoverEvent,
            TapGestureRecognizer,
        };
        use waft_services::{MouseCursorRef, SystemMouseCursors};

        use crate::mouse_tracker::MouseTracker;

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let recognizer = TapGestureRecognizer::new(&mut app);
        let text = TextSpan::new()
            .style(TextStyle::new().font_size(20.0))
            .children(vec![
                TextSpan::new().text("plain ").into_span(),
                TextSpan::new()
                    .text("link")
                    .recognizer(recognizer.erase())
                    .into_span(),
                TextSpan::new()
                    .text(" forbidden")
                    .mouse_cursor(SystemMouseCursors::FORBIDDEN)
                    .into_span(),
            ])
            .into_span();
        let paragraph = laid_out_span(
            &mut app,
            text,
            BoxConstraints::new().max_width(1000.0).max_height(1000.0),
        );
        let root = test_render_root(&mut app);
        let tracker = MouseTracker::new(
            &mut app,
            Rc::new(move |app: &mut App, position: Offset, _view| {
                let mut result = HitTestResult::new();
                root.hit_test(app, &mut BoxHitTestResult::wrap(&mut result), position);
                result
            }),
        );
        let plain = center_of(&boxes_of(&mut app, paragraph, 0, 5)[0]);
        let link = center_of(&boxes_of(&mut app, paragraph, 6, 10)[0]);
        let forbidden = center_of(&boxes_of(&mut app, paragraph, 11, 20)[0]);
        tracker.update_with_event(
            &mut app,
            &PointerEvent::Added(PointerAddedEvent {
                kind: waft_embedder::PointerDeviceKind::Mouse,
                position: plain,
                ..PointerAddedEvent::default()
            }),
            None,
        );
        let cursor_at = |app: &mut App, position: Offset| -> MouseCursorRef {
            tracker.update_with_event(
                app,
                &PointerEvent::Hover(PointerHoverEvent {
                    kind: waft_embedder::PointerDeviceKind::Mouse,
                    position,
                    ..PointerHoverEvent::default()
                }),
                None,
            );
            tracker.debug_device_active_cursor(app, 0).unwrap()
        };
        let basic: MouseCursorRef = SystemMouseCursors::BASIC.into();
        let click: MouseCursorRef = SystemMouseCursors::CLICK.into();
        let forbidden_cursor: MouseCursorRef = SystemMouseCursors::FORBIDDEN.into();
        assert!(*cursor_at(&mut app, link) == *click);
        assert!(*cursor_at(&mut app, plain) == *basic, "a plain span defers");
        assert!(*cursor_at(&mut app, forbidden) == *forbidden_cursor);
        recognizer.as_recognizer().dispose(&mut app);
    }

    /// A span that wraps over two lines is one annotation: the mouse enters it once on the
    /// first line, stays in it on the second, and exits it once.
    #[test]
    fn a_wrapped_span_is_entered_once_across_its_lines() {
        use std::cell::RefCell;

        use waft_gestures::{
            PointerAddedEvent, PointerEnterEvent, PointerEvent, PointerExitEvent, PointerHoverEvent,
        };

        use crate::mouse_tracker::MouseTracker;

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log: Rc<RefCell<Vec<&str>>> = Rc::default();
        let enter_log = Rc::clone(&log);
        let exit_log = Rc::clone(&log);
        let words = "one two three four five six seven eight";
        let text = TextSpan::new()
            .text(words)
            .style(TextStyle::new().font_size(20.0))
            .on_enter(Rc::new(move |_app: &mut App, _event: PointerEnterEvent| {
                enter_log.borrow_mut().push("enter")
            }))
            .on_exit(Rc::new(move |_app: &mut App, _event: PointerExitEvent| {
                exit_log.borrow_mut().push("exit")
            }))
            .into_span();
        let paragraph = laid_out_span(
            &mut app,
            text,
            BoxConstraints::new().max_width(120.0).max_height(1000.0),
        );
        let boxes = boxes_of(&mut app, paragraph, 0, 3);
        let first_line = center_of(&boxes[0]);
        let last = words.len() as i32;
        let last_box = *boxes_of(&mut app, paragraph, last - 1, last)
            .last()
            .unwrap();
        assert!(last_box.top > boxes[0].bottom - 1.0, "the span wraps");
        let last_line = center_of(&last_box);
        let root = test_render_root(&mut app);
        let tracker = MouseTracker::new(
            &mut app,
            Rc::new(move |app: &mut App, position: Offset, _view| {
                let mut result = HitTestResult::new();
                root.hit_test(app, &mut BoxHitTestResult::wrap(&mut result), position);
                result
            }),
        );
        let mouse = |position: Offset| {
            PointerEvent::Hover(PointerHoverEvent {
                kind: waft_embedder::PointerDeviceKind::Mouse,
                position,
                ..PointerHoverEvent::default()
            })
        };
        tracker.update_with_event(
            &mut app,
            &PointerEvent::Added(PointerAddedEvent {
                kind: waft_embedder::PointerDeviceKind::Mouse,
                position: Offset::new(5000.0, 5000.0),
                ..PointerAddedEvent::default()
            }),
            None,
        );
        tracker.update_with_event(&mut app, &mouse(first_line), None);
        tracker.update_with_event(&mut app, &mouse(last_line), None);
        assert_eq!(*log.borrow(), ["enter"]);
        tracker.update_with_event(&mut app, &mouse(Offset::new(5000.0, 5000.0)), None);
        assert_eq!(*log.borrow(), ["enter", "exit"]);
    }
}

#[cfg(test)]
mod selection_tests {
    //! The `Selection` group of `paragraph_test.dart`.
    //!
    //! Skipped: "can handle select all when contains widget span" is in `waft-widgets`'s
    //! `widget_span.rs`, as `WidgetSpan` is a widgets type.

    use waft_embedder::valo::Op;
    use waft_embedder::{Color, Offset, Rect, TextPosition};
    use waft_foundation::{AppCell, Handle};
    use waft_painting::TextSpan;

    use super::*;
    use crate::layer::{AnyLayer, OffsetLayer, PictureLayer};
    use crate::selection::{
        AnySelectable, SelectionEvent, SelectionExtendDirection, SelectionRegistrar,
        TextGranularity,
    };
    use crate::testing::EnginePhase;

    /// `paragraph_test.dart`'s `TestSelectionRegistrar`.
    struct TestSelectionRegistrar {
        selectables: Vec<AnySelectable>,
    }

    impl SelectionRegistrar for TestSelectionRegistrar {
        fn add(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
            app.get_mut(self).selectables.push(selectable);
        }

        fn remove(self: Handle<Self>, app: &mut App, selectable: AnySelectable) {
            app.get_mut(self).selectables.retain(|s| *s != selectable);
        }
    }

    const SELECTION_COLOR: Color = Color::from_argb(0xAF, 0x66, 0x94, 0xE8);

    fn span(text: &str) -> InlineSpanRef {
        TextSpan::new().text(text).into_span()
    }

    /// Dart's `TextSpan(children: <InlineSpan>[TextSpan(text: text)])`.
    fn nested_span(text: &str) -> InlineSpanRef {
        TextSpan::new().children(vec![span(text)]).into_span()
    }

    fn registrar(app: &mut App) -> Ref<TestSelectionRegistrar> {
        app.create(TestSelectionRegistrar {
            selectables: Vec::new(),
        })
    }

    fn selectables(app: &App, registrar: &Ref<TestSelectionRegistrar>) -> Vec<AnySelectable> {
        app.get(registrar).selectables.clone()
    }

    fn paragraph_with_registrar(
        app: &mut App,
        text: InlineSpanRef,
        registrar: &Ref<TestSelectionRegistrar>,
    ) -> RenderHandle<RenderParagraph> {
        let paragraph = RenderParagraph::new(app, text, TextDirection::Ltr, None);
        paragraph.set_registrar(app, Some(registrar.erase()));
        paragraph
    }

    /// The group's `selectionParagraph`.
    fn selection_paragraph(
        app: &mut App,
        paragraph: RenderHandle<RenderParagraph>,
        registrar: &Ref<TestSelectionRegistrar>,
        start: TextPosition,
        end: TextPosition,
    ) {
        for selectable in selectables(app, registrar) {
            let start_offset =
                paragraph.get_offset_for_caret(app, start, Rect::ZERO) + Offset::new(0.0, 5.0);
            selectable
                .dispatch_selection_event(app, &SelectionEvent::for_start(start_offset, None));
            let end_offset =
                paragraph.get_offset_for_caret(app, end, Rect::ZERO) + Offset::new(0.0, 5.0);
            selectable.dispatch_selection_event(app, &SelectionEvent::for_end(end_offset, None));
        }
    }

    /// `paragraph.selections[0]` as its start and end, where Dart expects one selection.
    fn the_selection(app: &App, paragraph: RenderHandle<RenderParagraph>) -> (i32, i32) {
        let selections = paragraph.selections(app);
        assert_eq!(selections.len(), 1);
        (selections[0].start(), selections[0].end())
    }

    /// `rendering_tester.dart`'s `layout(box, constraints:)`.
    fn lay_out(
        app: &mut App,
        paragraph: RenderHandle<RenderParagraph>,
        constraints: Option<BoxConstraints>,
    ) {
        crate::testing::layout_with(
            app,
            paragraph.as_box(),
            constraints,
            waft_painting::AlignmentGeometry::CENTER,
            EnginePhase::Layout,
        );
    }

    fn drawn_rects(ops: &[Op]) -> Vec<(Rect, Color)> {
        ops.iter()
            .filter_map(|op| match op {
                Op::DrawRect { rect, paint, .. } => Some((
                    Rect::from_ltrb(
                        rect.x as f64,
                        rect.y as f64,
                        (rect.x + rect.width) as f64,
                        (rect.y + rect.height) as f64,
                    ),
                    paint.color.into(),
                )),
                _ => None,
            })
            .collect()
    }

    /// flutter_test's `isSameColorAs`: equal within one 8-bit step per channel.
    fn assert_same_color_as(actual: Color, expected: Color) {
        let step = 1.0 / 255.0;
        assert!(
            actual.color_space == expected.color_space
                && (actual.a - expected.a).abs() <= step
                && (actual.r - expected.r).abs() <= step
                && (actual.g - expected.g).abs() <= step
                && (actual.b - expected.b).abs() <= step,
            "{actual:?} is not {expected:?}"
        );
    }

    // 'subscribe to SelectionRegistrar'.
    #[test]
    fn subscribe_to_selection_registrar() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let registrar = registrar(&mut app);
        let paragraph = paragraph_with_registrar(&mut app, span("1234567"), &registrar);
        assert_eq!(selectables(&app, &registrar).len(), 1);

        paragraph.set_text(&mut app, span(""));
        assert_eq!(selectables(&app, &registrar).len(), 0);
    }

    // 'paints selection highlight'.
    #[test]
    fn paints_selection_highlight() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let registrar = registrar(&mut app);
        let paragraph = paragraph_with_registrar(&mut app, span("1234567"), &registrar);
        paragraph.set_selection_color(&mut app, Some(SELECTION_COLOR));
        lay_out(&mut app, paragraph, None);
        let ops = crate::testing::painted_ops(&mut app, paragraph.as_object());
        assert!(drawn_rects(&ops).is_empty());
        selection_paragraph(
            &mut app,
            paragraph,
            &registrar,
            TextPosition::new(1),
            TextPosition::new(5),
        );

        let ops = crate::testing::painted_ops(&mut app, paragraph.as_object());
        let drawn = drawn_rects(&ops);
        assert_eq!(drawn.len(), 1);
        assert_eq!(drawn[0].0, Rect::from_ltwh(14.0, 0.0, 56.0, 14.0));
        assert_same_color_as(drawn[0].1, SELECTION_COLOR);
        // Selection highlight is painted before text.
        let drawn_items: Vec<&str> = ops
            .iter()
            .filter_map(|op| match op {
                Op::DrawRect { .. } => Some("Rect"),
                Op::GlyphRun { .. } => Some("Paragraph"),
                _ => None,
            })
            .collect();
        assert_eq!(drawn_items.first(), Some(&"Rect"));
        assert!(drawn_items[1..].iter().all(|item| *item == "Paragraph"));

        selection_paragraph(
            &mut app,
            paragraph,
            &registrar,
            TextPosition::new(2),
            TextPosition::new(4),
        );
        let ops = crate::testing::painted_ops(&mut app, paragraph.as_object());
        let drawn = drawn_rects(&ops);
        let last = drawn.last().expect("a selection rect");
        assert_eq!(last.0, Rect::from_ltwh(28.0, 0.0, 28.0, 14.0));
        assert_same_color_as(last.1, SELECTION_COLOR);
    }

    // 'paints selection highlights outside fade layer and handles after text'. Waft's fade
    // clips without its shader (see `PORTING.md`), so the case's checks of the fade's
    // `saveLayer` and shader rect are left out; the rest are Dart's.
    #[test]
    fn paints_selection_highlights_outside_fade_layer_and_handles_after_text() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let registrar = registrar(&mut app);
        let paragraph = paragraph_with_registrar(&mut app, span("a\na\na\na"), &registrar);
        paragraph.set_selection_color(&mut app, Some(SELECTION_COLOR));
        paragraph.set_max_lines(&mut app, Some(3));
        paragraph.set_overflow(&mut app, TextOverflow::Fade);
        lay_out(
            &mut app,
            paragraph,
            Some(BoxConstraints::new().max_width(100.0)),
        );

        for selectable in selectables(&app, &registrar) {
            selectable.dispatch_selection_event(&mut app, &SelectionEvent::SelectAll);
            let start = LayerLink::new(&mut app);
            let end = LayerLink::new(&mut app);
            selectable.push_handle_layers(&mut app, Some(start), Some(end));
        }

        let layer = OffsetLayer::new(&mut app, Offset::ZERO);
        let container = layer.as_container_layer();
        let bounds = paragraph.as_object().paint_bounds(&app);
        let mut context = PaintingContext::new(container, bounds);
        paragraph
            .as_object()
            .paint(&mut app, &mut context, Offset::ZERO);
        context.stop_recording_if_needed(&mut app);

        let mut children: Vec<AnyLayer> = Vec::new();
        let mut child = container.first_child(&app);
        while let Some(layer) = child {
            children.push(layer);
            child = layer.next_sibling(&app);
        }
        // The text's picture, then the handles' leaders pushed after it.
        assert_eq!(children.len(), 3);
        let picture = app
            .handle::<PictureLayer>(children[0].id())
            .and_then(|picture| picture.picture(&app))
            .expect("the text's picture");
        let ops = waft_embedder::flattened_ops(picture);
        let first_glyph = ops
            .iter()
            .position(|op| matches!(op, Op::GlyphRun { .. }))
            .expect("the text");
        assert!(
            ops[..first_glyph]
                .iter()
                .any(|op| matches!(op, Op::DrawRect { .. }))
        );
        assert!(
            !ops[first_glyph..]
                .iter()
                .any(|op| matches!(op, Op::DrawRect { .. }))
        );
        let leaders = children[1..]
            .iter()
            .filter(|layer| app.handle::<LeaderLayer>(layer.id()).is_some())
            .count();
        assert_eq!(leaders, 2);
    }

    // 'paints selection when tap at chinese character'.
    #[test]
    fn paints_selection_when_tap_at_chinese_character() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let registrar = registrar(&mut app);
        let paragraph = paragraph_with_registrar(&mut app, span("\u{4F60}\u{597D}"), &registrar);
        paragraph.set_selection_color(&mut app, Some(SELECTION_COLOR));
        lay_out(&mut app, paragraph, None);
        let ops = crate::testing::painted_ops(&mut app, paragraph.as_object());
        assert!(drawn_rects(&ops).is_empty());

        for selectable in selectables(&app, &registrar) {
            selectable.dispatch_selection_event(
                &mut app,
                &SelectionEvent::SelectWord {
                    global_position: Offset::new(7.0, 0.0),
                },
            );
        }

        let ops = crate::testing::painted_ops(&mut app, paragraph.as_object());
        let (rect, color) = *drawn_rects(&ops).last().expect("a selection rect");
        assert!(!rect.is_empty());
        assert_same_color_as(color, SELECTION_COLOR);
    }

    // 'getPositionForOffset works'.
    #[test]
    fn get_position_for_offset_works() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let paragraph = RenderParagraph::new(&mut app, span("1234567"), TextDirection::Ltr, None);
        lay_out(&mut app, paragraph, None);
        assert_eq!(
            paragraph.get_position_for_offset(&mut app, Offset::new(42.0, 14.0)),
            TextPosition::new(3)
        );
    }

    fn granularly_extend(
        app: &mut App,
        registrar: &Ref<TestSelectionRegistrar>,
        forward: bool,
        granularity: TextGranularity,
    ) {
        selectables(app, registrar)[0].dispatch_selection_event(
            app,
            &SelectionEvent::GranularlyExtendSelection {
                forward,
                is_end: true,
                granularity,
            },
        );
    }

    /// The paragraph the extend cases select in, laid out, with its one selectable.
    fn how_are_you(
        app: &mut App,
        registrar: &Ref<TestSelectionRegistrar>,
    ) -> RenderHandle<RenderParagraph> {
        let paragraph = paragraph_with_registrar(
            app,
            nested_span("how are you\nI am fine\nThank you"),
            registrar,
        );
        lay_out(app, paragraph, None);
        assert_eq!(selectables(app, registrar).len(), 1);
        paragraph
    }

    // 'can granularly extend selection - character'.
    #[test]
    fn can_granularly_extend_selection_character() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let registrar = registrar(&mut app);
        let paragraph = how_are_you(&mut app, &registrar);
        selection_paragraph(
            &mut app,
            paragraph,
            &registrar,
            TextPosition::new(4),
            TextPosition::new(5),
        );
        assert_eq!(the_selection(&app, paragraph), (4, 5)); // how [a]re you

        // Equivalent to sending shift + arrow-right
        granularly_extend(&mut app, &registrar, true, TextGranularity::Character);
        assert_eq!(the_selection(&app, paragraph), (4, 6)); // how [ar]e you

        // Equivalent to sending shift + arrow-left
        granularly_extend(&mut app, &registrar, false, TextGranularity::Character);
        assert_eq!(the_selection(&app, paragraph), (4, 5)); // how [a]re you
    }

    // 'can granularly extend selection - word'.
    #[test]
    fn can_granularly_extend_selection_word() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let registrar = registrar(&mut app);
        let paragraph = how_are_you(&mut app, &registrar);
        selection_paragraph(
            &mut app,
            paragraph,
            &registrar,
            TextPosition::new(4),
            TextPosition::new(5),
        );
        assert_eq!(the_selection(&app, paragraph), (4, 5)); // how [a]re you

        // Equivalent to sending shift + alt + arrow-right.
        granularly_extend(&mut app, &registrar, true, TextGranularity::Word);
        assert_eq!(the_selection(&app, paragraph), (4, 7)); // how [are] you

        // Equivalent to sending shift + alt + arrow-left.
        granularly_extend(&mut app, &registrar, false, TextGranularity::Word);
        assert_eq!(the_selection(&app, paragraph), (4, 4)); // how []are you

        // Equivalent to sending shift + alt + arrow-left.
        granularly_extend(&mut app, &registrar, false, TextGranularity::Word);
        assert_eq!(the_selection(&app, paragraph), (0, 4)); // [how ]are you
    }

    // 'can granularly extend selection - line'.
    #[test]
    fn can_granularly_extend_selection_line() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let registrar = registrar(&mut app);
        let paragraph = how_are_you(&mut app, &registrar);
        selection_paragraph(
            &mut app,
            paragraph,
            &registrar,
            TextPosition::new(4),
            TextPosition::new(5),
        );
        assert_eq!(the_selection(&app, paragraph), (4, 5)); // how [a]re you

        // Equivalent to sending shift + meta + arrow-right.
        granularly_extend(&mut app, &registrar, true, TextGranularity::Line);
        // how [are you]
        assert_eq!(the_selection(&app, paragraph), (4, 11));

        // Equivalent to sending shift + meta + arrow-left.
        granularly_extend(&mut app, &registrar, false, TextGranularity::Line);
        // [how ]are you
        assert_eq!(the_selection(&app, paragraph), (0, 4));
    }

    // 'can granularly extend selection - document'.
    #[test]
    fn can_granularly_extend_selection_document() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let registrar = registrar(&mut app);
        let paragraph = how_are_you(&mut app, &registrar);
        selection_paragraph(
            &mut app,
            paragraph,
            &registrar,
            TextPosition::new(14),
            TextPosition::new(15),
        );
        // how are you
        // I [a]m fine
        assert_eq!(the_selection(&app, paragraph), (14, 15));

        // Equivalent to sending shift + meta + arrow-down.
        granularly_extend(&mut app, &registrar, true, TextGranularity::Document);
        // how are you
        // I [am fine
        // Thank you]
        assert_eq!(the_selection(&app, paragraph), (14, 31));

        // Equivalent to sending shift + meta + arrow-up.
        granularly_extend(&mut app, &registrar, false, TextGranularity::Document);
        // [how are you
        // I ]am fine
        assert_eq!(the_selection(&app, paragraph), (0, 14));
    }

    // 'can granularly extend selection when no active selection'.
    #[test]
    fn can_granularly_extend_selection_when_no_active_selection() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let registrar = registrar(&mut app);
        let paragraph = how_are_you(&mut app, &registrar);
        assert!(paragraph.selections(&app).is_empty());

        // Equivalent to sending shift + alt + right.
        granularly_extend(&mut app, &registrar, true, TextGranularity::Word);
        // [how] are you
        assert_eq!(the_selection(&app, paragraph), (0, 3));

        // Remove selection
        selectables(&app, &registrar)[0].dispatch_selection_event(&mut app, &SelectionEvent::Clear);
        assert!(paragraph.selections(&app).is_empty());

        // Equivalent to sending shift + alt + left.
        granularly_extend(&mut app, &registrar, false, TextGranularity::Word);
        // how are you
        // I am fine
        // Thank [you]
        assert_eq!(the_selection(&app, paragraph), (28, 31));
    }

    fn directionally_extend(
        app: &mut App,
        registrar: &Ref<TestSelectionRegistrar>,
        dx: f64,
        direction: SelectionExtendDirection,
    ) {
        selectables(app, registrar)[0].dispatch_selection_event(
            app,
            &SelectionEvent::DirectionallyExtendSelection {
                dx,
                is_end: true,
                direction,
            },
        );
    }

    /// `MatrixUtils.transformPoint(transform, point).dx`.
    fn transformed_dx(transform: &Matrix4, point: Offset) -> f64 {
        waft_painting::transform_point(transform, point).dx()
    }

    // 'can directionally extend selection'.
    #[test]
    fn can_directionally_extend_selection() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let registrar = registrar(&mut app);
        let paragraph = how_are_you(&mut app, &registrar);
        selection_paragraph(
            &mut app,
            paragraph,
            &registrar,
            TextPosition::new(14),
            TextPosition::new(15),
        );
        // how are you
        // I [a]m fine
        assert_eq!(the_selection(&app, paragraph), (14, 15));

        let selectable = selectables(&app, &registrar)[0];
        let baseline = transformed_dx(
            &selectable.get_transform_to(&app, None),
            selectable
                .value(&app)
                .end_selection_point
                .expect("a selection")
                .local_position,
        );

        // Equivalent to sending shift + arrow-down.
        directionally_extend(
            &mut app,
            &registrar,
            baseline,
            SelectionExtendDirection::NextLine,
        );
        // how are you
        // I [am fine
        // Tha]nk you
        assert_eq!(the_selection(&app, paragraph), (14, 25));

        // Equivalent to sending shift + arrow-up.
        directionally_extend(
            &mut app,
            &registrar,
            baseline,
            SelectionExtendDirection::PreviousLine,
        );
        // how are you
        // I [a]m fine
        assert_eq!(the_selection(&app, paragraph), (14, 15));
    }

    // 'can directionally extend selection when no selection'.
    #[test]
    fn can_directionally_extend_selection_when_no_selection() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let registrar = registrar(&mut app);
        let paragraph = how_are_you(&mut app, &registrar);
        assert!(paragraph.selections(&app).is_empty());

        let selectable = selectables(&app, &registrar)[0];
        let size = selectable.size(&mut app);
        let baseline = transformed_dx(
            &selectable.get_transform_to(&app, None),
            Offset::new(size.width() / 2.0, 0.0),
        );

        // Equivalent to sending shift + arrow-down.
        directionally_extend(
            &mut app,
            &registrar,
            baseline,
            SelectionExtendDirection::Forward,
        );
        // [how ar]e you
        // I am fine
        assert_eq!(the_selection(&app, paragraph), (0, 6));

        // Remove selection
        selectable.dispatch_selection_event(&mut app, &SelectionEvent::Clear);
        assert!(paragraph.selections(&app).is_empty());

        // Equivalent to sending shift + arrow-up.
        directionally_extend(
            &mut app,
            &registrar,
            baseline,
            SelectionExtendDirection::Backward,
        );
        // how are you
        // I am fine
        // Thank [you]
        assert_eq!(the_selection(&app, paragraph), (28, 31));
    }
}
