//! Flutter counterpart: material_ui's `text_field.dart`, with its part
//! `generated/text_field_defaults_m3.g.dart` at the bottom of this file.
//!
//! [`TextFieldState`] is Dart's `_TextFieldState`: it owns the local controller and focus node
//! when the widget gives none, wraps an [`EditableText`] in an [`InputDecorator`] that rebuilds
//! on focus and text changes, and picks the cursor, selection colour and selection controls per
//! platform in `build`.
#![allow(deprecated)]

use std::any::TypeId;
use std::fmt::{self, Debug};
use std::rc::Rc;

use unicode_segmentation::UnicodeSegmentation;
use waft_cupertino::{
    CupertinoColors, CupertinoSpellCheckSuggestionsToolbar, CupertinoTextField, CupertinoTheme,
    cupertino_desktop_text_selection_handle_controls, cupertino_text_selection_handle_controls,
};
use waft_embedder::{
    BoxHeightStyle, BoxWidthStyle, Brightness, Clip, Color, Locale, Offset, Radius,
    SmartDashesType, SmartQuotesType, TargetPlatform, TextCapitalization, TextDecoration,
    TextDecorationStyle, TextEditingValue, TextInputAction, TextInputType, TextSelection,
};
use waft_foundation::{
    App, Handle, Listenable, ListenableObject, Listener, MergingListenable, OptionRetained, Ref,
    VoidCallback,
};
use waft_gestures::{DragStartBehavior, GestureTapCallback, PointerDownEvent, PointerUpEvent};
use waft_painting::{
    AnyTextStyle, EdgeInsets, TextAlign, TextAlignVertical, TextDirection, TextStyle,
};
use waft_rendering::HitTestBehavior;
use waft_services::{
    LengthLimitingTextInputFormatter, MaxLengthEnforcement, MouseCursor, MouseCursorRef,
    RestorationBucket, SelectionChangedCause, TextInputFormatterRef,
};
use waft_widgets::{
    AnimatedBuilder, AppPrivateCommandCallback, BuildContext, ContentInsertionConfiguration,
    DefaultSelectionStyle, EditableText, EditableTextContextMenuBuilder, EditableTextState,
    FocusNode, FocusNodeLeaf, FocusNodeRef, GlobalKey, IgnorePointer, IntoWidget, KeyRef,
    MediaQuery, MouseRegion, NavigationMode, RepaintBoundary, RestorableListenable,
    RestorableProperty, RestorableTextEditingController, RestorationMixin, RestorationMixinData,
    ScrollControllerRef, ScrollPhysicsRef, Semantics, SpellCheckConfiguration, State, StateData,
    StatefulWidget, SystemContextMenu, TapRegionCallback, TapRegionUpCallback,
    TextEditingControllerRef, TextFieldTapRegion, TextMagnifierConfiguration,
    TextSelectionControls, TextSelectionGestureDetectorBuilder,
    TextSelectionGestureDetectorBuilderBase, TextSelectionGestureDetectorBuilderData,
    TextSelectionGestureDetectorBuilderDelegate, ToolbarOptions, UndoHistoryController,
    UnmanagedRestorationScope, WidgetRef, WidgetState, WidgetStateMouseCursor, WidgetStateProperty,
    WidgetStateTextStyle, WidgetStates, debug_check_has_directionality,
};

use crate::adaptive_text_selection_toolbar::AdaptiveTextSelectionToolbar;
use crate::colors::Colors;
use crate::debug::{debug_check_has_material, debug_check_has_material_localizations};
use crate::desktop_text_selection::desktop_text_selection_handle_controls;
use crate::input_decorator::{InputDecoration, InputDecorationTheme, InputDecorator};
use crate::magnifier::TextMagnifier;
use crate::material_localizations::MaterialLocalizations;
use crate::material_state::MaterialStatesController;
use crate::selectable_text::IOS_HORIZONTAL_OFFSET;
use crate::spell_check_suggestions_toolbar::SpellCheckSuggestionsToolbar;
use crate::text_selection::material_text_selection_handle_controls;
use crate::theme::Theme;

/// Signature for the [`TextField::build_counter`] callback.
///
/// Its arguments after the context are Dart's named `currentLength` (the length of the string
/// currently in the input), `maxLength` (the maximum string length that can be entered into
/// the TextField) and `isFocused` (whether or not the TextField is currently focused).
pub type InputCounterWidgetBuilder =
    Rc<dyn Fn(&mut App, BuildContext, i32, Option<i32>, bool) -> Option<WidgetRef>>;

/// Dart's `_TextFieldSelectionGestureDetectorBuilder`.
struct TextFieldSelectionGestureDetectorBuilder {
    builder: TextSelectionGestureDetectorBuilderData,
    state: Handle<TextFieldState>,
}

impl TextSelectionGestureDetectorBuilder for TextFieldSelectionGestureDetectorBuilder {
    waft_widgets::text_selection_gesture_detector_builder_accessors!(builder);

    fn on_user_tap_always_called(self: Handle<Self>, app: &App) -> bool {
        app.get(self).state.widget(app).on_tap_always_called
    }

    fn on_user_tap(self: Handle<Self>, app: &mut App) {
        let state = app.get(self).state;
        if let Some(on_tap) = state.widget(app).on_tap.clone() {
            on_tap.call(app);
        }
    }
}

/// A Material Design text field.
///
/// A text field lets the user enter text, either with hardware keyboard or with
/// an onscreen keyboard.
///
/// The text field calls the [`on_changed`](Self::on_changed) callback whenever the user changes
/// the text in the field. If the user indicates that they are done typing in the field (e.g.,
/// by pressing a button on the soft keyboard), the text field calls the
/// [`on_submitted`](Self::on_submitted) callback.
///
/// By default, a text field has a [`decoration`](Self::decoration) that draws a divider below
/// the text field. If the decoration is `None`, the decoration is removed entirely, including
/// the extra padding introduced by the decoration to save space for the labels.
///
/// If [`decoration`](Self::decoration) is non-null (which is the default), the text field
/// requires one of its ancestors to be a `Material` widget.
///
/// To integrate the [`TextField`] into a `Form` with other `FormField` widgets, consider using
/// `TextFormField`.
#[allow(clippy::type_complexity)]
#[derive(Clone)]
pub struct TextField {
    /// See [`Widget::key`](waft_widgets::Widget::key).
    pub key: Option<KeyRef>,
    /// The configuration of the magnifier; `TextMagnifier::adaptive_magnifier_configuration`
    /// when `None`.
    pub magnifier_configuration: Option<TextMagnifierConfiguration>,
    /// {@macro flutter.widgets.editableText.groupId}
    pub group_id: TypeId,
    /// Controls the text being edited.
    ///
    /// If null, this widget will create its own `TextEditingController`.
    pub controller: Option<TextEditingControllerRef>,
    /// Defines the keyboard focus for this widget.
    ///
    /// If null, this widget will create its own [`FocusNode`].
    pub focus_node: Option<FocusNodeRef>,
    /// The decoration to show around the text field.
    ///
    /// By default, draws a horizontal line under the text field but can be configured to show
    /// an icon, label, hint text, and error text.
    ///
    /// Specify `None` to remove the decoration entirely (including the extra padding introduced
    /// by the decoration to save space for the labels).
    pub decoration: Option<InputDecoration>,
    /// {@macro flutter.widgets.editableText.keyboardType}
    ///
    /// `None`: Dart's default, [`TextInputType::TEXT`] for one line and
    /// [`TextInputType::MULTILINE`] otherwise ([`effective_keyboard_type`](Self::effective_keyboard_type)).
    pub keyboard_type: Option<TextInputType>,
    /// The type of action button to use for the keyboard.
    pub text_input_action: Option<TextInputAction>,
    /// {@macro flutter.widgets.editableText.textCapitalization}
    pub text_capitalization: TextCapitalization,
    /// The style to use for the text being edited.
    ///
    /// This text style is also used as the base style for the [`decoration`](Self::decoration).
    ///
    /// If null, [`TextTheme::title_medium`](crate::text_theme::TextTheme) is used (Material 2),
    /// or `body_large` (Material 3). A `WidgetStateTextStyle` is resolved against the field's
    /// states.
    pub style: Option<AnyTextStyle>,
    /// {@macro flutter.widgets.editableText.textAlign}
    pub text_align: TextAlign,
    /// {@macro cupertino_ui.InputDecorator.textAlignVertical}
    pub text_align_vertical: Option<TextAlignVertical>,
    /// {@macro flutter.widgets.editableText.textDirection}
    pub text_direction: Option<TextDirection>,
    /// {@macro flutter.widgets.editableText.autofocus}
    pub autofocus: bool,
    /// Represents the interactive "state" of this widget in terms of a set of [`WidgetState`]s,
    /// including [`WidgetState::Disabled`], [`WidgetState::Hovered`], [`WidgetState::Error`],
    /// and [`WidgetState::Focused`].
    pub states_controller: Option<Ref<MaterialStatesController>>,
    /// {@macro flutter.widgets.editableText.obscuringCharacter}
    pub obscuring_character: String,
    /// {@macro flutter.widgets.editableText.obscureText}
    pub obscure_text: bool,
    /// {@macro flutter.widgets.editableText.autocorrect}
    pub autocorrect: Option<bool>,
    /// {@macro flutter.services.TextInputConfiguration.smartDashesType}
    ///
    /// `None`: Dart's default from [`obscure_text`](Self::obscure_text)
    /// ([`effective_smart_dashes_type`](Self::effective_smart_dashes_type)).
    pub smart_dashes_type: Option<SmartDashesType>,
    /// {@macro flutter.services.TextInputConfiguration.smartQuotesType}
    ///
    /// `None`: Dart's default from [`obscure_text`](Self::obscure_text)
    /// ([`effective_smart_quotes_type`](Self::effective_smart_quotes_type)).
    pub smart_quotes_type: Option<SmartQuotesType>,
    /// {@macro flutter.services.TextInputConfiguration.enableSuggestions}
    pub enable_suggestions: bool,
    /// {@macro flutter.widgets.editableText.maxLines}
    pub max_lines: Option<i32>,
    /// {@macro flutter.widgets.editableText.minLines}
    pub min_lines: Option<i32>,
    /// {@macro flutter.widgets.editableText.expands}
    pub expands: bool,
    /// {@macro flutter.widgets.editableText.readOnly}
    pub read_only: bool,
    /// Configuration of toolbar options.
    ///
    /// If not set, select all and paste will default to be enabled. Copy and cut will be
    /// disabled if [`obscure_text`](Self::obscure_text) is true. If
    /// [`read_only`](Self::read_only) is true, paste and cut will be disabled regardless.
    #[deprecated(note = "Use `context_menu_builder` instead.")]
    pub toolbar_options: Option<ToolbarOptions>,
    /// {@macro flutter.widgets.editableText.showCursor}
    pub show_cursor: Option<bool>,
    /// The maximum number of characters (Unicode grapheme clusters) to allow in the text field.
    ///
    /// If set, a character counter will be displayed below the field showing how many
    /// characters have been entered. If set to a number greater than 0, it will also display
    /// the maximum number allowed. If set to [`TextField::NO_MAX_LENGTH`] then only the current
    /// character count is displayed.
    ///
    /// After `max_length` characters have been input, additional input is ignored, unless
    /// [`max_length_enforcement`](Self::max_length_enforcement) is set to
    /// [`MaxLengthEnforcement::None`].
    pub max_length: Option<i32>,
    /// Determines how the [`max_length`](Self::max_length) limit should be enforced.
    pub max_length_enforcement: Option<MaxLengthEnforcement>,
    /// Called when the user initiates a change to the TextField's value: when they have
    /// inserted or deleted text.
    pub on_changed: Option<Rc<dyn Fn(&mut App, String)>>,
    /// {@macro flutter.widgets.editableText.onEditingComplete}
    pub on_editing_complete: Option<Rc<dyn Fn(&mut App)>>,
    /// {@macro flutter.widgets.editableText.onSubmitted}
    pub on_submitted: Option<Rc<dyn Fn(&mut App, String)>>,
    /// {@macro flutter.widgets.editableText.onAppPrivateCommand}
    pub on_app_private_command: Option<AppPrivateCommandCallback>,
    /// {@macro flutter.widgets.editableText.inputFormatters}
    pub input_formatters: Option<Vec<TextInputFormatterRef>>,
    /// If false the text field is "disabled": it ignores taps and its
    /// [`decoration`](Self::decoration) is rendered in grey.
    ///
    /// If non-null this property overrides the decoration's `enabled` property.
    pub enabled: Option<bool>,
    /// Determines whether this widget ignores pointer events.
    ///
    /// Defaults to null, and when null, does nothing.
    pub ignore_pointers: Option<bool>,
    /// {@macro flutter.widgets.editableText.cursorWidth}
    pub cursor_width: f64,
    /// {@macro flutter.widgets.editableText.cursorHeight}
    pub cursor_height: Option<f64>,
    /// {@macro flutter.widgets.editableText.cursorRadius}
    pub cursor_radius: Option<Radius>,
    /// {@macro flutter.widgets.editableText.cursorOpacityAnimates}
    pub cursor_opacity_animates: Option<bool>,
    /// The color of the cursor.
    ///
    /// The cursor indicates the current location of text insertion point in the field.
    pub cursor_color: Option<Color>,
    /// The color of the cursor when the [`InputDecorator`] is showing an error.
    ///
    /// If this is null it will default to `TextStyle.color` of
    /// `InputDecoration.error_style`. If that is null, it will use `ColorScheme.error` of
    /// `ThemeData.color_scheme`.
    pub cursor_error_color: Option<Color>,
    /// Controls how tall the selection highlight boxes are computed to be.
    pub selection_height_style: Option<BoxHeightStyle>,
    /// Controls how wide the selection highlight boxes are computed to be.
    pub selection_width_style: Option<BoxWidthStyle>,
    /// The appearance of the keyboard.
    ///
    /// If unset, defaults to `ThemeData.brightness`.
    pub keyboard_appearance: Option<Brightness>,
    /// {@macro flutter.widgets.editableText.scrollPadding}
    pub scroll_padding: EdgeInsets,
    /// {@macro flutter.widgets.editableText.enableInteractiveSelection}
    ///
    /// `None`: Dart's default, `!read_only || !obscure_text`
    /// ([`effective_enable_interactive_selection`](Self::effective_enable_interactive_selection)).
    pub enable_interactive_selection: Option<bool>,
    /// {@macro flutter.widgets.editableText.selectAllOnFocus}
    pub select_all_on_focus: Option<bool>,
    /// {@macro flutter.widgets.editableText.selectionControls}
    pub selection_controls: Option<Rc<dyn TextSelectionControls>>,
    /// {@macro flutter.widgets.scrollable.dragStartBehavior}
    pub drag_start_behavior: DragStartBehavior,
    /// Called for the first tap in a series of taps.
    ///
    /// The text field builds a `GestureDetector` to handle input events like tap, to trigger
    /// focus requests, to move the caret, adjust the selection, etc. Handling some of those
    /// events by wrapping the text field with a competing `GestureDetector` is problematic.
    pub on_tap: Option<GestureTapCallback>,
    /// Whether [`on_tap`](Self::on_tap) should be called for every tap.
    ///
    /// Defaults to false, so [`on_tap`](Self::on_tap) is only called for each distinct tap.
    /// When enabled, [`on_tap`](Self::on_tap) is called for every tap including consecutive
    /// taps.
    pub on_tap_always_called: bool,
    /// {@macro flutter.widgets.editableText.onTapOutside}
    pub on_tap_outside: Option<TapRegionCallback>,
    /// {@macro flutter.widgets.editableText.onTapUpOutside}
    pub on_tap_up_outside: Option<TapRegionUpCallback>,
    /// The cursor for a mouse pointer when it enters or is hovering over the widget.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a `WidgetStateMouseCursor`, it is resolved
    /// against the field's states. If this property is null,
    /// `WidgetStateMouseCursor::textable()` is used.
    pub mouse_cursor: Option<MouseCursorRef>,
    /// Callback that generates a custom `InputDecoration.counter` widget.
    ///
    /// The returned widget is placed below the text field, as in the
    /// `InputDecoration.counter` property. If it returns null, no counter is shown.
    pub build_counter: Option<InputCounterWidgetBuilder>,
    /// {@macro flutter.widgets.editableText.scrollPhysics}
    pub scroll_physics: Option<ScrollPhysicsRef>,
    /// {@macro flutter.widgets.editableText.scrollController}
    pub scroll_controller: Option<ScrollControllerRef>,
    /// {@macro flutter.widgets.editableText.autofillHints}
    pub autofill_hints: Vec<String>,
    /// {@macro cupertino_ui.Material.clipBehavior}
    pub clip_behavior: Clip,
    /// Restoration ID to save and restore the state of the text field.
    ///
    /// If non-null, the text field will persist and restore its current scroll offset and -
    /// if no [`controller`](Self::controller) has been provided - the content of the text
    /// field. If a [`controller`](Self::controller) has been provided, it is the
    /// responsibility of the owner of that controller to persist and restore it.
    pub restoration_id: Option<String>,
    /// {@macro flutter.widgets.editableText.scribbleEnabled}
    #[deprecated(note = "Use `stylus_handwriting_enabled` instead.")]
    pub scribble_enabled: bool,
    /// {@macro flutter.widgets.editableText.stylusHandwritingEnabled}
    pub stylus_handwriting_enabled: bool,
    /// {@macro flutter.services.TextInputConfiguration.enableIMEPersonalizedLearning}
    pub enable_ime_personalized_learning: bool,
    /// {@macro flutter.services.TextInputConfiguration.enableInlinePrediction}
    pub enable_inline_prediction: Option<bool>,
    /// {@macro flutter.widgets.editableText.contentInsertionConfiguration}
    pub content_insertion_configuration: Option<ContentInsertionConfiguration>,
    /// {@macro flutter.widgets.EditableText.contextMenuBuilder}
    ///
    /// If not provided, will build a default menu based on the platform.
    pub context_menu_builder: Option<EditableTextContextMenuBuilder>,
    /// Determine whether this text field can request the primary focus.
    ///
    /// Defaults to true. If false, the text field will not request focus when tapped, or when
    /// its context menu is displayed. If false it will not be possible to move the focus to
    /// the text field with tab key.
    pub can_request_focus: bool,
    /// {@macro flutter.widgets.undoHistory.controller}
    pub undo_controller: Option<Ref<UndoHistoryController>>,
    /// {@macro flutter.services.TextInputConfiguration.hintLocales}
    pub hint_locales: Option<Vec<Locale>>,
    /// {@macro flutter.widgets.EditableText.spellCheckConfiguration}
    ///
    /// If [`SpellCheckConfiguration::misspelled_text_style`] is not specified in this
    /// configuration, then [`TextField::material_misspelled_text_style`] is used by default.
    pub spell_check_configuration: Option<SpellCheckConfiguration>,
}

#[allow(deprecated)]
impl TextField {
    /// If [`max_length`](Self::max_length) is set to this value, only the "current input
    /// length" part of the character counter is shown.
    pub const NO_MAX_LENGTH: i32 = -1;

    /// Creates a Material Design text field.
    pub fn new() -> TextField {
        TextField {
            key: None,
            magnifier_configuration: None,
            group_id: TypeId::of::<EditableText>(),
            controller: None,
            focus_node: None,
            decoration: Some(InputDecoration::new()),
            keyboard_type: None,
            text_input_action: None,
            text_capitalization: TextCapitalization::None,
            style: None,
            text_align: TextAlign::Start,
            text_align_vertical: None,
            text_direction: None,
            autofocus: false,
            states_controller: None,
            obscuring_character: "\u{2022}".into(),
            obscure_text: false,
            autocorrect: None,
            smart_dashes_type: None,
            smart_quotes_type: None,
            enable_suggestions: true,
            max_lines: Some(1),
            min_lines: None,
            expands: false,
            read_only: false,
            toolbar_options: None,
            show_cursor: None,
            max_length: None,
            max_length_enforcement: None,
            on_changed: None,
            on_editing_complete: None,
            on_submitted: None,
            on_app_private_command: None,
            input_formatters: None,
            enabled: None,
            ignore_pointers: None,
            cursor_width: 2.0,
            cursor_height: None,
            cursor_radius: None,
            cursor_opacity_animates: None,
            cursor_color: None,
            cursor_error_color: None,
            selection_height_style: None,
            selection_width_style: None,
            keyboard_appearance: None,
            scroll_padding: EdgeInsets::all(20.0),
            enable_interactive_selection: None,
            select_all_on_focus: None,
            selection_controls: None,
            drag_start_behavior: DragStartBehavior::Start,
            on_tap: None,
            on_tap_always_called: false,
            on_tap_outside: None,
            on_tap_up_outside: None,
            mouse_cursor: None,
            build_counter: None,
            scroll_physics: None,
            scroll_controller: None,
            autofill_hints: Vec::new(),
            clip_behavior: Clip::HardEdge,
            restoration_id: None,
            scribble_enabled: true,
            stylus_handwriting_enabled: EditableText::DEFAULT_STYLUS_HANDWRITING_ENABLED,
            enable_ime_personalized_learning: true,
            enable_inline_prediction: None,
            content_insertion_configuration: None,
            context_menu_builder: Some(Rc::new(Self::default_context_menu_builder)),
            can_request_focus: true,
            undo_controller: None,
            hint_locales: None,
            spell_check_configuration: None,
        }
    }

    /// Dart `TextField(key:)`.
    pub fn key(mut self, key: KeyRef) -> TextField {
        self.key = Some(key);
        self
    }

    /// Dart `TextField(groupId:)`.
    pub fn group_id(mut self, group_id: TypeId) -> TextField {
        self.group_id = group_id;
        self
    }

    /// Dart `TextField(controller:)`.
    pub fn controller(mut self, controller: TextEditingControllerRef) -> TextField {
        self.controller = Some(controller);
        self
    }

    /// Dart `TextField(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> TextField {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `TextField(undoController:)`.
    pub fn undo_controller(mut self, undo_controller: Ref<UndoHistoryController>) -> TextField {
        self.undo_controller = Some(undo_controller);
        self
    }

    /// Dart `TextField(decoration:)`.
    pub fn decoration(mut self, decoration: Option<InputDecoration>) -> TextField {
        self.decoration = decoration;
        self
    }

    /// Dart `TextField(keyboardType:)`.
    pub fn keyboard_type(mut self, keyboard_type: TextInputType) -> TextField {
        self.keyboard_type = Some(keyboard_type);
        self
    }

    /// Dart `TextField(textInputAction:)`.
    pub fn text_input_action(mut self, text_input_action: TextInputAction) -> TextField {
        self.text_input_action = Some(text_input_action);
        self
    }

    /// Dart `TextField(textCapitalization:)`.
    pub fn text_capitalization(mut self, text_capitalization: TextCapitalization) -> TextField {
        self.text_capitalization = text_capitalization;
        self
    }

    /// Dart `TextField(style:)`.
    pub fn style(mut self, style: impl Into<AnyTextStyle>) -> TextField {
        self.style = Some(style.into());
        self
    }

    /// Dart `TextField(textAlign:)`.
    pub fn text_align(mut self, text_align: TextAlign) -> TextField {
        self.text_align = text_align;
        self
    }

    /// Dart `TextField(textAlignVertical:)`.
    pub fn text_align_vertical(mut self, text_align_vertical: TextAlignVertical) -> TextField {
        self.text_align_vertical = Some(text_align_vertical);
        self
    }

    /// Dart `TextField(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> TextField {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `TextField(readOnly:)`.
    pub fn read_only(mut self, read_only: bool) -> TextField {
        self.read_only = read_only;
        self
    }

    /// Dart `TextField(toolbarOptions:)`.
    #[deprecated(note = "Use `context_menu_builder` instead.")]
    pub fn toolbar_options(mut self, toolbar_options: ToolbarOptions) -> TextField {
        self.toolbar_options = Some(toolbar_options);
        self
    }

    /// Dart `TextField(showCursor:)`.
    pub fn show_cursor(mut self, show_cursor: bool) -> TextField {
        self.show_cursor = Some(show_cursor);
        self
    }

    /// Dart `TextField(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> TextField {
        self.autofocus = autofocus;
        self
    }

    /// Dart `TextField(statesController:)`.
    pub fn states_controller(
        mut self,
        states_controller: Ref<MaterialStatesController>,
    ) -> TextField {
        self.states_controller = Some(states_controller);
        self
    }

    /// Dart `TextField(obscuringCharacter:)`.
    pub fn obscuring_character(mut self, obscuring_character: impl Into<String>) -> TextField {
        let obscuring_character = obscuring_character.into();
        debug_assert!(obscuring_character.encode_utf16().count() == 1);
        self.obscuring_character = obscuring_character;
        self
    }

    /// Dart `TextField(obscureText:)`.
    pub fn obscure_text(mut self, obscure_text: bool) -> TextField {
        self.obscure_text = obscure_text;
        self
    }

    /// Dart `TextField(autocorrect:)`.
    pub fn autocorrect(mut self, autocorrect: bool) -> TextField {
        self.autocorrect = Some(autocorrect);
        self
    }

    /// Dart `TextField(smartDashesType:)`.
    pub fn smart_dashes_type(mut self, smart_dashes_type: SmartDashesType) -> TextField {
        self.smart_dashes_type = Some(smart_dashes_type);
        self
    }

    /// Dart `TextField(smartQuotesType:)`.
    pub fn smart_quotes_type(mut self, smart_quotes_type: SmartQuotesType) -> TextField {
        self.smart_quotes_type = Some(smart_quotes_type);
        self
    }

    /// Dart `TextField(enableSuggestions:)`.
    pub fn enable_suggestions(mut self, enable_suggestions: bool) -> TextField {
        self.enable_suggestions = enable_suggestions;
        self
    }

    /// Dart `TextField(maxLines:)`.
    pub fn max_lines(mut self, max_lines: Option<i32>) -> TextField {
        debug_assert!(max_lines.is_none_or(|lines| lines > 0));
        self.max_lines = max_lines;
        self
    }

    /// Dart `TextField(minLines:)`.
    pub fn min_lines(mut self, min_lines: i32) -> TextField {
        debug_assert!(min_lines > 0);
        self.min_lines = Some(min_lines);
        self
    }

    /// Dart `TextField(expands:)`.
    pub fn expands(mut self, expands: bool) -> TextField {
        self.expands = expands;
        self
    }

    /// Dart `TextField(maxLength:)`.
    pub fn max_length(mut self, max_length: i32) -> TextField {
        debug_assert!(max_length == TextField::NO_MAX_LENGTH || max_length > 0);
        self.max_length = Some(max_length);
        self
    }

    /// Dart `TextField(maxLengthEnforcement:)`.
    pub fn max_length_enforcement(
        mut self,
        max_length_enforcement: MaxLengthEnforcement,
    ) -> TextField {
        self.max_length_enforcement = Some(max_length_enforcement);
        self
    }

    /// Dart `TextField(onChanged:)`.
    pub fn on_changed(mut self, on_changed: impl Fn(&mut App, String) + 'static) -> TextField {
        self.on_changed = Some(Rc::new(on_changed));
        self
    }

    /// Dart `TextField(onEditingComplete:)`.
    pub fn on_editing_complete(
        mut self,
        on_editing_complete: impl Fn(&mut App) + 'static,
    ) -> TextField {
        self.on_editing_complete = Some(Rc::new(on_editing_complete));
        self
    }

    /// Dart `TextField(onSubmitted:)`.
    pub fn on_submitted(mut self, on_submitted: impl Fn(&mut App, String) + 'static) -> TextField {
        self.on_submitted = Some(Rc::new(on_submitted));
        self
    }

    /// Dart `TextField(onAppPrivateCommand:)`.
    pub fn on_app_private_command(
        mut self,
        on_app_private_command: impl Fn(&mut App, &str) + 'static,
    ) -> TextField {
        self.on_app_private_command = Some(Rc::new(on_app_private_command));
        self
    }

    /// Dart `TextField(inputFormatters:)`.
    pub fn input_formatters(mut self, input_formatters: Vec<TextInputFormatterRef>) -> TextField {
        self.input_formatters = Some(input_formatters);
        self
    }

    /// Dart `TextField(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> TextField {
        self.enabled = Some(enabled);
        self
    }

    /// Dart `TextField(ignorePointers:)`.
    pub fn ignore_pointers(mut self, ignore_pointers: bool) -> TextField {
        self.ignore_pointers = Some(ignore_pointers);
        self
    }

    /// Dart `TextField(cursorWidth:)`.
    pub fn cursor_width(mut self, cursor_width: f64) -> TextField {
        self.cursor_width = cursor_width;
        self
    }

    /// Dart `TextField(cursorHeight:)`.
    pub fn cursor_height(mut self, cursor_height: f64) -> TextField {
        self.cursor_height = Some(cursor_height);
        self
    }

    /// Dart `TextField(cursorRadius:)`.
    pub fn cursor_radius(mut self, cursor_radius: Radius) -> TextField {
        self.cursor_radius = Some(cursor_radius);
        self
    }

    /// Dart `TextField(cursorOpacityAnimates:)`.
    pub fn cursor_opacity_animates(mut self, cursor_opacity_animates: bool) -> TextField {
        self.cursor_opacity_animates = Some(cursor_opacity_animates);
        self
    }

    /// Dart `TextField(cursorColor:)`.
    pub fn cursor_color(mut self, cursor_color: Color) -> TextField {
        self.cursor_color = Some(cursor_color);
        self
    }

    /// Dart `TextField(cursorErrorColor:)`.
    pub fn cursor_error_color(mut self, cursor_error_color: Color) -> TextField {
        self.cursor_error_color = Some(cursor_error_color);
        self
    }

    /// Dart `TextField(selectionHeightStyle:)`.
    pub fn selection_height_style(mut self, selection_height_style: BoxHeightStyle) -> TextField {
        self.selection_height_style = Some(selection_height_style);
        self
    }

    /// Dart `TextField(selectionWidthStyle:)`.
    pub fn selection_width_style(mut self, selection_width_style: BoxWidthStyle) -> TextField {
        self.selection_width_style = Some(selection_width_style);
        self
    }

    /// Dart `TextField(keyboardAppearance:)`.
    pub fn keyboard_appearance(mut self, keyboard_appearance: Brightness) -> TextField {
        self.keyboard_appearance = Some(keyboard_appearance);
        self
    }

    /// Dart `TextField(scrollPadding:)`.
    pub fn scroll_padding(mut self, scroll_padding: EdgeInsets) -> TextField {
        self.scroll_padding = scroll_padding;
        self
    }

    /// Dart `TextField(dragStartBehavior:)`.
    pub fn drag_start_behavior(mut self, drag_start_behavior: DragStartBehavior) -> TextField {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `TextField(enableInteractiveSelection:)`.
    pub fn enable_interactive_selection(mut self, enable_interactive_selection: bool) -> TextField {
        self.enable_interactive_selection = Some(enable_interactive_selection);
        self
    }

    /// Dart `TextField(selectAllOnFocus:)`.
    pub fn select_all_on_focus(mut self, select_all_on_focus: bool) -> TextField {
        self.select_all_on_focus = Some(select_all_on_focus);
        self
    }

    /// Dart `TextField(selectionControls:)`.
    pub fn selection_controls(
        mut self,
        selection_controls: Rc<dyn TextSelectionControls>,
    ) -> TextField {
        self.selection_controls = Some(selection_controls);
        self
    }

    /// Dart `TextField(onTap:)`.
    pub fn on_tap(mut self, on_tap: GestureTapCallback) -> TextField {
        self.on_tap = Some(on_tap);
        self
    }

    /// Dart `TextField(onTapAlwaysCalled:)`.
    pub fn on_tap_always_called(mut self, on_tap_always_called: bool) -> TextField {
        self.on_tap_always_called = on_tap_always_called;
        self
    }

    /// Dart `TextField(onTapOutside:)`.
    pub fn on_tap_outside(
        mut self,
        on_tap_outside: impl Fn(&mut App, PointerDownEvent) + 'static,
    ) -> TextField {
        self.on_tap_outside = Some(Rc::new(on_tap_outside));
        self
    }

    /// Dart `TextField(onTapUpOutside:)`.
    pub fn on_tap_up_outside(
        mut self,
        on_tap_up_outside: impl Fn(&mut App, PointerUpEvent) + 'static,
    ) -> TextField {
        self.on_tap_up_outside = Some(Rc::new(on_tap_up_outside));
        self
    }

    /// Dart `TextField(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> TextField {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `TextField(buildCounter:)`.
    pub fn build_counter(
        mut self,
        build_counter: impl Fn(&mut App, BuildContext, i32, Option<i32>, bool) -> Option<WidgetRef>
        + 'static,
    ) -> TextField {
        self.build_counter = Some(Rc::new(build_counter));
        self
    }

    /// Dart `TextField(scrollController:)`.
    pub fn scroll_controller(mut self, scroll_controller: ScrollControllerRef) -> TextField {
        self.scroll_controller = Some(scroll_controller);
        self
    }

    /// Dart `TextField(scrollPhysics:)`.
    pub fn scroll_physics(mut self, scroll_physics: ScrollPhysicsRef) -> TextField {
        self.scroll_physics = Some(scroll_physics);
        self
    }

    /// Dart `TextField(autofillHints:)`.
    pub fn autofill_hints(mut self, autofill_hints: Vec<String>) -> TextField {
        self.autofill_hints = autofill_hints;
        self
    }

    /// Dart `TextField(contentInsertionConfiguration:)`.
    pub fn content_insertion_configuration(
        mut self,
        content_insertion_configuration: ContentInsertionConfiguration,
    ) -> TextField {
        self.content_insertion_configuration = Some(content_insertion_configuration);
        self
    }

    /// Dart `TextField(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> TextField {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `TextField(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> TextField {
        self.restoration_id = Some(restoration_id.into());
        self
    }

    /// Dart `TextField(scribbleEnabled:)`.
    #[deprecated(note = "Use `stylus_handwriting_enabled` instead.")]
    pub fn scribble_enabled(mut self, scribble_enabled: bool) -> TextField {
        self.scribble_enabled = scribble_enabled;
        self
    }

    /// Dart `TextField(stylusHandwritingEnabled:)`.
    pub fn stylus_handwriting_enabled(mut self, stylus_handwriting_enabled: bool) -> TextField {
        self.stylus_handwriting_enabled = stylus_handwriting_enabled;
        self
    }

    /// Dart `TextField(enableIMEPersonalizedLearning:)`.
    pub fn enable_ime_personalized_learning(
        mut self,
        enable_ime_personalized_learning: bool,
    ) -> TextField {
        self.enable_ime_personalized_learning = enable_ime_personalized_learning;
        self
    }

    /// Dart `TextField(enableInlinePrediction:)`.
    pub fn enable_inline_prediction(mut self, enable_inline_prediction: bool) -> TextField {
        self.enable_inline_prediction = Some(enable_inline_prediction);
        self
    }

    /// Dart `TextField(contextMenuBuilder:)`.
    pub fn context_menu_builder(
        mut self,
        context_menu_builder: Option<EditableTextContextMenuBuilder>,
    ) -> TextField {
        self.context_menu_builder = context_menu_builder;
        self
    }

    /// Dart `TextField(canRequestFocus:)`.
    pub fn can_request_focus(mut self, can_request_focus: bool) -> TextField {
        self.can_request_focus = can_request_focus;
        self
    }

    /// Dart `TextField(spellCheckConfiguration:)`.
    pub fn spell_check_configuration(
        mut self,
        spell_check_configuration: SpellCheckConfiguration,
    ) -> TextField {
        self.spell_check_configuration = Some(spell_check_configuration);
        self
    }

    /// Dart `TextField(magnifierConfiguration:)`.
    pub fn magnifier_configuration(
        mut self,
        magnifier_configuration: TextMagnifierConfiguration,
    ) -> TextField {
        self.magnifier_configuration = Some(magnifier_configuration);
        self
    }

    /// Dart `TextField(hintLocales:)`.
    pub fn hint_locales(mut self, hint_locales: Vec<Locale>) -> TextField {
        self.hint_locales = Some(hint_locales);
        self
    }

    /// Dart's `keyboardType` field, as its constructor fills it.
    pub fn effective_keyboard_type(&self) -> TextInputType {
        self.keyboard_type.unwrap_or(if self.max_lines == Some(1) {
            TextInputType::TEXT
        } else {
            TextInputType::MULTILINE
        })
    }

    /// Dart's `smartDashesType` field, as its constructor fills it.
    pub fn effective_smart_dashes_type(&self) -> SmartDashesType {
        self.smart_dashes_type.unwrap_or(if self.obscure_text {
            SmartDashesType::Disabled
        } else {
            SmartDashesType::Enabled
        })
    }

    /// Dart's `smartQuotesType` field, as its constructor fills it.
    pub fn effective_smart_quotes_type(&self) -> SmartQuotesType {
        self.smart_quotes_type.unwrap_or(if self.obscure_text {
            SmartQuotesType::Disabled
        } else {
            SmartQuotesType::Enabled
        })
    }

    /// Dart's `enableInteractiveSelection` field, as its constructor fills it.
    pub fn effective_enable_interactive_selection(&self) -> bool {
        self.enable_interactive_selection
            .unwrap_or(!self.read_only || !self.obscure_text)
    }

    /// {@macro flutter.widgets.editableText.selectionEnabled}
    pub fn selection_enabled(&self) -> bool {
        self.effective_enable_interactive_selection()
    }

    fn default_context_menu_builder(
        app: &mut App,
        _context: BuildContext,
        editable_text_state: Handle<EditableTextState>,
    ) -> WidgetRef {
        if SystemContextMenu::is_supported_by_field(app, editable_text_state) {
            return SystemContextMenu::editable_text(app, editable_text_state).into_widget();
        }
        AdaptiveTextSelectionToolbar::editable_text(app, editable_text_state).into_widget()
    }

    /// The [`TextStyle`] used to indicate misspelled words in the Material style.
    ///
    /// See also:
    ///  * [`SpellCheckConfiguration::misspelled_text_style`], the style configured to mark
    ///    misspelled words with.
    ///  * `CupertinoTextField::cupertino_misspelled_text_style`, the style configured to mark
    ///    misspelled words with in the Cupertino style.
    pub fn material_misspelled_text_style() -> TextStyle {
        TextStyle::new()
            .decoration(TextDecoration::UNDERLINE)
            .decoration_color(Colors::RED.primary())
            .decoration_style(TextDecorationStyle::Wavy)
    }

    /// Default builder for [`TextField`]'s spell check suggestions toolbar.
    ///
    /// On Apple platforms, builds an iOS-style toolbar. Everywhere else, builds an
    /// Android-style toolbar.
    ///
    /// See also:
    ///  * `spell_check_configuration`, where this is typically specified for [`TextField`].
    ///  * [`SpellCheckConfiguration::spell_check_suggestions_toolbar_builder`], the parameter
    ///    for which this is the default value for [`TextField`].
    ///  * `CupertinoTextField::default_spell_check_suggestions_toolbar_builder`, which is like
    ///    this but specifies the default for `CupertinoTextField`.
    pub fn default_spell_check_suggestions_toolbar_builder(
        app: &mut App,
        _context: BuildContext,
        editable_text_state: Handle<EditableTextState>,
    ) -> WidgetRef {
        match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::MacOS => {
                CupertinoSpellCheckSuggestionsToolbar::editable_text(app, editable_text_state)
                    .into_widget()
            }
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => {
                SpellCheckSuggestionsToolbar::editable_text(app, editable_text_state).into_widget()
            }
        }
    }

    /// Dart's tear-off `TextField.defaultSpellCheckSuggestionsToolbarBuilder`: one value per
    /// thread, so that it keeps one identity, which [`SpellCheckConfiguration`]'s comparison
    /// reads.
    fn default_spell_check_suggestions_toolbar_builder_tear_off() -> EditableTextContextMenuBuilder
    {
        thread_local! {
            static BUILDER: EditableTextContextMenuBuilder =
                Rc::new(TextField::default_spell_check_suggestions_toolbar_builder);
        }
        BUILDER.with(Rc::clone)
    }

    /// Returns a new [`SpellCheckConfiguration`] where the given configuration has had any
    /// missing values replaced with their defaults for the Android platform.
    pub fn infer_android_spell_check_configuration(
        configuration: Option<&SpellCheckConfiguration>,
    ) -> SpellCheckConfiguration {
        let Some(configuration) = configuration else {
            return SpellCheckConfiguration::disabled();
        };
        if *configuration == SpellCheckConfiguration::disabled() {
            return SpellCheckConfiguration::disabled();
        }
        configuration
            .copy_with()
            .misspelled_text_style(
                configuration
                    .misspelled_text_style
                    .clone()
                    .unwrap_or_else(TextField::material_misspelled_text_style),
            )
            .spell_check_suggestions_toolbar_builder(
                configuration
                    .spell_check_suggestions_toolbar_builder
                    .clone()
                    .unwrap_or_else(
                        TextField::default_spell_check_suggestions_toolbar_builder_tear_off,
                    ),
            )
    }

    /// Dart's constructor asserts across fields.
    fn debug_check_constructor(&self) {
        debug_assert!(
            self.max_lines
                .is_none_or(|max| self.min_lines.is_none_or(|min| max >= min)),
            "minLines can't be greater than maxLines"
        );
        debug_assert!(
            !self.expands || (self.max_lines.is_none() && self.min_lines.is_none()),
            "minLines and maxLines must be null when expands is true."
        );
        debug_assert!(
            !self.obscure_text || self.max_lines == Some(1),
            "Obscured fields cannot be multiline."
        );
        // Assert the following instead of setting it directly to avoid surprising the user by
        // silently changing the value they set.
        debug_assert!(
            self.text_input_action != Some(TextInputAction::Newline)
                || self.max_lines == Some(1)
                || self.effective_keyboard_type() != TextInputType::TEXT,
            "Use keyboardType TextInputType.multiline when using TextInputAction.newline on a multiline TextField."
        );
    }
}

