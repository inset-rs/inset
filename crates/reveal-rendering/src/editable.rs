//! Flutter counterpart: `rendering/editable.dart`.
//!
//! A leaf until inline children, the custom-paint child boxes, `LeaderLayer`
//! handles, pointer recognizers, and `RelayoutWhenSystemFontsChangeMixin` land.
//! Caret and selection paint in `paint`, the same work Flutter's child painters
//! do.

use std::collections::HashMap;
use std::fmt;

use reveal_embedder::{
    BoxHeightStyle, BoxWidthStyle, Clip, Color, FontCollection, LineMetrics, Offset, Paint, Radius,
    Rect, Size, TargetPlatform, TextAffinity, TextAlign, TextBaseline, TextDirection,
    TextHeightBehavior, TextPosition, TextRange, TextSelection, clamp_double,
};
use reveal_foundation::{App, Handle, Listenable, Listener, ValueListenable, ValueNotifier};
use reveal_gestures::TapDownDetails;
use reveal_painting::{
    ClipContext, InlineSpanRef, PaintingBinding, TextPainter, TextScaler, TextWidthBasis,
};
use reveal_services::{AnyTextSelectionDelegate, SelectionChangedCause, TextLayoutMetrics};

use crate::box_::{BoxConstraints, RenderBox, RenderBoxData};
use crate::layer::{ClipRectLayer, ContainerLayer, LayerHandle, LayerLink, LeaderLayer};
use crate::object::{AnyRenderObject, RenderHandle, RenderObject, RenderObjectData};
use crate::painting_context::PaintingContext;
use crate::pipeline_owner::PipelineOwner;
use crate::text_boundary::WordBoundary;
use crate::viewport_offset::AnyViewportOffset;

const K_CARET_GAP: f64 = 1.0;
const K_CARET_HEIGHT_OFFSET: f64 = 2.0;

/// Represents the coordinates of the point in a selection, and the text
/// direction at that point, relative to top left of the [`RenderEditable`] that
/// holds the selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectionPoint {
    /// Coordinates of the lower left or lower right corner of the selection,
    /// relative to the top left of the [`RenderEditable`] object.
    pub point: Offset,
    /// Direction of the text at this edge of the selection.
    pub direction: Option<TextDirection>,
}

impl TextSelectionPoint {
    /// Creates a description of a point in a text selection.
    pub const fn new(point: Offset, direction: Option<TextDirection>) -> TextSelectionPoint {
        TextSelectionPoint { point, direction }
    }
}

impl fmt::Display for TextSelectionPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.direction {
            Some(TextDirection::Ltr) => write!(f, "{:?}-ltr", self.point),
            Some(TextDirection::Rtl) => write!(f, "{:?}-rtl", self.point),
            None => write!(f, "{:?}", self.point),
        }
    }
}

/// Starts at a text position and walks vertically through line metrics.
///
/// [`is_valid`](Self::is_valid) compares the editable's layout generation, not
/// Dart's `identical` on the metrics list.
pub struct VerticalCaretMovementRun {
    editable: RenderHandle<RenderEditable>,
    layout_generation: u64,
    current_offset: Offset,
    current_line: i32,
    current_text_position: TextPosition,
    line_metrics: Vec<LineMetrics>,
    position_cache: HashMap<i32, (Offset, TextPosition)>,
    is_valid: bool,
}

impl VerticalCaretMovementRun {
    /// Whether this run can still continue.
    pub fn is_valid(&mut self, app: &App) -> bool {
        if !self.is_valid {
            return false;
        }
        if self.editable.get(app).layout_generation != self.layout_generation {
            self.is_valid = false;
        }
        self.is_valid
    }

    /// The current text position.
    pub fn current(&self) -> TextPosition {
        debug_assert!(self.is_valid);
        self.current_text_position
    }

    fn get_text_position_for_line(
        &mut self,
        app: &App,
        line_number: i32,
    ) -> (Offset, TextPosition) {
        debug_assert!(self.is_valid);
        debug_assert!(line_number >= 0);
        if let Some(cached) = self.position_cache.get(&line_number) {
            return *cached;
        }
        debug_assert!(line_number != self.current_line);
        let new_offset = Offset::new(
            self.current_offset.dx(),
            self.line_metrics[line_number as usize].baseline,
        );
        let closest = self
            .editable
            .get(app)
            .text_painter
            .get_position_for_offset(new_offset);
        self.position_cache
            .insert(line_number, (new_offset, closest));
        (new_offset, closest)
    }

    /// Move to the next line.
    pub fn move_next(&mut self, app: &App) -> bool {
        debug_assert!(self.is_valid(app));
        if self.current_line + 1 >= self.line_metrics.len() as i32 {
            return false;
        }
        let (offset, position) = self.get_text_position_for_line(app, self.current_line + 1);
        self.current_line += 1;
        self.current_offset = offset;
        self.current_text_position = position;
        true
    }

    /// Move back to the previous element.
    pub fn move_previous(&mut self, app: &App) -> bool {
        debug_assert!(self.is_valid(app));
        if self.current_line <= 0 {
            return false;
        }
        let (offset, position) = self.get_text_position_for_line(app, self.current_line - 1);
        self.current_line -= 1;
        self.current_offset = offset;
        self.current_text_position = position;
        true
    }

    /// Move forward or backward by a number of elements determined by pixel `offset`.
    pub fn move_by_offset(&mut self, app: &App, offset: f64) -> bool {
        let initial_offset = self.current_offset;
        if offset >= 0.0 {
            while self.current_offset.dy() < initial_offset.dy() + offset {
                if !self.move_next(app) {
                    break;
                }
            }
        } else {
            while self.current_offset.dy() > initial_offset.dy() + offset {
                if !self.move_previous(app) {
                    break;
                }
            }
        }
        initial_offset != self.current_offset
    }
}

/// Displays some text in a scrollable container with a potentially blinking
/// cursor and with gesture recognizers.
///
/// This is the renderer for an editable text field. It does not directly
/// provide affordances for editing the text, but it does handle text selection
/// and manipulation of the text cursor.
pub struct RenderEditable {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    text_painter: TextPainter,
    text_intrinsics: Option<TextPainter>,
    fonts_override: Option<Handle<FontCollection>>,
    show_cursor: Handle<ValueNotifier<bool>>,
    #[allow(dead_code)]
    dispose_show_cursor: bool,
    has_focus: bool,
    start_handle_layer_link: Handle<LayerLink>,
    end_handle_layer_link: Handle<LayerLink>,
    max_lines: Option<i32>,
    min_lines: Option<i32>,
    expands: bool,
    selection_color: Option<Color>,
    selection: Option<TextSelection>,
    offset: AnyViewportOffset,
    /// Whether [`handle_event`](RenderObject::handle_event) will propagate pointer
    /// events to selection handlers.
    pub ignore_pointer: bool,
    read_only: bool,
    force_line: bool,
    obscuring_character: String,
    obscure_text: bool,
    cursor_color: Option<Color>,
    background_cursor_color: Option<Color>,
    cursor_width: f64,
    cursor_height: Option<f64>,
    cursor_radius: Option<Radius>,
    paint_cursor_above_text: bool,
    cursor_offset: Offset,
    device_pixel_ratio: f64,
    selection_height_style: BoxHeightStyle,
    selection_width_style: BoxWidthStyle,
    enable_interactive_selection: Option<bool>,
    prompt_rect_range: Option<TextRange>,
    prompt_rect_color: Option<Color>,
    clip_behavior: Clip,
    clip_rect_layer: LayerHandle<Handle<ClipRectLayer>>,
    leader_layer_handler: LayerHandle<Handle<LeaderLayer>>,
    text_selection_delegate: AnyTextSelectionDelegate,
    caret_prototype: Rect,
    max_scroll_extent: f64,
    last_tap_down_position: Option<Offset>,
    last_secondary_tap_down_position: Option<Offset>,
    layout_generation: u64,
    selection_start_in_viewport: Handle<ValueNotifier<bool>>,
    selection_end_in_viewport: Handle<ValueNotifier<bool>>,
}

