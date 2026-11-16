//! Flutter counterpart: material_ui's `text_form_field.dart`.
//!
//! [`TextFormField`] is a `FormField<String>` leaf: its widget carries the form field's
//! [`FormFieldData`] and the arguments Dart's constructor captures in the builder closure, kept
//! as the [`TextField`] the builder builds; [`TextFormField::build_field`] is that closure.
#![allow(deprecated)]

use std::any::TypeId;
use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{
    BoxHeightStyle, BoxWidthStyle, Brightness, Clip, Color, Locale, Radius, SmartDashesType,
    SmartQuotesType, TextCapitalization, TextEditingValue, TextInputAction, TextInputType,
};
use waft_foundation::{App, Handle, Listenable, Listener, OptionRetained, Ref, ValueChanged};
use waft_gestures::{DragStartBehavior, GestureTapCallback, PointerDownEvent, PointerUpEvent};
use waft_painting::{AnyTextStyle, EdgeInsets, TextAlign, TextAlignVertical, TextDirection};
use waft_services::{
    MaxLengthEnforcement, MouseCursorRef, RestorationBucket, TextInputFormatterRef,
};
use waft_widgets::{
    AnyFormFieldState, AutovalidateMode, BuildContext, ContentInsertionConfiguration, EditableText,
    EditableTextContextMenuBuilder, EditableTextState, FocusNodeRef, FormFieldData, FormFieldLeaf,
    FormFieldState, FormFieldStateData, FormFieldStateLeaf, IntoWidget, KeyRef,
    RestorableListenable, RestorableProperty, RestorableTextEditingController, RestorationMixin,
    RestorationMixinData, ScrollControllerRef, ScrollPhysicsRef, SpellCheckConfiguration, State,
    StateData, StatefulWidget, SystemContextMenu, TextEditingControllerRef,
    TextMagnifierConfiguration, TextSelectionControls, ToolbarOptions, UndoHistoryController,
    UnmanagedRestorationScope, WidgetRef,
};

use crate::adaptive_text_selection_toolbar::AdaptiveTextSelectionToolbar;
use crate::input_decorator::{InputDecoration, InputDecorationTheme};
use crate::material_state::MaterialStatesController;
use crate::text_field::TextField;

/// A [`FormField`](waft_widgets::FormField) that contains a [`TextField`].
///
/// This is a convenience widget that wraps a [`TextField`] widget in a `FormField`.
///
/// A `Form` ancestor is not required. The `Form` allows one to save, reset, or validate
/// multiple fields at once. To use without a `Form`, pass a `GlobalKey` to the constructor and
/// use `GlobalKey::current_state` to save or reset the form field.
///
/// When a [`controller`](Self::controller) is specified, its `TextEditingController::text`
/// defines the [`initial_value`](FormFieldData::initial_value). If this `FormField` is part of
/// a scrolling container that lazily constructs its children, like a `ListView` or a
/// `CustomScrollView`, then a [`controller`](Self::controller) should be specified. The
/// controller's lifetime should be managed by a stateful widget ancestor of the scrolling
/// container.
///
/// If a [`controller`](Self::controller) is not specified, [`initial_value`](Self::initial_value)
/// can be used to give the automatically generated controller an initial value.
///
/// For a documentation about the various parameters, see the [`TextField`] class and
/// `TextField::new`, the constructor.
pub struct TextFormField {
    form_field: FormFieldData<String>,
    /// {@macro flutter.widgets.editableText.groupId}
    pub group_id: TypeId,
    /// Controls the text being edited.
    ///
    /// If null, this widget will create its own `TextEditingController` and initialize its
    /// `TextEditingController::text` with [`initial_value`](FormFieldData::initial_value).
    pub controller: Option<TextEditingControllerRef>,
    /// Called when the user initiates a change to the TextField's value: when they have
    /// inserted or deleted text or reset the form.
    pub on_changed: Option<ValueChanged<String>>,
    /// Dart's constructor argument `initialValue`, which fills the form field's
    /// [`initial_value`](FormFieldData::initial_value) when there is no
    /// [`controller`](Self::controller).
    initial_value: Option<String>,
    /// The constructor's other arguments, which Dart's builder closure captures: the
    /// [`TextField`] it builds before the state fills the controller, the decoration, the
    /// `on_changed` handler and `enabled`.
    text_field: TextField,
}

impl TextFormField {
    /// Creates a [`FormField`](waft_widgets::FormField) that contains a [`TextField`].
    pub fn new() -> TextFormField {
        let mut form_field = FormFieldData::new(Rc::new(TextFormField::build_field));
        form_field.initial_value = Some(String::new());
        form_field.enabled = true;
        form_field.autovalidate_mode = AutovalidateMode::Disabled;
        TextFormField {
            form_field,
            group_id: TypeId::of::<EditableText>(),
            controller: None,
            on_changed: None,
            initial_value: None,
            text_field: TextField::new()
                .autocorrect(true)
                .context_menu_builder(Some(Rc::new(Self::default_context_menu_builder))),
        }
    }

    /// Dart `TextFormField(key:)`.
    pub fn key(mut self, key: KeyRef) -> TextFormField {
        self.form_field.key = Some(key);
        self
    }

    /// Dart `TextFormField(groupId:)`.
    pub fn group_id(mut self, group_id: TypeId) -> TextFormField {
        self.group_id = group_id;
        self
    }

    /// Dart `TextFormField(controller:)`.
    pub fn controller(mut self, controller: TextEditingControllerRef) -> TextFormField {
        self.controller = Some(controller);
        self.form_field.initial_value = self.form_field_initial_value();
        self
    }

    /// Dart `TextFormField(initialValue:)`.
    pub fn initial_value(mut self, initial_value: impl Into<String>) -> TextFormField {
        self.initial_value = Some(initial_value.into());
        self.form_field.initial_value = self.form_field_initial_value();
        self
    }

