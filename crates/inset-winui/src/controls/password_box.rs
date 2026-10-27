//! WinUI PasswordBox template and behavior, using Inset’s native editing engine.

use super::text_control::TextControl;
use crate::Brush;
use inset_embedder::Color;
use inset_foundation::{App, Handle};
use inset_services::TextInputFormatterRef;
use inset_widgets::*;
use std::{fmt, rc::Rc};

use super::text_box::{TextBox, TextChangedCallback};

/// WinUI PasswordRevealMode; Peek reveals only while the eye button or Alt+F8 is held.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PasswordRevealMode {
    /// Permit press-to-reveal after a new password was entered during this focus session.
    #[default]
    Peek,
    /// Always conceal the password and hide the reveal button.
    Hidden,
    /// Always display the password and hide the reveal button.
    Visible,
}

/// The WinUI PasswordBox control with its default Fluent template.
#[derive(Clone)]
pub struct PasswordBox {
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

    /// Controls whether the password can be temporarily revealed, stays hidden, or stays visible.
    pub password_reveal_mode: PasswordRevealMode,

    /// The character displayed for each concealed password character.
    pub password_char: char,

    /// Receives user password edits; listen to the controller for programmatic assignments.
    pub password_changed: Option<TextChangedCallback>,

    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl PasswordBox {
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
            password_reveal_mode: PasswordRevealMode::Peek,
            password_char: '●',
            password_changed: None,
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

    /// Sets PasswordRevealMode.
    pub fn password_reveal_mode(mut self, value: PasswordRevealMode) -> Self {
        self.password_reveal_mode = value;
        self
    }

    /// Sets PasswordChar.
    pub fn password_char(mut self, value: char) -> Self {
        assert!(
            value != '\0' && value.len_utf16() == 1,
            "PasswordChar must be one non-null UTF-16 character"
        );
        self.password_char = value;
        self
    }

    /// Sets PasswordChanged.
    pub fn password_changed(mut self, value: impl Fn(&mut App, String) + 'static) -> Self {
        self.password_changed = Some(Rc::new(value));
        self
    }

    /// Sets widget identity.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for PasswordBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PasswordBox")
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for PasswordBox {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let mut editor = TextBox::new(self.controller);
        editor.focus_node = self.focus_node;
        editor.header = self.header.clone();
        editor.description = self.description.clone();
        editor.placeholder_text = self.placeholder_text.clone();
        editor.is_enabled = self.is_enabled;
        editor.font_size = self.font_size;
        editor.padding = self.padding;
        editor.corner_radius = self.corner_radius;
        editor.foreground = self.foreground;
        editor.background = self.background;
        editor.border_brush = self.border_brush;
        editor.selection_highlight_color = self.selection_highlight_color;
        editor.max_length = self.max_length;
        editor.input_formatters = self.input_formatters.clone();
        editor.autofocus = self.autofocus;
        editor.context_menu_builder = self.context_menu_builder.clone();
        editor.text_changed = self.password_changed.clone();

        TextControl::new(
            editor,
            Some((self.password_reveal_mode, self.password_char)),
        )
        .into_widget()
    }
}