impl Default for TextField {
    fn default() -> TextField {
        TextField::new()
    }
}

impl Debug for TextField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextField")
            .field("controller", &self.controller)
            .field("focus_node", &self.focus_node)
            .field("undo_controller", &self.undo_controller)
            .field("enabled", &self.enabled)
            .field("decoration", &self.decoration)
            .field("keyboard_type", &self.effective_keyboard_type())
            .field("style", &self.style)
            .field("autofocus", &self.autofocus)
            .field("obscuring_character", &self.obscuring_character)
            .field("obscure_text", &self.obscure_text)
            .field("autocorrect", &self.autocorrect)
            .field("smart_dashes_type", &self.effective_smart_dashes_type())
            .field("smart_quotes_type", &self.effective_smart_quotes_type())
            .field("enable_suggestions", &self.enable_suggestions)
            .field("max_lines", &self.max_lines)
            .field("min_lines", &self.min_lines)
            .field("expands", &self.expands)
            .field("max_length", &self.max_length)
            .field("max_length_enforcement", &self.max_length_enforcement)
            .field("text_input_action", &self.text_input_action)
            .field("text_capitalization", &self.text_capitalization)
            .field("text_align", &self.text_align)
            .field("text_align_vertical", &self.text_align_vertical)
            .field("text_direction", &self.text_direction)
            .field("cursor_width", &self.cursor_width)
            .field("cursor_height", &self.cursor_height)
            .field("cursor_radius", &self.cursor_radius)
            .field("cursor_opacity_animates", &self.cursor_opacity_animates)
            .field("cursor_color", &self.cursor_color)
            .field("cursor_error_color", &self.cursor_error_color)
            .field("keyboard_appearance", &self.keyboard_appearance)
            .field("scroll_padding", &self.scroll_padding)
            .field("selection_enabled", &self.selection_enabled())
            .field("scroll_controller", &self.scroll_controller)
            .field("clip_behavior", &self.clip_behavior)
            .field(
                "stylus_handwriting_enabled",
                &self.stylus_handwriting_enabled,
            )
            .field(
                "enable_ime_personalized_learning",
                &self.enable_ime_personalized_learning,
            )
            .field("enable_inline_prediction", &self.enable_inline_prediction)
            .field("spell_check_configuration", &self.spell_check_configuration)
            .field("hint_locales", &self.hint_locales)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for TextField {
    type State = TextFieldState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        TextField::debug_check_constructor(self);
    }

    fn create_state(&self) -> TextFieldState {
        TextFieldState {
            state: StateData::new(),
            restoration: RestorationMixinData::new(),
            controller: None,
            focus_node: None,
            is_hovering: false,
            show_selection_handles: false,
            selection_gesture_detector_builder: None,
            force_press_enabled: false,
            editable_text_key: GlobalKey::new(),
            internal_states_controller: None,
        }
    }
}

/// Dart's `_TextFieldState`.
pub struct TextFieldState {
    state: StateData<TextField>,
    restoration: RestorationMixinData,
    controller: Option<Ref<RestorableTextEditingController>>,
    focus_node: Option<Ref<FocusNode>>,
    is_hovering: bool,
    show_selection_handles: bool,
    selection_gesture_detector_builder: Option<Ref<TextFieldSelectionGestureDetectorBuilder>>,
    // API for TextSelectionGestureDetectorBuilderDelegate.
    force_press_enabled: bool,
    editable_text_key: GlobalKey,
    // End of API for TextSelectionGestureDetectorBuilderDelegate.
    internal_states_controller: Option<Ref<MaterialStatesController>>,
}

impl TextFieldState {
    fn effective_controller(self: Handle<Self>, app: &App) -> TextEditingControllerRef {
        if let Some(controller) = self.widget(app).controller.clone() {
            return controller;
        }
        let restorable = app
            .get(self)
            .controller
            .handle()
            .expect("a local controller exists when the widget does not supply one");
        RestorableListenable::value(restorable, app)
            .keep(app)
            .into_erased()
    }

    fn effective_focus_node(self: Handle<Self>, app: &mut App) -> FocusNodeRef {
        if let Some(focus_node) = self.widget(app).focus_node.clone() {
            return focus_node;
        }
        if let Some(focus_node) = app.get(self).focus_node.as_ref() {
            return focus_node.erase();
        }
        let focus_node = FocusNode::new(app);
        app.get_mut(self).focus_node = Some(focus_node.clone());
        focus_node.into_erased()
    }

    fn effective_max_length_enforcement(self: Handle<Self>, app: &mut App) -> MaxLengthEnforcement {
        if let Some(enforcement) = self.widget(app).max_length_enforcement {
            return enforcement;
        }
        let context = self.context(app);
        let platform = Theme::of(app, context).platform;
        LengthLimitingTextInputFormatter::get_default_max_length_enforcement(Some(platform))
    }

    /// Dart's `needsCounter`.
    pub fn needs_counter(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        widget.max_length.is_some()
            && widget
                .decoration
                .as_ref()
                .is_some_and(|decoration| decoration.counter_text.is_none())
    }

    fn is_enabled(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        widget
            .enabled
            .or(widget
                .decoration
                .as_ref()
                .map(|decoration| decoration.enabled))
            .unwrap_or(true)
    }

    fn current_length(self: Handle<Self>, app: &App) -> i32 {
        character_count(self.effective_controller(app).text_value(app))
    }

    fn has_intrinsic_error(self: Handle<Self>, app: &mut App) -> bool {
        let Some(max_length) = self.widget(app).max_length else {
            return false;
        };
        if max_length <= 0 {
            return false;
        }
        if self.widget(app).controller.is_none() {
            !self.restore_pending(app) && self.current_length(app) > max_length
        } else {
            self.current_length(app) > max_length
        }
    }

    fn has_error(self: Handle<Self>, app: &mut App) -> bool {
        let decoration = self.widget(app).decoration.as_ref();
        decoration.is_some_and(|decoration| decoration.error_text.is_some())
            || decoration.is_some_and(|decoration| decoration.error.is_some())
            || self.has_intrinsic_error(app)
    }

    fn error_color(self: Handle<Self>, app: &mut App) -> Color {
        if let Some(color) = self.widget(app).cursor_error_color {
            return color;
        }
        if let Some(color) = self
            .get_effective_decoration(app)
            .error_style
            .and_then(|style| style.color.as_ref().map(|color| color.color()))
        {
            return color;
        }
        let context = self.context(app);
        Theme::of(app, context).color_scheme.error
    }

    fn get_effective_decoration(self: Handle<Self>, app: &mut App) -> InputDecoration {
        let context = self.context(app);
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let use_material3 = Theme::of(app, context).use_material3;
        let decoration_theme = InputDecorationTheme::of(app, context);
        let is_enabled = self.is_enabled(app);
        let widget = self.widget(app);
        let hint_max_lines = widget
            .decoration
            .as_ref()
            .and_then(|decoration| decoration.hint_max_lines)
            .or(decoration_theme.hint_max_lines)
            .or(widget.max_lines);
        let mut effective_decoration = widget
            .decoration
            .clone()
            .unwrap_or_default()
            .apply_defaults(decoration_theme)
            .copy_with()
            .enabled(is_enabled);
        if let Some(hint_max_lines) = hint_max_lines {
            effective_decoration = effective_decoration.hint_max_lines(hint_max_lines);
        }

        // No need to build anything if counter or counterText were given directly.
        if effective_decoration.counter.is_some() || effective_decoration.counter_text.is_some() {
            return effective_decoration;
        }

        // If buildCounter was provided, use it to generate a counter widget.
        let current_length = self.current_length(app);
        if effective_decoration.counter.is_none()
            && effective_decoration.counter_text.is_none()
            && let Some(build_counter) = self.widget(app).build_counter.clone()
        {
            let is_focused = self.effective_focus_node(app).has_focus(app);
            let max_length = self.widget(app).max_length;
            let built_counter = build_counter(app, context, current_length, max_length, is_focused);
            // If buildCounter returns null, don't add a counter widget to the field.
            if let Some(built_counter) = built_counter {
                let counter = Semantics::new()
                    .container(true)
                    .live_region(is_focused)
                    .child(built_counter);
                return effective_decoration.copy_with().counter(counter);
            }
            return effective_decoration;
        }

        let Some(max_length) = self.widget(app).max_length else {
            return effective_decoration;
        }; // No counter widget

        let mut counter_text = format!("{current_length}");
        let mut semantic_counter_text = String::new();

        // Handle a real maxLength (positive number)
        if max_length > 0 {
            // Show the maxLength in the counter
            counter_text += &format!("/{max_length}");
            let remaining = (max_length - current_length).clamp(0, max_length);
            semantic_counter_text = localizations.remaining_text_field_character_count(remaining);
        }

        if self.has_intrinsic_error(app) {
            let error_text = effective_decoration.error_text.clone().unwrap_or_default();
            let counter_style = match effective_decoration.error_style.clone() {
                Some(error_style) => error_style,
                None if use_material3 => AnyTextStyle::new(m3_counter_error_style(app, context)),
                None => AnyTextStyle::new(m2_counter_error_style(app, context)),
            };
            return effective_decoration
                .copy_with()
                .error_text(error_text)
                .counter_style(counter_style)
                .counter_text(counter_text)
                .semantic_counter_text(semantic_counter_text);
        }

        effective_decoration
            .copy_with()
            .counter_text(counter_text)
            .semantic_counter_text(semantic_counter_text)
    }

    fn can_request_focus(self: Handle<Self>, app: &mut App) -> bool {
        let context = self.context(app);
        let mode = MediaQuery::maybe_navigation_mode_of(app, context)
            .unwrap_or(NavigationMode::Traditional);
        match mode {
            NavigationMode::Traditional => {
                self.widget(app).can_request_focus && self.is_enabled(app)
            }
            NavigationMode::Directional => true,
        }
    }

    fn register_controller(self: Handle<Self>, app: &mut App) {
        let controller = app
            .get(self)
            .controller
            .handle()
            .expect("register_controller needs a local controller");
        self.register_for_restoration(app, controller.as_property(), "controller");
    }

    fn create_local_controller(self: Handle<Self>, app: &mut App, value: Option<TextEditingValue>) {
        debug_assert!(app.get(self).controller.is_none());
        let controller = match value {
            None => RestorableTextEditingController::new(app),
            Some(value) => RestorableTextEditingController::from_value(app, value),
        };
        app.get_mut(self).controller = Some(controller.clone());
        if !self.restore_pending(app) {
            self.register_controller(app);
        }
    }

    fn editable_text(self: Handle<Self>, app: &mut App) -> Option<Handle<EditableTextState>> {
        let key = app.get(self).editable_text_key.clone();
        key.current_state::<EditableTextState>(app)
    }

    fn request_keyboard(self: Handle<Self>, app: &mut App) {
        if let Some(editable_text) = self.editable_text(app) {
            editable_text.request_keyboard(app);
        }
    }

    fn should_show_selection_handles(
        self: Handle<Self>,
        app: &mut App,
        cause: Option<SelectionChangedCause>,
    ) -> bool {
        // When the text field is activated by something that doesn't trigger the
        // selection toolbar, we shouldn't show the handles either.
        let builder = app
            .get(self)
            .selection_gesture_detector_builder
            .handle()
            .expect("created in init_state");
        if !builder.should_show_selection_toolbar(app)
            || !builder.should_show_selection_handles(app)
        {
            return false;
        }

        if cause == Some(SelectionChangedCause::Keyboard) {
            return false;
        }

        if self.widget(app).read_only
            && self.effective_controller(app).selection(app).is_collapsed()
        {
            return false;
        }

        if !self.is_enabled(app) {
            return false;
        }

        if cause == Some(SelectionChangedCause::LongPress)
            || cause == Some(SelectionChangedCause::StylusHandwriting)
        {
            return true;
        }

        if !self.effective_controller(app).text_value(app).is_empty() {
            return true;
        }

        false
    }

    fn handle_focus_changed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_| {
            // Rebuild the widget on focus change to show/hide the text selection
            // highlight.
        });
        let has_focus = self.effective_focus_node(app).has_focus(app);
        self.states_controller(app)
            .update(app, WidgetState::Focused, has_focus);
    }

    fn handle_selection_changed(
        self: Handle<Self>,
        app: &mut App,
        selection: TextSelection,
        cause: Option<SelectionChangedCause>,
    ) {
        let will_show_selection_handles = self.should_show_selection_handles(app, cause);
        if will_show_selection_handles != app.get(self).show_selection_handles {
            self.set_state(app, |state| {
                state.show_selection_handles = will_show_selection_handles;
            });
        }

        let context = self.context(app);
        match Theme::of(app, context).platform {
            TargetPlatform::IOS
            | TargetPlatform::MacOS
            | TargetPlatform::Linux
            | TargetPlatform::Windows
            | TargetPlatform::Fuchsia
            | TargetPlatform::Android => {
                if cause == Some(SelectionChangedCause::LongPress)
                    && let Some(editable_text) = self.editable_text(app)
                {
                    editable_text.bring_into_view(app, selection.extent());
                }
            }
        }

        match Theme::of(app, context).platform {
            TargetPlatform::IOS | TargetPlatform::Fuchsia | TargetPlatform::Android => {}
            TargetPlatform::MacOS | TargetPlatform::Linux | TargetPlatform::Windows => {
                if cause == Some(SelectionChangedCause::Drag)
                    && let Some(editable_text) = self.editable_text(app)
                {
                    editable_text.hide_toolbar(app, true);
                }
            }
        }
    }

    fn handle_selection_handle_tapped(self: Handle<Self>, app: &mut App) {
        if self.effective_controller(app).selection(app).is_collapsed() {
            self.editable_text(app)
                .expect("the EditableText is mounted")
                .toggle_toolbar(app, true);
        }
    }

    fn handle_hover(self: Handle<Self>, app: &mut App, hovering: bool) {
        if hovering != app.get(self).is_hovering {
            self.set_state(app, |state| {
                state.is_hovering = hovering;
            });
            self.states_controller(app)
                .update(app, WidgetState::Hovered, hovering);
        }
    }

    fn handle_states_controller_change(self: Handle<Self>, app: &mut App) {
        // Force a rebuild to resolve WidgetStateProperty properties.
        self.set_state(app, |_| {});
    }

    fn states_controller(self: Handle<Self>, app: &App) -> Handle<MaterialStatesController> {
        self.widget(app)
            .states_controller
            .handle()
            .or(app.get(self).internal_states_controller.handle())
            .expect("created in init_states_controller")
    }

    fn init_states_controller(self: Handle<Self>, app: &mut App) {
        if self.widget(app).states_controller.is_none() {
            let controller = MaterialStatesController::new(app, None);
            app.get_mut(self).internal_states_controller = Some(controller);
        }
        self.update_states_controller(app);
        self.states_controller(app).add_listener(
            app,
            Listener::handle_method(self, Self::handle_states_controller_change),
        );
    }

    /// The four `_statesController.update` calls `_initStatesController` and
    /// `didUpdateWidget` both make.
    fn update_states_controller(self: Handle<Self>, app: &mut App) {
        let states_controller = self.states_controller(app);
        let is_enabled = self.is_enabled(app);
        states_controller.update(app, WidgetState::Disabled, !is_enabled);
        let is_hovering = app.get(self).is_hovering;
        states_controller.update(app, WidgetState::Hovered, is_hovering);
        let has_focus = self.effective_focus_node(app).has_focus(app);
        states_controller.update(app, WidgetState::Focused, has_focus);
        let has_error = self.has_error(app);
        states_controller.update(app, WidgetState::Error, has_error);
    }

    fn states(self: Handle<Self>, app: &App) -> WidgetStates {
        app.get(self.states_controller(app)).value().clone()
    }

    fn get_input_style_for_state(self: Handle<Self>, app: &mut App, style: TextStyle) -> TextStyle {
        let context = self.context(app);
        let use_material3 = Theme::of(app, context).use_material3;
        let states = self.states(app);
        let state_style = if use_material3 {
            m3_state_input_style(app, context)
        } else {
            m2_state_input_style(app, context)
        };
        let state_style = <dyn WidgetStateProperty<TextStyle>>::resolve_as(&state_style, &states);
        let provided_style =
            <dyn WidgetStateProperty<TextStyle>>::resolve_as(&AnyTextStyle::new(style), &states);
        provided_style.value().merge(Some(state_style.value()))
    }
}

impl RestorationMixin for TextFieldState {
    waft_widgets::restoration_mixin_accessors!();

    fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
        self.widget(app).restoration_id.as_deref()
    }

    fn restore_state(
        self: Handle<Self>,
        app: &mut App,
        _old_bucket: Option<Handle<RestorationBucket>>,
        _initial_restore: bool,
    ) {
        if app.get(self).controller.is_some() {
            self.register_controller(app);
        }
    }
}