    /// Dart `TextFormField(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> TextFormField {
        self.text_field = self.text_field.focus_node(focus_node);
        self
    }

    /// Dart `TextFormField(forceErrorText:)`.
    pub fn force_error_text(mut self, force_error_text: impl Into<String>) -> TextFormField {
        self.form_field.force_error_text = Some(force_error_text.into());
        self
    }

    /// Dart `TextFormField(decoration:)`.
    pub fn decoration(mut self, decoration: Option<InputDecoration>) -> TextFormField {
        self.text_field = self.text_field.decoration(decoration);
        self.form_field.enabled = self.effective_enabled();
        self
    }

    /// Dart `TextFormField(keyboardType:)`.
    pub fn keyboard_type(mut self, keyboard_type: TextInputType) -> TextFormField {
        self.text_field = self.text_field.keyboard_type(keyboard_type);
        self
    }

    /// Dart `TextFormField(textCapitalization:)`.
    pub fn text_capitalization(mut self, text_capitalization: TextCapitalization) -> TextFormField {
        self.text_field = self.text_field.text_capitalization(text_capitalization);
        self
    }

    /// Dart `TextFormField(textInputAction:)`.
    pub fn text_input_action(mut self, text_input_action: TextInputAction) -> TextFormField {
        self.text_field = self.text_field.text_input_action(text_input_action);
        self
    }

    /// Dart `TextFormField(style:)`.
    pub fn style(mut self, style: impl Into<AnyTextStyle>) -> TextFormField {
        self.text_field = self.text_field.style(style);
        self
    }

    /// Dart `TextFormField(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> TextFormField {
        self.text_field = self.text_field.text_direction(text_direction);
        self
    }

    /// Dart `TextFormField(textAlign:)`.
    pub fn text_align(mut self, text_align: TextAlign) -> TextFormField {
        self.text_field = self.text_field.text_align(text_align);
        self
    }

    /// Dart `TextFormField(textAlignVertical:)`.
    pub fn text_align_vertical(mut self, text_align_vertical: TextAlignVertical) -> TextFormField {
        self.text_field = self.text_field.text_align_vertical(text_align_vertical);
        self
    }

    /// Dart `TextFormField(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> TextFormField {
        self.text_field = self.text_field.autofocus(autofocus);
        self
    }

    /// Dart `TextFormField(readOnly:)`.
    pub fn read_only(mut self, read_only: bool) -> TextFormField {
        self.text_field = self.text_field.read_only(read_only);
        self
    }

    /// Dart `TextFormField(toolbarOptions:)`.
    #[deprecated(note = "Use `context_menu_builder` instead.")]
    pub fn toolbar_options(mut self, toolbar_options: ToolbarOptions) -> TextFormField {
        self.text_field = self.text_field.toolbar_options(toolbar_options);
        self
    }

    /// Dart `TextFormField(showCursor:)`.
    pub fn show_cursor(mut self, show_cursor: bool) -> TextFormField {
        self.text_field = self.text_field.show_cursor(show_cursor);
        self
    }

    /// Dart `TextFormField(obscuringCharacter:)`.
    pub fn obscuring_character(mut self, obscuring_character: impl Into<String>) -> TextFormField {
        self.text_field = self.text_field.obscuring_character(obscuring_character);
        self
    }

    /// Dart `TextFormField(obscureText:)`.
    pub fn obscure_text(mut self, obscure_text: bool) -> TextFormField {
        self.text_field = self.text_field.obscure_text(obscure_text);
        self
    }

    /// Dart `TextFormField(autocorrect:)`.
    pub fn autocorrect(mut self, autocorrect: bool) -> TextFormField {
        self.text_field = self.text_field.autocorrect(autocorrect);
        self
    }

    /// Dart `TextFormField(smartDashesType:)`.
    pub fn smart_dashes_type(mut self, smart_dashes_type: SmartDashesType) -> TextFormField {
        self.text_field = self.text_field.smart_dashes_type(smart_dashes_type);
        self
    }

    /// Dart `TextFormField(smartQuotesType:)`.
    pub fn smart_quotes_type(mut self, smart_quotes_type: SmartQuotesType) -> TextFormField {
        self.text_field = self.text_field.smart_quotes_type(smart_quotes_type);
        self
    }

    /// Dart `TextFormField(enableSuggestions:)`.
    pub fn enable_suggestions(mut self, enable_suggestions: bool) -> TextFormField {
        self.text_field = self.text_field.enable_suggestions(enable_suggestions);
        self
    }

    /// Dart `TextFormField(maxLengthEnforcement:)`.
    pub fn max_length_enforcement(
        mut self,
        max_length_enforcement: MaxLengthEnforcement,
    ) -> TextFormField {
        self.text_field = self
            .text_field
            .max_length_enforcement(max_length_enforcement);
        self
    }

    /// Dart `TextFormField(maxLines:)`.
    pub fn max_lines(mut self, max_lines: Option<i32>) -> TextFormField {
        self.text_field = self.text_field.max_lines(max_lines);
        self
    }

    /// Dart `TextFormField(minLines:)`.
    pub fn min_lines(mut self, min_lines: i32) -> TextFormField {
        self.text_field = self.text_field.min_lines(min_lines);
        self
    }

    /// Dart `TextFormField(expands:)`.
    pub fn expands(mut self, expands: bool) -> TextFormField {
        self.text_field = self.text_field.expands(expands);
        self
    }

    /// Dart `TextFormField(maxLength:)`.
    pub fn max_length(mut self, max_length: i32) -> TextFormField {
        self.text_field = self.text_field.max_length(max_length);
        self
    }

    /// Dart `TextFormField(onChanged:)`.
    pub fn on_changed(mut self, on_changed: impl Fn(&mut App, String) + 'static) -> TextFormField {
        self.on_changed = Some(Rc::new(on_changed));
        self
    }

    /// Dart `TextFormField(onTap:)`.
    pub fn on_tap(mut self, on_tap: GestureTapCallback) -> TextFormField {
        self.text_field = self.text_field.on_tap(on_tap);
        self
    }

    /// Dart `TextFormField(onTapAlwaysCalled:)`.
    pub fn on_tap_always_called(mut self, on_tap_always_called: bool) -> TextFormField {
        self.text_field = self.text_field.on_tap_always_called(on_tap_always_called);
        self
    }

    /// Dart `TextFormField(onTapOutside:)`.
    pub fn on_tap_outside(
        mut self,
        on_tap_outside: impl Fn(&mut App, PointerDownEvent) + 'static,
    ) -> TextFormField {
        self.text_field = self.text_field.on_tap_outside(on_tap_outside);
        self
    }

    /// Dart `TextFormField(onTapUpOutside:)`.
    pub fn on_tap_up_outside(
        mut self,
        on_tap_up_outside: impl Fn(&mut App, PointerUpEvent) + 'static,
    ) -> TextFormField {
        self.text_field = self.text_field.on_tap_up_outside(on_tap_up_outside);
        self
    }

    /// Dart `TextFormField(onEditingComplete:)`.
    pub fn on_editing_complete(
        mut self,
        on_editing_complete: impl Fn(&mut App) + 'static,
    ) -> TextFormField {
        self.text_field = self.text_field.on_editing_complete(on_editing_complete);
        self
    }

    /// Dart `TextFormField(onFieldSubmitted:)`.
    pub fn on_field_submitted(
        mut self,
        on_field_submitted: impl Fn(&mut App, String) + 'static,
    ) -> TextFormField {
        self.text_field = self.text_field.on_submitted(on_field_submitted);
        self
    }

    /// Dart `TextFormField(onSaved:)`.
    pub fn on_saved(
        mut self,
        on_saved: impl Fn(&mut App, Option<String>) + 'static,
    ) -> TextFormField {
        self.form_field.on_saved = Some(Rc::new(on_saved));
        self
    }

    /// Dart `TextFormField(validator:)`.
    pub fn validator(
        mut self,
        validator: impl Fn(&mut App, Option<&String>) -> Option<String> + 'static,
    ) -> TextFormField {
        self.form_field.validator = Some(Rc::new(validator));
        self
    }

    /// Dart `TextFormField(errorBuilder:)`.
    pub fn error_builder(
        mut self,
        error_builder: impl Fn(&mut App, BuildContext, String) -> WidgetRef + 'static,
    ) -> TextFormField {
        self.form_field.error_builder = Some(Rc::new(error_builder));
        self
    }

    /// Dart `TextFormField(inputFormatters:)`.
    pub fn input_formatters(
        mut self,
        input_formatters: Vec<TextInputFormatterRef>,
    ) -> TextFormField {
        self.text_field = self.text_field.input_formatters(input_formatters);
        self
    }

    /// Dart `TextFormField(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> TextFormField {
        self.text_field = self.text_field.enabled(enabled);
        self.form_field.enabled = self.effective_enabled();
        self
    }

    /// Dart `TextFormField(ignorePointers:)`.
    pub fn ignore_pointers(mut self, ignore_pointers: bool) -> TextFormField {
        self.text_field = self.text_field.ignore_pointers(ignore_pointers);
        self
    }

    /// Dart `TextFormField(cursorWidth:)`.
    pub fn cursor_width(mut self, cursor_width: f64) -> TextFormField {
        self.text_field = self.text_field.cursor_width(cursor_width);
        self
    }

    /// Dart `TextFormField(cursorHeight:)`.
    pub fn cursor_height(mut self, cursor_height: f64) -> TextFormField {
        self.text_field = self.text_field.cursor_height(cursor_height);
        self
    }

    /// Dart `TextFormField(cursorRadius:)`.
    pub fn cursor_radius(mut self, cursor_radius: Radius) -> TextFormField {
        self.text_field = self.text_field.cursor_radius(cursor_radius);
        self
    }

    /// Dart `TextFormField(cursorColor:)`.
    pub fn cursor_color(mut self, cursor_color: Color) -> TextFormField {
        self.text_field = self.text_field.cursor_color(cursor_color);
        self
    }

    /// Dart `TextFormField(cursorErrorColor:)`.
    pub fn cursor_error_color(mut self, cursor_error_color: Color) -> TextFormField {
        self.text_field = self.text_field.cursor_error_color(cursor_error_color);
        self
    }

    /// Dart `TextFormField(keyboardAppearance:)`.
    pub fn keyboard_appearance(mut self, keyboard_appearance: Brightness) -> TextFormField {
        self.text_field = self.text_field.keyboard_appearance(keyboard_appearance);
        self
    }

    /// Dart `TextFormField(scrollPadding:)`.
    pub fn scroll_padding(mut self, scroll_padding: EdgeInsets) -> TextFormField {
        self.text_field = self.text_field.scroll_padding(scroll_padding);
        self
    }

    /// Dart `TextFormField(enableInteractiveSelection:)`.
    pub fn enable_interactive_selection(
        mut self,
        enable_interactive_selection: bool,
    ) -> TextFormField {
        self.text_field = self
            .text_field
            .enable_interactive_selection(enable_interactive_selection);
        self
    }

    /// Dart `TextFormField(selectAllOnFocus:)`.
    pub fn select_all_on_focus(mut self, select_all_on_focus: bool) -> TextFormField {
        self.text_field = self.text_field.select_all_on_focus(select_all_on_focus);
        self
    }

    /// Dart `TextFormField(selectionControls:)`.
    pub fn selection_controls(
        mut self,
        selection_controls: Rc<dyn TextSelectionControls>,
    ) -> TextFormField {
        self.text_field = self.text_field.selection_controls(selection_controls);
        self
    }

    /// Dart `TextFormField(buildCounter:)`.
    pub fn build_counter(
        mut self,
        build_counter: impl Fn(&mut App, BuildContext, i32, Option<i32>, bool) -> Option<WidgetRef>
        + 'static,
    ) -> TextFormField {
        self.text_field = self.text_field.build_counter(build_counter);
        self
    }

    /// Dart `TextFormField(scrollPhysics:)`.
    pub fn scroll_physics(mut self, scroll_physics: ScrollPhysicsRef) -> TextFormField {
        self.text_field = self.text_field.scroll_physics(scroll_physics);
        self
    }

    /// Dart `TextFormField(autofillHints:)`.
    pub fn autofill_hints(mut self, autofill_hints: Vec<String>) -> TextFormField {
        self.text_field = self.text_field.autofill_hints(autofill_hints);
        self
    }

    /// Dart `TextFormField(autovalidateMode:)`.
    pub fn autovalidate_mode(mut self, autovalidate_mode: AutovalidateMode) -> TextFormField {
        self.form_field.autovalidate_mode = autovalidate_mode;
        self
    }

    /// Dart `TextFormField(scrollController:)`.
    pub fn scroll_controller(mut self, scroll_controller: ScrollControllerRef) -> TextFormField {
        self.text_field = self.text_field.scroll_controller(scroll_controller);
        self
    }

    /// Dart `TextFormField(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> TextFormField {
        let restoration_id = restoration_id.into();
        self.form_field.restoration_id = Some(restoration_id.clone());
        self.text_field = self.text_field.restoration_id(restoration_id);
        self
    }

    /// Dart `TextFormField(enableIMEPersonalizedLearning:)`.
    pub fn enable_ime_personalized_learning(
        mut self,
        enable_ime_personalized_learning: bool,
    ) -> TextFormField {
        self.text_field = self
            .text_field
            .enable_ime_personalized_learning(enable_ime_personalized_learning);
        self
    }

    /// Dart `TextFormField(enableInlinePrediction:)`.
    pub fn enable_inline_prediction(mut self, enable_inline_prediction: bool) -> TextFormField {
        self.text_field = self
            .text_field
            .enable_inline_prediction(enable_inline_prediction);
        self
    }

    /// Dart `TextFormField(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> TextFormField {
        self.text_field = self.text_field.mouse_cursor(mouse_cursor);
        self
    }

    /// Dart `TextFormField(contextMenuBuilder:)`.
    pub fn context_menu_builder(
        mut self,
        context_menu_builder: Option<EditableTextContextMenuBuilder>,
    ) -> TextFormField {
        self.text_field = self.text_field.context_menu_builder(context_menu_builder);
        self
    }

    /// Dart `TextFormField(spellCheckConfiguration:)`.
    pub fn spell_check_configuration(
        mut self,
        spell_check_configuration: SpellCheckConfiguration,
    ) -> TextFormField {
        self.text_field = self
            .text_field
            .spell_check_configuration(spell_check_configuration);
        self
    }

    /// Dart `TextFormField(magnifierConfiguration:)`.
    pub fn magnifier_configuration(
        mut self,
        magnifier_configuration: TextMagnifierConfiguration,
    ) -> TextFormField {
        self.text_field = self
            .text_field
            .magnifier_configuration(magnifier_configuration);
        self
    }

    /// Dart `TextFormField(undoController:)`.
    pub fn undo_controller(mut self, undo_controller: Ref<UndoHistoryController>) -> TextFormField {
        self.text_field = self.text_field.undo_controller(undo_controller);
        self
    }

    /// Dart `TextFormField(onAppPrivateCommand:)`.
    pub fn on_app_private_command(
        mut self,
        on_app_private_command: impl Fn(&mut App, &str) + 'static,
    ) -> TextFormField {
        self.text_field = self
            .text_field
            .on_app_private_command(on_app_private_command);
        self
    }

    /// Dart `TextFormField(cursorOpacityAnimates:)`.
    pub fn cursor_opacity_animates(mut self, cursor_opacity_animates: bool) -> TextFormField {
        self.text_field = self
            .text_field
            .cursor_opacity_animates(cursor_opacity_animates);
        self
    }

    /// Dart `TextFormField(selectionHeightStyle:)`.
    pub fn selection_height_style(
        mut self,
        selection_height_style: BoxHeightStyle,
    ) -> TextFormField {
        self.text_field = self
            .text_field
            .selection_height_style(selection_height_style);
        self
    }

    /// Dart `TextFormField(selectionWidthStyle:)`.
    pub fn selection_width_style(mut self, selection_width_style: BoxWidthStyle) -> TextFormField {
        self.text_field = self.text_field.selection_width_style(selection_width_style);
        self
    }

    /// Dart `TextFormField(dragStartBehavior:)`.
    pub fn drag_start_behavior(mut self, drag_start_behavior: DragStartBehavior) -> TextFormField {
        self.text_field = self.text_field.drag_start_behavior(drag_start_behavior);
        self
    }

    /// Dart `TextFormField(contentInsertionConfiguration:)`.
    pub fn content_insertion_configuration(
        mut self,
        content_insertion_configuration: ContentInsertionConfiguration,
    ) -> TextFormField {
        self.text_field = self
            .text_field
            .content_insertion_configuration(content_insertion_configuration);
        self
    }

    /// Dart `TextFormField(statesController:)`.
    pub fn states_controller(
        mut self,
        states_controller: Ref<MaterialStatesController>,
    ) -> TextFormField {
        self.text_field = self.text_field.states_controller(states_controller);
        self
    }

    /// Dart `TextFormField(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> TextFormField {
        self.text_field = self.text_field.clip_behavior(clip_behavior);
        self
    }

    /// Dart `TextFormField(scribbleEnabled:)`.
    #[deprecated(note = "Use `stylus_handwriting_enabled` instead.")]
    pub fn scribble_enabled(mut self, scribble_enabled: bool) -> TextFormField {
        self.text_field = self.text_field.scribble_enabled(scribble_enabled);
        self
    }

    /// Dart `TextFormField(stylusHandwritingEnabled:)`.
    pub fn stylus_handwriting_enabled(mut self, stylus_handwriting_enabled: bool) -> TextFormField {
        self.text_field = self
            .text_field
            .stylus_handwriting_enabled(stylus_handwriting_enabled);
        self
    }

    /// Dart `TextFormField(canRequestFocus:)`.
    pub fn can_request_focus(mut self, can_request_focus: bool) -> TextFormField {
        self.text_field = self.text_field.can_request_focus(can_request_focus);
        self
    }

    /// Dart `TextFormField(hintLocales:)`.
    pub fn hint_locales(mut self, hint_locales: Vec<Locale>) -> TextFormField {
        self.text_field = self.text_field.hint_locales(hint_locales);
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
        AdaptiveTextSelectionToolbar::editable_text(app, editable_text_state).into_widget()
    }

    /// Dart's `controller != null ? controller.text : (initialValue ?? '')`, which the
    /// constructor hands the form field; with a controller it is `None` here, as its text needs
    /// the App, and the state reads it when the widget arrives.
    fn form_field_initial_value(&self) -> Option<String> {
        match self.controller {
            Some(_) => None,
            None => Some(self.initial_value.clone().unwrap_or_default()),
        }
    }

    /// Dart's `enabled ?? decoration?.enabled ?? true`, which the constructor hands both the
    /// form field and the text field.
    fn effective_enabled(&self) -> bool {
        self.text_field
            .enabled
            .or(self
                .text_field
                .decoration
                .as_ref()
                .map(|decoration| decoration.enabled))
            .unwrap_or(true)
    }

    /// Dart's builder closure: the [`TextField`] for the form field's current state.
    fn build_field(app: &mut App, field: AnyFormFieldState<String>) -> WidgetRef {
        let state = field
            .downcast::<TextFormFieldState>(app)
            .expect("a TextFormField's state is a TextFormFieldState");
        let context = field.context(app);
        let decoration_theme = InputDecorationTheme::of(app, context);
        let widget = state.widget(app);
        let mut effective_decoration = widget
            .text_field
            .decoration
            .clone()
            .unwrap_or_default()
            .apply_defaults(decoration_theme);
        let error_builder = widget.form_field.error_builder.clone();

        if let Some(error_text) = field.error_text(app) {
            effective_decoration = match error_builder {
                Some(error_builder) => {
                    let error = error_builder(app, state.context(app), error_text);
                    effective_decoration.copy_with().error(error)
                }
                None => effective_decoration.copy_with().error_text(error_text),
            };
        }

        let widget = state.widget(app);
        let on_changed = widget.on_changed.clone();
        let on_changed_handler = move |app: &mut App, value: String| {
            field.did_change(app, Some(value.clone()));
            if let Some(on_changed) = &on_changed {
                on_changed(app, value);
            }
        };

        let text_field = &widget.text_field;
        let child = text_field
            .clone()
            .group_id(widget.group_id)
            .controller(state.effective_controller(app))
            .decoration(Some(effective_decoration))
            .smart_dashes_type(text_field.effective_smart_dashes_type())
            .smart_quotes_type(text_field.effective_smart_quotes_type())
            .on_changed(on_changed_handler)
            .enabled(widget.effective_enabled())
            .enable_interactive_selection(
                text_field
                    .enable_interactive_selection
                    .unwrap_or(!text_field.obscure_text || !text_field.read_only),
            )
            .selection_height_style(
                text_field
                    .selection_height_style
                    .unwrap_or_else(EditableText::default_selection_height_style),
            )
            .selection_width_style(
                text_field
                    .selection_width_style
                    .unwrap_or_else(EditableText::default_selection_width_style),
            );

        UnmanagedRestorationScope::new(child)
            .bucket(state.bucket(app).and_then(|bucket| bucket.upgrade(app)))
            .into_widget()
    }

    /// Dart's constructor asserts across fields.
    fn debug_check_constructor(&self) {
        let text_field = &self.text_field;
        debug_assert!(self.initial_value.is_none() || self.controller.is_none());
        debug_assert!(
            text_field
                .max_lines
                .is_none_or(|max| text_field.min_lines.is_none_or(|min| max >= min)),
            "minLines can't be greater than maxLines"
        );
        debug_assert!(
            !text_field.expands
                || (text_field.max_lines.is_none() && text_field.min_lines.is_none()),
            "minLines and maxLines must be null when expands is true."
        );
        debug_assert!(
            !text_field.obscure_text || text_field.max_lines == Some(1),
            "Obscured fields cannot be multiline."
        );
        debug_assert!(
            self.form_field.error_builder.is_none()
                || text_field
                    .decoration
                    .as_ref()
                    .is_none_or(|decoration| decoration.error_text.is_none()),
            "Declaring both errorBuilder and decoration.errorText is not supported."
        );
    }
}

