//! WinUI TextBox template and behavior, using Waft’s native editing engine.

use super::text_control::{TextControl, TextControlState};
use crate::Brush;
use std::{fmt, rc::Rc};
use waft_embedder::{Color, Offset, Rect, TextAlign, TextSelection};
use waft_foundation::{App, Handle};
use waft_rendering::RenderBox;
use waft_services::{SelectionChangedCause, TextInputFormatterRef};
use waft_widgets::*;

/// XAML `TextWrapping`: whether long lines break at the box's width.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextWrapping {
    /// Lines stay whole and scroll sideways, a single line and, with `AcceptsReturn`, each line
    /// of a multiline box.
    #[default]
    NoWrap,
    /// Long lines wrap within the editor width.
    Wrap,
}

/// XAML `ScrollBarVisibility` for `ScrollViewer.HorizontalScrollBarVisibility`, as far as the kit's scroll bar can draw it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScrollBarVisibility {
    /// The content does not scroll on this axis: it is laid out at the view's width and clipped.
    Disabled,
    /// A scroll bar shows when the content is wider than the view.
    Auto,
    /// The content scrolls, with no scroll bar; `TextBox`'s style default.
    #[default]
    Hidden,
}

/// Notification of a user edit after the native controller updates.
pub type TextChangedCallback = Rc<dyn Fn(&mut App, String)>;

/// Notification of a native selection change and its input cause.
pub type TextSelectionChangedCallback =
    Rc<dyn Fn(&mut App, TextSelection, Option<SelectionChangedCause>)>;

/// The WinUI TextBox control with its default Fluent template.
#[derive(Clone)]
pub struct TextBox {
    /// Owns the text, selection and composing range; the caller retains and disposes it.
    pub controller: AnyTextEditingController,

    /// An optional caller-owned focus node; otherwise the control creates one.
    pub focus_node: Option<Handle<FocusNode>>,

    /// Content above the editor, with the source header margin.
    pub header: Option<WidgetRef>,

    /// Supporting content below the editor.
    pub description: Option<WidgetRef>,

    /// Text displayed while the controller is empty.
    pub placeholder_text: String,

    /// Whether the editor accepts pointer and keyboard input.
    pub is_enabled: bool,

    /// The editor font size and basis of the helper-button width threshold; the theme's `ControlContentThemeFontSize` when `None`.
    pub font_size: Option<f64>,

    /// Editor padding in left, top, right, bottom order; the theme's `TextControlThemePadding` when `None`.
    pub padding: Option<[f64; 4]>,

    /// The template border and helper-button corner radius.
    pub corner_radius: Option<f64>,

    /// Overrides the normal editor foreground; visual states use their source theme resources.
    pub foreground: Option<Color>,

    /// Overrides the normal editor background.
    pub background: Option<Brush>,

    /// Overrides the normal editor border brush.
    pub border_brush: Option<Brush>,

    /// Overrides the selection fill while the editor has focus.
    pub selection_highlight_color: Option<Color>,

    /// Limits user edits through the native length formatter; programmatic assignments remain unrestricted.
    pub max_length: Option<i32>,

    /// Native formatters applied to user edits, following Flutter composition and undo behavior.
    pub input_formatters: Vec<TextInputFormatterRef>,

    /// Requests initial keyboard focus through the native editor.
    pub autofocus: bool,

    /// Overrides the desktop text-command menu.
    pub context_menu_builder: Option<EditableTextContextMenuBuilder>,

    /// Allows selection and copying while preventing edits.
    pub is_read_only: bool,

    /// Accepts line breaks and enables native wrapped multiline editing.
    pub accepts_return: bool,

    /// Wraps text independently of accepting Enter; with `NoWrap`, a multiline box keeps each line whole and scrolls sideways.
    pub text_wrapping: TextWrapping,

    /// `ScrollViewer.HorizontalScrollBarVisibility`: how a multiline box that does not wrap scrolls its lines sideways.
    pub horizontal_scroll_bar_visibility: ScrollBarVisibility,