impl RenderEditable {
    /// Creates a render object that implements the visual aspects of a text field.
    pub fn new(
        app: &mut App,
        text_direction: TextDirection,
        start_handle_layer_link: Handle<LayerLink>,
        end_handle_layer_link: Handle<LayerLink>,
        offset: AnyViewportOffset,
        text_selection_delegate: AnyTextSelectionDelegate,
    ) -> RenderHandle<Self> {
        let show_cursor = app.create(ValueNotifier::new(false));
        let selection_start_in_viewport = app.create(ValueNotifier::new(true));
        let selection_end_in_viewport = app.create(ValueNotifier::new(true));
        let mut text_painter = TextPainter::new();
        text_painter.set_text_direction(Some(text_direction));
        text_painter.set_max_lines(Some(1));
        RenderHandle::new_box(
            app,
            RenderEditable {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                text_painter,
                text_intrinsics: None,
                fonts_override: None,
                show_cursor,
                dispose_show_cursor: true,
                has_focus: false,
                start_handle_layer_link,
                end_handle_layer_link,
                max_lines: Some(1),
                min_lines: None,
                expands: false,
                selection_color: None,
                selection: None,
                offset,
                ignore_pointer: false,
                read_only: false,
                force_line: true,
                obscuring_character: "\u{2022}".into(),
                obscure_text: false,
                cursor_color: None,
                background_cursor_color: None,
                cursor_width: 1.0,
                cursor_height: None,
                cursor_radius: None,
                paint_cursor_above_text: false,
                cursor_offset: Offset::ZERO,
                device_pixel_ratio: 1.0,
                selection_height_style: BoxHeightStyle::Max,
                selection_width_style: BoxWidthStyle::Max,
                enable_interactive_selection: None,
                prompt_rect_range: None,
                prompt_rect_color: None,
                clip_behavior: Clip::HardEdge,
                clip_rect_layer: LayerHandle::new(),
                leader_layer_handler: LayerHandle::new(),
                text_selection_delegate,
                caret_prototype: Rect::ZERO,
                max_scroll_extent: 0.0,
                last_tap_down_position: None,
                last_secondary_tap_down_position: None,
                layout_generation: 0,
                selection_start_in_viewport,
                selection_end_in_viewport,
            },
        )
    }

    fn fonts(self: RenderHandle<Self>, app: &mut App) -> Handle<FontCollection> {
        match self.get(app).fonts_override {
            Some(fonts) => fonts,
            None => PaintingBinding::instance(app).fonts(app),
        }
    }

    fn painter_with_fonts(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> (&mut TextPainter, &mut FontCollection) {
        let fonts = self.fonts(app);
        let (this, fonts) = app.get_disjoint_mut(self.handle(), fonts);
        (&mut this.text_painter, fonts)
    }

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
        intrinsics.set_text_width_basis(painter.text_width_basis());
        intrinsics.set_text_height_behavior(painter.text_height_behavior());
        (intrinsics, fonts)
    }

    fn mark_needs_paint_listener(self: RenderHandle<Self>) -> Listener {
        Listener::handle_method(self.handle(), offset_or_cursor_changed)
    }

    /// The text to display.
    pub fn text(self: RenderHandle<Self>, app: &App) -> Option<InlineSpanRef> {
        self.get(app).text_painter.text().cloned()
    }

    /// Sets [`text`](Self::text).
    pub fn set_text(self: RenderHandle<Self>, app: &mut App, value: Option<InlineSpanRef>) {
        let current = self.get(app).text_painter.text().cloned();
        if current.as_ref().map(|s| s.as_ref() as *const _)
            == value.as_ref().map(|s| s.as_ref() as *const _)
        {
            return;
        }
        self.get_mut(app).text_painter.set_text(value);
        self.get_mut(app).text_intrinsics = None;
        self.mark_needs_layout(app);
    }

    /// How the text should be aligned horizontally.
    pub fn text_align(self: RenderHandle<Self>, app: &App) -> TextAlign {
        self.get(app).text_painter.text_align()
    }

    /// Sets [`text_align`](Self::text_align).
    pub fn set_text_align(self: RenderHandle<Self>, app: &mut App, value: TextAlign) {
        if self.text_align(app) == value {
            return;
        }
        self.get_mut(app).text_painter.set_text_align(value);
        self.mark_needs_paint(app);
    }

    /// The directionality of the text.
    pub fn text_direction(self: RenderHandle<Self>, app: &App) -> TextDirection {
        self.get(app)
            .text_painter
            .text_direction()
            .expect("an editable always has a direction")
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(self: RenderHandle<Self>, app: &mut App, value: TextDirection) {
        if self.get(app).text_painter.text_direction() == Some(value) {
            return;
        }
        self.get_mut(app)
            .text_painter
            .set_text_direction(Some(value));
        self.mark_needs_layout(app);
    }

    /// The color to use when painting the cursor.
    pub fn cursor_color(self: RenderHandle<Self>, app: &App) -> Option<Color> {
        self.get(app).cursor_color
    }

    /// Sets [`cursor_color`](Self::cursor_color).
    pub fn set_cursor_color(self: RenderHandle<Self>, app: &mut App, value: Option<Color>) {
        if self.get(app).cursor_color == value {
            return;
        }
        self.get_mut(app).cursor_color = value;
        self.mark_needs_paint(app);
    }

    /// The color to use when painting the background cursor aligned with the text
    /// while rendering the floating cursor.
    pub fn background_cursor_color(self: RenderHandle<Self>, app: &App) -> Option<Color> {
        self.get(app).background_cursor_color
    }

    /// Sets [`background_cursor_color`](Self::background_cursor_color).
    pub fn set_background_cursor_color(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<Color>,
    ) {
        if self.get(app).background_cursor_color == value {
            return;
        }
        self.get_mut(app).background_cursor_color = value;
        self.mark_needs_paint(app);
    }

    /// Whether to paint the cursor.
    pub fn show_cursor(self: RenderHandle<Self>, app: &App) -> Handle<ValueNotifier<bool>> {
        self.get(app).show_cursor
    }

    /// Sets [`show_cursor`](Self::show_cursor).
    pub fn set_show_cursor(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Handle<ValueNotifier<bool>>,
    ) {
        let old = self.get(app).show_cursor;
        if old == value {
            return;
        }
        if self.attached(app) {
            old.remove_listener(app, &self.mark_needs_paint_listener());
        }
        self.get_mut(app).show_cursor = value;
        self.get_mut(app).dispose_show_cursor = false;
        if self.attached(app) {
            value.add_listener(app, self.mark_needs_paint_listener());
        }
        self.mark_needs_paint(app);
    }

    /// Whether the editable is currently focused.
    pub fn has_focus(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).has_focus
    }

