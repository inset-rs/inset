//! Flutter counterpart: `cupertino/text_field.dart`.
//!
//! `_BaselineAlignedStack` waits.
//! Placeholder and editable share a `Stack` rather than baseline alignment.

use std::any::TypeId;
use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{
    BoxHeightStyle, BoxWidthStyle, Brightness, Clip, Color, FontWeight, Offset, Radius,
    SmartDashesType, SmartQuotesType, TargetPlatform, TextAffinity, TextAlign, TextCapitalization,
    TextDecoration, TextDecorationStyle, TextDirection, TextInputAction, TextInputType,
    TextSelection,
};
use waft_foundation::{
    App, Handle, Listenable, Listener, OptionRetained, Ref, ValueNotifier, VoidCallback,
};
use waft_gestures::{
    DragStartBehavior, GestureTapCallback, HitTestResult, PointerDownEvent, PointerUpEvent,
    TapDragEndDetails, TapDragUpDetails,
};
use waft_painting::{
    Alignment, AlignmentGeometry, AnyColor, Border, BorderRadius, BorderSide, BorderStyle,
    BoxDecoration, EdgeInsets, EdgeInsetsGeometry, TextAlignVertical, TextStyle,
};
use waft_rendering::{BoxHitTestResult, CrossAxisAlignment, HitTestBehavior};
use waft_services::{
    LengthLimitingTextInputFormatter, MaxLengthEnforcement, TextInputFormatterRef,
};
use waft_widgets::{
    Align, AutomaticKeepAliveClientMixin, AutomaticKeepAliveClientMixinData, BuildContext,
    Container, ContentInsertionConfiguration, Directionality, EditableText,
    EditableTextContextMenuBuilder, EditableTextState, Expanded, FocusNode, FocusNodeLeaf,
    FocusNodeRef, GestureDetector, GlobalKey, IgnorePointer, IntoWidget, KeyRef,
    MagnifierController, MagnifierInfo, MediaQuery, Padding, RepaintBoundary, RestorableListenable,
    RestorableProperty, RestorableTextEditingController, RestorationBucket, RestorationMixin,
    RestorationMixinData, Row, ScrollControllerRef, ScrollPhysicsRef, Semantics, SizedBox,
    SpellCheckConfiguration, Stack, State, StateData, StatefulWidget, SystemContextMenu, Text,
    TextEditingControllerLeaf, TextEditingControllerRef, TextFieldTapRegion,
    TextMagnifierConfiguration, TextSelectionControls, TextSelectionGestureDetectorBuilder,
    TextSelectionGestureDetectorBuilderBase, TextSelectionGestureDetectorBuilderData,
    TextSelectionGestureDetectorBuilderDelegate, ToolbarOptions, UndoHistoryController,
    UnfocusDisposition, UnmanagedRestorationScope, Visibility, WidgetRef,
};

use crate::adaptive_text_selection_toolbar::CupertinoAdaptiveTextSelectionToolbar;
use crate::colors::{CupertinoColors, CupertinoDynamicColor};
use crate::desktop_text_selection::cupertino_desktop_text_selection_handle_controls;
use crate::icons::CupertinoIcons;
use crate::localizations::CupertinoLocalizations;
use crate::magnifier::CupertinoTextMagnifier;
use crate::text_selection::cupertino_text_selection_handle_controls;
use crate::theme::CupertinoTheme;

/// Value inspected from Xcode 11 & iOS 13.0 Simulator.
const K_DEFAULT_ROUNDED_BORDER_SIDE_COLOR: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0x33000000), Color::new(0x33FFFFFF));

const K_DEFAULT_ROUNDED_BORDER_FILL: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xFFFFFFFF), Color::new(0xFF000000));

const K_DISABLED_BACKGROUND: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xFFFAFAFA), Color::new(0xFF050505));

fn clear_button_color() -> AnyColor {
    AnyColor::from(CupertinoDynamicColor::with_brightness(
        Color::new(0x33000000),
        Color::new(0x33FFFFFF),
    ))
}

/// An eyeballed value that moves the cursor slightly left of where it is
/// rendered for text on Android so it's positioning more accurately matches the
/// native iOS text cursor positioning.
///
/// This value is in device pixels, not logical pixels as is typically used
/// throughout the codebase.
const IOS_HORIZONTAL_CURSOR_OFFSET_PIXELS: f64 = -2.0;

/// Visibility of text field overlays based on the state of the current text entry.
///
/// Used to toggle the visibility behavior of the optional decorating widgets
/// surrounding the [`EditableText`] such as the clear text button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayVisibilityMode {
    /// Overlay will never appear regardless of the text entry state.
    Never,
    /// Overlay will only appear when the current text entry is not empty.
    Editing,
    /// Overlay will only appear when the current text entry is empty.
    NotEditing,
    /// Always show the overlay regardless of the text entry state.
    Always,
}

/// An iOS-style text field.
#[allow(clippy::type_complexity)]
pub struct CupertinoTextField {
    /// See [`Widget::key`](waft_widgets::Widget::key).
    pub key: Option<KeyRef>,
    /// {@macro flutter.widgets.editableText.groupId}
    pub group_id: TypeId,
    /// Controls the text being edited.
    ///
    /// If null, this widget will create its own [`TextEditingController`].
    pub controller: Option<TextEditingControllerRef>,
    /// {@macro flutter.widgets.Focus.focusNode}
    pub focus_node: Option<FocusNodeRef>,
    /// {@macro flutter.widgets.undoHistory.controller}
    pub undo_controller: Option<Ref<UndoHistoryController>>,
    /// Controls the [`BoxDecoration`] of the box behind the text input.
    ///
    /// Defaults to having a rounded rectangle grey border and can be null to have
    /// no box decoration.
    pub decoration: Option<BoxDecoration>,
    /// Padding around the text entry area between the [`prefix`](Self::prefix) and
    /// [`suffix`](Self::suffix) or the clear button when [`clear_button_mode`](Self::clear_button_mode)
    /// is not never.
    pub padding: EdgeInsetsGeometry,
    /// A lighter colored placeholder hint that appears on the first line of the
    /// text field when the text entry is empty.
    pub placeholder: Option<String>,
    /// The style to use for the placeholder text.
    pub placeholder_style: Option<TextStyle>,
    /// An optional widget to display before the text.
    pub prefix: Option<WidgetRef>,
    /// Controls the visibility of the [`prefix`](Self::prefix) widget based on the
    /// state of text entry when the [`prefix`](Self::prefix) argument is not null.
    pub prefix_mode: OverlayVisibilityMode,
    /// An optional widget to display after the text.
    pub suffix: Option<WidgetRef>,
    /// Controls the visibility of the [`suffix`](Self::suffix) widget based on the
    /// state of text entry when the [`suffix`](Self::suffix) argument is not null.
    pub suffix_mode: OverlayVisibilityMode,
    /// Controls the vertical alignment of the [`prefix`](Self::prefix) and the
    /// [`suffix`](Self::suffix) widget in relation to content.
    pub cross_axis_alignment: CrossAxisAlignment,
    /// Show an iOS-style clear button to clear the current text entry.
    pub clear_button_mode: OverlayVisibilityMode,
    /// The semantic label for the clear button used by screen readers.
    pub clear_button_semantic_label: Option<String>,
    /// {@macro flutter.widgets.editableText.keyboardType}
    pub keyboard_type: Option<TextInputType>,
    /// The type of action button to use for the keyboard.
    pub text_input_action: Option<TextInputAction>,
    /// {@macro flutter.widgets.editableText.textCapitalization}
    pub text_capitalization: TextCapitalization,
    /// The style to use for the text being edited.
    pub style: Option<TextStyle>,
    /// {@macro flutter.widgets.editableText.textAlign}
    pub text_align: TextAlign,
    /// {@macro flutter.material.InputDecorator.textAlignVertical}
    pub text_align_vertical: Option<TextAlignVertical>,
    /// {@macro flutter.widgets.editableText.textDirection}
    pub text_direction: Option<TextDirection>,
    /// {@macro flutter.widgets.editableText.readOnly}
    pub read_only: bool,
    /// Configuration of toolbar options.
    pub toolbar_options: Option<ToolbarOptions>,
    /// {@macro flutter.widgets.editableText.showCursor}
    pub show_cursor: Option<bool>,
    /// {@macro flutter.widgets.editableText.autofocus}
    pub autofocus: bool,
    /// {@macro flutter.widgets.editableText.obscuringCharacter}
    pub obscuring_character: String,
    /// {@macro flutter.widgets.editableText.obscureText}
    pub obscure_text: bool,
    /// {@macro flutter.widgets.editableText.autocorrect}
    pub autocorrect: Option<bool>,
    /// {@macro flutter.services.TextInputConfiguration.smartDashesType}
    pub smart_dashes_type: Option<SmartDashesType>,
    /// {@macro flutter.services.TextInputConfiguration.smartQuotesType}
    pub smart_quotes_type: Option<SmartQuotesType>,
    /// {@macro flutter.services.TextInputConfiguration.enableSuggestions}
    pub enable_suggestions: bool,
    /// {@macro flutter.widgets.editableText.maxLines}
    pub max_lines: Option<i32>,
    /// {@macro flutter.widgets.editableText.minLines}
    pub min_lines: Option<i32>,
    /// {@macro flutter.widgets.editableText.expands}
    pub expands: bool,
    /// The maximum number of characters (Unicode grapheme clusters) to allow in
    /// the text field.
    pub max_length: Option<i32>,
    /// Determines how the [`max_length`](Self::max_length) limit should be enforced.
    pub max_length_enforcement: Option<MaxLengthEnforcement>,
    /// {@macro flutter.widgets.editableText.onChanged}
    pub on_changed: Option<Rc<dyn Fn(&mut App, String)>>,
    /// {@macro flutter.widgets.editableText.onEditingComplete}
    pub on_editing_complete: Option<Rc<dyn Fn(&mut App)>>,
    /// {@macro flutter.widgets.editableText.onSubmitted}
    pub on_submitted: Option<Rc<dyn Fn(&mut App, String)>>,
    /// {@macro flutter.widgets.editableText.onTapOutside}
    pub on_tap_outside: Option<Rc<dyn Fn(&mut App, PointerDownEvent)>>,
    /// {@macro flutter.widgets.editableText.onTapUpOutside}
    pub on_tap_up_outside: Option<Rc<dyn Fn(&mut App, PointerUpEvent)>>,
    /// {@macro flutter.widgets.editableText.inputFormatters}
    pub input_formatters: Option<Vec<TextInputFormatterRef>>,
    /// Disables the text field when false.
    pub enabled: bool,
    /// {@macro flutter.widgets.editableText.cursorWidth}
    pub cursor_width: f64,
    /// {@macro flutter.widgets.editableText.cursorHeight}
    pub cursor_height: Option<f64>,
    /// {@macro flutter.widgets.editableText.cursorRadius}
    pub cursor_radius: Radius,
    /// {@macro flutter.widgets.editableText.cursorOpacityAnimates}
    pub cursor_opacity_animates: bool,
    /// The color to use when painting the cursor.
    pub cursor_color: Option<AnyColor>,
    /// Controls how tall the selection highlight boxes are computed to be.
    pub selection_height_style: Option<BoxHeightStyle>,
    /// Controls how wide the selection highlight boxes are computed to be.
    pub selection_width_style: Option<BoxWidthStyle>,
    /// The appearance of the keyboard.
    pub keyboard_appearance: Option<Brightness>,
    /// {@macro flutter.widgets.editableText.scrollPadding}
    pub scroll_padding: EdgeInsets,
    /// {@macro flutter.widgets.scrollable.dragStartBehavior}
    pub drag_start_behavior: DragStartBehavior,
    /// {@macro flutter.widgets.editableText.enableInteractiveSelection}
    pub enable_interactive_selection: Option<bool>,
    /// {@macro flutter.widgets.editableText.selectAllOnFocus}
    pub select_all_on_focus: Option<bool>,
    /// {@macro flutter.widgets.editableText.selectionControls}
    pub selection_controls: Option<Rc<dyn TextSelectionControls>>,
    /// {@macro flutter.widgets.editableText.scrollController}
    pub scroll_controller: Option<ScrollControllerRef>,
    /// {@macro flutter.widgets.editableText.scrollPhysics}
    pub scroll_physics: Option<ScrollPhysicsRef>,
    /// {@macro flutter.material.textfield.onTap}
    pub on_tap: Option<GestureTapCallback>,
    /// {@macro flutter.widgets.editableText.autofillHints}
    pub autofill_hints: Vec<String>,
    /// {@macro flutter.widgets.editableText.contentInsertionConfiguration}
    pub content_insertion_configuration: Option<ContentInsertionConfiguration>,
    /// {@macro flutter.material.Material.clipBehavior}
    pub clip_behavior: Clip,
    /// {@macro flutter.material.textfield.restorationId}
    pub restoration_id: Option<String>,
    /// {@macro flutter.widgets.editableText.scribbleEnabled}
    pub scribble_enabled: bool,
    /// {@macro flutter.widgets.editableText.stylusHandwritingEnabled}
    pub stylus_handwriting_enabled: bool,
    /// {@macro flutter.services.TextInputConfiguration.enableIMEPersonalizedLearning}
    pub enable_ime_personalized_learning: bool,
    /// {@macro flutter.services.TextInputConfiguration.enableInlinePrediction}
    pub enable_inline_prediction: Option<bool>,
    /// {@macro flutter.widgets.EditableText.contextMenuBuilder}
    pub context_menu_builder: Option<EditableTextContextMenuBuilder>,
    /// {@macro flutter.widgets.EditableText.spellCheckConfiguration}
    pub spell_check_configuration: Option<SpellCheckConfiguration>,
    /// Configuration for the text field magnifier.
    pub magnifier_configuration: Option<TextMagnifierConfiguration>,
}