    /// Limits the visible height in lines for a multiline editor, which grows with its text from
    /// one line up to it; past it, the content scrolls vertically.
    pub max_lines: Option<i32>,

    /// Alignment of editor and placeholder text.
    pub text_alignment: TextAlign,

    /// Overrides the placeholder foreground in every visual state.
    pub placeholder_foreground: Option<Color>,

    /// Receives user text edits and clear-button actions; listen to the controller for all assignments.
    pub text_changed: Option<TextChangedCallback>,

    /// Receives native user selection changes after the controller updates.
    pub selection_changed: Option<TextSelectionChangedCallback>,

    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl TextBox {
    /// Creates an editor using a caller-owned native editing controller.
    pub fn new(controller: AnyTextEditingController) -> Self {
        Self {
            controller,
            focus_node: None,
            header: None,
            description: None,
            placeholder_text: String::new(),
            is_enabled: true,
            font_size: None,
            padding: None,
            corner_radius: None,
            foreground: None,
            background: None,
            border_brush: None,
            selection_highlight_color: None,
            max_length: None,
            input_formatters: Vec::new(),
            autofocus: false,
            context_menu_builder: None,
            is_read_only: false,
            accepts_return: false,
            text_wrapping: TextWrapping::NoWrap,
            horizontal_scroll_bar_visibility: ScrollBarVisibility::Hidden,
            max_lines: None,
            text_alignment: TextAlign::Start,
            placeholder_foreground: None,
            text_changed: None,
            selection_changed: None,
            key: None,
        }
    }

    /// Sets FocusNode.
    pub fn focus_node(mut self, value: Handle<FocusNode>) -> Self {
        self.focus_node = Some(value);
        self
    }

    /// Sets Header.
    pub fn header<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.header = Some(value.into_widget());
        self
    }

    /// Sets Description.
    pub fn description<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.description = Some(value.into_widget());
        self
    }

    /// Sets PlaceholderText.
    pub fn placeholder_text(mut self, value: impl Into<String>) -> Self {
        self.placeholder_text = value.into();
        self
    }

    /// Sets IsEnabled.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }

    /// Sets FontSize.
    pub fn font_size(mut self, value: f64) -> Self {
        self.font_size = Some(value);
        self
    }

    /// Sets Padding.
    pub fn padding(mut self, value: [f64; 4]) -> Self {
        self.padding = Some(value);
        self
    }

    /// Sets CornerRadius.
    pub fn corner_radius(mut self, value: f64) -> Self {
        self.corner_radius = Some(value);
        self
    }

    /// Sets Foreground.
    pub fn foreground(mut self, value: Color) -> Self {
        self.foreground = Some(value);
        self
    }

    /// Sets Background.
    pub fn background(mut self, value: Brush) -> Self {
        self.background = Some(value);
        self
    }

    /// Sets BorderBrush.
    pub fn border_brush(mut self, value: Brush) -> Self {
        self.border_brush = Some(value);
        self
    }

    /// Sets SelectionHighlightColor.
    pub fn selection_highlight_color(mut self, value: Color) -> Self {
        self.selection_highlight_color = Some(value);
        self
    }

    /// Sets MaxLength.
    pub fn max_length(mut self, value: i32) -> Self {
        assert!(value >= 0, "MaxLength must be non-negative");
        self.max_length = Some(value);
        self
    }

    /// Sets InputFormatters.
    pub fn input_formatters(mut self, value: Vec<TextInputFormatterRef>) -> Self {
        self.input_formatters = value;
        self
    }

    /// Sets Autofocus.
    pub fn autofocus(mut self, value: bool) -> Self {
        self.autofocus = value;
        self
    }

    /// Sets ContextMenuBuilder.
    pub fn context_menu_builder(
        mut self,
        value: impl Fn(&mut App, BuildContext, Handle<EditableTextState>) -> WidgetRef + 'static,
    ) -> Self {
        self.context_menu_builder = Some(Rc::new(value));
        self
    }

    /// Sets IsReadOnly.
    pub fn is_read_only(mut self, value: bool) -> Self {
        self.is_read_only = value;
        self
    }

    /// Sets AcceptsReturn.
    pub fn accepts_return(mut self, value: bool) -> Self {
        self.accepts_return = value;
        self
    }

    /// Sets TextWrapping.
    pub fn text_wrapping(mut self, value: TextWrapping) -> Self {
        self.text_wrapping = value;
        self
    }

    /// Sets `ScrollViewer.HorizontalScrollBarVisibility`.
    pub fn horizontal_scroll_bar_visibility(mut self, value: ScrollBarVisibility) -> Self {
        self.horizontal_scroll_bar_visibility = value;
        self
    }

    /// Sets MaxLines.
    pub fn max_lines(mut self, value: i32) -> Self {
        self.max_lines = Some(value);
        self
    }

    /// Sets TextAlignment.
    pub fn text_alignment(mut self, value: TextAlign) -> Self {
        self.text_alignment = value;
        self
    }

    /// Sets PlaceholderForeground.
    pub fn placeholder_foreground(mut self, value: Color) -> Self {
        self.placeholder_foreground = Some(value);
        self
    }

    /// Sets TextChanged.
    pub fn text_changed(mut self, value: impl Fn(&mut App, String) + 'static) -> Self {
        self.text_changed = Some(Rc::new(value));
        self
    }

    /// Sets SelectionChanged.
    pub fn selection_changed(
        mut self,
        value: impl Fn(&mut App, TextSelection, Option<SelectionChangedCause>) + 'static,
    ) -> Self {
        self.selection_changed = Some(Rc::new(value));
        self
    }

    /// Sets widget identity.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for TextBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextBox")
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for TextBox {
    type State = TextBoxState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> TextBoxState {
        TextBoxState {
            state: StateData::new(),
        }
    }
}