    /// Sets [`has_focus`](Self::has_focus).
    pub fn set_has_focus(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).has_focus == value {
            return;
        }
        self.get_mut(app).has_focus = value;
    }

    /// The [`LayerLink`] of start selection handle.
    pub fn start_handle_layer_link(self: RenderHandle<Self>, app: &App) -> Handle<LayerLink> {
        self.get(app).start_handle_layer_link
    }

    /// Sets [`start_handle_layer_link`](Self::start_handle_layer_link).
    pub fn set_start_handle_layer_link(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Handle<LayerLink>,
    ) {
        if self.get(app).start_handle_layer_link == value {
            return;
        }
        self.get_mut(app).start_handle_layer_link = value;
        self.mark_needs_paint(app);
    }

    /// The [`LayerLink`] of end selection handle.
    pub fn end_handle_layer_link(self: RenderHandle<Self>, app: &App) -> Handle<LayerLink> {
        self.get(app).end_handle_layer_link
    }

    /// Sets [`end_handle_layer_link`](Self::end_handle_layer_link).
    pub fn set_end_handle_layer_link(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Handle<LayerLink>,
    ) {
        if self.get(app).end_handle_layer_link == value {
            return;
        }
        self.get_mut(app).end_handle_layer_link = value;
        self.mark_needs_paint(app);
    }

    /// The maximum number of lines for the text to span, wrapping if necessary.
    pub fn max_lines(self: RenderHandle<Self>, app: &App) -> Option<i32> {
        self.get(app).max_lines
    }

    /// Sets [`max_lines`](Self::max_lines).
    pub fn set_max_lines(self: RenderHandle<Self>, app: &mut App, value: Option<i32>) {
        debug_assert!(value.is_none_or(|n| n > 0));
        if self.get(app).max_lines == value {
            return;
        }
        self.get_mut(app).max_lines = value;
        self.get_mut(app)
            .text_painter
            .set_max_lines(if value == Some(1) { Some(1) } else { None });
        self.get_mut(app).text_intrinsics = None;
        self.mark_needs_layout(app);
    }

    /// The minimum number of lines to occupy when the content spans fewer lines.
    pub fn min_lines(self: RenderHandle<Self>, app: &App) -> Option<i32> {
        self.get(app).min_lines
    }

    /// Sets [`min_lines`](Self::min_lines).
    pub fn set_min_lines(self: RenderHandle<Self>, app: &mut App, value: Option<i32>) {
        debug_assert!(value.is_none_or(|n| n > 0));
        if self.get(app).min_lines == value {
            return;
        }
        self.get_mut(app).min_lines = value;
        self.mark_needs_layout(app);
    }

    /// Whether this widget's height will be sized to fill its parent.
    pub fn expands(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).expands
    }

    /// Sets [`expands`](Self::expands).
    pub fn set_expands(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).expands == value {
            return;
        }
        self.get_mut(app).expands = value;
        self.mark_needs_layout(app);
    }

    /// The color to use when painting the selection.
    pub fn selection_color(self: RenderHandle<Self>, app: &App) -> Option<Color> {
        self.get(app).selection_color
    }

    /// Sets [`selection_color`](Self::selection_color).
    pub fn set_selection_color(self: RenderHandle<Self>, app: &mut App, value: Option<Color>) {
        if self.get(app).selection_color == value {
            return;
        }
        self.get_mut(app).selection_color = value;
        self.mark_needs_paint(app);
    }

    /// The font scaling strategy to use when laying out and rendering the text.
    pub fn text_scaler(self: RenderHandle<Self>, app: &App) -> TextScaler {
        self.get(app).text_painter.text_scaler().clone()
    }

    /// Sets [`text_scaler`](Self::text_scaler).
    pub fn set_text_scaler(self: RenderHandle<Self>, app: &mut App, value: TextScaler) {
        if self.get(app).text_painter.text_scaler() == &value {
            return;
        }
        self.get_mut(app).text_painter.set_text_scaler(value);
        self.get_mut(app).text_intrinsics = None;
        self.mark_needs_layout(app);
    }

    /// The region of text that is selected, if any.
    pub fn selection(self: RenderHandle<Self>, app: &App) -> Option<TextSelection> {
        self.get(app).selection
    }

    /// Sets [`selection`](Self::selection).
    pub fn set_selection(self: RenderHandle<Self>, app: &mut App, value: Option<TextSelection>) {
        if self.get(app).selection == value {
            return;
        }
        self.get_mut(app).selection = value;
        self.mark_needs_paint(app);
    }

    /// The offset at which the text should be painted.
    pub fn offset(self: RenderHandle<Self>, app: &App) -> AnyViewportOffset {
        self.get(app).offset
    }

    /// Sets [`offset`](Self::offset).
    pub fn set_offset(self: RenderHandle<Self>, app: &mut App, value: AnyViewportOffset) {
        let old = self.get(app).offset;
        if old == value {
            return;
        }
        if self.attached(app) {
            old.remove_listener(app, &self.mark_needs_paint_listener());
        }
        self.get_mut(app).offset = value;
        if self.attached(app) {
            value.add_listener(app, self.mark_needs_paint_listener());
        }
        self.mark_needs_layout(app);
    }

    /// Whether the text can be changed.
    pub fn read_only(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).read_only
    }

    /// Sets [`read_only`](Self::read_only).
    pub fn set_read_only(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).read_only == value {
            return;
        }
        self.get_mut(app).read_only = value;
    }

    /// Whether this rendering object will take a full line regardless the text width.
    pub fn force_line(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).force_line
    }

    /// Sets [`force_line`](Self::force_line).
    pub fn set_force_line(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).force_line == value {
            return;
        }
        self.get_mut(app).force_line = value;
        self.mark_needs_layout(app);
    }

    /// {@macro dart.ui.textHeightBehavior}
    pub fn text_height_behavior(self: RenderHandle<Self>, app: &App) -> Option<TextHeightBehavior> {
        self.get(app).text_painter.text_height_behavior()
    }

    /// Sets [`text_height_behavior`](Self::text_height_behavior).
    pub fn set_text_height_behavior(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TextHeightBehavior>,
    ) {
        if self.text_height_behavior(app) == value {
            return;
        }
        self.get_mut(app)
            .text_painter
            .set_text_height_behavior(value);
        self.mark_needs_layout(app);
    }

    /// {@macro flutter.painting.textPainter.textWidthBasis}
    pub fn text_width_basis(self: RenderHandle<Self>, app: &App) -> TextWidthBasis {
        self.get(app).text_painter.text_width_basis()
    }

    /// Sets [`text_width_basis`](Self::text_width_basis).
    pub fn set_text_width_basis(self: RenderHandle<Self>, app: &mut App, value: TextWidthBasis) {
        if self.text_width_basis(app) == value {
            return;
        }
        self.get_mut(app).text_painter.set_text_width_basis(value);
        self.mark_needs_layout(app);
    }

    /// Character used for obscuring text if [`obscure_text`](Self::obscure_text) is true.
    pub fn obscuring_character(self: RenderHandle<Self>, app: &App) -> &str {
        &self.get(app).obscuring_character
    }

    /// Sets [`obscuring_character`](Self::obscuring_character).
    pub fn set_obscuring_character(
        self: RenderHandle<Self>,
        app: &mut App,
        value: impl Into<String>,
    ) {
        let value = value.into();
        if self.get(app).obscuring_character == value {
            return;
        }
        self.get_mut(app).obscuring_character = value;
        self.mark_needs_layout(app);
    }

    /// Whether to hide the text being edited (e.g., for passwords).
    pub fn obscure_text(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).obscure_text
    }

    /// Sets [`obscure_text`](Self::obscure_text).
    pub fn set_obscure_text(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).obscure_text == value {
            return;
        }
        self.get_mut(app).obscure_text = value;
    }

    /// How wide the cursor is.
    pub fn cursor_width(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).cursor_width
    }

    /// Sets [`cursor_width`](Self::cursor_width).
    pub fn set_cursor_width(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).cursor_width == value {
            return;
        }
        self.get_mut(app).cursor_width = value;
        self.mark_needs_layout(app);
    }

    /// How tall the cursor will be.
    pub fn cursor_height(self: RenderHandle<Self>, app: &mut App) -> f64 {
        self.get(app)
            .cursor_height
            .unwrap_or_else(|| self.preferred_line_height(app))
    }

    /// Sets the explicit cursor height; `None` uses [`preferred_line_height`](Self::preferred_line_height).
    pub fn set_cursor_height(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        if self.get(app).cursor_height == value {
            return;
        }
        self.get_mut(app).cursor_height = value;
        self.mark_needs_layout(app);
    }

    /// The radius of the cursor.
    pub fn cursor_radius(self: RenderHandle<Self>, app: &App) -> Option<Radius> {
        self.get(app).cursor_radius
    }

    /// Sets [`cursor_radius`](Self::cursor_radius).
    pub fn set_cursor_radius(self: RenderHandle<Self>, app: &mut App, value: Option<Radius>) {
        if self.get(app).cursor_radius == value {
            return;
        }
        self.get_mut(app).cursor_radius = value;
        self.mark_needs_paint(app);
    }

    /// Whether the cursor paints above the text.
    pub fn paint_cursor_above_text(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).paint_cursor_above_text
    }

    /// Sets [`paint_cursor_above_text`](Self::paint_cursor_above_text).
    pub fn set_paint_cursor_above_text(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).paint_cursor_above_text == value {
            return;
        }
        self.get_mut(app).paint_cursor_above_text = value;
        self.mark_needs_paint(app);
    }

    /// The offset that is used, in pixels, when painting the cursor on screen.
    pub fn cursor_offset(self: RenderHandle<Self>, app: &App) -> Offset {
        self.get(app).cursor_offset
    }

    /// Sets [`cursor_offset`](Self::cursor_offset).
    pub fn set_cursor_offset(self: RenderHandle<Self>, app: &mut App, value: Offset) {
        if self.get(app).cursor_offset == value {
            return;
        }
        self.get_mut(app).cursor_offset = value;
        self.mark_needs_paint(app);
    }

    /// The pixel ratio of the current device.
    pub fn device_pixel_ratio(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).device_pixel_ratio
    }

    /// Sets [`device_pixel_ratio`](Self::device_pixel_ratio).
    pub fn set_device_pixel_ratio(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).device_pixel_ratio == value {
            return;
        }
        self.get_mut(app).device_pixel_ratio = value;
        self.mark_needs_layout(app);
    }

    /// Controls how tall the selection highlight boxes are computed to be.
    pub fn selection_height_style(self: RenderHandle<Self>, app: &App) -> BoxHeightStyle {
        self.get(app).selection_height_style
    }

    /// Sets [`selection_height_style`](Self::selection_height_style).
    pub fn set_selection_height_style(
        self: RenderHandle<Self>,
        app: &mut App,
        value: BoxHeightStyle,
    ) {
        if self.get(app).selection_height_style == value {
            return;
        }
        self.get_mut(app).selection_height_style = value;
        self.mark_needs_paint(app);
    }

    /// Controls how wide the selection highlight boxes are computed to be.
    pub fn selection_width_style(self: RenderHandle<Self>, app: &App) -> BoxWidthStyle {
        self.get(app).selection_width_style
    }

    /// Sets [`selection_width_style`](Self::selection_width_style).
    pub fn set_selection_width_style(
        self: RenderHandle<Self>,
        app: &mut App,
        value: BoxWidthStyle,
    ) {
        if self.get(app).selection_width_style == value {
            return;
        }
        self.get_mut(app).selection_width_style = value;
        self.mark_needs_paint(app);
    }

    /// Whether to allow the user to change the selection.
    pub fn enable_interactive_selection(self: RenderHandle<Self>, app: &App) -> Option<bool> {
        self.get(app).enable_interactive_selection
    }

    /// Sets [`enable_interactive_selection`](Self::enable_interactive_selection).
    pub fn set_enable_interactive_selection(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<bool>,
    ) {
        if self.get(app).enable_interactive_selection == value {
            return;
        }
        self.get_mut(app).enable_interactive_selection = value;
        self.mark_needs_paint(app);
    }

    /// The [`TextSelectionDelegate`](reveal_services::TextSelectionDelegate) for
    /// this [`RenderEditable`].
    pub fn text_selection_delegate(
        self: RenderHandle<Self>,
        app: &App,
    ) -> AnyTextSelectionDelegate {
        self.get(app).text_selection_delegate
    }

    /// Sets [`text_selection_delegate`](Self::text_selection_delegate).
    pub fn set_text_selection_delegate(
        self: RenderHandle<Self>,
        app: &mut App,
        value: AnyTextSelectionDelegate,
    ) {
        self.get_mut(app).text_selection_delegate = value;
    }

    /// The color of the prompt rectangle for a pending autocorrection.
    pub fn prompt_rect_color(self: RenderHandle<Self>, app: &App) -> Option<Color> {
        self.get(app).prompt_rect_color
    }

    /// Sets [`prompt_rect_color`](Self::prompt_rect_color).
    pub fn set_prompt_rect_color(self: RenderHandle<Self>, app: &mut App, value: Option<Color>) {
        if self.get(app).prompt_rect_color == value {
            return;
        }
        self.get_mut(app).prompt_rect_color = value;
        self.mark_needs_paint(app);
    }

    /// Sets the range of text that will be changed by a pending autocorrection.
    pub fn set_prompt_rect_range(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TextRange>,
    ) {
        if self.get(app).prompt_rect_range == value {
            return;
        }
        self.get_mut(app).prompt_rect_range = value;
        self.mark_needs_paint(app);
    }

    /// The content will be clipped (or not) according to this option.
    pub fn clip_behavior(self: RenderHandle<Self>, app: &App) -> Clip {
        self.get(app).clip_behavior
    }

    /// Sets [`clip_behavior`](Self::clip_behavior).
    pub fn set_clip_behavior(self: RenderHandle<Self>, app: &mut App, value: Clip) {
        if self.get(app).clip_behavior == value {
            return;
        }
        self.get_mut(app).clip_behavior = value;
        self.mark_needs_paint(app);
    }

    /// Track whether the start of the selection is currently in the visible
    /// viewport.
    pub fn selection_start_in_viewport(
        self: RenderHandle<Self>,
        app: &App,
    ) -> Handle<ValueNotifier<bool>> {
        self.get(app).selection_start_in_viewport
    }

    /// Track whether the end of the selection is currently in the visible
    /// viewport.
    pub fn selection_end_in_viewport(
        self: RenderHandle<Self>,
        app: &App,
    ) -> Handle<ValueNotifier<bool>> {
        self.get(app).selection_end_in_viewport
    }

    fn update_selection_extents_visibility(
        self: RenderHandle<Self>,
        app: &mut App,
        effective_offset: Offset,
    ) {
        let Some(selection) = self.selection(app) else {
            return;
        };
        let start_notifier = self.get(app).selection_start_in_viewport;
        let end_notifier = self.get(app).selection_end_in_viewport;
        if !selection.is_valid() {
            start_notifier.set_value(app, false);
            end_notifier.set_value(app, false);
            return;
        }
        let visible_region = Offset::ZERO & self.size(app);
        let caret_prototype = self.get(app).caret_prototype;
        let start = TextPosition::with_affinity(selection.start(), selection.affinity);
        let end = TextPosition::with_affinity(selection.end(), selection.affinity);
        let (start_offset, end_offset) = {
            let (painter, fonts) = self.painter_with_fonts(app);
            (
                painter.get_offset_for_caret(fonts, start, caret_prototype),
                painter.get_offset_for_caret(fonts, end, caret_prototype),
            )
        };
        const VISIBLE_REGION_SLOP: f64 = 0.5;
        let region = visible_region.inflate(VISIBLE_REGION_SLOP);
        start_notifier.set_value(app, region.contains(start_offset + effective_offset));
        end_notifier.set_value(app, region.contains(end_offset + effective_offset));
    }

    /// If [`obscure_text`](Self::obscure_text) is true, returns the obscured text.
    pub fn plain_text(self: RenderHandle<Self>, app: &mut App) -> String {
        self.get_mut(app).text_painter.plain_text().to_owned()
    }

    /// An estimate of the height of a line in the text.
    pub fn preferred_line_height(self: RenderHandle<Self>, app: &mut App) -> f64 {
        let (painter, fonts) = self.painter_with_fonts(app);
        painter.preferred_line_height(fonts)
    }

    fn caret_margin(self: RenderHandle<Self>, app: &App) -> f64 {
        K_CARET_GAP + self.get(app).cursor_width
    }

    fn is_multiline(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).max_lines != Some(1)
    }

    fn paint_offset(self: RenderHandle<Self>, app: &App) -> Offset {
        let pixels = self.get(app).offset.pixels(app);
        if self.is_multiline(app) {
            Offset::new(0.0, -pixels)
        } else {
            Offset::new(-pixels, 0.0)
        }
    }

    fn viewport_extent(self: RenderHandle<Self>, app: &App) -> f64 {
        let size = self.size(app);
        if self.is_multiline(app) {
            size.height()
        } else {
            size.width()
        }
    }

    fn get_max_scroll_extent(self: RenderHandle<Self>, app: &App, content_size: Size) -> f64 {
        let size = self.size(app);
        if self.is_multiline(app) {
            (content_size.height() - size.height()).max(0.0)
        } else {
            (content_size.width() - size.width()).max(0.0)
        }
    }

    fn has_visual_overflow(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).max_scroll_extent > 0.0 || self.paint_offset(app) != Offset::ZERO
    }

    fn adjust_constraints(
        self: RenderHandle<Self>,
        app: &App,
        min_width: f64,
        max_width: f64,
    ) -> (f64, f64) {
        let available_max_width = (max_width - self.caret_margin(app)).max(0.0);
        let available_min_width = min_width.min(available_max_width);
        let min = if self.get(app).force_line {
            available_max_width
        } else {
            available_min_width
        };
        let max = if self.is_multiline(app) {
            available_max_width
        } else {
            f64::INFINITY
        };
        (min, max)
    }

    fn compute_text_metrics_if_needed(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let (min_width, max_width) =
            self.adjust_constraints(app, constraints.min_width, constraints.max_width);
        let (painter, fonts) = self.painter_with_fonts(app);
        painter.layout(fonts, min_width, max_width);
    }

    fn compute_caret_prototype(self: RenderHandle<Self>, app: &mut App) {
        let cursor_width = self.get(app).cursor_width;
        let cursor_height = self.cursor_height(app);
        let prototype = match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::MacOS => {
                Rect::from_ltwh(0.0, 0.0, cursor_width, cursor_height + 2.0)
            }
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => Rect::from_ltwh(
                0.0,
                K_CARET_HEIGHT_OFFSET,
                cursor_width,
                cursor_height - 2.0 * K_CARET_HEIGHT_OFFSET,
            ),
        };
        self.get_mut(app).caret_prototype = prototype;
    }

    fn snap_to_physical_pixel(
        self: RenderHandle<Self>,
        app: &App,
        source_offset: Offset,
    ) -> Offset {
        let global = self.as_box().local_to_global(app, source_offset, None);
        let pixel_multiple = 1.0 / self.get(app).device_pixel_ratio;
        Offset::new(
            if global.dx().is_finite() {
                (global.dx() / pixel_multiple).round() * pixel_multiple - global.dx()
            } else {
                0.0
            },
            if global.dy().is_finite() {
                (global.dy() / pixel_multiple).round() * pixel_multiple - global.dy()
            } else {
                0.0
            },
        )
    }

    /// Returns the local coordinates of the endpoints of the given selection.
    pub fn get_endpoints_for_selection(
        self: RenderHandle<Self>,
        app: &mut App,
        selection: TextSelection,
    ) -> Vec<TextSelectionPoint> {
        self.compute_text_metrics_if_needed(app);
        let paint_offset = self.paint_offset(app);
        let height_style = self.get(app).selection_height_style;
        let width_style = self.get(app).selection_width_style;
        let boxes = if selection.is_collapsed() {
            Vec::new()
        } else {
            self.get(app)
                .text_painter
                .get_boxes_for_selection(selection, height_style, width_style)
        };
        if boxes.is_empty() {
            let caret_prototype = self.get(app).caret_prototype;
            let (painter, fonts) = self.painter_with_fonts(app);
            let caret_offset =
                painter.get_offset_for_caret(fonts, selection.extent(), caret_prototype);
            let line_height = painter.preferred_line_height(fonts);
            let start = Offset::new(0.0, line_height) + caret_offset + paint_offset;
            return vec![TextSelectionPoint::new(start, None)];
        }
        let painter_width = self.get(app).text_painter.size().width();
        let start = Offset::new(
            clamp_double(boxes[0].start(), 0.0, painter_width),
            boxes[0].bottom,
        ) + paint_offset;
        let last = boxes[boxes.len() - 1];
        let end =
            Offset::new(clamp_double(last.end(), 0.0, painter_width), last.bottom) + paint_offset;
        vec![
            TextSelectionPoint::new(start, Some(boxes[0].direction)),
            TextSelectionPoint::new(end, Some(last.direction)),
        ]
    }

    /// Returns the smallest [`Rect`], in the local coordinate system, that covers
    /// the text within the [`TextRange`] specified.
    pub fn get_rect_for_composing_range(
        self: RenderHandle<Self>,
        app: &mut App,
        range: TextRange,
    ) -> Option<Rect> {
        if !range.is_valid() || range.is_collapsed() {
            return None;
        }
        self.compute_text_metrics_if_needed(app);
        let boxes = self.get(app).text_painter.get_boxes_for_selection(
            TextSelection::new(range.start, range.end),
            self.get(app).selection_height_style,
            self.get(app).selection_width_style,
        );
        if boxes.is_empty() {
            return None;
        }
        let paint_offset = self.paint_offset(app);
        let mut rect = boxes[0].to_rect();
        for box_ in boxes.iter().skip(1) {
            rect = rect.expand_to_include(box_.to_rect());
        }
        Some(rect.shift(paint_offset))
    }

    /// Returns the [`TextPosition`] closest to the given global point.
    pub fn get_position_for_point(
        self: RenderHandle<Self>,
        app: &mut App,
        global_position: Offset,
    ) -> TextPosition {
        self.compute_text_metrics_if_needed(app);
        let local =
            self.as_box().global_to_local(app, global_position, None) - self.paint_offset(app);
        self.get(app).text_painter.get_position_for_offset(local)
    }

    /// Returns the [`Rect`] in local coordinates for the caret at the given text
    /// position.
    pub fn get_local_rect_for_caret(
        self: RenderHandle<Self>,
        app: &mut App,
        caret_position: TextPosition,
    ) -> Rect {
        self.compute_text_metrics_if_needed(app);
        let caret_prototype = self.get(app).caret_prototype;
        let cursor_offset = self.get(app).cursor_offset;
        let (painter, fonts) = self.painter_with_fonts(app);
        let caret_offset = painter.get_offset_for_caret(fonts, caret_position, caret_prototype);
        let mut caret_rect = caret_prototype.shift(caret_offset + cursor_offset);
        let painter_width = painter.width();
        let size = self.size(app);
        let caret_margin = self.caret_margin(app);
        let scrollable_width = (painter_width + caret_margin).max(size.width());
        let caret_x = clamp_double(
            caret_rect.left,
            0.0,
            (scrollable_width - caret_margin).max(0.0),
        );
        caret_rect = Offset::new(caret_x, caret_rect.top) & caret_rect.size();
        let (painter, fonts) = self.painter_with_fonts(app);
        let full_height = painter.get_full_height_for_caret(fonts, caret_position, caret_prototype);
        caret_rect = match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::MacOS => {
                let height_diff = full_height - caret_rect.height();
                Rect::from_ltwh(
                    caret_rect.left,
                    caret_rect.top + height_diff / 2.0,
                    caret_rect.width(),
                    caret_rect.height(),
                )
            }
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => {
                let caret_height = self.cursor_height(app);
                let height_diff = full_height - caret_height;
                Rect::from_ltwh(
                    caret_rect.left,
                    caret_rect.top - K_CARET_HEIGHT_OFFSET + height_diff / 2.0,
                    caret_rect.width(),
                    caret_height,
                )
            }
        };
        caret_rect = caret_rect.shift(self.paint_offset(app));
        caret_rect.shift(self.snap_to_physical_pixel(app, caret_rect.top_left()))
    }

    fn set_text_editing_value(
        self: RenderHandle<Self>,
        app: &mut App,
        new_value: reveal_embedder::TextEditingValue,
        cause: SelectionChangedCause,
    ) {
        self.get(app)
            .text_selection_delegate
            .user_update_text_editing_value(app, new_value, cause);
    }

    fn apply_selection(
        self: RenderHandle<Self>,
        app: &mut App,
        mut next_selection: TextSelection,
        cause: SelectionChangedCause,
    ) {
        if next_selection.is_valid() {
            let text_length = utf16_len(
                &self
                    .get(app)
                    .text_selection_delegate
                    .text_editing_value(app)
                    .text,
            );
            next_selection = next_selection
                .copy_with()
                .base_offset(next_selection.base_offset.min(text_length))
                .extent_offset(next_selection.extent_offset.min(text_length));
        }
        let value = self
            .get(app)
            .text_selection_delegate
            .text_editing_value(app)
            .copy_with()
            .selection(next_selection);
        self.set_text_editing_value(app, value, cause);
    }

    /// If [`ignore_pointer`](Self::ignore_pointer) is false then this method is called by
    /// the internal gesture recognizer. When it is true, an ancestor must call it.
    pub fn handle_tap_down(self: RenderHandle<Self>, _app: &mut App, details: &TapDownDetails) {
        self.get_mut(_app).last_tap_down_position = Some(details.global_position);
    }

    /// The position of the most recent secondary tap down event on this text
    /// input.
    pub fn last_secondary_tap_down_position(self: RenderHandle<Self>, app: &App) -> Option<Offset> {
        self.get(app).last_secondary_tap_down_position
    }

    /// Tracks the position of a secondary tap event.
    ///
    /// Should be called before attempting to change the selection based on the
    /// position of a secondary tap.
    pub fn handle_secondary_tap_down(
        self: RenderHandle<Self>,
        app: &mut App,
        details: &TapDownDetails,
    ) {
        self.get_mut(app).last_tap_down_position = Some(details.global_position);
        self.get_mut(app).last_secondary_tap_down_position = Some(details.global_position);
    }

    /// Select the word at the last tap-down, or the position if the cause is a tap.
    pub fn handle_tap(self: RenderHandle<Self>, app: &mut App) {
        self.select_position(app, SelectionChangedCause::Tap);
    }

    /// Select the word around the last tap-down.
    pub fn handle_double_tap(self: RenderHandle<Self>, app: &mut App) {
        self.select_word(app, SelectionChangedCause::DoubleTap);
    }

    /// Select the word around the last long-press.
    pub fn handle_long_press(self: RenderHandle<Self>, app: &mut App) {
        self.select_word(app, SelectionChangedCause::LongPress);
    }

    /// Move the selection to the location of the last tap down.
    pub fn select_position(self: RenderHandle<Self>, app: &mut App, cause: SelectionChangedCause) {
        let from = self
            .get(app)
            .last_tap_down_position
            .expect("select_position needs a tap-down");
        self.select_position_at(app, from, None, cause);
    }

    /// Select text between the global positions `from` and `to`.
    pub fn select_position_at(
        self: RenderHandle<Self>,
        app: &mut App,
        from: Offset,
        to: Option<Offset>,
        cause: SelectionChangedCause,
    ) {
        self.compute_text_metrics_if_needed(app);
        let paint_offset = self.paint_offset(app);
        let from_position = self
            .get(app)
            .text_painter
            .get_position_for_offset(self.as_box().global_to_local(app, from, None) - paint_offset);
        let to_position = to.map(|to| {
            self.get(app).text_painter.get_position_for_offset(
                self.as_box().global_to_local(app, to, None) - paint_offset,
            )
        });
        let new_selection = TextSelection {
            base_offset: from_position.offset,
            extent_offset: to_position.unwrap_or(from_position).offset,
            affinity: from_position.affinity,
            is_directional: false,
        };
        self.apply_selection(app, new_selection, cause);
    }

    /// Select a word around the location of the last tap down.
    pub fn select_word(self: RenderHandle<Self>, app: &mut App, cause: SelectionChangedCause) {
        let from = self
            .get(app)
            .last_tap_down_position
            .expect("select_word needs a tap-down");
        self.select_words_in_range(app, from, None, cause);
    }

    /// Selects the set words of a paragraph that intersect a given range of global positions.
    pub fn select_words_in_range(
        self: RenderHandle<Self>,
        app: &mut App,
        from: Offset,
        to: Option<Offset>,
        cause: SelectionChangedCause,
    ) {
        self.compute_text_metrics_if_needed(app);
        let paint_offset = self.paint_offset(app);
        let from_position = self
            .get(app)
            .text_painter
            .get_position_for_offset(self.as_box().global_to_local(app, from, None) - paint_offset);
        let from_word = self.get_word_at_offset(app, from_position);
        let to_position = match to {
            None => from_position,
            Some(to) => self.get(app).text_painter.get_position_for_offset(
                self.as_box().global_to_local(app, to, None) - paint_offset,
            ),
        };
        let to_word = if to_position == from_position {
            from_word
        } else {
            self.get_word_at_offset(app, to_position)
        };
        let is_from_word_before_to_word = from_word.start() < to_word.end();
        self.apply_selection(
            app,
            TextSelection {
                base_offset: if is_from_word_before_to_word {
                    from_word.base().offset
                } else {
                    from_word.extent().offset
                },
                extent_offset: if is_from_word_before_to_word {
                    to_word.extent().offset
                } else {
                    to_word.base().offset
                },
                affinity: from_word.affinity,
                is_directional: false,
            },
            cause,
        );
    }

    /// Move the selection to the beginning or end of a word.
    pub fn select_word_edge(self: RenderHandle<Self>, app: &mut App, cause: SelectionChangedCause) {
        self.compute_text_metrics_if_needed(app);
        let from = self
            .get(app)
            .last_tap_down_position
            .expect("select_word_edge needs a tap-down");
        let paint_offset = self.paint_offset(app);
        let position = self
            .get(app)
            .text_painter
            .get_position_for_offset(self.as_box().global_to_local(app, from, None) - paint_offset);
        let word = self.get(app).text_painter.get_word_boundary(position);
        let new_selection = if position.offset <= word.start {
            TextSelection::collapsed(word.start, TextAffinity::Downstream)
        } else {
            TextSelection::collapsed(word.end, TextAffinity::Upstream)
        };
        self.apply_selection(app, new_selection, cause);
    }

    /// Returns a [`TextSelection`] that encompasses the word at the given
    /// [`TextPosition`].
    pub fn get_word_at_offset(
        self: RenderHandle<Self>,
        app: &mut App,
        position: TextPosition,
    ) -> TextSelection {
        let plain = self.plain_text(app);
        let length = utf16_len(&plain);
        if position.offset >= length {
            return TextSelection::from_position(TextPosition::with_affinity(
                length,
                TextAffinity::Upstream,
            ));
        }
        if self.get(app).obscure_text {
            return TextSelection::new(0, length);
        }
        let word = self.get(app).text_painter.get_word_boundary(position);
        let effective_offset = match position.affinity {
            TextAffinity::Upstream => position.offset - 1,
            TextAffinity::Downstream => position.offset,
        };
        debug_assert!(effective_offset >= 0);
        if effective_offset > 0
            && code_unit_at(&plain, effective_offset).is_some_and(TextLayoutMetrics::is_whitespace)
        {
            if let Some(previous_word) = self.get_previous_word(app, word.start) {
                match app.platform().target_platform() {
                    TargetPlatform::IOS => {
                        return TextSelection::new(previous_word.start, position.offset);
                    }
                    TargetPlatform::Android if self.get(app).read_only => {
                        return TextSelection::new(previous_word.start, position.offset);
                    }
                    _ => {}
                }
            } else {
                match app.platform().target_platform() {
                    TargetPlatform::IOS => {
                        if let Some(next_word) = self.get_next_word(app, word.start) {
                            return TextSelection::new(position.offset, next_word.end);
                        }
                        return TextSelection::collapsed(position.offset, position.affinity);
                    }
                    TargetPlatform::Android if self.get(app).read_only => {
                        return TextSelection::new(position.offset, position.offset + 1);
                    }
                    _ => {}
                }
            }
        }
        TextSelection::new(word.start, word.end)
    }

    fn get_next_word(
        self: RenderHandle<Self>,
        app: &mut App,
        mut offset: i32,
    ) -> Option<TextRange> {
        loop {
            let range = self
                .get(app)
                .text_painter
                .get_word_boundary(TextPosition::new(offset));
            if !range.is_valid() || range.is_collapsed() {
                return None;
            }
            if !self.only_whitespace(app, range) {
                return Some(range);
            }
            offset = range.end;
        }
    }

    fn get_previous_word(
        self: RenderHandle<Self>,
        app: &mut App,
        mut offset: i32,
    ) -> Option<TextRange> {
        while offset >= 0 {
            let range = self
                .get(app)
                .text_painter
                .get_word_boundary(TextPosition::new(offset));
            if !range.is_valid() || range.is_collapsed() {
                return None;
            }
            if !self.only_whitespace(app, range) {
                return Some(range);
            }
            offset = range.start - 1;
        }
        None
    }

    fn only_whitespace(self: RenderHandle<Self>, app: &mut App, range: TextRange) -> bool {
        let plain = self.plain_text(app);
        for i in range.start..range.end {
            if let Some(unit) = code_unit_at(&plain, i)
                && !TextLayoutMetrics::is_whitespace(unit)
            {
                return false;
            }
        }
        true
    }

    /// Return a [`TextSelection`] containing the line of the given [`TextPosition`].
    pub fn get_line_at_offset(
        self: RenderHandle<Self>,
        app: &mut App,
        position: TextPosition,
    ) -> TextSelection {
        self.compute_text_metrics_if_needed(app);
        let line = self.get(app).text_painter.get_line_boundary(position);
        if self.get(app).obscure_text {
            return TextSelection::new(0, utf16_len(&self.plain_text(app)));
        }
        TextSelection::new(line.start, line.end)
    }

    /// A [`WordBoundary`] over this render object's text and layout.
    ///
    /// Dart's `RenderEditable.wordBoundaries`, which forwards to the text painter's.
    pub fn word_boundaries(self: RenderHandle<Self>, app: &App) -> WordBoundary {
        WordBoundary::new(
            self.text(app).expect("RenderEditable always has text"),
            self,
        )
    }

    /// See `TextPainter.get_word_boundary`.
    pub fn get_word_boundary(
        self: RenderHandle<Self>,
        app: &mut App,
        position: TextPosition,
    ) -> TextRange {
        self.compute_text_metrics_if_needed(app);
        self.get(app).text_painter.get_word_boundary(position)
    }

    /// Returns the TextPosition above the given offset into the text.
    pub fn get_text_position_above(
        self: RenderHandle<Self>,
        app: &mut App,
        position: TextPosition,
    ) -> TextPosition {
        self.compute_text_metrics_if_needed(app);
        let line_height = self.preferred_line_height(app);
        self.get_text_position_vertical(app, position, -0.5 * line_height)
    }

    /// Returns the TextPosition below the given offset into the text.
    pub fn get_text_position_below(
        self: RenderHandle<Self>,
        app: &mut App,
        position: TextPosition,
    ) -> TextPosition {
        self.compute_text_metrics_if_needed(app);
        let line_height = self.preferred_line_height(app);
        self.get_text_position_vertical(app, position, 1.5 * line_height)
    }

    fn get_text_position_vertical(
        self: RenderHandle<Self>,
        app: &mut App,
        position: TextPosition,
        vertical_offset: f64,
    ) -> TextPosition {
        let caret_prototype = self.get(app).caret_prototype;
        let (painter, fonts) = self.painter_with_fonts(app);
        let caret_offset = painter.get_offset_for_caret(fonts, position, caret_prototype);
        painter.get_position_for_offset(caret_offset.translate(0.0, vertical_offset))
    }

    /// Starts a [`VerticalCaretMovementRun`] at the given location in the text.
    pub fn start_vertical_caret_movement(
        self: RenderHandle<Self>,
        app: &mut App,
        start_position: TextPosition,
    ) -> VerticalCaretMovementRun {
        self.compute_text_metrics_if_needed(app);
        let metrics = self.get_mut(app).text_painter.compute_line_metrics();
        let (painter, fonts) = self.painter_with_fonts(app);
        let (line, offset) = line_number_for(painter, fonts, start_position, &metrics);
        VerticalCaretMovementRun {
            editable: self,
            layout_generation: self.get(app).layout_generation,
            current_offset: offset,
            current_line: line,
            current_text_position: start_position,
            line_metrics: metrics,
            position_cache: HashMap::new(),
            is_valid: true,
        }
    }

    fn paint_highlights(
        self: RenderHandle<Self>,
        app: &App,
        canvas: &mut reveal_embedder::Canvas,
        offset: Offset,
    ) {
        let paint_offset = offset + self.paint_offset(app);
        let painter_size = self.get(app).text_painter.size();
        let clip = Rect::from_ltwh(0.0, 0.0, painter_size.width(), painter_size.height());
        if let (Some(color), Some(range)) = (
            self.get(app).prompt_rect_color,
            self.get(app).prompt_rect_range,
        ) && !range.is_collapsed()
        {
            paint_range(
                canvas,
                &self.get(app).text_painter,
                range,
                color,
                paint_offset,
                clip,
                self.get(app).selection_height_style,
                self.get(app).selection_width_style,
            );
        }
        if let (Some(color), Some(selection)) =
            (self.get(app).selection_color, self.get(app).selection)
            && !selection.is_collapsed()
        {
            paint_range(
                canvas,
                &self.get(app).text_painter,
                selection.range(),
                color,
                paint_offset,
                clip,
                self.get(app).selection_height_style,
                self.get(app).selection_width_style,
            );
        }
    }

    fn paint_caret(
        self: RenderHandle<Self>,
        app: &mut App,
        canvas: &mut reveal_embedder::Canvas,
        offset: Offset,
    ) {
        let Some(selection) = self.get(app).selection else {
            return;
        };
        let show_cursor = self.get(app).show_cursor;
        if !selection.is_collapsed() || !*show_cursor.value(app) {
            return;
        }
        let Some(color) = self.get(app).cursor_color else {
            return;
        };
        let caret = self
            .get_local_rect_for_caret(app, selection.extent())
            .shift(offset);
        canvas.draw_rect(
            caret,
            &Paint {
                color: color.into(),
                ..Paint::default()
            },
        );
    }

    fn paint_contents(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let effective = offset + self.paint_offset(app);
        if self.selection(app).is_some() {
            self.update_selection_extents_visibility(app, effective);
        }
        let above = self.get(app).paint_cursor_above_text;
        if !above {
            self.paint_caret(app, context.canvas(), offset);
        }
        self.paint_highlights(app, context.canvas(), offset);
        let (painter, fonts) = self.painter_with_fonts(app);
        painter.paint(fonts, context.canvas(), effective);
        if above {
            self.paint_caret(app, context.canvas(), offset);
        }
    }

    fn preferred_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let max_lines = self.get(app).max_lines;
        let min_lines = self.get(app).min_lines.or(max_lines);
        let line_height = self.preferred_line_height(app);
        let min_height = line_height * f64::from(min_lines.unwrap_or(0));
        match max_lines {
            None => {
                let estimated = if width.is_infinite() {
                    line_height * f64::from(count_hard_line_breaks(&self.plain_text(app)) + 1)
                } else {
                    let (min_width, max_width) = self.adjust_constraints(app, 0.0, width);
                    let (intrinsics, fonts) = self.text_intrinsics_with_fonts(app);
                    intrinsics.layout(fonts, min_width, max_width);
                    intrinsics.height()
                };
                estimated.max(min_height)
            }
            Some(1) => {
                let (min_width, max_width) = self.adjust_constraints(app, 0.0, width);
                let (intrinsics, fonts) = self.text_intrinsics_with_fonts(app);
                intrinsics.layout(fonts, min_width, max_width);
                intrinsics.height()
            }
            Some(max) if min_lines == Some(max) => min_height,
            Some(max) => {
                let max_height = line_height * f64::from(max);
                let (min_width, max_width) = self.adjust_constraints(app, 0.0, width);
                let (intrinsics, fonts) = self.text_intrinsics_with_fonts(app);
                intrinsics.layout(fonts, min_width, max_width);
                clamp_double(intrinsics.height(), min_height, max_height)
            }
        }
    }
}