impl CupertinoTextField {
    /// Creates an iOS-style text field.
    pub fn new() -> CupertinoTextField {
        CupertinoTextField {
            key: None,
            group_id: TypeId::of::<EditableText>(),
            controller: None,
            focus_node: None,
            undo_controller: None,
            decoration: Some(k_default_rounded_border_decoration()),
            padding: EdgeInsetsGeometry::all(7.0),
            placeholder: None,
            placeholder_style: Some(default_placeholder_style()),
            prefix: None,
            prefix_mode: OverlayVisibilityMode::Always,
            suffix: None,
            suffix_mode: OverlayVisibilityMode::Always,
            cross_axis_alignment: CrossAxisAlignment::Center,
            clear_button_mode: OverlayVisibilityMode::Never,
            clear_button_semantic_label: None,
            keyboard_type: None,
            text_input_action: None,
            text_capitalization: TextCapitalization::None,
            style: None,
            text_align: TextAlign::Start,
            text_align_vertical: None,
            text_direction: None,
            read_only: false,
            toolbar_options: None,
            show_cursor: None,
            autofocus: false,
            obscuring_character: "\u{2022}".into(),
            obscure_text: false,
            autocorrect: Some(true),
            smart_dashes_type: None,
            smart_quotes_type: None,
            enable_suggestions: true,
            max_lines: Some(1),
            min_lines: None,
            expands: false,
            max_length: None,
            max_length_enforcement: None,
            on_changed: None,
            on_editing_complete: None,
            on_submitted: None,
            on_tap_outside: None,
            on_tap_up_outside: None,
            input_formatters: None,
            enabled: true,
            cursor_width: 2.0,
            cursor_height: None,
            cursor_radius: Radius::circular(2.0),
            cursor_opacity_animates: true,
            cursor_color: None,
            selection_height_style: None,
            selection_width_style: None,
            keyboard_appearance: None,
            scroll_padding: EdgeInsets::all(20.0),
            drag_start_behavior: DragStartBehavior::Start,
            enable_interactive_selection: None,
            select_all_on_focus: None,
            selection_controls: None,
            scroll_controller: None,
            scroll_physics: None,
            on_tap: None,
            autofill_hints: Vec::new(),
            content_insertion_configuration: None,
            clip_behavior: Clip::HardEdge,
            restoration_id: None,
            scribble_enabled: true,
            stylus_handwriting_enabled: true,
            enable_ime_personalized_learning: true,
            enable_inline_prediction: None,
            context_menu_builder: Some(Rc::new(Self::default_context_menu_builder)),
            spell_check_configuration: None,
            magnifier_configuration: None,
        }
    }

    /// Creates a borderless iOS-style text field.
    pub fn borderless() -> CupertinoTextField {
        CupertinoTextField::new().decoration(None).autocorrect(None)
    }

    /// Dart `CupertinoTextField(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoTextField {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoTextField(groupId:)`.
    pub fn group_id(mut self, group_id: TypeId) -> CupertinoTextField {
        self.group_id = group_id;
        self
    }

    /// Dart `CupertinoTextField(controller:)`.
    pub fn controller(mut self, controller: TextEditingControllerRef) -> CupertinoTextField {
        self.controller = Some(controller);
        self
    }

    /// Dart `CupertinoTextField(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> CupertinoTextField {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `CupertinoTextField(undoController:)`.
    pub fn undo_controller(
        mut self,
        undo_controller: Ref<UndoHistoryController>,
    ) -> CupertinoTextField {
        self.undo_controller = Some(undo_controller);
        self
    }

    /// Dart `CupertinoTextField(decoration:)`.
    pub fn decoration(mut self, decoration: Option<BoxDecoration>) -> CupertinoTextField {
        self.decoration = decoration;
        self
    }