/// A `TextBox` in the tree, which a `GlobalKey` on the box reaches.
pub struct TextBoxState {
    state: StateData<TextBox>,
}

impl TextBoxState {
    /// `TextBox::GetRectFromCharacterIndex` (`TextBox.cpp`): the leading or, with
    /// `trailing_edge`, the trailing edge of the character at `char_index`, a UTF-16 index into
    /// the text, as a rectangle of no width and the line's height, in the box's own coordinates
    /// and where the character shows now, scrolled on either axis. `None` for an index outside
    /// the text, the end of the text and every index of an empty box among them, where WinUI
    /// throws `E_INVALIDARG`, and before the box is laid out.
    pub fn get_rect_from_character_index(
        self: Handle<Self>,
        app: &mut App,
        char_index: i32,
        trailing_edge: bool,
    ) -> Option<Rect> {
        let length = self
            .widget(app)
            .controller
            .text_value(app)
            .encode_utf16()
            .count();
        if char_index < 0 || char_index as usize >= length {
            return None;
        }
        let editable = self.text_control(app)?.render_editable(app)?;
        let character = TextSelection::new(char_index, char_index + 1);
        let boxes = editable.get_boxes_for_selection(app, character);
        let glyph = boxes.first()?;
        // RichEdit's `TA_LEFT` and `TA_RIGHT`: the character's left and right edges.
        let x = if trailing_edge {
            glyph.right
        } else {
            glyph.left
        };
        let top_left = editable
            .as_box()
            .local_to_global(app, Offset::new(x, glyph.top), None);
        let own = self.context(app).find_render_object(app)?.as_box()?;
        let local = own.global_to_local(app, top_left, None);
        Some(Rect::from_ltwh(
            local.dx(),
            local.dy(),
            0.0,
            glyph.bottom - glyph.top,
        ))
    }

    /// The shared editor this box builds.
    fn text_control(self: Handle<Self>, app: &App) -> Option<Handle<TextControlState>> {
        let mut found = None;
        self.context(app).visit_child_elements(app, &mut |child| {
            found = child.state_handle::<TextControlState>(app);
        });
        found
    }
}

impl State for TextBoxState {
    type Widget = TextBox;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        TextControl::new(self.widget(app).clone(), None).into_widget()
    }
}