fn offset_or_cursor_changed(this: Handle<RenderEditable>, app: &mut App) {
    RenderHandle::from_handle(this).mark_needs_paint(app);
}

fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

fn code_unit_at(text: &str, offset: i32) -> Option<i32> {
    text.encode_utf16().nth(offset as usize).map(i32::from)
}

fn count_hard_line_breaks(text: &str) -> i32 {
    text.encode_utf16()
        .filter(|unit| matches!(*unit, 0x000A | 0x0085 | 0x000B | 0x000C | 0x2028 | 0x2029))
        .count() as i32
}

fn line_number_for(
    painter: &mut TextPainter,
    fonts: &mut FontCollection,
    start_position: TextPosition,
    metrics: &[LineMetrics],
) -> (i32, Offset) {
    let offset = painter.get_offset_for_caret(fonts, start_position, Rect::ZERO);
    for line in metrics {
        if line.baseline > offset.dy() {
            return (line.line_number, Offset::new(offset.dx(), line.baseline));
        }
    }
    let last = metrics.last();
    (
        last.map(|m| m.line_number).unwrap_or(0).max(0),
        Offset::new(
            offset.dx(),
            last.map(|m| m.baseline + m.descent).unwrap_or(0.0),
        ),
    )
}

#[allow(clippy::too_many_arguments)]
fn paint_range(
    canvas: &mut reveal_embedder::Canvas,
    painter: &TextPainter,
    range: TextRange,
    color: Color,
    paint_offset: Offset,
    clip: Rect,
    height_style: BoxHeightStyle,
    width_style: BoxWidthStyle,
) {
    if !range.is_valid() || range.is_collapsed() {
        return;
    }
    let boxes = painter.get_boxes_for_selection(
        TextSelection::new(range.start, range.end),
        height_style,
        width_style,
    );
    let paint = Paint {
        color: color.into(),
        ..Paint::default()
    };
    for box_ in boxes {
        let rect = box_
            .to_rect()
            .shift(paint_offset)
            .intersect(clip.shift(paint_offset));
        if !rect.is_empty() {
            canvas.draw_rect(rect, &paint);
        }
    }
}