impl State for TextFieldState {
    type Widget = TextField;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let builder = app.create(TextFieldSelectionGestureDetectorBuilder {
            builder: TextSelectionGestureDetectorBuilderData::new(
                self.as_text_selection_gesture_detector_builder_delegate(),
            ),
            state: self,
        });
        app.get_mut(self).selection_gesture_detector_builder = Some(builder);
        if self.widget(app).controller.is_none() {
            self.create_local_controller(app, None);
        }
        let can_request_focus = self.widget(app).can_request_focus && self.is_enabled(app);
        let focus_node = self.effective_focus_node(app);
        focus_node.set_can_request_focus(app, can_request_focus);
        focus_node.add_listener(
            app,
            Listener::handle_method(self, Self::handle_focus_changed),
        );
        self.init_states_controller(app);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.did_change_dependencies_restoration(app);
        let can_request_focus = self.can_request_focus(app);
        self.effective_focus_node(app)
            .set_can_request_focus(app, can_request_focus);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &TextField) {
        self.did_update_restoration_id(app);
        if self.widget(app).controller.is_none() && old_widget.controller.is_some() {
            let value = old_widget
                .controller
                .as_ref()
                .map(|controller| controller.value(app).clone());
            self.create_local_controller(app, value);
        } else if self.widget(app).controller.is_some() && old_widget.controller.is_none() {
            let controller = app
                .get(self)
                .controller
                .handle()
                .expect("a local controller while the old widget had none");
            self.unregister_from_restoration(app, controller.as_property());
            RestorableProperty::dispose(controller, app);
            app.get_mut(self).controller = None;
        }

        if self.widget(app).focus_node != old_widget.focus_node {
            if let Some(old_focus) = old_widget
                .focus_node
                .clone()
                .or_else(|| app.get(self).focus_node.as_ref().map(Ref::erase))
            {
                old_focus.remove_listener(
                    app,
                    &Listener::handle_method(self, Self::handle_focus_changed),
                );
            }
            if let Some(focus) = self
                .widget(app)
                .focus_node
                .clone()
                .or_else(|| app.get(self).focus_node.as_ref().map(Ref::erase))
            {
                focus.add_listener(
                    app,
                    Listener::handle_method(self, Self::handle_focus_changed),
                );
            }
        }

        let can_request_focus = self.can_request_focus(app);
        self.effective_focus_node(app)
            .set_can_request_focus(app, can_request_focus);

        let read_only = self.widget(app).read_only;
        if self.effective_focus_node(app).has_focus(app)
            && read_only != old_widget.read_only
            && self.is_enabled(app)
            && self.effective_controller(app).selection(app).is_collapsed()
        {
            app.get_mut(self).show_selection_handles = !read_only;
        }

        if self.widget(app).states_controller == old_widget.states_controller {
            self.update_states_controller(app);
        } else {
            if let Some(old_states_controller) = old_widget.states_controller.clone() {
                old_states_controller.remove_listener(
                    app,
                    &Listener::handle_method(self, Self::handle_states_controller_change),
                );
            }
            if self.widget(app).states_controller.is_some()
                && let Some(internal) = app.get_mut(self).internal_states_controller.take()
            {
                internal.dispose(app);
            }
            self.init_states_controller(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.effective_focus_node(app).remove_listener(
            app,
            &Listener::handle_method(self, Self::handle_focus_changed),
        );
        if let Some(focus_node) = app.get(self).focus_node.handle() {
            focus_node.dispose(app);
        }
        if let Some(controller) = app.get(self).controller.handle() {
            RestorableProperty::dispose(controller, app);
        }
        self.states_controller(app).remove_listener(
            app,
            &Listener::handle_method(self, Self::handle_states_controller_change),
        );
        if let Some(internal) = app.get_mut(self).internal_states_controller.take() {
            internal.dispose(app);
        }
        self.dispose_restoration(app);
    }

    #[allow(deprecated)]
    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));
        debug_assert!(debug_check_has_material_localizations(app, context));
        debug_assert!(debug_check_has_directionality(
            app, context, None, None, None
        ));
        debug_assert!(
            !self
                .widget(app)
                .style
                .as_ref()
                .is_some_and(|style| !style.inherit
                    && (style.font_size.is_none() || style.text_baseline.is_none())),
            "inherit false style must supply fontSize and textBaseline"
        );

        let theme = Theme::of(app, context);
        let selection_style = DefaultSelectionStyle::of(app, context);
        let states = self.states(app);
        let provided_style = self
            .widget(app)
            .style
            .as_ref()
            .map(|style| <dyn WidgetStateProperty<TextStyle>>::resolve_as(style, &states));
        let base_style = if theme.use_material3 {
            m3_input_style(app, context)
        } else {
            theme
                .text_theme
                .title_medium
                .clone()
                .expect("the theme's text theme has a titleMedium")
        };
        let style = self
            .get_input_style_for_state(app, base_style)
            .merge(provided_style.as_ref().map(AnyTextStyle::value));
        let keyboard_appearance = self
            .widget(app)
            .keyboard_appearance
            .unwrap_or_else(|| theme.brightness());
        let controller = self.effective_controller(app);
        let focus_node = self.effective_focus_node(app);
        let mut formatters = self
            .widget(app)
            .input_formatters
            .clone()
            .unwrap_or_default();
        if let Some(max_length) = self.widget(app).max_length {
            formatters.push(TextInputFormatterRef(Rc::new(
                LengthLimitingTextInputFormatter::new(Some(max_length))
                    .max_length_enforcement(self.effective_max_length_enforcement(app)),
            )));
        }

        // Set configuration as disabled if not otherwise specified. If specified,
        // ensure that configuration uses the correct style for misspelled words for
        // the current platform, unless a custom style is specified.
        let spell_check_configuration = match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::MacOS => {
                CupertinoTextField::infer_ios_spell_check_configuration(
                    self.widget(app).spell_check_configuration.as_ref(),
                )
            }
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => TextField::infer_android_spell_check_configuration(
                self.widget(app).spell_check_configuration.as_ref(),
            ),
        };

        let mut text_selection_controls = self.widget(app).selection_controls.clone();
        let paint_cursor_above_text;
        let mut cursor_opacity_animates = self.widget(app).cursor_opacity_animates;
        let mut cursor_offset = None;
        let cursor_color;
        let selection_color;
        let mut autocorrection_text_rect_color = None;
        let mut cursor_radius = self.widget(app).cursor_radius;
        let mut handle_did_gain_accessibility_focus = None;
        let mut handle_did_lose_accessibility_focus = None;

        let has_error = self.has_error(app);
        let widget_cursor_color = self.widget(app).cursor_color;
        let selection_style_cursor_color = selection_style
            .cursor_color
            .as_ref()
            .map(|color| color.color());
        let selection_style_selection_color = selection_style
            .selection_color
            .as_ref()
            .map(|color| color.color());
        let this = self;
        let request_focus_on_accessibility_focus: VoidCallback = Rc::new(move |app| {
            // Automatically activate the TextField when it receives accessibility focus.
            let focus_node = this.effective_focus_node(app);
            if !focus_node.has_focus(app) && focus_node.can_request_focus(app) {
                focus_node.request_focus(app, None);
            }
        });
        let unfocus_on_accessibility_focus_lost: VoidCallback = Rc::new(move |app| {
            this.effective_focus_node(app)
                .unfocus(app, Default::default());
        });

        match theme.platform {
            TargetPlatform::IOS => {
                let cupertino_theme = CupertinoTheme::of(app, context);
                app.get_mut(self).force_press_enabled = true;
                text_selection_controls
                    .get_or_insert_with(cupertino_text_selection_handle_controls);
                paint_cursor_above_text = true;
                cursor_opacity_animates.get_or_insert(true);
                cursor_color = if has_error {
                    self.error_color(app)
                } else {
                    widget_cursor_color
                        .or(selection_style_cursor_color)
                        .unwrap_or_else(|| cupertino_theme.primary_color().color())
                };
                selection_color = selection_style_selection_color
                    .unwrap_or_else(|| cupertino_theme.primary_color().color().with_opacity(0.40));
                cursor_radius.get_or_insert(Radius::circular(2.0));
                cursor_offset = Some(Offset::new(
                    f64::from(IOS_HORIZONTAL_OFFSET)
                        / MediaQuery::device_pixel_ratio_of(app, context),
                    0.0,
                ));
                autocorrection_text_rect_color = Some(selection_color);
            }
            TargetPlatform::MacOS => {
                let cupertino_theme = CupertinoTheme::of(app, context);
                app.get_mut(self).force_press_enabled = false;
                text_selection_controls
                    .get_or_insert_with(cupertino_desktop_text_selection_handle_controls);
                paint_cursor_above_text = true;
                cursor_opacity_animates.get_or_insert(false);
                cursor_color = if has_error {
                    self.error_color(app)
                } else {
                    widget_cursor_color
                        .or(selection_style_cursor_color)
                        .unwrap_or_else(|| cupertino_theme.primary_color().color())
                };
                selection_color = selection_style_selection_color
                    .unwrap_or_else(|| cupertino_theme.primary_color().color().with_opacity(0.40));
                cursor_radius.get_or_insert(Radius::circular(2.0));
                cursor_offset = Some(Offset::new(
                    f64::from(IOS_HORIZONTAL_OFFSET)
                        / MediaQuery::device_pixel_ratio_of(app, context),
                    0.0,
                ));
                handle_did_gain_accessibility_focus = Some(request_focus_on_accessibility_focus);
                handle_did_lose_accessibility_focus = Some(unfocus_on_accessibility_focus_lost);
            }
            TargetPlatform::Android | TargetPlatform::Fuchsia => {
                app.get_mut(self).force_press_enabled = false;
                text_selection_controls.get_or_insert_with(material_text_selection_handle_controls);
                paint_cursor_above_text = false;
                cursor_opacity_animates.get_or_insert(false);
                cursor_color = if has_error {
                    self.error_color(app)
                } else {
                    widget_cursor_color
                        .or(selection_style_cursor_color)
                        .unwrap_or(theme.color_scheme.primary)
                };
                selection_color = selection_style_selection_color
                    .unwrap_or_else(|| theme.color_scheme.primary.with_opacity(0.40));
            }
            TargetPlatform::Linux => {
                app.get_mut(self).force_press_enabled = false;
                text_selection_controls.get_or_insert_with(desktop_text_selection_handle_controls);
                paint_cursor_above_text = false;
                cursor_opacity_animates.get_or_insert(false);
                cursor_color = if has_error {
                    self.error_color(app)
                } else {
                    widget_cursor_color
                        .or(selection_style_cursor_color)
                        .unwrap_or(theme.color_scheme.primary)
                };
                selection_color = selection_style_selection_color
                    .unwrap_or_else(|| theme.color_scheme.primary.with_opacity(0.40));
                handle_did_gain_accessibility_focus = Some(request_focus_on_accessibility_focus);
                handle_did_lose_accessibility_focus = Some(unfocus_on_accessibility_focus_lost);
            }
            TargetPlatform::Windows => {
                app.get_mut(self).force_press_enabled = false;
                text_selection_controls.get_or_insert_with(desktop_text_selection_handle_controls);
                paint_cursor_above_text = false;
                cursor_opacity_animates.get_or_insert(false);
                cursor_color = if has_error {
                    self.error_color(app)
                } else {
                    widget_cursor_color
                        .or(selection_style_cursor_color)
                        .unwrap_or(theme.color_scheme.primary)
                };
                selection_color = selection_style_selection_color
                    .unwrap_or_else(|| theme.color_scheme.primary.with_opacity(0.40));
                handle_did_gain_accessibility_focus = Some(request_focus_on_accessibility_focus);
                handle_did_lose_accessibility_focus = Some(unfocus_on_accessibility_focus_lost);
            }
        }

        let widget = self.widget(app);
        let mut editable = EditableText::new(
            controller.clone(),
            focus_node.clone(),
            style,
            cursor_color,
            CupertinoColors::INACTIVE_GRAY.color(),
        )
        .key(app.get(self).editable_text_key.clone().into())
        .read_only(widget.read_only || !self.is_enabled(app))
        .show_selection_handles(app.get(self).show_selection_handles)
        .keyboard_type(widget.effective_keyboard_type())
        .text_capitalization(widget.text_capitalization)
        .text_align(widget.text_align)
        .autofocus(widget.autofocus)
        .obscuring_character(widget.obscuring_character.clone())
        .obscure_text(widget.obscure_text)
        .smart_dashes_type(widget.effective_smart_dashes_type())
        .smart_quotes_type(widget.effective_smart_quotes_type())
        .enable_suggestions(widget.enable_suggestions)
        .max_lines(widget.max_lines)
        .expands(widget.expands)
        .on_selection_changed(move |app, selection, cause| {
            this.handle_selection_changed(app, selection, cause);
        })
        .group_id(widget.group_id)
        .on_selection_handle_tapped(Listener::handle_method(
            this,
            Self::handle_selection_handle_tapped,
        ))
        .input_formatters(formatters)
        .renderer_ignores_pointer(true)
        // TextField will handle the cursor
        .mouse_cursor(<dyn MouseCursor>::defer())
        .cursor_width(widget.cursor_width)
        .cursor_opacity_animates(cursor_opacity_animates.unwrap_or(false))
        .paint_cursor_above_text(paint_cursor_above_text)
        .scroll_padding(widget.scroll_padding)
        .keyboard_appearance(keyboard_appearance)
        .enable_interactive_selection(widget.effective_enable_interactive_selection())
        .drag_start_behavior(widget.drag_start_behavior)
        .autofill_hints(widget.autofill_hints.clone())
        .clip_behavior(widget.clip_behavior)
        .restoration_id("editable")
        .scribble_enabled(widget.scribble_enabled)
        .stylus_handwriting_enabled(widget.stylus_handwriting_enabled)
        .enable_ime_personalized_learning(widget.enable_ime_personalized_learning)
        .spell_check_configuration(spell_check_configuration)
        .magnifier_configuration(
            widget
                .magnifier_configuration
                .clone()
                .unwrap_or_else(|| TextMagnifier::adaptive_magnifier_configuration(app)),
        );
        if let Some(toolbar_options) = widget.toolbar_options {
            editable = editable.toolbar_options(toolbar_options);
        }
        if let Some(show_cursor) = widget.show_cursor {
            editable = editable.show_cursor(show_cursor);
        }
        if let Some(undo_controller) = widget.undo_controller.clone() {
            editable = editable.undo_controller(undo_controller);
        }
        if let Some(text_input_action) = widget.text_input_action {
            editable = editable.text_input_action(text_input_action);
        }
        if let Some(text_direction) = widget.text_direction {
            editable = editable.text_direction(text_direction);
        }
        if let Some(autocorrect) = widget.autocorrect {
            editable = editable.autocorrect(autocorrect);
        }
        if let Some(min_lines) = widget.min_lines {
            editable = editable.min_lines(min_lines);
        }
        // Only show the selection highlight when the text field is focused.
        if focus_node.has_focus(app) {
            editable = editable.selection_color(selection_color);
        }
        let widget = self.widget(app);
        if widget.selection_enabled()
            && let Some(text_selection_controls) = text_selection_controls
        {
            editable = editable.selection_controls(text_selection_controls);
        }
        // The callbacks pass through as the same values, as Dart hands its function objects on.
        editable.on_changed = widget.on_changed.clone();
        editable.on_editing_complete = widget.on_editing_complete.clone();
        editable.on_submitted = widget.on_submitted.clone();
        editable.on_app_private_command = widget.on_app_private_command.clone();
        editable.on_tap_outside = widget.on_tap_outside.clone();
        editable.on_tap_up_outside = widget.on_tap_up_outside.clone();
        if let Some(cursor_height) = widget.cursor_height {
            editable = editable.cursor_height(cursor_height);
        }
        if let Some(cursor_radius) = cursor_radius {
            editable = editable.cursor_radius(cursor_radius);
        }
        if let Some(selection_height_style) = widget.selection_height_style {
            editable = editable.selection_height_style(selection_height_style);
        }
        if let Some(selection_width_style) = widget.selection_width_style {
            editable = editable.selection_width_style(selection_width_style);
        }
        if let Some(cursor_offset) = cursor_offset {
            editable = editable.cursor_offset(cursor_offset);
        }
        if let Some(select_all_on_focus) = widget.select_all_on_focus {
            editable = editable.select_all_on_focus(select_all_on_focus);
        }
        if let Some(scroll_controller) = widget.scroll_controller.clone() {
            editable = editable.scroll_controller(scroll_controller);
        }
        if let Some(scroll_physics) = widget.scroll_physics.clone() {
            editable = editable.scroll_physics(scroll_physics);
        }
        if let Some(autocorrection_text_rect_color) = autocorrection_text_rect_color {
            editable = editable.autocorrection_text_rect_color(autocorrection_text_rect_color);
        }
        if let Some(enable_inline_prediction) = widget.enable_inline_prediction {
            editable = editable.enable_inline_prediction(enable_inline_prediction);
        }
        if let Some(content_insertion_configuration) =
            widget.content_insertion_configuration.clone()
        {
            editable = editable.content_insertion_configuration(content_insertion_configuration);
        }
        if let Some(context_menu_builder) = widget.context_menu_builder.clone() {
            editable = editable.context_menu_builder(context_menu_builder);
        }
        if let Some(hint_locales) = widget.hint_locales.clone() {
            editable = editable.hint_locales(hint_locales);
        }

        let bucket = self.bucket(app).and_then(|bucket| bucket.upgrade(app));
        let mut child = RepaintBoundary::new()
            .child(UnmanagedRestorationScope::new(editable).bucket(bucket))
            .into_widget();

        if self.widget(app).decoration.is_some() {
            let focus_node = focus_node.clone();
            let controller = controller.clone();
            let animation: Rc<dyn Listenable> = Rc::new(MergingListenable::new(vec![
                Some(Rc::new(focus_node.clone()) as Rc<dyn Listenable>),
                Some(Rc::new(controller.clone()) as Rc<dyn Listenable>),
            ]));
            child = AnimatedBuilder::new(animation, move |app, _context, child| {
                let widget = this.widget(app);
                let base_style = widget.style.as_ref().map(|style| style.value().clone());
                let text_align = widget.text_align;
                let text_align_vertical = widget.text_align_vertical;
                let expands = widget.expands;
                let is_hovering = app.get(this).is_hovering;
                let is_focused = focus_node.has_focus(app);
                let is_empty = controller.text_value(app).is_empty();
                let mut decorator = InputDecorator::new(this.get_effective_decoration(app))
                    .text_align(text_align)
                    .is_hovering(is_hovering)
                    .is_focused(is_focused)
                    .is_empty(is_empty)
                    .expands(expands);
                if let Some(base_style) = base_style {
                    decorator = decorator.base_style(base_style);
                }
                if let Some(text_align_vertical) = text_align_vertical {
                    decorator = decorator.text_align_vertical(text_align_vertical);
                }
                if let Some(child) = child {
                    decorator = decorator.child(child.clone());
                }
                decorator.into_widget()
            })
            .child(child)
            .into_widget();
        }
        let effective_mouse_cursor = <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(
            &self
                .widget(app)
                .mouse_cursor
                .clone()
                .unwrap_or_else(|| WidgetStateMouseCursor::textable().into()),
            &states,
        );

        let max_length = self.widget(app).max_length;
        let semantics_max_value_length = match max_length {
            Some(max_length)
                if self.effective_max_length_enforcement(app) != MaxLengthEnforcement::None
                    && max_length > 0 =>
            {
                Some(max_length)
            }
            _ => None,
        };

        let builder = app
            .get(self)
            .selection_gesture_detector_builder
            .handle()
            .expect("created in init_state");
        let gesture_detector = TextSelectionGestureDetectorBuilder::build_gesture_detector(
            builder,
            app,
            None,
            Some(HitTestBehavior::Translucent),
            child,
        );
        let ignoring = self
            .widget(app)
            .ignore_pointers
            .unwrap_or(!self.is_enabled(app));
        MouseRegion::new()
            .cursor(effective_mouse_cursor)
            .on_enter(Rc::new(move |app, _event| this.handle_hover(app, true)))
            .on_exit(Rc::new(move |app, _event| this.handle_hover(app, false)))
            .child(TextFieldTapRegion::new(
                IgnorePointer::new().ignoring(ignoring).child(
                    AnimatedBuilder::new(
                        Rc::new(controller.clone()), // changes the _currentLength
                        move |app, _context, child| {
                            this.build_semantics(
                                app,
                                semantics_max_value_length,
                                handle_did_gain_accessibility_focus.clone(),
                                handle_did_lose_accessibility_focus.clone(),
                                child,
                            )
                        },
                    )
                    .child(gesture_detector),
                ),
            ))
            .into_widget()
    }
}

impl TextFieldState {
    /// The `Semantics` the outer `AnimatedBuilder` of Dart's `build` returns.
    fn build_semantics(
        self: Handle<Self>,
        app: &mut App,
        semantics_max_value_length: Option<i32>,
        handle_did_gain_accessibility_focus: Option<VoidCallback>,
        handle_did_lose_accessibility_focus: Option<VoidCallback>,
        child: Option<&WidgetRef>,
    ) -> WidgetRef {
        let is_enabled = self.is_enabled(app);
        let mut semantics = Semantics::new()
            .enabled(is_enabled)
            .current_value_length(self.current_length(app));
        if let Some(max_value_length) = semantics_max_value_length {
            semantics = semantics.max_value_length(max_value_length);
        }
        if !self.widget(app).read_only {
            semantics = semantics.on_tap(Rc::new(move |app| {
                let controller = self.effective_controller(app);
                if !controller.selection(app).is_valid() {
                    let length = controller.text_value(app).encode_utf16().count() as i32;
                    controller
                        .set_selection(app, TextSelection::collapsed(length, Default::default()));
                }
                self.request_keyboard(app);
            }));
        }
        if let Some(on_did_gain_accessibility_focus) = handle_did_gain_accessibility_focus {
            semantics = semantics.on_did_gain_accessibility_focus(on_did_gain_accessibility_focus);
        }
        if let Some(on_did_lose_accessibility_focus) = handle_did_lose_accessibility_focus {
            semantics = semantics.on_did_lose_accessibility_focus(on_did_lose_accessibility_focus);
        }
        if is_enabled {
            semantics = semantics.on_focus(Rc::new(move |app| {
                let focus_node = self.effective_focus_node(app);
                debug_assert!(
                    focus_node.can_request_focus(app),
                    "Received SemanticsAction.focus from the engine. However, the FocusNode \
                     of this text field cannot gain focus. This likely indicates a bug. \
                     If this text field cannot be focused (e.g. because it is not \
                     enabled), then its corresponding semantics node must be configured \
                     such that the assistive technology cannot request focus on it."
                );

                if focus_node.can_request_focus(app) && !focus_node.has_focus(app) {
                    focus_node.request_focus(app, None);
                } else if !self.widget(app).read_only {
                    // If the platform requested focus, that means that previously the
                    // platform believed that the text field did not have focus (even
                    // though Flutter's widget system believed otherwise). This likely
                    // means that the on-screen keyboard is hidden, or more generally,
                    // there is no current editing session in this field. To correct
                    // that, keyboard must be requested.
                    //
                    // A concrete scenario where this can happen is when the user
                    // dismisses the keyboard on the web. The editing session is
                    // closed by the engine, but the text field widget stays focused
                    // in the framework.
                    self.request_keyboard(app);
                }
            }));
        }
        if let Some(child) = child {
            semantics = semantics.child(child.clone());
        }
        semantics.into_widget()
    }
}

impl TextSelectionGestureDetectorBuilderDelegate for TextFieldState {
    fn editable_text_key(self: Handle<Self>, app: &App) -> GlobalKey {
        app.get(self).editable_text_key.clone()
    }

    fn force_press_enabled(self: Handle<Self>, app: &App) -> bool {
        app.get(self).force_press_enabled
    }

    fn selection_enabled(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).selection_enabled() && self.is_enabled(app)
    }
}

/// Dart's `characters.length`: the extended grapheme clusters of UAX #29, as
/// `LengthLimitingTextInputFormatter` counts them.
fn character_count(text: &str) -> i32 {
    text.graphemes(true).count() as i32
}

fn m2_state_input_style(app: &mut App, context: BuildContext) -> AnyTextStyle {
    let theme = Theme::of(app, context);
    let disabled_color = theme.disabled_color;
    let title_medium_color = theme
        .text_theme
        .title_medium
        .as_ref()
        .and_then(|style| style.color.clone());
    WidgetStateTextStyle::resolve_with(move |states: &WidgetStates| {
        if states.contains(&WidgetState::Disabled) {
            return TextStyle::new().color(disabled_color);
        }
        let style = TextStyle::new();
        match title_medium_color.clone() {
            Some(color) => style.color(color),
            None => style,
        }
    })
    .into_any()
}

fn m2_counter_error_style(app: &mut App, context: BuildContext) -> TextStyle {
    let theme = Theme::of(app, context);
    theme
        .text_theme
        .body_small
        .clone()
        .expect("the theme's text theme has a bodySmall")
        .copy_with()
        .color(theme.color_scheme.error)
}

// Part of material_ui's `generated/text_field_defaults_m3.g.dart`.
//
// Do not edit by hand. The code is generated from data in the Material
// Design token database by the script:
//   packages/material_ui/tool/gen_defaults/bin/gen_defaults.dart.

fn m3_state_input_style(app: &mut App, context: BuildContext) -> AnyTextStyle {
    let body_large_color = Theme::of(app, context)
        .text_theme
        .body_large
        .clone()
        .expect("the theme's text theme has a bodyLarge")
        .color;
    WidgetStateTextStyle::resolve_with(move |states: &WidgetStates| {
        if states.contains(&WidgetState::Disabled) {
            let style = TextStyle::new();
            return match body_large_color.as_ref() {
                Some(color) => style.color(color.color().with_opacity(0.38)),
                None => style,
            };
        }
        let style = TextStyle::new();
        match body_large_color.clone() {
            Some(color) => style.color(color),
            None => style,
        }
    })
    .into_any()
}

fn m3_input_style(app: &mut App, context: BuildContext) -> TextStyle {
    Theme::of(app, context)
        .text_theme
        .body_large
        .clone()
        .expect("the theme's text theme has a bodyLarge")
}