impl Default for TextFormField {
    fn default() -> TextFormField {
        TextFormField::new()
    }
}

impl Debug for TextFormField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextFormField")
            .field("form_field", &self.form_field)
            .field("controller", &self.controller)
            .field("text_field", &self.text_field)
            .finish_non_exhaustive()
    }
}

impl FormFieldLeaf for TextFormField {
    type Value = String;
    waft_widgets::form_field_accessors!();
}

impl StatefulWidget for TextFormField {
    type State = TextFormFieldState;

    fn key(&self) -> Option<&KeyRef> {
        self.form_field.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        TextFormField::debug_check_constructor(self);
    }

    fn create_state(&self) -> TextFormFieldState {
        TextFormFieldState {
            state: StateData::new(),
            restoration: RestorationMixinData::new(),
            form_field_state: FormFieldStateData::new(&self.form_field),
            controller: None,
            initial_value: None,
            widget_initial_value: None,
        }
    }
}

/// Dart's `_TextFormFieldState`.
pub struct TextFormFieldState {
    state: StateData<TextFormField>,
    restoration: RestorationMixinData,
    form_field_state: FormFieldStateData<String>,
    controller: Option<Ref<RestorableTextEditingController>>,
    initial_value: Option<String>,
    /// Dart's `widget.initialValue` for a widget with a controller: the controller's text when
    /// the widget arrived, which Dart's constructor reads.
    widget_initial_value: Option<String>,
}