impl RenderObject for RenderEditable {
    crate::render_object_accessors!();

    fn visit_children(
        self: RenderHandle<Self>,
        _app: &App,
        _visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        // A leaf until inline children and the custom-paint boxes are ported.
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, _owner: Handle<PipelineOwner>) {
        let offset = self.get(app).offset;
        offset.add_listener(app, self.mark_needs_paint_listener());
        let show_cursor = self.get(app).show_cursor;
        show_cursor.add_listener(app, self.mark_needs_paint_listener());
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        let offset = self.get(app).offset;
        offset.remove_listener(app, &self.mark_needs_paint_listener());
        let show_cursor = self.get(app).show_cursor;
        show_cursor.remove_listener(app, &self.mark_needs_paint_listener());
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        LayerHandle::set_layer(app, |app| &mut self.get_mut(app).leader_layer_handler, None);
        LayerHandle::set_layer(app, |app| &mut self.get_mut(app).clip_rect_layer, None);
        let start_in_viewport = self.get(app).selection_start_in_viewport;
        let end_in_viewport = self.get(app).selection_end_in_viewport;
        app.get_mut(start_in_viewport).dispose();
        app.get_mut(end_in_viewport).dispose();
        crate::object::RenderObjectBase::dispose(self, app);
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let (min_width, max_width) =
            self.adjust_constraints(app, constraints.min_width, constraints.max_width);
        let (painter, fonts) = self.painter_with_fonts(app);
        painter.layout(fonts, min_width, max_width);
        self.compute_caret_prototype(app);
        let caret_margin = self.caret_margin(app);
        let painter_width = self.get(app).text_painter.width();
        let painter_height = self.get(app).text_painter.height();
        let width = if self.get(app).force_line {
            constraints.max_width
        } else {
            constraints.constrain_width(painter_width + caret_margin)
        };
        let preferred_height = match self.get(app).max_lines {
            None => {
                let min_lines = self.get(app).min_lines.unwrap_or(0);
                painter_height.max(self.preferred_line_height(app) * f64::from(min_lines))
            }
            Some(1) => painter_height,
            Some(max_lines) => {
                let line_height = self.preferred_line_height(app);
                let min_lines = self.get(app).min_lines.unwrap_or(max_lines);
                clamp_double(
                    painter_height,
                    line_height * f64::from(min_lines),
                    line_height * f64::from(max_lines),
                )
            }
        };
        let size = Size::new(width, constraints.constrain_height(preferred_height));
        self.set_size(app, size);
        let content_size = Size::new(painter_width + caret_margin, painter_height);
        self.get_mut(app).max_scroll_extent = self.get_max_scroll_extent(app, content_size);
        self.get_mut(app).layout_generation += 1;
        let viewport_extent = self.viewport_extent(app);
        let max_scroll = self.get(app).max_scroll_extent;
        let offset = self.get(app).offset;
        offset.apply_viewport_dimension(app, viewport_extent);
        offset.apply_content_dimensions(app, 0.0, max_scroll);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        self.compute_text_metrics_if_needed(app);
        if self.has_visual_overflow(app) && self.get(app).clip_behavior != Clip::None {
            let size = self.size(app);
            let clip_behavior = self.get(app).clip_behavior;
            let old = self.get(app).clip_rect_layer.layer();
            let layer = context.push_clip_rect(
                app,
                self.as_object().needs_compositing(app),
                offset,
                Offset::ZERO & size,
                |app, context, offset| {
                    self.paint_contents(app, context, offset);
                },
                clip_behavior,
                old,
            );
            LayerHandle::set_layer(app, |app| &mut self.get_mut(app).clip_rect_layer, layer);
        } else {
            LayerHandle::set_layer(app, |app| &mut self.get_mut(app).clip_rect_layer, None);
            self.paint_contents(app, context, offset);
        }
        if let Some(selection) = self.selection(app)
            && selection.is_valid()
        {
            let endpoints = self.get_endpoints_for_selection(app, selection);
            self.paint_handle_layers(app, context, &endpoints, offset);
        }
    }
}