fn m3_counter_error_style(app: &mut App, context: BuildContext) -> TextStyle {
    let theme = Theme::of(app, context);
    theme
        .text_theme
        .body_small
        .clone()
        .expect("the theme's text theme has a bodySmall")
        .copy_with()
        .color(theme.color_scheme.error)
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::time::Duration;

    use waft_embedder::{TextAffinity, TextPosition, TextRange};
    use waft_painting::TextSpan;
    use waft_rendering::{RenderBox as _, RenderEditable};
    use waft_services::{SystemMouseCursors, TextInputClient, TextInputFormatterRef};
    use waft_widgets::{
        AnyElement, Builder, Center, Column, FocusScope, IntoWidget, MediaQueryData, Opacity,
        Overlay, OverlayEntry, SizedBox, Text, TextEditingController, WidgetStates,
    };

    use waft_rendering::testing::{PaintCall, assert_more_or_less_equals_within, paints};
    use waft_services::{LogicalKeyboardKey, PhysicalKeyboardKey};
    use waft_widgets::testing::{
        TargetPlatformVariant, TestPointer, WidgetTester, expect, find, finds_nothing,
        finds_one_widget, test_widgets, test_widgets_variant,
    };

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::colors::Colors;
    use crate::constants::K_MIN_INTERACTIVE_DIMENSION;
    use crate::icon_button::IconButton;
    use crate::icons::Icons;
    use crate::input_border::OutlineInputBorder;
    use crate::input_decorator::InputDecorationThemeData;
    use crate::material::Material;
    use crate::scaffold::Scaffold;
    use crate::test_support::{PaintInspection, material_app};
    use crate::text_form_field::TextFormField;
    use crate::text_selection::material_text_selection_controls;
    use crate::text_theme::TextTheme;
    use crate::theme_data::ThemeData;
    use crate::typography::Typography;

    use waft_foundation::UniqueKey;
    use waft_test::{MockClipboard, TestPlatform};
    use waft_widgets::{Container, IntrinsicHeight};
    use waft_widgets::{FocusNodeLeaf, TextEditingControllerLeaf};

    const ALL_PLATFORMS: [TargetPlatform; 6] = [
        TargetPlatform::Android,
        TargetPlatform::Fuchsia,
        TargetPlatform::IOS,
        TargetPlatform::Linux,
        TargetPlatform::MacOS,
        TargetPlatform::Windows,
    ];
    const DESKTOP: [TargetPlatform; 3] = [
        TargetPlatform::Linux,
        TargetPlatform::MacOS,
        TargetPlatform::Windows,
    ];
    const MOBILE: [TargetPlatform; 3] = [
        TargetPlatform::Android,
        TargetPlatform::Fuchsia,
        TargetPlatform::IOS,
    ];

    const K_THREE_LINES: &str =
        "First line of text is\nSecond line goes until\nThird line of stuff";
    const K_MORE_THAN_FOUR_LINES: &str = "First line of text is\nSecond line goes until\nThird line of stuff\nFourth line won't display and ends at";

    /// A tester on `platform` with flutter_test's fonts, so a `Text` measures as in Flutter's
    /// tests.
    fn theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::new(&tester.cell.borrow())
    }

    fn m2_theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::builder()
            .use_material3(false)
            .build(&tester.cell.borrow())
    }

    /// Dart's `boilerplate(child:)`: the child centred on a `Material` in a `MaterialApp`.
    fn boilerplate<K>(tester: &WidgetTester, child: impl IntoWidget<K>) -> WidgetRef {
        material_app(
            theme(tester),
            Center::new()
                .child(Material::new().child(child))
                .into_widget(),
        )
    }

    /// Dart's `overlay(child:)`: the child centred on a `Material` in an entry of a bare
    /// `Overlay`, which gives the entry loose constraints, as Flutter's does.
    fn overlay<K>(tester: &WidgetTester, child: impl IntoWidget<K>) -> WidgetRef {
        let child = child.into_widget();
        let entry = {
            let mut app = tester.cell.borrow_mut();
            OverlayEntry::new(
                &mut app,
                Rc::new(move |_app, _context| {
                    Center::new()
                        .child(Material::new().child(child.clone()))
                        .into_widget()
                }),
                false,
                false,
                false,
            )
        };
        material_app(
            theme(tester),
            Overlay::new().initial_entries([entry]).into_widget(),
        )
    }

    /// Dart's `MaterialApp(theme: theme, home: Material(child: child))`.
    fn material_home<K>(theme: ThemeData, child: impl IntoWidget<K>) -> WidgetRef {
        material_app(theme, Material::new().child(child).into_widget())
    }

    fn controller_with(tester: &WidgetTester, text: &str) -> Ref<TextEditingController> {
        let mut app = tester.cell.borrow_mut();
        let controller = TextEditingController::new(&mut app);
        controller.set_text(&mut app, text);
        controller
    }

    fn editable_elements(tester: &WidgetTester) -> Vec<AnyElement> {
        find::by_type::<EditableText>().evaluate(tester)
    }

    fn editable_state_at(tester: &WidgetTester, index: usize) -> Handle<EditableTextState> {
        editable_elements(tester)[index]
            .state_handle::<EditableTextState>(&tester.cell.borrow())
            .expect("an EditableText state")
    }

    fn editable_state(tester: &WidgetTester) -> Handle<EditableTextState> {
        editable_state_at(tester, 0)
    }

    /// Reads the first `EditableText` widget: Dart's `tester.widget(find.byType(EditableText))`.
    fn editable<R>(tester: &WidgetTester, read: impl FnOnce(&EditableText) -> R) -> R {
        tester.widget::<EditableText, R>(editable_elements(tester)[0], read)
    }

    fn editable_at<R>(
        tester: &WidgetTester,
        index: usize,
        read: impl FnOnce(&EditableText) -> R,
    ) -> R {
        tester.widget::<EditableText, R>(editable_elements(tester)[index], read)
    }

    fn text_field_center(tester: &WidgetTester, index: usize) -> Offset {
        let element = find::by_type::<TextField>().evaluate(tester)[index];
        tester.get_rect(element).center()
    }

    /// Dart's `skipPastScrollingAnimation(tester)`.
    fn skip_past_scrolling_animation(tester: &mut WidgetTester) {
        tester.pump();
        tester.pump_by(Duration::from_millis(200));
    }

    /// Dart's `textOffsetToPosition(tester, offset)`: the global point just above the middle
    /// of the caret at `offset`.
    fn text_offset_to_position(tester: &WidgetTester, offset: i32) -> Offset {
        let editable = tester.all::<RenderEditable>()[0];
        let mut app = tester.cell.borrow_mut();
        let endpoints = editable.get_endpoints_for_selection(
            &mut app,
            TextSelection::collapsed(offset, TextAffinity::Downstream),
        );
        assert_eq!(endpoints.len(), 1);
        editable
            .as_box()
            .local_to_global(&app, endpoints[0].point, None)
            + Offset::new(0.0, -2.0)
    }

    fn selection(
        tester: &WidgetTester,
        controller: Handle<TextEditingController>,
    ) -> TextSelection {
        controller.selection(&tester.cell.borrow())
    }

    fn text(tester: &WidgetTester, controller: Handle<TextEditingController>) -> String {
        controller.text_value(&tester.cell.borrow()).to_owned()
    }

    fn text_color(tester: &WidgetTester, text: &str) -> Option<Color> {
        let element = find::text(text).evaluate(tester)[0];
        tester.widget::<Text, _>(element, |widget| {
            widget
                .style
                .as_ref()
                .and_then(|style| style.color.as_ref().map(|color| color.color()))
        })
    }

    fn has_focus(tester: &WidgetTester, focus_node: Handle<FocusNode>) -> bool {
        focus_node.has_focus(&mut tester.cell.borrow_mut())
    }

    /// A formatter standing in for Dart's `FilteringTextInputFormatter.deny(RegExp(r'[a-z]'),
    /// replacementString:)`, which Waft's `FilterPattern` cannot express.
    struct DenyLowercase(&'static str);

    impl waft_services::TextInputFormatter for DenyLowercase {
        fn format_edit_update(
            &self,
            _old_value: &TextEditingValue,
            new_value: &TextEditingValue,
        ) -> TextEditingValue {
            let text: String = new_value
                .text
                .chars()
                .map(|character| {
                    if character.is_ascii_lowercase() {
                        self.0.to_owned()
                    } else {
                        character.to_string()
                    }
                })
                .collect();
            let length = text.encode_utf16().count() as i32;
            TextEditingValue::new()
                .text(text)
                .selection(TextSelection::collapsed(length, TextAffinity::Downstream))
        }
    }

    /// A formatter standing in for Dart's `FilteringTextInputFormatter.allow(RegExp(r'\n[0-9]'))`.
    struct AllowNewlineDigit;

    impl waft_services::TextInputFormatter for AllowNewlineDigit {
        fn format_edit_update(
            &self,
            _old_value: &TextEditingValue,
            new_value: &TextEditingValue,
        ) -> TextEditingValue {
            let characters: Vec<char> = new_value.text.chars().collect();
            let mut text = String::new();
            let mut index = 0;
            while index + 1 < characters.len() {
                if characters[index] == '\n' && characters[index + 1].is_ascii_digit() {
                    text.push('\n');
                    text.push(characters[index + 1]);
                    index += 2;
                } else {
                    index += 1;
                }
            }
            let length = text.encode_utf16().count() as i32;
            TextEditingValue::new()
                .text(text)
                .selection(TextSelection::collapsed(length, TextAffinity::Downstream))
        }
    }

    fn formatter(
        formatter: impl waft_services::TextInputFormatter + 'static,
    ) -> TextInputFormatterRef {
        TextInputFormatterRef(Rc::new(formatter))
    }

    #[test]
    fn uses_default_selection_style_for_selection_and_cursor_colors_if_provided() {
        test_widgets(|tester| {
            let selection_color = Colors::ORANGE.primary();
            let cursor_color = Colors::RED.primary();
            let root = material_home(
                theme(tester),
                DefaultSelectionStyle::new(TextField::new().autofocus(true))
                    .selection_color(selection_color)
                    .cursor_color(cursor_color),
            );
            tester.pump_widget(root);
            tester.pump();
            let (selection, cursor) = editable(tester, |widget| {
                (widget.selection_color, widget.cursor_color)
            });
            assert_eq!(selection, Some(selection_color));
            assert_eq!(cursor, cursor_color);
        });
    }

    #[test]
    fn use_error_cursor_color_when_an_input_decoration_with_an_error_text_or_error_widget_is_provided()
     {
        test_widgets(|tester| {
            let teal = Colors::TEAL.primary();
            let root = material_home(
                theme(tester),
                TextField::new().autofocus(true).decoration(Some(
                    InputDecoration::new()
                        .error(Text::new("error"))
                        .error_style(TextStyle::new().color(teal)),
                )),
            );
            tester.pump_widget(root);
            tester.pump();
            assert_eq!(editable(tester, |widget| widget.cursor_color), teal);

            let root = material_home(
                theme(tester),
                TextField::new().autofocus(true).decoration(Some(
                    InputDecoration::new()
                        .error_text("error")
                        .error_style(TextStyle::new().color(teal)),
                )),
            );
            tester.pump_widget(root);
            tester.pump();
            assert_eq!(editable(tester, |widget| widget.cursor_color), teal);
        });
    }

    #[test]
    fn sets_cursor_opacity_animates_on_editable_text_correctly() {
        test_widgets(|tester| {
            let root = material_home(
                theme(tester),
                TextField::new()
                    .autofocus(true)
                    .cursor_opacity_animates(true),
            );
            tester.pump_widget(root);
            tester.pump();
            assert!(editable(tester, |widget| widget.cursor_opacity_animates));

            let root = material_home(
                theme(tester),
                TextField::new()
                    .autofocus(true)
                    .cursor_opacity_animates(false),
            );
            tester.pump_widget(root);
            tester.pump();
            assert!(!editable(tester, |widget| widget.cursor_opacity_animates));
        });
    }

    #[test]
    fn text_field_passes_on_editing_complete_to_editable_text() {
        test_widgets(|tester| {
            let field = TextField::new().on_editing_complete(|_app| {});
            let on_editing_complete = field.on_editing_complete.clone().expect("set above");
            let root = material_home(theme(tester), field);
            tester.pump_widget(root);

            assert_eq!(editable_elements(tester).len(), 1);
            let passed = editable(tester, |widget| widget.on_editing_complete.clone())
                .expect("the field passes its callback");
            assert!(Rc::ptr_eq(&passed, &on_editing_complete));
        });
    }

    /// Dart's `groupId: 'Group A'` and the rest: a distinct identity for each field's group.
    struct GroupA;
    struct GroupB;
    struct GroupC;

    #[test]
    fn the_second_text_field_is_clicked_triggers_the_on_tap_outside_callback_of_the_previous_text_field()
     {
        test_widgets(|tester| {
            let outside_click_a = Rc::new(Cell::new(false));
            let outside_click_b = Rc::new(Cell::new(false));
            let outside_click_c = Rc::new(Cell::new(false));
            let field = |group: TypeId, clicked: &Rc<Cell<bool>>| {
                let clicked = Rc::clone(clicked);
                Material::new()
                    .child(
                        TextField::new()
                            .group_id(group)
                            .on_tap_outside(move |_app, _event| clicked.set(true)),
                    )
                    .into_widget()
            };
            let root = material_app(
                theme(tester),
                waft_widgets::Align::new()
                    .alignment(waft_painting::AlignmentGeometry::TOP_LEFT)
                    .child(Column::new().children(vec![
                        Text::new("Outside").into_widget(),
                        field(TypeId::of::<GroupA>(), &outside_click_a),
                        field(TypeId::of::<GroupB>(), &outside_click_b),
                        field(TypeId::of::<GroupC>(), &outside_click_c),
                    ]))
                    .into_widget(),
            );
            tester.pump_widget(root);
            tester.pump();

            let click = |tester: &mut WidgetTester, index: usize, pointer: i64| {
                let center = text_field_center(tester, index);
                tester.tap_at(center, pointer);
                tester.enter_text(find::by_type::<EditableText>().at(index), "Hello");
                tester.pump();
            };

            let outside = || {
                (
                    outside_click_a.get(),
                    outside_click_b.get(),
                    outside_click_c.get(),
                )
            };
            assert_eq!(outside(), (false, false, false));

            click(tester, 0, 1);
            tester.show_keyboard(find::by_type::<EditableText>().at(0));
            assert_eq!(outside(), (false, false, false));

            click(tester, 1, 2);
            assert_eq!(outside(), (true, false, false));

            click(tester, 2, 3);
            assert_eq!(outside(), (true, true, false));

            let outside_text = find::text("Outside").evaluate(tester)[0];
            let at = tester.get_rect(outside_text).center();
            tester.tap_at(at, 4);
            assert_eq!(outside(), (true, true, true));
        });
    }

    #[test]
    fn cursor_has_expected_defaults() {
        test_widgets(|tester| {
            let root = overlay(tester, TextField::new());
            tester.pump_widget(root);
            let element = find::by_type::<TextField>().evaluate(tester)[0];
            let (width, height, radius) = tester.widget::<TextField, _>(element, |field| {
                (field.cursor_width, field.cursor_height, field.cursor_radius)
            });
            assert_eq!(width, 2.0);
            assert_eq!(height, None);
            assert_eq!(radius, None);
        });
    }

    #[test]
    fn cursor_has_expected_radius_value() {
        test_widgets(|tester| {
            let root = overlay(
                tester,
                TextField::new().cursor_radius(Radius::circular(3.0)),
            );
            tester.pump_widget(root);
            let element = find::by_type::<TextField>().evaluate(tester)[0];
            let (width, radius) = tester
                .widget::<TextField, _>(element, |field| (field.cursor_width, field.cursor_radius));
            assert_eq!(width, 2.0);
            assert_eq!(radius, Some(Radius::circular(3.0)));
        });
    }

    #[test]
    fn clip_behavior_has_expected_defaults() {
        test_widgets(|tester| {
            let root = overlay(tester, TextField::new());
            tester.pump_widget(root);
            let element = find::by_type::<TextField>().evaluate(tester)[0];
            let clip_behavior = tester.widget::<TextField, _>(element, |field| field.clip_behavior);
            assert_eq!(clip_behavior, Clip::HardEdge);
        });
    }

    #[test]
    fn caret_position_is_updated_on_tap() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = overlay(tester, TextField::new().controller(controller.erase()));
            tester.pump_widget(root);
            assert_eq!(selection(tester, controller.handle()).base_offset, -1);
            assert_eq!(selection(tester, controller.handle()).extent_offset, -1);

            let test_value = "abc def ghi";
            tester.enter_text(find::by_type::<EditableText>().at(0), test_value);
            skip_past_scrolling_animation(tester);

            // Tap to reposition the caret.
            let tap_index = test_value.find('e').unwrap() as i32;
            let e_pos = text_offset_to_position(tester, tap_index);
            tester.tap_at(e_pos, 1);
            tester.pump();

            assert_eq!(
                selection(tester, controller.handle()).base_offset,
                tap_index
            );
            assert_eq!(
                selection(tester, controller.handle()).extent_offset,
                tap_index
            );
        });
    }

    #[test]
    fn enable_interactive_selection_false_tap() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = overlay(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .enable_interactive_selection(false),
            );
            tester.pump_widget(root);
            assert_eq!(selection(tester, controller.handle()).base_offset, -1);
            assert_eq!(selection(tester, controller.handle()).extent_offset, -1);

            let test_value = "abc def ghi";
            tester.enter_text(find::by_type::<EditableText>().at(0), test_value);
            skip_past_scrolling_animation(tester);

            // Tap would ordinarily reposition the caret.
            let tap_index = test_value.find('e').unwrap() as i32;
            let e_pos = text_offset_to_position(tester, tap_index);
            tester.tap_at(e_pos, 1);
            tester.pump();

            assert_eq!(
                selection(tester, controller.handle()).base_offset,
                test_value.len() as i32
            );
            assert!(selection(tester, controller.handle()).is_collapsed());
        });
    }

    #[test]
    fn can_long_press_to_select() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = overlay(tester, TextField::new().controller(controller.erase()));
            tester.pump_widget(root);

            let test_value = "abc def ghi";
            tester.enter_text(find::by_type::<EditableText>().at(0), test_value);
            assert_eq!(text(tester, controller.handle()), test_value);
            skip_past_scrolling_animation(tester);

            assert!(selection(tester, controller.handle()).is_collapsed());

            // Long press the 'e' to select 'def'.
            let e_pos = text_offset_to_position(tester, test_value.find('e').unwrap() as i32);
            tester.long_press_at(e_pos, 7);
            tester.pump_and_settle();

            // 'def' is selected.
            assert_eq!(
                selection(tester, controller.handle()).base_offset,
                test_value.find('d').unwrap() as i32
            );
            assert_eq!(
                selection(tester, controller.handle()).extent_offset,
                test_value.find('f').unwrap() as i32 + 1
            );

            // Tapping elsewhere immediately collapses and moves the cursor.
            let h_pos = text_offset_to_position(tester, test_value.find('h').unwrap() as i32);
            tester.tap_at(h_pos, 8);
            tester.pump();

            assert!(selection(tester, controller.handle()).is_collapsed());
            assert_eq!(
                selection(tester, controller.handle()).base_offset,
                test_value.find('h').unwrap() as i32
            );
        });
    }

    #[test]
    fn mouse_long_press_is_just_like_a_tap() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = overlay(tester, TextField::new().controller(controller.erase()));
            tester.pump_widget(root);

            let test_value = "abc def ghi";
            tester.enter_text(find::by_type::<EditableText>().at(0), test_value);
            assert_eq!(text(tester, controller.handle()), test_value);
            skip_past_scrolling_animation(tester);

            assert!(selection(tester, controller.handle()).is_collapsed());

            // Long press the 'e' using a mouse device.
            let e_index = test_value.find('e').unwrap() as i32;
            let e_pos = text_offset_to_position(tester, e_index);
            let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.down(&mut gesture, e_pos);
            tester.pump_by(Duration::from_secs(2));
            tester.up(&gesture);
            tester.pump();

            // The cursor is placed just like a regular tap.
            assert_eq!(selection(tester, controller.handle()).base_offset, e_index);
            assert_eq!(
                selection(tester, controller.handle()).extent_offset,
                e_index
            );
        });
    }

    #[test]
    fn read_only_text_should_not_compose() {
        test_widgets(|tester| {
            let controller = {
                let mut app = tester.cell.borrow_mut();
                TextEditingController::from_value(
                    &mut app,
                    Some(
                        TextEditingValue::new()
                            .text("readonly")
                            // Simulate text composing.
                            .composing(TextRange::new(0, 8)),
                    ),
                )
            };
            let root = overlay(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .read_only(true),
            );
            tester.pump_widget(root);

            let render_editable = tester.all::<RenderEditable>()[0];
            let app = tester.cell.borrow();
            let span = render_editable.text(&app).expect("the field has a text");
            // There should be no composing.
            let expected = TextSpan::new().text("readonly");
            let expected = match span.style() {
                Some(style) => expected.style(style.clone()),
                None => expected,
            };
            assert!(span.eq_span(&*expected.into_span()), "{span:?}");
        });
    }

    #[test]
    fn enable_interactive_selection_false_long_press() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = overlay(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .enable_interactive_selection(false),
            );
            tester.pump_widget(root);

            let test_value = "abc def ghi";
            tester.enter_text(find::by_type::<EditableText>().at(0), test_value);
            assert_eq!(text(tester, controller.handle()), test_value);
            skip_past_scrolling_animation(tester);

            assert!(selection(tester, controller.handle()).is_collapsed());

            // Long press the 'e' to select 'def'.
            let e_pos = text_offset_to_position(tester, test_value.find('e').unwrap() as i32);
            tester.long_press_at(e_pos, 7);
            tester.pump_and_settle();

            assert!(selection(tester, controller.handle()).is_collapsed());
            assert_eq!(
                selection(tester, controller.handle()).base_offset,
                test_value.len() as i32
            );
        });
    }

    #[test]
    fn selection_updates_on_tap_down_desktop_platforms() {
        for platform in DESKTOP {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let controller = controller_with(tester, "");
                let root = material_home(
                    theme(tester),
                    TextField::new().controller(controller.erase()),
                );
                tester.pump_widget(root);

                tester.enter_text(find::by_type::<EditableText>().at(0), "abc def ghi");
                skip_past_scrolling_animation(tester);

                let e_pos = text_offset_to_position(tester, 5);
                let g_pos = text_offset_to_position(tester, 8);

                let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
                tester.down(&mut gesture, e_pos);
                tester.pump_and_settle();
                assert_eq!(
                    selection(tester, controller.handle()).base_offset,
                    5,
                    "{platform:?}"
                );
                assert_eq!(
                    selection(tester, controller.handle()).extent_offset,
                    5,
                    "{platform:?}"
                );

                tester.up(&gesture);
                tester.pump_and_settle_by(waft_gestures::K_DOUBLE_TAP_TIMEOUT);

                gesture.position = g_pos;
                tester.send(&gesture, waft_embedder::PointerChange::Down);
                tester.pump_and_settle();
                assert_eq!(
                    selection(tester, controller.handle()).base_offset,
                    8,
                    "{platform:?}"
                );
                assert_eq!(
                    selection(tester, controller.handle()).extent_offset,
                    8,
                    "{platform:?}"
                );

                // This should do nothing. The selection is set on tap down on desktop platforms.
                tester.up(&gesture);
                assert_eq!(
                    selection(tester, controller.handle()).base_offset,
                    8,
                    "{platform:?}"
                );
                assert_eq!(
                    selection(tester, controller.handle()).extent_offset,
                    8,
                    "{platform:?}"
                );
            });
        }
    }

    #[test]
    fn selection_updates_on_tap_up_mobile_platforms() {
        for platform in MOBILE {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let controller = controller_with(tester, "");
                let is_target_platform_apple = platform == TargetPlatform::IOS;
                let root = material_home(
                    theme(tester),
                    TextField::new().controller(controller.erase()),
                );
                tester.pump_widget(root);

                tester.enter_text(find::by_type::<EditableText>().at(0), "abc def ghi");
                skip_past_scrolling_animation(tester);

                let e_pos = text_offset_to_position(tester, 5);
                let g_pos = text_offset_to_position(tester, 8);

                let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
                tester.down(&mut gesture, e_pos);
                tester.up(&gesture);
                tester.pump_and_settle_by(waft_gestures::K_DOUBLE_TAP_TIMEOUT);

                gesture.position = g_pos;
                tester.send(&gesture, waft_embedder::PointerChange::Down);
                tester.pump_and_settle();
                assert_eq!(
                    selection(tester, controller.handle()).base_offset,
                    5,
                    "{platform:?}"
                );
                assert_eq!(
                    selection(tester, controller.handle()).extent_offset,
                    5,
                    "{platform:?}"
                );

                tester.up(&gesture);
                tester.pump_and_settle_by(waft_gestures::K_DOUBLE_TAP_TIMEOUT);
                assert_eq!(
                    selection(tester, controller.handle()).base_offset,
                    8,
                    "{platform:?}"
                );
                assert_eq!(
                    selection(tester, controller.handle()).extent_offset,
                    8,
                    "{platform:?}"
                );

                let mut touch_gesture = TestPointer::touch(2);
                tester.down(&mut touch_gesture, e_pos);
                tester.up(&touch_gesture);
                tester.pump_and_settle_by(waft_gestures::K_DOUBLE_TAP_TIMEOUT);
                // On iOS a tap to select, selects the word edge instead of the exact tap position.
                let expected = if is_target_platform_apple { 7 } else { 5 };
                assert_eq!(
                    selection(tester, controller.handle()).base_offset,
                    expected,
                    "{platform:?}"
                );
                assert_eq!(
                    selection(tester, controller.handle()).extent_offset,
                    expected,
                    "{platform:?}"
                );

                // Selection should stay the same since it is set on tap up for mobile platforms.
                touch_gesture.position = g_pos;
                tester.send(&touch_gesture, waft_embedder::PointerChange::Down);
                tester.pump();
                assert_eq!(
                    selection(tester, controller.handle()).base_offset,
                    expected,
                    "{platform:?}"
                );
                assert_eq!(
                    selection(tester, controller.handle()).extent_offset,
                    expected,
                    "{platform:?}"
                );

                tester.up(&touch_gesture);
                tester.pump_and_settle();
                assert_eq!(
                    selection(tester, controller.handle()).base_offset,
                    8,
                    "{platform:?}"
                );
                assert_eq!(
                    selection(tester, controller.handle()).extent_offset,
                    8,
                    "{platform:?}"
                );
            });
        }
    }

    #[test]
    fn an_obscured_text_field_is_selectable_by_default() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            // Obscure text and don't enable or disable selection.
            let root = overlay(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .obscure_text(true),
            );
            tester.pump_widget(root);
            tester.enter_text(find::by_type::<EditableText>().at(0), "abcdefghi");
            skip_past_scrolling_animation(tester);
            assert!(selection(tester, controller.handle()).is_collapsed());

            // Long press does select text.
            let e_pos = text_offset_to_position(tester, 1);
            tester.long_press_at(e_pos, 7);
            tester.pump_and_settle();
            assert!(!selection(tester, controller.handle()).is_collapsed());
        });
    }

    #[test]
    fn an_obscured_text_field_is_not_selectable_when_disabled() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            // Explicitly disabled selection on obscured text.
            let root = overlay(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .obscure_text(true)
                    .enable_interactive_selection(false),
            );
            tester.pump_widget(root);
            tester.enter_text(find::by_type::<EditableText>().at(0), "abcdefghi");
            skip_past_scrolling_animation(tester);
            assert!(selection(tester, controller.handle()).is_collapsed());

            // Long press doesn't select text.
            let e_pos2 = text_offset_to_position(tester, 1);
            tester.long_press_at(e_pos2, 7);
            tester.pump_and_settle();
            assert!(selection(tester, controller.handle()).is_collapsed());
        });
    }

    #[test]
    fn an_obscured_text_field_is_not_selectable_when_read_only() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            // Explicitly disabled selection on obscured text that is read-only.
            let root = overlay(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .obscure_text(true)
                    .read_only(true),
            );
            tester.pump_widget(root);
            tester.enter_text(find::by_type::<EditableText>().at(0), "abcdefghi");
            skip_past_scrolling_animation(tester);
            assert!(selection(tester, controller.handle()).is_collapsed());

            // Long press doesn't select text.
            let e_pos2 = text_offset_to_position(tester, 1);
            tester.long_press_at(e_pos2, 7);
            tester.pump_and_settle();
            assert!(selection(tester, controller.handle()).is_collapsed());
        });
    }

    #[test]
    fn an_obscured_text_field_is_selected_as_one_word() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = overlay(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .obscure_text(true),
            );
            tester.pump_widget(root);
            tester.enter_text(find::by_type::<EditableText>().at(0), "abcde fghi");
            skip_past_scrolling_animation(tester);

            // Long press does select text.
            let b_pos = text_offset_to_position(tester, 1);
            tester.long_press_at(b_pos, 7);
            tester.pump_and_settle();
            let selection = selection(tester, controller.handle());
            assert!(!selection.is_collapsed());
            assert_eq!(selection.base_offset, 0);
            assert_eq!(selection.extent_offset, 10);
        });
    }

    #[test]
    fn text_field_errortext_trumps_helper_text() {
        test_widgets(|tester| {
            let root = overlay(
                tester,
                TextField::new().decoration(Some(
                    InputDecoration::new()
                        .error_text("error text")
                        .helper_text("helper text"),
                )),
            );
            tester.pump_widget(root);
            expect(tester, &find::text("helper text"), finds_nothing());
            expect(tester, &find::text("error text"), finds_one_widget());
        });
    }

    #[test]
    fn cannot_enter_new_lines_onto_single_line_text_field() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = boilerplate(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .decoration(None),
            );
            tester.pump_widget(root);
            tester.enter_text(find::by_type::<EditableText>().at(0), "abc\ndef");
            assert_eq!(text(tester, controller.handle()), "abcdef");
        });
    }

    #[test]
    fn injected_formatters_are_chained() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = boilerplate(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .decoration(None)
                    .input_formatters(vec![formatter(DenyLowercase("#"))]),
            );
            tester.pump_widget(root);
            tester.enter_text(
                find::by_type::<EditableText>().at(0),
                "a一b二c三\nd四e五f六",
            );
            // The default single line formatter replaces \n with empty string.
            assert_eq!(text(tester, controller.handle()), "#一#二#三#四#五#六");
        });
    }

    #[test]
    fn chained_formatters_are_in_sequence() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = boilerplate(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .decoration(None)
                    .max_lines(Some(2))
                    .input_formatters(vec![
                        formatter(DenyLowercase("12\n")),
                        formatter(AllowNewlineDigit),
                    ]),
            );
            tester.pump_widget(root);
            tester.enter_text(find::by_type::<EditableText>().at(0), "a1b2c3");
            // The first formatter turns it into
            // 12\n112\n212\n3
            // The second formatter turns it into
            // \n1\n2\n3
            // Multiline is allowed since maxLine != 1.
            assert_eq!(text(tester, controller.handle()), "\n1\n2\n3");
        });
    }

    #[test]
    fn do_not_add_length_limiting_formatter_to_the_user_supplied_list() {
        test_widgets(|tester| {
            let formatters: Vec<TextInputFormatterRef> = Vec::new();
            let field = TextField::new()
                .decoration(None)
                .max_length(5)
                .input_formatters(formatters.clone());
            let root = overlay(tester, field);
            tester.pump_widget(root);
            assert!(formatters.is_empty());
            let element = find::by_type::<TextField>().evaluate(tester)[0];
            let given = tester.widget::<TextField, _>(element, |field| {
                field.input_formatters.as_ref().map(Vec::len)
            });
            assert_eq!(given, Some(0));
        });
    }

    #[test]
    fn text_field_drops_selection_color_when_losing_focus() {
        test_widgets(|tester| {
            let controller1 = controller_with(tester, "");
            let selection_color = Colors::ORANGE.primary();
            let cursor_color = Colors::RED.primary();
            let root = overlay(
                tester,
                DefaultSelectionStyle::new(Column::new().children(vec![
                    TextField::new()
                        .controller(controller1.erase())
                        .into_widget(),
                    TextField::new().into_widget(),
                ]))
                .selection_color(selection_color)
                .cursor_color(cursor_color),
            );
            tester.pump_widget(root);

            let expected = TextSelection::new(0, 4);

            let first = text_field_center(tester, 0);
            tester.tap_at(first, 1);
            tester.enter_text(find::by_type::<EditableText>().at(0), "abcd");
            tester.pump();

            let second = text_field_center(tester, 1);
            tester.tap_at(second, 2);
            tester.enter_text(find::by_type::<EditableText>().at(1), "dcba");
            tester.pump();

            // Focus and selection is active on first TextField, so the second TextFields
            // selectionColor should be dropped.
            tester.tap_at(first, 3);
            controller1.set_selection(&mut tester.cell.borrow_mut(), TextSelection::new(0, 4));
            tester.pump();
            assert_eq!(selection(tester, controller1.handle()), expected);
            assert_eq!(
                editable_at(tester, 0, |widget| widget.selection_color),
                Some(selection_color)
            );
            assert_eq!(
                editable_at(tester, 1, |widget| widget.selection_color),
                None
            );

            // Focus and selection is active on second TextField, so the first TextFields
            // selectionColor should be dropped.
            tester.tap_at(second, 4);
            tester.pump();
            assert_eq!(
                editable_at(tester, 0, |widget| widget.selection_color),
                None
            );
            assert_eq!(
                editable_at(tester, 1, |widget| widget.selection_color),
                Some(selection_color)
            );
        });
    }

    #[test]
    fn max_length_limits_input() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = boilerplate(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .max_length(10),
            );
            tester.pump_widget(root);
            tester.enter_text(find::by_type::<EditableText>().at(0), "0123456789101112");
            assert_eq!(text(tester, controller.handle()), "0123456789");
        });
    }

    #[test]
    fn max_length_limits_input_with_surrogate_pairs() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = boilerplate(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .max_length(10),
            );
            tester.pump_widget(root);
            let surrogate_pair = "😆";
            tester.enter_text(
                find::by_type::<EditableText>().at(0),
                &format!("{surrogate_pair}0123456789101112"),
            );
            assert_eq!(
                text(tester, controller.handle()),
                format!("{surrogate_pair}012345678")
            );
        });
    }

    #[test]
    fn max_length_limits_input_with_grapheme_clusters() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = boilerplate(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .max_length(10),
            );
            tester.pump_widget(root);
            let grapheme_cluster = "👨\u{200d}👩\u{200d}👦";
            tester.enter_text(
                find::by_type::<EditableText>().at(0),
                &format!("{grapheme_cluster}0123456789101112"),
            );
            assert_eq!(
                text(tester, controller.handle()),
                format!("{grapheme_cluster}012345678")
            );
        });
    }

    #[test]
    fn max_length_limits_input_in_the_center_of_a_maxed_out_field() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let test_value = "0123456789";
            let root = boilerplate(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .max_length(10),
            );
            tester.pump_widget(root);

            // Max out the character limit in the field.
            tester.enter_text(find::by_type::<EditableText>().at(0), test_value);
            assert_eq!(text(tester, controller.handle()), test_value);

            // Entering more characters at the end does nothing.
            tester.enter_text(
                find::by_type::<EditableText>().at(0),
                &format!("{test_value}9999999"),
            );
            assert_eq!(text(tester, controller.handle()), test_value);

            // Entering text in the middle of the field also does nothing.
            tester.enter_text(find::by_type::<EditableText>().at(0), "0123455555555556789");
            assert_eq!(text(tester, controller.handle()), test_value);
        });
    }

    #[test]
    fn max_length_limits_input_length_even_if_decoration_is_null() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = boilerplate(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .decoration(None)
                    .max_length(10),
            );
            tester.pump_widget(root);
            tester.enter_text(find::by_type::<EditableText>().at(0), "0123456789101112");
            assert_eq!(text(tester, controller.handle()), "0123456789");
        });
    }

    #[test]
    fn max_length_still_works_with_other_formatters() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = boilerplate(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .max_length(10)
                    .input_formatters(vec![formatter(DenyLowercase("#"))]),
            );
            tester.pump_widget(root);
            tester.enter_text(
                find::by_type::<EditableText>().at(0),
                "a一b二c三\nd四e五f六",
            );
            // The default single line formatter replaces \n with empty string.
            assert_eq!(text(tester, controller.handle()), "#一#二#三#四#五");
        });
    }

    #[test]
    fn max_length_isnt_enforced_when_max_length_enforcement_none() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = boilerplate(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .max_length(10)
                    .max_length_enforcement(MaxLengthEnforcement::None),
            );
            tester.pump_widget(root);
            tester.enter_text(find::by_type::<EditableText>().at(0), "0123456789101112");
            assert_eq!(text(tester, controller.handle()), "0123456789101112");
        });
    }

    #[test]
    fn max_length_shows_warning_when_max_length_enforcement_none() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let deep_purple_accent = Colors::DEEP_PURPLE_ACCENT.primary();
            let root = boilerplate(
                tester,
                TextField::new()
                    .decoration(Some(
                        InputDecoration::new()
                            .error_style(TextStyle::new().color(deep_purple_accent)),
                    ))
                    .controller(controller.erase())
                    .max_length(10)
                    .max_length_enforcement(MaxLengthEnforcement::None),
            );
            tester.pump_widget(root);

            tester.enter_text(find::by_type::<EditableText>().at(0), "0123456789101112");
            tester.pump();

            assert_eq!(text(tester, controller.handle()), "0123456789101112");
            expect(tester, &find::text("16/10"), finds_one_widget());
            assert_eq!(text_color(tester, "16/10"), Some(deep_purple_accent));

            tester.enter_text(find::by_type::<EditableText>().at(0), "0123456789");
            tester.pump();

            assert_eq!(text(tester, controller.handle()), "0123456789");
            expect(tester, &find::text("10/10"), finds_one_widget());
            assert_ne!(text_color(tester, "10/10"), Some(deep_purple_accent));
        });
    }

    /// Dart's 'maxLength shows warning when maxLengthEnforcement.none with surrogate pairs.'
    /// and its grapheme-cluster twin.
    fn check_max_length_warning_with(tester: &mut WidgetTester, prefix: &str) {
        let controller = controller_with(tester, "");
        let deep_purple_accent = Colors::DEEP_PURPLE_ACCENT.primary();
        let root = boilerplate(
            tester,
            TextField::new()
                .decoration(Some(
                    InputDecoration::new().error_style(TextStyle::new().color(deep_purple_accent)),
                ))
                .controller(controller.erase())
                .max_length(10)
                .max_length_enforcement(MaxLengthEnforcement::None),
        );
        tester.pump_widget(root);

        let long = format!("{prefix}012345678910111");
        tester.enter_text(find::by_type::<EditableText>().at(0), &long);
        tester.pump();

        assert_eq!(text(tester, controller.handle()), long);
        expect(tester, &find::text("16/10"), finds_one_widget());
        assert_eq!(text_color(tester, "16/10"), Some(deep_purple_accent));

        let full = format!("{prefix}012345678");
        tester.enter_text(find::by_type::<EditableText>().at(0), &full);
        tester.pump();

        assert_eq!(text(tester, controller.handle()), full);
        expect(tester, &find::text("10/10"), finds_one_widget());
        assert_ne!(text_color(tester, "10/10"), Some(deep_purple_accent));
    }

    #[test]
    fn max_length_shows_warning_when_max_length_enforcement_none_with_surrogate_pairs() {
        test_widgets(|tester| check_max_length_warning_with(tester, "😆"));
    }

    #[test]
    fn max_length_shows_warning_when_max_length_enforcement_none_with_grapheme_clusters() {
        test_widgets(|tester| check_max_length_warning_with(tester, "👨\u{200d}👩\u{200d}👦"));
    }

    #[test]
    fn max_length_shows_warning_in_material_3() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let deep_purple_accent = Colors::DEEP_PURPLE_ACCENT.primary();
            let theme = ThemeData::from(ColorScheme::light().with_error(deep_purple_accent))
                .build(&tester.cell.borrow());
            let root = material_app(
                theme,
                Center::new()
                    .child(
                        Material::new().child(
                            TextField::new()
                                .controller(controller.erase())
                                .max_length(10)
                                .max_length_enforcement(MaxLengthEnforcement::None),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            tester.enter_text(find::by_type::<EditableText>().at(0), "0123456789101112");
            tester.pump();

            assert_eq!(text(tester, controller.handle()), "0123456789101112");
            expect(tester, &find::text("16/10"), finds_one_widget());
            assert_eq!(text_color(tester, "16/10"), Some(deep_purple_accent));

            tester.enter_text(find::by_type::<EditableText>().at(0), "0123456789");
            tester.pump();

            assert_eq!(text(tester, controller.handle()), "0123456789");
            expect(tester, &find::text("10/10"), finds_one_widget());
            assert_ne!(text_color(tester, "10/10"), Some(deep_purple_accent));
        });
    }

    #[test]
    fn setting_max_length_shows_counter() {
        test_widgets(|tester| {
            let root = material_home(
                theme(tester),
                Center::new().child(TextField::new().max_length(10)),
            );
            tester.pump_widget(root);

            expect(tester, &find::text("0/10"), finds_one_widget());

            tester.enter_text(find::by_type::<EditableText>().at(0), "01234");
            tester.pump();

            expect(tester, &find::text("5/10"), finds_one_widget());
        });
    }

    #[test]
    fn max_length_counter_measures_surrogate_pairs_as_one_character() {
        test_widgets(|tester| {
            let root = material_home(
                theme(tester),
                Center::new().child(TextField::new().max_length(10)),
            );
            tester.pump_widget(root);

            expect(tester, &find::text("0/10"), finds_one_widget());

            tester.enter_text(find::by_type::<EditableText>().at(0), "😆");
            tester.pump();

            expect(tester, &find::text("1/10"), finds_one_widget());
        });
    }

    #[test]
    fn max_length_counter_measures_grapheme_clusters_as_one_character() {
        test_widgets(|tester| {
            let root = material_home(
                theme(tester),
                Center::new().child(TextField::new().max_length(10)),
            );
            tester.pump_widget(root);

            expect(tester, &find::text("0/10"), finds_one_widget());

            tester.enter_text(
                find::by_type::<EditableText>().at(0),
                "👨\u{200d}👩\u{200d}👦",
            );
            tester.pump();

            expect(tester, &find::text("1/10"), finds_one_widget());
        });
    }

    #[test]
    fn setting_max_length_to_text_field_no_max_length_shows_only_entered_length() {
        test_widgets(|tester| {
            let root = material_home(
                theme(tester),
                Center::new().child(TextField::new().max_length(TextField::NO_MAX_LENGTH)),
            );
            tester.pump_widget(root);

            expect(tester, &find::text("0"), finds_one_widget());

            tester.enter_text(find::by_type::<EditableText>().at(0), "01234");
            tester.pump();

            expect(tester, &find::text("5"), finds_one_widget());
        });
    }

    #[test]
    fn passing_a_build_counter_shows_returned_widget() {
        test_widgets(|tester| {
            let root = material_home(
                theme(tester),
                Center::new().child(
                    TextField::new()
                        .build_counter(|_app, _context, current_length, max_length, _is_focused| {
                            let max_length =
                                max_length.map_or_else(|| "null".to_owned(), |max| max.to_string());
                            Some(
                                Text::new(format!("{current_length} of {max_length}"))
                                    .into_widget(),
                            )
                        })
                        .max_length(10),
                ),
            );
            tester.pump_widget(root);

            expect(tester, &find::text("0 of 10"), finds_one_widget());

            tester.enter_text(find::by_type::<EditableText>().at(0), "01234");
            tester.pump();

            expect(tester, &find::text("5 of 10"), finds_one_widget());
        });
    }

    #[test]
    fn disabled_text_field_hides_helper_and_counter() {
        test_widgets(|tester| {
            let helper_text = "helper text";
            let counter_text = "counter text";
            let error_text = "error text";
            let build_frame = |tester: &WidgetTester, enabled: bool, has_error: bool| {
                let mut decoration = InputDecoration::new()
                    .label_text("label text")
                    .helper_text(helper_text)
                    .counter_text(counter_text)
                    .enabled(enabled);
                if has_error {
                    decoration = decoration.error_text(error_text);
                }
                material_home(
                    m2_theme(tester),
                    Center::new().child(TextField::new().decoration(Some(decoration))),
                )
            };
            let transparent = Some(Colors::TRANSPARENT);

            let root = build_frame(tester, true, false);
            tester.pump_widget(root);
            assert_ne!(text_color(tester, helper_text), transparent);
            assert_ne!(text_color(tester, counter_text), transparent);
            let root = build_frame(tester, true, true);
            tester.pump_widget(root);
            assert_ne!(text_color(tester, error_text), transparent);

            // When enabled is false, the helper/error and counter are not visible.
            let root = build_frame(tester, false, false);
            tester.pump_widget(root);
            assert_eq!(text_color(tester, helper_text), transparent);
            assert_eq!(text_color(tester, counter_text), transparent);
            let root = build_frame(tester, false, true);
            tester.pump_widget(root);
            assert_eq!(text_color(tester, counter_text), transparent);
            assert_eq!(text_color(tester, error_text), transparent);
        });
    }

    #[test]
    fn disabled_text_field_has_default_m2_disabled_text_style_for_the_input_text() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "Atwater Peel Sherbrooke Bonaventure");
            let root = material_home(
                m2_theme(tester),
                Center::new().child(
                    TextField::new()
                        .controller(controller.erase())
                        .enabled(false),
                ),
            );
            tester.pump_widget(root);
            let color = editable(tester, |widget| {
                widget.style.color.as_ref().map(|color| color.color())
            });
            // Colors.black38 is the default disabled color for ThemeData.light().
            assert_eq!(color, Some(Colors::BLACK38));
        });
    }

    #[test]
    fn disabled_text_field_has_default_m3_disabled_text_style_for_the_input_text() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "Atwater Peel Sherbrooke Bonaventure");
            let theme = theme(tester);
            let expected = theme
                .text_theme
                .body_large
                .as_ref()
                .and_then(|style| style.color.as_ref())
                .map(|color| color.color().with_opacity(0.38));
            let root = material_home(
                theme,
                Center::new().child(
                    TextField::new()
                        .controller(controller.erase())
                        .enabled(false),
                ),
            );
            tester.pump_widget(root);
            let color = editable(tester, |widget| {
                widget.style.color.as_ref().map(|color| color.color())
            });
            assert_eq!(color, expected);
        });
    }

    fn states(tester: &WidgetTester, controller: Handle<MaterialStatesController>) -> WidgetStates {
        tester.cell.borrow().get(controller).value().clone()
    }

    fn state_set(states: &[WidgetState]) -> WidgetStates {
        let mut set = WidgetStates::new();
        for state in states {
            set.insert(*state);
        }
        set
    }

    #[test]
    fn enabled_text_field_states_controller() {
        test_widgets(|tester| {
            let text_editing_controller =
                controller_with(tester, "Atwater Peel Sherbrooke Bonaventure");
            let count = Rc::new(Cell::new(0));
            let states_controller = {
                let mut app = tester.cell.borrow_mut();
                let states_controller = MaterialStatesController::new(&mut app, None);
                let count = Rc::clone(&count);
                states_controller.add_listener(
                    &mut app,
                    Listener::new(move |_app| count.set(count.get() + 1)),
                );
                states_controller
            };
            let build = |tester: &WidgetTester, field: TextField| {
                material_home(
                    theme(tester),
                    Center::new().child(
                        field
                            .states_controller(states_controller.clone())
                            .controller(text_editing_controller.erase()),
                    ),
                )
            };
            let root = build(tester, TextField::new());
            tester.pump_widget(root);

            let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.add_pointer(&mut gesture, Offset::new(1.0, 1.0));
            let editable_element = editable_elements(tester)[0];
            let center = tester.get_rect(editable_element).center();
            tester.move_to(&mut gesture, center, false);
            tester.pump();
            assert_eq!(
                states(tester, states_controller.handle()),
                state_set(&[WidgetState::Hovered])
            );
            assert_eq!(count.get(), 1);

            tester.move_to(&mut gesture, Offset::ZERO, false);
            tester.pump();
            assert_eq!(states(tester, states_controller.handle()), state_set(&[]));
            assert_eq!(count.get(), 2);

            tester.move_to(&mut gesture, center, false);
            tester.send(&gesture, waft_embedder::PointerChange::Down);
            tester.pump();
            tester.up(&gesture);
            tester.pump();
            assert_eq!(
                states(tester, states_controller.handle()),
                state_set(&[WidgetState::Hovered, WidgetState::Focused])
            );
            assert_eq!(count.get(), 4); // adds hovered and pressed - two changes.

            tester.move_to(&mut gesture, Offset::ZERO, false);
            tester.pump();
            assert_eq!(
                states(tester, states_controller.handle()),
                state_set(&[WidgetState::Focused])
            );
            assert_eq!(count.get(), 5);

            tester.send(&gesture, waft_embedder::PointerChange::Down);
            tester.pump();
            assert_eq!(states(tester, states_controller.handle()), state_set(&[]));
            assert_eq!(count.get(), 6);
            tester.up(&gesture);
            tester.pump();

            tester.move_to(&mut gesture, center, false);
            tester.send(&gesture, waft_embedder::PointerChange::Down);
            tester.pump();
            tester.up(&gesture);
            tester.pump();
            assert_eq!(
                states(tester, states_controller.handle()),
                state_set(&[WidgetState::Hovered, WidgetState::Focused])
            );
            assert_eq!(count.get(), 8); // adds hovered and pressed - two changes.

            // If the text field is rebuilt disabled, then the focused state is
            // removed.
            let root = build(tester, TextField::new().enabled(false));
            tester.pump_widget(root);
            tester.pump_and_settle();
            assert_eq!(
                states(tester, states_controller.handle()),
                state_set(&[WidgetState::Hovered, WidgetState::Disabled])
            );
            assert_eq!(count.get(), 10); // removes focused and adds disabled - two changes.

            tester.move_to(&mut gesture, Offset::ZERO, false);
            tester.pump();
            assert_eq!(
                states(tester, states_controller.handle()),
                state_set(&[WidgetState::Disabled])
            );
            assert_eq!(count.get(), 11);

            // If the text field is rebuilt enabled and in an error state, then the error
            // state is added.
            let root = build(
                tester,
                TextField::new().decoration(Some(InputDecoration::new().error_text("error"))),
            );
            tester.pump_widget(root);
            tester.pump_and_settle();
            assert_eq!(
                states(tester, states_controller.handle()),
                state_set(&[WidgetState::Error])
            );
            assert_eq!(count.get(), 13); // removes disabled and adds error - two changes.

            // If the text field is rebuilt without an error, then the error
            // state is removed.
            let root = build(tester, TextField::new());
            tester.pump_widget(root);
            tester.pump_and_settle();
            assert_eq!(states(tester, states_controller.handle()), state_set(&[]));
            assert_eq!(count.get(), 14);
        });
    }

    #[test]
    fn disabled_text_field_states_controller() {
        test_widgets(|tester| {
            let count = Rc::new(Cell::new(0));
            let controller = {
                let mut app = tester.cell.borrow_mut();
                let controller = MaterialStatesController::new(&mut app, None);
                let count = Rc::clone(&count);
                controller.add_listener(
                    &mut app,
                    Listener::new(move |_app| count.set(count.get() + 1)),
                );
                controller
            };
            let root = material_home(
                theme(tester),
                Center::new().child(
                    TextField::new()
                        .states_controller(controller.clone())
                        .enabled(false),
                ),
            );
            tester.pump_widget(root);
            assert_eq!(
                states(tester, controller.handle()),
                state_set(&[WidgetState::Disabled])
            );
            assert_eq!(count.get(), 1);
        });
    }

    #[test]
    fn provided_style_correctly_resolves_for_material_states() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "Atwater Peel Sherbrooke Bonaventure");
            let red = Colors::RED.primary();
            let blue = Colors::BLUE.primary();
            let build_frame = |tester: &WidgetTester, enabled: bool| {
                material_home(
                    theme(tester),
                    Center::new().child(
                        TextField::new()
                            .controller(controller.erase())
                            .enabled(enabled)
                            .style(WidgetStateTextStyle::resolve_with(
                                move |states: &WidgetStates| {
                                    if states.contains(&WidgetState::Disabled) {
                                        return TextStyle::new().color(red);
                                    }
                                    TextStyle::new().color(blue)
                                },
                            )),
                    ),
                )
            };
            let style_color = |tester: &WidgetTester| {
                editable(tester, |widget| {
                    widget.style.color.as_ref().map(|color| color.color())
                })
            };

            let root = build_frame(tester, false);
            tester.pump_widget(root);
            assert_eq!(style_color(tester), Some(red));
            let root = build_frame(tester, true);
            tester.pump_widget(root);
            assert_eq!(style_color(tester), Some(blue));
        });
    }

    #[test]
    fn disabled_text_field_cant_be_traversed_to() {
        test_widgets(|tester| {
            let (focus_node1, focus_node2) = {
                let mut app = tester.cell.borrow_mut();
                (FocusNode::new(&mut app), FocusNode::new(&mut app))
            };
            let root = material_home(
                theme(tester),
                FocusScope::new(Center::new().child(Column::new().children(vec![
                    TextField::new()
                        .focus_node(focus_node1.erase())
                        .autofocus(true)
                        .max_length(10)
                        .enabled(true)
                        .into_widget(),
                    TextField::new()
                        .focus_node(focus_node2.erase())
                        .max_length(10)
                        .enabled(false)
                        .into_widget(),
                ]))),
            );
            tester.pump_widget(root);

            tester.pump();
            assert!(focus_node1.has_primary_focus(&tester.cell.borrow()));
            assert!(!focus_node2.has_primary_focus(&tester.cell.borrow()));

            assert!(
                !focus_node1
                    .as_node()
                    .next_focus(&mut tester.cell.borrow_mut())
            );
            tester.pump();

            assert!(focus_node1.has_primary_focus(&tester.cell.borrow()));
            assert!(!focus_node2.has_primary_focus(&tester.cell.borrow()));
        });
    }

    #[test]
    fn text_field_loses_focus_when_disabled() {
        test_widgets(|tester| {
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut());

            let root = boilerplate(
                tester,
                TextField::new()
                    .focus_node(focus_node.erase())
                    .autofocus(true)
                    .enabled(true),
            );
            tester.pump_widget(root);
            assert!(has_focus(tester, focus_node.handle()));

            let root = boilerplate(
                tester,
                TextField::new()
                    .focus_node(focus_node.erase())
                    .autofocus(true)
                    .enabled(false),
            );
            tester.pump_widget(root);
            assert!(!has_focus(tester, focus_node.handle()));

            let directional = |enabled: bool| {
                Builder::new({
                    let focus_node = focus_node.clone();
                    move |app, context| {
                        let data: MediaQueryData = MediaQuery::of(app, context)
                            .copy_with()
                            .navigation_mode(NavigationMode::Directional);
                        MediaQuery::new(
                            data,
                            TextField::new()
                                .focus_node(focus_node.clone().erase())
                                .autofocus(true)
                                .enabled(enabled),
                        )
                        .into_widget()
                    }
                })
            };
            let root = boilerplate(tester, directional(true));
            tester.pump_widget(root);
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump();

            assert!(has_focus(tester, focus_node.handle()));

            let root = boilerplate(tester, directional(false));
            tester.pump_widget(root);
            tester.pump();

            assert!(has_focus(tester, focus_node.handle()));
        });
    }

    #[test]
    fn on_tap_is_called_upon_tap() {
        test_widgets(|tester| {
            let tap_count = Rc::new(Cell::new(0));
            let counter = Rc::clone(&tap_count);
            let root = overlay(
                tester,
                TextField::new().on_tap(Listener::new(move |_app| counter.set(counter.get() + 1))),
            );
            tester.pump_widget(root);

            assert_eq!(tap_count.get(), 0);
            let center = text_field_center(tester, 0);
            tester.tap_at(center, 1);
            // Wait a bit so they're all single taps and not double taps.
            tester.pump_by(Duration::from_millis(300));
            tester.tap_at(center, 2);
            tester.pump_by(Duration::from_millis(300));
            tester.tap_at(center, 3);
            tester.pump_by(Duration::from_millis(300));
            assert_eq!(tap_count.get(), 3);
        });
    }

    #[test]
    fn on_tap_is_not_called_field_is_disabled() {
        test_widgets(|tester| {
            let tap_count = Rc::new(Cell::new(0));
            let counter = Rc::clone(&tap_count);
            let root = overlay(
                tester,
                TextField::new()
                    .enabled(false)
                    .on_tap(Listener::new(move |_app| counter.set(counter.get() + 1))),
            );
            tester.pump_widget(root);

            assert_eq!(tap_count.get(), 0);
            let center = text_field_center(tester, 0);
            tester.tap_at(center, 1);
            tester.tap_at(center, 2);
            tester.tap_at(center, 3);
            assert_eq!(tap_count.get(), 0);
        });
    }

    fn style_is_merged_with_theme(tester: &mut WidgetTester, use_material3: bool) {
        let blue = Colors::BLUE.shade500();
        let text_theme = if use_material3 {
            TextTheme::new().body_large(TextStyle::new().color(blue))
        } else {
            TextTheme::new().title_medium(TextStyle::new().color(blue))
        };
        let theme_data = ThemeData::builder()
            .use_material3(use_material3)
            .text_theme(text_theme)
            .build(&tester.cell.borrow());
        let theme_style = if use_material3 {
            theme_data.text_theme.body_large.clone()
        } else {
            theme_data.text_theme.title_medium.clone()
        }
        .expect("the theme has the style");
        let build_frame = |theme_data: &ThemeData, style: TextStyle| {
            material_home(
                theme_data.clone(),
                Center::new().child(TextField::new().style(style)),
            )
        };

        // Empty TextStyle is overridden by theme
        tester.pump_widget(build_frame(&theme_data, TextStyle::new()));
        let style = editable(tester, |widget| widget.style.clone());
        assert_eq!(style.color, theme_style.color);
        assert_eq!(style.background, theme_style.background);
        assert_eq!(style.shadows, theme_style.shadows);
        assert_eq!(style.decoration, theme_style.decoration);
        assert_eq!(style.word_spacing, theme_style.word_spacing);

        // Properties set on TextStyle override theme
        let set_color = Colors::RED.primary();
        tester.pump_widget(build_frame(&theme_data, TextStyle::new().color(set_color)));
        let style = editable(tester, |widget| widget.style.clone());
        assert_eq!(style.color.map(|color| color.color()), Some(set_color));

        // inherit: false causes nothing to be merged in from theme
        tester.pump_widget(build_frame(
            &theme_data,
            TextStyle::new()
                .font_size(24.0)
                .text_baseline(waft_embedder::TextBaseline::Alphabetic)
                .inherit(false),
        ));
        let style = editable(tester, |widget| widget.style.clone());
        assert!(style.color.is_none());
    }

    #[test]
    fn text_field_style_is_merged_with_theme() {
        test_widgets(|tester| style_is_merged_with_theme(tester, false));
    }

    #[test]
    fn text_field_style_is_merged_with_theme_in_material_3() {
        test_widgets(|tester| style_is_merged_with_theme(tester, true));
    }

    fn style_frame(tester: &WidgetTester, style: TextStyle) -> WidgetRef {
        material_home(theme(tester), TextField::new().style(style))
    }

    #[test]
    fn style_enforces_required_fields() {
        test_widgets(|tester| {
            let root = style_frame(
                tester,
                TextStyle::new()
                    .inherit(false)
                    .font_size(12.0)
                    .text_baseline(waft_embedder::TextBaseline::Alphabetic),
            );
            tester.pump_widget(root);

            // With inherit not set to false, will pickup required fields from theme
            let root = style_frame(tester, TextStyle::new().font_size(12.0));
            tester.pump_widget(root);
        });
    }

    #[test]
    #[should_panic(expected = "inherit false style must supply fontSize and textBaseline")]
    fn style_enforces_required_fields_an_inherit_false_style_without_a_baseline_throws() {
        test_widgets(|tester| {
            let root = style_frame(tester, TextStyle::new().inherit(false).font_size(12.0));
            tester.pump_widget(root);
        });
    }

    #[test]
    fn selection_controls_is_passed_to_editable_text() {
        test_widgets(|tester| {
            let controls = material_text_selection_controls();
            let root = material_home(
                theme(tester),
                Scaffold::new().body(TextField::new().selection_controls(Rc::clone(&controls))),
            );
            tester.pump_widget(root);
            let passed = editable(tester, |widget| widget.selection_controls.clone())
                .expect("the field passes its controls");
            assert!(Rc::ptr_eq(&passed, &controls));
        });
    }

    fn selection_overlay_visibility(tester: &WidgetTester) -> (bool, bool) {
        let state = editable_state(tester);
        let app = tester.cell.borrow();
        let overlay = state
            .selection_overlay(&app)
            .expect("the field has a selection overlay");
        (
            overlay.handles_are_visible(&app),
            overlay.toolbar_is_visible(&app),
        )
    }

    #[test]
    fn tap_shows_handles_but_not_toolbar() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "abc def ghi");
            let root = material_home(
                theme(tester),
                TextField::new().controller(controller.erase()),
            );
            tester.pump_widget(root);

            // Tap to trigger the text field.
            let center = text_field_center(tester, 0);
            tester.tap_at(center, 1);
            tester.pump();

            assert_eq!(selection_overlay_visibility(tester), (true, false));
        });
    }

    #[test]
    fn tap_in_empty_text_field_does_not_show_handles_nor_toolbar() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = material_home(
                theme(tester),
                TextField::new().controller(controller.erase()),
            );
            tester.pump_widget(root);

            // Tap to trigger the text field.
            let center = text_field_center(tester, 0);
            tester.tap_at(center, 1);
            tester.pump();

            assert_eq!(selection_overlay_visibility(tester), (false, false));
        });
    }

    #[test]
    fn mouse_tap_does_not_show_handles_nor_toolbar() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "abc def ghi");
            let root = material_home(
                theme(tester),
                TextField::new().controller(controller.erase()),
            );
            tester.pump_widget(root);

            // Long press to trigger the text field.
            let text_field_pos = text_field_center(tester, 0);
            let mut gesture = TestPointer::mouse(7, waft_gestures::K_PRIMARY_BUTTON);
            tester.down(&mut gesture, text_field_pos);
            tester.pump();
            tester.up(&gesture);
            tester.pump();

            assert_eq!(selection_overlay_visibility(tester), (false, false));
        });
    }

    #[test]
    fn long_press_shows_handles_and_toolbar() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "abc def ghi");
            let root = material_home(
                theme(tester),
                TextField::new().controller(controller.erase()),
            );
            tester.pump_widget(root);

            // Long press to trigger the text field.
            let center = text_field_center(tester, 0);
            tester.long_press_at(center, 1);
            tester.pump();

            assert_eq!(selection_overlay_visibility(tester), (true, true));
        });
    }

    #[test]
    fn long_press_in_empty_text_field_shows_handles_and_toolbar() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = material_home(
                theme(tester),
                TextField::new().controller(controller.erase()),
            );
            tester.pump_widget(root);

            // Tap to trigger the text field.
            let center = text_field_center(tester, 0);
            tester.long_press_at(center, 1);
            tester.pump();

            assert_eq!(selection_overlay_visibility(tester), (true, true));
        });
    }

    /// Dart's 'Double tap shows handles and toolbar' and 'Double tap in empty text field shows
    /// toolbar but not handles': a double tap on a field holding `text`.
    fn double_tap_visibility(tester: &mut WidgetTester, text: &str) -> (bool, bool) {
        let controller = controller_with(tester, text);
        let root = material_home(
            theme(tester),
            TextField::new().controller(controller.erase()),
        );
        tester.pump_widget(root);

        // Double tap to trigger the text field.
        let center = text_field_center(tester, 0);
        tester.tap_at(center, 1);
        tester.pump_by(Duration::from_millis(50));
        tester.tap_at(center, 2);
        tester.pump();

        selection_overlay_visibility(tester)
    }

    #[test]
    fn double_tap_shows_handles_and_toolbar() {
        test_widgets(|tester| {
            assert_eq!(double_tap_visibility(tester, "abc def ghi"), (true, true));
        });
    }

    #[test]
    fn double_tap_in_empty_text_field_shows_toolbar_but_not_handles() {
        test_widgets(|tester| {
            assert_eq!(double_tap_visibility(tester, ""), (false, true));
        });
    }

    /// The global caret points of the controller's selection: Dart's
    /// `globalize(renderEditable.getEndpointsForSelection(selection), renderEditable)`.
    fn global_endpoints(tester: &WidgetTester, selection: TextSelection) -> Vec<Offset> {
        let editable = tester.all::<RenderEditable>()[0];
        let mut app = tester.cell.borrow_mut();
        editable
            .get_endpoints_for_selection(&mut app, selection)
            .into_iter()
            .map(|point| editable.as_box().local_to_global(&app, point.point, None))
            .collect()
    }

    /// Dart's `startGesture(from, pointer: 7)`, a frame, `moveTo(to)`, a frame, `up`, a frame.
    fn drag_handle(tester: &mut WidgetTester, from: Offset, to: Offset) {
        let mut gesture = TestPointer::touch(7);
        tester.down(&mut gesture, from);
        tester.pump();
        tester.move_to(&mut gesture, to, true);
        tester.pump();
        tester.up(&gesture);
        tester.pump();
    }

    #[test]
    fn can_drag_handles_to_change_selection_on_android_and_fuchsia() {
        for platform in [TargetPlatform::Android, TargetPlatform::Fuchsia] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let controller = controller_with(tester, "");
                let root = overlay(
                    tester,
                    TextField::new()
                        .drag_start_behavior(DragStartBehavior::Down)
                        .controller(controller.erase()),
                );
                tester.pump_widget(root);

                let test_value = "abc def ghi";
                tester.enter_text(find::by_type::<EditableText>().at(0), test_value);
                skip_past_scrolling_animation(tester);

                // Long press the 'e' to select 'def'.
                let e_pos = text_offset_to_position(tester, test_value.find('e').unwrap() as i32);
                let mut gesture = TestPointer::touch(7);
                tester.down(&mut gesture, e_pos);
                tester.pump_by(Duration::from_secs(2));
                tester.up(&gesture);
                tester.pump();
                // skip past the frame where the opacity is zero
                tester.pump_by(Duration::from_millis(200));

                let long_pressed = selection(tester, controller.handle());
                assert_eq!(long_pressed.base_offset, 4, "{platform:?}");
                assert_eq!(long_pressed.extent_offset, 7, "{platform:?}");

                // On Fuchsia the desktop-style toolbar shown by the long press covers the
                // handles; hide it so the drags below reach them.
                editable_state(tester).hide_toolbar(&mut tester.cell.borrow_mut(), false);
                tester.pump();

                let endpoints = global_endpoints(tester, long_pressed);
                assert_eq!(endpoints.len(), 2);

                // Drag the right handle 2 letters to the right.
                // We use a small offset because the endpoint is on the very corner
                // of the handle.
                let handle_pos = endpoints[1] + Offset::new(1.0, 1.0);
                let new_handle_pos = text_offset_to_position(tester, test_value.len() as i32);
                drag_handle(tester, handle_pos, new_handle_pos);

                assert_eq!(
                    selection(tester, controller.handle()).base_offset,
                    4,
                    "{platform:?}"
                );
                assert_eq!(
                    selection(tester, controller.handle()).extent_offset,
                    11,
                    "{platform:?}"
                );

                // Drag the left handle 2 letters to the left.
                let handle_pos = endpoints[0] + Offset::new(-1.0, 1.0);
                let new_handle_pos = text_offset_to_position(tester, 2);
                drag_handle(tester, handle_pos, new_handle_pos);

                assert_eq!(
                    selection(tester, controller.handle()).base_offset,
                    2,
                    "{platform:?}"
                );
                assert_eq!(
                    selection(tester, controller.handle()).extent_offset,
                    11,
                    "{platform:?}"
                );

                // Drag the left handle 2 letters to the left again.
                let endpoints = global_endpoints(tester, selection(tester, controller.handle()));
                let handle_pos = endpoints[0] + Offset::new(-1.0, 1.0);
                let new_handle_pos = text_offset_to_position(tester, 0);
                drag_handle(tester, handle_pos, new_handle_pos);

                assert_eq!(
                    selection(tester, controller.handle()).base_offset,
                    0,
                    "{platform:?}"
                );
                assert_eq!(
                    selection(tester, controller.handle()).extent_offset,
                    11,
                    "{platform:?}"
                );
            });
        }
    }

    #[test]
    fn tapping_selection_handles_toggles_the_toolbar() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "abc def ghi");
            let root = material_home(
                theme(tester),
                TextField::new().controller(controller.erase()),
            );
            tester.pump_widget(root);

            // Tap to position the cursor and show the selection handles.
            let e_pos = text_offset_to_position(tester, 5); // Index of 'e'.
            tester.tap_at(e_pos, 7);
            tester.pump_and_settle();

            assert_eq!(selection_overlay_visibility(tester), (true, false));

            let endpoints = global_endpoints(tester, selection(tester, controller.handle()));
            assert_eq!(endpoints.len(), 1);

            // Tap the handle to show the toolbar.
            let handle_pos = endpoints[0] + Offset::new(0.0, 1.0);
            tester.tap_at(handle_pos, 7);
            tester.pump();
            assert!(selection_overlay_visibility(tester).1);

            // Tap the handle again to hide the toolbar.
            tester.tap_at(handle_pos, 7);
            tester.pump();
            assert!(!selection_overlay_visibility(tester).1);
        });
    }

    #[test]
    fn mouse_long_press_does_not_show_handles_nor_toolbar() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "abc def ghi");
            let root = material_home(
                theme(tester),
                TextField::new().controller(controller.erase()),
            );
            tester.pump_widget(root);

            // Long press to trigger the text field.
            let text_field_pos = text_field_center(tester, 0);
            let mut gesture = TestPointer::mouse(7, waft_gestures::K_PRIMARY_BUTTON);
            tester.down(&mut gesture, text_field_pos);
            tester.pump_by(Duration::from_secs(2));
            tester.up(&gesture);
            tester.pump();

            assert_eq!(selection_overlay_visibility(tester), (false, false));
        });
    }

    #[test]
    fn mouse_double_tap_does_not_show_handles_nor_toolbar() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "abc def ghi");
            let root = material_home(
                theme(tester),
                TextField::new().controller(controller.erase()),
            );
            tester.pump_widget(root);

            // Double tap to trigger the text field.
            let text_field_pos = text_field_center(tester, 0);
            let mut gesture = TestPointer::mouse(7, waft_gestures::K_PRIMARY_BUTTON);
            tester.down(&mut gesture, text_field_pos);
            tester.pump_by(Duration::from_millis(50));
            tester.up(&gesture);
            tester.pump();
            tester.send(&gesture, waft_embedder::PointerChange::Down);
            tester.pump();
            tester.up(&gesture);
            tester.pump();

            assert_eq!(selection_overlay_visibility(tester), (false, false));
        });
    }

    #[test]
    fn by_default_text_field_is_at_least_k_min_interactive_dimension_high() {
        test_widgets(|tester| {
            let root = material_app(
                theme(tester),
                Scaffold::new()
                    .body(Center::new().child(TextField::new()))
                    .into_widget(),
            );
            tester.pump_widget(root);
            let element = find::by_type::<TextField>().evaluate(tester)[0];
            let height = tester.get_rect(element).height();
            assert!(height >= K_MIN_INTERACTIVE_DIMENSION, "{height}");
        });
    }

    #[test]
    fn when_text_is_very_small_text_field_still_doesnt_go_below_k_min_interactive_dimension_height()
    {
        test_widgets(|tester| {
            let root = material_app(
                theme(tester),
                Scaffold::new()
                    .body(
                        Center::new()
                            .child(TextField::new().style(TextStyle::new().font_size(2.0))),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);
            let element = find::by_type::<TextField>().evaluate(tester)[0];
            let height = tester.get_rect(element).height();
            assert_eq!(height, K_MIN_INTERACTIVE_DIMENSION);
        });
    }

    #[test]
    fn a_build_counter_that_returns_null_doesnt_affect_the_size_of_the_text_field() {
        test_widgets(|tester| {
            let text_field1_key = GlobalKey::new();
            let text_field2_key = GlobalKey::new();
            let root = material_app(
                theme(tester),
                Scaffold::new()
                    .body(Column::new().children(vec![
                        TextField::new()
                            .key(text_field1_key.clone().into())
                            .into_widget(),
                        TextField::new()
                            .key(text_field2_key.clone().into())
                            .max_length(1)
                            .build_counter(|_app, _context, _current, _max, _focused| None)
                            .into_widget(),
                    ]))
                    .into_widget(),
            );
            tester.pump_widget(root);
            tester.pump_and_settle();
            let size = |key: &GlobalKey| {
                let render = tester.render_object(key).as_box().expect("a render box");
                render.size(&tester.cell.borrow())
            };
            assert_eq!(size(&text_field1_key), size(&text_field2_key));
        });
    }

    fn cursor_frame<K>(tester: &WidgetTester, field: impl IntoWidget<K>) -> WidgetRef {
        material_home(
            theme(tester),
            MouseRegion::new()
                .cursor(SystemMouseCursors::FORBIDDEN.into())
                .child(field),
        )
    }

    #[test]
    fn text_field_changes_mouse_cursor_when_hovered() {
        test_widgets(|tester| {
            let decoration = || {
                // Add an icon so that the left edge is not the text area
                InputDecoration::new().icon(waft_widgets::Icon::new(Some(Icons::person())))
            };
            let root = cursor_frame(
                tester,
                TextField::new()
                    .mouse_cursor(SystemMouseCursors::GRAB.into())
                    .decoration(Some(decoration())),
            );
            tester.pump_widget(root);

            let element = find::by_type::<TextField>().evaluate(tester)[0];
            let rect = tester.get_rect(element);
            // Center, which is within the text area
            let center = rect.center();
            // Top left, which is not the text area
            let edge = rect.top_left() + Offset::new(1.0, 1.0);

            let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.add_pointer(&mut gesture, center);
            tester.pump();
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::GRAB.into());

            // Test default cursor
            let root = cursor_frame(tester, TextField::new().decoration(Some(decoration())));
            tester.pump_widget(root);
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());
            tester.move_to(&mut gesture, edge, false);
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());
            tester.move_to(&mut gesture, center, false);

            // Test default cursor when disabled
            let root = cursor_frame(
                tester,
                TextField::new()
                    .enabled(false)
                    .decoration(Some(decoration())),
            );
            tester.pump_widget(root);
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::BASIC.into());
            tester.move_to(&mut gesture, edge, false);
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::BASIC.into());
            tester.move_to(&mut gesture, center, false);
        });
    }

    #[test]
    fn text_fields_can_inherit_error_style_color_from_input_decoration_theme_data() {
        test_widgets(|tester| {
            let green = Colors::GREEN.primary();
            let decoration_theme =
                InputDecorationThemeData::new().error_style(TextStyle::new().color(green));
            let field =
                || TextField::new().decoration(Some(InputDecoration::new().error_text("error")));

            // Global theme.
            let theme_data = ThemeData::builder()
                .input_decoration_theme(decoration_theme.clone())
                .build(&tester.cell.borrow());
            let root = material_app(theme_data, Scaffold::new().body(field()).into_widget());
            tester.pump_widget(root);
            assert_eq!(editable(tester, |widget| widget.cursor_color), green);

            // Local theme.
            let root = material_app(
                theme(tester),
                Scaffold::new()
                    .body(
                        InputDecorationTheme::new()
                            .with_data(decoration_theme)
                            .child(field()),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);
            assert_eq!(editable(tester, |widget| widget.cursor_color), green);
        });
    }

    #[test]
    fn text_field_can_inherit_decoration_from_local_input_decoration_theme_data() {
        test_widgets(|tester| {
            let decoration = InputDecoration::new().label_text("Label");
            let decoration_theme = InputDecorationThemeData::new()
                .error_style(TextStyle::new().color(Colors::GREEN.primary()));
            let root = material_app(
                theme(tester),
                Scaffold::new()
                    .body(
                        InputDecorationTheme::new()
                            .with_data(decoration_theme.clone())
                            .child(TextField::new().decoration(Some(decoration.clone()))),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            let element = find::by_type::<InputDecorator>().evaluate(tester)[0];
            let given = tester
                .widget::<InputDecorator, _>(element, |decorator| decorator.decoration.clone());
            let expected = decoration
                .apply_defaults(decoration_theme)
                .copy_with()
                .enabled(true)
                .hint_max_lines(1);
            assert_eq!(given, expected);
        });
    }

    #[test]
    fn text_field_passes_enable_inline_prediction_to_editable_text() {
        test_widgets(|tester| {
            let root = material_app(
                theme(tester),
                Scaffold::new()
                    .body(TextField::new().enable_inline_prediction(true))
                    .into_widget(),
            );
            tester.pump_widget(root);
            assert_eq!(
                editable(tester, |widget| widget.enable_inline_prediction),
                Some(true)
            );
        });
    }

    #[test]
    fn text_field_enable_inline_prediction_defaults_to_null() {
        test_widgets(|tester| {
            let root = material_app(
                theme(tester),
                Scaffold::new().body(TextField::new()).into_widget(),
            );
            tester.pump_widget(root);
            assert_eq!(
                editable(tester, |widget| widget.enable_inline_prediction),
                None
            );
        });
    }

    /// The `MaxLengthEnforcement` group's `setupWidget`.
    fn max_length_enforcement_field(
        platform: TargetPlatform,
        enforcement: Option<MaxLengthEnforcement>,
    ) -> WidgetTester {
        let mut tester = WidgetTester::on(platform);
        let mut field = TextField::new().max_length(5);
        if let Some(enforcement) = enforcement {
            field = field.max_length_enforcement(enforcement);
        }
        let root = material_home(theme(&tester), field);
        tester.pump_widget(root);
        tester.pump_and_settle();
        tester
    }

    fn update_editing_value(tester: &WidgetTester, value: TextEditingValue) {
        let state = editable_state(tester);
        TextInputClient::update_editing_value(state, &mut tester.cell.borrow_mut(), value);
    }

    fn current_value(tester: &WidgetTester) -> TextEditingValue {
        let state = editable_state(tester);
        TextInputClient::current_text_editing_value(state, &tester.cell.borrow())
            .expect("the field has a value")
    }

    fn value(text: &str) -> TextEditingValue {
        TextEditingValue::new().text(text)
    }

    fn composing(text: &str, start: i32, end: i32) -> TextEditingValue {
        TextEditingValue::new()
            .text(text)
            .composing(TextRange::new(start, end))
    }

    #[test]
    fn max_length_enforcement_using_none_enforcement() {
        test_widgets(|_tester| {
            let tester = max_length_enforcement_field(
                TargetPlatform::Android,
                Some(MaxLengthEnforcement::None),
            );

            update_editing_value(&tester, value("abc"));
            assert_eq!(current_value(&tester).text, "abc");
            assert_eq!(current_value(&tester).composing, TextRange::EMPTY);

            update_editing_value(&tester, composing("abcdef", 3, 6));
            assert_eq!(current_value(&tester).text, "abcdef");
            assert_eq!(current_value(&tester).composing, TextRange::new(3, 6));

            update_editing_value(&tester, value("abcdef"));
            assert_eq!(current_value(&tester).text, "abcdef");
            assert_eq!(current_value(&tester).composing, TextRange::EMPTY);
        });
    }

    #[test]
    fn max_length_enforcement_using_enforced() {
        test_widgets(|_tester| {
            let tester = max_length_enforcement_field(
                TargetPlatform::Android,
                Some(MaxLengthEnforcement::Enforced),
            );

            update_editing_value(&tester, value("abc"));
            assert_eq!(current_value(&tester).text, "abc");
            assert_eq!(current_value(&tester).composing, TextRange::EMPTY);

            update_editing_value(&tester, composing("abcde", 3, 5));
            assert_eq!(current_value(&tester).text, "abcde");
            assert_eq!(current_value(&tester).composing, TextRange::new(3, 5));

            update_editing_value(&tester, composing("abcdef", 3, 6));
            assert_eq!(current_value(&tester).text, "abcde");
            assert_eq!(current_value(&tester).composing, TextRange::new(3, 5));

            update_editing_value(&tester, value("abcdef"));
            assert_eq!(current_value(&tester).text, "abcde");
            assert_eq!(current_value(&tester).composing, TextRange::new(3, 5));
        });
    }

    #[test]
    fn max_length_enforcement_using_truncate_after_composition_ends() {
        test_widgets(|_tester| {
            let tester = max_length_enforcement_field(
                TargetPlatform::Android,
                Some(MaxLengthEnforcement::TruncateAfterCompositionEnds),
            );

            update_editing_value(&tester, value("abc"));
            assert_eq!(current_value(&tester).text, "abc");
            assert_eq!(current_value(&tester).composing, TextRange::EMPTY);

            update_editing_value(&tester, composing("abcde", 3, 5));
            assert_eq!(current_value(&tester).text, "abcde");
            assert_eq!(current_value(&tester).composing, TextRange::new(3, 5));

            update_editing_value(&tester, composing("abcdef", 3, 6));
            assert_eq!(current_value(&tester).text, "abcdef");
            assert_eq!(current_value(&tester).composing, TextRange::new(3, 6));

            update_editing_value(&tester, value("abcdef"));
            assert_eq!(current_value(&tester).text, "abcde");
            assert_eq!(current_value(&tester).composing, TextRange::EMPTY);
        });
    }

    #[test]
    fn max_length_enforcement_using_default_behavior_for_different_platforms() {
        for platform in ALL_PLATFORMS {
            test_widgets(|_tester| {
                let tester = max_length_enforcement_field(platform, None);

                update_editing_value(&tester, value("侬好啊"));
                assert_eq!(current_value(&tester).text, "侬好啊");
                assert_eq!(current_value(&tester).composing, TextRange::EMPTY);

                update_editing_value(&tester, composing("侬好啊旁友", 3, 5));
                assert_eq!(current_value(&tester).text, "侬好啊旁友");
                assert_eq!(current_value(&tester).composing, TextRange::new(3, 5));

                update_editing_value(&tester, composing("侬好啊旁友们", 3, 6));
                if matches!(
                    platform,
                    TargetPlatform::IOS
                        | TargetPlatform::MacOS
                        | TargetPlatform::Linux
                        | TargetPlatform::Fuchsia
                ) {
                    assert_eq!(current_value(&tester).text, "侬好啊旁友们", "{platform:?}");
                    assert_eq!(
                        current_value(&tester).composing,
                        TextRange::new(3, 6),
                        "{platform:?}"
                    );
                } else {
                    assert_eq!(current_value(&tester).text, "侬好啊旁友", "{platform:?}");
                    assert_eq!(
                        current_value(&tester).composing,
                        TextRange::new(3, 5),
                        "{platform:?}"
                    );
                }

                update_editing_value(&tester, value("侬好啊旁友"));
                assert_eq!(current_value(&tester).text, "侬好啊旁友");
                assert_eq!(current_value(&tester).composing, TextRange::EMPTY);
            });
        }
    }

    #[test]
    fn cannot_request_focus_when_can_request_focus_is_false() {
        test_widgets(|tester| {
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut());

            // Default test. The canRequestFocus is true by default and the text field can be
            // focused
            let root = boilerplate(tester, TextField::new().focus_node(focus_node.erase()));
            tester.pump_widget(root);
            assert!(!has_focus(tester, focus_node.handle()));
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump();
            assert!(has_focus(tester, focus_node.handle()));

            // Set canRequestFocus to false: the text field cannot be focused when it is
            // tapped/long pressed.
            let root = boilerplate(
                tester,
                TextField::new()
                    .focus_node(focus_node.erase())
                    .can_request_focus(false),
            );
            tester.pump_widget(root);

            assert!(!has_focus(tester, focus_node.handle()));
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump();
            assert!(!has_focus(tester, focus_node.handle()));

            // The text field cannot be focused if it is tapped.
            let center = text_field_center(tester, 0);
            tester.tap_at(center, 1);
            tester.pump();
            assert!(!has_focus(tester, focus_node.handle()));

            // The text field cannot be focused if it is long pressed.
            tester.long_press_at(center, 2);
            tester.pump();
            assert!(!has_focus(tester, focus_node.handle()));
        });
    }

    fn tap_region_frame(tester: &WidgetTester, focus_node: FocusNodeRef) -> WidgetRef {
        material_app(
            theme(tester),
            Scaffold::new()
                .body(
                    Center::new().child(
                        SizedBox::square(Some(100.0)).child(
                            Opacity::new(0.5).child(
                                TextField::new()
                                    .autofocus(true)
                                    .focus_node(focus_node)
                                    .decoration(Some(
                                        InputDecoration::new()
                                            .hint_text("Placeholder")
                                            .border(OutlineInputBorder::new()),
                                    )),
                            ),
                        ),
                    ),
                )
                .into_widget(),
        )
    }

    #[test]
    fn tap_region_integration_tapping_outside_loses_focus_on_desktop() {
        for platform in DESKTOP {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let focus_node = FocusNode::new(&mut tester.cell.borrow_mut());
                let root = tap_region_frame(tester, focus_node.erase());
                tester.pump_widget(root);
                tester.pump();
                assert!(focus_node.has_primary_focus(&tester.cell.borrow()));

                tester.tap_at(Offset::new(10.0, 10.0), 1);
                tester.pump();

                assert!(
                    !focus_node.has_primary_focus(&tester.cell.borrow()),
                    "{platform:?}"
                );
            });
        }
    }

    #[test]
    fn tap_region_integration_tapping_outside_doesnt_lose_focus_on_mobile() {
        for platform in MOBILE {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let focus_node = FocusNode::new(&mut tester.cell.borrow_mut());
                let root = tap_region_frame(tester, focus_node.erase());
                tester.pump_widget(root);
                tester.pump();
                assert!(focus_node.has_primary_focus(&tester.cell.borrow()));

                tester.tap_at(Offset::new(10.0, 10.0), 1);
                tester.pump();

                // Focus is lost on mobile browsers, but not mobile apps.
                assert!(
                    focus_node.has_primary_focus(&tester.cell.borrow()),
                    "{platform:?}"
                );
            });
        }
    }

    #[test]
    fn infer_android_spell_check_configuration_fills_the_material_defaults() {
        assert_eq!(
            TextField::infer_android_spell_check_configuration(None),
            SpellCheckConfiguration::disabled()
        );
        assert_eq!(
            TextField::infer_android_spell_check_configuration(Some(
                &SpellCheckConfiguration::disabled()
            )),
            SpellCheckConfiguration::disabled()
        );
        let inferred = TextField::infer_android_spell_check_configuration(Some(
            &SpellCheckConfiguration::new(),
        ));
        assert_eq!(
            inferred.misspelled_text_style,
            Some(TextField::material_misspelled_text_style())
        );
        assert!(Rc::ptr_eq(
            &inferred
                .spell_check_suggestions_toolbar_builder
                .expect("the default builder"),
            &TextField::default_spell_check_suggestions_toolbar_builder_tear_off(),
        ));
    }

    /// text_field_restoration_test's `TestWidget`: a field of three lines in a 50 wide box,
    /// with an outside controller when `use_external`.
    fn restoration_widget(controller: Option<TextEditingControllerRef>) -> WidgetRef {
        let mut field = TextField::new().restoration_id("text").max_lines(Some(3));
        if let Some(controller) = controller {
            field = field.controller(controller.clone());
        }
        Material::new()
            .child(waft_widgets::Align::new().child(SizedBox::new().width(50.0).child(field)))
            .into_widget()
    }

    fn restored(data: Option<waft_embedder::RestorationMap>) -> WidgetTester {
        let mut tester = WidgetTester::restoring(data);
        let theme = theme(&tester);
        tester.pump_widget(crate::test_support::restorable_material_app(
            "root",
            theme,
            restoration_widget(None),
        ));
        tester
    }

    #[test]
    fn the_local_controller_s_text_is_restored() {
        test_widgets(|_tester| {
            let text = "Hello World";
            let alternative_text = "Something Else";
            let mut tester = restored(None);
            expect(&tester, &find::text(text), finds_nothing());

            tester.enter_text(find::by_type::<EditableText>().at(0), text);
            skip_past_scrolling_animation(&mut tester);

            let data = tester.restoration_data();
            let tester = restored(data.clone());
            assert_eq!(current_value(&tester).text, text);

            let mut tester = restored(data.clone());
            tester.enter_text(find::by_type::<EditableText>().at(0), alternative_text);
            skip_past_scrolling_animation(&mut tester);
            assert_eq!(current_value(&tester).text, alternative_text);

            let tester = restored(data);
            assert_eq!(current_value(&tester).text, text);
        });
    }

    fn render_editable(tester: &WidgetTester) -> waft_rendering::RenderHandle<RenderEditable> {
        editable_state(tester).render_editable(&tester.cell.borrow())
    }

    #[test]
    fn cursor_radius_is_2_0() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let root = material_home(theme(tester), TextField::new());
                tester.pump_widget(root);
                let radius = render_editable(tester).cursor_radius(&tester.cell.borrow());
                assert_eq!(radius, Some(Radius::circular(2.0)), "{platform:?}");
            });
        }
    }

    #[test]
    fn cursor_animates_on_ios() {
        test_widgets_variant(TargetPlatformVariant::only(TargetPlatform::IOS), |tester| {
            let root = material_home(theme(tester), TextField::new());
            tester.pump_widget(root);

            let center = text_field_center(tester, 0);
            tester.tap_at(center, 1);
            tester.pump();

            let opacity = |tester: &WidgetTester| {
                render_editable(tester)
                    .cursor_color(&tester.cell.borrow())
                    .expect("the cursor has a color")
                    .opacity()
            };
            assert_eq!(opacity(tester), 1.0);

            let mut walltime_microsecond = 0;
            let mut last_verified_opacity = 1.0;
            let mut verify_key_frame = |tester: &mut WidgetTester, opacity_at: f64, at: u64| {
                let delta = 1;
                assert!(at - delta > walltime_microsecond);
                tester.pump_by(Duration::from_micros(at - delta - walltime_microsecond));

                // Instead of verifying the opacity at each key frame, this function
                // verifies the opacity immediately *before* each key frame to avoid
                // fp precision issues.
                let actual = opacity(tester);
                assert!(
                    (actual - last_verified_opacity).abs() <= 0.01,
                    "opacity at {} microseconds: {actual}",
                    at - delta
                );

                walltime_microsecond = at - delta;
                last_verified_opacity = opacity_at;
            };

            verify_key_frame(tester, 1.0, 500000);
            verify_key_frame(tester, 0.75, 537500);
            verify_key_frame(tester, 0.5, 575000);
            verify_key_frame(tester, 0.25, 612500);
            verify_key_frame(tester, 0.0, 650000);
            verify_key_frame(tester, 0.0, 850000);
            verify_key_frame(tester, 0.25, 887500);
            verify_key_frame(tester, 0.5, 925000);
            verify_key_frame(tester, 0.75, 962500);
            verify_key_frame(tester, 1.0, 1000000);
        });
    }

    #[test]
    fn cursor_does_not_animate_on_non_ios_platforms() {
        for platform in ALL_PLATFORMS
            .into_iter()
            .filter(|platform| *platform != TargetPlatform::IOS)
        {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let root = material_home(theme(tester), TextField::new().max_lines(Some(3)));
                tester.pump_widget(root);

                let center = text_field_center(tester, 0);
                tester.tap_at(center, 1);
                tester.pump();
                // Wait for the current animation to finish. If the cursor never stops its
                // blinking animation the test will timeout.
                tester.pump_and_settle();

                for _ in 0..40 {
                    tester.pump_by(Duration::from_millis(100));
                    let running = waft_scheduler::SchedulerBinding::transient_callback_count(
                        &mut tester.cell.borrow_mut(),
                    ) > 0;
                    assert!(!running, "{platform:?}");
                }
            });
        }
    }

    #[test]
    fn cursor_does_not_animate_on_android() {
        test_widgets(|tester| {
            let default_cursor_color = theme(tester).color_scheme.primary;
            let root = material_home(
                theme(tester),
                TextField::new()
                    .max_lines(Some(3))
                    .cursor_color(default_cursor_color),
            );
            tester.pump_widget(root);

            let center = text_field_center(tester, 0);
            tester.tap_at(center, 1);
            tester.pump();

            let alpha = |tester: &WidgetTester| {
                render_editable(tester)
                    .cursor_color(&tester.cell.borrow())
                    .expect("the cursor has a color")
                    .alpha()
            };
            let paints_nothing = |tester: &WidgetTester| {
                let render = render_editable(tester).as_object();
                paints(&tester.painted_ops(render), &[PaintCall::rect(None)]).is_err()
            };
            let paints_cursor = |tester: &WidgetTester| {
                let render = render_editable(tester).as_object();
                paints(
                    &tester.painted_ops(render),
                    &[PaintCall::rect(Some(default_cursor_color))],
                )
                .is_ok()
            };

            tester.pump();
            assert_eq!(alpha(tester), 255);
            assert!(paints_cursor(tester));

            // Android cursor goes from exactly on to exactly off on the 500ms dot.
            tester.pump_by(Duration::from_millis(499));
            assert_eq!(alpha(tester), 255);
            assert!(paints_cursor(tester));

            tester.pump_by(Duration::from_millis(1));
            assert_eq!(alpha(tester), 0);
            // Don't try to draw the cursor.
            assert!(paints_nothing(tester));

            tester.pump_by(Duration::from_millis(500));
            assert_eq!(alpha(tester), 255);
            assert!(paints_cursor(tester));

            tester.pump_by(Duration::from_millis(500));
            assert_eq!(alpha(tester), 0);
            assert!(paints_nothing(tester));
        });
    }

    /// The cursor and selection settings `_TextFieldState.build` picks for `platform`, as the
    /// `EditableText` receives them.
    fn platform_settings(platform: TargetPlatform) -> (bool, bool, Option<Radius>, Option<Offset>) {
        let mut tester = WidgetTester::on(platform);
        let root = material_home(theme(&tester), TextField::new());
        tester.pump_widget(root);
        editable(&tester, |widget| {
            (
                widget.paint_cursor_above_text,
                widget.cursor_opacity_animates,
                widget.cursor_radius,
                widget.cursor_offset,
            )
        })
    }

    #[test]
    fn the_cursor_settings_follow_the_platform() {
        test_widgets(|_tester| {
            // iOSHorizontalOffset (-2.0) over the test view's device pixel ratio of 3.
            let apple_offset = Some(Offset::new(-2.0 / 3.0, 0.0));
            assert_eq!(
                platform_settings(TargetPlatform::IOS),
                (true, true, Some(Radius::circular(2.0)), apple_offset)
            );
            assert_eq!(
                platform_settings(TargetPlatform::MacOS),
                (true, false, Some(Radius::circular(2.0)), apple_offset)
            );
            for platform in [
                TargetPlatform::Android,
                TargetPlatform::Fuchsia,
                TargetPlatform::Linux,
                TargetPlatform::Windows,
            ] {
                assert_eq!(
                    platform_settings(platform),
                    (false, false, None, None),
                    "{platform:?}"
                );
            }
        });
    }

    #[test]
    fn the_force_press_and_selection_color_follow_the_platform() {
        test_widgets(|_tester| {
            for platform in ALL_PLATFORMS {
                let mut tester = WidgetTester::on(platform);
                let theme = theme(&tester);
                let primary = theme.color_scheme.primary;
                let root = material_home(theme, TextField::new().autofocus(true));
                tester.pump_widget(root);
                tester.pump();
                let state = find::by_type::<TextField>().evaluate(&tester)[0]
                    .state_handle::<TextFieldState>(&tester.cell.borrow())
                    .expect("a TextField state");
                let force_press = state.force_press_enabled(&tester.cell.borrow());
                assert_eq!(force_press, platform == TargetPlatform::IOS, "{platform:?}");
                let selection_color = editable(&tester, |widget| widget.selection_color);
                let expected = match platform {
                    TargetPlatform::IOS | TargetPlatform::MacOS => {
                        let element = find::by_type::<EditableText>().evaluate(&tester)[0];
                        let cupertino = CupertinoTheme::of(&mut tester.cell.borrow_mut(), element);
                        cupertino.primary_color().color().with_opacity(0.40)
                    }
                    _ => primary.with_opacity(0.40),
                };
                assert_eq!(selection_color, Some(expected), "{platform:?}");
            }
        });
    }

    #[test]
    fn text_field_has_consistent_size() {
        test_widgets(|tester| {
            let key: KeyRef = UniqueKey::new().into();
            let value = Rc::new(RefCell::new(None::<String>));
            let seen = Rc::clone(&value);
            let root = overlay(
                tester,
                TextField::new()
                    .key(key.clone())
                    .decoration(Some(InputDecoration::new().hint_text("Placeholder")))
                    .on_changed(move |_app, text| *seen.borrow_mut() = Some(text)),
            );
            tester.pump_widget(root);
            let empty_size = tester.get_size(find::by_key(key.clone()));

            let check_text = |tester: &mut WidgetTester, text: &str| {
                assert_eq!(*value.borrow(), None, "before {text:?}");
                tester.enter_text(find::by_type::<TextField>(), text);
                assert_eq!(*value.borrow(), Some(text.to_owned()));
                *value.borrow_mut() = None;
                skip_past_scrolling_animation(tester);
                assert_eq!(tester.get_size(find::by_key(key.clone())), empty_size);
            };
            check_text(tester, " ");
            check_text(tester, "Test");
        });
    }

    #[test]
    fn render_editable_sets_correct_text_editing_value() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "how are you");
            let icon = UniqueKey::new();
            let key: KeyRef = icon.clone().into();
            let clearing = controller.clone();
            let root = material_home(
                theme(tester),
                TextField::new()
                    .controller(controller.erase())
                    .decoration(Some(
                        InputDecoration::new().suffix_icon(
                            IconButton::new(
                                Some(Listener::new(move |app| clearing.handle().clear(app))),
                                waft_widgets::Icon::new(Some(Icons::cancel())),
                            )
                            .key(key.clone()),
                        ),
                    )),
            );
            tester.pump_widget(root);
            tester.tap(find::by_key(key.clone()));
            tester.pump();
            assert_eq!(text(tester, controller.handle()), "");
            assert_eq!(
                selection(tester, controller.handle()),
                TextSelection::collapsed(0, TextAffinity::Downstream)
            );
        });
    }

    #[test]
    fn overflowing_a_line_with_spaces_stops_the_cursor_at_the_end() {
        test_widgets(|tester| {
            let key: KeyRef = UniqueKey::new().into();
            let controller = controller_with(tester, "");
            let root = overlay(
                tester,
                Theme::new(
                    m2_theme(tester),
                    TextField::new()
                        .key(key.clone())
                        .controller(controller.erase())
                        .max_lines(None),
                ),
            );
            tester.pump_widget(root);
            assert_eq!(selection(tester, controller.handle()).base_offset, -1);
            assert_eq!(selection(tester, controller.handle()).extent_offset, -1);

            let one_line = "enough text to be exactly at the end of the line.";
            tester.enter_text(find::by_type::<TextField>(), one_line);
            skip_past_scrolling_animation(tester);
            let one_line_size = tester.get_size(find::by_key(key.clone()));

            tester.tap_at(text_offset_to_position(tester, one_line.len() as i32), 1);
            tester.pump();

            let two_lines = "enough text to overflow the first line and go to the second";
            tester.enter_text(find::by_type::<TextField>(), two_lines);
            skip_past_scrolling_animation(tester);
            assert!(tester.get_size(find::by_key(key.clone())).height() > one_line_size.height());
            let two_line_size = tester.get_size(find::by_key(key.clone()));

            let spaces = format!("{one_line}          ");
            assert_eq!(spaces.len(), two_lines.len());
            tester.enter_text(find::by_type::<TextField>(), &spaces);
            skip_past_scrolling_animation(tester);
            assert_eq!(
                tester.get_size(find::by_key(key.clone())).height(),
                one_line_size.height()
            );

            let spaces_overflow = format!("{one_line}         a");
            assert_eq!(spaces_overflow.len(), two_lines.len());
            tester.enter_text(find::by_type::<TextField>(), &spaces_overflow);
            skip_past_scrolling_animation(tester);
            assert_eq!(
                tester.get_size(find::by_key(key.clone())).height(),
                two_line_size.height()
            );

            tester.enter_text(find::by_type::<TextField>(), &spaces);
            skip_past_scrolling_animation(tester);
            tester.tap_at(text_offset_to_position(tester, spaces.len() as i32), 1);
            tester.pump();

            let render = render_editable(tester);
            let width = {
                let app = tester.cell.borrow();
                render.size(&app).width()
            };
            let caret = {
                let mut app = tester.cell.borrow_mut();
                render
                    .get_local_rect_for_caret(&mut app, TextPosition::new(spaces.len() as i32))
                    .bottom_right()
            };
            assert_eq!(caret.dx(), width - 1.0);
        });
    }

    #[test]
    fn overflowing_a_line_with_spaces_stops_the_cursor_at_the_end_rtl_direction() {
        test_widgets(|tester| {
            let root = overlay(
                tester,
                TextField::new()
                    .text_direction(TextDirection::Rtl)
                    .max_lines(None),
            );
            tester.pump_widget(root);

            let one_line = "enough text to be exactly at the end of the line.";
            let spaces = format!("{one_line}          ");
            tester.enter_text(find::by_type::<TextField>(), &spaces);
            skip_past_scrolling_animation(tester);
            tester.tap_at(text_offset_to_position(tester, spaces.len() as i32), 1);
            tester.pump();

            let render = render_editable(tester);
            let caret = {
                let mut app = tester.cell.borrow_mut();
                render
                    .get_local_rect_for_caret(&mut app, TextPosition::new(spaces.len() as i32))
                    .top_left()
            };
            assert!(caret.dx() >= 0.0);
        });
    }

    /// Dart's `textFieldBuilder`: a 34pt field whose height follows `maxLines` and `minLines`.
    fn text_field_with_lines(
        tester: &WidgetTester,
        key: &KeyRef,
        max_lines: Option<i32>,
        min_lines: Option<i32>,
    ) -> WidgetRef {
        let mut field = TextField::new()
            .key(key.clone())
            .style(TextStyle::new().color(Colors::BLACK).font_size(34.0))
            .max_lines(max_lines)
            .decoration(Some(InputDecoration::new().hint_text("Placeholder")));
        if let Some(min_lines) = min_lines {
            field = field.min_lines(min_lines);
        }
        boilerplate(tester, field)
    }

    #[test]
    fn text_field_height_with_min_lines_unset() {
        test_widgets(|tester| {
            let key: KeyRef = UniqueKey::new().into();
            tester.pump_widget(text_field_with_lines(tester, &key, Some(1), None));
            let empty_size = tester.get_size(find::by_key(key.clone()));

            tester.enter_text(find::by_type::<TextField>(), "No wrapping here.");
            tester.pump_widget(text_field_with_lines(tester, &key, Some(1), None));
            assert_eq!(tester.get_size(find::by_key(key.clone())), empty_size);

            // Even entering multiline text wakes no room on a single-line field.
            tester.enter_text(find::by_type::<TextField>(), K_THREE_LINES);
            tester.pump_widget(text_field_with_lines(tester, &key, Some(1), None));
            assert_eq!(tester.get_size(find::by_key(key.clone())), empty_size);

            // maxLines: 3 makes the field three lines tall.
            tester.enter_text(find::by_type::<TextField>(), "");
            tester.pump_widget(text_field_with_lines(tester, &key, Some(3), None));
            let three_line = tester.get_size(find::by_key(key.clone()));
            assert!(three_line.height() > empty_size.height());
            assert_eq!(three_line.width(), empty_size.width());

            // Filling three lines stays the same size.
            tester.enter_text(find::by_type::<TextField>(), K_THREE_LINES);
            tester.pump_widget(text_field_with_lines(tester, &key, Some(3), None));
            assert_eq!(tester.get_size(find::by_key(key.clone())), three_line);

            // An extra line does not grow past the max.
            tester.enter_text(find::by_type::<TextField>(), K_MORE_THAN_FOUR_LINES);
            tester.pump_widget(text_field_with_lines(tester, &key, Some(3), None));
            assert_eq!(tester.get_size(find::by_key(key.clone())), three_line);

            // maxLines: 4 is taller than three.
            tester.pump_widget(text_field_with_lines(tester, &key, Some(4), None));
            let four_line = tester.get_size(find::by_key(key.clone()));
            assert!(four_line.height() > three_line.height());
            assert_eq!(four_line.width(), three_line.width());

            // With no max, an empty field is one line and grows with the text.
            tester.enter_text(find::by_type::<TextField>(), "");
            tester.pump_widget(text_field_with_lines(tester, &key, None, None));
            assert_eq!(tester.get_size(find::by_key(key.clone())), empty_size);
            tester.enter_text(find::by_type::<TextField>(), K_THREE_LINES);
            tester.pump();
            assert_eq!(tester.get_size(find::by_key(key.clone())), three_line);
            tester.enter_text(find::by_type::<TextField>(), K_MORE_THAN_FOUR_LINES);
            tester.pump();
            assert!(tester.get_size(find::by_key(key.clone())).height() > four_line.height());
            assert_eq!(
                tester.get_size(find::by_key(key.clone())).width(),
                four_line.width()
            );
        });
    }

    #[test]
    fn text_field_height_with_min_lines_and_max_lines() {
        test_widgets(|tester| {
            let key: KeyRef = UniqueKey::new().into();
            tester.pump_widget(text_field_with_lines(tester, &key, Some(1), None));
            let empty_size = tester.get_size(find::by_key(key.clone()));

            tester.enter_text(find::by_type::<TextField>(), "No wrapping here.");
            tester.pump_widget(text_field_with_lines(tester, &key, Some(1), None));
            assert_eq!(tester.get_size(find::by_key(key.clone())), empty_size);

            // min and max the same locks the height.
            tester.pump_widget(text_field_with_lines(tester, &key, Some(3), Some(3)));
            let three_line = tester.get_size(find::by_key(key.clone()));
            assert!(three_line.height() > empty_size.height());
            assert_eq!(three_line.width(), empty_size.width());

            // A null max grows past the min.
            tester.pump_widget(text_field_with_lines(tester, &key, None, Some(3)));
            assert_eq!(tester.get_size(find::by_key(key.clone())), three_line);
            tester.enter_text(find::by_type::<TextField>(), K_MORE_THAN_FOUR_LINES);
            tester.pump();
            assert!(tester.get_size(find::by_key(key.clone())).height() > three_line.height());
            assert_eq!(
                tester.get_size(find::by_key(key.clone())).width(),
                three_line.width()
            );

            // min and max expand through the range.
            tester.enter_text(find::by_type::<TextField>(), "");
            tester.pump_widget(text_field_with_lines(tester, &key, Some(4), Some(3)));
            assert_eq!(tester.get_size(find::by_key(key.clone())), three_line);
            tester.enter_text(find::by_type::<TextField>(), K_MORE_THAN_FOUR_LINES);
            tester.pump();
            assert!(tester.get_size(find::by_key(key.clone())).height() > three_line.height());
            assert_eq!(
                tester.get_size(find::by_key(key.clone())).width(),
                three_line.width()
            );

            // minLines can't be greater than maxLines, and maxLines can't be the default 1.
            for (max_lines, min_lines) in [(Some(2), Some(3)), (Some(1), Some(3))] {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    tester.pump_widget(text_field_with_lines(tester, &key, max_lines, min_lines));
                }));
                assert!(result.is_err(), "{max_lines:?} and {min_lines:?}");
            }
        });
    }

    #[test]
    fn multiline_text_when_wrapped_in_expanded() {
        test_widgets(|tester| {
            let key: KeyRef = UniqueKey::new().into();
            let builder = |tester: &WidgetTester,
                           max_lines: Option<i32>,
                           min_lines: Option<i32>,
                           expands: bool| {
                let mut field = TextField::new()
                    .key(key.clone())
                    .style(TextStyle::new().color(Colors::BLACK).font_size(34.0))
                    .max_lines(max_lines)
                    .expands(expands)
                    .decoration(Some(InputDecoration::new().hint_text("Placeholder")));
                if let Some(min_lines) = min_lines {
                    field = field.min_lines(min_lines);
                }
                boilerplate(
                    tester,
                    Column::new().children([waft_widgets::Expanded::new(field).into_widget()]),
                )
            };
            let border = || {
                find::descendant(
                    find::by_type::<crate::input_decorator::InputDecorator>(),
                    find::by_type::<waft_widgets::CustomPaint>(),
                )
                .first()
            };
            tester.pump_widget(builder(tester, Some(1), None, false));
            let unexpanded = tester.get_size(border());

            tester.pump_widget(builder(tester, None, None, true));
            let expanded = tester.get_size(border());
            assert!(expanded.height() > unexpanded.height());
            assert_eq!(expanded.width(), unexpanded.width());

            for (max_lines, min_lines) in [(Some(4), None), (None, Some(1))] {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    tester.pump_widget(builder(tester, max_lines, min_lines, true));
                }));
                assert!(result.is_err(), "{max_lines:?} and {min_lines:?}");
            }
        });
    }

    #[test]
    fn multiline_text_when_wrapped_in_intrinsic_height() {
        test_widgets(|tester| {
            let key: KeyRef = UniqueKey::new().into();
            let intrinsic_key: KeyRef = UniqueKey::new().into();
            let builder = |tester: &WidgetTester, wrap: bool| {
                let field = TextFormField::new()
                    .key(key.clone())
                    .style(TextStyle::new().color(Colors::BLACK).font_size(34.0))
                    .max_lines(None)
                    .decoration(Some(InputDecoration::new().counter_text("I am counter")));
                let widget = if wrap {
                    IntrinsicHeight::new()
                        .child(field)
                        .key(intrinsic_key.clone())
                        .into_widget()
                } else {
                    field.into_widget()
                };
                boilerplate(tester, Column::new().children([widget]))
            };

            tester.pump_widget(builder(tester, false));
            assert!(
                find::by_key(intrinsic_key.clone())
                    .evaluate(tester)
                    .is_empty()
            );
            let unwrapped = tester.get_size(find::by_type::<EditableText>());

            tester.pump_widget(builder(tester, true));
            let wrapped = tester.get_size(find::by_type::<EditableText>());
            assert_eq!(wrapped.height(), unwrapped.height());
            assert_eq!(wrapped.width(), unwrapped.width());
        });
    }

    #[test]
    fn errortext_empty_string() {
        test_widgets(|tester| {
            let key: KeyRef = UniqueKey::new().into();
            let builder = |tester: &WidgetTester, error_text: Option<&str>| {
                let mut decoration = InputDecoration::new().counter_text("");
                if let Some(error_text) = error_text {
                    decoration = decoration.error_text(error_text);
                }
                boilerplate(
                    tester,
                    Column::new().children([TextFormField::new()
                        .key(key.clone())
                        .max_length(3)
                        .max_length_enforcement(MaxLengthEnforcement::None)
                        .decoration(Some(decoration))
                        .into_widget()]),
                )
            };
            tester.pump_widget(builder(tester, None));
            let null_size = tester.get_size(find::by_key(key.clone()));

            tester.pump_widget(builder(tester, Some("im errorText")));
            let error_size = tester.get_size(find::by_key(key.clone()));
            assert!(error_size.height() > null_size.height());
            assert_eq!(error_size.width(), null_size.width());

            // An empty error text still makes room for the line, though it shows nothing.
            tester.pump_widget(builder(tester, Some("")));
            assert_eq!(
                tester.get_size(find::by_key(key.clone())).height(),
                error_size.height()
            );
            assert_eq!(
                tester.get_size(find::by_key(key.clone())).width(),
                null_size.width()
            );
        });
    }

    #[test]
    fn growable_text_field_when_content_height_exceeds_parent() {
        test_widgets(|tester| {
            const HEIGHT: f64 = 200.0;
            const PADDING: f64 = 24.0;
            let key: KeyRef = UniqueKey::new().into();
            let theme = m2_theme(tester);
            let builder = |tester: &WidgetTester, decoration: InputDecoration| {
                boilerplate(
                    tester,
                    Theme::new(
                        theme.clone(),
                        SizedBox::new().height(HEIGHT).child(
                            TextField::new()
                                .key(key.clone())
                                .max_lines(None)
                                .decoration(Some(decoration)),
                        ),
                    ),
                )
            };
            tester.pump_widget(builder(tester, InputDecoration::new()));
            let editable = find::by_type::<EditableText>();
            tester.enter_text(find::by_type::<TextField>(), &("a\n".repeat(11)));
            tester.pump();
            let width = tester.get_size(editable.clone()).width();
            assert_eq!(
                tester.get_size(editable.clone()),
                waft_embedder::Size::new(width, HEIGHT - PADDING)
            );

            const COUNTER_HEIGHT: f64 = 40.0;
            const SUPPORTING_TEXT_GAP: f64 = 8.0;
            const COUNTER_SPACE: f64 = COUNTER_HEIGHT + SUPPORTING_TEXT_GAP;
            tester.pump_widget(builder(
                tester,
                InputDecoration::new().counter(Container::new().height(COUNTER_HEIGHT)),
            ));
            assert_eq!(
                tester.get_size(editable.clone()),
                waft_embedder::Size::new(width, HEIGHT - PADDING - COUNTER_SPACE)
            );

            tester.pump_widget(builder(
                tester,
                InputDecoration::new().helper_text("I am helperText"),
            ));
            assert_eq!(
                tester.get_size(editable.clone()),
                waft_embedder::Size::new(width, HEIGHT - PADDING - 12.0 - SUPPORTING_TEXT_GAP)
            );

            tester.pump_widget(builder(
                tester,
                InputDecoration::new()
                    .counter(Container::new().height(COUNTER_HEIGHT))
                    .helper_text("I am helperText"),
            ));
            assert_eq!(
                tester.get_size(editable.clone()),
                waft_embedder::Size::new(width, HEIGHT - PADDING - COUNTER_SPACE)
            );

            tester.pump_widget(builder(
                tester,
                InputDecoration::new().label_text("I am labelText"),
            ));
            assert_eq!(
                tester.get_size(editable.clone()),
                waft_embedder::Size::new(width, HEIGHT - PADDING - 16.0)
            );

            tester.pump_widget(builder(
                tester,
                InputDecoration::new()
                    .counter(Container::new().height(COUNTER_HEIGHT))
                    .label_text("I am labelText"),
            ));
            assert_eq!(
                tester.get_size(editable.clone()),
                waft_embedder::Size::new(width, HEIGHT - PADDING - COUNTER_SPACE - 16.0)
            );

            tester.pump_widget(builder(
                tester,
                InputDecoration::new()
                    .counter(Container::new().height(COUNTER_HEIGHT))
                    .label_text("I am labelText")
                    .prefix(SizedBox::new().width(10.0).height(60.0)),
            ));
            assert_eq!(
                tester.get_size(editable.clone()).height(),
                HEIGHT - PADDING - 16.0 - COUNTER_SPACE
            );
        });
    }

    #[test]
    fn multiline_hint_text_will_wrap_up_to_max_lines() {
        test_widgets(|tester| {
            let key: KeyRef = UniqueKey::new().into();
            let builder = |tester: &WidgetTester, max_lines: Option<i32>, hint: &str| {
                boilerplate(
                    tester,
                    TextField::new()
                        .key(key.clone())
                        .style(TextStyle::new().color(Colors::BLACK).font_size(34.0))
                        .max_lines(max_lines)
                        .decoration(Some(InputDecoration::new().hint_text(hint))),
                )
            };
            let placeholder = "Placeholder";
            tester.pump_widget(builder(tester, None, placeholder));
            let one_line = tester.get_size(find::text(placeholder));

            tester.pump_widget(builder(tester, None, placeholder));
            assert_eq!(tester.get_size(find::text(placeholder)), one_line);

            let multiple = "Here's a text, which is more than one line, to demonstrate the multiple line hint text";
            tester.pump_widget(builder(tester, Some(3), multiple));
            let max_lines = tester.widget::<Text, _>(find::text(multiple), |text| text.max_lines);
            assert_eq!(max_lines, Some(3));
            assert!(tester.get_size(find::text(multiple)).width() >= one_line.width());
            assert!(tester.get_size(find::text(multiple)).height() >= one_line.height());
        });
    }

    #[test]
    fn text_field_smoke_test() {
        test_widgets(|tester| {
            let value = Rc::new(RefCell::new(None::<String>));
            let seen = Rc::clone(&value);
            let root = overlay(
                tester,
                TextField::new()
                    .decoration(None)
                    .on_changed(move |_app, text| *seen.borrow_mut() = Some(text)),
            );
            tester.pump_widget(root);

            tester.enter_text(find::by_type::<TextField>(), "Hello World");
            assert_eq!(*value.borrow(), Some("Hello World".to_owned()));
        });
    }

    #[test]
    fn text_field_with_global_key() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let value = Rc::new(RefCell::new(None::<String>));
            let seen = Rc::clone(&value);
            let root = overlay(
                tester,
                TextField::new()
                    .key(key.clone().into())
                    .decoration(Some(InputDecoration::new().hint_text("Placeholder")))
                    .on_changed(move |_app, text| *seen.borrow_mut() = Some(text)),
            );
            tester.pump_widget(root);

            tester.enter_text(find::by_global_key(&key), "Hello World");
            assert_eq!(*value.borrow(), Some("Hello World".to_owned()));
        });
    }

    #[test]
    fn text_field_with_default_helper_style() {
        test_widgets(|tester| {
            let theme_data = ThemeData::builder()
                .use_material3(false)
                .hint_color(Colors::BLUE.shade500())
                .build(&tester.cell.borrow());
            let expected_size = Typography::english_like2014()
                .body_small
                .as_ref()
                .and_then(|style| style.font_size);
            let root = overlay(
                tester,
                Theme::new(
                    theme_data.clone(),
                    TextField::new()
                        .decoration(Some(InputDecoration::new().helper_text("helper text"))),
                ),
            );
            tester.pump_widget(root);
            let (color, font_size) = tester.widget::<Text, _>(find::text("helper text"), |text| {
                let style = text.style.as_ref().expect("the helper has a style");
                (
                    style.color.as_ref().map(|color| color.color()),
                    style.font_size,
                )
            });
            assert_eq!(color, Some(theme_data.hint_color));
            assert_eq!(font_size, expected_size);
        });
    }

    #[test]
    fn text_field_with_specified_helper_style() {
        test_widgets(|tester| {
            let style = TextStyle::new()
                .inherit(false)
                .color(Colors::PINK.shade500())
                .font_size(10.0);
            let root = overlay(
                tester,
                TextField::new().decoration(Some(
                    InputDecoration::new()
                        .helper_text("helper text")
                        .helper_style(style.clone()),
                )),
            );
            tester.pump_widget(root);
            let actual =
                tester.widget::<Text, _>(find::text("helper text"), |text| text.style.clone());
            assert_eq!(actual, Some(style));
        });
    }

    #[test]
    fn text_field_with_default_hint_style() {
        test_widgets(|tester| {
            let style = TextStyle::new()
                .color(Colors::PINK.shade500())
                .font_size(10.0);
            let theme_data = theme(tester);
            let expected_color = theme_data.color_scheme.on_surface_variant();
            let root = overlay(
                tester,
                TextField::new()
                    .style(style.clone())
                    .decoration(Some(InputDecoration::new().hint_text("Placeholder"))),
            );
            tester.pump_widget(root);
            let (color, font_size) = tester.widget::<Text, _>(find::text("Placeholder"), |text| {
                let style = text.style.as_ref().expect("the hint has a style");
                (
                    style.color.as_ref().map(|color| color.color()),
                    style.font_size,
                )
            });
            assert_eq!(color, Some(expected_color));
            assert_eq!(font_size, style.font_size);
        });
    }

    #[test]
    fn material2_text_field_with_default_hint_style() {
        test_widgets(|tester| {
            let style = TextStyle::new()
                .color(Colors::PINK.shade500())
                .font_size(10.0);
            let theme_data = ThemeData::builder()
                .use_material3(false)
                .hint_color(Colors::BLUE.shade500())
                .build(&tester.cell.borrow());
            let root = overlay(
                tester,
                Theme::new(
                    theme_data.clone(),
                    TextField::new()
                        .style(style.clone())
                        .decoration(Some(InputDecoration::new().hint_text("Placeholder"))),
                ),
            );
            tester.pump_widget(root);
            let (color, font_size) = tester.widget::<Text, _>(find::text("Placeholder"), |text| {
                let style = text.style.as_ref().expect("the hint has a style");
                (
                    style.color.as_ref().map(|color| color.color()),
                    style.font_size,
                )
            });
            assert_eq!(color, Some(theme_data.hint_color));
            assert_eq!(font_size, style.font_size);
        });
    }

    #[test]
    fn text_field_with_specified_hint_style() {
        test_widgets(|tester| {
            let hint_style = TextStyle::new()
                .inherit(false)
                .color(Colors::PINK.shade500())
                .font_size(10.0);
            let root = overlay(
                tester,
                TextField::new().decoration(Some(
                    InputDecoration::new()
                        .hint_text("Placeholder")
                        .hint_style(hint_style.clone()),
                )),
            );
            tester.pump_widget(root);
            let actual =
                tester.widget::<Text, _>(find::text("Placeholder"), |text| text.style.clone());
            assert_eq!(actual, Some(hint_style));
        });
    }

    #[test]
    fn text_field_with_specified_prefix_style() {
        test_widgets(|tester| {
            let prefix_style = TextStyle::new()
                .inherit(false)
                .color(Colors::PINK.shade500())
                .font_size(10.0);
            let root = overlay(
                tester,
                TextField::new().decoration(Some(
                    InputDecoration::new()
                        .prefix_text("Prefix:")
                        .prefix_style(prefix_style.clone()),
                )),
            );
            tester.pump_widget(root);
            let actual = tester.widget::<Text, _>(find::text("Prefix:"), |text| text.style.clone());
            assert_eq!(actual, Some(prefix_style));
        });
    }

    #[test]
    fn text_field_with_specified_suffix_style() {
        test_widgets(|tester| {
            let suffix_style = TextStyle::new()
                .color(Colors::PINK.shade500())
                .font_size(10.0);
            let root = overlay(
                tester,
                TextField::new().decoration(Some(
                    InputDecoration::new()
                        .suffix_text(".com")
                        .suffix_style(suffix_style.clone()),
                )),
            );
            tester.pump_widget(root);
            let actual = tester.widget::<Text, _>(find::text(".com"), |text| text.style.clone());
            assert_eq!(actual, Some(suffix_style));
        });
    }

    /// Dart's `getOpacity(tester, text)`: the value of the nearest `FadeTransition` above
    /// `text`.
    fn text_opacity(tester: &WidgetTester, text: &str) -> f64 {
        let fade = find::ancestor(
            find::text(text),
            find::by_type::<waft_widgets::FadeTransition>(),
        )
        .first();
        tester.widget::<waft_widgets::FadeTransition, _>(fade, |fade| {
            fade.opacity.value(&tester.cell.borrow())
        })
    }

    #[test]
    fn text_field_prefix_and_suffix_appear_correctly_with_no_hint_or_label() {
        test_widgets(|tester| {
            let second_key: KeyRef = UniqueKey::new().into();
            let root = overlay(
                tester,
                Column::new().children([
                    TextField::new()
                        .decoration(Some(InputDecoration::new().label_text("First")))
                        .into_widget(),
                    TextField::new()
                        .key(second_key.clone())
                        .decoration(Some(
                            InputDecoration::new()
                                .prefix_text("Prefix")
                                .suffix_text("Suffix"),
                        ))
                        .into_widget(),
                ]),
            );
            tester.pump_widget(root);
            expect(tester, &find::text("Prefix"), finds_one_widget());
            expect(tester, &find::text("Suffix"), finds_one_widget());

            tester.tap(find::by_key(second_key.clone()));
            tester.pump();
            expect(tester, &find::text("Prefix"), finds_one_widget());
            expect(tester, &find::text("Suffix"), finds_one_widget());

            tester.enter_text(find::by_key(second_key), "Hi");
            tester.pump();
            tester.pump_by(Duration::from_secs(1));
            expect(tester, &find::text("Prefix"), finds_one_widget());
            expect(tester, &find::text("Suffix"), finds_one_widget());
        });
    }

    #[test]
    fn text_field_prefix_and_suffix_appear_correctly_with_hint_text() {
        test_widgets(|tester| {
            let hint_style = TextStyle::new()
                .inherit(false)
                .color(Colors::PINK.shade500())
                .font_size(10.0);
            let second_key: KeyRef = UniqueKey::new().into();
            let root = overlay(
                tester,
                Column::new().children([
                    TextField::new()
                        .decoration(Some(InputDecoration::new().label_text("First")))
                        .into_widget(),
                    TextField::new()
                        .key(second_key.clone())
                        .decoration(Some(
                            InputDecoration::new()
                                .hint_text("Hint")
                                .hint_style(hint_style.clone())
                                .prefix_text("Prefix")
                                .suffix_text("Suffix"),
                        ))
                        .into_widget(),
                ]),
            );
            tester.pump_widget(root);

            assert_eq!(text_opacity(tester, "Prefix"), 0.0);
            assert_eq!(text_opacity(tester, "Suffix"), 0.0);
            assert_eq!(text_opacity(tester, "Hint"), 1.0);

            tester.tap(find::by_key(second_key.clone()));
            tester.pump_and_settle();
            assert_eq!(text_opacity(tester, "Prefix"), 1.0);
            assert_eq!(text_opacity(tester, "Suffix"), 1.0);
            assert_eq!(text_opacity(tester, "Hint"), 1.0);

            tester.enter_text(find::by_key(second_key), "Hi");
            tester.pump_and_settle();
            assert_eq!(text_opacity(tester, "Prefix"), 1.0);
            assert_eq!(text_opacity(tester, "Suffix"), 1.0);
            assert_eq!(text_opacity(tester, "Hint"), 0.0);

            let prefix = tester.widget::<Text, _>(find::text("Prefix"), |text| text.style.clone());
            assert_eq!(prefix, Some(hint_style.clone()));
            let suffix = tester.widget::<Text, _>(find::text("Suffix"), |text| text.style.clone());
            assert_eq!(suffix, Some(hint_style));
        });
    }

    #[test]
    fn text_field_prefix_and_suffix_appear_correctly_with_label_text() {
        test_widgets(|tester| {
            let prefix_style = TextStyle::new()
                .color(Colors::PINK.shade500())
                .font_size(10.0);
            let suffix_style = TextStyle::new()
                .color(Colors::GREEN.shade500())
                .font_size(12.0);
            let second_key: KeyRef = UniqueKey::new().into();
            let root = overlay(
                tester,
                Column::new().children([
                    TextField::new()
                        .decoration(Some(InputDecoration::new().label_text("First")))
                        .into_widget(),
                    TextField::new()
                        .key(second_key.clone())
                        .decoration(Some(
                            InputDecoration::new()
                                .label_text("Label")
                                .prefix_text("Prefix")
                                .prefix_style(prefix_style.clone())
                                .suffix_text("Suffix")
                                .suffix_style(suffix_style.clone()),
                        ))
                        .into_widget(),
                ]),
            );
            tester.pump_widget(root);

            assert_eq!(text_opacity(tester, "Prefix"), 0.0);
            assert_eq!(text_opacity(tester, "Suffix"), 0.0);
            expect(tester, &find::text("Label"), finds_one_widget());

            tester.tap(find::by_key(second_key.clone()));
            tester.pump_and_settle();
            assert_eq!(text_opacity(tester, "Prefix"), 1.0);
            assert_eq!(text_opacity(tester, "Suffix"), 1.0);
            expect(tester, &find::text("Label"), finds_one_widget());

            tester.enter_text(find::by_key(second_key), "Hi");
            tester.pump_and_settle();
            assert_eq!(text_opacity(tester, "Prefix"), 1.0);
            assert_eq!(text_opacity(tester, "Suffix"), 1.0);
            expect(tester, &find::text("Label"), finds_one_widget());

            let prefix = tester.widget::<Text, _>(find::text("Prefix"), |text| text.style.clone());
            assert_eq!(prefix, Some(prefix_style));
            let suffix = tester.widget::<Text, _>(find::text("Suffix"), |text| text.style.clone());
            assert_eq!(suffix, Some(suffix_style));
        });
    }

    #[test]
    fn text_field_label_text_animates() {
        test_widgets(|tester| {
            let second_key: KeyRef = UniqueKey::new().into();
            let root = overlay(
                tester,
                Column::new().children([
                    TextField::new()
                        .decoration(Some(InputDecoration::new().label_text("First")))
                        .into_widget(),
                    TextField::new()
                        .key(second_key.clone())
                        .decoration(Some(InputDecoration::new().label_text("Second")))
                        .into_widget(),
                ]),
            );
            tester.pump_widget(root);
            let mut position = tester.get_top_left(find::text("Second")).dy();

            tester.tap(find::by_key(second_key));
            tester.pump();
            tester.pump_by(Duration::from_millis(50));
            let new_position = tester.get_top_left(find::text("Second")).dy();
            assert!(new_position < position);

            tester.pump_by(Duration::from_millis(50));
            position = new_position;
            let new_position = tester.get_top_left(find::text("Second")).dy();
            assert!(new_position < position);
        });
    }

    #[test]
    fn icon_is_separated_from_input_label_by_16_12() {
        test_widgets(|tester| {
            let root = overlay(
                tester,
                TextField::new().decoration(Some(
                    InputDecoration::new()
                        .icon(waft_widgets::Icon::new(Some(Icons::phone())))
                        .label_text("label")
                        .filled(true),
                )),
            );
            tester.pump_widget(root);
            let icon_right = tester
                .get_top_right(find::by_type::<waft_widgets::Icon>())
                .dx();
            assert_eq!(
                icon_right + 16.0 + 16.0,
                tester.get_top_left(find::text("label")).dx()
            );
            assert_eq!(
                icon_right + 16.0 + 16.0,
                tester.get_top_left(find::by_type::<EditableText>()).dx()
            );
        });
    }

    #[test]
    fn controller_can_update_server() {
        test_widgets(|tester| {
            let controller1 = controller_with(tester, "Initial Text");
            let controller2 = controller_with(tester, "More Text");
            let current: Rc<RefCell<Option<TextEditingControllerRef>>> = Rc::default();
            let set_state: Rc<RefCell<Option<waft_widgets::StateSetter>>> = Rc::default();
            let root = overlay(tester, {
                let current = Rc::clone(&current);
                let set_state = Rc::clone(&set_state);
                waft_widgets::StatefulBuilder::new(move |_app, _context, setter| {
                    *set_state.borrow_mut() = Some(setter);
                    let mut field = TextField::new();
                    if let Some(controller) = current.borrow().clone() {
                        field = field.controller(controller);
                    }
                    field.into_widget()
                })
            });
            tester.pump_widget(root);
            assert!(tester.test_text_input().editing_state().is_none());

            tester.tap(find::by_type::<TextField>());
            tester.pump();
            assert_eq!(
                tester
                    .test_text_input()
                    .editing_state()
                    .expect("the field sent a state")
                    .text,
                ""
            );

            let setter = set_state.borrow().clone().expect("the builder ran");
            tester.update(|app| {
                setter(app, &mut |_app| {
                    *current.borrow_mut() = Some(controller1.clone().erase());
                })
            });
            tester.pump();
            assert_eq!(
                tester.test_text_input().editing_state().unwrap().text,
                "Initial Text"
            );

            controller1
                .handle()
                .set_text(&mut tester.cell.borrow_mut(), "Updated Text");
            tester.cell.checkpoint();
            assert_eq!(
                tester.test_text_input().editing_state().unwrap().text,
                "Updated Text"
            );

            let setter = set_state.borrow().clone().expect("the builder ran");
            tester.update(|app| {
                setter(app, &mut |_app| {
                    *current.borrow_mut() = Some(controller2.clone().erase());
                })
            });
            tester.pump();
            assert_eq!(
                tester.test_text_input().editing_state().unwrap().text,
                "More Text"
            );

            controller1
                .handle()
                .set_text(&mut tester.cell.borrow_mut(), "Ignored Text");
            tester.cell.checkpoint();
            assert_eq!(
                tester.test_text_input().editing_state().unwrap().text,
                "More Text"
            );

            controller2
                .handle()
                .set_text(&mut tester.cell.borrow_mut(), "Additional Text");
            tester.cell.checkpoint();
            assert_eq!(
                tester.test_text_input().editing_state().unwrap().text,
                "Additional Text"
            );

            controller2
                .handle()
                .set_selection(&mut tester.cell.borrow_mut(), TextSelection::new(0, 5));
            tester.cell.checkpoint();
            let state = tester.test_text_input().editing_state().unwrap();
            assert_eq!(state.selection.base_offset, 0);
            assert_eq!(state.selection.extent_offset, 5);

            controller2.handle().clear(&mut tester.cell.borrow_mut());
            tester.cell.checkpoint();
            assert_eq!(tester.test_text_input().editing_state().unwrap().text, "");

            controller2
                .handle()
                .set_text(&mut tester.cell.borrow_mut(), "The Final Cut");
            tester.cell.checkpoint();
            assert_eq!(
                tester.test_text_input().editing_state().unwrap().text,
                "The Final Cut"
            );
            let setter = set_state.borrow().clone().expect("the builder ran");
            tester.update(|app| setter(app, &mut |_app| *current.borrow_mut() = None));
            tester.pump();
            assert_eq!(
                tester.test_text_input().editing_state().unwrap().text,
                "The Final Cut"
            );

            controller2
                .handle()
                .set_text(&mut tester.cell.borrow_mut(), "Goodbye Cruel World");
            assert_eq!(
                tester.test_text_input().editing_state().unwrap().text,
                "The Final Cut"
            );
        });
    }

    #[test]
    fn multiline_text_field_scrolls_the_caret_into_view() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = overlay(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .max_lines(Some(6)),
            );
            tester.pump_widget(root);

            let tall_text = "a\nb\nc\nd\ne\nf\ng";
            tester.enter_text(find::by_type::<TextField>(), tall_text);
            skip_past_scrolling_animation(tester);

            let scrollable = |tester: &WidgetTester| {
                let element = find::by_type::<waft_widgets::Scrollable>().evaluate(tester)[0];
                element
                    .state_handle::<waft_widgets::ScrollableState>(&tester.cell.borrow())
                    .expect("a Scrollable state")
            };
            let pixels = |tester: &WidgetTester| {
                let position = scrollable(tester).position(&tester.cell.borrow());
                position.pixels(&tester.cell.borrow())
            };
            assert_eq!(pixels(tester), 0.0);

            tester.test_text_input().update_editing_value(
                TextEditingValue::new()
                    .text(tall_text)
                    .selection(TextSelection::collapsed(
                        tall_text.len() as i32,
                        TextAffinity::Downstream,
                    )),
            );
            tester.pump();
            skip_past_scrolling_animation(tester);

            let render = render_editable(tester);
            let line_height = {
                let mut app = tester.cell.borrow_mut();
                render.preferred_line_height(&mut app)
            };
            assert_more_or_less_equals_within(pixels(tester), line_height, 0.1);
        });
    }

    #[test]
    fn selection_is_consistent_with_text_length() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "abcde");
            controller.handle().set_selection(
                &mut tester.cell.borrow_mut(),
                TextSelection::collapsed(5, TextAffinity::Downstream),
            );
            controller
                .handle()
                .set_text(&mut tester.cell.borrow_mut(), "");
            let selection = controller.handle().selection(&tester.cell.borrow());
            assert!(selection.start() <= 0);
            assert!(selection.end() <= 0);

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                controller.handle().set_selection(
                    &mut tester.cell.borrow_mut(),
                    TextSelection::collapsed(10, TextAffinity::Downstream),
                );
            }));
            assert!(result.is_err(), "an out-of-range selection is rejected");
        });
    }

    #[test]
    fn clearing_text_field_with_suffix_icon_does_not_cause_text_selection_exception() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "Prefilled text.");
            let clearing = controller.clone();
            let root = boilerplate(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .decoration(Some(InputDecoration::new().suffix_icon(IconButton::new(
                        Some(Listener::new(move |app| clearing.handle().clear(app))),
                        waft_widgets::Icon::new(Some(Icons::close())),
                    )))),
            );
            tester.pump_widget(root);
            tester.tap(find::by_type::<IconButton>());
            assert_eq!(text(tester, controller.handle()), "");
        });
    }

    #[test]
    fn max_length_limits_input_in_the_center_of_a_maxed_out_field_with_collapsed_selection() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = boilerplate(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .max_length(10),
            );
            tester.pump_widget(root);
            let value = "0123456789";

            tester.show_keyboard(find::by_type::<TextField>());
            tester.test_text_input().update_editing_value(
                TextEditingValue::new()
                    .text(value)
                    .selection(TextSelection::collapsed(10, TextAffinity::Downstream)),
            );
            tester.pump();
            assert_eq!(text(tester, controller.handle()), value);

            tester.show_keyboard(find::by_type::<TextField>());
            tester.test_text_input().update_editing_value(
                TextEditingValue::new()
                    .text(&format!("{value}9999999"))
                    .selection(TextSelection::collapsed(17, TextAffinity::Downstream)),
            );
            tester.pump();
            assert_eq!(text(tester, controller.handle()), value);

            tester.show_keyboard(find::by_type::<TextField>());
            tester.test_text_input().update_editing_value(
                TextEditingValue::new()
                    .text("0123455555555556789")
                    .selection(TextSelection::collapsed(19, TextAffinity::Downstream)),
            );
            tester.pump();
            assert_eq!(text(tester, controller.handle()), value);
        });
    }

    #[test]
    fn max_length_limits_input_in_the_center_of_a_maxed_out_field_with_non_collapsed_selection() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = boilerplate(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .max_length(10)
                    .max_length_enforcement(MaxLengthEnforcement::Enforced),
            );
            tester.pump_widget(root);
            let value = "0123456789";

            tester.show_keyboard(find::by_type::<TextField>());
            tester.test_text_input().update_editing_value(
                TextEditingValue::new()
                    .text(value)
                    .selection(TextSelection::new(8, 10)),
            );
            tester.pump();
            assert_eq!(text(tester, controller.handle()), value);

            tester.show_keyboard(find::by_type::<TextField>());
            tester.test_text_input().update_editing_value(
                TextEditingValue::new()
                    .text("01234569999999")
                    .selection(TextSelection::collapsed(14, TextAffinity::Downstream)),
            );
            tester.pump();
            assert_eq!(text(tester, controller.handle()), "0123456999");
        });
    }

    #[test]
    fn cursor_blinks() {
        test_widgets(|tester| {
            let root = overlay(
                tester,
                TextField::new().decoration(Some(InputDecoration::new().hint_text("Placeholder"))),
            );
            tester.pump_widget(root);
            tester.show_keyboard(find::by_type::<TextField>());
            let state = editable_state(tester);

            let check_cursor_toggle = |tester: &mut WidgetTester| {
                let interval = EditableTextState::cursor_blink_interval();
                let initial = state.cursor_currently_visible(&tester.cell.borrow());
                tester.pump_by(interval);
                assert_eq!(
                    state.cursor_currently_visible(&tester.cell.borrow()),
                    !initial
                );
                tester.pump_by(interval);
                assert_eq!(
                    state.cursor_currently_visible(&tester.cell.borrow()),
                    initial
                );
                tester.pump_by(interval / 10);
                assert_eq!(
                    state.cursor_currently_visible(&tester.cell.borrow()),
                    initial
                );
                tester.pump_by(interval);
                assert_eq!(
                    state.cursor_currently_visible(&tester.cell.borrow()),
                    !initial
                );
                tester.pump_by(interval);
                assert_eq!(
                    state.cursor_currently_visible(&tester.cell.borrow()),
                    initial
                );
            };
            check_cursor_toggle(tester);
            tester.show_keyboard(find::by_type::<TextField>());

            // The same with a non-empty field.
            tester.test_text_input().update_editing_value(
                TextEditingValue::new()
                    .text("X")
                    .selection(TextSelection::collapsed(1, TextAffinity::Downstream)),
            );
            tester.cell.checkpoint();
            assert_eq!(editable_state(tester), state);
            check_cursor_toggle(tester);
        });
    }

    /// Dart's `TestFormatter`: records the values a formatter is asked about.
    #[derive(Clone)]
    struct RecordingFormatter(Rc<RefCell<Option<(TextEditingValue, TextEditingValue)>>>);

    impl waft_services::TextInputFormatter for RecordingFormatter {
        fn format_edit_update(
            &self,
            old_value: &TextEditingValue,
            new_value: &TextEditingValue,
        ) -> TextEditingValue {
            *self.0.borrow_mut() = Some((old_value.clone(), new_value.clone()));
            new_value.clone()
        }
    }

    #[test]
    fn text_input_formatter_gets_correct_selection_value() {
        test_widgets(|tester| {
            let recorded = Rc::new(RefCell::new(None));
            let controller = controller_with(tester, "123");
            let root = boilerplate(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .input_formatters(vec![TextInputFormatterRef(Rc::new(RecordingFormatter(
                        Rc::clone(&recorded),
                    )))]),
            );
            tester.pump_widget(root);
            tester.tap(find::by_type::<TextField>());
            tester.pump_and_settle();

            tester.send_key_event(
                PhysicalKeyboardKey::BACKSPACE,
                LogicalKeyboardKey::BACKSPACE,
            );
            tester.pump_and_settle();

            let (old, new) = recorded.borrow().clone().expect("the formatter ran");
            assert_eq!(old.text, "123");
            assert_eq!(
                old.selection,
                TextSelection::collapsed(3, TextAffinity::Upstream)
            );
            assert_eq!(new.text, "12");
            assert_eq!(
                new.selection,
                TextSelection::collapsed(2, TextAffinity::Downstream)
            );
        });
    }

    #[test]
    fn mobile_obscure_text_control_test() {
        test_widgets_variant(
            TargetPlatformVariant::only(TargetPlatform::Android),
            |tester| {
                let root = overlay(
                    tester,
                    TextField::new()
                        .obscure_text(true)
                        .decoration(Some(InputDecoration::new().hint_text("Placeholder"))),
                );
                tester.pump_widget(root);
                tester.show_keyboard(find::by_type::<TextField>());

                tester.test_text_input().update_editing_value(
                    TextEditingValue::new()
                        .text("ABC")
                        .selection(TextSelection::collapsed(3, TextAffinity::Downstream)),
                );
                tester.pump();

                tester.test_text_input().update_editing_value(
                    TextEditingValue::new()
                        .text("ABCX")
                        .selection(TextSelection::collapsed(4, TextAffinity::Downstream)),
                );
                tester.pump();

                let render = render_editable(tester);
                let text = {
                    let mut app = tester.cell.borrow_mut();
                    render.plain_text(&mut app)
                };
                assert!(text.ends_with('X'), "{text:?}");

                tester.pump_by(Duration::from_secs(2));
                let text = {
                    let mut app = tester.cell.borrow_mut();
                    render.plain_text(&mut app)
                };
                assert!(text.ends_with('\u{2022}'), "{text:?}");
            },
        );
    }

    #[test]
    fn desktop_obscure_text_control_test() {
        let variant = TargetPlatformVariant::new([
            TargetPlatform::MacOS,
            TargetPlatform::Linux,
            TargetPlatform::Windows,
        ]);
        test_widgets_variant(variant, |tester| {
            let root = overlay(
                tester,
                TextField::new()
                    .obscure_text(true)
                    .decoration(Some(InputDecoration::new().hint_text("Placeholder"))),
            );
            tester.pump_widget(root);
            tester.show_keyboard(find::by_type::<TextField>());

            tester.test_text_input().update_editing_value(
                TextEditingValue::new()
                    .text("ABC")
                    .selection(TextSelection::collapsed(3, TextAffinity::Downstream)),
            );
            tester.pump();

            tester.test_text_input().update_editing_value(
                TextEditingValue::new()
                    .text("ABCX")
                    .selection(TextSelection::collapsed(4, TextAffinity::Downstream)),
            );
            tester.pump();

            let render = render_editable(tester);
            let text = {
                let mut app = tester.cell.borrow_mut();
                render.plain_text(&mut app)
            };
            assert!(text.ends_with('\u{2022}'), "{text:?}");
        });
    }

    /// A tester on `platform` whose clipboard is `clipboard`.
    fn tester_with_clipboard(platform: TargetPlatform) -> (WidgetTester, MockClipboard) {
        let clipboard = MockClipboard::new();
        let tester = WidgetTester::with_platform(
            TestPlatform::new()
                .on(platform)
                .with_clipboard(Rc::new(clipboard.clone())),
        );
        (tester, clipboard)
    }

    /// Pumps `field` over a `Material` on a `MaterialApp`, as the keyboard cases do.
    /// Dart's keyboard cases: enter `text`, then tap the field, after a frame first when
    /// `pump_before_tap` (Dart's own cases differ in whether they pump before the tap).
    fn keyboard_case(
        tester: &mut WidgetTester,
        field: TextField,
        text: &str,
        pump_before_tap: bool,
    ) {
        let root = overlay(tester, field);
        tester.pump_widget(root);
        tester.enter_text(find::by_type::<TextField>(), text);
        tester.cell.checkpoint();
        if pump_before_tap {
            tester.pump();
        }
        tester.tap(find::by_type::<TextField>());
        tester.pump_and_settle();
    }

    fn selection_delta(tester: &WidgetTester, controller: Handle<TextEditingController>) -> i32 {
        let selection = selection(tester, controller);
        selection.extent_offset - selection.base_offset
    }

    #[test]
    fn keyboard_shift_test_1() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            keyboard_case(
                tester,
                TextField::new().controller(controller.erase()),
                "a big house",
                true,
            );
            tester.send_key_down_event(
                PhysicalKeyboardKey::SHIFT_LEFT,
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            tester.send_key_event(
                PhysicalKeyboardKey::ARROW_LEFT,
                LogicalKeyboardKey::ARROW_LEFT,
            );
            tester.send_key_up_event(
                PhysicalKeyboardKey::SHIFT_LEFT,
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            assert_eq!(selection_delta(tester, controller.handle()), -1);
        });
    }

    #[test]
    fn keyboard_shift_test_2() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = overlay(tester, TextField::new().controller(controller.erase()));
            tester.pump_widget(root);
            tester.show_keyboard(find::by_type::<TextField>());
            tester.test_text_input().update_editing_value(
                TextEditingValue::new()
                    .text("abcdefghi")
                    .selection(TextSelection::collapsed(3, TextAffinity::Downstream))
                    .composing(TextRange::new(0, 9)),
            );
            tester.pump();

            tester.send_key_down_event(
                PhysicalKeyboardKey::SHIFT_LEFT,
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            tester.send_key_down_event(
                PhysicalKeyboardKey::ARROW_RIGHT,
                LogicalKeyboardKey::ARROW_RIGHT,
            );
            tester.pump_and_settle();
            assert_eq!(selection_delta(tester, controller.handle()), 1);
        });
    }

    #[test]
    fn keyboard_control_shift_test() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            keyboard_case(
                tester,
                TextField::new().controller(controller.erase()),
                "their big house",
                false,
            );

            tester.send_key_down_event(
                PhysicalKeyboardKey::CONTROL_LEFT,
                LogicalKeyboardKey::CONTROL_LEFT,
            );
            tester.send_key_down_event(
                PhysicalKeyboardKey::SHIFT_LEFT,
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            tester.send_key_down_event(
                PhysicalKeyboardKey::ARROW_RIGHT,
                LogicalKeyboardKey::ARROW_RIGHT,
            );
            tester.pump_and_settle();
            assert_eq!(selection_delta(tester, controller.handle()), 5);
        });
    }

    #[test]
    fn keyboard_down_and_up_test() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            keyboard_case(
                tester,
                TextField::new().controller(controller.erase()),
                "a big house",
                true,
            );

            tester.send_key_down_event(
                PhysicalKeyboardKey::SHIFT_LEFT,
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            tester.send_key_down_event(PhysicalKeyboardKey::ARROW_UP, LogicalKeyboardKey::ARROW_UP);
            tester.pump_and_settle();
            assert_eq!(selection_delta(tester, controller.handle()), -11);

            tester.send_key_up_event(PhysicalKeyboardKey::ARROW_UP, LogicalKeyboardKey::ARROW_UP);
            tester.send_key_up_event(
                PhysicalKeyboardKey::SHIFT_LEFT,
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            tester.pump_and_settle();
            tester.send_key_down_event(
                PhysicalKeyboardKey::SHIFT_LEFT,
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            tester.send_key_down_event(
                PhysicalKeyboardKey::ARROW_DOWN,
                LogicalKeyboardKey::ARROW_DOWN,
            );
            tester.pump_and_settle();
            assert_eq!(selection_delta(tester, controller.handle()), 0);
        });
    }

    #[test]
    fn read_only_keyboard_selection_test() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "readonly");
            let root = overlay(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .read_only(true),
            );
            tester.pump_widget(root);
            tester.tap(find::by_type::<TextField>());
            tester.pump_and_settle();

            tester.send_key_down_event(
                PhysicalKeyboardKey::SHIFT_LEFT,
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            tester.send_key_down_event(
                PhysicalKeyboardKey::ARROW_LEFT,
                LogicalKeyboardKey::ARROW_LEFT,
            );
            assert_eq!(selection_delta(tester, controller.handle()), -1);
        });
    }

    #[test]
    fn copy_paste_test() {
        test_widgets(|_tester| {
            let (mut tester, clipboard) = tester_with_clipboard(TargetPlatform::Android);
            let controller = controller_with(&tester, "");
            keyboard_case(
                &mut tester,
                TextField::new()
                    .controller(controller.erase())
                    .max_lines(Some(3)),
                "a big house\njumped over a mouse",
                false,
            );
            controller.handle().set_selection(
                &mut tester.cell.borrow_mut(),
                TextSelection::collapsed(0, TextAffinity::Downstream),
            );
            tester.pump();

            tester.send_key_down_event(
                PhysicalKeyboardKey::SHIFT_LEFT,
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            for _ in 0..5 {
                tester.send_key_event(
                    PhysicalKeyboardKey::ARROW_RIGHT,
                    LogicalKeyboardKey::ARROW_RIGHT,
                );
                tester.pump_and_settle();
            }
            tester.send_key_up_event(
                PhysicalKeyboardKey::SHIFT_LEFT,
                LogicalKeyboardKey::SHIFT_LEFT,
            );

            tester.send_key_down_event(
                PhysicalKeyboardKey::CONTROL_RIGHT,
                LogicalKeyboardKey::CONTROL_RIGHT,
            );
            tester.send_key_event(PhysicalKeyboardKey::KEY_C, LogicalKeyboardKey::KEY_C);
            tester.pump_and_settle();
            tester.send_key_up_event(
                PhysicalKeyboardKey::CONTROL_RIGHT,
                LogicalKeyboardKey::CONTROL_RIGHT,
            );
            tester.pump_and_settle();
            assert_eq!(clipboard.clipboard_data().as_deref(), Some("a big"));

            tester.send_key_event(
                PhysicalKeyboardKey::ARROW_RIGHT,
                LogicalKeyboardKey::ARROW_RIGHT,
            );
            tester.pump_and_settle();

            tester.send_key_down_event(
                PhysicalKeyboardKey::CONTROL_RIGHT,
                LogicalKeyboardKey::CONTROL_RIGHT,
            );
            tester.send_key_down_event(PhysicalKeyboardKey::KEY_V, LogicalKeyboardKey::KEY_V);
            tester.pump_and_settle();
            tester.pump_by(Duration::from_millis(200));
            tester.send_key_up_event(PhysicalKeyboardKey::KEY_V, LogicalKeyboardKey::KEY_V);
            tester.send_key_up_event(
                PhysicalKeyboardKey::CONTROL_RIGHT,
                LogicalKeyboardKey::CONTROL_RIGHT,
            );
            tester.pump_and_settle();

            expect(
                &tester,
                &find::text("a biga big house\njumped over a mouse"),
                finds_one_widget(),
            );
        });
    }

    #[test]
    fn paste_does_not_crash_after_calling_text_controller_text_setter() {
        test_widgets(|_tester| {
            let (mut tester, clipboard) = tester_with_clipboard(TargetPlatform::Android);
            clipboard.set_clipboard_data(Some("I love Flutter!".to_owned()));
            let controller = controller_with(&tester, "");
            let theme = theme(&tester);
            let root = material_home(
                theme,
                TextField::new()
                    .controller(controller.clone().erase())
                    .obscure_text(true),
            );
            tester.pump_widget(root);
            tester.tap(find::by_type::<TextField>());
            tester.pump_and_settle();

            controller
                .handle()
                .set_text(&mut tester.cell.borrow_mut(), "");

            tester.send_key_down_event(
                PhysicalKeyboardKey::CONTROL_RIGHT,
                LogicalKeyboardKey::CONTROL_RIGHT,
            );
            tester.send_key_down_event(PhysicalKeyboardKey::KEY_V, LogicalKeyboardKey::KEY_V);
            tester.pump_and_settle();
            tester.pump_by(Duration::from_millis(200));
            tester.send_key_up_event(PhysicalKeyboardKey::KEY_V, LogicalKeyboardKey::KEY_V);
            tester.send_key_up_event(
                PhysicalKeyboardKey::CONTROL_RIGHT,
                LogicalKeyboardKey::CONTROL_RIGHT,
            );
            tester.pump_and_settle();

            expect(&tester, &find::text("I love Flutter!"), finds_one_widget());
        });
    }

    #[test]
    fn cut_test() {
        test_widgets(|_tester| {
            let (mut tester, clipboard) = tester_with_clipboard(TargetPlatform::Android);
            let controller = controller_with(&tester, "");
            keyboard_case(
                &mut tester,
                TextField::new()
                    .controller(controller.erase())
                    .max_lines(Some(3)),
                "a big house\njumped over a mouse",
                false,
            );
            controller.handle().set_selection(
                &mut tester.cell.borrow_mut(),
                TextSelection::collapsed(0, TextAffinity::Downstream),
            );
            tester.pump();

            for _ in 0..5 {
                tester.send_key_down_event(
                    PhysicalKeyboardKey::SHIFT_LEFT,
                    LogicalKeyboardKey::SHIFT_LEFT,
                );
                tester.send_key_event(
                    PhysicalKeyboardKey::ARROW_RIGHT,
                    LogicalKeyboardKey::ARROW_RIGHT,
                );
                tester.pump_and_settle();
                tester.send_key_up_event(
                    PhysicalKeyboardKey::SHIFT_LEFT,
                    LogicalKeyboardKey::SHIFT_LEFT,
                );
                tester.pump_and_settle();
            }

            tester.send_key_down_event(
                PhysicalKeyboardKey::CONTROL_RIGHT,
                LogicalKeyboardKey::CONTROL_RIGHT,
            );
            tester.send_key_event(PhysicalKeyboardKey::KEY_X, LogicalKeyboardKey::KEY_X);
            tester.pump_and_settle();
            tester.send_key_up_event(
                PhysicalKeyboardKey::CONTROL_RIGHT,
                LogicalKeyboardKey::CONTROL_RIGHT,
            );
            tester.pump_and_settle();
            assert_eq!(clipboard.clipboard_data().as_deref(), Some("a big"));

            for _ in 0..5 {
                tester.send_key_event(
                    PhysicalKeyboardKey::ARROW_RIGHT,
                    LogicalKeyboardKey::ARROW_RIGHT,
                );
                tester.pump_and_settle();
            }

            tester.send_key_down_event(
                PhysicalKeyboardKey::CONTROL_RIGHT,
                LogicalKeyboardKey::CONTROL_RIGHT,
            );
            tester.send_key_down_event(PhysicalKeyboardKey::KEY_V, LogicalKeyboardKey::KEY_V);
            tester.pump_and_settle();
            tester.pump_by(Duration::from_millis(200));
            tester.send_key_up_event(PhysicalKeyboardKey::KEY_V, LogicalKeyboardKey::KEY_V);
            tester.send_key_up_event(
                PhysicalKeyboardKey::CONTROL_RIGHT,
                LogicalKeyboardKey::CONTROL_RIGHT,
            );
            tester.pump_and_settle();

            expect(
                &tester,
                &find::text(" housa bige\njumped over a mouse"),
                finds_one_widget(),
            );
        });
    }

    #[test]
    fn select_all_test() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            keyboard_case(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .max_lines(Some(3)),
                "a big house\njumped over a mouse",
                false,
            );

            tester.send_key_down_event(
                PhysicalKeyboardKey::CONTROL_LEFT,
                LogicalKeyboardKey::CONTROL_LEFT,
            );
            tester.send_key_event(PhysicalKeyboardKey::KEY_A, LogicalKeyboardKey::KEY_A);
            tester.pump_and_settle();
            tester.send_key_up_event(
                PhysicalKeyboardKey::CONTROL_LEFT,
                LogicalKeyboardKey::CONTROL_LEFT,
            );
            tester.pump_and_settle();

            tester.send_key_down_event(PhysicalKeyboardKey::DELETE, LogicalKeyboardKey::DELETE);
            tester.pump_and_settle();
            tester.pump_by(Duration::from_millis(200));
            tester.send_key_up_event(PhysicalKeyboardKey::DELETE, LogicalKeyboardKey::DELETE);
            tester.pump_and_settle();

            expect(tester, &find::text(""), finds_one_widget());
        });
    }

    #[test]
    fn delete_test() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            keyboard_case(
                tester,
                TextField::new()
                    .controller(controller.erase())
                    .max_lines(Some(3)),
                "a big house\njumped over a mouse",
                false,
            );
            controller.handle().set_selection(
                &mut tester.cell.borrow_mut(),
                TextSelection::collapsed(0, TextAffinity::Downstream),
            );
            tester.pump();

            for _ in 0..6 {
                tester.send_key_event(PhysicalKeyboardKey::DELETE, LogicalKeyboardKey::DELETE);
                tester.pump_and_settle();
            }
            expect(
                tester,
                &find::text("house\njumped over a mouse"),
                finds_one_widget(),
            );

            tester.send_key_down_event(
                PhysicalKeyboardKey::CONTROL_LEFT,
                LogicalKeyboardKey::CONTROL_LEFT,
            );
            tester.send_key_event(PhysicalKeyboardKey::KEY_A, LogicalKeyboardKey::KEY_A);
            tester.pump_and_settle();
            tester.send_key_up_event(
                PhysicalKeyboardKey::CONTROL_LEFT,
                LogicalKeyboardKey::CONTROL_LEFT,
            );
            tester.pump_and_settle();

            tester.send_key_event(PhysicalKeyboardKey::DELETE, LogicalKeyboardKey::DELETE);
            tester.pump_and_settle();
            expect(tester, &find::text(""), finds_one_widget());
        });
    }

    fn set_read_only(
        tester: &mut WidgetTester,
        setter: &Rc<RefCell<Option<waft_widgets::StateSetter>>>,
        flag: &Rc<Cell<bool>>,
        read_only: bool,
    ) {
        flag.set(read_only);
        let setter = setter.borrow().clone().expect("the builder ran");
        tester.update(|app| setter(app, &mut |_app| {}));
        tester.pump();
    }

    #[test]
    fn dynamically_switching_to_read_only_should_close_input_connection() {
        test_widgets(|tester| {
            let read_only = Rc::new(Cell::new(false));
            let setter: Rc<RefCell<Option<waft_widgets::StateSetter>>> = Rc::default();
            let controller = controller_with(tester, "readonly");
            let (flag, stored) = (Rc::clone(&read_only), Rc::clone(&setter));
            let root = overlay(
                tester,
                waft_widgets::StatefulBuilder::new(move |_app, _context, setter| {
                    *stored.borrow_mut() = Some(setter);
                    TextField::new()
                        .controller(controller.clone().erase())
                        .read_only(flag.get())
                        .into_widget()
                }),
            );
            tester.pump_widget(root);
            tester.tap(find::by_type::<TextField>());
            tester.pump();
            assert!(tester.test_text_input().has_any_clients());

            set_read_only(tester, &setter, &read_only, true);
            assert!(!tester.test_text_input().has_any_clients());
        });
    }

    #[test]
    fn dynamically_switching_to_non_read_only_should_open_input_connection() {
        test_widgets(|tester| {
            let read_only = Rc::new(Cell::new(true));
            let setter: Rc<RefCell<Option<waft_widgets::StateSetter>>> = Rc::default();
            let controller = controller_with(tester, "readonly");
            let (flag, stored) = (Rc::clone(&read_only), Rc::clone(&setter));
            let root = overlay(
                tester,
                waft_widgets::StatefulBuilder::new(move |_app, _context, setter| {
                    *stored.borrow_mut() = Some(setter);
                    TextField::new()
                        .controller(controller.clone().erase())
                        .read_only(flag.get())
                        .into_widget()
                }),
            );
            tester.pump_widget(root);
            tester.tap(find::by_type::<TextField>());
            tester.pump();
            assert!(!tester.test_text_input().has_any_clients());

            set_read_only(tester, &setter, &read_only, false);
            assert!(tester.test_text_input().has_any_clients());
        });
    }

    #[test]
    fn dynamically_switching_between_read_only_and_not_should_hide_or_show_collapse_cursor() {
        test_widgets(|tester| {
            let read_only = Rc::new(Cell::new(true));
            let setter: Rc<RefCell<Option<waft_widgets::StateSetter>>> = Rc::default();
            let controller = controller_with(tester, "readonly");
            let (flag, stored) = (Rc::clone(&read_only), Rc::clone(&setter));
            let root = overlay(
                tester,
                waft_widgets::StatefulBuilder::new(move |_app, _context, setter| {
                    *stored.borrow_mut() = Some(setter);
                    TextField::new()
                        .controller(controller.clone().erase())
                        .read_only(flag.get())
                        .into_widget()
                }),
            );
            tester.pump_widget(root);
            tester.tap(find::by_type::<TextField>());
            tester.pump();
            assert!(!selection_overlay_visibility(tester).1);

            set_read_only(tester, &setter, &read_only, false);
            tester.pump_and_settle();
            assert!(selection_overlay_visibility(tester).0);

            set_read_only(tester, &setter, &read_only, true);
            tester.pump_and_settle();
            assert!(!selection_overlay_visibility(tester).0);
        });
    }

    #[test]
    fn swapping_controllers_should_update_selection() {
        test_widgets(|tester| {
            let first = controller_with(tester, "readonly");
            let current: Rc<RefCell<TextEditingControllerRef>> =
                Rc::new(RefCell::new(first.clone().erase()));
            let setter: Rc<RefCell<Option<waft_widgets::StateSetter>>> = Rc::default();
            let (stored, value) = (Rc::clone(&setter), Rc::clone(&current));
            let root = overlay(
                tester,
                waft_widgets::StatefulBuilder::new(move |_app, _context, setter| {
                    *stored.borrow_mut() = Some(setter);
                    TextField::new()
                        .controller(value.borrow().clone())
                        .read_only(true)
                        .into_widget()
                }),
            );
            tester.pump_widget(root);
            let at = text_offset_to_position(tester, 3);
            tester.long_press_at(at, 7);
            tester.pump_and_settle();
            let state = editable_state(tester);
            let selection_now = |tester: &WidgetTester| {
                state
                    .selection_overlay(&tester.cell.borrow())
                    .expect("a selection overlay")
                    .value(&tester.cell.borrow())
                    .selection
            };
            assert_eq!(selection_now(tester).base_offset, 0);
            assert_eq!(selection_now(tester).extent_offset, 8);

            let base = {
                let app = tester.cell.borrow();
                first.value(&app).clone()
            };
            let second = {
                let mut app = tester.cell.borrow_mut();
                TextEditingController::from_value(
                    &mut app,
                    Some(base.copy_with().selection(TextSelection::new(1, 7))),
                )
            };
            *current.borrow_mut() = second.erase();
            let setter = setter.borrow().clone().expect("the builder ran");
            tester.update(|app| setter(app, &mut |_app| {}));
            tester.pump();
            assert_eq!(selection_now(tester).base_offset, 1);
            assert_eq!(selection_now(tester).extent_offset, 7);
        });
    }

    /// Drags `pointer` from `from` to `to`, with a frame after each step.
    fn mouse_drag(tester: &mut WidgetTester, pointer: &mut TestPointer, from: Offset, to: Offset) {
        tester.down(pointer, from);
        tester.pump();
        tester.move_to(pointer, to, true);
        tester.pump();
        tester.up(pointer);
        tester.pump_and_settle();
    }

    #[test]
    fn can_select_text_by_dragging_with_a_mouse() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = material_home(
                theme(tester),
                TextField::new()
                    .drag_start_behavior(DragStartBehavior::Down)
                    .controller(controller.erase()),
            );
            tester.pump_widget(root);
            tester.enter_text(find::by_type::<TextField>(), "abc def ghi");
            skip_past_scrolling_animation(tester);
            let text = "abc def ghi";
            let e = text_offset_to_position(tester, text.find('e').unwrap() as i32);
            let g = text_offset_to_position(tester, text.find('g').unwrap() as i32);
            let mut pointer = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            mouse_drag(tester, &mut pointer, e, g);
            let selection_now = selection(tester, controller.handle());
            assert_eq!(selection_now.base_offset, text.find('e').unwrap() as i32);
            assert_eq!(selection_now.extent_offset, text.find('g').unwrap() as i32);
        });
    }

    #[test]
    fn dragging_in_opposite_direction_also_works() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = material_home(
                theme(tester),
                TextField::new()
                    .drag_start_behavior(DragStartBehavior::Down)
                    .controller(controller.erase()),
            );
            tester.pump_widget(root);
            tester.enter_text(find::by_type::<TextField>(), "abc def ghi");
            skip_past_scrolling_animation(tester);
            let text = "abc def ghi";
            let e = text_offset_to_position(tester, text.find('e').unwrap() as i32);
            let g = text_offset_to_position(tester, text.find('g').unwrap() as i32);
            let mut pointer = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            mouse_drag(tester, &mut pointer, g, e);
            let selection_now = selection(tester, controller.handle());
            assert_eq!(selection_now.base_offset, text.find('g').unwrap() as i32);
            assert_eq!(selection_now.extent_offset, text.find('e').unwrap() as i32);
        });
    }

    #[test]
    fn slow_mouse_dragging_also_selects_text() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = material_home(
                theme(tester),
                TextField::new()
                    .drag_start_behavior(DragStartBehavior::Down)
                    .controller(controller.erase()),
            );
            tester.pump_widget(root);
            tester.enter_text(find::by_type::<TextField>(), "abc def ghi");
            skip_past_scrolling_animation(tester);
            let text = "abc def ghi";
            let e = text_offset_to_position(tester, text.find('e').unwrap() as i32);
            let g = text_offset_to_position(tester, text.find('g').unwrap() as i32);
            let mut pointer = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.down(&mut pointer, e);
            tester.pump_by(Duration::from_secs(2));
            tester.move_to(&mut pointer, g, true);
            tester.pump();
            tester.up(&pointer);
            let selection_now = selection(tester, controller.handle());
            assert_eq!(selection_now.base_offset, text.find('e').unwrap() as i32);
            assert_eq!(selection_now.extent_offset, text.find('g').unwrap() as i32);
        });
    }

    #[test]
    fn continuous_dragging_does_not_cause_flickering() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "abc def ghi");
            let changed = Rc::new(Cell::new(0));
            let count = Rc::clone(&changed);
            controller.handle().add_listener(
                &mut tester.cell.borrow_mut(),
                Listener::new(move |_app| count.set(count.get() + 1)),
            );
            let root = material_home(
                theme(tester),
                TextField::new()
                    .drag_start_behavior(DragStartBehavior::Down)
                    .controller(controller.erase())
                    .style(TextStyle::new().font_size(10.0)),
            );
            tester.pump_widget(root);

            let c = text_offset_to_position(tester, 2);
            let g = text_offset_to_position(tester, 8);
            let h = text_offset_to_position(tester, 9);
            let mut pointer = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.down(&mut pointer, c);
            tester.pump();
            tester.move_to(&mut pointer, g, true);
            tester.pump_and_settle();
            assert!(changed.get() > 0);
            changed.set(0);
            let selection_now = selection(tester, controller.handle());
            assert_eq!(selection_now.base_offset, 2);
            assert_eq!(selection_now.extent_offset, 8);

            tester.move_to(&mut pointer, g + Offset::new(2.0, 0.0), true);
            tester.pump_and_settle();
            assert_eq!(changed.get(), 0);

            tester.move_to(&mut pointer, h, true);
            tester.pump();
            tester.up(&pointer);
            tester.pump_and_settle();
            assert_eq!(changed.get(), 1);
            let selection_now = selection(tester, controller.handle());
            assert_eq!(selection_now.base_offset, 2);
            assert_eq!(selection_now.extent_offset, 9);
        });
    }

    #[test]
    fn can_move_cursor_when_dragging_android() {
        for platform in [TargetPlatform::Android, TargetPlatform::Fuchsia] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let controller = controller_with(tester, "");
                let root = material_home(
                    theme(tester),
                    TextField::new()
                        .drag_start_behavior(DragStartBehavior::Down)
                        .controller(controller.erase()),
                );
                tester.pump_widget(root);
                tester.enter_text(find::by_type::<TextField>(), "abc def ghi");
                skip_past_scrolling_animation(tester);
                let text = "abc def ghi";
                let e = text_offset_to_position(tester, text.find('e').unwrap() as i32);
                let d = text_offset_to_position(tester, text.find('d').unwrap() as i32);
                let g = text_offset_to_position(tester, text.find('g').unwrap() as i32);

                let mut pointer = TestPointer::touch(1);
                tester.down(&mut pointer, e);
                tester.pump();
                tester.up(&pointer);
                tester.pump_and_settle_by(waft_gestures::K_DOUBLE_TAP_TIMEOUT);
                assert!(selection(tester, controller.handle()).is_collapsed());
                assert_eq!(
                    selection(tester, controller.handle()).base_offset,
                    text.find('e').unwrap() as i32
                );

                tester.down(&mut pointer, d);
                tester.pump();
                tester.move_to(&mut pointer, g, true);
                tester.pump_and_settle();
                assert!(selection(tester, controller.handle()).is_collapsed());
                assert_eq!(
                    selection(tester, controller.handle()).base_offset,
                    text.find('g').unwrap() as i32
                );
            });
        }
    }

    #[test]
    fn can_move_cursor_when_dragging_android_multiline() {
        for platform in [TargetPlatform::Android, TargetPlatform::Fuchsia] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let controller = controller_with(tester, "");
                let root = material_home(
                    theme(tester),
                    TextField::new()
                        .drag_start_behavior(DragStartBehavior::Down)
                        .controller(controller.erase())
                        .max_lines(None),
                );
                tester.pump_widget(root);
                tester.enter_text(find::by_type::<TextField>(), "abc\ndef\nghi");
                skip_past_scrolling_animation(tester);
                let text = "abc\ndef\nghi";
                let a = text_offset_to_position(tester, 0);
                let c = text_offset_to_position(tester, 2);
                let g = text_offset_to_position(tester, text.find('g').unwrap() as i32);

                let mut pointer = TestPointer::touch(1);
                tester.down(&mut pointer, a);
                tester.pump();
                tester.up(&pointer);
                tester.pump_and_settle_by(waft_gestures::K_DOUBLE_TAP_TIMEOUT);
                assert_eq!(selection(tester, controller.handle()).base_offset, 0);

                tester.down(&mut pointer, c);
                tester.pump();
                tester.move_to(&mut pointer, g, true);
                tester.pump_and_settle();
                assert!(selection(tester, controller.handle()).is_collapsed());
                assert_eq!(
                    selection(tester, controller.handle()).base_offset,
                    text.find('g').unwrap() as i32
                );
            });
        }
    }

    #[test]
    fn can_double_click_and_drag_with_a_mouse_to_select_word_by_word() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = material_home(
                theme(tester),
                TextField::new()
                    .drag_start_behavior(DragStartBehavior::Down)
                    .controller(controller.erase()),
            );
            tester.pump_widget(root);
            tester.enter_text(find::by_type::<TextField>(), "abc def ghi");
            skip_past_scrolling_animation(tester);
            let text = "abc def ghi";
            let e = text_offset_to_position(tester, text.find('e').unwrap() as i32);
            let h = text_offset_to_position(tester, text.find('h').unwrap() as i32);
            let mut pointer = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.down(&mut pointer, e);
            tester.pump();
            tester.up(&pointer);
            tester.pump();
            assert!(selection(tester, controller.handle()).is_collapsed());
            assert_eq!(
                selection(tester, controller.handle()).base_offset,
                text.find('e').unwrap() as i32
            );

            tester.down(&mut pointer, e);
            tester.pump();
            assert_eq!(selection(tester, controller.handle()).base_offset, 4);
            assert_eq!(selection(tester, controller.handle()).extent_offset, 7);

            tester.move_to(&mut pointer, h, true);
            tester.pump_and_settle();
            assert_eq!(
                selection(tester, controller.handle()).base_offset,
                text.find('d').unwrap() as i32
            );
            assert_eq!(
                selection(tester, controller.handle()).extent_offset,
                text.find('i').unwrap() as i32 + 1
            );
        });
    }

    #[test]
    fn tap_moves_cursor_to_the_edge_of_the_word_it_tapped() {
        test_widgets_variant(TargetPlatformVariant::only(TargetPlatform::IOS), |tester| {
            let controller = controller_with(tester, "");
            let root = material_home(
                theme(tester),
                Center::new().child(TextField::new().controller(controller.erase())),
            );
            tester.pump_widget(root);
            controller.handle().set_text(
                &mut tester.cell.borrow_mut(),
                "Atwater Peel Sherbrooke Bonaventure",
            );
            tester.pump();

            let at = text_offset_to_position(tester, 3);
            tester.tap_at(at, 1);
            tester.pump();
            let selection_now = selection(tester, controller.handle());
            assert!(selection_now.is_collapsed());
            assert_eq!(selection_now.base_offset, 7);
            assert_eq!(selection_now.affinity, TextAffinity::Upstream);
            assert!(!selection_overlay_visibility(tester).1);
        });
    }

    #[test]
    fn tap_with_a_mouse_does_not_move_cursor_to_the_edge_of_the_word() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let controller = controller_with(tester, "");
                let root = material_home(
                    theme(tester),
                    Center::new().child(TextField::new().controller(controller.erase())),
                );
                tester.pump_widget(root);
                controller.handle().set_text(
                    &mut tester.cell.borrow_mut(),
                    "Atwater Peel Sherbrooke Bonaventure",
                );
                tester.pump();

                let at = text_offset_to_position(tester, 3);
                let mut pointer = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
                tester.down(&mut pointer, at);
                tester.up(&pointer);
                let selection_now = selection(tester, controller.handle());
                assert!(selection_now.is_collapsed());
                assert_eq!(selection_now.base_offset, 3, "{platform:?}");
            });
        }
    }

    #[test]
    fn tap_moves_cursor_to_the_position_tapped() {
        for platform in [
            TargetPlatform::Android,
            TargetPlatform::Fuchsia,
            TargetPlatform::Linux,
            TargetPlatform::Windows,
        ] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let controller = controller_with(tester, "");
                let root = material_home(
                    theme(tester),
                    Center::new().child(TextField::new().controller(controller.erase())),
                );
                tester.pump_widget(root);
                controller.handle().set_text(
                    &mut tester.cell.borrow_mut(),
                    "Atwater Peel Sherbrooke Bonaventure",
                );
                tester.pump();

                let at = text_offset_to_position(tester, 3);
                tester.tap_at(at, 1);
                tester.pump();
                let selection_now = selection(tester, controller.handle());
                assert!(selection_now.is_collapsed());
                assert_eq!(selection_now.base_offset, 3, "{platform:?}");
                assert!(!selection_overlay_visibility(tester).1, "{platform:?}");
            });
        }
    }

    #[test]
    fn ios_select_word_edge_works_correctly() {
        test_widgets_variant(TargetPlatformVariant::only(TargetPlatform::IOS), |tester| {
            let controller = controller_with(tester, "blah1 blah2");
            let root = material_home(
                theme(tester),
                TextField::new().controller(controller.erase()),
            );
            tester.pump_widget(root);
            assert_eq!(
                selection(tester, controller.handle()),
                TextSelection::new(-1, -1)
            );

            let pos1 = text_offset_to_position(tester, 1);
            let mut gesture = TestPointer::touch(1);
            tester.down(&mut gesture, pos1);
            tester.pump();
            tester.up(&gesture);
            tester.pump_and_settle();
            assert_eq!(
                selection(tester, controller.handle()),
                TextSelection::collapsed(5, TextAffinity::Upstream)
            );

            let pos0 = text_offset_to_position(tester, 0);
            gesture = TestPointer::touch(2);
            tester.down(&mut gesture, pos0);
            tester.pump();
            tester.up(&gesture);
            tester.pump_and_settle();
            assert_eq!(
                selection(tester, controller.handle()),
                TextSelection::collapsed(0, TextAffinity::Downstream)
            );
        });
    }

    #[test]
    fn double_tap_does_not_select_word_on_read_only_obscured_field() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let controller = controller_with(tester, "");
                let root = material_home(
                    theme(tester),
                    Center::new().child(
                        TextField::new()
                            .obscure_text(true)
                            .read_only(true)
                            .controller(controller.erase()),
                    ),
                );
                tester.pump_widget(root);
                controller.handle().set_text(
                    &mut tester.cell.borrow_mut(),
                    "Atwater Peel Sherbrooke Bonaventure",
                );
                tester.pump();

                let at = text_offset_to_position(tester, 10);
                tester.tap_at(at, 1);
                tester.pump_by(Duration::from_millis(500));
                let first = selection(tester, controller.handle());
                assert!(first.is_collapsed(), "{platform:?}");

                tester.tap_at(at, 2);
                tester.pump_by(Duration::from_millis(50));
                tester.tap_at(at, 3);
                tester.pump_and_settle();
                let second = selection(tester, controller.handle());
                assert!(second.is_collapsed(), "{platform:?}");
                assert_eq!(second.base_offset, first.base_offset, "{platform:?}");
                assert!(!selection_overlay_visibility(tester).1, "{platform:?}");
            });
        }
    }

    #[test]
    fn can_double_tap_and_drag_to_select_word_by_word() {
        test_widgets(|tester| {
            let controller = controller_with(tester, "");
            let root = material_home(
                theme(tester),
                TextField::new()
                    .drag_start_behavior(DragStartBehavior::Down)
                    .controller(controller.erase()),
            );
            tester.pump_widget(root);
            tester.enter_text(find::by_type::<TextField>(), "abc def ghi");
            skip_past_scrolling_animation(tester);
            let text = "abc def ghi";
            let e = text_offset_to_position(tester, text.find('e').unwrap() as i32);
            let h = text_offset_to_position(tester, text.find('h').unwrap() as i32);

            let mut pointer = TestPointer::touch(1);
            tester.down(&mut pointer, e);
            tester.pump();
            tester.up(&pointer);
            tester.pump();
            assert!(selection(tester, controller.handle()).is_collapsed());
            assert_eq!(
                selection(tester, controller.handle()).base_offset,
                text.find('e').unwrap() as i32
            );

            tester.down(&mut pointer, e);
            tester.pump_and_settle();
            assert_eq!(selection(tester, controller.handle()).base_offset, 4);
            assert_eq!(selection(tester, controller.handle()).extent_offset, 7);

            tester.move_to(&mut pointer, h, true);
            tester.pump_and_settle();
            assert!(!selection_overlay_visibility(tester).1);
            assert_eq!(
                selection(tester, controller.handle()).base_offset,
                text.find('d').unwrap() as i32
            );
            assert_eq!(
                selection(tester, controller.handle()).extent_offset,
                text.find('i').unwrap() as i32 + 1
            );

            tester.up(&pointer);
            tester.pump();
            assert!(selection_overlay_visibility(tester).1);
        });
    }
}