impl TextFormFieldState {
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

    /// Records what Dart's constructor gives the form field as `initialValue` when the widget
    /// has a controller: the controller's text.
    fn read_widget_initial_value(self: Handle<Self>, app: &mut App) {
        let text = self
            .widget(app)
            .controller
            .as_ref()
            .map(|controller| controller.text_value(app).to_owned());
        app.get_mut(self).widget_initial_value = text;
    }

    fn handle_controller_changed(self: Handle<Self>, app: &mut App) {
        // Suppress changes that originated from within this class.
        //
        // In the case where a controller has been passed in to this widget, we
        // register this change listener. In these cases, we'll also receive change
        // notifications for changes originating from within this class -- for
        // example, the reset() method. In such cases, the FormField value will
        // already have been set.
        let text = self.effective_controller(app).text_value(app).to_owned();
        if self.value(app) != Some(&text) {
            self.did_change(app, Some(text));
        }
    }
}

impl RestorationMixin for TextFormFieldState {
    waft_widgets::restoration_mixin_accessors!();

    fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
        FormFieldState::restoration_id(self, app)
    }

    fn restore_state(
        self: Handle<Self>,
        app: &mut App,
        old_bucket: Option<Handle<RestorationBucket>>,
        initial_restore: bool,
    ) {
        FormFieldState::restore_state(self, app, old_bucket, initial_restore);
        if app.get(self).controller.is_some() {
            self.register_controller(app);
        }
        // Make sure to update the internal [FormFieldState] value to sync up with
        // text editing controller value.
        let text = self.effective_controller(app).text_value(app).to_owned();
        self.set_value(app, Some(text));
    }
}