impl RenderBox for RenderEditable {
    crate::render_box_accessors!();

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, _height: f64) -> f64 {
        let (min_width, max_width) = self.adjust_constraints(app, 0.0, f64::INFINITY);
        let (intrinsics, fonts) = self.text_intrinsics_with_fonts(app);
        intrinsics.layout(fonts, min_width, max_width);
        intrinsics.min_intrinsic_width()
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, _height: f64) -> f64 {
        let (min_width, max_width) = self.adjust_constraints(app, 0.0, f64::INFINITY);
        let (intrinsics, fonts) = self.text_intrinsics_with_fonts(app);
        intrinsics.layout(fonts, min_width, max_width);
        intrinsics.max_intrinsic_width() + self.caret_margin(app)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.compute_max_intrinsic_height(app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.preferred_height(app, width)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let (min_width, max_width) =
            self.adjust_constraints(app, constraints.min_width, constraints.max_width);
        let force_line = self.get(app).force_line;
        let caret_margin = self.caret_margin(app);
        let (intrinsics, fonts) = self.text_intrinsics_with_fonts(app);
        intrinsics.layout(fonts, min_width, max_width);
        let painter_width = intrinsics.width();
        let width = if force_line {
            constraints.max_width
        } else {
            constraints.constrain_width(painter_width + caret_margin)
        };
        let height = self.preferred_height(app, constraints.max_width);
        Size::new(width, constraints.constrain_height(height))
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        self.compute_text_metrics_if_needed(app);
        Some(
            self.get(app)
                .text_painter
                .compute_distance_to_actual_baseline(baseline),
        )
    }

    fn hit_test_self(self: RenderHandle<Self>, _app: &App, _position: Offset) -> bool {
        true
    }
}