    /// Dart `CupertinoTextField(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> CupertinoTextField {
        self.padding = padding.into();
        self
    }

    /// Dart `CupertinoTextField(placeholder:)`.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> CupertinoTextField {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Dart `CupertinoTextField(placeholderStyle:)`.
    pub fn placeholder_style(mut self, placeholder_style: Option<TextStyle>) -> CupertinoTextField {
        self.placeholder_style = placeholder_style;
        self
    }

    /// Dart `CupertinoTextField(prefix:)`.
    pub fn prefix<K>(mut self, prefix: impl IntoWidget<K>) -> CupertinoTextField {
        self.prefix = Some(prefix.into_widget());
        self
    }

    /// Dart `CupertinoTextField(prefixMode:)`.
    pub fn prefix_mode(mut self, prefix_mode: OverlayVisibilityMode) -> CupertinoTextField {
        self.prefix_mode = prefix_mode;
        self
    }

    /// Dart `CupertinoTextField(suffix:)`.
    pub fn suffix<K>(mut self, suffix: impl IntoWidget<K>) -> CupertinoTextField {
        self.suffix = Some(suffix.into_widget());
        self
    }

    /// Dart `CupertinoTextField(suffixMode:)`.
    pub fn suffix_mode(mut self, suffix_mode: OverlayVisibilityMode) -> CupertinoTextField {
        self.suffix_mode = suffix_mode;
        self
    }

    /// Dart `CupertinoTextField(crossAxisAlignment:)`.
    pub fn cross_axis_alignment(
        mut self,
        cross_axis_alignment: CrossAxisAlignment,
    ) -> CupertinoTextField {
        self.cross_axis_alignment = cross_axis_alignment;
        self
    }

    /// Dart `CupertinoTextField(clearButtonMode:)`.
    pub fn clear_button_mode(
        mut self,
        clear_button_mode: OverlayVisibilityMode,
    ) -> CupertinoTextField {
        self.clear_button_mode = clear_button_mode;
        self
    }

    /// Dart `CupertinoTextField(clearButtonSemanticLabel:)`.
    pub fn clear_button_semantic_label(
        mut self,
        clear_button_semantic_label: impl Into<String>,
    ) -> CupertinoTextField {
        self.clear_button_semantic_label = Some(clear_button_semantic_label.into());
        self
    }

    /// Dart `CupertinoTextField(keyboardType:)`.
    pub fn keyboard_type(mut self, keyboard_type: TextInputType) -> CupertinoTextField {
        self.keyboard_type = Some(keyboard_type);
        self
    }

    /// Dart `CupertinoTextField(textInputAction:)`.
    pub fn text_input_action(mut self, text_input_action: TextInputAction) -> CupertinoTextField {
        self.text_input_action = Some(text_input_action);
        self
    }

    /// Dart `CupertinoTextField(textCapitalization:)`.
    pub fn text_capitalization(
        mut self,
        text_capitalization: TextCapitalization,
    ) -> CupertinoTextField {
        self.text_capitalization = text_capitalization;
        self
    }

    /// Dart `CupertinoTextField(style:)`.
    pub fn style(mut self, style: TextStyle) -> CupertinoTextField {
        self.style = Some(style);
        self
    }

    /// Dart `CupertinoTextField(textAlign:)`.
    pub fn text_align(mut self, text_align: TextAlign) -> CupertinoTextField {
        self.text_align = text_align;
        self
    }

    /// Dart `CupertinoTextField(textAlignVertical:)`.
    pub fn text_align_vertical(
        mut self,
        text_align_vertical: TextAlignVertical,
    ) -> CupertinoTextField {
        self.text_align_vertical = Some(text_align_vertical);
        self
    }

    /// Dart `CupertinoTextField(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> CupertinoTextField {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `CupertinoTextField(readOnly:)`.
    pub fn read_only(mut self, read_only: bool) -> CupertinoTextField {
        self.read_only = read_only;
        self
    }

    /// Dart `CupertinoTextField(toolbarOptions:)`.
    pub fn toolbar_options(mut self, toolbar_options: ToolbarOptions) -> CupertinoTextField {
        self.toolbar_options = Some(toolbar_options);
        self
    }

    /// Dart `CupertinoTextField(showCursor:)`.
    pub fn show_cursor(mut self, show_cursor: bool) -> CupertinoTextField {
        self.show_cursor = Some(show_cursor);
        self
    }

    /// Dart `CupertinoTextField(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> CupertinoTextField {
        self.autofocus = autofocus;
        self
    }

    /// Dart `CupertinoTextField(obscuringCharacter:)`.
    pub fn obscuring_character(
        mut self,
        obscuring_character: impl Into<String>,
    ) -> CupertinoTextField {
        let obscuring_character = obscuring_character.into();
        debug_assert!(
            obscuring_character.chars().count() == 1,
            "obscuringCharacter must be a single character"
        );
        self.obscuring_character = obscuring_character;
        self
    }

    /// Dart `CupertinoTextField(obscureText:)`.
    pub fn obscure_text(mut self, obscure_text: bool) -> CupertinoTextField {
        debug_assert!(!obscure_text || self.max_lines == Some(1));
        self.obscure_text = obscure_text;
        self
    }

    /// Dart `CupertinoTextField(autocorrect:)`.
    pub fn autocorrect(mut self, autocorrect: Option<bool>) -> CupertinoTextField {
        self.autocorrect = autocorrect;
        self
    }

    /// Dart `CupertinoTextField(smartDashesType:)`.
    pub fn smart_dashes_type(mut self, smart_dashes_type: SmartDashesType) -> CupertinoTextField {
        self.smart_dashes_type = Some(smart_dashes_type);
        self
    }

    /// Dart `CupertinoTextField(smartQuotesType:)`.
    pub fn smart_quotes_type(mut self, smart_quotes_type: SmartQuotesType) -> CupertinoTextField {
        self.smart_quotes_type = Some(smart_quotes_type);
        self
    }

    /// Dart `CupertinoTextField(enableSuggestions:)`.
    pub fn enable_suggestions(mut self, enable_suggestions: bool) -> CupertinoTextField {
        self.enable_suggestions = enable_suggestions;
        self
    }

    /// Dart `CupertinoTextField(maxLines:)`.
    pub fn max_lines(mut self, max_lines: Option<i32>) -> CupertinoTextField {
        debug_assert!(max_lines.is_none_or(|lines| lines > 0));
        debug_assert!(
            max_lines.is_none_or(|max| self.min_lines.is_none_or(|min| max >= min)),
            "minLines can't be greater than maxLines"
        );
        debug_assert!(!self.expands || max_lines.is_none());
        debug_assert!(!self.obscure_text || max_lines == Some(1));
        self.max_lines = max_lines;
        self
    }

    /// Dart `CupertinoTextField(minLines:)`.
    pub fn min_lines(mut self, min_lines: i32) -> CupertinoTextField {
        debug_assert!(min_lines > 0);
        debug_assert!(
            self.max_lines.is_none_or(|max| max >= min_lines),
            "minLines can't be greater than maxLines"
        );
        debug_assert!(!self.expands);
        self.min_lines = Some(min_lines);
        self
    }

    /// Dart `CupertinoTextField(expands:)`.
    pub fn expands(mut self, expands: bool) -> CupertinoTextField {
        debug_assert!(!expands || (self.max_lines.is_none() && self.min_lines.is_none()));
        self.expands = expands;
        self
    }

    /// Dart `CupertinoTextField(maxLength:)`.
    pub fn max_length(mut self, max_length: i32) -> CupertinoTextField {
        debug_assert!(max_length > 0);
        self.max_length = Some(max_length);
        self
    }

    /// Dart `CupertinoTextField(maxLengthEnforcement:)`.
    pub fn max_length_enforcement(
        mut self,
        max_length_enforcement: MaxLengthEnforcement,
    ) -> CupertinoTextField {
        self.max_length_enforcement = Some(max_length_enforcement);
        self
    }

    /// Dart `CupertinoTextField(onChanged:)`.
    pub fn on_changed(
        mut self,
        on_changed: impl Fn(&mut App, String) + 'static,
    ) -> CupertinoTextField {
        self.on_changed = Some(Rc::new(on_changed));
        self
    }

    /// Dart `CupertinoTextField(onEditingComplete:)`.
    pub fn on_editing_complete(
        mut self,
        on_editing_complete: impl Fn(&mut App) + 'static,
    ) -> CupertinoTextField {
        self.on_editing_complete = Some(Rc::new(on_editing_complete));
        self
    }

    /// Dart `CupertinoTextField(onSubmitted:)`.
    pub fn on_submitted(
        mut self,
        on_submitted: impl Fn(&mut App, String) + 'static,
    ) -> CupertinoTextField {
        self.on_submitted = Some(Rc::new(on_submitted));
        self
    }

    /// Dart `CupertinoTextField(onTapOutside:)`.
    pub fn on_tap_outside(
        mut self,
        on_tap_outside: impl Fn(&mut App, PointerDownEvent) + 'static,
    ) -> CupertinoTextField {
        self.on_tap_outside = Some(Rc::new(on_tap_outside));
        self
    }

    /// Dart `CupertinoTextField(onTapUpOutside:)`.
    pub fn on_tap_up_outside(
        mut self,
        on_tap_up_outside: impl Fn(&mut App, PointerUpEvent) + 'static,
    ) -> CupertinoTextField {
        self.on_tap_up_outside = Some(Rc::new(on_tap_up_outside));
        self
    }

    /// Dart `CupertinoTextField(inputFormatters:)`.
    pub fn input_formatters(
        mut self,
        input_formatters: Vec<TextInputFormatterRef>,
    ) -> CupertinoTextField {
        self.input_formatters = Some(input_formatters);
        self
    }

    /// Dart `CupertinoTextField(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> CupertinoTextField {
        self.enabled = enabled;
        self
    }

    /// Dart `CupertinoTextField(cursorWidth:)`.
    pub fn cursor_width(mut self, cursor_width: f64) -> CupertinoTextField {
        self.cursor_width = cursor_width;
        self
    }

    /// Dart `CupertinoTextField(cursorHeight:)`.
    pub fn cursor_height(mut self, cursor_height: f64) -> CupertinoTextField {
        self.cursor_height = Some(cursor_height);
        self
    }

    /// Dart `CupertinoTextField(cursorRadius:)`.
    pub fn cursor_radius(mut self, cursor_radius: Radius) -> CupertinoTextField {
        self.cursor_radius = cursor_radius;
        self
    }

    /// Dart `CupertinoTextField(cursorOpacityAnimates:)`.
    pub fn cursor_opacity_animates(mut self, cursor_opacity_animates: bool) -> CupertinoTextField {
        self.cursor_opacity_animates = cursor_opacity_animates;
        self
    }

    /// Dart `CupertinoTextField(cursorColor:)`.
    pub fn cursor_color(mut self, cursor_color: impl Into<AnyColor>) -> CupertinoTextField {
        self.cursor_color = Some(cursor_color.into());
        self
    }

    /// Dart `CupertinoTextField(selectionHeightStyle:)`.
    pub fn selection_height_style(
        mut self,
        selection_height_style: BoxHeightStyle,
    ) -> CupertinoTextField {
        self.selection_height_style = Some(selection_height_style);
        self
    }

    /// Dart `CupertinoTextField(selectionWidthStyle:)`.
    pub fn selection_width_style(
        mut self,
        selection_width_style: BoxWidthStyle,
    ) -> CupertinoTextField {
        self.selection_width_style = Some(selection_width_style);
        self
    }

    /// Dart `CupertinoTextField(keyboardAppearance:)`.
    pub fn keyboard_appearance(mut self, keyboard_appearance: Brightness) -> CupertinoTextField {
        self.keyboard_appearance = Some(keyboard_appearance);
        self
    }

    /// Dart `CupertinoTextField(scrollPadding:)`.
    pub fn scroll_padding(mut self, scroll_padding: EdgeInsets) -> CupertinoTextField {
        self.scroll_padding = scroll_padding;
        self
    }

    /// Dart `CupertinoTextField(dragStartBehavior:)`.
    pub fn drag_start_behavior(
        mut self,
        drag_start_behavior: DragStartBehavior,
    ) -> CupertinoTextField {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `CupertinoTextField(enableInteractiveSelection:)`.
    pub fn enable_interactive_selection(
        mut self,
        enable_interactive_selection: bool,
    ) -> CupertinoTextField {
        self.enable_interactive_selection = Some(enable_interactive_selection);
        self
    }

    /// Dart `CupertinoTextField(selectAllOnFocus:)`.
    pub fn select_all_on_focus(mut self, select_all_on_focus: bool) -> CupertinoTextField {
        self.select_all_on_focus = Some(select_all_on_focus);
        self
    }

    /// Dart `CupertinoTextField(selectionControls:)`.
    pub fn selection_controls(
        mut self,
        selection_controls: Rc<dyn TextSelectionControls>,
    ) -> CupertinoTextField {
        self.selection_controls = Some(selection_controls);
        self
    }

    /// Dart `CupertinoTextField(scrollController:)`.
    pub fn scroll_controller(
        mut self,
        scroll_controller: ScrollControllerRef,
    ) -> CupertinoTextField {
        self.scroll_controller = Some(scroll_controller);
        self
    }

    /// Dart `CupertinoTextField(scrollPhysics:)`.
    pub fn scroll_physics(mut self, scroll_physics: ScrollPhysicsRef) -> CupertinoTextField {
        self.scroll_physics = Some(scroll_physics);
        self
    }

    /// Dart `CupertinoTextField(onTap:)`.
    pub fn on_tap(mut self, on_tap: GestureTapCallback) -> CupertinoTextField {
        self.on_tap = Some(on_tap);
        self
    }

    /// Dart `CupertinoTextField(autofillHints:)`.
    pub fn autofill_hints(mut self, autofill_hints: Vec<String>) -> CupertinoTextField {
        self.autofill_hints = autofill_hints;
        self
    }

    /// Dart `CupertinoTextField(contentInsertionConfiguration:)`.
    pub fn content_insertion_configuration(
        mut self,
        content_insertion_configuration: ContentInsertionConfiguration,
    ) -> CupertinoTextField {
        self.content_insertion_configuration = Some(content_insertion_configuration);
        self
    }

    /// Dart `CupertinoTextField(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> CupertinoTextField {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `CupertinoTextField(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> CupertinoTextField {
        self.restoration_id = Some(restoration_id.into());
        self
    }

    /// Dart `CupertinoTextField(scribbleEnabled:)`.
    pub fn scribble_enabled(mut self, scribble_enabled: bool) -> CupertinoTextField {
        self.scribble_enabled = scribble_enabled;
        self
    }

    /// Dart `CupertinoTextField(stylusHandwritingEnabled:)`.
    pub fn stylus_handwriting_enabled(
        mut self,
        stylus_handwriting_enabled: bool,
    ) -> CupertinoTextField {
        self.stylus_handwriting_enabled = stylus_handwriting_enabled;
        self
    }

    /// Dart `CupertinoTextField(enableIMEPersonalizedLearning:)`.
    pub fn enable_ime_personalized_learning(
        mut self,
        enable_ime_personalized_learning: bool,
    ) -> CupertinoTextField {
        self.enable_ime_personalized_learning = enable_ime_personalized_learning;
        self
    }

    /// Dart `CupertinoTextField(enableInlinePrediction:)`.
    pub fn enable_inline_prediction(
        mut self,
        enable_inline_prediction: bool,
    ) -> CupertinoTextField {
        self.enable_inline_prediction = Some(enable_inline_prediction);
        self
    }

    /// Dart `CupertinoTextField(contextMenuBuilder:)`.
    pub fn context_menu_builder(
        mut self,
        context_menu_builder: EditableTextContextMenuBuilder,
    ) -> CupertinoTextField {
        self.context_menu_builder = Some(context_menu_builder);
        self
    }

    fn default_context_menu_builder(
        app: &mut App,
        _context: BuildContext,
        editable_text_state: Handle<EditableTextState>,
    ) -> WidgetRef {
        if SystemContextMenu::is_supported_by_field(app, editable_text_state) {
            return SystemContextMenu::editable_text(app, editable_text_state).into_widget();
        }
        CupertinoAdaptiveTextSelectionToolbar::editable_text(app, editable_text_state).into_widget()
    }

    /// Dart `CupertinoTextField(spellCheckConfiguration:)`.
    pub fn spell_check_configuration(
        mut self,
        spell_check_configuration: SpellCheckConfiguration,
    ) -> CupertinoTextField {
        self.spell_check_configuration = Some(spell_check_configuration);
        self
    }

    /// Dart `CupertinoTextField(magnifierConfiguration:)`.
    pub fn magnifier_configuration(
        mut self,
        magnifier_configuration: TextMagnifierConfiguration,
    ) -> CupertinoTextField {
        self.magnifier_configuration = Some(magnifier_configuration);
        self
    }

    /// {@macro flutter.widgets.editableText.selectionEnabled}
    pub fn selection_enabled(&self) -> bool {
        self.enable_interactive_selection
            .unwrap_or(!self.read_only || !self.obscure_text)
    }

    /// The [`TextStyle`] used to indicate misspelled words in the Cupertino style.
    pub fn cupertino_misspelled_text_style() -> TextStyle {
        TextStyle::new()
            .decoration(TextDecoration::UNDERLINE)
            .decoration_color(CupertinoColors::SYSTEM_RED)
            .decoration_style(TextDecorationStyle::Dotted)
    }

    /// The color of the selection highlight when the spell check menu is visible.
    pub const K_MISSPELLED_SELECTION_COLOR: Color = Color::new(0x62FF9699);

    /// Dart's `static final _iosMagnifierConfiguration`: one value per thread, so that its
    /// builder keeps one identity, which `TextMagnifierConfiguration`'s comparison reads.
    fn ios_magnifier_configuration() -> TextMagnifierConfiguration {
        thread_local! {
            static CONFIGURATION: TextMagnifierConfiguration = TextMagnifierConfiguration::new()
                .with_magnifier_builder(Rc::new(
                    |app: &mut App,
                     _context: BuildContext,
                     controller: Ref<MagnifierController>,
                     magnifier_info: Ref<ValueNotifier<MagnifierInfo>>| {
                        match app.platform().target_platform() {
                            TargetPlatform::Android | TargetPlatform::IOS => Some(
                                CupertinoTextMagnifier::new(controller, magnifier_info)
                                    .into_widget(),
                            ),
                            TargetPlatform::Fuchsia
                            | TargetPlatform::Linux
                            | TargetPlatform::MacOS
                            | TargetPlatform::Windows => None,
                        }
                    },
                ));
        }
        CONFIGURATION.with(TextMagnifierConfiguration::clone)
    }

    /// Returns a new [`SpellCheckConfiguration`] where the given configuration has
    /// had any missing values replaced with their defaults for the iOS platform.
    pub fn infer_ios_spell_check_configuration(
        configuration: Option<&SpellCheckConfiguration>,
    ) -> SpellCheckConfiguration {
        match configuration {
            None => SpellCheckConfiguration::disabled(),
            Some(configuration)
                if *configuration == SpellCheckConfiguration::disabled()
                    || !configuration.spell_check_enabled() =>
            {
                SpellCheckConfiguration::disabled()
            }
            Some(configuration) => {
                let mut inferred = configuration.copy_with();
                if inferred.misspelled_text_style.is_none() {
                    inferred =
                        inferred.misspelled_text_style(Self::cupertino_misspelled_text_style());
                }
                if inferred.misspelled_selection_color.is_none() {
                    inferred =
                        inferred.misspelled_selection_color(Self::K_MISSPELLED_SELECTION_COLOR);
                }
                inferred
            }
        }
    }
}