impl FormFieldStateLeaf for TextFormFieldState {
    waft_widgets::form_field_state_accessors!();

    fn did_change(self: Handle<Self>, app: &mut App, value: Option<String>) {
        FormFieldState::did_change(self, app, value.clone());

        let controller = self.effective_controller(app);
        if Some(controller.text_value(app)) != value.as_deref() {
            controller.set_value(app, TextEditingValue::new().text(value.unwrap_or_default()));
        }
    }

    fn reset(self: Handle<Self>, app: &mut App) {
        // Set the controller value before calling super.reset() to let
        // _handleControllerChanged suppress the change.
        let initial_value = app.get(self).initial_value.clone().unwrap_or_default();
        self.effective_controller(app)
            .set_value(app, TextEditingValue::new().text(initial_value));
        FormFieldState::reset(self, app);
        if self.widget(app).controller.is_some() {
            let widget_initial_value = app.get(self).widget_initial_value.clone();
            self.set_value(app, widget_initial_value);
        }
        if let Some(on_changed) = self.widget(app).on_changed.clone() {
            let text = self.effective_controller(app).text_value(app).to_owned();
            on_changed(app, text);
        }
    }
}

impl State for TextFormFieldState {
    type Widget = TextFormField;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        FormFieldState::init_state(self, app);
        match self.widget(app).controller.clone() {
            None => {
                let value = self
                    .widget(app)
                    .form_field
                    .initial_value
                    .clone()
                    .map(|initial_value| TextEditingValue::new().text(initial_value));
                self.create_local_controller(app, value);
            }
            Some(controller) => {
                controller.add_listener(
                    app,
                    Listener::handle_method(self, Self::handle_controller_changed),
                );
            }
        }
        self.read_widget_initial_value(app);
        let initial_value = self
            .widget(app)
            .form_field
            .initial_value
            .clone()
            .or_else(|| app.get(self).widget_initial_value.clone());
        app.get_mut(self).initial_value = initial_value;
        if self.widget(app).controller.is_some() {
            let value = app.get(self).widget_initial_value.clone();
            self.set_value(app, value);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &TextFormField) {
        FormFieldState::did_update_widget(self, app, old_widget);
        self.read_widget_initial_value(app);
        if self.widget(app).controller != old_widget.controller {
            if let Some(old_controller) = old_widget.controller.clone() {
                old_controller.remove_listener(
                    app,
                    &Listener::handle_method(self, Self::handle_controller_changed),
                );
            }
            if let Some(controller) = self.widget(app).controller.clone() {
                controller.add_listener(
                    app,
                    Listener::handle_method(self, Self::handle_controller_changed),
                );
            }

            if let Some(old_controller) = old_widget.controller.clone()
                && self.widget(app).controller.is_none()
            {
                let value = old_controller.value(app).clone();
                self.create_local_controller(app, Some(value));
            }

            if let Some(controller) = self.widget(app).controller.clone() {
                let text = controller.text_value(app).to_owned();
                self.set_value(app, Some(text));
                if old_widget.controller.is_none() {
                    let local = app
                        .get(self)
                        .controller
                        .handle()
                        .expect("a local controller while the old widget had none");
                    self.unregister_from_restoration(app, local.as_property());
                    RestorableProperty::dispose(local, app);
                    app.get_mut(self).controller = None;
                }
            }
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        FormFieldState::did_change_dependencies(self, app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        FormFieldState::deactivate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(controller) = self.widget(app).controller.clone() {
            controller.remove_listener(
                app,
                &Listener::handle_method(self, Self::handle_controller_changed),
            );
        }
        if let Some(controller) = app.get(self).controller.handle() {
            RestorableProperty::dispose(controller, app);
        }
        FormFieldState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        FormFieldState::build(self, app, context)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use waft_embedder::{TextAffinity, TextSelection};
    use waft_services::TextInputClient;
    use waft_widgets::{
        AnyElement, Center, Form, FormState, GlobalKey, ListView, StateSetter, StatefulBuilder,
        Text, TextEditingController,
    };

    use waft_widgets::testing::{WidgetTester, test_widgets};

    use super::*;
    use crate::material::Material;
    use crate::scaffold::Scaffold;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;
    use waft_widgets::TextEditingControllerLeaf;

    /// Dart's `MaterialApp(home: Material(child: Center(child: field)))`.
    fn centered<K>(tester: &WidgetTester, field: impl IntoWidget<K>) -> WidgetRef {
        material_app(
            ThemeData::new(&tester.cell.borrow()),
            Material::new()
                .child(Center::new().child(field))
                .into_widget(),
        )
    }

    fn editable_state(tester: &WidgetTester) -> Handle<EditableTextState> {
        tester.find_by_type::<EditableText>()[0]
            .state_handle::<EditableTextState>(&tester.cell.borrow())
            .expect("an EditableText state")
    }

    fn field_state(tester: &WidgetTester) -> Handle<TextFormFieldState> {
        tester.find_by_type::<TextFormField>()[0]
            .state_handle::<TextFormFieldState>(&tester.cell.borrow())
            .expect("a TextFormField state")
    }

    fn text_field(tester: &WidgetTester) -> AnyElement {
        tester.find_by_type::<TextField>()[0]
    }

    /// Dart's `tester.showKeyboard(find.byType(TextField))`.
    fn show_keyboard(tester: &mut WidgetTester) {
        let state = editable_state(tester);
        state.request_keyboard(&mut tester.cell.borrow_mut());
        tester.pump();
    }

    /// Dart's `tester.enterText(finder, text)`.
    fn enter_text(tester: &mut WidgetTester, text: &str) {
        show_keyboard(tester);
        let state = editable_state(tester);
        let length = text.encode_utf16().count() as i32;
        let value = TextEditingValue::new()
            .text(text)
            .selection(TextSelection::collapsed(length, TextAffinity::Downstream));
        TextInputClient::update_editing_value(state, &mut tester.cell.borrow_mut(), value);
        tester.cell.checkpoint();
    }

    #[test]
    fn text_form_field_accepts_text_field_no_max_length_as_value_to_max_length_parameter() {
        let _ = TextFormField::new()
            .max_length(TextField::NO_MAX_LENGTH)
            .into_widget();
    }

    #[test]
    fn passes_text_align_to_underlying_text_field() {
        test_widgets(|tester| {
            let root = centered(tester, TextFormField::new().text_align(TextAlign::Center));
            tester.pump_widget(root);

            assert_eq!(tester.find_by_type::<TextField>().len(), 1);
            let text_align =
                tester.widget::<TextField, _>(text_field(tester), |field| field.text_align);
            assert_eq!(text_align, TextAlign::Center);
        });
    }

    #[test]
    fn passes_text_input_action_to_underlying_text_field() {
        test_widgets(|tester| {
            let root = centered(
                tester,
                TextFormField::new().text_input_action(TextInputAction::Next),
            );
            tester.pump_widget(root);

            let action =
                tester.widget::<TextField, _>(text_field(tester), |field| field.text_input_action);
            assert_eq!(action, Some(TextInputAction::Next));
        });
    }

    #[test]
    // Dart's 3.14 and 6.28 are plain values, not approximations of π and τ.
    #[allow(clippy::approx_constant)]
    fn passes_cursor_attributes_to_underlying_text_field() {
        test_widgets(|tester| {
            let cursor_color = crate::colors::Colors::PURPLE.primary();
            let root = centered(
                tester,
                TextFormField::new()
                    .cursor_width(3.14)
                    .cursor_height(6.28)
                    .cursor_radius(Radius::circular(4.0))
                    .cursor_color(cursor_color),
            );
            tester.pump_widget(root);

            let (width, height, radius, color) =
                tester.widget::<TextField, _>(text_field(tester), |field| {
                    (
                        field.cursor_width,
                        field.cursor_height,
                        field.cursor_radius,
                        field.cursor_color,
                    )
                });
            assert_eq!(width, 3.14);
            assert_eq!(height, Some(6.28));
            assert_eq!(radius, Some(Radius::circular(4.0)));
            assert_eq!(color, Some(cursor_color));
        });
    }

    #[test]
    fn on_field_submit_callbacks_are_called() {
        test_widgets(|tester| {
            let called = Rc::new(Cell::new(false));
            let flag = Rc::clone(&called);
            let root = centered(
                tester,
                TextFormField::new().on_field_submitted(move |_app, _value| flag.set(true)),
            );
            tester.pump_widget(root);

            show_keyboard(tester);
            let state = editable_state(tester);
            TextInputClient::perform_action(
                state,
                &mut tester.cell.borrow_mut(),
                TextInputAction::Done,
            );
            tester.pump();
            assert!(called.get());
        });
    }

    #[test]
    fn on_changed_callbacks_are_called() {
        test_widgets(|tester| {
            let value = Rc::new(RefCell::new(String::new()));
            let seen = Rc::clone(&value);
            let root = centered(
                tester,
                TextFormField::new().on_changed(move |_app, v| *seen.borrow_mut() = v),
            );
            tester.pump_widget(root);

            enter_text(tester, "Soup");
            tester.pump();
            assert_eq!(*value.borrow(), "Soup");
        });
    }

    fn validate_counting(tester: &mut WidgetTester, enabled: Option<bool>) {
        let validate_called = Rc::new(Cell::new(0));
        let counter = Rc::clone(&validate_called);
        let mut field = TextFormField::new()
            .autovalidate_mode(AutovalidateMode::Always)
            .validator(move |_app, _value| {
                counter.set(counter.get() + 1);
                None
            });
        if let Some(enabled) = enabled {
            field = field.enabled(enabled);
        }
        let root = centered(tester, field);
        tester.pump_widget(root);

        assert_eq!(validate_called.get(), 1);
        enter_text(tester, "a");
        tester.pump();
        assert_eq!(validate_called.get(), 2);
    }

    #[test]
    fn autovalidate_mode_is_passed_to_super() {
        test_widgets(|tester| validate_counting(tester, None));
    }

    #[test]
    fn validate_is_called_if_widget_is_enabled() {
        test_widgets(|tester| validate_counting(tester, Some(true)));
    }

    #[test]
    fn passing_a_build_counter_shows_returned_widget() {
        test_widgets(|tester| {
            let root = centered(
                tester,
                TextFormField::new()
                    .build_counter(|_app, _context, current_length, max_length, _is_focused| {
                        let max_length =
                            max_length.map_or_else(|| "null".to_owned(), |max| max.to_string());
                        Some(Text::new(format!("{current_length} of {max_length}")).into_widget())
                    })
                    .max_length(10),
            );
            tester.pump_widget(root);

            assert_eq!(tester.find_text("0 of 10").len(), 1);

            enter_text(tester, "01234");
            tester.pump();

            assert_eq!(tester.find_text("5 of 10").len(), 1);
        });
    }

    #[test]
    fn readonly_text_form_field_will_hide_cursor_by_default() {
        test_widgets(|tester| {
            let root = centered(
                tester,
                TextFormField::new()
                    .initial_value("readonly")
                    .read_only(true),
            );
            tester.pump_widget(root);

            let element = tester.find_by_type::<EditableText>()[0];
            let show_cursor =
                tester.widget::<EditableText, _>(element, EditableText::effective_show_cursor);
            assert!(!show_cursor);
        });
    }

    fn editable_text_value(tester: &WidgetTester) -> String {
        let state = editable_state(tester);
        TextInputClient::current_text_editing_value(state, &tester.cell.borrow())
            .expect("the field has a value")
            .text
    }

    #[test]
    fn reset_resets_the_text_fields_value_to_the_initial_value() {
        test_widgets(|tester| {
            let root = centered(tester, TextFormField::new().initial_value("initialValue"));
            tester.pump_widget(root);

            enter_text(tester, "changedValue");

            let state = field_state(tester);
            state.reset(&mut tester.cell.borrow_mut());
            tester.pump();

            assert_eq!(editable_text_value(tester), "initialValue");
        });
    }

    #[test]
    fn reset_resets_the_text_fields_value_to_the_controller_initial_value() {
        test_widgets(|tester| {
            let controller = {
                let mut app = tester.cell.borrow_mut();
                let controller = TextEditingController::new(&mut app);
                controller.set_text(&mut app, "initialValue");
                controller
            };
            let root = centered(tester, TextFormField::new().controller(controller.erase()));
            tester.pump_widget(root);

            enter_text(tester, "changedValue");

            let state = field_state(tester);
            state.reset(&mut tester.cell.borrow_mut());
            tester.pump();

            assert_eq!(editable_text_value(tester), "initialValue");
        });
    }

    #[test]
    fn did_change_changes_text_fields_value() {
        test_widgets(|tester| {
            let root = centered(tester, TextFormField::new().initial_value("initialValue"));
            tester.pump_widget(root);

            assert_eq!(editable_text_value(tester), "initialValue");

            let state = field_state(tester);
            state.did_change(&mut tester.cell.borrow_mut(), Some("changedValue".into()));
            tester.pump();

            assert_eq!(editable_text_value(tester), "changedValue");
        });
    }

    #[test]
    fn on_changed_callbacks_value_and_form_field_state_value_are_sync() {
        test_widgets(|tester| {
            let called = Rc::new(Cell::new(false));
            let state_slot: Rc<Cell<Option<Handle<TextFormFieldState>>>> = Rc::new(Cell::new(None));
            let (flag, slot) = (Rc::clone(&called), Rc::clone(&state_slot));
            let root = centered(
                tester,
                TextFormField::new().on_changed(move |app, value| {
                    flag.set(true);
                    let state = slot.get().expect("the case records the state");
                    assert_eq!(state.value(app), Some(&value));
                }),
            );
            tester.pump_widget(root);

            state_slot.set(Some(field_state(tester)));

            enter_text(tester, "Soup");

            assert!(called.get());
        });
    }

    #[test]
    fn text_form_field_on_changed_is_called_when_the_form_is_reset() {
        test_widgets(|tester| {
            let state_key = GlobalKey::new();
            let form_key = GlobalKey::new();
            let value = Rc::new(RefCell::new("initialValue".to_owned()));
            let seen = Rc::clone(&value);
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                Scaffold::new()
                    .body(
                        Form::new(
                            TextFormField::new()
                                .key(state_key.clone().into())
                                .initial_value(value.borrow().clone())
                                .on_changed(move |_app, new_value| *seen.borrow_mut() = new_value),
                        )
                        .key(form_key.clone().into()),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            let field_value = |tester: &WidgetTester| {
                let app = tester.cell.borrow();
                let state = state_key
                    .current_state::<TextFormFieldState>(&app)
                    .expect("the field mounted");
                state.value(&app).cloned()
            };

            // Initial value is 'initialValue'.
            assert_eq!(field_value(tester).as_deref(), Some("initialValue"));
            assert_eq!(*value.borrow(), "initialValue");

            // Change value to 'changedValue'.
            enter_text(tester, "changedValue");
            assert_eq!(field_value(tester).as_deref(), Some("changedValue"));
            assert_eq!(*value.borrow(), "changedValue");

            // Should be back to 'initialValue' when the form is reset.
            {
                let mut app = tester.cell.borrow_mut();
                let form = form_key
                    .current_state::<FormState>(&app)
                    .expect("the form mounted");
                form.reset(&mut app);
            }
            tester.pump();
            assert_eq!(field_value(tester).as_deref(), Some("initialValue"));
            assert_eq!(*value.borrow(), "initialValue");
        });
    }

    #[test]
    fn is_valid_returns_false_when_force_error_text_is_set_and_will_change_error_display() {
        test_widgets(|tester| {
            let field_key1 = GlobalKey::new();
            let field_key2 = GlobalKey::new();
            let valid_string = "Valid string";
            let validator = move |_app: &mut App, s: Option<&String>| {
                if s.map(String::as_str) == Some(valid_string) {
                    None
                } else {
                    Some("Error text".to_owned())
                }
            };
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                Center::new()
                    .child(
                        Material::new().child(Form::new(ListView::new().children(vec![
                        TextFormField::new()
                            .key(field_key1.clone().into())
                            .initial_value(valid_string)
                            .validator(validator)
                            .autovalidate_mode(AutovalidateMode::Disabled)
                            .into_widget(),
                        TextFormField::new()
                            .key(field_key2.clone().into())
                            .initial_value("")
                            .force_error_text("Forcing error.")
                            .validator(validator)
                            .autovalidate_mode(AutovalidateMode::Disabled)
                            .into_widget(),
                    ]))),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            let mut app = tester.cell.borrow_mut();
            let state1 = field_key1
                .current_state::<TextFormFieldState>(&app)
                .expect("the first field mounted");
            let state2 = field_key2
                .current_state::<TextFormFieldState>(&app)
                .expect("the second field mounted");
            assert!(state1.is_valid(&mut app));
            assert!(!state1.has_error(&app));
            assert!(!state2.is_valid(&mut app));
            assert!(state2.has_error(&app));
        });
    }

    #[test]
    fn widget_returned_by_error_builder_is_shown() {
        test_widgets(|tester| {
            let root = centered(
                tester,
                TextFormField::new()
                    .autovalidate_mode(AutovalidateMode::Always)
                    .validator(|_app, _value| Some("validation error".to_owned()))
                    .error_builder(|_app, _context, error_text| {
                        Text::new(format!("**{error_text}**")).into_widget()
                    }),
            );
            tester.pump_widget(root);
            tester.pump();

            assert_eq!(tester.find_text("**validation error**").len(), 1);
        });
    }

    #[test]
    #[should_panic(
        expected = "Declaring both errorBuilder and decoration.errorText is not supported."
    )]
    fn text_form_field_asserts_when_both_error_builder_and_decoration_error_text_are_provided() {
        let _ = TextFormField::new()
            .decoration(Some(
                InputDecoration::new().error_text("decoration error text"),
            ))
            .error_builder(|_app, _context, error_text| Text::new(error_text).into_widget())
            .into_widget();
    }

    #[test]
    fn text_form_field_resets_to_initial_value_after_set_state() {
        test_widgets(|tester| {
            let form_key = GlobalKey::new();
            let controller = {
                let mut app = tester.cell.borrow_mut();
                let controller = TextEditingController::new(&mut app);
                controller.set_text(&mut app, "Initial Value");
                controller.clone()
            };
            let set_state: Rc<RefCell<Option<StateSetter>>> = Rc::new(RefCell::new(None));
            let setter_slot = Rc::clone(&set_state);
            let key_for_form = form_key.clone();
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                Scaffold::new()
                    .body(StatefulBuilder::new({
                        let controller = controller.clone();
                        move |_app, _context, setter| {
                            *setter_slot.borrow_mut() = Some(setter);
                            Form::new(TextFormField::new().controller(controller.erase()))
                                .key(key_for_form.clone().into())
                                .into_widget()
                        }
                    }))
                    .into_widget(),
            );
            tester.pump_widget(root);

            enter_text(tester, "Changed");
            tester.pump();
            assert_eq!(controller.text_value(&tester.cell.borrow()), "Changed");

            let setter = set_state.borrow().clone().expect("the builder ran");
            setter(&mut tester.cell.borrow_mut(), &mut |_app| {});
            tester.pump();

            {
                let mut app = tester.cell.borrow_mut();
                let form = form_key
                    .current_state::<FormState>(&app)
                    .expect("the form mounted");
                form.reset(&mut app);
            }
            tester.pump();

            assert_eq!(
                controller.text_value(&tester.cell.borrow()),
                "Initial Value"
            );
        });
    }
}