impl RenderEditable {
    /// Flutter's `_paintHandleLayers`: an empty leader at each selection endpoint, clamped into
    /// the box, for the selection handles to follow.
    fn paint_handle_layers(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        endpoints: &[TextSelectionPoint],
        offset: Offset,
    ) {
        let size = self.size(app);
        let clamp = |point: Offset| {
            Offset::new(
                point.dx().clamp(0.0, size.width()),
                point.dy().clamp(0.0, size.height()),
            )
        };
        let Some(start) = endpoints.first() else {
            return;
        };
        let start_point = clamp(start.point);
        let (start_link, end_link) = {
            let this = self.get(app);
            (this.start_handle_layer_link, this.end_handle_layer_link)
        };
        let start_layer = LeaderLayer::new(app, start_link);
        start_layer.set_offset(app, start_point + offset);
        LayerHandle::set_layer(
            app,
            |app| &mut self.get_mut(app).leader_layer_handler,
            Some(start_layer),
        );
        context.push_layer(
            app,
            start_layer.as_container_layer(),
            |_, _, _| {},
            Offset::ZERO,
            None,
        );
        if let Some(end) = endpoints.get(1) {
            let end_point = clamp(end.point);
            let end_layer = LeaderLayer::new(app, end_link);
            end_layer.set_offset(app, end_point + offset);
            context.push_layer(
                app,
                end_layer.as_container_layer(),
                |_, _, _| {},
                Offset::ZERO,
                None,
            );
        } else if self
            .selection(app)
            .is_some_and(|selection| selection.is_collapsed())
        {
            let end_layer = LeaderLayer::new(app, end_link);
            end_layer.set_offset(app, start_point + offset);
            context.push_layer(
                app,
                end_layer.as_container_layer(),
                |_, _, _| {},
                Offset::ZERO,
                None,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::TextAffinity;
    use reveal_foundation::{AppCell, ValueListenable};
    use reveal_painting::{TextSpan, TextStyle};
    use reveal_services::{TextEditingValue, TextSelectionDelegate};

    use super::*;
    use crate::layer::{ContainerLayer, ErasedLayer, OffsetLayer};
    use crate::pipeline_owner::PipelineOwner;
    use crate::proxy_box::RenderRepaintBoundary;
    use crate::viewport_offset::{FixedViewportOffset, ViewportOffset};

    #[derive(Default)]
    struct TestDelegate {
        value: TextEditingValue,
    }

    impl TextSelectionDelegate for TestDelegate {
        fn text_editing_value(self: Handle<Self>, app: &App) -> TextEditingValue {
            app.get(self).value.clone()
        }

        fn user_update_text_editing_value(
            self: Handle<Self>,
            app: &mut App,
            value: TextEditingValue,
            _cause: SelectionChangedCause,
        ) {
            app.get_mut(self).value = value;
        }
    }

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

    fn laid_out(app: &mut App, text: &str) -> RenderHandle<RenderEditable> {
        install_fonts(app);
        let delegate = app.create(TestDelegate {
            value: TextEditingValue::new()
                .text(text)
                .selection(TextSelection::collapsed(0, TextAffinity::Downstream)),
        });
        let start = LayerLink::new(app);
        let end = LayerLink::new(app);
        let offset = FixedViewportOffset::zero(app).as_viewport_offset();
        let editable = RenderEditable::new(
            app,
            TextDirection::Ltr,
            start,
            end,
            offset,
            delegate.as_text_selection_delegate(),
        );
        editable.set_text(app, Some(span(text)));
        editable.set_cursor_color(app, Some(Color::from_argb(255, 0, 0, 0)));
        let root = RenderRepaintBoundary::new(app, Some(editable.as_box()));
        let owner = PipelineOwner::new(app, None);
        owner.set_root_node(app, Some(root.as_object()));
        root.schedule_initial_layout(app);
        root.layout(
            app,
            BoxConstraints::tight_for(Some(200.0), Some(40.0)),
            false,
        );
        let paint_root = OffsetLayer::new(app, Offset::ZERO);
        paint_root.as_layer().attach(app, root.as_object().id());
        root.as_object()
            .schedule_initial_paint(app, paint_root.as_container_layer());
        editable
    }

    #[test]
    fn move_by_word_boundary_skips_spaces_and_punctuation() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let text = concat!(
            "ABC   ABC\n",                       // [0, 10)
            "A\u{41}\u{301}    \u{41}\u{301}\n", // [10, 20)
            "         \n",                       // [20, 30)
            "ABC!!!ABC\n",                       // [30, 40)
            "  !ABC !!\n",                       // [40, 50)
            "A  \u{115CB}\u{115CB} A\n",         // [50, 60)
        );
        let editable = laid_out(&mut app, text);
        let boundary = editable.word_boundaries(&app).move_by_word_boundary();

        // 4 points to the 2nd whitespace in the first line.
        // Don't break between horizontal spaces and letters/numbers.
        assert_eq!(boundary.get_leading_text_boundary_at(&mut app, 4), Some(0));
        assert_eq!(boundary.get_trailing_text_boundary_at(&mut app, 4), Some(9));

        // Works when words are starting/ending with a combining diacritical mark.
        assert_eq!(
            boundary.get_leading_text_boundary_at(&mut app, 14),
            Some(10)
        );
        assert_eq!(
            boundary.get_trailing_text_boundary_at(&mut app, 14),
            Some(19)
        );

        // Do break before and after newlines.
        assert_eq!(
            boundary.get_leading_text_boundary_at(&mut app, 24),
            Some(20)
        );
        assert_eq!(
            boundary.get_trailing_text_boundary_at(&mut app, 24),
            Some(29)
        );

        // Do not break on punctuations.
        assert_eq!(
            boundary.get_leading_text_boundary_at(&mut app, 34),
            Some(30)
        );
        assert_eq!(
            boundary.get_trailing_text_boundary_at(&mut app, 34),
            Some(39)
        );

        // Ok to break if next to punctuations or separating spaces.
        assert_eq!(
            boundary.get_leading_text_boundary_at(&mut app, 44),
            Some(43)
        );
        assert_eq!(
            boundary.get_trailing_text_boundary_at(&mut app, 44),
            Some(46)
        );

        // 54 points to a low surrogate of a punctuation.
        assert_eq!(
            boundary.get_leading_text_boundary_at(&mut app, 54),
            Some(50)
        );
        assert_eq!(
            boundary.get_trailing_text_boundary_at(&mut app, 54),
            Some(59)
        );
    }

    #[test]
    fn display_names_the_direction() {
        let point = Offset::new(1.0, 2.0);
        assert_eq!(
            TextSelectionPoint::new(point, Some(TextDirection::Ltr)).to_string(),
            format!("{point:?}-ltr")
        );
        assert_eq!(
            TextSelectionPoint::new(point, None).to_string(),
            format!("{point:?}")
        );
    }

    #[test]
    fn layout_gives_the_text_a_size() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let editable = laid_out(&mut app, "hi");
        let size = editable.size(&app);
        assert!(size.width() > 0.0);
        assert!(size.height() > 0.0);
    }

    #[test]
    fn select_position_updates_the_delegate() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let editable = laid_out(&mut app, "hello");
        let origin = editable.as_box().local_to_global(&app, Offset::ZERO, None);
        editable.select_position_at(
            &mut app,
            origin + Offset::new(2.0, 2.0),
            None,
            SelectionChangedCause::Tap,
        );
        let value = editable
            .text_selection_delegate(&app)
            .text_editing_value(&app);
        assert!(value.selection.is_valid());
        assert!(value.selection.is_collapsed());
    }

    #[test]
    fn handle_secondary_tap_down_records_the_position() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let editable = laid_out(&mut app, "hello");
        let at = Offset::new(12.0, 8.0);
        editable.handle_secondary_tap_down(&mut app, &TapDownDetails::new(at, None, None));
        assert_eq!(editable.last_secondary_tap_down_position(&app), Some(at));
    }

    #[test]
    fn selection_extents_default_in_the_viewport() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let editable = laid_out(&mut app, "hello");
        assert!(
            *editable.selection_start_in_viewport(&app).value(&app),
            "Flutter defaults selectionStartInViewport to true"
        );
        assert!(
            *editable.selection_end_in_viewport(&app).value(&app),
            "Flutter defaults selectionEndInViewport to true"
        );
    }
}