impl Default for CupertinoTextField {
    fn default() -> CupertinoTextField {
        CupertinoTextField::new()
    }
}

impl Debug for CupertinoTextField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoTextField")
            .field("controller", &self.controller)
            .field("focus_node", &self.focus_node)
            .field("placeholder", &self.placeholder)
            .field("enabled", &self.enabled)
            .field("obscure_text", &self.obscure_text)
            .field("max_lines", &self.max_lines)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoTextField {
    type State = CupertinoTextFieldState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoTextFieldState {
        CupertinoTextFieldState {
            state: StateData::new(),
            restoration: RestorationMixinData::new(),
            automatic_keep_alive_client: AutomaticKeepAliveClientMixinData::new(),
            clear_global_key: GlobalKey::new(),
            editable_text_key: GlobalKey::new(),
            controller: None,
            focus_node: None,
            listening_controller: None,
            selection_gesture_detector_builder: None,
        }
    }
}

/// Dart's `_CupertinoTextFieldSelectionGestureDetectorBuilder`.
pub struct CupertinoTextFieldSelectionGestureDetectorBuilder {
    builder: TextSelectionGestureDetectorBuilderData,
    state: Handle<CupertinoTextFieldState>,
}

impl TextSelectionGestureDetectorBuilder for CupertinoTextFieldSelectionGestureDetectorBuilder {
    waft_widgets::text_selection_gesture_detector_builder_accessors!(builder);

    fn on_single_tap_up(self: Handle<Self>, app: &mut App, details: TapDragUpDetails) {
        let state = app.get(self).state;
        let clear_key = app.get(state).clear_global_key.clone();
        if let Some(context) = clear_key.current_context(app)
            && let Some(object) = context.find_render_object(app)
            && let Some(render_box) = object.as_box()
        {
            let local_offset = render_box.global_to_local(app, details.global_position, None);
            let mut result = HitTestResult::new();
            if render_box.hit_test(app, &mut BoxHitTestResult::wrap(&mut result), local_offset) {
                return;
            }
        }
        TextSelectionGestureDetectorBuilderBase::on_single_tap_up(self, app, details);
        if let Some(on_tap) = state.widget(app).on_tap.clone() {
            on_tap.call(app);
        }
    }

    fn on_drag_selection_end(self: Handle<Self>, app: &mut App, details: TapDragEndDetails) {
        let state = app.get(self).state;
        state.request_keyboard(app);
        TextSelectionGestureDetectorBuilderBase::on_drag_selection_end(self, app, details);
    }
}

/// Dart's `_CupertinoTextFieldState`.
pub struct CupertinoTextFieldState {
    state: StateData<CupertinoTextField>,
    restoration: RestorationMixinData,
    automatic_keep_alive_client: AutomaticKeepAliveClientMixinData,
    clear_global_key: GlobalKey,
    editable_text_key: GlobalKey,
    controller: Option<Ref<RestorableTextEditingController>>,
    focus_node: Option<Ref<FocusNode>>,
    listening_controller: Option<TextEditingControllerRef>,
    selection_gesture_detector_builder:
        Option<Ref<CupertinoTextFieldSelectionGestureDetectorBuilder>>,
}

impl CupertinoTextFieldState {
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

    fn effective_focus_node(self: Handle<Self>, app: &App) -> FocusNodeRef {
        self.widget(app)
            .focus_node
            .clone()
            .or_else(|| app.get(self).focus_node.as_ref().map(Ref::erase))
            .expect("init_state creates a local focus node")
    }

    fn effective_max_length_enforcement(self: Handle<Self>, app: &App) -> MaxLengthEnforcement {
        self.widget(app).max_length_enforcement.unwrap_or_else(|| {
            LengthLimitingTextInputFormatter::get_default_max_length_enforcement(None)
        })
    }

    fn register_controller(self: Handle<Self>, app: &mut App) {
        let controller = app
            .get(self)
            .controller
            .handle()
            .expect("register_controller needs a local controller");
        self.register_for_restoration(app, controller.as_property(), "controller");
    }

    fn create_local_controller(
        self: Handle<Self>,
        app: &mut App,
        value: Option<waft_services::TextEditingValue>,
    ) {
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

    fn listen_to_controller(self: Handle<Self>, app: &mut App) {
        if let Some(restorable) = app.get(self).controller.handle()
            && !RestorableProperty::is_registered(restorable, app)
        {
            return;
        }
        let controller = self.effective_controller(app);
        if app.get(self).listening_controller.handle() == Some(controller.handle()) {
            return;
        }
        self.unlisten_controller(app);
        controller.add_listener(
            app,
            Listener::handle_method(self, Self::handle_controller_changed),
        );
        app.get_mut(self).listening_controller = Some(controller);
    }

    fn unlisten_controller(self: Handle<Self>, app: &mut App) {
        if let Some(controller) = app.get(self).listening_controller.clone() {
            controller.remove_listener(
                app,
                &Listener::handle_method(self, Self::handle_controller_changed),
            );
            app.get_mut(self).listening_controller = None;
        }
    }

    fn handle_controller_changed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_| {});
        AutomaticKeepAliveClientMixin::update_keep_alive(self, app);
    }

    fn handle_focus_changed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_| {});
    }

    fn request_keyboard(self: Handle<Self>, app: &mut App) {
        let key = app.get(self).editable_text_key.clone();
        if let Some(editable) = key.current_state::<EditableTextState>(app) {
            editable.request_keyboard(app);
            return;
        }
        self.effective_focus_node(app).request_focus(app, None);
    }

    fn should_show_attachment(attachment: OverlayVisibilityMode, has_text: bool) -> bool {
        match attachment {
            OverlayVisibilityMode::Never => false,
            OverlayVisibilityMode::Always => true,
            OverlayVisibilityMode::Editing => has_text,
            OverlayVisibilityMode::NotEditing => !has_text,
        }
    }

    fn has_decoration(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        widget.placeholder.is_some()
            || widget.clear_button_mode != OverlayVisibilityMode::Never
            || widget.prefix.is_some()
            || widget.suffix.is_some()
    }

    fn text_align_vertical(self: Handle<Self>, app: &App) -> TextAlignVertical {
        if let Some(align) = self.widget(app).text_align_vertical {
            return align;
        }
        if self.has_decoration(app) {
            TextAlignVertical::CENTER
        } else {
            TextAlignVertical::TOP
        }
    }

    fn on_clear_button_tapped(self: Handle<Self>, app: &mut App) {
        let controller = self.effective_controller(app);
        let had_text = !controller.text_value(app).is_empty();
        controller.clear(app);
        if had_text && let Some(on_changed) = self.widget(app).on_changed.clone() {
            on_changed(app, controller.text_value(app).to_string());
        }
    }

    fn build_clear_button(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let enabled = self.widget(app).enabled;
        let color = CupertinoDynamicColor::resolve(&clear_button_color(), app, context);
        let this = self;
        let on_tap = Listener::new(move |app| this.on_clear_button_tapped(app));
        let clear_label = match self.widget(app).clear_button_semantic_label.clone() {
            Some(clear_label) => clear_label,
            None => <dyn CupertinoLocalizations>::of(app, context).clear_button_label(),
        };
        let icon = waft_widgets::Icon::new(Some(CupertinoIcons::clear_thick_circled()))
            .size(18.0)
            .color(color);
        let mut detector =
            GestureDetector::new().key(app.get(self).clear_global_key.clone().into());
        if enabled {
            detector = detector.on_tap(on_tap);
        }
        Semantics::new()
            .button(true)
            .label(clear_label)
            .child(
                detector.child(Padding::new(EdgeInsetsGeometry::symmetric(0.0, 6.0)).child(icon)),
            )
            .into_widget()
    }

    fn add_text_dependent_attachments(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        editable_text: WidgetRef,
        placeholder_style: &TextStyle,
    ) -> WidgetRef {
        if !self.has_decoration(app) {
            return editable_text;
        }
        let controller = self.effective_controller(app);
        let has_text = !controller.text_value(app).is_empty();
        let text_align = self.widget(app).text_align;
        let max_lines = self.widget(app).max_lines;
        let padding = self.widget(app).padding;
        let placeholder_text = self.widget(app).placeholder.clone();
        let prefix_mode = self.widget(app).prefix_mode;
        let suffix_mode = self.widget(app).suffix_mode;
        let clear_button_mode = self.widget(app).clear_button_mode;
        let prefix_widget = self.widget(app).prefix.clone();
        let suffix_widget = self.widget(app).suffix.clone();
        let widget_text_direction = self.widget(app).text_direction;
        let placeholder = placeholder_text.map(|placeholder_text| {
            let mut text = Text::new(placeholder_text)
                .style(placeholder_style.clone())
                .text_align(text_align);
            if has_text {
                text = text.max_lines(1);
            } else if let Some(max_lines) = max_lines {
                text = text.max_lines(max_lines);
            }
            if let Some(overflow) = placeholder_style.overflow {
                text = text.overflow(overflow);
            }
            Visibility::new(
                SizedBox::new()
                    .width(f64::INFINITY)
                    .child(Padding::new(padding).child(text)),
            )
            .maintain_animation(true)
            .maintain_size(true)
            .maintain_state(true)
            .visible(!has_text)
            .into_widget()
        });
        let prefix = prefix_widget.filter(|_| Self::should_show_attachment(prefix_mode, has_text));
        let show_user_suffix = Self::should_show_attachment(suffix_mode, has_text);
        let show_clear_button = Self::should_show_attachment(clear_button_mode, has_text);
        let suffix = match (show_user_suffix, show_clear_button) {
            (false, false) => None,
            (true, false) => suffix_widget,
            (true, true) => suffix_widget.or_else(|| Some(self.build_clear_button(app, context))),
            (false, true) => Some(self.build_clear_button(app, context)),
        };
        let text_direction =
            widget_text_direction.unwrap_or_else(|| Directionality::of(app, context));
        let stacked = match placeholder {
            None => editable_text,
            Some(placeholder) => Stack::new()
                .alignment(AlignmentGeometry::TOP_START)
                .children(vec![placeholder, editable_text])
                .into_widget(),
        };
        let mut children = Vec::new();
        if let Some(prefix) = prefix {
            children.push(prefix);
        }
        children.push(Expanded::new(Directionality::new(text_direction, stacked)).into_widget());
        if let Some(suffix) = suffix {
            children.push(suffix);
        }
        Row::new()
            .cross_axis_alignment(self.widget(app).cross_axis_alignment)
            .children(children)
            .into_widget()
    }
}

impl RestorationMixin for CupertinoTextFieldState {
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

impl AutomaticKeepAliveClientMixin for CupertinoTextFieldState {
    fn automatic_keep_alive_client_data(
        self: Handle<Self>,
        app: &App,
    ) -> &AutomaticKeepAliveClientMixinData {
        &app.get(self).automatic_keep_alive_client
    }

    fn automatic_keep_alive_client_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AutomaticKeepAliveClientMixinData {
        &mut app.get_mut(self).automatic_keep_alive_client
    }

    fn want_keep_alive(self: Handle<Self>, app: &App) -> bool {
        let Some(controller) = app.get(self).controller.handle() else {
            return false;
        };
        RestorableProperty::is_registered(controller, app)
            && !RestorableListenable::value(controller, app)
                .text_value(app)
                .is_empty()
    }
}

impl State for CupertinoTextFieldState {
    type Widget = CupertinoTextField;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        AutomaticKeepAliveClientMixin::init_state(self, app);
        if self.widget(app).controller.is_none() {
            self.create_local_controller(app, None);
        }
        if self.widget(app).focus_node.is_none() {
            app.get_mut(self).focus_node = Some(FocusNode::new(app));
        }
        let enabled = self.widget(app).enabled;
        let focus = self.effective_focus_node(app);
        focus.set_can_request_focus(app, enabled);
        focus.add_listener(
            app,
            Listener::handle_method(self, Self::handle_focus_changed),
        );
        if self.widget(app).controller.is_some() {
            self.listen_to_controller(app);
        }
        let builder = app.create(CupertinoTextFieldSelectionGestureDetectorBuilder {
            builder: TextSelectionGestureDetectorBuilderData::new(
                self.as_text_selection_gesture_detector_builder_delegate(),
            ),
            state: self,
        });
        app.get_mut(self).selection_gesture_detector_builder = Some(builder);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.did_change_dependencies_restoration(app);
        self.listen_to_controller(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &CupertinoTextField) {
        self.did_update_restoration_id(app);
        if self.widget(app).controller.is_none() && old_widget.controller.is_some() {
            let value = old_widget
                .controller
                .as_ref()
                .map(|controller| controller.value(app).clone());
            self.create_local_controller(app, value);
        } else if self.widget(app).controller.is_some()
            && old_widget.controller.is_none()
            && let Some(controller) = app.get(self).controller.handle()
        {
            self.unregister_from_restoration(app, controller.as_property());
            RestorableProperty::dispose(controller, app);
            app.get_mut(self).controller = None;
        }
        if self.widget(app).focus_node != old_widget.focus_node {
            let old_focus = old_widget
                .focus_node
                .clone()
                .or_else(|| app.get(self).focus_node.as_ref().map(Ref::erase));
            if let Some(old_focus) = old_focus {
                old_focus.remove_listener(
                    app,
                    &Listener::handle_method(self, Self::handle_focus_changed),
                );
            }
            if self.widget(app).focus_node.is_some() {
                if let Some(local) = app.get(self).focus_node.handle() {
                    local.dispose(app);
                    app.get_mut(self).focus_node = None;
                }
            } else if app.get(self).focus_node.is_none() {
                app.get_mut(self).focus_node = Some(FocusNode::new(app));
            }
            let focus = self.effective_focus_node(app);
            focus.add_listener(
                app,
                Listener::handle_method(self, Self::handle_focus_changed),
            );
        }
        let enabled = self.widget(app).enabled;
        self.effective_focus_node(app)
            .set_can_request_focus(app, enabled);
        self.listen_to_controller(app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        AutomaticKeepAliveClientMixin::deactivate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.unlisten_controller(app);
        let focus = self.effective_focus_node(app);
        focus.remove_listener(
            app,
            &Listener::handle_method(self, Self::handle_focus_changed),
        );
        if let Some(focus) = app.get(self).focus_node.handle() {
            focus.dispose(app);
        }
        if let Some(controller) = app.get(self).controller.handle() {
            RestorableProperty::dispose(controller, app);
        }
        self.dispose_restoration(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let _ = AutomaticKeepAliveClientMixin::build(self, app);
        self.listen_to_controller(app);
        debug_assert_widget_invariants(self, app);

        let widget_enabled = self.widget(app).enabled;
        let read_only = self.widget(app).read_only;
        let autofocus = self.widget(app).autofocus;
        let obscure_text = self.widget(app).obscure_text;
        let max_lines = self.widget(app).max_lines;
        let min_lines = self.widget(app).min_lines;
        let expands = self.widget(app).expands;
        let text_align = self.widget(app).text_align;
        let text_direction = self.widget(app).text_direction;
        let text_capitalization = self.widget(app).text_capitalization;
        let keyboard_type = self
            .widget(app)
            .keyboard_type
            .unwrap_or(if max_lines == Some(1) {
                TextInputType::TEXT
            } else {
                TextInputType::MULTILINE
            });
        let smart_dashes_type = self
            .widget(app)
            .smart_dashes_type
            .unwrap_or(if obscure_text {
                SmartDashesType::Disabled
            } else {
                SmartDashesType::Enabled
            });
        let smart_quotes_type = self
            .widget(app)
            .smart_quotes_type
            .unwrap_or(if obscure_text {
                SmartQuotesType::Disabled
            } else {
                SmartQuotesType::Enabled
            });
        let enable_interactive_selection = self.widget(app).selection_enabled();
        let cursor_offset = Offset::new(
            IOS_HORIZONTAL_CURSOR_OFFSET_PIXELS / MediaQuery::device_pixel_ratio_of(app, context),
            0.0,
        );

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

        let theme_data = CupertinoTheme::of(app, context);
        let style = self.widget(app).style.clone();
        let resolved_style = style.map(|style| {
            let mut resolved = style.copy_with();
            if let Some(color) =
                CupertinoDynamicColor::maybe_resolve(style.color.as_ref(), app, context)
            {
                resolved = resolved.color(color);
            }
            if let Some(color) =
                CupertinoDynamicColor::maybe_resolve(style.background_color.as_ref(), app, context)
            {
                resolved = resolved.background_color(color);
            }
            resolved
        });
        let text_style = theme_data
            .text_theme()
            .text_style()
            .merge(resolved_style.as_ref());

        let placeholder_style_in = self.widget(app).placeholder_style.clone();
        let resolved_placeholder_style = placeholder_style_in.map(|style| {
            let mut resolved = style.copy_with();
            if let Some(color) =
                CupertinoDynamicColor::maybe_resolve(style.color.as_ref(), app, context)
            {
                resolved = resolved.color(color);
            }
            if let Some(color) =
                CupertinoDynamicColor::maybe_resolve(style.background_color.as_ref(), app, context)
            {
                resolved = resolved.background_color(color);
            }
            resolved
        });
        let placeholder_style = text_style.merge(resolved_placeholder_style.as_ref());

        let keyboard_appearance = self.widget(app).keyboard_appearance;
        let keyboard_appearance =
            keyboard_appearance.unwrap_or_else(|| CupertinoTheme::brightness_of(app, context));
        let cursor_color_in = self.widget(app).cursor_color.clone();
        let cursor_color =
            CupertinoDynamicColor::maybe_resolve(cursor_color_in.as_ref(), app, context)
                .unwrap_or_else(|| theme_data.primary_color());
        let disabled_color =
            CupertinoDynamicColor::resolve(&K_DISABLED_BACKGROUND.into_any(), app, context);
        let decoration = self.widget(app).decoration.clone();
        let decoration_color = CupertinoDynamicColor::maybe_resolve(
            decoration
                .as_ref()
                .and_then(|decoration| decoration.color.as_ref()),
            app,
            context,
        );
        let is_default_decoration =
            decoration.as_ref() == Some(&k_default_rounded_border_decoration());
        let resolved_border = if is_default_decoration {
            let color = CupertinoDynamicColor::resolve(
                &K_DEFAULT_ROUNDED_BORDER_SIDE_COLOR.into_any(),
                app,
                context,
            )
            .color();
            Some(Border::from_border_side(BorderSide::new(
                color,
                0.0,
                BorderStyle::Solid,
                BorderSide::STROKE_ALIGN_INSIDE,
            )))
        } else {
            resolve_box_border(
                decoration
                    .as_ref()
                    .and_then(|decoration| decoration.border.as_deref()),
                app,
                context,
            )
        };
        // Use the default disabled color only if the box decoration was not set.
        let effective_decoration = decoration.map(|decoration| {
            let color = if widget_enabled {
                decoration_color.clone()
            } else if is_default_decoration {
                Some(disabled_color.clone())
            } else {
                decoration.color.clone()
            };
            let mut resolved = decoration.copy_with();
            if let Some(border) = resolved_border {
                resolved = resolved.border(border);
            }
            if let Some(color) = color {
                resolved = resolved.color(color);
            }
            resolved
        });

        let selection_color =
            theme_data
                .primary_color()
                .color()
                .with_values(Some(0.2), None, None, None, None);
        let spell_check_configuration = CupertinoTextField::infer_ios_spell_check_configuration(
            self.widget(app).spell_check_configuration.as_ref(),
        );
        let controller = self.effective_controller(app);
        let focus_node = self.effective_focus_node(app);
        let has_focus = focus_node.has_focus(app);
        let background_cursor_color =
            CupertinoDynamicColor::resolve(&CupertinoColors::INACTIVE_GRAY, app, context).color();

        let mut editable = EditableText::new(
            controller,
            focus_node,
            text_style.clone(),
            cursor_color.color(),
            background_cursor_color,
        )
        .read_only(read_only || !widget_enabled)
        .show_selection_handles(false)
        .keyboard_type(keyboard_type)
        .text_capitalization(text_capitalization)
        .text_align(text_align)
        .autofocus(autofocus)
        .obscuring_character(self.widget(app).obscuring_character.clone())
        .obscure_text(obscure_text)
        .smart_dashes_type(smart_dashes_type)
        .smart_quotes_type(smart_quotes_type)
        .enable_suggestions(self.widget(app).enable_suggestions)
        .max_lines(max_lines)
        .expands(expands)
        .group_id(self.widget(app).group_id)
        .input_formatters(formatters)
        .renderer_ignores_pointer(true)
        .cursor_width(self.widget(app).cursor_width)
        .cursor_radius(self.widget(app).cursor_radius)
        .cursor_opacity_animates(self.widget(app).cursor_opacity_animates)
        .cursor_offset(cursor_offset)
        .paint_cursor_above_text(true)
        .autocorrection_text_rect_color(selection_color)
        .scroll_padding(self.widget(app).scroll_padding)
        .keyboard_appearance(keyboard_appearance)
        .drag_start_behavior(self.widget(app).drag_start_behavior)
        .enable_interactive_selection(enable_interactive_selection)
        .clip_behavior(self.widget(app).clip_behavior)
        .restoration_id("editable")
        .scribble_enabled(self.widget(app).scribble_enabled)
        .stylus_handwriting_enabled(self.widget(app).stylus_handwriting_enabled)
        .enable_ime_personalized_learning(self.widget(app).enable_ime_personalized_learning)
        .spell_check_configuration(spell_check_configuration)
        .key(app.get(self).editable_text_key.clone().into());

        if let Some(show_cursor) = self.widget(app).show_cursor {
            editable = editable.show_cursor(show_cursor);
        }
        if let Some(text_input_action) = self.widget(app).text_input_action {
            editable = editable.text_input_action(text_input_action);
        }
        if let Some(text_direction) = text_direction {
            editable = editable.text_direction(text_direction);
        }
        if let Some(autocorrect) = self.widget(app).autocorrect {
            editable = editable.autocorrect(autocorrect);
        }
        if let Some(min_lines) = min_lines {
            editable = editable.min_lines(min_lines);
        }
        if has_focus {
            editable = editable.selection_color(selection_color);
        }
        if let Some(on_changed) = self.widget(app).on_changed.clone() {
            editable = editable.on_changed(move |app, text| on_changed(app, text));
        }
        if let Some(on_editing_complete) = self.widget(app).on_editing_complete.clone() {
            editable = editable.on_editing_complete(move |app| on_editing_complete(app));
        }
        if let Some(on_submitted) = self.widget(app).on_submitted.clone() {
            editable = editable.on_submitted(move |app, text| on_submitted(app, text));
        }
        if let Some(on_tap_outside) = self.widget(app).on_tap_outside.clone() {
            editable = editable.on_tap_outside(move |app, event| on_tap_outside(app, event));
        }
        if let Some(on_tap_up_outside) = self.widget(app).on_tap_up_outside.clone() {
            editable = editable.on_tap_up_outside(move |app, event| on_tap_up_outside(app, event));
        }
        if let Some(cursor_height) = self.widget(app).cursor_height {
            editable = editable.cursor_height(cursor_height);
        }
        if let Some(selection_height_style) = self.widget(app).selection_height_style {
            editable = editable.selection_height_style(selection_height_style);
        }
        if let Some(selection_width_style) = self.widget(app).selection_width_style {
            editable = editable.selection_width_style(selection_width_style);
        }
        if let Some(select_all_on_focus) = self.widget(app).select_all_on_focus {
            editable = editable.select_all_on_focus(select_all_on_focus);
        }
        if let Some(scroll_controller) = self.widget(app).scroll_controller.clone() {
            editable = editable.scroll_controller(scroll_controller);
        }
        if let Some(scroll_physics) = self.widget(app).scroll_physics.clone() {
            editable = editable.scroll_physics(scroll_physics);
        }
        if let Some(toolbar_options) = self.widget(app).toolbar_options {
            editable = editable.toolbar_options(toolbar_options);
        }
        if !self.widget(app).autofill_hints.is_empty() {
            editable = editable.autofill_hints(self.widget(app).autofill_hints.clone());
        }
        if let Some(content_insertion_configuration) =
            self.widget(app).content_insertion_configuration.clone()
        {
            editable = editable.content_insertion_configuration(content_insertion_configuration);
        }
        if let Some(context_menu_builder) = self.widget(app).context_menu_builder.clone() {
            editable = editable.context_menu_builder(context_menu_builder);
        }
        editable = editable.magnifier_configuration(
            self.widget(app)
                .magnifier_configuration
                .clone()
                .unwrap_or_else(CupertinoTextField::ios_magnifier_configuration),
        );
        if let Some(undo_controller) = self.widget(app).undo_controller.clone() {
            editable = editable.undo_controller(undo_controller);
        }
        if let Some(enable_inline_prediction) = self.widget(app).enable_inline_prediction {
            editable = editable.enable_inline_prediction(enable_inline_prediction);
        }
        let mut text_selection_controls = self.widget(app).selection_controls.clone();
        let mut handle_did_gain_accessibility_focus: Option<VoidCallback> = None;
        let mut handle_did_lose_accessibility_focus: Option<VoidCallback> = None;
        match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::Android | TargetPlatform::Fuchsia => {
                if text_selection_controls.is_none() {
                    text_selection_controls = Some(cupertino_text_selection_handle_controls());
                }
            }
            TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => {
                if text_selection_controls.is_none() {
                    text_selection_controls =
                        Some(cupertino_desktop_text_selection_handle_controls());
                }
                handle_did_gain_accessibility_focus = Some(Rc::new(move |app: &mut App| {
                    // Automatically activate the TextField when it receives accessibility focus.
                    let focus_node = self.effective_focus_node(app);
                    if !focus_node.has_focus(app) && focus_node.can_request_focus(app) {
                        focus_node.request_focus(app, None);
                    }
                }));
                handle_did_lose_accessibility_focus = Some(Rc::new(move |app: &mut App| {
                    self.effective_focus_node(app)
                        .unfocus(app, UnfocusDisposition::Scope);
                }));
            }
        }
        if let Some(selection_controls) = text_selection_controls
            && enable_interactive_selection
        {
            editable = editable.selection_controls(selection_controls);
        }

        let bucket = self.bucket(app).and_then(|bucket| bucket.upgrade(app));
        let padded_editable = Padding::new(self.widget(app).padding).child(
            RepaintBoundary::new().child(UnmanagedRestorationScope::new(editable).bucket(bucket)),
        );
        let text_align_vertical = self.text_align_vertical(app);
        let attachments = self.add_text_dependent_attachments(
            app,
            context,
            padded_editable.into_widget(),
            &placeholder_style,
        );
        let aligned = Align::new()
            .alignment(AlignmentGeometry::Alignment(Alignment::new(
                -1.0,
                text_align_vertical.y,
            )))
            .width_factor(1.0)
            .height_factor(1.0)
            .child(attachments);
        let builder = app
            .get(self)
            .selection_gesture_detector_builder
            .handle()
            .expect("created in initState");
        let gestured = TextSelectionGestureDetectorBuilder::build_gesture_detector(
            builder,
            app,
            None,
            Some(HitTestBehavior::Translucent),
            aligned.into_widget(),
        );
        let mut container = Container::new();
        if let Some(decoration) = effective_decoration {
            container = container.decoration(decoration);
        } else if !widget_enabled {
            container = container.color(disabled_color);
        }
        let mut semantics = Semantics::new().enabled(widget_enabled);
        if widget_enabled && !read_only {
            semantics = semantics.on_tap(Rc::new(move |app: &mut App| {
                let controller = self.effective_controller(app);
                if !controller.selection(app).is_valid() {
                    let length = controller.text_value(app).encode_utf16().count() as i32;
                    controller.set_selection(
                        app,
                        TextSelection::collapsed(length, TextAffinity::Downstream),
                    );
                }
                self.request_keyboard(app);
            }));
        }
        if let Some(handle_did_gain_accessibility_focus) = handle_did_gain_accessibility_focus {
            semantics =
                semantics.on_did_gain_accessibility_focus(handle_did_gain_accessibility_focus);
        }
        if let Some(handle_did_lose_accessibility_focus) = handle_did_lose_accessibility_focus {
            semantics =
                semantics.on_did_lose_accessibility_focus(handle_did_lose_accessibility_focus);
        }
        if widget_enabled {
            semantics = semantics.on_focus(Rc::new(move |app: &mut App| {
                let focus_node = self.effective_focus_node(app);
                debug_assert!(
                    focus_node.can_request_focus(app),
                    "Received SemanticsAction.focus from the engine. However, the FocusNode of \
                     this text field cannot gain focus. This likely indicates a bug. If this text \
                     field cannot be focused (e.g. because it is not enabled), then its \
                     corresponding semantics node must be configured such that the assistive \
                     technology cannot request focus on it."
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
        semantics
            .child(TextFieldTapRegion::new(
                IgnorePointer::new()
                    .ignoring(!widget_enabled)
                    .child(container.child(gestured)),
            ))
            .into_widget()
    }
}

impl TextSelectionGestureDetectorBuilderDelegate for CupertinoTextFieldState {
    fn editable_text_key(self: Handle<Self>, app: &App) -> GlobalKey {
        app.get(self).editable_text_key.clone()
    }

    fn force_press_enabled(self: Handle<Self>, _app: &App) -> bool {
        true
    }

    fn selection_enabled(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).selection_enabled()
    }
}

fn default_placeholder_style() -> TextStyle {
    TextStyle::new()
        .font_weight(FontWeight::W400)
        .color(CupertinoColors::PLACEHOLDER_TEXT)
}

fn k_default_rounded_border_decoration() -> BoxDecoration {
    BoxDecoration::new()
        .color(K_DEFAULT_ROUNDED_BORDER_FILL.into_any())
        .border(Border::from_border_side(BorderSide::new(
            K_DEFAULT_ROUNDED_BORDER_SIDE_COLOR.color,
            0.0,
            BorderStyle::Solid,
            BorderSide::STROKE_ALIGN_INSIDE,
        )))
        .border_radius(BorderRadius::all(Radius::circular(5.0)))
}

fn resolve_box_border(
    border: Option<&dyn waft_painting::BoxBorder>,
    app: &mut App,
    context: BuildContext,
) -> Option<Border> {
    let border = border?;
    let border = border.as_any().downcast_ref::<Border>()?;
    let mut resolve_side = |side: BorderSide| {
        if side == BorderSide::NONE {
            side
        } else {
            side.copy_with(
                Some(
                    CupertinoDynamicColor::resolve(&AnyColor::new(side.color), app, context)
                        .color(),
                ),
                None,
                None,
                None,
            )
        }
    };
    Some(Border {
        top: resolve_side(border.top),
        left: resolve_side(border.left),
        bottom: resolve_side(border.bottom),
        right: resolve_side(border.right),
    })
}

fn debug_assert_widget_invariants(state: Handle<CupertinoTextFieldState>, app: &App) {
    let widget = state.widget(app);
    debug_assert!(widget.obscuring_character.chars().count() == 1);
    debug_assert!(widget.max_lines.is_none_or(|lines| lines > 0));
    debug_assert!(widget.min_lines.is_none_or(|lines| lines > 0));
    debug_assert!(
        widget
            .max_lines
            .is_none_or(|max| widget.min_lines.is_none_or(|min| max >= min)),
        "minLines can't be greater than maxLines"
    );
    debug_assert!(!widget.expands || (widget.max_lines.is_none() && widget.min_lines.is_none()));
    debug_assert!(!widget.obscure_text || widget.max_lines == Some(1));
    debug_assert!(widget.max_length.is_none_or(|length| length > 0));
}

#[cfg(test)]
mod tests {
    use super::*;
    use waft_widgets::TextEditingController;

    use crate::adaptive_text_selection_toolbar::CupertinoAdaptiveTextSelectionToolbar;
    use crate::app::CupertinoApp;
    use waft_embedder::{TextAffinity, TextSelection};
    use waft_gestures::{K_PRIMARY_BUTTON, K_SECONDARY_BUTTON};
    use waft_rendering::HitTestBehavior;
    use waft_services::{LogicalKeyboardKey, PhysicalKeyboardKey, TextInputClient};
    use waft_widgets::testing::{
        TargetPlatformVariant, TestPointer, WidgetTester, expect, find, finds_widgets,
        test_widgets, test_widgets_variant,
    };
    use waft_widgets::{Column, Expanded, Listener, SizedBox};

    /// The platform of the mouse and keyboard cases: they pin the Apple key bindings and
    /// selection gestures, as Dart's cases of them run under `TargetPlatformVariant.only`.
    fn on_ios(body: impl Fn(&mut WidgetTester) + Sync) {
        test_widgets_variant(TargetPlatformVariant::only(TargetPlatform::IOS), body);
    }

    #[test]
    fn default_decoration_is_the_rounded_border() {
        assert_eq!(
            CupertinoTextField::new().decoration,
            Some(k_default_rounded_border_decoration())
        );
        assert!(CupertinoTextField::borderless().decoration.is_none());
        assert_eq!(
            k_default_rounded_border_decoration(),
            k_default_rounded_border_decoration()
        );
    }

    #[test]
    fn a_text_field_with_a_placeholder_builds() {
        test_widgets(|tester| {
            tester.pump_widget(
                CupertinoApp::new().home(
                    CupertinoTextField::new()
                        .placeholder("Name")
                        .autofocus(true),
                ),
            );
        });
    }

    /// A mouse click with `buttons` at `at`: the mouse hovers there, presses and releases.
    fn click(tester: &mut WidgetTester, at: Offset, pointer: i64, buttons: i64) {
        let mut mouse = TestPointer::mouse(pointer, buttons);
        tester.down(&mut mouse, at);
        tester.up(&mouse);
    }

    /// A right click at `at`, and the frame it asks for.
    fn right_click(tester: &mut WidgetTester, at: Offset, pointer: i64) {
        click(tester, at, pointer, K_SECONDARY_BUTTON);
        tester.pump();
    }

    /// The magnifier the `EditableText` of `CupertinoApp(home: CupertinoTextField())` builds on
    /// the tester's platform, as the magnifier builder tests call its builder.
    fn default_magnifier(tester: &mut WidgetTester) -> Option<WidgetRef> {
        tester.pump_widget(CupertinoApp::new().home(CupertinoTextField::new()));
        let context = tester.element(find::by_type::<CupertinoTextField>());
        let magnifier_builder = tester.widget(
            find::by_type::<EditableText>(),
            |editable_text: &EditableText| {
                editable_text.magnifier_configuration.magnifier_builder()
            },
        );
        let mut app = tester.cell.borrow_mut();
        let controller = MagnifierController::new(&mut app);
        let magnifier_info = app.create(ValueNotifier::new(MagnifierInfo::EMPTY));
        magnifier_builder(&mut app, context, controller, magnifier_info)
    }

    #[test]
    fn should_build_cupertino_magnifier_on_ios_and_android() {
        let variant = TargetPlatformVariant::new([TargetPlatform::IOS, TargetPlatform::Android]);
        test_widgets_variant(variant, |tester| {
            let magnifier = default_magnifier(tester).expect("a magnifier");
            assert!(waft_widgets::downcast_widget::<CupertinoTextMagnifier>(&*magnifier).is_some());
        });
    }

    #[test]
    fn should_build_nothing_on_all_platforms_but_ios_and_android() {
        let variant =
            TargetPlatformVariant::all_excluding(&[TargetPlatform::IOS, TargetPlatform::Android]);
        test_widgets_variant(variant, |tester| {
            assert!(default_magnifier(tester).is_none());
        });
    }

    #[test]
    fn the_default_magnifier_configuration_keeps_one_builder() {
        assert!(Rc::ptr_eq(
            &CupertinoTextField::ios_magnifier_configuration().magnifier_builder(),
            &CupertinoTextField::ios_magnifier_configuration().magnifier_builder(),
        ));
    }

    fn field_point(app: &mut App, key: &GlobalKey, local: Offset) -> Offset {
        let context = key.current_context(app).expect("the field mounted");
        let render_box = context
            .find_render_object(app)
            .expect("the field has a render object")
            .as_box()
            .expect("the field is a box");
        render_box.local_to_global(app, local, None)
    }

    fn mount_field(
        tester: &mut WidgetTester,
        text: &str,
    ) -> (Ref<TextEditingController>, GlobalKey) {
        let controller = tester.update(TextEditingController::new);
        tester.update(|app| controller.set_text(app, text));
        let key = GlobalKey::new();
        let key_ref: KeyRef = key.clone().into();
        tester.pump_widget(
            CupertinoApp::new().home(
                CupertinoTextField::new()
                    .key(key_ref)
                    .controller(controller.clone().into_erased()),
            ),
        );
        (controller, key)
    }

    /// Clicks the field so it takes focus, which is what puts it under the shortcuts.
    fn focus_field(tester: &mut WidgetTester, key: &GlobalKey) {
        let at = field_point(&mut tester.cell.borrow_mut(), key, Offset::new(12.0, 12.0));
        click(tester, at, 1, K_PRIMARY_BUTTON);
        tester.pump();
    }

    fn utf16_len(text: &str) -> i32 {
        text.encode_utf16().count() as i32
    }

    /// A grapheme of four code points joined by zero-width joiners: eleven UTF-16 code units
    /// that the caret must cross in one step.
    const FAMILY: &str = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";

    /// A field at the top of the page, with an opaque region under it to click on.
    ///
    /// `mount_field` gives the field the whole view, so there is no outside to tap.
    fn mount_field_with_room_below(
        tester: &mut WidgetTester,
        text: &str,
    ) -> (Ref<TextEditingController>, GlobalKey) {
        let controller = tester.update(TextEditingController::new);
        tester.update(|app| controller.set_text(app, text));
        let key = GlobalKey::new();
        let key_ref: KeyRef = key.clone().into();
        tester.pump_widget(
            CupertinoApp::new().home(
                Column::new().children([
                    SizedBox::new()
                        .height(40.0)
                        .child(
                            CupertinoTextField::new()
                                .key(key_ref)
                                .controller(controller.clone().into_erased()),
                        )
                        .into_widget(),
                    // Something for a click below the field to land on, the way the gallery's
                    // scrollable covers its page.
                    Expanded::new(
                        Listener::new()
                            .behavior(HitTestBehavior::Opaque)
                            .child(SizedBox::expand()),
                    )
                    .into_widget(),
                ]),
            ),
        );
        (controller, key)
    }

    #[test]
    fn a_click_outside_the_field_drops_its_focus() {
        on_ios(|tester| {
            let (_controller, key) = mount_field_with_room_below(tester, "Hello world");
            focus_field(tester, &key);
            let state = key
                .current_state::<CupertinoTextFieldState>(&tester.cell.borrow())
                .expect("the field mounted");
            let has_focus = |tester: &WidgetTester| {
                let mut app = tester.cell.borrow_mut();
                state.effective_focus_node(&app).has_focus(&mut app)
            };
            assert!(has_focus(tester), "clicking the field should focus it");

            // Well below the field. The tap region surface is what turns this into the field's
            // on_tap_outside.
            click(tester, Offset::new(200.0, 250.0), 1, K_PRIMARY_BUTTON);
            tester.pump();

            assert!(
                !has_focus(tester),
                "a click outside the field should unfocus it"
            );
        });
    }

    #[test]
    fn backspace_deletes_a_whole_grapheme() {
        on_ios(|tester| {
            // The host reports plain key presses, so the field keeps its own key bindings and
            // backspace reaches them.
            let text = format!("a{FAMILY}");
            let (controller, key) = mount_field(tester, &text);
            focus_field(tester, &key);
            tester.update(|app| {
                controller.set_selection(
                    app,
                    TextSelection::collapsed(utf16_len(&text), TextAffinity::Downstream),
                )
            });
            tester.pump();

            tester.send_key_event(
                PhysicalKeyboardKey::BACKSPACE,
                LogicalKeyboardKey::BACKSPACE,
            );
            tester.pump();

            assert_eq!(
                controller.value(&tester.cell.borrow()).text,
                "a",
                "one backspace should take the whole family emoji"
            );
        });
    }

    #[test]
    fn a_right_click_keeps_a_selection_it_lands_on_and_takes_the_word_otherwise() {
        on_ios(|tester| {
            let (controller, key) = mount_field(tester, "Hello world again");
            focus_field(tester, &key);
            tester.update(|app| controller.set_selection(app, TextSelection::new(0, 11)));
            tester.pump();

            let inside = field_point(&mut tester.cell.borrow_mut(), &key, Offset::new(20.0, 12.0));
            right_click(tester, inside, 2);
            let kept = controller.selection(&tester.cell.borrow());
            assert_eq!(
                (kept.start(), kept.end()),
                (0, 11),
                "a right click on the selection leaves it alone, so the menu acts on it"
            );

            // The middle of the third glyph of "again": after the field's 7 of padding,
            // FlutterTest's glyphs are 17 wide at the field's 17-point text.
            let outside = field_point(
                &mut tester.cell.borrow_mut(),
                &key,
                Offset::new(7.0 + 14.5 * 17.0, 12.0),
            );
            right_click(tester, outside, 3);
            let word = controller.selection(&tester.cell.borrow());
            assert_eq!(
                (word.start(), word.end()),
                (12, 17),
                "a right click off the selection takes the word under it"
            );
        });
    }

    #[test]
    fn a_right_button_drag_selects_nothing() {
        on_ios(|tester| {
            let (controller, key) = mount_field(tester, "Hello world again");
            focus_field(tester, &key);
            tester.update(|app| {
                controller.set_selection(app, TextSelection::collapsed(0, TextAffinity::Downstream))
            });
            tester.pump();

            // The drift a hand makes between pressing the right button and reaching the menu.
            let start = field_point(&mut tester.cell.borrow_mut(), &key, Offset::new(20.0, 12.0));
            let mut mouse = TestPointer::mouse(2, K_SECONDARY_BUTTON);
            tester.down(&mut mouse, start);
            for step in 1..=4 {
                let to = Offset::new(start.dx() + f64::from(step) * 25.0, start.dy());
                tester.move_to(&mut mouse, to, true);
            }
            tester.up(&mouse);
            tester.pump();

            let selection = controller.selection(&tester.cell.borrow());
            assert!(
                selection.is_collapsed(),
                "the right button never drag-selects, got {selection:?}"
            );
        });
    }

    #[test]
    fn meta_a_selects_all_the_text() {
        on_ios(|tester| {
            let (controller, key) = mount_field(tester, "Hello world");
            focus_field(tester, &key);

            tester.send_key_down_event(
                PhysicalKeyboardKey::META_LEFT,
                LogicalKeyboardKey::META_LEFT,
            );
            tester.send_key_down_event_with_character(
                PhysicalKeyboardKey::KEY_A,
                LogicalKeyboardKey::KEY_A,
                "a",
            );
            tester.send_key_up_event(PhysicalKeyboardKey::KEY_A, LogicalKeyboardKey::KEY_A);
            tester.send_key_up_event(
                PhysicalKeyboardKey::META_LEFT,
                LogicalKeyboardKey::META_LEFT,
            );
            tester.pump();

            let selection = controller.selection(&tester.cell.borrow());
            assert_eq!(
                (selection.start(), selection.end()),
                (0, utf16_len("Hello world")),
                "meta+A should select the whole field, got {selection:?}"
            );
        });
    }

    #[test]
    fn an_arrow_key_walks_the_caret_over_a_whole_grapheme() {
        on_ios(|tester| {
            let text = format!("a{FAMILY}b");
            let (controller, key) = mount_field(tester, &text);
            focus_field(tester, &key);
            tester.update(|app| {
                controller.set_selection(app, TextSelection::collapsed(0, TextAffinity::Downstream))
            });
            tester.pump();
            let arrow_right = |tester: &mut WidgetTester| {
                tester.send_key_event(
                    PhysicalKeyboardKey::ARROW_RIGHT,
                    LogicalKeyboardKey::ARROW_RIGHT,
                );
                tester.pump();
            };

            arrow_right(tester);
            assert_eq!(
                controller.selection(&tester.cell.borrow()).base_offset,
                1,
                "past the 'a'"
            );

            arrow_right(tester);
            assert_eq!(
                controller.selection(&tester.cell.borrow()).base_offset,
                utf16_len(&text) - 1,
                "one press should step over the whole family emoji"
            );
        });
    }

    #[test]
    fn shift_and_an_arrow_key_extend_the_selection() {
        on_ios(|tester| {
            let (controller, key) = mount_field(tester, "Hello world");
            focus_field(tester, &key);
            tester.update(|app| {
                controller.set_selection(app, TextSelection::collapsed(0, TextAffinity::Downstream))
            });
            tester.pump();

            tester.send_key_down_event(
                PhysicalKeyboardKey::SHIFT_LEFT,
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            tester.send_key_event(
                PhysicalKeyboardKey::ARROW_RIGHT,
                LogicalKeyboardKey::ARROW_RIGHT,
            );
            tester.send_key_up_event(
                PhysicalKeyboardKey::SHIFT_LEFT,
                LogicalKeyboardKey::SHIFT_LEFT,
            );
            tester.pump();

            let selection = controller.selection(&tester.cell.borrow());
            assert_eq!((selection.base_offset, selection.extent_offset), (0, 1));
        });
    }

    #[test]
    fn the_macos_delete_selector_removes_a_whole_grapheme() {
        on_ios(|tester| {
            // What `NSStandardKeyBindingResponding` sends for the backspace key. The macOS and
            // iOS shortcut tables leave backspace to the host, so this is the path it takes.
            let text = format!("a{FAMILY}");
            let (controller, key) = mount_field(tester, &text);
            focus_field(tester, &key);
            let end = utf16_len(&text);
            tester.update(|app| {
                controller
                    .set_selection(app, TextSelection::collapsed(end, TextAffinity::Downstream))
            });
            tester.pump();

            tester.update(|app| {
                let state = key
                    .current_state::<CupertinoTextFieldState>(app)
                    .expect("the field mounted");
                let editable_key = state.editable_text_key(app);
                let editable = editable_key
                    .current_state::<EditableTextState>(app)
                    .expect("the field mounts an EditableText");
                TextInputClient::perform_selector(editable, app, "deleteBackward:");
            });
            tester.pump();

            assert_eq!(
                controller.value(&tester.cell.borrow()).text,
                "a",
                "one deleteBackward: should take the whole family emoji"
            );
        });
    }

    #[test]
    fn a_mouse_click_moves_the_caret() {
        on_ios(|tester| {
            let (controller, key) = mount_field(tester, "Hello world");
            let at = field_point(&mut tester.cell.borrow_mut(), &key, Offset::new(12.0, 12.0));
            click(tester, at, 1, K_PRIMARY_BUTTON);
            tester.pump();
            let selection = controller.selection(&tester.cell.borrow());
            assert!(selection.is_collapsed(), "{selection:?}");
            assert!(selection.base_offset >= 0);
        });
    }

    #[test]
    fn a_mouse_drag_selects_a_range() {
        on_ios(|tester| {
            let (controller, key) = mount_field(tester, "Hello world");
            let start = field_point(&mut tester.cell.borrow_mut(), &key, Offset::new(8.0, 12.0));
            let end = field_point(&mut tester.cell.borrow_mut(), &key, Offset::new(80.0, 12.0));
            let mut mouse = TestPointer::mouse(1, K_PRIMARY_BUTTON);
            tester.down(&mut mouse, start);
            let mid = Offset::new((start.dx() + end.dx()) / 2.0, start.dy());
            tester.move_to(&mut mouse, mid, true);
            tester.move_to(&mut mouse, end, true);
            tester.up(&mouse);
            tester.pump();
            let selection = controller.selection(&tester.cell.borrow());
            assert!(
                !selection.is_collapsed(),
                "expected a range, got {selection:?}"
            );
        });
    }

    #[test]
    fn a_mouse_double_click_selects_a_word() {
        on_ios(|tester| {
            let (controller, key) = mount_field(tester, "Hello world");
            let at = field_point(&mut tester.cell.borrow_mut(), &key, Offset::new(16.0, 12.0));
            click(tester, at, 1, K_PRIMARY_BUTTON);
            click(tester, at, 2, K_PRIMARY_BUTTON);
            tester.pump();
            let selection = controller.selection(&tester.cell.borrow());
            assert!(
                !selection.is_collapsed() && selection.end() - selection.start() > 1,
                "expected a word, got {selection:?}"
            );
        });
    }

    #[test]
    fn a_secondary_mouse_click_shows_the_selection_toolbar() {
        on_ios(|tester| {
            let (_controller, key) = mount_field(tester, "Hello world");
            let at = field_point(&mut tester.cell.borrow_mut(), &key, Offset::new(16.0, 12.0));
            click(tester, at, 1, K_SECONDARY_BUTTON);
            tester.pump();
            // Right-click should insert the adaptive selection toolbar.
            expect(
                tester,
                &find::by_type::<CupertinoAdaptiveTextSelectionToolbar>().skip_offstage(false),
                finds_widgets(),
            );
        });
    }
}
